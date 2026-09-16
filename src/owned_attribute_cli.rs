//! Shared offline passive recipe publication from caller-supplied definitions and policy.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::StatDefId,
    owned_schema::{DefinitionAddress, DefinitionDescriptor, DefinitionEntry, StatSchema},
};
use poe_optimizer_import::{
    owned_attribute_recipe::{
        AttributeRecipeLimits, AttributeRecipePolicy, compile_owned_attribute_recipe,
    },
    owned_passive_views::{ViewRecipeLimits, ViewRecipePolicy, compile_owned_passive_views},
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, PassiveDeclarationRefinement, SuccessorBundleLimits,
        TreePolicyTransitionInput, transition_owned_catalog_with_tree,
        transition_owned_catalog_with_tree_refinement,
    },
    owned_tree_catalog::{TreeCatalogInput, TreeCatalogLimits},
};
use std::{
    error::Error,
    io::{self, Write},
    path::PathBuf,
};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Prior published owned bundle directory, including its artifact manifest.
    input: PathBuf,
    /// Finite exported tree catalog. Never source code or a character build.
    #[arg(long)]
    catalog: PathBuf,
    /// Reviewed passive conversion policy, including exact source facts.
    #[arg(long)]
    policy: PathBuf,
    /// Ordered owned statistics descriptors; new IDs must match the registry sequence.
    #[arg(long)]
    statistics: PathBuf,
    /// New destination directory; existing output is never replaced.
    #[arg(long)]
    output: PathBuf,
}

fn augment_statistics(
    base: &StagedOwnedRecipe,
    prior: &OwnedRecipeInput,
    entries: Vec<DefinitionEntry<StatDefId, StatSchema>>,
    limits: SuccessorBundleLimits,
) -> Result<OwnedRecipeInput, Box<dyn Error>> {
    augment_definitions(
        base,
        prior,
        entries
            .into_iter()
            .map(DefinitionDescriptor::Stat)
            .collect(),
        limits,
    )
}

/// Keep older numeric hosts restricted to their Actor-stat/unit contract.
pub(crate) fn augment_definitions(
    base: &StagedOwnedRecipe,
    prior: &OwnedRecipeInput,
    entries: Vec<DefinitionDescriptor>,
    limits: SuccessorBundleLimits,
) -> Result<OwnedRecipeInput, Box<dyn Error>> {
    super::owned_definition_cli::append_definitions(
        base,
        prior,
        entries,
        super::owned_definition_cli::AppendKind::ActorNumbers,
        limits,
    )
}

enum Conversion {
    Attributes,
    Views,
}
impl Conversion {
    fn label(&self) -> &'static str {
        match self {
            Self::Attributes => "attribute",
            Self::Views => "passive view",
        }
    }
    fn receipt_key(&self) -> &'static str {
        match self {
            Self::Attributes => "attributes",
            Self::Views => "views",
        }
    }
}
pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    run_conversion(args, Conversion::Attributes)
}
pub(crate) fn run_views(args: Args) -> Result<(), Box<dyn Error>> {
    run_conversion(args, Conversion::Views)
}
fn run_conversion(args: Args, conversion: Conversion) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let catalog: TreeCatalogInput = serde_json::from_slice(&read(&args.catalog, &mut remaining)?)?;
    let policy_bytes = read(&args.policy, &mut remaining)?;
    let statistics = serde_json::from_slice(&read(&args.statistics, &mut remaining)?)?;
    let previous_tree = prior.tree.clone().ok_or_else(|| {
        invalid(format!(
            "{} compilation requires a prior tree policy",
            conversion.label()
        ))
    })?;
    if previous_tree.content.catalog
        != digest_owned(
            "owned-tree-catalog-v1",
            &catalog,
            TreeCatalogLimits::default().max_wire_bytes,
        )?
    {
        return Err(invalid(format!(
            "{} catalog differs from the prior tree policy",
            conversion.label()
        )));
    }
    let tree = TreePolicyTransitionInput::RebindPrior {
        prior: Box::new(previous_tree),
    };
    let append = CatalogAppend {
        mappings: vec![],
        source: prior.mapping.input().source.clone(),
        item_policies: CatalogItemPolicyMode::RebindPrior,
    };
    let augmented = augment_statistics(&prior.base, &prior.input.prior, statistics, limits)?;
    // Validate all predecessor bindings before rebinding for compiler input. No
    // artifact is published until the final transition also validates.
    let staged = transition_owned_catalog_with_tree(
        prior.successor_input(augmented),
        append.clone(),
        tree.clone(),
        limits,
    )?;
    prior.check_transition(&staged)?;
    let (successor, refined, after_definitions, receipt) = match &conversion {
        Conversion::Attributes => {
            let policy: AttributeRecipePolicy = serde_json::from_slice(&policy_bytes)?;
            let compiled = compile_owned_attribute_recipe(
                staged.assembled(),
                staged.mapping(),
                &catalog,
                &policy,
                AttributeRecipeLimits::default(),
            )?;
            (
                compiled.successor,
                compiled.refined,
                compiled.receipt.after_definitions.clone(),
                serde_json::to_value(compiled.receipt)?,
            )
        }
        Conversion::Views => {
            let policy: ViewRecipePolicy = serde_json::from_slice(&policy_bytes)?;
            let compiled = compile_owned_passive_views(
                staged.assembled(),
                staged.mapping(),
                &catalog,
                &policy,
                ViewRecipeLimits::default(),
            )?;
            (
                compiled.successor,
                compiled.refined,
                compiled.receipt.after_definitions.clone(),
                serde_json::to_value(compiled.receipt)?,
            )
        }
    };
    let finalized = if refined.is_empty() {
        transition_owned_catalog_with_tree(prior.successor_input(successor), append, tree, limits)?
    } else {
        let nodes = refined
            .into_iter()
            .map(|address| match address {
                DefinitionAddress::PassiveNode(id) => Ok(id),
                _ => Err(invalid(
                    "passive refinement contains a non-passive definition",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        transition_owned_catalog_with_tree_refinement(
            prior.successor_input(successor),
            append,
            tree,
            PassiveDeclarationRefinement {
                schema_version: 1,
                before: prior.base.schema().identity().clone(),
                after: after_definitions,
                nodes,
            },
            limits,
        )?
    };
    prior.check_transition(&finalized)?;
    super::owned_recipe_cli::publish_artifacts(&args.output, finalized.artifacts())?;
    let mut stdout = io::stdout().lock();
    let mut report = serde_json::Map::new();
    report.insert(conversion.receipt_key().into(), receipt);
    report.insert(
        "publication".into(),
        serde_json::to_value(finalized.transition())?,
    );
    serde_json::to_writer(&mut stdout, &report)?;
    stdout.write_all(b"\n")?;
    Ok(())
}
