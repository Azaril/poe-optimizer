//! Offline display-observation compilation; native use reads only finite data.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_item_observation_policy::{
        ItemObservationPolicyLimits, ItemObservationPolicyRequest, compile_owned_item_observations,
    },
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
    /// Checked prior bundle; no source checkout is needed.
    input: PathBuf,
    /// Finite constructed-base facts from offline acquisition.
    #[arg(long)]
    catalog: PathBuf,
    /// Reviewed display grammar, family and exact catalog digest.
    #[arg(long)]
    policy: PathBuf,
    /// New destination; an existing directory is never replaced.
    #[arg(long)]
    output: PathBuf,
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let compiler_limits = ItemObservationPolicyLimits::default();
    let mut remaining = compiler_limits.catalog.max_catalog_bytes;
    let catalog = read(&args.catalog, &mut remaining)?;
    let mut remaining = compiler_limits.max_request_bytes;
    let policy: ItemObservationPolicyRequest =
        serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let lines = poe_optimizer_import::owned_item_lines::OwnedItemLinePolicy::new(
        prior.input.items.clone(),
        prior.base.schema(),
        compiler_limits.items,
    )?;
    let source = poe_optimizer_import::owned_item_source::ItemSourceLayoutPolicy::new(
        prior.input.item_source.clone(),
        &lines,
        prior.base.schema(),
        compiler_limits.item_source,
    )?;
    let compiled = compile_owned_item_observations(
        &catalog,
        &policy,
        &lines,
        &source,
        prior.base.schema(),
        compiler_limits,
    )?;
    let mut input = prior.successor_input(prior.input.prior.clone());
    input.items = compiled.items;
    input.item_source = compiled.item_source;
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("observation compilation requires a prior tree policy"))?,
        ),
    };
    let finalized = transition_owned_catalog_with_tree_compact(
        input,
        CatalogAppend {
            mappings: vec![],
            source: prior.mapping.input().source.clone(),
            item_policies: CatalogItemPolicyMode::SuppliedSuccessor,
        },
        tree,
        limits,
    )?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "observations": compiled.receipt, "publication": finalized.transition(),
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}

#[cfg(feature = "pob")]
#[derive(clap::Args)]
pub(crate) struct ExportArgs {
    /// Optional pinned reference checkout; used only for offline acquisition.
    #[arg(long)]
    source_root: PathBuf,
    /// New destination receiving finite catalog and authentication evidence.
    #[arg(long)]
    output: PathBuf,
}
#[cfg(feature = "pob")]
pub(crate) fn export(args: ExportArgs) -> Result<(), Box<dyn Error>> {
    let result = poe_optimizer_pob::owned_item_observations::export_owned_item_observations(
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
        ],
    )?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(&mut stdout, result.evidence())?;
    stdout.write_all(b"\n")?;
    Ok(())
}
