//! Published usage activation reaches the existing native Offering application.
//! Final inputs/scaling remain finite fixture boundaries; no full-build claim.
#[path = "support/skill_usage_inputs_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{
    owned_build::*, owned_definitions::SlotOwnerDefId, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, sync::Arc};

fn check(report: &OwnedEffectsReport, expected: [f64; 2]) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(report.application_groups.len(), 2);
    for (recipient, expected) in expected.into_iter().enumerate() {
        assert_eq!(known(&group(report, recipient).value), expected);
    }
}
fn activation(report: &OwnedEffectsReport, index: usize) -> &EffectValue {
    let input = inputs();
    &report
        .effects
        .iter()
        .find(|row| {
            row.key.invocation.program == input.program
                && row.key.effect == input.effect
                && row.key.invocation.entity
                    == ConcreteEntity::Skill(Box::new(offering_target(index)))
                && matches!(row.key.invocation.origin, RuleOrigin::Usage { .. })
        })
        .expect("one exact native usage producer")
        .value
}

#[test]
fn composed_real_usage_policy_controls_existing_offering_delivery() {
    let mut world = World::new(&[Offering::at_final_level(22)], [(0., 1.), (25., 1.)]);
    let request = world.request();
    assert_eq!(request.scenario().input().usage, vec![usage(0, true)]);
    assert!(
        !request.build().input().choices.iter().any(|choice| choice
            .choice
            .slot
            .slot
            .key()
            .as_str()
            == "fixture.pain-offering.active")
    );
    let enabled = world.evaluate();
    check(&enabled, [62., 77.]);
    assert_eq!(
        activation(&enabled, 0),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    for recipient in 0..2 {
        let delivered = group(&enabled, recipient);
        assert_eq!(delivered.co_winners.len(), 1);
        assert!(matches!(&delivered.co_winners[0].invocation.origin,
            RuleOrigin::EffectApplication { source: ConcreteEntity::Skill(source), recipient: ConcreteEntity::Actor(target), .. }
            if **source == offering_target(0) && *target == actor(recipient)));
    }
    // Whole-record scenario replacement passes through Core composition before
    // the same native program and effect-application engine execute.
    world.component.intrinsic.f.scenario.usage = vec![usage(0, false)];
    let disabled = world.evaluate();
    assert_eq!(
        activation(&disabled, 0),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert!(
        disabled
            .application_groups
            .iter()
            .all(|g| g.value == EffectValue::Inactive)
    );
    world.component.intrinsic.f.scenario.usage.clear();
    world.preferences.clear();
    let missing = world.evaluate();
    assert!(
        missing
            .application_groups
            .iter()
            .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
    );
    assert!(
        !missing
            .effects
            .iter()
            .any(|row| matches!(row.key.invocation.origin, RuleOrigin::Usage { .. }))
    );
}

#[test]
fn duplicate_source_activation_keeps_exact_recipient_scaling_and_maximum_winners() {
    let mut high = Offering::at_final_level(22);
    high.increased = 30.;
    let mut low = Offering::at_final_level(3);
    low.increased = 300.;
    let mut world = World::new(&[high, low], [(0., 1.), (200., 1.)]);
    let enabled = world.evaluate();
    check(&enabled, [96., 204.]);
    for (recipient, winner) in [(0, 1), (1, 0)] {
        assert!(
            matches!(&group(&enabled, recipient).co_winners[0].invocation.origin,
            RuleOrigin::EffectApplication { source: ConcreteEntity::Skill(source), .. } if **source == offering_target(winner))
        );
    }
    world.component.intrinsic.f.scenario.usage = vec![usage(0, false)];
    let first_off = world.evaluate();
    check(&first_off, [96., 144.]);
    assert_eq!(
        activation(&first_off, 0),
        &EffectValue::Known {
            value: ParameterValue::Boolean(false)
        }
    );
    assert_eq!(
        activation(&first_off, 1),
        &EffectValue::Known {
            value: ParameterValue::Boolean(true)
        }
    );
    world.component.intrinsic.f.scenario.usage = vec![usage(1, false)];
    check(&world.evaluate(), [80., 204.]);
    let tied = World::new(
        &[Offering::at_final_level(22), Offering::at_final_level(22)],
        [(0., 1.); 2],
    )
    .evaluate();
    check(&tied, [62.; 2]);
    assert!(
        tied.application_groups
            .iter()
            .all(|g| g.co_winners.len() == 2)
    );
}

#[test]
fn usage_keeps_final_input_readiness_and_partial_application_membership_unresolved() {
    for enabled in [true, false] {
        let mut offering = Offering::at_final_level(22);
        offering.active = Some(enabled);
        let mut world = World::new(&[offering], [(0., 1.); 2]);
        world.component.missing_final_producer();
        let report = world.evaluate();
        assert!(
            matches!(activation(&report, 0), EffectValue::Unresolved { .. }),
            "{report:?}"
        );
        assert!(
            report
                .application_groups
                .iter()
                .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
        );
    }
    let mut world = World::new(&[Offering::at_final_level(22)], [(0., 1.); 2]);
    world.component.restore_partial_registry();
    let report = world.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialEffectApplications)
    );
    assert!(
        report
            .application_groups
            .iter()
            .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
    );
}

#[test]
fn published_usage_plans_reuse_scratch_and_isolate_rayon_workers() {
    let a = Arc::new(World::new(&[Offering::at_final_level(22)], [(0., 1.), (25., 1.)]).compile());
    let mut disabled = Offering::at_final_level(22);
    disabled.active = Some(false);
    let b = World::new(&[disabled], [(0., 1.); 2]).compile();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    check(&expected, [62., 77.]);
    assert!(
        b.evaluate(&mut scratch)
            .unwrap()
            .application_groups
            .iter()
            .all(|g| g.value == EffectValue::Inactive)
    );
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    let reports: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|report| report == &expected));
}

fn vectors() -> Value {
    let authoring: Value = asset("authoring.json");
    let bytes = fs::read(asset_path("reference-vectors.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&bytes)),
        authoring["source_validation"]["reference_vectors_sha256"]
    );
    let vectors: Value = serde_json::from_slice(&bytes).unwrap();
    for name in ["source_revision", "source_manifest_sha256", "source_files"] {
        assert_eq!(vectors[name], authoring[name]);
    }
    assert_eq!(
        vectors["source_evidence_sha256"],
        authoring["source_validation"]["evidence_sha256"]
    );
    assert_eq!(vectors["whole_build_parity"], false);
    assert_eq!(vectors["native_inventory_authority"], false);
    vectors
}

#[test]
fn measured_activation_and_count_controls_have_separate_native_meanings() {
    let vectors = vectors();
    let cases = vectors["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 3);
    let find = |name: &str| cases.iter().find(|case| case["name"] == name).unwrap();
    let original = find("original-05");
    let triple = find("pain-count-three");
    let disabled = find("pain-global-false");
    assert_eq!(original["loaded_count"], 1);
    assert_eq!(triple["loaded_count"], 3);
    assert_eq!(disabled["loaded_global_1"], false);
    for mode in ["MAIN", "CALCS"] {
        assert_eq!(
            original["modes"][mode]["actions"].as_array().unwrap().len(),
            1
        );
        assert!(
            disabled["modes"][mode]["actions"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        for metric in ["minion_combined_dps", "minion_speed"] {
            assert_eq!(
                original["modes"][mode][metric],
                triple["modes"][mode][metric]
            );
            assert!(
                disabled["modes"][mode][metric].as_f64().unwrap()
                    < original["modes"][mode][metric].as_f64().unwrap()
            );
        }
        assert_eq!(
            original["modes"][mode]["actions"][0]["minion_present"],
            false
        );
    }
    for case in cases {
        let mut offering = Offering::at_final_level(22);
        offering.active = Some(case["loaded_global_1"].as_bool().unwrap());
        let report = World::new(&[offering], [(0., 1.); 2]).evaluate();
        if case["loaded_global_1"] == true {
            check(&report, [62.; 2]);
        } else {
            assert!(
                report
                    .application_groups
                    .iter()
                    .all(|g| g.value == EffectValue::Inactive)
            );
        }
    }
    // Source DPS above authenticates the causal distinction only. It is never
    // injected into the native component or compared as full native DPS parity.
}

#[test]
fn published_assets_only_add_the_declared_boolean_usage_owner() {
    let input = inputs();
    assert_eq!(input.schema_version, 1);
    assert_eq!(input.policy, def(0x3259));
    assert_eq!(input.parameter.slot, def(0x325a));
    assert_eq!(input.effect_active, def(0x3227));
    assert_eq!(input.gem, def(0x86b));
    assert_eq!(input.skill, def(0x2a2));
    assert_eq!(
        input.primary_supply,
        pain::slot(SlotOwnerDefId::Gem(input.gem.clone()), 0x3221)
    );
    let extension: Value = asset("extension.json");
    assert_eq!(extension["schema"].as_array().unwrap().len(), 2);
    let owners: Vec<DefinitionRules> = serde_json::from_value(extension["owners"].clone()).unwrap();
    assert_eq!(owners.len(), 1);
    assert!(owners[0].programs.is_complete());
    assert_eq!(owners[0].programs.members.len(), 1);
    assert_eq!(owners[0].programs.members[0].effects.len(), 1);
    assert_eq!(
        extension["schema"][1]["value"]["value"]["schema"]["value"]["presence"],
        "required_once"
    );
    let applications: DeclaredSet<EffectApplicationRule> = pain::asset("applications.json");
    assert!(!applications.is_complete());
    let original: Value = pain::asset("extension.json");
    assert!(
        original["owners"]
            .as_array()
            .unwrap()
            .iter()
            .all(|owner| owner["programs"]["closure"]["kind"] == "partial")
    );
    let policy: Value = asset("policy.json");
    assert_eq!(policy["kind"], "pob_occurrence_usage_v3");
    let physical = policy["physical"].as_array().unwrap();
    assert_eq!(physical.len(), 1);
    assert_eq!(physical[0]["gem"], json!(input.gem));
    assert_eq!(physical[0]["primary"], json!(input.skill));
    let policies = physical[0]["policies"].as_array().unwrap();
    assert_eq!(policies.len(), 1);
    assert_eq!(policies[0]["policy"], json!(input.policy));
    let parameters = policies[0]["parameters"].as_array().unwrap();
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0]["slot"], json!(input.parameter));
    assert_eq!(parameters[0]["source"]["kind"], "occurrence");
    let source = &parameters[0]["source"]["value"];
    assert_eq!(source["missing"]["kind"], "pending");
    assert_eq!(
        source["codec"]["codec"]["value"]["tokens"],
        json!([{"token":"true","value":true},{"token":"false","value":false}])
    );
}

fn source_rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|o| o.is_empty()), "{value}");
        &[]
    }
}
#[test]
#[ignore = "requires authenticated complete active-occurrence source evidence"]
fn compact_usage_observations_match_the_original_full_source_witness() {
    let vectors = vectors();
    let authoring: Value = asset("authoring.json");
    let proof = &authoring["source_validation"];
    let off = fs::read(root().join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(off, on);
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(
        source["manifest_sha256"],
        authoring["source_manifest_sha256"]
    );
    assert_eq!(source["files"], authoring["source_files"]);
    assert_eq!(source["native_build_parity"], false);
    for case in vectors["cases"].as_array().unwrap() {
        let observed = if case["name"] == "original-05" {
            &source["originals"][4]
        } else {
            &source["controls"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["name"] == case["name"])
                .unwrap()["state"]
        };
        let selected: Vec<_> = source_rows(&observed["selected"])
            .iter()
            .filter(|row| row["physical_id"] == case["physical_id"])
            .collect();
        assert_eq!(selected.len(), 1);
        let row = selected[0];
        assert_eq!(
            case["original_xml_sha256"],
            source["original_xml_sha256"][4]
        );
        assert_eq!(case["attributes"], row["attributes"]);
        for (projected, raw) in [
            ("loaded_count", "count"),
            ("loaded_global_1", "global_1"),
            ("loaded_global_2", "global_2"),
        ] {
            assert_eq!(case[projected], row["loaded"][raw]);
        }
        for flag in [
            "saved_instances_preserved",
            "source_methods_preserved",
            "selected_state_preserved",
            "main_and_calcs_outputs_preserved",
        ] {
            assert_eq!(case[flag], observed[flag]);
            assert_eq!(case[flag], true);
        }
        for mode in ["MAIN", "CALCS"] {
            let captured = source_rows(&row[mode]);
            let projected = case["modes"][mode]["actions"].as_array().unwrap();
            assert_eq!(projected.len(), captured.len());
            for (expected, actual) in projected.iter().zip(captured) {
                for (field, value) in expected.as_object().unwrap() {
                    assert_eq!(value, &actual[field], "{} {mode} {field}", case["name"]);
                }
            }
            for (projected, metric) in [
                ("minion_combined_dps", "CombinedDPS"),
                ("minion_speed", "Speed"),
            ] {
                assert_eq!(
                    case["modes"][mode][projected].as_f64(),
                    observed["actor_outputs"][mode]["minion"][metric].as_f64()
                );
            }
        }
    }
}
