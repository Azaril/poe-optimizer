//! Config-owned Enemy resistance BASE components, not final resistance or DPS.
#[path = "support/configuration_resistance_fixture.rs"]
mod fixture;

use fixture::Fixture;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

#[test]
fn authored_defaults_keep_absence_zero_negative_and_high_overrides_distinct() {
    let mut fixture = Fixture::new(true);
    assert_eq!(fixture.scenario.assumptions.len(), 4);
    let report = fixture.evaluate();
    for (index, expected) in [50., 50., 50., 0.].into_iter().enumerate() {
        assert_eq!(
            fixture.inputs.inputs[index].default_value,
            fixture.quantity(expected)
        );
        fixture.assert_known(&report, index, expected);
    }
    for (index, value) in [0., -25., 90., 1000.].into_iter().enumerate() {
        fixture.set(index, Some(true), Some(value));
    }
    let report = fixture.evaluate();
    for (index, value) in [0., -25., 90., 1000.].into_iter().enumerate() {
        fixture.assert_known(&report, index, value);
    }
    // Presence is explicit; no raw quantity row is fabricated for absence.
    fixture.set(0, Some(false), None);
    fixture.assert_known(&fixture.evaluate(), 0, 50.);
    assert!(
        !fixture
            .scenario
            .assumptions
            .iter()
            .any(|value| value.input == fixture.inputs.inputs[0].value_input)
    );
}

#[test]
fn missing_inputs_invalid_scope_and_units_do_not_become_defaults() {
    let mut fixture = Fixture::new(true);
    fixture.set(0, None, None);
    fixture.set(1, Some(true), None);
    let report = fixture.evaluate();
    for index in [0, 1] {
        assert!(matches!(
            fixture.contribution(&report, index),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            }
        ));
    }
    fixture.assert_known(&report, 2, 50.);
    fixture.assert_known(&report, 3, 0.);

    let mut wrong_target = Fixture::new(true);
    wrong_target.scenario.assumptions[0].target = AssumptionTarget::Environment;
    assert!(wrong_target.plan().is_err());
    let mut wrong_unit = Fixture::new(true);
    wrong_unit.set(0, Some(true), Some(1.));
    let foreign_unit: UnitDefId =
        DefId::parse(wrong_unit.inputs.namespace.clone(), "different-unit").unwrap();
    wrong_unit.scenario.assumptions.last_mut().unwrap().value =
        ParameterValue::Quantity(FiniteQuantity::new(1., foreign_unit).unwrap());
    assert!(wrong_unit.plan().is_err());
    let mut duplicate = Fixture::new(true);
    duplicate
        .scenario
        .assumptions
        .push(duplicate.scenario.assumptions[0].clone());
    assert!(duplicate.plan().is_err());
    let mut outside_range = Fixture::new(true);
    outside_range.set(0, Some(true), Some(1_000_001.));
    assert!(outside_range.plan().is_err());
}

#[test]
fn scratch_and_parallel_requests_are_isolated_and_partial_owner_stays_partial() {
    let first = Fixture::new(true);
    let plan_a = first.plan().unwrap();
    let mut second = Fixture::new(true);
    second.set(0, Some(true), Some(0.));
    second.set(1, Some(true), Some(-25.));
    let plan_b = second.plan().unwrap();
    assert_ne!(plan_a.identity(), plan_b.identity());
    let mut scratch = plan_a.new_scratch();
    let expected_a = plan_a.evaluate(&mut scratch).unwrap();
    let expected_b = plan_b.evaluate(&mut scratch).unwrap();
    first.assert_known(&expected_a, 0, 50.);
    second.assert_known(&expected_b, 0, 0.);
    second.assert_known(&expected_b, 1, -25.);
    assert_eq!(plan_a.evaluate(&mut scratch).unwrap(), expected_a);
    let plan_a = Arc::new(plan_a);
    let plan_b = Arc::new(plan_b);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|index| {
                let a = plan_a.clone();
                let b = plan_b.clone();
                let expected_a = &expected_a;
                let expected_b = &expected_b;
                scope.spawn(move || {
                    let mut scratch = a.new_scratch();
                    for turn in 0..8 {
                        let (plan, expected) = if (index + turn) % 2 == 0 {
                            (&a, expected_a)
                        } else {
                            (&b, expected_b)
                        };
                        assert_eq!(&plan.evaluate(&mut scratch).unwrap(), expected);
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().unwrap();
        }
    });
    let partial = Fixture::new(false);
    let report = partial.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject
                    == Some(SchemaSubject::Definition(
                        partial.inputs.encounter.address()
                    )))
    );
    for index in 0..4 {
        assert_eq!(
            partial.contribution(&report, index),
            &EffectValue::Known {
                value: partial.inputs.inputs[index].default_value.clone()
            }
        );
        assert!(matches!(
            partial.sum(&report, index),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
    }
}

#[test]
#[ignore = "requires independently verified complete-source JSON via POE_OPTIMIZER_TEST_CONFIGURATION_SOURCE"]
fn complete_source_config_owned_base_records_match_authored_native_programs() {
    let path = std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_SOURCE").expect(
        "set to source-jit-off.json or source-jit-on.json from the complete source witness",
    );
    let observed: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let authoring: Value = serde_json::from_slice(
        &fs::read(
            workspace
                .join("data/owned/poe2/3887ae68/configuration-resistance-inputs/authoring.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        observed["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(observed["source_revision"], authoring["source_revision"]);
    assert_eq!(
        observed["source_hash"].as_str().unwrap(),
        authoring["source_manifest_sha256"].as_str().unwrap()
    );
    let originals = workspace.join("tests/fixtures/builds/breadth-20260908");
    let manifest: Value =
        serde_json::from_slice(&fs::read(originals.join("index.json")).unwrap()).unwrap();
    assert_eq!(observed["evidence"]["business_method_wrappers"], false);
    assert_eq!(observed["evidence"]["native_effect_coverage"], false);
    let mut names: Vec<_> = (1..=5).map(|n| format!("original-{n:02}")).collect();
    names.extend(
        [
            "numeric-placeholders",
            "numeric-input-priority",
            "numeric-zero",
            "numeric-malformed-last",
        ]
        .into_iter()
        .map(str::to_owned),
    );
    for name in names {
        let cases: Vec<_> = observed["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["name"] == name)
            .collect();
        assert_eq!(cases.len(), 1, "{name}");
        assert_eq!(cases[0]["available"], true, "{name}");
        if let Some(number) = name.strip_prefix("original-") {
            let filename = format!("build-{number}.xml");
            let original_rows: Vec<_> = manifest["builds"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["xml"] == filename)
                .collect();
            assert_eq!(original_rows.len(), 1, "{name} fixture manifest");
            let expected_hash = original_rows[0]["xml_sha256"].as_str().unwrap();
            assert_eq!(
                format!(
                    "{:x}",
                    Sha256::digest(fs::read(originals.join(filename)).unwrap())
                ),
                expected_hash,
                "{name} original XML bytes"
            );
            assert_eq!(
                cases[0]["xml_sha256"].as_str().unwrap(),
                expected_hash,
                "{name} source observation"
            );
        }
        let state = &cases[0]["state"];
        for preserved in [
            "methods_preserved",
            "objects_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(state[preserved], true, "{name} {preserved}");
        }
        let saved = &state["saved"];
        let mut fixture = Fixture::new(true);
        for index in 0..4 {
            let key = fixture.inputs.inputs[index].source_name.clone();
            let value = &saved["input"][&key];
            let raw = if value.is_null() {
                None
            } else {
                Some(
                    value
                        .as_f64()
                        .expect("observed raw override must be numeric"),
                )
            };
            fixture.set(index, Some(raw.is_some()), raw);
        }
        let report = fixture.evaluate();
        for index in 0..4 {
            let source_name = &fixture.inputs.inputs[index].source_name;
            let record_name = source_name.strip_prefix("enemy").unwrap();
            let records: Vec<_> = saved["enemy_mods"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["name"] == record_name
                        && row["type"] == "BASE"
                        && row["source"] == "EnemyConfig"
                })
                .collect();
            assert_eq!(records.len(), 1, "{name} {record_name}");
            assert_eq!(records[0]["flags"], 0);
            assert_eq!(records[0]["keyword_flags"], 0);
            let tags = &records[0]["tags"];
            assert!(
                tags.as_array().is_some_and(Vec::is_empty)
                    || tags.as_object().is_some_and(serde_json::Map::is_empty),
                "{name} {record_name} must have a known empty tag collection"
            );
            let expected = records[0]["value"].as_f64().unwrap();
            fixture.assert_known(&report, index, expected);
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(
                    saved["modes"][mode]["enemy_base"][record_name]["enemy_config"].as_f64(),
                    Some(expected),
                    "{name} {mode} {record_name}"
                );
            }
        }
    }
}
