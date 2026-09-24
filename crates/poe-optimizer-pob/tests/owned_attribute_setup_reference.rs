//! Optional complete-source setup observations, not native evaluator parity.
//! The existing headless bootstrap supplies real Build/UI objects; no setup,
//! attribute, parser, store, or cache function is replaced or reimplemented.
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
fn original_setup_cache_partition_and_candidate_order_are_observed_in_real_environments() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/owned-attribute-setup-01");
    fs::create_dir_all(&destination).unwrap();
    if let Some(jit_mode) = std::env::var_os("POE_ATTRIBUTE_SETUP_SOURCE_CHILD") {
        let jit_enabled = jit_mode == "on";
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let before = |lua: &Lua| {
            lua.globals().set("attributeSetupJitEnabled", jit_enabled)?;
            lua.load("if attributeSetupJitEnabled then jit.on() else jit.off(); jit.flush() end")
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
            Some(&observe_setup),
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
    for jit_mode in ["off", "on"] {
        let log_path = destination.join(format!("jit-{jit_mode}.log"));
        let log = fs::File::create(&log_path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "original_setup_cache_partition_and_candidate_order_are_observed_in_real_environments",
                "--nocapture",
            ])
            .env("POE_ATTRIBUTE_SETUP_SOURCE_CHILD", jit_mode)
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
                    "source setup child failed: {}",
                    log_path.display()
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("source setup exceeded 180 seconds: {}", log_path.display());
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

fn observe_setup(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVATION)
        .set_name("@owned-attribute-setup-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}

const OBSERVATION: &str = r#"
local calcs=require("Modules.CalcBase")
local function sourceFunction(f,path,line)
 local info=debug.getinfo(f,"S")
 local actualSource=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actualSource:sub(-#path)==path,
  "unexpected source for "..path..": "..tostring(info.what).." "..info.source)
 if line then assert(info.linedefined==line,"unexpected line for "..path..": "..info.linedefined) end
 return f
end
sourceFunction(calcs.initEnv,"Modules/CalcSetup.lua",717)
sourceFunction(calcs.perform,"Modules/CalcPerform.lua",1193)
sourceFunction(wipeEnv,"Modules/CalcSetup.lua",466)
sourceFunction(specCopy,"Modules/Common.lua",542)
sourceFunction(mergeDB,"Modules/Common.lua",530)
local function upvalue(f,wanted)
 for index=1,100 do
  local name,value=debug.getupvalue(f,index)
  if not name then break end
  if name==wanted then return value end
 end
 error("missing complete original closure: "..wanted)
end
local actorAttributes=upvalue(calcs.perform,"doActorAttribsConditions")
local calculateAttributes=upvalue(actorAttributes,"calculateAttributes")
sourceFunction(actorAttributes,"Modules/CalcPerform.lua",264)
sourceFunction(calculateAttributes,"Modules/CalcPerform.lua",233)

-- Keep original build-02 as a complete carrier, then supply explicit component
-- inputs. These synthetic modifiers/items do not claim obtainable game rolls.
local originalConfig=build.configTab.modList
local config=new("ModList"):ModList()
config:AddList(originalConfig)
local marker="AttributeSetupConfigProbe"
config:NewMod("Str","BASE",100000,marker)
config:NewMod("Str","MORE",1,marker)
config:NewMod("Condition:AttributeSetupFlag","FLAG",true,marker)
config:NewMod("Condition:DexHigherThanStr","FLAG",true,marker)
build.configTab.modList=config
local function item(id,body)
 local result=new("Item")
 result.id=id
 result:ParseRaw("Rarity: RARE\nAttribute Setup Probe\nIron Ring\nItem Level: 80\nQuality: 0\nImplicits: 0\n"..body,nil,false)
 result:BuildModList()
 assert(result.baseName=="Iron Ring")
 return result
end
local first=item(900001,"+11 to Strength\n+11 to Strength\n+3 to Strength\n1% more Strength")
local second=item(900002,"+3 to Strength\n+11 to Strength\n+11 to Strength\n1% more Strength")
local function override(candidate,initial)
 return {repSlotName="Ring 1",repItem=candidate,conditions=initial and {"AttributeSetupOverride"} or nil}
end
local function ownRows(db,stat)
 local result={}
 for index,mod in ipairs(db.mods[stat] or {}) do
  result[#result+1]={index=index,name=mod.name,type=mod.type,value=mod.value,
   source=mod.source or "",flags=mod.flags,keyword_flags=mod.keywordFlags,tags=#mod}
 end
 return result
end
local function findSource(db,stat,wanted)
 local result={}
 for _,mod in ipairs(db.mods[stat] or {}) do
  if mod.source==wanted then result[#result+1]=mod end
 end
 return result
end
local function assertItemOrder(env,candidate,values,absent)
 assert(env.player.itemList["Ring 1"]==candidate)
 local mods=findSource(env.modDB,"Str",candidate.modSource)
 assert(#mods==4)
 for index=1,3 do
  assert(mods[index].type=="BASE" and mods[index].value==values[index])
 end
 assert(mods[1]~=mods[2] and mods[2]~=mods[3] and mods[1]~=mods[3])
 assert(mods[4].type=="MORE" and mods[4].value==1)
 if absent then assert(#findSource(env.modDB,"Str",absent.modSource)==0) end
 return ownRows(env.modDB,"Str")
end
local function groupSnapshot(env)
 return {local_rows=ownRows(env.modDB,"Str"),
  parent_rows=env.modDB.parent and ownRows(env.modDB.parent,"Str") or {},
  more=env.modDB:More(nil,"Str"),base=env.modDB:Sum("BASE",nil,"Str")}
end
local cold,parent,enemyParent,minionParent=calcs.initEnv(build,"CALCULATOR",override(first,true))
assert(not cold.modDB.parent and not parent.parent and parent.actor~=cold.player)
assert(cold.player.output==nil and cold.modDB:GetStat("Str")==0)
assert(#findSource(cold.modDB,"Str",marker)==2 and #findSource(parent,"Str",marker)==2)
assert(#findSource(parent,"Str",first.modSource)==0)
-- specCopy copies lists/conditions/multipliers, but keeps modifier identities.
assert(cold.modDB.mods.Str~=parent.mods.Str)
assert(findSource(cold.modDB,"Str",marker)[1]==findSource(parent,"Str",marker)[1])
assert(parent.conditions~=cold.modDB.conditions and parent.multipliers~=cold.modDB.multipliers)
assert(parent.actor.output==nil and parent:GetStat("Str")==0)
assert(cold.modDB:GetCondition("AttributeSetupOverride"))
assert(not parent:GetCondition("AttributeSetupOverride")) -- override follows specCopy
local coldRows=assertItemOrder(cold,first,{11,11,3},second)
local coldSnapshot=groupSnapshot(cold)

local function cached(existing,flags,candidate,initial)
 return calcs.initEnv(build,"CALCULATOR",override(candidate,initial),{
  cachedPlayerDB=parent,cachedEnemyDB=enemyParent,cachedMinionDB=minionParent,
  env=existing,accelerate=flags})
end
local hot=cached(nil,nil,first,true)
assert(hot~=cold and hot.player~=cold.player and hot.modDB.parent==parent)
assert(hot.player.output==nil and hot.modDB:GetStat("Str")==0)
assert(#findSource(hot.modDB,"Str",marker)==0)
assert(#findSource(hot.modDB.parent,"Str",marker)==2)
assertItemOrder(hot,first,{11,11,3},second)
local hotSnapshot=groupSnapshot(hot)
-- The fixture has no other Str MORE contributions at setup. The same two 1%
-- effects are rounded together cold, and in separate real cache groups hot.
local function moreCount(db)
 local count=0
 for _,mod in ipairs(db.mods.Str or {}) do if mod.type=="MORE" then count=count+1 end end
 return count
end
assert(moreCount(cold.modDB)==2 and moreCount(hot.modDB)==1 and moreCount(parent)==1)
assert(coldSnapshot.more==1.02 and hotSnapshot.more==1.01*1.01)
assert(coldSnapshot.more~=hotSnapshot.more)

-- These are queries on stores produced by real setup, not manually constructed
-- parents. Explicit false falls through, but cfg.overrideCond=false is final.
hot.modDB.conditions.Combat=false
assert(parent.conditions.Combat==true and hot.modDB:GetCondition("Combat"))
hot.modDB.conditions.AttributeSetupFlag=false
assert(hot.modDB:GetCondition("AttributeSetupFlag"))
assert(not hot.modDB:GetCondition("AttributeSetupFlag",nil,true))
assert(not hot.modDB:GetCondition("Combat",{overrideCond={Combat=false}}))
assert(not hot.modDB:GetCondition("AttributeSetupFlag",{overrideCond={AttributeSetupFlag=false}}))
assert(hot.modDB:GetStat("Str",{skillStats={Str=7}})==7)

local entries={}
local function performAndObserve(env,label)
 local oldHook=debug.gethook()
 assert(not oldHook,"observer requires an unused debug hook")
 local seen=0
 local function observer(event)
  if event~="call" then return end
  local info=debug.getinfo(2,"f")
  if info.func~=calculateAttributes then return end
  local name,db=debug.getlocal(2,1)
  assert(name=="modDB")
  if db~=env.modDB then return end
  seen=seen+1
  local entry={label=label,values={},conditions={}}
  for _,stat in ipairs({"Str","Dex","Int"}) do
   assert(db.actor.output[stat]==nil,"perform must clear reused attributes before S1")
   assert(db:GetStat(stat)==0)
   entry.values[stat]=db:GetStat(stat)
  end
  for _,condition in ipairs({"Combat","DexHigherThanStr","AttributeSetupFlag","AttributeSetupOverride","AttributeSetupStale"}) do
   entry.conditions[condition]={local_value=db.conditions[condition] or false,
    resolved=db:GetCondition(condition) or false}
  end
  assert(db.actor.output.AttributeSetupStale==nil)
  assert(db:GetCondition("DexHigherThanStr"),"C0 includes the actual cached/local config FLAG")
  entries[#entries+1]=entry
 end
 debug.sethook(observer,"c")
 local ok,message=pcall(calcs.perform,env,true)
 debug.sethook()
 assert(ok,message)
 assert(seen==1,"expected one player attribute stage in complete perform")
 return env.player.output
end
local coldOutput=performAndObserve(cold,"cold")
local hotOutput=performAndObserve(hot,"cached_fresh")
assert(coldOutput.Str>coldOutput.Dex and hotOutput.Str>hotOutput.Dex)
assert(cold.modDB.conditions.DexHigherThanStr==false and hot.modDB.conditions.DexHigherThanStr==false)
assert(cold.modDB:GetCondition("DexHigherThanStr") and hot.modDB:GetCondition("DexHigherThanStr"))
assert(hot.modDB:GetStat("Str",{skillStats={Str=7}})==hotOutput.Str)
hotOutput.AttributeSetupStale=123
hot.modDB.conditions.AttributeSetupStale=true
-- Testing zero versus absent is separate from deriving real final attributes.
hotOutput.AttributeSetupZero=0
assert(hot.modDB:GetStat("AttributeSetupZero",{skillStats={AttributeSetupZero=7}})==0)
assert(hot.modDB:GetStat("AttributeSetupMissing")==0)

-- Callers must declare unchanged dimensions truthfully. This deliberate wrong
-- flag retains A when asked for B, a cache invalidation witness, not a valid edit.
local stale=cached(hot,{nodeAlloc=true,requirementsItems=true,requirementsGems=true},second,false)
assert(stale==hot and stale.player.output==hotOutput)
assert(stale.modDB:GetStat("AttributeSetupStale")==123)
assert(not stale.modDB:GetCondition("AttributeSetupStale"))
assert(not stale.modDB:GetCondition("AttributeSetupOverride"))
assertItemOrder(stale,first,{11,11,3},second)
local staleRows=ownRows(stale.modDB,"Str")

local changed=cached(hot,{nodeAlloc=true,requirementsGems=true},second,false)
assert(changed==hot and changed.player.output==hotOutput)
local secondRows=assertItemOrder(changed,second,{3,11,11},first)
performAndObserve(changed,"reused_after_item_edit")
assert(changed.player.output~=hotOutput)
assert(changed.modDB:GetStat("AttributeSetupStale")==0)
local restored=cached(changed,{nodeAlloc=true,requirementsGems=true},first,false)
local restoredRows=assertItemOrder(restored,first,{11,11,3},second)
performAndObserve(restored,"reused_after_restore")
assert(restored.modDB.parent==parent and parent.actor.output==nil)

-- Complete Common helpers can be observed independently on the real stores.
local copied=specCopy({modDB=restored.modDB})
assert(not copied.parent and #findSource(copied,"Str",marker)==0)
assert(#findSource(copied,"Str",first.modSource)==4)
assert(copied:GetStat("Str")==0) -- specCopy does not copy actor output
build.configTab.modList=originalConfig
return {
 carrier="build-02.xml",scope="complete-source setup/component evidence; no native or original-build numeric parity",
 cold=coldSnapshot,cached_fresh=hotSnapshot,attribute_entries=entries,
 candidate_rows={cold=coldRows,incorrect_items_unchanged=staleRows,changed=secondRows,restored=restoredRows},
 actual_cache_partition=true,configuration_parent_is_not_parent_actor=true,
 stale_output_survives_setup_but_not_perform=true,incorrect_unchanged_flag_retains_old_item=true,
 spec_copy_is_local_only=true
}
"#;
