//! Independent checked headers for migration tests; no real-build defaults.
#[allow(dead_code)]
#[path = "../../../../tests/support/owned_evaluation_release_fixture.rs"]
mod evaluation;
use poe_optimizer_core::{
    owned_definitions::OwnedDefinitionKey, owned_rules::RuleOperationsVersion,
    owned_schema::DeclaredSet,
};
use poe_optimizer_data::{
    owned_schema::OwnedDefinitionSchemaPackage,
    owned_stages::OwnedEvaluationStages,
    owned_support_inputs::OwnedSupportInputBindings,
    owned_support_outputs::{OwnedSupportOutputBindings, SupportOutputDependencies},
    owned_support_receiving::OwnedSupportReceiving,
    owned_supports::OwnedSupportPreparation,
};
use poe_optimizer_import::{
    owned_item_lines::OwnedItemLinePolicy, owned_mapping::OwnedMappingIndex,
    owned_normalize::GemQualityPolicy, owned_recipe::assemble_owned_recipe, owned_release::*,
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};

pub fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

/// Build independently checked predecessor contracts from the finite fixture.
/// Rebind only constituent identities; preserve their complete semantic inputs.
pub fn contract(
    mut input: OwnedReleaseInput,
    schema_version: u32,
    operations: &str,
    release: &str,
) -> StagedOwnedRelease {
    input.recipe.schema.schema_version = schema_version;
    input.recipe.schema.release = key(release);
    let definitions =
        OwnedDefinitionSchemaPackage::new(input.recipe.schema.clone(), Default::default()).unwrap();
    input.recipe.rules.definitions = definitions.identity().clone();
    input.recipe.rules.operations_version = key(operations);
    let applications = RuleOperationsVersion::parse(operations)
        .unwrap()
        .supports_effect_applications();
    if applications && input.recipe.rules.effect_applications.is_none() {
        // This finite fixture has no source/recipient applications. V15+
        // requires that reviewed empty inventory to be authored explicitly.
        input.recipe.rules.effect_applications = Some(DeclaredSet::complete(vec![]));
    }
    input.recipe.routing.definitions = definitions.identity().clone();
    let recipe = assemble_owned_recipe(input.recipe.clone(), Default::default()).unwrap();
    input.mapping.definitions = definitions.identity().clone();
    let mapping = OwnedMappingIndex::new(
        input.mapping.clone(),
        recipe.registry(),
        &definitions,
        Default::default(),
    )
    .unwrap();
    input.roles.definitions = definitions.identity().clone();
    input.roles.mapping = *mapping.identity();
    if let GemQualityPolicy::Attributes(quality) = &mut input.normalization.gem_quality {
        quality.definitions = definitions.identity().clone();
    }
    if let Some(gems) = &mut input.normalization.gem_inputs {
        gems.definitions = definitions.identity().clone();
    }
    input.rewards.definitions = definitions.identity().clone();
    input.rewards.mapping = *mapping.identity();
    input.items.definitions = definitions.identity().clone();
    let items =
        OwnedItemLinePolicy::new(input.items.clone(), &definitions, Default::default()).unwrap();
    input.item_source.item_lines = *items.identity();
    if let Some(tree) = input.tree.take() {
        input.tree = Some(
            OwnedTreeNormalizationPolicy::bind_new(
                tree.content,
                recipe.registry(),
                &definitions,
                &mapping,
                &input.normalization,
                Default::default(),
            )
            .unwrap()
            .input()
            .clone(),
        );
    }
    if let Some(evaluation) = &mut input.evaluation {
        evaluation.metrics.definitions = definitions.identity().clone();
        if let Some(s) = &mut evaluation.support {
            if applications && s.stages.effect_applications.is_none() {
                s.stages.effect_applications = Some(DeclaredSet::complete(vec![]));
            }
            s.stages.definitions = definitions.identity().clone();
            s.stages.rules = *recipe.rules().identity();
            s.stages.routing = *recipe.routing().identity();
            let stages = OwnedEvaluationStages::new(
                s.stages.clone(),
                &definitions,
                recipe.rules(),
                recipe.routing(),
                Default::default(),
            )
            .unwrap();
            s.preparation.definitions = definitions.identity().clone();
            s.preparation.rules = *recipe.rules().identity();
            let preparation = OwnedSupportPreparation::new(
                s.preparation.clone(),
                &definitions,
                recipe.rules(),
                Default::default(),
            )
            .unwrap();
            s.inputs.definitions = definitions.identity().clone();
            s.inputs.rules = *recipe.rules().identity();
            s.inputs.preparation = *preparation.identity();
            s.inputs.stages = *stages.identity();
            let inputs = OwnedSupportInputBindings::new(
                s.inputs.clone(),
                &definitions,
                recipe.rules(),
                &preparation,
                &stages,
                Default::default(),
            )
            .unwrap();
            s.receiving.definitions = definitions.identity().clone();
            s.receiving.rules = *recipe.rules().identity();
            s.receiving.preparation = *preparation.identity();
            s.receiving.inputs = *inputs.identity();
            s.receiving.stages = *stages.identity();
            let receiving = OwnedSupportReceiving::new(
                s.receiving.clone(),
                &definitions,
                recipe.rules(),
                &preparation,
                &inputs,
                &stages,
                Default::default(),
            )
            .unwrap();
            if let Some(outputs) = &mut s.outputs {
                outputs.definitions = definitions.identity().clone();
                outputs.rules = *recipe.rules().identity();
                outputs.preparation = *preparation.identity();
                outputs.inputs = *inputs.identity();
                outputs.receiving = *receiving.identity();
                outputs.stages = *stages.identity();
                OwnedSupportOutputBindings::new(
                    outputs.clone(),
                    &SupportOutputDependencies {
                        definitions: &definitions,
                        rules: recipe.rules(),
                        preparation: &preparation,
                        inputs: &inputs,
                        receiving: &receiving,
                        stages: &stages,
                    },
                    Default::default(),
                )
                .unwrap();
            }
        }
    }
    assemble_owned_release(input, Default::default()).unwrap()
}

pub fn evaluated_prior() -> StagedOwnedRelease {
    contract(
        evaluation::fixture().input,
        4,
        "owned-domain-operations-v15",
        "evaluated-v15",
    )
}
