//! Offline numeric recipe compilation; host I/O does not interpret game effects.
use super::owned_tree_cli::{load_checked_bundle, read};
use poe_optimizer_import::{
    owned_modifier_value_recipe::{
        ModifierValuePolicy, ModifierValueRecipeLimits, compile_owned_modifier_values,
    },
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_bundle_compact, transition_owned_catalog_with_tree_compact,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Checked prior bundle with explicit canonical input and output declarations.
    input: PathBuf,
    /// Schema-bound component slots, precision, scalar stages and sign policy.
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
    let recipe_limits = ModifierValueRecipeLimits::default();
    let mut remaining = recipe_limits.max_policy_bytes;
    let policy: ModifierValuePolicy = serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let compiled = compile_owned_modifier_values(&prior.base, &policy, recipe_limits)?;
    let input = prior.successor_input(compiled.successor);
    let finalized = if let Some(tree) = &prior.tree {
        transition_owned_catalog_with_tree_compact(
            input,
            CatalogAppend {
                mappings: vec![],
                source: prior.mapping.input().source.clone(),
                item_policies: CatalogItemPolicyMode::RebindPrior,
            },
            TreePolicyTransitionInput::RebindPrior {
                prior: Box::new(tree.clone()),
            },
            limits,
        )?
    } else {
        transition_owned_bundle_compact(input, limits)?
    };
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "modifier_values": compiled.receipt,
            "publication": finalized.transition(),
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
