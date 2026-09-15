//! Offline attribute recipe publication from caller-supplied definitions and policy.
use super::owned_tree_cli::{invalid, load_checked_bundle, read};
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::{StatDefId, StatDefinition},
    owned_schema::{
        DefinitionAddress, DefinitionDescriptor, DefinitionEntry, DefinitionSchemaIndex,
        RuleEntityKind, SchemaState, StatSchema,
    },
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_attribute_recipe::{
        AttributeRecipeLimits, AttributeRecipePolicy, compile_owned_attribute_recipe,
    },
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
    /// Reviewed attribute-choice conversion policy, including exact source facts.
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
    if entries.len() > limits.recipe.schema.max_entries {
        return Err(invalid("statistics entry limit"));
    }
    if entries.windows(2).any(|pair| pair[0].id >= pair[1].id) {
        return Err(invalid("statistics must be in unique canonical ID order"));
    }
    let mut registry = base.registry().clone();
    let mut schema = base.schema().input().clone();
    for entry in entries {
        if !matches!(&entry.schema, SchemaState::Known(stat) if stat.targets == [RuleEntityKind::Actor])
        {
            return Err(invalid("statistics must be known Actor descriptors"));
        }
        let descriptor = DefinitionDescriptor::Stat(entry.clone());
        match base
            .schema()
            .lookup_definition(&DefinitionAddress::Stat(entry.id.clone()))
        {
            Some(previous) if previous == &descriptor => {}
            Some(_) => return Err(invalid("statistics changed an existing descriptor")),
            None => {
                let allocated = registry.allocate_definition::<StatDefinition>()?;
                if allocated != entry.id {
                    return Err(invalid(
                        "statistics ID is not the next canonical registry allocation",
                    ));
                }
                schema.definitions.push(descriptor);
            }
        }
    }
    base.registry().validate_successor(&registry)?;
    let schema = OwnedDefinitionSchemaPackage::new(schema, limits.recipe.schema)?;
    let mut successor = prior.clone();
    successor.registry = registry.input().clone();
    successor.schema = schema.input().clone();
    successor.rules.definitions = schema.identity().clone();
    successor.routing.definitions = schema.identity().clone();
    Ok(successor)
}

pub(crate) fn run(args: Args) -> Result<(), Box<dyn Error>> {
    let limits = SuccessorBundleLimits::default();
    let mut remaining = limits.max_input_bytes;
    let prior = load_checked_bundle(&args.input, &mut remaining, limits)?;
    let catalog: TreeCatalogInput = serde_json::from_slice(&read(&args.catalog, &mut remaining)?)?;
    let policy: AttributeRecipePolicy =
        serde_json::from_slice(&read(&args.policy, &mut remaining)?)?;
    let statistics = serde_json::from_slice(&read(&args.statistics, &mut remaining)?)?;
    let previous_tree = prior
        .tree
        .clone()
        .ok_or_else(|| invalid("attribute compilation requires a prior tree policy"))?;
    if previous_tree.content.catalog
        != digest_owned(
            "owned-tree-catalog-v1",
            &catalog,
            TreeCatalogLimits::default().max_wire_bytes,
        )?
    {
        return Err(invalid(
            "attribute catalog differs from the prior tree policy",
        ));
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
    let compiled = compile_owned_attribute_recipe(
        staged.assembled(),
        staged.mapping(),
        &catalog,
        &policy,
        AttributeRecipeLimits::default(),
    )?;
    let finalized = if compiled.refined.is_empty() {
        transition_owned_catalog_with_tree(
            prior.successor_input(compiled.successor),
            append,
            tree,
            limits,
        )?
    } else {
        let nodes = compiled
            .refined
            .into_iter()
            .map(|address| match address {
                DefinitionAddress::PassiveNode(id) => Ok(id),
                _ => Err(invalid(
                    "attribute refinement contains a non-passive definition",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;
        transition_owned_catalog_with_tree_refinement(
            prior.successor_input(compiled.successor),
            append,
            tree,
            PassiveDeclarationRefinement {
                schema_version: 1,
                before: prior.base.schema().identity().clone(),
                after: compiled.receipt.after_definitions.clone(),
                nodes,
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
            "attributes": compiled.receipt,
            "publication": finalized.transition(),
        }),
    )?;
    stdout.write_all(b"\n")?;
    Ok(())
}
