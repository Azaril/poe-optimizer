//! Optional pinned original parser/list/ordered-overlay oracle. No native profile
//! or handwritten attribution algorithm supplies reference results.
#![cfg(not(target_arch = "wasm32"))]
// Reuse only the bootstrap from the broader component oracle helper.
#[path = "support/owned_item_range_reference.rs"]
mod owned_item_range_reference;
#[allow(dead_code)]
#[path = "support/owned_item_reference.rs"]
mod owned_item_reference;
use owned_item_range_reference::*;
use owned_item_reference::{PINS, REVISION, sha256};
use roxmltree::Document;
use serde_json::json;
const ORIGINAL_02: &str =
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml");
const ORIGINAL_05: &str =
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
const HASH_02: &str = "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631";
const HASH_05: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
fn item_xml<'a>(xml: &'a str, id: &str) -> &'a str {
    let doc = Document::parse(xml).unwrap();
    let item = doc
        .descendants()
        .find(|node| node.has_tag_name("Item") && node.attribute("id") == Some(id))
        .unwrap();
    &xml[item.range()]
}
#[test]
#[ignore = "optional pinned original ParseRaw/ItemsTab oracle; requires vendor source"]
fn original_staff_range_targets_and_ordered_zero_duplicate_writes_use_source_functions() {
    assert_eq!(sha256(ORIGINAL_05.as_bytes()), HASH_05);
    let oracle = RangeOracle::new();
    let raw = item_xml(ORIGINAL_05, "28");
    let mut ledger = vec![];
    for (writes, expected) in [
        (vec![], 11),
        (vec!["0"], 1),
        (vec!["1"], 20),
        (vec!["0", "1", "0.5"], 11),
        (vec!["1", "0"], 1),
    ] {
        let node = oracle.xml_item(raw);
        for range in &writes {
            oracle.append_write(&node, Some("1"), Some(range));
        }
        let (item, events) = oracle.load(&node).unwrap();
        let result = oracle.snapshot(&item);
        assert_eq!(result.item_level, None);
        assert_eq!(result.lists["buffModLines"].len(), 0);
        assert_eq!(result.lists["enchantModLines"].len(), 0);
        assert_eq!(result.lists["runeModLines"].len(), 0);
        assert_eq!(result.lists["implicitModLines"].len(), 1);
        assert_eq!(result.lists["explicitModLines"].len(), 1);
        assert_eq!(
            result.lists["implicitModLines"][0].text,
            "Grants Skill: Level (1-20) Firebolt"
        );
        assert_eq!(
            result.lists["explicitModLines"][0].text,
            "128% increased Spell Damage"
        );
        assert_eq!(result.grants, vec![("FireboltPlayer".into(), expected)]);
        let explicit = &result.lists["explicitModLines"][0];
        assert!(
            explicit
                .rows
                .iter()
                .any(|row| row.name == "Damage" && row.numeric_value == Some(128.0))
        );
        assert_eq!(events.len(), 3 + writes.len());
        for (index, range) in writes.iter().enumerate() {
            assert_eq!(
                events[3 + index].lists["implicitModLines"][0].range,
                Some(range.parse::<f64>().unwrap())
            );
        }
        ledger.push(json!({"appended_id1_writes":writes,"source_events":events,"final":result}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"case":"original05-actual-parse-and-overlay","upstream_revision":REVISION,"source_pins_lf_sha256":PINS,"extra_pins_lf_sha256":EXTRA_PINS,"raw_xml_sha256":HASH_05,"vectors":ledger,"coverage":"actual Item ParseRaw and exact ItemsTab child loop, followed by original BuildModList; no owned-provider resolution or full-build parity"})).unwrap());
}
#[test]
#[ignore = "optional pinned original ParseRaw/ItemsTab oracle; requires vendor source"]
fn original_spear_source_lists_exclude_runes_from_overlay_indices_and_preserve_speed() {
    assert_eq!(sha256(ORIGINAL_02.as_bytes()), HASH_02);
    let oracle = RangeOracle::new();
    let node = oracle.xml_item(item_xml(ORIGINAL_02, "26"));
    let (item, events) = oracle.load(&node).unwrap();
    let result = oracle.snapshot(&item);
    assert_eq!(
        events.len(),
        8,
        "one ParseRaw text and seven ordered overlays"
    );
    assert_eq!(result.item_level, Some(81));
    assert!(!result.lists["runeModLines"].is_empty());
    assert!(
        result.lists["runeModLines"]
            .iter()
            .any(|line| line.bonded == Some(true))
    );
    let speed = result.lists["explicitModLines"]
        .iter()
        .find(|line| line.text == "49% increased Attack Speed")
        .unwrap();
    assert!(
        speed
            .rows
            .iter()
            .any(|row| row.name == "Speed" && row.numeric_value == Some(49.0))
    );
    let node = oracle.xml_item(item_xml(ORIGINAL_02, "26"));
    oracle.append_write(&node, Some("1"), Some("0"));
    let (changed, _) = oracle.load(&node).unwrap();
    let changed = oracle.snapshot(&changed);
    assert_eq!(result.lists["runeModLines"], changed.lists["runeModLines"]);
    assert_ne!(
        result.lists["implicitModLines"],
        changed.lists["implicitModLines"]
    );
    println!("{}",serde_json::to_string_pretty(&json!({"case":"original02-source-rune-list-contrast","upstream_revision":REVISION,"raw_xml_sha256":HASH_02,"source_events":events,"final":result,"after_id1_zero":changed,"coverage":"reference rune reconstruction/list attribution only; production adapter may remain partial; explicit affix metadata does not replace speed49"})).unwrap());
}

#[path = "../../poe-optimizer-import/tests/support/item_range_vectors.rs"]
mod item_range_vectors;
#[test]
#[ignore = "optional pinned Common/ItemTools arithmetic oracle; requires vendor source"]
fn original_symmetric_half_offset_and_crossing_zero_range_preserve_ieee_order() {
    use mlua::{Function, Table};
    for warm in [false, true] {
        let oracle = owned_item_reference::ItemOracle::new(warm);
        let round: Function = oracle.lua.globals().get("roundSymmetric").unwrap();
        for &(value, expected) in item_range_vectors::SYMMETRIC_HALF_OFFSET {
            for _ in 0..if warm { 200 } else { 1 } {
                let actual: f64 = round.call(value).unwrap();
                assert_eq!(
                    actual.to_bits(),
                    expected.to_bits(),
                    "roundSymmetric({value:?}), warm={warm}: {actual:?} != {expected:?}"
                );
            }
        }
        let format: Function = oracle
            .lua
            .globals()
            .get::<Table>("itemLib")
            .unwrap()
            .get("formatValue")
            .unwrap();
        for (value, expected) in [
            (-2.5, -3.0),
            (-0.5, -1.0),
            (0.5, 1.0),
            (2.5, 3.0),
            (f64::from_bits(0x3fdfffffffffffff), 1.0),
        ] {
            let actual: String = format.call((value, 1.0, 1.0, 1.0)).unwrap();
            assert_eq!(actual.parse::<f64>().unwrap(), expected);
        }
        // The actual applyRange source evaluates -3 + .7 * (2 - -3) = .5.
        // Convex interpolation would produce .4999999999999998 and round to0.
        assert_eq!(
            oracle.range("(-3-2)% to Fire Resistance", 0.7),
            "1% to Fire Resistance"
        );
    }
}
