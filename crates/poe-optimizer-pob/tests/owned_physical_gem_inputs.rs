//! Complete authenticated source loading/setup methods establish physical gem
//! input observations, not owned support delivery or complete-build parity.
//! JIT mode is varied separately from source environment/cache reuse: every
//! calculation contrast below calls initEnv without an accelerated environment.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::Value as Json;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_original_gem_load_and_setup_keep_corruption_inputs_independent() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-physical-gem-inputs-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os("POE_PHYSICAL_GEM_INPUT_SOURCE_CHILD") {
        let jit_enabled = mode == "on";
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("physicalGemJitEnabled", jit_enabled)?;
            lua.load("if physicalGemJitEnabled then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let result = source::observe_with_build_hook_unwrapped(
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
        fs::write(
            destination.join(if jit_enabled {
                "jit-on.json"
            } else {
                "jit-off.json"
            }),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    for mode in ["off", "on"] {
        let log_path = destination.join(format!("jit-{mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "complete_original_gem_load_and_setup_keep_corruption_inputs_independent",
                "--nocapture",
            ])
            .env("POE_PHYSICAL_GEM_INPUT_SOURCE_CHILD", mode)
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
    let off: Json =
        serde_json::from_slice(&fs::read(destination.join("jit-off.json")).unwrap()).unwrap();
    let on: Json =
        serde_json::from_slice(&fs::read(destination.join("jit-on.json")).unwrap()).unwrap();
    assert_eq!(off["additional_observation"], on["additional_observation"]);
}

fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-physical-gem-input-source-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}

const OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S")
 local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line,
  "unexpected complete source function: "..path..":"..tostring(info.linedefined))
 return f
end
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
original(calcs.initEnv,"Modules/CalcSetup.lua",717)
original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local function fields(gem)
 return {level=gem.level,quality=gem.quality,corrupted=gem.corrupted,corrupt_level=gem.corruptLevel,
  gem_id=gem.gemId,skill_id=gem.skillId}
end
local function load(attributes)
 local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
 local before,after
 -- Observation only; the complete original ProcessSocketGroup still executes,
 -- including original errors. The real shared tab/source methods are untouched.
 function tab:ProcessSocketGroup(group)
  before=fields(group.gemList[1])
  local result=process(self,group)
  after=fields(group.gemList[1])
  return result
 end
 local node={elem="Skill",attrib={enabled="true"},{elem="Gem",attrib=attributes}}
 local ok,err=pcall(loadSkill,tab,node,1)
 local group=tab.skillSets[1].socketGroupList[1]
 assert((group~=nil)==ok)
 return {ok=ok,before=before,after=after,error=not ok and tostring(err) or nil},group
end
local observed={load_cases={},catalog={},setup_cases={}}
local function remember(label,attributes)
 local row,group=load(attributes)
 row.label=label
 observed.load_cases[#observed.load_cases+1]=row
 return row,group
end
local kinds={"SparkPlayer","SupportRapidCastingPlayerTwo"}
for _,skill in ipairs(kinds) do
 local isSupport=data.skills[skill].support==true
 for _,flag in ipairs({{label="missing"},{label="true",value="true"},{label="false",value="false"},
  {label="nil",value="nil"},{label="malformed",value="TRUE"}}) do
  for _,delta in ipairs({{label="missing",value=nil,expected=0},{label="nil",value="nil",expected=0},
   {label="malformed",value="bad",expected=0},{label="zero",value="0",expected=0},
   {label="positive",value="1",expected=1},{label="negative",value="-1",expected=-1},
   {label="fractional",value="0.5",expected=0.5}}) do
   local row=remember(skill.."/"..flag.label.."/"..delta.label,
    {skillId=skill,level=isSupport and "1" or "20",quality="20",corrupted=flag.value,corruptLevel=delta.value})
   assert(row.ok and row.before and row.after)
   assert(row.before.corrupted==(flag.value=="true") and row.after.corrupted==row.before.corrupted)
   assert(row.before.corrupt_level==delta.expected and row.after.corrupt_level==delta.expected)
   assert(row.after.level==(isSupport and 1 or 20) and row.after.quality==20)
  end
 end
 for _,case in ipairs({{label="zero",value="0",expected=1},
  {label="negative",value="-3",expected=1},{label="large",value="1000000",expected=isSupport and 1 or 40},
  {label="fractional",value="1.5",expected=isSupport and 1 or 20}}) do
  local row=remember(skill.."/level/"..case.label,{skillId=skill,level=case.value,quality="0"})
  assert(row.ok and row.after.level==case.expected,skill.." "..case.label)
 end
 for _,case in ipairs({{label="missing"},{label="malformed",value="bad"},{label="nil",value="nil"}}) do
  local row=remember(skill.."/level/"..case.label,{skillId=skill,level=case.value,quality="0"})
  assert(not row.ok and row.before and row.before.level==nil and row.after==nil)
 end
 for _,case in ipairs({{label="missing"},{label="malformed",value="bad"},{label="nil",value="nil"},
  {label="zero",value="0",expected=0},{label="twenty",value="20",expected=20},
  {label="negative",value="-2",expected=-2},{label="fractional",value="0.5",expected=0.5},
  {label="beyond-ui",value="100",expected=100}}) do
  local row=remember(skill.."/quality/"..case.label,
   {skillId=skill,level=isSupport and "1" or "20",quality=case.value})
  assert(row.ok and row.after.quality==case.expected)
 end
end

-- Catalog inputs are enumerated from the fully constructed source, independently
-- of provider schema completeness. Physical identity aliases remain separate.
local names={}
for id,gem in pairs(data.gems) do
 if gem.grantedEffect.support and #gem.grantedEffectList==1 then names[#names+1]=id end
end
table.sort(names)
assert(#names==515)
local singlePotential=0
for _,id in ipairs(names) do
 local gem=data.gems[id]
 local effect=gem.grantedEffect
 local excluded=gem.additionalGrantedEffectId1~=nil
 if excluded then
  assert(id=="Metadata/Items/Gems/SkillGemConcussiveRunesSupport")
  assert(gem.additionalGrantedEffectId1=="ConcussiveRunesPlayer" and data.skills.ConcussiveRunesPlayer==nil)
 else singlePotential=singlePotential+1 end
 assert(gem.additionalGrantedEffectId2==nil and gem.additionalStatSet1==nil)
 assert(gem.naturalMaxLevel==1 and #effect.levels==1 and effect.levels[1]~=nil)
 local keys=0
 for key in pairs(effect.levels) do assert(key==1);keys=keys+1 end
 assert(keys==1 and not effect.hideFromSideBar)
 local rows={}
 for _,quality in ipairs({0,20}) do
  local row,group=load({gemId=gem.gameId,variantId=gem.variantId,level="1",quality=tostring(quality),
   corrupted="false",corruptLevel="0"})
  assert(row.ok and row.after.level==1 and row.after.quality==quality)
  -- Duplicate external IDs can resolve another physical definition. Retain that
  -- fact as evidence; this sweep does not grant every source spelling identity.
  rows[#rows+1]={quality=quality,resolved_id=group.gemList[1].gemData.id,
   exact_identity=group.gemList[1].gemData==gem}
 end
 observed.catalog[#observed.catalog+1]={id=id,effect=effect.id,natural_max_level=gem.naturalMaxLevel,
  level_keys=keys,excluded_by_declared_additional_effect=excluded,rows=rows}
end

assert(singlePotential==514)

-- The untouched Twister build is an environment carrier. Only its skill groups
-- are replaced with explicitly loaded component inputs. No claim is made about
-- the original build's selected action or final damage/defence totals.
local savedGroups=build.skillsTab.socketGroupList
local savedMain=build.mainSocketGroup
for _,case in ipairs({
 {label="false-zero",flag="false",delta="0",expected=20,present=false},
 {label="true-zero",flag="true",delta="0",expected=20,present=true},
 {label="false-positive",flag="false",delta="1",expected=21,present=false},
 {label="true-positive",flag="true",delta="1",expected=21,present=true},
 {label="false-negative",flag="false",delta="-1",expected=19,present=false},
 {label="true-negative",flag="true",delta="-1",expected=19,present=true},
 {label="false-clamped",flag="false",delta="-50",expected=1,present=false},
 {label="true-from-item",flag="true",delta="1",expected=21,present=false,origin="fromItem"},
 {label="true-from-tree",flag="true",delta="1",expected=21,present=false,origin="fromTree"},
}) do
 local row,group=load({skillId="SparkPlayer",level="20",quality="20",corrupted=case.flag,corruptLevel=case.delta})
 local supportRow,supportGroup=load({skillId="SupportRapidCastingPlayerTwo",level="1",quality="0",
  corrupted="true",corruptLevel="4"})
 assert(row.ok and supportRow.ok)
 local gem=group.gemList[1]
 if case.origin then gem[case.origin]=true end
 local support=supportGroup.gemList[1]
 group.gemList[2]=support
 build.skillsTab.socketGroupList={group}
 build.mainSocketGroup=1
 local env=calcs.initEnv(build,"MAIN")
 local active=env.player.mainSkill
 assert(active.activeEffect.srcInstance==gem and active.activeEffect.grantedEffect.id=="SparkPlayer")
 assert(active.activeEffect.level==case.expected,case.label.." level "..active.activeEffect.level)
 assert(gem.level==20 and gem.corruptLevel==tonumber(case.delta))
 assert(support.supportEffect and support.supportEffect.level==1 and support.level==1)
 assert(support.supportEffect.srcInstance==support and support.corruptLevel==4 and support.corrupted)
 local records={}
 for _,mod in ipairs(active.skillModList) do
  if mod.name=="GemCorruptionLevel" and mod.source=="Corruption" then
   records[#records+1]={name=mod.name,type=mod.type,value=mod.value,flags=mod.flags,keyword_flags=mod.keywordFlags,tags=#mod}
  end
 end
 assert(#records==(case.present and 1 or 0),case.label.." corruption count")
 if case.present then
  assert(records[1].type=="BASE" and records[1].value==tonumber(case.delta))
  assert(records[1].flags==0 and records[1].keyword_flags==0 and records[1].tags==0)
 end
 assert((active.skillCfg.skillCond.GemCorrupted==true)==case.present)
 observed.setup_cases[#observed.setup_cases+1]={label=case.label,raw_level=gem.level,
  active_level=active.activeEffect.level,support_level=support.supportEffect.level,
  corruption_condition=active.skillCfg.skillCond.GemCorrupted==true,corruption_records=records}
end
build.skillsTab.socketGroupList=savedGroups
build.mainSocketGroup=savedMain
return observed
"#;
