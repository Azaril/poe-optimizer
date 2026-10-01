//! Observations around authenticated complete PoB loading; no source replacements.
use mlua::{Function, Lua, LuaSerdeExt, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::Value as Json;

pub fn install(lua: &Lua) -> Result<Function, RuntimeError> {
    Ok(lua
        .load(INSTALL)
        .set_name("@scoped-allocation-authentication")
        .eval()?)
}
pub fn observe(lua: &Lua) -> Result<Json, RuntimeError> {
    let value: Value = lua
        .load(OBSERVE)
        .set_name("@scoped-allocation-observation")
        .eval()?;
    Ok(lua.from_value(value)?)
}
const INSTALL: &str = r#"
local function original(f,path,first)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first,path..":"..tostring(first).." actual "..s..":"..tostring(i.linedefined))
 return f
end
local c=common.classes.PassiveSpec;local t=common.classes.TreeTab;local calcs=require("Modules.CalcBase")
local wrapper=original(c.PassiveSpec,"Modules/Common.lua",167);local constructor
for i=1,20 do local name,value=debug.getupvalue(wrapper,i);if not name then break end;if name=="originalFunc"then assert(not constructor);constructor=value end end
original(assert(constructor),"Classes/PassiveSpec.lua",33)
local methods={PassiveSpec=wrapper,Init=original(c.Init,"Classes/PassiveSpec.lua",45),Load=original(c.Load,"Classes/PassiveSpec.lua",117),
 PostLoad=original(c.PostLoad,"Classes/PassiveSpec.lua",353),ImportFromNodeList=original(c.ImportFromNodeList,"Classes/PassiveSpec.lua",358),
 Save=original(c.Save,"Classes/PassiveSpec.lua",252),CountAllocNodes=original(c.CountAllocNodes,"Classes/PassiveSpec.lua",1029),
 SetGrantedPassiveNodes=original(c.SetGrantedPassiveNodes,"Classes/PassiveSpec.lua",1190),BuildAllDependsAndPaths=original(c.BuildAllDependsAndPaths,"Classes/PassiveSpec.lua",1442),
 CanPathThroughAllocMode=original(c.CanPathThroughAllocMode,"Classes/PassiveSpec.lua",863),FindStartFromNode=original(c.FindStartFromNode,"Classes/PassiveSpec.lua",1060),
 ReplaceNode=original(c.ReplaceNode,"Classes/PassiveSpec.lua",2047),SwitchAttributeNode=original(c.SwitchAttributeNode,"Classes/PassiveSpec.lua",2718)}
local prior={};for _,s in ipairs(build.treeTab.specList)do prior[s]=true end
_allocationAccess={methods=methods,prior=prior,treeLoad=original(t.Load,"Classes/TreeTab.lua",492),treePostLoad=original(t.PostLoad,"Classes/TreeTab.lua",521),initEnv=original(calcs.initEnv,"Modules/CalcSetup.lua",717),nodeMods=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200),nodeListMods=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)}
return function()_allocationAccess.finished=true end
"#;

const OBSERVE: &str = r#"
local auth=assert(_allocationAccess);assert(auth.finished);local calcs=require("Modules.CalcBase")
local doc,err=common.xml.ParseXML(accessXml);assert(doc and not err)
local tree;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Tree"then assert(not tree);tree=n end end;assert(tree)
local specs={};for _,n in ipairs(tree)do if type(n)=="table"and n.elem=="Spec"then specs[#specs+1]=n end end
local spec=build.spec;local tab=build.treeTab;local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local selected={spec=tab.activeSpec,items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local raw=assert(specs[selected.spec]);assert(spec==tab.specList[selected.spec] and env.spec==spec)
local function field(v)return {type=type(v),value=v}end
local function ids(map)local r={};for id in pairs(map)do assert(type(id)=="number");r[#r+1]=id end;table.sort(r);return r end
local function csv(value)local r={};for token in (value or ""):gmatch("%d+")do r[#r+1]=tonumber(token)end;table.sort(r);return r end
local function plain(v,depth)
 depth=depth or 0;assert(depth<12);local ty=type(v)
 if ty=="string"or ty=="number"or ty=="boolean"or ty=="nil"then return v end
 assert(ty=="table");local out={};for k,x in pairs(v)do assert(type(k)=="string"or type(k)=="number");out[k]=plain(x,depth+1)end;return out
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function noderow(id,n)
 local providers={};for _,p in ipairs(n.intuitiveLeapLikesAffecting or {})do providers[#providers+1]=p.id end;table.sort(providers)
 local depends={};for _,p in ipairs(n.depends or {})do depends[#depends+1]=p.id end;table.sort(depends)
 local finalMods={};for _,m in ipairs(n.finalModList or {})do
  local conditions={};for _,tag in ipairs(m)do if tag.type=="Condition"then conditions[#conditions+1]=plain(tag)end end
  finalMods[#finalMods+1]={name=m.name,type=m.type,source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,value_type=type(m.value),conditions=conditions}
 end
 local linked,alllinked={},{};for _,p in ipairs(n.linked or {})do alllinked[#alllinked+1]=p.id;if p.alloc then linked[#linked+1]=p.id end end;table.sort(linked);table.sort(alllinked)
 return {id=id,effective_id=n.id,type=n.type,name=n.name,display_name=n.dn,ascendancy=field(n.ascendancyName),alloc=field(n.alloc),alloc_mode=n.allocMode,
  connected=field(n.connectedToStart),free=field(n.isFreeAllocate),granted=field(n.isGrantedPassive),radius_providers=providers,
  final_modifiers=finalMods,depends=depends,unlock=plain(n.unlockConstraint),linked_allocated=linked,linked_ids=alllinked,multiple_choice=field(n.isMultipleChoice),choice_option=field(n.isMultipleChoiceOption)}
end
local function snapshot(s,nodes,withspec)
 local out={ids=ids(nodes),nodes={}};for _,id in ipairs(out.ids)do out.nodes[#out.nodes+1]=noderow(id,nodes[id])end
 if withspec then
  out.counts={s:CountAllocNodes()};out.class=s.curClassId;out.class_internal=s.curClass.integerId;out.ascendancy=s.curAscendClassId;
  out.class_root=s.curClass.startNodeId;out.ascendancy_root=s.curAscendClass.startNodeId;out.jewels={};out.zero_jewel_nodes={}
  for node,itemid in pairs(s.jewels)do if itemid>0 then
   local item=assert(build.itemsTab.items[itemid]);out.jewels[#out.jewels+1]={node=node,item=itemid,name=item.name,title=item.title,radius=item.jewelRadiusIndex,
    alternate_class_start=field(item.jewelData.alternateClassStart),intuitive_leap=field(item.jewelData.intuitiveLeapLike),
    from_nothing=field(item.jewelData.fromNothingKeystone),from_nothing_keystones=plain(item.jewelData.fromNothingKeystones),limit_disabled=field(item.jewelData.limitDisabled)}
   else assert(itemid==0);out.zero_jewel_nodes[#out.zero_jewel_nodes+1]=node end end
  table.sort(out.jewels,function(a,b)return a.node<b.node end);table.sort(out.zero_jewel_nodes)
  out.node_radius_rules=plain(s.intuitiveLeapLikeNodes);out.subgraph_ids=ids(s.subGraphs);out.alloc_subgraph=plain(s.allocSubgraphNodes)
 end;return out
end
local saved={};local seen={};for i,s in ipairs(tab.specList)do assert(not auth.prior[s]and not seen[s]);seen[s]=true;saved[i]={object=s,allocation_table=s.allocNodes,state=snapshot(s,s.allocNodes,true)}end
local primitives={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then primitives[k]=v end end
local result={selected=selected,source_attributes=plain(raw.attrib),source_children={},requested_ids=csv(raw.attrib.nodes),retained=snapshot(spec,spec.allocNodes,true),main=snapshot(spec,env.allocNodes,false),main_granted_ids=ids(env.grantedPassives),main_grant_records=plain(env.modDB.mods.GrantedPassive or {}),fresh_specs=true,main_spec_matches_selected=true}
for _,n in ipairs(raw)do if type(n)=="table"then result.source_children[#result.source_children+1]=plain(n)end end
local requested={};for _,id in ipairs(result.requested_ids)do requested[id]=true end
result.removed_ids={};for _,id in ipairs(result.requested_ids)do if not spec.allocNodes[id]then result.removed_ids[#result.removed_ids+1]=id end end
result.added_ids={};for _,id in ipairs(result.retained.ids)do if not requested[id]then result.added_ids[#result.added_ids+1]=id end end
local savedxml={elem="Spec"};spec:Save(savedxml);result.resaved_ids=csv(savedxml.attrib.nodes)
local requestedNodes={};for id in pairs(requested)do if spec.nodes[id]then requestedNodes[id]=spec.nodes[id]end end
result.requested_nodes=snapshot(spec,requestedNodes,false)
result.mode_compatibility={}
for _,id in ipairs(result.requested_nodes.ids)do
 local node=spec.nodes[id];result.mode_compatibility[#result.mode_compatibility+1]={id=id,modes={spec:CanPathThroughAllocMode(0,node),spec:CanPathThroughAllocMode(1,node),spec:CanPathThroughAllocMode(2,node)}}
end
result.active_weapon_set=build.itemsTab.activeItemSet.useSecondWeaponSet and 2 or 1
result.main_weapon_conditions={one=plain(env.modDB.mods["Condition:WeaponSet1"] or {}),two=plain(env.modDB.mods["Condition:WeaponSet2"] or {})}
assert(build.spec==spec and tab.activeSpec==selected.spec and build.itemsTab.activeItemSetId==selected.items and build.skillsTab.activeSkillSetId==selected.skills and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
for i,s in ipairs(saved)do assert(tab.specList[i]==s.object and s.object.allocNodes==s.allocation_table and equal(s.state,snapshot(s.object,s.object.allocNodes,true)))end
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(primitives)do assert(output[k]==v)end
for name,f in pairs(auth.methods)do assert(common.classes.PassiveSpec[name]==f)end
assert(common.classes.TreeTab.Load==auth.treeLoad and common.classes.TreeTab.PostLoad==auth.treePostLoad and calcs.initEnv==auth.initEnv)
assert(calcs.buildModListForNode==auth.nodeMods and calcs.buildModListForNodeList==auth.nodeListMods)
result.saved_state_preserved=true;result.main_output_preserved=true;result.original_functions_preserved=true
return result
"#;
