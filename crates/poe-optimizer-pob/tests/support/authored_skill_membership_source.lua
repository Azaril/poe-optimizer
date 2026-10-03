-- Read-only source ownership census. The original loader-return observer has
-- already retained exact source objects and removed its hook before evaluation.
assert(djinnOriginals and djinnOriginals.preserved_after_load)
assert(sniperLoaderHookRemoved and sniperRequestedJitVerified and not debug.gethook())
assert(jit.status()==sniperActorJit)
local refs=djinnOriginals.refs
local calcs=require("Modules.CalcBase")
assert(refs.load_skills==common.classes.SkillsTab.Load and refs.load_skill==common.classes.SkillsTab.LoadSkill)
assert(refs.process_group==common.classes.SkillsTab.ProcessSocketGroup)
assert(refs.init==calcs.initEnv and refs.create==calcs.createActiveSkill and refs.mods==calcs.buildActiveSkillModList)
assert(refs.perform==calcs.perform and refs.output==calcs.buildOutput)
local function scalars(t)
 local r={}
 for k,v in pairs(t or {}) do
  if type(k)=="string" and (type(v)=="string" or type(v)=="boolean" or type(v)=="number") then
   if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then r[k]=tostring(v) else r[k]=v end
  end
 end
 return r
end
local doc,err=common.xml.ParseXML(membershipXml);assert(doc and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)
 if type(n)~="table" or not n.elem then return end
 assert(nextOrdinal<100000);ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1
 for _,child in ipairs(n) do enumerate(child) end
end
enumerate(doc[1])
local function tree(n,depth)
 assert(depth<8)
 local r={source_ordinal=ordinals[n],name=n.elem,attributes=scalars(n.attrib),children={}}
 for _,child in ipairs(n) do if type(child)=="table" and child.elem then r.children[#r.children+1]=tree(child,depth+1) end end
 return r
end
local skills
for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Skills" then assert(not skills);skills=n end end
assert(skills)
local selection={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
 spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local envs={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local groups,gems,sourceRows,presets={},{},{},{}
local function item_id(item)
 if not item then return nil end
 local id
 for k,v in pairs(build.itemsTab.items) do if v==item then assert(not id);id=k end end
 assert(id,"source item must be an actual loaded item")
 return id
end
local function gem_state(g)
 local d=g.gemData;local e=d and d.grantedEffect or g.grantedEffect
 local effects={}
 for i,x in ipairs(d and d.grantedEffectList or (e and {e} or {})) do
  effects[#effects+1]={index=i,id=x.id,support=x.support==true,from_tree=x.fromTree==true,from_item=x.fromItem==true}
 end
 return {fields=scalars(g),catalog=d and {id=d.id,game_id=d.gameId,variant_id=d.variantId,primary_effect=d.grantedEffect.id},effects=effects}
end
local function group_state(g)
 return {fields=scalars(g),source_item_id=item_id(g.sourceItem),source_node_id=g.sourceNode and g.sourceNode.id}
end
for _,set in ipairs(skills) do if type(set)=="table" and set.elem=="SkillSet" then
 local sid=assert(tonumber(set.attrib.id));local runtimeSet=assert(build.skillsTab.skillSets[sid])
 presets[#presets+1]={id=sid,source_ordinal=ordinals[set],attributes=scalars(set.attrib),selected=sid==selection.skills}
 local gi=0
 for _,node in ipairs(set) do if type(node)=="table" and node.elem=="Skill" then
  gi=gi+1;assert(#sourceRows<4096)
  local captured=assert(sniperLoadedGroups[sid][gi]);assert(captured.set==runtimeSet)
  local g=captured.group;assert(not groups[g]);groups[g]={preset=sid,source_ordinal=ordinals[node],source_group_index=gi}
  local runtimeIndex
  for i,v in ipairs(runtimeSet.socketGroupList) do if v==g then assert(not runtimeIndex);runtimeIndex=i end end
  local row={preset=sid,source_ordinal=ordinals[node],preset_source_ordinal=ordinals[set],source_group_index=gi,
   selected=sid==selection.skills,attributes=scalars(node.attrib),loaded_object_exact=true,runtime_present=runtimeIndex~=nil,
   runtime_group_index=runtimeIndex,state=group_state(g),gems={}}
  local childIndex=0
  for _,child in ipairs(node) do if type(child)=="table" then
   childIndex=childIndex+1;local gem=assert(captured.gems[childIndex]);assert(not gems[gem])
   gems[gem]={source_ordinal=ordinals[child],group_source_ordinal=ordinals[node],preset=sid,source_gem_index=childIndex}
   local currentIndex
   for i,v in ipairs(g.gemList) do if v==gem then assert(not currentIndex);currentIndex=i end end
   row.gems[#row.gems+1]={source_ordinal=ordinals[child],source_index=childIndex,source=tree(child,0),loaded_object_exact=true,
    retained_in_group=currentIndex~=nil,current_index=currentIndex,state=gem_state(gem)}
  end end
  assert(childIndex==#captured.gems)
  sourceRows[#sourceRows+1]=row
 end end
end end
local runtimeRows,runtimeOrigins={},{ }
for _,p in ipairs(presets) do
 local set=build.skillsTab.skillSets[p.id]
 for i,g in ipairs(set.socketGroupList) do
  assert(#runtimeRows<4096)
  local origin=groups[g];local row={preset=p.id,index=i,selected=p.selected,state=group_state(g),gems={},
   saved_group_present=origin~=nil,source_ordinal=origin and origin.source_ordinal}
  runtimeOrigins[g]={preset=p.id,index=i,source_ordinal=origin and origin.source_ordinal}
  for j,gem in ipairs(g.gemList) do
   local source=gems[gem]
   row.gems[#row.gems+1]={index=j,saved_object_present=source~=nil,source_ordinal=source and source.source_ordinal,state=gem_state(gem)}
  end
  runtimeRows[#runtimeRows+1]=row
 end
end
local grants,actions,outputs={},{},{}
for _,mode in ipairs({"MAIN","CALCS"}) do
 local env=envs[mode];grants[mode]={};actions[mode]={}
 for _,grant in ipairs(env.grantedSkills or {}) do
  assert(#grants[mode]<4096)
  local matched={}
  for i,g in ipairs(build.skillsTab.socketGroupList) do
   if g.source==grant.source and g.slot==grant.slotName and g.gemList[1] and g.gemList[1].skillId==grant.skillId then
    matched[#matched+1]={runtime_group_index=i,group_source_item_exact=g.sourceItem==grant.sourceItem,
     group_source_node_exact=g.sourceNode==grant.sourceNode,source_ordinal=groups[g] and groups[g].source_ordinal}
   end
  end
  grants[mode][#grants[mode]+1]={fields=scalars(grant),source_item_id=item_id(grant.sourceItem),
   source_node_id=grant.sourceNode and grant.sourceNode.id,matched_groups=matched}
 end
 for i,a in ipairs(env.player.activeSkillList) do
  assert(i<=4096);local effect=assert(a.activeEffect);local group=runtimeOrigins[a.socketGroup];local source=gems[effect.srcInstance]
  local gemIndex
  if a.socketGroup then for j,g in ipairs(a.socketGroup.gemList) do if g==effect.srcInstance then assert(not gemIndex);gemIndex=j end end end
  actions[mode][#actions[mode]+1]={index=i,effect=effect.grantedEffect.id,actor_is_player=a.actor==env.player,
   is_main=env.player.mainSkill==a,output_available=a.output~=nil,group=group,source=source,
   exact_runtime_gem_index=gemIndex,source_instance_present=effect.srcInstance~=nil}
 end
 outputs[mode]={available=(mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)~=nil}
end
assert(not debug.gethook() and jit.status()==sniperActorJit)
return {selection=selection,presets=presets,saved_groups=sourceRows,runtime_groups=runtimeRows,granted_skills=grants,
 actions=actions,outputs=outputs,source_functions=djinnOriginals.auth,output_lifecycle=djinnOriginals.output_lifecycle,
 exact_loader_capture=true,observer_removed_before_evaluation=true,requested_jit_mode_preserved=true,business_wrappers=false}
