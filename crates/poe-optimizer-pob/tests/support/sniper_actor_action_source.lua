-- Read-only complete-lifecycle observation. Saved physical objects are joined
-- by exact identity, never by effect name or by the source count helper.
if sniperActorPhase=="before" then
 assert(djinnOriginals and not debug.gethook())
 local loadSkill,loadSkills=djinnOriginals.refs.load_skill,djinnOriginals.refs.load_skills
 local calls=0
 sniperLoadedGroups={}
 local function capture(event)
  if event~="return" then return end
  local fn=debug.getinfo(2,"f").func
  if fn==loadSkill then
   calls=calls+1;assert(calls<=4096)
   local locals={}
   for i=1,128 do local name,value=debug.getlocal(2,i);if not name then break end;locals[name]=value end
   local node,self,sid=assert(locals.node),assert(locals.self),assert(locals.skillSetId)
   if node.elem~="Skill" then return end
   local set=assert(self.skillSets[sid]);local gi=#set.socketGroupList
   local group=assert(set.socketGroupList[gi])
   assert(group==locals.socketGroup,"original LoadSkill must append its own exact object")
   sniperLoadedGroups[sid]=sniperLoadedGroups[sid] or {}
   assert(not sniperLoadedGroups[sid][gi])
   local gems={}
   for i,gem in ipairs(group.gemList) do gems[i]=gem end
   sniperLoadedGroups[sid][gi]={group=group,set=set,gems=gems}
  elseif fn==loadSkills then
   assert(calls>0);debug.sethook();sniperLoaderHookRemoved=true
   assert(not debug.gethook() and jit.status()==sniperActorJit)
   sniperRequestedJitVerified=true
  end
 end
 debug.sethook(capture,"r")
 return function()
  if sniperActorDirect and not sniperLoaderHookRemoved then
   -- An original LoadSkill error is caught by PoB's normal callback and left
   -- in launch.promptMsg; Load never returns to remove this observer. Preserve
   -- that source error for the harness's existing check_prompt. A capture
   -- failure remains sticky and must never become an admitted source failure.
   local prompt=launch and launch.promptMsg
   assert(type(prompt)=="string" and #prompt>0
    and not prompt:find("sniper-original-loader-object-observer",1,true),
    "incomplete source loader without an independent original source error")
   assert(debug.gethook()==capture,"failed source load must retain our exact observer")
   assert(common.classes.SkillsTab.LoadSkill==loadSkill and common.classes.SkillsTab.Load==loadSkills)
   assert(jit.status()==sniperActorJit,"failed source load changed requested JIT mode")
   debug.sethook()
   assert(not debug.gethook(),"failed source observer cleanup")
   return
  end
  assert(sniperLoaderHookRemoved and not debug.gethook(),"source loader observer must be removed before observation")
  assert(jit.status()==sniperActorJit and sniperRequestedJitVerified)
  assert(common.classes.SkillsTab.LoadSkill==loadSkill and common.classes.SkillsTab.Load==loadSkills)
  sniperOriginalFinish()
 end
end
local calcs = require("Modules.CalcBase")
local directMode=sniperActorDirect==true
-- A test-only catalog selects which physical families to observe. The default
-- retains the published Sniper report shape and values exactly.
local families=sniperActorFamilies or { ["Metadata/Items/Gems/SkillGemSkeletalSniper"]="SummonSkeletalSnipersPlayer" }
local effects={}
for gem,effect in pairs(families) do assert(type(gem)=="string" and type(effect)=="string" and not effects[effect]);effects[effect]=true end
assert(djinnOriginals and djinnOriginals.preserved_after_load)
local refs = djinnOriginals.refs
assert(refs.load_skills == common.classes.SkillsTab.Load and refs.load_skill == common.classes.SkillsTab.LoadSkill)
assert(refs.process_group == common.classes.SkillsTab.ProcessSocketGroup)
assert(refs.init == calcs.initEnv and refs.create == calcs.createActiveSkill and refs.mods == calcs.buildActiveSkillModList)
assert(refs.perform == calcs.perform and refs.output == calcs.buildOutput)
local childFunction = calcs.createMinionSkills
local info = debug.getinfo(childFunction, "S")
assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-#"Modules/CalcActiveSkill.lua") == "Modules/CalcActiveSkill.lua" and info.linedefined == 1116)
local function scalars(t)
 local r = {}
 for k, v in pairs(t or {}) do
  if type(k) == "string" and (type(v) == "string" or type(v) == "number" or type(v) == "boolean") then r[k] = v end
 end
 return r
end
local function same(a,b)
 for k,v in pairs(a) do
  if type(v)=="table" then if type(b[k])~="table" or not same(v,b[k]) then return false end
  elseif b[k]~=v then return false end
 end
 for k in pairs(b) do if a[k]==nil then return false end end
 return true
end
local function maps(t)
 local r = {}
 for effect, values in pairs(t or {}) do
  assert(type(effect)=="string" and type(values)=="table")
  local entries = {}
  for index,value in pairs(values) do
   assert(type(index)=="number" and type(value)=="number")
   entries[#entries+1]={child_index=index,stat_set_index=value}
  end
  table.sort(entries,function(a,b)return a.child_index<b.child_index end)
  r[effect]=entries
 end
 return r
end
local function fields(g)
 local r={gem_id=g.gemId,skill_id=g.skillId,level=g.level,quality=g.quality,enabled=g.enabled,count=g.count,
  global_1=g.enableGlobal1,global_2=g.enableGlobal2,actor_main=g.skillMinion,actor_calcs=g.skillMinionCalcs,
  child_main=g.skillMinionSkill,child_calcs=g.skillMinionSkillCalcs,
  child_maps_main=maps(g.skillMinionSkillStatSetIndexLookup),child_maps_calcs=maps(g.skillMinionSkillStatSetIndexLookupCalcs),
  parent_maps_main=scalars(g.statSet),parent_maps_calcs=scalars(g.statSetCalcs)}
 if directMode then r.corrupted=g.corrupted;r.corruption_level=g.corruptLevel end
 return r
end
local function groupFields(g)
 return {enabled=g.enabled,slot_enabled=g.slotEnabled,group_count=g.groupCount,include_in_full_dps=g.includeInFullDPS,
  main=g.mainActiveSkill,calcs=g.mainActiveSkillCalcs,source=g.source,slot=g.slot}
end
local function tree(n, ordinals, depth)
 assert(depth<8)
 local r={source_ordinal=ordinals[n],name=n.elem,attributes=scalars(n.attrib),children={}}
 for _,c in ipairs(n) do if type(c)=="table" and c.elem then r.children[#r.children+1]=tree(c,ordinals,depth+1) end end
 return r
end
local function statSets(effect)
 local r={}
 for i,s in ipairs(effect.statSets or {}) do
  r[#r+1]={index=i,id=s.id,label=s.label,stat_description_scope=s.statDescriptionScope,base_flags=scalars(s.baseFlags)}
 end
 return r
end
local function selectedSet(effect,mode)
 local chosen=assert(mode=="CALCS" and effect.statSetCalcs or effect.statSet)
 assert(effect.grantedEffect.statSets[chosen.index]==chosen.statSet and chosen.statSet)
 return {index=chosen.index,declared_table_identity=true,id=chosen.statSet.id,label=chosen.statSet.label,
  stat_description_scope=chosen.statSet.statDescriptionScope}
end
local selection={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
 spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local environments={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local outputs={MAIN=assert(build.calcsTab.mainOutput),CALCS=assert(build.calcsTab.calcsOutput)}
local snapshots={MAIN=scalars(outputs.MAIN),CALCS=scalars(outputs.CALCS)}
local doc,err=common.xml.ParseXML(sniperActorXml);assert(doc and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)
 if type(n)~="table" or not n.elem then return end
 assert(nextOrdinal<100000)
 ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1
 for _,c in ipairs(n) do enumerate(c) end
end
enumerate(doc[1])
local skills
for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Skills" then assert(not skills);skills=n end end
assert(skills)
local origins,rows,saved={},{},{}
local earlier=sniperActorPhysicalReferences
sniperActorPhysicalReferences=sniperActorPhysicalReferences or {}
for _,set in ipairs(skills) do if type(set)=="table" and set.elem=="SkillSet" then
 local sid=assert(tonumber(set.attrib.id))
 local runtimeSet=assert(build.skillsTab.skillSets[sid])
 local gi=0
 for _,node in ipairs(set) do if type(node)=="table" and node.elem=="Skill" then
  gi=gi+1
  local captured=assert(sniperLoadedGroups[sid][gi])
  local group,gemIndex=captured.group,0
  assert(captured.set==runtimeSet)
  for _,child in ipairs(node) do if type(child)=="table" and child.elem=="Gem" then
   gemIndex=gemIndex+1
   if families[child.attrib.gemId] and (not directMode or node.attrib.source==nil or node.attrib.source=="" or node.attrib.source=="nil") then
    local gem=assert(captured.gems[gemIndex])
    assert(group.gemList[gemIndex]==gem,"source Sniper physical object was replaced")
    local runtimeIndex
    for i,g in ipairs(runtimeSet.socketGroupList) do if g==group then assert(not runtimeIndex);runtimeIndex=i end end
    assert(runtimeIndex,"source Sniper group was removed")
    assert(#rows<256 and not origins[gem])
    local d=assert(gem.gemData)
    assert((directMode and d.gameId==child.attrib.gemId and d.variantId==child.attrib.variantId or not directMode and d.id==child.attrib.gemId) and gem.skillId==child.attrib.skillId)
    assert(d.grantedEffect.id==families[child.attrib.gemId] and d.grantedEffectList[1]==d.grantedEffect)
    if earlier then
     local p=assert(earlier[ordinals[child]])
     assert(p.gem==gem and p.group==group and p.set==runtimeSet)
    else sniperActorPhysicalReferences[ordinals[child]]={gem=gem,group=group,set=runtimeSet} end
    local effects={}
    for i,effect in ipairs(d.grantedEffectList) do effects[#effects+1]={index=i,id=effect.id} end
    local row={source_ordinal=ordinals[child],group_source_ordinal=ordinals[node],preset_source_ordinal=ordinals[set],
     preset=sid,source_group_index=gi,group=runtimeIndex,index=gemIndex,selected=sid==selection.skills,attributes=scalars(child.attrib),
     source_tree=tree(child,ordinals,0),group_attributes=scalars(node.attrib),loaded=fields(gem),group_state=groupFields(group),
     effects=effects,resolved_additional_count=#d.additionalGrantedEffects,MAIN={},CALCS={}}
    origins[gem]=row;rows[#rows+1]=row
    saved[#saved+1]={gem=gem,group=group,set=runtimeSet,row=row}
   end
  end end
 end end
end end
local selectors={}
for mode,env in pairs(environments) do
 local main=assert(env.player.mainSkill)
 selectors[mode]={group=env.mainSocketGroup,effect=main.activeEffect.grantedEffect.id,
  selected_actor_type=env.minion and env.minion.type,selected_child=env.minion and env.minion.mainSkill and env.minion.mainSkill.activeEffect.grantedEffect.id}
 local seenActors,seenChildren={},{}
 for _,summon in ipairs(env.player.activeSkillList) do
  local effect=summon.activeEffect
  if effects[effect.grantedEffect.id] and (not directMode or origins[effect.srcInstance]) then
   local row=assert(origins[effect.srcInstance],"summon has no exact saved physical source")
   assert(row.selected and summon.socketGroup==build.skillsTab.skillSets[row.preset].socketGroupList[row.group])
   assert(summon.actor==env.player and effect.grantedEffect==effect.srcInstance.gemData.grantedEffect)
   local actor=assert(summon.minion)
   assert(not seenActors[actor]);seenActors[actor]=true
   assert(actor.minionData==env.data.minions[actor.type] and actor.parent==env.player and actor.enemy==env.enemy)
   local choices={}
   for _,id in ipairs(summon.minionList) do choices[#choices+1]=id end
   local declared={}
   for i,id in ipairs(actor.minionData.skillList) do declared[#declared+1]={index=i,id=id,resolved=env.data.skills[id]~=nil} end
   local children={}
   for i,child in ipairs(actor.activeSkillList) do
    assert(not seenChildren[child]);seenChildren[child]=true
    assert(child.actor==actor and child.summonSkill==summon)
    local childEffect=child.activeEffect
    assert(childEffect.grantedEffect==env.data.skills[childEffect.grantedEffect.id])
    children[#children+1]={index=i,effect=childEffect.grantedEffect.id,name=childEffect.grantedEffect.name,
     exact_actor=true,exact_summon=true,source_instance_present=childEffect.srcInstance~=nil,selected=actor.mainSkill==child,
     stat_sets=statSets(childEffect.grantedEffect),stat_set=selectedSet(childEffect,mode),level=childEffect.level,quality=childEffect.quality,
     output_available=child.output~=nil,output=scalars(child.output)}
   end
   local selectedIndex
   for i,child in ipairs(actor.activeSkillList) do if child==actor.mainSkill then assert(not selectedIndex);selectedIndex=i end end
   local nameField=mode=="CALCS" and summon==main and "skillMinionCalcs" or "skillMinion"
   local childField=mode=="CALCS" and "skillMinionSkillCalcs" or "skillMinionSkill"
   row[mode][#row[mode]+1]={source_ordinal=row.source_ordinal,exact_physical_object=true,exact_group=true,
    effect=effect.grantedEffect.id,is_main_skill=summon==main,stat_set=selectedSet(effect,mode),
    actor_selector_field=nameField,actor_selector_value=effect.srcInstance[nameField],child_selector_field=childField,
    child_selector_value=effect.srcInstance[childField],minion_choices=choices,
    actor={type=actor.type,source_data_identity=true,unique_actor_identity=true,item_set_present=actor.itemSet~=nil,
     level=actor.level,is_selected_actor=env.minion==actor,declared_children=declared,children=children,
     selected_child_index=selectedIndex,selected_child=actor.mainSkill and actor.mainSkill.activeEffect.grantedEffect.id,
     output_available=actor.output~=nil,output=scalars(actor.output)}}
  end
 end
end
for _,s in ipairs(saved) do
 assert(s.set.socketGroupList[s.row.group]==s.group and s.group.gemList[s.row.index]==s.gem)
 assert(same(fields(s.gem),s.row.loaded) and same(groupFields(s.group),s.row.group_state))
end
assert(build.skillsTab.activeSkillSetId==selection.skills and build.itemsTab.activeItemSetId==selection.items)
assert(build.treeTab.activeSpec==selection.spec and build.configTab.activeConfigSetId==selection.config and build.mainSocketGroup==selection.group)
for mode,env in pairs(environments) do
 assert((mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)==env)
 assert((mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)==outputs[mode])
 assert(same(scalars(outputs[mode]),snapshots[mode]))
end
assert(refs.load_skill==build.skillsTab.LoadSkill and refs.process_group==build.skillsTab.ProcessSocketGroup)
assert(refs.output==calcs.buildOutput and refs.perform==calcs.perform and childFunction==calcs.createMinionSkills)
assert(sniperLoaderHookRemoved and not debug.gethook() and jit.status()==sniperActorJit)
local result={saved=rows,selection=selection,selectors=selectors,outputs=snapshots,output_revision=build.outputRevision,
 loader_observer_removed=true,requested_jit_mode_verified=true,
 build_flag=build.buildFlag==true,original_functions=djinnOriginals.auth,output_lifecycle=djinnOriginals.output_lifecycle,
 source_methods_preserved=true,exact_physical_objects=true,physical_objects_preserved_across_stages=true,
 saved_inputs_preserved=true,selected_state_preserved=true,reported_outputs_preserved=true}
if directMode then
 result.direct=assert(sniperDirectExtra)(rows,saved,origins,environments,effects,scalars,statSets,selectedSet,fields,groupFields)
 result.exact_source_objects=true;result.source_objects_preserved_across_stages=true
 result.exact_physical_objects=nil;result.physical_objects_preserved_across_stages=nil
 for _,row in ipairs(rows) do
  row.source_kind="manual_direct"
  for _,mode in ipairs({"MAIN","CALCS"}) do for _,action in ipairs(row[mode]) do
   action.exact_source_object=action.exact_physical_object;action.exact_physical_object=nil
  end end
 end
end
return result
