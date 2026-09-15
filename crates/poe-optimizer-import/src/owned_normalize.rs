//! Fresh, conservative PoB-to-owned draft normalization.
//!
//! This adapter consumes collected evidence and injected owned mapping/role/value
//! artifacts. It never loads a game snapshot, selects a PoB UI view, runs source
//! programs, computes a build, or fills missing mechanics with profile defaults.
//! Unconverted semantics remain real pending fields/collection obligations.
use crate::{
    build_instance::{AuthoredInstanceId, SourceOccurrenceId},
    owned_item_lines::*,
    owned_item_source::*,
    owned_mapping::*,
    owned_reward_policy::*,
    owned_skill_catalog::*,
    owned_source::*,
    owned_tree_policy::{OwnedTreeNormalizationPolicy, TreePolicyError},
    owned_value::ValueCodecKind,
    owned_value_policy::*,
};
use poe_optimizer_core::{
    build_identity::*,
    data::DataIdentity,
    owned_build::*,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_draft::*,
    owned_schema::*,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

mod items;
mod quality;
mod tree;
pub use items::{NormalizedItemLine, NormalizedItemText};
pub use quality::{GemQualityKindRule, GemQualityPolicy, GemQualityPolicyInput};

/// The caller supplies desired measurements. There is no built-in metric list.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImportQueryTemplate {
    pub id: QueryId,
    pub metric: ExternalSelector,
    pub target: ImportQueryTarget,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ImportQueryTarget {
    Player,
    /// A source target awaiting semantic correspondence, not a zero measurement.
    Unresolved(OwnedDefinitionKey),
}
/// Reviewed syntax interpretation, separate from owned game definitions/rules.
/// No gameplay defaults are built into the production normalizer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NormalizationPolicy {
    pub version: OwnedDefinitionKey,
    pub namespace: GameVersionNamespace,
    pub character_level: ValueRecipeInput,
    pub gem_level: ValueRecipeInput,
    pub gem_enabled: ValueRecipeInput,
    pub group_enabled: ValueRecipeInput,
    /// Exact source attribute spellings; lexical failures never equal Missing.
    pub manual_skill_sources: Vec<SourceComponent>,
    pub empty_item_keys: Vec<SourceComponent>,
    /// Only these reviewed nonempty source prefixes admit physical support
    /// children on generated groups. Other generated rows retain source evidence
    /// and pending semantic obligations.
    pub generated_support_prefixes: Vec<String>,
    /// Exact CSV attributes on Spec (e.g. nodes). Weapon-set overlays are not
    /// independent allocations; their accounting/access remains pending here.
    pub allocation_attribute: String,
    /// Explicitly permits one active gem in a manual group to receive supports.
    /// Multiple active gems always need declared payload/target conversion.
    pub single_active_support_target: bool,
    /// Exact source slot relations. Empty means no equipment scope conversion.
    pub equipment_loadouts: Vec<EquipmentLoadoutRule>,
    /// Explicit authored amount plus a reviewed exact source-kind convention.
    pub gem_quality: GemQualityPolicy,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EquipmentLoadoutRule {
    pub source_slot: SourceComponent,
    pub destination: EquipmentSlotDefId,
    pub scope: ImportEquipmentScope,
}

/// Keys are injected import-local correspondence, never owned definition IDs.
/// A key is allocated only when a unique, mapped source Slot actually exposes it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ImportEquipmentScope {
    Shared,
    Selected { loadouts: Vec<OwnedDefinitionKey> },
}
#[derive(Clone, Copy, Debug)]
pub struct NormalizationLimits {
    pub draft: DraftLimits,
    pub mapping: OwnedMappingLimits,
    pub value: ValuePolicyLimits,
    pub max_work: usize,
    pub max_origin_links: usize,
    pub max_policy_bytes: usize,
}
impl Default for NormalizationLimits {
    fn default() -> Self {
        Self {
            draft: DraftLimits::default(),
            mapping: OwnedMappingLimits::default(),
            value: ValuePolicyLimits::default(),
            max_work: 500_000,
            max_origin_links: 200_000,
            max_policy_bytes: 1024 * 1024,
        }
    }
}
#[derive(Debug, thiserror::Error)]
pub enum NormalizationError {
    #[error("invalid normalization limit: {0}")]
    InvalidLimit(&'static str),
    #[error("normalization exceeds {0}")]
    Limit(&'static str),
    #[error("fresh normalization needs source lineage and an allocator at/above its watermark")]
    AllocatorBinding,
    #[error("normalization artifact bindings disagree")]
    Binding,
    #[error("invalid normalization policy: {0}")]
    Policy(&'static str),
    #[error("mapped target has the wrong semantic domain")]
    MappingDomain,
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Value(#[from] ValuePolicyError),
    #[error(transparent)]
    Reward(#[from] RewardPolicyError),
    #[error(transparent)]
    Item(#[from] ItemLineError),
    #[error(transparent)]
    ItemSource(#[from] ItemSourceError),
    #[error(transparent)]
    Tree(#[from] TreePolicyError),
    #[error(transparent)]
    Identity(#[from] BuildIdentityError),
    #[error(transparent)]
    Structure(#[from] StructuralError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
}
type Result<T> = std::result::Result<T, NormalizationError>;
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum OwnedOriginTarget {
    WeaponLoadout {
        key: OwnedDefinitionKey,
        id: WeaponLoadoutId,
    },
    Item(ItemRecordId),
    ItemReference(ItemRecordId),
    Modifier(ModifierInstanceId),
    Equipment(ItemSlotUseId),
    Reward(RewardSelectionId),
    Gem(GemInstanceId),
    Skill(SkillUseId),
    Support(SupportAssignmentId),
    Allocation(AllocationId),
    ImplicitPassive {
        character: CharacterPresetId,
        node: PassiveNodeDefId,
    },
    CharacterPreset(CharacterPresetId),
    EquipmentPreset(EquipmentPresetId),
    AllocationPreset(AllocationPresetId),
    SkillPreset(SkillPresetId),
    ChoicePreset(ChoicePresetId),
    ScenarioPreset(ScenarioPresetId),
    QueryPreset(QueryPresetId),
    Issue(DraftIssueId),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum SourceDisposition {
    Contributes,
    SourceOnly(OwnedDefinitionKey),
}
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct SourceOwnedOrigin {
    pub source: SourceOccurrenceId,
    pub disposition: SourceDisposition,
    pub links: Vec<OwnedOriginTarget>,
}
#[derive(Clone, Debug, Serialize)]
pub struct FreshNormalizationSidecar {
    pub schema_version: u32,
    pub source_sha256: String,
    pub source_bytes: usize,
    pub source_schema: u32,
    pub revision: BuildRevision,
    pub allocator_before: InstanceAllocatorState,
    pub allocator_after: InstanceAllocatorState,
    pub source_allocator: InstanceAllocatorState,
    pub policy: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub mapping_source: OwnedContentDigest,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub skill_roles: OwnedContentDigest,
    pub reward_policy: OwnedContentDigest,
    pub item_policy: OwnedContentDigest,
    pub item_source_policy: OwnedContentDigest,
    pub tree_policy: Option<OwnedContentDigest>,
    pub item_texts: Vec<NormalizedItemText>,
    pub draft: OwnedContentDigest,
    pub origins: Vec<SourceOwnedOrigin>,
}
/// Immutable result. The host publishes it and the returned watermark atomically;
/// repeating a fresh call is not restore, edit, adoption or exclusive-ID authority.
#[derive(Debug)]
pub struct NormalizedImport {
    draft: DraftSession,
    sidecar: FreshNormalizationSidecar,
}
/// Exact immutable artifact dependencies. Construction itself grants no authority;
/// normalize_fresh validates their content bindings before allocating anything.
pub struct NormalizationArtifacts<'a, I> {
    pub mappings: &'a OwnedMappingIndex,
    pub registry: &'a OwnedIdRegistry,
    pub definitions: &'a I,
    pub roles: &'a OwnedSkillRoleIndex,
    pub rewards: &'a OwnedRewardPolicy,
    pub items: &'a OwnedItemLinePolicy,
    pub item_source: &'a ItemSourceLayoutPolicy,
    pub tree: Option<&'a OwnedTreeNormalizationPolicy>,
}
impl NormalizedImport {
    pub fn draft(&self) -> &DraftSession {
        &self.draft
    }
    pub fn sidecar(&self) -> &FreshNormalizationSidecar {
        &self.sidecar
    }
    pub fn allocator_after(&self) -> InstanceAllocatorState {
        self.sidecar.allocator_after
    }
    pub fn into_parts(self) -> (DraftSession, FreshNormalizationSidecar) {
        (self.draft, self.sidecar)
    }
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).expect("fixed adapter diagnostic")
}
fn complete<T>(members: Vec<T>) -> DraftList<T> {
    DraftList {
        members,
        completion: DraftListCompletion::Complete,
    }
}
fn component(
    b: &Builder<'_, '_>,
    row: &SourceEvidenceRow<'_>,
    name: &str,
) -> Option<SourceComponent> {
    match b.attributes[row.occurrence().id().ordinal() as usize].get(name) {
        None => Some(SourceComponent::Missing),
        Some((_, v)) => v.decoded().ok().map(|v| SourceComponent::Text(v.into())),
    }
}
fn definition<T>(target: &SchemaSubject, f: impl Fn(&DefinitionAddress) -> Option<T>) -> Option<T> {
    match target {
        SchemaSubject::Definition(v) => f(v),
        _ => None,
    }
}
fn catalog(kind: ExternalCatalogKind, key: SourceComponent) -> ExternalSelector {
    ExternalSelector::Catalog {
        kind,
        key,
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    }
}
enum ScalarValue {
    Selected(ParameterValue),
    Defaulted(ParameterValue),
    Absent,
    Missing,
    Unavailable,
    Malformed,
}
struct Builder<'e, 's> {
    evidence: &'e SourceProjectEvidence<'s>,
    mappings: &'e OwnedMappingIndex,
    roles: &'e OwnedSkillRoleIndex,
    rewards: &'e OwnedRewardPolicy,
    items: &'e OwnedItemLinePolicy,
    item_source: &'e ItemSourceLayoutPolicy,
    item_texts: Vec<NormalizedItemText>,
    allocator: InstanceAllocator,
    limits: NormalizationLimits,
    work: usize,
    links: usize,
    issues: usize,
    origins: Vec<SourceOwnedOrigin>,
    attributes: Vec<BTreeMap<&'e str, (u32, &'e SourceAttributeEvidence<'s>)>>,
}
impl Builder<'_, '_> {
    fn charge(&mut self, n: usize) -> Result<()> {
        self.work = self
            .work
            .checked_add(n)
            .ok_or(NormalizationError::Limit("work"))?;
        if self.work > self.limits.max_work {
            return Err(NormalizationError::Limit("work"));
        }
        Ok(())
    }
    fn id<T: BuildInstanceId>(&mut self) -> Result<T> {
        self.charge(1)?;
        Ok(self.allocator.allocate()?)
    }
    fn link(&mut self, source: SourceOccurrenceId, target: OwnedOriginTarget) -> Result<()> {
        self.charge(1)?;
        if self.links >= self.limits.max_origin_links {
            return Err(NormalizationError::Limit("origin links"));
        }
        self.links += 1;
        self.origins[source.ordinal() as usize].links.push(target);
        Ok(())
    }
    fn issue(&mut self, source: SourceOccurrenceId) -> Result<DraftIssueId> {
        if self.issues >= self.limits.draft.max_issues {
            return Err(NormalizationError::Limit("issues"));
        }
        self.issues += 1;
        let id = self.id()?;
        self.link(source, OwnedOriginTarget::Issue(id))?;
        Ok(id)
    }
    fn pending<T>(&mut self, source: SourceOccurrenceId, code: &str) -> Result<DraftField<T>> {
        Ok(DraftField::Pending(PendingValue {
            id: self.issue(source)?,
            code: key(code),
            candidates: vec![],
        }))
    }
    fn closure<T>(
        &mut self,
        source: SourceOccurrenceId,
        code: &str,
        members: Vec<T>,
    ) -> Result<DraftList<T>> {
        Ok(DraftList {
            members,
            completion: DraftListCompletion::Pending {
                id: self.issue(source)?,
                code: key(code),
            },
        })
    }
    fn quality(&mut self, s: SourceOccurrenceId) -> Result<DraftQuality> {
        Ok(DraftQuality::Pending(PendingValue {
            id: self.issue(s)?,
            code: key("quality-not-converted"),
            candidates: vec![],
        }))
    }
    fn mapped<T>(
        &mut self,
        s: SourceOccurrenceId,
        selector: Option<ExternalSelector>,
        project: impl Fn(&DefinitionAddress) -> Option<T>,
    ) -> Result<DraftField<T>> {
        match selector.as_ref().and_then(|v| self.mappings.lookup(v)) {
            Some(MappingOutcome::Mapped { target, .. }) => Ok(DraftField::from(
                definition(target, &project).ok_or(NormalizationError::MappingDomain)?,
            )),
            Some(MappingOutcome::Ambiguous { candidates, issue }) => {
                if candidates.len() > self.limits.draft.max_candidates_per_field {
                    return Err(NormalizationError::Limit("mapping candidates"));
                }
                self.charge(candidates.len())?;
                let candidates = candidates
                    .iter()
                    .map(|v| definition(v, &project).ok_or(NormalizationError::MappingDomain))
                    .collect::<Result<Vec<_>>>()?;
                let code = issue.clone();
                Ok(DraftField::Pending(PendingValue {
                    id: self.issue(s)?,
                    code,
                    candidates,
                }))
            }
            _ => self.pending(s, "definition-unmapped"),
        }
    }
    fn scalar(
        &mut self,
        row: &SourceEvidenceRow<'_>,
        recipe: &ValueRecipe,
    ) -> Result<Option<ParameterValue>> {
        Ok(match self.scalar_value(row, recipe)? {
            ScalarValue::Selected(value) | ScalarValue::Defaulted(value) => Some(value),
            ScalarValue::Absent
            | ScalarValue::Missing
            | ScalarValue::Unavailable
            | ScalarValue::Malformed => None,
        })
    }
    fn scalar_value(
        &mut self,
        row: &SourceEvidenceRow<'_>,
        recipe: &ValueRecipe,
    ) -> Result<ScalarValue> {
        let mut candidates = vec![];
        for tier in &recipe.input().tiers {
            for selector in &tier.selectors {
                self.charge(1)?;
                // Recipe policy validation confines these scalar contexts to
                // their own exact attributes. Config/default traversal is separate.
                if let Some((index, a)) = self.attributes[row.occurrence().id().ordinal() as usize]
                    .get(selector.name.as_str())
                {
                    candidates.push(ValueCandidate {
                        selector,
                        origin: SourceAttributeRef {
                            occurrence: row.occurrence().id(),
                            index: *index,
                        },
                        value: match a.decoded() {
                            Ok(v) => CandidateValue::Decoded(v),
                            Err(e) => CandidateValue::Unavailable(e),
                        },
                    });
                }
            }
        }
        Ok(match recipe.decide(&candidates)?.outcome {
            ValueOutcome::Selected { value, .. } => ScalarValue::Selected(value),
            ValueOutcome::Defaulted { value } => ScalarValue::Defaulted(value),
            ValueOutcome::Absent => ScalarValue::Absent,
            ValueOutcome::Pending {
                reason: ValuePendingReason::Missing,
            } => ScalarValue::Missing,
            ValueOutcome::Pending {
                reason: ValuePendingReason::Unavailable(_),
            } => ScalarValue::Unavailable,
            ValueOutcome::Pending {
                reason: ValuePendingReason::Decode(_),
            } => ScalarValue::Malformed,
        })
    }
    fn level(
        &mut self,
        row: Option<&SourceEvidenceRow<'_>>,
        origin: SourceOccurrenceId,
        recipe: &ValueRecipe,
    ) -> Result<DraftField<u16>> {
        if let Some(row) = row
            && let Some(ParameterValue::Integer(v)) = self.scalar(row, recipe)?
            && let Ok(v) = u16::try_from(v.get())
        {
            return Ok(v.into());
        }
        self.pending(origin, "level-unresolved")
    }
    fn enabled(
        &mut self,
        row: &SourceEvidenceRow<'_>,
        group: &SourceEvidenceRow<'_>,
        gem: &ValueRecipe,
        parent: &ValueRecipe,
    ) -> Result<DraftField<bool>> {
        let a = self.scalar(row, gem)?;
        let b = self.scalar(group, parent)?;
        if let (Some(ParameterValue::Boolean(a)), Some(ParameterValue::Boolean(b))) = (a, b) {
            Ok((a && b).into())
        } else {
            self.pending(row.occurrence().id(), "enabled-unresolved")
        }
    }
    fn ancestor(
        &mut self,
        mut s: SourceOccurrenceId,
        name: &str,
    ) -> Result<Option<SourceOccurrenceId>> {
        while let Some(p) = self.evidence.rows()[s.ordinal() as usize]
            .occurrence()
            .parent()
        {
            self.charge(1)?;
            let row = &self.evidence.rows()[p.ordinal() as usize];
            if !row.occurrence().has_namespace_context() && row.occurrence().name() == name {
                return Ok(Some(p));
            }
            s = p;
        }
        Ok(None)
    }
}
fn validate_policy(
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<[ValueRecipe; 4]> {
    let hard = NormalizationLimits::default();
    for (name, value, max) in [
        ("work", limits.max_work, hard.max_work),
        (
            "origin links",
            limits.max_origin_links,
            hard.max_origin_links,
        ),
        (
            "policy bytes",
            limits.max_policy_bytes,
            hard.max_policy_bytes,
        ),
    ] {
        if value == 0 || value > max {
            return Err(NormalizationError::InvalidLimit(name));
        }
    }
    digest_owned(
        "owned-normalization-policy-v3",
        policy,
        limits.max_policy_bytes,
    )?;
    if policy.allocation_attribute.is_empty() || policy.allocation_attribute.len() > 128 {
        return Err(NormalizationError::Policy("allocation attribute"));
    }
    for values in [&policy.manual_skill_sources, &policy.empty_item_keys] {
        if values.len() > 64 || values.iter().collect::<BTreeSet<_>>().len() != values.len() {
            return Err(NormalizationError::Policy("source token table"));
        }
    }
    if policy
        .empty_item_keys
        .iter()
        .any(|v| matches!(v, SourceComponent::Missing))
    {
        return Err(NormalizationError::Policy(
            "missing item reference is not an empty slot",
        ));
    }
    if policy.generated_support_prefixes.len() > 64
        || policy
            .generated_support_prefixes
            .iter()
            .any(|v| v.is_empty() || v.len() > 128)
        || policy
            .generated_support_prefixes
            .iter()
            .collect::<BTreeSet<_>>()
            .len()
            != policy.generated_support_prefixes.len()
    {
        return Err(NormalizationError::Policy(
            "generated support origin prefixes",
        ));
    }
    let inputs = [
        &policy.character_level,
        &policy.gem_level,
        &policy.gem_enabled,
        &policy.group_enabled,
    ];
    for (index, input) in inputs.iter().enumerate() {
        if input.codec.namespace != policy.namespace {
            return Err(NormalizationError::Binding);
        }
        if (index < 2 && !matches!(input.codec.codec, ValueCodecKind::Integer { .. }))
            || (index >= 2 && !matches!(input.codec.codec, ValueCodecKind::Boolean { .. }))
        {
            return Err(NormalizationError::Policy("scalar output type"));
        }
        if input
            .tiers
            .iter()
            .flat_map(|t| &t.selectors)
            .any(|s| s.lane != ValueLane::Attribute)
        {
            return Err(NormalizationError::Policy("scalar source lane"));
        }
    }
    Ok([
        ValueRecipe::new(inputs[0].clone(), limits.value)?,
        ValueRecipe::new(inputs[1].clone(), limits.value)?,
        ValueRecipe::new(inputs[2].clone(), limits.value)?,
        ValueRecipe::new(inputs[3].clone(), limits.value)?,
    ])
}

// Compile exact source-slot relations once. Unknown mapping coverage does not
// become a fallback scope; positively contradictory artifact facts are rejected.
fn equipment_loadout_rules<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<BTreeMap<&'p str, &'p EquipmentLoadoutRule>> {
    if policy.equipment_loadouts.len() > 128 {
        return Err(NormalizationError::Policy("equipment loadout rules"));
    }
    let mut sources = BTreeSet::new();
    let mut keys = BTreeSet::new();
    let mut admitted = BTreeMap::new();
    for rule in &policy.equipment_loadouts {
        let SourceComponent::Text(source) = &rule.source_slot else {
            return Err(NormalizationError::Policy("missing equipment slot rule"));
        };
        if source.is_empty()
            || source.len() > limits.mapping.max_string_bytes
            || !sources.insert(source.as_str())
        {
            return Err(NormalizationError::Policy(
                "duplicate or invalid equipment slot rule",
            ));
        }
        if rule.destination.namespace() != &policy.namespace {
            return Err(NormalizationError::Binding);
        }
        if let ImportEquipmentScope::Selected { loadouts } = &rule.scope {
            if loadouts.is_empty()
                || loadouts.len() > 64
                || loadouts.iter().collect::<BTreeSet<_>>().len() != loadouts.len()
            {
                return Err(NormalizationError::Policy("equipment loadout keys"));
            }
            keys.extend(loadouts.iter());
            if keys.len() > 64 {
                return Err(NormalizationError::Policy("equipment loadout keys"));
            }
        }
        let selector = catalog(ExternalCatalogKind::EquipmentSlot, rule.source_slot.clone());
        let Some(MappingOutcome::Mapped { target, .. }) = mappings.lookup(&selector) else {
            continue;
        };
        if target
            != &SchemaSubject::Definition(DefinitionAddress::EquipmentSlot(
                rule.destination.clone(),
            ))
        {
            return Err(NormalizationError::Policy(
                "equipment loadout mapping contradiction",
            ));
        }
        match definitions.definition(&rule.destination) {
            SchemaLookup::Known(schema) => {
                if !matches!(
                    (&rule.scope, schema.scope),
                    (
                        ImportEquipmentScope::Shared,
                        ScopePolicy::Shared | ScopePolicy::Either
                    ) | (
                        ImportEquipmentScope::Selected { .. },
                        ScopePolicy::Selected | ScopePolicy::Either
                    )
                ) {
                    return Err(NormalizationError::Policy(
                        "equipment loadout schema contradiction",
                    ));
                }
            }
            SchemaLookup::Unmapped(_) => {} // Explicit import fact; schema binding remains unresolved.
            SchemaLookup::Missing => continue,
            SchemaLookup::NamespaceMismatch | SchemaLookup::InconsistentIndex => {
                return Err(NormalizationError::Binding);
            }
        }
        admitted.insert(source.as_str(), rule);
    }
    Ok(admitted)
}

/// The same source-policy validation is used by fresh import and offline package
/// transitions. Compiled adapters never become part of the native game package.
struct CompiledNormalizationInputs<'p> {
    recipes: [ValueRecipe; 4],
    equipment_rules: BTreeMap<&'p str, &'p EquipmentLoadoutRule>,
    gem_quality: Option<quality::CompiledGemQuality>,
}
fn compile_normalization_inputs<'p, I: DefinitionSchemaIndex>(
    policy: &'p NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    limits: NormalizationLimits,
) -> Result<CompiledNormalizationInputs<'p>> {
    let recipes = validate_policy(policy, limits)?;
    mappings.validate_limits(limits.mapping)?;
    if mappings.input().source.system != ExternalSourceSystem::PathOfBuilding2
        || policy.namespace != *definitions.namespace()
    {
        return Err(NormalizationError::Binding);
    }
    Ok(CompiledNormalizationInputs {
        recipes,
        equipment_rules: equipment_loadout_rules(policy, mappings, definitions, limits)?,
        gem_quality: quality::compile(&policy.gem_quality, definitions, limits)?,
    })
}

pub(crate) fn validate_normalization_inputs<I: DefinitionSchemaIndex>(
    policy: &NormalizationPolicy,
    mappings: &OwnedMappingIndex,
    definitions: &I,
    queries: &[ImportQueryTemplate],
    limits: NormalizationLimits,
) -> Result<()> {
    compile_normalization_inputs(policy, mappings, definitions, limits)?;
    validate_normalization_queries(queries, limits)
}

pub(crate) fn validate_normalization_queries(
    queries: &[ImportQueryTemplate],
    limits: NormalizationLimits,
) -> Result<()> {
    if queries.len() > limits.draft.input.max_collection_entries
        || queries.iter().map(|q| &q.id).collect::<BTreeSet<_>>().len() != queries.len()
    {
        return Err(NormalizationError::Policy("query list"));
    }
    if queries.iter().any(|q| {
        !matches!(
            q.metric,
            ExternalSelector::Catalog {
                kind: ExternalCatalogKind::Metric,
                ..
            }
        )
    }) {
        return Err(NormalizationError::Policy("query metric domain"));
    }
    Ok(())
}

/// One deterministic, fresh import. All source alternatives survive. Definition
/// identities can be known while intrinsic values/effects/roles remain pending.
/// No mutable registry, evaluator, UI or legacy selected-view API is accepted.
pub fn normalize_fresh<I: DefinitionSchemaIndex>(
    evidence: &SourceProjectEvidence<'_>,
    allocator_before: InstanceAllocatorState,
    artifacts: NormalizationArtifacts<'_, I>,
    policy: &NormalizationPolicy,
    queries: &[ImportQueryTemplate],
    limits: NormalizationLimits,
) -> Result<NormalizedImport> {
    let NormalizationArtifacts {
        mappings,
        registry,
        definitions,
        roles,
        rewards,
        items,
        item_source,
        tree,
    } = artifacts;
    let CompiledNormalizationInputs {
        recipes,
        equipment_rules,
        gem_quality,
    } = compile_normalization_inputs(policy, mappings, definitions, limits)?;
    rewards.verify_bindings(mappings, definitions)?;
    items.verify_bindings(definitions)?;
    item_source.verify_bindings(items, definitions)?;
    if let Some(tree) = tree {
        tree.verify_bindings(registry, definitions, mappings, policy)?;
    }
    let policy_digest = digest_owned(
        "owned-normalization-policy-v3",
        &(policy, queries),
        limits.max_policy_bytes,
    )?;
    let identity = evidence.identity();
    if allocator_before.lineage() != identity.lineage
        || allocator_before.last_issued() < identity.allocator.last_issued()
    {
        return Err(NormalizationError::AllocatorBinding);
    }
    if roles.input().mapping != *mappings.identity()
        || roles.input().definitions != *definitions.identity()
    {
        return Err(NormalizationError::Binding);
    }
    mappings.verify_bindings(
        registry,
        definitions,
        &mappings.input().source,
        &mappings.input().policy_version,
        limits.mapping,
    )?;
    validate_normalization_queries(queries, limits)?;
    let root = evidence.rows()[0].occurrence().id();
    let mut b = Builder {
        evidence,
        mappings,
        roles,
        rewards,
        items,
        item_source,
        item_texts: vec![],
        allocator: InstanceAllocator::from_state(allocator_before),
        limits,
        work: 0,
        links: 0,
        issues: 0,
        origins: vec![],
        attributes: vec![],
    };
    b.charge(evidence.rows().len())?;
    for row in evidence.rows() {
        b.charge(row.attributes().len())?;
        let mut attrs = BTreeMap::new();
        for (index, a) in row.attributes().iter().enumerate() {
            if a.origin().namespace.is_none() {
                if matches!(
                    a.origin().name.as_str(),
                    "gemId"
                        | "variantId"
                        | "source"
                        | "itemId"
                        | "name"
                        | "classInternalId"
                        | "ascendancyInternalId"
                        | "treeVersion"
                ) && a.raw().len() > limits.mapping.max_string_bytes
                {
                    return Err(NormalizationError::Limit("source selector bytes"));
                }
                attrs.insert(a.origin().name.as_str(), (index as u32, a));
            }
        }
        b.attributes.push(attrs);
    }
    b.origins = evidence
        .rows()
        .iter()
        .map(|row| SourceOwnedOrigin {
            source: row.occurrence().id(),
            disposition: SourceDisposition::Contributes,
            links: vec![],
        })
        .collect();
    let mut draft = DraftSessionInput {
        allocator: allocator_before,
        revision: identity.revision,
        game_version: policy.namespace.clone(),
        weapon_loadouts: b.closure(root, "loadout-policy-not-converted", vec![])?,
        items: complete(vec![]),
        gems: complete(vec![]),
        rewards: b.closure(root, "rewards-not-converted", vec![])?,
        equipment: complete(vec![]),
        allocations: complete(vec![]),
        skills: complete(vec![]),
        supports: complete(vec![]),
        payload_links: complete(vec![]),
        character_presets: complete(vec![]),
        equipment_presets: complete(vec![]),
        allocation_presets: complete(vec![]),
        skill_presets: complete(vec![]),
        choice_presets: complete(vec![]),
        scenario_presets: complete(vec![]),
        query_presets: complete(vec![]),
        saved_variants: b.closure(root, "saved-selection-not-converted", vec![])?,
    };
    let mut item_ids = BTreeMap::new();
    let mut equipment_sets = BTreeMap::new();
    let mut skill_sets = BTreeMap::new();
    let mut spec_sets = BTreeMap::new();
    let mut spec_characters = BTreeMap::new();
    let mut fallback_issues = vec![];
    let builds: Vec<_> = evidence
        .sections(SourceSectionKind::Build)
        .iter()
        .map(|s| &evidence.rows()[s.ordinal() as usize])
        .collect();
    let build_ids: BTreeSet<_> = builds.iter().map(|r| r.occurrence().id()).collect();
    // Base records and independent presets, in source order. Source-authored IDs
    // are only correspondence: every owned occurrence is newly allocated.
    for row in evidence.rows() {
        let s = row.occurrence().id();
        b.charge(1)?;
        match row.authored_instance() {
            Some(AuthoredInstanceId::ItemRecord(_)) => {
                let id = b.id()?;
                item_ids.insert(s, id);
                b.link(s, OwnedOriginTarget::Item(id))?;
                draft
                    .items
                    .members
                    .push(items::normalize_item(&mut b, row, id)?);
            }
            Some(AuthoredInstanceId::ItemSet(_)) => {
                let id = b.id()?;
                b.link(s, OwnedOriginTarget::EquipmentPreset(id))?;
                equipment_sets.insert(s, draft.equipment_presets.members.len());
                draft.equipment_presets.members.push(EquipmentPresetDraft {
                    id,
                    equipment: b.closure(s, "equipment-membership-not-converted", vec![])?,
                });
            }
            Some(AuthoredInstanceId::SkillSet(_)) => {
                let id = b.id()?;
                b.link(s, OwnedOriginTarget::SkillPreset(id))?;
                skill_sets.insert(s, draft.skill_presets.members.len());
                draft.skill_presets.members.push(SkillPresetDraft {
                    id,
                    skills: b.closure(s, "skill-membership-not-converted", vec![])?,
                    supports: b.closure(s, "support-membership-not-converted", vec![])?,
                    payload_links: b.closure(s, "payload-membership-not-converted", vec![])?,
                });
            }
            Some(AuthoredInstanceId::PassiveSpec(_)) => {
                let id = b.id()?;
                b.link(s, OwnedOriginTarget::AllocationPreset(id))?;
                spec_sets.insert(s, draft.allocation_presets.members.len());
                draft
                    .allocation_presets
                    .members
                    .push(AllocationPresetDraft {
                        id,
                        allocations: if tree.is_some() {
                            complete(vec![])
                        } else {
                            b.closure(s, "allocation-membership-not-converted", vec![])?
                        },
                        equipment: b.closure(
                            s,
                            "allocation-equipment-membership-not-converted",
                            vec![],
                        )?,
                    });
                let id = b.id()?;
                b.link(s, OwnedOriginTarget::CharacterPreset(id))?;
                let (class_field, ascendancy) = if let Some(tree) = tree {
                    tree::character_fields(&mut b, row, tree)?
                } else {
                    let class = component(&b, row, "classInternalId");
                    let class_field = b.mapped(
                        s,
                        class.clone().map(|key| {
                            ExternalSelector::Definition(ExternalOwnerSelector::Class { key })
                        }),
                        |v| match v {
                            DefinitionAddress::Class(id) => Some(id.clone()),
                            _ => None,
                        },
                    )?;
                    let asc = class.zip(component(&b, row, "ascendancyInternalId")).map(
                        |(class, key)| {
                            ExternalSelector::Definition(ExternalOwnerSelector::Ascendancy {
                                class,
                                key,
                            })
                        },
                    );
                    let ascendancy = b.mapped(s, asc, |v| match v {
                        DefinitionAddress::Ascendancy(id) => Some(Some(id.clone())),
                        _ => None,
                    })?;
                    (class_field, ascendancy)
                };
                let level = b.level(
                    if builds.len() == 1 {
                        Some(builds[0])
                    } else {
                        None
                    },
                    s,
                    &recipes[0],
                )?;
                let rewards = b.closure(s, "character-rewards-not-converted", vec![])?;
                if let DraftListCompletion::Pending { id, .. } = rewards.completion {
                    fallback_issues.push(id);
                }
                spec_characters.insert(s, draft.character_presets.members.len());
                draft.character_presets.members.push(CharacterPresetDraft {
                    id,
                    class: class_field,
                    ascendancy,
                    level,
                    rewards,
                });
            }
            Some(AuthoredInstanceId::ConfigSet(_)) => {
                add_config(&mut b, &mut draft, s, &mut fallback_issues)?;
            }
            _ => {}
        }
    }
    if draft.choice_presets.members.is_empty() {
        add_config(&mut b, &mut draft, root, &mut fallback_issues)?;
    }
    // Only actually observed, unique ordinary slots prove equipment scopes and
    // named loadout members. Empty item references still expose a slot; no item
    // identity, saved active selector or source-name heuristic supplies a default.
    let mut equipment_scopes = BTreeMap::new();
    let mut loadout_ids = BTreeMap::new();
    let mut slot_uniqueness = BTreeMap::new();
    let source_root = evidence.rows()[0].occurrence();
    if source_root.name() == "PathOfBuilding2" && !source_root.has_namespace_context() {
        for row in evidence.rows() {
            b.charge(1)?;
            let occurrence = row.occurrence();
            if occurrence.name() != "Slot"
                || occurrence.has_namespace_context()
                || !matches!(
                    row.authored_instance(),
                    Some(AuthoredInstanceId::ItemSlotUse(_))
                )
            {
                continue;
            }
            let Some(parent) = occurrence
                .parent()
                .filter(|p| equipment_sets.contains_key(p))
            else {
                continue;
            };
            let Some(Ok(name)) = row.attribute("name").map(|attribute| attribute.decoded()) else {
                continue;
            };
            let Some(rule) = equipment_rules.get(name) else {
                continue;
            };
            let unique = if let Some(unique) = slot_uniqueness.get(&(parent, name)) {
                *unique
            } else {
                // Conservative charge bounds candidate allocation in the indexed lookup,
                // including duplicate and lexically unavailable same-scope keys.
                b.charge(evidence.rows()[parent.ordinal() as usize].children().len())?;
                let unique = matches!(
                    evidence.lookup_key(SourceKeyQuery {
                        parent: Some(parent),
                        element: SourceQName {
                            namespace: None,
                            local: "Slot"
                        },
                        attribute: SourceQName {
                            namespace: None,
                            local: "name"
                        },
                        value: name,
                    }),
                    Ok(SourceKeyLookup::Unique(_))
                );
                slot_uniqueness.insert((parent, name), unique);
                unique
            };
            if !unique {
                continue;
            }
            let scope = match &rule.scope {
                ImportEquipmentScope::Shared => LoadoutScope::Shared,
                ImportEquipmentScope::Selected { loadouts } => {
                    b.charge(loadouts.len())?;
                    let mut ids = Vec::with_capacity(loadouts.len());
                    for key in loadouts {
                        let id = if let Some(id) = loadout_ids.get(key) {
                            *id
                        } else {
                            let id = b.id()?;
                            loadout_ids.insert(key.clone(), id);
                            draft.weapon_loadouts.members.push(id);
                            id
                        };
                        b.link(
                            occurrence.id(),
                            OwnedOriginTarget::WeaponLoadout {
                                key: key.clone(),
                                id,
                            },
                        )?;
                        ids.push(id);
                    }
                    ids.sort();
                    LoadoutScope::Selected { loadouts: ids }
                }
            };
            equipment_scopes.insert(occurrence.id(), scope);
        }
    }
    // Receiving occurrences stay distinct and follow their owning independent
    // preset. Composition selects the union; no stock/copy claim is made.
    for row in evidence.rows() {
        let s = row.occurrence().id();
        b.charge(1)?;
        if !matches!(
            row.authored_instance(),
            Some(AuthoredInstanceId::ItemSlotUse(_))
        ) {
            continue;
        }
        let item_key = component(&b, row, "itemId");
        b.charge(policy.empty_item_keys.len())?;
        if item_key
            .as_ref()
            .is_some_and(|v| policy.empty_item_keys.contains(v))
        {
            b.origins[s.ordinal() as usize].disposition =
                SourceDisposition::SourceOnly(key("explicit-empty-equipment-use"));
            continue;
        }
        let id = b.id()?;
        b.link(s, OwnedOriginTarget::Equipment(id))?;
        let items_parent = b.ancestor(s, "Items")?.or_else(|| {
            let sections = evidence.sections(SourceSectionKind::Items);
            if sections.len() == 1 {
                Some(sections[0])
            } else {
                None
            }
        });
        let joined = match (&item_key, items_parent) {
            (Some(SourceComponent::Text(value)), Some(parent)) => {
                match evidence.lookup_key(SourceKeyQuery {
                    parent: Some(parent),
                    element: SourceQName {
                        namespace: None,
                        local: "Item",
                    },
                    attribute: SourceQName {
                        namespace: None,
                        local: "id",
                    },
                    value,
                }) {
                    Ok(SourceKeyLookup::Unique(a)) => item_ids.get(&a.occurrence).copied(),
                    _ => None,
                }
            }
            _ => None,
        };
        let item = if let Some(item) = joined {
            b.link(s, OwnedOriginTarget::ItemReference(item))?;
            item.into()
        } else {
            b.pending(s, "item-reference-unresolved")?
        };
        let destination = if row.occurrence().name() == "Slot" {
            DraftEquipmentDestination::CharacterSlot(
                b.mapped(
                    s,
                    component(&b, row, "name")
                        .map(|key| catalog(ExternalCatalogKind::EquipmentSlot, key)),
                    |v| match v {
                        DefinitionAddress::EquipmentSlot(id) => Some(id.clone()),
                        _ => None,
                    },
                )?,
            )
        } else {
            DraftEquipmentDestination::Pending(PendingValue {
                id: b.issue(s)?,
                code: key("socket-destination-not-converted"),
                candidates: vec![],
            })
        };
        draft.equipment.members.push(EquipmentDraft {
            id,
            item,
            destination,
            scope: match equipment_scopes.remove(&s) {
                Some(scope) => scope.into(),
                None => b.pending(s, "equipment-scope-not-converted")?,
            },
        });
        if let Some(parent) = b.ancestor(s, "ItemSet")?
            && let Some(index) = equipment_sets.get(&parent)
        {
            draft.equipment_presets.members[*index]
                .equipment
                .members
                .push(id);
        } else if let Some(parent) = b.ancestor(s, "Spec")?
            && let Some(index) = spec_sets.get(&parent)
        {
            draft.allocation_presets.members[*index]
                .equipment
                .members
                .push(id);
        }
    }
    if let Some(tree) = tree {
        tree::allocations(
            &mut b,
            &mut draft,
            tree::TreeContext {
                specs: &spec_sets,
                characters: &spec_characters,
                loadouts: &loadout_ids,
                definitions,
                tree,
                base: policy,
            },
        )?;
    } else {
        // Every nonempty authored node token is retained. Malformed empty tokens and
        // undecodable lists retain the real preset membership obligation. Pool, loadout overlay, attribute
        // choice and special access require owned definitions; disconnected != illegal.
        for (s, index) in &spec_sets {
            let row = &evidence.rows()[s.ordinal() as usize];
            if let Some(Ok(nodes)) = row
                .attribute(&policy.allocation_attribute)
                .map(|v| v.decoded())
            {
                for node in nodes.split(',') {
                    b.charge(1)?;
                    if node.is_empty() {
                        continue;
                    }
                    if node.len() > limits.mapping.max_string_bytes {
                        return Err(NormalizationError::Limit("node selector bytes"));
                    }
                    b.charge(row.attributes().len())?;
                    let id = b.id()?;
                    b.link(*s, OwnedOriginTarget::Allocation(id))?;
                    let selector = component(&b, row, "treeVersion").map(|tree_version| {
                        ExternalSelector::Definition(ExternalOwnerSelector::PassiveNode {
                            tree_version,
                            node_id: SourceComponent::Text(node.into()),
                            view: SourceComponent::Missing,
                        })
                    });
                    draft.allocations.members.push(AllocationDraft {
                        id,
                        node: b.mapped(*s, selector, |v| match v {
                            DefinitionAddress::PassiveNode(id) => Some(id.clone()),
                            _ => None,
                        })?,
                        pool: b.pending(*s, "point-pool-not-converted")?,
                        scope: b.pending(*s, "allocation-scope-not-converted")?,
                        access: DraftAllocationAccess::Pending(PendingValue {
                            id: b.issue(*s)?,
                            code: key("allocation-access-not-converted"),
                            candidates: vec![],
                        }),
                        choices: b.closure(*s, "allocation-choices-not-converted", vec![])?,
                    });
                    draft.allocation_presets.members[*index]
                        .allocations
                        .members
                        .push(id);
                }
            }
        }
    }
    // Physical manual gems are distinct from generated representations. Exact
    // role metadata selects typed SkillUse or SupportAssignment, never a name.
    let mut group_skills: BTreeMap<SourceOccurrenceId, Vec<SkillUseId>> = BTreeMap::new();
    let mut group_sources = BTreeMap::new();
    let mut unresolved_groups = BTreeSet::new();
    let mut support_rows = vec![];
    for row in evidence.rows() {
        if matches!(
            row.authored_instance(),
            Some(AuthoredInstanceId::SkillGroup(_))
        ) {
            group_sources.insert(row.occurrence().id(), component(&b, row, "source"));
            for child in row.children() {
                b.charge(1)?;
                if !matches!(
                    evidence.rows()[child.ordinal() as usize].authored_instance(),
                    Some(AuthoredInstanceId::SkillEntry(_))
                ) {
                    unresolved_groups.insert(row.occurrence().id());
                }
            }
        }
    }
    for row in evidence.rows() {
        let s = row.occurrence().id();
        b.charge(1)?;
        if !matches!(
            row.authored_instance(),
            Some(AuthoredInstanceId::SkillEntry(_))
        ) {
            continue;
        }
        let Some(group_id) = b.ancestor(s, "Skill")? else {
            continue;
        };
        let group = &evidence.rows()[group_id.ordinal() as usize];
        b.charge(policy.manual_skill_sources.len())?;
        let source = group_sources.get(&group_id).and_then(|v| v.as_ref());
        let manual = source.is_some_and(|v| policy.manual_skill_sources.contains(v));
        let selector = component(&b, row, "gemId")
            .zip(component(&b, row, "variantId"))
            .map(|(game_id, variant_id)| {
                ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                    game_id,
                    variant_id,
                })
            });
        let catalog_row = selector
            .as_ref()
            .and_then(|selector| b.roles.lookup(selector))
            .and_then(|outcome| match outcome {
                MappingOutcome::Mapped {
                    target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
                    ..
                } => b.roles.role(gem),
                _ => None,
            });
        let role = catalog_row.and_then(|row| match row.role {
            OwnedGemRole::Known(role) => Some(role),
            _ => None,
        });
        let physical = catalog_row
            .is_some_and(|row| matches!(row.materialization, OwnedGemMaterialization::Physical));
        if role.is_none() {
            unresolved_groups.insert(group_id);
        }
        // Source Gem rows also encode provider-granted abilities. A manual
        // group and a gemId cannot prove physical ownership: require separately
        // compiled materialization evidence before creating any owned instance.
        // Reviewed physical supports on generated targets keep targets pending.
        let physical_id = component(&b, row, "gemId")
            .is_some_and(|v| matches!(v,SourceComponent::Text(id) if !id.is_empty()));
        b.charge(policy.generated_support_prefixes.len())?;
        let generated_support = source.is_some_and(|v| match v {
            SourceComponent::Text(value) => policy
                .generated_support_prefixes
                .iter()
                .any(|prefix| value.starts_with(prefix)),
            _ => false,
        }) && role == Some(AuthoredGemRole::SupportAssignment);
        if !physical || !physical_id || (!manual && !generated_support) {
            unresolved_groups.insert(group_id);
            continue;
        }
        let gem_id = b.id()?;
        b.link(s, OwnedOriginTarget::Gem(gem_id))?;
        let gem_definition = b.mapped(s, selector, |v| match v {
            DefinitionAddress::Gem(id) => Some(id.clone()),
            _ => None,
        })?;
        let gem = GemDraft {
            id: gem_id,
            quality: b.gem_quality(row, &gem_definition, gem_quality.as_ref(), definitions)?,
            definition: gem_definition,
            parameters: b.closure(s, "gem-parameters-not-converted", vec![])?,
            level: b.level(Some(row), s, &recipes[1])?,
        };
        draft.gems.members.push(gem);
        let preset = b
            .ancestor(s, "SkillSet")?
            .and_then(|s| skill_sets.get(&s).copied());
        match role {
            Some(AuthoredGemRole::SkillUse) if manual => {
                let id = b.id()?;
                b.link(s, OwnedOriginTarget::Skill(id))?;
                draft.skills.members.push(SkillDraft {
                    id,
                    source: DraftAuthoredSkillSource::Gem(gem_id.into()),
                    enabled: b.enabled(row, group, &recipes[2], &recipes[3])?,
                    scope: b.pending(s, "skill-scope-not-converted")?,
                });
                group_skills.entry(group_id).or_default().push(id);
                if let Some(preset) = preset {
                    draft.skill_presets.members[preset].skills.members.push(id);
                }
            }
            Some(AuthoredGemRole::SupportAssignment) => {
                support_rows.push((s, group_id, gem_id, preset, manual));
            }
            _ => {}
        }
    }
    for (s, group_id, gem, preset, manual) in support_rows {
        let row = &evidence.rows()[s.ordinal() as usize];
        let group = &evidence.rows()[group_id.ordinal() as usize];
        let id = b.id()?;
        b.link(s, OwnedOriginTarget::Support(id))?;
        let target = if manual
            && !unresolved_groups.contains(&group_id)
            && policy.single_active_support_target
            && group_skills.get(&group_id).is_some_and(|v| v.len() == 1)
        {
            DraftSkillTarget::Authored(group_skills[&group_id][0].into())
        } else {
            DraftSkillTarget::Pending(PendingValue {
                id: b.issue(s)?,
                code: key("support-target-not-converted"),
                candidates: vec![],
            })
        };
        draft.supports.members.push(SupportDraft {
            id,
            support: gem.into(),
            target,
            enabled: b.enabled(row, group, &recipes[2], &recipes[3])?,
        });
        if let Some(preset) = preset {
            draft.skill_presets.members[preset]
                .supports
                .members
                .push(id);
        }
    }
    let query_id = b.id()?;
    b.link(root, OwnedOriginTarget::QueryPreset(query_id))?;
    let mut requests = vec![];
    for q in queries {
        b.charge(1)?;
        let metric = b.mapped(root, Some(q.metric.clone()), |v| match v {
            DefinitionAddress::Metric(id) => Some(id.clone()),
            _ => None,
        })?;
        let target = match &q.target {
            ImportQueryTarget::Player => DraftMetricTarget::Actor(DraftActorKey::Player),
            ImportQueryTarget::Unresolved(code) => DraftMetricTarget::Pending(PendingValue {
                id: b.issue(root)?,
                code: code.clone(),
                candidates: vec![],
            }),
        };
        requests.push(MetricRequestDraft {
            id: q.id.clone(),
            metric,
            target,
        });
    }
    draft.query_presets.members.push(QueryPresetDraft {
        id: query_id,
        queries: QueryDraft {
            game_version: policy.namespace.clone(),
            requests: complete(requests),
        },
    });
    // Unknown source semantics cannot become "UI-only" by default. They point to
    // real selected character/choice obligations until individually converted.
    for row in evidence.rows() {
        let s = row.occurrence().id();
        if !b.origins[s.ordinal() as usize].links.is_empty()
            || matches!(
                b.origins[s.ordinal() as usize].disposition,
                SourceDisposition::SourceOnly(_)
            )
        {
            continue;
        }
        let cached = matches!(row.occurrence().name(), "PlayerStat" | "MinionStat")
            && !row.occurrence().has_namespace_context()
            && row
                .occurrence()
                .parent()
                .is_some_and(|p| build_ids.contains(&p));
        if cached {
            b.origins[s.ordinal() as usize].disposition =
                SourceDisposition::SourceOnly(key("cached-reference-output"));
        } else {
            for issue in &fallback_issues {
                b.link(s, OwnedOriginTarget::Issue(*issue))?;
            }
        }
    }
    draft.allocator = b.allocator.state();
    let draft = DraftSession::new(draft, limits.draft)?;
    let sidecar = FreshNormalizationSidecar {
        schema_version: 9,
        source_sha256: identity.source_sha256.into(),
        source_bytes: identity.source_bytes,
        source_schema: identity.instance_import_schema,
        revision: identity.revision,
        allocator_before,
        allocator_after: b.allocator.state(),
        source_allocator: identity.allocator,
        policy: policy_digest,
        mapping: *mappings.identity(),
        mapping_source: *mappings.source_identity(),
        registry: registry.identity()?,
        definitions: definitions.identity().clone(),
        skill_roles: *roles.identity(),
        reward_policy: *rewards.identity(),
        item_policy: *items.identity(),
        item_source_policy: *item_source.identity(),
        tree_policy: tree.map(|tree| *tree.identity()),
        item_texts: b.item_texts,
        draft: draft.digest(limits.draft.input.max_wire_bytes)?,
        origins: b.origins,
    };
    // Bound the evidence artifact too; nothing is returned on a late failure.
    digest_owned(
        "owned-normalization-sidecar-v9",
        &sidecar,
        limits.draft.input.max_wire_bytes,
    )?;
    Ok(NormalizedImport { draft, sidecar })
}
fn add_config(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    s: SourceOccurrenceId,
    fallback: &mut Vec<DraftIssueId>,
) -> Result<()> {
    let id = b.id()?;
    b.link(s, OwnedOriginTarget::ChoicePreset(id))?;
    let choices = b.closure(s, "configuration-roles-not-converted", vec![])?;
    if let DraftListCompletion::Pending { id, .. } = choices.completion {
        fallback.push(id);
    }
    let mut rewards = b.closure(s, "configuration-rewards-not-converted", vec![])?;
    add_rewards(b, draft, s, &mut rewards)?;
    draft.choice_presets.members.push(ChoicePresetDraft {
        id,
        choices,
        rewards,
    });
    let id = b.id()?;
    b.link(s, OwnedOriginTarget::ScenarioPreset(id))?;
    draft.scenario_presets.members.push(ScenarioPresetDraft {
        id,
        scenario: ScenarioDraft {
            game_version: draft.game_version.clone(),
            enemy: EnemyDraft {
                encounter: b.pending(s, "encounter-not-converted")?,
                level: b.pending(s, "enemy-level-not-converted")?,
            },
            assumptions: b.closure(s, "external-assumptions-not-converted", vec![])?,
            usage: b.closure(s, "usage-not-converted", vec![])?,
        },
    });
    Ok(())
}

/// Read only one exact saved configuration. Finite injected recipes supply game
/// keys, defaults and outcomes; this adapter only recognizes source syntax.
/// The policy contributes known rewards but never closes the reward catalog.
fn add_rewards(
    b: &mut Builder<'_, '_>,
    draft: &mut DraftSessionInput,
    source: SourceOccurrenceId,
    output: &mut DraftList<RewardSelectionId>,
) -> Result<()> {
    let DraftListCompletion::Pending { id: obligation, .. } = output.completion else {
        return Err(NormalizationError::Policy(
            "reward catalog must remain partial",
        ));
    };
    if b.rewards.rules().len() == 0 {
        return Ok(());
    }
    let evidence = b.evidence;
    let row = &evidence.rows()[source.ordinal() as usize];
    let scope = if row.occurrence().name() == "ConfigSet" {
        Some(row)
    } else {
        // Legacy direct Config can be read only if it is the unique root scope.
        let configs = row
            .children()
            .iter()
            .filter_map(|id| {
                let child = &evidence.rows()[id.ordinal() as usize];
                (child.occurrence().name() == "Config"
                    && !child.occurrence().has_namespace_context())
                .then_some(child)
            })
            .collect::<Vec<_>>();
        b.charge(row.children().len())?;
        if configs.len() == 1 {
            Some(configs[0])
        } else {
            None
        }
    };
    let Some(scope) = scope else {
        return Ok(());
    };
    let mut unknown_scope = false;
    let mut inputs: BTreeMap<&str, Vec<&SourceEvidenceRow<'_>>> = BTreeMap::new();
    for id in scope.children() {
        b.charge(1)?;
        let child = &evidence.rows()[id.ordinal() as usize];
        b.link(*id, OwnedOriginTarget::Issue(obligation))?;
        if child.occurrence().has_namespace_context() {
            unknown_scope = true;
            continue;
        }
        match child.occurrence().name() {
            "Input" | "Placeholder" => {
                match child.attribute("name").and_then(|v| v.decoded().ok()) {
                    Some(name) if !name.is_empty() => inputs.entry(name).or_default().push(child),
                    _ => unknown_scope = true,
                }
            }
            // These source records declare modifiers, never scalar inputs.
            "CustomModifierBlock" => {}
            _ => unknown_scope = true,
        }
    }
    if unknown_scope {
        return Ok(());
    }
    let policy = b.rewards;
    for rule in policy.rules() {
        b.charge(1)?;
        let mut candidates = vec![];
        let mut shape_pending = false;
        let admitted: BTreeSet<_> = rule
            .recipe
            .tiers
            .iter()
            .flat_map(|t| &t.selectors)
            .map(|s| (s.name.as_str(), s.lane))
            .collect();
        b.charge(admitted.len())?;
        for selector in rule.recipe.tiers.iter().flat_map(|tier| &tier.selectors) {
            b.charge(1)?;
            let attribute = match selector.lane {
                ValueLane::InputBoolean => "boolean",
                ValueLane::InputString => "string",
                _ => return Err(NormalizationError::Policy("unsupported reward value lane")),
            };
            if let Some(rows) = inputs.get(selector.name.as_str()) {
                for row in rows {
                    b.charge(1)?;
                    let attrs = &b.attributes[row.occurrence().id().ordinal() as usize];
                    // A present wrong-shaped value must not be mistaken for
                    // absence and activate a missing-value default. Placeholder
                    // writes/precedence are deliberately not inferred here.
                    let actual_lane = if attrs.contains_key("boolean") {
                        Some(ValueLane::InputBoolean)
                    } else if attrs.contains_key("string") {
                        Some(ValueLane::InputString)
                    } else {
                        None
                    };
                    if row.occurrence().name() != "Input"
                        || !row.children().is_empty()
                        || row.attributes().len() != 2
                        || attrs.len() != 2
                        || !actual_lane
                            .is_some_and(|lane| admitted.contains(&(selector.name.as_str(), lane)))
                    {
                        shape_pending = true;
                        continue;
                    }
                    // Multiple admitted typed lanes for one name remain distinct
                    // candidates. Another tier's lane does not invalidate this row.
                    if actual_lane != Some(selector.lane) {
                        continue;
                    }
                    if candidates.len() >= b.limits.value.max_candidates {
                        return Err(NormalizationError::Limit("reward candidates"));
                    }
                    let (index, value) = attrs[attribute];
                    candidates.push(ValueCandidate {
                        selector,
                        origin: SourceAttributeRef {
                            occurrence: row.occurrence().id(),
                            index,
                        },
                        value: match value.decoded() {
                            Ok(v) => CandidateValue::Decoded(v),
                            Err(error) => CandidateValue::Unavailable(error),
                        },
                    });
                }
            }
        }
        if shape_pending {
            continue;
        }
        let decision = policy.decide(&rule.recipe.id, &candidates)?;
        if let RewardOutcome::Reward {
            definition,
            parameters,
        } = decision.outcome
        {
            b.charge(parameters.len())?;
            let id = b.id()?;
            b.link(source, OwnedOriginTarget::Reward(id))?;
            if let ValueOutcome::Selected { origin, .. } = decision.value.outcome {
                b.link(origin.occurrence, OwnedOriginTarget::Reward(id))?;
            }
            output.members.push(id);
            draft.rewards.members.push(RewardDraft {
                id,
                definition: DraftField::from(definition),
                parameters: complete(parameters.into_iter().map(Into::into).collect()),
            });
        }
    }
    Ok(())
}
