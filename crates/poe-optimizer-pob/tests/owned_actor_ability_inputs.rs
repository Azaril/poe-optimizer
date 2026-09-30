//! Complete-source minion ability input observations, not numerical/build parity.
//! The untouched original is captured before separately labelled actor-level probes.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str =
    "complete_source_actor_abilities_keep_effect_level_actor_level_and_quality_distinct";
const CLERIC_TEST: &str = "original_cleric_keeps_main_selection_and_exact_generated_heal_inputs";
const CHILD: &str = "POE_ACTOR_ABILITY_INPUT_SOURCE_CHILD";
const SOURCE_FILES: [&str; 12] = [
    "src/Classes/SkillsTab.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Modules/CalcTools.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/Data.lua",
    "src/Data/Misc.lua",
    "src/Data/Skills/act_int.lua",
    "src/Data/Minions.lua",
    "src/Data/Skills/minion.lua",
    "src/Data/Skills/sup_int.lua",
    "src/Data/Skills/sup_str.lua",
];

#[test]
fn complete_source_actor_abilities_keep_effect_level_actor_level_and_quality_distinct() {
    run_source_case(
        TEST,
        "build-05.xml",
        "owned-actor-ability-inputs-01",
        OBSERVATION,
        assert_observations,
    );
}

#[test]
fn original_cleric_keeps_main_selection_and_exact_generated_heal_inputs() {
    run_source_case(
        CLERIC_TEST,
        "build-01.xml",
        "owned-cleric-ability-inputs-01",
        CLERIC_OBSERVATION,
        assert_cleric_observations,
    );
}

fn run_source_case(
    test: &str,
    fixture: &str,
    destination: &str,
    observation: &str,
    check: fn(&Json),
) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs").join(destination);
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let enabled = mode == "on";
        let fixture_directory = root.join("tests/fixtures/builds/breadth-20260908");
        let xml = fs::read_to_string(fixture_directory.join(fixture)).unwrap();
        let manifest = read(&fixture_directory.join("index.json"));
        let entry = manifest["builds"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["xml"] == fixture)
            .unwrap();
        let xml_sha256 = format!("{:x}", Sha256::digest(xml.as_bytes()));
        // Match the independently recorded input commitment without modifying
        // its skill selection or any authored group before source evaluation.
        assert_eq!(entry["xml_sha256"], xml_sha256);
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("actorAbilityJitEnabled", enabled)?;
            lua.load("if actorAbilityJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let observation_hook = |lua: &Lua| observe(lua, observation);
        let mut result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&before),
            None,
            Some(&observation_hook),
        )
        .unwrap();
        let files: Vec<_> = SOURCE_FILES
            .iter()
            .map(|path| json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))
            .collect();
        result["evidence"] = json!({
            "manifest_sha256":pinned::manifest_sha256(),
            "files":files,
            "scope":"complete_source_actor_ability_inputs",
            "fixture":fixture,
            "xml_sha256":xml_sha256,
            "native_activation_proven":false,
            "full_build_numeric_parity":false,
        });
        // Source observations, including component errors, precede domain assertions.
        fs::write(
            destination.join(format!(
                "source-jit-{}.json",
                if enabled { "on" } else { "off" }
            )),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        check(&result["additional_observation"]);
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", test, "--nocapture"])
            .env(CHILD, mode)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source child failed: {}",
                    log_path.display()
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source child deadline: {}", log_path.display());
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let off = read(&destination.join("source-jit-off.json"));
    let on = read(&destination.join("source-jit-on.json"));
    assert_eq!(off["source_hash"], on["source_hash"]);
    assert_eq!(off["evidence"], on["evidence"]);
    assert!(
        off["additional_observation"] == on["additional_observation"],
        "JIT modes differ in complete-source actor ability observations"
    );
}
fn read(path: &Path) -> Json {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn observe(lua: &Lua, observation: &str) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(format!("{COMMON_OBSERVATION}\n{observation}"))
        .set_name("@owned-actor-ability-input-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
fn assert_child(child: &Json, actor_level: &Json) {
    assert_eq!(child["effect_level"], 1);
    assert_eq!(child["quality"], 0);
    assert_eq!(&child["actor_level"], actor_level);
    assert_eq!(child["has_physical_source"], false);
    assert_eq!(child["has_gem_data"], false);
    assert_eq!(child["same_actor"], true);
    assert_eq!(child["same_summoner"], true);
    let levels = child["effect_levels"].as_array().unwrap();
    assert_eq!(levels.len(), 1);
    assert_eq!(levels[0]["key"], 1);
    assert_eq!(levels[0]["requirement"], 0);
}
fn assert_snapshot(snapshot: &Json, family: &str) {
    let children = snapshot["children"].as_array().unwrap();
    assert_eq!(children.len(), 2);
    let expected = match family {
        "sniper" => ["MinionMeleeBow", "GasShotSkeletonSniperMinion"],
        "storm_mage" => ["ArcSkeletonMageMinion", "DeathStormSkeletonStormMageMinion"],
        _ => panic!("unexpected family"),
    };
    for (child, id) in children.iter().zip(expected) {
        assert_eq!(child["id"], id);
        assert_child(child, &snapshot["actor_level"]);
    }
    if family == "sniper" {
        assert_eq!(children[0]["stat_sets"].as_array().unwrap().len(), 1);
        let gas = children[1]["stat_sets"].as_array().unwrap();
        assert_eq!(gas.len(), 3);
        for (row, label) in gas.iter().zip(["Impact", "Poison Cloud", "Explosion"]) {
            assert_eq!(row["label"], label);
            assert_eq!(row["levels"].as_array().unwrap().len(), 1);
            assert_eq!(row["levels"][0]["actor_level"], 1);
        }
    } else {
        let arc = &children[0]["stat_sets"][0];
        let rows = arc["levels"].as_array().unwrap();
        assert_eq!(rows.len(), 4);
        for (row, level) in rows.iter().zip([1, 20, 40, 60]) {
            assert_eq!(row["actor_level"], level);
            assert_eq!(row["interpolation"][0], 3);
            assert_eq!(row["interpolation"][1], 3);
        }
    }
}
fn assert_observations(result: &Json) {
    let original = &result["untouched_original"];
    assert_eq!(original["status"], "ok", "{}", original["error"]);
    let original = &original["value"];
    assert_eq!(original["summoning_id"], "SummonSkeletalSnipersPlayer");
    assert_eq!(original["physical_level"], 20);
    assert_eq!(original["physical_quality"], 0);
    assert_eq!(original["summoning_effect_level"], 22);
    assert_eq!(original["actor_level"], 44);
    assert_eq!(original["selected_ability"], "MinionMeleeBow");
    assert_snapshot(original, "sniper");

    let probes = result["component_probes"].as_array().unwrap();
    assert_eq!(probes.len(), 10);
    for probe in probes {
        assert_eq!(probe["status"], "ok", "{}", probe["error"]);
        let family = probe["family"].as_str().unwrap();
        let snapshot = &probe["value"];
        assert_eq!(snapshot["physical_level"], 20);
        assert_eq!(snapshot["physical_quality"], probe["parent_quality"]);
        assert_eq!(snapshot["actor_level"], probe["requested_actor_level"]);
        assert_snapshot(snapshot, family);
    }
    // Independent caster contrast: fixed ability table row 1 still scales raw
    // spell stats with actual actor level. No native formula is reimplemented.
    let storm: Vec<_> = probes
        .iter()
        .filter(|probe| probe["family"] == "storm_mage" && probe["parent_quality"] == 20)
        .collect();
    assert_eq!(storm.len(), 4);
    for pair in storm.windows(2) {
        for stat in [
            "spell_minimum_base_lightning_damage",
            "spell_maximum_base_lightning_damage",
        ] {
            let get = |probe: &Json| {
                probe["value"]["children"][0]["stat_sets"][0]["raw_stats"][stat]
                    .as_f64()
                    .unwrap()
            };
            assert!(
                get(pair[1]) > get(pair[0]),
                "Arc actor-level interpolation must change {stat}"
            );
        }
    }
    for family in ["sniper", "storm_mage"] {
        let values: Vec<_> = probes
            .iter()
            .filter(|probe| probe["family"] == family && probe["requested_actor_level"] == 40)
            .collect();
        assert_eq!(values.len(), 2);
        assert_ne!(values[0]["parent_quality"], values[1]["parent_quality"]);
        assert!(
            values[0]["value"]["children"] == values[1]["value"]["children"],
            "physical parent quality 0/20 must not become child effect quality: {family}"
        );
    }
}

fn assert_cleric_snapshot(snapshot: &Json) {
    assert_eq!(snapshot["summoning_id"], "SummonSkeletalClericsPlayer");
    assert_eq!(snapshot["actor_type"], "RaisedSkeletonCleric");
    assert_eq!(snapshot["alternate_level_policy"], false);
    assert_eq!(snapshot["actor_level"], snapshot["table_actor_level"]);
    assert_eq!(snapshot["selected_ability"], "HealSkeletonClericMinion");
    assert_eq!(snapshot["minion_types"], json!(["Cooldown", "Spell"]));
    assert_eq!(
        snapshot["source_types"],
        json!([
            "CreatesMinion",
            "CreatesSkeletonMinion",
            "CreatesUndeadMinion",
            "HasReservation",
            "Minion",
            "MinionsCanExplode",
            "MultipleReservation",
            "Persistent"
        ])
    );
    let children = snapshot["children"].as_array().unwrap();
    assert_eq!(children.len(), 1);
    let child = &children[0];
    assert_eq!(child["id"], "HealSkeletonClericMinion");
    assert_child(child, &snapshot["actor_level"]);
    assert_eq!(
        child["source_types"],
        json!(["AttackInPlace", "Buff", "Duration", "Spell"])
    );
    assert_eq!(child["final_types"], child["source_types"]);
    assert_eq!(child["retained_supports"], snapshot["retained_supports"]);
    let sets = child["stat_sets"].as_array().unwrap();
    assert_eq!(sets.len(), 1);
    assert_eq!(sets[0]["label"], "Heal");
    assert_eq!(
        sets[0]["raw_stats"]["skeletal_cleric_grants_base_life_regeneration_rate_per_minute"]
            .as_f64()
            .unwrap(),
        776.0
    );
    // Unlike the Storm Mage's effectiveness interpolation, Heal uses static
    // interpolation1 at effect-level row1. Its other actorLevel-labelled data
    // rows do not make the pinned source choose a different raw healing value.
    let levels = sets[0]["levels"].as_array().unwrap();
    assert!(levels.len() > 1);
    assert_eq!(levels[0]["actor_level"], 1);
    for row in levels {
        assert_eq!(row["interpolation"], json!([1]));
    }
    let inventory = snapshot["declared_abilities"].as_array().unwrap();
    assert_eq!(inventory.len(), 3);
    for (row, (id, available)) in inventory.iter().zip([
        ("HealSkeletonClericMinion", true),
        ("ResurrectSkeletonClericMinion", false),
        ("DoLiterallyNothing", false),
    ]) {
        assert_eq!(row["id"], id);
        assert_eq!(row["loaded_definition"], available);
        assert_eq!(row["emitted"], available);
    }
    assert_eq!(snapshot["child_inherits_same_support_list"], true);
}

fn assert_cleric_observations(result: &Json) {
    let original = &result["untouched_original"];
    assert_eq!(original["status"], "ok", "{}", original["error"]);
    let original = &original["value"];
    assert_eq!(original["main_socket_group"], 1);
    assert_eq!(original["main_skill"], "SummonSandDjinnPlayer");
    assert_eq!(original["supporting_group"], 5);
    assert_eq!(original["group_enabled"], true);
    assert_eq!(original["physical_enabled"], true);
    assert_eq!(original["is_main_skill"], false);
    let snapshot = &original["cleric"];
    assert_eq!(snapshot["physical_level"], 19);
    assert_eq!(snapshot["physical_quality"], 20);
    assert_eq!(snapshot["summoning_effect_level"], 30);
    assert_eq!(snapshot["actor_level"], 60);
    assert_cleric_snapshot(snapshot);
    assert_eq!(
        snapshot["retained_supports"],
        json!([
            "SupportLastGaspPlayer",
            "SupportRapidCastingPlayerTwo",
            "SupportMeatShieldPlayerTwo",
            "SupportElementalArmyPlayer"
        ])
    );
    assert!(
        snapshot["final_types"]
            .as_array()
            .unwrap()
            .contains(&json!("Duration"))
    );

    let probes = result["component_probes"].as_array().unwrap();
    assert_eq!(probes.len(), 5);
    for probe in probes {
        assert_eq!(probe["status"], "ok", "{}", probe["error"]);
        let snapshot = &probe["value"];
        assert_eq!(snapshot["physical_level"], probe["physical_level"]);
        assert_eq!(snapshot["physical_quality"], probe["physical_quality"]);
        let (effect, actor) = match probe["physical_level"].as_i64().unwrap() {
            1 => (12, 24),
            19 => (30, 60),
            20 => (31, 62),
            40 => (40, 80),
            level => panic!("unreviewed physical level {level}"),
        };
        assert_eq!(snapshot["summoning_effect_level"], effect);
        assert_eq!(snapshot["actor_level"], actor);
        assert_cleric_snapshot(snapshot);
    }
    let quality: Vec<_> = probes
        .iter()
        .filter(|p| p["physical_level"] == 19)
        .collect();
    assert_eq!(quality.len(), 2);
    assert_ne!(
        quality[0]["physical_quality"],
        quality[1]["physical_quality"]
    );
    assert_eq!(
        quality[0]["value"]["children"],
        quality[1]["value"]["children"]
    );
    let levels: Vec<_> = probes
        .iter()
        .filter(|p| p["physical_quality"] == 20)
        .collect();
    assert_eq!(levels.len(), 4);
    for pair in levels.windows(2) {
        let left = &pair[0]["value"];
        let right = &pair[1]["value"];
        assert!(
            right["summoning_effect_level"].as_i64().unwrap()
                > left["summoning_effect_level"].as_i64().unwrap()
        );
        assert!(right["actor_level"].as_i64().unwrap() > left["actor_level"].as_i64().unwrap());
    }
}

const COMMON_OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line)
 return f
end
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
local init=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local perform=original(calcs.perform,"Modules/CalcPerform.lua",1193)
local children=original(calcs.createMinionSkills,"Modules/CalcActiveSkill.lua",1116)
local create=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local mods=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local stats=original(calcLib.buildSkillInstanceStats,"Modules/CalcTools.lua",161)
local validate=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local function typeNames(types)
 local result={}
 for name,id in pairs(SkillType) do
  if type(name)=="string" and types and types[id] then result[#result+1]=name end
 end
 table.sort(result)
 return result
end
local function effectIds(effects)
 local result={}
 for _,effect in ipairs(effects) do result[#result+1]=effect.grantedEffect.id end
 return result
end
local function capture(summoner)
 local actor=assert(summoner.minion,"summoner must have a constructed actor")
 local effect=summoner.activeEffect
 local out={summoning_id=effect.grantedEffect.id,summoning_effect_level=effect.level,
  physical_level=effect.srcInstance and effect.srcInstance.level,
  physical_quality=effect.srcInstance and effect.srcInstance.quality,
  actor_type=actor.type,actor_level=actor.level,
  selected_ability=actor.mainSkill and actor.mainSkill.activeEffect.grantedEffect.id,children={},
  source_types=typeNames(effect.grantedEffect.skillTypes),final_types=typeNames(summoner.skillTypes),
  minion_types=typeNames(summoner.minionSkillTypes),
  table_actor_level=data.minionLevelTable[effect.level],
  alternate_level_policy=not not (summoner.skillData.minionLevelIsEnemyLevel or summoner.skillData.minionLevelIsTriggeredSkillLevel or summoner.skillData.minionLevelIsPlayerLevel or summoner.skillData.minionLevel),
  retained_supports=effectIds(summoner.supportList),admitted_effects=effectIds(summoner.effectList)}
 for _,child in ipairs(assert(actor.activeSkillList)) do
  local e=child.activeEffect
  local row={id=e.grantedEffect.id,effect_level=e.level,quality=e.quality,actor_level=e.actorLevel,
   has_physical_source=e.srcInstance~=nil,has_gem_data=e.gemData~=nil,
   same_actor=child.actor==actor,same_summoner=child.summonSkill==summoner,effect_levels={},stat_sets={},
   source_types=typeNames(e.grantedEffect.skillTypes),final_types=typeNames(child.skillTypes),
   retained_supports=effectIds(child.supportList),admitted_effects=effectIds(child.effectList)}
  for level,value in ipairs(e.grantedEffect.levels) do
   row.effect_levels[#row.effect_levels+1]={key=level,requirement=value.levelRequirement}
  end
  for index,set in ipairs(e.grantedEffect.statSets) do
   local entry={index=index,label=set.label,levels={},raw_stats=stats(e,e.grantedEffect,set,false)}
   for level,value in ipairs(set.levels) do
    entry.levels[#entry.levels+1]={key=level,actor_level=value.actorLevel,interpolation=value.statInterpolation}
   end
   row.stat_sets[#row.stat_sets+1]=entry
  end
  out.children[#out.children+1]=row
 end
 return out
end
local function observed(f)
 local ok,value=pcall(f)
 if ok then return {status="ok",value=value} end
 return {status="error",error=tostring(value)}
end
"#;

const OBSERVATION: &str = r#"
local result={untouched_original=observed(function()
 return capture(build.calcsTab.mainEnv.player.mainSkill)
end),component_probes={}}
local savedGroups,savedMain=build.skillsTab.socketGroupList,build.mainSocketGroup
for _,family in ipairs({{name="sniper",skill="SummonSkeletalSnipersPlayer"},{name="storm_mage",skill="SummonSkeletalStormMagesPlayer"}}) do
 for _,case in ipairs({{level=1,quality=20},{level=20,quality=20},{level=40,quality=20},{level=100,quality=20},{level=40,quality=0}}) do
  local probe=observed(function()
   local effect=assert(data.skills[family.skill])
   local gem=assert(data.gems[assert(data.gemForSkill[effect])])
   local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
   loadSkill(tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib={gemId=gem.gameId,variantId=gem.variantId,
    level="20",quality=tostring(case.quality),corrupted="false",corruptLevel="0"}}},1)
   local group=assert(tab.skillSets[1].socketGroupList[1]);local physical=assert(group.gemList[1])
   build.skillsTab.socketGroupList={group};build.mainSocketGroup=1
   -- Establish the actor and its ModDB with the complete original lifecycle.
   -- No accelerated specEnv is passed; JIT mode is not cache warmness.
   local env=init(build,"MAIN")
   perform(env,true)
   local summoner
   for _,skill in ipairs(env.player.activeSkillList) do
    if skill.activeEffect.srcInstance==physical and skill.activeEffect.grantedEffect==effect then
     assert(not summoner,"ambiguous summoner source");summoner=skill
    end
   end
   assert(summoner and summoner.minion,"expected constructed family actor")
   local previousLevel=summoner.minion.level
   -- This controlled component input deliberately does not rebuild actor weapon
   -- or defence baselines. Only ability inputs/raw skill-stat scaling are claimed.
   summoner.minion.level=case.level
   children(env,summoner) -- Includes unchanged complete buildActiveSkillModList.
   local output=capture(summoner);output.actor_level_before_probe=previousLevel
   assert(calcs.createMinionSkills==children and calcs.buildActiveSkillModList==mods)
   assert(calcs.createActiveSkill==create and calcLib.buildSkillInstanceStats==stats)
   assert(calcLib.validateGemLevel==validate and build.skillsTab.ProcessSocketGroup==process)
   return output
  end)
  probe.family=family.name;probe.requested_actor_level=case.level;probe.parent_quality=case.quality
  result.component_probes[#result.component_probes+1]=probe
 end
end
build.skillsTab.socketGroupList=savedGroups;build.mainSocketGroup=savedMain
return result
"#;

const CLERIC_OBSERVATION: &str = r#"
local function clericCapture(summoner,env)
 local out=capture(summoner)
 out.declared_abilities={}
 for _,id in ipairs(assert(summoner.minion.minionData.skillList)) do
  local emitted=false
  for _,child in ipairs(summoner.minion.activeSkillList) do
   if child.activeEffect.grantedEffect.id==id then assert(not emitted);emitted=true end
  end
  out.declared_abilities[#out.declared_abilities+1]={id=id,loaded_definition=env.data.skills[id]~=nil,emitted=emitted}
 end
 out.child_inherits_same_support_list=summoner.minion.activeSkillList[1].supportList==summoner.supportList
 return out
end
local function findSummoner(env,physical)
 local found
 for _,skill in ipairs(env.player.activeSkillList) do
  if skill.activeEffect.srcInstance==physical and skill.activeEffect.grantedEffect.id=="SummonSkeletalClericsPlayer" then
   assert(not found,"ambiguous Cleric source");found=skill
  end
 end
 return assert(found,"Cleric physical source is not in activeSkillList")
end
local result={untouched_original=observed(function()
 local env=build.calcsTab.mainEnv
 local groupIndex,group,physical
 for index,candidate in ipairs(build.skillsTab.socketGroupList) do
  for _,gem in ipairs(candidate.gemList) do
   if gem.gemData and gem.gemData.grantedEffect.id=="SummonSkeletalClericsPlayer" then
    assert(not physical,"ambiguous saved Cleric group");groupIndex=index;group=candidate;physical=gem
   end
  end
 end
 assert(physical and group)
 local summoner=findSummoner(env,physical)
 return {main_socket_group=build.mainSocketGroup,main_skill=env.player.mainSkill.activeEffect.grantedEffect.id,
  supporting_group=groupIndex,group_enabled=not not group.enabled,physical_enabled=not not physical.enabled,
  is_main_skill=summoner==env.player.mainSkill,cleric=clericCapture(summoner,env)}
end),component_probes={}}
local savedGroups,savedMain=build.skillsTab.socketGroupList,build.mainSocketGroup
for _,case in ipairs({{level=1,quality=20},{level=19,quality=20},{level=20,quality=20},{level=40,quality=20},{level=19,quality=0}}) do
 local probe=observed(function()
  local effect=assert(data.skills.SummonSkeletalClericsPlayer)
  local gem=assert(data.gems[assert(data.gemForSkill[effect])])
  local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
  loadSkill(tab,{elem="Skill",attrib={enabled="true"},{elem="Gem",attrib={gemId=gem.gameId,variantId=gem.variantId,
   level=tostring(case.level),quality=tostring(case.quality),corrupted="false",corruptLevel="0"}}},1)
  local group=assert(tab.skillSets[1].socketGroupList[1]);local physical=assert(group.gemList[1])
  build.skillsTab.socketGroupList={group};build.mainSocketGroup=1
  -- Labelled component input: physical gem level/quality only. Full original
  -- initialization derives effective level, actor level and all child inputs.
  -- No environment, derived value, source function or data row is fabricated.
  local env=init(build,"MAIN")
  perform(env,true)
  local output=clericCapture(findSummoner(env,physical),env)
  assert(calcs.createMinionSkills==children and calcs.buildActiveSkillModList==mods)
  assert(calcs.createActiveSkill==create and calcLib.buildSkillInstanceStats==stats)
  assert(calcLib.validateGemLevel==validate and build.skillsTab.ProcessSocketGroup==process)
  return output
 end)
 probe.physical_level=case.level;probe.physical_quality=case.quality
 result.component_probes[#result.component_probes+1]=probe
end
build.skillsTab.socketGroupList=savedGroups;build.mainSocketGroup=savedMain
assert(build.calcsTab.mainEnv.player.mainSkill.activeEffect.grantedEffect.id=="SummonSandDjinnPlayer")
return result
"#;
