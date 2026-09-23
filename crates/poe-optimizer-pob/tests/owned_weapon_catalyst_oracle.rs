//! Optional source observations for shared catalyst input transport.
//! This does not establish catalyst crafting legality, native parity, or a
//! complete imported item/build. Every observation starts with a fresh Item.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_formatter_runtime.rs"]
mod formatter;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

use formatter::FormatterOracle;
use mlua::Table;
use serde_json::json;
use sha2::{Digest, Sha256};

#[derive(Clone, Copy)]
struct Headers {
    name: &'static str,
    text: &'static str,
    selection: bool,
    amount: Option<f64>,
    matching_scalar: f64,
    matching_cold: f64,
}

fn cold(rows: Table) -> f64 {
    assert!(rows.raw_len() <= 64);
    let mut amounts = vec![];
    for row in rows.sequence_values::<Table>() {
        let row = row.unwrap();
        if row.get::<String>("name").unwrap() == "ColdResist" {
            assert_eq!(row.get::<String>("type").unwrap(), "BASE");
            amounts.push(row.get::<f64>("value").unwrap());
        }
    }
    assert_eq!(amounts.len(), 1, "one exact source ColdResist row");
    amounts[0]
}

#[test]
#[ignore = "optional authenticated fresh Item catalyst header oracle; requires pinned vendor submodule"]
fn fresh_ring_and_weapon_catalyst_headers_share_scalar_semantics() {
    let oracle = FormatterOracle::new();
    let policy: Table = oracle.source.lua.globals().get("itemPolicy").unwrap();
    let catalysts: Table = policy.get("catalysts").unwrap();
    let tul = catalysts
        .sequence_values::<String>()
        .map(|value| value.unwrap())
        .position(|name| name == "Tul's")
        .unwrap()
        + 1;
    let cases = [
        Headers {
            name: "absent",
            text: "",
            selection: false,
            amount: None,
            matching_scalar: 1.0,
            matching_cold: 25.0,
        },
        Headers {
            name: "amount-only",
            text: "CatalystQuality: 37\n",
            selection: false,
            amount: Some(37.0),
            matching_scalar: 1.0,
            matching_cold: 25.0,
        },
        Headers {
            name: "selection-only-default-twenty",
            text: "Catalyst: Tul's\n",
            selection: true,
            amount: None,
            matching_scalar: 1.2,
            matching_cold: 30.0,
        },
        Headers {
            name: "explicit-zero",
            text: "Catalyst: Tul's\nCatalystQuality: 0\n",
            selection: true,
            amount: Some(0.0),
            matching_scalar: 1.0,
            matching_cold: 25.0,
        },
        Headers {
            name: "explicit-twenty",
            text: "Catalyst: Tul's\nCatalystQuality: 20\n",
            selection: true,
            amount: Some(20.0),
            matching_scalar: 1.2,
            matching_cold: 30.0,
        },
    ];
    let mut observations = vec![];
    for (base, receiving_slots) in [("Sapphire Ring", 3), ("Grand Spear", 2)] {
        for headers in cases {
            for quality in [0, 20] {
                for (label, matches) in [("cold", true), ("fire", false)] {
                    // The fixture deliberately supplies a legal source grammar,
                    // not an assertion about game affix or catalyst eligibility.
                    let raw = format!(
                        "Rarity: RARE\nOracle Fixture\n{base}\nQuality: {quality}\n{}Implicits: 1\n{{tags:{label}}}{{range:0.5}}+(20-30)% to Cold Resistance",
                        headers.text,
                    );
                    let item = oracle.source.parse(&raw);
                    assert_eq!(item.get::<String>("baseName").unwrap(), base);
                    assert_eq!(
                        item.get::<Option<f64>>("catalyst").unwrap(),
                        headers.selection.then_some(tul as f64),
                        "{base}/{}",
                        headers.name,
                    );
                    assert_eq!(
                        item.get::<Option<f64>>("catalystQuality").unwrap(),
                        headers.amount
                    );
                    assert_eq!(item.get::<f64>("quality").unwrap(), f64::from(quality));
                    let lines: Table = item.get("implicitModLines").unwrap();
                    assert_eq!(lines.raw_len(), 1);
                    let line: Table = lines.raw_get(1).unwrap();
                    assert!(line.get::<Option<String>>("extra").unwrap().is_none());
                    assert_eq!(line.get::<f64>("range").unwrap(), 0.5);
                    let tags: Vec<String> = line
                        .get::<Table>("modTags")
                        .unwrap()
                        .sequence_values()
                        .map(|value| value.unwrap())
                        .collect();
                    assert_eq!(tags, [label]);
                    let expected_scalar = if matches {
                        headers.matching_scalar
                    } else {
                        1.0
                    };
                    let expected_cold = if matches { headers.matching_cold } else { 25.0 };
                    assert_eq!(line.get::<f64>("valueScalar").unwrap(), expected_scalar);
                    assert_eq!(cold(line.get("modList").unwrap()), expected_cold);
                    assert_eq!(item.get::<Table>("modMagnitudeMods").unwrap().raw_len(), 0);
                    let slots: Table = item.get("slotModList").unwrap();
                    assert_eq!(slots.raw_len(), receiving_slots);
                    let values: Vec<_> = slots
                        .sequence_values::<Table>()
                        .map(|rows| cold(rows.unwrap()))
                        .collect();
                    assert!(values.iter().all(|value| *value == expected_cold));
                    observations.push(json!({
                        "base":base,"headers":headers.name,"ordinary_quality":quality,
                        "source_property":label,"matching_property":matches,
                        "stored_catalyst":headers.selection.then_some(tul),
                        "stored_catalyst_amount":headers.amount,"scalar":expected_scalar,
                        "cold_by_receiving_slot":values,
                    }));
                }
            }
        }
    }
    assert_eq!(observations.len(), 40);
    let source_pins: Vec<_> = [
        "src/Classes/Item.lua",
        "src/Modules/ItemTools.lua",
        "src/Modules/Common.lua",
        "src/Data/ModScalability.lua",
    ]
    .into_iter()
    .map(|path| {
        let text = runtime::verified(path).unwrap();
        json!({"path":path,"sha256_lf":format!("{:x}",Sha256::digest(text.as_bytes()))})
    })
    .collect();
    println!("{}", serde_json::to_string_pretty(&json!({
        "scope":"fresh authenticated Item header/scalar behavior on ring and weapon; ordinary quality is independent; crafting legality and native/build parity are not asserted",
        "source_pins":source_pins,"cases":observations,
    })).unwrap());
}
