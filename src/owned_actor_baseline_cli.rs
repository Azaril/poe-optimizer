//! Native offline actor conversion with explicitly schema-bound targets.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_actor_baseline_recipe::{
        ActorBaselinePolicy, ActorBaselineRecipeLimits, compile_owned_actor_baselines,
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
    /// Checked prior bundle with the policy's actor/stat declarations already present.
    input: PathBuf,
    /// Finite catalog; this command never loads or executes source scripts.
    #[arg(long)]
    catalog: PathBuf,
    /// Explicit actor targets, scalar fields, curve selection and level bindings.
    #[arg(long)]
    policy: PathBuf,
    /// New output directory; existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let actor_limits = ActorBaselineRecipeLimits::default();
    let mut remaining = actor_limits.max_catalog_bytes;
    let catalog = read(&args.catalog, &mut remaining)?;
    let mut remaining = actor_limits.max_policy_bytes;
    let policy: ActorBaselinePolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let compiled = compile_owned_actor_baselines(&prior.base, &catalog, &policy, actor_limits)?;
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let tree =
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(prior.tree.clone().ok_or_else(|| {
                invalid("actor baseline compilation requires a prior tree policy")
            })?),
        };
    let finalized = transition_owned_catalog_with_tree_compact(
        prior.successor_input(compiled.successor),
        append,
        tree,
        limits,
    )?;
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({"actor_baselines":compiled.receipt,"publication":finalized.transition()}),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
