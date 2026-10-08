//! Actual Gem source domains; unrelated provider mechanics remain unresolved.
#[allow(dead_code)]
#[path = "support/owned_empty_payload_inventory.rs"]
mod catalog_evidence;
#[path = "support/owned_gem_support_domains.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn entire_catalog_has_explicit_known_or_unmapped_gem_source_domains() {
    family::check_authored();
}

#[test]
fn additional_supports_and_missing_effect_evidence_cannot_claim_absence() {
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    let catalog = family::identities();
    assert!(
        records
            .iter()
            .any(|r| family::assignment_only(r, &catalog) == Some(false))
    );
    let record = records
        .iter()
        .find(|r| {
            r["additional_effects"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
                && family::assignment_only(r, &catalog) == Some(true)
        })
        .unwrap();
    let mut control = record.clone();
    let primary = control["primary_effect"].clone();
    let additional = control["effects"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|e| e["id"] != primary)
        .unwrap();
    additional["support"] = json!(true);
    assert_eq!(
        family::assignment_only(&control, &catalog),
        None,
        "classification must match authenticated identities"
    );
    for pointer in [
        "/effects/0/support",
        "/primary_support",
        "/additional_effects",
        "/declared_references",
        "/selector_resolves_same",
    ] {
        let mut broken = record.clone();
        *broken.pointer_mut(pointer).unwrap() = Value::Null;
        assert_eq!(
            family::assignment_only(&broken, &catalog),
            None,
            "{pointer}"
        );
    }
    for field in ["resolves_as_effect", "present_in_constructed_effect_list"] {
        let mut broken = record.clone();
        broken["declared_references"][0][field] = json!(false);
        assert_eq!(family::assignment_only(&broken, &catalog), None);
    }
    let mut duplicated = record.clone();
    let extra = duplicated["effects"][0].clone();
    duplicated["effects"].as_array_mut().unwrap().push(extra);
    assert_eq!(family::assignment_only(&duplicated, &catalog), None);
}

#[test]
fn stat_set_metadata_is_not_an_additional_effect_origin() {
    let catalog = family::identities();
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    let ice = records
        .iter()
        .find(|r| r["primary_effect"] == "IceNovaPlayer")
        .unwrap();
    assert_eq!(ice["declared_references"].as_array().unwrap().len(), 2);
    assert_eq!(family::assignment_only(ice, &catalog), Some(true));

    // The distinction applies across the catalog, not only to the selected build.
    let mut stat_set_gems = 0;
    for record in &records {
        let gem = catalog
            .gem_by_key(record["gem_id"].as_str().unwrap())
            .unwrap();
        if !gem.declared_additional_stat_sets.is_empty() {
            stat_set_gems += 1;
            assert!(
                family::assignment_only(record, &catalog).is_some(),
                "{}",
                gem.key
            );
        }
    }
    assert_eq!(stat_set_gems, 111);

    for (field, value) in [
        ("field", json!("additionalGrantedEffectId1")),
        ("field", json!("unknownReference1")),
        ("id", json!("unrelated-stat-set")),
        ("resolves_as_effect", json!(true)),
        ("present_in_constructed_effect_list", json!(true)),
    ] {
        let mut broken = ice.clone();
        broken["declared_references"][0][field] = value;
        assert_eq!(family::assignment_only(&broken, &catalog), None);
    }
    let mut incomplete = ice.clone();
    incomplete["declared_references"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(family::assignment_only(&incomplete, &catalog), None);
    let mut duplicate = ice.clone();
    duplicate["declared_references"][1] = duplicate["declared_references"][0].clone();
    assert_eq!(family::assignment_only(&duplicate, &catalog), None);
}

#[test]
fn absent_additional_effects_do_not_inherit_stat_set_treatment() {
    let catalog = family::identities();
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    let mut unresolved = 0;
    for record in records {
        let gem = catalog
            .gem_by_key(record["gem_id"].as_str().unwrap())
            .unwrap();
        if gem
            .constructed_additional_effects
            .iter()
            .any(|r| catalog.skill_by_id(&r.id).is_none())
        {
            unresolved += 1;
            assert_eq!(family::assignment_only(&record, &catalog), None);
            let mut omitted = record;
            omitted["declared_references"] = json!([]);
            assert_eq!(family::assignment_only(&omitted, &catalog), None);
        }
    }
    assert_eq!(unresolved, 8);
}

#[test]
fn display_order_does_not_supply_support_capability_or_drop_additional_effects() {
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    let catalog = family::identities();
    for mut record in records {
        let expected = family::assignment_only(&record, &catalog);
        record["effects"].as_array_mut().unwrap().reverse();
        assert_eq!(family::assignment_only(&record, &catalog), expected);
    }
}

#[test]
#[ignore = "requires GEM_SUPPORT_DOMAINS_PRIOR and fresh GEM_SUPPORT_DOMAINS_OUTPUT"]
fn publish_gem_domains_preserving_all_five_originals() {
    publication::run_with_expected_selected_counts(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_PRIOR")
                .expect("checked predecessor"),
        ),
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GEM_SUPPORT_DOMAINS_OUTPUT")
                .expect("fresh output"),
        ),
        &family::data(),
        &[],
        &["authoring.json"],
        family::stage,
        json!({"gem_source_domains":966,"known_gem_source_domains":929,"unmapped_gem_source_domains":37,
            "unresolved_gem_effect_construction":8,
            "whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 4],
    );
}
