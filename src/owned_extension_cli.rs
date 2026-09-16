//! Publish directly authored owned rule data through the checked successor seam.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, RecipeExtensionLimits, extend_owned_recipe},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_membership_refinement_compact,
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
    /// Checked prior owned bundle directory.
    input: PathBuf,
    /// Project-owned schema additions and rule expressions; never source code.
    #[arg(long)]
    extension: PathBuf,
    /// New destination directory; an existing directory is never replaced.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let extension_limits = RecipeExtensionLimits::default();
    let mut remaining = extension_limits.max_wire_bytes;
    let extension: OwnedRecipeExtension =
        serde_json::from_slice(&read(&args.extension, &mut remaining)?)?;
    let extended = extend_owned_recipe(&prior.base, &extension, extension_limits)?;
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("recipe extension requires a prior tree policy"))?,
        ),
    };
    let input = prior.successor_input(extended.successor);
    let finalized = match extended.refinement {
        Some(refinement) => transition_owned_catalog_with_membership_refinement_compact(
            input, append, tree, refinement, limits,
        )?,
        None => transition_owned_catalog_with_tree_compact(input, append, tree, limits)?,
    };
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "extension": extended.receipt, "publication": finalized.transition(),
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
