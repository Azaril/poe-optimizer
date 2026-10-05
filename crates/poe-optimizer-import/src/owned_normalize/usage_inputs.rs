//! Finite source-to-usage projection. This does not by itself prove a usage or
//! physical Gem inventory; the private attachment token participates in V2 proof.
use super::*;
use crate::owned_value::WhitespacePolicy;
mod occurrences;
pub use occurrences::{OccurrenceUsagePolicy, OccurrenceUsageRule, OccurrenceUsageTarget};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInputPolicy {
    PobPhysicalPrimarySkillV1 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<PrimarySkillUsageInput>,
    },
    PobPhysicalPrimarySkillV2 {
        definitions: DataIdentity,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<PrimarySkillUsageInput>,
        numeric_gems: Vec<PrimarySkillNumericUsageInput>,
    },
    /// Preserves the historical physical projections and adds independently
    /// proved occurrence targets. It does not certify any usage inventory.
    PobOccurrenceUsageV3 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        gems: Vec<PrimarySkillUsageInput>,
        numeric_gems: Vec<PrimarySkillNumericUsageInput>,
        occurrences: Vec<OccurrenceUsageRule>,
    },
}
impl UsageInputPolicy {
    /// Only these historical Boolean rows can participate in physical inventory
    /// proof. Numeric source transport never gains that authority.
    pub(super) fn boolean_rows(&self) -> &[PrimarySkillUsageInput] {
        match self {
            Self::PobPhysicalPrimarySkillV1 { gems, .. }
            | Self::PobPhysicalPrimarySkillV2 { gems, .. }
            | Self::PobOccurrenceUsageV3 { gems, .. } => gems,
        }
    }
}

/// A catalog-bound physical row and its exact primary supply. V1 transports
/// required Boolean parameters only; it invents neither source defaults nor
/// count/population semantics. `attributes` admits a source frame, not values.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimarySkillUsageInput {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub primary: SkillDefId,
    pub supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub grant: DeclaredSlot<GrantSlotDefId>,
    pub policy: UsagePolicyDefId,
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub parameters: Vec<GemParameterInput>,
}

/// A primary occurrence's numeric usage facts. These finite source frames do
/// not certify minion choices, extra effects, physical inputs or usage inventory.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PrimarySkillNumericUsageInput {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
    pub primary: SkillDefId,
    pub supply: DeclaredSlot<SkillGrantSlotDefId>,
    pub grant: DeclaredSlot<GrantSlotDefId>,
    pub policy: UsagePolicyDefId,
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub group_attributes: Vec<String>,
    pub group_guards: Vec<GemInputGuard>,
    pub parameters: Vec<UsageParameterInput>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageParameterInput {
    pub slot: DeclaredSlot<ParameterSlotDefId>,
    pub source: UsageValueSource,
}
/// Override precedence depends on source presence, never truthiness or successful
/// decoding. A present zero wins; malformed overrides remain unresolved.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageValueSource {
    Occurrence {
        value: ValueRecipeInput,
    },
    ContainingGroupOverride {
        group: Box<ValueRecipeInput>,
        occurrence: ValueRecipeInput,
        fallback_admission: UsageFallbackAdmission,
    },
}
/// Requested transport is distinct from source-consumer correspondence. The
/// stricter mode refuses ambiguous effect matches when the parent has no override.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageFallbackAdmission {
    RequestedOccurrence,
    UniqueReviewedPrimary {
        companions: Vec<UsageGroupCompanion>,
    },
}
/// Catalog-pinned, reviewed negative effect evidence. Publication must authenticate
/// that this exact row's primary and all declared/constructed/resolved additional
/// effects exclude the target. Role lookup alone proves only primary exclusion.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsageGroupCompanion {
    pub gem: GemDefId,
    pub game_id: String,
    pub variant_id: String,
    pub skill_id: String,
    pub name_spec: String,
}

struct UsageRow<'p> {
    gem: &'p GemDefId,
    game_id: &'p String,
    variant_id: &'p String,
    skill_id: &'p String,
    name_spec: &'p String,
    primary: &'p SkillDefId,
    supply: &'p DeclaredSlot<SkillGrantSlotDefId>,
    grant: &'p DeclaredSlot<GrantSlotDefId>,
    policy: &'p UsagePolicyDefId,
    attributes: &'p [String],
    guards: &'p [GemInputGuard],
    legacy: Option<&'p PrimarySkillUsageInput>,
    numeric: Option<&'p PrimarySkillNumericUsageInput>,
}
impl<'p> From<&'p PrimarySkillUsageInput> for UsageRow<'p> {
    fn from(row: &'p PrimarySkillUsageInput) -> Self {
        Self {
            gem: &row.gem,
            game_id: &row.game_id,
            variant_id: &row.variant_id,
            skill_id: &row.skill_id,
            name_spec: &row.name_spec,
            primary: &row.primary,
            supply: &row.supply,
            grant: &row.grant,
            policy: &row.policy,
            attributes: &row.attributes,
            guards: &row.guards,
            legacy: Some(row),
            numeric: None,
        }
    }
}
impl<'p> From<&'p PrimarySkillNumericUsageInput> for UsageRow<'p> {
    fn from(row: &'p PrimarySkillNumericUsageInput) -> Self {
        Self {
            gem: &row.gem,
            game_id: &row.game_id,
            variant_id: &row.variant_id,
            skill_id: &row.skill_id,
            name_spec: &row.name_spec,
            primary: &row.primary,
            supply: &row.supply,
            grant: &row.grant,
            policy: &row.policy,
            attributes: &row.attributes,
            guards: &row.guards,
            legacy: None,
            numeric: Some(row),
        }
    }
}
impl UsageRow<'_> {
    fn parameter_count(&self) -> usize {
        self.legacy.map_or_else(
            || self.numeric.unwrap().parameters.len(),
            |row| row.parameters.len(),
        )
    }
}

pub(super) struct CompiledUsageInputs<'p> {
    rules: BTreeMap<&'p GemDefId, BoundUsage<'p>>,
    occurrences: Option<occurrences::CompiledOccurrences<'p>>,
    pub work: usize,
}
struct BoundUsage<'p> {
    input: UsageRow<'p>,
    attributes: Vec<&'p str>,
    group_attributes: Vec<&'p str>,
    parameters: Vec<BoundParameter<'p>>,
}
struct BoundParameter<'p> {
    slot: DeclaredSlot<ParameterSlotDefId>,
    occurrence: ValueRecipe,
    group: Option<ValueRecipe>,
    schema: ValueSchema,
    fallback: BoundFallback<'p>,
}
enum BoundFallback<'p> {
    RequestedOccurrence,
    UniqueReviewedPrimary(BTreeMap<(&'p str, &'p str), &'p UsageGroupCompanion>),
}

/// Acyclic commitment to the complete existing source-to-usage projection.
pub fn usage_inputs_identity(
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-usage-inputs-v1",
        &policy.usage_inputs,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

/// Constructible only after attaching the real source-linked preference. Its
/// presence proves neither complete usage inventory nor numerical consumers.
pub(super) struct AttachedPrimaryUsage<'p> {
    input: &'p PrimarySkillUsageInput,
    source: SourceOccurrenceId,
}
pub(super) struct UsageInputContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub skill: SkillUseId,
    pub preset: &'a mut SkillPresetDraft,
    pub inventory_proof: bool,
}
impl AttachedPrimaryUsage<'_> {
    pub(super) fn proves(
        &self,
        source: SourceOccurrenceId,
        gem: &GemDefId,
        policy: &UsagePolicyDefId,
    ) -> bool {
        self.source == source && self.input.gem == *gem && self.input.policy == *policy
    }
}

fn invalid<T>(reason: &'static str) -> Result<T> {
    Err(NormalizationError::Policy(reason))
}
fn charge(work: &mut usize, amount: usize, limits: NormalizationLimits) -> Result<()> {
    *work = work
        .checked_add(amount)
        .filter(|value| *value <= limits.max_work)
        .ok_or(NormalizationError::Limit("usage input work"))?;
    Ok(())
}

/// Only checked, inherited policies may call this while constructing a new
/// release. The caller validates the full resulting policy again afterward.
pub(crate) fn rebind(
    policy: &mut NormalizationPolicy,
    definitions: &DataIdentity,
    roles: &OwnedSkillRoleIndex,
    limits: NormalizationLimits,
) -> Result<()> {
    if policy.usage_inputs.is_none() {
        return Ok(());
    }
    let scalar = gem_inventory_scalar_inputs_identity(policy, limits)?;
    let (bound_definitions, bound_roles, scalar_inputs) =
        match policy.usage_inputs.as_mut().unwrap() {
            UsageInputPolicy::PobPhysicalPrimarySkillV1 {
                definitions,
                roles,
                scalar_inputs,
                ..
            }
            | UsageInputPolicy::PobPhysicalPrimarySkillV2 {
                definitions,
                roles,
                scalar_inputs,
                ..
            }
            | UsageInputPolicy::PobOccurrenceUsageV3 {
                definitions,
                roles,
                scalar_inputs,
                ..
            } => (definitions, roles, scalar_inputs),
        };
    *bound_definitions = definitions.clone();
    *bound_roles = *roles.identity();
    *scalar_inputs = scalar;
    Ok(())
}

pub(super) fn compile<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    definitions: &I,
    roles: &OwnedSkillRoleIndex,
    mappings: &OwnedMappingIndex,
    limits: NormalizationLimits,
) -> Result<Option<CompiledUsageInputs<'p>>> {
    let Some(usage_policy) = &policy.usage_inputs else {
        return Ok(None);
    };
    let (identity, role_identity, catalog, scalar_inputs, gems, numeric_gems) = match usage_policy {
        UsageInputPolicy::PobPhysicalPrimarySkillV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
        } => (definitions, roles, catalog, scalar_inputs, gems, &[][..]),
        UsageInputPolicy::PobPhysicalPrimarySkillV2 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
            numeric_gems,
        }
        | UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
            numeric_gems,
            ..
        } => (
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
            numeric_gems.as_slice(),
        ),
    };
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *identity
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
    {
        return Err(NormalizationError::Binding);
    }
    usage_inputs_identity(policy, limits)?;
    let bytes = serde_json::to_vec(&policy.usage_inputs)
        .map_err(|_| NormalizationError::Policy("usage input encoding"))?;
    if bytes.len() > limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES) {
        return Err(NormalizationError::Limit("usage input bytes"));
    }
    let mut compiled = CompiledUsageInputs {
        rules: BTreeMap::new(),
        occurrences: None,
        work: 0,
    };
    charge(&mut compiled.work, bytes.len(), limits)?;
    if gems.len().saturating_add(numeric_gems.len()) > 4096 {
        return Err(NormalizationError::Limit("usage input rows"));
    }
    let mut selectors = BTreeSet::new();
    for input in gems
        .iter()
        .map(UsageRow::from)
        .chain(numeric_gems.iter().map(UsageRow::from))
    {
        if input.attributes.len() > 64
            || input.guards.len() > 64
            || input.parameter_count() == 0
            || input.parameter_count() > 64
            || compiled.rules.contains_key(input.gem)
            || !selectors.insert((input.game_id, input.variant_id))
        {
            return invalid("duplicate or oversized usage input row");
        }
        if [
            &input.game_id,
            &input.variant_id,
            &input.skill_id,
            &input.name_spec,
        ]
        .iter()
        .any(|value| value.is_empty() || value.len() > limits.mapping.max_string_bytes)
        {
            return invalid("usage input source identity");
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(input.game_id.clone()),
            variant_id: SourceComponent::Text(input.variant_id.clone()),
        });
        if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
            basis: MappingBasis::Exact,
        }) if gem == input.gem)
        {
            return invalid("usage input exact source mapping");
        }
        let Some(role) = roles.role(input.gem) else {
            return invalid("usage input physical role");
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            || role.primary != OwnedPrimarySkill::Known(input.primary.clone())
        {
            return invalid("usage input physical primary role");
        }
        let SchemaLookup::Known(gem) = definitions.definition(input.gem) else {
            return invalid("usage input gem schema");
        };
        charge(
            &mut compiled.work,
            gem.roles.len()
                + gem.skills.members.len()
                + gem.declarations.skill_grants.members.len()
                + gem.declarations.grants.members.len(),
            limits,
        )?;
        if !gem.roles.contains(&AuthoredGemRole::SkillUse)
            || !gem.skills.members.contains(input.primary)
            || !matches!(
                definitions.definition(input.primary),
                SchemaLookup::Known(_)
            )
            || input.supply.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || input.grant.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || !gem.declarations.skill_grants.members.contains(input.supply)
            || !gem.declarations.grants.members.contains(input.grant)
        {
            return invalid("usage input declared primary supply");
        }
        let SchemaLookup::Known(supply) = definitions.slot(input.supply) else {
            return invalid("usage input supply schema");
        };
        let SchemaLookup::Known(grant) = definitions.slot(input.grant) else {
            return invalid("usage input grant schema");
        };
        charge(&mut compiled.work, grant.provider_roles.len(), limits)?;
        if &supply.skill != input.primary
            || grant.target != GrantTarget::Skill(input.supply.clone())
            || !grant.provider_roles.contains(&ProviderRole::SkillUse)
        {
            return invalid("usage input primary grant target");
        }
        let SchemaLookup::Known(usage) = definitions.definition(input.policy) else {
            return invalid("usage input policy schema");
        };
        charge(
            &mut compiled.work,
            usage.targets.len() + usage.declarations.parameters.members.len(),
            limits,
        )?;
        if !usage.targets.contains(&UsageTargetKind::Skill)
            || !usage.declarations.parameters.is_complete()
            || usage.declarations.parameters.members.len() != input.parameter_count()
        {
            return invalid("usage input policy target or parameters");
        }
        let declared: BTreeSet<_> = usage
            .declarations
            .parameters
            .members
            .iter()
            .cloned()
            .collect();
        let attributes: BTreeSet<_> = input.attributes.iter().map(String::as_str).collect();
        if attributes.len() != input.attributes.len()
            || attributes
                .iter()
                .any(|value| value.is_empty() || value.len() > 128)
            || ["gemId", "variantId", "skillId", "nameSpec"]
                .iter()
                .any(|value| !attributes.contains(value))
        {
            return invalid("usage input attribute frame");
        }
        validate_guards(input.guards, &attributes, limits)?;
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for parameter in input.legacy.into_iter().flat_map(|row| &row.parameters) {
            if parameter.slot.declaration != SlotOwnerDefId::UsagePolicy(input.policy.clone())
                || !declared.contains(&parameter.slot)
                || !slots.insert(parameter.slot.clone())
            {
                return invalid("usage input parameter owner");
            }
            let SchemaLookup::Known(schema) = definitions.slot(&parameter.slot) else {
                return invalid("usage input parameter schema");
            };
            charge(&mut compiled.work, schema.sites.len(), limits)?;
            if schema.value != ValueSchema::Boolean
                || schema.presence != SlotPresence::RequiredOnce
                || !schema.sites.contains(&ParameterSite::UsagePolicyParameter)
                || parameter.value.codec.namespace != *definitions.namespace()
                || !matches!(parameter.value.codec.codec, ValueCodecKind::Boolean { .. })
                || !matches!(parameter.value.missing, MissingValuePolicy::Pending)
                || !parameter.value.numeric_aliases.is_empty()
                || parameter
                    .value
                    .tiers
                    .iter()
                    .flat_map(|tier| &tier.selectors)
                    .any(|selector| {
                        selector.lane != ValueLane::Attribute
                            || !attributes.contains(selector.name.as_str())
                    })
            {
                return invalid("usage input Boolean source recipe");
            }
            parameters.push(BoundParameter {
                slot: parameter.slot.clone(),
                occurrence: ValueRecipe::new(parameter.value.clone(), limits.value)?,
                group: None,
                schema: schema.value.clone(),
                fallback: BoundFallback::RequestedOccurrence,
            });
        }
        let mut group_attributes = Vec::new();
        if let Some(numeric) = input.numeric {
            if numeric.group_attributes.len() > 64 || numeric.group_guards.len() > 64 {
                return invalid("oversized numeric usage group frame");
            }
            let group_names: BTreeSet<_> = numeric
                .group_attributes
                .iter()
                .map(String::as_str)
                .collect();
            if group_names.len() != numeric.group_attributes.len()
                || group_names
                    .iter()
                    .any(|name| name.is_empty() || name.len() > 128)
            {
                return invalid("numeric usage group frame");
            }
            validate_guards(&numeric.group_guards, &group_names, limits)?;
            group_attributes = numeric
                .group_attributes
                .iter()
                .map(String::as_str)
                .collect();
            for parameter in &numeric.parameters {
                if parameter.slot.declaration != SlotOwnerDefId::UsagePolicy(input.policy.clone())
                    || !declared.contains(&parameter.slot)
                    || !slots.insert(parameter.slot.clone())
                {
                    return invalid("usage input parameter owner");
                }
                let SchemaLookup::Known(schema) = definitions.slot(&parameter.slot) else {
                    return invalid("usage input parameter schema");
                };
                charge(&mut compiled.work, schema.sites.len(), limits)?;
                if schema.presence != SlotPresence::RequiredOnce
                    || schema.sites != [ParameterSite::UsagePolicyParameter]
                {
                    return invalid("numeric usage parameter site or presence");
                }
                let (occurrence, group, fallback) = match &parameter.source {
                    UsageValueSource::Occurrence { value } => {
                        (value, None, BoundFallback::RequestedOccurrence)
                    }
                    UsageValueSource::ContainingGroupOverride {
                        group,
                        occurrence,
                        fallback_admission,
                    } => {
                        let fallback = compile_fallback(
                            fallback_admission,
                            input.gem,
                            input.primary,
                            roles,
                            &mut compiled.work,
                            limits,
                        )?;
                        (occurrence, Some(group), fallback)
                    }
                };
                let occurrence = numeric_recipe(
                    occurrence,
                    &schema.value,
                    &attributes,
                    definitions,
                    limits,
                    false,
                )?;
                let group = group
                    .map(|recipe| {
                        numeric_recipe(
                            recipe,
                            &schema.value,
                            &group_names,
                            definitions,
                            limits,
                            false,
                        )
                    })
                    .transpose()?;
                parameters.push(BoundParameter {
                    slot: parameter.slot.clone(),
                    occurrence,
                    group,
                    schema: schema.value.clone(),
                    fallback,
                });
            }
        }
        if slots != declared {
            return invalid("usage input parameter coverage");
        }
        compiled.rules.insert(
            input.gem,
            BoundUsage {
                attributes: input.attributes.iter().map(String::as_str).collect(),
                input,
                group_attributes,
                parameters,
            },
        );
    }
    if let UsageInputPolicy::PobOccurrenceUsageV3 {
        source,
        occurrences,
        ..
    } = usage_policy
    {
        if source != &roles.input().compilation.source
            || roles.input().mapping != *mappings.identity()
            || !provenance_is_subset(source, &mappings.input().source)
        {
            return Err(NormalizationError::Binding);
        }
        compiled.occurrences = Some(occurrences::compile(
            occurrences,
            definitions,
            roles,
            mappings,
            &mut compiled.work,
            limits,
        )?);
    }
    Ok(Some(compiled))
}

fn validate_guards(
    guards: &[GemInputGuard],
    attributes: &BTreeSet<&str>,
    limits: NormalizationLimits,
) -> Result<()> {
    let mut names = BTreeSet::new();
    for guard in guards {
        if !attributes.contains(guard.attribute.as_str()) || !names.insert(&guard.attribute)
            || guard.allowed.is_empty() || guard.allowed.len() > 64
            || guard.allowed.iter().collect::<BTreeSet<_>>().len() != guard.allowed.len()
            || guard.allowed.iter().any(|value| matches!(value, SourceComponent::Text(text) if text.len() > limits.mapping.max_string_bytes))
        { return invalid("usage input guards"); }
    }
    Ok(())
}

fn numeric_recipe<I: DefinitionSchemaIndex>(
    input: &ValueRecipeInput,
    schema: &ValueSchema,
    attributes: &BTreeSet<&str>,
    definitions: &I,
    limits: NormalizationLimits,
    aliases: bool,
) -> Result<ValueRecipe> {
    let matching = match (&input.codec.codec, schema) {
        (ValueCodecKind::Integer { .. }, ValueSchema::Integer(_)) => true,
        (ValueCodecKind::Boolean { .. }, ValueSchema::Boolean) if aliases => true,
        (ValueCodecKind::Quantity { unit, .. }, ValueSchema::Quantity(range)) => {
            unit == range.minimum.unit()
                && unit == range.maximum.unit()
                && matches!(definitions.definition(unit), SchemaLookup::Known(_))
        }
        _ => false,
    };
    if !matching
        || input.codec.namespace != *definitions.namespace()
        || input.codec.whitespace != WhitespacePolicy::Exact
        || !matches!(input.missing, MissingValuePolicy::Pending)
        || (!aliases && !input.numeric_aliases.is_empty())
        || input.tiers.len() != 1
        || input.tiers[0].duplicates != DuplicatePolicy::Reject
        || input.tiers[0].selectors.len() != 1
        || input.tiers[0].selectors[0].lane != ValueLane::Attribute
        || !attributes.contains(input.tiers[0].selectors[0].name.as_str())
    {
        return invalid("numeric usage source recipe");
    }
    Ok(ValueRecipe::new(input.clone(), limits.value)?)
}

fn compile_fallback<'p>(
    admission: &'p UsageFallbackAdmission,
    gem: &GemDefId,
    primary: &SkillDefId,
    roles: &OwnedSkillRoleIndex,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<BoundFallback<'p>> {
    let UsageFallbackAdmission::UniqueReviewedPrimary { companions } = admission else {
        return Ok(BoundFallback::RequestedOccurrence);
    };
    if companions.len() > 64 {
        return invalid("numeric usage companion count");
    }
    charge(work, companions.len(), limits)?;
    let mut known = BTreeMap::new();
    let mut ids = BTreeSet::new();
    for companion in companions {
        if [
            &companion.game_id,
            &companion.variant_id,
            &companion.skill_id,
            &companion.name_spec,
        ]
        .iter()
        .any(|value| value.is_empty() || value.len() > limits.mapping.max_string_bytes)
            || companion.gem == *gem
            || !ids.insert(&companion.gem)
        {
            return invalid("numeric usage companion identity");
        }
        let selector = ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text(companion.game_id.clone()),
            variant_id: SourceComponent::Text(companion.variant_id.clone()),
        });
        if !matches!(roles.lookup(&selector), Some(MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)), basis: MappingBasis::Exact,
        }) if gem == &companion.gem)
        {
            return invalid("numeric usage companion mapping");
        }
        let Some(role) = roles.role(&companion.gem) else {
            return invalid("numeric usage companion role");
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || !matches!(role.role, OwnedGemRole::Known(_))
            || !matches!(&role.primary, OwnedPrimarySkill::Known(value) if value != primary)
            || known
                .insert(
                    (companion.game_id.as_str(), companion.variant_id.as_str()),
                    companion,
                )
                .is_some()
        {
            return invalid("numeric usage companion primary");
        }
    }
    Ok(BoundFallback::UniqueReviewedPrimary(known))
}

fn fallback_admitted(
    b: &mut Builder<'_, '_>,
    fallback: &BoundFallback<'_>,
    row: &SourceEvidenceRow<'_>,
    group: &SourceEvidenceRow<'_>,
) -> Result<bool> {
    let BoundFallback::UniqueReviewedPrimary(companions) = fallback else {
        return Ok(true);
    };
    // The shared complete census already establishes every child as a plain Gem.
    // Inspect even disabled siblings: source effect matching does not skip them.
    b.charge(group.children().len())?;
    let evidence = b.evidence;
    for id in group.children() {
        if *id == row.occurrence().id() {
            continue;
        }
        let sibling = &evidence.rows()[id.ordinal() as usize];
        source_shape::charge_frame(b, sibling, &["gemId", "variantId", "skillId", "nameSpec"])?;
        if sibling.occurrence().name() != "Gem"
            || !source_shape::plain_row(sibling, skill_source_census::GEM_ATTRIBUTES, true)
        {
            return Ok(false);
        }
        let (Some(game), Some(variant)) = (
            source_shape::value(sibling, "gemId"),
            source_shape::value(sibling, "variantId"),
        ) else {
            return Ok(false);
        };
        let Some(companion) = companions.get(&(game, variant)) else {
            return Ok(false);
        };
        if source_shape::value(sibling, "skillId") != Some(companion.skill_id.as_str())
            || source_shape::value(sibling, "nameSpec") != Some(companion.name_spec.as_str())
        {
            return Ok(false);
        }
    }
    Ok(true)
}

// Both numeric source locations are already structurally proved. Selecting an
// override by attribute presence prevents malformed values or zero falling back.
fn selected_recipe<'a, 's>(
    b: &mut Builder<'_, '_>,
    parameter: &'a BoundParameter<'_>,
    row: &'a SourceEvidenceRow<'s>,
    group: &'a SourceEvidenceRow<'s>,
) -> Result<Option<(&'a SourceEvidenceRow<'s>, &'a ValueRecipe)>> {
    if let Some(recipe) = &parameter.group {
        let name = &recipe.input().tiers[0].selectors[0].name;
        b.charge(group.attributes().len())?;
        if group.attribute(name).is_some() {
            return Ok(Some((group, recipe)));
        }
    }
    Ok(fallback_admitted(b, &parameter.fallback, row, group)?
        .then_some((row, &parameter.occurrence)))
}

// Shared decoding preserves source-presence precedence and the legacy issue
// allocation order. Only the physical adapter may turn `converted` into its
// private inventory-attachment capability.
fn decode_parameters(
    b: &mut Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    group: &SourceEvidenceRow<'_>,
    inputs: &[BoundParameter<'_>],
) -> Result<(DraftList<ParameterDraft>, bool)> {
    let source = row.occurrence().id();
    let mut parameters = Vec::new();
    let mut converted = true;
    for parameter in inputs {
        b.charge(1)?;
        let Some((value_row, recipe)) = selected_recipe(b, parameter, row, group)? else {
            converted = false;
            continue;
        };
        for selector in recipe.input().tiers.iter().flat_map(|tier| &tier.selectors) {
            b.charge(value_row.attributes().len())?;
            if let Some(value) = value_row.attribute(&selector.name) {
                b.charge(value.raw().len())?;
            }
        }
        match b.scalar_value(value_row, recipe) {
            Ok(ScalarValue::Selected(value))
                if gem_inputs::value_valid(&value, &parameter.schema) =>
            {
                parameters.push(
                    ParameterAssignment {
                        slot: parameter.slot.clone(),
                        value,
                    }
                    .into(),
                );
            }
            Ok(_) | Err(NormalizationError::Value(ValuePolicyError::MultipleValues { .. })) => {
                converted = false
            }
            Err(error) => return Err(error),
        }
    }
    let parameters = if converted {
        complete(parameters)
    } else {
        b.closure(source, "usage-parameters-not-converted", parameters)?
    };
    Ok((parameters, converted))
}

impl<'p> CompiledUsageInputs<'p> {
    pub(super) fn attach(
        &self,
        b: &mut Builder<'_, '_>,
        context: UsageInputContext<'_, '_>,
    ) -> Result<Option<AttachedPrimaryUsage<'p>>> {
        let UsageInputContext {
            row,
            group,
            gem,
            skill,
            preset,
            inventory_proof,
        } = context;
        b.charge(1)?;
        let DraftField::Known { value: gem } = gem else {
            return Ok(None);
        };
        let Some(bound) = self.rules.get(gem) else {
            return Ok(None);
        };
        let source = row.occurrence().id();
        // A participating row establishes an unresolved preset inventory even
        // when its finite value proof fails. Never confuse no record with none.
        if preset.usage_preferences.is_none() {
            preset.usage_preferences =
                Some(b.closure(source, "usage-preferences-not-converted", vec![])?);
        }
        source_shape::charge_row(b, row)?;
        if row.occurrence().name() != "Gem"
            || group.occurrence().name() != "Skill"
            || row.occurrence().parent() != Some(group.occurrence().id())
            || !source_shape::plain_row(row, &bound.attributes, true)
            || !b.gem_guards_match(row, bound.input.guards)?
        {
            return Ok(None);
        }
        for (name, expected) in [
            ("gemId", &bound.input.game_id),
            ("variantId", &bound.input.variant_id),
            ("skillId", &bound.input.skill_id),
            ("nameSpec", &bound.input.name_spec),
        ] {
            if source_shape::value(row, name) != Some(expected.as_str()) {
                return Ok(None);
            }
        }
        if let Some(numeric) = bound.input.numeric {
            source_shape::charge_row(b, group)?;
            if !source_shape::plain_row(group, &bound.group_attributes, false)
                || !source_shape::container_text(group)
                || !b.gem_guards_match(group, &numeric.group_guards)?
            {
                return Ok(None);
            }
            let Some(sets) = skill_source_census::sets(b)? else {
                return Ok(None);
            };
            let Some(set) = b.ancestor(source, "SkillSet")? else {
                return Ok(None);
            };
            b.charge(sets.len())?;
            if !sets.contains(&set)
                || group.occurrence().parent() != Some(set)
                || !exact_preset(b, preset, skill, set)?
            {
                return Ok(None);
            }
        }
        let (parameters, converted) = decode_parameters(b, row, group, &bound.parameters)?;
        preset
            .usage_preferences
            .as_mut()
            .expect("initialized above")
            .members
            .push(UsagePolicyDraft {
                policy: bound.input.policy.clone().into(),
                target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(skill),
                        grant_path: vec![],
                    },
                    slot: bound.input.supply.clone(),
                })))
                .into(),
                parameters,
            });
        // The preset already has its source-set origin. This exact Gem row also
        // contributed its preference, independently of the Gem/Skill links.
        b.link(source, OwnedOriginTarget::SkillPreset(preset.id))?;
        if bound.input.numeric.is_some() {
            // Parent overrides and frame guards are independent contributing
            // evidence. Preserve their exact origin without duplicate links.
            let parent = group.occurrence().id();
            let target = OwnedOriginTarget::SkillPreset(preset.id);
            b.charge(b.origins[parent.ordinal() as usize].links.len())?;
            if !b.origins[parent.ordinal() as usize].links.contains(&target) {
                b.link(parent, target)?;
            }
        }
        if !inventory_proof || !converted || bound.input.legacy.is_none() {
            return Ok(None);
        }
        b.charge(preset.skills.members.len())?;
        if !preset.skills.members.contains(&skill)
            || !matches!(&preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Pending { code, .. } if code.as_str() == "usage-preferences-not-converted")
        {
            return Ok(None);
        }
        let Some(set) = b.ancestor(source, "SkillSet")? else {
            return Ok(None);
        };
        b.charge(b.origins[set.ordinal() as usize].links.len())?;
        if !b.origins[set.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::SkillPreset(preset.id))
        {
            return Ok(None);
        }
        Ok(Some(AttachedPrimaryUsage {
            input: bound
                .input
                .legacy
                .expect("numeric rows cannot certify physical inventory"),
            source,
        }))
    }
}

fn exact_preset(
    b: &mut Builder<'_, '_>,
    preset: &SkillPresetDraft,
    skill: SkillUseId,
    set: SourceOccurrenceId,
) -> Result<bool> {
    b.charge(preset.skills.members.len())?;
    b.charge(b.origins[set.ordinal() as usize].links.len())?;
    Ok(preset.skills.members.contains(&skill)
        && b.origins[set.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::SkillPreset(preset.id)))
}

impl CompiledUsageInputs<'_> {
    pub(super) fn materialize_occurrences(
        &self,
        b: &mut Builder<'_, '_>,
        draft: &mut DraftSessionInput,
        context: generated_skill_sources::Context<'_>,
    ) -> Result<()> {
        if let Some(compiled) = &self.occurrences {
            occurrences::materialize(b, draft, compiled, context)?;
        }
        Ok(())
    }
}
