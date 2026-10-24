//! Independent local arithmetic examples evaluated from the tracked DSL release.
use super::{invalid, tracked_allocation, tracked_release, tracked_sources};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::FiniteQuantity,
    owned_rules::RuleProgram,
    owned_schema::{ComputedValueType, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_rules::{EffectDisposition, RuleFact, RuleScratch};
use poe_optimizer_import::owned_recipe::CheckedOwnedRecipe;

fn program<'a>(recipe: &'a CheckedOwnedRecipe, name: &str) -> (SchemaSubject, &'a RuleProgram) {
    let rules = recipe.rules().input();
    let receiver = rules
        .receivers
        .members
        .iter()
        .find(|receiver| receiver.id.as_str() == name)
        .unwrap();
    let owner = SchemaSubject::Definition(receiver.stat.address());
    let program = rules
        .owners
        .iter()
        .find(|entry| entry.owner == owner)
        .unwrap()
        .programs
        .members
        .iter()
        .find(|program| program.id.as_str() == name)
        .unwrap();
    (owner, program)
}

fn assert_value(
    recipe: &CheckedOwnedRecipe,
    name: &str,
    values: &[(&str, f64)],
    expected: f64,
    scratch: &mut RuleScratch,
) {
    let (owner, program) = program(recipe, name);
    let facts: Vec<_> = values
        .iter()
        .map(|&(name, value)| {
            let read = program
                .reads
                .iter()
                .find(|read| read.id.as_str() == name)
                .unwrap();
            let ComputedValueType::Quantity { unit } = &read.value_type else {
                panic!("{name} must remain a quantity read")
            };
            RuleFact {
                read: read.id.clone(),
                value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
            }
        })
        .collect();
    let compiled = recipe.compiled_rules();
    let result = compiled
        .evaluate(
            &owner,
            &program.id,
            &facts,
            recipe.schema().as_ref(),
            scratch,
        )
        .unwrap();
    assert_eq!(result.effects.len(), 1, "{name}: {result:?}");
    let effect = result
        .effects
        .iter()
        .find(|effect| effect.id.as_str() == "derive")
        .unwrap();
    assert!(
        matches!(&effect.disposition, EffectDisposition::Applied { value: ParameterValue::Quantity(value) } if value.value() == expected),
        "{name}: expected {expected}, got {result:?}"
    );
}

#[test]
fn tracked_local_arithmetic_preserves_independent_values_and_missing_fact_refusal() {
    let recipe = tracked_release().release().assembled().checked();
    let compiled = recipe.compiled_rules();
    let mut scratch = compiled.new_scratch();
    // Independently reviewed Item.lua examples. Supplying these facts does not
    // establish that importing an item reconstructs its original rune lifecycle.
    for (name, values, expected) in [
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.4),
                ("quality", 20.0),
                ("quality-effect", 0.0),
                ("increase", 49.0),
            ],
            2.09,
        ),
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.4),
                ("quality", 20.0),
                ("quality-effect", 2.0),
                ("increase", 49.0),
            ],
            2.16,
        ),
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.005),
                ("quality", 0.0),
                ("quality-effect", 0.0),
                ("increase", 0.0),
            ],
            1.0,
        ),
        (
            "pre-override-critical-chance",
            vec![
                ("raw", 12.0),
                ("flat", 2.43),
                ("quality", 20.0),
                ("quality-effect", 0.0),
                ("increase", 0.0),
            ],
            14.43,
        ),
        (
            "pre-override-critical-chance",
            vec![
                ("raw", 12.0),
                ("flat", 2.43),
                ("quality", 20.0),
                ("quality-effect", 2.0),
                ("increase", 0.0),
            ],
            15.87,
        ),
        (
            "rounded-physical-minimum",
            vec![
                ("raw", 55.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 0.0),
                ("increase", 203.0),
            ],
            200.0,
        ),
        (
            "rounded-physical-maximum",
            vec![
                ("raw", 91.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 0.0),
                ("increase", 203.0),
            ],
            331.0,
        ),
        (
            "rounded-physical-minimum",
            vec![
                ("raw", 55.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 1.0),
                ("increase", 203.0),
            ],
            167.0,
        ),
        (
            "rounded-physical-maximum",
            vec![
                ("raw", 91.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 1.0),
                ("increase", 203.0),
            ],
            276.0,
        ),
    ] {
        assert_value(recipe, name, &values, expected, &mut scratch);
        // A previous successful evaluation must not supply missing facts to the
        // next call, including when the next row changes inputs on this program.
        let (owner, program) = program(recipe, name);
        let result = compiled
            .evaluate(
                &owner,
                &program.id,
                &[],
                recipe.schema().as_ref(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1, "{name}: {result:?}");
        let effect = result
            .effects
            .iter()
            .find(|effect| effect.id.as_str() == "derive")
            .unwrap();
        assert!(
            matches!(effect.disposition, EffectDisposition::Unresolved { .. }),
            "{name}: missing facts must remain unresolved: {result:?}"
        );
    }
}

#[test]
fn changing_an_authored_divisor_changes_the_value_and_invalidates_the_old_commitment() {
    let baseline = tracked_release().release();
    let values = [
        ("raw", 1.4),
        ("quality", 20.0),
        ("quality-effect", 2.0),
        ("increase", 49.0),
    ];
    assert_value(
        baseline.assembled().checked(),
        "pre-override-attack-rate",
        &values,
        2.16,
        &mut baseline
            .assembled()
            .checked()
            .compiled_rules()
            .new_scratch(),
    );

    let mut changed = tracked_sources().clone();
    let mut matches = changed.files.iter().filter(|(path, bytes)| {
        path.ends_with(".rule")
            && std::str::from_utf8(bytes)
                .unwrap()
                .starts_with("program \"pre-override-attack-rate\" ")
    });
    let path = matches
        .next()
        .expect("declared attack-rate DSL source")
        .0
        .clone();
    assert!(
        matches.next().is_none(),
        "attack-rate DSL source must be unique"
    );
    let original = changed.text(&path);
    let divisor = "quantity(8, unit.percentage_points)";
    assert_eq!(
        original.matches(divisor).count(),
        1,
        "unique quality divisor"
    );
    let edited = original.replace(divisor, "quantity(10, unit.percentage_points)");
    changed.replace_plain(&path, edited);
    invalid(
        changed.build_with_allocation(tracked_allocation(), Default::default()),
        "assembled input identity differs from expected_input",
    );

    // A new authored release may select a new identity; it must not silently
    // retain the original release's commitment after changing a formula.
    let commitment = format!(" expected_input=\"{}\"", baseline.receipt().input);
    assert_eq!(changed.manifest.matches(&commitment).count(), 1);
    changed.manifest = changed.manifest.replacen(&commitment, "", 1);
    let altered = changed
        .build_with_allocation(tracked_allocation(), Default::default())
        .unwrap();
    assert_ne!(altered.release().receipt().input, baseline.receipt().input);
    assert_ne!(altered.release().receipt().rules, baseline.receipt().rules);
    assert_value(
        altered.release().assembled().checked(),
        "pre-override-attack-rate",
        &values,
        2.14,
        &mut altered
            .release()
            .assembled()
            .checked()
            .compiled_rules()
            .new_scratch(),
    );
}
