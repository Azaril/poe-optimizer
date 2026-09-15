//! Explicit offline package succession. This is not runtime binding repair.
//! Prior import facts are validated before rebind; successor mechanics are fully
//! constructed and compiled. Existing schema declarations are immutable here.
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
};
use poe_optimizer_core::{
    data::DataIdentity,
    owned_content::{ContentDigestError, OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const OWNED_SUCCESSOR_VERSION: u32 = 1;

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

/// Aggregate bytes, top-level declarations, rule programs/table rows, routes and
/// queries are bounded across the transition. Nested constructors additionally
/// bound schema members, expression edges, closure gaps and source/value text.
#[derive(Clone, Copy, Debug)]
pub struct SuccessorBundleLimits {
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
    #[error(
        "query set name must be unique lowercase ASCII letters, digits, hyphens or underscores, at most 64 bytes"
    )]
    QuerySetName,
    #[error("invalid catalog append: {0}")]
    CatalogConflict(&'static str),
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
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
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
    pub schema_policy: &'static str,
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
) -> Result<()> {
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
    Ok(())
}
fn preserve(before: &StagedOwnedRecipe, after: &StagedOwnedRecipe) -> Result<()> {
    before.registry().validate_successor(after.registry())?;
    let old = &before.registry().input().entries;
    if after.registry().input().entries.get(..old.len()) != Some(old.as_slice()) {
        return Err(SuccessorBundleError::ChangedRegistry);
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
    if before
        .schema()
        .input()
        .definitions
        .iter()
        .any(|row| definitions.get(&row.address()).copied() != Some(row))
        || before
            .schema()
            .input()
            .slots
            .iter()
            .any(|row| slots.get(&row.address()).copied() != Some(row))
    {
        return Err(SuccessorBundleError::ChangedDeclaration);
    }
    Ok(())
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
    finalize_successor(input, None, limits)
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
    finalize_successor(input, Some(append), limits)
}

fn finalize_successor(
    input: SuccessorBundleInput,
    append: Option<CatalogAppend>,
    limits: SuccessorBundleLimits,
) -> Result<StagedSuccessorBundle> {
    limits.validate()?;
    if input.schema_version != OWNED_SUCCESSOR_VERSION {
        return Err(SuccessorBundleError::Version(input.schema_version));
    }
    let input_digest = match &append {
        None => digest_owned("owned-successor-input-v1", &input, limits.max_input_bytes)?,
        Some(append) => digest_owned(
            "owned-catalog-successor-input-v1",
            &(&input, append),
            limits.max_input_bytes,
        )?,
    };
    preflight(&input, append.as_ref(), limits)?;
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
    let recipe = input.successor;
    let after = assemble_owned_recipe(recipe.clone(), limits.recipe)?;
    preserve(&before, &after)?;
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
    let mut normalization = input.normalization;
    if let GemQualityPolicy::Attributes(quality) = &mut normalization.gem_quality {
        quality.definitions = after.schema().identity().clone();
    }
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
    let after_bindings = bindings(&after, &mapping, &roles, &normalization, &rewards, limits)?;
    let mut transition = SuccessorBundleTransition {
        schema_version: OWNED_SUCCESSOR_VERSION,
        document_kind: "owned_successor_bundle",
        input: input_digest,
        before: before_bindings,
        after: after_bindings,
        preserved_registry_entries: before.registry().input().entries.len(),
        preserved_definitions: before.schema().input().definitions.len(),
        preserved_slots: before.schema().input().slots.len(),
        query_sets: input.query_sets.len(),
        query_rows: input.query_sets.iter().map(|s| s.queries.len()).sum(),
        items: *items.identity(),
        item_source: *item_source.identity(),
        schema_policy: "exact_prior_declarations_new_addresses_only",
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
    add(&mut extra, &mut left, "recipe.json".into(), &recipe)?;
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
        transition,
        extra,
    })
}
