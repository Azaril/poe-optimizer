//! Standalone offline assembly of an explicitly authored owned data release.
//!
//! All dependency bindings must already be exact. This module never repairs
//! bindings, compares a predecessor, executes a source backend or establishes
//! numerical coverage. Monotonic successor contracts remain independent.
use crate::{
    owned_item_lines::{ItemLineError, ItemLineLimits, ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{
        ItemSourceError, ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput, ItemSourceLimits,
    },
    owned_mapping::{MappingPackageInput, OwnedMappingError, OwnedMappingIndex, SourcePin},
    owned_normalize::{
        NormalizationError, NormalizationLimits, NormalizationPolicy,
        validate_normalization_inputs, validate_normalization_queries,
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
    owned_successor::NamedQuerySet,
    owned_tree_policy::{
        OwnedTreeNormalizationPolicy, TreeNormalizationPackageInput, TreePolicyError,
        TreePolicyLimits,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

pub const OWNED_RELEASE_VERSION: u32 = 1;

/// Declared authoring provenance, not independently verified ancestry. A compiler
/// may establish a stronger contract before supplying its own provenance record.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseProvenance {
    pub kind: OwnedDefinitionKey,
    pub prior_input: OwnedContentDigest,
    pub authoring_input: OwnedContentDigest,
}

/// Full authored release contents. Constituents must bind this exact recipe;
/// source identities and unresolved semantics remain explicit imported data.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseInput {
    pub schema_version: u32,
    pub recipe: OwnedRecipeInput,
    pub mapping: MappingPackageInput,
    pub roles: OwnedSkillRolePackageInput,
    pub normalization: NormalizationPolicy,
    pub rewards: RewardPolicyInput,
    pub items: ItemLinePolicyInput,
    pub item_source: ItemSourceLayoutPolicyInput,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<TreeNormalizationPackageInput>,
    pub query_sets: Vec<NamedQuerySet>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<OwnedReleaseProvenance>,
}

/// Aggregate ceilings complement the independently bounded component loaders.
/// Query commitments include the normalization policy once per named query set,
/// matching the complete pair consumed by fresh normalization.
#[derive(Clone, Copy, Debug)]
pub struct OwnedReleaseLimits {
    pub max_input_bytes: usize,
    pub max_output_bytes: usize,
    pub max_artifact_bytes: usize,
    pub max_validation_entries: usize,
    pub max_query_sets: usize,
    pub max_queries: usize,
    pub max_query_commitment_bytes: usize,
    pub max_provenance_entries: usize,
    pub recipe: OwnedRecipeLimits,
    pub catalog: SkillCatalogLimits,
    pub normalization: NormalizationLimits,
    pub rewards: RewardPolicyLimits,
    pub items: ItemLineLimits,
    pub item_source: ItemSourceLimits,
    pub tree: TreePolicyLimits,
}
impl Default for OwnedReleaseLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 64 * 1024 * 1024,
            max_output_bytes: 64 * 1024 * 1024,
            max_artifact_bytes: 64 * 1024 * 1024,
            max_validation_entries: 2_000_000,
            max_query_sets: 64,
            max_queries: 100_000,
            max_query_commitment_bytes: 64 * 1024 * 1024,
            max_provenance_entries: 1024,
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
impl OwnedReleaseLimits {
    pub(crate) fn validate(self) -> Result<()> {
        let hard = Self::default();
        for (name, value, maximum) in [
            ("input bytes", self.max_input_bytes, hard.max_input_bytes),
            ("output bytes", self.max_output_bytes, hard.max_output_bytes),
            (
                "artifact bytes",
                self.max_artifact_bytes,
                hard.max_artifact_bytes,
            ),
            (
                "validation entries",
                self.max_validation_entries,
                hard.max_validation_entries,
            ),
            ("query sets", self.max_query_sets, hard.max_query_sets),
            ("queries", self.max_queries, hard.max_queries),
            (
                "query commitment bytes",
                self.max_query_commitment_bytes,
                hard.max_query_commitment_bytes,
            ),
            (
                "provenance entries",
                self.max_provenance_entries,
                hard.max_provenance_entries,
            ),
        ] {
            if value == 0 || value > maximum {
                return Err(OwnedReleaseError::InvalidLimit(name));
            }
        }
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum OwnedReleaseError {
    #[error("unsupported owned data release version {0}")]
    Version(u32),
    #[error("invalid owned data release limit: {0}")]
    InvalidLimit(&'static str),
    #[error("owned data release exceeds {0}")]
    Limit(&'static str),
    #[error("invalid owned data release: {0}")]
    Invalid(&'static str),
    #[error(
        "query set names must be unique lowercase ASCII letters, digits, hyphens or underscores, at most 64 bytes"
    )]
    QuerySetName,
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
    Tree(#[from] TreePolicyError),
    #[error(transparent)]
    Digest(#[from] ContentDigestError),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
type Result<T> = std::result::Result<T, OwnedReleaseError>;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseArtifactManifest {
    pub file: String,
    pub bytes: usize,
    pub sha256: String,
}

/// Commits canonical authored contents and every emitted artifact except this
/// receipt itself. It attests validation and serialization, not source truth,
/// compatibility with older releases, calculation or numerical parity.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedReleaseReceipt {
    pub schema_version: u32,
    pub document_kind: String,
    pub input: OwnedContentDigest,
    pub registry: OwnedContentDigest,
    pub definitions: DataIdentity,
    pub rules: OwnedContentDigest,
    pub compiled_rules: OwnedContentDigest,
    pub routing: OwnedContentDigest,
    pub mapping: OwnedContentDigest,
    pub roles: OwnedContentDigest,
    pub normalization: OwnedContentDigest,
    pub rewards: OwnedContentDigest,
    pub items: OwnedContentDigest,
    pub item_source: OwnedContentDigest,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree: Option<OwnedContentDigest>,
    /// Deterministic union of compatible constituent source footprints. Original
    /// per-component pins remain unchanged in their own committed artifacts.
    pub source: SourcePin,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provenance: Vec<OwnedReleaseProvenance>,
    pub query_sets: usize,
    pub query_rows: usize,
    pub query_policy_bytes: usize,
    pub artifacts: Vec<OwnedReleaseArtifactManifest>,
}

struct Artifact {
    name: String,
    bytes: Vec<u8>,
}

/// Validated immutable contents. Only the host performs filesystem publication;
/// returned artifact names are fixed basenames or validated query labels.
pub struct StagedOwnedRelease {
    input: OwnedReleaseInput,
    assembled: StagedOwnedRecipe,
    mapping: OwnedMappingIndex,
    roles: OwnedSkillRoleIndex,
    rewards: OwnedRewardPolicy,
    items: OwnedItemLinePolicy,
    item_source: ItemSourceLayoutPolicy,
    tree: Option<OwnedTreeNormalizationPolicy>,
    receipt: OwnedReleaseReceipt,
    extra: Vec<Artifact>,
}
impl StagedOwnedRelease {
    /// Canonical complete input reconstructible from the publication, including
    /// ordered queries and explicitly declared authoring provenance.
    pub fn input(&self) -> &OwnedReleaseInput {
        &self.input
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
        &self.input.normalization
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
        &self.input.query_sets
    }
    pub fn receipt(&self) -> &OwnedReleaseReceipt {
        &self.receipt
    }
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

fn charge(left: &mut usize, count: usize, name: &'static str) -> Result<()> {
    *left = left
        .checked_sub(count)
        .ok_or(OwnedReleaseError::Limit(name))?;
    Ok(())
}

struct SizeWriter {
    written: usize,
    maximum: usize,
    exceeded: bool,
}
impl io::Write for SizeWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.maximum - self.written {
            self.exceeded = true;
            return Err(io::Error::other("owned release byte limit exceeded"));
        }
        self.written += bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
fn serialized_size<T: Serialize>(value: &T, maximum: usize, name: &'static str) -> Result<usize> {
    let mut writer = SizeWriter {
        written: 0,
        maximum,
        exceeded: false,
    };
    let result = serde_json::to_writer(&mut writer, value);
    if writer.exceeded {
        return Err(OwnedReleaseError::Limit(name));
    }
    result?;
    Ok(writer.written)
}

pub(crate) fn preflight(input: &OwnedReleaseInput, limits: OwnedReleaseLimits) -> Result<usize> {
    let mut left = limits.max_validation_entries;
    let recipe = &input.recipe;
    for count in [
        recipe.registry.entries.len(),
        recipe.schema.definitions.len(),
        recipe.schema.slots.len(),
        recipe.rules.tables.len(),
        recipe.rules.owners.len(),
        recipe.rules.receivers.members.len(),
        recipe.routing.outputs.len(),
        input.mapping.entries.len(),
        input.mapping.source.files.len(),
        input.roles.roles.len(),
        input.roles.compilation.source.files.len(),
        input.rewards.rules.len(),
        input.items.rules.len(),
        input.item_source.rule_layouts.len(),
        input.item_source.template_layouts.len(),
        input.item_source.source.files.len(),
        input.provenance.len(),
        input.query_sets.len(),
    ] {
        charge(&mut left, count, "validation entries")?;
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
            for count in [
                program.reads.len(),
                program.nodes.len(),
                program.effects.len(),
            ] {
                charge(&mut left, count, "validation entries")?;
            }
        }
    }
    for receiver in &recipe.rules.receivers.members {
        charge(&mut left, receiver.targets.len(), "validation entries")?;
    }
    for output in &recipe.routing.outputs {
        charge(&mut left, output.routes.members.len(), "validation entries")?;
    }
    for rule in &input.rewards.rules {
        charge(&mut left, rule.outcomes.len(), "validation entries")?;
    }
    if let Some(tree) = &input.tree {
        let content = &tree.content;
        for count in [
            content.source.files.len(),
            content.classes.len(),
            content.ascendancies.len(),
            content.tokens.len(),
            content.attributes.len(),
            content.syntax.weapon_overlays.len(),
            content.syntax.ignored_spec_children.len(),
        ] {
            charge(&mut left, count, "validation entries")?;
        }
        for row in &content.attributes {
            charge(&mut left, row.lanes.len(), "validation entries")?;
        }
    }
    if input.provenance.len() > limits.max_provenance_entries {
        return Err(OwnedReleaseError::Limit("provenance entries"));
    }
    if input.query_sets.len() > limits.max_query_sets {
        return Err(OwnedReleaseError::Limit("query sets"));
    }
    let mut names = BTreeSet::new();
    let mut rows = limits.max_queries;
    let mut commitment_left = limits.max_query_commitment_bytes;
    for set in &input.query_sets {
        let name = set.name.as_str();
        if name.is_empty()
            || name.len() > 64
            || !name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
            || !names.insert(name)
        {
            return Err(OwnedReleaseError::QuerySetName);
        }
        charge(&mut rows, set.queries.len(), "queries")?;
        charge(&mut left, set.queries.len(), "validation entries")?;
        validate_normalization_queries(&set.queries, limits.normalization)?;
        let bytes = serialized_size(
            &(&input.normalization, &set.queries),
            commitment_left.min(limits.normalization.max_policy_bytes),
            "query commitment bytes",
        )?;
        charge(&mut commitment_left, bytes, "query commitment bytes")?;
    }
    Ok(limits.max_query_commitment_bytes - commitment_left)
}

fn source_union(
    mapping: &OwnedMappingIndex,
    roles: &OwnedSkillRoleIndex,
    item_source: &ItemSourceLayoutPolicy,
    tree: Option<&OwnedTreeNormalizationPolicy>,
) -> Result<SourcePin> {
    let root = &mapping.input().source;
    let mut files = BTreeMap::new();
    let sources = [
        Some(root),
        Some(&roles.input().compilation.source),
        Some(&item_source.input().source),
        tree.map(|v| &v.input().content.source),
    ];
    for source in sources.into_iter().flatten() {
        if source.system != root.system || source.revision != root.revision {
            return Err(OwnedReleaseError::Invalid(
                "constituent source identity differs",
            ));
        }
        for file in &source.files {
            if let Some(previous) = files.insert(file.path.as_str(), file)
                && previous.sha256 != file.sha256
            {
                return Err(OwnedReleaseError::Invalid(
                    "constituent source file hash differs",
                ));
            }
        }
    }
    Ok(SourcePin {
        system: root.system,
        revision: root.revision.clone(),
        files: files.into_values().cloned().collect(),
    })
}

fn add<T: Serialize>(
    extra: &mut Vec<Artifact>,
    left: &mut usize,
    maximum: usize,
    name: String,
    value: &T,
) -> Result<()> {
    // Check both ceilings by streaming before allocating the retained buffer.
    let size = serialized_size(value, maximum.min(*left), "artifact/output bytes")?;
    charge(left, size, "output bytes")?;
    extra.push(Artifact {
        name,
        bytes: serde_json::to_vec(value)?,
    });
    Ok(())
}

pub fn decode_owned_release(
    bytes: &[u8],
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    limits.validate()?;
    if bytes.len() > limits.max_input_bytes {
        return Err(OwnedReleaseError::Limit("input bytes"));
    }
    assemble_owned_release(serde_json::from_slice(bytes)?, limits)
}

pub fn assemble_owned_release(
    mut input: OwnedReleaseInput,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    limits.validate()?;
    if input.schema_version != OWNED_RELEASE_VERSION {
        return Err(OwnedReleaseError::Version(input.schema_version));
    }
    // Bound the entire typed input before constructor clones or indexing. This
    // initial digest is only a resource check; the receipt commits canonical data.
    digest_owned(
        "owned-data-release-input-v1",
        &input,
        limits.max_input_bytes,
    )?;
    let query_policy_bytes = preflight(&input, limits)?;
    let mut assembled = assemble_owned_recipe(input.recipe.clone(), limits.recipe)?;
    let canonical_recipe = OwnedRecipeInput {
        schema_version: input.recipe.schema_version,
        registry: assembled.registry().input().clone(),
        schema: assembled.schema().input().clone(),
        rules: assembled.rules().input().clone(),
        routing: assembled.routing().input().clone(),
    };
    if input.recipe != canonical_recipe {
        assembled = assemble_owned_recipe(canonical_recipe.clone(), limits.recipe)?;
    }
    input.recipe = canonical_recipe;
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        assembled.registry(),
        assembled.schema(),
        limits.catalog.mapping,
    )?;
    let roles = OwnedSkillRoleIndex::new(
        input.roles.clone(),
        &mapping,
        assembled.schema(),
        limits.catalog,
    )?;
    validate_normalization_inputs(
        &input.normalization,
        &mapping,
        assembled.schema(),
        &[],
        limits.normalization,
    )?;
    let normalization = digest_owned(
        "owned-normalization-policy-v3",
        &input.normalization,
        limits.normalization.max_policy_bytes,
    )?;
    let rewards = OwnedRewardPolicy::new(
        input.rewards.clone(),
        &mapping,
        assembled.schema(),
        limits.rewards,
    )?;
    let items = OwnedItemLinePolicy::new(input.items.clone(), assembled.schema(), limits.items)?;
    let item_source = ItemSourceLayoutPolicy::new(
        input.item_source.clone(),
        &items,
        assembled.schema(),
        limits.item_source,
    )?;
    let tree = input
        .tree
        .as_ref()
        .map(|value| {
            OwnedTreeNormalizationPolicy::new(
                value.clone(),
                assembled.registry(),
                assembled.schema(),
                &mapping,
                &input.normalization,
                limits.tree,
            )
        })
        .transpose()?;
    let source = source_union(&mapping, &roles, &item_source, tree.as_ref())?;
    // Canonicalization changes ordering only, never an incoming dependency binding.
    input.mapping = mapping.input().clone();
    input.roles = roles.input().clone();
    input.rewards = rewards.input().clone();
    input.items = items.input().clone();
    input.item_source = item_source.input().clone();
    input.tree = tree.as_ref().map(|value| value.input().clone());
    let input_digest = digest_owned(
        "owned-data-release-input-v1",
        &input,
        limits.max_input_bytes,
    )?;
    let mut extra = vec![];
    let mut left = limits.max_output_bytes;
    for artifact in assembled.artifacts() {
        if artifact.bytes().len() > limits.max_artifact_bytes {
            return Err(OwnedReleaseError::Limit("artifact bytes"));
        }
        charge(&mut left, artifact.bytes().len(), "output bytes")?;
    }
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "mapping.json".into(),
        mapping.input(),
    )?;
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "roles.json".into(),
        roles.input(),
    )?;
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "normalization.json".into(),
        &input.normalization,
    )?;
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "rewards.json".into(),
        rewards.input(),
    )?;
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "items.json".into(),
        items.input(),
    )?;
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "item-source.json".into(),
        item_source.input(),
    )?;
    for set in &input.query_sets {
        add(
            &mut extra,
            &mut left,
            limits.max_artifact_bytes,
            format!("queries-{}.json", set.name.as_str()),
            &set.queries,
        )?;
    }
    if let Some(tree) = &tree {
        add(
            &mut extra,
            &mut left,
            limits.max_artifact_bytes,
            "tree-normalization.json".into(),
            tree.input(),
        )?;
    }
    let mut receipt = OwnedReleaseReceipt {
        schema_version: OWNED_RELEASE_VERSION,
        document_kind: "owned_data_release".into(),
        input: input_digest,
        registry: assembled.registry().identity()?,
        definitions: assembled.schema().identity().clone(),
        rules: *assembled.rules().identity(),
        compiled_rules: assembled.manifest().compiled_rules,
        routing: *assembled.routing().identity(),
        mapping: *mapping.identity(),
        roles: *roles.identity(),
        normalization,
        rewards: *rewards.identity(),
        items: *items.identity(),
        item_source: *item_source.identity(),
        tree: tree.as_ref().map(|value| *value.identity()),
        source,
        provenance: input.provenance.clone(),
        query_sets: input.query_sets.len(),
        query_rows: input
            .query_sets
            .iter()
            .map(|value| value.queries.len())
            .sum(),
        query_policy_bytes,
        artifacts: vec![],
    };
    receipt.artifacts = assembled
        .artifacts()
        .iter()
        .map(|value| (value.name(), value.bytes()))
        .chain(
            extra
                .iter()
                .map(|value| (value.name.as_str(), value.bytes.as_slice())),
        )
        .map(|(file, bytes)| OwnedReleaseArtifactManifest {
            file: file.into(),
            bytes: bytes.len(),
            sha256: format!("{:x}", Sha256::digest(bytes)),
        })
        .collect();
    add(
        &mut extra,
        &mut left,
        limits.max_artifact_bytes,
        "release.json".into(),
        &receipt,
    )?;
    Ok(StagedOwnedRelease {
        input,
        assembled,
        mapping,
        roles,
        rewards,
        items,
        item_source,
        tree,
        receipt,
        extra,
    })
}
