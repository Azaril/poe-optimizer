//! Finite source-to-usage projection. This does not by itself prove a usage or
//! physical Gem inventory; a private per-policy token supports its independent proof.
use super::*;
use crate::owned_value::WhitespacePolicy;
mod configuration;
mod inventory;
mod occurrences;
pub use crate::owned_configuration_numeric::ConfigurationNumericSelection;
pub use configuration::{ConfigurationUsageIntentInput, ConfigurationUsageParameterInput};
pub(super) use configuration::{
    ProvenConfigurationUsage,
    check_projection_destinations as check_configuration_projection_destinations,
    checked_receipts as checked_configuration_receipts,
    control_matches as configuration_control_matches, materialize as materialize_configuration,
};
pub(super) use inventory::{
    AuthoredInputProof, InventoryReceipts, record_generated_input_disposition,
    record_input_disposition, record_support_input_disposition,
};
pub use inventory::{
    UsageInventoryDestination, UsageInventoryField, UsageInventoryFieldSelector,
    UsageInventoryPolicy, UsageInventoryRow, UsageInventorySource,
};
pub(super) use occurrences::MaterializedUsage;
pub use occurrences::{OccurrenceUsagePolicy, OccurrenceUsageRule, OccurrenceUsageTarget};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInputPolicy {
    /// Exact physical, authored and generated occurrence projections share typed
    /// policy selections. Projection alone never certifies an input inventory.
    PobOccurrenceUsageV3 {
        definitions: DataIdentity,
        source: SourcePin,
        roles: OwnedContentDigest,
        catalog: OwnedContentDigest,
        scalar_inputs: OwnedContentDigest,
        physical: Vec<PrimarySkillUsageInput>,
        occurrences: Vec<OccurrenceUsageRule>,
        /// Config-scoped values remain Scenario-owned; native preparation
        /// resolves their recipients against the independently composed build.
        scenario: Vec<ConfigurationUsageIntentInput>,
        /// Absence supplies no inventory-completion authority. This is an
        /// additive current-format capability, not a historical reader.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inventory: Option<UsageInventoryPolicy>,
    },
}
impl UsageInputPolicy {
    pub(super) fn physical_rows(&self) -> &[PrimarySkillUsageInput] {
        let Self::PobOccurrenceUsageV3 { physical, .. } = self;
        physical
    }
}

/// Authored usage projections and source evidence, without dependency seals.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum UsageInputContent {
    PobOccurrenceUsageV3 {
        source: SourcePin,
        catalog: OwnedContentDigest,
        physical: Vec<PrimarySkillUsageInput>,
        occurrences: Vec<OccurrenceUsageRule>,
        scenario: Vec<ConfigurationUsageIntentInput>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        inventory: Option<UsageInventoryPolicy>,
    },
}

impl UsageInputContent {
    /// The aggregate extracts content only from its checked normalization owner.
    pub(super) fn from_checked(input: UsageInputPolicy) -> Self {
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions: _,
            source,
            roles: _,
            catalog,
            scalar_inputs: _,
            physical,
            occurrences,
            scenario,
            inventory,
        } = input;
        Self::PobOccurrenceUsageV3 {
            source,
            catalog,
            physical,
            occurrences,
            scenario,
            inventory,
        }
    }

    /// The aggregate computes the scalar commitment from constructed prerequisites.
    /// This input still requires the existing shared semantic validators.
    pub(super) fn into_input<I: DefinitionSchemaIndex>(
        self,
        definitions: &I,
        roles: &OwnedSkillRoleIndex,
        scalar_inputs: OwnedContentDigest,
    ) -> UsageInputPolicy {
        let Self::PobOccurrenceUsageV3 {
            source,
            catalog,
            physical,
            occurrences,
            scenario,
            inventory,
        } = self;
        UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions: definitions.identity().clone(),
            source,
            roles: *roles.identity(),
            catalog,
            scalar_inputs,
            physical,
            occurrences,
            scenario,
            inventory,
        }
    }
}

/// An exact physical primary target and its independent typed usage selections.
/// Empty group attributes deliberately declare no parent source frame; group
/// reads or guards require named attributes. Inventory proof is separate.
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
    pub attributes: Vec<String>,
    pub guards: Vec<GemInputGuard>,
    pub group_attributes: Vec<String>,
    pub group_guards: Vec<GemInputGuard>,
    pub policies: Vec<OccurrenceUsagePolicy>,
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
    /// Read only the containing group's saved value. Absence stays unresolved;
    /// the occurrence cannot supply a fallback for an independent group fact.
    ContainingGroup {
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

pub(super) struct CompiledUsageInputs<'p> {
    rules: BTreeMap<&'p GemDefId, BoundUsage<'p>>,
    occurrences: Option<occurrences::CompiledOccurrences<'p>>,
    scenario: configuration::CompiledConfigurationUsage<'p>,
    inventory: Option<inventory::CompiledInventory<'p>>,
    pub work: usize,
}
struct BoundUsage<'p> {
    input: &'p PrimarySkillUsageInput,
    attributes: Vec<&'p str>,
    group_attributes: Vec<&'p str>,
    policies: Vec<BoundPolicy<'p>>,
}
struct BoundPolicy<'p> {
    policy: &'p UsagePolicyDefId,
    parameters: Vec<BoundParameter<'p>>,
}
struct BoundParameter<'p> {
    slot: DeclaredSlot<ParameterSlotDefId>,
    source: BoundUsageValue<'p>,
    schema: ValueSchema,
}

enum BoundUsageValue<'p> {
    Occurrence(ValueRecipe),
    ContainingGroup(ValueRecipe),
    ContainingGroupOverride {
        group: Box<ValueRecipe>,
        occurrence: ValueRecipe,
        fallback: BoundFallback<'p>,
    },
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
    usage_policy_identity(&policy.usage_inputs, limits)
}

/// Commit the actual constructed optional prerequisite without a policy shell.
pub(super) fn usage_policy_identity(
    input: &Option<UsageInputPolicy>,
    limits: NormalizationLimits,
) -> Result<OwnedContentDigest> {
    Ok(digest_owned(
        "owned-usage-inputs-v1",
        input,
        limits.max_policy_bytes.min(MAX_NORMALIZATION_POLICY_BYTES),
    )?)
}

/// Constructible only after attaching the real source-linked preference. Its
/// presence proves neither complete usage inventory nor numerical consumers.
pub(super) struct AttachedPrimaryUsage<'p> {
    gem: &'p GemDefId,
    policies: Vec<&'p UsagePolicyDefId>,
    source: SourceOccurrenceId,
    group: SourceOccurrenceId,
    preset: SkillPresetId,
    skill: SkillUseId,
    bindings: Vec<PresetUsageBindingDraft>,
}
pub(super) struct UsageInputContext<'a, 's> {
    pub row: &'a SourceEvidenceRow<'s>,
    pub group: &'a SourceEvidenceRow<'s>,
    pub gem: &'a DraftField<GemDefId>,
    pub skill: SkillUseId,
    pub preset: &'a mut SkillPresetDraft,
    pub inventory_proof: bool,
    pub receipts: &'a mut InventoryReceipts,
}
impl AttachedPrimaryUsage<'_> {
    pub(super) fn proves(
        &self,
        source: SourceOccurrenceId,
        gem: &GemDefId,
        policy: &UsagePolicyDefId,
    ) -> bool {
        self.source == source && self.gem == gem && self.policies.contains(&policy)
    }

    pub(super) fn binding(
        &self,
        source: SourceOccurrenceId,
        group: SourceOccurrenceId,
        preset: SkillPresetId,
        skill: SkillUseId,
        policy: &UsagePolicyDefId,
    ) -> Option<&PresetUsageBindingDraft> {
        if self.source != source
            || !self.policies.contains(&policy)
            || self.group != group
            || self.preset != preset
            || self.skill != skill
        {
            return None;
        }
        let mut matching = self
            .bindings
            .iter()
            .filter(|binding| matches!(&binding.selection.policy, DraftField::Known { value } if value == policy));
        let binding = matching.next()?;
        matching.next().is_none().then_some(binding)
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
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: bound_definitions,
        roles: bound_roles,
        scalar_inputs,
        ..
    } = policy.usage_inputs.as_mut().unwrap();
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
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: identity,
        source,
        roles: role_identity,
        catalog,
        scalar_inputs,
        physical,
        occurrences,
        scenario,
        inventory,
    } = usage_policy;
    if identity != definitions.identity()
        || role_identity != roles.identity()
        || roles.input().definitions != *identity
        || catalog != &roles.input().compilation.catalog_digest
        || scalar_inputs != &gem_inventory_scalar_inputs_identity(policy, limits)?
        || source != &roles.input().compilation.source
        || roles.input().mapping != *mappings.identity()
        || !provenance_is_subset(source, &mappings.input().source)
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
        scenario: configuration::CompiledConfigurationUsage::default(),
        inventory: None,
        work: 0,
    };
    charge(&mut compiled.work, bytes.len(), limits)?;
    if physical.len() > 4096 {
        return Err(NormalizationError::Limit("usage input rows"));
    }
    let mut selectors = BTreeSet::new();
    for input in physical {
        if input.attributes.len() > 64
            || input.guards.len() > 64
            || input.group_attributes.len() > 64
            || input.group_guards.len() > 64
            || compiled.rules.contains_key(&input.gem)
            || !selectors.insert((&input.game_id, &input.variant_id))
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
        }) if gem == &input.gem)
        {
            return invalid("usage input exact source mapping");
        }
        let Some(role) = roles.role(&input.gem) else {
            return invalid("usage input physical role");
        };
        if role.materialization != OwnedGemMaterialization::Physical
            || role.role != OwnedGemRole::Known(AuthoredGemRole::SkillUse)
            || role.primary != OwnedPrimarySkill::Known(input.primary.clone())
        {
            return invalid("usage input physical primary role");
        }
        let SchemaLookup::Known(gem) = definitions.definition(&input.gem) else {
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
            || !gem.skills.members.contains(&input.primary)
            || !matches!(
                definitions.definition(&input.primary),
                SchemaLookup::Known(_)
            )
            || input.supply.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || input.grant.declaration != SlotOwnerDefId::Gem(input.gem.clone())
            || !gem
                .declarations
                .skill_grants
                .members
                .contains(&input.supply)
            || !gem.declarations.grants.members.contains(&input.grant)
        {
            return invalid("usage input declared primary supply");
        }
        let SchemaLookup::Known(supply) = definitions.slot(&input.supply) else {
            return invalid("usage input supply schema");
        };
        let SchemaLookup::Known(grant) = definitions.slot(&input.grant) else {
            return invalid("usage input grant schema");
        };
        charge(&mut compiled.work, grant.provider_roles.len(), limits)?;
        if supply.skill != input.primary
            || grant.target != GrantTarget::Skill(input.supply.clone())
            || !grant.provider_roles.contains(&ProviderRole::SkillUse)
        {
            return invalid("usage input primary grant target");
        }
        let attributes: BTreeSet<_> = input.attributes.iter().map(String::as_str).collect();
        let groups: BTreeSet<_> = input.group_attributes.iter().map(String::as_str).collect();
        if attributes.len() != input.attributes.len()
            || groups.len() != input.group_attributes.len()
            || attributes
                .iter()
                .chain(&groups)
                .any(|name| name.is_empty() || name.len() > 128)
            || ["gemId", "variantId", "skillId", "nameSpec"]
                .iter()
                .any(|name| !attributes.contains(name))
        {
            return invalid("usage input attribute frame");
        }
        validate_guards(&input.guards, &attributes, limits)?;
        validate_guards(&input.group_guards, &groups, limits)?;
        let policies = compile_policies(
            &input.policies,
            PolicyCompilation {
                definitions,
                roles,
                limits,
                gem: &input.gem,
                skill: &input.primary,
                attributes: &attributes,
                groups: &groups,
                allow_aliases: false,
            },
            &mut compiled.work,
        )?;
        compiled.rules.insert(
            &input.gem,
            BoundUsage {
                attributes: input.attributes.iter().map(String::as_str).collect(),
                group_attributes: input.group_attributes.iter().map(String::as_str).collect(),
                input,
                policies,
            },
        );
    }
    if !occurrences.is_empty() {
        compiled.occurrences = Some(occurrences::compile(
            occurrences,
            definitions,
            roles,
            mappings,
            &mut compiled.work,
            limits,
        )?);
    }
    compiled.scenario = configuration::compile(scenario, definitions, &mut compiled.work, limits)?;
    let mut inventory_work = 0;
    compiled.inventory =
        inventory::compile(inventory.as_ref(), &compiled, limits, &mut inventory_work)?;
    charge(&mut compiled.work, inventory_work, limits)?;
    Ok(Some(compiled))
}

struct PolicyCompilation<'a, I> {
    definitions: &'a I,
    roles: &'a OwnedSkillRoleIndex,
    limits: NormalizationLimits,
    gem: &'a GemDefId,
    skill: &'a SkillDefId,
    attributes: &'a BTreeSet<&'a str>,
    groups: &'a BTreeSet<&'a str>,
    allow_aliases: bool,
}
/// Destination-neutral ownership checks shared by exact occurrence and
/// Scenario intent transport. Source frame/selection semantics stay separate.
fn policy_schema<'a, I: DefinitionSchemaIndex>(
    definitions: &'a I,
    policy: &UsagePolicyDefId,
    parameters: usize,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<&'a UsagePolicySchema> {
    charge(work, 1, limits)?;
    let SchemaLookup::Known(schema) = definitions.definition(policy) else {
        return invalid("usage policy schema");
    };
    charge(
        work,
        schema.targets.len() + schema.declarations.parameters.members.len(),
        limits,
    )?;
    if !schema.targets.contains(&UsageTargetKind::Skill)
        || !schema.declarations.parameters.is_complete()
        || parameters == 0
        || parameters > 64
        || schema.declarations.parameters.members.len() != parameters
    {
        return invalid("usage policy parameters");
    }
    Ok(schema)
}
fn parameter_schema<'a, I: DefinitionSchemaIndex>(
    definitions: &'a I,
    policy: &UsagePolicyDefId,
    schema: &UsagePolicySchema,
    slot: &DeclaredSlot<ParameterSlotDefId>,
    work: &mut usize,
    limits: NormalizationLimits,
) -> Result<&'a ParameterSlotSchema> {
    charge(work, schema.declarations.parameters.members.len(), limits)?;
    if slot.declaration != SlotOwnerDefId::UsagePolicy(policy.clone())
        || !schema.declarations.parameters.members.contains(slot)
    {
        return invalid("usage parameter authority");
    }
    let SchemaLookup::Known(value) = definitions.slot(slot) else {
        return invalid("usage parameter schema");
    };
    charge(work, value.sites.len(), limits)?;
    if value.presence != SlotPresence::RequiredOnce
        || value.sites != [ParameterSite::UsagePolicyParameter]
    {
        return invalid("usage parameter site");
    }
    Ok(value)
}
fn compile_policies<'p, I: DefinitionSchemaIndex>(
    inputs: &'p [OccurrenceUsagePolicy],
    context: PolicyCompilation<'_, I>,
    work: &mut usize,
) -> Result<Vec<BoundPolicy<'p>>> {
    let PolicyCompilation {
        definitions,
        roles,
        limits,
        gem,
        skill,
        attributes,
        groups,
        allow_aliases,
    } = context;
    if inputs.is_empty() || inputs.len() > 64 {
        return invalid("occurrence usage policies");
    }
    let mut ids = BTreeSet::new();
    let mut policies = Vec::new();
    for policy in inputs {
        if !ids.insert(&policy.policy) {
            return invalid("duplicate occurrence usage policy");
        }
        let schema = policy_schema(
            definitions,
            &policy.policy,
            policy.parameters.len(),
            work,
            limits,
        )?;
        let mut slots = BTreeSet::new();
        let mut parameters = Vec::new();
        for parameter in &policy.parameters {
            if !slots.insert(&parameter.slot) {
                return invalid("duplicate occurrence usage parameter");
            }
            let value = parameter_schema(
                definitions,
                &policy.policy,
                schema,
                &parameter.slot,
                work,
                limits,
            )?;
            let recipe = |input: &ValueRecipeInput, names: &BTreeSet<&str>| {
                typed_usage_recipe(
                    input,
                    &value.value,
                    names,
                    definitions,
                    limits,
                    allow_aliases,
                )
            };
            let source = match &parameter.source {
                UsageValueSource::Occurrence { value } => {
                    BoundUsageValue::Occurrence(recipe(value, attributes)?)
                }
                UsageValueSource::ContainingGroup { value } => {
                    BoundUsageValue::ContainingGroup(recipe(value, groups)?)
                }
                UsageValueSource::ContainingGroupOverride {
                    group,
                    occurrence,
                    fallback_admission,
                } => BoundUsageValue::ContainingGroupOverride {
                    occurrence: recipe(occurrence, attributes)?,
                    group: Box::new(recipe(group, groups)?),
                    fallback: compile_fallback(
                        fallback_admission,
                        gem,
                        skill,
                        roles,
                        work,
                        limits,
                    )?,
                },
            };
            parameters.push(BoundParameter {
                slot: parameter.slot.clone(),
                source,
                schema: value.value.clone(),
            });
        }
        policies.push(BoundPolicy {
            policy: &policy.policy,
            parameters,
        });
    }
    Ok(policies)
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

fn typed_usage_recipe<I: DefinitionSchemaIndex>(
    input: &ValueRecipeInput,
    schema: &ValueSchema,
    attributes: &BTreeSet<&str>,
    definitions: &I,
    limits: NormalizationLimits,
    aliases: bool,
) -> Result<ValueRecipe> {
    let matching = match (&input.codec.codec, schema) {
        (ValueCodecKind::Integer { .. }, ValueSchema::Integer(_)) => true,
        (ValueCodecKind::Boolean { .. }, ValueSchema::Boolean) => true,
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
        return invalid("typed usage source recipe");
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
    // Source effect matching needs complete sibling evidence. Independent local
    // reads need only their authored frame and exact saved-preset ancestry.
    if skill_source_census::sets(b)?.is_none() {
        return Ok(false);
    }
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

// Both typed source locations are already structurally proved. Selecting an
// override by attribute presence prevents malformed values or zero falling back.
fn selected_recipe<'a, 's>(
    b: &mut Builder<'_, '_>,
    parameter: &'a BoundParameter<'_>,
    row: &'a SourceEvidenceRow<'s>,
    group: &'a SourceEvidenceRow<'s>,
) -> Result<Option<(&'a SourceEvidenceRow<'s>, &'a ValueRecipe)>> {
    match &parameter.source {
        BoundUsageValue::Occurrence(recipe) => Ok(Some((row, recipe))),
        BoundUsageValue::ContainingGroup(recipe) => Ok(Some((group, recipe))),
        BoundUsageValue::ContainingGroupOverride {
            group: recipe,
            occurrence,
            fallback,
        } => {
            let name = &recipe.input().tiers[0].selectors[0].name;
            b.charge(group.attributes().len())?;
            if group.attribute(name).is_some() {
                return Ok(Some((group, recipe)));
            }
            Ok(fallback_admitted(b, fallback, row, group)?.then_some((row, occurrence)))
        }
    }
}

// Shared decoding preserves source-presence precedence and the deterministic issue
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
            receipts,
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
            || !b.gem_guards_match(row, &bound.input.guards)?
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
        if !bound.group_attributes.is_empty() {
            source_shape::charge_row(b, group)?;
            if !source_shape::plain_row(group, &bound.group_attributes, false)
                || !source_shape::container_text(group)
                || !b.gem_guards_match(group, &bound.input.group_guards)?
            {
                return Ok(None);
            }
        }
        let Some(sets) = skill_source_census::container_sets(b)? else {
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
        let mut attached = Vec::new();
        let mut bindings = Vec::new();
        for policy in &bound.policies {
            b.charge(1)?;
            let (parameters, converted) = decode_parameters(b, row, group, &policy.parameters)?;
            let selection = UsagePolicyDraft {
                policy: policy.policy.clone().into(),
                target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                    provider: ProviderKey {
                        root: ProviderRoot::SkillUse(skill),
                        grant_path: vec![],
                    },
                    slot: bound.input.supply.clone(),
                })))
                .into(),
                parameters,
            };
            if converted && self.inventory.is_some() {
                inventory::record_usage(
                    b,
                    row,
                    group,
                    preset.id,
                    inventory::UsageInventorySource::Physical { gem: gem.clone() },
                    !bound.group_attributes.is_empty(),
                    &PresetUsageBindingDraft {
                        selection: selection.clone(),
                        applicability:
                            poe_optimizer_core::owned_preset_intent::PresetApplicability::Required,
                    },
                    &policy.parameters,
                    receipts,
                )?;
            }
            if converted && inventory_proof && self.inventory.is_some() {
                // Only whole-inventory accounting needs a live binding snapshot.
                // The designated policy is selected by the retained source proof.
                b.charge(selection.parameters.members.len().saturating_add(1))?;
                bindings.push(PresetUsageBindingDraft {
                    selection: selection.clone(),
                    applicability:
                        poe_optimizer_core::owned_preset_intent::PresetApplicability::Required,
                });
            }
            preset
                .usage_preferences
                .as_mut()
                .expect("initialized above")
                .members
                .push(selection);
            if converted {
                attached.push(policy.policy);
            }
        }
        // The preset already has its source-set origin. This exact Gem row also
        // contributed its preference, independently of the Gem/Skill links.
        b.link(source, OwnedOriginTarget::SkillPreset(preset.id))?;
        if !bound.group_attributes.is_empty() {
            // Parent overrides and frame guards are independent contributing
            // evidence. Preserve their exact origin without duplicate links.
            let parent = group.occurrence().id();
            let target = OwnedOriginTarget::SkillPreset(preset.id);
            b.charge(b.origins[parent.ordinal() as usize].links.len())?;
            if !b.origins[parent.ordinal() as usize].links.contains(&target) {
                b.link(parent, target)?;
            }
        }
        if !inventory_proof
            || attached.is_empty()
            || !matches!(&preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Pending { code, .. } if code.as_str() == "usage-preferences-not-converted")
        {
            return Ok(None);
        }
        Ok(Some(AttachedPrimaryUsage {
            gem: &bound.input.gem,
            policies: attached,
            source,
            group: group.occurrence().id(),
            preset: preset.id,
            skill,
            bindings,
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
    pub(super) fn declares_configuration_selector(
        &self,
        b: &mut Builder<'_, '_>,
        selector: &ValueSelector,
    ) -> Result<bool> {
        configuration::declares_selector(b, &self.scenario, selector)
    }

    pub(super) fn collect_configuration(
        &self,
        b: &mut Builder<'_, '_>,
    ) -> Result<BTreeMap<SourceOccurrenceId, ProvenConfigurationUsage>> {
        configuration::collect(b, &self.scenario)
    }
    pub(super) fn materialize_occurrences(
        &self,
        b: &mut Builder<'_, '_>,
        draft: &mut DraftSessionInput,
        context: generated_skill_sources::Context<'_>,
        receipts: &mut InventoryReceipts,
    ) -> Result<Vec<MaterializedUsage>> {
        match &self.occurrences {
            Some(compiled) => occurrences::materialize(
                b,
                draft,
                compiled,
                context,
                self.inventory.is_some(),
                receipts,
            ),
            None => Ok(vec![]),
        }
    }
    pub(super) fn inventory_enabled(&self) -> bool {
        self.inventory.is_some()
    }
    pub(super) fn complete_inventory(
        &self,
        b: &mut Builder<'_, '_>,
        draft: &mut DraftSessionInput,
        presets: &BTreeMap<SourceOccurrenceId, usize>,
        receipts: &InventoryReceipts,
    ) -> Result<()> {
        inventory::complete(b, draft, presets, self.inventory.as_ref(), receipts)
    }
}
