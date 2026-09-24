//! Ordinary defensive source members and numeric formatting, not item-local
//! ownership, equipped activation, actor defences, or complete-build parity.
//! Original parser/formatter/Item methods execute completely; bounded observers
//! record initial parsing separately from the later original assembly call.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/item_loading_runtime.rs"]
mod runtime;

fn source() -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle.lua.load(r##"
defenceFamilies={
 {"armour-base","%s to Armour","BASE",{"Armour"},true},
 {"evasion-base","%s to Evasion Rating","BASE",{"Evasion"},true},
 {"energy-shield-base","%s to maximum Energy Shield","BASE",{"EnergyShield"},true},
 {"ward-base","%s to maximum Runic Ward","BASE",{"Ward"},true},
 {"armour-evasion-base","%s to Armour and Evasion Rating","BASE",{"ArmourAndEvasion"},false},
 {"armour-energy-shield-base","%s to Armour and Energy Shield","BASE",{"ArmourAndEnergyShield"},false},
 {"evasion-energy-shield-base","%s to Evasion Rating and Energy Shield","BASE",{"EvasionAndEnergyShield"},true},
 {"triple-defence-base","%s to Armour, Evasion and Energy Shield","BASE",{"Armour","Evasion","EnergyShield"},false},
 {"block-chance-base","%s%% Chance to Block","BASE",{"BlockChance"},true},
 {"armour-increase","%s%% increased Armour","INC",{"Armour"},true},
 {"evasion-increase","%s%% increased Evasion Rating","INC",{"Evasion"},true},
 {"energy-shield-increase","%s%% increased Energy Shield","INC",{"EnergyShield"},true},
 {"ward-increase","%s%% increased Runic Ward","INC",{"Ward"},true},
 {"armour-evasion-increase","%s%% increased Armour and Evasion","INC",{"ArmourAndEvasion"},true},
 {"armour-energy-shield-increase","%s%% increased Armour and Energy Shield","INC",{"ArmourAndEnergyShield"},true},
 {"evasion-energy-shield-increase","%s%% increased Evasion and Energy Shield","INC",{"EvasionAndEnergyShield"},true},
 {"triple-defence-increase","%s%% increased Armour, Evasion and Energy Shield","INC",{"Armour","Evasion","EnergyShield"},true},
 {"defences-increase","%s%% increased Defences","INC",{"Defences"},false},
 {"block-chance-increase","%s%% increased Block chance","INC",{"BlockChance"},true},
}
defenceInputLists={"classRequirementModLines","buffModLines","enchantModLines","runeModLines","implicitModLines","explicitModLines"}
local trace={calls={},formats={},assembly=false}
defenceInputTrace=trace
local parse,format=modLib.parseMod,itemLib.applyRange
local class=common.classes.Item
local build=class.BuildModList
function class:BuildModList(...)
 local before=trace.assembly;trace.assembly=true
 local result=build(self,...);trace.assembly=before;return result
end
function modLib.parseMod(text,combined,...)
 local mods,extra=parse(text,combined,...)
 if not trace.assembly then
  assert(#trace.calls<512 and #text<=16384,"defence parser observation bound")
  trace.calls[#trace.calls+1]={text=text,combined=combined==true,mods=mods and copyTable(mods),extra=extra}
 end
 return mods,extra
end
function itemLib.applyRange(text,range,scalar,corrupted,...)
 local result=format(text,range,scalar,corrupted,...)
 if not trace.assembly then
  assert(#trace.formats<512 and #text<=16384,"defence formatter observation bound")
  trace.formats[#trace.formats+1]={text=text,range=range,scalar=scalar,corrupted=corrupted,output=result}
 end
 return result
end
function defenceInputReset()
 trace.calls={};trace.formats={};trace.assembly=false
end
function defenceInputLine(family,number,reduced)
 local format=family[2]
 if family[3]=="BASE" and number:sub(1,1)~="-" then number="+"..number end
 if reduced then format=format:gsub("increased","reduced") end
 return string.format(format,number)
end
function defenceInput(base,headers,body)
 defenceInputReset()
 local raw="Rarity: RARE\nDefence Input Probe\n"..base.."\nItem Level: 80\nQuality: 0\n"..(headers or "").."\nImplicits: 0\n"..body
 assert(#raw<=32768)
 local item=new("Item"):Item(raw,nil,false)
 assert(item.baseName==base)
 return item
end
function defenceInputAssertMods(mods,family,value)
 assert(mods and #mods==#family[4],family[1].." effect count")
 for index,name in ipairs(family[4]) do
  local mod=mods[index]
  assert(mod.name==name and mod.type==family[3],family[1].." effect shape")
  assert(mod.value==value,family[1].." value "..tostring(mod.value).." expected "..tostring(value))
  assert(mod.flags==0 and mod.keywordFlags==0 and #mod==0,family[1].." flags/tags")
 end
end
function defenceInputAssertMember(item,text,family,value)
 local found={}
 for _,list in ipairs(defenceInputLists) do
  for _,member in ipairs(item[list]) do
   if member.line==text then found[#found+1]={list,member} end
  end
 end
 assert(#found==1 and found[1][1]=="explicitModLines",family[1].." physical member")
 local member=found[1][2]
 assert(not member.extra and #member.modTags==0)
 defenceInputAssertMods(member.modList,family,value)
 for _,call in ipairs(trace.calls) do assert(not call.combined,family[1].." combined retry") end
 local formats={}
 for _,entry in ipairs(trace.formats) do if entry.text==text then formats[#formats+1]=entry end end
 assert(#formats==1 and formats[1].scalar==1,family[1].." initial format")
 local calls={}
 for _,entry in ipairs(trace.calls) do if entry.text==formats[1].output then calls[#calls+1]=entry end end
 assert(#calls==1 and not calls[1].extra,family[1].." initial parse")
 defenceInputAssertMods(calls[1].mods,family,value)
 return #member.modList
end
function defenceInputFormat(family,number,magnitude,base,reduced)
 defenceInputReset()
 local formatted=itemLib.applyRange(defenceInputLine(family,number,reduced),1,magnitude,base)
 local mods,extra=modLib.parseMod(formatted)
 return formatted,mods,extra
end
"##).set_name("@owned-defence-input-complete-method-observers").exec().unwrap();
    oracle
}

#[test]
fn all_1756_constructed_bases_preserve_each_of_19_supplied_defence_members() {
    source().lua.load(r##"
local bases={}
local generatedBases=0
for name,base in pairs(data.itemBases) do
 bases[#bases+1]=name
 local generated=false
 for _,field in ipairs({"flask","charm"}) do
  if base[field] and base[field].buff and #base[field].buff>0 then generated=true end
 end
 if generated then generatedBases=generatedBases+1 end
end
table.sort(bases)
assert(#bases==1756 and #defenceFamilies==19 and generatedBases==13)
local parses,effects=0,0
for index,base in ipairs(bases) do
 for _,family in ipairs(defenceFamilies) do
  local text=defenceInputLine(family,"11")
  local item=defenceInput(base,"",text.."\nwhile stationary")
  effects=effects+defenceInputAssertMember(item,text,family,11)
  parses=parses+1
 end
 if index%128==0 then collectgarbage("collect") end
end
assert(parses==33364 and effects==40388)
-- Generated-buff bases are observed, not admitted by the owned source guard.
-- The source admission policy independently excludes generated buff members.
print("defence source catalog: bases=1756 families=19 fresh_parses=33364 supplied_effects=40388 generated_buff_bases=13; component input evidence only")
"##).set_name("@source-defence-full-base-member-sweep").exec().unwrap();
}

#[test]
fn exact_scalability_catalog_distinguishes_15_modern_and_four_fallback_families() {
    source()
        .lua
        .load(
            r##"
local modern,fallback=0,0
for _,family in ipairs(defenceFamilies) do
 local key=string.format(family[2],"#")
 local entry=data.modScalability[key]
 if family[5] then
  modern=modern+1
  assert(entry and #entry==1 and entry[1].isScalable==true and entry[1].formats==nil,family[1])
  if family[3]=="INC" then
   local reduced=data.modScalability[key:gsub("increased","reduced")]
   assert(reduced and #reduced==1 and reduced[1].isScalable==true)
   assert(#reduced[1].formats==1 and reduced[1].formats[1]=="negate")
  end
 else fallback=fallback+1;assert(entry==nil,family[1]) end
 for _,number in ipairs({"0","0000011","1000000","1000001"}) do
  local text=defenceInputLine(family,number)
  local item=defenceInput("Iron Ring","",text.."\nwhile stationary")
  defenceInputAssertMember(item,text,family,tonumber(number))
 end
 if family[3]=="INC" then
  for _,number in ipairs({"0","11","1000000"}) do
   local text=defenceInputLine(family,number,true)
   local item=defenceInput("Iron Ring","",text.."\nwhile stationary")
   defenceInputAssertMember(item,text,family,-tonumber(number))
  end
 end
end
assert(modern==15 and fallback==4)
-- The source accepts 1000001 above, but this does not widen the owned policy's
-- independently declared maximum 1000000.
"##,
        )
        .set_name("@source-defence-scalability-and-integer-boundaries")
        .exec()
        .unwrap();
}

#[test]
fn signed_and_decimal_source_shapes_remain_distinct_from_reviewed_integer_admission() {
    source()
        .lua
        .load(
            r##"
for _,family in ipairs(defenceFamilies) do
 for _,case in ipairs({{"10.25",10.25,10},{"10.5",10.5,11}}) do
  defenceInputReset()
  local text=defenceInputLine(family,case[1])
  local direct,extra=modLib.parseMod(text)
  if family[3]=="BASE" then
   assert(not extra);defenceInputAssertMods(direct,family,case[2])
  else
   -- Ordinary INC/RED forms are integer-only in ModParser.lua63/65.
   assert(not direct or extra,family[1].." decimal direct parser unexpectedly complete")
  end
  local item=defenceInput("Iron Ring","",text.."\nwhile stationary")
  if family[5] or family[3]=="BASE" then
   defenceInputAssertMember(item,text,family,family[5] and case[3] or case[2])
  else
   local sawInitial,sawRetry=false,false
   for _,call in ipairs(defenceInputTrace.calls) do
    if call.text==text then sawInitial=true;assert(not call.mods or call.extra) end
    if call.combined then sawRetry=true end
   end
   assert(sawInitial and sawRetry)
  end
 end
 if family[3]=="BASE" then
  local text=defenceInputLine(family,"-10.5")
  defenceInputReset()
  local direct,extra=modLib.parseMod(text)
  assert(not extra);defenceInputAssertMods(direct,family,-10.5)
  local item=defenceInput("Iron Ring","",text.."\nwhile stationary")
  defenceInputAssertMember(item,text,family,family[5] and -11 or -10.5)
 else
  local text=defenceInputLine(family,"10.5",true)
  defenceInputReset()
  local direct,extra=modLib.parseMod(text)
  assert(not direct or extra)
  local item=defenceInput("Iron Ring","",text.."\nwhile stationary")
  if family[5] then
   defenceInputAssertMember(item,text,family,-11)
  else
   local sawRetry=false
   for _,call in ipairs(defenceInputTrace.calls) do
    if call.text==text then assert(not call.mods or call.extra) end
    if call.combined then sawRetry=true end
   end
   assert(sawRetry)
  end
 end
end
-- These observations distinguish a parsed numeric component from the value
-- produced by the original formatter. Signed BASE and decimal raw forms remain
-- source-only contrasts, not additional source-admitted owned occurrences.
"##,
        )
        .set_name("@source-defence-signed-decimal-versus-formatted-values")
        .exec()
        .unwrap();
}

#[test]
fn legacy_fixed_decimal_formatting_preserves_lexical_precision_and_absent_base_factor() {
    source()
        .lua
        .load(
            r##"
assert(data.defaultHighPrecision==1)
local cases=0
for _,family in ipairs(defenceFamilies) do
 if not family[5] then
  for _,entry in ipairs({
   {number="0.15",magnitude=1.5,expected=0.2},
   {number="0.15",magnitude=1.5,base=1,expected=0.3},
   {number="11",magnitude=1.05,expected=11},
   {number="11.0",magnitude=1.05,expected=11.5},
   {number="0.15",magnitude=1,expected=0.15},
   {number="0.15",magnitude=1,base=1,expected=0.15},
   {number="11",magnitude=1,base=0.5,expected=6},
  }) do
   local formatted,mods,extra=defenceInputFormat(family,entry.number,entry.magnitude,entry.base)
   assert(formatted==defenceInputLine(family,tostring(entry.expected)),family[1].." formatted text: "..formatted)
   if family[3]=="INC" and entry.expected%1~=0 then
    -- Complete fallback formatting succeeded, but a fractional INC result is
    -- not a complete ordinary source modifier. Numeric output is insufficient.
    assert(not mods or extra)
   else
    assert(not extra);defenceInputAssertMods(mods,family,entry.expected)
   end
   cases=cases+1
  end
 end
end
assert(cases==28)
local leadingDot=0
for _,family in ipairs(defenceFamilies) do
 if not family[5] then
  local formatted,mods,extra=defenceInputFormat(family,".15",1.5,nil)
  assert(formatted==defenceInputLine(family,".22"),family[1].." leading-dot output "..formatted)
  if family[3]=="BASE" then
   assert(not extra);defenceInputAssertMods(mods,family,0.22)
  else assert(not mods or extra) end
  leadingDot=leadingDot+1
 end
end
assert(leadingDot==4)
-- '.15' contains a point, but the original fallback selects decimal precision
-- only when a digit precedes it (ItemTools.lua319). It scales the substring 15
-- as an integer, producing '.22', whereas the explicit owned decimal setting
-- with numeric component 0.15 would produce 0.2. Decimal source eligibility is
-- unfinished; HasDecimalPoint alone is not a general PoB formatting decision.
-- In fallback formatting an explicit base factor of 1 is observable when
-- another scalar activates the slow path. Missing is not equivalent to 1.
-- Above results include the source's tostring and reparsing, not just numeric
-- intermediate values or a reimplementation of its formula.
-- Bounded complete-Item history witness: each later non-unity magnitude pass
-- formats from original text with explicit base=1 (Item1685). Returning to unity
-- skips that update and preserves the previously parsed list.
local text="11% increased Defences"
for _,case in ipairs({
 {"25% increased explicit modifier magnitudes\n25% reduced explicit modifier magnitudes",13,"13% increased Defences"},
 {"25% reduced explicit modifier magnitudes\n25% increased explicit modifier magnitudes",8,"8% increased Defences"},
}) do
 local item=defenceInput("Iron Ring","Crafted: true",text.."\n"..case[1])
 assert(#item.modMagnitudeMods==2)
 local target
 for _,member in ipairs(item.explicitModLines) do if member.line==text then target=member end end
 assert(target and target.valueScalar==1 and not target.extra)
 defenceInputAssertMods(target.modList,defenceFamilies[18],case[2])
 local formats={}
 for _,entry in ipairs(defenceInputTrace.formats) do if entry.text==text then formats[#formats+1]=entry end end
 assert(#formats==2 and formats[1].corrupted==nil and formats[2].corrupted==1)
 assert(formats[1].output==text and formats[2].output==case[3])
end
-- Equal final magnitude (1) retains unequal prior INC values (13 versus 8).
-- Source also guards assignment by modList presence, not complete parsing:
-- a negative result is parsed as BASE with leftover text, then installed with
-- extra. It is not a nil-result retention witness or an active defence bonus.
local incomplete=defenceInput("Iron Ring","Crafted: true",text.."\n25% increased explicit modifier magnitudes\n200% reduced explicit modifier magnitudes")
local target
for _,member in ipairs(incomplete.explicitModLines) do if member.line==text then target=member end end
assert(target and target.valueScalar==-0.75 and target.extra:find("increased",1,true))
assert(#target.modList==1)
local mod=target.modList[1]
assert(mod.name=="Defences" and mod.type=="BASE" and mod.value==-9)
assert(mod.flags==0 and mod.keywordFlags==0 and #mod==0)
local observed=false
for _,entry in ipairs(defenceInputTrace.calls) do
 if entry.text=="-9% increased Defences" then
  assert(entry.mods and #entry.mods==1 and entry.extra:find("increased",1,true))
  assert(entry.mods[1].type=="BASE" and entry.mods[1].value==-9)
  observed=true
 end
end
assert(observed)
for _,mod in ipairs(incomplete.baseModList) do assert(mod.name~="Defences") end
-- Item1687 also preserves the prior list when the parser returns nil; that
-- branch is source-audited, not claimed exercised by this incomplete BASE case.
-- These are component history observations, not native lifecycle adoption or
-- complete-source/build parity.
"##,
        )
        .set_name("@source-defence-legacy-lexical-and-base-presence")
        .exec()
        .unwrap();
}

#[test]
fn modern_formatting_quantizes_before_magnitude_and_retains_global_es_exception() {
    source().lua.load(r##"
local count=0
for _,family in ipairs(defenceFamilies) do
 if family[5] then
  for _,entry in ipairs({
   {"10.5",1.2,1,13},{"11",0.5,1,5},{"11",1,0.5,6},{"10.5",1.2,0.5,7},
  }) do
   local formatted,mods,extra=defenceInputFormat(family,entry[1],entry[2],entry[3])
   assert(not extra);defenceInputAssertMods(mods,family,entry[4]);count=count+1
  end
 end
end
assert(count==60)
defenceInputReset()
local ordinary,extra=modLib.parseMod("11% increased Energy Shield")
assert(not extra and #ordinary==1 and ordinary[1].name=="EnergyShield" and #ordinary[1]==0)
local global,globalExtra=modLib.parseMod("11% increased maximum Energy Shield")
assert(not globalExtra and #global==1 and global[1].name=="EnergyShield")
assert(global[1].type=="INC" and global[1].value==11 and global[1].flags==0 and global[1].keywordFlags==0)
assert(#global[1]==1 and global[1][1].type=="Global")
-- The maximum-ES INC wording is not one of these 19 ordinary untagged families.
"##).set_name("@source-defence-modern-formatting-and-global-exception").exec().unwrap();
}
