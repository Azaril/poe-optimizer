//! Checked owned data releases, with separate consumption and publication paths.
//!
//! Both paths use the same constituent validation. The runtime artifact reader
//! never invokes offline assembly. Bindings must already be exact; no path repairs
//! them, runs a source backend or establishes numerical coverage.
use crate::{
    owned_item_lines::{ItemLineError, ItemLineLimits, ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{
        ItemSourceError, ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput, ItemSourceLimits,
    },
    owned_mapping::{MappingPackageInput, OwnedMappingError, OwnedMappingIndex, SourcePin},
    owned_normalize::{
        NamedQuerySet, NormalizationError, NormalizationLimits, NormalizationPolicy,
        validate_configuration_reward_inventory, validate_item_modifier_membership,
        validate_item_parameter_inputs, validate_normalization_inputs,
        validate_normalization_queries, validate_passive_socket_placement,
        validate_role_bound_normalization,
    },
    owned_recipe::{
        OwnedRecipeError, OwnedRecipeInput, OwnedRecipeLimits, StagedOwnedRecipe,
        stage_checked_recipe,
    },
    owned_release_evaluation::{
        CheckedOwnedReleaseEvaluation, OwnedReleaseEvaluationError, OwnedReleaseEvaluationInput,
        OwnedReleaseEvaluationLimits, OwnedReleaseEvaluationReceipt,
    },
    owned_reward_policy::{
        OwnedRewardPolicy, RewardPolicyError, RewardPolicyInput, RewardPolicyLimits,
        RewardRuleOutput,
    },
    owned_skill_roles::{
        OwnedSkillRoleIndex, OwnedSkillRolePackageInput, SkillCatalogError, SkillCatalogLimits,
    },
    owned_tree_policy::{
        OwnedTreeNormalizationPolicy, TreeNormalizationPackageInput, TreePolicyError,
        TreePolicyLimits,
    },
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{SchemaClosure, SchemaState},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    io,
};

mod checked;
mod constituents;
mod loader;
mod source;
pub use checked::CheckedOwnedRelease;
use checked::ReleaseView;
pub use loader::load_owned_release_artifacts;
pub use source::check_release_source_dependencies;

pub const OWNED_RELEASE_VERSION: u32 = 1;
pub const OWNED_EVALUATION_RELEASE_VERSION: u32 = 2;

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
    /// Version 2 requires this explicit group; version 1 never carries it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<OwnedReleaseEvaluationInput>,
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
    pub evaluation: OwnedReleaseEvaluationLimits,
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
            evaluation: OwnedReleaseEvaluationLimits::default(),
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
        self.evaluation.validate()?;
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
    Evaluation(#[from] OwnedReleaseEvaluationError),
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation: Option<OwnedReleaseEvaluationReceipt>,
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
    checked: CheckedOwnedRelease,
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
        self.checked.mapping()
    }
    pub fn roles(&self) -> &OwnedSkillRoleIndex {
        self.checked.roles()
    }
    pub fn normalization(&self) -> &NormalizationPolicy {
        self.checked.normalization()
    }
    pub fn rewards(&self) -> &OwnedRewardPolicy {
        self.checked.rewards()
    }
    pub fn items(&self) -> &OwnedItemLinePolicy {
        self.checked.items()
    }
    pub fn item_source(&self) -> &ItemSourceLayoutPolicy {
        self.checked.item_source()
    }
    pub fn tree(&self) -> Option<&OwnedTreeNormalizationPolicy> {
        self.checked.tree()
    }
    pub fn evaluation(&self) -> Option<&CheckedOwnedReleaseEvaluation> {
        self.checked.evaluation()
    }
    pub fn query_sets(&self) -> &[NamedQuerySet] {
        &self.input.query_sets
    }
    pub fn receipt(&self) -> &OwnedReleaseReceipt {
        self.checked.receipt()
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
    preflight_view(&input.into(), limits)
}

fn preflight_view(input: &ReleaseView<'_>, limits: OwnedReleaseLimits) -> Result<usize> {
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
    if let Some(discovery) = &recipe.rules.support_discovery {
        charge(&mut left, discovery.providers.len(), "validation entries")?;
        for row in &discovery.providers {
            if let SchemaState::Unmapped { gaps } = &row.domain {
                charge(&mut left, gaps.len(), "validation entries")?;
            }
        }
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
    if let Some(applications) = &recipe.rules.existing_actor_rules {
        charge(&mut left, applications.members.len(), "validation entries")?;
        if let SchemaClosure::Partial { gaps } = &applications.closure {
            charge(&mut left, gaps.len(), "validation entries")?;
        }
        for application in &applications.members {
            charge(&mut left, application.targets.len(), "validation entries")?;
        }
    }
    if let Some(queries) = &recipe.rules.contribution_queries {
        charge(&mut left, queries.members.len(), "validation entries")?;
        if let SchemaClosure::Partial { gaps } = &queries.closure {
            charge(&mut left, gaps.len(), "validation entries")?;
        }
        for query in &queries.members {
            charge(&mut left, query.groups.len(), "validation entries")?;
            for group in &query.groups {
                charge(&mut left, group.members.members.len(), "validation entries")?;
                if let SchemaClosure::Partial { gaps } = &group.members.closure {
                    charge(&mut left, gaps.len(), "validation entries")?;
                }
                for member in &group.members.members {
                    if let Some(order) = &member.order {
                        charge(&mut left, order.slot_ranks.len(), "validation entries")?;
                    }
                    match &member.producer {
                        poe_optimizer_core::owned_rules::ContributionProducer::ProgramEffect(producer) => {
                            if let poe_optimizer_core::owned_rules::ContributionOrigin::EquipmentUse { slots }
                                | poe_optimizer_core::owned_rules::ContributionOrigin::ItemModifier { slots } = &producer.origin
                            {
                                charge(&mut left, slots.len(), "validation entries")?;
                            }
                        }
                        poe_optimizer_core::owned_rules::ContributionProducer::ApplicationGroup(producer) => {
                            charge(&mut left, producer.declarations.len(), "validation entries")?;
                        }
                    }
                }
            }
        }
    }
    if let Some(applications) = &recipe.rules.effect_applications {
        charge(&mut left, applications.members.len(), "validation entries")?;
        if let SchemaClosure::Partial { gaps } = &applications.closure {
            charge(&mut left, gaps.len(), "validation entries")?;
        }
        for application in &applications.members {
            for count in [
                application.targets.len(),
                application.stacking.len(),
                application.program.reads.len(),
                application.program.nodes.len(),
                application.program.effects.len(),
            ] {
                charge(&mut left, count, "validation entries")?;
            }
        }
    }
    for output in &recipe.routing.outputs {
        charge(&mut left, output.routes.members.len(), "validation entries")?;
    }
    for rule in &input.rewards.rules {
        let entries = match &rule.output {
            RewardRuleOutput::Cases(outcomes) => outcomes.len(),
            RewardRuleOutput::NumericParameter(numeric) => {
                numeric.fixed_parameters.len().saturating_add(1)
            }
        };
        charge(&mut left, entries, "validation entries")?;
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
    if let Some(evaluation) = &input.evaluation {
        crate::owned_release_evaluation::preflight(evaluation, limits.evaluation, &mut left)?;
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
    for set in input.query_sets {
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
    input: OwnedReleaseInput,
    limits: OwnedReleaseLimits,
) -> Result<StagedOwnedRelease> {
    stage_checked_owned_release(CheckedOwnedRelease::check((&input).into(), limits)?)
}

/// Offline publication of a validated release; evaluation uses the checked
/// packages directly and never calls this artifact-producing operation.
pub fn stage_checked_owned_release(mut checked: CheckedOwnedRelease) -> Result<StagedOwnedRelease> {
    let limits = checked.limits;
    let input = OwnedReleaseInput::from_checked(&checked);
    let assembled = stage_checked_recipe(checked.recipe().clone(), limits.recipe)?;
    let mapping = checked.mapping();
    let roles = checked.roles();
    let rewards = checked.rewards();
    let items = checked.items();
    let item_source = checked.item_source();
    let tree = checked.tree();
    let evaluation = checked.evaluation();
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
    if let Some(evaluation) = &evaluation {
        add(
            &mut extra,
            &mut left,
            limits.max_artifact_bytes,
            "metrics.json".into(),
            evaluation.metrics().input(),
        )?;
        if let Some(support) = evaluation.support() {
            add(
                &mut extra,
                &mut left,
                limits.max_artifact_bytes,
                "support-stages.json".into(),
                support.stages().input(),
            )?;
            add(
                &mut extra,
                &mut left,
                limits.max_artifact_bytes,
                "support-preparation.json".into(),
                support.preparation().input(),
            )?;
            add(
                &mut extra,
                &mut left,
                limits.max_artifact_bytes,
                "support-inputs.json".into(),
                support.inputs().input(),
            )?;
            add(
                &mut extra,
                &mut left,
                limits.max_artifact_bytes,
                "support-receiving.json".into(),
                support.receiving().input(),
            )?;
            if let Some(outputs) = support.outputs() {
                add(
                    &mut extra,
                    &mut left,
                    limits.max_artifact_bytes,
                    "support-outputs.json".into(),
                    outputs.input(),
                )?;
            }
        }
    }
    let mut receipt = checked.receipt().clone();
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
    checked.receipt = receipt;
    Ok(StagedOwnedRelease {
        input,
        assembled,
        checked,
        extra,
    })
}
