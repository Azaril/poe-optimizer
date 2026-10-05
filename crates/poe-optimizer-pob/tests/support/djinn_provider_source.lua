-- Observe the unchanged complete source lifecycle. No business method is replaced.
local function original(f,path,line)
 assert(type(f)=="function","missing function "..path..":"..line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==line,"unexpected function "..s..":"..i.linedefined)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,wanted)
 for index=1,100 do local name,value=debug.getupvalue(f,index);if not name then break end;if name==wanted then return value end end
 error("missing original upvalue "..wanted)
end
local calcs=require("Modules.CalcBase")
local methods={
 {"load_skills",common.classes.SkillsTab,"Load","Classes/SkillsTab.lua",407},
 {"load_skill",common.classes.SkillsTab,"LoadSkill","Classes/SkillsTab.lua",303},
 {"process_group",common.classes.SkillsTab,"ProcessSocketGroup","Classes/SkillsTab.lua",1242},
 {"load_spec",common.classes.PassiveSpec,"Load","Classes/PassiveSpec.lua",117},
 {"post_load_spec",common.classes.PassiveSpec,"PostLoad","Classes/PassiveSpec.lua",353},
 {"calcs_constructor_wrapper",common.classes.CalcsTab,"CalcsTab","Modules/Common.lua",167},
 {"compare_constructor_wrapper",common.classes.CompareTab,"CompareTab","Modules/Common.lua",167},
 {"calcs_tab_output",common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {"init",calcs,"initEnv","Modules/CalcSetup.lua",717},
 {"create",calcs,"createActiveSkill","Modules/CalcActiveSkill.lua",144},
 {"mods",calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
 {"minion_skills",calcs,"createMinionSkills","Modules/CalcActiveSkill.lua",1116},
 {"perform",calcs,"perform","Modules/CalcPerform.lua",1193},
 {"output",calcs,"buildOutput","Modules/Calcs.lua",469},
}
local constructor,constructorAuth=original(upvalue(common.classes.CalcsTab.CalcsTab,"originalFunc"),"Classes/CalcsTab.lua",22)
local compareConstructor,compareAuth=original(upvalue(common.classes.CompareTab.CompareTab,"originalFunc"),"Classes/CompareTab.lua",121)
if djinnPhase=="before"then
 local refs,auth={},{};for _,m in ipairs(methods)do refs[m[1]],auth[m[1]]=original(m[2][m[3]],m[4],m[5])end
 auth.calcs_constructor_original=constructorAuth
 auth.compare_constructor_original=compareAuth
 local outputBytecode=string.dump(refs.output)
 assert(upvalue(refs.output,"calcs")==calcs)
 local moduleStart=#_configuration_source_modules
 djinnOriginals={refs=refs,auth=auth,constructor=constructor,compare_constructor=compareConstructor}
 return function()
  for _,m in ipairs(methods)do if m[1]~="output"then assert(m[2][m[3]]==refs[m[1]],"source method changed: "..m[1])end end
  assert(upvalue(common.classes.CalcsTab.CalcsTab,"originalFunc")==constructor,"source constructor body changed")
  assert(upvalue(common.classes.CompareTab.CompareTab,"originalFunc")==compareConstructor,"source comparison constructor body changed")
  -- Build constructs CalcsTab and CompareTab, whose unchanged constructors
  -- reload Modules/Calcs at lines 29 and 178 respectively. That
  -- recreates buildOutput while the required calculation modules stay cached.
  local reloads=0;local reloadTrace={}
  for index=moduleStart+1,#_configuration_source_modules do
   local path=_configuration_source_modules[index]:gsub("\\","/")
   if #reloadTrace<64 then reloadTrace[#reloadTrace+1]=path end
   if path:sub(-#"Modules/Calcs.lua")=="Modules/Calcs.lua"then reloads=reloads+1 end
  end
  assert(reloads==2,"unexpected calculation module reload count "..reloads.."; entries "..(#_configuration_source_modules-moduleStart).."; bounded trace "..table.concat(reloadTrace," | "))
  local output,outputAuth=original(calcs.buildOutput,"Modules/Calcs.lua",469)
  assert(output~=refs.output,"expected fresh calculation module closure")
  assert(string.dump(output)==outputBytecode,"reloaded buildOutput bytecode changed")
  assert(upvalue(output,"calcs")==calcs and build.calcsTab.calcs==calcs,"calculation table identity changed")
  assert(build.compareTab.calcs==calcs,"comparison calculation table identity changed")
  assert(outputAuth.last==auth.output.last)
  refs.output=output
  djinnOriginals.output_lifecycle={module_reloads=reloads,recreated=true,bytecode_equal=true,calcs_upvalue_exact=true,tab_calcs_exact=true,compare_calcs_exact=true}
  djinnOriginals.preserved_after_load=true
 end
end
assert(djinnPhase=="observe"and djinnOriginals.preserved_after_load)
assert(constructor==djinnOriginals.constructor,"source constructor body changed during observation")
assert(compareConstructor==djinnOriginals.compare_constructor)
local function scalar(v)if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end;return v end
local function scalars(t)
 local r,positions={},{};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then
  if djinnOccurrenceNumericEntries and type(k)=="number"then positions[#positions+1]={index=k,value=scalar(v)}else r[k]=scalar(v)end
 end end
 if #positions>0 then table.sort(positions,function(a,b)return a.index<b.index end);assert(r.numeric_entries==nil);r.numeric_entries=positions end
 return r
end
local function plain(v,depth)
 if type(v)~="table"then assert(v==nil or type(v)=="number"or type(v)=="boolean"or type(v)=="string");return scalar(v)end
 depth=(depth or 0)+1;assert(depth<16);local r={};local n=0;local positions={}
 for k,x in pairs(v)do n=n+1;assert(n<10000)
  if djinnOccurrenceNumericEntries and type(k)=="number"then positions[#positions+1]={index=k,value=plain(x,depth)}
  else r[k]=plain(x,depth)end
 end
 if #positions>0 then table.sort(positions,function(a,b)return a.index<b.index end);assert(r.numeric_entries==nil);r.numeric_entries=positions end
 return r
end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local effects={SummonSandDjinnPlayer=13289,SummonWaterDjinnPlayer=32705}
local function key(sid,effect,source)return tostring(sid).."/"..effect.."/"..(source or"manual")end
local function gem_fields(g)
 return {fields=scalars(g),stat_set=plain(g.statSet),stat_set_calcs=plain(g.statSetCalcs),minion_stat_sets=plain(g.skillMinionSkillStatSetIndexLookup),minion_stat_sets_calcs=plain(g.skillMinionSkillStatSetIndexLookupCalcs),gem_definition=g.gemData and g.gemData.id,effect=g.gemData and g.gemData.grantedEffect.id or g.grantedEffect and g.grantedEffect.id,from_tree=g.gemData and g.gemData.grantedEffect.fromTree or g.grantedEffect and g.grantedEffect.fromTree}
end
local function group_fields(g)local gems={};for _,gem in ipairs(g.gemList)do gems[#gems+1]=gem_fields(gem)end;return {fields=scalars(g),source_node_id=g.sourceNode and g.sourceNode.id,gems=gems}end
local doc,err=common.xml.ParseXML(djinnXml);assert(doc and not err)
local ordinal={};local nextOrdinal=0
local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinal[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,c in ipairs(n)do enumerate(c)end end;enumerate(doc[1])
local skills,tree;for _,n in ipairs(doc[1])do if type(n)=="table"then if n.elem=="Skills"then assert(not skills);skills=n elseif n.elem=="Tree"then assert(not tree);tree=n end end end;assert(skills and tree)
local selected={skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,items=build.itemsTab.activeItemSetId,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup,calcs_group=build.calcsTab.calcsEnv.mainSocketGroup}
local rawGroups,rawByKey={},{}
for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then
 local gi=0;for _,g in ipairs(set)do if type(g)=="table"and g.elem=="Skill"then
  gi=gi+1;local gems={};for _,gem in ipairs(g)do if type(gem)=="table"and gem.elem=="Gem"then gems[#gems+1]={source=ordinal[gem],attributes=plain(gem.attrib),children=plain(gem)}end end
  if gems[1]and effects[gems[1].attributes.skillId]then
   local k=key(tonumber(set.attrib.id),gems[1].attributes.skillId,g.attrib.source);assert(not rawByKey[k])
   local row={key=k,set=tonumber(set.attrib.id),index=gi,source=ordinal[g],attributes=plain(g.attrib),gems=gems};rawGroups[#rawGroups+1]=row;rawByKey[k]=row
  end
 end end
end end
local rawSpec;local si=0;for _,s in ipairs(tree)do if type(s)=="table"and s.elem=="Spec"then si=si+1;if si==selected.spec then rawSpec={source=ordinal[s],attributes=plain(s.attrib)}end end end;assert(rawSpec)
local saved,allGroups,origins={}, {}, {}
for sid,set in pairs(build.skillsTab.skillSets)do for gi,g in ipairs(set.socketGroupList)do
 local state=group_fields(g);local refs={};for i,gem in ipairs(g.gemList)do refs[i]=gem end
 saved[#saved+1]={set=sid,index=gi,group=g,gems=refs,state=state}
 if g.gemList[1]and effects[g.gemList[1].skillId]then
  local k=key(sid,g.gemList[1].skillId,g.source);local raw=rawByKey[k]
  local row={key=k,set=sid,index=gi,selected=sid==selected.skills,source=g.source,raw_source=raw and raw.source,raw_present=raw~=nil,state=state,group=g}
  if raw then assert(#g.gemList==#raw.gems)end
  for i,gem in ipairs(g.gemList)do
   assert(not origins[gem]);if raw then assert(gem.skillId==raw.gems[i].attributes.skillId)end
   origins[gem]={key=k,index=i,source=raw and raw.gems[i].source,source_present=raw~=nil,effect=gem.skillId}
  end
  allGroups[#allGroups+1]=row
 end
end end
table.sort(allGroups,function(a,b)if a.set~=b.set then return a.set<b.set end;return a.index<b.index end)
local envs={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local outputs={MAIN=assert(build.calcsTab.mainOutput),CALCS=assert(build.calcsTab.calcsOutput)}
local outputCopies={MAIN=scalars(outputs.MAIN),CALCS=scalars(outputs.CALCS)}
local function support(effect)
 local origin=origins[effect.srcInstance]
 return {effect=effect.grantedEffect.id,level=effect.level,quality=effect.quality,source_instance_present=effect.srcInstance~=nil,origin=plain(origin),origin_known=origin~=nil}
end
local function support_rows(a)
 local candidates,accepted,rejected={},{},{}
 for i,e in ipairs(a.supportList)do
  local present=false;for _,v in ipairs(a.effectList)do if v==e then assert(not present);present=true end end
  local row=support(e);row.candidate_index=i;row.accepted=present;candidates[#candidates+1]=row
  if present then accepted[#accepted+1]=i else rejected[#rejected+1]=i end
 end
 for i,e in ipairs(a.effectList)do if i>1 and e.grantedEffect.support then local found=false;for _,c in ipairs(a.supportList)do if c==e then found=true end end;assert(found,"accepted support missing candidate identity")end end
 return {candidates=candidates,accepted_indices=accepted,rejected_indices=rejected}
end
local function skill(a,env)
 local e=a.activeEffect;local set=assert(env.mode=="CALCS"and e.statSetCalcs or e.statSet)
 assert(e.grantedEffect.statSets[set.index]==set.statSet)
 return {effect=e.grantedEffect.id,level=e.level,quality=e.quality,stat_set_index=set.index,stat_set_id=set.statSet.id,flags=scalars(set.skillFlags),source_instance_present=e.srcInstance~=nil,supports=support_rows(a),skill_data=scalars(a.skillData),output_available=a.output~=nil,output=a.output and scalars(a.output)}
end
local function action(a,env,g)
 assert(a.activeEffect.srcInstance==g.gemList[1]and a.socketGroup==g and a.actor==env.player)
 local row=skill(a,env);row.source_instance=plain(origins[a.activeEffect.srcInstance]);row.actor_is_player=true;row.group_exact=true;row.is_main=env.player.mainSkill==a
 local effectIndex;local gem=g.gemList[1]
 for index,effect in ipairs(gem.gemData and gem.gemData.grantedEffectList or{gem.grantedEffect})do
  if effect==a.activeEffect.grantedEffect then assert(not effectIndex);effectIndex=index end
 end
 assert(effectIndex,"action is not an exact effect of its source instance");row.granted_effect_index=effectIndex
 local minion=a.minion;row.minion_available=minion~=nil
 if minion then
  local children={};local chosen
  for i,child in ipairs(minion.activeSkillList)do
   assert(child.actor==minion and child.summonSkill==a and child.supportList==a.supportList)
   local r=skill(child,env);r.index=i;r.actor_is_parent_minion=true;r.summon_skill_exact=true;r.support_list_same_parent=true;r.selected=minion.mainSkill==child
   if r.selected then assert(not chosen);chosen=i end;children[#children+1]=r
  end
  assert(chosen)
  row.minion={type=minion.type,level=minion.level,parent_is_player=minion.parent==env.player,enemy_exact=minion.enemy==env.enemy,selected_index=chosen,children=children,output_available=minion.output~=nil,output=minion.output and scalars(minion.output)}
 end
 return row
end
local result={selection=selected,raw_groups=rawGroups,raw_spec=rawSpec,runtime_groups={},modes={},outputs=outputCopies,method_sources=djinnOriginals.auth,output_lifecycle=djinnOriginals.output_lifecycle}
for _,g in ipairs(allGroups)do
 local row={key=g.key,set=g.set,index=g.index,selected=g.selected,source=g.source,raw_source=g.raw_source,raw_present=g.raw_present,state=g.state,actions={MAIN={},CALCS={}},source_nodes={}}
 for mode,env in pairs(envs)do
  local node=g.group.sourceNode
  row.source_nodes[mode]={present=node~=nil,id=node and node.id,exact_allocated=node~=nil and env.allocNodes[node.id]==node,exact_selected_spec=node~=nil and build.spec.allocNodes[node.id]==node}
  for _,a in ipairs(env.player.activeSkillList)do if a.activeEffect.srcInstance==g.group.gemList[1]then assert(g.selected);row.actions[mode][#row.actions[mode]+1]=action(a,env,g.group)end end
 end
 result.runtime_groups[#result.runtime_groups+1]=row
end
for mode,env in pairs(envs)do
 local nodes={};for id in pairs(env.allocNodes)do nodes[#nodes+1]=id end;table.sort(nodes)
 local granted={};for _,grant in ipairs(env.grantedSkillsNodes)do if effects[grant.skillId]then
  local node=assert(grant.sourceNode);assert(env.allocNodes[node.id]==node)
  local raw={};for _,g in ipairs(node.grantedSkills)do if g.skillId==grant.skillId then raw[#raw+1]=scalars(g)end end
  granted[#granted+1]={fields=scalars(grant),source_node_id=node.id,source_node_exact=true,node_grants=raw}
 end end;table.sort(granted,function(a,b)return a.source_node_id<b.source_node_id end)
 local active={};for _,a in ipairs(env.player.activeSkillList)do if effects[a.activeEffect.grantedEffect.id]then
  local origin=assert(origins[a.activeEffect.srcInstance],"Djinn action has no exact runtime group")
  active[#active+1]={effect=a.activeEffect.grantedEffect.id,origin=plain(origin)}
 end end
 local main=assert(env.player.mainSkill)
 result.modes[mode]={allocated_ids=nodes,grants=granted,actions=active,main_effect=main.activeEffect.grantedEffect.id,main_group=env.mainSocketGroup,player_output=scalars(env.player.output),minion_output_available=env.minion~=nil and env.minion.output~=nil,minion_output=env.minion and env.minion.output and scalars(env.minion.output)}
end
for _,s in ipairs(saved)do local g=build.skillsTab.skillSets[s.set].socketGroupList[s.index];assert(g==s.group and equal(group_fields(g),s.state));for i,gem in ipairs(s.gems)do assert(g.gemList[i]==gem)end end
assert(selected.skills==build.skillsTab.activeSkillSetId and selected.spec==build.treeTab.activeSpec and selected.items==build.itemsTab.activeItemSetId and selected.config==build.configTab.activeConfigSetId and selected.main_group==build.mainSocketGroup and selected.calcs_group==build.calcsTab.calcsEnv.mainSocketGroup)
for mode,env in pairs(envs)do assert((mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)==env);assert((mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)==outputs[mode]);assert(equal(scalars(outputs[mode]),outputCopies[mode]))end
for _,m in ipairs(methods)do assert(m[2][m[3]]==djinnOriginals.refs[m[1]])end
assert(upvalue(common.classes.CalcsTab.CalcsTab,"originalFunc")==djinnOriginals.constructor)
assert(upvalue(common.classes.CompareTab.CompareTab,"originalFunc")==djinnOriginals.compare_constructor)
result.original_functions_preserved=true;result.saved_instances_preserved=true;result.selected_state_preserved=true;result.outputs_preserved=true
return result
