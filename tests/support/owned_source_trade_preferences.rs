//! Reviewed source-only boundary for saved trade-tool weighting preferences.
use super::empty_party::evidence;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/source-trade-preferences")
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
                "src/Classes/ItemsTab.lua",
                "../mana-supplier-sources/equipment-ItemsTab.lua.gz",
            ),
            ("src/Classes/TradeQuery.lua", "evidence/TradeQuery.lua.gz"),
            (
                "src/Classes/TradeQueryGenerator.lua",
                "evidence/TradeQueryGenerator.lua.gz",
            ),
            (
                "src/Classes/ButtonControl.lua",
                "evidence/ButtonControl.lua.gz",
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
                "src/Classes/ItemsTab.lua",
                140,
                188,
                "fresh_trade_object_and_explicit_price_button",
            ),
            (
                "src/Classes/ItemsTab.lua",
                1193,
                1320,
                "complete_items_load",
            ),
            (
                "src/Classes/ItemsTab.lua",
                1322,
                1411,
                "complete_items_save",
            ),
            (
                "src/Classes/TradeQuery.lua",
                30,
                71,
                "fresh_trade_constructor",
            ),
            (
                "src/Classes/TradeQuery.lua",
                276,
                696,
                "explicit_trade_pane",
            ),
            ("src/Classes/TradeQuery.lua", 698, 801, "weight_editor"),
            (
                "src/Classes/TradeQuery.lua",
                814,
                932,
                "trade_result_reduction_and_comparison",
            ),
            (
                "src/Classes/TradeQuery.lua",
                1103,
                1484,
                "explicit_search_controls",
            ),
            (
                "src/Classes/TradeQueryGenerator.lua",
                118,
                143,
                "generator_constructor",
            ),
            (
                "src/Classes/TradeQueryGenerator.lua",
                164,
                200,
                "weighted_result_comparison",
            ),
            (
                "src/Classes/TradeQueryGenerator.lua",
                1144,
                1530,
                "explicit_request_controls",
            ),
            (
                "src/Classes/ButtonControl.lua",
                9,
                23,
                "button_constructor_and_click",
            ),
            (
                "src/Classes/ButtonControl.lua",
                107,
                130,
                "button_input_dispatch",
            ),
        ],
    );
    assert_eq!(
        facts["scope"],
        json!({
            "fresh_load_only":true,"unique_ordinary_items_frame":true,
            "complete_weight_frame_required":true,"source_preferences_preserved":true,
            "native_gameplay_inputs":false,"native_objective_inference":false,
            "native_inventory_completion":false,"future_trade_actions_implemented":false
        })
    );
    assert_eq!(
        facts["review"],
        json!({
            "load_destination":"tradeQuery.statSortSelectionList",
            "saved_fields":["label","stat","weightMult"],
            "source_normalization":"known power-stat names refresh label/transform; unmatched names are not inserted",
            "entry_point":"ItemsTab priceDisplayItem onClick -> TradeQuery.PriceItem",
            "consumer":"explicit trade result comparison and query weighting",
            "native_policy":"source-only metadata; no objective or metric mapping",
            "duplicate_stat_choices":"preserve every source occurrence",
            "unsupported_frames":"retain fallback obligations"
        })
    );
    // Assertions document the manually reviewed paths in the complete pinned
    // files. They do not infer source safety from a literal-name scan.
    let items = &sources["src/Classes/ItemsTab.lua"];
    assert!(items.contains("self.tradeQuery.statSortSelectionList = { }"));
    assert!(items.contains("t_insert(self.tradeQuery.statSortSelectionList, statSort)"));
    assert!(items.contains("self.tradeQuery:PriceItem()"));
    let query = &sources["src/Classes/TradeQuery.lua"];
    assert!(query.contains(
        "self.tradeQueryGenerator = new(\"TradeQueryGenerator\"):TradeQueryGenerator(self)"
    ));
    assert!(query.contains("WeightedRatioOutputs(baseOutput, output, self.statSortSelectionList)"));
    let button = &sources["src/Classes/ButtonControl.lua"];
    assert!(button.contains("self.onClick = onClick"));
    assert!(button.contains("function ButtonClass:Click()"));
    let raw = fs::read(data().join("source-facts.json")).unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(&raw)),
        "78a392fb7e5f4deae3628df489e3be984ae6a3ce14480ad19c82b2116748b7df"
    );
    assert_eq!(*facts, serde_json::from_slice::<Value>(&raw).unwrap());
}
