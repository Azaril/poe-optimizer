//! Optional source-only catalyst/magnitude observations. No native implementation
//! or full-build parity is certified by this test.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_formatter_runtime.rs"]
mod formatter;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
use formatter::FormatterOracle;
use mlua::{Table, Value};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};

const ORIGINAL: &str = include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml");
const ORIGINAL_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const TAG: &str = "{tags:cold_resistance,elemental_resistance,elemental,cold,resistance}";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn cold(rows: Table) -> f64 {
    assert!(rows.raw_len() <= 64);
    let values: Vec<_> = rows
        .sequence_values::<Table>()
        .map(Result::unwrap)
        .filter(|row| row.get::<String>("name").unwrap() == "ColdResist")
        .map(|row| row.get::<f64>("value").unwrap())
        .collect();
    assert_eq!(values.len(), 1, "one exact ColdResist source row");
    values[0]
}
fn observation(oracle: &FormatterOracle, raw: &str) -> Json {
    assert!(raw.len() < 16_384);
    let item = oracle.source.parse(raw);
    assert!(item.get::<Option<String>>("baseName").unwrap().is_some());
    let lines: Table = item.get("implicitModLines").unwrap();
    assert_eq!(lines.raw_len(), 1);
    let line: Table = lines.raw_get(1).unwrap();
    let tags: Vec<String> = line
        .get::<Table>("modTags")
        .unwrap()
        .sequence_values()
        .map(Result::unwrap)
        .collect();
    let slots: Table = item.get("slotModList").unwrap();
    assert_eq!(slots.raw_len(), 3); // Source ring receiving contexts, not owned row creation.
    let slot_cold: Vec<_> = slots
        .sequence_values::<Table>()
        .map(|row| cold(row.unwrap()))
        .collect();
    assert!(slot_cold.iter().all(|value| *value == slot_cold[0]));
    json!({
        "raw_sha256":hash(raw.as_bytes()),
        "line":line.get::<String>("line").unwrap(),
        "tags":tags,
        "range":line.get::<Option<f64>>("range").unwrap(),
        "scalar":line.get::<Option<f64>>("valueScalar").unwrap(),
        "parsed_cold":cold(line.get("modList").unwrap()),
        "assembled_cold":slot_cold[0],
        "assembled_cold_by_source_slot":slot_cold,
        "catalyst":item.get::<Option<f64>>("catalyst").unwrap(),
        "catalyst_quality":item.get::<Option<f64>>("catalystQuality").unwrap(),
        "ordinary_quality":item.get::<Option<f64>>("quality").unwrap(),
        "magnitude_count":item.get::<Table>("modMagnitudeMods").unwrap().raw_len(),
    })
}
fn insert_headers(raw: &str, headers: &str) -> String {
    assert_eq!(raw.matches("Implicits: 1").count(), 1);
    raw.replacen("Implicits: 1", &format!("{headers}\nImplicits: 1"), 1)
}
fn number(n: Option<f64>) -> Value {
    n.map_or(Value::Nil, Value::Number)
}

#[test]
#[ignore = "optional complete pinned Item/ModParser oracle; requires vendor submodule"]
fn ordinary_catalyst_and_magnitude_stages_preserve_labels_and_distinguish_fixed_history() {
    assert_eq!(hash(ORIGINAL.as_bytes()), ORIGINAL_HASH);
    let doc = roxmltree::Document::parse(ORIGINAL).unwrap();
    let item = doc
        .descendants()
        .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some("26"))
        .unwrap();
    let texts: Vec<_> = item
        .children()
        .filter(|n| n.is_text())
        .filter_map(|n| n.text())
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    assert_eq!(texts.len(), 1);
    let raw = texts[0];
    assert_eq!(raw.matches(TAG).count(), 1);
    assert!(raw.contains("{range:0.5}+(20-30)% to Cold Resistance"));
    let ranges: Vec<_> = item
        .children()
        .filter(|n| n.has_tag_name("ModRange"))
        .map(|n| n.attribute("range").unwrap())
        .collect();
    assert_eq!(ranges, ["0.5", "0.5"]);
    let oracle = FormatterOracle::new();
    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/Common.lua",
        "src/Data/ModScalability.lua",
    ]
    .into_iter()
    .map(|path| {
        let text = runtime::verified(path).unwrap();
        json!({"path":path,"sha256_lf":hash(text.as_bytes())})
    })
    .collect();
    let policy: Table = oracle.source.lua.globals().get("itemPolicy").unwrap();
    let catalysts: Table = policy.get("catalysts").unwrap();
    let tul = catalysts
        .sequence_values::<String>()
        .map(Result::unwrap)
        .position(|name| name == "Tul's")
        .unwrap()
        + 1;
    let mut scalars = vec![];
    for (name, tags, quality, unscalable, expected) in [
        ("cold", vec!["cold"], Some(20.0), false, 1.2),
        ("any-match", vec!["life", "cold"], Some(20.0), false, 1.2),
        (
            "compound-label-is-not-cold",
            vec!["cold_resistance"],
            Some(20.0),
            false,
            1.0,
        ),
        ("case-sensitive", vec!["Cold"], Some(20.0), false, 1.0),
        ("empty-labels", vec![], Some(20.0), false, 1.0),
        ("nil-amount-default", vec!["cold"], None, false, 1.2),
        ("explicit-zero", vec!["cold"], Some(0.0), false, 1.0),
        (
            "scalar-operation-order",
            vec!["cold"],
            Some(0.7),
            false,
            1.0070000000000001,
        ),
        ("unscalable", vec!["cold"], Some(20.0), true, 1.0),
    ] {
        let row = oracle.source.lua.create_table().unwrap();
        row.set(
            "modTags",
            oracle
                .source
                .lua
                .create_sequence_from(tags.clone())
                .unwrap(),
        )
        .unwrap();
        row.set("prefix", true).unwrap();
        row.set("unscalable", unscalable).unwrap();
        let result = oracle.observe(
            "catalyst",
            &[
                Value::Number(tul as f64),
                Value::Table(row),
                number(quality),
            ],
        );
        assert!(result.get::<bool>("ok").unwrap());
        let value: f64 = result.get("value").unwrap();
        assert_eq!(value, expected, "{name}");
        scalars.push(json!({"case":name,"labels":tags,"quality":quality,"value":value}));
    }
    let mut ring = vec![];
    for (name, headers, expected) in [
        ("original-unmodified", "", 25.0),
        ("ordinary-quality-only", "Quality: 20", 25.0),
        ("default-amount", "Catalyst: Tul's", 30.0),
        ("zero", "Catalyst: Tul's\nCatalystQuality: 0", 25.0),
        (
            "fractional-before-truncation",
            "Catalyst: Tul's\nCatalystQuality: 2",
            25.0,
        ),
        ("next-integral", "Catalyst: Tul's\nCatalystQuality: 4", 26.0),
        ("twenty", "Catalyst: Tul's\nCatalystQuality: 20", 30.0),
        (
            "different-catalyst",
            "Catalyst: Xoph's\nCatalystQuality: 20",
            25.0,
        ),
        ("descriptor-alias", "Quality (Cold Modifiers): 30%", 32.0),
    ] {
        let input = if headers.is_empty() {
            raw.to_owned()
        } else {
            insert_headers(raw, headers)
        };
        let observed = observation(&oracle, &input);
        assert_eq!(observed["assembled_cold"], expected, "{name}: {observed}");
        assert_eq!(
            observed["tags"],
            json!([
                "cold_resistance",
                "elemental_resistance",
                "elemental",
                "cold",
                "resistance"
            ])
        );
        ring.push(json!({"case":name,"observation":observed}));
    }
    let mut formatting = vec![];
    for (value, base, scalar, expected) in [
        (25.5, 1.0, 1.2, "31"),
        (-3.0, 1.0, 1.2, "-3"),
        (-3.0, 1.5, 1.2, "-4"),
    ] {
        let result = oracle.observe(
            "value",
            &[
                Value::Number(value),
                Value::Number(base),
                Value::Number(scalar),
                Value::Number(1.0),
            ],
        );
        assert!(result.get::<bool>("ok").unwrap());
        let text: String = result.get("value").unwrap();
        assert_eq!(text, expected);
        formatting.push(
            json!({"value":value,"base_scalar":base,"magnitude_scalar":scalar,"formatted":text}),
        );
    }
    // The source groups (100 + q) / 100. Reassociation can cross the later
    // truncate boundary even though it is algebraically equal over real numbers.
    let tagged = oracle.source.lua.create_table().unwrap();
    tagged
        .set(
            "modTags",
            oracle.source.lua.create_sequence_from(["cold"]).unwrap(),
        )
        .unwrap();
    let source_scalar = oracle
        .observe(
            "catalyst",
            &[
                Value::Number(tul as f64),
                Value::Table(tagged),
                Value::Number(0.7),
            ],
        )
        .get::<f64>("value")
        .unwrap();
    let regrouped = 1.0 + 0.7 / 100.0;
    assert_ne!(source_scalar, regrouped);
    for (case, scalar, expected) in [
        ("source-scalar-grouping", source_scalar, "1007"),
        ("reassociated-diagnostic", regrouped, "1006"),
    ] {
        let result = oracle.observe(
            "value",
            &[
                Value::Number(1000.0),
                Value::Number(1.0),
                Value::Number(scalar),
                Value::Number(1.0),
            ],
        );
        assert!(result.get::<bool>("ok").unwrap());
        let text: String = result.get("value").unwrap();
        assert_eq!(text, expected);
        formatting
            .push(json!({"case":case,"value":1000,"magnitude_scalar":scalar,"formatted":text}));
    }
    let mut ordered = vec![];
    for (name, transforms, scalar, ranged, fixed) in [
        (
            "add-then-double",
            "20% increased implicit modifier magnitudes\nImplicit modifier magnitudes are doubled",
            2.4,
            60.0,
            60.0,
        ),
        (
            "double-then-add",
            "Implicit modifier magnitudes are doubled\n20% increased implicit modifier magnitudes",
            2.2,
            55.0,
            55.0,
        ),
        (
            "return-to-one-from-above",
            "20% increased implicit modifier magnitudes\n20% reduced implicit modifier magnitudes",
            1.0,
            25.0,
            30.0,
        ),
        (
            "return-to-one-from-below",
            "20% reduced implicit modifier magnitudes\n20% increased implicit modifier magnitudes",
            1.0,
            25.0,
            20.0,
        ),
    ] {
        for (form, line, expected) in [
            (
                "ranged",
                "{tags:cold}{range:0.5}+(20-30)% to Cold Resistance",
                ranged,
            ),
            ("fixed", "{tags:cold}+25% to Cold Resistance", fixed),
        ] {
            let input = format!(
                "Rarity: Rare\nSource-only diagnostic\nSapphire Ring\nCrafted: true\nImplicits: 1\n{line}\n{transforms}"
            );
            let observed = observation(&oracle, &input);
            assert_eq!(observed["magnitude_count"], 2);
            assert_eq!(observed["scalar"], scalar, "{name}/{form}: {observed}");
            assert_eq!(
                observed["assembled_cold"], expected,
                "{name}/{form}: {observed}"
            );
            ordered.push(json!({"case":name,"form":form,"observation":observed}));
        }
    }
    println!("{}",serde_json::to_string_pretty(&json!({
        "scope":"source-only component oracle; no native parity or lifecycle adoption",
        "original_xml_sha256":ORIGINAL_HASH,"source_pins":pins,
        "source_stage":"complete original Item constructor/ParseRaw/BuildModList; raw range equals saved ModRange for original05 ring; no selected-build calculations",
        "scalars":scalars,"ring":ring,"formatting":formatting,"ordered_magnitude":ordered,
    })).unwrap());
}
