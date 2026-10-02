//! Native ordinary-minion guaranteed accuracy and actual enemy-block consumer.
#[path = "support/minion_accuracy_fixture.rs"]
mod fixture;
#[path = "../../../tests/support/minion_accuracy_source_vectors.rs"]
mod source_vectors;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*,
    owned_rules::*,
    owned_schema::{DefinitionDescriptor, SchemaSubject, SlotAddress, SlotDescriptor},
};
use poe_optimizer_engine::owned_plan::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::Arc};

fn close(actual: f64, expected: f64) {
    assert!((actual - expected).abs() < 1e-10, "{actual} != {expected}");
}
fn check(report: &OwnedEffectsReport, index: usize, block: f64, hit: f64) {
    assert_eq!(
        actor_value(report, index, 0x321a),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    close(known(action_value(report, index, 0x321c)), 100.);
    close(known(action_value(report, index, 0x321f)), block);
    close(known(action_value(report, index, 0x3220)), hit);
}
#[test]
fn real_programs_connect_config_actor_and_exact_action_with_dynamic_levels() {
    for (block, expected) in [
        (None, 0.),
        (Some(0.), 0.),
        (Some(37.), 37.),
        (Some(12.5), 12.5),
        (Some(-7.), 0.),
        (Some(140.), 100.),
    ] {
        let w = World::new([22, 1], block);
        let r = w.evaluate();
        assert!(r.gaps.is_empty(), "{:?}", r.gaps);
        for index in 0..2 {
            check(&r, index, expected, 100. - expected);
        }
        assert_ne!(actor_value(&r, 0, 0x1c), actor_value(&r, 1, 0x1c));
        assert_eq!(known(action_value(&r, 0, 0x3212)), 208.);
    }
}
#[test]
fn block_aggregates_all_sources_and_preserves_source_clamp_order_and_action_scope() {
    let mut w = World::new([22, 1], Some(37.));
    w.contribute(
        "fixture-additional-enemy-block",
        RuleEntity::Enemy,
        0x3218,
        quantity(12.5, 2),
    );
    w.action_reduction(15., 3.);
    let r = w.evaluate();
    check(&r, 0, 34.5, 65.5);
    check(&r, 1, 46.5, 53.5);
    for (block, reduction, effective) in [
        (140., 30., 70.),
        (37., 50., 0.),
        (99., -10., 109.),
        (-7., -10., 3.),
        (37.25, 0.125, 37.125),
    ] {
        let mut w = World::new([22, 1], Some(block));
        w.action_reduction(reduction, reduction);
        let r = w.evaluate();
        check(&r, 0, effective, 100. - effective);
    }
}
#[test]
fn flags_require_complete_membership_and_inheritance_never_reuses_default_accuracy() {
    let mut w = World::new([22, 1], Some(37.));
    w.contribute(
        "fixture-inheritance-present",
        RuleEntity::Player,
        0x3219,
        integer(1),
    );
    let r = w.evaluate();
    assert_eq!(
        actor_value(&r, 0, 0x321a),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert!(!matches!(
        action_value(&r, 0, 0x321c),
        EffectValue::Known { .. }
    ));
    assert!(matches!(
        action_value(&r, 0, 0x3220),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new([22, 1], Some(37.));
    w.contribute(
        "fixture-enemy-cannot-block",
        RuleEntity::Enemy,
        0x321e,
        integer(1),
    );
    check(&w.evaluate(), 0, 0., 100.);
    for (entity, stat) in [(RuleEntity::Player, 0x3219), (RuleEntity::Enemy, 0x321e)] {
        let mut w = World::new([22, 1], Some(37.));
        w.contribute("fixture-invalid-negative-total", entity, stat, integer(-1));
        let r = w.evaluate();
        assert!(!matches!(
            action_value(&r, 0, 0x3220),
            EffectValue::Known { .. }
        ));
    }
    let mut w = World::new([22, 1], Some(37.));
    w.restore_partial();
    let r = w.evaluate();
    assert!(!r.gaps.is_empty());
    assert!(matches!(
        action_value(&r, 0, 0x3220),
        EffectValue::Unresolved { .. }
    ));
}
#[test]
fn missing_assumptions_producers_and_parent_inputs_stay_explicit() {
    for (presence, raw) in [(None, None), (Some(true), None)] {
        let mut w = World::new([22, 1], Some(37.));
        w.set_block(presence, raw);
        assert!(matches!(
            action_value(&w.evaluate(), 0, 0x3220),
            EffectValue::Unresolved { .. }
        ));
    }
    let mut w = World::new([22, 1], None);
    w.set_block(Some(false), None);
    check(&w.evaluate(), 0, 0., 100.);
    let mut w = World::new([22, 1], Some(37.));
    let actor = SchemaSubject::Slot(SlotAddress::Actor(actor_slot()));
    w.intrinsic
        .f
        .owner_mut(&actor)
        .programs
        .members
        .retain(|p| p.id.as_str() != "intrinsic-minion-cannot-be-evaded");
    assert!(matches!(
        action_value(&w.evaluate(), 0, 0x3220),
        EffectValue::Unresolved { .. }
    ));
    let mut w = World::new([22, 1], Some(37.));
    w.intrinsic.missing_final_input();
    let r = w.evaluate();
    assert!(matches!(
        actor_value(&r, 0, 0x1c),
        EffectValue::Unresolved { .. }
    ));
    assert!(matches!(
        action_value(&r, 0, 0x3212),
        EffectValue::Unresolved { .. }
    ));
    // The guaranteed-hit branch is level independent. It cannot make missing
    // level-dependent damage known or claim actor/whole-build readiness.
}
#[test]
fn plans_preserve_scratch_and_parallel_isolation() {
    let mut w = World::new([22, 1], Some(37.));
    w.action_reduction(15., 3.);
    let p = Arc::new(w.compile());
    let mut scratch = p.new_scratch();
    let a = p.evaluate(&mut scratch).unwrap();
    check(&a, 0, 22., 78.);
    check(&a, 1, 34., 66.);
    let b = World::new([1, 22], Some(80.)).compile();
    b.evaluate(&mut scratch).unwrap();
    assert_eq!(a, p.evaluate(&mut scratch).unwrap());
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let p = Arc::clone(&p);
            std::thread::spawn(move || {
                let mut s = p.new_scratch();
                let a = p.evaluate(&mut s).unwrap();
                for _ in 0..8 {
                    assert_eq!(a, p.evaluate(&mut s).unwrap());
                }
                a
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(a, worker.join().unwrap());
    }
}
#[test]
fn production_recipe_preserves_partial_membership_and_exact_existing_route() {
    let e: Value = asset("minion-accuracy", "extension.json");
    assert_eq!(e["schema"].as_array().unwrap().len(), 8);
    for owner in e["owners"].as_array().unwrap() {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    let r: Value = asset("minion-accuracy", "routes.json");
    assert_eq!(r[0]["routes"]["members"].as_array().unwrap().len(), 1);
    assert_eq!(r[0]["routes"]["members"][0]["selection"]["kind"], "exact");
    assert_eq!(r[0]["routes"]["closure"]["kind"], "partial");
}

fn read(path: impl AsRef<std::path::Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
#[test]
#[ignore = "requires checked combined configuration/native publication"]
fn published_data_retains_exact_programs_dependencies_and_previous_intrinsic_routes() {
    let package = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_ACCURACY_PACKAGE").expect("published package"),
    );
    authenticate_prerequisites(&package);
    authenticate_source();
    let schema = read(package.join("schema.json"));
    let rules = read(package.join("rules.json"));
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(schema["definitions"].clone()).unwrap();
    let routes = read(package.join("routing.json"));
    for family in ["configuration-block-inputs", "minion-accuracy"] {
        let e: Value = asset(family, "extension.json");
        for row in e["schema"].as_array().unwrap() {
            let definition: DefinitionDescriptor =
                serde_json::from_value(row["value"].clone()).unwrap();
            assert!(definitions.contains(&definition));
        }
        let owners: Vec<DefinitionRules> = serde_json::from_value(e["owners"].clone()).unwrap();
        let actual: Vec<DefinitionRules> = serde_json::from_value(rules["owners"].clone()).unwrap();
        for owner in owners {
            let found = actual.iter().find(|r| r.owner == owner.owner).unwrap();
            assert_eq!(found.programs.closure, owner.programs.closure);
            for p in owner.programs.members {
                assert!(found.programs.members.contains(&p));
            }
        }
    }
    let expected: Value = asset("minion-attack-source", "routes.json");
    let added: Value = asset("minion-accuracy", "routes.json");
    let route = routes["outputs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["output"] == expected[0]["output"])
        .unwrap();
    for row in expected[0]["routes"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .chain(added[0]["routes"]["members"].as_array().unwrap())
    {
        assert!(route["routes"]["members"].as_array().unwrap().contains(row));
    }
    assert_eq!(route["routes"]["members"].as_array().unwrap().len(), 5);
    replay_golden();
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn golden() -> Value {
    read(repository().join("tests/fixtures/calibration/minion-accuracy-3887ae68.json"))
}
fn records(value: &Value) -> &[Value] {
    if let Some(a) = value.as_array() {
        a
    } else {
        assert!(
            value.as_object().is_some_and(|o| o.is_empty()),
            "not records: {value}"
        );
        &[]
    }
}
fn number(value: &Value) -> f64 {
    value.as_f64().unwrap()
}
fn replay_golden() {
    let g = golden();
    let authoring: Value = asset("minion-accuracy", "authoring.json");
    assert_eq!(g["source_revision"], authoring["source_revision"]);
    assert_eq!(g["source_hash"], authoring["source_manifest_sha256"]);
    let mut bypasses = 0;
    let mut zeros = 0;
    let mut placeholders = 0;
    let rows = g["sniper"].as_array().unwrap();
    assert_eq!(rows.len(), 13);
    for row in rows {
        assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
        assert_eq!(row["summon_effect_id"], "SummonSkeletalSnipersPlayer");
        assert_eq!(row["consumer"]["effect_id"], "MinionMeleeBow");
        let raw = &row["config"]["enemy_block"];
        // V2's selected saved value: Input (including zero), then Placeholder.
        // These remain request assumptions. They are not copied final PoB stats.
        let saved = if raw["input_present"] == true {
            Some(number(&raw["input"]))
        } else if raw["placeholder_present"] == true {
            placeholders += 1;
            Some(number(&raw["placeholder"]))
        } else {
            None
        };
        if saved == Some(0.) {
            zeros += 1;
        }
        let config = records(&row["config"]["block_records"]);
        match saved {
            Some(n) => {
                assert_eq!(config.len(), 1);
                close(number(&config[0]["value"]), n);
                assert_eq!(config[0]["mod"]["source"], "Config");
            }
            None => assert!(config.is_empty()),
        }
        let passes = row["consumer"]["passes"].as_array().unwrap();
        assert_eq!(passes.len(), 1);
        let pass = &passes[0];
        let flags = &pass["flags"];
        assert_eq!(flags["attack"], true);
        assert_eq!(flags["player_minion_accuracy_equals_accuracy"], false);
        assert_eq!(flags["actor_cannot_be_evaded"], true);
        assert_eq!(pass["skill_inherits_actor_modifiers"], true);
        assert!(records(&pass["inheritance_records"]).is_empty());
        assert!(
            records(&pass["actor_flag_records"])
                .iter()
                .any(|r| r["mod"]["source"] == "Minion Attacks always hit"
                    && r["mod"]["name"] == "CannotBeEvaded"
                    && r["mod"]["value"] == 1)
        );
        // All measured numerical BASE entries in this finite corpus are Config.
        // Refuse unexplained sources instead of compensating with observed totals.
        close(number(&pass["block"]["base"]), saved.unwrap_or(0.));
        close(number(&pass["block"]["reduction"]), 0.);
        assert!(records(&pass["block"]["reduction_records"]).is_empty());
        let level = u16::try_from(row["effective_level"].as_u64().unwrap()).unwrap();
        let mut world = World::new([level, 1], saved);
        if flags["enemy_cannot_block_attacks"] == true {
            assert!(!records(&pass["block"]["cannot_block_records"]).is_empty());
            // Authenticated custom-source contrast only. This deliberately adds
            // a finite fixture producer; the release admits no custom parser.
            world.contribute(
                "fixture-source-cannot-block",
                RuleEntity::Enemy,
                0x321e,
                integer(1),
            );
            bypasses += 1;
        } else {
            assert!(records(&pass["block"]["cannot_block_records"]).is_empty());
        }
        let report = world.evaluate();
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        assert_eq!(
            actor_value(&report, 0, 0x1c),
            &EffectValue::Known {
                value: integer(row["actor_level"].as_i64().unwrap())
            }
        );
        for (stat, field) in [
            (0x321c, "AccuracyHitChance"),
            (0x321f, "enemyBlockChance"),
            (0x3220, "HitChance"),
        ] {
            close(
                known(action_value(&report, 0, stat)),
                number(&pass["output"][field]),
            );
        }
    }
    assert!(
        bypasses > 0 && zeros > 0 && placeholders > 0,
        "non-vacuous source controls"
    );
}
#[test]
fn committed_full_source_vectors_match_native_minion_attack_consumers() {
    replay_golden();
}

fn authenticate_source() {
    let a: Value = asset("minion-accuracy", "authoring.json");
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let off = fs::read(repository().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    assert_eq!(
        off,
        fs::read(repository().join(proof["evidence_on_json"].as_str().unwrap())).unwrap()
    );
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(source["source_revision"], a["source_revision"]);
    assert_eq!(source["source_hash"], a["source_manifest_sha256"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert!(
            source["evidence"]["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|actual| actual["path"] == pin["path"] && actual["sha256"] == pin["sha256"])
        );
    }
    assert_eq!(source_vectors::project(&source), golden());
}

fn authenticate_prerequisites(package: &std::path::Path) {
    let snapshot: Value =
        serde_json::from_str(include_str!("support/minion_attack_source_snapshot.json")).unwrap();
    let schema = read(package.join("schema.json"));
    let rules = read(package.join("rules.json"));
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(schema["definitions"].clone()).unwrap();
    let slots: Vec<SlotDescriptor> = serde_json::from_value(schema["slots"].clone()).unwrap();
    let owners: Vec<DefinitionRules> = serde_json::from_value(rules["owners"].clone()).unwrap();
    let tables: Vec<IntegerRuleTable> = serde_json::from_value(rules["tables"].clone()).unwrap();
    let old_defs: Vec<DefinitionDescriptor> =
        serde_json::from_value(snapshot["schema_definitions"].clone()).unwrap();
    let old_slots: Vec<SlotDescriptor> =
        serde_json::from_value(snapshot["schema_slots"].clone()).unwrap();
    for definition in old_defs {
        assert!(
            definitions.contains(&definition),
            "actual prerequisite definition {:?}",
            definition.address()
        );
    }
    for slot in old_slots {
        assert!(
            slots.contains(&slot),
            "actual prerequisite slot {:?}",
            slot.address()
        );
    }
    let mut expected_owners: Vec<DefinitionRules> =
        serde_json::from_value(snapshot["owners"].clone()).unwrap();
    let mut expected_tables: Vec<IntegerRuleTable> =
        serde_json::from_value(snapshot["tables"].clone()).unwrap();
    for family in ["minion-attack-source", "minion-accuracy"] {
        let dependencies: Vec<DefinitionDescriptor> = asset(family, "dependencies.json");
        for definition in dependencies {
            assert!(
                definitions.contains(&definition),
                "actual dependency {:?}",
                definition.address()
            );
        }
        let extension: Value = asset(family, "extension.json");
        for row in extension["schema"].as_array().unwrap() {
            let definition: DefinitionDescriptor =
                serde_json::from_value(row["value"].clone()).unwrap();
            assert!(definitions.contains(&definition));
        }
        expected_owners.extend(
            serde_json::from_value::<Vec<DefinitionRules>>(extension["owners"].clone()).unwrap(),
        );
        expected_tables.extend(
            serde_json::from_value::<Vec<IntegerRuleTable>>(extension["tables"].clone()).unwrap(),
        );
    }
    for expected in expected_owners {
        let owner = owners.iter().find(|o| o.owner == expected.owner).unwrap();
        assert_eq!(owner.programs.closure, expected.programs.closure);
        for program in expected.programs.members {
            assert_eq!(
                owner
                    .programs
                    .members
                    .iter()
                    .filter(|p| p.id == program.id)
                    .count(),
                1
            );
            assert!(
                owner.programs.members.contains(&program),
                "unchanged actual program {}",
                program.id
            );
        }
    }
    for table in expected_tables {
        assert!(tables.contains(&table), "actual table {}", table.id);
    }
}
