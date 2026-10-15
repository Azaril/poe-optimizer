#[path = "support/owned_mana_supplier_sources.rs"]
mod proof;

use poe_optimizer_core::{
    owned_draft::{DraftLimits, decode_draft},
    owned_rules::{ContributionKind, RuleEffectKind},
};
use serde_json::{Value, json};

fn refresh_draft_commitments(i: &mut proof::Inputs) {
    let draft = decode_draft(
        &serde_json::to_vec(&i.draft).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let digest = serde_json::to_value(
        draft
            .digest(DraftLimits::default().input.max_wire_bytes)
            .unwrap(),
    )
    .unwrap();
    i.sidecar["draft"] = digest.clone();
    i.report["draft_digest"] = digest;
    i.report["sidecar_sha256"] = json!(proof::hash(&serde_json::to_vec(&i.sidecar).unwrap()));
}

#[test]
fn retained_supplier_bodies_and_exact_selected_sources_reconstruct_without_pob() {
    proof::check_authored().unwrap();
}

#[test]
fn zero_inactive_and_application_native_suppliers_require_new_review() {
    let i = proof::load();
    proof::check_candidate_supplier_inventory(&i.rules).unwrap();
    let collector = proof::collectors()[0].clone();
    let mut program = collector.programs.members[0].clone();
    let stat = match &program.effects[0].effect {
        RuleEffectKind::Derive { stat, .. } => stat.clone(),
        _ => unreachable!(),
    };
    program.reads.clear();
    program.nodes=serde_json::from_value(json!([
        {"id":"resolved","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0,"unit":{"kind":"unit","namespace":{"game":"poe2","version":"owned-mechanics-v1"},"key":"def.0000000000000002"}}}}},
        {"id":"inactive","expression":{"kind":"literal","value":{"kind":"boolean","value":false}}}
    ])).unwrap();
    program.effects[0].effect = RuleEffectKind::Contribute {
        entity: poe_optimizer_core::owned_rules::RuleEntity::Current,
        stat,
        contribution: ContributionKind::Add,
        value: "resolved".parse().unwrap(),
    };
    for inactive in [false, true] {
        let mut changed = i.rules.clone();
        let owner = changed
            .owners
            .iter_mut()
            .find(|o| !proof::collectors().iter().any(|c| c.owner == o.owner))
            .unwrap();
        let mut p = program.clone();
        if inactive {
            p.effects[0].when = Some("inactive".parse().unwrap());
        }
        owner.programs.members.push(p);
        let err = proof::check_candidate_supplier_inventory(&changed).unwrap_err();
        assert!(
            err.starts_with("unreviewed ordinary Mana supplier:"),
            "{err}"
        );
    }
    let mut changed = i.rules.clone();
    let application = &mut changed.effect_applications.as_mut().unwrap().members[0];
    application.program = program;
    application.activation = "inactive".parse().unwrap();
    let err = proof::check_candidate_supplier_inventory(&changed).unwrap_err();
    assert!(
        err.starts_with("unreviewed application Mana supplier:"),
        "{err}"
    );
    let mut changed = i.rules;
    let owner = changed
        .owners
        .iter_mut()
        .find(|o| o.owner == collector.owner)
        .unwrap();
    owner.programs.members[0].effects[0].when = Some("inactive".parse().unwrap());
    assert_eq!(
        proof::check_candidate_supplier_inventory(&changed).unwrap_err(),
        "reviewed adjustment collector body changed"
    );
}

#[test]
fn source_record_refusals_include_zero_conditional_nested_and_incoming_sources() {
    let names = proof::relevant_names();
    assert_eq!(names.len(), 15);
    for name in names {
        let body = json!({"outer":{"kind":"application","enabled":false,"nested":{"name":name,"type":"BASE","value":0,"_positions":[{"index":1,"value":{"type":"Condition","var":"Disabled"}}]}}});
        let err = proof::check_neutral_body_inventory(&body).unwrap_err();
        assert_eq!(err, "unreviewed Mana source record at /outer/nested");
        assert_eq!(
            proof::source_candidates(&body)[0]["body"],
            body["outer"]["nested"]
        );
    }
}

#[test]
fn recomputed_membership_changes_and_foreign_lineages_refuse_at_correspondence() {
    let original = proof::load();
    for mutation in 0..9 {
        let mut i = original.clone();
        let expected = match mutation {
            0 => {
                let id = i.selection["build"]["equipment"].clone();
                let p = i.draft["draft"]["equipment_presets"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == id)
                    .unwrap();
                p["equipment"]["members"].as_array_mut().unwrap().remove(5);
                refresh_draft_commitments(&mut i);
                "selected source memberships or authored values changed"
            }
            1 => {
                let id = i.selection["build"]["skills"].clone();
                let p = i.draft["draft"]["skill_presets"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == id)
                    .unwrap();
                let bindings = p["intent"]["generated_inputs"]["members"]
                    .as_array_mut()
                    .unwrap();
                bindings.remove(0);
                refresh_draft_commitments(&mut i);
                "selected source memberships or authored values changed"
            }
            2 => {
                // Move a real item use into the allocation preset's socket domain.
                // The typed value remains a real ID; recomputed content digests
                // cannot authorize the changed placement membership.
                let use_id = i.draft["draft"]["equipment"]["members"][0]["id"].clone();
                let id = i.selection["build"]["allocations"].clone();
                let p = i.draft["draft"]["allocation_presets"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|p| p["id"] == id)
                    .unwrap();
                p["equipment"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(use_id);
                refresh_draft_commitments(&mut i);
                "selected source memberships or authored values changed"
            }
            3 => {
                i.selection["build"]["skills"]["lineage"] =
                    json!("00112233445566778899aabbccddeeff");
                "selected presets or loadout changed"
            }
            4 => {
                i.sidecar["policy"] = json!("unreviewed");
                "current normalization policy commitments differ"
            }
            5 => {
                let links = i.sidecar["origins"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|o| o["links"].as_array_mut().unwrap())
                    .find(|l| l["kind"] == "equipment")
                    .unwrap();
                links["value"]["lineage"] = json!("00112233445566778899aabbccddeeff");
                refresh_draft_commitments(&mut i);
                "source rows, bindings or unresolved obligations changed"
            }
            6 => {
                let links = i.sidecar["origins"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|o| o["links"].as_array_mut().unwrap())
                    .find(|l| l["kind"] == "generated_skill_input")
                    .unwrap();
                links["value"]["target"]["provider"]["grant_path"] = json!(["unreviewed"]);
                refresh_draft_commitments(&mut i);
                "source rows, bindings or unresolved obligations changed"
            }
            7 => {
                i.sidecar["origins"].as_array_mut().unwrap().pop();
                refresh_draft_commitments(&mut i);
                "source rows, bindings or unresolved obligations changed"
            }
            _ => {
                let links = i.sidecar["origins"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .flat_map(|o| o["links"].as_array_mut().unwrap())
                    .find(|l| l["kind"] == "generated_skill_input")
                    .unwrap();
                links["value"]["skill_preset"]["lineage"] =
                    json!("00112233445566778899aabbccddeeff");
                refresh_draft_commitments(&mut i);
                "source rows, bindings or unresolved obligations changed"
            }
        };
        assert_eq!(
            proof::reconcile_selected_inputs(&i).unwrap_err(),
            expected,
            "mutation {mutation}"
        );
    }
}

#[test]
#[ignore = "explicit retention from reviewed original-source acquisition and canonical import"]
fn retain_mana_supplier_source_packet() {
    proof::retain();
}

#[test]
fn source_candidate_census_preserves_duplicate_multiplicity_and_raw_conditions() {
    let record = json!({"name":"ExtraMana","type":"BASE","value":0,"condition":"unavailable"});
    let source = json!({"records":[record.clone(),record]});
    let rows = proof::source_candidates(&source);
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0]["path"], rows[1]["path"]);
    assert_eq!(rows[0]["body"], rows[1]["body"]);
    assert_eq!(proof::source_candidates(&Value::Null), Vec::<Value>::new());
}

#[test]
fn initialization_exclusion_refuses_missing_records_visits_and_new_zero_suppliers() {
    let (setup, original) = proof::initialization_fixture();
    proof::review_initialization_bodies(&setup, &original).unwrap();
    for mutation in 0..4 {
        let mut state = original.clone();
        let expected = match mutation {
            0 => {
                state["state"]["baseline"].as_array_mut().unwrap().pop();
                "missing baseline construction visit"
            }
            1 => {
                state["state"]["baseline"][0]["player_store"][0]["buckets"]
                    .as_object_mut()
                    .unwrap()
                    .remove("Mana");
                "baseline record names or multiplicity differ from complete reviewed writer bodies"
            }
            2 => {
                state["state"]["baseline"][0]["player_store"][0]["buckets"]["ExtraMana"] = json!([{"position":1,"body":{"name":"ExtraMana","type":"BASE","value":0,"enabled":false}}]);
                "unreviewed Mana source record at /0/buckets/ExtraMana/0/body"
            }
            _ => {
                state["state"]["baseline"][0]["class_fields"]["base_int"] = json!(14);
                "class attribute source projection differs"
            }
        };
        assert_eq!(
            proof::review_initialization_bodies(&setup, &state).unwrap_err(),
            expected,
            "mutation {mutation}"
        );
    }
    assert_eq!(
        proof::review_initialization_bodies(&(setup + "\n-- altered body\n"), &original)
            .unwrap_err(),
        "reviewed initialization source bytes changed"
    );
}
