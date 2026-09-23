//! Grouping-only oracle: complete original UpdateRunes, authenticated in isolation.
//! The parser callback records no modifier semantics; no UI/legacy runtime loads.
use mlua::{Function, HookTriggers, Lua, LuaOptions, StdLib, Table, VmState};
use poe_optimizer_pob::source;
use std::{
    path::Path,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicUsize, Ordering},
    },
};
struct Sources {
    method: String,
    catalog: String,
}
fn sources() -> &'static Sources {
    static SOURCES: OnceLock<Sources> = OnceLock::new();
    SOURCES.get_or_init(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
        let item = source::read_verified_text(&root, "src/Classes/Item.lua").unwrap();
        let catalog = source::read_verified_text(&root, "src/Data/ModRunes.lua").unwrap();
        let start = "function ItemClass:UpdateRunes()\n";
        let next = "function ItemClass:ApplySocketedRuneDisplayScalars()\n";
        assert_eq!(item.matches(start).count(), 1);
        assert_eq!(item.matches(next).count(), 1);
        let a = item.find(start).unwrap();
        let b = item.find(next).unwrap();
        assert!(a < b && b - a < 16 * 1024);
        assert!(catalog.len() < 2 * 1024 * 1024);
        Sources {
            method: item[a..b].into(),
            catalog,
        }
    })
}
type SyntheticRow<'a> = (&'a str, &'a str, f64);
fn oracle(
    rows: Option<&[SyntheticRow<'_>]>,
    names: &[&str],
    broad: Option<&str>,
    specific: &str,
) -> mlua::Result<Vec<String>> {
    assert!(names.len() <= 16);
    assert!(rows.is_none_or(|r| r.len() <= 16));
    let lua = Lua::new_with(
        StdLib::STRING | StdLib::TABLE | StdLib::JIT,
        LuaOptions::default(),
    )?;
    lua.set_memory_limit(16 * 1024 * 1024)?;
    lua.load("jit.off(); jit.flush(); jit=nil").exec()?;
    let instructions = Arc::new(AtomicUsize::new(0));
    lua.set_hook(
        HookTriggers::new().every_nth_instruction(100),
        move |_, _| {
            if instructions.fetch_add(100, Ordering::Relaxed) >= 1_000_000 {
                return Err(mlua::Error::RuntimeError(
                    "grouping oracle instruction bound".into(),
                ));
            }
            Ok(VmState::Continue)
        },
    )?;
    let env = lua.create_table()?;
    for name in ["ipairs", "pairs", "tonumber"] {
        env.raw_set(name, lua.globals().get::<Function>(name)?)?;
    }
    let table_lib: Table = lua.globals().get("table")?;
    env.raw_set("t_insert", table_lib.get::<Function>("insert")?)?;
    lua.load("function wipeTable(t) for k in pairs(t) do t[k]=nil end end; ItemClass={}; modLib={parseMod=function(_) return {} end}")
        .set_environment(env.clone()).exec()?;
    let runes = if let Some(rows) = rows {
        let runes = lua.create_table()?;
        for (name, text, order) in rows {
            let row = lua.create_table()?;
            row.raw_set("type", "Rune")?;
            row.raw_set(1, *text)?;
            row.raw_set("statOrder", lua.create_sequence_from([*order])?)?;
            let entry = lua.create_table()?;
            entry.raw_set("weapon", row)?;
            runes.raw_set(*name, entry)?;
        }
        runes
    } else {
        lua.load(&sources().catalog)
            .set_name("verified-ModRunes.lua")
            .set_mode(mlua::chunk::ChunkMode::Text)
            .set_environment(lua.create_table()?)
            .eval::<Table>()?
    };
    let mods = lua.create_table()?;
    mods.raw_set("Runes", runes)?;
    let data = lua.create_table()?;
    data.raw_set("itemMods", mods)?;
    env.raw_set("data", data)?;
    lua.load(&sources().method)
        .set_name("verified-original-UpdateRunes")
        .set_mode(mlua::chunk::ChunkMode::Text)
        .set_environment(env.clone())
        .exec()?;
    let class: Table = env.raw_get("ItemClass")?;
    let update: Function = class.raw_get("UpdateRunes")?;
    let item = lua.create_table()?;
    item.raw_set("runeModLines", lua.create_table()?)?;
    item.raw_set("socketedSoulCoreTypes", lua.create_table()?)?;
    item.raw_set("runes", lua.create_sequence_from(names.iter().copied())?)?;
    item.raw_set("itemSocketCount", names.len())?;
    let broad = broad.map(str::to_owned);
    let specific = specific.to_owned();
    item.raw_set(
        "GetSocketedAugmentTypes",
        lua.create_function(move |_, _: Table| Ok((broad.clone(), specific.clone())))?,
    )?;
    update.call::<()>(item.clone())?;
    let lines: Table = item.raw_get("runeModLines")?;
    lines
        .sequence_values::<Table>()
        .map(|row| row?.raw_get::<String>("line"))
        .collect()
}
#[test]
fn original_grouping_keeps_first_meaning_and_ignores_extra_unsigned_numbers() {
    for (a, b, expected) in [
        (
            "20% increased Damage",
            "5% reduced Damage",
            "25% increased Damage",
        ),
        ("-20% Resistance", "+8% Resistance", "-28% Resistance"),
        ("2 damage", "3 to 4 damage", "5 damage"),
        ("No numbers", "3 to 4 damage", "No numbers"),
    ] {
        assert_eq!(
            oracle(
                Some(&[("A", a, 1.0), ("B", b, 1.0)]),
                &["A", "B"],
                Some("weapon"),
                "spear"
            )
            .unwrap(),
            vec![expected]
        );
    }
    assert!(
        oracle(
            Some(&[("A", "3 to 4 damage", 1.0), ("B", "2 damage", 1.0)]),
            &["A", "B"],
            Some("weapon"),
            "spear"
        )
        .is_err()
    );
}
#[test]
fn original_each_step_formatting_and_formatted_order_keys_match_native_expectations() {
    assert_eq!(
        oracle(
            Some(&[
                ("A", "1.2345678901234 value", 1.0),
                ("B", "0.00000000000006 value", 1.0)
            ]),
            &["A", "B", "B"],
            Some("weapon"),
            "spear"
        )
        .unwrap(),
        vec!["1.2345678901236 value"]
    );
    assert_eq!(
        oracle(
            Some(&[
                ("A", "2 value", 100000000000000.0),
                ("B", "3 value", 100000000000001.0)
            ]),
            &["A", "B"],
            Some("weapon"),
            "spear"
        )
        .unwrap(),
        vec!["5 value"]
    );
}
#[test]
fn original_real_weapon_caster_and_bonded_only_rows_match_preparation_scope() {
    assert_eq!(
        oracle(None, &["Greater Iron Rune"], Some("weapon"), "spear").unwrap(),
        vec![
            "18% increased Physical Damage",
            "Bonded: 20% increased effect of Fully Broken Armour"
        ]
    );
    assert_eq!(
        oracle(
            None,
            &["Greater Iron Rune", "Greater Iron Rune"],
            Some("weapon"),
            "quarterstaff"
        )
        .unwrap(),
        vec![
            "36% increased Physical Damage",
            "Bonded: 40% increased effect of Fully Broken Armour"
        ]
    );
    assert_eq!(
        oracle(None, &["Adept Rune"], Some("caster"), "staff").unwrap(),
        vec!["Bonded: +80 to Evasion Rating", "+9 to Dexterity"]
    );
    assert_eq!(
        oracle(None, &["Idol of Silk"], Some("armour"), "buckler").unwrap(),
        vec!["Bonded: 30% increased Parry Range"]
    );
    assert_eq!(
        oracle(None, &["Raven-Touched Shard"], Some("armour"), "helmet").unwrap(),
        vec!["Raven-Touched"]
    );
}

#[test]
fn original_multi_digit_decimal_tokenization_and_repeated_merges_are_preserved() {
    for (first, second, names, expected) in [
        ("12.5 value", "12.5 value", vec!["A", "B"], "24.10 value"),
        (
            "12.5 value",
            "12.5 value",
            vec!["A", "B", "B"],
            "36.15 value",
        ),
        (
            "123.45 value",
            "123.45 value",
            vec!["A", "B", "B"],
            "369.135 value",
        ),
        (
            "-12.5 value",
            "+12.5 value",
            vec!["A", "B", "B"],
            "-36.15 value",
        ),
        ("1.25 value", "12.5 value", vec!["A", "B"], "13.25 value"),
        (
            "1.25 value",
            "12.5 value",
            vec!["A", "B", "B"],
            "25.30 value",
        ),
    ] {
        assert_eq!(
            oracle(
                Some(&[("A", first, 1.0), ("B", second, 1.0)]),
                &names,
                Some("weapon"),
                "spear"
            )
            .unwrap(),
            vec![expected]
        );
    }
}
