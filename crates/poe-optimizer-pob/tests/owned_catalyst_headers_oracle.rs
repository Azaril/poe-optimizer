//! Optional pinned source observations; not native header admission or full-build parity.
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
const RING_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const SPEAR_HASH: &str = "91366bd82a9afdd12ae7d8f695508a1b8d99116567010e082a9d31c4c4d4f631";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn raw_item(xml: &str) -> String {
    let doc = roxmltree::Document::parse(xml).unwrap();
    let items: Vec<_> = doc
        .descendants()
        .filter(|n| n.has_tag_name("Item") && n.attribute("id") == Some("26"))
        .collect();
    assert_eq!(items.len(), 1);
    let texts: Vec<_> = items[0]
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
fn headers(raw: &str) -> Vec<&str> {
    raw.lines()
        .filter(|line| {
            line.starts_with("Catalyst:")
                || line.starts_with("CatalystQuality:")
                || line.starts_with("Quality:")
                || line.starts_with("Quality (")
        })
        .collect()
}
fn fields(item: &Table, names: &Table) -> Json {
    let id: Option<usize> = item.get("catalyst").unwrap();
    json!({
        "catalyst":id.map(|id| names.raw_get::<String>(id).unwrap()),
        "catalyst_quality":item.get::<Option<f64>>("catalystQuality").unwrap(),
        "ordinary_quality":item.get::<Option<f64>>("quality").unwrap(),
        "base":item.get::<String>("baseName").unwrap(),
    })
}
fn observe(oracle: &runtime::Oracle, names: &Table, raw: &str) -> Json {
    assert!(raw.len() < 16_384);
    let item = oracle.parse(raw);
    let before = fields(&item, names);
    let canonical: String = item
        .get::<Function>("BuildRaw")
        .unwrap()
        .call(item.clone())
        .unwrap();
    assert!(canonical.len() < 32_768);
    let canonical_headers = headers(&canonical);
    for prefix in ["Catalyst:", "CatalystQuality:", "Quality:"] {
        assert!(
            canonical_headers
                .iter()
                .filter(|line| line.starts_with(prefix))
                .count()
                <= 1
        );
    }
    assert!(
        !canonical_headers
            .iter()
            .any(|line| line.starts_with("Quality ("))
    );
    let after = fields(&oracle.parse(&canonical), names);
    assert_eq!(before, after, "canonical header roundtrip: {raw}");
    json!({"raw_sha256":hash(raw.as_bytes()),"fields":before,"canonical_headers":canonical_headers,"canonical_raw_sha256":hash(canonical.as_bytes())})
}

#[test]
#[ignore = "optional complete pinned Item/ModParser oracle; requires vendor submodule"]
fn canonical_headers_preserve_zero_and_source_tolerance_is_not_absence_authority() {
    assert_eq!(hash(RING_XML.as_bytes()), RING_HASH);
    assert_eq!(hash(SPEAR_XML.as_bytes()), SPEAR_HASH);
    let ring = raw_item(RING_XML);
    let spear = raw_item(SPEAR_XML);
    assert!(ring.contains("Sapphire Ring"));
    assert!(spear.contains("Grand Spear"));
    assert!(headers(&ring).is_empty());
    assert_eq!(headers(&spear), ["Quality: 20"]);
    let oracle = runtime::Oracle::new();
    let names: Table = oracle
        .lua
        .globals()
        .get::<Table>("itemPolicy")
        .unwrap()
        .get("catalysts")
        .unwrap();
    assert_eq!(names.raw_len(), 13);
    let mut cases = vec![];
    for (name, text, catalyst, amount) in [
        ("absent", "", None, None),
        (
            "recognized-omitted-amount",
            "Catalyst: Tul's",
            Some("Tul's"),
            None,
        ),
        ("unknown", "Catalyst: Unknown", None, None),
        ("case-sensitive", "Catalyst: tul's", None, None),
        (
            "known-then-unknown",
            "Catalyst: Tul's\nCatalyst: Unknown",
            Some("Tul's"),
            None,
        ),
        (
            "unknown-then-known",
            "Catalyst: Unknown\nCatalyst: Tul's",
            Some("Tul's"),
            None,
        ),
        (
            "known-then-known",
            "Catalyst: Tul's\nCatalyst: Xoph's",
            Some("Xoph's"),
            None,
        ),
        (
            "equal-duplicates",
            "Catalyst: Tul's\nCatalyst: Tul's",
            Some("Tul's"),
            None,
        ),
        (
            "zero",
            "Catalyst: Tul's\nCatalystQuality: 0",
            Some("Tul's"),
            Some(0.0),
        ),
        (
            "amount-before-kind",
            "CatalystQuality: 20\nCatalyst: Tul's",
            Some("Tul's"),
            Some(20.0),
        ),
        ("orphan-amount", "CatalystQuality: 20", None, Some(20.0)),
        (
            "malformed-amount",
            "Catalyst: Tul's\nCatalystQuality: bad",
            Some("Tul's"),
            None,
        ),
        (
            "later-malformed-clears-amount",
            "Catalyst: Tul's\nCatalystQuality: 20\nCatalystQuality: bad",
            Some("Tul's"),
            None,
        ),
        (
            "later-valid-amount",
            "Catalyst: Tul's\nCatalystQuality: bad\nCatalystQuality: 20",
            Some("Tul's"),
            Some(20.0),
        ),
        ("number-prefix", "CatalystQuality: 20junk", None, Some(20.0)),
        ("percent-suffix", "CatalystQuality: 20%", None, Some(20.0)),
        ("exponent-prefix", "CatalystQuality: 1e2", None, Some(1.0)),
        (
            "repeated-decimal-point",
            "CatalystQuality: 1.2.3",
            None,
            None,
        ),
        ("extra-value-space", "CatalystQuality:  20", None, None),
        (
            "signed-fraction",
            "CatalystQuality: +.5junk",
            None,
            Some(0.5),
        ),
        (
            "descriptor-alias",
            "Quality (Cold Modifiers): 30%",
            Some("Tul's"),
            Some(30.0),
        ),
        (
            "unknown-descriptor",
            "Quality (Unknown Modifiers): 30%",
            None,
            Some(30.0),
        ),
        (
            "unknown-descriptor-retains-kind",
            "Catalyst: Tul's\nQuality (Unknown Modifiers): 30%",
            Some("Tul's"),
            Some(30.0),
        ),
        (
            "descriptor-fraction-captures-last-integer",
            "Quality (Cold Modifiers): 20.5%",
            Some("Tul's"),
            Some(5.0),
        ),
        (
            "descriptor-sign-not-captured",
            "Quality (Cold Modifiers): -20%",
            Some("Tul's"),
            Some(20.0),
        ),
    ] {
        let input = add_headers(&ring, text);
        let result = observe(&oracle, &names, &input);
        assert_eq!(
            result["fields"]["catalyst"],
            json!(catalyst),
            "{name}: {result}"
        );
        assert_eq!(
            result["fields"]["catalyst_quality"],
            json!(amount),
            "{name}: {result}"
        );
        assert_eq!(result["fields"]["ordinary_quality"], Json::Null, "{name}");
        if name == "zero" {
            assert_eq!(
                result["canonical_headers"],
                json!(["Catalyst: Tul's", "CatalystQuality: 0"])
            );
        }
        cases.push(json!({"case":name,"observation":result}));
    }
    let mut quality = vec![];
    for (name, input, expected) in [
        ("ring-absent", ring.clone(), None),
        ("ring-zero", add_headers(&ring, "Quality: 0"), Some(0.0)),
        (
            "ring-explicit",
            add_headers(&ring, "Quality: 20"),
            Some(20.0),
        ),
        ("ring-malformed", add_headers(&ring, "Quality: bad"), None),
        (
            "ring-duplicate-malformed",
            add_headers(&ring, "Quality: 20\nQuality: bad"),
            None,
        ),
        ("spear-original", spear.clone(), Some(20.0)),
        (
            "spear-absent",
            spear.replacen("Quality: 20\n", "", 1),
            Some(0.0),
        ),
        (
            "spear-zero",
            spear.replacen("Quality: 20", "Quality: 0", 1),
            Some(0.0),
        ),
        (
            "spear-malformed",
            spear.replacen("Quality: 20", "Quality: bad", 1),
            Some(0.0),
        ),
    ] {
        let result = observe(&oracle, &names, &input);
        assert_eq!(
            result["fields"]["ordinary_quality"],
            json!(expected),
            "{name}: {result}"
        );
        assert_eq!(result["fields"]["catalyst"], Json::Null);
        quality.push(json!({"case":name,"observation":result}));
    }
    // Malformed source aliases are errors, not a valid absent amount. Call the same
    // source constructor fallibly, rather than reproducing the parser in Rust.
    let parser: Function = oracle.lua.globals().get("item_loading_parse").unwrap();
    let error = parser
        .call::<Table>(add_headers(&ring, "Quality (Cold Modifiers): 20"))
        .unwrap_err()
        .to_string();
    assert!(error.contains("nil"), "unexpected source error: {error}");
    assert!(error.contains("src/Classes/Item.lua"), "{error}");
    let item = oracle.parse(&add_headers(&ring, "Catalyst: Tul's\nCatalystQuality: 20"));
    item.get::<Function>("ParseRaw")
        .unwrap()
        .call::<()>((item.clone(), ring.clone()))
        .unwrap();
    let reparsed = fields(&item, &names);
    assert_eq!(reparsed["catalyst"], "Tul's");
    assert_eq!(reparsed["catalyst_quality"], 20.0);
    assert_eq!(fields(&oracle.parse(&ring), &names)["catalyst"], Json::Null);
    let pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Classes/ItemsTab.lua",
        "src/Modules/Common.lua",
        "src/Data/Bases/ring.lua",
        "src/Data/Bases/spear.lua",
    ]
    .into_iter()
    .map(|path| {
        let source = runtime::verified(path).unwrap();
        if path == "src/Classes/ItemsTab.lua" {
            assert_eq!(
                source
                    .matches("item:BuildAndParseRaw()\n\t\tt_insert(child, item.raw)")
                    .count(),
                1
            );
        }
        json!({"path":path,"sha256_lf":hash(source.as_bytes())})
    })
    .collect();
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"source-only header contract; importer may reject tolerant source syntax; no native lifecycle adoption",
        "source_pins":pins,"original_xml_sha256":[RING_HASH,SPEAR_HASH],
        "catalyst_headers":cases,"ordinary_quality":quality,
        "malformed_descriptor_error":error,"same_object_reparse_retains_catalyst":reparsed,
        "export":"authenticated ItemsTab.Save calls BuildAndParseRaw before serializing single item.raw text",
    })).unwrap());
}
