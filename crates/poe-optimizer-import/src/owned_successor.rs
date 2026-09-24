//! Explicit offline package succession. This is not runtime binding repair.
//! Prior import facts are validated before rebind; successor mechanics are fully
//! constructed and compiled. Changes to existing declarations require a typed,
//! endpoint-bound refinement; ordinary succession preserves declarations exactly.
use crate::{
    owned_item_lines::{ItemLineError, ItemLineLimits, ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{
        ItemSourceError, ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput, ItemSourceLimits,
    },
    owned_mapping::{
        MappingEntry, MappingPackageInput, OwnedMappingError, OwnedMappingIndex, SourcePin,
    },
    owned_normalize::{
        GemQualityPolicy, ImportQueryTemplate, NormalizationError, NormalizationLimits,
        NormalizationPolicy, validate_normalization_inputs, validate_normalization_queries,
    },
    owned_recipe::{
        OwnedRecipeError, OwnedRecipeInput, OwnedRecipeLimits, StagedOwnedRecipe,
        assemble_owned_recipe,
    },
    owned_reward_policy::{
        OwnedRewardPolicy, RewardPolicyError, RewardPolicyInput, RewardPolicyLimits,
    },
    owned_skill_catalog::{
        OwnedSkillRoleIndex, OwnedSkillRolePackageInput, SkillCatalogError, SkillCatalogLimits,
    },
    owned_tree_policy::{
        OwnedTreeNormalizationPolicy, TreeNormalizationContent, TreeNormalizationPackageInput,
        TreePolicyError, TreePolicyLimits,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::{ClassDefId, OwnedDefinitionKey, PassiveNodeDefId},
    owned_schema::{
        DeclaredSlots, DefinitionAddress, DefinitionDescriptor, DefinitionSchemaIndex,
        SchemaClosure, SchemaLookup, SchemaState,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

#[path = "owned_schema_membership.rs"]
mod membership;
pub use membership::SchemaMembershipRefinement;
#[path = "owned_schema_gems.rs"]
mod gems;
pub use gems::GemSchemaRefinement;

pub const OWNED_SUCCESSOR_VERSION: u32 = 1;
/// Compact publication envelope; the typed authoring input remains version 1.
pub const OWNED_COMPACT_SUCCESSOR_VERSION: u32 = 2;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PublicationFormat {
    LegacyV1,
    CompactV2,
}

/// Explicit authoring assertion that these passive owners have no further ports.
/// Only closure metadata may change: every known member, physical identity, pool,
/// adjacency and slot descriptor is preserved. Rule coverage remains independent.
/// Whole-schema identities bind the assertion to exact reviewed endpoints.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PassiveDeclarationRefinement {
    pub schema_version: u32,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub nodes: Vec<PassiveNodeDefId>,
}

/// The closed set of owners whose input-port closure may be reviewed here.
/// Numerical coverage and every field outside `declarations` remain independent.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DeclarationRefinementOwner {
    PassiveNode(PassiveNodeDefId),
    Class(ClassDefId),
}
impl DeclarationRefinementOwner {
    pub fn address(&self) -> DefinitionAddress {
        match self {
            Self::PassiveNode(id) => DefinitionAddress::PassiveNode(id.clone()),
            Self::Class(id) => DefinitionAddress::Class(id.clone()),
        }
    }
}
/// V2 generalizes reviewed input-port closure without changing the V1 wire format.
/// Both endpoints bind the complete schemas, including all unchanged fields.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclarationClosureRefinement {
    pub schema_version: u32,
    pub before: DataIdentity,
    pub after: DataIdentity,
    pub owners: Vec<DeclarationRefinementOwner>,
}
/// Untagged to preserve published V1/V2 bytes. The disjoint `nodes`, `owners`
/// and `subjects` fields reject mixed shapes; validation checks exact versions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SchemaDeclarationRefinement {
    LegacyPassiveV1(PassiveDeclarationRefinement),
    OwnersV2(DeclarationClosureRefinement),
    MembershipV3(SchemaMembershipRefinement),
    GemsV4(GemSchemaRefinement),
}
impl From<PassiveDeclarationRefinement> for SchemaDeclarationRefinement {
    fn from(value: PassiveDeclarationRefinement) -> Self {
        Self::LegacyPassiveV1(value)
    }
}
impl From<DeclarationClosureRefinement> for SchemaDeclarationRefinement {
    fn from(value: DeclarationClosureRefinement) -> Self {
        Self::OwnersV2(value)
    }
}
impl From<SchemaMembershipRefinement> for SchemaDeclarationRefinement {
    fn from(value: SchemaMembershipRefinement) -> Self {
        Self::MembershipV3(value)
    }
}
impl From<GemSchemaRefinement> for SchemaDeclarationRefinement {
    fn from(value: GemSchemaRefinement) -> Self {
        Self::GemsV4(value)
    }
}
impl SchemaDeclarationRefinement {
    fn len(&self) -> usize {
        match self {
            Self::LegacyPassiveV1(v) => v.nodes.len(),
            Self::OwnersV2(v) => v.owners.len(),
            Self::MembershipV3(v) => v.subjects.len(),
            Self::GemsV4(v) => v.gems.len().saturating_add(v.parameters.len()),
        }
    }
    /// Closure-only owners from the V1/V2 contracts. V3 membership subjects are
    /// intentionally separate and never imply port closure.
    pub fn owners(&self) -> impl Iterator<Item = DeclarationRefinementOwner> + '_ {
        let (nodes, owners): (&[PassiveNodeDefId], &[DeclarationRefinementOwner]) = match self {
            Self::LegacyPassiveV1(v) => (&v.nodes, &[]),
            Self::OwnersV2(v) => (&[], &v.owners),
            Self::MembershipV3(_) | Self::GemsV4(_) => (&[], &[]),
        };
        nodes
            .iter()
            .cloned()
            .map(DeclarationRefinementOwner::PassiveNode)
            .chain(owners.iter().cloned())
    }
    fn validate_endpoints(&self, before: &DataIdentity, after: &DataIdentity) -> Result<()> {
        let (version_ok, old, new) = match self {
            Self::LegacyPassiveV1(v) => (v.schema_version == 1, &v.before, &v.after),
            Self::OwnersV2(v) => (v.schema_version == 2, &v.before, &v.after),
            Self::GemsV4(v) => (
                v.schema_version == 4 && v.before != v.after,
                &v.before,
                &v.after,
            ),
            Self::MembershipV3(v) => (
                v.schema_version == 3 && v.before != v.after,
                &v.before,
                &v.after,
            ),
        };
        if !version_ok || old != before || new != after {
            return Err(SuccessorBundleError::Refinement(
                "version or endpoint binding",
            ));
        }
        if self.len() == 0 {
            return Err(SuccessorBundleError::Refinement("empty policy"));
        }
        Ok(())
    }
    /// Checks manifest consistency against the available current schema only.
    /// This does not authenticate provenance or prove a missing predecessor's history.
    pub fn validate_current_metadata<I: DefinitionSchemaIndex>(
        &self,
        before: &DataIdentity,
        after: &DataIdentity,
        index: &I,
    ) -> Result<()> {
        self.validate_endpoints(before, after)?;
        if after != index.identity() {
            return Err(SuccessorBundleError::Refinement("current endpoint binding"));
        }
        if let Self::MembershipV3(policy) = self {
            return membership::validate_current(policy, index);
        }
        if let Self::GemsV4(policy) = self {
            return gems::validate_current(policy, index);
        }
        let mut seen = BTreeSet::new();
        for owner in self.owners() {
            if !seen.insert(owner.address()) {
                return Err(SuccessorBundleError::Refinement("duplicate owner"));
            }
            let declarations = match &owner {
                DeclarationRefinementOwner::PassiveNode(id) => match index.definition(id) {
                    SchemaLookup::Known(s) => &s.declarations,
                    _ => return Err(SuccessorBundleError::Refinement("unknown current owner")),
                },
                DeclarationRefinementOwner::Class(id) => match index.definition(id) {
                    SchemaLookup::Known(s) => &s.declarations,
                    _ => return Err(SuccessorBundleError::Refinement("unknown current owner")),
                },
            };
            if !declarations_complete(declarations) {
                return Err(SuccessorBundleError::Refinement(
                    "current owner ports are incomplete",
                ));
            }
        }
        Ok(())
    }
}
fn declarations_complete(ports: &DeclaredSlots) -> bool {
    ports.parameters.is_complete()
        && ports.choices.is_complete()
        && ports.grants.is_complete()
        && ports.actors.is_complete()
        && ports.skill_grants.is_complete()
        && ports.outputs.is_complete()
        && ports.sockets.is_complete()
}
fn close_declarations(descriptor: &mut DefinitionDescriptor) -> Result<()> {
    let ports = match descriptor {
        DefinitionDescriptor::PassiveNode(entry) => match &mut entry.schema {
            SchemaState::Known(schema) => &mut schema.declarations,
            _ => return Err(SuccessorBundleError::Refinement("unmapped owner")),
        },
        DefinitionDescriptor::Class(entry) => match &mut entry.schema {
            SchemaState::Known(schema) => &mut schema.declarations,
            _ => return Err(SuccessorBundleError::Refinement("unmapped owner")),
        },
        _ => return Err(SuccessorBundleError::Refinement("unsupported owner")),
    };
    for closure in [
        &mut ports.parameters.closure,
        &mut ports.choices.closure,
        &mut ports.grants.closure,
        &mut ports.actors.closure,
        &mut ports.skill_grants.closure,
        &mut ports.outputs.closure,
        &mut ports.sockets.closure,
    ] {
        *closure = SchemaClosure::Complete;
    }
    Ok(())
}

/// Independent ordered query lists. Names are publication labels, not game IDs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NamedQuerySet {
    pub name: OwnedDefinitionKey,
    pub queries: Vec<ImportQueryTemplate>,
}

/// All bindings are explicit. Mapping, roles, normalization and rewards must
/// initially bind `prior`. Item policies must already bind `successor`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessorBundleInput {
    pub schema_version: u32,
    pub prior: OwnedRecipeInput,
    pub successor: OwnedRecipeInput,
    pub mapping: MappingPackageInput,
    pub roles: OwnedSkillRolePackageInput,
    pub normalization: NormalizationPolicy,
    pub rewards: RewardPolicyInput,
    pub query_sets: Vec<NamedQuerySet>,
    pub items: ItemLinePolicyInput,
    pub item_source: ItemSourceLayoutPolicyInput,
}

/// A catalog may add only new external selectors. Existing selectors, including
/// explicit Unmapped/Ambiguous outcomes, must be handled by a separate edit policy.
/// `source.files` is an additional/overlapping footprint in the same source
/// system and revision. Matching old pins are retained; conflicts are rejected.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CatalogAppend {
    pub mappings: Vec<MappingEntry>,
    pub source: SourcePin,
    pub item_policies: CatalogItemPolicyMode,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CatalogItemPolicyMode {
    /// `SuccessorBundleInput.items/item_source` already bind the successor.
    SuppliedSuccessor,
    /// They must first bind the prior package. Only exact dependency identities
    /// are then changed; source interpretation and provenance stay immutable.
    RebindPrior,
}

/// Tree interpretation is authored explicitly, or carried only after old bindings validate.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TreePolicyTransitionInput {
    Install {
        content: Box<TreeNormalizationContent>,
    },
    RebindPrior {
        prior: Box<TreeNormalizationPackageInput>,
    },
}

/// Aggregate bytes, top-level declarations, rule programs/table rows, routes and
/// queries are bounded across the transition. Nested constructors additionally
/// bound schema members, expression edges, closure gaps and source/value text.
#[derive(Clone, Copy, Debug)]
pub struct SuccessorBundleLimits {
    /// V1 bounds the aggregate typed transition. V2 separately bounds the prior
    /// recipe and the remaining commitment stream (at most twice this total).
    /// The prior recipe additionally obeys `recipe.max_wire_bytes`.
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_validation_entries: usize,
    pub max_query_sets: usize,
    pub max_queries: usize,
    pub recipe: OwnedRecipeLimits,
    pub catalog: SkillCatalogLimits,
    pub normalization: NormalizationLimits,
    pub rewards: RewardPolicyLimits,
    pub items: ItemLineLimits,
    pub item_source: ItemSourceLimits,
    pub tree: TreePolicyLimits,
}
impl Default for SuccessorBundleLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_validation_entries: 2_000_000,
            max_query_sets: 64,
            max_queries: 100_000,
            recipe: OwnedRecipeLimits::default(),
            catalog: SkillCatalogLimits::default(),
            normalization: NormalizationLimits::default(),
            rewards: RewardPolicyLimits::default(),
            items: ItemLineLimits::default(),
            item_source: ItemSourceLimits::default(),
            tree: TreePolicyLimits::default(),
        }
    }
}
impl SuccessorBundleLimits {
    fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("input bytes", self.max_input_bytes, hard.max_input_bytes),
            ("output bytes", self.max_output_bytes, hard.max_output_bytes),
            (
                "validation entries",
                self.max_validation_entries,
                hard.max_validation_entries,
            ),
            ("query sets", self.max_query_sets, hard.max_query_sets),
            ("queries", self.max_queries, hard.max_queries),
        ] {
            if value == 0 || value > maximum {
                return Err(SuccessorBundleError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}
#[derive(Debug, thiserror::Error)]
pub enum SuccessorBundleError {
    #[error("unsupported successor bundle version {0}")]
    Version(u32),
    #[error("invalid successor bundle limit: {0}")]
    InvalidLimit(&'static str),
    #[error("successor bundle exceeds {0}")]
    Limit(&'static str),
    #[error("successor changed existing registry history")]
    ChangedRegistry,
    #[error("successor changed an existing schema declaration")]
    ChangedDeclaration,
    #[error("invalid declaration closure refinement: {0}")]
    Refinement(&'static str),
    #[error(
        "query set name must be unique lowercase ASCII letters, digits, hyphens or underscores, at most 64 bytes"
    )]
    QuerySetName,
    #[error("invalid catalog append: {0}")]
    CatalogConflict(&'static str),
    #[error(transparent)]
    Tree(#[from] TreePolicyError),
    #[error(transparent)]
    Recipe(#[from] OwnedRecipeError),
    #[error(transparent)]
    Mapping(#[from] OwnedMappingError),
    #[error(transparent)]
    Roles(#[from] SkillCatalogError),
    #[error(transparent)]
    Normalization(#[from] NormalizationError),
    #[error(transparent)]
    Rewards(#[from] RewardPolicyError),
    #[error(transparent)]
    Items(#[from] ItemLineError),
    #[error(transparent)]
    ItemSource(#[from] ItemSourceError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, SuccessorBundleError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SuccessorArtifactManifest {
    pub file: String,
    pub bytes: usize,
    pub sha256: String,
}
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SuccessorBindings {
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub mapping: OwnedContentDigest,
    pub roles: OwnedContentDigest,
    pub normalization: OwnedContentDigest,
    pub rewards: OwnedContentDigest,
    pub rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
}
#[derive(Debug, Serialize)]
pub struct SuccessorBundleTransition {
    pub schema_version: u32,
    pub document_kind: &'static str,
    pub input: OwnedContentDigest,
    pub before: SuccessorBindings,
    pub after: SuccessorBindings,
    pub preserved_registry_entries: usize,
    pub preserved_definitions: usize,
    pub preserved_slots: usize,
    pub query_sets: usize,
    pub query_rows: usize,
    pub items: OwnedContentDigest,
    pub item_source: OwnedContentDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tree: Option<OwnedContentDigest>,
    pub schema_policy: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_refinement: Option<SchemaDeclarationRefinement>,
    pub item_policy_mode: &'static str,
    pub source_execution: bool,
    pub calculation: &'static str,
    pub whole_build_parity: &'static str,
    /// Covers every emitted artifact except this transition itself.
    pub artifacts: Vec<SuccessorArtifactManifest>,
}
struct Artifact {
    name: String,
    bytes: Vec<u8>,
}
pub struct StagedSuccessorBundle {
    recipe: OwnedRecipeInput,
    assembled: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    normalization: NormalizationPolicy,
    rewards: OwnedRewardPolicy,
    items: OwnedItemLinePolicy,
    item_source: ItemSourceLayoutPolicy,
    queries: Vec<NamedQuerySet>,
    tree: Option<OwnedTreeNormalizationPolicy>,
    transition: SuccessorBundleTransition,
    extra: Vec<Artifact>,
}
impl StagedSuccessorBundle {
    pub fn recipe(&self) -> &OwnedRecipeInput {
        &self.recipe
    }
    pub fn assembled(&self) -> &StagedOwnedRecipe {
        &self.assembled
    }
    pub fn mapping(&self) -> &OwnedMappingIndex {
        &self.mapping
    }
    pub fn roles(&self) -> &OwnedSkillRoleIndex {
        &self.roles
    }
    pub fn normalization(&self) -> &NormalizationPolicy {
        &self.normalization
    }
    pub fn rewards(&self) -> &OwnedRewardPolicy {
        &self.rewards
    }
    pub fn items(&self) -> &OwnedItemLinePolicy {
        &self.items
    }
    pub fn item_source(&self) -> &ItemSourceLayoutPolicy {
        &self.item_source
    }
    pub fn tree(&self) -> Option<&OwnedTreeNormalizationPolicy> {
        self.tree.as_ref()
    }
    pub fn query_sets(&self) -> &[NamedQuerySet] {
        &self.queries
    }
    pub fn transition(&self) -> &SuccessorBundleTransition {
        &self.transition
    }
    /// Only validated safe basenames; a host must publish to a new destination.
    pub fn artifacts(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.assembled
            .artifacts()
            .iter()
            .map(|a| (a.name(), a.bytes()))
            .chain(
                self.extra
                    .iter()
                    .map(|a| (a.name.as_str(), a.bytes.as_slice())),
            )
    }
}

fn charge(left: &mut usize, count: usize, label: &'static str) -> Result<()> {
    *left = left
        .checked_sub(count)
        .ok_or(SuccessorBundleError::Limit(label))?;
    Ok(())
}
fn preflight(
    input: &SuccessorBundleInput,
    append: Option<&CatalogAppend>,
    limits: SuccessorBundleLimits,
) -> Result<usize> {
    let mut left = limits.max_validation_entries;
    if let Some(append) = append {
        for count in [append.mappings.len(), append.source.files.len()] {
            charge(&mut left, count, "validation entries")?;
            if count > limits.catalog.mapping.max_collection_entries {
                return Err(SuccessorBundleError::Limit("catalog append entries"));
            }
        }
    }
    for recipe in [&input.prior, &input.successor] {
        for n in [
            recipe.registry.entries.len(),
            recipe.schema.definitions.len(),
            recipe.schema.slots.len(),
        ] {
            charge(&mut left, n, "validation entries")?;
        }
        for n in [
            recipe.rules.tables.len(),
            recipe.rules.owners.len(),
            recipe.rules.receivers.members.len(),
            recipe.routing.outputs.len(),
        ] {
            charge(&mut left, n, "validation entries")?;
        }
        for table in &recipe.rules.tables {
            charge(&mut left, table.rows.len(), "validation entries")?;
        }
        for owner in &recipe.rules.owners {
            charge(
                &mut left,
                owner.programs.members.len(),
                "validation entries",
            )?;
            for program in &owner.programs.members {
                for n in [
                    program.reads.len(),
                    program.nodes.len(),
                    program.effects.len(),
                ] {
                    charge(&mut left, n, "validation entries")?;
                }
            }
        }
        for receiver in &recipe.rules.receivers.members {
            charge(&mut left, receiver.targets.len(), "validation entries")?;
        }
        for output in &recipe.routing.outputs {
            charge(&mut left, output.routes.members.len(), "validation entries")?;
        }
    }
    for n in [
        input.mapping.entries.len(),
        input.roles.roles.len(),
        input.rewards.rules.len(),
        input.items.rules.len(),
        input.item_source.rule_layouts.len(),
        input.item_source.template_layouts.len(),
    ] {
        charge(&mut left, n, "validation entries")?;
    }
    for rule in &input.rewards.rules {
        charge(&mut left, rule.outcomes.len(), "validation entries")?;
    }
    if input.query_sets.len() > limits.max_query_sets {
        return Err(SuccessorBundleError::Limit("query sets"));
    }
    let mut names = BTreeSet::new();
    let mut rows = limits.max_queries;
    for set in &input.query_sets {
        let name = set.name.as_str();
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
            || !names.insert(name)
        {
            return Err(SuccessorBundleError::QuerySetName);
        }
        charge(&mut rows, set.queries.len(), "queries")?;
        charge(&mut left, set.queries.len(), "validation entries")?;
        validate_normalization_queries(&set.queries, limits.normalization)?;
        digest_owned(
            "owned-normalization-policy-v3",
            &(&input.normalization, &set.queries),
            limits.normalization.max_policy_bytes,
        )?;
    }
    Ok(left)
}
fn preserve(
    before: &StagedOwnedRecipe,
    after: &StagedOwnedRecipe,
    refinement: Option<&SchemaDeclarationRefinement>,
    validation_left: &mut usize,
) -> Result<(usize, usize)> {
    before.registry().validate_successor(after.registry())?;
    let old = &before.registry().input().entries;
    if after.registry().input().entries.get(..old.len()) != Some(old.as_slice()) {
        return Err(SuccessorBundleError::ChangedRegistry);
    }
    if let Some(SchemaDeclarationRefinement::GemsV4(policy)) = refinement {
        return gems::preserve(policy, before, after, validation_left);
    }
    if let Some(SchemaDeclarationRefinement::MembershipV3(policy)) = refinement {
        if policy.schema_version != 3
            || policy.before == policy.after
            || &policy.before != before.schema().identity()
            || &policy.after != after.schema().identity()
        {
            return Err(SuccessorBundleError::Refinement(
                "version or endpoint binding",
            ));
        }
        if policy.subjects.is_empty() {
            return Err(SuccessorBundleError::Refinement("empty policy"));
        }
        return membership::preserve(policy, before, after, validation_left);
    }
    let definitions: BTreeMap<_, _> = after
        .schema()
        .input()
        .definitions
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    let slots: BTreeMap<_, _> = after
        .schema()
        .input()
        .slots
        .iter()
        .map(|row| (row.address(), row))
        .collect();
    let mut refined = BTreeSet::new();
    if let Some(policy) = refinement {
        policy.validate_endpoints(before.schema().identity(), after.schema().identity())?;
        for owner in policy.owners() {
            let address = owner.address();
            if address.namespace() != &before.schema().input().namespace || !refined.insert(address)
            {
                return Err(SuccessorBundleError::Refinement(
                    "foreign or duplicate owner",
                ));
            }
        }
    }
    let count = refined.len();
    for row in &before.schema().input().definitions {
        let address = row.address();
        let next = definitions.get(&address).copied();
        if refined.remove(&address) {
            let mut expected = row.clone();
            close_declarations(&mut expected)?;
            if &expected == row || next != Some(&expected) {
                return Err(SuccessorBundleError::Refinement(
                    "not an exact closure refinement",
                ));
            }
        } else if next != Some(row) {
            return Err(SuccessorBundleError::ChangedDeclaration);
        }
    }
    if !refined.is_empty() {
        return Err(SuccessorBundleError::Refinement(
            "owner absent from predecessor",
        ));
    }
    if before
        .schema()
        .input()
        .slots
        .iter()
        .any(|row| slots.get(&row.address()).copied() != Some(row))
    {
        return Err(SuccessorBundleError::ChangedDeclaration);
    }
    Ok((count, 0))
}
fn append_mapping(
    input: &mut MappingPackageInput,
    append: &CatalogAppend,
    limits: SuccessorBundleLimits,
) -> Result<()> {
    if input.source.system != append.source.system
        || input.source.revision != append.source.revision
    {
        return Err(SuccessorBundleError::CatalogConflict(
            "source context differs",
        ));
    }
    let count = input
        .entries
        .len()
        .checked_add(append.mappings.len())
        .ok_or(SuccessorBundleError::Limit("mapping entries"))?;
    if count > limits.catalog.mapping.max_collection_entries {
        return Err(SuccessorBundleError::Limit("mapping entries"));
    }
    // Compare before appending; a failed call never mutates the caller's input.
    let mut selectors: BTreeSet<_> = input.entries.iter().map(|entry| &entry.source).collect();
    for entry in &append.mappings {
        if !selectors.insert(&entry.source) {
            return Err(SuccessorBundleError::CatalogConflict(
                "existing or duplicate mapping selector",
            ));
        }
    }
    let mut pins: BTreeMap<_, _> = input
        .source
        .files
        .iter()
        .map(|pin| (pin.path.as_str(), pin))
        .collect();
    let mut supplied = BTreeSet::new();
    for pin in &append.source.files {
        if !supplied.insert(pin.path.as_str()) {
            return Err(SuccessorBundleError::CatalogConflict(
                "duplicate supplied source path",
            ));
        }
        if let Some(previous) = pins.get(pin.path.as_str()) {
            if previous.sha256 != pin.sha256 {
                return Err(SuccessorBundleError::CatalogConflict("source hash differs"));
            }
        } else {
            pins.insert(pin.path.as_str(), pin);
        }
    }
    if pins.len() > limits.catalog.mapping.max_collection_entries {
        return Err(SuccessorBundleError::Limit("source pins"));
    }
    let files = pins.into_values().cloned().collect();
    input.entries.extend(append.mappings.iter().cloned());
    input.source.files = files;
    // The complete mapping constructor subsequently validates all appended
    // selectors, targets, source text and aggregate budgets before publication.
    Ok(())
}

fn bindings(
    recipe: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    policy: &NormalizationPolicy,
    rewards: &OwnedRewardPolicy,
    limits: SuccessorBundleLimits,
) -> Result<SuccessorBindings> {
    Ok(SuccessorBindings {
        registry: recipe.registry().identity()?,
        definitions: recipe.schema().identity().clone(),
        mapping: *mapping.identity(),
        roles: *roles.identity(),
        normalization: digest_owned(
            "owned-normalization-policy-v3",
            policy,
            limits.normalization.max_policy_bytes,
        )?,
        rewards: *rewards.identity(),
        rules: *recipe.rules().identity(),
        routing: *recipe.routing().identity(),
    })
}
fn add<T: Serialize>(
    extra: &mut Vec<Artifact>,
    left: &mut usize,
    name: String,
    value: &T,
) -> Result<()> {
    // Streaming bounded serialization before allocating the retained byte buffer.
    digest_owned("owned-successor-artifact-v1", value, *left)?;
    let bytes = serde_json::to_vec(value)?;
    charge(left, bytes.len(), "output bytes")?;
    extra.push(Artifact { name, bytes });
    Ok(())
}

pub fn decode_successor_bundle(
    bytes: &[u8],
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    limits.validate()?;
    if bytes.len() > limits.max_input_bytes {
        return Err(SuccessorBundleError::Limit("input bytes"));
    }
    transition_owned_bundle(serde_json::from_slice(bytes)?, limits)
}

/// Validate both endpoints and prior bindings, prove exact declaration/history
/// preservation, then perform the explicit rebind. Semantic program changes are
/// accepted only through the normal rule/routing constructors and compiler.
/// Source pins remain offline provenance; no checkout is opened or authenticated.
pub fn transition_owned_bundle(
    input: SuccessorBundleInput,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(input, None, None, None, limits)
}

/// Add catalog identities/mappings to one successor package. This shares every
/// recipe, prior-binding, history and publication check with the carry-only API.
/// The compiler supplies a fully assembled successor recipe; no IDs are allocated
/// here and no source code is evaluated. Repeated catalog compilation should reuse
/// exact prior identities and submit no already-existing selector in `mappings`.
pub fn transition_owned_catalog(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(input, Some(append), None, None, limits)
}

/// One typed tree artifact joins the same checked finalization/publication path.
pub fn transition_owned_catalog_with_tree(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(input, Some(append), Some(tree), None, limits)
}

/// Publish canonical constituents once, without a duplicate `recipe.json`.
/// The V2 input commitment hashes the exact prior recipe separately, then the
/// successor and all remaining supplied inputs. Each stream is independently
/// bounded by `max_input_bytes`; the prior also obeys the recipe byte limit.
/// Neither this commitment nor the publication manifest authenticates history.
pub fn transition_owned_catalog_with_tree_compact(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        Some(append),
        Some(tree),
        None,
        limits,
        PublicationFormat::CompactV2,
    )
}

/// Carry-only V2 publication, sharing the same endpoint and preservation checks.
pub fn transition_owned_bundle_compact(
    input: SuccessorBundleInput,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        None,
        None,
        None,
        limits,
        PublicationFormat::CompactV2,
    )
}

/// Refine reviewed passive port closure while carrying the tree/import artifacts
/// through the same validated finalizer. This is an explicit offline operation;
/// native loaders never infer or apply refinements to stale packages.
pub fn transition_owned_catalog_with_tree_refinement(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: PassiveDeclarationRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
    )
}

/// Reviewed passive port closure with compact V2 publication. This uses the same
/// endpoint-bound refinement and preservation checks as the legacy envelope.
pub fn transition_owned_catalog_with_tree_refinement_compact(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: PassiveDeclarationRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
        PublicationFormat::CompactV2,
    )
}

/// Generalized, endpoint-bound input-port closure. All prior bindings, registry
/// history, declarations and publication checks use the same finalizer as V1.
pub fn transition_owned_catalog_with_declaration_refinement(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: DeclarationClosureRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
    )
}

/// Validate exact append-only registry history and every previous descriptor,
/// permitting only the endpoint-bound partial-set additions declared by policy.
/// The finalizer uses the same implementation with its remaining shared budget.
pub fn validate_schema_membership_refinement(
    policy: &SchemaMembershipRefinement,
    before: &StagedOwnedRecipe,
    after: &StagedOwnedRecipe,
) -> Result<()> {
    let mut left = SuccessorBundleLimits::default().max_validation_entries;
    charge(&mut left, policy.subjects.len(), "refinement entries")?;
    preserve(before, after, Some(&policy.clone().into()), &mut left).map(|_| ())
}

/// V3 monotonic membership refinement in the existing legacy publication envelope.
pub fn transition_owned_catalog_with_membership_refinement(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: SchemaMembershipRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
    )
}

/// V3 monotonic membership refinement with compact V2 publication.
pub fn transition_owned_catalog_with_membership_refinement_compact(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: SchemaMembershipRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
        PublicationFormat::CompactV2,
    )
}

/// Validate only the declared physical Gem knowledge migration, exact prior
/// role/source evidence, and unchanged mechanical programs and routing.
pub fn validate_gem_schema_refinement(
    policy: &GemSchemaRefinement,
    before: &StagedOwnedRecipe,
    after: &StagedOwnedRecipe,
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
) -> Result<()> {
    let mut left = SuccessorBundleLimits::default().max_validation_entries;
    if mapping.input().registry != before.registry().identity()? {
        return Err(SuccessorBundleError::Refinement(
            "gem migration prior registry binding",
        ));
    }
    policy.validate_prior_catalog(mapping, roles, after.schema())?;
    preserve(before, after, Some(&policy.clone().into()), &mut left).map(|_| ())
}

/// V4 physical Gem knowledge migration through the checked compact publisher.
/// Mapping/role contents, prior item policies and tree content are carried exactly.
pub fn transition_owned_catalog_with_gem_refinement_compact(
    input: SuccessorBundleInput,
    append: CatalogAppend,
    tree: TreePolicyTransitionInput,
    refinement: GemSchemaRefinement,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        Some(append),
        Some(tree),
        Some(refinement.into()),
        limits,
        PublicationFormat::CompactV2,
    )
}

/// Install an explicitly authored normalization policy without changing recipes.
/// The true prior policy is validated and committed before replacement. New
/// policy bindings must already match the successor; stale inputs are rejected.
pub fn transition_owned_normalization_with_tree_compact(
    input: SuccessorBundleInput,
    prior_tree: TreeNormalizationPackageInput,
    normalization: NormalizationPolicy,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    if input.prior != input.successor {
        return Err(SuccessorBundleError::Refinement(
            "normalization transition changed recipe",
        ));
    }
    let append = CatalogAppend {
        mappings: vec![],
        source: input.mapping.source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    finalize_successor_operation(
        input,
        Some(append),
        Some(TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior_tree),
        }),
        None,
        Some(normalization),
        limits,
        PublicationFormat::CompactV2,
    )
}

fn finalize_successor(
    input: SuccessorBundleInput,
    append: Option<CatalogAppend>,
    tree_update: Option<TreePolicyTransitionInput>,
    refinement: Option<SchemaDeclarationRefinement>,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_with_format(
        input,
        append,
        tree_update,
        refinement,
        limits,
        PublicationFormat::LegacyV1,
    )
}

/// Borrowed commitment: no second recipe serialization inside the main stream.
/// Every field of the original input remains bound, including ordered queries
/// and exact policies. Optional operations use their full typed representations.
#[derive(Serialize)]
struct CompactInputCommitment<'a> {
    schema_version: u32,
    prior_recipe: OwnedContentDigest,
    successor: &'a OwnedRecipeInput,
    mapping: &'a MappingPackageInput,
    roles: &'a OwnedSkillRolePackageInput,
    normalization: &'a NormalizationPolicy,
    rewards: &'a RewardPolicyInput,
    query_sets: &'a [NamedQuerySet],
    items: &'a ItemLinePolicyInput,
    item_source: &'a ItemSourceLayoutPolicyInput,
    append: &'a Option<CatalogAppend>,
    tree: &'a Option<TreePolicyTransitionInput>,
    refinement: &'a Option<SchemaDeclarationRefinement>,
}
fn compact_input_digest(
    input: &SuccessorBundleInput,
    append: &Option<CatalogAppend>,
    tree: &Option<TreePolicyTransitionInput>,
    refinement: &Option<SchemaDeclarationRefinement>,
    limits: SuccessorBundleLimits,
) -> Result<OwnedContentDigest> {
    let prior_recipe = digest_owned(
        "owned-recipe-input-v1",
        &input.prior,
        limits.max_input_bytes.min(limits.recipe.max_wire_bytes),
    )?;
    Ok(digest_owned(
        "owned-successor-input-v2",
        &CompactInputCommitment {
            schema_version: input.schema_version,
            prior_recipe,
            successor: &input.successor,
            mapping: &input.mapping,
            roles: &input.roles,
            normalization: &input.normalization,
            rewards: &input.rewards,
            query_sets: &input.query_sets,
            items: &input.items,
            item_source: &input.item_source,
            append,
            tree,
            refinement,
        },
        limits.max_input_bytes,
    )?)
}

fn finalize_successor_with_format(
    input: SuccessorBundleInput,
    append: Option<CatalogAppend>,
    tree_update: Option<TreePolicyTransitionInput>,
    refinement: Option<SchemaDeclarationRefinement>,
    limits: SuccessorBundleLimits,
    format: PublicationFormat,
) -> Result<StagedSuccessorBundle> {
    finalize_successor_operation(input, append, tree_update, refinement, None, limits, format)
}

fn finalize_successor_operation(
    input: SuccessorBundleInput,
    append: Option<CatalogAppend>,
    tree_update: Option<TreePolicyTransitionInput>,
    refinement: Option<SchemaDeclarationRefinement>,
    replacement_normalization: Option<NormalizationPolicy>,
    limits: SuccessorBundleLimits,
    format: PublicationFormat,
) -> Result<StagedSuccessorBundle> {
    limits.validate()?;
    if let Some(SchemaDeclarationRefinement::GemsV4(policy)) = &refinement
        && (!matches!(&append, Some(a) if a.mappings.is_empty()
            && a.source == policy.source && a.item_policies == CatalogItemPolicyMode::RebindPrior)
            || !matches!(
                &tree_update,
                Some(TreePolicyTransitionInput::RebindPrior { .. })
            ))
    {
        return Err(SuccessorBundleError::Refinement(
            "gem migration must preserve prior catalog, item policies and tree",
        ));
    }
    if input.schema_version != OWNED_SUCCESSOR_VERSION {
        return Err(SuccessorBundleError::Version(input.schema_version));
    }
    let mut input_digest = if format == PublicationFormat::CompactV2 {
        compact_input_digest(&input, &append, &tree_update, &refinement, limits)?
    } else if let Some(policy) = &refinement {
        match policy {
            SchemaDeclarationRefinement::LegacyPassiveV1(policy) => digest_owned(
                "owned-passive-refinement-successor-input-v1",
                &(&input, &append, &tree_update, policy),
                limits.max_input_bytes,
            )?,
            SchemaDeclarationRefinement::OwnersV2(policy) => digest_owned(
                "owned-declaration-closure-successor-input-v2",
                &(&input, &append, &tree_update, policy),
                limits.max_input_bytes,
            )?,
            SchemaDeclarationRefinement::GemsV4(policy) => digest_owned(
                "owned-gem-schema-successor-input-v4",
                &(&input, &append, &tree_update, policy),
                limits.max_input_bytes,
            )?,
            SchemaDeclarationRefinement::MembershipV3(policy) => digest_owned(
                "owned-schema-membership-successor-input-v3",
                &(&input, &append, &tree_update, policy),
                limits.max_input_bytes,
            )?,
        }
    } else if let Some(tree) = &tree_update {
        digest_owned(
            "owned-tree-catalog-successor-input-v1",
            &(&input, &append, tree),
            limits.max_input_bytes,
        )?
    } else {
        match &append {
            None => digest_owned("owned-successor-input-v1", &input, limits.max_input_bytes)?,
            Some(append) => digest_owned(
                "owned-catalog-successor-input-v1",
                &(&input, append),
                limits.max_input_bytes,
            )?,
        }
    };
    if let Some(policy) = &replacement_normalization {
        input_digest = digest_owned(
            "owned-normalization-replacement-successor-input-v1",
            &(input_digest, policy),
            limits.max_input_bytes,
        )?;
    }
    let mut validation_left = preflight(&input, append.as_ref(), limits)?;
    if let Some(policy) = &refinement {
        charge(&mut validation_left, policy.len(), "refinement entries")?;
    }
    if let Some(tree) = &tree_update {
        let content = match tree {
            TreePolicyTransitionInput::Install { content } => content.as_ref(),
            TreePolicyTransitionInput::RebindPrior { prior } => &prior.content,
        };
        let mut count = 0usize;
        for n in [
            content.classes.len(),
            content.ascendancies.len(),
            content.tokens.len(),
            content.attributes.len(),
            content.source.files.len(),
            content.syntax.weapon_overlays.len(),
            content.syntax.ignored_spec_children.len(),
        ] {
            count = count
                .checked_add(n)
                .ok_or(SuccessorBundleError::Limit("validation entries"))?;
        }
        for row in &content.attributes {
            count = count
                .checked_add(row.lanes.len())
                .ok_or(SuccessorBundleError::Limit("validation entries"))?;
        }
        charge(&mut validation_left, count, "validation entries")?;
    }
    let item_policy_mode = append
        .as_ref()
        .map_or(CatalogItemPolicyMode::SuppliedSuccessor, |a| {
            a.item_policies
        });
    let before = assemble_owned_recipe(input.prior, limits.recipe)?;
    let old_mapping = OwnedMappingIndex::new(
        input.mapping,
        before.registry(),
        before.schema(),
        limits.catalog.mapping,
    )?;
    let old_roles =
        OwnedSkillRoleIndex::new(input.roles, &old_mapping, before.schema(), limits.catalog)?;
    validate_normalization_inputs(
        &input.normalization,
        &old_mapping,
        before.schema(),
        &[],
        limits.normalization,
    )?;
    let old_rewards =
        OwnedRewardPolicy::new(input.rewards, &old_mapping, before.schema(), limits.rewards)?;
    let before_bindings = bindings(
        &before,
        &old_mapping,
        &old_roles,
        &input.normalization,
        &old_rewards,
        limits,
    )?;
    let tree_content = match tree_update {
        None => None,
        Some(TreePolicyTransitionInput::Install { content }) => Some(*content),
        Some(TreePolicyTransitionInput::RebindPrior { prior }) => {
            let tree = OwnedTreeNormalizationPolicy::new(
                *prior,
                before.registry(),
                before.schema(),
                &old_mapping,
                &input.normalization,
                limits.tree,
            )?;
            Some(tree.input().content.clone())
        }
    };
    let mut recipe = input.successor;
    let mut after = assemble_owned_recipe(recipe.clone(), limits.recipe)?;
    if format == PublicationFormat::CompactV2 {
        // V1 commits supplied order before canonicalization. V2 publishes the
        // reconstructible canonical recipe; its transition input above still
        // commits the original typed input, including supplied ordering.
        let canonical = OwnedRecipeInput {
            schema_version: recipe.schema_version,
            registry: after.registry().input().clone(),
            schema: after.schema().input().clone(),
            rules: after.rules().input().clone(),
            routing: after.routing().input().clone(),
        };
        if recipe != canonical {
            after = assemble_owned_recipe(canonical.clone(), limits.recipe)?;
        }
        recipe = canonical;
    }
    if let Some(SchemaDeclarationRefinement::GemsV4(policy)) = &refinement {
        policy.validate_prior_catalog(&old_mapping, &old_roles, after.schema())?;
    }
    let (refined_definitions, refined_slots) =
        preserve(&before, &after, refinement.as_ref(), &mut validation_left)?;
    let mut next_mapping = old_mapping.input().clone();
    next_mapping.registry = after.registry().identity()?;
    next_mapping.definitions = after.schema().identity().clone();
    if let Some(append) = &append {
        append_mapping(&mut next_mapping, append, limits)?;
    }
    let mapping = OwnedMappingIndex::new(
        next_mapping,
        after.registry(),
        after.schema(),
        limits.catalog.mapping,
    )?;
    let mut next_roles = old_roles.input().clone();
    next_roles.definitions = after.schema().identity().clone();
    next_roles.mapping = *mapping.identity();
    let roles = OwnedSkillRoleIndex::new(next_roles, &mapping, after.schema(), limits.catalog)?;
    let normalization = if let Some(replacement) = replacement_normalization {
        // Explicit replacement is already successor-bound; never repair it.
        replacement
    } else {
        let mut normalization = input.normalization;
        if let GemQualityPolicy::Attributes(quality) = &mut normalization.gem_quality {
            quality.definitions = after.schema().identity().clone();
        }
        if let Some(inputs) = &mut normalization.gem_inputs {
            inputs.definitions = after.schema().identity().clone();
        }
        normalization
    };
    validate_normalization_inputs(
        &normalization,
        &mapping,
        after.schema(),
        &[],
        limits.normalization,
    )?;
    for set in &input.query_sets {
        // The new schema identity can change encoded policy size; check exactly
        // the pair normalize_fresh will consume, not just the prior-bound pair.
        digest_owned(
            "owned-normalization-policy-v3",
            &(&normalization, &set.queries),
            limits.normalization.max_policy_bytes,
        )?;
    }
    let mut next_rewards = old_rewards.input().clone();
    next_rewards.definitions = after.schema().identity().clone();
    next_rewards.mapping = *mapping.identity();
    let rewards = OwnedRewardPolicy::new(next_rewards, &mapping, after.schema(), limits.rewards)?;
    let (item_input, mut source_input) = match item_policy_mode {
        CatalogItemPolicyMode::SuppliedSuccessor => (input.items, input.item_source),
        CatalogItemPolicyMode::RebindPrior => {
            let old_items = OwnedItemLinePolicy::new(input.items, before.schema(), limits.items)?;
            let old_source = ItemSourceLayoutPolicy::new(
                input.item_source,
                &old_items,
                before.schema(),
                limits.item_source,
            )?;
            let mut items = old_items.input().clone();
            items.definitions = after.schema().identity().clone();
            (items, old_source.input().clone())
        }
    };
    let items = OwnedItemLinePolicy::new(item_input, after.schema(), limits.items)?;
    if item_policy_mode == CatalogItemPolicyMode::RebindPrior {
        source_input.item_lines = *items.identity();
    }
    let item_source =
        ItemSourceLayoutPolicy::new(source_input, &items, after.schema(), limits.item_source)?;
    let tree = tree_content
        .map(|content| {
            OwnedTreeNormalizationPolicy::bind_new(
                content,
                after.registry(),
                after.schema(),
                &mapping,
                &normalization,
                limits.tree,
            )
        })
        .transpose()?;
    let after_bindings = bindings(&after, &mapping, &roles, &normalization, &rewards, limits)?;
    let mut transition = SuccessorBundleTransition {
        schema_version: match format {
            PublicationFormat::LegacyV1 => OWNED_SUCCESSOR_VERSION,
            PublicationFormat::CompactV2 => OWNED_COMPACT_SUCCESSOR_VERSION,
        },
        document_kind: "owned_successor_bundle",
        input: input_digest,
        before: before_bindings,
        after: after_bindings,
        preserved_registry_entries: before.registry().input().entries.len(),
        preserved_definitions: before.schema().input().definitions.len() - refined_definitions,
        preserved_slots: before.schema().input().slots.len() - refined_slots,
        query_sets: input.query_sets.len(),
        query_rows: input.query_sets.iter().map(|s| s.queries.len()).sum(),
        items: *items.identity(),
        item_source: *item_source.identity(),
        tree: tree.as_ref().map(|tree| *tree.identity()),
        schema_policy: match &refinement {
            Some(SchemaDeclarationRefinement::LegacyPassiveV1(_)) => {
                "explicit_passive_declaration_closure"
            }
            Some(SchemaDeclarationRefinement::OwnersV2(_)) => "explicit_input_declaration_closure",
            Some(SchemaDeclarationRefinement::MembershipV3(_)) => {
                "explicit_partial_schema_membership"
            }
            Some(SchemaDeclarationRefinement::GemsV4(_)) => "explicit_gem_schema_knowledge",
            None => "exact_prior_declarations_new_addresses_only",
        },
        schema_refinement: refinement,
        item_policy_mode: match item_policy_mode {
            CatalogItemPolicyMode::SuppliedSuccessor => "explicit_successor_bound_inputs",
            CatalogItemPolicyMode::RebindPrior => "validated_prior_binding_rebind",
        },
        source_execution: false,
        calculation: "not_run",
        whole_build_parity: "not_established",
        artifacts: vec![],
    };
    drop(before);
    let mut extra = vec![];
    let mut left = limits.max_output_bytes;
    for artifact in after.artifacts() {
        charge(&mut left, artifact.bytes().len(), "output bytes")?;
    }
    if format == PublicationFormat::LegacyV1 {
        add(&mut extra, &mut left, "recipe.json".into(), &recipe)?;
    }
    add(
        &mut extra,
        &mut left,
        "mapping.json".into(),
        mapping.input(),
    )?;
    add(&mut extra, &mut left, "roles.json".into(), roles.input())?;
    add(
        &mut extra,
        &mut left,
        "normalization.json".into(),
        &normalization,
    )?;
    add(
        &mut extra,
        &mut left,
        "rewards.json".into(),
        rewards.input(),
    )?;
    add(&mut extra, &mut left, "items.json".into(), items.input())?;
    add(
        &mut extra,
        &mut left,
        "item-source.json".into(),
        item_source.input(),
    )?;
    for set in &input.query_sets {
        add(
            &mut extra,
            &mut left,
            format!("queries-{}.json", set.name.as_str()),
            &set.queries,
        )?;
    }
    if let Some(append) = &append {
        add(&mut extra, &mut left, "catalog-append.json".into(), append)?;
    }
    if let Some(tree) = &tree {
        add(
            &mut extra,
            &mut left,
            "tree-normalization.json".into(),
            tree.input(),
        )?;
    }
    transition.artifacts = after
        .artifacts()
        .iter()
        .map(|a| (a.name(), a.bytes()))
        .chain(extra.iter().map(|a| (a.name.as_str(), a.bytes.as_slice())))
        .map(|(name, bytes)| SuccessorArtifactManifest {
            file: name.into(),
            bytes: bytes.len(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        })
        .collect();
    add(&mut extra, &mut left, "transition.json".into(), &transition)?;
    Ok(StagedSuccessorBundle {
        recipe,
        assembled: after,
        mapping,
        roles,
        normalization,
        rewards,
        items,
        item_source,
        queries: input.query_sets,
        tree,
        transition,
        extra,
    })
}
