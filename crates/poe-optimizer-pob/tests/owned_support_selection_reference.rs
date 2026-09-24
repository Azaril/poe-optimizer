//! Selection precedes applicability in the pinned source. These optional tests
//! invoke complete original closures and do not grant native support coverage.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value};
use sha2::{Digest, Sha256};
#[path = "support/item_loading_runtime.rs"]
#[allow(dead_code)]
mod runtime;

fn oracle(warm: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.globals().set("sourceWarm", warm).unwrap();
    oracle
        .lua
        .load(
            r#"
local originalRequire=require
selectionCalcs=LoadModule("Modules/CalcBase")
require=function(name)
 if name=="Modules.CalcBase" then return selectionCalcs end
 return originalRequire(name)
end
LoadModule("Modules/CalcSetup")
LoadModule("Modules/CalcActiveSkill")
require=originalRequire
function selectionInput(id,level,quality)
 local effect=assert(data.skills[id],id)
 local gem=data.gemForSkill[effect]
 return {grantedEffect=effect,gemData=gem and data.gems[gem],
  level=level or 1,quality=quality or 0,superseded=false,isSupporting={}}
end
function selectionActive(id,supports,instance)
 local effect=selectionInput(id,instance and instance.level,instance and instance.quality)
 effect.statSet={index=1};effect.statSetCalcs={index=1}
 effect.srcInstance=instance
 local actor={enemy={}}
 actor.enemy.player=actor
 return selectionCalcs.createActiveSkill(effect,supports,{mode="MAIN"},actor)
end
if sourceWarm then jit.on() else jit.off();jit.flush() end
"#,
        )
        .set_name("@original-support-selection-closure-access")
        .exec()
        .unwrap();
    let init: Function = oracle
        .lua
        .globals()
        .get::<Table>("selectionCalcs")
        .unwrap()
        .get("initEnv")
        .unwrap();
    for (name, global, line) in [
        ("addBestSupport", "selectionBest", 586),
        ("processGrantedEffect", "selectionProcess", 630),
    ] {
        let function = original_upvalue(&oracle.lua, &init, name);
        assert_eq!(
            function.info().source.as_deref(),
            Some("@src/Modules/CalcSetup.lua")
        );
        assert_eq!(function.info().line_defined, Some(line));
        oracle.lua.globals().set(global, function).unwrap();
    }
    oracle
}

fn original_upvalue(lua: &Lua, function: &Function, requested: &str) -> Function {
    let mut found = None;
    for index in 1..=i32::from(function.info().num_upvalues) {
        // SAFETY: inspect one existing upvalue of a rooted Function at a valid
        // index. Return exactly the copied name and value, without mutating the
        // original closure or loading the unsafe Lua debug library.
        let (name, value): (String, Value) = unsafe {
            lua.exec_raw(function.clone(), |state| {
                let name = mlua::ffi::lua_getupvalue(state, 1, index);
                mlua::ffi::lua_pushstring(state, name);
                mlua::ffi::lua_insert(state, -2);
                mlua::ffi::lua_remove(state, 1);
            })
        }
        .unwrap();
        if name == requested {
            assert!(found.is_none());
            found = Some(value.as_function().unwrap().clone());
        }
    }
    found.expect("authenticated complete original closure")
}

#[test]
fn same_effect_priority_and_superseded_flags_depend_on_mode_and_encounter_order() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
for _,mode in ipairs({"MAIN","CALCS"}) do
 for _,id in ipairs({"SupportElementalArmamentPlayerTwo","SupportMeatShieldPlayerTwo"}) do
  local first=selectionInput(id,1,10)
  local equal=selectionInput(id,1,10)
  local higherQuality=selectionInput(id,1,20)
  -- Raw helper ranking input, not a claim that this support has a level 2 row.
  local higherLevel=selectionInput(id,2,0)
  local supports={}
  selectionBest(first,supports,mode)
  selectionBest(equal,supports,mode)
  assert(#supports==1 and supports[1]==first and equal.superseded)
  selectionBest(higherQuality,supports,mode)
  assert(supports[1]==higherQuality)
  assert(first.superseded==(mode=="MAIN"))
  selectionBest(higherLevel,supports,mode)
  assert(supports[1]==higherLevel)
  assert(higherQuality.superseded==(mode=="MAIN"))
  assert(not higherLevel.superseded)
  local lower=selectionInput(id,1,100)
  selectionBest(lower,supports,mode)
  assert(supports[1]==higherLevel and lower.superseded)
 end
end
-- Applicability is not a duplicate-selector. Passing two equal effects directly
-- into complete createActiveSkill retains both; CalcSetup is a separate stage.
local a=selectionInput("SupportElementalArmamentPlayerTwo")
local b=selectionInput("SupportElementalArmamentPlayerTwo")
local active=selectionActive("TwisterPlayer",{a,b},{})
assert(#active.effectList==3 and active.effectList[2]==a and active.effectList[3]==b)
"#,
            )
            .set_name("@source-same-effect-support-selection")
            .exec()
            .unwrap();
    }
}

#[test]
fn different_family_members_use_last_encounter_and_overlaps_can_repeat_one_effect() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
for _,mode in ipairs({"MAIN","CALCS"}) do
 for _,pair in ipairs({
  {"SupportElementalArmamentPlayer","SupportElementalArmamentPlayerTwo"},
  {"SupportMeatShieldPlayer","SupportMeatShieldPlayerTwo"},
 }) do
  for _,reverse in ipairs({false,true}) do
   local first=selectionInput(pair[reverse and 2 or 1],2,100)
   local last=selectionInput(pair[reverse and 1 or 2],1,0)
   assert(first.grantedEffect~=last.grantedEffect)
   assert(first.grantedEffect.gemFamily[1]==last.grantedEffect.gemFamily[1])
   local list={first};selectionBest(last,list,mode)
   assert(#list==1 and list[1]==last and not last.superseded)
   assert(first.superseded==(mode=="MAIN"))
  end
 end
 -- Real definitions, explicit authored combination rather than an original
 -- fixture claim: Salvo overlaps both existing families and replaces both slots.
 local multishot=selectionInput("SupportMultishotPlayer")
 local unleash=selectionInput("SupportUnleashPlayer")
 local salvo=selectionInput("SupportSalvoPlayer")
 local list={multishot,unleash};selectionBest(salvo,list,mode)
 assert(#list==2 and list[1]==salvo and list[2]==salvo)
 assert(multishot.superseded==(mode=="MAIN") and unleash.superseded==(mode=="MAIN"))
 local active=selectionActive("TwisterPlayer",list,{})
 assert(#active.effectList==3 and active.effectList[2]==salvo and active.effectList[3]==salvo)
end
"#,
            )
            .set_name("@source-family-support-selection")
            .exec()
            .unwrap();
    }
}

#[test]
fn synthetic_plus_version_inputs_expose_the_family_branch_precedence() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local function synthetic(id,plus,families)
 local effect=selectionInput("SupportElementalArmamentPlayerTwo")
 local original=effect.grantedEffect;effect.grantedEffect={}
 for key,value in pairs(original) do effect.grantedEffect[key]=value end
 effect.grantedEffect.id=id;effect.grantedEffect.plusVersionOf=plus
 effect.grantedEffect.gemFamily=families
 return effect
end
for _,mode in ipairs({"MAIN","CALCS"}) do
 for _,reverse in ipairs({false,true}) do
  local base=synthetic("test-base",nil,nil)
  local plus=synthetic("test-plus","test-base",nil)
  local list={reverse and plus or base}
  selectionBest(reverse and base or plus,list,mode)
  assert(#list==1 and list[1]==plus)
  assert(base.superseded==(reverse or mode=="MAIN"))
 end
 -- Both nonnil family lists enter the family elseif, even when no values
 -- overlap. That suppresses the plusVersionOf branches below it.
 local base=synthetic("test-base",nil,{"test-base-family"})
 local plus=synthetic("test-plus","test-base",{"test-plus-family"})
 local list={base};selectionBest(plus,list,mode)
 assert(#list==2 and list[1]==base and list[2]==plus)
 assert(not base.superseded and not plus.superseded)
 -- If the families overlap, even a later base replaces an earlier plus.
 base=synthetic("test-base",nil,{"test-shared"})
 plus=synthetic("test-plus","test-base",{"test-shared"})
 list={plus};selectionBest(base,list,mode)
 assert(#list==1 and list[1]==base)
end
"#,
            )
            .set_name("@synthetic-input-source-plus-version-precedence")
            .exec()
            .unwrap();
    }
}

#[test]
fn original_twister_and_cleric_rows_keep_selection_order_and_exact_effect_origins() {
    let directory = runtime::repository().join("tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("index.json")).unwrap()).unwrap();
    for warm in [false, true] {
        let oracle = oracle(warm);
        for number in [1, 2] {
            let xml =
                std::fs::read_to_string(directory.join(format!("build-{number:02}.xml"))).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(xml.as_bytes())),
                manifest["builds"][number - 1]["xml_sha256"]
                    .as_str()
                    .unwrap()
            );
            oracle.lua.globals().set("selectionXml", xml).unwrap();
            oracle
                .lua
                .globals()
                .set("selectionOriginal", number)
                .unwrap();
            oracle.lua.load(r#"
local roots,err=originalXml.ParseXML(selectionXml);assert(roots,err)
local skills
for _,child in ipairs(roots[1]) do if child.elem=="Skills" then skills=child end end
assert(skills)
local set
for _,child in ipairs(skills) do
 if child.elem=="SkillSet" and child.attrib.id==skills.attrib.activeSkillSet then set=child end
end
assert(set)
local target=selectionOriginal==1 and "SummonSkeletalClericsPlayer" or "TwisterPlayer"
local expected=selectionOriginal==1 and {
 "SupportLastGaspPlayer","SupportRapidCastingPlayerTwo",
 "SupportMeatShieldPlayerTwo","SupportElementalArmyPlayer",
} or {
 "SupportRetreatPlayerTwo","SupportElementalArmamentPlayerTwo",
 "SupportProjectileAccelerationPlayerThree","SupportSalvoPlayer",
 "ProlongedDurationSupportPlayerTwo",
}
local group
for _,candidate in ipairs(set) do
 if candidate.elem=="Skill" and candidate[1] and candidate[1].attrib.skillId==target then
  assert(not group);group=candidate
 end
end
assert(group and group.attrib.enabled=="true" and not group.attrib.source and not group.attrib.slot)
-- Inputs are the exact saved selected-group rows. Surrounding item, socket and
-- gem-property modifiers are explicitly empty component inputs, not whole-build
-- effective levels or final support-delivery authority.
for _,mode in ipairs({"MAIN","CALCS"}) do
 local list,instances={},{}
 local env={mode=mode,modDB=new("ModList"):ModList(),player={itemList={},modDB=new("ModList"):ModList()}}
 local cfg,processed={},{}
 for index,node in ipairs(group) do
  assert(node.elem=="Gem" and node.attrib.enabled=="true")
  local a=node.attrib
  local gem=assert(assert(data.gemsByGameId[a.gemId])[a.variantId])
  assert(gem.grantedEffect.id==a.skillId)
  local instance={gemData=gem,level=assert(tonumber(a.level)),quality=assert(tonumber(a.quality)),sourceRow=index}
  instances[index]=instance
  selectionProcess(gem.grantedEffect,instance,env,cfg,index,{},processed,{list})
  for _,additional in ipairs(gem.additionalGrantedEffects) do
   selectionProcess(additional,instance,env,cfg,index,{},processed,{list})
  end
 end
 assert(#list==#expected)
 for index,id in ipairs(expected) do
  local effect=list[index]
  assert(effect.grantedEffect.id==id and effect.srcInstance==instances[index+1])
  assert(effect.level==1 and effect.quality==0 and not effect.superseded)
  if mode=="MAIN" then assert(effect.srcInstance.supportEffect==effect and effect.srcInstance.displayEffect==effect)
  else assert(not effect.srcInstance.supportEffect and not effect.srcInstance.displayEffect) end
 end
 local active=selectionActive(target,list,instances[1])
 assert(#active.effectList==#expected+1)
 for index,effect in ipairs(list) do
  assert(active.effectList[index+1]==effect)
  assert(effect.isSupporting[instances[1]] and effect.activeSkillLevel==instances[1].level)
 end
 if selectionOriginal==1 then assert(active.skillTypes[SkillType.Duration])
 else assert(active.skillTypes[SkillType.HasSeals] and active.skillTypes[SkillType.SupportedBySalvo]) end
end
"#).set_name("@original-selected-group-support-selection").exec().unwrap();
        }
    }
}
