//! Complete original attribute-stage functions with explicit component inputs.
//! These observations do not execute a full build or certify effective activation.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Lua, Table, Value};
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn original_upvalue(lua: &Lua, function: &Function, requested: &str) -> Function {
    let mut found = None;
    for index in 1..=i32::from(function.info().num_upvalues) {
        // SAFETY: read one valid existing upvalue of a rooted Function. Return
        // its copied name and value without changing source code or closure state.
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
    found.expect("complete authenticated original function")
}

fn oracle(warm: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.globals().set("attributesWarm", warm).unwrap();
    oracle
        .lua
        .load(
            r#"
local originalRequire=require
attributesCalcs=LoadModule("Modules/CalcBase")
require=function(name)
 if name=="Modules.CalcBase" then return attributesCalcs end
 return originalRequire(name)
end
LoadModule("Modules/CalcPerform")
require=originalRequire
function attributesActor(parent)
 local actor={output={},itemList={},weaponData1={type="None"},weaponData2={}}
 actor.modDB=new("ModDB"):ModDB(parent)
 actor.modDB.actor=actor
 return actor
end
function observeAttributeReads(actor)
 local reads={}
 local original=actor.modDB.GetStat
 actor.modDB.GetStat=function(self,stat,cfg)
  local result=original(self,stat,cfg)
  reads[#reads+1]={stat,result}
  return result
 end
 return reads
end
if attributesWarm then jit.on() else jit.off();jit.flush() end
"#,
        )
        .set_name("@original-actor-attribute-stage-inputs")
        .exec()
        .unwrap();
    let perform: Function = oracle
        .lua
        .globals()
        .get::<Table>("attributesCalcs")
        .unwrap()
        .get("perform")
        .unwrap();
    let full = original_upvalue(&oracle.lua, &perform, "doActorAttribsConditions");
    let calculate = original_upvalue(&oracle.lua, &full, "calculateAttributes");
    for (function, global, line) in [
        (full, "originalActorAttributes", 264),
        (calculate, "originalCalculateAttributes", 233),
    ] {
        assert_eq!(
            function.info().source.as_deref(),
            Some("@src/Modules/CalcPerform.lua")
        );
        assert_eq!(function.info().line_defined, Some(line));
        oracle.lua.globals().set(global, function).unwrap();
    }
    oracle
}

#[test]
fn zero_base_skips_increase_and_more_reads_and_signed_values_round_before_clamp() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local actor=attributesActor()
actor.output.Probe=2
actor.modDB:NewMod("Str","INC",5,"Test",{type="PerStat",stat="Probe"})
actor.modDB:NewMod("Str","MORE",10,"Test",{type="PerStat",stat="Probe"})
local reads=observeAttributeReads(actor)
originalCalculateAttributes(actor.modDB,actor.output,nil,actor.modDB.conditions)
assert(actor.output.Str==0 and actor.output.Dex==0 and actor.output.Int==0)
assert(#reads==0,"zero base must not evaluate INC or MORE tags")
actor.modDB:NewMod("Str","BASE",10,"Test")
originalCalculateAttributes(actor.modDB,actor.output,nil,actor.modDB.conditions)
assert(actor.output.Str==13 and #reads==4)
for _,read in ipairs(reads) do assert(read[1]=="Probe" and read[2]==2) end
local signed=attributesActor()
signed.modDB:NewMod("Str","BASE",-1.5,"Test")
signed.modDB:NewMod("Dex","BASE",2.5,"Test")
signed.modDB:NewMod("Int","BASE",3.4999,"Test")
assert(round(-1.5)==-1 and round(-2.5)==-2)
originalCalculateAttributes(signed.modDB,signed.output,nil,signed.modDB.conditions)
assert(signed.output.Str==0 and signed.output.Dex==3 and signed.output.Int==3)
"#,
            )
            .set_name("@source-attribute-lazy-reads-and-rounding")
            .exec()
            .unwrap();
    }
}

#[test]
fn attribute_values_preserve_multiplication_and_parent_more_rounding_boundaries() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local actor=attributesActor()
local db=actor.modDB
db:NewMod("Str","BASE",7,"Test")
db:NewMod("Str","INC",10,"Test")
db:NewMod("Str","MORE",10,"Test")
local base=db:Sum("BASE",nil,"Str")
local inc,more=calcLib.mods(db,nil,"Str")
local actual=calcLib.val(db,"Str")
assert(actual==base*(inc*more))
assert(actual~=(base*inc)*more,"the contrast must expose reassociation")
-- Each ModDB rounds its local MORE product before multiplying its parent's
-- result. Flattening the same modifiers into one database is a different input.
assert(not data.highPrecisionMods.Str)
local parent=new("ModDB"):ModDB()
parent:NewMod("Str","MORE",0.4,"Parent")
local child=attributesActor(parent)
child.modDB:NewMod("Str","BASE",100,"Test")
child.modDB:NewMod("Str","MORE",0.4,"Child")
local flat=attributesActor()
flat.modDB:NewMod("Str","BASE",100,"Test")
flat.modDB:NewMod("Str","MORE",0.4,"Parent")
flat.modDB:NewMod("Str","MORE",0.4,"Child")
assert(child.modDB:More(nil,"Str")==1)
assert(flat.modDB:More(nil,"Str")==1.01)
originalCalculateAttributes(child.modDB,child.output,nil,child.modDB.conditions)
originalCalculateAttributes(flat.modDB,flat.output,nil,flat.modDB.conditions)
assert(child.output.Str==100 and flat.output.Str==101)
"#,
            )
            .set_name("@source-attribute-arithmetic-boundaries")
            .exec()
            .unwrap();
    }
}

#[test]
fn cross_stat_reads_are_sequential_and_comparisons_refresh_between_exactly_two_passes() {
    for warm in [false, true] {
        oracle(warm)
            .lua
            .load(
                r#"
local actor=attributesActor()
actor.output.Str=0;actor.output.Dex=0;actor.output.Int=0
for _,entry in ipairs({{"Str",10,"Int"},{"Dex",20,"Str"},{"Int",30,"Dex"}}) do
 actor.modDB:NewMod(entry[1],"BASE",entry[2],"Test")
 actor.modDB:NewMod(entry[1],"BASE",1,"Test",{type="PerStat",stat=entry[3]})
end
local reads=observeAttributeReads(actor)
originalCalculateAttributes(actor.modDB,actor.output,nil,actor.modDB.conditions)
local expected={{"Int",0},{"Str",10},{"Dex",30},{"Int",60},{"Str",70},{"Dex",90}}
assert(#reads==#expected)
for index,row in ipairs(expected) do assert(reads[index][1]==row[1] and reads[index][2]==row[2]) end
assert(actor.output.Str==70 and actor.output.Dex==90 and actor.output.Int==120)
assert(actor.output.LowestAttribute==70 and actor.modDB.conditions.IntSingleHighestAttribute)
-- A second invocation advances this explicit feedback input again. The source
-- does two ordered passes, not convergence or simultaneous stat snapshots.
originalCalculateAttributes(actor.modDB,actor.output,nil,actor.modDB.conditions)
assert(actor.output.Str==190 and actor.output.Dex==210 and actor.output.Int==240)
local conditional=attributesActor()
conditional.modDB:NewMod("Str","BASE",10,"Test")
conditional.modDB:NewMod("Dex","BASE",20,"Test")
conditional.modDB:NewMod("Int","BASE",5,"Test")
conditional.modDB:NewMod("Str","BASE",100,"Test",{type="Condition",var="DexHigherThanInt"})
originalCalculateAttributes(conditional.modDB,conditional.output,nil,conditional.modDB.conditions)
assert(conditional.output.Str==110 and conditional.output.Dex==20 and conditional.output.Int==5)
assert(conditional.modDB.conditions.StrHighestAttribute)
assert(not conditional.modDB.conditions.DexHighestAttribute)
"#,
            )
            .set_name("@source-ordered-two-pass-attributes")
            .exec()
            .unwrap();
    }
}

#[test]
fn comparison_flags_distinguish_ties_and_single_highest_attributes() {
    for warm in [false, true] {
        oracle(warm).lua.load(r#"
local names={"Str","Dex","Int"}
for _,case in ipairs({
 {values={10,10,10},highest={true,true,true},single={false,false},tie=true},
 {values={30,20,10},highest={true,false,false},single={false,false},tie=false},
 {values={10,30,20},highest={false,true,false},single={false,true},tie=false},
 {values={10,20,30},highest={false,false,true},single={true,false},tie=false},
 {values={30,30,10},highest={true,true,false},single={false,false},tie=true},
 {values={10,30,30},highest={false,true,true},single={false,false},tie=true},
}) do
 local actor=attributesActor()
 for index,name in ipairs(names) do actor.modDB:NewMod(name,"BASE",case.values[index],"Test") end
 originalCalculateAttributes(actor.modDB,actor.output,nil,actor.modDB.conditions)
 local conditions=actor.modDB.conditions
 assert(actor.output.LowestAttribute==10 and conditions.TwoHighestAttributesEqual==case.tie)
 for index,name in ipairs(names) do
  assert(conditions[name.."HighestAttribute"]==case.highest[index])
  for other,otherName in ipairs(names) do
   if index~=other then assert(conditions[name.."HigherThan"..otherName]==(case.values[index]>case.values[other])) end
  end
 end
 assert(conditions.IntSingleHighestAttribute==case.single[1])
 assert(conditions.DexSingleHighestAttribute==case.single[2])
 assert(conditions.StrSingleHighestAttribute==nil,"source does not publish this extra condition")
end
"#).set_name("@source-attribute-comparison-flags").exec().unwrap();
    }
}

#[test]
fn full_original_actor_stage_emits_inherent_bonuses_with_explicit_gates() {
    for warm in [false, true] {
        oracle(warm).lua.load(r#"
assert(data.misc.AccuracyPerDexBase==6)
for _,case in ipairs({
 {flags={},expected={24,42,10}},
 {flags={"DoubledInherentAttributeBonuses"},expected={48,84,20}},
 {flags={"HalvesLifeFromStrength"},expected={12,42,10}},
 {flags={"DoubledInherentAttributeBonuses","HalvesLifeFromStrength"},expected={24,84,20}},
 {flags={"NoAttributeBonuses","DoubledInherentAttributeBonuses"},expected={0,0,0}},
 {flags={"NoStrengthAttributeBonuses"},expected={0,42,10}},
 {flags={"NoStrBonusToLife"},expected={0,42,10}},
 {flags={"NoDexterityAttributeBonuses"},expected={24,0,10}},
 {flags={"NoDexBonusToAccuracy"},expected={24,0,10}},
 {flags={"NoIntelligenceAttributeBonuses"},expected={24,42,0}},
 {flags={"NoIntBonusToMana"},expected={24,42,0}},
 {flags={},accuracy=0,expected={24,0,10}},
 {flags={"DoubledInherentAttributeBonuses"},accuracy=2.5,expected={48,35,20}},
}) do
 local actor=attributesActor()
 local db=actor.modDB
 for _,entry in ipairs({{"Str",12},{"Dex",7},{"Int",5},{"PresenceRadius",1},{"SurroundedRadius",1}}) do
  db:NewMod(entry[1],"BASE",entry[2],"Test")
 end
 for _,flag in ipairs(case.flags) do db:NewMod(flag,"FLAG",true,"Test") end
 if case.accuracy~=nil then db:NewMod("DexAccBonusOverride","OVERRIDE",case.accuracy,"Test") end
 -- Explicit unarmed, out-of-combat component input. Complete original function
 -- runs, including ordinary conditions and radius work; no UI is stubbed.
 originalActorAttributes({data=data,mode="MAIN",mode_combat=false,mode_effective=false,player=actor},actor)
 assert(actor.output.Str==12 and actor.output.Dex==7 and actor.output.Int==5 and actor.output.TotalAttr==24)
 for index,entry in ipairs({{"Life","Strength"},{"Accuracy","Dexterity"},{"Mana","Intelligence"}}) do
  assert(db:Sum("BASE",{source=entry[2]},entry[1])==case.expected[index])
 end
 if case.accuracy==0 then
  assert(#db.mods.Accuracy==1 and db.mods.Accuracy[1].value==0,"explicit zero override is not absence")
 end
 assert(actor.output.Life==nil and actor.output.Mana==nil and actor.output.Accuracy==nil)
end
"#).set_name("@source-full-actor-inherent-attribute-stage").exec().unwrap();
    }
}
