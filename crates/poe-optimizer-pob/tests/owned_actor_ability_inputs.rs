//! Complete-source minion ability input observations, not numerical/build parity.
//! The untouched original is captured before separately labelled actor-level probes.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str =
    "complete_source_actor_abilities_keep_effect_level_actor_level_and_quality_distinct";
const CHILD: &str = "POE_ACTOR_ABILITY_INPUT_SOURCE_CHILD";
const SOURCE_FILES: [&str; 10] = [
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
];

#[test]
fn complete_source_actor_abilities_keep_effect_level_actor_level_and_quality_distinct() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-actor-ability-inputs-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "on" || mode == "off");
        let enabled = mode == "on";
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("actorAbilityJitEnabled", enabled)?;
            lua.load("if actorAbilityJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let mut result = source::observe_with_build_hook_unwrapped(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&before),
            None,
            Some(&observe),
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
        assert_observations(&result["additional_observation"]);
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
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
fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
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

const OBSERVATION: &str = r#"
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
local function capture(summoner)
 local actor=assert(summoner.minion,"summoner must have a constructed actor")
 local effect=summoner.activeEffect
 local out={summoning_id=effect.grantedEffect.id,summoning_effect_level=effect.level,
  physical_level=effect.srcInstance and effect.srcInstance.level,
  physical_quality=effect.srcInstance and effect.srcInstance.quality,
  actor_type=actor.type,actor_level=actor.level,
  selected_ability=actor.mainSkill and actor.mainSkill.activeEffect.grantedEffect.id,children={}}
 for _,child in ipairs(assert(actor.activeSkillList)) do
  local e=child.activeEffect
  local row={id=e.grantedEffect.id,effect_level=e.level,quality=e.quality,actor_level=e.actorLevel,
   has_physical_source=e.srcInstance~=nil,has_gem_data=e.gemData~=nil,
   same_actor=child.actor==actor,same_summoner=child.summonSkill==summoner,effect_levels={},stat_sets={}}
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
