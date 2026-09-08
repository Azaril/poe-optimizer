//! Independently read original base families and execute original Item local code.
use super::*;
use poe_optimizer_data::game_data::{ArmourBaseData, EquipmentSlot};
use poe_optimizer_data::tree_data::{SourceTable, SourceValue};
fn source_table(actual: Table, expected: &SourceTable) {
    assert_eq!(
        actual.clone().pairs::<mlua::Value, mlua::Value>().count(),
        expected.named.len() + expected.indexed.len()
    );
    for (key, value) in &expected.named {
        source_value(actual.get(key.as_str()).unwrap(), value);
    }
    for (key, value) in &expected.indexed {
        source_value(actual.get(*key).unwrap(), value);
    }
}
fn source_value(actual: mlua::Value, expected: &SourceValue) {
    match expected {
        SourceValue::Table(value) => source_table(actual.as_table().unwrap().clone(), value),
        SourceValue::String(value) => assert_eq!(
            actual.as_string().unwrap().to_str().unwrap().as_ref(),
            value
        ),
        SourceValue::Boolean(value) => assert_eq!(actual.as_boolean().unwrap(), *value),
        SourceValue::Integer(value) => assert_eq!(
            actual
                .as_number()
                .or_else(|| actual.as_integer().map(|v| v as f64))
                .unwrap(),
            *value as f64
        ),
        SourceValue::Number(value) => assert_eq!(
            actual
                .as_number()
                .or_else(|| actual.as_integer().map(|v| v as f64))
                .unwrap(),
            *value
        ),
    }
}
fn assert_base(lua: &Lua, raw: Table, base: &ArmourBaseData) -> Table {
    source_table(raw.clone(), &base.source);
    assert_eq!(raw.clone().pairs::<String, mlua::Value>().count(), 8);
    assert_eq!(raw.get::<u32>("quality").unwrap(), base.quality);
    assert_eq!(raw.get::<u32>("socketLimit").unwrap(), 3);
    let slot = match base.slot {
        EquipmentSlot::Helmet => "Helmet",
        EquipmentSlot::Gloves => "Gloves",
        EquipmentSlot::Boots => "Boots",
        EquipmentSlot::Amulet => panic!("wrong slot"),
    };
    assert_eq!(raw.get::<String>("type").unwrap(), slot);
    let ratings: Table = raw.get("armour").unwrap();
    let injected = lua.create_table().unwrap();
    let injected_ratings = lua.create_table().unwrap();
    for (field, value) in [
        ("Armour", base.armour),
        ("Evasion", base.evasion),
        ("EnergyShield", base.energy_shield),
    ] {
        assert_eq!(
            ratings.get::<Option<f64>>(field).unwrap().unwrap_or(0.0),
            value
        );
        injected_ratings.set(field, value).unwrap();
    }
    let req: Table = raw.get("req").unwrap();
    for (field, value) in [
        ("level", base.requirements.level),
        ("str", base.requirements.attributes.strength),
        ("dex", base.requirements.attributes.dexterity),
        ("int", base.requirements.attributes.intelligence),
    ] {
        assert_eq!(req.get::<Option<u32>>(field).unwrap().unwrap_or(0), value);
    }
    injected.set("armour", injected_ratings).unwrap();
    injected
}
#[test]
fn fresh_fixed_armour_package_matches_every_original_base_and_local_data_cold_and_warm() {
    let extracted = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap();
    let package = &extracted.package;
    assert_eq!(package.armour_bases.len(), 288);
    assert_eq!(package.actor.modifier_rules.len(), 329);
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let bases = lua.create_table().unwrap();
        for name in ["helmet", "gloves", "boots"] {
            lua.load(&oracle.sources[&format!("src/Data/Bases/{name}.lua")])
                .eval::<Function>()
                .unwrap()
                .call::<()>(bases.clone())
                .unwrap();
        }
        assert_eq!(bases.clone().pairs::<String, Table>().count(), 649);
        let source = &oracle.sources["src/Classes/Item.lua"];
        let calculate:Function=lua.load(format!(
            "local t_remove=table.remove;local m_floor=math.floor;{}; return function(base,quality,lines) local self={{base=base,quality=quality,armourData={{}}}};local modList=new('ModList'):ModList();for _,line in ipairs(lines)do local mods,extra=modLib.parseMod(itemLib.applyRange(line,1,1));assert(mods and not extra,line);for _,m in ipairs(mods)do modList:AddMod(m)end end;{};local remaining={{}};for _,mod in ipairs(modList)do remaining[#remaining+1]=mod end;return self.armourData,remaining end",
            section(source,"local function calcLocal(","-- Build list of modifiers"),
            section(source,"\t\tlocal armourData = self.armourData","\telseif self.base.flask then")
        )).eval().unwrap();
        let line_sets: &[&[&str]] = &[
            &[],
            &[
                "+11 to Armour",
                "+7 to Evasion Rating",
                "+3 to Energy Shield",
                "25% increased Defences",
            ],
            &[
                "+3 to Armour and Energy Shield",
                "+5 to Evasion Rating and Energy Shield",
                "+7 to Armour and Evasion",
                "17% increased Armour and Energy Shield",
                "19% increased Evasion and Energy Shield",
                "23% increased Armour and Evasion",
            ],
            &[
                "+11 to maximum Energy Shield",
                "30% increased Energy Shield",
                "20% increased maximum Energy Shield",
                "25% increased Global Armour",
                "+13% to all Resistances",
            ],
            &[
                "+17.5 to Evasion Rating",
                "100% reduced Evasion Rating",
                "+3 to Armour and Energy Shield",
                "200% reduced Armour and Energy Shield",
                "17% increased Armour if Strength is higher than Intelligence",
            ],
        ];
        for base in &package.armour_bases {
            let raw: Table = bases.get(base.name.as_str()).unwrap();
            let injected = assert_base(lua, raw.clone(), base);
            for quality in [0, 20] {
                for lines in line_sets {
                    let lines = lua.create_sequence_from(lines.iter().copied()).unwrap();
                    let (reference, left): (Table, Table) = calculate
                        .call((raw.clone(), quality, lines.clone()))
                        .unwrap();
                    let (actual, remaining): (Table, Table) =
                        calculate.call((injected.clone(), quality, lines)).unwrap();
                    let convert = |t| lua.from_value::<Value>(mlua::Value::Table(t)).unwrap();
                    assert_eq!(
                        convert(reference),
                        convert(actual),
                        "{} quality{quality} warm{warm}",
                        base.name
                    );
                    assert_eq!(
                        convert(left),
                        convert(remaining),
                        "{} remaining global records",
                        base.name
                    );
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 5760);
}
