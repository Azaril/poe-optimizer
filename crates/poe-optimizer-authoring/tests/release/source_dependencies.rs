//! Original dependency seals are checked before source composition. That check
//! is not a certificate for executable rules or evaluation artifacts.
use super::{DslRelease, ReleaseBuildError, fixture, tracked_release};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{GameVersionNamespace, OwnedDefinitionKey},
    owned_schema::{DefinitionAddress, SchemaSubject},
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_recipe::OwnedRecipeError,
    owned_release::{
        OwnedReleaseError, OwnedReleaseInput, OwnedReleaseLimits, check_release_source_dependencies,
    },
};

fn other_digest() -> OwnedContentDigest {
    digest_owned("authoring-source-unrelated", &37, 100).unwrap()
}

fn shell(input: &OwnedReleaseInput) -> OwnedReleaseInput {
    let mut input = input.clone();
    for owner in &mut input.recipe.rules.owners {
        owner.programs.members.clear();
    }
    input
}

#[test]
fn original_dependency_seals_and_source_pins_cannot_be_repaired_by_source_checking() {
    let input = shell(&fixture().input);
    let source = check_release_source_dependencies(&input, Default::default()).unwrap();
    assert_eq!(source.schema().identity(), &input.recipe.rules.definitions);
    let mutations: &[fn(&mut OwnedReleaseInput)] = &[
        |v| v.recipe.schema.release = OwnedDefinitionKey::new("changed-schema").unwrap(),
        |v| v.recipe.rules.definitions.content_sha256 = "1".repeat(64),
        |v| v.recipe.rules.namespace = GameVersionNamespace::new("foreign", "v1").unwrap(),
        |v| v.recipe.routing.definitions.content_sha256 = "1".repeat(64),
        |v| v.mapping.registry = other_digest(),
        |v| v.mapping.definitions.content_sha256 = "1".repeat(64),
        |v| v.roles.mapping = other_digest(),
        |v| v.roles.definitions.content_sha256 = "1".repeat(64),
        |v| v.rewards.mapping = other_digest(),
        |v| v.rewards.definitions.content_sha256 = "1".repeat(64),
        |v| v.items.definitions.content_sha256 = "1".repeat(64),
        |v| v.item_source.item_lines = other_digest(),
        |v| v.normalization.namespace = GameVersionNamespace::new("foreign", "v1").unwrap(),
        |v| v.item_source.source.revision = "1".repeat(40),
        |v| v.item_source.source.files[0].sha256 = "1".repeat(64),
    ];
    for (index, mutate) in mutations.iter().enumerate() {
        let mut bad = input.clone();
        mutate(&mut bad);
        assert_ne!(bad, input, "mutation {index}");
        let before = serde_json::to_vec(&bad).unwrap();
        assert!(
            check_release_source_dependencies(&bad, Default::default()).is_err(),
            "mutation {index}"
        );
        assert_eq!(serde_json::to_vec(&bad).unwrap(), before);
    }
}

#[test]
fn source_schema_membership_requires_active_registry_addresses() {
    let mut input = shell(&fixture().input);
    let subject = input
        .recipe
        .schema
        .definitions
        .iter()
        .map(|row| row.address())
        .find(|address| matches!(address, DefinitionAddress::Stat(_)))
        .map(SchemaSubject::Definition)
        .unwrap();
    let mut registry =
        OwnedIdRegistry::new(input.recipe.registry.clone(), Default::default()).unwrap();
    registry
        .retire(
            &subject,
            OwnedDefinitionKey::new("retired-test-stat").unwrap(),
        )
        .unwrap();
    input.recipe.registry = registry.input().clone();
    assert!(matches!(
        check_release_source_dependencies(&input, Default::default()),
        Err(OwnedReleaseError::Recipe(OwnedRecipeError::Unregistered(found)))
            if *found == subject
    ));
}

#[test]
fn source_checks_honor_original_release_recipe_and_constituent_budgets() {
    let input = shell(&fixture().input);
    let mut exact = OwnedReleaseLimits {
        max_input_bytes: serde_json::to_vec(&input).unwrap().len(),
        ..Default::default()
    };
    exact.recipe.max_wire_bytes = serde_json::to_vec(&input.recipe).unwrap().len();
    exact.recipe.max_registry_checks =
        input.recipe.schema.definitions.len() + input.recipe.schema.slots.len();
    check_release_source_dependencies(&input, exact).unwrap();
    for case in 0..5 {
        let mut limited = exact;
        match case {
            0 => limited.max_input_bytes -= 1,
            1 => limited.recipe.max_wire_bytes -= 1,
            2 => limited.recipe.max_registry_checks -= 1,
            3 => limited.max_validation_entries = 1,
            4 => limited.normalization.max_policy_bytes = 1,
            _ => unreachable!(),
        }
        assert!(
            check_release_source_dependencies(&input, limited).is_err(),
            "budget {case}"
        );
    }
}

#[test]
fn stale_import_binding_is_rejected_before_lowering_then_late_dsl_failure_stays_a_failure() {
    let dsl = DslRelease::new(&fixture().input, false);
    let path = dsl.source_for("action-final");
    let mut source = dsl.source;
    source.replace_plain(&path, "not a program".into());
    let roles = serde_json::to_string(&dsl.expected.roles).unwrap();
    let mut bad_roles = dsl.expected.roles.clone();
    bad_roles.mapping = other_digest();
    source.replace_plain("roles.json", serde_json::to_string(&bad_roles).unwrap());
    let before = source.files.clone();
    assert!(matches!(
        source.build(Default::default()),
        Err(ReleaseBuildError::Assembly(OwnedReleaseError::Roles(_)))
    ));
    assert_eq!(source.files, before);
    source.replace_plain("roles.json", roles);
    let before = source.files.clone();
    assert!(matches!(
        source.build(Default::default()),
        Err(ReleaseBuildError::Packet { .. })
    ));
    assert_eq!(source.files, before);
}

#[test]
fn source_dependency_success_cannot_admit_sealed_evaluation_sources() {
    let dsl = DslRelease::new(&fixture().input, false);
    let mut input = shell(&dsl.expected);
    let metrics = &mut input.evaluation.as_mut().unwrap().metrics;
    metrics.definitions.content_sha256 = "1".repeat(64);
    let metrics = serde_json::to_string(metrics).unwrap();
    // The original schema/import dependencies are valid, while the final
    // evaluation mapping deliberately points elsewhere.
    check_release_source_dependencies(&input, Default::default()).unwrap();
    let mut source = dsl.source;
    source.replace_plain("metrics.json", metrics);
    assert!(matches!(
        source.build(Default::default()),
        Err(ReleaseBuildError::Json { path, .. }) if path == "metrics.json"
    ));
}

#[test]
fn maintained_nested_gem_input_seal_is_checked_against_the_original_schema() {
    let mut input = shell(tracked_release().release().input());
    let policy = input.normalization.gem_inputs.as_mut().unwrap();
    policy.definitions.content_sha256 = "1".repeat(64);
    assert!(matches!(
        check_release_source_dependencies(&input, Default::default()),
        Err(OwnedReleaseError::Normalization(_))
    ));
}
