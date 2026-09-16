//! Offline class-base source conversion and checked owned-bundle publication.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_import::{
    owned_class_bases::{ClassBaseRecipeLimits, ClassBaseRecipePolicy, compile_owned_class_bases},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, DeclarationClosureRefinement,
        DeclarationRefinementOwner, SuccessorBundleLimits, TreePolicyTransitionInput,
        transition_owned_catalog_with_declaration_refinement, transition_owned_catalog_with_tree,
    },
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Prior published owned bundle, including its checked artifact manifest.
    input: PathBuf,
    /// Raw source tree JSON used only for offline conversion; its pin must match.
    #[arg(long)]
    source_tree: PathBuf,
    /// Reviewed source field to owned-stat bindings.
    #[arg(long)]
    policy: PathBuf,
    /// New destination directory; an existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let source = read(&args.source_tree, &mut remaining)?;
    let policy: ClassBaseRecipePolicy =
        serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(
            prior
                .tree
                .clone()
                .ok_or_else(|| invalid("class-base compilation requires a prior tree policy"))?,
        ),
    };
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let compiled = compile_owned_class_bases(
        &prior.base,
        &prior.mapping,
        &source,
        &policy,
        ClassBaseRecipeLimits::default(),
    )?;
    let finalized = if compiled.refined.is_empty() {
        transition_owned_catalog_with_tree(
            prior.successor_input(compiled.successor),
            append,
            tree,
            limits,
        )?
    } else {
        transition_owned_catalog_with_declaration_refinement(
            prior.successor_input(compiled.successor),
            append,
            tree,
            DeclarationClosureRefinement {
                schema_version: 2,
                before: prior.base.schema().identity().clone(),
                after: compiled.receipt.after_definitions.clone(),
                owners: compiled
                    .refined
                    .into_iter()
                    .map(DeclarationRefinementOwner::Class)
                    .collect(),
            },
            limits,
        )?
    };
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    serde_json::to_writer(
        &mut stdout,
        &serde_json::json!({
            "class_bases": compiled.receipt,
            "publication": finalized.transition(),
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
