//! Actual pinned Item.calcLocal, weapon assembly and ModParser source oracles.
//! No expected local-stat formula is rewritten in Lua.
use super::*;
use poe_optimizer_data::game_data::{self, ItemModifierRoll};
use poe_optimizer_engine::{
    CompiledGameData,
    mace::{self, MaceWeapon, MaceWeaponData},
    weapon::{self, WeaponStats},
};
const ITEM: &str = include_str!("../../../../vendor/path-of-building-poe2/src/Classes/Item.lua");
const BASES: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Data/Bases/mace.lua");

pub(super) fn modifier_table(lua: &Lua, modifiers: &[ModifierInput]) -> Table {
    let rows = lua.create_table().unwrap();
    for modifier in modifiers {
        let row = lua.create_table().unwrap();
        let ModifierKind::Numeric(kind) = modifier.kind else {
            panic!("numeric oracle input")
        };
        let ModifierValue::Number(value) = modifier.value else {
            panic!("numeric oracle input")
        };
        row.set("name", modifier.name.as_str()).unwrap();
        row.set("type", kind.upstream_name()).unwrap();
        row.set("value", value).unwrap();
        row.set("flags", modifier.flags as f64).unwrap();
        row.set("keywordFlags", modifier.keyword_flags as f64)
            .unwrap();
        row.set("source", modifier.source.as_deref()).unwrap();
        for kind in &modifier.tag_kinds {
            let tag = lua.create_table().unwrap();
            tag.set("type", kind.as_str()).unwrap();
            row.push(tag).unwrap();
        }
        rows.push(row).unwrap();
    }
    rows
}
struct WeaponOracle {
    oracle: Oracle,
    local: Function,
    assemble: Function,
    parser: Function,
}
impl WeaponOracle {
    fn new(warm: bool) -> Self {
        for (path, text) in [
            ("src/Classes/Item.lua", ITEM),
            ("src/Data/Bases/mace.lua", BASES),
            ("src/Modules/ModParser.lua", character_parity::PARSER),
        ] {
            let identity = mace::SOURCE_FILES
                .iter()
                .find(|entry| entry.path == path)
                .unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(text.replace("\r\n", "\n"))),
                identity.sha256,
                "{path}"
            );
        }
        let oracle = SparkOracle::new(warm).oracle;
        let lua = &oracle.lua;
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
        let parser = lua
            .load(character_parity::PARSER)
            .set_name("pinned-item-ModParser")
            .eval()
            .unwrap();
        let item = ITEM.replace("\r\n", "\n");
        let local_source = section(
            &item,
            "local function calcLocal(",
            "-- Build list of modifiers in a given slot number",
        );
        let header = "local t_remove=table.remove; local m_floor=math.floor; local dmgTypeList={'Physical','Lightning','Cold','Fire','Chaos'};";
        let local = lua.load(format!("{header}\n{local_source}\nreturn function(mods,name,kind,flags,warm) local result,list,second; for i=1,(warm and 200 or 1) do list=copyTable(mods); result=calcLocal(list,name,kind,flags); second=calcLocal(list,name,kind,flags) end; return result,second,list end")).set_name("pinned-item-calcLocal").eval().unwrap();
        let block = section(
            &item,
            "\tif self.base.weapon then\n\t\tlocal weaponData",
            "\n\t\tfor _, value in ipairs(modList:List(nil, \"WeaponData\"))",
        );
        let assemble = lua.load(format!("{header}\n{local_source}\nlocal function calculate(input) local self={{base={{weapon=input.base,type='One Handed Mace'}},quality=input.quality,name='Resolved supplied base',weaponData={{}}}}; local slotNum=1; local modList=copyTable(input.mods); {block}\nend; return {{stats=self.weaponData[1],remaining=modList}} end; return function(input,warm) local result; for i=1,(warm and 200 or 1) do result=calculate(input) end; return result end")).set_name("pinned-item-weapon-assembly").eval().unwrap();
        Self {
            oracle,
            local,
            assemble,
            parser,
        }
    }
    fn parse(&self, line: &str) -> Vec<ModifierInput> {
        let mut last = None;
        for _ in 0..if self.oracle.warm { 200 } else { 1 } {
            let (rows, extra): (Table, Option<String>) = self.parser.call(line).unwrap();
            assert!(extra.is_none(), "unparsed source text: {extra:?}");
            last = Some(rows);
        }
        last.unwrap()
            .sequence_values::<Table>()
            .map(|row| {
                let row = row.unwrap();
                ModifierInput {
                    name: row.get("name").unwrap(),
                    kind: ModifierKind::Numeric(
                        match row.get::<String>("type").unwrap().as_str() {
                            "BASE" => NumericKind::Base,
                            "INC" => NumericKind::Increased,
                            "MORE" => NumericKind::More,
                            other => panic!("unrepresented source kind {other}"),
                        },
                    ),
                    value: ModifierValue::Number(row.get("value").unwrap()),
                    flags: row.get::<f64>("flags").unwrap() as u64,
                    keyword_flags: row.get::<f64>("keywordFlags").unwrap() as u64,
                    source: None,
                    tag_kinds: row
                        .sequence_values::<Table>()
                        .map(|tag| tag.unwrap().get("type").unwrap())
                        .collect(),
                }
            })
            .collect()
    }
    fn assembly(
        &self,
        base: MaceWeaponData<'_>,
        quality: u32,
        modifiers: &[ModifierInput],
    ) -> Table {
        let lua = &self.oracle.lua;
        let input = lua.create_table().unwrap();
        let raw = lua.create_table().unwrap();
        for (name, value) in [
            ("PhysicalMin", base.physical_minimum),
            ("PhysicalMax", base.physical_maximum),
            ("FireMin", base.fire_minimum),
            ("FireMax", base.fire_maximum),
            ("AttackRateBase", base.attack_rate),
            ("CritChanceBase", base.critical_chance),
            ("Range", 11.0),
        ] {
            raw.set(name, value).unwrap();
        }
        input.set("base", raw).unwrap();
        input.set("quality", quality).unwrap();
        input.set("mods", modifier_table(lua, modifiers)).unwrap();
        self.assemble.call((input, self.oracle.warm)).unwrap()
    }
}
fn assert_stats(actual: WeaponStats, result: &Table) {
    let expected: Table = result.get("stats").unwrap();
    for (name, actual) in [
        ("PhysicalMin", actual.physical_minimum),
        ("PhysicalMax", actual.physical_maximum),
        ("FireMin", actual.fire_minimum),
        ("FireMax", actual.fire_maximum),
        ("AttackSpeedInc", actual.attack_speed_increased),
        ("AttackRate", actual.attack_rate),
        ("CritChance", actual.critical_chance),
    ] {
        assert_number(
            Some(actual),
            Some(expected.get::<Option<f64>>(name).unwrap().unwrap_or(0.0)),
        );
    }
    assert_eq!(
        actual.physical_present,
        expected.contains_key("PhysicalMin").unwrap()
    );
    assert_eq!(
        actual.fire_present,
        expected.contains_key("FireMin").unwrap()
    );
}
fn identities(rows: &Table) -> Vec<String> {
    rows.clone()
        .sequence_values::<Table>()
        .map(|row| row.unwrap().get::<String>("source").unwrap())
        .collect()
}
#[test]
fn literal_local_queries_preserve_exact_flags_keywords_first_tag_and_consumed_order() {
    for warm in [false, true] {
        let oracle = WeaponOracle::new(warm);
        for kind in [
            NumericKind::Base,
            NumericKind::Increased,
            NumericKind::More,
            NumericKind::Override,
        ] {
            for flags in [0, 1, 3, 256, 1_u64 << 40] {
                let mut modifiers = Vec::new();
                for candidate_flags in [0, 1, 3, 256, 1_u64 << 40] {
                    for keywords in [0, 1] {
                        for tags in [
                            vec![],
                            vec!["InSlot"],
                            vec!["Condition"],
                            vec!["InSlot", "Condition"],
                            vec!["Condition", "InSlot"],
                            vec!["InSlot", "Unknown"],
                        ] {
                            let mut value = modifier("Local", kind, 7.25);
                            value.flags = candidate_flags;
                            value.keyword_flags = keywords;
                            value.tag_kinds = tags.into_iter().map(str::to_owned).collect();
                            value.source = Some(modifiers.len().to_string());
                            modifiers.push(value);
                        }
                    }
                }
                let source = modifier_table(&oracle.oracle.lua, &modifiers);
                let (expected, second, remaining): (f64, f64, Table) = oracle
                    .local
                    .call((source, "Local", kind.upstream_name(), flags as f64, warm))
                    .unwrap();
                let before = modifiers.len();
                let result =
                    weapon::consume_local_numeric(&mut modifiers, "Local", kind, flags).unwrap();
                assert_number(Some(result.value), Some(expected));
                assert_eq!(result.consumed, before - modifiers.len());
                assert_eq!(
                    modifiers
                        .iter()
                        .map(|row| row.source.clone().unwrap())
                        .collect::<Vec<_>>(),
                    identities(&remaining)
                );
                assert_number(
                    Some(
                        weapon::consume_local_numeric(&mut modifiers, "Local", kind, flags)
                            .unwrap()
                            .value,
                    ),
                    Some(second),
                );
            }
        }
    }
}
#[test]
fn actual_item_assembly_matches_duplicate_rolls_quality_rounding_zero_endpoints_and_rates() {
    for warm in [false, true] {
        let oracle = WeaponOracle::new(warm);
        let lines = [
            "Adds 3 to 7 Physical Damage",
            "Adds 1 to 9 Fire Damage",
            "19% increased Physical Damage",
            "13% increased Attack Speed",
            "2500% increased Critical Hit Chance",
        ];
        let parsed: Vec<Vec<ModifierInput>> = lines.iter().map(|line| oracle.parse(line)).collect();
        let mut combined = parsed.iter().flatten().cloned().collect::<Vec<_>>();
        combined.extend(parsed[0].clone());
        let mut variants = vec![vec![], combined];
        variants.extend(parsed);
        for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
            for quality in [0, 1, 7, 19, 20] {
                for mods in &variants {
                    let mut remaining = mods.clone();
                    let actual =
                        weapon::assemble_local_weapon(weapon.data(), quality, &mut remaining)
                            .unwrap();
                    let expected = oracle.assembly(weapon.data(), quality, mods);
                    assert_stats(actual, &expected);
                    assert!(remaining.is_empty());
                    assert_eq!(expected.get::<Table>("remaining").unwrap().raw_len(), 0);
                }
            }
        }
        for edge in [
            0.0, 0.0049, 0.005, 0.0149, 0.015, 0.49, 0.5, 0.505, 1.0049, 1.005, 1.015, 2.995,
        ] {
            let base = MaceWeaponData {
                physical_minimum: edge,
                physical_maximum: edge + 7.0,
                fire_minimum: edge,
                fire_maximum: edge + 3.0,
                attack_rate: edge,
                critical_chance: edge,
                ..MaceWeapon::WoodenClub.data()
            };
            for quality in [0, 1, 20] {
                for mods in &variants {
                    let mut remaining = mods.clone();
                    let actual =
                        weapon::assemble_local_weapon(base, quality, &mut remaining).unwrap();
                    assert_stats(actual, &oracle.assembly(base, quality, mods));
                }
            }
        }
    }
}
#[test]
fn source_parser_distinguishes_local_added_damage_from_global_attack_modifiers() {
    for warm in [false, true] {
        let oracle = WeaponOracle::new(warm);
        let mut mods = oracle.parse("Adds 3 to 7 Physical Damage");
        let local = mods.len();
        mods.extend(oracle.parse("Adds 3 to 7 Physical Damage to Attacks"));
        let expected = oracle.assembly(MaceWeapon::WoodenClub.data(), 0, &mods);
        let original = mods.len();
        let actual =
            weapon::assemble_local_weapon(MaceWeapon::WoodenClub.data(), 0, &mut mods).unwrap();
        assert_stats(actual, &expected);
        assert_eq!(original - mods.len(), local);
        assert_eq!(
            mods.len(),
            expected.get::<Table>("remaining").unwrap().raw_len()
        );
        assert!(
            mods.iter()
                .all(|modifier| modifier.keyword_flags == 65_536 && modifier.flags == 0),
            "{mods:?}"
        );
    }
}
fn rolls(data: &CompiledGameData, values: &[Vec<f64>]) -> Vec<ItemModifierRoll> {
    data.snapshot()
        .package()
        .item_modifier_rules
        .iter()
        .zip(values)
        .map(|(rule, values)| ItemModifierRoll {
            rule_id: rule.id.clone(),
            values: values.clone(),
        })
        .collect()
}
#[test]
fn supplied_local_weapon_rolls_match_actual_parser_item_and_full_mace_source_with_all_supports() {
    let data = CompiledGameData::bundled().unwrap();
    let values: Vec<_> = data
        .snapshot()
        .package()
        .item_modifier_rules
        .iter()
        .map(|rule| match rule.modifiers[0].stat {
            game_data::LocalWeaponStat::PhysicalMinimum => vec![3.0, 7.0],
            game_data::LocalWeaponStat::FireMinimum => vec![1.0, 9.0],
            game_data::LocalWeaponStat::PhysicalDamage => vec![19.0],
            game_data::LocalWeaponStat::Speed => vec![13.0],
            game_data::LocalWeaponStat::CriticalChance => vec![2500.0],
            _ => unreachable!(),
        })
        .collect();
    let rolls = rolls(&data, &values);
    let keys = [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ];
    for warm in [false, true] {
        let oracle = WeaponOracle::new(warm);
        let pipeline = mace_parity::MaceOracle::new(warm);
        let mut mods = vec![];
        for roll in &rolls {
            let rule = data
                .snapshot()
                .package()
                .item_modifier_rule(&roll.rule_id)
                .unwrap();
            let mut line = rule.template.clone();
            for (index, value) in roll.values.iter().enumerate() {
                line = line.replace(&format!("{{{index}}}"), &value.to_string());
            }
            mods.extend(oracle.parse(&line));
        }
        for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
            for quality in [0, 20] {
                for armour in [0.0, 1500.0] {
                    for speed in [0.0, 6.0] {
                        for keys in &keys {
                            let keys: Vec<String> = keys.iter().map(|key| (*key).into()).collect();
                            let ids: Vec<_> = keys
                                .iter()
                                .map(|key| {
                                    data.snapshot()
                                        .package()
                                        .support(key)
                                        .unwrap()
                                        .skill_id
                                        .as_str()
                                })
                                .collect();
                            let input = mace::MaceInput {
                                weapon,
                                quality,
                                enemy_armour: armour,
                                enemy_fire_resistance: 70.0,
                                ..mace_parity::input()
                            };
                            let mut character = mace::default_character();
                            character.modifiers.skill_speed_increased = speed;
                            let prepared = data
                                .prepare_mace_weapon(weapon, quality, input.item_level, &rolls)
                                .unwrap();
                            assert!(
                                prepared.stats().critical_chance
                                    > data.snapshot().package().character.critical_chance_cap
                            );
                            let native = mace::evaluate_with_components(
                                &input,
                                &character,
                                &data,
                                &prepared,
                                data.mace_support_loadout(&keys).unwrap(),
                            )
                            .unwrap();
                            assert!(native.crit_chance <= native.hit_chance);
                            mace_parity::compare(
                                native,
                                pipeline.calculate_with_local_modifiers(
                                    &input, &character, &ids, &mods,
                                ),
                            );
                        }
                    }
                }
            }
        }
    }
}

fn custom_data(edit: impl FnOnce(&mut game_data::GameDataPackage)) -> CompiledGameData {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    let snapshot = game_data::GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &game_data::TrustPolicy::AllowCustom,
        &game_data::LoadLimits::default(),
    )
    .unwrap();
    CompiledGameData::compile(std::sync::Arc::new(snapshot)).unwrap()
}
#[test]
fn custom_base_rounding_suppression_and_critical_cap_follow_the_complete_source_pipeline() {
    for warm in [false, true] {
        let pipeline = mace_parity::MaceOracle::new(warm);
        for (minimum, fire, rate, critical, cap) in [
            (0.49, 0.49, 1.005, 9.995, 5.25),
            (0.0, 0.0, 0.0049, 100.0, 100.0),
            (0.5, 0.5, 1.015, 3.555, 0.0),
        ] {
            let data = custom_data(|package| {
                let weapon = package
                    .weapons
                    .iter_mut()
                    .find(|weapon| weapon.id == "smithing_hammer")
                    .unwrap();
                weapon.physical_minimum = minimum;
                weapon.physical_maximum = 7.5;
                weapon.fire_minimum = fire;
                weapon.fire_maximum = 3.5;
                weapon.attack_rate = rate;
                weapon.critical_chance = critical;
                package.character.critical_chance_cap = cap;
            });
            for quality in [0, 20] {
                for keys in [
                    vec![],
                    vec!["brutality_i".to_owned(), "rapid_attacks_i".to_owned()],
                ] {
                    let ids: Vec<_> = keys
                        .iter()
                        .map(|key| {
                            data.snapshot()
                                .package()
                                .support(key)
                                .unwrap()
                                .skill_id
                                .as_str()
                        })
                        .collect();
                    let input = mace::MaceInput {
                        weapon: MaceWeapon::SmithingHammer,
                        quality,
                        enemy_armour: 1500.0,
                        enemy_fire_resistance: 70.0,
                        ..mace_parity::input()
                    };
                    let mut character = mace::default_character();
                    character.modifiers.skill_speed_increased = 6.0;
                    let prepared = data
                        .prepare_mace_weapon(input.weapon, quality, input.item_level, &[])
                        .unwrap();
                    let actual = mace::evaluate_with_components(
                        &input,
                        &character,
                        &data,
                        &prepared,
                        data.mace_support_loadout(&keys).unwrap(),
                    )
                    .unwrap();
                    mace_parity::compare(
                        actual,
                        pipeline.calculate_with_weapon_overrides(
                            &input,
                            &character,
                            &ids,
                            &[],
                            Some((data.weapon(input.weapon), cap)),
                        ),
                    );
                }
            }
        }
    }
}
#[test]
fn injected_global_critical_cap_and_fractional_spark_chance_match_actual_offence_rounding() {
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm);
        for chance in [0.0, 3.555, 3.565, 9.0049, 99.999] {
            for cap in [0.0, 1.0, 5.0, 37.25, 100.0] {
                let data = custom_data(|package| {
                    package.spark.critical_chance = chance;
                    package.character.critical_chance_cap = cap;
                });
                let input = SparkInput {
                    character_level: 60,
                    resistance_penalty: -60.0,
                    enemy_lightning_resistance: 50.0,
                    quests: SparkQuestRewards::default(),
                };
                let character = spark::default_character();
                let actual = spark::evaluate_with_data(&input, &character, &data).unwrap();
                let expected =
                    oracle.calculate_with_critical_data(&input, &character, Some((chance, cap)));
                for (name, value) in [
                    ("CritChance", actual.crit_chance),
                    ("AverageHit", actual.average_hit),
                    ("TotalDPS", actual.hit_dps),
                    ("Life", actual.life),
                    ("Mana", actual.mana),
                ] {
                    assert_number(Some(value), Some(expected.get(name).unwrap()));
                }
                assert!(actual.crit_chance <= cap);
            }
        }
    }
}
