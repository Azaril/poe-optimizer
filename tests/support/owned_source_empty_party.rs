//! Full pinned source authority for the childless fresh Party presentation frame.
#[path = "owned_source_presentation_evidence.rs"]
pub(super) mod evidence;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/source-empty-party")
}
pub fn facts() -> Value {
    serde_json::from_slice(&fs::read(data().join("source-facts.json")).unwrap()).unwrap()
}
pub fn authenticate(facts: &Value) {
    assert_eq!(
        facts["destinations"],
        json!([
            "All",
            "Party Member Stats",
            "Aura",
            "Curse",
            "Warcry Skills",
            "Link Skills",
            "EnemyConditions",
            "EnemyMods"
        ])
    );
    assert_eq!(
        facts["defaults"],
        json!({"destination":"All","append":false,"ShowAdvanceTools":false})
    );
    assert_eq!(
        facts["scope"],
        json!({"unique_ordinary_root_leaf":true,"child_records":false,"native_inventory_completion":false,"party_payload_delivery":false,"import_exportParty":false,"future_explicit_user_actions":false})
    );
    let spans = [
        (
            "src/Classes/PartyTab.lua",
            16,
            514,
            "fresh_constructor_and_explicit_import_controls",
        ),
        ("src/Classes/PartyTab.lua", 516, 573, "load"),
        ("src/Classes/PartyTab.lua", 575, 716, "save_and_display"),
        (
            "src/Classes/DropDownControl.lua",
            20,
            65,
            "dropdown_constructor",
        ),
        (
            "src/Classes/DropDownControl.lua",
            142,
            157,
            "selection_without_callback",
        ),
        (
            "src/Modules/Build.lua",
            34,
            635,
            "fresh_build_construction_and_saver_load",
        ),
    ];
    evidence::authenticate(
        &data(),
        facts,
        &[
            ("src/Classes/PartyTab.lua", "evidence/PartyTab.lua.gz"),
            (
                "src/Classes/DropDownControl.lua",
                "evidence/DropDownControl.lua.gz",
            ),
            ("src/Modules/Build.lua", "evidence/Build.lua.gz"),
        ],
        &spans,
    );
}
