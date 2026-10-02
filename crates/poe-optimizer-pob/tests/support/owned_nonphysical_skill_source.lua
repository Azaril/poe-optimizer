-- Complete original loader observation. No source method or role is replaced.
local calcs = require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S"); local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line,path..":"..line)
 return f
end
local function scalars(t)
 local out={}
 for k,v in pairs(t or {}) do
  if type(v)=="number" or type(v)=="string" or type(v)=="boolean" then
   out[k]=type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or v
  end
 end
 return out
end
local function clone(v,seen)
 if type(v)~="table" then return v end
 seen=seen or {};if seen[v] then return seen[v] end
 local out={};seen[v]=out;for k,e in pairs(v) do out[k]=clone(e,seen) end;return out
end
local function equal(a,b,seen)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 seen=seen or {};if seen[a] then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a) do if not equal(v,b[k],seen) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function keys(t) local out={};for k in pairs(t) do out[k]=true end;return out end
local function allocations(spec)
 local out={};for id,node in pairs(spec.allocNodes) do out[id]={object=node,id=node.id,alloc=node.alloc,mode=node.allocMode} end;return out
end
local function upvalue(f,name)
 for index=1,128 do local key,value=debug.getupvalue(f,index);if not key then break end;if key==name then return value end end
 error("missing original upvalue "..name)
end
local methods={
 {common.classes.SkillsTab,"LoadSkill","Classes/SkillsTab.lua",303},
 {common.classes.SkillsTab,"ProcessSocketGroup","Classes/SkillsTab.lua",1242},
 {common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {calcs,"initEnv","Modules/CalcSetup.lua",717},
 {calcs,"createActiveSkill","Modules/CalcActiveSkill.lua",144},
}
local function gemState(gem)
 local effect=gem.grantedEffect or (gem.gemData and gem.gemData.grantedEffect)
 local data=build.data
 return {name=gem.nameSpec,gem_id=gem.gemId,skill_id=gem.skillId,level=gem.level,quality=gem.quality,enabled=gem.enabled,
  gem_data_present=gem.gemData~=nil,gem_data_id=gem.gemData and gem.gemData.id,
  direct_effect_present=gem.grantedEffect~=nil,resolved_effect=effect and effect.id,
  effect_name=effect and effect.name,effect_hidden=effect and not not effect.hidden,
  support_present=effect and effect.support~=nil or false,support=effect and not not effect.support or false,
  effect_is_source_definition=effect and data.skills[effect.id]==effect or false,
  mapped_gem_by_effect=effect and data.gemForSkill[effect],mapped_gem_by_id=effect and data.gemForSkill[effect.id],
  mapped_gem_by_effect_present=effect and data.gemForSkill[effect]~=nil or false,
  mapped_gem_by_id_present=effect and data.gemForSkill[effect.id]~=nil or false,
  triggered=not not gem.triggered,explode_source_present=gem.explodeSource~=nil,
  explode_source=gem.explodeSource and {id=gem.explodeSource.id,mod_source=gem.explodeSource.modSource,name=gem.explodeSource.name}}
end
if nonphysicalSkillPhase=="before" then
 local refs={};for index,row in ipairs(methods) do refs[index]=original(row[1][row[2]],row[3],row[4]) end
 local process=original(upvalue(calcs.initEnv,"processGrantedEffect"),"Modules/CalcSetup.lua",630)
 local auth={refs=refs,process=process,loaded={},processed={},support_calls={},environments={}}
 local groupIds,nextGroup={},0
 local function groupId(group)
  if not groupIds[group] then nextGroup=nextGroup+1;groupIds[group]=nextGroup end;return groupIds[group]
 end
 local function groupState(group)
  local gems={};for _,gem in ipairs(group.gemList) do gems[#gems+1]=gemState(gem) end
  return {observer_group=groupId(group),source=group.source,label=group.label,slot=group.slot,enabled=group.enabled,
   no_supports_present=group.noSupports~=nil,no_supports=not not group.noSupports,gems=gems}
 end
 auth.groupState=groupState
 local function targets(lists)
  local saved={};for _,list in ipairs(lists or {}) do local values={};for i,value in ipairs(list) do values[i]=value end;saved[#saved+1]={list=list,values=values} end;return saved
 end
 local supportFrame
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local function hook(event)
  local f=debug.getinfo(2,"f").func
  if f~=refs[1] and f~=refs[2] and f~=refs[4] and f~=process then return end
  local vars={};for index=1,128 do local name,value=debug.getlocal(2,index);if not name then break end;vars[name]=value end
  if f==process and vars.grantedEffect and vars.grantedEffect.id=="EnemyExplode" then
   if event=="call" then
    assert(not supportFrame);supportFrame={gem=vars.gemInstance,targets=targets(vars.targetListList),mode=vars.env.mode}
   elseif event=="return" then
    local frame=assert(supportFrame);assert(frame.gem==vars.gemInstance)
    local unchanged=true;local sizes={}
    for _,saved in ipairs(frame.targets) do
     sizes[#sizes+1]=#saved.values
     if #saved.list~=#saved.values then unchanged=false end
     for i,value in ipairs(saved.values) do if saved.list[i]~=value then unchanged=false end end
    end
    assert(not vars.grantedEffect.support and unchanged,"non-support effect changed support targets")
    auth.support_calls[#auth.support_calls+1]={mode=frame.mode,gem=gemState(frame.gem),target_sizes=sizes,target_lists_unchanged=unchanged}
    supportFrame=nil
   end
  elseif event=="return" and f==refs[1] and vars.socketGroup then
   local group=vars.socketGroup;local row=groupState(group)
   row.skill_set=vars.skillSetId;row.raw_group=scalars(vars.node.attrib);row.raw_children={}
   for _,child in ipairs(vars.node) do row.raw_children[#row.raw_children+1]={element=child.elem,attributes=scalars(child.attrib)} end
   assert(#row.raw_children==#group.gemList)
   auth.loaded[#auth.loaded+1]={group=group,state=row,gems={}}
   for i,gem in ipairs(group.gemList) do auth.loaded[#auth.loaded].gems[i]=gem end
  elseif event=="return" and f==refs[2] and vars.socketGroup and vars.socketGroup.source=="Explode" then
   auth.processed[#auth.processed+1]=groupState(vars.socketGroup)
  elseif event=="return" and f==refs[4] and vars.env then
   local env=vars.env;local providers={}
   for _,provider in ipairs(env.explodeSources or {}) do
    local itemId;for id,item in pairs(build.itemsTab.items) do if item==provider then itemId=id end end
    providers[#providers+1]={id=provider.id,item_id=itemId,mod_source=provider.modSource,name=provider.name}
   end
   table.sort(providers,function(a,b)return tostring(a.mod_source or a.id)<tostring(b.mod_source or b.id) end)
   auth.environments[env]={mode=env.mode,providers=providers}
  end
 end
 local enabled=jit.status();jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"cr")
 nonphysicalSkillAuth=auth
 return function()
  assert(debug.gethook()==hook and not supportFrame);debug.sethook(oldHook,oldMask,oldCount);assert(jit.status()==enabled)
  for index,row in ipairs(methods) do assert(row[1][row[2]]==refs[index]) end
  assert(upvalue(calcs.initEnv,"processGrantedEffect")==process);auth.finished=true
 end
end
local auth=assert(nonphysicalSkillAuth);assert(auth.finished and #auth.loaded>0)
local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local mainOutput,calcsOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local savedMain,savedCalcs=clone(mainOutput),clone(calcsOutput)
local savedItems,savedSets,savedSkills,savedConfig=clone(build.itemsTab.items),clone(build.itemsTab.itemSets),clone(build.skillsTab.skillSets),clone(build.configTab.configSets)
local savedSpecs,savedSpecKeys={},keys(build.treeTab.specList)
for i,spec in ipairs(build.treeTab.specList) do savedSpecs[i]={object=spec,nodes=spec.allocNodes,allocations=allocations(spec),jewels=clone(spec.jewels)} end
local function selected()
 return {items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup}
end
local selection=selected()
local loaded,final,selectedLoadedCount,selectedPhysicalSupports={}, {},0,0
for _,row in ipairs(auth.loaded) do
 loaded[#loaded+1]=row.state
 if row.state.skill_set==selection.skills then
  selectedLoadedCount=selectedLoadedCount+#row.state.gems
  for _,gem in ipairs(row.state.gems) do if gem.gem_data_present and gem.support then selectedPhysicalSupports=selectedPhysicalSupports+1 end end
 end
end
for index,group in ipairs(build.skillsTab.socketGroupList) do
 local row=auth.groupState(group);row.index=index;row.loaded_group=false;row.retained_loaded_gems={}
 for _,old in ipairs(auth.loaded) do
  if old.group==group then
   row.loaded_group=true;row.loaded_skill_set=old.state.skill_set
   for _,gem in ipairs(group.gemList) do for i,prior in ipairs(old.gems) do if gem==prior then row.retained_loaded_gems[#row.retained_loaded_gems+1]=i end end end
  end
 end
 final[#final+1]=row
end
local function environment(env)
 local observed=auth.environments[env]
 -- CALCS may use a source cache. Its actual active skills are still inspected.
 local effects={}
 for _,active in ipairs(env.player.activeSkillList) do
  if active.activeEffect.grantedEffect.id=="EnemyExplode" then
   local supports={};for _,support in ipairs(active.supportList or {}) do supports[#supports+1]=support.grantedEffect.id end
   effects[#effects+1]={effect=active.activeEffect.grantedEffect.id,gem=gemState(active.activeEffect.srcInstance),supports=supports,output=scalars(active.output)}
  end
 end
 return {mode=env.mode,providers=observed and observed.providers or nil,original_init_observed=observed~=nil,
  main_effect=env.player.mainSkill.activeEffect.grantedEffect.id,explosion_effects=effects,output=scalars(env.player.output)}
end
local providerItems={}
for id,item in pairs(build.itemsTab.items) do
 if item.baseModList then
  local mods={}
  for _,mod in ipairs(item.baseModList) do
   if mod.name=="CanExplode" or mod.name=="ExplodeMod" then
    mods[#mods+1]={name=mod.name,type=mod.type,value=clone(mod.value),source=mod.source,flags=mod.flags,keyword_flags=mod.keywordFlags}
   end
  end
  if #mods>0 then providerItems[#providerItems+1]={item_id=id,name=item.name,base=item.baseName,mods=mods} end
 end
end
table.sort(providerItems,function(a,b)return a.item_id<b.item_id end)
local result={selected=selection,loaded_groups=loaded,final_groups=final,process_events=auth.processed,
 selected_loaded_gems=selectedLoadedCount,selected_loaded_physical_supports=selectedPhysicalSupports,
 support_process_calls=auth.support_calls,provider_items=providerItems,main=environment(mainEnv),calcs=environment(calcsEnv),
 main_output=scalars(mainOutput),calcs_output=scalars(calcsOutput)}
assert(equal(build.itemsTab.items,savedItems) and equal(build.itemsTab.itemSets,savedSets) and equal(build.skillsTab.skillSets,savedSkills) and equal(build.configTab.configSets,savedConfig))
assert(equal(keys(build.treeTab.specList),savedSpecKeys))
for i,saved in ipairs(savedSpecs) do
 local spec=build.treeTab.specList[i];assert(spec==saved.object and spec.allocNodes==saved.nodes and equal(spec.jewels,saved.jewels))
 local actual=allocations(spec);assert(equal(keys(actual),keys(saved.allocations)))
 for id,node in pairs(actual) do local prior=saved.allocations[id];assert(node.object==prior.object and node.id==prior.id and node.alloc==prior.alloc and node.mode==prior.mode) end
end
assert(equal(mainOutput,savedMain) and equal(calcsOutput,savedCalcs) and equal(selected(),selection))
assert(build.calcsTab.mainEnv==mainEnv and build.calcsTab.calcsEnv==calcsEnv and build.calcsTab.mainOutput==mainOutput and build.calcsTab.calcsOutput==calcsOutput)
for index,row in ipairs(methods) do assert(row[1][row[2]]==auth.refs[index]) end
assert(upvalue(calcs.initEnv,"processGrantedEffect")==auth.process)
result.original_functions_preserved=true;result.loaded_state_preserved=true;result.saved_specs_preserved=true;result.cached_outputs_preserved=true
result.business_method_wrappers=false;result.synthetic_role_injection=false
return result
