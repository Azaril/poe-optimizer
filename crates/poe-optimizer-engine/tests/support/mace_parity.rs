//! Selected-branch oracle executes original pinned item, modifier, resource and
//! offence functions. Full-build independent fixtures are a separate parity gate.
use super::*;
use poe_optimizer_engine::mace::{self, MaceInput, MaceOutput, MaceWeapon};
const BASES: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Data/Bases/mace.lua");
const SKILLS: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Data/Skills/other.lua");
const SUPPORTS: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Data/Skills/sup_str.lua");
const STAT_MAP: &str =
    include_str!("../../../../vendor/path-of-building-poe2/src/Data/SkillStatMap.lua");
const ITEM: &str = include_str!("../../../../vendor/path-of-building-poe2/src/Classes/Item.lua");

fn source_checks() {
    for record in mace::SOURCE_FILES {
        let source = match record.path {
            "src/Modules/ModParser.lua" => character_parity::PARSER,
            "src/Data/Misc.lua" => SPARK_MISC,
            "src/Data/QuestRewards.lua" => SPARK_QUESTS,
            "src/Data/Bases/mace.lua" => BASES,
            "src/Data/Skills/other.lua" => SKILLS,
            "src/Data/Skills/sup_str.lua" => SUPPORTS,
            "src/Data/SkillStatMap.lua" => STAT_MAP,
            "src/Classes/Item.lua" => ITEM,
            "src/TreeData/0_5/tree.lua" => SPARK_TREE,
            "src/Modules/CalcSetup.lua" => SPARK_SETUP,
            "src/Modules/CalcPerform.lua" => SPARK_PERFORM,
            "src/Modules/CalcOffence.lua" => SPARK_OFFENCE,
            "src/Modules/CalcDefence.lua" => SPARK_DEFENCE,
            "src/Modules/ConfigOptions.lua" => SPARK_CONFIG,
            "src/Modules/CalcTools.lua" => SPARK_TOOLS,
            "src/Modules/Data.lua" => DATA,
            "src/Modules/Common.lua" => COMMON,
            path => panic!("Unverified native Mace source: {path}"),
        };
        assert_eq!(
            format!("{:x}", Sha256::digest(source.replace("\r\n", "\n"))),
            record.sha256,
            "{} changed",
            record.path
        );
    }
}

struct MaceOracle {
    oracle: Oracle,
    calculate: Function,
}
impl MaceOracle {
    fn new(warm: bool) -> Self {
        source_checks();
        // Reuse only source/module setup, not Spark's arithmetic or expected output.
        let spark = SparkOracle::new(warm);
        let oracle = spark.oracle;
        let lua = &oracle.lua;
        let tree: Table = lua.load(SPARK_TREE).eval().unwrap();
        let classes: Table = tree.get("classes").unwrap();
        lua.globals()
            .set("maceClass", classes.get::<Table>(mace::CLASS_ID).unwrap())
            .unwrap();
        let bases = lua.create_table().unwrap();
        lua.load(BASES)
            .eval::<Function>()
            .unwrap()
            .call::<()>(bases.clone())
            .unwrap();
        lua.globals().set("maceBases", bases).unwrap();
        lua.globals()
            .set(
                "mod",
                lua.globals()
                    .get::<Table>("modLib")
                    .unwrap()
                    .get::<Function>("createMod")
                    .unwrap(),
            )
            .unwrap();
        lua.load(section(
            &SKILLS.replace("\r\n", "\n"),
            "skills[\"Melee1HMacePlayer\"] = {",
            "\nskills[\"Melee2HMacePlayer\"]",
        ))
        .exec()
        .unwrap();
        lua.load(section(
            &SUPPORTS.replace("\r\n", "\n"),
            "skills[\"SupportBrutalityPlayer\"] = {",
            "\nskills[\"SupportBrutalityPlayerTwo\"]",
        ))
        .exec()
        .unwrap();
        lua.load(format!("local flag=function(name) return modLib.createMod(name,'FLAG',true,'Support') end; maceSupportFlags = {{ {} }}", section(&STAT_MAP.replace("\r\n", "\n"), "[\"deal_no_elemental_damage\"] = {", "[\"all_damage_can_ignite\"]"))).exec().unwrap();

        let item = ITEM.replace("\r\n", "\n");
        let mut weapon_function = String::from(
            "local t_remove=table.remove; local m_floor=math.floor; local dmgTypeList={'Physical','Lightning','Cold','Fire','Chaos'}; ",
        );
        weapon_function.push_str(section(
            &item,
            "local function calcLocal(",
            "-- Build list of modifiers in a given slot number",
        ));
        weapon_function.push_str("return function(input) local self={base=maceBases[input.weapon],quality=input.quality,name=input.weapon,weaponData={}}; local slotNum=1; local modList={}; ");
        weapon_function.push_str(section(
            &item,
            "\tif self.base.weapon then\n\t\tlocal weaponData",
            "\n\t\tfor _, value in ipairs(modList:List(nil, \"WeaponData\"))",
        ));
        weapon_function.push_str("\nend; return self.weaponData[1] end");
        lua.globals()
            .set(
                "maceWeapon",
                lua.load(weapon_function)
                    .set_name("pinned-Mace-item-weapon-block")
                    .eval::<Function>()
                    .unwrap(),
            )
            .unwrap();
        let setup = SPARK_SETUP.replace("\r\n", "\n");
        let perform = SPARK_PERFORM.replace("\r\n", "\n");
        let offence = SPARK_OFFENCE.replace("\r\n", "\n");
        let defence = SPARK_DEFENCE.replace("\r\n", "\n");
        // The unmodified offence prefix contains calcDamage and actual conversion
        // helpers, along with their captured bit flags/modifier-name construction.
        let mut body = section(
            &offence,
            "-- Path of Building",
            "---Calculates the area percentage",
        )
        .to_owned();
        body.push_str("\nreturn function(input) local m_modf=math.modf; local modDB=new('ModDB'):ModDB(); local output={Str=input.character.strength,Dex=input.character.dexterity,Int=input.character.intelligence}; modDB.actor={output=output}; modDB.multipliers.Level=input.level; for _, mod in ipairs(input.character.mods) do modDB:AddMod(copyTable(mod)) end; local env={mode_effective=true,modDB=modDB,partyMembers={modDB=modDB},configInput={resistancePenalty=input.penalty,enemyFireResist=input.resistance}}; ");
        body.push_str(section(
            &setup,
            "\t\tmodDB:NewMod(\"Life\", \"BASE\", data.characterConstants",
            "\t\tmodDB:NewMod(\"ManaRegen\"",
        ));
        for prefix in [
            "modDB:NewMod(\"Accuracy\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"CritMultiplier\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"CritChanceCap\",",
            "modDB:NewMod(\"FireResist\",",
            "modDB:NewMod(\"ColdResist\",",
            "modDB:NewMod(\"LightningResist\",",
            "modDB:NewMod(\"ChaosResist\",",
            "modDB:NewMod(\"FireResistMax\",",
            "modDB:NewMod(\"ColdResistMax\",",
            "modDB:NewMod(\"LightningResistMax\",",
            "modDB:NewMod(\"ChaosResistMax\",",
        ] {
            body.push_str(source_line(&setup, prefix));
            body.push('\n');
        }
        body.push_str("for field, quest in pairs(sparkQuests) do if input.quests[field] then modDB:NewMod(quest.name,quest.kind,quest.value,'Quest') end end\n");
        body.push_str(section(
            &perform,
            "\t-- Add attribute bonuses\n",
            "\t-- Calculate Presence / Surrounded",
        ));
        body.push_str("sparkCalcs.doActorLifeManaSpirit({modDB=modDB,output=output},true); characterDefences(modDB,output,input.level); local resistTypeList={'Fire','Cold','Lightning','Chaos'}; ");
        body.push_str(section(
            &defence,
            "\tfor _, elem in ipairs(resistTypeList) do\n\t\tlocal min, max, total, dotTotal",
            "\n\t\toutput[elem..\"ResistOverCap\"]",
        ));
        body.push_str("\nend; local source=maceWeapon(input); output.Weapon=source; local enemyDB=new('ModDB'):ModDB(); enemyDB:NewMod('Armour','BASE',input.armour,'Config'); enemyDB:NewMod('Evasion','BASE',input.evasion,'Config'); enemyDB:NewMod('FireResist','BASE',input.resistance,'Config'); local skillModList=modDB; local cfg={flags=OR64(ModFlag.Attack,ModFlag.Melee,ModFlag.Hit)}; local skillCfg=cfg; local skillData={}; local activeSkill={skillModList=skillModList,activeEffect={grantedEffect=skills.Melee1HMacePlayer,grantedEffectLevel=skills.Melee1HMacePlayer.levels[1]},conversionTable={},gainTable={}}; local globalOutput={ActionSpeedMod=1}; local skillFlags={hit=true}; local isAttack=true; ");
        body.push_str("if input.brutality then local support=skills.SupportBrutalityPlayer.statSets[1]; for _, stat in ipairs(support.constantStats) do for _, mod in ipairs(support.statMap[stat[1]]) do local resolved=copyTable(mod); resolved.value=stat[2]; modDB:AddMod(resolved) end end; for _, name in ipairs(support.stats) do for _, flag in ipairs(maceSupportFlags[name]) do modDB:AddMod(copyTable(flag)) end end end; ");
        body.push_str(section(
            &offence,
            "\tlocal function calcResistForType(",
            "\n\tlocal function runSkillFunc(",
        ));
        body.push_str(section(
            &offence,
            "\t\t-- Calculate hit chance\n",
            "\n\t\tif breakdown then\n\t\t\tbreakdown.Accuracy",
        ));
        body.push_str(section(
            &offence,
            "\t\t\tlocal enemyEvasion = m_max(round(calcLib.val(enemyDB, \"Evasion\"))",
            "\n\t\t\t-- Accounting for mods",
        ));
        body.push_str("\noutput.EffectiveEvasion=enemyEvasion; output.enemyBlockChance=0; ");
        body.push_str(source_line(
            &offence,
            "output.HitChance = output.AccuracyHitChance *",
        ));
        body.push_str("\nlocal baseTime; ");
        body.push_str(source_line(&offence, "baseTime = (1 / ( source.AttackRate"));
        body.push('\n');
        body.push_str(source_line(
            section(&offence, "\t\t\tif skillModList:Sum(\"BASE\", skillCfg, \"Multiplier:TraumaStacks\") == 0 then", "\n\t\t\tif skillFlags.warcry then"),
            "local inc = skillModList:Sum(\"INC\", cfg, \"Speed\")",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.Speed = 1 / (baseTime / round(",
        ));
        body.push_str("\nglobalOutput.Speed=output.Speed; local baseCrit=source.CritChance; base,inc,more=0,0,1; ");
        body.push_str(source_line(
            &offence,
            "output.CritChance = round((baseCrit + base)",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.CritChance = m_min(output.CritChance, skillModList:Override",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "output.CritChance = output.CritChance * output.AccuracyHitChance",
        ));
        body.push('\n');
        body.push_str(section(
            &offence,
            "\t\t\t\tlocal extraDamage = skillModList:Sum(\"BASE\", cfg, \"CritMultiplier\")",
            "\n\t\t\t\toutput.CritMultiplier = 1 + m_max(0, extraDamage)",
        ));
        body.push_str(source_line(
            &offence,
            "output.CritMultiplier = 1 + m_max(0, extraDamage)",
        ));
        body.push('\n');
        body.push_str(section(
            &offence,
            "\t-- Cache global damage disabling flags\n",
            "\n\t-- Calculate damage conversion percentages",
        ));
        // Explicit resolved identity conversion/gain tables: no converted/added damage.
        body.push_str("for _, damageType in ipairs(dmgTypeList) do activeSkill.conversionTable[damageType]={mult=1}; activeSkill.gainTable[damageType]={}; for _, other in ipairs(dmgTypeList) do activeSkill.conversionTable[damageType][other]=0; activeSkill.gainTable[damageType][other]=0 end end; ");
        body.push_str(section(
            &offence,
            "\t\t-- Calculate base hit damage\n",
            "\n\t\t\tif breakdown then\n\t\t\t\tif (baseMin",
        ));
        body.push_str("\nend; local totalHitAvg,totalCritAvg=0,0; for pass=1,2 do for _, damageType in ipairs(dmgTypeList) do local damageTypeHitMin,damageTypeHitMax,damageTypeHitAvg=0,0,0; if canDeal[damageType] then damageTypeHitMin,damageTypeHitMax=calcDamage(activeSkill,output,cfg,nil,damageType,0); local allMult=1; ");
        body.push_str(section(
            &offence,
            "\t\t\t\t\tif pass == 1 then\n\t\t\t\t\t\t-- Apply crit multiplier",
            "\n\t\t\t\t\tif skillModList:Flag(skillCfg, \"LuckyHits\")",
        ));
        body.push_str("\nlocal damageTypeHitAvgNotLucky,damageTypeHitAvgLucky; local damageTypeLuckyChance=0; ");
        for prefix in [
            "damageTypeHitAvgNotLucky = (damageTypeHitMin / 2",
            "damageTypeHitAvgLucky = (damageTypeHitMin / 3",
            "damageTypeHitAvg = damageTypeHitAvgNotLucky *",
        ] {
            body.push_str(source_line(&offence, prefix));
            body.push('\n');
        }
        body.push_str("local resist; if damageType=='Physical' then local enemyArmourMin=0;\n");
        body.push_str(section(
            &offence,
            "\t\t\t\t\t\t\tlocal enemyArmour = enemyDB:Override",
            "\n\t\t\t\t\t\t\tlocal ChanceToIgnoreEnemyPhysicalDamageReduction",
        ));
        body.push_str(source_line(
            &offence,
            "resist = m_min(m_max(-data.misc.NegArmourDmgBonusCap,",
        ));
        body.push_str("\nelse resist=calcResistForType(damageType,cfg) end; local effMult=1; local effectiveResist=resist; ");
        body.push_str(source_line(
            &offence,
            "effMult = effMult * (1 - effectiveResist / 100)",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "damageTypeHitAvg = damageTypeHitAvg * effMult",
        ));
        body.push_str("\nend; if pass==1 then\n");
        body.push_str(source_line(
            &offence,
            "totalCritAvg = totalCritAvg + damageTypeHitAvg",
        ));
        body.push_str("\nelse\n");
        body.push_str(source_line(
            &offence,
            "output[damageType..\"HitAverage\"] = damageTypeHitAvg",
        ));
        body.push('\n');
        body.push_str(source_line(
            &offence,
            "totalHitAvg = totalHitAvg + damageTypeHitAvg",
        ));
        body.push_str("\nend; end; end; output.DpsMultiplier=1; local quantityMultiplier=1; ");
        for prefix in [
            "output.AverageHit = totalHitAvg *",
            "output.AverageDamage = output.AverageHit * output.HitChance / 100",
            "output.TotalDPS = output.AverageDamage *",
        ] {
            body.push_str(source_line(&offence, prefix));
            body.push('\n');
        }
        body.push_str("output.EffectiveResist=calcResistForType('Fire',cfg); return output end");
        let calculate = lua
            .load(body)
            .set_name("pinned-Mace-closed-pipeline-sections")
            .eval()
            .unwrap();
        Self { oracle, calculate }
    }
    fn calculate(&self, input: &MaceInput) -> Table {
        self.calculate_with_character(input, &mace::default_character())
    }

    fn calculate_with_character(
        &self,
        input: &MaceInput,
        character: &poe_optimizer_engine::character::CharacterInput,
    ) -> Table {
        let lua = &self.oracle.lua;
        let table = lua.create_table().unwrap();
        table.set("level", input.character_level).unwrap();
        table
            .set("character", character_parity::input_table(lua, character))
            .unwrap();
        table.set("weapon", input.weapon.data().name).unwrap();
        table.set("quality", input.quality).unwrap();
        table.set("brutality", input.brutality).unwrap();
        table.set("penalty", input.resistance_penalty).unwrap();
        table.set("armour", input.enemy_armour).unwrap();
        table.set("evasion", input.enemy_evasion).unwrap();
        table
            .set("resistance", input.enemy_fire_resistance)
            .unwrap();
        let quests = lua.create_table().unwrap();
        for (name, enabled) in [
            ("candlemass", input.quests.candlemass),
            ("molten_shrine", input.quests.molten_shrine),
            ("silent_hall", input.quests.silent_hall),
            ("beira", input.quests.beira),
            ("garukhan", input.quests.garukhan),
            ("blackjaw", input.quests.blackjaw),
        ] {
            quests.set(name, enabled).unwrap();
        }
        table.set("quests", quests).unwrap();
        let wrapper:Function=lua.load("return function(f,input,warm) local result; for i=1,(warm and 200 or 1) do result=f(input) end; return result end").eval().unwrap();
        wrapper
            .call((self.calculate.clone(), table, self.oracle.warm))
            .unwrap()
    }
}

fn input() -> MaceInput {
    MaceInput {
        character_level: 60,
        weapon: MaceWeapon::WoodenClub,
        quality: 0,
        item_level: 1,
        brutality: false,
        resistance_penalty: -60.0,
        quests: SparkQuestRewards::default(),
        enemy_armour: 0.0,
        enemy_evasion: mace::monster_evasion(60).unwrap(),
        enemy_fire_resistance: 0.0,
    }
}
fn compare(output: MaceOutput, expected: Table) {
    for (name, actual) in [
        ("Str", output.strength),
        ("Dex", output.dexterity),
        ("Int", output.intelligence),
        ("Life", output.life),
        ("Mana", output.mana),
        ("EnergyShield", output.energy_shield),
        ("Armour", output.armour),
        ("Evasion", output.evasion),
        ("FireResist", output.fire_resistance),
        ("ColdResist", output.cold_resistance),
        ("LightningResist", output.lightning_resistance),
        ("ChaosResist", output.chaos_resistance),
        ("Accuracy", output.accuracy),
        ("HitChance", output.hit_chance),
        ("AverageHit", output.main_hand_average_hit),
        ("AverageDamage", output.average_damage),
        ("TotalDPS", output.hit_dps),
        ("Speed", output.attack_rate),
        ("CritChance", output.crit_chance),
        ("CritMultiplier", output.crit_multiplier),
        ("PhysicalHitAverage", output.physical_hit_average),
        ("FireHitAverage", output.fire_hit_average),
        ("EffectiveResist", output.effective_enemy_fire_resistance),
        ("EffectiveEvasion", output.effective_enemy_evasion),
    ] {
        let value: f64 = expected.get(name).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(
            (actual - value).abs() <= 1e-12 * value.abs().max(1.0),
            "{name}: native {actual} vs source {value}"
        );
    }
    let weapon: Table = expected.get("Weapon").unwrap();
    for (name, actual) in [
        ("PhysicalMin", output.weapon_physical_minimum),
        ("PhysicalMax", output.weapon_physical_maximum),
        ("FireMin", output.weapon_fire_minimum),
        ("FireMax", output.weapon_fire_maximum),
    ] {
        assert_eq!(
            actual,
            weapon.get::<Option<f64>>(name).unwrap().unwrap_or(0.0),
            "{name}"
        );
    }
}

#[test]
fn mace_matches_actual_item_resource_hit_crit_and_damage_source_in_interpreted_and_warmed_modes() {
    for warm in [false, true] {
        let oracle = MaceOracle::new(warm);
        for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
            for brutality in [false, true] {
                for level in [1, 2, 20, 60, 61, 100] {
                    for quality in 0..=20 {
                        let case = MaceInput {
                            character_level: level,
                            weapon,
                            brutality,
                            quality,
                            item_level: 100 - quality,
                            enemy_evasion: mace::monster_evasion(level).unwrap(),
                            ..input()
                        };
                        compare(mace::evaluate(&case).unwrap(), oracle.calculate(&case));
                    }
                }
                for armour in [0.0, 0.01, 80.0, 240.0, 1500.0, 1e9, f64::MAX] {
                    for evasion in [0.0, 24.0, 590.49, 590.5, 591.0, 1e6, f64::MAX] {
                        for resistance in [-200.0, -25.5, 0.0, 50.0, 80.0, 90.0, 200.0] {
                            let case = MaceInput {
                                weapon,
                                brutality,
                                quality: 20,
                                enemy_armour: armour,
                                enemy_evasion: evasion,
                                enemy_fire_resistance: resistance,
                                ..input()
                            };
                            compare(mace::evaluate(&case).unwrap(), oracle.calculate(&case));
                        }
                    }
                }
            }
        }
        // Mixed requests reuse exactly one Lua/native program and return to the original.
        for mask in [63u8, 0, 1, 2, 4, 8, 16, 32, 63] {
            let quests = SparkQuestRewards {
                candlemass: mask & 1 != 0,
                molten_shrine: mask & 2 != 0,
                silent_hall: mask & 4 != 0,
                beira: mask & 8 != 0,
                garukhan: mask & 16 != 0,
                blackjaw: mask & 32 != 0,
            };
            for penalty in [-60.0, -20.5, -0.25, 0.0] {
                let case = MaceInput {
                    quests,
                    resistance_penalty: penalty,
                    ..input()
                };
                compare(mace::evaluate(&case).unwrap(), oracle.calculate(&case));
            }
        }
    }
}

#[test]
fn mace_versioned_data_bounds_and_stateless_evaluation_are_explicit() {
    let oracle = MaceOracle::new(false);
    let lua = &oracle.oracle.lua;
    let data: Table = lua.globals().get("data").unwrap();
    let table: Table = data.get("monsterEvasionTable").unwrap();
    let armour: Table = data.get("monsterArmourTable").unwrap();
    for level in 1..=100 {
        assert_eq!(
            mace::monster_evasion(level).unwrap(),
            table.get::<f64>(level).unwrap()
        );
        assert_eq!(
            mace::monster_armour(level).unwrap(),
            armour.get::<f64>(level).unwrap()
        );
    }
    for level in [0, 101, u32::MAX] {
        assert!(mace::monster_evasion(level).is_err());
        assert!(mace::monster_armour(level).is_err());
    }
    let character: Table = lua.globals().get("maceClass").unwrap();
    assert_eq!(character.get::<String>("name").unwrap(), "Warrior");
    assert_eq!(
        character.get::<f64>("base_str").unwrap(),
        mace::data().strength
    );
    assert_eq!(
        character.get::<f64>("base_dex").unwrap(),
        mace::data().dexterity
    );
    assert_eq!(
        character.get::<f64>("base_int").unwrap(),
        mace::data().intelligence
    );
    let misc: Table = data.get("misc").unwrap();
    assert_eq!(
        misc.get::<f64>("EnemyPhysicalDamageReductionCap").unwrap(),
        mace::data().enemy_physical_reduction_cap
    );
    let bases: Table = lua.globals().get("maceBases").unwrap();
    for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
        let base: Table = bases.get(weapon.data().name).unwrap();
        let req: Table = base.get("req").unwrap();
        assert_eq!(
            req.get::<Option<u32>>("str").unwrap().unwrap_or(0),
            weapon.data().required_strength
        );
        assert_eq!(req.get::<Option<u32>>("level").unwrap().unwrap_or(0), 0);
    }
    let skills: Table = lua.globals().get("skills").unwrap();
    let mace_skill: Table = skills.get(mace::SKILL_ID).unwrap();
    let levels: Table = mace_skill.get("levels").unwrap();
    let first: Table = levels.get(1).unwrap();
    assert!(
        first
            .get::<Option<f64>>("baseMultiplier")
            .unwrap()
            .is_none()
    );
    let mut case = input();
    let expected = mace::evaluate(&case).unwrap();
    for level in [0, 101, u32::MAX] {
        case.character_level = level;
        assert!(mace::evaluate(&case).is_err());
    }
    case = input();
    for level in [0, 101, u32::MAX] {
        case.item_level = level;
        assert!(mace::evaluate(&case).is_err());
    }
    case = input();
    case.quality = 21;
    assert!(mace::evaluate(&case).is_err());
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -1.0] {
        case = MaceInput {
            enemy_armour: invalid,
            ..input()
        };
        assert!(mace::evaluate(&case).is_err());
        case = MaceInput {
            enemy_evasion: invalid,
            ..input()
        };
        assert!(mace::evaluate(&case).is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -200.01, 200.01] {
        case = MaceInput {
            enemy_fire_resistance: invalid,
            ..input()
        };
        assert!(mace::evaluate(&case).is_err());
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -60.01, 0.01] {
        case = MaceInput {
            resistance_penalty: invalid,
            ..input()
        };
        assert!(mace::evaluate(&case).is_err());
    }
    assert_eq!(mace::evaluate(&input()).unwrap(), expected);
    assert_eq!(
        mace::evaluate(&MaceInput {
            item_level: 100,
            ..input()
        })
        .unwrap(),
        expected
    );
    // Subtle regression: mitigation caps at monster75%, not player90%.
    let capped = mace::evaluate(&MaceInput {
        enemy_armour: f64::MAX,
        ..input()
    })
    .unwrap();
    assert_eq!(
        capped.physical_hit_average,
        expected.physical_hit_average * 0.25
    );
}

#[test]
fn all_class_entrances_match_actual_mace_source_with_armour_and_brutality() {
    for warm in [false, true] {
        let oracle = MaceOracle::new(warm);
        for entrance in character_parity::entrances(&oracle.oracle.lua) {
            for level in [1, 60, 100] {
                for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
                    for brutality in [false, true] {
                        let case = MaceInput {
                            character_level: level,
                            weapon,
                            quality: 20,
                            brutality,
                            enemy_armour: 100.0,
                            enemy_fire_resistance: 50.0,
                            ..input()
                        };
                        compare(
                            mace::evaluate_with_character(&case, &entrance.character).unwrap(),
                            oracle.calculate_with_character(&case, &entrance.character),
                        );
                    }
                }
            }
        }
        // Fractional boundaries cover the source's two-decimal speed multiplier
        // rounding and endpoint damage rounding before critical damage/armour.
        for value in [
            0.0,
            0.49,
            0.5,
            0.51,
            4.49,
            4.5,
            9.99,
            10.0,
            10.01,
            1_000_000.0,
        ] {
            let character = poe_optimizer_engine::character::CharacterInput {
                modifiers: poe_optimizer_engine::character::CharacterModifiers {
                    skill_speed_increased: value,
                    attack_damage_increased: value,
                    melee_damage_increased: value,
                    energy_shield_flat: value,
                    armour_flat: value,
                    evasion_flat: value,
                    ..Default::default()
                },
                ..mace::default_character()
            };
            for weapon in [MaceWeapon::WoodenClub, MaceWeapon::SmithingHammer] {
                let case = MaceInput {
                    weapon,
                    brutality: true,
                    enemy_armour: 100.0,
                    ..input()
                };
                compare(
                    mace::evaluate_with_character(&case, &character).unwrap(),
                    oracle.calculate_with_character(&case, &character),
                );
            }
        }
        let before = mace::evaluate(&input()).unwrap();
        let mut invalid = mace::default_character();
        invalid.modifiers.armour_flat = f64::NAN;
        assert!(mace::evaluate_with_character(&input(), &invalid).is_err());
        assert_eq!(mace::evaluate(&input()).unwrap(), before);
    }
}
