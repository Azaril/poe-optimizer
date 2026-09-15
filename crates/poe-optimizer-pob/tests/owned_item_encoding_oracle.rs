//! Optional source-only member encoding observations, not native effect closure.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use mlua::{Function, Table};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
const RING_XML: &str = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
const SPEAR_XML: &str =
    include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml");
const TAG: &str = "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn raw_item(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let item = doc
        .descendants()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some("26"))
        .unwrap();
    let texts: Vec<_> = item
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    assert_eq!(texts.len(), 1);
    texts[0].to_owned()
}
fn add_headers(raw: &str, headers: &str) -> String {
    assert_eq!(raw.matches("Implicits: 1").count(), 1);
    raw.replacen("Implicits: 1", &format!("{headers}\nImplicits: 1"), 1)
}
fn cold(item: &Table) -> Vec<f64> {
    let slots: Table = item.get("slotModList").unwrap();
    assert_eq!(slots.raw_len(), 3);
    slots
        .sequence_values::<Table>()
        .map(|slot| {
            let slot = slot.unwrap();
            assert!(slot.raw_len() <= 128);
            slot.sequence_values::<Table>()
                .map(Result::unwrap)
                .filter(|row| row.get::<String>("name").unwrap() == "ColdResist")
                .map(|row| row.get::<f64>("value").unwrap())
                .sum()
        })
        .collect()
}
fn members(item: &Table) -> Json {
    let mut output = vec![];
    for category in [
        "enchantModLines",
        "runeModLines",
        "implicitModLines",
        "explicitModLines",
    ] {
        let list: Table = item.get(category).unwrap();
        assert!(list.raw_len() <= 64);
        for line in list.sequence_values::<Table>().map(Result::unwrap) {
            let tags: Option<Vec<String>> = line
                .get::<Option<Table>>("modTags")
                .unwrap()
                .map(|tags| tags.sequence_values().map(Result::unwrap).collect());
            let mut row = json!({"category":category,"text":line.get::<String>("line").unwrap(),"tags":tags,"range":line.get::<Option<f64>>("range").unwrap(),"corrupted_range":line.get::<Option<f64>>("corruptedRange").unwrap(),"scalar":line.get::<Option<f64>>("valueScalar").unwrap(),"display_scalar":line.get::<Option<f64>>("displayValueScalar").unwrap(),"extra":line.get::<Option<String>>("extra").unwrap(),"augment_type":line.get::<Option<String>>("augmentType").unwrap()});
            for flag in [
                "unscalable",
                "disabled",
                "prefix",
                "suffix",
                "desecrated",
                "rune",
                "enchant",
                "bonded",
                "socketedRuneEffectAlreadyApplied",
            ] {
                row[flag] = json!(line.get::<Option<bool>>(flag).unwrap().unwrap_or(false));
            }
            output.push(row);
        }
    }
    json!(output)
}
fn canonical(item: &Table) -> String {
    let raw: String = item
        .get::<Function>("BuildRaw")
        .unwrap()
        .call(item.clone())
        .unwrap();
    assert!(raw.len() < 32_768);
    raw
}
fn ring_observation(oracle: &runtime::Oracle, input: &str) -> Json {
    assert!(input.len() < 16_384);
    let item = oracle.parse(input);
    let before = cold(&item);
    let rows = members(&item);
    let raw = canonical(&item);
    let after = cold(&oracle.parse(&raw));
    assert_eq!(before, after, "fresh canonical roundtrip: {input}");
    json!({"input_sha256":hash(input.as_bytes()),"members":rows,"canonical_text":raw,"cold_by_source_slot":before,"magnitude_count":item.get::<Table>("modMagnitudeMods").unwrap().raw_len(),"advanced_copy":item.get::<bool>("advancedCopy").unwrap(),"crafted":item.get::<Option<bool>>("crafted").unwrap().unwrap_or(false)})
}

#[test]
#[ignore = "optional complete pinned Item/ModParser oracle; requires vendor submodule"]
fn ordinary_ranges_remain_nominal_while_rune_export_can_bake_effect_scaling() {
    assert_eq!(
        hash(RING_XML.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    assert_eq!(
        hash(SPEAR_XML.as_bytes()),
        "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631"
    );
    let ring = raw_item(RING_XML);
    let spear = raw_item(SPEAR_XML);
    assert_eq!(ring.matches(TAG).count(), 1);
    let oracle = runtime::Oracle::new();
    let catalyst = "Catalyst: Tul's\nCatalystQuality: 20";
    let mut ranges = vec![];
    for (name, headers, flags, transforms, expected, count) in [
        ("original", "", "", "", 25.0, 0),
        ("catalyst", catalyst, "", "", 30.0, 0),
        (
            "corrupted-item-state-only",
            "Catalyst: Tul's\nCatalystQuality: 20\nCorrupted",
            "",
            "",
            30.0,
            0,
        ),
        (
            "corrupted-member-factor",
            "",
            "{corruptedRange:1.5}",
            "",
            38.0,
            0,
        ),
        (
            "corruption-before-catalyst",
            catalyst,
            "{corruptedRange:1.5}",
            "",
            45.0,
            0,
        ),
        (
            "unscalable-catalyst-and-magnitude",
            catalyst,
            "{unscalable}",
            "20% increased implicit modifier magnitudes",
            25.0,
            1,
        ),
        (
            "unscalable-still-corrupted",
            catalyst,
            "{unscalable}{corruptedRange:1.5}",
            "20% increased implicit modifier magnitudes",
            38.0,
            1,
        ),
        (
            "implicit-magnitude",
            catalyst,
            "",
            "20% increased implicit modifier magnitudes",
            35.0,
            1,
        ),
        (
            "wrong-category",
            catalyst,
            "",
            "20% increased explicit modifier magnitudes",
            30.0,
            1,
        ),
        (
            "all-tags-required",
            catalyst,
            "",
            "20% increased cold fire modifier magnitudes",
            30.0,
            1,
        ),
        (
            "addition-then-double",
            catalyst,
            "",
            "20% increased implicit modifier magnitudes\nImplicit modifier magnitudes are doubled",
            70.0,
            2,
        ),
        (
            "double-then-addition",
            catalyst,
            "",
            "Implicit modifier magnitudes are doubled\n20% increased implicit modifier magnitudes",
            65.0,
            2,
        ),
        ("disabled-member", catalyst, "{disabled}", "", 0.0, 0),
        (
            "unknown-sibling-is-not-proof-of-closure",
            catalyst,
            "",
            "Unmapped source observation member",
            30.0,
            0,
        ),
    ] {
        let input = add_headers(&ring, headers).replacen(TAG, &format!("{TAG}{flags}"), 1);
        let input = if transforms.is_empty() {
            input
        } else {
            format!("{input}\n{transforms}")
        };
        let observed = ring_observation(&oracle, &input);
        assert_eq!(
            observed["cold_by_source_slot"],
            json!(vec![expected; 3]),
            "{name}: {observed}"
        );
        assert_eq!(observed["magnitude_count"], count, "{name}: {observed}");
        assert!(
            observed["canonical_text"]
                .as_str()
                .unwrap()
                .contains("+(20-30)% to Cold Resistance"),
            "nominal endpoints: {name}"
        );
        let rows = observed["members"].as_array().unwrap();
        let cold_row = rows
            .iter()
            .find(|r| r["text"] == "+(20-30)% to Cold Resistance")
            .unwrap();
        assert_eq!(cold_row["category"], "implicitModLines");
        assert_eq!(
            cold_row["tags"],
            json!([
                "cold_resistance",
                "elemental_resistance",
                "elemental",
                "cold",
                "resistance"
            ])
        );
        if name == "unknown-sibling-is-not-proof-of-closure" {
            assert!(
                rows.iter()
                    .any(|r| r["extra"].as_str() == Some("Unmapped source observation member"))
            );
        }
        ranges.push(json!({"case":name,"observation":observed}));
    }
    let rounded = ring.replace("{range:0.5}", "{range:0.25}");
    let rounded = ring_observation(&oracle, &add_headers(&rounded, catalyst));
    assert_eq!(rounded["cold_by_source_slot"], json!(vec![27.0; 3])); //22.5Ã¢â€ â€™23, then truncate(23Ãƒâ€”1.2)=27.
    let mut fixed = vec![];
    for (name, crafted, expected) in [
        ("legacy-fixed-no-nominal-proof", false, 30.0),
        ("authored-fixed-with-crafting-gate", true, 36.0),
    ] {
        let text = format!(
            "Rarity: RARE\nEncoding diagnostic\nSapphire Ring\n{}Implicits: 1\n{{tags:cold}}+30% to Cold Resistance\n20% increased implicit modifier magnitudes",
            if crafted { "Crafted: true\n" } else { "" }
        );
        let observed = ring_observation(&oracle, &text);
        assert_eq!(
            observed["cold_by_source_slot"],
            json!(vec![expected; 3]),
            "{name}: {observed}"
        );
        assert_eq!(observed["advanced_copy"], false);
        assert_eq!(observed["crafted"], crafted);
        fixed.push(json!({"case":name,"observation":observed}));
    }
    let mut runes = vec![];
    for (name, extra, display) in [
        ("original02-unmodified", "", 18),
        (
            "socketed-rune-effect-variant",
            "\n100% increased effect of Socketed Runes",
            36,
        ),
    ] {
        let input = format!("{spear}{extra}");
        let item = oracle.parse(&input);
        let before = members(&item);
        let raw = canonical(&item);
        let reparsed = oracle.parse(&raw);
        let after = members(&reparsed);
        assert!(before.as_array().unwrap().iter().any(
            |r| r["category"] == "runeModLines" && r["text"] == "18% increased Physical Damage"
        ));
        assert!(after.as_array().unwrap().iter().any(
            |r| r["category"] == "runeModLines" && r["text"] == "18% increased Physical Damage"
        ));
        assert!(
            raw.lines().any(|line| line.contains("{rune}")
                && line.ends_with(&format!("{display}% increased Physical Damage"))),
            "{name}: {raw}"
        );
        assert!(raw.contains("49% increased Attack Speed"));
        runes.push(json!({"case":name,"input_sha256":hash(input.as_bytes()),"members":before,"canonical_text":raw,"reparsed_members":after,"scope":"known Rune header rebuilds original rune data; exported baked line is not another nominal modifier"}));
    }
    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/Common.lua",
        "src/Data/ModScalability.lua",
        "src/Data/ModRunes.lua",
    ]
    .into_iter()
    .map(|path| json!({"path":path,"sha256_lf":hash(runtime::verified(path).unwrap().as_bytes())}))
    .collect();
    println!("{}",serde_json::to_string_pretty(&json!({"scope":"source-only member encoding and ordered scaling; no native completeness or selected-build parity","source_pins":pins,"ranges":ranges,"rounding_before_magnitude":rounded,"fixed":fixed,"runes":runes})).unwrap());
}
