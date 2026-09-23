//! Source-layout conversion uses finite inputs; source acquisition is optional.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_item_layouts::{ItemLayoutLimits, ItemLayoutPolicy, compile_owned_item_layouts},
    owned_item_lines::OwnedItemLinePolicy,
    owned_item_source::ItemSourceLayoutPolicy,
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};
#[derive(clap::Args)]
pub(crate) struct Args {
    /// Checked prior owned bundle.
    input: PathBuf,
    /// Finite generated-prefix evidence; never source code.
    #[arg(long)]
    catalog: PathBuf,
    /// Exact artifact identities and existing template/header bindings.
    #[arg(long)]
    policy: PathBuf,
    /// New destination; existing directories are never replaced.
    #[arg(long)]
    output: PathBuf,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let converter = ItemLayoutLimits::default();
    let mut remaining = converter.max_catalog_bytes;
    let catalog = read(&args.catalog, &mut remaining)?;
    let mut remaining = converter.max_policy_bytes;
    let policy: ItemLayoutPolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let lines =
        OwnedItemLinePolicy::new(prior.input.items.clone(), prior.base.schema(), limits.items)?;
    let source = ItemSourceLayoutPolicy::new(
        prior.input.item_source.clone(),
        &lines,
        prior.base.schema(),
        limits.item_source,
    )?;
    let compiled = compile_owned_item_layouts(
        &lines,
        &source,
        prior.base.schema(),
        &catalog,
        &policy,
        converter,
    )?;
    let mut input = prior.successor_input(prior.input.prior.clone());
    input.item_source = compiled.item_source;
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::SuppliedSuccessor,
    };
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("item-layout conversion requires a prior tree policy"))?,
        ),
    };
    let finalized = transition_owned_catalog_with_tree_compact(input, append, tree, limits)?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"item_layouts":compiled.receipt,"publication":finalized.transition()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
#[cfg(feature = "pob")]
#[derive(clap::Args)]
pub(crate) struct ExportArgs {
    /// Pinned checkout used only for offline data construction.
    #[arg(long)]
    source_root: PathBuf,
    /// New directory for finite catalog and acquisition evidence.
    #[arg(long)]
    output: PathBuf,
}
#[cfg(feature = "pob")]
pub(crate) fn export(args: ExportArgs) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_item_layouts::export_owned_item_layouts(
        &args.source_root,
        Default::default(),
    )?;
    let mut evidence = serde_json::to_vec_pretty(result.evidence())?;
    evidence.push(b'\n');
    super::owned_recipe_cli::publish_artifacts(
        &args.output,
        [
            ("catalog.json", result.catalog_bytes()),
            ("evidence.json", evidence.as_slice()),
            ("base-catalog.json", result.base_export().catalog_bytes()),
        ],
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"item_layouts":result.evidence(),"bytes":result.catalog_bytes().len()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
