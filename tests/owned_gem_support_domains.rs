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
    let record = records
        .iter()
        .find(|r| {
            r["additional_effects"]
                .as_array()
                .is_some_and(|v| !v.is_empty())
                && family::assignment_only(r) == Some(true)
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
    assert_eq!(family::assignment_only(&control), Some(false));
    for pointer in [
        "/effects/0/support",
        "/primary_support",
        "/additional_effects",
        "/declared_references",
        "/selector_resolves_same",
    ] {
        let mut broken = record.clone();
        *broken.pointer_mut(pointer).unwrap() = Value::Null;
        assert_eq!(family::assignment_only(&broken), None, "{pointer}");
    }
    for field in ["resolves_as_effect", "present_in_constructed_effect_list"] {
        let mut broken = record.clone();
        broken["declared_references"][0][field] = json!(false);
        assert_eq!(family::assignment_only(&broken), None);
    }
    let mut duplicated = record.clone();
    let extra = duplicated["effects"][0].clone();
    duplicated["effects"].as_array_mut().unwrap().push(extra);
    assert_eq!(family::assignment_only(&duplicated), None);
}

#[test]
fn display_order_does_not_supply_support_capability_or_drop_additional_effects() {
    let records: Vec<Value> = catalog_evidence::read("source-records.json");
    for mut record in records {
        let expected = family::assignment_only(&record);
        record["effects"].as_array_mut().unwrap().reverse();
        assert_eq!(family::assignment_only(&record), expected);
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
        json!({"gem_source_domains":966,"known_gem_source_domains":818,"unmapped_gem_source_domains":148,
            "unresolved_gem_effect_construction":119,
            "whole_build_parity":false,"expected_retired_item_text_diagnostics":0,"expected_retired_selected_input_issues":0}),
        [107, 117, 109, 123, 4],
    );
}
