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
        lua.globals().set("data", data).unwrap();
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
        lua.load("skills={}; mod=modLib.createMod;").exec().unwrap();
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
            lua.load("for i=1,200 do sourceCalcs.hitChance(i*3,i*7,false); sourceCalcs.monsterHitChance(i*3,i*7); sourceCalcs.deflectChance(i*3,i*7); modLib.parseMod(tostring(i)..'% increased Spell Damage'); local actor={modDB=new('ModDB'):ModDB(),output={}}; sourceCalcs.doActorLifeManaSpirit(actor,true); end").exec().unwrap();
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
                "/mace/brutality/physical_more",
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
            ("mace/brutality", "SupportBrutalityPlayer"),
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
fn reviewed_quest_defaults_and_every_typed_entrance_match_actual_configuration_and_parser() {
    let p = package();
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
        assert_eq!(p["entrance_effects"].as_array().unwrap().len(), 16);
        let mut seen = BTreeSet::new();
        for record in p["entrance_effects"].as_array().unwrap() {
            let class_id = record["class_id"].as_u64().unwrap();
            let physical = record["physical_node_id"].as_u64().unwrap();
            assert!(seen.insert((class_id, physical)));
            let class = classes
                .clone()
                .sequence_values::<Table>()
                .map(Result::unwrap)
                .find(|class| class.get::<u64>("integerId").unwrap() == class_id)
                .unwrap();
            let class_name: String = class.get("name").unwrap();
            let bundled = &p["tree"]["classes"][class_id.to_string()];
            for (field, key) in [
                ("base_strength", "base_str"),
                ("base_dexterity", "base_dex"),
                ("base_intelligence", "base_int"),
            ] {
                assert_eq!(
                    bundled[field].as_f64().unwrap(),
                    class.get::<f64>(key).unwrap()
                );
            }
            let node = nodes
                .clone()
                .pairs::<mlua::Value, Table>()
                .map(Result::unwrap)
                .map(|(_, node)| node)
                .find(|node| node.get::<Option<u64>>("skill").unwrap() == Some(physical))
                .unwrap();
            let effective = node
                .get::<Option<Table>>("options")
                .unwrap()
                .and_then(|options| options.get::<Option<Table>>(class_name).unwrap())
                .unwrap_or(node);
            let source_id = effective
                .get::<Option<u64>>("id")
                .unwrap()
                .unwrap_or(physical);
            assert_eq!(record["effective_node_id"].as_u64().unwrap(), source_id);
            let stats: Table = effective.get("stats").unwrap();
            let effects = record["effects"].as_array().unwrap();
            assert_eq!(effects.len(), stats.raw_len());
            for (index, effect) in effects.iter().enumerate() {
                let text: String = stats.get(index + 1).unwrap();
                let (mods, extra): (Table, Option<String>) = parser.call(text.as_str()).unwrap();
                assert!(extra.is_none(), "{text}");
                assert_eq!(
                    canonical(lua, typed_effect(lua, effect)),
                    canonical(lua, mods),
                    "{class_id}:{physical} effect{index} {text}"
                );
            }
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
            ("mace/brutality", "SupportBrutalityPlayer"),
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
        assert_eq!(package["mace"]["brutality"]["color"], color_name);
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
