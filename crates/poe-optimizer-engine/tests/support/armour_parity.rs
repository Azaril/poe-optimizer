//! Unchanged pinned local armour/GetArmourDataValue and per-slot receiving oracles.
use super::*;
use poe_optimizer_data::game_data::{
    ActorCondition, ActorModifierEffect, ActorModifierRecord, ActorModifierTag,
    ActorNumericOperation as Op, ActorStat, EquipmentSlot,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{ActorModifierLayer, ActorScratch},
    armour::{self, ArmourBaseValues, ArmourSlots, ArmourStats},
    mace,
};
const ITEM: &str = include_str!("../../../../vendor/path-of-building-poe2/src/Classes/Item.lua");
pub(super) struct ArmourOracle {
    oracle: Oracle,
    assemble: Function,
    getter: Function,
}
impl ArmourOracle {
    pub(super) fn new(warm: bool) -> Self {
        let identity = mace::SOURCE_FILES
            .iter()
            .find(|entry| entry.path == "src/Classes/Item.lua")
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(ITEM.replace("\r\n", "\n"))),
            identity.sha256
        );
        let oracle = SparkOracle::new(warm).oracle;
        let lua = &oracle.lua;
        const MODLIST: &str =
            include_str!("../../../../vendor/path-of-building-poe2/src/Classes/ModList.lua");
        let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(MODLIST.replace("\r\n", "\n"))),
            snapshot.package().manifest.provenance["src/Classes/ModList.lua"]
        );
        lua.load(MODLIST)
            .set_name("unchanged-ModList-for-armour")
            .exec()
            .unwrap();
        let item = ITEM.replace("\r\n", "\n");
        let local = section(
            &item,
            "local function calcLocal(",
            "-- Build list of modifiers in a given slot number",
        );
        let block = section(
            &item,
            "\t\tlocal armourData = self.armourData\n",
            "\n\t\tfor _, value in ipairs(modList:List(nil, \"ArmourData\")) do",
        );
        let assemble = lua.load(format!("local t_remove=table.remove; {local} local function calculate(input) local self={{base={{armour=input.base}},quality=input.quality,armourData={{}},modSource=input.source}}; local modList=new('ModList'):ModList(); for _,mod in ipairs(input.mods) do modList:AddMod(copyTable(mod)) end; {block} return {{stats=self.armourData,remaining=modList}} end; return function(input,warm) local result; for i=1,(warm and 200 or 1) do result=calculate(input) end; return result end")).set_name("unchanged-Item-local-armour").eval().unwrap();
        let getter = lua.load(format!("local ItemClass={{}}; {} return function(fixed,coefficient,level,warm) local item={{armourData={{Armour=fixed,ArmourPerLevel=coefficient}}}}; local result; for i=1,(warm and 200 or 1) do result=ItemClass.GetArmourDataValue(item,'Armour',level) end; return result end",section(&item,"function ItemClass:GetArmourDataValue(","-- Calculate local modifiers"))).set_name("unchanged-GetArmourDataValue").eval().unwrap();
        Self {
            oracle,
            assemble,
            getter,
        }
    }
    fn assembly(
        &self,
        base: ArmourBaseValues,
        quality: u32,
        modifiers: &[ModifierInput],
    ) -> (ArmourStats, Table) {
        self.assembly_with_penalty(base, quality, modifiers, None, "old fixed-only oracle")
    }
    pub(super) fn assembly_with_penalty(
        &self,
        base: ArmourBaseValues,
        quality: u32,
        modifiers: &[ModifierInput],
        penalty: Option<f64>,
        source: &str,
    ) -> (ArmourStats, Table) {
        let lua = &self.oracle.lua;
        let input = lua.create_table().unwrap();
        let bases = lua.create_table().unwrap();
        for (name, value) in [
            ("Armour", base.armour),
            ("Evasion", base.evasion),
            ("EnergyShield", base.energy_shield),
        ] {
            bases.set(name, value).unwrap();
        }
        bases.set("MovementPenalty", penalty).unwrap();
        input.set("source", source).unwrap();
        input.set("base", bases).unwrap();
        input.set("quality", quality).unwrap();
        input
            .set("mods", weapon_parity::modifier_table(lua, modifiers))
            .unwrap();
        let result: Table = self.assemble.call((input, self.oracle.warm)).unwrap();
        let stats: Table = result.get("stats").unwrap();
        (
            ArmourStats {
                base_armour: stats.get("ArmourBase").unwrap(),
                base_evasion: stats.get("EvasionBase").unwrap(),
                base_energy_shield: stats.get("EnergyShieldBase").unwrap(),
                armour: stats.get("Armour").unwrap(),
                evasion: stats.get("Evasion").unwrap(),
                energy_shield: stats.get("EnergyShield").unwrap(),
            },
            result.get("remaining").unwrap(),
        )
    }
}
fn raw(name: &str, kind: NumericKind, value: f64) -> ModifierInput {
    ModifierInput {
        name: name.into(),
        kind: ModifierKind::Numeric(kind),
        value: ModifierValue::Number(value),
        flags: 0,
        keyword_flags: 0,
        source: Some("local source".into()),
        tag_kinds: vec![],
    }
}
fn record(stat: ActorStat, operation: Op, value: f64) -> ActorModifierRecord {
    ActorModifierRecord {
        stat,
        effect: ActorModifierEffect::Numeric { operation, value },
        source: Some("armour source".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn raw_records(records: &[ActorModifierRecord]) -> Vec<ModifierInput> {
    records
        .iter()
        .map(|record| {
            let ActorModifierEffect::Numeric { operation, value } = record.effect else {
                panic!("numeric corpus")
            };
            let mut value = raw(
                record.stat.upstream_name(),
                match operation {
                    Op::Base => NumericKind::Base,
                    Op::Increased => NumericKind::Increased,
                    _ => panic!("numeric corpus"),
                },
                value,
            );
            value.tag_kinds = record
                .tags
                .iter()
                .map(|tag| {
                    match tag {
                        ActorModifierTag::Global => "Global",
                        ActorModifierTag::Condition { .. } => "Condition",
                    }
                    .into()
                })
                .collect();
            value
        })
        .collect()
}
#[test]
fn original_local_armour_matches_signed_pairs_quality_rounding_and_literal_consumption() {
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = ArmourOracle::new(warm);
        for name in [
            "Armour",
            "Evasion",
            "EnergyShield",
            "ArmourAndEvasion",
            "ArmourAndEnergyShield",
            "EvasionAndEnergyShield",
        ] {
            for flat in [-1000.5, -1.5, -0.5, 0.0, 0.499999999999, 0.5, 17.25] {
                for inc in [-250.0, -100.0, -99.5, 0.0, 33.333333] {
                    for quality in [0, 7, 20] {
                        let base = ArmourBaseValues {
                            armour: 10.25,
                            evasion: 20.5,
                            energy_shield: 30.75,
                        };
                        let mut records = vec![
                            raw(name, NumericKind::Base, flat),
                            raw(name, NumericKind::Increased, inc),
                            raw("Defences", NumericKind::Increased, 0.125),
                        ];
                        for tags in [
                            vec!["Global"],
                            vec!["Condition"],
                            vec!["InSlot", "Condition"],
                            vec!["Condition", "InSlot"],
                        ] {
                            let mut tagged = raw(name, NumericKind::Base, 0.75);
                            tagged.tag_kinds = tags.into_iter().map(String::from).collect();
                            records.push(tagged);
                        }
                        let mut wrong_flags = raw(name, NumericKind::Increased, 400.0);
                        wrong_flags.flags = 1;
                        records.push(wrong_flags);
                        let mut wrong_keywords = raw(name, NumericKind::Base, 400.0);
                        wrong_keywords.keyword_flags = 1;
                        records.push(wrong_keywords);
                        let (expected, leftovers) = oracle.assembly(base, quality, &records);
                        let actual =
                            armour::assemble_local_armour(base, quality, &mut records).unwrap();
                        assert_eq!(actual, expected, "{warm}/{name}/{flat}/{inc}/{quality}");
                        assert_eq!(records.len(), leftovers.raw_len());
                        for (actual, expected) in
                            records.iter().zip(leftovers.sequence_values::<Table>())
                        {
                            let expected = expected.unwrap();
                            assert_eq!(actual.name, expected.get::<String>("name").unwrap());
                            assert_eq!(actual.flags as f64, expected.get::<f64>("flags").unwrap());
                            assert_eq!(
                                actual.keyword_flags as f64,
                                expected.get::<f64>("keywordFlags").unwrap()
                            );
                            assert_eq!(actual.tag_kinds.len(), expected.raw_len());
                        }
                        cases += 1;
                    }
                }
            }
        }
        // Deliberately sensitive pairing order and separate quality multiplier.
        let mut records = vec![
            raw("EnergyShield", NumericKind::Increased, 1e6),
            raw("ArmourAndEnergyShield", NumericKind::Increased, 1e-11),
            raw("EvasionAndEnergyShield", NumericKind::Increased, -1e6),
            raw("Defences", NumericKind::Increased, -100.0),
        ];
        let base = ArmourBaseValues {
            armour: 1.5,
            evasion: 1.5,
            energy_shield: 1e6,
        };
        let (expected, _) = oracle.assembly(base, 20, &records);
        assert_eq!(
            armour::assemble_local_armour(base, 20, &mut records).unwrap(),
            expected
        );
        cases += 1;
    }
    eprintln!("Local armour original-source assembly: {cases} cold/warm cases");
}
#[test]
fn original_armour_getter_rounds_per_level_separately_even_when_fixed_results_are_negative() {
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = ArmourOracle::new(warm);
        for fixed in [-100.0, -1.0, -0.0, 0.0, 1.0, 100.0] {
            for coefficient in [-0.51, -0.5, -0.01, 0.0, 0.005, 0.5, 0.51] {
                for level in [0, 1, 60, 100] {
                    let expected: f64 = oracle
                        .getter
                        .call((fixed, coefficient, level, warm))
                        .unwrap();
                    assert_eq!(
                        armour::armour_data_value(fixed, coefficient, level)
                            .unwrap()
                            .to_bits(),
                        expected.to_bits()
                    );
                    cases += 1;
                }
            }
        }
    }
    assert!(armour::armour_data_value(f64::NAN, 0.0, 1).is_err());
    eprintln!(
        "GetArmourDataValue original-source arithmetic: {cases} cold/warm cases; per-level profile admission remains unsupported"
    );
}
#[test]
fn original_receiver_uses_local_slots_then_global_base_with_final_attribute_conditions() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    let scenario = data.receiving_scenario(SparkQuestRewards::default(), -20.0);
    let bases = [
        EquipmentSlot::Helmet,
        EquipmentSlot::Gloves,
        EquipmentSlot::Boots,
    ]
    .map(|slot| {
        data.snapshot()
            .package()
            .armour_bases
            .iter()
            .find(|base| base.slot == slot)
            .unwrap()
    });
    let mut cases = 0;
    for warm in [false, true] {
        let local = ArmourOracle::new(warm);
        let receiver = actor_parity::ActorOracle::new_receiving(warm);
        for mask in 0..8 {
            for quality in [0, 7, 20] {
                for inc in [-250.0, 33.333333] {
                    let items = bases.map(|base| {
                        let mut conditional = record(ActorStat::EnergyShield, Op::Base, 17.5);
                        conditional.tags.push(ActorModifierTag::Condition {
                            variables: vec![ActorCondition::IntHigherThanDex],
                            negated: false,
                        });
                        let mut global = record(ActorStat::ArmourAndEvasion, Op::Increased, 0.125);
                        global.tags.push(ActorModifierTag::Global);
                        let records = vec![
                            record(ActorStat::Armour, Op::Base, -1.5),
                            record(ActorStat::EvasionAndEnergyShield, Op::Base, 20.5),
                            record(ActorStat::Defences, Op::Increased, inc),
                            record(ActorStat::Int, Op::Base, 25.0),
                            conditional,
                            global,
                        ];
                        let expected = local
                            .assembly(
                                ArmourBaseValues {
                                    armour: base.armour,
                                    evasion: base.evasion,
                                    energy_shield: base.energy_shield,
                                },
                                quality,
                                &raw_records(&records),
                            )
                            .0;
                        let prepared = data.prepare_armour(&base.id, quality, 1, &records).unwrap();
                        assert_eq!(prepared.stats(), expected);
                        (prepared, expected)
                    });
                    let mut globals = vec![
                        record(ActorStat::Armour, Op::Base, 0.5),
                        record(ActorStat::Defences, Op::Increased, inc),
                    ];
                    let mut expected_slots = vec![];
                    for (i, (item, expected)) in items.iter().enumerate() {
                        if mask & (1 << i) != 0 {
                            globals.extend_from_slice(item.global_records());
                            expected_slots.push((["Helmet", "Gloves", "Boots"][i], *expected));
                        }
                    }
                    let slots = ArmourSlots {
                        helmet: (mask & 1 != 0).then_some(&items[0].0),
                        gloves: (mask & 2 != 0).then_some(&items[1].0),
                        boots: (mask & 4 != 0).then_some(&items[2].0),
                        body_armour: None,
                    };
                    let expected = receiver.calculate_receiving_with_armour(
                        &data,
                        60,
                        quests,
                        scenario,
                        &character,
                        &[globals.clone()],
                        &expected_slots,
                    );
                    let actual = data
                        .prepare_actor_with_armour(
                            60,
                            quests,
                            scenario,
                            &character,
                            &[globals.clone()],
                            slots,
                        )
                        .unwrap();
                    receiving_parity::compare(actual.receiving().unwrap(), &expected);
                    let program = data.compile_actor_modifiers(&globals).unwrap();
                    let compiled = data
                        .evaluate_actor_with_armour(
                            60,
                            quests,
                            scenario,
                            &character,
                            &[ActorModifierLayer {
                                programs: &[&program],
                            }],
                            slots,
                            &mut ActorScratch::default(),
                        )
                        .unwrap();
                    assert_eq!(actual.values(), compiled.values());
                    assert_eq!(actual.receiving(), compiled.receiving());
                    cases += 1;
                }
            }
        }
    }
    eprintln!("Original per-slot receiving after original local armour: {cases} cold/warm cases");
}

#[test]
fn original_item_capture_formatter_preserves_symmetric_rounding_and_source_precision() {
    const TOOLS: &str =
        include_str!("../../../../vendor/path-of-building-poe2/src/Modules/ItemTools.lua");
    let text = TOOLS.replace("\r\n", "\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(text.as_bytes())),
        "24e114bc64d8e213d4ed970c34fa013088e7b298050c65055c179a0b5f302973",
        "Review scalar-one formatting after changing ItemTools"
    );
    let block = section(&text, "function itemLib.formatValue(", "local antonyms =");
    let common = COMMON.replace("\r\n", "\n");
    let symmetric = section(
        &common,
        "function roundSymmetric(",
        "-- Use rounding formula for positive numbers",
    );
    let mut cases = 0;
    for warm in [false, true] {
        let oracle = SparkOracle::new(warm).oracle;
        let calculate:Function=oracle.lua.load(format!("local m_min=math.min; local m_floor=math.floor; local m_ceil=math.ceil; {symmetric} local itemLib={{}}; {block} return function(value,precision,display,required,warm) local result; for i=1,(warm and 200 or 1) do result=itemLib.formatValue(value,1,1,precision,display,required) end; return tonumber(result) end")).set_name("unchanged-ItemTools-formatValue-scalar-one").eval().unwrap();
        for precision in [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 10.0, 12.0, 15.0, 20.0, 60.0, 100.0, 1000.0, 10000.0,
        ] {
            for display in [None, Some(0), Some(1), Some(2)] {
                for required in [false, true] {
                    for value in [
                        -999999.999,
                        -17.5,
                        -1.555,
                        -0.505,
                        -0.5,
                        -0.005,
                        -0.0,
                        0.0,
                        0.005,
                        0.4999999999,
                        0.5,
                        0.505,
                        1.555,
                        17.5,
                        999999.999,
                    ] {
                        let expected: f64 = calculate
                            .call((value, precision, display, required, warm))
                            .unwrap();
                        let actual = poe_optimizer_engine::item_format::format_item_capture(
                            value, precision, display,
                        )
                        .unwrap();
                        assert_eq!(
                            actual, expected,
                            "value={value},precision={precision},display={display:?},required={required},warm={warm}"
                        );
                        cases += 1;
                    }
                }
            }
        }
    }
    assert!(poe_optimizer_engine::item_format::format_item_capture(f64::NAN, 1.0, None).is_err());
    assert!(poe_optimizer_engine::item_format::format_item_capture(1.0, 0.0, None).is_err());
    assert!(poe_optimizer_engine::item_format::format_item_capture(1.0, 1.0, Some(3)).is_err());
    eprintln!("Original item scalar-one formatter: {cases} cold/warm numeric capture cases");
}
