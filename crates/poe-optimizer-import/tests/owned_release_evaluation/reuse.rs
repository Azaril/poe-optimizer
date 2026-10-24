//! Reusing checked recipe work never grants complete release authority.
use super::{files, fixture, key, stage};
use poe_optimizer_core::owned_content::{ContentDigestError, digest_owned};
use poe_optimizer_import::{
    owned_recipe::{CheckedOwnedRecipe, OwnedRecipeError, OwnedRecipeLimits},
    owned_release::*,
};
use std::sync::Arc;

fn recipe(input: &OwnedReleaseInput, limits: OwnedRecipeLimits) -> CheckedOwnedRecipe {
    CheckedOwnedRecipe::new(input.recipe.clone(), limits).unwrap()
}

fn error(result: Result<StagedOwnedRelease, OwnedReleaseError>) -> OwnedReleaseError {
    result.err().expect("release must reject this control")
}

#[test]
fn exact_checked_recipe_reuses_compiled_handles_and_preserves_every_artifact() {
    let input = fixture().input.clone();
    let baseline = stage(input.clone());
    let checked = recipe(&input, Default::default());
    let compiled = Arc::clone(checked.compiled_rules());
    let schema = Arc::clone(checked.schema());
    let rules = Arc::clone(checked.rules());
    let routing = Arc::clone(checked.routing());
    let reused =
        assemble_owned_release_with_checked_recipe(input, checked, Default::default()).unwrap();
    assert!(Arc::ptr_eq(
        reused.assembled().checked().compiled_rules(),
        &compiled
    ));
    assert!(Arc::ptr_eq(reused.assembled().checked().schema(), &schema));
    assert!(Arc::ptr_eq(reused.assembled().checked().rules(), &rules));
    assert!(Arc::ptr_eq(
        reused.assembled().checked().routing(),
        &routing
    ));
    assert_eq!(reused.input(), baseline.input());
    assert_eq!(reused.receipt(), baseline.receipt());
    assert_eq!(files(&reused), files(&baseline));

    // Ordinary runtime loading checks its own supplied bytes and compiles its own
    // recipe; the publication's in-memory authority is not a loader shortcut.
    let loaded = load_owned_release_artifacts(&files(&reused), Default::default()).unwrap();
    assert!(!Arc::ptr_eq(loaded.recipe().compiled_rules(), &compiled));
    assert_eq!(loaded.receipt(), reused.receipt());
}

#[test]
fn constructor_sorting_preserves_exact_original_input_authority() {
    let baseline = stage(fixture().input.clone());
    let mut input = fixture().input.clone();
    input.recipe.registry.entries.reverse();
    input.recipe.schema.definitions.reverse();
    assert_ne!(input.recipe, fixture().input.recipe);
    let checked = recipe(&input, Default::default());
    let compiled = Arc::clone(checked.compiled_rules());
    let reused =
        assemble_owned_release_with_checked_recipe(input, checked, Default::default()).unwrap();
    assert!(Arc::ptr_eq(
        reused.assembled().checked().compiled_rules(),
        &compiled
    ));
    assert_eq!(reused.input(), baseline.input());
    assert_eq!(files(&reused), files(&baseline));
}

#[test]
fn changed_recipe_constituents_cannot_borrow_another_inputs_validation() {
    let original = fixture().input.clone();
    let checked = recipe(&original, Default::default());
    assemble_owned_release_with_checked_recipe(
        original.clone(),
        checked.clone(),
        Default::default(),
    )
    .unwrap();
    for case in 0..4 {
        let mut input = original.clone();
        match case {
            0 => input.recipe.registry.entries.reverse(),
            1 => input.recipe.schema.definitions.reverse(),
            2 => input.recipe.rules.release = key("different-rules"),
            _ => input.recipe.routing.release = key("different-routing"),
        }
        assert_ne!(input.recipe, original.recipe);
        assert!(matches!(
            assemble_owned_release_with_checked_recipe(input, checked.clone(), Default::default()),
            Err(OwnedReleaseError::Recipe(OwnedRecipeError::InputMismatch))
        ));
    }
}

#[test]
fn different_stricter_and_looser_recipe_limits_recheck_instead_of_reusing() {
    let input = fixture().input.clone();
    let baseline = stage(input.clone());
    let ordinary = OwnedRecipeLimits::default();
    let mut stricter = ordinary;
    stricter.compile.max_work -= 1;
    for (initial, requested) in [(ordinary, stricter), (stricter, ordinary)] {
        let checked = recipe(&input, initial);
        let compiled = Arc::clone(checked.compiled_rules());
        let limits = OwnedReleaseLimits {
            recipe: requested,
            ..Default::default()
        };
        let rebuilt =
            assemble_owned_release_with_checked_recipe(input.clone(), checked, limits).unwrap();
        assert!(!Arc::ptr_eq(
            rebuilt.assembled().checked().compiled_rules(),
            &compiled
        ));
        assert_eq!(files(&rebuilt), files(&baseline));
    }
    // Non-default limits may reuse too, once those exact limits have checked the
    // recipe. This is not a hard-coded default-budget exception.
    let checked = recipe(&input, stricter);
    let compiled = Arc::clone(checked.compiled_rules());
    let reused = assemble_owned_release_with_checked_recipe(
        input,
        checked,
        OwnedReleaseLimits {
            recipe: stricter,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(Arc::ptr_eq(
        reused.assembled().checked().compiled_rules(),
        &compiled
    ));
}

#[test]
fn current_recipe_budgets_reject_with_the_same_errors_as_ordinary_assembly() {
    let input = fixture().input.clone();
    let checked = recipe(&input, Default::default());
    assemble_owned_release_with_checked_recipe(input.clone(), checked.clone(), Default::default())
        .unwrap();
    for case in 0..8 {
        let mut limits = OwnedReleaseLimits::default();
        match case {
            0 => limits.recipe.registry.max_entries = 1,
            1 => limits.recipe.schema.max_entries = 1,
            2 => limits.recipe.rules.max_nodes = 1,
            3 => limits.recipe.compile.max_nodes = 1,
            4 => limits.recipe.routing.max_wire_bytes = 1,
            5 => limits.recipe.max_registry_checks = 1,
            6 => limits.recipe.max_wire_bytes = 1,
            _ => limits.recipe.max_output_bytes = 1,
        }
        let ordinary = error(assemble_owned_release(input.clone(), limits));
        let reused = error(assemble_owned_release_with_checked_recipe(
            input.clone(),
            checked.clone(),
            limits,
        ));
        assert_eq!(reused.to_string(), ordinary.to_string(), "budget {case}");
    }
}

#[test]
fn release_preflight_and_publication_limits_still_apply_before_recipe_reuse() {
    let input = fixture().input.clone();
    let checked = recipe(&input, Default::default());
    let baseline = stage(input.clone());
    let total: usize = baseline.artifacts().map(|(_, bytes)| bytes.len()).sum();
    let mut foreign = input.clone();
    foreign.recipe.rules.release = key("foreign-recipe");
    let bounded = OwnedReleaseLimits {
        max_validation_entries: 1,
        max_semantic_bytes: 1,
        ..Default::default()
    };
    assert!(matches!(
        assemble_owned_release_with_checked_recipe(foreign, checked.clone(), bounded),
        Err(OwnedReleaseError::Limit("validation entries"))
    ));
    assert!(matches!(
        assemble_owned_release_with_checked_recipe(
            input.clone(),
            checked.clone(),
            OwnedReleaseLimits {
                max_semantic_bytes: 1,
                ..Default::default()
            },
        ),
        Err(OwnedReleaseError::Digest(ContentDigestError::TooLarge {
            maximum: 1
        }))
    ));
    for limits in [
        OwnedReleaseLimits {
            max_output_bytes: total - 1,
            ..Default::default()
        },
        OwnedReleaseLimits {
            max_artifact_bytes: 1,
            ..Default::default()
        },
    ] {
        let ordinary = error(assemble_owned_release(input.clone(), limits));
        let reused = error(assemble_owned_release_with_checked_recipe(
            input.clone(),
            checked.clone(),
            limits,
        ));
        assert_eq!(reused.to_string(), ordinary.to_string());
    }
}

#[test]
fn recipe_reuse_does_not_bypass_import_or_evaluation_dependency_validation() {
    let original = fixture().input.clone();
    let checked = recipe(&original, Default::default());
    assemble_owned_release_with_checked_recipe(
        original.clone(),
        checked.clone(),
        Default::default(),
    )
    .unwrap();
    let foreign = digest_owned("foreign-release-dependency", &1, 1024).unwrap();
    for case in 0..2 {
        let mut input = original.clone();
        if case == 0 {
            input.mapping.registry = foreign;
        } else {
            input
                .evaluation
                .as_mut()
                .unwrap()
                .support
                .as_mut()
                .unwrap()
                .stages
                .rules = foreign;
        }
        let ordinary = error(assemble_owned_release(input.clone(), Default::default()));
        let reused = error(assemble_owned_release_with_checked_recipe(
            input,
            checked.clone(),
            Default::default(),
        ));
        assert_eq!(
            reused.to_string(),
            ordinary.to_string(),
            "dependency {case}"
        );
    }
}
