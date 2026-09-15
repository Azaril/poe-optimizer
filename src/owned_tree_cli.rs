//! Offline tree compiler host; one checked bundle and one publication transaction.
use poe_optimizer_core::{
    owned_content::OwnedContentDigest,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionSchemaIndex, SchemaLookup},
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput},
    owned_mapping::{MappingPackageInput, OwnedMappingIndex},
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe, assemble_owned_recipe},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, NamedQuerySet, OWNED_SUCCESSOR_VERSION,
        PassiveDeclarationRefinement, StagedSuccessorBundle, SuccessorBindings,
        SuccessorBundleInput, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree,
    },
    owned_tree_catalog::{
        TreeCatalogInput, TreeCatalogLimits, TreeCatalogPolicy,
        compile_owned_tree_catalog_extension,
    },
    owned_tree_policy::{OwnedTreeNormalizationPolicy, TreeNormalizationPackageInput},
};
use serde::{
    Deserialize,
    de::{DeserializeOwned, IgnoredAny},
};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    error::Error,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Prior published successor bundle directory, including its artifact manifest.
    input: PathBuf,
    /// Finite exported tree catalog. Never source code or a character build.
    #[arg(long)]
    catalog: PathBuf,
    /// Explicit tree compilation and source-syntax policy.
    #[arg(long)]
    policy: PathBuf,
    /// New destination directory; existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Artifact {
    file: String,
    bytes: usize,
    sha256: String,
}
// Metadata is not provenance authentication. Every consumed artifact is independently
// decoded through its production constructor; unused fields are still required.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    document_kind: String,
    #[serde(rename = "input")]
    _input: IgnoredAny,
    before: SuccessorBindings,
    after: SuccessorBindings,
    #[serde(rename = "preserved_registry_entries")]
    _registry_count: IgnoredAny,
    #[serde(rename = "preserved_definitions")]
    _definition_count: IgnoredAny,
    #[serde(rename = "preserved_slots")]
    _slot_count: IgnoredAny,
    query_sets: usize,
    query_rows: usize,
    items: OwnedContentDigest,
    item_source: OwnedContentDigest,
    tree: Option<OwnedContentDigest>,
    schema_refinement: Option<PassiveDeclarationRefinement>,
    #[serde(rename = "schema_policy")]
    _schema_policy: IgnoredAny,
    #[serde(rename = "item_policy_mode")]
    _item_policy_mode: IgnoredAny,
    #[serde(rename = "source_execution")]
    _source_execution: IgnoredAny,
    #[serde(rename = "calculation")]
    _calculation: IgnoredAny,
    #[serde(rename = "whole_build_parity")]
    _parity: IgnoredAny,
    artifacts: Vec<Artifact>,
}
struct PriorBundle {
    files: BTreeMap<String, Vec<u8>>,
    manifest: Manifest,
}
const REQUIRED: &[&str] = &[
    "recipe.json",
    "registry.json",
    "schema.json",
    "rules.json",
    "routing.json",
    "manifest.json",
    "mapping.json",
    "roles.json",
    "normalization.json",
    "rewards.json",
    "items.json",
    "item-source.json",
];
pub(crate) fn invalid(message: impl Into<String>) -> Box<dyn Error> {
    io::Error::new(io::ErrorKind::InvalidData, message.into()).into()
}
pub(crate) fn read(path: &Path, left: &mut usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(*left as u64 + 1)
        .read_to_end(&mut bytes)?;
    *left = left
        .checked_sub(bytes.len())
        .ok_or_else(|| invalid("aggregate tree/bundle input byte limit"))?;
    Ok(bytes)
}
fn query_name(name: &str) -> Option<&str> {
    let label = name.strip_prefix("queries-")?.strip_suffix(".json")?;
    (!label.is_empty()
        && label.len() <= 64
        && label
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-' || b == b'_'))
    .then_some(label)
}
fn load_bundle(
    root: &Path,
    left: &mut usize,
    limits: SuccessorBundleLimits,
) -> Result<PriorBundle, Box<dyn Error>> {
    let manifest: Manifest = serde_json::from_slice(&read(&root.join("transition.json"), left)?)?;
    if manifest.schema_version != OWNED_SUCCESSOR_VERSION
        || manifest.document_kind != "owned_successor_bundle"
        || manifest.artifacts.len() > REQUIRED.len() + limits.max_query_sets + 2
    {
        return Err(invalid("unsupported or oversized prior bundle manifest"));
    }
    let mut files = BTreeMap::new();
    for row in &manifest.artifacts {
        if !(REQUIRED.contains(&row.file.as_str())
            || ["catalog-append.json", "tree-normalization.json"].contains(&row.file.as_str())
            || query_name(&row.file).is_some())
        {
            return Err(invalid(format!(
                "unrecognized prior artifact: {}",
                row.file
            )));
        }
        if files.contains_key(&row.file) {
            return Err(invalid("duplicate prior artifact"));
        }
        let data = read(&root.join(&row.file), left)?;
        if data.len() != row.bytes || format!("{:x}", Sha256::digest(&data)) != row.sha256 {
            return Err(invalid(format!(
                "prior artifact hash/size differs: {}",
                row.file
            )));
        }
        files.insert(row.file.clone(), data);
    }
    if REQUIRED.iter().any(|name| !files.contains_key(*name)) {
        return Err(invalid("prior bundle omitted a required artifact"));
    }
    let has_tree = files.contains_key("tree-normalization.json");
    if has_tree != manifest.tree.is_some()
        || has_tree != root.join("tree-normalization.json").exists()
    {
        return Err(invalid(
            "prior bundle tree artifact was omitted or has no declared identity",
        ));
    }
    if files
        .keys()
        .filter(|name| query_name(name).is_some())
        .count()
        != manifest.query_sets
    {
        return Err(invalid("prior manifest query-set count differs"));
    }
    // Directory membership must agree too; rewriting counts cannot hide a query file.
    for (index, entry) in std::fs::read_dir(root)?.enumerate() {
        if index >= REQUIRED.len() + limits.max_query_sets + 4 {
            return Err(invalid("prior bundle directory entry limit"));
        }
        let name = entry?.file_name();
        if let Some(name) = name.to_str()
            && query_name(name).is_some()
            && !files.contains_key(name)
        {
            return Err(invalid("prior bundle omitted a query artifact"));
        }
    }
    Ok(PriorBundle { files, manifest })
}
fn decode<T: DeserializeOwned>(
    files: &BTreeMap<String, Vec<u8>>,
    name: &str,
) -> Result<T, Box<dyn Error>> {
    Ok(serde_json::from_slice(
        files
            .get(name)
            .ok_or_else(|| invalid(format!("missing {name}")))?,
    )?)
}
/// The same checked publication loader serves every offline compiler host.
pub(crate) struct CheckedPriorBundle {
    pub(crate) input: SuccessorBundleInput,
    pub(crate) base: StagedOwnedRecipe,
    pub(crate) mapping: OwnedMappingIndex,
    pub(crate) tree: Option<TreeNormalizationPackageInput>,
    after: SuccessorBindings,
    previous_content: Option<poe_optimizer_import::owned_tree_policy::TreeNormalizationContent>,
}
impl CheckedPriorBundle {
    pub(crate) fn successor_input(&self, successor: OwnedRecipeInput) -> SuccessorBundleInput {
        let mut input = self.input.clone();
        input.successor = successor;
        input
    }
    pub(crate) fn check_transition(
        &self,
        staged: &StagedSuccessorBundle,
    ) -> Result<(), Box<dyn Error>> {
        if staged.transition().before != self.after {
            return Err(invalid("prior manifest endpoint identities differ"));
        }
        if let Some(previous) = &self.previous_content
            && staged.tree().map(|tree| &tree.input().content) != Some(previous)
        {
            return Err(invalid(
                "tree content revision needs an explicit migration; unchanged reruns are supported",
            ));
        }
        Ok(())
    }
}
pub(crate) fn load_checked_bundle(
    root: &Path,
    remaining: &mut usize,
    limits: SuccessorBundleLimits,
) -> Result<CheckedPriorBundle, Box<dyn Error>> {
    let PriorBundle { files, manifest } = load_bundle(root, remaining, limits)?;
    let tree_identity = manifest.tree;
    let prior: OwnedRecipeInput = decode(&files, "recipe.json")?;
    let base = assemble_owned_recipe(prior.clone(), limits.recipe)?;
    for artifact in base.artifacts() {
        if files.get(artifact.name()).map(Vec::as_slice) != Some(artifact.bytes()) {
            return Err(invalid(format!(
                "prior recipe and artifact disagree: {}",
                artifact.name()
            )));
        }
    }
    let mapping_input: MappingPackageInput = decode(&files, "mapping.json")?;
    let mapping = OwnedMappingIndex::new(
        mapping_input.clone(),
        base.registry(),
        base.schema(),
        limits.catalog.mapping,
    )?;
    let normalization = decode(&files, "normalization.json")?;
    let previous_tree: Option<TreeNormalizationPackageInput> = if tree_identity.is_some() {
        Some(decode(&files, "tree-normalization.json")?)
    } else {
        None
    };
    let previous_content = previous_tree
        .clone()
        .map(|tree| {
            let checked = OwnedTreeNormalizationPolicy::new(
                tree,
                base.registry(),
                base.schema(),
                &mapping,
                &normalization,
                limits.tree,
            )?;
            if Some(*checked.identity()) != tree_identity {
                return Err(invalid("prior tree identity differs"));
            }
            Ok::<_, Box<dyn Error>>(checked.input().content.clone())
        })
        .transpose()?;
    let queries = files
        .keys()
        .filter_map(|name| query_name(name).map(|label| (name, label)))
        .map(|(name, label)| {
            Ok(NamedQuerySet {
                name: OwnedDefinitionKey::new(label)?,
                queries: decode(&files, name)?,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
    if queries
        .iter()
        .try_fold(0usize, |sum, set| sum.checked_add(set.queries.len()))
        != Some(manifest.query_rows)
    {
        return Err(invalid("prior manifest query-row count differs"));
    }
    let items: ItemLinePolicyInput = decode(&files, "items.json")?;
    let item_source: ItemSourceLayoutPolicyInput = decode(&files, "item-source.json")?;
    let prior_items = OwnedItemLinePolicy::new(items.clone(), base.schema(), limits.items)?;
    let prior_source = ItemSourceLayoutPolicy::new(
        item_source.clone(),
        &prior_items,
        base.schema(),
        limits.item_source,
    )?;
    if *prior_items.identity() != manifest.items || *prior_source.identity() != manifest.item_source
    {
        return Err(invalid("prior manifest item-policy identities differ"));
    }
    if let Some(refinement) = &manifest.schema_refinement {
        if refinement.schema_version != 1
            || refinement.before != manifest.before.definitions
            || refinement.after != manifest.after.definitions
            || refinement.nodes.is_empty()
        {
            return Err(invalid("prior schema refinement metadata differs"));
        }
        let mut seen = std::collections::BTreeSet::new();
        for node in &refinement.nodes {
            let SchemaLookup::Known(schema) = base.schema().definition(node) else {
                return Err(invalid("prior schema refinement node is unknown"));
            };
            let declarations = &schema.declarations;
            if !seen.insert(node)
                || !declarations.parameters.is_complete()
                || !declarations.choices.is_complete()
                || !declarations.grants.is_complete()
                || !declarations.actors.is_complete()
                || !declarations.skill_grants.is_complete()
                || !declarations.outputs.is_complete()
                || !declarations.sockets.is_complete()
            {
                return Err(invalid("prior schema refinement nodes differ"));
            }
        }
    }
    Ok(CheckedPriorBundle {
        input: SuccessorBundleInput {
            schema_version: OWNED_SUCCESSOR_VERSION,
            successor: prior.clone(),
            prior,
            mapping: mapping_input,
            roles: decode(&files, "roles.json")?,
            normalization,
            rewards: decode(&files, "rewards.json")?,
            query_sets: queries,
            items,
            item_source,
        },
        base,
        mapping,
        tree: previous_tree,
        after: manifest.after,
        previous_content,
    })
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let catalog: TreeCatalogInput = serde_json::from_slice(&read(&args.catalog, &mut remaining)?)?;
    let policy: TreeCatalogPolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let compiled = compile_owned_tree_catalog_extension(
        &prior.base,
        &prior.mapping,
        &catalog,
        &policy,
        TreeCatalogLimits::default(),
    )?;
    let tree = TreePolicyTransitionInput::Install {
        content: Box::new(compiled.content),
    };
    let staged = transition_owned_catalog_with_tree(
        prior.successor_input(compiled.successor),
        CatalogAppend {
            mappings: compiled.new_mappings,
            source: catalog.source,
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        tree,
        limits,
    )?;
    prior.check_transition(&staged)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, staged.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"catalog":compiled.receipt,"publication":staged.transition()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
