//! Authenticated original support preparation, modifier construction and transfer.
//! The host supplies empty surroundings, not a complete imported build.
#[path = "item_loading_runtime.rs"]
mod runtime;
use mlua::{Function, Lua, Table, Value};

pub fn oracle(warm: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.load(r#"
local originalRequire=require
outputCalcs=LoadModule("Modules/CalcBase")
require=function(name)
 if name=="Modules.CalcBase" then return outputCalcs end
 return originalRequire(name)
end
LoadModule("Modules/CalcSetup")
LoadModule("Modules/CalcActiveSkill")
LoadModule("Modules/CalcPerform")
require=originalRequire
function outputInstance(id)
 local effect=assert(data.skills[id],id)
 local gem=data.gemForSkill[effect]
 return {grantedEffect=effect,gemData=gem and data.gems[gem],level=1,quality=0,
  actorLevel=90,statSet={index=1},statSetCalcs={index=1},isSupporting={},srcInstance={level=1}}
end
function outputEnvironment()
 local enemy={modDB=new("ModDB"):ModDB()}
 local player={enemy=enemy,modDB=new("ModDB"):ModDB(),itemList={},
  weaponData1={type="Spear"},weaponData2={},level=90}
 enemy.player=player
 return {mode="MAIN",data=data,configInput={},configPlaceholder={},modDB=player.modDB,
  player=player,enemy=enemy,auxSkillList={},build={characterLevel=90},enemyLevel=90}
end
function outputObserve(id,supportIds,controlledTypes)
 local env=outputEnvironment()
 local selected={}
 for _,supportId in ipairs(supportIds) do outputSelect(outputInstance(supportId),selected,"MAIN") end
 local instance=outputInstance(id)
 if controlledTypes then
  instance.grantedEffect=copyTable(instance.grantedEffect,true)
  instance.grantedEffect.skillTypes=controlledTypes
 end
 local active=outputCalcs.createActiveSkill(instance,selected,env,env.player)
 env.player.mainSkill=active
 outputCalcs.buildActiveSkillModList(env,active)
 return {env=env,active=active,selected=selected}
end
function outputCleric(supportIds)
 local result=outputObserve("SummonSkeletalClericsPlayer",supportIds)
 local minion=assert(result.active.minion)
 minion.modDB=new("ModDB"):ModDB()
 outputTransfer(result.active.skillModList,result.active.skillCfg,minion)
 outputCalcs.createMinionSkills(result.env,result.active)
 return result
end
function outputConstants(id)
 local result={}
 for _,row in ipairs(assert(data.skills[id]).statSets[1].constantStats) do result[row[1]]=row[2] end
 return result
end
"#).set_name("@owned-support-output-observation-host").exec().unwrap();
    let calcs: Table = oracle.lua.globals().get("outputCalcs").unwrap();
    for (owner, local, target, line) in [
        ("initEnv", "addBestSupport", "outputSelect", 586),
        ("perform", "addMinionModifiers", "outputTransfer", 1161),
    ] {
        let function = original_upvalue(&oracle.lua, &calcs.get(owner).unwrap(), local);
        let expected = if owner == "initEnv" {
            "@src/Modules/CalcSetup.lua"
        } else {
            "@src/Modules/CalcPerform.lua"
        };
        assert_eq!(function.info().source.as_deref(), Some(expected));
        assert_eq!(function.info().line_defined, Some(line));
        oracle.lua.globals().set(target, function).unwrap();
    }
    for (name, line) in [
        ("createActiveSkill", 144),
        ("buildActiveSkillModList", 426),
        ("createMinionSkills", 1116),
    ] {
        let function: Function = calcs.get(name).unwrap();
        assert_eq!(
            function.info().source.as_deref(),
            Some("@src/Modules/CalcActiveSkill.lua")
        );
        assert_eq!(function.info().line_defined, Some(line));
    }
    oracle.lua.globals().set("outputWarm", warm).unwrap();
    oracle
        .lua
        .load("if outputWarm then jit.on() else jit.off();jit.flush() end")
        .exec()
        .unwrap();
    oracle
}

fn original_upvalue(lua: &Lua, function: &Function, requested: &str) -> Function {
    let mut found = None;
    for index in 1..=i32::from(function.info().num_upvalues) {
        // SAFETY: copy one rooted existing closure upvalue without changing it.
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
    found.expect("original named closure")
}
