//! Executes the actual modifier parser and unchanged calculation branches for
//! every pinned class and its two effective ordinary entrance nodes.
use super::*;
use poe_optimizer_engine::character::{
    self, CharacterAttributes, CharacterInput, CharacterModifiers,
};

pub(super) const PARSER: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Modules/ModParser.lua");

pub(super) fn install_defence_oracle(lua: &Lua) {
    let defence = SPARK_DEFENCE.replace("\r\n", "\n");
    let setup = SPARK_SETUP.replace("\r\n", "\n");
    let mut body = String::from(
        "return function(modDB,output,level) local m_min,m_max=math.min,math.max; local actor={itemList={},level=level}; ",
    );
    body.push_str(source_line(
        &setup,
        "modDB:NewMod(\"Evasion\", \"BASE\", data.characterConstants",
    ));
    body.push('\n');
    body.push_str(section(
        &defence,
        "\t\tlocal resourceList = {",
        "\n\t\toutput.LowestOfArmourAndEvasion",
    ));
    body.push_str("\nend");
    lua.globals()
        .set(
            "characterDefences",
            lua.load(body)
                .set_name("pinned-character-defence-branches")
                .eval::<Function>()
                .unwrap(),
        )
        .unwrap();
}

pub(super) fn input_table(lua: &Lua, input: &CharacterInput) -> Table {
    let table = lua.create_table().unwrap();
    table.set("strength", input.attributes.strength).unwrap();
    table.set("dexterity", input.attributes.dexterity).unwrap();
    table
        .set("intelligence", input.attributes.intelligence)
        .unwrap();
    let flags: Table = lua.globals().get("ModFlag").unwrap();
    let flag = |name| flags.get::<f64>(name).unwrap();
    let create: Function = lua
        .globals()
        .get::<Table>("modLib")
        .unwrap()
        .get("createMod")
        .unwrap();
    let modifiers = lua.create_table().unwrap();
    let m = input.modifiers;
    for (name, kind, value, flags) in [
        ("Armour", "BASE", m.armour_flat, 0.0),
        ("Evasion", "BASE", m.evasion_flat, 0.0),
        ("EnergyShield", "BASE", m.energy_shield_flat, 0.0),
        ("Speed", "INC", m.skill_speed_increased, 0.0),
        ("WarcrySpeed", "INC", m.skill_speed_increased, 0.0),
        ("TotemPlacementSpeed", "INC", m.skill_speed_increased, 0.0),
        ("Damage", "INC", m.spell_damage_increased, flag("Spell")),
        ("Damage", "INC", m.attack_damage_increased, flag("Attack")),
        ("Damage", "INC", m.melee_damage_increased, flag("Melee")),
        (
            "Damage",
            "INC",
            m.projectile_damage_increased,
            flag("Projectile"),
        ),
    ] {
        if value != 0.0 {
            modifiers
                .push(
                    create
                        .call::<Table>((name, kind, value, "Entrance", flags))
                        .unwrap(),
                )
                .unwrap();
        }
    }
    if m.minion_damage_increased != 0.0 {
        let nested = lua.create_table().unwrap();
        nested
            .set(
                "mod",
                create
                    .call::<Table>(("Damage", "INC", m.minion_damage_increased, "Entrance"))
                    .unwrap(),
            )
            .unwrap();
        modifiers
            .push(
                create
                    .call::<Table>(("MinionModifier", "LIST", nested, "Entrance"))
                    .unwrap(),
            )
            .unwrap();
    }
    table.set("mods", modifiers).unwrap();
    table
}

pub(super) struct Entrance {
    pub label: String,
    pub character: CharacterInput,
    pub stats: Vec<String>,
}

/// Resolve source node options by class name. IDs identify records, never output
/// values; the test fails on any previously unreviewed effect or missing record.
pub(super) fn entrances(lua: &Lua) -> Vec<Entrance> {
    let tree: Table = lua.load(SPARK_TREE).eval().unwrap();
    let classes: Table = tree.get("classes").unwrap();
    let nodes: Table = tree.get("nodes").unwrap();
    let mut result = Vec::new();
    for class in classes.sequence_values::<Table>() {
        let class = class.unwrap();
        let name: String = class.get("name").unwrap();
        let ids = match name.as_str() {
            "Ranger" | "Huntress" => [13828, 56651],
            "Warrior" => [3936, 38646],
            "Mercenary" => [59779, 59915],
            "Druid" => [13855, 50084],
            "Witch" | "Sorceress" => [44871, 4739],
            "Monk" => [10364, 52980],
            other => panic!("Unreviewed class {other}"),
        };
        for id in ids {
            let node = nodes
                .clone()
                .pairs::<mlua::Value, Table>()
                .filter_map(|pair| pair.ok().map(|(_, node)| node))
                .find(|node| node.get::<Option<u32>>("skill").unwrap() == Some(id))
                .unwrap();
            let effective = node
                .get::<Option<Table>>("options")
                .unwrap()
                .and_then(|options| options.get::<Option<Table>>(name.as_str()).unwrap())
                .unwrap_or(node);
            let stats: Vec<String> = effective
                .get::<Table>("stats")
                .unwrap()
                .sequence_values::<String>()
                .map(Result::unwrap)
                .collect();
            let mut m = CharacterModifiers::default();
            for text in &stats {
                match text.as_str() {
                    "+20 to Armour" => m.armour_flat += 20.0,
                    "+10 to Armour" => m.armour_flat += 10.0,
                    "+8 to Evasion Rating" => m.evasion_flat += 8.0,
                    "+16 to Evasion Rating" => m.evasion_flat += 16.0,
                    "+5 to maximum Energy Shield" => m.energy_shield_flat += 5.0,
                    "+10 to maximum Energy Shield" => m.energy_shield_flat += 10.0,
                    "4% increased Skill Speed" => m.skill_speed_increased += 4.0,
                    "10% increased Spell Damage" => m.spell_damage_increased += 10.0,
                    "10% increased Attack Damage" => m.attack_damage_increased += 10.0,
                    "10% increased Melee Damage" => m.melee_damage_increased += 10.0,
                    "10% increased Projectile Damage" => m.projectile_damage_increased += 10.0,
                    "Minions deal 10% increased Damage" => m.minion_damage_increased += 10.0,
                    other => panic!("Unreviewed entrance stat {other}"),
                }
            }
            result.push(Entrance {
                label: format!("{name}:{id}"),
                character: CharacterInput {
                    attributes: CharacterAttributes {
                        strength: class.get("base_str").unwrap(),
                        dexterity: class.get("base_dex").unwrap(),
                        intelligence: class.get("base_int").unwrap(),
                    },
                    modifiers: m,
                },
                stats,
            });
        }
    }
    assert_eq!(result.len(), 16);
    result
}

#[test]
fn entrance_typed_fields_match_actual_pinned_mod_parser_including_flags_and_minion_scope() {
    assert_eq!(
        format!("{:x}", Sha256::digest(PARSER.replace("\r\n", "\n"))),
        "6973c25f296c813187a85024e69737f0e69db43fc3fc8f281e1ac32e4409df95"
    );
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        let lua = &oracle.oracle.lua;
        // Unused gem/keystone lookup catalogs are empty; the complete original
        // parser processes the exact ordinary stat text and flags under test.
        lua.load("data.gems={}; data.keystones={}; data.skills={};")
            .exec()
            .unwrap();
        lua.load(section(
            &DATA.replace("\r\n", "\n"),
            "data.ailmentTypeList =",
            "data.buildupTypes =",
        ))
        .exec()
        .unwrap();
        let parser: Function = lua
            .load(PARSER)
            .set_name("pinned-ModParser-entrance-stats")
            .eval()
            .unwrap();
        let canonical:Function=lua.load("return function(mod) if mod.name=='MinionModifier' then return mod.name..'/'..mod.type..'/'..mod.value.mod.name..'/'..mod.value.mod.type..'/'..mod.value.mod.value..'/'..mod.value.mod.flags end; return mod.name..'/'..mod.type..'/'..mod.value..'/'..mod.flags..'/'..mod.keywordFlags end").eval().unwrap();
        for entrance in entrances(lua) {
            let mut expected = Vec::<String>::new();
            for stat in entrance.stats {
                for _ in 0..if warm { 200 } else { 1 } {
                    let (mods, extra): (Table, Option<String>) =
                        parser.call(stat.as_str()).unwrap();
                    assert!(extra.is_none(), "Unparsed stat {stat}: {extra:?}");
                    drop(mods);
                }
                let (mods, _): (Table, Option<String>) = parser.call(stat).unwrap();
                for modifier in mods.sequence_values::<Table>() {
                    expected.push(canonical.call(modifier.unwrap()).unwrap());
                }
            }
            let input = input_table(lua, &entrance.character);
            let mut actual: Vec<String> = input
                .get::<Table>("mods")
                .unwrap()
                .sequence_values::<Table>()
                .map(|modi| canonical.call(modi.unwrap()).unwrap())
                .collect();
            expected.sort();
            actual.sort();
            assert_eq!(actual, expected, "{}", entrance.label);
        }
    }
}

#[test]
fn all_class_entrances_match_actual_spark_resource_defence_and_damage_source() {
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        let cases = entrances(&oracle.oracle.lua);
        for entrance in cases {
            for level in [1, 60, 100] {
                for resistance in [-200.0, 0.0, 90.0] {
                    let input = SparkInput {
                        character_level: level,
                        resistance_penalty: -60.0,
                        enemy_lightning_resistance: resistance,
                        quests: SparkQuestRewards::default(),
                    };
                    let expected = oracle.calculate_with_character(&input, &entrance.character);
                    let output =
                        spark::evaluate_with_character(&input, &entrance.character).unwrap();
                    for (name, actual) in [
                        ("Life", output.life),
                        ("Mana", output.mana),
                        ("Str", output.strength),
                        ("Dex", output.dexterity),
                        ("Int", output.intelligence),
                        ("EnergyShield", output.energy_shield),
                        ("Armour", output.armour),
                        ("Evasion", output.evasion),
                        ("Speed", output.cast_rate),
                        ("AverageHit", output.average_hit),
                        ("TotalDPS", output.hit_dps),
                    ] {
                        assert_number(Some(actual), Some(expected.get(name).unwrap()));
                    }
                }
            }
        }
    }
}

#[test]
fn explicit_character_guards_preserve_defaults_and_reject_nonfinite_or_unsupported_numeric_inputs()
{
    let spark_input = SparkInput {
        character_level: 60,
        resistance_penalty: -60.0,
        enemy_lightning_resistance: 0.0,
        quests: SparkQuestRewards::default(),
    };
    assert_eq!(
        spark::evaluate(&spark_input),
        spark::evaluate_with_character(&spark_input, &spark::default_character())
    );
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -1.0,
        1_000_001.0,
        0.5,
    ] {
        for field in 0..3 {
            let mut character = spark::default_character();
            match field {
                0 => character.attributes.strength = value,
                1 => character.attributes.dexterity = value,
                _ => character.attributes.intelligence = value,
            };
            assert!(spark::evaluate_with_character(&spark_input, &character).is_err());
        }
    }
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        -0.01,
        1_000_001.0,
    ] {
        for field in 0..9 {
            let mut c = spark::default_character();
            let m = &mut c.modifiers;
            match field {
                0 => m.armour_flat = value,
                1 => m.evasion_flat = value,
                2 => m.energy_shield_flat = value,
                3 => m.skill_speed_increased = value,
                4 => m.spell_damage_increased = value,
                5 => m.attack_damage_increased = value,
                6 => m.melee_damage_increased = value,
                7 => m.projectile_damage_increased = value,
                _ => m.minion_damage_increased = value,
            };
            assert!(spark::evaluate_with_character(&spark_input, &c).is_err());
        }
    }
    let before = spark::evaluate(&spark_input).unwrap();
    let extremes = CharacterInput {
        attributes: CharacterAttributes {
            strength: 1_000_000.0,
            dexterity: 1_000_000.0,
            intelligence: 1_000_000.0,
        },
        modifiers: CharacterModifiers {
            armour_flat: 1_000_000.0,
            evasion_flat: 1_000_000.0,
            energy_shield_flat: 1_000_000.0,
            skill_speed_increased: 1_000_000.0,
            spell_damage_increased: 1_000_000.0,
            attack_damage_increased: 1_000_000.0,
            melee_damage_increased: 1_000_000.0,
            projectile_damage_increased: 1_000_000.0,
            minion_damage_increased: 1_000_000.0,
        },
    };
    let output = spark::evaluate_with_character(&spark_input, &extremes).unwrap();
    assert!(output.hit_dps.is_finite() && output.life.is_finite() && output.mana.is_finite());
    assert_eq!(spark::evaluate(&spark_input).unwrap(), before);
    let mut zero = spark::default_character();
    zero.attributes = CharacterAttributes::default();
    assert_eq!(
        spark::evaluate_with_character(&spark_input, &zero)
            .unwrap()
            .evasion,
        character::base_evasion()
    );
}
