//! Bounded ordinary physical component; no saved query or complete-build claim.
#[path = "support/owned_enemy_physical_mitigation_native.rs"]
mod armour_native;
#[path = "support/owned_outgoing_critical_native.rs"]
mod critical_native;
#[path = "support/owned_physical_hit_dps.rs"]
mod dps;
#[allow(dead_code)]
#[path = "support/owned_outgoing_critical.rs"]
mod family;
#[path = "support/owned_physical_hit_dps_native.rs"]
mod numerical;
#[allow(dead_code)]
#[path = "support/owned_plan_replay.rs"]
mod replay;
#[path = "support/owned_physical_hit_dps_source.rs"]
mod source;
use serde_json::json;

#[test]
fn physical_dps_source_domain_retains_full_inputs_and_late_factory_boundaries() {
    source::check();
}

#[test]
fn physical_dps_zero_false_inactive_and_nested_multiplicity_sources_refuse() {
    let baseline = source::capture(0);
    for name in [
        "DPS",
        "QuantityMultiplier",
        "OnlyFinalRepeat",
        "CanCreateHazards",
        "HasSeals",
        "GrenadeActivateTwice",
        "RuthlessBlowMaxCount",
    ] {
        let mut changed = baseline.clone();
        changed["state"]["baseline"][0]["synthetic"] = json!({"parent":{"bucket":[{"name":name,"type":"BASE","value":0,
                "tags":[{"type":"Condition","var":"inactive","neg":true}]}]}});
        assert!(
            source::review_capture(&changed, "jit-off").is_err(),
            "{name}"
        );
    }
    for field in source::domain()["forbidden_fields"].as_array().unwrap() {
        let mut changed = baseline.clone();
        changed["state"]["skill_construction"][0]["skill_data"][field.as_str().unwrap()] =
            json!(false);
        assert!(
            source::review_capture(&changed, "jit-off").is_err(),
            "{field}"
        );
    }
}

#[test]
fn physical_dps_final_repeat_missing_containers_and_selected_recipe_changes_refuse() {
    let baseline = source::capture(0);
    let mut changed = baseline.clone();
    changed["state"]["configuration"][0]["input"]["repeatMode"] = json!("FINAL");
    assert!(source::review_capture(&changed, "jit-off").is_err());
    let mut changed = baseline;
    changed["state"].as_object_mut().unwrap().remove("baseline");
    assert!(source::review_capture(&changed, "jit-off").is_err());
    let mut recipes = source::selected();
    recipes["skills"][0]["synthetic"] = json!({"name":"DPS","type":"MORE","value":0});
    assert!(source::review_selected(&recipes).is_err());
}

#[test]
fn physical_dps_late_source_writer_inventory_cannot_be_bypassed() {
    let evidence: serde_json::Value = dps::read("source-evidence.json");
    let pin = evidence["complete_bodies"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["source_path"] == "src/Modules/CalcOffence.lua")
        .unwrap();
    use std::io::Read;
    let compressed = dps::authenticate(&pin["archive"], 128 * 1024);
    let mut text = String::new();
    flate2::read::GzDecoder::new(&compressed[..])
        .read_to_string(&mut text)
        .unwrap();
    source::review_body("src/Modules/CalcOffence.lua", &text).unwrap();
    text.push_str("\nskillModList:NewMod(\"DPS\", \"MORE\", 0, \"synthetic inactive\")\n");
    assert!(source::review_body("src/Modules/CalcOffence.lua", &text).is_err());
}
