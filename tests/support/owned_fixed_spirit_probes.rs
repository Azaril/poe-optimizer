//! Admission controls use an actual body base with a complete miniature source
//! layout. They do not close any unchanged original item or final Spirit metric.
use super::release;
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
        .join("tests/fixtures/builds/breadth-20260908/build-01.xml");
    let original = fs::read_to_string(&fixture).unwrap();
    let start = original.find("<Item id=\"3\">").unwrap();
    let end = start + original[start..].find("</Item>").unwrap() + "</Item>".len();
    let mut cases: Vec<(String, String, String, Option<f64>)> = [1, 34, 48, 49, 1_000_000]
        .into_iter()
        .map(|amount| {
            (
                format!("integer-{amount}"),
                String::new(),
                format!("+{amount} to Spirit"),
                Some(f64::from(amount)),
            )
        })
        .collect();
    cases.extend([
        (
            "untagged-catalyst".into(),
            "Catalyst: Neural\nCatalystQuality: 20\n".into(),
            "+34 to Spirit".into(),
            Some(34.),
        ),
        (
            "untagged-negative-quality".into(),
            "Catalyst: Neural\nCatalystQuality: -200\n".into(),
            "+34 to Spirit".into(),
            Some(34.),
        ),
    ]);
    for (name, line) in [
        ("zero", "+0 to Spirit"),
        ("negative", "-34 to Spirit"),
        ("decimal", "+34.5 to Spirit"),
        ("above-bound", "+1000001 to Spirit"),
        ("tagged", "{tags:mana}+34 to Spirit"),
        ("corrupted-range", "{corruptedRange:1.5}+34 to Spirit"),
        ("disabled", "{disabled}+34 to Spirit"),
        ("unknown-prefix", "Unknown source semantics\n+34 to Spirit"),
    ] {
        cases.push((name.into(), String::new(), line.into(), None));
    }
    for (name, headers, line, amount) in &cases {
        let text = format!(
            "<Item id=\"3\">\nRarity: RARE\nFixed Spirit admission control\nRuneforged Adherent's Raiment\n{headers}Implicits: 0\n{line}\n+29 to Intelligence\n</Item>"
        );
        let xml = format!("{}{}{}", &original[..start], text, &original[end..]);
        let input = out.join(format!("probe-{name}.xml"));
        fs::write(&input, xml).unwrap();
        let dest = out.join(format!("probe-{name}"));
        release::normalize(package, &input, 1, &dest);
        let draft = read(dest.join("draft.json"));
        let sidecar = read(dest.join("sidecar.json"));
        let items: Vec<_> = draft["draft"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|i| i["template"]["value"]["key"] == "def.0000000000002014")
            .collect();
        assert_eq!(items.len(), 1);
        let item = items[0];
        let spirit: Vec<_> = item["modifiers"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m["definition"]["value"]["key"] == "def.000000000000314d")
            .collect();
        let source: Vec<_> = sidecar["item_texts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| {
                s["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|l| l["text"] == "Fixed Spirit admission control")
            })
            .collect();
        assert_eq!(source.len(), 1);
        let source = source[0];
        let spirit_line: Vec<_> = source["lines"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["text"].as_str().unwrap().contains("to Spirit"))
            .collect();
        assert_eq!(spirit_line.len(), 1);
        assert_eq!(item["modifiers"]["completion"]["kind"], "pending", "{name}");
        assert_eq!(item["modifier_order"]["kind"], "pending", "{name}");
        assert_eq!(
            item["parameters"]["completion"]["kind"], "pending",
            "{name}"
        );
        if let Some(amount) = amount {
            assert_eq!(spirit.len(), 1, "{name}");
            let modifier = spirit[0];
            assert_eq!(modifier["rolls"]["completion"], json!({"kind":"complete"}));
            let rolls = modifier["rolls"]["members"].as_array().unwrap();
            assert_eq!(rolls.len(), 24);
            assert_eq!(
                rolls[0]["slot"]["value"]["slot"]["key"],
                "def.000000000000314e"
            );
            assert_eq!(rolls[0]["value"]["value"]["kind"], "quantity");
            assert_eq!(rolls[0]["value"]["value"]["value"]["value"], *amount);
            assert_eq!(
                rolls[0]["value"]["value"]["value"]["unit"]["key"],
                "def.000000000000295a"
            );
            for flag in &rolls[1..22] {
                assert_eq!(
                    flag["value"],
                    json!({"kind":"known","value":{"kind":"boolean","value":false}})
                );
            }
            assert_eq!(rolls[22]["value"]["value"]["value"]["value"], 1.);
            assert_eq!(
                rolls[22]["value"]["value"]["value"]["unit"]["key"],
                "def.0000000000000001"
            );
            assert_eq!(rolls[23]["value"]["value"]["kind"], "option");
            assert_eq!(
                rolls[23]["value"]["value"]["value"]["key"],
                "def.00000000000030e2"
            );
            assert!(
                rolls
                    .iter()
                    .all(|r| r["slot"]["value"]["declaration"]["definition"]["key"]
                        == "def.000000000000314d")
            );
            assert_eq!(source["attribution"]["layout"]["status"], "proven");
            assert_eq!(spirit_line[0]["outcome"]["kind"], "known");
            assert_eq!(spirit_line[0]["outcome"]["value"]["rule"], "fixed-spirit");
            assert_eq!(spirit_line[0]["modifiers"], json!([modifier["id"]]));
        } else {
            assert!(
                spirit.is_empty(),
                "{name} stays outside reviewed input authority"
            );
            assert_eq!(spirit_line[0]["outcome"]["kind"], "pending", "{name}");
            assert_eq!(
                source["attribution"]["layout"]["status"], "pending",
                "{name}"
            );
        }
    }
    assert_eq!(fs::read_to_string(fixture).unwrap(), original);
    cases.len()
}
