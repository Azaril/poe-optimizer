//! Original full Item.lua defence header behavior, independent of native rules.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[path = "support/defence_header_source.rs"]
mod source;
use mlua::{Table, Value};
use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataSnapshot, TrustPolicy, bundled_snapshot,
};
use poe_optimizer_import::item_loading::*;
use std::collections::BTreeMap;
fn canonical(value: Value) -> serde_json::Value {
    match value {
        Value::Nil => serde_json::json!(["nil"]),
        Value::Boolean(v) => serde_json::json!(["boolean", v]),
        Value::Integer(v) => {
            serde_json::json!(["number", format!("{:016x}", (v as f64).to_bits())])
        }
        Value::Number(v) => serde_json::json!(["number", format!("{:016x}", v.to_bits())]),
        Value::String(v) => serde_json::json!(["string", v.to_str().unwrap().to_owned()]),
        Value::Table(t) => {
            let mut rows = t
                .pairs::<Value, Value>()
                .map(|row| {
                    let (k, v) = row.unwrap();
                    (canonical(k), canonical(v))
                })
                .collect::<Vec<_>>();
            rows.sort_by_key(|(k, _)| k.to_string());
            serde_json::json!(["table", rows])
        }
        other => panic!("unsupported source snapshot {other:?}"),
    }
}
fn check_before(snapshot: &GameDataSnapshot, source: &source::Source, raw: &str) -> ItemState {
    source.parse(raw);
    let before = source.before();
    let mut provider = BuiltinItemLoadProvider::new(snapshot);
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    machine.apply_text(raw, &mut provider).unwrap();
    assert!(
        matches!(
            machine.status(),
            ItemLoadStatus::Pending | ItemLoadStatus::NoBase
        ),
        "{:?}",
        machine.pending()
    );
    if machine.status() == ItemLoadStatus::Pending {
        assert_eq!(
            machine.pending().map(|p| p.kind),
            Some(DependencyKind::Assembly),
            "{:?}",
            machine.pending()
        );
    }
    reference::compare_state(machine.state(), &before);
    let calls = machine
        .state()
        .parser_calls
        .iter()
        .map(|request| (request.text.clone(), request.combined))
        .collect::<Vec<_>>();
    assert_eq!(
        calls,
        source.calls(),
        "complete preassembly parser sequence for {raw}"
    );
    machine.into_state()
}
#[test]
fn all_original_defence_header_aliases_numeric_shapes_and_nil_removal_match_preassembly() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut values = vec![
        "17".to_owned(),
        "-0".into(),
        "-2.5".into(),
        "+.5".into(),
        "+12e3".into(),
        "12% (augmented)".into(),
        "1.2.3".into(),
        "invalid".into(),
        "0x10".into(),
    ];
    values.push("9".repeat(400));
    let mut count = 0;
    for (header, key) in [
        ("Armour", "Armour"),
        ("Evasion Rating", "Evasion"),
        ("Evasion", "Evasion"),
        ("Energy Shield", "EnergyShield"),
        ("Ward", "Ward"),
        ("Runic Ward", "Ward"),
    ] {
        for value in &values {
            let raw = format!("Rarity: NORMAL\nAmber Amulet\n{header}: {value}");
            let state = check_before(&snapshot, &source, &raw);
            assert!(
                state.armour_data.is_some(),
                "even nil assignment allocates table"
            );
            assert!(!state.retained_fields.contains_key("hidden_specs"));
            if let Some(table) = state.armour_data {
                assert!(table.keys().all(|name| name == key));
            }
            count += 1;
        }
    }
    for body in [
        "Armour: 7\nEvasion: 8\nArmour: invalid",
        "Ward: 7\nRunic Ward: invalid",
        "Evasion Rating: 7\nEvasion: 9",
        "Energy Shield: invalid",
    ] {
        check_before(
            &snapshot,
            &source,
            &format!("Rarity: NORMAL\nAmber Amulet\n{body}"),
        );
        count += 1;
    }
    eprintln!("Original defence header preassembly numeric/alias matrix: {count} paired cases");
}
#[test]
fn defence_headers_continue_into_existing_explicit_and_implicit_processing() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let mut count = 0;
    for header in [
        "Armour",
        "Evasion Rating",
        "Evasion",
        "Energy Shield",
        "Ward",
        "Runic Ward",
    ] {
        for (prefix, suffix) in [
            ("Implicits: 0\n+10 to maximum Life\n", "\n+11 to Dexterity"),
            ("Implicits: 2\n+10 to maximum Life\n", "\n+11 to Dexterity"),
            ("Implicits: 0\n", "\n+11 to Dexterity"),
        ] {
            let raw = format!("Rarity: NORMAL\nAmber Amulet\n{prefix}{header}: 17{suffix}");
            check_before(&snapshot, &source, &raw);
            count += 1;
        }
    }
    eprintln!("Original defence header section-continuation matrix: {count} paired cases");
}
fn with_bases(
    snapshot: &GameDataSnapshot,
    source: &source::Source,
    bases: &[(&str, &str)],
) -> GameDataSnapshot {
    let mut package = snapshot.package().clone();
    for &(name, template) in bases {
        source.copy_base(name, template);
        let mut row = package
            .item_loading
            .bases
            .iter()
            .find(|base| base.name == template)
            .unwrap()
            .clone();
        row.name = name.into();
        package.item_loading.bases.push(row);
    }
    package
        .item_loading
        .bases
        .sort_by(|a, b| a.name.cmp(&b.name));
    package.refresh_section_digests().unwrap();
    GameDataLoader::from_bytes(
        &serde_json::to_vec(&package).unwrap(),
        &TrustPolicy::AllowCustom,
        &Default::default(),
    )
    .unwrap()
}
const ARMOUR_ES: &str = "Two-Toned Boots (Armour/Energy Shield)";
const ARMOUR_EVASION: &str = "Two-Toned Boots (Armour/Evasion)";
const EVASION_ES: &str = "Two-Toned Boots (Evasion/Energy Shield)";
#[test]
fn authentic_missing_rewrite_targets_and_injected_bases_preserve_order_and_partial_state() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    for name in [ARMOUR_ES, ARMOUR_EVASION, EVASION_ES] {
        assert!(
            source.base(name).is_none(),
            "pin authentically lacks {name}"
        );
        assert!(
            snapshot.item_loading().base(name).is_none(),
            "complete injected catalog lacks same target"
        );
    }
    check_before(
        &snapshot,
        &source,
        "Rarity: NORMAL\nTwo-Toned Boots\nEvasion Rating: 17\nEnergy Shield: 23",
    );
    let source = source::Source::new();
    let custom = with_bases(
        &snapshot,
        &source,
        &[
            (ARMOUR_ES, "Rusted Greathelm"),
            (ARMOUR_EVASION, "Amber Amulet"),
            (EVASION_ES, "Frayed Shoes"),
        ],
    );
    for (body, expected) in [
        ("Evasion Rating: 17\nEnergy Shield: 23", EVASION_ES),
        ("Energy Shield: 23\nEvasion Rating: 17", ARMOUR_EVASION),
        ("Evasion: 17", ARMOUR_ES),
        ("Evasion Rating: invalid", ARMOUR_EVASION),
        ("Evasion Rating: 17\nEnergy Shield: invalid", EVASION_ES),
        (
            "Evasion Rating: 17\nEvasion Rating: invalid\nEnergy Shield: 23",
            EVASION_ES,
        ),
    ] {
        let state = check_before(
            &custom,
            &source,
            &format!("Rarity: NORMAL\n{ARMOUR_ES}\n{body}"),
        );
        assert_eq!(state.base_name.as_deref(), Some(expected));
        assert_eq!(
            state.item_type.as_deref(),
            Some("Helmet"),
            "rewrite must not call full base-selection routine"
        );
        assert_eq!(
            state.base_lines.len(),
            1,
            "rewrite must not invent another authored base line"
        );
    }
    // Start definition exists, first rewrite target is authoritatively absent,
    // and a later header can still match the retained baseName and rebind it.
    let source = source::Source::new();
    let partial = with_bases(
        &snapshot,
        &source,
        &[
            (ARMOUR_ES, "Rusted Greathelm"),
            (EVASION_ES, "Frayed Shoes"),
        ],
    );
    let lost = check_before(
        &partial,
        &source,
        &format!("Rarity: NORMAL\n{ARMOUR_ES}\nEvasion Rating: invalid"),
    );
    assert_eq!(lost.base_name.as_deref(), Some(ARMOUR_EVASION));
    assert!(!lost.base_present);
    assert_eq!(lost.armour_data, Some(BTreeMap::new()));
    let rebound = check_before(
        &partial,
        &source,
        &format!("Rarity: NORMAL\n{ARMOUR_ES}\nEvasion Rating: invalid\nEnergy Shield: 17"),
    );
    assert_eq!(rebound.base_name.as_deref(), Some(EVASION_ES));
    assert!(rebound.base_present);
}
fn replay_sequence(
    snapshot: &GameDataSnapshot,
    source: &source::Source,
    sequence: &[&str],
) -> Vec<ItemState> {
    let mut machine = ItemLoadMachine::new(snapshot.item_loading());
    let mut item = None;
    let mut retained_pointer = None;
    let mut states = vec![];
    for &raw in sequence {
        match &item {
            None => item = Some(source.parse(raw)),
            Some(item) => source.reparse(item, raw),
        }
        let item = item.as_ref().unwrap();
        let mut provider = reference::FrozenAssembly {
            dependencies: reference::OriginalDependencies::new(&source.oracle),
            stages: source.stages().into(),
        };
        machine.apply_text(raw, &mut provider).unwrap();
        if machine.status() == ItemLoadStatus::NoBase {
            assert_eq!(provider.stages.len(), 1);
        } else {
            assert!(provider.stages.is_empty());
            assert_eq!(machine.status(), ItemLoadStatus::Complete);
        }
        reference::compare_state(machine.state(), &source.snapshot(item));
        if let Some(table) = item.get::<Option<Table>>("armourData").unwrap() {
            let pointer = table.to_pointer();
            if let Some(prior) = retained_pointer {
                assert_eq!(
                    pointer, prior,
                    "original table identity persists across reparses and assembly"
                );
            }
            retained_pointer = Some(pointer);
        }
        states.push(machine.state().clone());
    }
    states
}
#[test]
fn original_assembly_then_reparse_preserves_absent_empty_and_retained_armour_tables() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let states = replay_sequence(
        &snapshot,
        &source,
        &[
            "Rarity: NORMAL\nAmber Amulet",
            "Rarity: NORMAL\nAmber Amulet\nArmour: invalid",
            "Rarity: NORMAL\nAmber Amulet",
            "Rarity: NORMAL\nAmber Amulet\nEvasion: 7\nWard: 9",
            "Rarity: NORMAL\nAmber Amulet\nRunic Ward: invalid",
            "Rarity: NORMAL\nunknown missing base\nEnergy Shield: 5",
        ],
    );
    assert!(states[0].armour_data.is_none());
    assert_eq!(states[1].armour_data, Some(BTreeMap::new()));
    assert_eq!(states[2].armour_data, Some(BTreeMap::new()));
    assert_eq!(
        states[4]
            .armour_data
            .as_ref()
            .unwrap()
            .keys()
            .cloned()
            .collect::<Vec<_>>(),
        ["Evasion"]
    );
    let other = replay_sequence(
        &snapshot,
        &source,
        &[
            "Rarity: NORMAL\nRusted Greathelm\nArmour: 900",
            "Rarity: NORMAL\nAmber Amulet",
            "Rarity: NORMAL\nAmber Amulet\nArmour: invalid",
        ],
    );
    assert_ne!(
        other[0].armour_data.as_ref().unwrap().get("Armour"),
        Some(&ItemNumber::new(900.0)),
        "actual source assembly replaces authored display amount"
    );
    eprintln!(
        "Original assembly/reparse lifecycle: {} full states including retained table identity",
        states.len() + other.len()
    );
}
#[test]
fn stale_base_identity_can_rebind_on_reparse_without_general_base_selection() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let custom = with_bases(
        &snapshot,
        &source,
        &[
            (ARMOUR_ES, "Rusted Greathelm"),
            (ARMOUR_EVASION, "Amber Amulet"),
            (EVASION_ES, "Frayed Shoes"),
        ],
    );
    let raw = format!("Rarity: NORMAL\n{ARMOUR_ES}\nArmour: 13");
    let states = replay_sequence(
        &custom,
        &source,
        &[
            &raw,
            "Rarity: NORMAL\ncaller missing identity\nEvasion Rating: invalid",
            "Rarity: NORMAL\ncaller missing identity\nEnergy Shield: 23",
        ],
    );
    assert_eq!(states[1].base_name.as_deref(), Some(ARMOUR_EVASION));
    assert_eq!(states[2].base_name.as_deref(), Some(EVASION_ES));
    for state in &states[1..] {
        assert!(state.base_present);
        assert!(
            state.base_lines.is_empty(),
            "header rebinding did not select an authored base line"
        );
        assert_eq!(state.item_type.as_deref(), Some("Helmet"));
    }
}
