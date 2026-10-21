//! Source-only accounting for a complete fresh-load Import preference frame.
use super::empty_party::evidence;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/source-import-preferences")
}
pub fn facts() -> Value {
    serde_json::from_slice(&fs::read(data().join("source-facts.json")).unwrap()).unwrap()
}
pub fn authenticate(facts: &Value) {
    let sources = evidence::authenticate(
        &data(),
        facts,
        &[
            (
                "src/Modules/Build.lua",
                "../source-empty-party/evidence/Build.lua.gz",
            ),
            (
                "src/Classes/ImportTab.lua",
                "../configuration-campaign-progression/ImportTab.lua.gz",
            ),
            (
                "src/Classes/DropDownControl.lua",
                "../source-empty-party/evidence/DropDownControl.lua.gz",
            ),
            ("src/Classes/EditControl.lua", "evidence/EditControl.lua.gz"),
            (
                "src/Modules/CalcPerform.lua",
                "../configuration-distance-inputs/CalcPerform.lua.gz",
            ),
        ],
        &[
            (
                "src/Modules/Build.lua",
                34,
                635,
                "fresh_build_construction_and_saver_load",
            ),
            (
                "src/Classes/ImportTab.lua",
                556,
                574,
                "complete_import_load",
            ),
            (
                "src/Classes/ImportTab.lua",
                576,
                592,
                "complete_import_save",
            ),
            (
                "src/Classes/DropDownControl.lua",
                142,
                156,
                "selection_without_callback",
            ),
            (
                "src/Classes/EditControl.lua",
                100,
                108,
                "text_notification_requires_explicit_argument",
            ),
            (
                "src/Modules/CalcPerform.lua",
                1214,
                1222,
                "party_export_is_a_calculation_input",
            ),
        ],
    );
    assert_eq!(
        facts["scope"],
        json!({
            "fresh_load_only":true,"unique_ordinary_root_import":true,
            "childless_complete_frame_required":true,"source_preferences_preserved":true,
            "native_gameplay_inputs":false,"native_inventory_completion":false,
            "future_import_or_export_actions_implemented":false
        })
    );
    assert_eq!(
        facts["review"],
        json!({
            "saved_fields":["lastRealm","lastLeague","lastAccountHash","lastCharacterHash","importLink","useGeneratedItemText","exportParty"],
            "boolean_fields":["useGeneratedItemText","exportParty"],
            "boolean_domain":["true","false"],
            "export_party_true":"retain fallback obligation",
            "absent_export_party":"fresh Load explicitly sets false",
            "load_callbacks":"SelByValue does not invoke a callback; SetText without notify does not invoke changeFunc",
            "native_policy":"source-only metadata; no build or scenario inputs",
            "unsupported_frames":"retain fallback obligations"
        })
    );
    // The complete pinned bodies carry the evidence. These assertions record
    // the reviewed branch, not a safety inference from a keyword search.
    let import = &sources["src/Classes/ImportTab.lua"];
    assert!(import.contains(
        "self.controls.enablePartyExportBuffs.state = xml.attrib.exportParty == \"true\""
    ));
    assert!(import.contains(
        "self.build.partyTab.enableExportBuffs = self.controls.enablePartyExportBuffs.state"
    ));
    assert!(import.contains("self.controls.accountName:SetText(accountName)"));
    assert!(sources["src/Classes/EditControl.lua"].contains("if notify and self.changeFunc then"));
    assert!(
        sources["src/Modules/CalcPerform.lua"]
            .contains("env.build.partyTab.enableExportBuffs and env.mode ~= \"CALCULATOR\"")
    );
    let raw = fs::read(data().join("source-facts.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        "a040c32d77105afee7452980b8c0fe2f22d96afa4c809ed48f78c4092bdbb9aa"
    );
    assert_eq!(*facts, serde_json::from_slice::<Value>(&raw).unwrap());
}
