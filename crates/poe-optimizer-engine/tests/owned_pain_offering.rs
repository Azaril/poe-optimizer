//! Authored Pain Offering arithmetic within an explicitly finite component.
//! This does not prove saved usage, support-final inputs, or full-build parity.
#[path = "support/pain_offering_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf, sync::Arc};

fn delivered(report: &OwnedEffectsReport, recipient: usize) -> f64 {
    known(&group(report, recipient).value)
}
fn check(report: &OwnedEffectsReport, values: [f64; 2]) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(report.application_groups.len(), 2);
    for (recipient, value) in values.into_iter().enumerate() {
        assert_eq!(delivered(report, recipient), value);
    }
}
#[test]
fn real_physical_supply_and_injected_level_table_reach_both_exact_snipers() {
    for (level, expected) in [(1, 20.), (3, 24.), (22, 62.), (32, 80.), (40, 88.)] {
        let world = World::new(&[Offering::at_final_level(level)], [(0., 1.); 2]);
        let report = world.evaluate();
        check(&report, [expected; 2]);
        for index in 0..2 {
            let group = group(&report, index);
            assert_eq!(group.candidates.len(), 1);
            assert_eq!(group.co_winners, group.candidates);
            assert!(matches!(&group.co_winners[0].invocation.origin,
                RuleOrigin::EffectApplication {
                    source: ConcreteEntity::Skill(source),
                    recipient: ConcreteEntity::Actor(recipient), ..
                } if source.as_ref() == &offering_target(0) && *recipient == actor(index)
            ));
        }
        // The raw source remains level20. The final level is a separately
        // supplied test boundary, never the observed Damage INC itself.
        assert_eq!(world.intrinsic.f.build.gems.last().unwrap().level, 20);
    }
}
#[test]
fn source_and_recipient_scaling_happen_before_group_maximum() {
    let mut high = Offering::at_final_level(22);
    high.increased = 30.;
    let mut low = Offering::at_final_level(3);
    low.increased = 300.;
    // Source0 wins for recipient1 but source1 wins for recipient0. Grouping
    // sources before applying recipient scaling would produce the wrong result.
    let report = World::new(&[high.clone(), low.clone()], [(0., 1.), (200., 1.)]).evaluate();
    check(&report, [96., 204.]);
    for (recipient, source) in [(0, 1), (1, 0)] {
        let row = group(&report, recipient);
        assert_eq!(row.candidates.len(), 2);
        assert_eq!(row.co_winners.len(), 1);
        assert!(matches!(&row.co_winners[0].invocation.origin,
            RuleOrigin::EffectApplication { source:ConcreteEntity::Skill(actual),.. }
                if actual.as_ref() == &offering_target(source)
        ));
    }
    let reversed = World::new(&[low, high], [(0., 1.), (200., 1.)]).evaluate();
    check(&reversed, [96., 204.]);
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
fn authored_scaling_keeps_magnitude_more_factors_and_source_rounding() {
    let mut offering = Offering::at_final_level(22);
    offering.increased = 30.;
    offering.more = 1.2;
    offering.magnitude = 0.75;
    let report = World::new(&[offering], [(10., 1.1), (40., 0.9)]).evaluate();
    check(&report, [85., 85.]);
    // The source first rounds to two decimals, then truncates to integer INC.
    // This deliberately straddles the integer boundary before rounding.
    // The negative decimal lies just below the binary tie: the literal product
    // is -0.9950000000000001 and the biased hundred is -99.00000000000001.
    for (scale, expected) in [(0.04974, 0.), (0.04975, 1.), (-0.04975, -1.)] {
        let mut offering = Offering::at_final_level(1);
        offering.magnitude = scale;
        check(
            &World::new(&[offering], [(0., 1.); 2]).evaluate(),
            [expected; 2],
        );
    }
}
#[test]
fn exact_reference_control_inputs_keep_the_original_binary_operation_order() {
    // These are finite boundary inputs, not observed Damage values supplied to
    // the evaluator. The authored level22 table supplies the base62 itself.
    for (increased, more, recipient_more, magnitude, expected) in [
        (0., 0.75, 0.43, 1., 20.),
        (-175., 1., 0.43, 1., -19.),
        (0., 1.25, 1.5, 1., 116.),
        (0., 1., 1., 1.5, 93.),
    ] {
        let mut offering = Offering::at_final_level(22);
        offering.increased = increased;
        offering.more = more;
        offering.magnitude = magnitude;
        check(
            &World::new(&[offering], [(0., recipient_more); 2]).evaluate(),
            [expected; 2],
        );
    }
}
#[test]
fn activation_final_inputs_and_incomplete_membership_remain_distinct() {
    let mut absent = Offering::at_final_level(22);
    absent.active = Some(false);
    let mut inactive = World::new(&[absent], [(0., 1.); 2]);
    // Optional unused scaling facts are lazy after an explicit false activation.
    inactive.intrinsic.f.build.choices.retain(|row| {
        row.choice.slot.slot.key().as_str() != "fixture.pain-offering.source-magnitude"
    });
    let report = inactive.evaluate();
    assert!(report.gaps.is_empty());
    assert!(
        report
            .application_groups
            .iter()
            .all(|g| g.value == EffectValue::Inactive)
    );
    let mut unknown = Offering::at_final_level(22);
    unknown.active = None;
    let report = World::new(&[unknown, Offering::at_final_level(3)], [(0., 1.); 2]).evaluate();
    assert!(
        report
            .application_groups
            .iter()
            .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
    );
    let mut missing = World::new(&[Offering::at_final_level(22)], [(0., 1.); 2]);
    missing.missing_final_producer();
    let report = missing.evaluate();
    assert!(
        report
            .application_groups
            .iter()
            .all(|g| matches!(g.value, EffectValue::Unresolved { .. }))
    );
    let mut partial = World::new(&[Offering::at_final_level(22)], [(0., 1.); 2]);
    partial.restore_partial_registry();
    let report = partial.evaluate();
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
    for level in [0, 41] {
        let report = World::new(&[Offering::at_final_level(level)], [(0., 1.); 2]).evaluate();
        assert!(
            report
                .application_groups
                .iter()
                .all(|g| !matches!(g.value, EffectValue::Known { .. }))
        );
    }
}
#[test]
fn plans_reuse_scratch_and_isolate_rayon_workers() {
    let a = Arc::new(World::new(&[Offering::at_final_level(22)], [(0., 1.), (25., 1.)]).compile());
    let b = World::new(&[Offering::at_final_level(3)], [(100., 1.), (0., 1.)]).compile();
    let mut scratch = a.new_scratch();
    let expected = a.evaluate(&mut scratch).unwrap();
    check(&expected, [62., 77.]);
    check(&b.evaluate(&mut scratch).unwrap(), [48., 24.]);
    assert_eq!(expected, a.evaluate(&mut scratch).unwrap());
    let workers: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| {
            let mut scratch = a.new_scratch();
            let report = a.evaluate(&mut scratch).unwrap();
            for _ in 0..4 {
                assert_eq!(report, a.evaluate(&mut scratch).unwrap());
            }
            report
        })
        .collect();
    assert!(workers.iter().all(|report| report == &expected));
}
#[test]
fn production_assets_keep_the_unimplemented_domains_explicit() {
    let applications: DeclaredSet<EffectApplicationRule> = asset("applications.json");
    assert!(!applications.is_complete());
    assert_eq!(applications.members.len(), 1);
    assert_eq!(
        applications.members[0].targets,
        [EffectApplicationTarget::OwnedSlot { slot: actor_slot() }]
    );
    assert_eq!(
        applications.members[0].source,
        EffectApplicationSource::Skill { skill: def(0x2a2) }
    );
    let extension: Value = asset("extension.json");
    for owner in extension["owners"].as_array().unwrap() {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    let bindings: Value = asset("bindings.json");
    assert_eq!(
        bindings["physical_final_level"]["key"],
        "def.0000000000003225"
    );
    assert_eq!(bindings["effect_active"]["key"], "def.0000000000003227");
    assert_eq!(bindings["delivered_damage"]["key"], "def.000000000000322d");
    let tables: Vec<IntegerRuleTable> =
        serde_json::from_value(extension["tables"].clone()).unwrap();
    assert_eq!(tables.len(), 1);
    // No complete real original or support-final producer is manufactured by
    // publishing the real supply and component arithmetic.
    let owners: Vec<DefinitionRules> = serde_json::from_value(extension["owners"].clone()).unwrap();
    assert!(owners.iter().flat_map(|o| &o.programs.members).flat_map(|p| &p.effects).all(|e| {
        !matches!(&e.effect, RuleEffectKind::Derive { stat, .. } if *stat == def(0x3225) || *stat == def(0x3226))
    }));
}

fn source_rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(
            value.as_object().is_some_and(|map| map.is_empty()),
            "not a source list: {value}"
        );
        &[]
    }
}
fn single<'a>(rows: impl Iterator<Item = &'a Value>, label: &str) -> &'a Value {
    let found: Vec<_> = rows.collect();
    assert_eq!(found.len(), 1, "exact {label}");
    found[0]
}
fn authenticated_source() -> Value {
    let authoring: Value = asset("authoring.json");
    let proof = &authoring["source_validation"];
    assert_eq!(
        proof["status"], "passed",
        "fresh complete-source witness required"
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let off = fs::read(root.join(proof["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root.join(proof["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(off, on, "both JIT modes have the same canonical evidence");
    assert_eq!(off.len() as u64, proof["evidence_bytes"].as_u64().unwrap());
    assert_eq!(
        format!("{:x}", Sha256::digest(&off)),
        proof["evidence_sha256"]
    );
    let source: Value = serde_json::from_slice(&off).unwrap();
    assert_eq!(source["source_revision"], authoring["source_revision"]);
    assert_eq!(source["source_hash"], authoring["source_manifest_sha256"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        single(
            source_rows(&source["evidence"]["files"])
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"]),
            "authenticated source pin",
        );
    }
    assert_eq!(source["evidence"]["business_method_wrappers"], false);
    assert_eq!(source["evidence"]["whole_build_parity"], false);
    source
}

#[test]
#[ignore = "requires the fresh passed Pain Offering source evidence named by authoring.json"]
fn fresh_source_modifiers_match_native_finite_application_delivery() {
    let source = authenticated_source();
    let cases = source_rows(&source["cases"]);
    let original = single(
        cases.iter().filter(|row| row["name"] == "original-05"),
        "original build",
    );
    let definition = &original["state"]["offering_definition"];
    assert_eq!(definition["source_tables_preserved"], true);
    assert_eq!(definition["physical_effect_identity"], true);
    let stat_set = single(
        source_rows(&definition["stat_sets"]).iter(),
        "Offering stat set",
    );
    let damage_column = source_rows(&stat_set["stats"])
        .iter()
        .position(|stat| stat == "pain_offering_damage_+%")
        .unwrap();
    let extension: Value = asset("extension.json");
    let table = single(
        source_rows(&extension["tables"])
            .iter()
            .filter(|table| table["id"] == "pain-offering.damage-increase"),
        "authored Damage table",
    );
    let levels = source_rows(&stat_set["levels"]);
    let owned_rows = source_rows(&table["rows"]);
    assert_eq!(levels.len(), 40);
    assert_eq!(owned_rows.len(), levels.len());
    for (index, (level, owned)) in levels.iter().zip(owned_rows).enumerate() {
        assert_eq!(level["level"], index + 1);
        assert_eq!(
            owned["value"]["value"].as_f64().unwrap(),
            level["values"][damage_column].as_f64().unwrap()
        );
        assert_eq!(owned["value"]["unit"]["key"], "def.0000000000000002");
    }
    let mut compared_candidates = 0;
    let mut tied_groups = 0;
    for name in [
        "original-05",
        "offering-level-1",
        "offering-duplicate-equal",
        "offering-higher-first",
        "offering-higher-last",
        "offering-source-buff-effect",
        "offering-recipient-buff-effect",
        "offering-both-buff-effects",
        "offering-positive-half-tie",
        "offering-negative-half-tie",
        "offering-combined-more",
        "offering-magnitude",
    ] {
        let case = single(cases.iter().filter(|row| row["name"] == name), name);
        assert_eq!(case["available"], true, "{name}");
        let env = &case["state"]["main"];
        let selected = single(
            source_rows(&env["actors"]).iter().filter(|actor| {
                actor["actor_profile"] == "RaisedSkeletonSniper"
                    && actor["is_environment_minion"] == true
            }),
            "selected reference Sniper",
        );
        let selected_source = single(
            source_rows(&env["selected_minion"]["matches"]).iter(),
            "selected source identity",
        );
        assert_eq!(selected_source["source"], selected["source_occurrence"]);
        let child = single(
            source_rows(&selected["children"])
                .iter()
                .filter(|child| child["effect_id"] == "MinionMeleeBow"),
            "reference basic attack",
        );
        let call = single(
            source_rows(&child["damage_calls"])
                .iter()
                .filter(|call| call["damage_type"] == "Physical" && call["critical"] == false),
            "noncritical physical call",
        );
        let received = single(
            source_rows(&call["increased_records"])
                .iter()
                .filter(|row| {
                    row["mod"]["source"] == "Skill:PainOfferingPlayer"
                        && row["mod"]["name"] == "Damage"
                }),
            "actually received Offering modifier",
        );
        let expected = received["value"].as_f64().unwrap();
        let events = source_rows(&env["offering_merge_events"]);
        assert_eq!(
            events.len(),
            if name.starts_with("offering-higher-") || name == "offering-duplicate-equal" {
                2
            } else {
                1
            }
        );
        let mut source_identities = BTreeSet::new();
        let mut offerings = Vec::new();
        let mut candidate_values = Vec::new();
        let mut recipient = None;
        for event in events {
            assert_eq!(event["source_effect"], "PainOfferingPlayer");
            assert_eq!(event["recipient_profile"], "RaisedSkeletonSniper");
            assert_eq!(event["minion_destination"], true);
            assert_eq!(event["recipient_occurrence"], env["selected_minion"]);
            assert!(
                source_identities
                    .insert(serde_json::to_string(&event["source_occurrence"]).unwrap()),
                "distinct exact Offering occurrences"
            );
            let skill = single(
                source_rows(&env["skills"]).iter().filter(|skill| {
                    skill["effect_id"] == "PainOfferingPlayer"
                        && skill["source_occurrence"] == event["source_occurrence"]
                }),
                "source of observed merge",
            );
            assert_eq!(skill["effective_level"], event["source_level"]);
            let buff = single(source_rows(&skill["buffs"]).iter(), "source Offering buff");
            assert_eq!(buff["fields"]["applyMinions"], true);
            assert_eq!(buff["fields"]["applyNotPlayer"], true);
            let scaling = &buff["scaling"];
            assert_eq!(scaling["source_store_is_skill"], true);
            assert_eq!(scaling["recipient_occurrence"], env["selected_minion"]);
            let input = |key: &str| scaling[key]["value"].as_f64().unwrap();
            let current_recipient = (input("recipient_increased"), input("recipient_more"));
            if let Some(previous) = recipient {
                assert_eq!(current_recipient, previous, "same actual recipient");
            }
            recipient = Some(current_recipient);
            let mut offering = Offering::at_final_level(skill["effective_level"].as_i64().unwrap());
            offering.raw_level = u16::try_from(skill["physical_level"].as_u64().unwrap()).unwrap();
            offering.increased = input("source_buff_increased");
            offering.more = input("source_buff_more");
            // Resolution of these measured scaling channels is a finite input
            // boundary. The expected delivered Damage is never fed as an input.
            offering.magnitude =
                (1. + input("source_magnitude_increased") / 100.) * input("source_magnitude_more");
            offerings.push(offering);
            let candidate = single(
                source_rows(&event["source_modifiers"])
                    .iter()
                    .filter(|row| row["name"] == "Damage" && row["type"] == "INC"),
                "scaled source Damage",
            );
            candidate_values.push(candidate["value"].as_f64().unwrap());
        }
        let world = World::new(&offerings, [recipient.unwrap(), (0., 1.)]);
        let report = world.evaluate();
        assert!(report.gaps.is_empty(), "{name}: {:?}", report.gaps);
        // Only actor0 is joined to the actually evaluated source recipient.
        // The second finite actor is covered by independent isolation tests.
        assert_eq!(delivered(&report, 0), expected, "{name}");
        let native = group(&report, 0);
        assert_eq!(native.candidates.len(), events.len());
        let winners: Vec<_> = candidate_values
            .iter()
            .enumerate()
            .filter(|(_, value)| **value == expected)
            .map(|(index, _)| offering_target(index))
            .collect();
        assert!(!winners.is_empty());
        assert_eq!(native.co_winners.len(), winners.len(), "{name}");
        for winner in &native.co_winners {
            assert!(
                matches!(&winner.invocation.origin, RuleOrigin::EffectApplication { source:ConcreteEntity::Skill(source), recipient:ConcreteEntity::Actor(recipient), .. }
                if winners.contains(source.as_ref()) && *recipient == actor(0))
            );
        }
        compared_candidates += events.len();
        tied_groups += usize::from(winners.len() > 1);
    }
    assert_eq!(compared_candidates, 15);
    assert_eq!(tied_groups, 1);
}
