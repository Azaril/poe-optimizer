//! Complete authenticated Item methods with explicit item/component inputs.
//! This source-only evidence does not establish native assembly, equipped
//! activation, obtainable rolls, or complete-build parity. JIT mode is separate
//! from the fresh/reused item lifecycle exercised explicitly below.
#![cfg(not(target_arch = "wasm32"))]
use mlua::{Function, Table};
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn oracle(jit_enabled: bool) -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    // Bootstrap has already constructed unique items, so Common has wrapped the
    // class constructor. Reload the complete authenticated class module to
    // observe its original constructor before Common performs that wrapping.
    // No source method is sliced, rewritten, or substituted.
    oracle
        .lua
        .load(runtime::verified("src/Classes/Item.lua").unwrap())
        .set_name("@src/Classes/Item.lua")
        .exec()
        .unwrap();
    let original_constructor: Function = oracle
        .lua
        .load("return common.classes.Item.Item")
        .eval()
        .unwrap();
    assert_eq!(
        original_constructor.info().source.as_deref(),
        Some("@src/Classes/Item.lua")
    );
    assert_eq!(original_constructor.info().line_defined, Some(93));
    oracle
        .lua
        .globals()
        .set("defenceAssemblyJitEnabled", jit_enabled)
        .unwrap();
    oracle.lua.load(r#"
function defenceRaw(base,quality,body)
 return "Rarity: RARE\nDefence Assembly Probe\n"..base.."\nItem Level: 80\nQuality: "..quality.."\nImplicits: 0\n"..(body or "")
end
function defenceItem(base,quality,body)
 -- Execute the real constructor, including sanitiseText, ParseRaw and BuildModList.
 local item=new("Item"):Item(defenceRaw(base,quality,body),nil,false)
 assert(item.baseName==base and item.quality==quality)
 return item
end
function defenceComponent(profile,quality)
 -- Synthetic raw profile inputs on a fully constructed original Item. These
 -- isolate arithmetic and presence; they are not claimed obtainable item rolls.
 local item=defenceItem("Garment",quality,"")
 item.base=copyTable(item.base)
 item.base.armour=profile and copyTable(profile) or nil
 item.armourData=nil
 item:BuildModList()
 return item
end
function defenceList(rows)
 local list=new("ModList"):ModList()
 for _,row in ipairs(rows or {}) do list:NewMod(unpack(row)) end
 return list
end
function defenceSlot(item,rows,slot)
 local input=defenceList(rows)
 local output=item:BuildModListForSlotNum(input,slot)
 return output,input
end
function defenceSources(list)
 local result={}
 for _,mod in ipairs(list) do
  if mod.source and mod.source:sub(1,6)=="Probe:" then result[#result+1]=mod.source end
 end
 return table.concat(result,",")
end
function defenceNamed(list,name)
 local result={}
 for _,mod in ipairs(list) do if mod.name==name then result[#result+1]=mod end end
 return result
end
defenceMethodProbe=new("Item"):Item()
if defenceAssemblyJitEnabled then jit.on() else jit.off(); jit.flush() end
"#).set_name("@owned-defence-complete-method-inputs").exec().unwrap();
    let probe: Table = oracle.lua.globals().get("defenceMethodProbe").unwrap();
    let wrapped_constructor: Function = probe.get("Item").unwrap();
    assert_eq!(
        wrapped_constructor.info().source.as_deref(),
        Some("@src/Modules/Common.lua")
    );
    assert_eq!(wrapped_constructor.info().line_defined, Some(167));
    // The existing host only substitutes GUI control kinds in this dispatcher;
    // Item delegates to the original Common.new factory. The exercised Item
    // constructor closure and Common constructor wrapper are checked above.
    let new: Function = oracle.lua.globals().get("new").unwrap();
    assert_eq!(
        new.info().source.as_deref(),
        Some("@item-loading-test-observer")
    );
    assert_eq!(new.info().line_defined, Some(100));
    for (name, line) in [
        ("ParseRaw", 468),
        ("GetArmourDataValue", 2373),
        ("BuildModListForSlotNum", 2414),
        ("BuildModListsForSlots", 2671),
        ("BuildModList", 2694),
    ] {
        let method: Function = probe.get(name).unwrap();
        assert_eq!(
            method.info().source.as_deref(),
            Some("@src/Classes/Item.lua")
        );
        assert_eq!(method.info().line_defined, Some(line));
    }
    oracle
}

#[test]
fn actual_constructor_preserves_pure_hybrid_ward_empty_and_absent_profiles() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled).lua.load(r#"
for _,case in ipairs({
 {"Rusted Cuirass",0,{45,0,0,0}}, {"Rusted Cuirass",20,{54,0,0,0}},
 {"Adherent Cuffs",0,{98,0,27,0}}, {"Adherent Cuffs",20,{118,0,32,0}},
 {"Runeforged Adherent Cuffs",0,{39,0,11,134}},
 {"Runeforged Adherent Cuffs",20,{47,0,13,161}},
 {"Runefather's Grasping Mail",0,{0,0,0,550}},
 {"Runefather's Grasping Mail",20,{0,0,0,660}},
 {"Fists of Stone",0,{0,0,0,0}}, {"Garment",20,{0,0,0,0}},
}) do
 local item=defenceItem(case[1],case[2],"")
 assert(type(item.base.armour)=="table" and type(item.armourData)=="table")
 for index,name in ipairs({"Armour","Evasion","EnergyShield","Ward"}) do
  assert(item.armourData[name]==case[3][index],case[1].." "..name)
  assert(item.armourData[name.."Base"]==(item.base.armour[name] or 0))
  assert(item:GetArmourDataValue(name)==case[3][index])
 end
 local before=item.armourData
 item:BuildModList()
 assert(item.armourData==before)
 for index,name in ipairs({"Armour","Evasion","EnergyShield","Ward"}) do
  assert(item:GetArmourDataValue(name,100)==case[3][index])
 end
end
local absent=defenceItem("Iron Ring",0,"+11 to Armour")
assert(absent.base.armour==nil and absent.armourData==nil)
assert(absent:GetArmourDataValue("Armour",100)==0)
for slot=1,3 do
 local mods=defenceNamed(absent.slotModList[slot],"Armour")
 assert(#mods==1 and mods[1].type=="BASE" and mods[1].value==11)
end
-- Real parsed ordinary local lines exercise the whole assembly path, not only
-- synthetic modifier records supplied to the complete slot method.
local parsed=defenceItem("Rusted Cuirass",20,"+11 to Armour\n+7 to Evasion Rating\n+3 to Energy Shield\n25% increased Defences")
assert(parsed.armourData.Armour==84 and parsed.armourData.Evasion==11 and parsed.armourData.EnergyShield==5)
assert(#defenceNamed(parsed.modList,"Armour")==0 and #defenceNamed(parsed.modList,"Defences")==0)
"#).set_name("@source-defence-constructor-profile-presence").exec().unwrap();
    }
}

#[test]
fn local_hybrid_groups_quality_rounding_and_order_are_not_flattened() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled).lua.load(r#"
local item=defenceComponent({Armour=100,Evasion=200,EnergyShield=300,Ward=400},20)
local left,input=defenceSlot(item,{
 {"Armour","BASE",11,"Probe:A"},{"Evasion","BASE",13,"Probe:E"},
 {"EnergyShield","BASE",17,"Probe:S"},{"Ward","BASE",19,"Probe:W"},
 {"ArmourAndEvasion","BASE",2,"Probe:AE"},{"ArmourAndEnergyShield","BASE",3,"Probe:AS"},
 {"EvasionAndEnergyShield","BASE",5,"Probe:ES"},
 {"Armour","INC",10,"Probe:Ai"},{"Evasion","INC",20,"Probe:Ei"},
 {"EnergyShield","INC",30,"Probe:Si"},{"Ward","INC",40,"Probe:Wi"},
 {"ArmourAndEvasion","INC",7,"Probe:AEi"},{"ArmourAndEnergyShield","INC",11,"Probe:ASi"},
 {"EvasionAndEnergyShield","INC",13,"Probe:ESi"},{"Defences","INC",17,"Probe:Di"},
})
assert(item.armourData.Armour==202 and item.armourData.Evasion==414)
assert(item.armourData.EnergyShield==667 and item.armourData.Ward==789)
assert(item.armourData.ArmourBase==100 and item.armourData.EnergyShieldBase==300)
assert(#input==15 and defenceSources(left)=="") -- original input list is retained
for _,case in ipairs({{1,120},{0,144},{-1,144}}) do
 local q=defenceComponent({Armour=100},20)
 defenceSlot(q,{{"Armour","INC",20,"Probe:inc"},{"AlternateQualityArmour","BASE",case[1],"Probe:alt"}})
 assert(q.armourData.Armour==case[2])
 assert(q.quality==20)
end
local negative=defenceComponent({},0)
defenceSlot(negative,{{"Armour","BASE",-1.5,"Probe:signed"},{"Evasion","BASE",2.5,"Probe:tie"}})
assert(negative.armourData.Armour==-1 and negative.armourData.Evasion==3)
-- Local BASE folds before the raw base is added. This numeric witness would
-- differ if raw base were inserted at the beginning of one flattened fold.
local grouped=defenceComponent({Armour=1000000},0)
defenceSlot(grouped,{{"Armour","BASE",-1000000,"Probe:a"},{"Armour","BASE",0.49999999997,"Probe:b"}})
assert(grouped.armourData.Armour==1)
local first=defenceComponent({},0)
defenceSlot(first,{{"Armour","BASE",1000000,"Probe:a"},{"Armour","BASE",0.49999999997,"Probe:b"},{"Armour","BASE",-1000000,"Probe:c"}})
local second=defenceComponent({},0)
defenceSlot(second,{{"Armour","BASE",1000000,"Probe:a"},{"Armour","BASE",-1000000,"Probe:c"},{"Armour","BASE",0.49999999997,"Probe:b"}})
assert(first.armourData.Armour==1 and second.armourData.Armour==0)
"#).set_name("@source-defence-group-quality-and-rounding").exec().unwrap();
    }
}

#[test]
fn per_level_getter_rounds_separately_and_keeps_item_multiplication_order() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
local item=defenceComponent({Evasion=5,EnergyShield=5,Ward=5},10)
local left=defenceSlot(item,{
 {"EvasionPerLevel","BASE",0.5,"Probe:E"},
 {"EnergyShieldPerLevel","BASE",0.5,"Probe:S"},
 {"WardPerLevel","BASE",0.5,"Probe:W"},
 {"ArmourPerLevel","BASE",100,"Probe:A-not-assembled"},
})
for _,name in ipairs({"Evasion","EnergyShield","Ward"}) do
 assert(item.armourData[name]==6 and item.armourData[name.."PerLevel"]==0.55)
 assert(item:GetArmourDataValue(name)==6 and item:GetArmourDataValue(name,0)==6)
 assert(item:GetArmourDataValue(name,1)==7 and item:GetArmourDataValue(name,2)==7)
 assert(item:GetArmourDataValue(name,-1)==5)
end
assert(item.armourData.ArmourPerLevel==nil and item:GetArmourDataValue("Armour",100)==0)
assert(defenceSources(left)=="Probe:A-not-assembled")
local ordered=defenceComponent({},10)
defenceSlot(ordered,{{"EvasionPerLevel","BASE",7,"Probe:E"},{"Evasion","INC",10,"Probe:inc"}})
assert(ordered.armourData.EvasionPerLevel==(7*1.1)*1.1)
assert(ordered.armourData.EvasionPerLevel~=7*(1.1*1.1))
local signed=defenceComponent({},0)
defenceSlot(signed,{{"WardPerLevel","BASE",-1.5,"Probe:negative"}})
assert(signed:GetArmourDataValue("Ward",1)==-1)
assert(signed:GetArmourDataValue("Ward",3)==-4)
"#,
            )
            .set_name("@source-defence-per-level-getter")
            .exec()
            .unwrap();
    }
}

#[test]
fn optional_block_and_movement_presence_survive_zero_values() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
local blockRows={{"BlockChance","BASE",2.9,"Probe:base"},{"BlockChance","INC",50,"Probe:inc"}}
local present=defenceComponent({Armour=100,BlockChance=0,MovementPenalty=0},100)
local left=defenceSlot(present,blockRows)
assert(present.armourData.BlockChance==4 and present.armourData.Armour==200)
assert(defenceSources(left)=="")
local penalty=defenceNamed(left,"MovementSpeed")
assert(#penalty==1 and penalty[1].type=="BASE" and penalty[1].value==0)
assert(#penalty[1]==1 and penalty[1][1].type=="Condition")
assert(penalty[1][1].var=="IgnoreMovementPenalties" and penalty[1][1].neg==true)
local absent=defenceComponent({},100)
local absentLeft=defenceSlot(absent,blockRows)
assert(absent.armourData.BlockChance==nil and absent:GetArmourDataValue("BlockChance")==0)
assert(defenceSources(absentLeft)=="Probe:base,Probe:inc")
assert(#defenceNamed(absentLeft,"MovementSpeed")==0)
local ordinary=defenceItem("Splintered Tower Shield",20,"")
assert(ordinary.armourData.BlockChance==26 and ordinary.armourData.Armour==22)
local ordinaryPenalty=defenceNamed(ordinary.modList,"MovementSpeed")
assert(#ordinaryPenalty==1 and ordinaryPenalty[1].value==-0.03)
local raw=defenceComponent({MovementPenalty=0.05},200)
local rawLeft=defenceSlot(raw,{})
assert(defenceNamed(rawLeft,"MovementSpeed")[1].value==-0.05)
"#,
            )
            .set_name("@source-defence-optional-block-and-penalty")
            .exec()
            .unwrap();
    }
}

#[test]
fn complete_slot_method_preserves_ordered_eligibility_removal_and_first_tag_behavior() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled).lua.load(r#"
local item=defenceComponent({},0)
local left,input=defenceSlot(item,{
 {"Armour","BASE",2,"Probe:first"},{"Armour","BASE",2,"Probe:repeat"},
 {"Armour","BASE",100,"Probe:flagged",ModFlag.Attack},
 {"Armour","BASE",100,"Probe:keyword",0,KeywordFlag.Attack},
 {"Armour","BASE",100,"Probe:condition",{type="Condition",var="Missing"}},
 {"Armour","BASE",3,"Probe:slot",{type="InSlot",num=1}},
 {"Armour","BASE",5,"Probe:slot-then-condition",{type="InSlot",num=1},{type="Condition",var="Missing"}},
 {"Armour","BASE",100,"Probe:condition-then-slot",{type="Condition",var="Missing"},{type="InSlot",num=1}},
 {"Armour","BASE",100,"Probe:slot-number",{type="SlotNumber",num=1}},
 {"Armour","BASE",100,"Probe:wrong-slot",{type="InSlot",num=2}},
 {"Armour","MORE",100,"Probe:more"},
},1)
assert(item.armourData.Armour==12)
assert(defenceSources(left)=="Probe:flagged,Probe:keyword,Probe:condition,Probe:condition-then-slot,Probe:slot-number,Probe:more")
assert(#input==11 and input[1]~=input[2])
assert(input[7][1].type=="InSlot" and input[7][2].type=="Condition")
-- The matching InSlot first tag admits a trailing condition without evaluating
-- it; the reversed order is retained. This is source behavior, not new policy.
assert(input[7][2].var=="Missing")
for _,mod in ipairs(left) do
 if mod.source=="Probe:flagged" then
  assert(mod~=input[3] and mod.flags==ModFlag.Attack and mod.sourceSlot=="Body Armour")
 end
end
"#).set_name("@source-defence-local-eligibility-and-removal").exec().unwrap();
    }
}

#[test]
fn ordered_armour_data_overrides_run_after_assembly_and_do_not_consume_list_records() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
local item=defenceComponent({Armour=100,EnergyShield=100},20)
local left=defenceSlot(item,{
 {"EnergyShield","INC",50,"Probe:inc"},
 {"ArmourData","LIST",{key="EnergyShield",value=99},"Probe:first"},
 {"ArmourData","LIST",{key="EnergyShield",value=0},"Probe:last-zero"},
 {"ArmourData","LIST",{key="ArmourBase",value=7},"Probe:base-override"},
 {"ArmourData","LIST",{key="WardPerLevel",value=0.5},"Probe:level-override"},
})
assert(item.armourData.Armour==120 and item.armourData.ArmourBase==7)
assert(item.armourData.EnergyShield==0 and item.armourData.EnergyShieldBase==100)
assert(item:GetArmourDataValue("Ward",1)==1)
assert(defenceSources(left)=="Probe:first,Probe:last-zero,Probe:base-override,Probe:level-override")
local reverse=defenceComponent({EnergyShield=100},20)
defenceSlot(reverse,{
 {"ArmourData","LIST",{key="EnergyShield",value=0},"Probe:first-zero"},
 {"ArmourData","LIST",{key="EnergyShield",value=99},"Probe:last"},
})
assert(reverse.armourData.EnergyShield==99)
"#,
            )
            .set_name("@source-defence-post-assembly-overrides")
            .exec()
            .unwrap();
    }
}

#[test]
fn reused_items_retain_unwritten_defence_fields_and_reconcile_crafted_quality() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
-- Actual constructor/ParseRaw lifecycle using unchanged real base definitions.
local reused=defenceItem("Splintered Tower Shield",0,"")
local stored=reused.armourData
assert(stored.BlockChance==26 and stored.Armour==18)
reused:ParseRaw(defenceRaw("Garment",0,""),nil,false)
assert(reused.armourData==stored and stored.Armour==0 and stored.BlockChance==26)
local fresh=defenceItem("Garment",0,"")
assert(fresh.armourData.BlockChance==nil and fresh:GetArmourDataValue("BlockChance")==0)
reused:ParseRaw(defenceRaw("Iron Ring",0,""),nil,false)
assert(reused.base.armour==nil and reused.armourData==stored)
assert(reused:GetArmourDataValue("BlockChance")==26)
assert(defenceItem("Iron Ring",0,"").armourData==nil)
-- The ordinary BASE modifier is a component input. A previously constructed
-- item already has craftedQuality=0; reconciliation then adds only the delta.
local q=defenceComponent({Armour=100},20)
assert(q.craftedQuality==0)
for _,case in ipairs({{5,25,125},{5,25,125},{7,27,127},{0,20,120}}) do
 defenceSlot(q,{{"Quality","BASE",case[1],"Probe:quality"}})
 assert(q.quality==case[2] and q.armourData.Armour==case[3])
 assert(q.craftedQuality==case[1])
end
-- Explicit first-assembly state contrasts with the reused item above. No
-- source callback is substituted; these are inputs to the complete method.
local initial=defenceComponent({Armour=100},20)
initial.craftedQuality=nil
for _,case in ipairs({{5,20,120},{7,22,122},{0,15,115}}) do
 defenceSlot(initial,{{"Quality","BASE",case[1],"Probe:quality"}})
 assert(initial.quality==case[2] and initial.armourData.Armour==case[3])
end
"#,
            )
            .set_name("@source-defence-fresh-and-reused-lifecycle")
            .exec()
            .unwrap();
    }
}

#[test]
fn whole_item_assembly_distinguishes_empty_absent_and_explicit_global_defences() {
    for jit_enabled in [false, true] {
        oracle(jit_enabled)
            .lua
            .load(
                r#"
for _,base in ipairs({"Garment","Iron Ring"}) do
 -- The same parsed modifier shape is an explicit assembly contrast, not a
 -- general local/global inference rule for the owned data model.
 local item=defenceItem(base,0,"+11 to maximum Energy Shield")
 local parsed=defenceNamed(item.baseModList,"EnergyShield")
 assert(#parsed==1 and parsed[1].type=="BASE" and parsed[1].value==11)
 assert(parsed[1].flags==0 and parsed[1].keywordFlags==0 and #parsed[1]==0)
 if base=="Garment" then
  assert(type(item.base.armour)=="table" and item.armourData.EnergyShield==11)
  assert(#defenceNamed(item.modList,"EnergyShield")==0)
 else
  assert(item.base.armour==nil and item.armourData==nil)
  for slot=1,3 do assert(#defenceNamed(item.slotModList[slot],"EnergyShield")==1) end
 end
end
-- The maximum-energy-shield INC exception is explicitly Global in the original
-- parser. It survives assembly on an actual ES base while ordinary wording is
-- consumed locally. This does not claim later actor activation or final ES.
local localEs=defenceItem("Adherent Cuffs",0,"11% increased Energy Shield")
local globalEs=defenceItem("Adherent Cuffs",0,"11% increased maximum Energy Shield")
assert(localEs.armourData.EnergyShield==30 and globalEs.armourData.EnergyShield==27)
assert(#defenceNamed(localEs.modList,"EnergyShield")==0)
local remaining=defenceNamed(globalEs.modList,"EnergyShield")
assert(#remaining==1 and remaining[1].type=="INC" and remaining[1].value==11)
assert(remaining[1].flags==0 and remaining[1].keywordFlags==0)
assert(#remaining[1]==1 and remaining[1][1].type=="Global")
"#,
            )
            .set_name("@source-defence-whole-item-presence-and-global-exception")
            .exec()
            .unwrap();
    }
}
