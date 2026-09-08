//! Actual source attribute/inherent-bonus/max-resource function execution.
use super::*;
use poe_optimizer_data::game_data::{
    self, ActorCondition as AC, ActorModifierEffect as Effect, ActorModifierRecord as Record,
    ActorModifierTag as Tag, ActorNumericOperation as Op, ActorStat as Stat,
};
use poe_optimizer_engine::{
    CompiledGameData,
    actor::{ActorQuestSelection, ActorResourceOutput},
    character::{CharacterAttributes, CharacterInput},
};
use std::sync::Arc;

fn numeric(stat: Stat, operation: Op, value: f64) -> Record {
    Record {
        stat,
        effect: Effect::Numeric { operation, value },
        source: Some("Explicit test input".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn flag(stat: Stat, value: bool) -> Record {
    Record {
        stat,
        effect: Effect::Flag { value },
        source: Some("Explicit test input".into()),
        flags: 0,
        keyword_flags: 0,
        tags: vec![],
    }
}
fn conditional(mut record: Record, conditions: Vec<AC>, negated: bool) -> Record {
    record.tags.push(Tag::Condition {
        variables: conditions,
        negated,
    });
    record
}
pub(super) struct ActorOracle {
    oracle: Oracle,
    calculate: Function,
}
impl ActorOracle {
    fn new(warm: bool) -> Self {
        Self::new_scoped(warm, false)
    }
    pub(super) fn new_receiving(warm: bool) -> Self {
        Self::new_scoped(warm, true)
    }
    fn new_scoped(warm: bool, receiving: bool) -> Self {
        let oracle = SparkOracle::new(warm).oracle;
        let lua = &oracle.lua;
        let perform = SPARK_PERFORM.replace("\r\n", "\n");
        let setup = SPARK_SETUP.replace("\r\n", "\n");
        let offence = SPARK_OFFENCE.replace("\r\n", "\n");
        let mut body = String::from("local m_min,m_max=math.min,math.max; local ItemClass={}; ");
        if receiving {
            let item =
                include_str!("../../../../vendor/path-of-building-poe2/src/Classes/Item.lua")
                    .replace("\r\n", "\n");
            body.push_str(section(
                &item,
                "function ItemClass:GetArmourDataValue(",
                "-- Calculate local modifiers",
            ));
        }
        body.push_str(section(
            &perform,
            "local function calculateAttributes(",
            "-- Calculate attributes, and set conditions",
        ));
        body.push_str("local function calculate(input) local parent; for i=#input.layers,2,-1 do local db=new('ModDB'):ModDB(parent); for _,mod in ipairs(input.layers[i]) do db:AddMod(copyTable(mod)) end; parent=db end; local modDB=new('ModDB'):ModDB(parent); local output={}; local condList=modDB.conditions; local actor={modDB=modDB,output=output,itemList=input.armour_items or {},level=input.level}; for _,item in pairs(actor.itemList) do item.GetArmourDataValue=ItemClass.GetArmourDataValue end; modDB.actor=actor; local breakdown=nil; modDB.multipliers.Level=input.level; ");
        body.push_str("for _,stat in ipairs({'Str','Dex','Int'}) do modDB:NewMod(stat,'BASE',input.attributes[stat],'Base') end; ");
        body.push_str(section(
            &setup,
            "\t\tmodDB:NewMod(\"Life\", \"BASE\", data.characterConstants",
            "\t\tmodDB:NewMod(\"ManaRegen\"",
        ));
        body.push_str(source_line(&setup, "modDB:NewMod(\"Spirit\", \"BASE\","));
        body.push('\n');
        body.push_str(source_line(
            &setup,
            "modDB:NewMod(\"Accuracy\", \"BASE\", data.characterConstants",
        ));
        body.push('\n');
        if receiving {
            body.push_str("local env={configInput={resistancePenalty=input.receiving_penalty}}; ");
            body.push_str(source_line(
                &setup,
                "modDB:NewMod(\"Evasion\", \"BASE\", data.characterConstants",
            ));
            body.push('\n');
            for prefix in resistance_parity::SETUP_PREFIXES {
                body.push_str(source_line(&setup, prefix));
                body.push('\n');
            }
        }
        body.push_str("for _,mod in ipairs(input.quests) do modDB:AddMod(copyTable(mod)) end; for _,mod in ipairs(input.layers[1] or {}) do modDB:AddMod(copyTable(mod)) end; calculateAttributes(modDB,output,nil,condList); ");
        body.push_str(source_line(
            &perform,
            "output.TotalAttr = output.Str + output.Dex + output.Int",
        ));
        body.push('\n');
        body.push_str(section(
            &perform,
            "\t-- Add attribute bonuses\n",
            "\t-- Calculate Presence / Surrounded",
        ));
        body.push_str("sparkCalcs.doActorLifeManaSpirit(actor,true); local skillModList=modDB; local cfg={flags=0,keywordFlags=0}; ");
        for prefix in [
            "local base = skillModList:Sum(\"BASE\", cfg, \"Accuracy\")",
            "local inc = skillModList:Sum(\"INC\", cfg, \"Accuracy\")",
            "local more = skillModList:More(\"MORE\", cfg, \"Accuracy\")",
            "output.Accuracy = m_max(0, m_floor(base * (1 + inc / 100) * more))",
        ] {
            body.push_str(source_line(&offence, prefix));
            body.push('\n');
        }
        if receiving {
            let defence = SPARK_DEFENCE.replace("\r\n", "\n");
            body.push_str(section(
                &defence,
                "\t\tlocal resourceList = {",
                "\n\t\toutput[\"Gear:Ward\"]",
            ));
            body.push_str("\nplayerResistances(modDB,output); ");
        }
        body.push_str("return {output=output,conditions=condList} end; return function(input,warm) local last; for i=1,(warm and 200 or 1) do last=calculate(input) end; return last end");
        let calculate = lua
            .load(format!("local m_floor=math.floor; {body}"))
            .set_name("pinned-actor-attributes-inherent-resources")
            .eval()
            .unwrap();
        Self { oracle, calculate }
    }
    fn calculate(
        &self,
        data: &CompiledGameData,
        level: u32,
        quests: ActorQuestSelection,
        character: &CharacterInput,
        layers: &[Vec<Record>],
    ) -> Table {
        self.calculate_impl(data, level, quests, character, layers, None, &[])
    }
    pub(super) fn calculate_receiving(
        &self,
        data: &CompiledGameData,
        level: u32,
        quests: ActorQuestSelection,
        scenario: poe_optimizer_engine::actor::ReceivingScenario,
        character: &CharacterInput,
        layers: &[Vec<Record>],
    ) -> Table {
        self.calculate_impl(data, level, quests, character, layers, Some(scenario), &[])
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn calculate_receiving_with_armour(
        &self,
        data: &CompiledGameData,
        level: u32,
        quests: ActorQuestSelection,
        scenario: poe_optimizer_engine::actor::ReceivingScenario,
        character: &CharacterInput,
        layers: &[Vec<Record>],
        armour: &[(&str, poe_optimizer_engine::armour::ArmourStats)],
    ) -> Table {
        self.calculate_impl(
            data,
            level,
            quests,
            character,
            layers,
            Some(scenario),
            armour,
        )
    }
    #[allow(clippy::too_many_arguments)]
    fn calculate_impl(
        &self,
        data: &CompiledGameData,
        level: u32,
        quests: ActorQuestSelection,
        character: &CharacterInput,
        layers: &[Vec<Record>],
        receiving: Option<poe_optimizer_engine::actor::ReceivingScenario>,
        armour: &[(&str, poe_optimizer_engine::armour::ArmourStats)],
    ) -> Table {
        let lua = &self.oracle.lua;
        let input = lua.create_table().unwrap();
        input.set("level", level).unwrap();
        let items = lua.create_table().unwrap();
        for (slot, stats) in armour {
            let item = lua.create_table().unwrap();
            let values = lua.create_table().unwrap();
            for (name, value) in [
                ("Armour", stats.armour),
                ("Evasion", stats.evasion),
                ("EnergyShield", stats.energy_shield),
            ] {
                values.set(name, value).unwrap();
            }
            item.set("armourData", values).unwrap();
            items.set(*slot, item).unwrap();
        }
        input.set("armour_items", items).unwrap();
        input
            .set("receiving_penalty", receiving.map(|v| v.resistance_penalty))
            .unwrap();
        let attrs = lua.create_table().unwrap();
        for (name, value) in [
            ("Str", character.attributes.strength),
            ("Dex", character.attributes.dexterity),
            ("Int", character.attributes.intelligence),
        ] {
            attrs.set(name, value).unwrap();
        }
        input.set("attributes", attrs).unwrap();
        let raw_layers = lua.create_table().unwrap();
        for layer in layers {
            raw_layers.push(record_table(lua, layer)).unwrap();
        }
        input.set("layers", raw_layers).unwrap();
        let package = data.snapshot().package();
        let mut records = vec![];
        for (enabled, stat, op, value) in [
            (
                quests.candlemass,
                Stat::Life,
                Op::Base,
                package.quests.flat_life,
            ),
            (
                quests.molten_shrine,
                Stat::Life,
                Op::Increased,
                package.quests.life_increased,
            ),
            (
                quests.silent_hall,
                Stat::Mana,
                Op::Increased,
                package.quests.mana_increased,
            ),
        ] {
            if enabled {
                records.push(numeric(stat, op, value));
            }
        }
        for (enabled, quest) in quests.spirit.into_iter().zip(&package.actor.spirit_quests) {
            if enabled {
                records.extend(quest.modifiers.clone());
            }
        }
        if let Some(scenario) = receiving {
            for (enabled, stat) in scenario.resistance_quests.into_iter().zip([
                Stat::FireResist,
                Stat::ColdResist,
                Stat::LightningResist,
            ]) {
                if enabled {
                    records.push(numeric(stat, Op::Base, package.quests.elemental_resistance));
                }
            }
            let lua_data: Table = lua.globals().get("data").unwrap();
            let misc: Table = lua_data.get("misc").unwrap();
            misc.set("ResistFloor", package.defence.resistance_floor)
                .unwrap();
            misc.set("MaxResistCap", package.defence.resistance_maximum_cap)
                .unwrap();
            let constants: Table = lua_data.get("characterConstants").unwrap();
            constants
                .set(
                    "base_maximum_all_resistances_%",
                    package.defence.player_resistance_cap,
                )
                .unwrap();
            constants
                .set("base_evasion_rating", package.character.base_evasion)
                .unwrap();
        }
        input.set("quests", record_table(lua, &records)).unwrap();
        // Precision is an explicit source data input, including injected More entries.
        let precision = lua.create_table().unwrap();
        for (name, entries) in &package.actor.high_precision_mods {
            let values = lua.create_table().unwrap();
            for (op, value) in entries {
                values.set(op.upstream_name(), *value).unwrap();
            }
            precision.set(name.as_str(), values).unwrap();
        }
        lua.globals()
            .get::<Table>("data")
            .unwrap()
            .set("highPrecisionMods", precision)
            .unwrap();
        self.calculate.call((input, self.oracle.warm)).unwrap()
    }
}
fn record_table(lua: &Lua, records: &[Record]) -> Table {
    let rows = lua.create_table().unwrap();
    for record in records {
        let row = lua.create_table().unwrap();
        row.set("name", record.stat.upstream_name()).unwrap();
        match record.effect {
            Effect::Numeric { operation, value } => {
                row.set("type", operation.upstream_name()).unwrap();
                row.set("value", value).unwrap();
            }
            Effect::Flag { value } => {
                row.set("type", "FLAG").unwrap();
                row.set("value", value).unwrap();
            }
        }
        row.set("source", record.source.as_deref()).unwrap();
        row.set("flags", record.flags as f64).unwrap();
        row.set("keywordFlags", record.keyword_flags as f64)
            .unwrap();
        for source in &record.tags {
            let tag = lua.create_table().unwrap();
            match source {
                Tag::Global => tag.set("type", "Global").unwrap(),
                Tag::Condition { variables, negated } => {
                    tag.set("type", "Condition").unwrap();
                    tag.set(
                        "varList",
                        lua.create_sequence_from(
                            variables.iter().map(|condition| condition.upstream_name()),
                        )
                        .unwrap(),
                    )
                    .unwrap();
                    tag.set("neg", *negated).unwrap();
                }
            }
            row.push(tag).unwrap();
        }
        rows.push(row).unwrap();
    }
    rows
}
fn compare(actual: ActorResourceOutput, result: Table) {
    let expected: Table = result.get("output").unwrap();
    for (name, value) in [
        ("Str", actual.attributes.strength),
        ("Dex", actual.attributes.dexterity),
        ("Int", actual.attributes.intelligence),
        ("LowestAttribute", actual.lowest_attribute),
        ("TotalAttr", actual.total_attributes),
        ("Life", actual.life),
        ("Mana", actual.mana),
        ("Spirit", actual.spirit),
        ("Accuracy", actual.accuracy),
        ("LowLifePercentage", actual.low_life_percentage),
        ("FullLifePercentage", actual.full_life_percentage),
        (
            "LowestOfMaximumLifeAndMaximumMana",
            actual.lowest_of_maximum_life_and_maximum_mana,
        ),
    ] {
        assert_number(Some(value), Some(expected.get(name).unwrap()));
    }
    for (name, value) in [
        ("LifeHasOverride", actual.life_has_override),
        ("ManaHasOverride", actual.mana_has_override),
        ("SpiritHasOverride", actual.spirit_has_override),
        ("ChaosInoculation", actual.chaos_inoculation),
    ] {
        assert_eq!(
            value,
            expected.get::<Option<bool>>(name).unwrap().unwrap_or(false),
            "{name}"
        );
    }
    let conditions: Table = result.get("conditions").unwrap();
    for (name, value) in actual.conditions() {
        assert_eq!(value, conditions.get::<bool>(name).unwrap(), "{name}");
    }
    assert_eq!(
        actual.full_life_from_chaos_inoculation,
        conditions
            .get::<Option<bool>>("FullLife")
            .unwrap()
            .unwrap_or(false)
    );
}
fn run(
    oracle: &ActorOracle,
    data: &CompiledGameData,
    level: u32,
    quests: ActorQuestSelection,
    character: &CharacterInput,
    layers: &[Vec<Record>],
) -> ActorResourceOutput {
    let actual = data
        .prepare_actor_resources(level, quests, character, layers)
        .unwrap()
        .values();
    let programs: Vec<Vec<_>> = layers
        .iter()
        .map(|records| {
            records
                .chunks(2)
                .map(|chunk| data.compile_actor_modifiers(chunk).unwrap())
                .collect()
        })
        .collect();
    let references: Vec<Vec<_>> = programs
        .iter()
        .map(|programs| programs.iter().collect())
        .collect();
    let compiled_layers: Vec<_> = references
        .iter()
        .map(|programs| poe_optimizer_engine::actor::ActorModifierLayer { programs })
        .collect();
    let compiled = data
        .evaluate_actor_resources(
            level,
            quests,
            character,
            &compiled_layers,
            &mut poe_optimizer_engine::actor::ActorScratch::default(),
        )
        .unwrap()
        .values();
    assert_eq!(
        compiled, actual,
        "compiled source fragments vs complete DB preparation"
    );
    let source = oracle.calculate(data, level, quests, character, layers);
    compare(actual, source.clone());
    compare(compiled, source);
    actual
}
#[test]
fn shared_base_level_class_and_quest_records_match_actual_actor_source_in_both_modes() {
    let data = CompiledGameData::bundled().unwrap();
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for level in [1, 60, 100] {
            for attributes in [
                CharacterAttributes::default(),
                CharacterAttributes {
                    strength: 7.0,
                    dexterity: 7.0,
                    intelligence: 7.0,
                },
                CharacterAttributes {
                    strength: 11.0,
                    dexterity: 7.0,
                    intelligence: 7.0,
                },
            ] {
                let character = CharacterInput {
                    attributes,
                    ..CharacterInput::default()
                };
                for bits in 0..8 {
                    let quests = ActorQuestSelection {
                        candlemass: bits & 1 != 0,
                        molten_shrine: bits & 2 != 0,
                        silent_hall: bits & 4 != 0,
                        spirit: std::array::from_fn(|i| bits & (1 << i) == 0),
                    };
                    run(&oracle, &data, level, quests, &character, &[]);
                }
            }
        }
    }
}
#[test]
fn numeric_actor_targets_follow_original_base_inc_more_override_and_rounding_semantics() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for stat in [
            Stat::Str,
            Stat::Dex,
            Stat::Int,
            Stat::Life,
            Stat::Mana,
            Stat::Spirit,
            Stat::Accuracy,
        ] {
            for op in [Op::Base, Op::Increased, Op::More, Op::Override] {
                for value in [
                    -101.0, -100.0, -99.5, -1.0, -0.5, 0.0, 0.0049, 0.005, 0.495, 0.5, 1.0, 7.995,
                    100.0, 1000.0,
                ] {
                    run(
                        &oracle,
                        &data,
                        60,
                        quests,
                        &character,
                        &[vec![numeric(stat, op, value)]],
                    );
                }
            }
        }
    }
}
#[test]
fn attribute_comparison_conditions_execute_exactly_two_passes_then_inherent_bonuses() {
    let data = CompiledGameData::bundled().unwrap();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        let character = CharacterInput {
            attributes: CharacterAttributes {
                strength: 20.0,
                dexterity: 10.0,
                intelligence: 10.0,
            },
            ..CharacterInput::default()
        };
        let flipping = vec![
            conditional(
                numeric(Stat::Str, Op::Base, -15.0),
                vec![AC::StrHigherThanDex],
                false,
            ),
            conditional(
                flag(Stat::DoubledInherentAttributeBonuses, true),
                vec![AC::TwoHighestAttributesEqual],
                false,
            ),
        ];
        assert_eq!(
            run(&oracle, &data, 60, quests, &character, &[flipping])
                .attributes
                .strength,
            5.0
        );
        for condition in [
            AC::TwoHighestAttributesEqual,
            AC::DexHigherThanInt,
            AC::StrHigherThanInt,
            AC::IntHigherThanDex,
            AC::StrHigherThanDex,
            AC::IntHigherThanStr,
            AC::DexHigherThanStr,
            AC::StrHighestAttribute,
            AC::IntHighestAttribute,
            AC::DexHighestAttribute,
            AC::IntSingleHighestAttribute,
            AC::DexSingleHighestAttribute,
        ] {
            for negated in [false, true] {
                for attrs in [(7.0, 7.0, 7.0), (9.0, 8.0, 7.0), (7.0, 8.0, 9.0)] {
                    let character = CharacterInput {
                        attributes: CharacterAttributes {
                            strength: attrs.0,
                            dexterity: attrs.1,
                            intelligence: attrs.2,
                        },
                        ..CharacterInput::default()
                    };
                    let layers = vec![vec![
                        conditional(
                            numeric(Stat::Dex, Op::Increased, 20.0),
                            vec![condition],
                            negated,
                        ),
                        conditional(
                            numeric(Stat::Life, Op::Override, 0.0),
                            vec![condition, AC::StrHighestAttribute],
                            negated,
                        ),
                    ]];
                    run(&oracle, &data, 60, quests, &character, &layers);
                }
            }
        }
    }
}
#[test]
fn all_inherent_bonus_flags_zero_dex_override_and_parent_layers_match_source() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for stat in [
            Stat::NoAttributeBonuses,
            Stat::DoubledInherentAttributeBonuses,
            Stat::NoStrengthAttributeBonuses,
            Stat::NoStrBonusToLife,
            Stat::HalvesLifeFromStrength,
            Stat::NoDexterityAttributeBonuses,
            Stat::NoDexBonusToAccuracy,
            Stat::NoIntelligenceAttributeBonuses,
            Stat::NoIntBonusToMana,
            Stat::ChaosInoculation,
        ] {
            for enabled in [false, true] {
                run(
                    &oracle,
                    &data,
                    60,
                    quests,
                    &character,
                    &[
                        vec![
                            flag(stat, false),
                            numeric(Stat::DexAccBonusOverride, Op::Override, 0.0),
                        ],
                        vec![
                            flag(stat, enabled),
                            numeric(Stat::DexAccBonusOverride, Op::Override, 17.5),
                        ],
                    ],
                );
            }
        }
        for value in [-2.5, 0.0, 0.5, 1.0, 7.0, 10.0] {
            run(
                &oracle,
                &data,
                60,
                quests,
                &character,
                &[vec![numeric(
                    Stat::DexAccBonusOverride,
                    Op::Override,
                    value,
                )]],
            );
        }
    }
}
#[test]
fn complete_pool_function_preserves_donor_extra_total_override_and_chaos_order() {
    let data = CompiledGameData::bundled().unwrap();
    let character = data.default_mace_character();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for conversion in [
            Stat::LifeConvertToEnergyShield,
            Stat::LifeConvertToArmour,
            Stat::LifeConvertToEvasion,
            Stat::ManaConvertToEnergyShield,
            Stat::ManaConvertToArmour,
            Stat::ManaConvertToEvasion,
            Stat::SpiritConvertToEnergyShield,
            Stat::SpiritConvertToArmour,
            Stat::SpiritConvertToEvasion,
        ] {
            for value in [-150.0, 0.0, 99.5, 100.0, 150.0] {
                run(
                    &oracle,
                    &data,
                    60,
                    quests,
                    &character,
                    &[vec![
                        numeric(conversion, Op::Base, value),
                        numeric(Stat::ExtraLife, Op::Base, 3.125),
                        numeric(Stat::ExtraMana, Op::Base, 5.25),
                        numeric(Stat::ExtraSpirit, Op::Base, 8.75),
                        numeric(Stat::LifeTotal, Op::Base, -2.5),
                        numeric(Stat::ManaTotal, Op::Base, 2.5),
                        numeric(Stat::SpiritTotal, Op::Base, 0.5),
                        numeric(Stat::Life, Op::More, 17.25),
                        numeric(Stat::Mana, Op::Increased, 13.5),
                        numeric(Stat::Spirit, Op::More, -11.25),
                    ]],
                );
            }
        }
        for value in [-17.5, 0.0, 0.49, 0.5, 1.5] {
            for ci in [false, true] {
                let layers = vec![
                    vec![
                        numeric(Stat::Life, Op::Override, value),
                        numeric(Stat::Mana, Op::Override, value),
                        numeric(Stat::Spirit, Op::Override, value),
                        flag(Stat::ChaosInoculation, ci),
                        numeric(Stat::LowLifePercentage, Op::Base, value),
                        numeric(Stat::FullLifePercentage, Op::Base, value),
                    ],
                    vec![numeric(Stat::Life, Op::Override, 999.0)],
                ];
                run(&oracle, &data, 60, quests, &character, &layers);
            }
        }
    }
}
#[test]
fn injected_more_precision_is_used_for_attributes_and_resource_parent_products() {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    package
        .actor
        .high_precision_mods
        .insert("Life".into(), BTreeMap::from([(Op::More, 4)]));
    package
        .actor
        .high_precision_mods
        .insert("Str".into(), BTreeMap::from([(Op::More, 3)]));
    package.refresh_section_digests().unwrap();
    let snapshot = game_data::GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &game_data::TrustPolicy::AllowCustom,
        &game_data::LoadLimits::default(),
    )
    .unwrap();
    let data = CompiledGameData::compile(Arc::new(snapshot)).unwrap();
    let character = CharacterInput {
        attributes: CharacterAttributes {
            strength: 500000.0,
            dexterity: 7.0,
            intelligence: 7.0,
        },
        ..CharacterInput::default()
    };
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for value in [-1.234, 0.005, 13.333, 17.25] {
            let layers = vec![
                vec![
                    numeric(Stat::Str, Op::More, value),
                    numeric(Stat::Life, Op::More, value),
                    conditional(
                        numeric(Stat::Life, Op::More, 7.25),
                        vec![AC::IntSingleHighestAttribute],
                        false,
                    ),
                ],
                vec![numeric(Stat::Life, Op::More, 13.25)],
            ];
            let output = run(&oracle, &data, 60, quests, &character, &layers);
            if value == 13.333 {
                let reviewed = CompiledGameData::bundled().unwrap();
                let old = reviewed
                    .prepare_actor_resources(60, quests, &character, &layers)
                    .unwrap()
                    .values();
                assert_ne!(output.attributes.strength, old.attributes.strength);
                assert_ne!(output.life, old.life);
            }
        }
    }
}

#[test]
fn reused_source_programs_follow_actual_source_when_equipment_and_passives_change() {
    use poe_optimizer_engine::actor::{ActorModifierLayer, ActorScratch};
    let data = CompiledGameData::bundled().unwrap();
    let config = vec![
        conditional(
            numeric(Stat::Str, Op::Base, -30.0),
            vec![AC::StrHigherThanInt],
            false,
        ),
        numeric(Stat::Life, Op::More, 13.0),
    ];
    let gear = [
        vec![],
        vec![
            numeric(Stat::Str, Op::Base, 20.0),
            numeric(Stat::Life, Op::More, 13.0),
        ],
        vec![
            numeric(Stat::Dex, Op::Base, 20.0),
            numeric(Stat::Mana, Op::Override, 0.0),
        ],
    ];
    let trees = [
        vec![],
        vec![numeric(Stat::Str, Op::Base, 10.0)],
        vec![
            numeric(Stat::Int, Op::Base, 20.0),
            numeric(Stat::Life, Op::More, 17.0),
        ],
    ];
    let config_program = data.compile_actor_modifiers(&config).unwrap();
    let gear_programs = gear
        .iter()
        .map(|records| data.compile_actor_modifiers(records).unwrap())
        .collect::<Vec<_>>();
    let tree_programs = trees
        .iter()
        .map(|records| data.compile_actor_modifiers(records).unwrap())
        .collect::<Vec<_>>();
    let quests = data.actor_quest_selection(SparkQuestRewards::default());
    let mut scratch = ActorScratch::default();
    for warm in [false, true] {
        let oracle = ActorOracle::new(warm);
        for character in [
            data.default_mace_character(),
            data.default_spark_character(),
        ] {
            for (gear_index, gear_records) in gear.iter().enumerate() {
                for (tree_index, tree_records) in trees.iter().enumerate() {
                    let sources = [
                        &config_program,
                        &gear_programs[gear_index],
                        &tree_programs[tree_index],
                    ];
                    let actual = data
                        .evaluate_actor_resources(
                            60,
                            quests,
                            &character,
                            &[ActorModifierLayer { programs: &sources }],
                            &mut scratch,
                        )
                        .unwrap()
                        .values();
                    let full_records = [
                        config.as_slice(),
                        gear_records.as_slice(),
                        tree_records.as_slice(),
                    ]
                    .concat();
                    let full = data
                        .prepare_actor_resources(
                            60,
                            quests,
                            &character,
                            std::slice::from_ref(&full_records),
                        )
                        .unwrap()
                        .values();
                    assert_eq!(actual, full);
                    compare(
                        actual,
                        oracle.calculate(&data, 60, quests, &character, &[full_records]),
                    );
                }
            }
        }
    }
}
