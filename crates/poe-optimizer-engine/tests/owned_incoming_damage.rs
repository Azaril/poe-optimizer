//! Incoming hit preparation using published owned data; not full damage or EHP parity.
#[path = "support/incoming_damage_fixture.rs"]
mod fixture;
use fixture::{Fixture, asset, key, measured_cases, project_source_case, replay};
use poe_optimizer_core::{
    owned_definitions::BoundedInteger,
    owned_schema::{SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

#[test]
fn pinnacle_default_hit_inputs_follow_level_and_independent_raw_overrides() {
    let mut f = Fixture::new(true);
    for (level, base, chaos) in [
        (1, 965., 386.),
        (81, 965., 386.),
        (82, 965., 386.),
        (83, 993., 397.),
        (84, 1022., 409.),
        (85, 1051., 420.),
    ] {
        f.scenario.enemy.level = level;
        let report = f.evaluate();
        f.assert_damage(&report, [base, base, base, base, chaos], base * 4. + chaos);
        f.assert_penetration(&report, [3.; 3]);
    }
    f.scenario.enemy.level = 82;
    for (raw, total) in [(0., 3281.), (125., 3406.), (-25., 3256.), (12.25, 3293.25)] {
        f.set(0, Some(true), Some(raw));
        f.assert_damage(&f.evaluate(), [raw, 965., 965., 965., 386.], total);
    }
    // A stale raw value behind explicit absence cannot replace the default.
    f.set(0, Some(false), Some(777.));
    f.assert_damage(&f.evaluate(), [965., 965., 965., 965., 386.], 4246.);
    for index in 0..8 {
        f.set(index, Some(true), Some(index as f64 + 0.25));
    }
    let report = f.evaluate();
    f.assert_damage(&report, [0.25, 1.25, 2.25, 3.25, 4.25], 11.25);
    f.assert_penetration(&report, [5.25, 6.25, 7.25]);
}

#[test]
fn min_max_membership_and_ordered_damage_sum_are_real_native_dependencies() {
    let mut f = Fixture::new(true);
    f.contribute("physical-one", 0, 2., 10.);
    f.contribute("physical-two", 0, 4., -2.);
    f.contribute("chaos", 4, 0.5, 1.5);
    let report = f.evaluate();
    f.assert_damage(&report, [972., 965., 965., 965., 387.], 4254.);
    f.assert_penetration(&report, [3.; 3]);

    // Synthetic cancellation distinguishes source's ordered accumulation from
    // regrouping. Contributions are explicit finite-world sources, not facts
    // guessed from an original build's observed aggregate.
    let mut f = Fixture::new(true);
    for index in 0..5 {
        f.set(index, Some(true), Some(0.));
    }
    f.contribute("large-physical", 0, 2e16, 0.);
    f.contribute("small-lightning", 1, 2., 0.);
    f.contribute("negative-cold", 2, -2e16, 0.);
    f.assert_damage(&f.evaluate(), [1e16, 1., -1e16, 0., 0.], 0.);

    let f = Fixture::new(false);
    let report = f.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms
                && gap.subject == Some(SchemaSubject::Definition(f.inputs.encounter.address())))
    );
    for row in &f.inputs.damage {
        assert!(matches!(
            f.value(&report, &row.output_stat),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
    }
    assert!(matches!(
        f.value(&report, &f.inputs.total_stat),
        EffectValue::Unresolved { .. }
    ));
}

#[test]
fn missing_raw_presence_category_and_table_keys_remain_distinct() {
    let mut f = Fixture::new(true);
    for (present, raw) in [(None, None), (None, Some(12.)), (Some(true), None)] {
        f.set(0, present, raw);
        let report = f.evaluate();
        assert!(matches!(
            f.value(&report, &f.inputs.damage[0].output_stat),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            }
        ));
        f.assert_stat(
            &report,
            &f.inputs.damage[4].output_stat,
            &f.inputs.damage_unit,
            386.,
        );
    }
    f.set(0, Some(false), None);
    f.category(None);
    let report = f.evaluate();
    for row in &f.inputs.damage {
        assert!(matches!(
            f.value(&report, &row.output_stat),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingInput,
                ..
            }
        ));
    }
    f.category(Some("Average"));
    let table = f
        .tables
        .iter_mut()
        .find(|table| table.id == key("monster-damage-by-level"))
        .unwrap();
    table.rows.drain(..82);
    table.minimum = BoundedInteger::new(83).unwrap();
    let report = f.evaluate();
    assert!(
        matches!(f.value(&report, &f.inputs.damage[0].output_stat), EffectValue::UnsupportedDomain { key, minimum, .. }
        if key.get() == 82 && minimum.get() == 83)
    );
    // Overriding all five damage lanes makes the out-of-domain default unused.
    for index in 0..5 {
        f.set(index, Some(true), Some(index as f64));
    }
    f.assert_damage(&f.evaluate(), [0., 1., 2., 3., 4.], 10.);
    f.tables.clear();
    assert!(
        f.plan().is_err(),
        "referenced table must exist even when branches are inactive"
    );
}

#[test]
fn damage_over_time_bypasses_hit_inputs_and_contributor_reads_lazily() {
    let mut f = Fixture::new(true);
    f.category(Some("DamageOverTime"));
    for index in 0..8 {
        f.set(index, None, None);
    }
    let table = f
        .tables
        .iter_mut()
        .find(|table| table.id == key("monster-damage-by-level"))
        .unwrap();
    table.rows.drain(..82);
    table.minimum = BoundedInteger::new(83).unwrap();
    let report = f.evaluate();
    f.assert_damage(&report, [0.; 5], 0.);
    f.assert_penetration(&report, [0.; 3]);
    f.category(Some("Average"));
    assert!(matches!(
        f.value(&f.evaluate(), &f.inputs.damage[0].output_stat),
        EffectValue::Unresolved { .. }
    ));
    let mut partial = Fixture::new(false);
    partial.category(Some("DamageOverTime"));
    assert!(
        matches!(
            partial.value(&partial.evaluate(), &partial.inputs.total_stat),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ),
        "inactive-hit results cannot close the real producer inventory"
    );
}

#[test]
fn all_declared_hit_categories_and_penetration_overrides_keep_distinct_inputs() {
    let mut f = Fixture::new(true);
    for category in [
        "Average",
        "Untyped",
        "Melee",
        "Projectile",
        "Spell",
        "SpellProjectile",
    ] {
        f.category(Some(category));
        f.assert_damage(&f.evaluate(), [965., 965., 965., 965., 386.], 4246.);
    }
    for (index, value) in [(5, 0.), (6, -2.5), (7, 150.)] {
        f.set(index, Some(true), Some(value));
    }
    let report = f.evaluate();
    f.assert_damage(&report, [965., 965., 965., 965., 386.], 4246.);
    f.assert_penetration(&report, [0., -2.5, 150.]);
    f.set(6, Some(true), None);
    let report = f.evaluate();
    assert!(matches!(
        f.value(&report, &f.inputs.penetration[1].output_stat),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            ..
        }
    ));
    f.assert_stat(
        &report,
        &f.inputs.penetration[0].output_stat,
        &f.inputs.percent_unit,
        0.,
    );
}

#[test]
fn incoming_plans_reuse_scratch_and_parallel_workers_without_state_leaks() {
    let mut f = Fixture::new(true);
    let a = Arc::new(f.plan().unwrap());
    f.scenario.enemy.level = 85;
    f.set(0, Some(true), Some(125.));
    f.set(5, Some(true), Some(-7.));
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
                        let (plan, expected) = if (turn + offset) % 2 == 0 {
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
fn measured_incoming_stage_vectors_match_native_programs_in_default_ci() {
    let vectors = asset("incoming-damage-inputs", "reference-vectors.json");
    let authoring = asset("incoming-damage-inputs", "authoring.json");
    let vector_bytes = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../data/owned/poe2/3887ae68/incoming-damage-inputs/reference-vectors.json"),
    )
    .unwrap();
    assert_eq!(
        authoring["source_validation"]["reference_vectors_sha256"],
        format!("{:x}", Sha256::digest(&vector_bytes))
    );
    assert_eq!(vectors["schema_version"], 1);
    assert_eq!(authoring["source_validation"]["status"], "passed");
    assert_eq!(vectors["source_revision"], authoring["source_revision"]);
    assert_eq!(vectors["source_hash"], authoring["source_manifest_sha256"]);
    let fixture = Fixture::new(true);
    let table = fixture
        .tables
        .iter()
        .find(|table| table.id == key("monster-damage-by-level"))
        .unwrap();
    let source_table = vectors["tables"].as_array().unwrap();
    assert_eq!(source_table.len(), 85);
    assert_eq!(table.rows.len(), source_table.len());
    assert_eq!(table.minimum.get(), 1);
    for (native, source) in table.rows.iter().zip(source_table) {
        assert_eq!(
            native,
            &fixture.quantity(source.as_f64().unwrap(), &fixture.inputs.damage_unit)
        );
    }
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let index: Value =
        serde_json::from_slice(&fs::read(directory.join("index.json")).unwrap()).unwrap();
    let originals = vectors["originals"].as_array().unwrap();
    assert_eq!(originals.len(), 5);
    for original in originals {
        let name = original["name"].as_str().unwrap();
        let digest = format!(
            "{:x}",
            Sha256::digest(fs::read(directory.join(name)).unwrap())
        );
        assert_eq!(original["sha256"], digest);
        let entries: Vec<_> = index["builds"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|entry| entry["xml"] == name)
            .collect();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0]["xml_sha256"], digest);
    }
    let cases = measured_cases(&vectors);
    let mut identities = BTreeSet::new();
    let mut original_modes = BTreeSet::new();
    for case in &cases {
        assert!(
            identities.insert((&case.name, &case.mode)),
            "duplicate measured case"
        );
        if let Some(ordinal) = case.name.strip_prefix("original-") {
            let filename = format!("build-{ordinal}.xml");
            let original = originals
                .iter()
                .find(|pin| pin["name"] == filename)
                .unwrap();
            assert_eq!(original["sha256"], case.xml_sha256);
            original_modes.insert((&case.name, &case.mode));
        }
        replay(case);
    }
    assert_eq!(original_modes.len(), 10, "all five originals in both modes");
    assert!(
        cases.len() > original_modes.len(),
        "source counterfactuals required"
    );
}

#[test]
#[ignore = "requires verified full reference via POE_OPTIMIZER_TEST_INCOMING_DAMAGE_SOURCE"]
fn complete_source_incoming_stage_authenticates_the_committed_projection() {
    let path = std::env::var_os("POE_OPTIMIZER_TEST_INCOMING_DAMAGE_SOURCE")
        .expect("explicit complete source evidence");
    let bytes = fs::read(path).unwrap();
    let observed: Value = serde_json::from_slice(&bytes).unwrap();
    let vectors = asset("incoming-damage-inputs", "reference-vectors.json");
    let authoring = asset("incoming-damage-inputs", "authoring.json");
    assert_eq!(
        authoring["source_validation"]["evidence_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    assert_eq!(observed["source_revision"], vectors["source_revision"]);
    assert_eq!(observed["source_hash"], vectors["source_hash"]);
    assert_eq!(observed["evidence"]["files"], vectors["source_files"]);
    assert_eq!(observed["evidence"]["originals"], vectors["originals"]);
    let fixture = Fixture::new(true);
    let mut projected = Vec::new();
    for case in observed["cases"].as_array().unwrap() {
        if case["available"] == false {
            assert_eq!(case["expected_failure"], true);
            continue;
        }
        assert_eq!(case["available"], true);
        let observed_table = case["state"]["monster_damage_table"].as_array().unwrap();
        assert_eq!(observed_table.len(), 100);
        let projected_table = vectors["tables"].as_array().unwrap();
        assert_eq!(projected_table.len(), 85);
        for (observed, projected) in observed_table[..85].iter().zip(projected_table) {
            assert_eq!(observed.as_f64().unwrap(), projected.as_f64().unwrap());
        }
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
            "hook_restored",
        ] {
            assert_eq!(case["state"][field], true);
        }
        assert_eq!(case["state"]["business_method_wrappers"], false);
        for mode in ["main", "calcs"] {
            let projection = project_source_case(case, mode, &fixture.inputs);
            replay(&projection);
            projected.push(projection);
        }
    }
    let committed = measured_cases(&vectors);
    assert_eq!(projected.len(), committed.len());
    for (observed, expected) in projected.iter().zip(&committed) {
        assert_eq!(observed, expected, "{} {}", expected.name, expected.mode);
    }
}
