//! Complete authenticated source loading/setup methods establish physical gem
//! input observations, not owned support delivery or complete-build parity.
//! JIT mode is varied separately from source environment/cache reuse: every
//! calculation contrast below calls initEnv without an accelerated environment.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Lua, LuaSerdeExt, Value};
use poe_optimizer_core::{owned_build::ParameterValue, owned_content::digest_owned};
use poe_optimizer_data::skill_identities::SkillIdentityCatalog;
use poe_optimizer_import::{
    owned_value::OwnedValueCodec,
    owned_value_policy::{MissingValuePolicy, ValueLane, ValueRecipeInput},
};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::Value as Json;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_original_gem_load_and_setup_keep_corruption_inputs_independent() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-physical-gem-inputs-03");
    fs::create_dir_all(&destination).unwrap();
    if let Some(mode) = std::env::var_os("POE_PHYSICAL_GEM_INPUT_SOURCE_CHILD") {
        let jit_enabled = mode == "on";
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let (reviewed, policy) = reviewed_gems(&root);
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("physicalGemJitEnabled", jit_enabled)?;
            lua.globals()
                .set("reviewedPhysicalGemKeys", lua.to_value(&reviewed)?)?;
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
        assert_reviewed_scalar_conversions(&policy, &result["additional_observation"]);
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

// The reviewed conversion is authored input policy. Its selectors are checked
// independently against the complete identity catalog, then against actual
// fully constructed source objects below; names and external aliases are not
// used as physical identity authority.
fn reviewed_gems(root: &Path) -> (Vec<String>, Json) {
    let policy: Json = serde_json::from_slice(
        &fs::read(root.join("data/owned/poe2/3887ae68/support-gem-inputs/policy.json")).unwrap(),
    )
    .unwrap();
    let identities = SkillIdentityCatalog::new(
        serde_json::from_slice(
            &fs::read(root.join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(
        policy["catalog_digest"],
        serde_json::to_value(
            digest_owned(
                "owned-skill-source-catalog-v1",
                identities.data(),
                64 * 1024 * 1024,
            )
            .unwrap(),
        )
        .unwrap(),
    );
    assert_eq!(
        policy["level"],
        serde_json::json!({"minimum":1,"maximum":1})
    );
    let skills: BTreeMap<_, _> = identities
        .data()
        .skills
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect();
    let mut expected: Vec<_> = identities
        .data()
        .gems
        .iter()
        .filter(|gem| {
            let effect = skills[gem.primary_effect_id.as_str()];
            effect.support == Some(true)
                && effect.from_tree != Some(true)
                && gem.declared_additional_effects.is_empty()
                && gem.constructed_additional_effects.is_empty()
                && gem.additional_effects.is_empty()
                && gem.declared_additional_stat_sets.is_empty()
                && gem.effect_list == [gem.primary_effect_id.clone()]
        })
        .map(|gem| gem.key.clone())
        .collect();
    expected.sort();
    let selected: Vec<String> = serde_json::from_value(policy["source_gems"].clone()).unwrap();
    assert_eq!(selected, expected, "reviewed finite conversion selection");
    assert_eq!(selected.len(), 514);
    (selected, policy)
}

// An authenticated source run proves only the compared cases. The authored
// finite grammar intentionally does not inherit Lua's malformed/missing numeric
// fallback; known source values outside that grammar stay Pending during import.
fn assert_reviewed_scalar_conversions(policy: &Json, observation: &Json) {
    let mut codecs = BTreeMap::new();
    for parameter in policy["parameters"].as_array().unwrap() {
        let recipe: ValueRecipeInput = serde_json::from_value(parameter["value"].clone()).unwrap();
        assert!(matches!(recipe.missing, MissingValuePolicy::Pending));
        assert_eq!(recipe.tiers.len(), 1);
        assert_eq!(recipe.tiers[0].selectors.len(), 1);
        let selector = &recipe.tiers[0].selectors[0];
        assert_eq!(selector.lane, ValueLane::Attribute);
        let codec = OwnedValueCodec::new(recipe.codec, Default::default()).unwrap();
        assert!(codecs.insert(selector.name.clone(), codec).is_none());
    }
    assert_eq!(codecs.len(), 2);
    let flag = &codecs["corrupted"];
    let delta = &codecs["corruptLevel"];
    assert!(delta.decode("nil").is_err());
    assert!(delta.decode("bad").is_err());
    assert!(flag.decode("TRUE").is_err());
    let compare =
        |codec: &OwnedValueCodec, input: &str, output: &Json| match codec.decode(input).unwrap() {
            ParameterValue::Boolean(value) => assert_eq!(output.as_bool(), Some(value)),
            ParameterValue::Quantity(value) => assert_eq!(output.as_f64(), Some(value.value())),
            _ => panic!("unexpected physical Gem input value type"),
        };
    let mut flag_cases = 0;
    let mut delta_cases = 0;
    for row in observation["load_cases"].as_array().unwrap() {
        if row["ok"] != true {
            continue;
        }
        for (attribute, field, codec, count) in [
            ("corrupted", "corrupted", flag, &mut flag_cases),
            ("corruptLevel", "corrupt_level", delta, &mut delta_cases),
        ] {
            if let Some(input) = row["attributes"][attribute].as_str()
                && codec.decode(input).is_ok()
            {
                compare(codec, input, &row["before"][field]);
                compare(codec, input, &row["after"][field]);
                *count += 1;
            }
        }
    }
    assert_eq!((flag_cases, delta_cases), (42, 40));
    let mut physical_cases = 0;
    for row in observation["catalog"].as_array().unwrap() {
        if row["reviewed"] != true {
            continue;
        }
        for input in row["input_rows"].as_array().unwrap() {
            compare(
                flag,
                input["flag"].as_str().unwrap(),
                &input["loaded"]["corrupted"],
            );
            compare(
                delta,
                input["delta"].as_str().unwrap(),
                &input["loaded"]["corrupt_level"],
            );
            physical_cases += 1;
        }
    }
    assert_eq!(physical_cases, 514 * 3);
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
local reviewed={}; local reviewedCount=0
for _,id in ipairs(reviewedPhysicalGemKeys) do
 assert(not reviewed[id]);reviewed[id]=true;reviewedCount=reviewedCount+1
end
assert(reviewedCount==514)
local observed={load_cases={},catalog={},setup_cases={},reviewed_gems=reviewedCount}
local function remember(label,attributes)
 local row,group=load(attributes)
 row.label=label;row.attributes=attributes
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
local singlePotential=0;local reviewedSeen=0
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
 if reviewed[id] then
  reviewedSeen=reviewedSeen+1
  assert(gem.id==id and effect==data.skills[effect.id] and effect.support==true)
  assert(not excluded and gem.grantedEffectList[1]==effect and not effect.fromTree)
  for key in pairs(gem) do
   assert(not key:match("^additionalGrantedEffectId%d+$") and not key:match("^additionalStatSet%d+$"))
  end
 end
 local rows={}
 for _,quality in ipairs({0,20}) do
  local row,group=load({gemId=gem.gameId,variantId=gem.variantId,level="1",quality=tostring(quality),
   corrupted="false",corruptLevel="0"})
  assert(row.ok and row.after.level==1 and row.after.quality==quality)
  if reviewed[id] then
   assert(group.gemList[1].gemData==gem and row.after.gem_id==id)
   assert(row.before.corrupted==false and row.after.corrupted==false)
   assert(row.before.corrupt_level==0 and row.after.corrupt_level==0)
  end
  -- Duplicate external IDs can resolve another physical definition. Retain that
  -- fact as evidence; this sweep does not grant every source spelling identity.
  rows[#rows+1]={quality=quality,resolved_id=group.gemList[1].gemData.id,
   exact_identity=group.gemList[1].gemData==gem}
 end
 local inputRows={}
 if reviewed[id] then
  for _,case in ipairs({
   {flag="true",delta="-1",expected=-1},
   {flag="false",delta="0.5",expected=0.5},
   {flag="true",delta="0.5",expected=0.5},
  }) do
   local row,group=load({gemId=gem.gameId,variantId=gem.variantId,level="1",quality="20",
    corrupted=case.flag,corruptLevel=case.delta})
   assert(row.ok and group.gemList[1].gemData==gem and row.after.gem_id==id)
   assert(row.before.corrupted==(case.flag=="true") and row.after.corrupted==row.before.corrupted)
   assert(row.before.corrupt_level==case.expected and row.after.corrupt_level==case.expected)
   assert(row.after.level==1 and row.after.quality==20)
   inputRows[#inputRows+1]={flag=case.flag,delta=case.delta,loaded=fields(group.gemList[1])}
  end
 end
 observed.catalog[#observed.catalog+1]={id=id,effect=effect.id,natural_max_level=gem.naturalMaxLevel,
  level_keys=keys,excluded_by_declared_additional_effect=excluded,reviewed=reviewed[id]==true,
  rows=rows,input_rows=inputRows}
end

assert(singlePotential==514 and reviewedSeen==reviewedCount)

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
