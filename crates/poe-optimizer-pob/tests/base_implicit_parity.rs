//! Exact original baseHasImplicitLine and full preassembly Item state regressions.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_native.rs"]
mod reference;
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;
#[allow(dead_code)]
#[path = "support/defence_header_source.rs"]
mod source;

use mlua::{Function, Lua, Table, Value};
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_loading::{ItemLoadingCatalog, ItemMetadataValue},
};
use poe_optimizer_import::item_loading::*;

#[test]
fn original_helper_preserves_general_patterns_short_circuits_and_range_text_semantics() {
    let lua = Lua::new();
    let original = runtime::verified("src/Classes/Item.lua").unwrap();
    let start = original
        .find("local function baseHasImplicitLine(")
        .unwrap();
    let end = original[start..]
        .find("\n-- Special function")
        .map(|offset| start + offset)
        .unwrap();
    let helper: Function = lua
        .load(format!(
            "{}\nreturn baseHasImplicitLine",
            &original[start..end]
        ))
        .set_name("@original-Item-baseHasImplicitLine")
        .eval()
        .unwrap();
    for (definition, line, expected) in [
        (
            "Grants Skill: Level (1-20) A",
            "Grants Skill: Level 18 A",
            true,
        ),
        (
            "Grants Skill: Level (1-20) A",
            "Grants Skill: Level 999 A",
            true,
        ),
        (
            "Grants Skill: Level (1-20) A",
            "Grants Skill: Level -1 A",
            false,
        ),
        (
            "Grants Skill: (1-20) and (3-7)",
            "Grants Skill: 2 and 9",
            true,
        ),
        ("Grants Skill: A.B", "Grants Skill: AxB", true),
        ("Grants Skill: A%.B", "Grants Skill: A.B", true),
        ("Grants Skill: (A)%1", "Grants Skill: AA", true),
        ("Grants Skill: [AB]+", "Grants Skill: ABBA", true),
        ("Grants Skill: A", "prefix Grants Skill: A", false),
        ("Grants Skill: A", "Grants Skill: AB", false),
        ("Other\n\nGrants Skill: (1-2)", "Grants Skill: 7", true),
        ("Grants Skill: A\r\n", "Grants Skill: A", false),
        ("Grants Skill: A\r\n", "Grants Skill: A\r", true),
        ("Grants Skill: A\0ignored", "Grants Skill: A", true),
        ("Grants Skill: [", "Grants Skill: [", true),
        ("Grants Skill: A\nGrants Skill: [", "Grants Skill: A", true),
        ("Grants Skill: [", "Other", false),
        ("Other [", "Other", false),
    ] {
        let base = lua.create_table().unwrap();
        base.set("implicit", definition).unwrap();
        assert_eq!(
            helper.call::<bool>((base, line)).unwrap(),
            expected,
            "{definition:?} / {line:?}"
        );
    }
    let base = lua.create_table().unwrap();
    base.set("implicit", "Grants Skill: [").unwrap();
    let error = helper.call::<bool>((base, "Grants Skill: A")).unwrap_err();
    assert!(error.to_string().contains("malformed pattern"), "{error}");
}

fn paired(
    snapshot: &GameDataSnapshot,
    catalog: &ItemLoadingCatalog,
    source: &source::Source,
    raw: &str,
) -> ItemState {
    source.parse(raw);
    let mut provider = BuiltinItemLoadProvider::new(snapshot);
    let mut machine = ItemLoadMachine::new(catalog);
    machine.apply_text(raw, &mut provider).unwrap();
    assert_eq!(
        machine.pending().map(|pending| pending.kind),
        Some(DependencyKind::Assembly),
        "{raw}\n{:?}",
        machine.pending()
    );
    reference::compare_state(machine.state(), &source.before());
    assert_eq!(
        machine
            .state()
            .parser_calls
            .iter()
            .map(|call| (call.text.clone(), call.combined))
            .collect::<Vec<_>>(),
        source.calls(),
        "complete original parser call sequence for {raw}"
    );
    machine.into_state()
}

#[test]
fn granted_skill_range_implicits_match_complete_original_item_state() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    for (base, grant) in [
        (
            "Rattling Sceptre",
            "Grants Skill: Level 18 Skeletal Warrior Minion",
        ),
        (
            "Shrine Sceptre",
            "Grants Skill: Level 16 Purity of Lightning",
        ),
        ("Ashen Staff", "Grants Skill: Level 17 Firebolt"),
    ] {
        assert!(
            snapshot
                .item_loading()
                .base(base)
                .unwrap()
                .implicit()
                .unwrap()
                .contains("(1-20)")
        );
        for header in ["Implicits: 0\n", "Implicits: 1\n", "--------\n", ""] {
            let raw = format!("Rarity: NORMAL\n{base}\n{header}{grant}\n+12 to maximum Life");
            let state = paired(&snapshot, snapshot.item_loading(), &source, &raw);
            assert!(
                state
                    .implicit_mod_lines
                    .iter()
                    .any(|row| row.line == grant && row.flags.contains("implicit")),
                "{raw}"
            );
        }
    }
}

fn with_implicit(
    snapshot: &GameDataSnapshot,
    source: &source::Source,
    implicit: &str,
) -> ItemLoadingCatalog {
    let mut data = snapshot.item_loading().data().clone();
    data.bases
        .iter_mut()
        .find(|base| base.name == "Rattling Sceptre")
        .unwrap()
        .fields
        .fields
        .insert("implicit".into(), ItemMetadataValue::Text(implicit.into()));
    source
        .base("Rattling Sceptre")
        .unwrap()
        .set("implicit", implicit)
        .unwrap();
    ItemLoadingCatalog::new(data).unwrap()
}

#[test]
fn caller_base_patterns_and_reached_errors_match_original_source_order() {
    let snapshot = bundled_snapshot().unwrap();
    let source = source::Source::new();
    let grant = "Grants Skill: Level 18 Skeletal Warrior Minion";
    for implicit in [
        "Grants Skill: Level (1-20) Skele.al Warrior Minion",
        "Grants Skill: Level (%d+) Skeletal Warrior Minion",
        "Grants Skill: Level [0-9]+ Skeletal Warrior Minion",
        "Grants Skill: Level 18 Skeletal Warrior Minion\nGrants Skill: [",
        "Other\n\nGrants Skill: Level (1-20) Skeletal Warrior Minion",
    ] {
        let catalog = with_implicit(&snapshot, &source, implicit);
        let raw = format!("Rarity: NORMAL\nRattling Sceptre\nImplicits: 0\n{grant}");
        let state = paired(&snapshot, &catalog, &source, &raw);
        assert!(state.implicit_mod_lines[0].flags.contains("implicit"));
    }
    let catalog = with_implicit(&snapshot, &source, "Grants Skill: [");
    let raw = format!("Rarity: NORMAL\nRattling Sceptre\nImplicits: 0\n{grant}");
    source.clear();
    let original_error = source
        .oracle
        .lua
        .globals()
        .get::<Function>("item_loading_parse")
        .unwrap()
        .call::<Table>(raw.as_str())
        .unwrap_err();
    assert!(
        original_error.to_string().contains("malformed pattern"),
        "{original_error}"
    );
    assert!(
        source.calls().is_empty(),
        "the error precedes modifier parsing"
    );
    let mut provider = BuiltinItemLoadProvider::new(&snapshot);
    let mut machine = ItemLoadMachine::new(&catalog);
    machine.apply_text(&raw, &mut provider).unwrap_err();
    assert_eq!(machine.status(), ItemLoadStatus::SourceError);
    assert!(machine.state().parser_calls.is_empty());

    // Source guards avoid even a reached malformed base pattern in these modes.
    for raw in [
        format!("Rarity: NORMAL\nRattling Sceptre\nCrafted: true\nImplicits: 0\n{grant}"),
        format!("Caller Unique\nRattling Sceptre\nImplicits: 0\n{grant}"),
    ] {
        let state = paired(&snapshot, &catalog, &source, &raw);
        assert!(state.implicit_mod_lines.is_empty());
    }
}

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
