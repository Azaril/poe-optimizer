//! Complete miniature source layouts and deliberate exclusions, using the exact
//! source-witness text. No complete item or equipment-legality claim is made.
use super::{family, release};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
pub fn run(package: &Path, out: &Path) -> usize {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/builds/breadth-20260908/build-04.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let start = original.find("<Item id=\"19\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + "</Item>".len();
    let vectors: Value = family::read("source-vectors.json");
    let controls = vectors["report"]["builds"][0]["state"]["constructors"]
        .as_array()
        .unwrap();
    let mut cases: Vec<(String, Option<f64>)> = [0, 1, 16, 20, 44, 1_000_000]
        .into_iter()
        .map(|v| (format!("integer-{v}"), Some(f64::from(v))))
        .collect();
    for (name, amount) in [
        ("leading-zero", 44.),
        ("untagged-carapace", 44.),
        ("untagged-negative-quality", 44.),
        ("armour-global", 11.),
    ] {
        cases.push((name.into(), Some(amount)));
    }
    for name in [
        "integer-1000001",
        "tagged-carapace",
        "decimal",
        "reduced",
        "negative",
        "ranged",
        "corrupted-range",
        "disabled",
        "unknown-predecessor",
        "armour-local",
    ] {
        cases.push((name.into(), None));
    }
    for (name, expected) in &cases {
        let control = controls.iter().find(|p| p["name"] == *name).unwrap();
        let raw = control["raw"].as_str().unwrap();
        let text = format!("<Item id=\"19\">\n{raw}\n</Item>");
        let input = out.join(format!("probe-{name}.xml"));
        fs::write(
            &input,
            format!("{}{}{}", &original[..start], text, &original[end..]),
        )
        .unwrap();
        let destination = out.join(format!("probe-{name}"));
        release::normalize(package, &input, 4, &destination);
        let draft = read(destination.join("draft.json"));
        let sidecar = read(destination.join("sidecar.json"));
        let mut sources = sidecar["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| {
                s["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|l| l["text"] == "Global Energy Shield witness")
            });
        let source = sources.next().unwrap();
        assert!(sources.next().is_none());
        let records: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|item| {
                item["modifiers"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(move |m| (item, m))
            })
            .filter(|(_, m)| m["definition"]["value"]["key"] == "def.0000000000003334")
            .collect();
        if let Some(amount) = expected {
            assert_eq!(records.len(), 1, "{name}");
            let (item, modifier) = records[0];
            assert_eq!(item["modifiers"]["completion"]["kind"], "pending");
            assert_eq!(item["modifier_order"]["kind"], "pending");
            assert_eq!(item["parameters"]["completion"]["kind"], "pending");
            assert_eq!(modifier["rolls"]["completion"]["kind"], "complete");
            let rolls = modifier["rolls"]["members"].as_array().unwrap();
            assert_eq!(rolls.len(), 24);
            assert_eq!(
                rolls[0]["slot"]["value"]["slot"]["key"],
                "def.0000000000003335"
            );
            assert_eq!(rolls[0]["value"]["value"]["value"]["value"], *amount);
            assert_eq!(
                rolls[0]["value"]["value"]["value"]["unit"]["key"],
                "def.0000000000000002"
            );
            for flag in &rolls[1..22] {
                assert_eq!(
                    flag["value"],
                    json!({"kind":"known","value":{"kind":"boolean","value":false}})
                );
            }
            assert_eq!(rolls[22]["value"]["value"]["value"]["value"], 1.);
            assert_eq!(
                rolls[23]["value"]["value"]["value"]["key"],
                "def.00000000000030e2"
            );
            assert!(
                rolls
                    .iter()
                    .all(|r| r["slot"]["value"]["declaration"]["definition"]["key"]
                        == "def.0000000000003334")
            );
            assert_eq!(
                source["attribution"]["layout"]["status"], "proven",
                "{name}"
            );
            let line = source["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["modifiers"] == json!([modifier["id"]]))
                .unwrap();
            assert_eq!(line["outcome"]["kind"], "known");
            assert_eq!(
                line["outcome"]["value"]["rule"],
                "fixed-global-energy-shield-increase"
            );
        } else {
            assert!(records.is_empty(), "{name}: no global modifier admission");
            if name != "armour-local" {
                assert_eq!(
                    source["attribution"]["layout"]["status"], "pending",
                    "{name}"
                );
            }
        }
    }
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
    cases.len()
}
