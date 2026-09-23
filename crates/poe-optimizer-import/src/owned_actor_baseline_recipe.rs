//! Compile finite actor acquisition into ordinary owned expressions and tables.
//! No source execution, actor discovery, level inference or source-name dispatch
//! is present in the resulting native package. Existing coverage stays unchanged.
use crate::{
    owned_actor_baselines::*, owned_mapping::SourcePin, owned_recipe::*, owned_recipe_extension::*,
};
use poe_optimizer_core::{
    owned_build::{DeclaredSlot, ParameterValue},
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselinePolicy {
    pub schema_version: u32,
    pub version: OwnedDefinitionKey,
    pub catalog_sha256: String,
    /// Explicit acquisition provenance claim, checked against the byte-bound
    /// catalog. Only the optional authenticated exporter proves source derivation.
    pub source: SourcePin,
    pub bindings: Vec<ActorBaselineBinding>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineBinding {
    pub profile: String,
    pub actor: DeclaredSlot<ActorSlotDefId>,
    pub program: OwnedDefinitionKey,
    pub fields: Vec<ActorScalarBinding>,
    pub curves: Vec<ActorCurveBinding>,
    /// Omit when some existing producer supplies the actor level. Never inferred
    /// from a gem definition, a source profile, or a selected tree occurrence.
    pub summon_level: Option<ActorSummonLevelBinding>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorScalarBinding {
    pub field: ActorBaselineField,
    pub target: ActorScalarTarget,
    pub when_absent: ActorFactAbsence,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorFactAbsence {
    Reject,
    Omit,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ActorScalarTarget {
    Quantity { stat: StatDefId, unit: UnitDefId },
    Boolean { stat: StatDefId },
}
impl ActorScalarTarget {
    fn stat(&self) -> &StatDefId {
        match self {
            Self::Quantity { stat, .. } | Self::Boolean { stat } => stat,
        }
    }
    fn value_type(&self) -> ComputedValueType {
        match self {
            Self::Quantity { unit, .. } => ComputedValueType::Quantity { unit: unit.clone() },
            Self::Boolean { .. } => ComputedValueType::Boolean,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorBaselineField {
    AttackTime,
    DamageScale,
    DamageSpread,
    CriticalChance,
    AttackRange,
    BaseDamageIgnoresAttackSpeed,
    Hostile,
    Accuracy,
    Armour,
    Evasion,
    EnergyShield,
    Life,
    BaseMovementSpeed,
    FireResistance,
    ColdResistance,
    LightningResistance,
    ChaosResistance,
    CompanionFireResistance,
    CompanionColdResistance,
    CompanionLightningResistance,
    CompanionChaosResistance,
    CompanionReservation,
    SpectreReservation,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorDamageCurve {
    Allied,
    Hostile,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorCurveBinding {
    pub curve: ActorDamageCurve,
    pub actor_level: StatDefId,
    pub table: OwnedDefinitionKey,
    pub stat: StatDefId,
    pub unit: UnitDefId,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorSummonLevelBinding {
    pub parameter: DeclaredSlot<ParameterSlotDefId>,
    pub stat: StatDefId,
    pub table: OwnedDefinitionKey,
    pub program: OwnedDefinitionKey,
}
#[derive(Clone, Copy, Debug)]
pub struct ActorBaselineRecipeLimits {
    pub max_catalog_bytes: usize,
    pub max_policy_bytes: usize,
    pub max_profiles: usize,
    pub max_bindings: usize,
    pub max_rows: usize,
    pub max_work: usize,
    pub extension: RecipeExtensionLimits,
}
impl Default for ActorBaselineRecipeLimits {
    fn default() -> Self {
        Self {
            max_catalog_bytes: 8 * 1024 * 1024,
            max_policy_bytes: 2 * 1024 * 1024,
            max_profiles: 4096,
            max_bindings: 4096,
            max_rows: 4096,
            max_work: 2_000_000,
            extension: Default::default(),
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum ActorBaselineRecipeError {
    #[error("actor baseline compiler limit: {0}")]
    Limit(&'static str),
    #[error("invalid actor baseline policy/catalog: {0}")]
    Invalid(&'static str),
    #[error("actor baseline catalog/source binding differs")]
    Binding,
    #[error(transparent)]
    Extension(#[from] RecipeExtensionError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, ActorBaselineRecipeError>;
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineRemainingCoverage {
    pub profile: String,
    pub actor: DeclaredSlot<ActorSlotDefId>,
    pub unconverted_fields: Vec<String>,
    pub unconverted_modifiers: usize,
    pub unconverted_flags: usize,
    pub child_skills: usize,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBaselineRecipeReceipt {
    pub catalog_sha256: String,
    pub policy: OwnedContentDigest,
    pub actors: usize,
    pub scalar_facts: usize,
    pub absent_facts: usize,
    pub curve_outputs: usize,
    pub level_projections: usize,
    pub extension: RecipeExtensionReceipt,
    pub remaining_coverage: Vec<ActorBaselineRemainingCoverage>,
    pub work_used: usize,
}
#[derive(Debug)]
pub struct StagedActorBaselineRecipe {
    pub successor: OwnedRecipeInput,
    pub extension: OwnedRecipeExtension,
    pub receipt: ActorBaselineRecipeReceipt,
}
fn invalid(message: &'static str) -> ActorBaselineRecipeError {
    ActorBaselineRecipeError::Invalid(message)
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("bounded compiler key")
}
fn charge(left: &mut usize, n: usize) -> Result<()> {
    *left = left
        .checked_sub(n)
        .ok_or(ActorBaselineRecipeError::Limit("work"))?;
    Ok(())
}
fn actor_subject(actor: &DeclaredSlot<ActorSlotDefId>) -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor.clone()))
}
fn owner_subject(owner: &SlotOwnerDefId) -> SchemaSubject {
    macro_rules! address { ($($variant:ident),+) => { match owner { $(SlotOwnerDefId::$variant(id)=>SchemaSubject::Definition(id.address()),)+ } }; }
    address!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        PassiveNode,
        UsagePolicy
    )
}
fn checked_stat(
    base: &StagedOwnedRecipe,
    stat: &StatDefId,
    value: &ComputedValueType,
) -> Result<()> {
    let SchemaLookup::Known(schema) = base.schema().definition(stat) else {
        return Err(invalid("target stat must be Known"));
    };
    if schema.targets != [RuleEntityKind::Actor] || &schema.value != value {
        return Err(invalid("stat value type/unit or Actor scope differs"));
    }
    if let ComputedValueType::Quantity { unit } = value
        && !matches!(base.schema().definition(unit), SchemaLookup::Known(_))
    {
        return Err(invalid("unit must be Known"));
    }
    Ok(())
}
fn value(
    profile: &ActorBaselineProfile,
    field: ActorBaselineField,
) -> Result<Option<ActorScalarFact>> {
    use ActorBaselineField::*;
    let number = match field {
        AttackTime => return Ok(profile.attack_time.map(ActorScalarFact::Number)),
        DamageScale => return Ok(profile.damage_scale.map(ActorScalarFact::Number)),
        DamageSpread => return Ok(profile.damage_spread.map(ActorScalarFact::Number)),
        CriticalChance => return Ok(profile.critical_chance.map(ActorScalarFact::Number)),
        AttackRange => return Ok(profile.attack_range.map(ActorScalarFact::Number)),
        BaseDamageIgnoresAttackSpeed => {
            return Ok(profile
                .base_damage_ignores_attack_speed
                .map(ActorScalarFact::Boolean));
        }
        Hostile => return Ok(profile.hostile.map(ActorScalarFact::Boolean)),
        Accuracy => "accuracy",
        Armour => "armour",
        Evasion => "evasion",
        EnergyShield => "energyShield",
        Life => "life",
        BaseMovementSpeed => "baseMovementSpeed",
        FireResistance => "fireResist",
        ColdResistance => "coldResist",
        LightningResistance => "lightningResist",
        ChaosResistance => "chaosResist",
        CompanionFireResistance => "companionFireResist",
        CompanionColdResistance => "companionColdResist",
        CompanionLightningResistance => "companionLightningResist",
        CompanionChaosResistance => "companionChaosResist",
        CompanionReservation => "companionReservation",
        SpectreReservation => "spectreReservation",
    };
    match profile.extra_facts.get(number) {
        Some(v @ ActorScalarFact::Number(_)) => Ok(Some(v.clone())),
        None => Ok(None),
        _ => Err(invalid("selected numeric source fact has another kind")),
    }
}
fn table(
    id: OwnedDefinitionKey,
    first: u32,
    rows: Vec<ParameterValue>,
    value_type: ComputedValueType,
) -> Result<IntegerRuleTable> {
    if first == 0 || rows.is_empty() {
        return Err(invalid("table needs a positive nonempty domain"));
    }
    let maximum = i64::from(first)
        .checked_add(i64::try_from(rows.len()).map_err(|_| invalid("table domain"))? - 1)
        .ok_or(invalid("table domain"))?;
    Ok(IntegerRuleTable {
        id,
        minimum: BoundedInteger::new(first.into()).map_err(|_| invalid("table domain"))?,
        maximum: BoundedInteger::new(maximum).map_err(|_| invalid("table domain"))?,
        rows,
        value_type,
    })
}
fn insert_table(
    tables: &mut BTreeMap<OwnedDefinitionKey, IntegerRuleTable>,
    table: IntegerRuleTable,
) -> Result<()> {
    if let Some(old) = tables.get(&table.id) {
        if old != &table {
            return Err(invalid("shared table ID has different data"));
        }
    } else {
        tables.insert(table.id.clone(), table);
    }
    Ok(())
}
fn checked_actor(base: &StagedOwnedRecipe, actor: &DeclaredSlot<ActorSlotDefId>) -> Result<()> {
    if !matches!(base.schema().slot(actor), SchemaLookup::Known(_)) {
        return Err(invalid("actor slot must be Known with exact declaration"));
    }
    macro_rules! declared { ($($variant:ident),+) => { match &actor.declaration { $(SlotOwnerDefId::$variant(id)=>matches!(base.schema().definition(id),SchemaLookup::Known(s) if s.declarations.actors.members.contains(actor)),)+ } }; }
    if !declared!(
        Class,
        Ascendancy,
        Reward,
        ItemTemplate,
        Modifier,
        Gem,
        Skill,
        PassiveNode,
        UsagePolicy
    ) {
        return Err(invalid("actor is not declared by a Known owner"));
    }
    Ok(())
}
fn clean(s: &str) -> bool {
    !s.is_empty() && s.len() <= 1024 && !s.chars().any(char::is_control)
}
fn validate_catalog<'a>(
    catalog: &'a ActorBaselineCatalog,
    limits: ActorBaselineRecipeLimits,
    left: &mut usize,
) -> Result<BTreeMap<&'a str, &'a ActorBaselineProfile>> {
    if catalog.schema_version != OWNED_ACTOR_BASELINE_VERSION || catalog.profiles.is_empty() {
        return Err(invalid("catalog version or empty profiles"));
    }
    if catalog.profiles.len() > limits.max_profiles {
        return Err(ActorBaselineRecipeError::Limit("profiles"));
    }
    charge(left, catalog.profiles.len())?;
    if !clean(&catalog.source.revision)
        || catalog.source.files.is_empty()
        || catalog.source.files.len() > 64
    {
        return Err(invalid("source membership"));
    }
    let mut pins = BTreeSet::new();
    for pin in &catalog.source.files {
        charge(left, pin.path.len() + pin.sha256.len() + 1)?;
        if !clean(&pin.path)
            || !pins.insert(pin.path.as_str())
            || pin.path.starts_with('/')
            || pin.path.contains(['\\', ':'])
            || pin
                .path
                .split('/')
                .any(|p| p.is_empty() || p == "." || p == "..")
            || pin.sha256.len() != 64
            || !pin
                .sha256
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(invalid("source file pin"));
        }
    }
    for length in [
        catalog.summon_levels.rows.len(),
        catalog.allied_damage.rows.len(),
        catalog.hostile_damage.rows.len(),
    ] {
        charge(left, length)?;
        if length == 0 || length > limits.max_rows {
            return Err(ActorBaselineRecipeError::Limit("table rows"));
        }
    }
    if catalog.summon_levels.first_level == 0
        || catalog.allied_damage.first_level == 0
        || catalog.hostile_damage.first_level == 0
        || catalog.summon_levels.rows.contains(&0)
        || catalog
            .allied_damage
            .rows
            .iter()
            .chain(&catalog.hostile_damage.rows)
            .any(|v| !v.is_finite())
    {
        return Err(invalid("invalid raw level table"));
    }
    let mut profiles = BTreeMap::new();
    for row in &catalog.profiles {
        charge(
            left,
            row.extra_facts.len()
                + row.unconverted_fields.len()
                + row.child_skills.len()
                + row.unconverted_modifiers.as_ref().map_or(0, Vec::len)
                + row.unconverted_flags.as_ref().map_or(0, BTreeMap::len),
        )?;
        charge(
            left,
            row.key.len()
                + row.name.len()
                + row.source_module.len()
                + row
                    .child_skills
                    .iter()
                    .chain(&row.unconverted_fields)
                    .map(String::len)
                    .sum::<usize>()
                + row
                    .extra_facts
                    .iter()
                    .map(|(name, value)| {
                        name.len()
                            + match value {
                                ActorScalarFact::Text(text) => text.len(),
                                _ => 1,
                            }
                    })
                    .sum::<usize>()
                + row
                    .unconverted_modifiers
                    .iter()
                    .flatten()
                    .map(|m| m.name.len() + m.operation.len())
                    .sum::<usize>()
                + row
                    .unconverted_flags
                    .iter()
                    .flat_map(|m| m.keys())
                    .map(String::len)
                    .sum::<usize>(),
        )?;
        if !clean(&row.key)
            || !clean(&row.name)
            || !pins.contains(row.source_module.as_str())
            || profiles.insert(row.key.as_str(), row).is_some()
        {
            return Err(invalid("duplicate, invalid or unpinned profile"));
        }
        if [
            row.attack_time,
            row.damage_scale,
            row.damage_spread,
            row.critical_chance,
            row.attack_range,
        ]
        .into_iter()
        .flatten()
        .any(|v| !v.is_finite())
        {
            return Err(invalid("nonfinite raw actor fact"));
        }
        if row
            .child_skills
            .iter()
            .chain(&row.unconverted_fields)
            .any(|v| !clean(v))
            || row.weapon_family.as_ref().is_some_and(|v| !clean(v))
        {
            return Err(invalid("invalid actor text"));
        }
        let coverage = row.unconverted_fields.iter().collect::<BTreeSet<_>>();
        if coverage.len() != row.unconverted_fields.len() {
            return Err(invalid("duplicate coverage field"));
        }
        for (name, fact) in &row.extra_facts {
            if !clean(name)
                || !coverage.contains(name)
                || match fact {
                    ActorScalarFact::Number(v) => !v.is_finite(),
                    ActorScalarFact::Text(s) => !clean(s),
                    ActorScalarFact::Boolean(_) => false,
                }
            {
                return Err(invalid("invalid or uncovered extra fact"));
            }
        }
        if row
            .unconverted_modifiers
            .iter()
            .flatten()
            .any(|m| !clean(&m.name) || !clean(&m.operation))
            || row
                .unconverted_flags
                .iter()
                .flat_map(|m| m.keys())
                .any(|k| !clean(k))
        {
            return Err(invalid("invalid unconverted coverage"));
        }
    }
    Ok(profiles)
}
type Programs =
    BTreeMap<OwnedDefinitionKey, (SchemaSubject, BTreeMap<OwnedDefinitionKey, RuleProgram>)>;
fn add_program(programs: &mut Programs, owner: SchemaSubject, program: RuleProgram) -> Result<()> {
    let owner_key = match &owner {
        SchemaSubject::Definition(a) => a.key(),
        SchemaSubject::Slot(a) => a.key(),
    }
    .clone();
    let (prior, rows) = programs
        .entry(owner_key)
        .or_insert_with(|| (owner.clone(), BTreeMap::new()));
    if prior != &owner || rows.insert(program.id.clone(), program).is_some() {
        return Err(invalid("duplicate owner/program binding"));
    }
    Ok(())
}
fn own_writer(
    owner: &SchemaSubject,
    entity: RuleEntity,
    stat: &StatDefId,
    outputs: &BTreeSet<(DeclaredSlot<ActorSlotDefId>, StatDefId)>,
    output_stats: &BTreeSet<StatDefId>,
) -> bool {
    if !matches!(entity, RuleEntity::Current | RuleEntity::Actor) {
        return false;
    }
    match owner {
        SchemaSubject::Slot(SlotAddress::Actor(slot)) => {
            outputs.contains(&(slot.clone(), stat.clone()))
        }
        _ => output_stats.contains(stat), // Unknown contextual overlap is not permission to replace.
    }
}
fn writers_clear(
    base: &StagedOwnedRecipe,
    desired: &Programs,
    outputs: &BTreeSet<(DeclaredSlot<ActorSlotDefId>, StatDefId)>,
    left: &mut usize,
) -> Result<()> {
    charge(left, outputs.len())?;
    let output_stats = outputs
        .iter()
        .map(|(_, stat)| stat.clone())
        .collect::<BTreeSet<_>>();
    for owner in &base.rules().input().owners {
        let owner_key = match &owner.owner {
            SchemaSubject::Definition(a) => a.key(),
            SchemaSubject::Slot(a) => a.key(),
        };
        for program in &owner.programs.members {
            charge(left, program.effects.len() + 1)?;
            if desired
                .get(owner_key)
                .is_some_and(|(o, p)| o == &owner.owner && p.get(&program.id) == Some(program))
            {
                continue;
            }
            for effect in &program.effects {
                let collision = match &effect.effect {
                    RuleEffectKind::ProjectActorStat { actor, stat, .. } => {
                        outputs.contains(&(actor.clone(), stat.clone()))
                    }
                    RuleEffectKind::Derive { entity, stat, .. }
                    | RuleEffectKind::Contribute { entity, stat, .. } => {
                        own_writer(&owner.owner, *entity, stat, outputs, &output_stats)
                    }
                    _ => false,
                };
                if collision {
                    return Err(invalid("existing actor output writer conflicts"));
                }
            }
        }
    }
    for receiver in &base.rules().input().receivers.members {
        charge(left, receiver.targets.len() + 1)?;
        if receiver.targets.iter().any(|t|matches!(t,StatReceiverTarget::OwnedSlot{slot} if outputs.contains(&(slot.clone(),receiver.stat.clone())))) { return Err(invalid("existing actor receiver conflicts")); }
    }
    Ok(())
}
/// Lower explicit finite profile bindings to the existing native expression and
/// table contract. All IDs must already exist; this does not refine a schema.
pub fn compile_owned_actor_baselines(
    base: &StagedOwnedRecipe,
    catalog_bytes: &[u8],
    policy: &ActorBaselinePolicy,
    limits: ActorBaselineRecipeLimits,
) -> Result<StagedActorBaselineRecipe> {
    let hard = ActorBaselineRecipeLimits::default();
    for (name, value, max) in [
        (
            "catalog bytes",
            limits.max_catalog_bytes,
            hard.max_catalog_bytes,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
        ("profiles", limits.max_profiles, hard.max_profiles),
        ("bindings", limits.max_bindings, hard.max_bindings),
        ("rows", limits.max_rows, hard.max_rows),
        ("work", limits.max_work, hard.max_work),
    ] {
        if value == 0 || value > max {
            return Err(ActorBaselineRecipeError::Limit(name));
        }
    }
    if catalog_bytes.len() > limits.max_catalog_bytes {
        return Err(ActorBaselineRecipeError::Limit("catalog bytes"));
    }
    let policy_digest = digest_owned(
        "owned-actor-baseline-policy-v1",
        policy,
        limits.max_policy_bytes,
    )?;
    let hash = format!("{:x}", Sha256::digest(catalog_bytes));
    if hash != policy.catalog_sha256 {
        return Err(ActorBaselineRecipeError::Binding);
    }
    let catalog: ActorBaselineCatalog = serde_json::from_slice(catalog_bytes)?;
    if catalog.source != policy.source {
        return Err(ActorBaselineRecipeError::Binding);
    }
    if policy.schema_version != 1 {
        return Err(invalid("policy version"));
    }
    if policy.bindings.is_empty() || policy.bindings.len() > limits.max_bindings {
        return Err(ActorBaselineRecipeError::Limit("bindings"));
    }
    let mut left = limits.max_work;
    let profiles = validate_catalog(&catalog, limits, &mut left)?;
    charge(&mut left, policy.bindings.len())?;
    let mut bindings = policy.bindings.iter().collect::<Vec<_>>();
    bindings.sort_by(|a, b| a.actor.cmp(&b.actor));
    let mut actors = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    let mut programs = Programs::new();
    let mut tables = BTreeMap::new();
    let (mut scalar_facts, mut absent_facts, mut curve_outputs, mut level_projections) =
        (0, 0, 0, 0);
    let mut remaining_coverage = Vec::new();
    for binding in bindings {
        charge(&mut left, binding.fields.len() + binding.curves.len() + 1)?;
        if !actors.insert(binding.actor.clone()) {
            return Err(invalid("duplicate actor binding"));
        }
        checked_actor(base, &binding.actor)?;
        let row = *profiles
            .get(binding.profile.as_str())
            .ok_or(invalid("profile is absent"))?;
        let mut program = RuleProgram {
            id: binding.program.clone(),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![],
            effects: vec![],
        };
        let mut fields = binding.fields.iter().collect::<Vec<_>>();
        fields.sort_by_key(|v| v.field);
        let mut selected = BTreeSet::new();
        for (index, field) in fields.iter().enumerate() {
            if !selected.insert(field.field) {
                return Err(invalid("duplicate scalar selector"));
            }
            let boolean = matches!(
                field.field,
                ActorBaselineField::Hostile | ActorBaselineField::BaseDamageIgnoresAttackSpeed
            );
            if boolean != matches!(field.target, ActorScalarTarget::Boolean { .. }) {
                return Err(invalid("scalar target kind differs"));
            }
            checked_stat(base, field.target.stat(), &field.target.value_type())?;
            if !outputs.insert((binding.actor.clone(), field.target.stat().clone())) {
                return Err(invalid("duplicate actor output stat"));
            }
            let Some(value) = value(row, field.field)? else {
                if field.when_absent == ActorFactAbsence::Reject {
                    return Err(invalid("required source fact absent"));
                }
                absent_facts += 1;
                continue;
            };
            let value = match (value, &field.target) {
                (ActorScalarFact::Number(v), ActorScalarTarget::Quantity { unit, .. }) => {
                    ParameterValue::Quantity(
                        FiniteQuantity::new(v, unit.clone())
                            .map_err(|_| invalid("nonfinite source value"))?,
                    )
                }
                (ActorScalarFact::Boolean(v), ActorScalarTarget::Boolean { .. }) => {
                    ParameterValue::Boolean(v)
                }
                _ => return Err(invalid("source scalar kind differs from target")),
            };
            let id = key(&format!("fact-{index}"));
            program.nodes.push(RuleNode {
                id: id.clone(),
                expression: RuleExpression::Literal { value },
            });
            program.effects.push(RuleEffect {
                id: id.clone(),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: field.target.stat().clone(),
                    value: id,
                },
            });
            scalar_facts += 1;
        }
        let mut curves = binding.curves.iter().collect::<Vec<_>>();
        curves.sort_by(|a, b| a.stat.cmp(&b.stat));
        for (index, curve) in curves.iter().enumerate() {
            checked_stat(base, &curve.actor_level, &ComputedValueType::Integer)?;
            let value_type = ComputedValueType::Quantity {
                unit: curve.unit.clone(),
            };
            checked_stat(base, &curve.stat, &value_type)?;
            if !outputs.insert((binding.actor.clone(), curve.stat.clone())) {
                return Err(invalid("duplicate actor output stat"));
            }
            let source = match curve.curve {
                ActorDamageCurve::Allied => &catalog.allied_damage,
                ActorDamageCurve::Hostile => &catalog.hostile_damage,
            };
            charge(&mut left, source.rows.len() + 4)?;
            let rows = source
                .rows
                .iter()
                .map(|v| {
                    FiniteQuantity::new(*v, curve.unit.clone())
                        .map(ParameterValue::Quantity)
                        .map_err(|_| invalid("nonfinite curve"))
                })
                .collect::<Result<Vec<_>>>()?;
            insert_table(
                &mut tables,
                table(curve.table.clone(), source.first_level, rows, value_type)?,
            )?;
            let level = key(&format!("level-{index}"));
            let result = key(&format!("curve-{index}"));
            program.reads.push(RuleRead {
                id: level.clone(),
                value_type: ComputedValueType::Integer,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Actor,
                    stat: curve.actor_level.clone(),
                },
            });
            program.nodes.push(RuleNode {
                id: level.clone(),
                expression: RuleExpression::Read {
                    input: level.clone(),
                },
            });
            program.nodes.push(RuleNode {
                id: result.clone(),
                expression: RuleExpression::LookupIntegerTable {
                    table: curve.table.clone(),
                    key: level,
                },
            });
            program.effects.push(RuleEffect {
                id: result.clone(),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: curve.stat.clone(),
                    value: result,
                },
            });
            curve_outputs += 1;
        }
        if !program.effects.is_empty() {
            add_program(&mut programs, actor_subject(&binding.actor), program)?;
        }
        if let Some(level) = &binding.summon_level {
            if level.parameter.declaration != binding.actor.declaration {
                return Err(invalid("summon parameter belongs to another declaration"));
            }
            if !matches!(base.schema().slot(&level.parameter),SchemaLookup::Known(p) if matches!(p.value,ValueSchema::Integer(_)))
            {
                return Err(invalid("summon parameter must be Known Integer"));
            }
            checked_stat(base, &level.stat, &ComputedValueType::Integer)?;
            if !outputs.insert((binding.actor.clone(), level.stat.clone())) {
                return Err(invalid("duplicate actor output stat"));
            }
            charge(&mut left, catalog.summon_levels.rows.len() + 4)?;
            let rows = catalog
                .summon_levels
                .rows
                .iter()
                .map(|v| {
                    BoundedInteger::new((*v).into())
                        .map(ParameterValue::Integer)
                        .map_err(|_| invalid("summon level integer"))
                })
                .collect::<Result<Vec<_>>>()?;
            insert_table(
                &mut tables,
                table(
                    level.table.clone(),
                    catalog.summon_levels.first_level,
                    rows,
                    ComputedValueType::Integer,
                )?,
            )?;
            add_program(
                &mut programs,
                owner_subject(&binding.actor.declaration),
                RuleProgram {
                    id: level.program.clone(),
                    context: RuleEntityKind::Actor,
                    reads: vec![RuleRead {
                        id: key("level"),
                        value_type: ComputedValueType::Integer,
                        source: RuleReadSource::Parameter {
                            slot: level.parameter.clone(),
                        },
                    }],
                    nodes: vec![
                        RuleNode {
                            id: key("level"),
                            expression: RuleExpression::Read {
                                input: key("level"),
                            },
                        },
                        RuleNode {
                            id: key("actor-level"),
                            expression: RuleExpression::LookupIntegerTable {
                                table: level.table.clone(),
                                key: key("level"),
                            },
                        },
                    ],
                    effects: vec![RuleEffect {
                        id: key("actor-level"),
                        when: None,
                        effect: RuleEffectKind::ProjectActorStat {
                            actor: binding.actor.clone(),
                            stat: level.stat.clone(),
                            value: key("actor-level"),
                        },
                    }],
                },
            )?;
            level_projections += 1;
        }
        remaining_coverage.push(ActorBaselineRemainingCoverage {
            profile: binding.profile.clone(),
            actor: binding.actor.clone(),
            unconverted_fields: row.unconverted_fields.clone(),
            unconverted_modifiers: row.unconverted_modifiers.as_ref().map_or(0, Vec::len),
            unconverted_flags: row.unconverted_flags.as_ref().map_or(0, BTreeMap::len),
            child_skills: row.child_skills.len(),
        });
    }
    writers_clear(base, &programs, &outputs, &mut left)?;
    let mut owners = Vec::new();
    let indexed: BTreeMap<_, _> = base
        .rules()
        .input()
        .owners
        .iter()
        .map(|o| {
            let id = match &o.owner {
                SchemaSubject::Definition(a) => a.key(),
                SchemaSubject::Slot(a) => a.key(),
            };
            (id, o)
        })
        .collect();
    charge(&mut left, indexed.len())?;
    for (id, (owner, rows)) in programs {
        let prior = indexed
            .get(&id)
            .filter(|p| p.owner == owner)
            .ok_or(invalid("baseline requires an existing covered rule owner"))?;
        owners.push(DefinitionRules {
            owner,
            programs: DeclaredSet {
                members: rows.into_values().collect(),
                closure: prior.programs.closure.clone(),
            },
        });
    }
    let extension = OwnedRecipeExtension {
        schema_version: 1,
        version: policy.version.clone(),
        schema: vec![],
        operations_version: None,
        tables: tables.into_values().collect(),
        owners,
        receivers: vec![],
    };
    let extended = extend_owned_recipe(base, &extension, limits.extension)?;
    if extended.refinement.is_some() {
        return Err(invalid("actor baseline unexpectedly refined schema"));
    }
    Ok(StagedActorBaselineRecipe {
        successor: extended.successor,
        extension,
        receipt: ActorBaselineRecipeReceipt {
            catalog_sha256: hash,
            policy: policy_digest,
            actors: actors.len(),
            scalar_facts,
            absent_facts,
            curve_outputs,
            level_projections,
            extension: extended.receipt,
            remaining_coverage,
            work_used: limits.max_work - left,
        },
    })
}
