//! Shared host-side registry appends with explicit per-command definition kinds.
use super::owned_tree_cli::invalid;
use poe_optimizer_core::{
    owned_definitions::{
        CapabilityDefinition, ItemTemplateDefinition, StatDefinition, UnitDefinition,
    },
    owned_schema::{
        DefinitionAddress, DefinitionDescriptor, DefinitionSchemaIndex, RuleEntityKind, SchemaState,
    },
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe},
    owned_successor::SuccessorBundleLimits,
};
use std::error::Error;

pub(crate) enum AppendKind {
    ActorNumbers,
    ItemBases,
}

/// IDs are supplied by the authoring policy, then checked against the single
/// registry sequence. Existing descriptors must match exactly. Forward unit or
/// template references are validated only after the complete append is assembled.
pub(crate) fn append_definitions(
    base: &StagedOwnedRecipe,
    prior: &OwnedRecipeInput,
    entries: Vec<DefinitionDescriptor>,
    kind: AppendKind,
    limits: SuccessorBundleLimits,
) -> Result<OwnedRecipeInput, Box<dyn Error>> {
    if entries.len() > limits.recipe.schema.max_entries {
        return Err(invalid("definition entry limit"));
    }
    if entries
        .windows(2)
        .any(|pair| pair[0].address().key() >= pair[1].address().key())
    {
        return Err(invalid("definitions must be in unique canonical ID order"));
    }
    let mut registry = base.registry().clone();
    let mut schema = base.schema().input().clone();
    for descriptor in entries {
        let allowed = match (&kind, &descriptor) {
            (AppendKind::ActorNumbers, DefinitionDescriptor::Stat(entry)) => {
                matches!(&entry.schema,
                SchemaState::Known(stat) if stat.targets == [RuleEntityKind::Actor])
            }
            (AppendKind::ActorNumbers, DefinitionDescriptor::Unit(entry)) => {
                matches!(&entry.schema, SchemaState::Known(_))
            }
            (AppendKind::ItemBases, DefinitionDescriptor::ItemTemplate(entry)) => {
                matches!(&entry.schema, SchemaState::Known(_))
            }
            (AppendKind::ItemBases, DefinitionDescriptor::Capability(entry)) => {
                matches!(&entry.schema,
                SchemaState::Known(capability) if capability.targets == [RuleEntityKind::EquipmentUse])
            }
            _ => false,
        };
        if !allowed {
            return Err(invalid(match kind {
                AppendKind::ActorNumbers => "definitions must be known Actor statistics or units",
                AppendKind::ItemBases => {
                    "definitions must be known ItemTemplate or EquipmentUse Capability descriptors"
                }
            }));
        }
        match base.schema().lookup_definition(&descriptor.address()) {
            Some(previous) if previous == &descriptor => {}
            Some(_) => return Err(invalid("definition append changed an existing descriptor")),
            None => {
                let allocated = match &descriptor {
                    DefinitionDescriptor::Stat(_) => {
                        DefinitionAddress::Stat(registry.allocate_definition::<StatDefinition>()?)
                    }
                    DefinitionDescriptor::Unit(_) => {
                        DefinitionAddress::Unit(registry.allocate_definition::<UnitDefinition>()?)
                    }
                    DefinitionDescriptor::ItemTemplate(_) => DefinitionAddress::ItemTemplate(
                        registry.allocate_definition::<ItemTemplateDefinition>()?,
                    ),
                    DefinitionDescriptor::Capability(_) => DefinitionAddress::Capability(
                        registry.allocate_definition::<CapabilityDefinition>()?,
                    ),
                    _ => unreachable!("validated definition kind"),
                };
                if allocated != descriptor.address() {
                    return Err(invalid(
                        "definition ID is not the next canonical registry allocation",
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
