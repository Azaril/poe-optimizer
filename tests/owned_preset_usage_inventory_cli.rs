#[path = "support/owned_preset_usage_inventory_expansion.rs"]
mod expansion;
#[path = "support/owned_preset_usage_inventory.rs"]
mod family;
#[path = "support/owned_preset_usage_inventory_positive.rs"]
mod positive;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use family::{checked_output, checked_predecessor, publish, root, stage, write};
use serde_json::json;
use std::{fs, path::PathBuf};
#[test]
fn checked_packet_preserves_the_complete_saved_source_census() {
    family::check();
}

#[test]
fn companion_evidence_cannot_drop_or_relabel_source_construction() {
    family::check();
    let original = family::read(family::data().join("companion-additions.json"));
    for field in [
        "missing_references",
        "declared_additional_effects",
        "constructed_additional_effects",
        "resolved_additional_effects",
        "additional_stat_sets",
    ] {
        let mut changed = original.clone();
        changed["additions"][0]["role"]["construction"]["value"][field] =
            json!(["unreviewed source reference"]);
        assert!(
            std::panic::catch_unwind(|| family::check_companion_evidence(&changed)).is_err(),
            "{field}"
        );
    }
    let mut changed = original.clone();
    changed["additions"][0]["companion"]["skill_id"] = json!("IceNovaPlayer");
    assert!(std::panic::catch_unwind(|| family::check_companion_evidence(&changed)).is_err());
    let mut changed = original.clone();
    changed["additions"][0]["role"]["role"]["value"] = json!("skill_use");
    assert!(std::panic::catch_unwind(|| family::check_companion_evidence(&changed)).is_err());
}

#[test]
#[ignore = "explicit immutable predecessor/output paths; all five originals and independent repeats"]
fn publish_bounded_usage_inventory_with_all_five_unchanged() {
    let prior_path =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_USAGE_INVENTORY_PRIOR").unwrap());
    let out = PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_USAGE_INVENTORY_OUTPUT").unwrap());
    assert!(!out.exists());
    let _authoring = checked_predecessor(&prior_path);
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    let mut cases = Vec::new();
    for case in 1..=5 {
        let source = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&source).unwrap();
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &source, case, &old);
        release::normalize(&package, &source, case, &new);
        let repeated = out.join(format!("repeated-original-{case:02}"));
        release::normalize(&package, &source, case, &repeated);
        assert_eq!(
            checked_output(&new, &package, case, &next, &xml),
            checked_output(&repeated, &package, case, &next, &xml),
            "Original{case:02}: independent fresh successor imports preserve all content"
        );
        assert_eq!(
            checked_output(&old, &prior_path, case, &prior, &xml),
            checked_output(&new, &package, case, &next, &xml),
            "Original{case:02}: exact complete content, no field or issue dropping"
        );
        let old_check = selected::finalize_with_definitions(
            &xml,
            &old,
            &out.join(format!("prior-selection-{case:02}.json")),
            &prior_path.join("schema.json"),
        );
        let new_check = selected::finalize_with_definitions(
            &xml,
            &new,
            &out.join(format!("selection-{case:02}.json")),
            &package.join("schema.json"),
        );
        let mut oi = old_check["finalization"]["issues"].clone();
        let mut ni = new_check["finalization"]["issues"].clone();
        selected::canonical(&mut oi);
        selected::canonical(&mut ni);
        assert_eq!(oi, ni);
        assert_eq!(
            ni.as_array().unwrap().len(),
            [107, 118, 110, 124, 5][case - 1]
        );
        assert_eq!(fs::read(&source).unwrap(), xml);
        write(out.join(format!("report-{case:02}.json")), &new_check);
        cases.push(json!({"original":case,"selected_pending_issues":ni.as_array().unwrap().len(),"whole_draft_inverse":true,"fresh_repeat_identical":true,"retired_issues":0}));
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":cases,"queries":110,"rebuild_byte_identical":true,"fresh_repeat_identical":true,"complete_original_builds":0,"calculation":"not_run","source_builds_unchanged":true}),
    );
}

#[test]
#[ignore = "published prior/successor packages; exact real-group positive and nonneutral refusal"]
fn real_ice_nova_group_completes_only_its_usage_inventory() {
    positive::run();
}

#[test]
#[ignore = "published expanded package; source-byte accounting frame and nine omission controls"]
fn real_support_expansion_needs_every_added_family() {
    expansion::run();
}
