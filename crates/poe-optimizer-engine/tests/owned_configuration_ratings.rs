//! Injected Config-owned rating producers, before hit chance or mitigation.
#[path = "support/configuration_ratings_fixture.rs"]
mod fixture;
use fixture::{asset, fixture};
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

#[test]
fn rating_defaults_follow_enemy_level_and_preserve_raw_overrides() {
    let mut f = fixture(true);
    assert_eq!(f.scenario.assumptions.len(), 2);
    for (level, armour, evasion) in [
        (1, 8063., 1175.),
        (20, 8063., 1175.),
        (81, 8063., 1175.),
        (82, 8063., 1175.),
        (83, 8526., 1198.),
        (84, 9017., 1220.),
        (85, 9533., 1244.),
    ] {
        f.scenario.enemy.level = level;
        let report = f.evaluate();
        f.assert_known(&report, 0, armour);
        f.assert_known(&report, 1, evasion);
    }
    for value in [0., -25., 999999., 12.25] {
        for index in 0..2 {
            f.set(index, Some(true), Some(value));
        }
        let report = f.evaluate();
        for index in 0..2 {
            f.assert_known(&report, index, value);
        }
    }
    f.set(0, Some(false), None);
    f.assert_known(&f.evaluate(), 0, 9533.);
}

#[test]
fn incomplete_inputs_and_table_domains_do_not_fabricate_ratings() {
    let mut f = fixture(true);
    for (present, raw) in [(None, None), (None, Some(50.)), (Some(true), None)] {
        f.set(0, present, raw);
        assert!(matches!(
            f.contribution(&f.evaluate(), 0),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            }
        ));
    }
    f.set(0, Some(false), None);
    // A deliberately narrower injected table cannot clamp a missing key.
    f.tables[0].rows.drain(..82);
    f.tables[0].minimum = BoundedInteger::new(83).unwrap();
    assert_eq!(
        f.contribution(&f.evaluate(), 0),
        &EffectValue::UnsupportedDomain {
            node: OwnedDefinitionKey::new("monster-rating").unwrap(),
            table: OwnedDefinitionKey::new("monster-armour-by-level").unwrap(),
            key: BoundedInteger::new(82).unwrap(),
            minimum: BoundedInteger::new(83).unwrap(),
            maximum: BoundedInteger::new(85).unwrap(),
        }
    );
    // An unused default branch does not demand the missing table key.
    f.set(0, Some(true), Some(0.));
    f.assert_known(&f.evaluate(), 0, 0.);
    f.tables.clear();
    assert!(
        f.plan().is_err(),
        "missing declared table must fail compilation"
    );

    let f = fixture(false);
    let report = f.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject == Some(SchemaSubject::Definition(f.inputs.encounter.address())))
    );
    for (index, value) in [8063., 1175.].into_iter().enumerate() {
        assert_eq!(
            f.contribution(&report, index),
            &EffectValue::Known {
                value: f.quantity(value)
            }
        );
        assert!(matches!(
            f.sum(&report, index),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
    }
}

#[test]
fn rating_plans_rebind_levels_without_scratch_or_parallel_leaks() {
    let mut f = fixture(true);
    f.scenario.enemy.level = 20;
    let a = Arc::new(f.plan().unwrap());
    f.scenario.enemy.level = 85;
    let b = Arc::new(f.plan().unwrap());
    let expected_a = a.evaluate(&mut a.new_scratch()).unwrap();
    let expected_b = b.evaluate(&mut b.new_scratch()).unwrap();
    assert_ne!(expected_a, expected_b);
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    assert_eq!(b.evaluate(&mut scratch).unwrap(), expected_b);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|offset| {
                let a = a.clone();
                let b = b.clone();
                let expected_a = &expected_a;
                let expected_b = &expected_b;
                scope.spawn(move || {
                    let mut scratch = a.new_scratch();
                    for turn in 0..8 {
                        let (plan, expected) = if (offset + turn) % 2 == 0 {
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
}

#[test]
#[ignore = "requires verified full reference via POE_OPTIMIZER_TEST_ENEMY_RATINGS_SOURCE"]
fn complete_source_rating_records_match_native_tables_and_programs() {
    let path = std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_RATINGS_SOURCE")
        .expect("explicit source evidence");
    let observed: Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let authoring = asset("configuration-rating-inputs", "authoring.json");
    let extension = asset("configuration-rating-inputs", "extension.json");
    assert_eq!(authoring["source_validation"]["status"], "passed");
    assert_eq!(observed["source_revision"], authoring["source_revision"]);
    assert_eq!(observed["source_hash"], authoring["source_manifest_sha256"]);
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let originals = workspace.join("tests/fixtures/builds/breadth-20260908");
    let manifest: Value =
        serde_json::from_slice(&fs::read(originals.join("index.json")).unwrap()).unwrap();
    let mut checked = 0;
    let mut originals_checked = 0;
    for case in observed["cases"].as_array().unwrap() {
        assert_eq!(case["available"], true);
        let state = &case["state"];
        for preserved in [
            "methods_preserved",
            "objects_preserved",
            "outputs_preserved",
        ] {
            assert_eq!(state[preserved], true);
        }
        let saved = &state["saved"];
        if saved["input"]["enemyIsBoss"] != "Pinnacle" {
            continue;
        }
        let name = case["name"].as_str().unwrap();
        if let Some(number) = name.strip_prefix("original-") {
            let filename = format!("build-{number}.xml");
            let rows: Vec<_> = manifest["builds"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| r["xml"] == filename)
                .collect();
            assert_eq!(rows.len(), 1);
            let hash = format!(
                "{:x}",
                Sha256::digest(fs::read(originals.join(filename)).unwrap())
            );
            assert_eq!(case["xml_sha256"], hash);
            assert_eq!(rows[0]["xml_sha256"], hash);
            originals_checked += 1;
        }
        let mut f = fixture(true);
        f.scenario.enemy.level = u16::try_from(saved["enemy_level"].as_u64().unwrap()).unwrap();
        let observed_levels = state["source_data"]["levels"].as_array().unwrap();
        assert_eq!(observed_levels.len(), 85);
        for index in 0..2 {
            let source_name = &f.inputs.inputs[index].source_name;
            let raw = &saved["input"][source_name];
            let raw = if raw.is_null() {
                None
            } else {
                Some(raw.as_f64().unwrap())
            };
            f.set(index, Some(raw.is_some()), raw);
            for (cell, source) in f.tables[index].rows.iter().zip(observed_levels) {
                let field = if index == 0 { "armour" } else { "evasion" };
                assert_eq!(cell, &f.quantity(source[field].as_f64().unwrap()));
            }
            assert_eq!(f.tables[index].rows.len(), 85);
            for (offset, level) in observed_levels.iter().enumerate() {
                assert_eq!(level["level"].as_u64(), Some(offset as u64 + 1));
            }
            let mean_name = if index == 0 {
                "pinnacle_armour_mean"
            } else {
                "pinnacle_evasion_mean"
            };
            let means: Vec<_> = extension["owners"][0]["programs"]["members"][index]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|node| node["id"] == "boss-mean-percent")
                .collect();
            assert_eq!(means.len(), 1);
            assert_eq!(
                means[0]["expression"]["value"]["value"]["value"].as_f64(),
                state["source_data"][mean_name].as_f64()
            );
        }
        let report = f.evaluate();
        for index in 0..2 {
            let record_name = f.inputs.inputs[index]
                .source_name
                .strip_prefix("enemy")
                .unwrap();
            let records: Vec<_> = saved["enemy_mods"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    row["name"] == record_name && row["type"] == "BASE" && row["source"] == "Config"
                })
                .collect();
            assert_eq!(records.len(), 1, "{name} {record_name}");
            assert_eq!(records[0]["flags"], 0);
            assert_eq!(records[0]["keyword_flags"], 0);
            let tags = &records[0]["tags"];
            assert!(
                tags.as_array().is_some_and(Vec::is_empty)
                    || tags.as_object().is_some_and(serde_json::Map::is_empty)
            );
            let expected = records[0]["value"].as_f64().unwrap();
            f.assert_known(&report, index, expected);
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(
                    saved["modes"][mode]["enemy_level"].as_u64(),
                    Some(u64::from(f.scenario.enemy.level))
                );
                assert_eq!(
                    saved["modes"][mode]["enemy_base"][record_name]["config"].as_f64(),
                    Some(expected),
                    "{name} {mode} {record_name}"
                );
            }
            checked += 1;
        }
    }
    assert_eq!(originals_checked, 5);
    assert_eq!(checked, 32);
}
