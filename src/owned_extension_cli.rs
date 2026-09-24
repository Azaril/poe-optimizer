//! Publish directly authored owned rule data through the checked successor seam.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, RecipeExtensionLimits, extend_owned_recipe},
    owned_recipe_membership_patch::{
        RecipeMembershipPatchLimits, compile_owned_recipe_membership_patch,
        decode_recipe_membership_patch,
    },
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
    /// Optional finite membership additions bound to the exact prior and extension.
    #[arg(long)]
    membership_patch: Option<PathBuf>,
    /// Exact successor-bound item line policy; requires --item-source.
    #[arg(long, requires = "item_source")]
    items: Option<PathBuf>,
    /// Exact source layout policy bound to --items; requires --items.
    #[arg(long, requires = "items")]
    item_source: Option<PathBuf>,
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
    let (extended, membership_receipt) = if let Some(path) = args.membership_patch {
        let patch_limits = RecipeMembershipPatchLimits {
            extension: extension_limits,
            ..Default::default()
        };
        let mut remaining = patch_limits.max_request_bytes;
        let patch = decode_recipe_membership_patch(&read(&path, &mut remaining)?, patch_limits)?;
        let patched =
            compile_owned_recipe_membership_patch(&prior.base, &extension, &patch, patch_limits)?;
        (patched.staged, Some(patched.receipt))
    } else {
        (
            extend_owned_recipe(&prior.base, &extension, extension_limits)?,
            None,
        )
    };
    let mut input = prior.successor_input(extended.successor);
    let item_policies = match (args.items, args.item_source) {
        (None, None) => CatalogItemPolicyMode::RebindPrior,
        (Some(items), Some(source)) => {
            // Bound each supplied wire before decoding. The shared finalizer
            // validates the exact successor schema and item-line digest; this
            // explicit mode never rewrites a stale caller binding.
            let mut remaining = limits.items.max_wire_bytes;
            input.items = serde_json::from_slice(&read(&items, &mut remaining)?)?;
            let mut remaining = limits.item_source.max_wire_bytes;
            input.item_source = serde_json::from_slice(&read(&source, &mut remaining)?)?;
            CatalogItemPolicyMode::SuppliedSuccessor
        }
        _ => {
            return Err(invalid(
                "--items and --item-source must be supplied together",
            ));
        }
    };
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies,
    };
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("recipe extension requires a prior tree policy"))?,
        ),
    };
    let finalized = match extended.refinement {
        Some(refinement) => transition_owned_catalog_with_membership_refinement_compact(
            input, append, tree, refinement, limits,
        )?,
        None => transition_owned_catalog_with_tree_compact(input, append, tree, limits)?,
    };
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    let mut receipt = serde_json::json!({
        "extension": extended.receipt, "publication": finalized.transition(),
    });
    if let Some(membership) = membership_receipt {
        receipt["membership_patch"] = serde_json::to_value(membership)?;
    }
    serde_json::to_writer(&mut stdout, &receipt)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
