//! Pinned source observations for the native zero-member metadata contract.
//! This does not certify native final values, editor replay, or whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use mlua::{Function, Table, Value};
use serde_json::json;
use sha2::{Digest, Sha256};

const MEMBER: &str = "17% increased Physical Damage";

fn raw(headers: &str) -> String {
    format!(
        "Rarity: RARE\nMetadata Probe\nCrude Bow\n{headers}\nItem Level: 80\nImplicits: 0\n{MEMBER}"
    )
}

fn fresh(oracle: &runtime::Oracle, raw: &str, high_quality: bool) -> Table {
    assert!(raw.len() < 4096);
    let item = oracle
        .lua
        .globals()
        .get::<Function>("new")
        .unwrap()
        .call::<Table>("Item")
        .unwrap();
    // Match the saved XML boundary's direct ParseRaw call; do not run ItemsTab
    // or the constructor's additional sanitiseText transform on the fixture.
    item.get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((item.clone(), raw, Value::Nil, high_quality))
        .unwrap();
    assert_eq!(item.get::<String>("baseName").unwrap(), "Crude Bow");
    item
}

fn assert_one_plain_member(item: &Table, baseline_range: Option<f64>) {
    for name in [
        "classRequirementModLines",
        "buffModLines",
        "enchantModLines",
        "runeModLines",
        "implicitModLines",
    ] {
        assert_eq!(item.get::<Table>(name).unwrap().raw_len(), 0, "{name}");
    }
    let explicit: Table = item.get("explicitModLines").unwrap();
    assert_eq!(explicit.raw_len(), 1);
    let line: Table = explicit.raw_get(1).unwrap();
    assert_eq!(line.get::<String>("line").unwrap(), MEMBER);
    assert_eq!(line.get::<Option<f64>>("range").unwrap(), baseline_range);
    assert_eq!(line.get::<Table>("modTags").unwrap().raw_len(), 0);
    for flag in ["crafted", "rune", "fractured", "desecrated", "unscalable"] {
        assert!(!line.get::<Option<bool>>(flag).unwrap().unwrap_or(false));
    }
    assert!(!item.get::<bool>("advancedCopy").unwrap());
}

fn quality(item: &Table) -> f64 {
    item.get("quality").unwrap()
}

fn normalize_quality(item: &Table) {
    item.get::<Function>("NormaliseQuality")
        .unwrap()
        .call::<()>(item.clone())
        .unwrap();
}

#[test]
fn complete_pinned_parser_distinguishes_metadata_membership_from_source_lifecycle() {
    let oracle = runtime::Oracle::new();
    let baseline = fresh(&oracle, &raw(""), false);
    let baseline_line: Table = baseline
        .get::<Table>("explicitModLines")
        .unwrap()
        .raw_get(1)
        .unwrap();
    let baseline_range = baseline_line.get::<Option<f64>>("range").unwrap();
    // Item.lua:1327 assigns the source affix default even to a fixed line.
    // Keep this actual completed-parser state as the contrast, rather than
    // assuming that no authored range means no source range field.
    let default_range: f64 = oracle
        .lua
        .globals()
        .get::<Table>("main")
        .unwrap()
        .get("defaultItemAffixQuality")
        .unwrap();
    assert_eq!(baseline_range, Some(default_range));
    assert_ne!(baseline_range, Some(0.125));
    assert_one_plain_member(&baseline, baseline_range);
    let mut cases = 0;
    for (headers, expected) in [
        ("Unique ID: opaque", "opaque"),
        ("Unique ID: first\nUnique ID: last", "last"),
        ("Unique ID: same\nUnique ID: same", "same"),
        ("Unique ID: {range:0.125}", "{range:0.125}"),
        (
            "Unique ID: {tags:physical,damage}",
            "{tags:physical,damage}",
        ),
        (
            "Unique ID: {rune}{crafted}{fractured}",
            "{rune}{crafted}{fractured}",
        ),
        ("Unique ID: {unknown:opaque}", "{unknown:opaque}"),
        ("Unique ID: {variant:unclosed", "{variant:unclosed"),
        ("Unique ID: a:b", "a:b"),
    ] {
        let item = fresh(&oracle, &raw(headers), false);
        assert_eq!(item.get::<String>("uniqueID").unwrap(), expected);
        assert_one_plain_member(&item, baseline_range);
        assert_eq!(quality(&item), 0.0);
        assert!(item.get::<Option<f64>>("catalyst").unwrap().is_none());
        assert!(
            item.get::<Option<f64>>("catalystQuality")
                .unwrap()
                .is_none()
        );
        cases += 1;
    }

    // Source allows later metadata; native v5 deliberately admits preamble only.
    let item = fresh(&oracle, &format!("{}\nUnique ID: after", raw("")), false);
    assert_eq!(item.get::<String>("uniqueID").unwrap(), "after");
    assert_one_plain_member(&item, baseline_range);
    cases += 1;

    // Use the actual source syntax function, not a Rust recreation of its pattern.
    let syntax: Table = oracle.lua.globals().get("itemSyntax").unwrap();
    let parse_spec: Function = syntax.get("parseItemSpec").unwrap();
    for text in ["Unique ID:", "Unique ID: ", "Unique ID:value"] {
        let (name, _): (Option<String>, Option<String>) = parse_spec.call(text).unwrap();
        assert!(name.is_none(), "{text}");
        cases += 1;
    }

    let source_default: f64 = oracle
        .lua
        .globals()
        .get::<Table>("main")
        .unwrap()
        .get("defaultItemQuality")
        .unwrap();
    assert_eq!(source_default, 20.0);
    for quality_header in ["", "Quality: 0", "Quality: 10"] {
        let expected_fresh = if quality_header == "Quality: 10" {
            10.0
        } else {
            0.0
        };
        let without = fresh(&oracle, &raw(quality_header), false);
        let with = fresh(
            &oracle,
            &raw(&format!("Unique ID: imported\n{quality_header}")),
            false,
        );
        assert_eq!(quality(&without), expected_fresh);
        assert_eq!(quality(&with), expected_fresh);
        normalize_quality(&without);
        normalize_quality(&with);
        assert_eq!(quality(&without), source_default);
        assert_eq!(quality(&with), expected_fresh);
        cases += 2;
    }
    assert_eq!(quality(&fresh(&oracle, &raw(""), true)), source_default);
    assert_eq!(
        quality(&fresh(&oracle, &raw("Unique ID: imported"), true)),
        0.0
    );
    cases += 2;

    let reused = fresh(&oracle, &raw("Unique ID: retained"), false);
    reused
        .get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((reused.clone(), raw("")))
        .unwrap();
    assert_eq!(reused.get::<String>("uniqueID").unwrap(), "retained");
    normalize_quality(&reused);
    assert_eq!(quality(&reused), 0.0);
    assert!(
        fresh(&oracle, &raw(""), false)
            .get::<Option<String>>("uniqueID")
            .unwrap()
            .is_none()
    );
    cases += 1;

    // Selection controls are pre-scanned before the Unique ID early continue.
    // Markup normalization is also original source, including the obfuscated case.
    for (value, expected_uid, version) in [
        (
            "{variant:1}{group:1}opaque",
            "{variant:1}{group:1}opaque",
            None,
        ),
        (
            "{variant:1}{group:1}{version:1}opaque",
            "{variant:1}{group:1}{version:1}opaque",
            Some(1),
        ),
        (
            "{[variant]:1}{[group]:1}opaque",
            "{variant:1}{group:1}opaque",
            None,
        ),
    ] {
        let versions = if version.is_some() {
            "Version: Alpha\n"
        } else {
            ""
        };
        let item = fresh(
            &oracle,
            &raw(&format!("{versions}Variant: One\nUnique ID: {value}")),
            false,
        );
        assert_eq!(item.get::<String>("uniqueID").unwrap(), expected_uid);
        let groups: Table = item.get("variantGroups").unwrap();
        let group: Table = groups.raw_get(1).unwrap();
        let eligible: Table = group.raw_get(1).unwrap();
        assert!(eligible.raw_get::<bool>(version.unwrap_or(0)).unwrap());
        assert_eq!(
            item.get::<Table>("variantGroupSelections")
                .unwrap()
                .raw_get::<i64>(1)
                .unwrap(),
            1
        );
        assert_one_plain_member(&item, baseline_range);
        cases += 1;
    }

    let unique = raw("Unique ID: opaque").replacen("Rarity: RARE", "Rarity: UNIQUE", 1);
    assert_eq!(
        fresh(&oracle, &unique, false)
            .get::<String>("rarity")
            .unwrap(),
        "UNIQUE"
    );
    let relic = unique.replacen("Unique ID: opaque", "Unique ID: Foil Unique", 1);
    let relic = fresh(&oracle, &relic, false);
    assert_eq!(relic.get::<String>("rarity").unwrap(), "RELIC");
    assert_one_plain_member(&relic, baseline_range);
    cases += 2;

    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/Common.lua",
        "src/Modules/Main.lua",
        "src/Data/Bases/bow.lua",
    ]
    .into_iter()
    .map(|path| {
        let source = runtime::verified(path).unwrap();
        if path == "src/Classes/ItemsTab.lua" {
            assert_eq!(source.matches("item:ParseRaw(child)").count(), 1);
        }
        json!({"path":path,"sha256_lf":format!("{:x}",Sha256::digest(source.as_bytes()))})
    })
    .collect();
    assert_eq!(cases, 27);
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"complete pinned Item ParseRaw and NormaliseQuality; source-only metadata membership and lifecycle observations; no UI replay or final-build parity",
        "cases":cases,"source_pins":pins,
        "fresh_xml_quality":"absent quality becomes zero with or without Unique ID; later raising and highQuality differ",
        "control_exceptions":["variant/version/group pre-scan", "GGG markup normalization", "Foil Unique rarity pre-scan"],
        "native_admission":"separately tested; stricter position/control rejection remains intentional"
    })).unwrap());
}
