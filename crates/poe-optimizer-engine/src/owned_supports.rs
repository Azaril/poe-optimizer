//! Bounded native ordered support selection and type preparation.
//!
//! Callers supply resolved preparation facts. This component neither reads raw
//! gems nor discovers receivers, emits modifiers, or grants build coverage.
use poe_optimizer_core::{
    build_identity::SupportAssignmentId,
    owned_build::SkillTarget,
    owned_content::OwnedContentDigest,
    owned_definitions::{BoundedInteger, FiniteQuantity, GemDefId, OwnedDefinitionKey},
    owned_schema::{DeclaredSet, SchemaState},
    owned_supports::{
        SupportPreparationDefinition, SupportPreparationPolicy, SupportTypePredicate,
    },
};
use poe_optimizer_data::owned_supports::OwnedSupportPreparation;
use serde::Serialize;
use std::{collections::BTreeSet, fmt};
mod build;
mod selection;
pub use build::{
    EffectiveSupportValues, prepare_build_supports, prepare_build_supports_with_budget,
};
pub use selection::{
    SelectedSupports, SupportSelectionOutcome, prepare_selected_supports,
    prepare_selected_supports_with_budget, select_supports, select_supports_with_budget,
};

/// Ordering is the slice order, independently of the assignment identifiers.
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedSupportOrigin {
    pub assignment: SupportAssignmentId,
    pub gem: GemDefId,
    pub enabled: Option<bool>,
    /// Already resolved by preparation dependencies; never a physical-gem fallback.
    pub effective_level: Option<BoundedInteger>,
    pub effective_quality: Option<FiniteQuantity>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportTypeContext {
    pub skill_types: DeclaredSet<OwnedDefinitionKey>,
    /// An absent summoner collection falls back to the child's collection;
    /// a present empty collection intentionally prevents that fallback.
    pub minion_types: Option<DeclaredSet<OwnedDefinitionKey>>,
}

/// Explicit receiving facts. `summoner: None` means there is no summoner, not an
/// unknown summoner. Unknown type membership uses a Partial declared set.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupportPreparationTarget {
    pub target: SkillTarget,
    pub enabled: Option<bool>,
    pub types: SupportTypeContext,
    pub summoner: Option<SupportTypeContext>,
    pub cannot_be_supported: Option<bool>,
    pub has_gem: Option<bool>,
    pub from_item: Option<bool>,
    pub is_player_actor: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub struct SupportPreparationLimits {
    pub max_origins: usize,
    pub max_types: usize,
    pub max_predicate_depth: usize,
    pub max_target_depth: usize,
    pub max_work: usize,
}
impl Default for SupportPreparationLimits {
    fn default() -> Self {
        Self {
            max_origins: 4096,
            max_types: 65536,
            max_predicate_depth: 64,
            max_target_depth: 256,
            max_work: 16 * 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub enum SupportPreparationError {
    Invalid(&'static str),
    Limit(&'static str),
}
impl fmt::Display for SupportPreparationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(message) => f.write_str(message),
            Self::Limit(name) => write!(f, "support preparation exceeds {name}"),
        }
    }
}
impl std::error::Error for SupportPreparationError {}
type Result<T> = std::result::Result<T, SupportPreparationError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SupportPreparationGap {
    OriginOrder,
    TargetEnabled,
    OriginEnabled,
    MissingDefinition,
    UnmappedDefinition,
    EffectiveLevel,
    EffectiveQuality,
    Applicability,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PreparedSupportPosition {
    pub position: usize,
    pub origin_index: usize,
    pub assignment: SupportAssignmentId,
    pub applicable: bool,
    /// True if the preparation algorithm visited an eligible position. It can
    /// remain false for a finally eligible support behind the retry frontier.
    pub type_additions_applied: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct PreparedSupports {
    pub preparation: OwnedContentDigest,
    pub target: SkillTarget,
    pub ordered_origins: Vec<SupportAssignmentId>,
    pub disabled_origins: Vec<usize>,
    pub selected: Vec<PreparedSupportPosition>,
    pub final_types: Vec<OwnedDefinitionKey>,
    pub final_types_complete: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum SupportPreparationOutcome {
    Known(PreparedSupports),
    Inactive {
        target: SkillTarget,
    },
    Unresolved {
        reason: SupportPreparationGap,
        origin_index: Option<usize>,
    },
}

fn unresolved(
    reason: SupportPreparationGap,
    origin_index: Option<usize>,
) -> SupportPreparationOutcome {
    SupportPreparationOutcome::Unresolved {
        reason,
        origin_index,
    }
}

struct Budget {
    limits: SupportPreparationLimits,
    remaining: usize,
}
impl SupportPreparationLimits {
    pub(crate) fn validate(self) -> Result<()> {
        let limits = self;
        let hard = SupportPreparationLimits::default();
        for (value, maximum) in [
            (limits.max_origins, hard.max_origins),
            (limits.max_types, hard.max_types),
            (limits.max_predicate_depth, hard.max_predicate_depth),
            (limits.max_target_depth, hard.max_target_depth),
            (limits.max_work, hard.max_work),
        ] {
            if value == 0 || value > maximum {
                return Err(SupportPreparationError::Invalid(
                    "invalid preparation limits",
                ));
            }
        }
        Ok(())
    }
}
impl Budget {
    fn new(limits: SupportPreparationLimits) -> Result<Self> {
        limits.validate()?;
        Ok(Self {
            limits,
            remaining: limits.max_work,
        })
    }
    fn charge(&mut self, amount: usize) -> Result<()> {
        match self.remaining.checked_sub(amount) {
            Some(remaining) => {
                self.remaining = remaining;
                Ok(())
            }
            None => {
                self.remaining = 0;
                Err(SupportPreparationError::Limit("work"))
            }
        }
    }
}

/// One allowance spans binding and policy execution. Validation errors before
/// an attempt do not consume work; all charged work, including failed attempts,
/// is deducted before returning. A component cannot replenish the outer budget.
fn with_budget<T>(
    limits: SupportPreparationLimits,
    remaining_work: &mut usize,
    run: impl FnOnce(&mut Budget) -> Result<T>,
) -> Result<T> {
    let mut budget = Budget::new(limits)?;
    let allowance = (*remaining_work).min(limits.max_work);
    budget.remaining = allowance;
    let result = budget.charge(1).and_then(|()| run(&mut budget));
    *remaining_work -= allowance - budget.remaining;
    result
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Truth {
    False,
    True,
    Unknown,
}
impl Truth {
    fn fact(value: Option<bool>) -> Self {
        match value {
            Some(true) => Self::True,
            Some(false) => Self::False,
            None => Self::Unknown,
        }
    }
    fn not(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::True => Self::False,
            Self::Unknown => Self::Unknown,
        }
    }
    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }
    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
}

struct Types {
    members: BTreeSet<OwnedDefinitionKey>,
    complete: bool,
}
impl Types {
    fn member(&self, id: &OwnedDefinitionKey, budget: &mut Budget) -> Result<Truth> {
        // This conservative scan charge also bounds the tree's comparisons.
        budget.charge(self.members.len() + 1)?;
        Ok(if self.members.contains(id) {
            Truth::True
        } else if self.complete {
            Truth::False
        } else {
            Truth::Unknown
        })
    }
    fn add(&mut self, types: &[OwnedDefinitionKey], budget: &mut Budget) -> Result<()> {
        budget.charge(types.len())?;
        for id in types {
            budget.charge(self.members.len() + 1)?;
            if !self.members.contains(id) {
                if self.members.len() >= budget.limits.max_types {
                    return Err(SupportPreparationError::Limit("types"));
                }
                self.members.insert(id.clone());
            }
        }
        Ok(())
    }
}

fn validate_types(
    types: &DeclaredSet<OwnedDefinitionKey>,
    package: &OwnedSupportPreparation,
    budget: &mut Budget,
) -> Result<()> {
    if types.members.len() > budget.limits.max_types {
        return Err(SupportPreparationError::Limit("types"));
    }
    budget.charge(types.members.len())?;
    let mut seen = BTreeSet::new();
    for id in &types.members {
        budget.charge(package.input().types.len() + seen.len() + 1)?;
        if !package.input().types.contains(id) {
            return Err(SupportPreparationError::Invalid(
                "target references an undeclared support type",
            ));
        }
        if !seen.insert(id) {
            return Err(SupportPreparationError::Invalid("duplicate target type"));
        }
    }
    Ok(())
}

fn known_types(types: &DeclaredSet<OwnedDefinitionKey>, budget: &mut Budget) -> Result<Types> {
    budget.charge(types.members.len())?;
    Ok(Types {
        members: types.members.iter().cloned().collect(),
        complete: types.is_complete(),
    })
}

fn predicate(
    expression: &SupportTypePredicate,
    types: &Types,
    minion: Option<&Types>,
    depth: usize,
    budget: &mut Budget,
) -> Result<Truth> {
    if depth > budget.limits.max_predicate_depth {
        return Err(SupportPreparationError::Limit("predicate depth"));
    }
    budget.charge(1)?;
    Ok(match expression {
        SupportTypePredicate::Type(id) => {
            let own = types.member(id, budget)?;
            match (own, minion) {
                (Truth::True, _) | (_, None) => own,
                (_, Some(minion)) => own.or(minion.member(id, budget)?),
            }
        }
        SupportTypePredicate::Not(value) => {
            predicate(value, types, minion, depth + 1, budget)?.not()
        }
        SupportTypePredicate::All(values) => {
            budget.charge(values.len())?;
            let mut value = Truth::True;
            for expression in values {
                value = value.and(predicate(expression, types, minion, depth + 1, budget)?);
                if value == Truth::False {
                    break;
                }
            }
            value
        }
        SupportTypePredicate::Any(values) => {
            budget.charge(values.len())?;
            let mut value = Truth::False;
            for expression in values {
                value = value.or(predicate(expression, types, minion, depth + 1, budget)?);
                if value == Truth::True {
                    break;
                }
            }
            value
        }
    })
}

struct TargetTypes {
    current: Types,
    minion: Option<Types>,
    summoner: Option<(Types, Option<Types>)>,
}
fn applicability(
    definition: &SupportPreparationDefinition,
    target: &SupportPreparationTarget,
    types: &TargetTypes,
    budget: &mut Budget,
) -> Result<Truth> {
    budget.charge(1)?;
    let mut result = Truth::fact(target.cannot_be_supported).not();
    if result == Truth::False {
        return Ok(result);
    }
    if definition.gems_only {
        result = result.and(Truth::fact(target.has_gem));
    }
    if definition.from_item && definition.is_support {
        result = result.and(Truth::fact(target.from_item).not());
    }
    if definition.is_trigger {
        result = result.and(Truth::fact(target.is_player_actor));
    }
    if result == Truth::False {
        return Ok(result);
    }
    let effective = types
        .summoner
        .as_ref()
        .map_or(&types.current, |(own, _)| own);
    let minion = types
        .summoner
        .as_ref()
        .and_then(|(_, minion)| minion.as_ref())
        .or(types.minion.as_ref());
    if let Some(excludes) = &definition.excludes {
        result = result.and(predicate(excludes, effective, None, 1, budget)?.not());
    }
    if result == Truth::False {
        return Ok(result);
    }
    if let Some(requires) = &definition.requires {
        result = result.and(predicate(
            requires,
            effective,
            if definition.ignore_minion_types {
                None
            } else {
                minion
            },
            1,
            budget,
        )?);
    }
    Ok(result)
}

/// Execute the package's versioned native preparation policy. A Known result is
/// component evidence only: it is not receiver binding, legality, or build parity.
pub fn prepare_supports(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    target: &SupportPreparationTarget,
    limits: SupportPreparationLimits,
) -> Result<SupportPreparationOutcome> {
    let mut work = limits.max_work;
    prepare_supports_with_budget(package, origins, target, limits, &mut work)
}

/// Execute preparation under a shared evaluation-attempt work budget. The
/// component's limit also applies; neither successful nor failed calls reset
/// the caller's remaining allowance. This does not add any coverage authority.
pub fn prepare_supports_with_budget(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    target: &SupportPreparationTarget,
    limits: SupportPreparationLimits,
    remaining_work: &mut usize,
) -> Result<SupportPreparationOutcome> {
    with_budget(limits, remaining_work, |budget| {
        prepare_supports_inner(package, origins, target, budget)
    })
}

fn prepare_supports_inner(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    target: &SupportPreparationTarget,
    budget: &mut Budget,
) -> Result<SupportPreparationOutcome> {
    // Preserve the original combined call's validation and uncertainty order.
    validate_origin_count(origins, budget)?;
    validate_target_address(package, target, budget)?;
    validate_origins(package, origins, budget)?;
    if let Some(outcome) = target_activity(target) {
        return Ok(outcome);
    }
    let types = target_types(package, target, budget)?;
    match selection::select_validated(package, origins, budget)? {
        SupportSelectionOutcome::Known(selected) => {
            admit_selected(package, &selected, target, types, budget)
        }
        SupportSelectionOutcome::Unresolved {
            reason,
            origin_index,
        } => Ok(unresolved(reason, origin_index)),
    }
}

fn validate_origin_count(origins: &[ResolvedSupportOrigin], budget: &mut Budget) -> Result<()> {
    let limits = budget.limits;
    if origins.len() > limits.max_origins {
        return Err(SupportPreparationError::Limit("origins"));
    }
    budget.charge(origins.len())?;
    Ok(())
}

fn validate_target_address(
    package: &OwnedSupportPreparation,
    target: &SupportPreparationTarget,
    budget: &mut Budget,
) -> Result<()> {
    let limits = budget.limits;
    if let SkillTarget::Generated(skill) = &target.target {
        if skill.provider.grant_path.len() > limits.max_target_depth {
            return Err(SupportPreparationError::Limit("target depth"));
        }
        budget.charge(skill.provider.grant_path.len() + 1)?;
        if skill.slot.slot.namespace() != &package.input().namespace
            || skill.slot.declaration.namespace() != &package.input().namespace
            || skill.provider.grant_path.iter().any(|slot| {
                slot.slot.namespace() != &package.input().namespace
                    || slot.declaration.namespace() != &package.input().namespace
            })
        {
            return Err(SupportPreparationError::Invalid(
                "foreign support target namespace",
            ));
        }
    }
    Ok(())
}

fn validate_origins(
    package: &OwnedSupportPreparation,
    origins: &[ResolvedSupportOrigin],
    budget: &mut Budget,
) -> Result<()> {
    let mut seen = BTreeSet::new();
    for origin in origins {
        budget.charge(seen.len() + 1)?;
        if !seen.insert(origin.assignment) {
            return Err(SupportPreparationError::Invalid(
                "duplicate support origin assignment",
            ));
        }
        if origin.gem.namespace() != &package.input().namespace {
            return Err(SupportPreparationError::Invalid(
                "foreign support origin namespace",
            ));
        }
        if origin
            .effective_quality
            .as_ref()
            .is_some_and(|quality| quality.unit() != &package.input().quality_unit)
        {
            return Err(SupportPreparationError::Invalid(
                "effective support quality unit differs from package",
            ));
        }
    }
    Ok(())
}

fn target_activity(target: &SupportPreparationTarget) -> Option<SupportPreparationOutcome> {
    match target.enabled {
        Some(false) => Some(SupportPreparationOutcome::Inactive {
            target: target.target.clone(),
        }),
        None => Some(unresolved(SupportPreparationGap::TargetEnabled, None)),
        Some(true) => None,
    }
}

fn target_types(
    package: &OwnedSupportPreparation,
    target: &SupportPreparationTarget,
    budget: &mut Budget,
) -> Result<TargetTypes> {
    for context in std::iter::once(&target.types).chain(target.summoner.as_ref()) {
        validate_types(&context.skill_types, package, budget)?;
        if let Some(minion) = &context.minion_types {
            validate_types(minion, package, budget)?;
        }
    }
    Ok(TargetTypes {
        current: known_types(&target.types.skill_types, budget)?,
        minion: target
            .types
            .minion_types
            .as_ref()
            .map(|types| known_types(types, budget))
            .transpose()?,
        summoner: target
            .summoner
            .as_ref()
            .map(|context| {
                Ok((
                    known_types(&context.skill_types, budget)?,
                    context
                        .minion_types
                        .as_ref()
                        .map(|types| known_types(types, budget))
                        .transpose()?,
                ))
            })
            .transpose()?,
    })
}

fn admit_selected(
    package: &OwnedSupportPreparation,
    selection: &SelectedSupports,
    target: &SupportPreparationTarget,
    mut types: TargetTypes,
    budget: &mut Budget,
) -> Result<SupportPreparationOutcome> {
    let origins = selection.origins();
    let selected = selection.selected_origin_indices();
    // Resolve only retained definitions once, even when the preparation policy
    // revisits positions. Repeated positions preserve distinct admission state.
    budget.charge(origins.len() + selected.len())?;
    let mut definitions = vec![None; origins.len()];
    for &origin in selected {
        if definitions[origin].is_none() {
            budget.charge(package.input().supports.len() + 1)?;
            let Some(SchemaState::Known(definition)) =
                package.preparation_for(&origins[origin].gem)
            else {
                return Err(SupportPreparationError::Invalid(
                    "sealed support selection has no definition",
                ));
            };
            definitions[origin] = Some(definition);
        }
    }
    budget.charge(selected.len())?;
    let mut applied = vec![false; selected.len()];
    let mut rejected = Vec::new();
    for (position, &origin) in selected.iter().enumerate() {
        let definition = definitions[origin].expect("selected definition");
        match applicability(definition, target, &types, budget)? {
            Truth::True => {
                types.current.add(&definition.added_types, budget)?;
                applied[position] = true;
            }
            Truth::False => {
                budget.charge(1)?;
                rejected.push(position);
            }
            Truth::Unknown => {
                return Ok(unresolved(
                    SupportPreparationGap::Applicability,
                    Some(origin),
                ));
            }
        }
    }
    // Preserve the reviewed policy's retry frontier without representing a Lua
    // table. Accepted entries do not truncate the current pass; the first such
    // position terminates the next pass. Every progressing pass consumes one
    // previously rejected position, giving an origins+1 bound.
    let mut frontier = rejected.len();
    for _ in 0..=selected.len() {
        budget.charge(1)?;
        let mut first_accepted = None;
        for (index, &position) in rejected[..frontier].iter().enumerate() {
            budget.charge(1)?;
            let origin = selected[position];
            let definition = definitions[origin].expect("selected definition");
            match applicability(definition, target, &types, budget)? {
                Truth::True => {
                    first_accepted.get_or_insert(index);
                    applied[position] = true;
                    types.current.add(&definition.added_types, budget)?;
                }
                Truth::False => {}
                Truth::Unknown => {
                    return Ok(unresolved(
                        SupportPreparationGap::Applicability,
                        Some(origin),
                    ));
                }
            }
        }
        match first_accepted {
            Some(next) => frontier = next,
            None => break,
        }
    }
    budget.charge(
        selected.len()
            + origins.len()
            + selection.disabled_origins().len()
            + types.current.members.len(),
    )?;
    let mut positions = Vec::with_capacity(selected.len());
    for (position, &origin) in selected.iter().enumerate() {
        let applicable = match applicability(
            definitions[origin].expect("selected definition"),
            target,
            &types,
            budget,
        )? {
            Truth::True => true,
            Truth::False => false,
            Truth::Unknown => {
                return Ok(unresolved(
                    SupportPreparationGap::Applicability,
                    Some(origin),
                ));
            }
        };
        positions.push(PreparedSupportPosition {
            position,
            origin_index: origin,
            assignment: origins[origin].assignment,
            applicable,
            type_additions_applied: applied[position],
        });
    }
    Ok(SupportPreparationOutcome::Known(PreparedSupports {
        preparation: *package.identity(),
        target: target.target.clone(),
        ordered_origins: origins.iter().map(|origin| origin.assignment).collect(),
        disabled_origins: selection.disabled_origins().to_vec(),
        selected: positions,
        final_types: types.current.members.into_iter().collect(),
        final_types_complete: types.current.complete,
    }))
}
