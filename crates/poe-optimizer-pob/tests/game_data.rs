//! Independent data oracle: package values are compared with evaluated pinned Lua
//! data/functions and parser output, never with native constants or golden metrics.
use mlua::{Function, Lua, LuaSerdeExt, Table};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
fn section<'a>(source: &'a str, begin: &str, end: &str) -> &'a str {
    let start = source
        .find(begin)
        .unwrap_or_else(|| panic!("Missing source anchor {begin}"));
    &source[start
        ..start
            + source[start..]
                .find(end)
                .unwrap_or_else(|| panic!("Missing source terminator {end}"))]
}
fn line<'a>(source: &'a str, prefix: &str) -> &'a str {
    let matches: Vec<_> = source
        .lines()
        .filter(|line| line.trim_start().starts_with(prefix))
        .collect();
    assert_eq!(matches.len(), 1, "Ambiguous source line {prefix}");
    matches[0]
}
struct Oracle {
    lua: Lua,
    sources: BTreeMap<String, String>,
}
impl Oracle {
    fn new(warm: bool) -> Self {
        let manifest: Value =
            serde_json::from_str(include_str!("../data/pob-source-manifest.json")).unwrap();
        assert_eq!(
            manifest["upstream_revision"],
            poe_optimizer_pob::source::UPSTREAM_REVISION
        );
        let paths = [
            "src/Modules/Common.lua",
            "src/Data/Global.lua",
            "src/Data/Misc.lua",
            "src/Modules/Data.lua",
            "src/Modules/ModTools.lua",
            "src/Modules/ModParser.lua",
            "src/Classes/ModStore.lua",
            "src/Classes/ModDB.lua",
            "src/Classes/ModList.lua",
            "src/Classes/PassiveTree.lua",
            "src/Data/Bases/amulet.lua",
            "src/Data/Bases/helmet.lua",
            "src/Modules/ItemTools.lua",
            "src/Data/ModScalability.lua",
            "src/Data/Bases/gloves.lua",
            "src/Data/Bases/boots.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcPerform.lua",
            "src/Modules/CalcDefence.lua",
            "src/Modules/CalcOffence.lua",
            "src/Modules/CalcTools.lua",
            "src/Modules/ConfigOptions.lua",
            "src/Classes/ConfigTab.lua",
            "src/Classes/Item.lua",
            "src/Classes/SkillsTab.lua",
            "src/Data/QuestRewards.lua",
            "src/Data/Gems.lua",
            "src/Data/Skills/act_int.lua",
            "src/Data/Skills/other.lua",
            "src/Data/Skills/sup_str.lua",
            "src/Data/Skills/sup_dex.lua",
            "src/Data/SkillStatMap.lua",
            "src/Data/Bases/mace.lua",
            "src/TreeData/0_5/tree.lua",
        ];
        let mut sources = BTreeMap::new();
        for path in paths {
            let text =
                fs::read_to_string(repository().join("vendor/path-of-building-poe2").join(path))
                    .unwrap()
                    .replace("\r\n", "\n");
            let record = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .find(|record| record["path"] == path)
                .unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(text.as_bytes())),
                record["sha256"].as_str().unwrap(),
                "Pinned source changed: {path}"
            );
            sources.insert(path.into(), text);
        }
        let lua = Lua::new();
        lua.set_memory_limit(96 * 1024 * 1024).unwrap();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        let common = &sources["src/Modules/Common.lua"];
        lua.load(format!(
            "local s_format=string.format; local m_floor=math.floor; common={{}}; {}\n{}\n{}",
            section(common, "-- Class library\n", "function codePointToUTF8"),
            section(
                common,
                "function round(val, dec)\n",
                "\n--- Rounds down a number"
            ),
            section(common, "function copyTable(tbl, noRecurse)\n", "\ndo\n")
        ))
        .exec()
        .unwrap();
        lua.load(&sources["src/Data/Global.lua"]).exec().unwrap();
        let data: Table = lua.load(&sources["src/Data/Misc.lua"]).eval().unwrap();
        lua.globals().set("data", data.clone()).unwrap();
        data.set(
            "modScalability",
            lua.load(&sources["src/Data/ModScalability.lua"])
                .eval::<Table>()
                .unwrap(),
        )
        .unwrap();
        lua.load(format!(
            "local m_floor=math.floor;local m_ceil=math.ceil;{}\n{}",
            section(
                common,
                "function roundSymmetric(val, dec)",
                "-- Symmetric ceil with precision:"
            ),
            section(
                common,
                "function wipeTable(tbl)",
                "-- Search a table for a value"
            )
        ))
        .exec()
        .unwrap();
        lua.load(section(
            &sources["src/Modules/ItemTools.lua"],
            "local t_insert = table.insert",
            "function itemLib.formatModLine(",
        ))
        .exec()
        .unwrap();
        let data_source = &sources["src/Modules/Data.lua"];
        for (begin, end) in [
            ("data.misc = {", "\ndata.skillColorMap = "),
            ("data.ailmentTypeList =", "data.buildupTypes ="),
            ("data.highPrecisionMods = {", "data.weaponTypeInfo = {"),
        ] {
            lua.load(section(data_source, begin, end)).exec().unwrap();
        }
        lua.load("modLib={};").exec().unwrap();
        lua.load(section(
            &sources["src/Modules/ModTools.lua"],
            "function modLib.createMod(",
            "\nmodLib.parseMod,",
        ))
        .exec()
        .unwrap();
        lua.load(&sources["src/Classes/ModStore.lua"])
            .exec()
            .unwrap();
        lua.load(&sources["src/Classes/ModDB.lua"]).exec().unwrap();
        lua.load(&sources["src/Classes/ModList.lua"])
            .exec()
            .unwrap();
        lua.load(format!("local t_insert=table.insert;local t_sort=table.sort;local s_format=string.format;local band=AND64;{}", section(&sources["src/Modules/ModTools.lua"],"function modLib.formatFlags(","-- Check if a mod contains a specific tag already"))).exec().unwrap();
        lua.load(format!(
            "PassiveTreeClass={{}};local t_insert=table.insert;local t_remove=table.remove;{}",
            section(
                &sources["src/Classes/PassiveTree.lua"],
                "function PassiveTreeClass:ProcessStats(",
                "-- Common processing code for nodes"
            )
        ))
        .exec()
        .unwrap();
        let gems: Table = lua.load(&sources["src/Data/Gems.lua"]).eval().unwrap();
        // Full gem source is retained separately. ModParser's unused dynamic
        // extra-skill catalog is empty; ordinary numeric lines do not query it.
        lua.globals().set("sourceGems", gems).unwrap();
        lua.load("data.gems={};data.keystones={};data.skills={};")
            .exec()
            .unwrap();
        let parser: Function = lua
            .load(&sources["src/Modules/ModParser.lua"])
            .eval()
            .unwrap();
        lua.globals()
            .get::<Table>("modLib")
            .unwrap()
            .set("parseMod", parser)
            .unwrap();
        lua.load(format!(
            "{}\nmod=makeSkillMod;flag=makeFlagMod;skill=makeSkillDataMod;skills={{}}",
            section(
                &sources["src/Modules/Data.lua"],
                "local function makeSkillMod(",
                "local function processMod("
            )
        ))
        .exec()
        .unwrap();
        let stat_map = lua
            .load(&sources["src/Data/SkillStatMap.lua"])
            .eval::<Function>()
            .unwrap()
            .call::<Table>((
                lua.globals().get::<Function>("mod").unwrap(),
                lua.globals().get::<Function>("flag").unwrap(),
                lua.globals().get::<Function>("skill").unwrap(),
            ))
            .unwrap();
        lua.globals().set("sourceSupportStatMap", stat_map).unwrap();
        for (path, start, end) in [
            (
                "src/Data/Skills/act_int.lua",
                "skills[\"SparkPlayer\"] = {",
                "\nskills[\"SummonSpectrePlayer\"]",
            ),
            (
                "src/Data/Skills/other.lua",
                "skills[\"Melee1HMacePlayer\"] = {",
                "\nskills[\"Melee2HMacePlayer\"]",
            ),
            (
                "src/Data/Skills/sup_str.lua",
                "skills[\"SupportBrutalityPlayer\"] = {",
                "\nskills[\"SupportBrutalityPlayerTwo\"]",
            ),
            (
                "src/Data/Skills/sup_str.lua",
                "skills[\"SupportMeleePhysicalDamagePlayer\"] = {",
                "\nskills[\"SupportHeftPlayer\"]",
            ),
            (
                "src/Data/Skills/sup_dex.lua",
                "skills[\"SupportRapidAttacksPlayer\"] = {",
                "\nskills[\"SupportRapidAttacksPlayerTwo\"]",
            ),
        ] {
            lua.load(section(&sources[path], start, end))
                .exec()
                .unwrap();
        }
        let bases = lua.create_table().unwrap();
        lua.load(&sources["src/Data/Bases/mace.lua"])
            .eval::<Function>()
            .unwrap()
            .call::<()>(bases.clone())
            .unwrap();
        lua.globals().set("sourceBases", bases).unwrap();
        let quests: Table = lua
            .load(&sources["src/Data/QuestRewards.lua"])
            .eval()
            .unwrap();
        lua.globals()
            .get::<Table>("data")
            .unwrap()
            .set("questRewards", quests)
            .unwrap();
        lua.load(format!(
            "{}\nlocal config={{}}; addQuestModsRewardsConfigOptions(config); return config",
            section(
                &sources["src/Modules/ConfigOptions.lua"],
                "local function addQuestModsRewardsConfigOptions(",
                "\nlocal configSettings = {"
            )
        ))
        .eval::<Table>()
        .map(|table| lua.globals().set("sourceQuestConfig", table).unwrap())
        .unwrap();
        lua.load("package.loaded['Modules.CalcBase']={};")
            .exec()
            .unwrap();
        let calcs: Table = lua
            .load(&sources["src/Modules/CalcDefence.lua"])
            .eval()
            .unwrap();
        lua.globals().set("sourceCalcs", calcs).unwrap();
        lua.load(section(
            &sources["src/Modules/CalcTools.lua"],
            "calcLib = { }",
            "-- Validate the level of the given gem",
        ))
        .exec()
        .unwrap();
        let tree: Table = lua
            .load(&sources["src/TreeData/0_5/tree.lua"])
            .eval()
            .unwrap();
        lua.globals().set("sourceTree", tree).unwrap();
        if warm {
            lua.load("for i=1,200 do sourceCalcs.hitChance(i*3,i*7,false); sourceCalcs.monsterHitChance(i*3,i*7); sourceCalcs.deflectChance(i*3,i*7); modLib.parseMod(tostring(i)..'% increased Spell Damage'); modLib.parseMod('-'..tostring(i)..'% to all Elemental Resistances'); modLib.parseMod('+'..tostring(i)..'% to Chaos Resistance'); modLib.parseMod('+'..tostring(i)..'% to Fire Resistance'); local actor={modDB=new('ModDB'):ModDB(),output={}}; sourceCalcs.doActorLifeManaSpirit(actor,true); end").exec().unwrap();
        }
        Self { lua, sources }
    }
    fn number(&self, expr: &str) -> f64 {
        self.lua.load(format!("return {expr}")).eval().unwrap()
    }
    fn json(&self, expr: &str) -> Value {
        let value: mlua::Value = self.lua.load(format!("return {expr}")).eval().unwrap();
        self.lua.from_value(value).unwrap()
    }
    fn resource_initialization(&self) -> Table {
        let setup = &self.sources["src/Modules/CalcSetup.lua"];
        let mut body = String::from("local modDB=new('ModDB'):ModDB(); ");
        for prefix in [
            "modDB:NewMod(\"Life\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"Mana\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"Accuracy\", \"BASE\", data.characterConstants",
            "modDB:NewMod(\"CritChanceCap\", \"BASE\",",
        ] {
            body.push_str(line(setup, prefix));
            body.push('\n');
        }
        body.push_str("return modDB");
        self.lua.load(body).eval().unwrap()
    }
    fn attribute_bonuses(&self) -> Table {
        self.lua.load(format!("local modDB=new('ModDB'):ModDB(); local output={{Str=1,Dex=1,Int=1}}; {}\nreturn modDB",section(&self.sources["src/Modules/CalcPerform.lua"],"\t-- Add attribute bonuses\n","\t-- Calculate Presence / Surrounded"))).eval().unwrap()
    }
    fn mod_value(&self, db: Table, name: &str) -> f64 {
        let f: Function = self
            .lua
            .load("return function(db,name) return db:Sum('BASE',nil,name) end")
            .eval()
            .unwrap();
        f.call((db, name)).unwrap()
    }
}
fn at<'a>(package: &'a Value, path: &str) -> &'a Value {
    package
        .pointer(path)
        .unwrap_or_else(|| panic!("Missing package field {path}"))
}
fn number(package: &Value, path: &str, expected: f64) {
    assert_eq!(at(package, path).as_f64().unwrap(), expected, "{path}");
}

fn package() -> Value {
    serde_json::to_value(
        poe_optimizer_data::game_data::bundled_snapshot()
            .unwrap()
            .package(),
    )
    .unwrap()
}

#[test]
fn reviewed_character_skill_weapon_monster_values_match_actual_pinned_lua() {
    let package = package();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        for (path, expr) in [
            (
                "/character/base_evasion",
                "data.characterConstants.base_evasion_rating",
            ),
            (
                "/character/critical_damage_bonus",
                "data.characterConstants.base_critical_hit_damage_bonus",
            ),
            (
                "/character/life_per_level",
                "data.characterConstants.life_per_level",
            ),
            (
                "/character/mana_per_level",
                "data.characterConstants.mana_per_level",
            ),
            (
                "/character/accuracy_per_level",
                "data.characterConstants.accuracy_rating_per_level",
            ),
            (
                "/character/accuracy_per_dexterity",
                "data.misc.AccuracyPerDexBase",
            ),
            ("/defence/armour_ratio", "data.misc.ArmourRatio"),
            (
                "/defence/deflection_chance_cap",
                "data.misc.DeflectionChanceCap",
            ),
            ("/defence/resistance_floor", "data.misc.ResistFloor"),
            (
                "/defence/player_resistance_cap",
                "data.characterConstants['base_maximum_all_resistances_%']",
            ),
            ("/defence/enemy_resistance_cap", "data.misc.MaxResistCap"),
            ("/defence/resistance_maximum_cap", "data.misc.MaxResistCap"),
            (
                "/defence/enemy_physical_reduction_cap",
                "data.monsterConstants['maximum_physical_damage_reduction_%']",
            ),
            (
                "/spark/lightning_minimum",
                "skills.SparkPlayer.statSets[1].levels[1][1]",
            ),
            (
                "/spark/lightning_maximum",
                "skills.SparkPlayer.statSets[1].levels[1][2]",
            ),
            ("/spark/cast_time", "skills.SparkPlayer.castTime"),
            (
                "/spark/critical_chance",
                "skills.SparkPlayer.levels[1].critChance",
            ),
            (
                "/supports/0/modifiers/0/value",
                "skills.SupportBrutalityPlayer.statSets[1].constantStats[1][2]",
            ),
            ("/encounters/normal_level_cap", "data.misc.MaxEnemyLevel"),
        ] {
            number(&package, path, oracle.number(expr));
        }
        let db = oracle.resource_initialization();
        let mods: Table = db.get("mods").unwrap();
        for (stat, path) in [
            ("Life", "/character/initial_life"),
            ("Mana", "/character/initial_mana"),
        ] {
            let modifier: Table = mods.get::<Table>(stat).unwrap().get(1).unwrap();
            number(
                &package,
                path,
                modifier.get::<Table>(1).unwrap().get("base").unwrap(),
            );
        }
        let bonuses = oracle.attribute_bonuses();
        number(
            &package,
            "/character/life_per_strength",
            oracle.mod_value(bonuses.clone(), "Life"),
        );
        number(
            &package,
            "/character/mana_per_intelligence",
            oracle.mod_value(bonuses.clone(), "Mana"),
        );
        number(
            &package,
            "/character/accuracy_per_dexterity",
            oracle.mod_value(bonuses, "Accuracy"),
        );
        let pools:Table=oracle.lua.load("local actor={modDB=new('ModDB'):ModDB(),output={}}; sourceCalcs.doActorLifeManaSpirit(actor,true); return actor.output").eval().unwrap();
        number(
            &package,
            "/character/minimum_life",
            pools.get("Life").unwrap(),
        );
        number(
            &package,
            "/character/minimum_mana",
            pools.get("Mana").unwrap(),
        );
        for (section, skill) in [
            ("spark", "SparkPlayer"),
            ("mace", "Melee1HMacePlayer"),
            ("supports/0", "SupportBrutalityPlayer"),
            ("supports/1", "SupportMeleePhysicalDamagePlayer"),
            ("supports/2", "SupportRapidAttacksPlayer"),
        ] {
            let gem:Table=oracle.lua.load("return function(id) for _,gem in pairs(sourceGems) do if gem.grantedEffectId==id then return gem end end; error('source gem missing') end").eval::<Function>().unwrap().call(skill).unwrap();
            for (field, source) in [
                ("skill_id", "grantedEffectId"),
                ("game_id", "gameId"),
                ("variant_id", "variantId"),
                ("name", "name"),
            ] {
                assert_eq!(
                    at(&package, &format!("/{section}/{field}"))
                        .as_str()
                        .unwrap(),
                    gem.get::<String>(source).unwrap()
                );
            }
        }
        for (profile, class_name) in [("spark", "Sorceress"), ("mace", "Warrior")] {
            let id = at(&package, &format!("/{profile}/default_class_id"))
                .as_u64()
                .unwrap();
            let resolved:Table=oracle.lua.load("return function(id) for _,class in pairs(sourceTree.classes) do if class.integerId==id then return class end end; error('source class missing') end").eval::<Function>().unwrap().call(id).unwrap();
            assert_eq!(resolved.get::<String>("name").unwrap(), class_name);
        }
        let weapons = package["weapons"].as_array().unwrap();
        assert_eq!(weapons.len(), 2);
        let mut seen = BTreeSet::new();
        for weapon in weapons {
            let name = weapon["name"].as_str().unwrap();
            assert!(seen.insert(name));
            let source: Table = oracle
                .lua
                .globals()
                .get::<Table>("sourceBases")
                .unwrap()
                .get(name)
                .unwrap();
            let stats: Table = source.get("weapon").unwrap();
            for (field, key) in [
                ("physical_minimum", "PhysicalMin"),
                ("physical_maximum", "PhysicalMax"),
                ("fire_minimum", "FireMin"),
                ("fire_maximum", "FireMax"),
                ("attack_rate", "AttackRateBase"),
                ("critical_chance", "CritChanceBase"),
            ] {
                assert_eq!(
                    weapon[field].as_f64().unwrap(),
                    stats.get::<Option<f64>>(key).unwrap().unwrap_or(0.0),
                    "{name}:{field}"
                );
            }
            let requirements: Table = source.get("req").unwrap();
            assert_eq!(
                weapon["requirements"]["level"].as_u64().unwrap(),
                requirements
                    .get::<Option<u64>>("level")
                    .unwrap()
                    .unwrap_or(0)
            );
            for (field, key) in [
                ("strength", "str"),
                ("dexterity", "dex"),
                ("intelligence", "int"),
            ] {
                assert_eq!(
                    weapon["requirements"]["attributes"][field]
                        .as_u64()
                        .unwrap(),
                    requirements.get::<Option<u64>>(key).unwrap().unwrap_or(0),
                    "{name} requirement {field}"
                );
            }
        }
        assert_eq!(seen, BTreeSet::from(["Wooden Club", "Smithing Hammer"]));
        for (key, source) in [
            ("armour", "data.monsterArmourTable"),
            ("evasion", "data.monsterEvasionTable"),
        ] {
            let expected = oracle.json(source);
            let actual = package["monsters"][key].as_array().unwrap();
            let expected = expected.as_array().unwrap();
            assert_eq!(actual.len(), 100);
            assert_eq!(actual.len(), expected.len());
            for (index, (a, b)) in actual.iter().zip(expected).enumerate() {
                assert_eq!(a.as_f64(), b.as_f64(), "monster {key} level{}", index + 1);
            }
        }
        for path in [
            "src/Data/Gems.lua",
            "src/Data/Misc.lua",
            "src/Data/QuestRewards.lua",
            "src/Modules/ModParser.lua",
            "src/Modules/ConfigOptions.lua",
            "src/Modules/CalcSetup.lua",
            "src/Modules/CalcPerform.lua",
            "src/Modules/CalcDefence.lua",
            "src/Modules/CalcOffence.lua",
            "src/Data/Skills/act_int.lua",
            "src/Data/Skills/other.lua",
            "src/Data/Skills/sup_str.lua",
            "src/Data/Skills/sup_dex.lua",
            "src/Data/SkillStatMap.lua",
            "src/Data/Bases/mace.lua",
            "src/TreeData/0_5/tree.lua",
        ] {
            assert_eq!(
                package["manifest"]["provenance"][path].as_str().unwrap(),
                format!("{:x}", Sha256::digest(oracle.sources[path].as_bytes())),
                "{path} provenance"
            );
        }
    }
}

/// These are literal parameters in the pinned formula, not independent native
/// expected values. Extract each unique anchored expression, then execute the
/// unchanged full formula separately across a grid to verify injected operands.
fn literal(source: &str, start: &str, end: &str) -> f64 {
    let begin = source.find(start).unwrap() + start.len();
    let tail = &source[begin..];
    tail[..tail.find(end).unwrap()].trim().parse().unwrap()
}
#[test]
fn reviewed_defence_formula_parameters_match_literal_source_and_executable_functions() {
    let p = package();
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let source = &o.sources["src/Modules/CalcDefence.lua"];
        let hit = section(
            source,
            "function calcs.hitChance(",
            "-- Calculate monster hit chance",
        );
        let monster = section(
            source,
            "function calcs.monsterHitChance(",
            "-- Calculate Deflect chance",
        );
        let deflect = section(
            source,
            "function calcs.deflectChance(",
            "-- Calculate damage reduction from armour",
        );
        for (key, value) in [
            ("hit_accuracy_multiplier", literal(hit, "accuracy * ", " )")),
            ("hit_evasion_multiplier", literal(hit, "evasion * ", " )")),
            ("hit_chance_floor", literal(hit, "return ", "\n")),
            (
                "hit_chance_cap",
                literal(hit, "m_min(round(rawChance), ", ")"),
            ),
            (
                "monster_evasion_multiplier",
                literal(monster, "1 - ( ", " * evasion"),
            ),
            (
                "monster_accuracy_multiplier",
                literal(monster, "evasion + ", " * accuracy"),
            ),
            (
                "deflection_rating_multiplier",
                literal(deflect, "deflection * ", " )"),
            ),
            (
                "deflection_chance_multiplier",
                literal(deflect, " ) * ", " - "),
            ),
            ("deflection_chance_offset", literal(deflect, " - ", "\n")),
            (
                "deflection_rating_floor",
                literal(deflect, "deflection < ", " then"),
            ),
        ] {
            number(&p, &format!("/defence/{key}"), value);
        }
        let d = &p["defence"];
        let n = |key: &str| d[key].as_f64().unwrap();
        let calc: Table = o.lua.globals().get("sourceCalcs").unwrap();
        for evasion in [0.0, 1.0, 24.0, 591.0, 1_000_000.0] {
            for accuracy in [1.0, 7.0, 42.0, 396.0, 1_000_000.0] {
                let raw = (accuracy * n("hit_accuracy_multiplier"))
                    / (accuracy + evasion * n("hit_evasion_multiplier"))
                    * 100.0;
                let expected = (raw + 0.5)
                    .floor()
                    .clamp(n("hit_chance_floor"), n("hit_chance_cap"));
                let actual: f64 = calc
                    .get::<Function>("hitChance")
                    .unwrap()
                    .call((evasion, accuracy, false))
                    .unwrap();
                assert_eq!(actual, expected);
                let raw = (1.0
                    - (n("monster_evasion_multiplier") * evasion)
                        / (evasion + n("monster_accuracy_multiplier") * accuracy))
                    * 100.0;
                let expected = (raw + 0.5)
                    .floor()
                    .clamp(n("hit_chance_floor"), n("hit_chance_cap"));
                let actual: f64 = calc
                    .get::<Function>("monsterHitChance")
                    .unwrap()
                    .call((evasion, accuracy))
                    .unwrap();
                assert_eq!(actual, expected);
                for deflection in [0.0, 0.99, 1.0, 42.0, 1_000_000.0] {
                    let expected = if deflection < n("deflection_rating_floor") {
                        0.0
                    } else {
                        let not_deflect = accuracy
                            / (accuracy + deflection * n("deflection_rating_multiplier"))
                            * n("deflection_chance_multiplier")
                            - n("deflection_chance_offset");
                        (100.0 - (not_deflect + 0.5).floor()).clamp(0.0, n("deflection_chance_cap"))
                    };
                    let actual: f64 = calc
                        .get::<Function>("deflectChance")
                        .unwrap()
                        .call((deflection, accuracy))
                        .unwrap();
                    assert_eq!(actual, expected);
                }
            }
        }
    }
}

fn canonical(lua: &Lua, mods: Table) -> Vec<String> {
    let function:Function=lua.load("return function(mod) if mod.name=='MinionModifier' then return mod.name..'/'..mod.type..'/'..mod.value.mod.name..'/'..mod.value.mod.type..'/'..mod.value.mod.value..'/'..mod.value.mod.flags end; return mod.name..'/'..mod.type..'/'..mod.value..'/'..mod.flags..'/'..mod.keywordFlags end").eval().unwrap();
    let mut result = mods
        .sequence_values::<Table>()
        .map(|m| function.call(m.unwrap()).unwrap())
        .collect::<Vec<_>>();
    result.sort();
    result
}
fn typed_effect(lua: &Lua, effect: &Value) -> Table {
    let value = effect["value"].as_f64().unwrap();
    let stat = effect["stat"].as_str().unwrap();
    let constructor:Function=lua.load(r#"return function(stat,value)
        local mod=modLib.createMod
        local resistance={fire_resistance_flat='FireResist',cold_resistance_flat='ColdResist',lightning_resistance_flat='LightningResist',chaos_resistance_flat='ChaosResist',elemental_resistance_flat='ElementalResist'}
        if resistance[stat] then return {mod(resistance[stat],'BASE',value)} end
        if stat=='armour_flat' then return {mod('Armour','BASE',value)} end
        if stat=='evasion_flat' then return {mod('Evasion','BASE',value)} end
        if stat=='energy_shield_flat' then return {mod('EnergyShield','BASE',value)} end
        if stat=='skill_speed_increased' then return {mod('Speed','INC',value),mod('WarcrySpeed','INC',value),mod('TotemPlacementSpeed','INC',value)} end
        if stat=='minion_damage_increased' then return {mod('MinionModifier','LIST',{mod=mod('Damage','INC',value)})} end
        local flags={spell_damage_increased=ModFlag.Spell,attack_damage_increased=ModFlag.Attack,melee_damage_increased=ModFlag.Melee,projectile_damage_increased=ModFlag.Projectile}
        assert(flags[stat],'unknown typed operation')
        return {mod('Damage','INC',value,nil,flags[stat])}
    end"#).eval().unwrap();
    constructor.call((stat, value)).unwrap()
}
#[test]
fn reviewed_quest_defaults_and_every_typed_passive_match_actual_configuration_and_parser() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    assert_quests_and_passives(snapshot.package());
}
fn assert_quests_and_passives(data: &poe_optimizer_data::game_data::GameDataPackage) {
    let p = serde_json::to_value(data).unwrap();
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let lua = &o.lua;
        let parser: Function = lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        let quests: Table = lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get("questRewards")
            .unwrap();
        let configs: Table = lua.globals().get("sourceQuestConfig").unwrap();
        for (index, (info, name, kind, path)) in [
            ("Candlemass", "Life", "BASE", "flat_life"),
            ("Molten Shrine", "Life", "INC", "life_increased"),
            ("Silent Hall", "Mana", "INC", "mana_increased"),
            ("Beira", "ColdResist", "BASE", "elemental_resistance"),
            (
                "Sisters of Garukhan Shrine",
                "LightningResist",
                "BASE",
                "elemental_resistance",
            ),
            ("Blackjaw", "FireResist", "BASE", "elemental_resistance"),
        ]
        .into_iter()
        .enumerate()
        {
            let quest = quests
                .clone()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|q| q.get::<String>("Info").unwrap() == info)
                .unwrap();
            let key = format!(
                "quest{}{}{}",
                quest.get::<String>("Description").unwrap(),
                quest.get::<String>("Area").unwrap(),
                quest.get::<String>("Info").unwrap()
            );
            assert_eq!(p["quests"]["config_keys"][index].as_str().unwrap(), key);
            let config = configs
                .clone()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|c| c.get::<Option<String>>("var").unwrap().as_deref() == Some(&key))
                .unwrap();
            assert_eq!(
                p["quests"]["default_enabled"][index].as_bool().unwrap(),
                config.get::<bool>("defaultState").unwrap()
            );
            let text: String = quest.get("Stat").unwrap();
            let (mods, extra): (Table, Option<String>) = parser.call(text).unwrap();
            assert!(extra.is_none());
            assert_eq!(mods.raw_len(), 1);
            let modifier: Table = mods.get(1).unwrap();
            assert_eq!(modifier.get::<String>("name").unwrap(), name);
            assert_eq!(modifier.get::<String>("type").unwrap(), kind);
            number(
                &p,
                &format!("/quests/{path}"),
                modifier.get("value").unwrap(),
            );
        }
        let tree: Table = lua.globals().get("sourceTree").unwrap();
        let classes: Table = tree.get("classes").unwrap();
        let nodes: Table = tree.get("nodes").unwrap();
        assert_eq!(data.passive_effects.len(), 1270);
        for class in classes
            .clone()
            .sequence_values::<Table>()
            .map(Result::unwrap)
        {
            let id = class.get::<u64>("integerId").unwrap();
            for (field, key) in [
                ("base_strength", "base_str"),
                ("base_dexterity", "base_dex"),
                ("base_intelligence", "base_int"),
            ] {
                assert_eq!(
                    p["tree"]["classes"][id.to_string()][field]
                        .as_f64()
                        .unwrap(),
                    class.get::<f64>(key).unwrap()
                );
            }
        }
        let select: Function = lua
            .load(
                r#"return function(nodes,id,kind,selector)
          local node
          for _,n in pairs(nodes) do if n.skill==id then node=copyTable(n);break end end
          assert(node,'missing physical source node')
          if kind~='base' then
            local option=node.options and node.options[selector]
            assert(option,'missing actual source option')
            for k,v in pairs(option)do node[k]=type(v)=="table" and copyTable(v) or v end
          end
          local sourceId=node.id or id
          node.sd=copyTable(node.stats);node.id=id;node.dn=node.name
          PassiveTreeClass:ProcessStats(node)
          assert(not node.unknown and not node.extra,'whole source parser coverage')
          return sourceId,node.modList
        end"#,
            )
            .eval()
            .unwrap();
        let set_source: Function = lua.load("return modLib.setSource").eval().unwrap();
        for record in &data.passive_effects {
            use poe_optimizer_data::class_tree::PassiveViewSelector;
            let selector = match &record.key.selector {
                PassiveViewSelector::Base => mlua::Value::Nil,
                PassiveViewSelector::Class { class_id } => {
                    lua.to_value(&data.tree.classes[class_id].name).unwrap()
                }
                PassiveViewSelector::Ascendancy { ascendancy_id } => lua
                    .to_value(&data.tree.ascendancies[ascendancy_id].name)
                    .unwrap(),
                PassiveViewSelector::Attribute { option } => {
                    mlua::Value::Integer(i64::from(option.source_index()))
                }
            };
            let kind = if matches!(record.key.selector, PassiveViewSelector::Base) {
                "base"
            } else {
                "option"
            };
            let (effective, actual): (u32, Table) = select
                .call((nodes.clone(), record.key.physical_node_id, kind, selector))
                .unwrap();
            assert_eq!(effective, record.effective_node_id);
            let mut remaining = actual
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            // Actor records are compared field-for-field, including nested tags/source.
            for actor in &record.actor_modifiers {
                let position = remaining
                    .iter()
                    .position(|m| m.get::<String>("name").unwrap() == actor.stat.upstream_name())
                    .unwrap();
                assert_actor_source_modifier(remaining.remove(position), actor);
            }
            // Scalar expansion is independent of the production converter and retains
            // every nested source/tag/scope field in the JSON comparison.
            let mut expected = Vec::new();
            for effect in &record.effects {
                for modifier in typed_effect(lua, &serde_json::to_value(effect).unwrap())
                    .sequence_values::<Table>()
                    .map(Result::unwrap)
                {
                    let modifier: Table = set_source
                        .call((modifier, format!("Tree:{}", record.key.physical_node_id)))
                        .unwrap();
                    expected.push(
                        serde_json::to_string(
                            &lua.from_value::<Value>(mlua::Value::Table(modifier))
                                .unwrap(),
                        )
                        .unwrap(),
                    );
                }
            }
            let mut actual = remaining
                .into_iter()
                .map(|m| {
                    serde_json::to_string(&lua.from_value::<Value>(mlua::Value::Table(m)).unwrap())
                        .unwrap()
                })
                .collect::<Vec<_>>();
            actual.sort();
            expected.sort();
            assert_eq!(actual, expected, "{:?}", record.key);
        }
    }
}

#[test]
fn reviewed_encounter_defaults_execute_original_config_branches_and_level_resolution() {
    let p = package();
    for warm in [false, true] {
        let o = Oracle::new(warm);
        let lua = &o.lua;
        let source = &o.sources["src/Modules/ConfigOptions.lua"];
        let penalty = lua
            .load(format!(
                "return {}",
                line(source, "{ var = \"resistancePenalty\"")
                    .trim()
                    .trim_end_matches(',')
            ))
            .eval::<Table>()
            .unwrap();
        number(
            &p,
            "/encounters/default_resistance_penalty",
            penalty
                .get::<Table>("list")
                .unwrap()
                .get::<Table>(penalty.get::<usize>("defaultIndex").unwrap())
                .unwrap()
                .get("val")
                .unwrap(),
        );
        let boss_text = section(
            source,
            "\t{ var = \"enemyIsBoss\"",
            "\t{ var = \"deliriousPercentage\"",
        );
        let boss: Table = lua
            .load(format!("return {{{boss_text}}}"))
            .eval::<Table>()
            .unwrap()
            .get(1)
            .unwrap();
        assert_eq!(
            p["encounters"]["default_boss"].as_str().unwrap(),
            boss.get::<Table>("list")
                .unwrap()
                .get::<Table>(boss.get::<usize>("defaultIndex").unwrap())
                .unwrap()
                .get::<String>("val")
                .unwrap()
        );
        let update:Function=lua.load(format!("local ConfigTabClass={{}}; local m_min=math.min; {}\nreturn ConfigTabClass.UpdateLevel",section(&o.sources["src/Classes/ConfigTab.lua"],"function ConfigTabClass:UpdateLevel()","\nfunction ConfigTabClass:BuildModList()"))).eval().unwrap();
        lua.globals().set("sourceUpdateLevel", update).unwrap();
        let build:Function=lua.load(r#"return function(level)
            local build={characterLevel=level};local config={build=build,configSets={{input={},placeholder={}}},activeConfigSetId=1,UpdateLevel=sourceUpdateLevel};build.configTab=config;
            config.varControls=setmetatable({},{__index=function(t,key)local control={SetPlaceholder=function(_,value)config.configSets[1].placeholder[key]=value end};rawset(t,key,control);return control end});return build
        end"#).eval().unwrap();
        for (branch, next, local_start, path) in [
            (
                "\t\tif val == \"None\" then",
                "\t\telseif val == \"Boss\" then",
                "\t\t\tlocal defaultResist =",
                "normal_elemental_resistance",
            ),
            (
                "\t\telseif val == \"Boss\" then",
                "\t\telseif val == \"Pinnacle\" then",
                "\t\t\tlocal defaultEleResist =",
                "standard_elemental_resistance",
            ),
            (
                "\t\telseif val == \"Pinnacle\" then",
                "\t\telseif val == \"Uber\" then",
                "\t\t\tlocal defaultEleResist =",
                "pinnacle_elemental_resistance",
            ),
        ] {
            let branch = section(source, branch, next);
            let block = section(branch, local_start, "\n\t\t\tlocal defaultDamage");
            let apply:Function=lua.load(format!("return function(build) local m_max=math.max; {block}\nreturn build.configTab end")).eval().unwrap();
            for level in [1, 60, 100] {
                let config: Table = apply.call(build.call::<Table>(level).unwrap()).unwrap();
                let values: Table = config
                    .get::<Table>("configSets")
                    .unwrap()
                    .get::<Table>(1)
                    .unwrap()
                    .get("placeholder")
                    .unwrap();
                let raw: mlua::Value = values.get("enemyLightningResist").unwrap();
                let resistance = match raw {
                    mlua::Value::String(s) if s.to_str().unwrap().is_empty() => 0.0,
                    mlua::Value::Number(n) => n,
                    mlua::Value::Integer(n) => n as f64,
                    _ => panic!("Unsupported placeholder {raw:?}"),
                };
                number(&p, &format!("/encounters/{path}"), resistance);
                if path == "pinnacle_elemental_resistance" {
                    number(
                        &p,
                        "/encounters/pinnacle_level",
                        values.get("enemyLevel").unwrap(),
                    );
                }
                if path == "normal_elemental_resistance" && level == 100 {
                    number(
                        &p,
                        "/encounters/normal_level_cap",
                        config.get("enemyLevel").unwrap(),
                    );
                }
            }
        }
    }
}

#[test]
fn requirements_match_source_gem_functions_support_counts_and_maximum_aggregation() {
    let package = package();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        oracle
            .lua
            .load(section(
                &oracle.sources["src/Modules/CalcTools.lua"],
                "function calcLib.getGemStatRequirement(",
                "-- Build table of stats for the given skill instance statset",
            ))
            .exec()
            .unwrap();
        let requirements: Function = oracle
            .lua
            .load("return calcLib.getGemStatRequirement")
            .eval()
            .unwrap();
        for (path, skill) in [
            ("spark", "SparkPlayer"),
            ("mace", "Melee1HMacePlayer"),
            ("supports/0", "SupportBrutalityPlayer"),
            ("supports/1", "SupportMeleePhysicalDamagePlayer"),
            ("supports/2", "SupportRapidAttacksPlayer"),
        ] {
            let gem: Table = oracle.lua.load("return function(id) for _,gem in pairs(sourceGems) do if gem.grantedEffectId==id then return gem end end; error('source gem missing') end").eval::<Function>().unwrap().call(skill).unwrap();
            let effect: Table = oracle
                .lua
                .globals()
                .get::<Table>("skills")
                .unwrap()
                .get(skill)
                .unwrap();
            let level: u32 = effect
                .get::<Table>("levels")
                .unwrap()
                .get::<Table>(1)
                .unwrap()
                .get("levelRequirement")
                .unwrap();
            let support = effect
                .get::<Option<bool>>("support")
                .unwrap()
                .unwrap_or(false);
            number(
                &package,
                &format!("/{path}/requirements/level"),
                level as f64,
            );
            for (attr, key) in [
                ("strength", "reqStr"),
                ("dexterity", "reqDex"),
                ("intelligence", "reqInt"),
            ] {
                let multiplier: u32 = gem.get(key).unwrap();
                let value: u32 = requirements.call((level, multiplier, support)).unwrap();
                number(
                    &package,
                    &format!("/{path}/requirements/attributes/{attr}"),
                    value as f64,
                );
                assert!(
                    line(
                        &oracle.sources["src/Classes/SkillsTab.lua"],
                        &format!("gemInstance.{key} =")
                    )
                    .contains("calcLib.getGemStatRequirement(gemInstance.reqLevel,")
                );
            }
        }
        // Nontrivial function inputs prove support status suppresses individual
        // attribute requirements; source gem multipliers are percentages, not costs.
        assert!(requirements.call::<u32>((20, 100, false)).unwrap() > 0);
        assert_eq!(requirements.call::<u32>((20, 100, true)).unwrap(), 0);
        assert_eq!(requirements.call::<u32>((20, 0, false)).unwrap(), 0);
        let count: Function = oracle.lua.load(format!(
            "return function(groups) local t_insert=table.insert; local env={{build={{skillsTab={{socketGroupList=groups}},calcsTab={{}}}},modDB={{multipliers={{}}}},requirementsTableGems={{}}}};{}\nreturn env.requirementsTableGems[1] end",
            section(&oracle.sources["src/Modules/CalcSetup.lua"], "\tlocal slotSupportGemSocketsCount = { R = 0, G = 0, B = 0 }", "\t-- Merge Requirements Tables")
        )).eval().unwrap();
        let groups: Table = oracle.lua.load("return {{enabled=true,gemList={{supportEffect={grantedEffect={color=1}}},{supportEffect={grantedEffect={color=2}}},{supportEffect={grantedEffect={color=3}}}}}}").eval().unwrap();
        let costs: Table = count.call(groups).unwrap();
        for (attr, source_attr) in [
            ("strength", "Str"),
            ("dexterity", "Dex"),
            ("intelligence", "Int"),
        ] {
            number(
                &package,
                &format!("/mace/support_attribute_costs/{attr}"),
                costs.get(source_attr).unwrap(),
            );
        }
        let color: u32 = oracle
            .lua
            .load("return skills.SupportBrutalityPlayer.color")
            .eval()
            .unwrap();
        let color_name = match color {
            1 => "red",
            2 => "green",
            3 => "blue",
            _ => panic!("unsupported source color"),
        };
        assert_eq!(package["supports"][0]["color"], color_name);
        // Supports in disabled groups and hidden effects do not contribute;
        // colors aggregate across all enabled groups, including inactive skills.
        let groups: Table = oracle.lua.load("return {{enabled=true,gemList={{supportEffect={grantedEffect={color=1}}},{supportEffect={grantedEffect={color=1}}},{supportEffect={grantedEffect={color=2,hidden=true}}}}},{enabled=true,gemList={{supportEffect={grantedEffect={color=1}}}}},{enabled=false,gemList={{supportEffect={grantedEffect={color=2}}}}}}").eval().unwrap();
        let counted: Table = count.call(groups).unwrap();
        assert_eq!(
            counted.get::<u32>("Str").unwrap(),
            3 * costs.get::<u32>("Str").unwrap()
        );
        assert_eq!(counted.get::<u32>("Dex").unwrap(), 0);
        let aggregate: Function = oracle.lua.load(format!(
            "return function(requirements) local m_floor=math.floor;local m_max=math.max;local modDB=new('ModDB'):ModDB();local env={{requirementsTable=requirements}};local output={{Str=15,Dex=7,Int=7}};local breakdown=nil;{}\nreturn output end",
            section(&oracle.sources["src/Modules/CalcPerform.lua"], "\t-- Process attribute requirements", "\t-- Calculate number of active heralds and auras affecting self")
        )).eval().unwrap();
        for (weapon, active, support_count, expected) in [
            (11, 0, 1, 11),
            (11, 13, 1, 13),
            (11, 0, 3, 15),
            (0, 0, 0, 0),
        ] {
            let sources: Table = oracle.lua.load(format!("return {{{{source='Item',sourceSlot='Weapon 1',sourceItem={{base={{weapon={{}}}}}},Str={weapon}}},{{source='Gem',Str={active},sourceGem={{gemData={{tags={{}}}}}}}},{{source='Support Gems',Str={}}}}}", support_count * costs.get::<u32>("Str").unwrap())).eval().unwrap();
            let result: Table = aggregate.call(sources).unwrap();
            assert_eq!(
                result.get::<Option<u32>>("ReqStr").unwrap().unwrap_or(0),
                expected
            );
        }
    }
}

#[test]
fn original_cold_and_warm_parser_distinguishes_signed_elemental_chaos_and_individual_resistances() {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let parser: Function = oracle
            .lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        for (text, stat, value) in [
            ("+8% to Fire Resistance", "fire_resistance_flat", 8.0),
            (
                "-20% to all Elemental Resistances",
                "elemental_resistance_flat",
                -20.0,
            ),
            ("+7% to Chaos Resistance", "chaos_resistance_flat", 7.0),
            ("-3.5% to Cold Resistance", "cold_resistance_flat", -3.5),
            (
                "+4.25% to Lightning Resistance",
                "lightning_resistance_flat",
                4.25,
            ),
        ] {
            let (mods, extra): (Table, Option<String>) = parser.call(text).unwrap();
            assert!(extra.is_none(), "{text}");
            assert_eq!(
                canonical(&oracle.lua, mods),
                canonical(
                    &oracle.lua,
                    typed_effect(&oracle.lua, &serde_json::json!({"stat":stat,"value":value}))
                ),
                "{text}; warm={warm}"
            );
        }
    }
}

#[test]
fn support_catalog_matches_raw_source_maps_flags_families_and_type_expressions_cold_and_warm() {
    use poe_optimizer_data::game_data::{
        SupportDamageType, SupportOperation, SupportScope, SupportSkillType, SupportStat,
    };
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let package = snapshot.package();
    let types = [
        (SupportSkillType::Attack, "Attack"),
        (SupportSkillType::MeleeSingleTarget, "MeleeSingleTarget"),
        (SupportSkillType::Melee, "Melee"),
        (SupportSkillType::Area, "Area"),
        (SupportSkillType::AttackInPlace, "AttackInPlace"),
        (SupportSkillType::Damage, "Damage"),
        (SupportSkillType::DamageOverTime, "DamageOverTime"),
        (SupportSkillType::CrossbowAmmoSkill, "CrossbowAmmoSkill"),
        (SupportSkillType::Herald, "Herald"),
        (SupportSkillType::NoAttackOrCastTime, "NoAttackOrCastTime"),
    ];
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        oracle
            .lua
            .load(section(
                &oracle.sources["src/Modules/CalcTools.lua"],
                "local typeExpressionStack = { }",
                "-- Check if given gem is of the given type",
            ))
            .exec()
            .unwrap();
        let can_support: Function = oracle
            .lua
            .globals()
            .get::<Table>("calcLib")
            .unwrap()
            .get("canGrantedEffectSupportActiveSkill")
            .unwrap();
        let source_types: Table = oracle.lua.globals().get("SkillType").unwrap();
        let skills: Table = oracle.lua.globals().get("skills").unwrap();
        let source_mace: Table = skills.get(package.mace.skill_id.as_str()).unwrap();
        let mace_types: Table = source_mace.get("skillTypes").unwrap();
        assert_eq!(
            mace_types.clone().pairs::<u32, bool>().count(),
            package.mace.skill_types.len()
        );
        for (kind, name) in types {
            let value = mace_types
                .get::<Option<bool>>(source_types.get::<u32>(name).unwrap())
                .unwrap()
                .unwrap_or(false);
            assert_eq!(package.mace.skill_types.contains(&kind), value);
        }
        assert_eq!(
            source_mace
                .get::<Table>("levels")
                .unwrap()
                .get::<Table>(1)
                .unwrap()
                .get::<Table>("cost")
                .unwrap()
                .get::<f64>("Mana")
                .unwrap(),
            package.mace.mana_cost
        );
        for support in &package.supports {
            let raw: Table = skills.get(support.skill_id.as_str()).unwrap();
            let family: Vec<String> = raw
                .get::<Table>("gemFamily")
                .unwrap()
                .sequence_values()
                .collect::<mlua::Result<_>>()
                .unwrap();
            assert_eq!(family, vec![support.family.clone()]);
            let level: Table = raw
                .get::<Table>("levels")
                .unwrap()
                .get(support.level)
                .unwrap();
            assert_eq!(
                level.get::<Option<f64>>("manaMultiplier").unwrap(),
                support.mana_multiplier
            );
            let source_color: u32 = raw.get("color").unwrap();
            assert_eq!(
                serde_json::to_value(support.color).unwrap(),
                ["red", "green", "blue"][(source_color - 1) as usize]
            );
            let stats: Table = raw.get::<Table>("statSets").unwrap().get(1).unwrap();
            let constants: Table = stats.get("constantStats").unwrap();
            let local_map: Option<Table> = stats.get("statMap").unwrap();
            let shared_map: Table = oracle.lua.globals().get("sourceSupportStatMap").unwrap();
            assert_eq!(constants.raw_len(), support.modifiers.len());
            for (index, modifier) in support.modifiers.iter().enumerate() {
                let constant: Table = constants.get(index + 1).unwrap();
                assert_eq!(constant.get::<f64>(2).unwrap(), modifier.value);
                let stat: String = constant.get(1).unwrap();
                let mapping: Table = local_map
                    .as_ref()
                    .and_then(|map| map.get::<Option<Table>>(stat.as_str()).unwrap())
                    .unwrap_or_else(|| shared_map.get(stat.as_str()).unwrap());
                assert_eq!(mapping.raw_len(), 1);
                let raw_mod: Table = mapping.get(1).unwrap();
                assert_eq!(
                    raw_mod.get::<String>("name").unwrap(),
                    match modifier.stat {
                        SupportStat::PhysicalDamage => "PhysicalDamage",
                        SupportStat::Speed => "Speed",
                    }
                );
                assert_eq!(
                    raw_mod.get::<String>("type").unwrap(),
                    match modifier.operation {
                        SupportOperation::Increased => "INC",
                        SupportOperation::More => "MORE",
                    }
                );
                let flag = match modifier.scope {
                    SupportScope::Any => 0,
                    SupportScope::Melee => oracle.number("ModFlag.Melee") as u32,
                    SupportScope::Attack => oracle.number("ModFlag.Attack") as u32,
                };
                assert_eq!(raw_mod.get::<u32>("flags").unwrap(), flag);
                assert_eq!(raw_mod.get::<u32>("keywordFlags").unwrap(), 0);
                assert!(matches!(
                    raw_mod.get::<mlua::Value>("value").unwrap(),
                    mlua::Value::Nil
                ));
                assert_eq!(raw_mod.raw_len(), 0);
            }
            let mut source_disabled = Vec::new();
            for stat in stats
                .get::<Table>("stats")
                .unwrap()
                .sequence_values::<String>()
            {
                let mapping: Table = shared_map.get(stat.unwrap().as_str()).unwrap();
                for raw_mod in mapping.sequence_values::<Table>() {
                    let raw_mod = raw_mod.unwrap();
                    assert_eq!(raw_mod.get::<String>("type").unwrap(), "FLAG");
                    assert!(raw_mod.get::<bool>("value").unwrap());
                    assert_eq!(raw_mod.get::<u32>("flags").unwrap(), 0);
                    assert_eq!(raw_mod.get::<u32>("keywordFlags").unwrap(), 0);
                    assert_eq!(raw_mod.raw_len(), 0);
                    source_disabled.push(match raw_mod.get::<String>("name").unwrap().as_str() {
                        "DealNoPhysical" => SupportDamageType::Physical,
                        "DealNoFire" => SupportDamageType::Fire,
                        "DealNoCold" => SupportDamageType::Cold,
                        "DealNoLightning" => SupportDamageType::Lightning,
                        "DealNoChaos" => SupportDamageType::Chaos,
                        other => panic!("unrepresented source flag {other}"),
                    });
                }
            }
            assert_eq!(source_disabled, support.disable_damage);
            // Independent source predicate covers every subset of the represented vocabulary.
            // Repeated calls also exercise LuaJIT warm traces when enabled.
            for mask in 0_u32..(1 << types.len()) {
                let active_types = oracle.lua.create_table().unwrap();
                let mut native_types = Vec::new();
                for (index, (kind, name)) in types.iter().enumerate() {
                    if mask & (1 << index) != 0 {
                        active_types
                            .set(source_types.get::<u32>(*name).unwrap(), true)
                            .unwrap();
                        native_types.push(*kind);
                    }
                }
                let active = oracle.lua.create_table().unwrap();
                let effect = oracle.lua.create_table().unwrap();
                effect.set("grantedEffect", source_mace.clone()).unwrap();
                active.set("activeEffect", effect).unwrap();
                active.set("skillTypes", active_types).unwrap();
                let expected: bool = can_support.call((raw.clone(), active)).unwrap();
                assert_eq!(
                    support.eligibility.admits(&native_types),
                    expected,
                    "{} mask {mask} warm {warm}",
                    support.id
                );
            }
        }
    }
}

#[test]
fn local_item_rule_captures_flags_and_operations_match_original_parser_cold_and_warm() {
    use poe_optimizer_data::game_data::{LocalWeaponOperation, LocalWeaponStat};
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    let package = snapshot.package();
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        assert_eq!(
            package.character.critical_chance_cap,
            oracle.mod_value(oracle.resource_initialization(), "CritChanceCap")
        );
        let parse: Function = oracle.lua.load("return function(text) local mods,extra=modLib.parseMod(text);return mods,extra end").eval().unwrap();
        if warm {
            oracle.lua.load("for i=1,500 do modLib.parseMod('Adds '..i..' to '..(i+1)..' Physical Damage');modLib.parseMod(i..'% increased Attack Speed');modLib.parseMod(i..'% increased Critical Hit Chance') end").exec().unwrap();
        }
        for rule in &package.item_modifier_rules {
            for first in [0, 1, 7, 17, 99, 100, 333, 999, 1000000] {
                let values = if rule.captures.len() == 2 {
                    vec![first, (first + 13).min(1000000)]
                } else {
                    vec![first]
                };
                let mut text = rule.template.clone();
                for (index, value) in values.iter().enumerate() {
                    text = text.replace(&format!("{{{index}}}"), &value.to_string());
                }
                let (mods, extra): (Table, Option<String>) = parse.call(text.as_str()).unwrap();
                assert!(extra.is_none(), "{text}: {extra:?}");
                assert_eq!(mods.raw_len(), rule.modifiers.len(), "{text}");
                for (index, mapping) in rule.modifiers.iter().enumerate() {
                    let actual: Table = mods.get(index + 1).unwrap();
                    let expected_name = match mapping.stat {
                        LocalWeaponStat::PhysicalMinimum => "PhysicalMin",
                        LocalWeaponStat::PhysicalMaximum => "PhysicalMax",
                        LocalWeaponStat::FireMinimum => "FireMin",
                        LocalWeaponStat::FireMaximum => "FireMax",
                        LocalWeaponStat::PhysicalDamage => "PhysicalDamage",
                        LocalWeaponStat::Speed => "Speed",
                        LocalWeaponStat::CriticalChance => "CritChance",
                    };
                    assert_eq!(
                        actual.get::<String>("name").unwrap(),
                        expected_name,
                        "{text}"
                    );
                    assert_eq!(
                        actual.get::<String>("type").unwrap(),
                        match mapping.operation {
                            LocalWeaponOperation::Base => "BASE",
                            LocalWeaponOperation::Increased => "INC",
                        }
                    );
                    assert_eq!(
                        actual.get::<f64>("value").unwrap(),
                        f64::from(values[mapping.capture as usize]),
                        "{text}"
                    );
                    assert_eq!(actual.get::<u64>("flags").unwrap(), mapping.flags, "{text}");
                    assert_eq!(
                        actual.get::<u64>("keywordFlags").unwrap(),
                        mapping.keyword_flags,
                        "{text}"
                    );
                    assert_eq!(actual.raw_len(), 0, "unexpected source tag: {text}");
                    assert!(actual.get::<mlua::Value>("source").unwrap().is_nil());
                }
            }
        }
        // These superficially similar source lines are global/conditional or do
        // not match the reviewed numeric grammar. Never reinterpret them locally.
        for text in [
            "Adds 1 to 2 Physical Damage to Attacks",
            "Adds 1 to 2 Fire Damage to Spells",
            "10% increased Attack Speed while holding a Shield",
        ] {
            let (mods, extra): (Table, Option<String>) = parse.call(text).unwrap();
            let local = extra.is_none()
                && mods.clone().sequence_values::<Table>().all(|m| {
                    let m = m.unwrap();
                    m.get::<u64>("keywordFlags").unwrap() == 0 && m.raw_len() == 0
                });
            assert!(!local, "source scope must not look local: {text}");
        }
        for text in [
            "1.5% increased Physical Damage",
            "1.5% increased Attack Speed",
            "1.5% increased Critical Hit Chance",
            "Adds 1.5 to 2 Physical Damage",
            "Adds 1 to 2.5 Fire Damage",
        ] {
            let result: (mlua::Value, Option<String>) = parse.call(text).unwrap();
            assert!(
                result.1.is_some() || result.0.is_nil(),
                "unexpected complete decimal source grammar: {text}"
            );
        }
    }
}

fn assert_actor_source_modifier(
    actual: Table,
    expected: &poe_optimizer_data::game_data::ActorModifierRecord,
) {
    use poe_optimizer_data::game_data::*;
    assert_eq!(
        actual.get::<String>("name").unwrap(),
        expected.stat.upstream_name()
    );
    match expected.effect {
        ActorModifierEffect::Numeric { operation, value } => {
            assert_eq!(
                actual.get::<String>("type").unwrap(),
                operation.upstream_name()
            );
            assert_eq!(actual.get::<f64>("value").unwrap(), value);
        }
        ActorModifierEffect::Flag { value } => {
            assert_eq!(actual.get::<String>("type").unwrap(), "FLAG");
            assert_eq!(actual.get::<bool>("value").unwrap(), value);
        }
    }
    assert_eq!(actual.get::<u64>("flags").unwrap(), expected.flags);
    assert_eq!(
        actual.get::<u64>("keywordFlags").unwrap(),
        expected.keyword_flags
    );
    assert_eq!(
        actual.get::<Option<String>>("source").unwrap(),
        expected.source
    );
    assert_eq!(actual.raw_len(), expected.tags.len());
    assert_eq!(
        actual.clone().pairs::<mlua::Value, mlua::Value>().count(),
        5 + usize::from(expected.source.is_some()) + expected.tags.len()
    );
    for (index, expected_tag) in expected.tags.iter().enumerate() {
        let tag: Table = actual.get(index + 1).unwrap();
        match expected_tag {
            ActorModifierTag::Global => {
                assert_eq!(tag.get::<String>("type").unwrap(), "Global");
                assert_eq!(tag.pairs::<mlua::Value, mlua::Value>().count(), 1);
            }
            ActorModifierTag::Condition { variables, negated } => {
                assert_eq!(tag.get::<String>("type").unwrap(), "Condition");
                let negative = tag.get::<Option<bool>>("neg").unwrap();
                assert_eq!(negative.unwrap_or(false), *negated);
                let names: Vec<String> =
                    if let Some(var) = tag.get::<Option<String>>("var").unwrap() {
                        vec![var]
                    } else {
                        tag.get::<Table>("varList")
                            .unwrap()
                            .sequence_values()
                            .map(Result::unwrap)
                            .collect()
                    };
                assert_eq!(
                    names,
                    variables
                        .iter()
                        .map(|v| v.upstream_name().to_owned())
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    tag.pairs::<mlua::Value, mlua::Value>().count(),
                    2 + usize::from(negative.is_some())
                );
            }
        }
    }
}
#[test]
fn actor_rules_match_independent_original_parser_for_signed_fractional_and_conditional_inputs() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    assert_actor_rules(snapshot.package());
}
fn assert_actor_rules(data: &poe_optimizer_data::game_data::GameDataPackage) {
    use poe_optimizer_data::game_data::*;
    let mut checked = 0;
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        if warm {
            oracle.lua.load("for i=1,500 do modLib.parseMod('+'..i..' to Strength');modLib.parseMod(i..'% increased maximum Life if Strength is higher than Intelligence');modLib.parseMod(i..'% less maximum Mana');modLib.parseMod('Gain no inherent bonuses from attributes');modLib.parseMod(i..'% increased Global Armour');modLib.parseMod('+'..i..'% to all Resistances');modLib.parseMod(i..'% increased maximum Energy Shield if Strength is higher than Intelligence') end").exec().unwrap();
        }
        let parser: Function = oracle
            .lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        for rule in &data.actor.modifier_rules {
            let values: &[f64] = match rule.captures.first() {
                Some(ActorCaptureKind::SignedDecimal) => {
                    &[-17.5, -1.0, 0.0, 1.0, 2.25, 17.0, 999.0, 1_000_000.0]
                }
                Some(_) => &[0.0, 1.0, 17.0, 999.0, 1_000_000.0],
                None => &[0.0],
            };
            for value in values {
                let word = if rule.captures.first() == Some(&ActorCaptureKind::SignedDecimal) {
                    format!("{value:+}")
                } else {
                    value.to_string()
                };
                let text = rule.template.replace("{0}", &word);
                let (mods, extra): (Table, Option<String>) = parser.call(text.as_str()).unwrap();
                assert!(extra.is_none(), "{text} warm{warm}");
                assert_eq!(mods.raw_len(), rule.modifiers.len());
                for (index, mapping) in rule.modifiers.iter().enumerate() {
                    let effect = match mapping.effect {
                        ActorRuleEffect::Numeric {
                            operation,
                            value: operand,
                        } => ActorModifierEffect::Numeric {
                            operation,
                            value: match operand {
                                ActorRuleValue::Capture { index, multiplier } => {
                                    assert_eq!(index, 0);
                                    value * multiplier
                                }
                                ActorRuleValue::Constant { value } => value,
                            },
                        },
                        ActorRuleEffect::Flag { value } => ActorModifierEffect::Flag { value },
                    };
                    let record = ActorModifierRecord {
                        stat: mapping.stat,
                        effect,
                        source: None,
                        flags: mapping.flags,
                        keyword_flags: mapping.keyword_flags,
                        tags: mapping.tags.clone(),
                    };
                    assert_actor_source_modifier(mods.get(index + 1).unwrap(), &record);
                }
                checked += 1;
            }
        }
        for text in [
            "1.5% increased Strength",
            "-1% more maximum Life",
            "1e3% increased Spirit",
        ] {
            let (mods, extra): (Option<Table>, Option<String>) = parser.call(text).unwrap();
            assert!(mods.is_none() || extra.is_some(), "{text}");
        }
        let (mods, extra): (Table, Option<String>) = parser.call("+5 to all Attributes").unwrap();
        assert!(extra.is_none());
        assert_eq!(
            mods.raw_len(),
            4,
            "All bookkeeping output must never silently disappear"
        );
        assert_eq!(
            mods.get::<Table>(4).unwrap().get::<String>("name").unwrap(),
            "All"
        );
    }
    assert_eq!(checked, 3750);
}
#[test]
fn actor_constants_precision_and_spirit_quests_match_independent_cold_and_warm_source() {
    use poe_optimizer_data::game_data::*;
    let snapshot = bundled_snapshot().unwrap();
    let data = &snapshot.package().actor;
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let precision: Table = lua
            .globals()
            .get::<Table>("data")
            .unwrap()
            .get("highPrecisionMods")
            .unwrap();
        assert_eq!(
            precision.clone().pairs::<String, Table>().count(),
            data.high_precision_mods.len()
        );
        for (name, operations) in &data.high_precision_mods {
            let original: Table = precision.get(name.as_str()).unwrap();
            assert_eq!(
                original.clone().pairs::<String, u8>().count(),
                operations.len()
            );
            for (operation, places) in operations {
                assert_eq!(
                    original.get::<u8>(operation.upstream_name()).unwrap(),
                    *places
                );
            }
        }
        let baseline:Table=lua.load("local actor={modDB=new('ModDB'):ModDB(),output={}};sourceCalcs.doActorLifeManaSpirit(actor,true);return actor.output").eval().unwrap();
        assert_eq!(baseline.get::<f64>("Spirit").unwrap(), data.minimum_spirit);
        assert_eq!(
            baseline.get::<f64>("LowLifePercentage").unwrap() / 100.0,
            data.low_life_threshold
        );
        assert_eq!(
            baseline.get::<f64>("FullLifePercentage").unwrap() / 100.0,
            data.full_life_threshold
        );
        let spirit: f64 = lua
            .load(format!(
                "local modDB=new('ModDB'):ModDB();{};return modDB:Sum('BASE',nil,'Spirit')",
                line(
                    &oracle.sources["src/Modules/CalcSetup.lua"],
                    "modDB:NewMod(\"Spirit\", \"BASE\", 0,"
                )
            ))
            .eval()
            .unwrap();
        assert_eq!(spirit, data.initial_spirit);
        let normal: f64 = lua
            .load(format!(
                "{};return inherentAttributeMultiplier",
                line(
                    &oracle.sources["src/Modules/CalcPerform.lua"],
                    "local inherentAttributeMultiplier ="
                )
            ))
            .eval()
            .unwrap();
        assert_eq!(normal, data.attribute_bonus_multiplier);
        let bonuses:Function=lua.load(format!("return function(flags) local modDB=new('ModDB'):ModDB();for _,flag in ipairs(flags) do modDB:NewMod(flag,'FLAG',true) end;local output={{Str=1,Dex=1,Int=1}};{};return modDB:Sum('BASE',nil,'Life') end",section(&oracle.sources["src/Modules/CalcPerform.lua"],"\t-- Add attribute bonuses\n","\t-- Calculate Presence / Surrounded"))).eval().unwrap();
        assert_eq!(
            bonuses
                .call::<f64>(
                    lua.create_sequence_from(["DoubledInherentAttributeBonuses"])
                        .unwrap()
                )
                .unwrap()
                / snapshot.package().character.life_per_strength,
            data.doubled_attribute_bonus_multiplier
        );
        assert_eq!(
            bonuses
                .call::<f64>(
                    lua.create_sequence_from(["HalvesLifeFromStrength"])
                        .unwrap()
                )
                .unwrap()
                / normal,
            data.halved_life_per_strength
        );
        let life:f64=lua.load("local actor={modDB=new('ModDB'):ModDB(),output={}};actor.modDB:NewMod('ChaosInoculation','FLAG',true);sourceCalcs.doActorLifeManaSpirit(actor,true);assert(actor.modDB.conditions.FullLife);return actor.output.Life").eval().unwrap();
        assert_eq!(life, data.chaos_inoculation_life);
        lua.load(section(
            &oracle.sources["src/Modules/ModTools.lua"],
            "function modLib.setSource(",
            "function modLib.hasTag(",
        ))
        .exec()
        .unwrap();
        let configs:Table=lua.load(format!("local StripEscapes=function(text) assert(not text:find('^',1,true));return text end;{};local config={{}};addQuestModsRewardsConfigOptions(config);return config",section(&oracle.sources["src/Modules/ConfigOptions.lua"],"local function questModsRewards(","\nlocal configSettings = {"))).eval().unwrap();
        for quest in &data.spirit_quests {
            let config = configs
                .clone()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|c| {
                    c.get::<Option<String>>("var").unwrap().as_deref() == Some(&quest.config_key)
                })
                .unwrap();
            assert_eq!(
                config.get::<bool>("defaultState").unwrap(),
                quest.default_enabled
            );
            assert_eq!(config.get::<String>("type").unwrap(), "check");
            let db: Table = lua.load("return new('ModDB'):ModDB()").eval().unwrap();
            let apply: Function = config.get("apply").unwrap();
            apply.call::<()>((true, db.clone(), db.clone())).unwrap();
            let mods: Table = db.get("mods").unwrap();
            assert_eq!(mods.clone().pairs::<String, Table>().count(), 1);
            let spirit: Table = mods.get("Spirit").unwrap();
            assert_eq!(spirit.raw_len(), 1);
            assert_actor_source_modifier(spirit.get(1).unwrap(), &quest.modifiers[0]);
        }
    }
}

#[test]
fn complete_jewellery_base_values_and_implicit_expansions_match_cold_and_warm_source() {
    let snapshot = poe_optimizer_data::game_data::bundled_snapshot().unwrap();
    assert_jewellery(snapshot.package());
}
fn assert_jewellery(data: &poe_optimizer_data::game_data::GameDataPackage) {
    assert_eq!(data.jewellery_bases.len(), 7);
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let bases = lua.create_table().unwrap();
        lua.load(&oracle.sources["src/Data/Bases/amulet.lua"])
            .eval::<Function>()
            .unwrap()
            .call::<()>(bases.clone())
            .unwrap();
        let parser: Function = lua
            .globals()
            .get::<Table>("modLib")
            .unwrap()
            .get("parseMod")
            .unwrap();
        for base in &data.jewellery_bases {
            let raw: Table = bases.get(base.name.as_str()).unwrap();
            assert_eq!(raw.clone().pairs::<String, mlua::Value>().count(), 5);
            assert_eq!(raw.get::<String>("type").unwrap(), "Amulet");
            let tags: Table = raw.get("tags").unwrap();
            assert_eq!(tags.clone().pairs::<String, bool>().count(), 2);
            assert!(tags.get::<bool>("amulet").unwrap() && tags.get::<bool>("default").unwrap());
            let req: Table = raw.get("req").unwrap();
            assert_eq!(base.requirements.level, req.get::<u32>("level").unwrap());
            for (value, name) in [
                (base.requirements.attributes.strength, "str"),
                (base.requirements.attributes.dexterity, "dex"),
                (base.requirements.attributes.intelligence, "int"),
            ] {
                assert_eq!(value, req.get::<Option<u32>>(name).unwrap().unwrap_or(0));
            }
            let text: String = raw.get("implicit").unwrap();
            assert_eq!(text, base.implicit.source_text);
            let range:Function=lua.load("return function(s)local a,b=s:match('^%+%((%d+)%-(%d+)%)');return tonumber(a),tonumber(b)end").eval().unwrap();
            let (min, max): (f64, f64) = range.call(text.as_str()).unwrap();
            assert_eq!((min, max), (base.implicit.minimum, base.implicit.maximum));
            let types: Table = raw.get("implicitModTypes").unwrap();
            assert_eq!(types.raw_len(), 1);
            assert_eq!(
                types
                    .get::<Table>(1)
                    .unwrap()
                    .sequence_values::<String>()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>(),
                base.implicit.modifier_types
            );
            let rule = data
                .actor
                .modifier_rule(&base.implicit.actor_rule_id)
                .unwrap();
            for value in [min, max] {
                let line = rule.template.replace("{0}", &format!("+{value}"));
                let (mods, extra): (Table, Option<String>) = parser.call(line).unwrap();
                assert!(extra.is_none());
                assert_eq!(mods.raw_len(), rule.modifiers.len());
                for (index, mapping) in rule.modifiers.iter().enumerate() {
                    use poe_optimizer_data::game_data::*;
                    let record = ActorModifierRecord {
                        stat: mapping.stat,
                        effect: ActorModifierEffect::Numeric {
                            operation: ActorNumericOperation::Base,
                            value,
                        },
                        source: None,
                        flags: mapping.flags,
                        keyword_flags: mapping.keyword_flags,
                        tags: mapping.tags.clone(),
                    };
                    assert_actor_source_modifier(mods.get(index + 1).unwrap(), &record);
                }
            }
        }
    }
}

/// Run before installing a changed bundled package: no native evaluator or compiled
/// package supplies either side of these original-source comparisons.
#[test]
fn fresh_reviewed_receiving_package_matches_independent_original_source() {
    let extracted = poe_optimizer_pob::game_data::extract_pinned_game_data_for_review(
        &repository().join("vendor/path-of-building-poe2"),
    )
    .unwrap();
    let data = &extracted.package;
    assert_eq!(data.actor.modifier_rules.len(), 329);
    assert_quests_and_passives(data);
    assert_actor_rules(data);
    assert_jewellery(data);
    assert_receiving_queries(data);
}
fn assert_receiving_queries(data: &poe_optimizer_data::game_data::GameDataPackage) {
    for warm in [false, true] {
        let oracle = Oracle::new(warm);
        let lua = &oracle.lua;
        let source = &oracle.sources["src/Modules/CalcDefence.lua"];
        let resources: Table = lua
            .load(format!(
                "local modDB={{Flag=function() return false end}};{};return resourceList",
                section(
                    source,
                    "local resourceList = {",
                    "\n\t\tfor _, source in ipairs(resourceList) do"
                )
            ))
            .eval()
            .unwrap();
        assert_eq!(resources.raw_len(), 6, "Ward remains outside this profile");
        for (index, query) in data.receiving_defence.resources.iter().enumerate() {
            let actual: Table = resources.get(index + 1).unwrap();
            assert_eq!(
                actual.get::<String>("name").unwrap(),
                query.stat.upstream_name()
            );
            let actual = actual
                .get::<Table>("mods")
                .unwrap()
                .sequence_values::<String>()
                .map(Result::unwrap)
                .collect::<Vec<_>>();
            assert_eq!(
                actual,
                query
                    .query_stats
                    .iter()
                    .map(|v| v.upstream_name().to_owned())
                    .collect::<Vec<_>>()
            );
        }
        let (types, elemental): (Table, Table) = lua
            .load(format!(
                "{};{};return resistTypeList,isElemental",
                line(source, "local resistTypeList ="),
                line(source, "local isElemental =")
            ))
            .eval()
            .unwrap();
        assert_eq!(types.raw_len(), data.receiving_defence.resistances.len());
        for (index, query) in data.receiving_defence.resistances.iter().enumerate() {
            let name: String = types.get(index + 1).unwrap();
            let mut expected = vec![format!("{name}Resist")];
            if elemental
                .get::<Option<bool>>(name.as_str())
                .unwrap()
                .unwrap_or(false)
            {
                expected.push("ElementalResist".into());
            }
            assert_eq!(expected[0], query.stat.upstream_name());
            assert_eq!(
                expected,
                query
                    .query_stats
                    .iter()
                    .map(|v| v.upstream_name().to_owned())
                    .collect::<Vec<_>>()
            );
        }
    }
}

#[path = "support/armour_source.rs"]
mod armour_source;

#[path = "support/item_formatting_source.rs"]
mod item_formatting_source;
