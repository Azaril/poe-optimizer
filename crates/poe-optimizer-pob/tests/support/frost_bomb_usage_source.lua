-- Read-only supplement to active_gem_occurrence_source: every saved Frost source,
-- including archived presets, joins to its exact physical object and actions.
local calcs=require("Modules.CalcBase")
assert(djinnOriginals and djinnOriginals.preserved_after_load)
local refs=djinnOriginals.refs
assert(refs.load_skills==common.classes.SkillsTab.Load and refs.load_skill==common.classes.SkillsTab.LoadSkill)
assert(refs.process_group==common.classes.SkillsTab.ProcessSocketGroup)
assert(refs.init==calcs.initEnv and refs.create==calcs.createActiveSkill and refs.mods==calcs.buildActiveSkillModList)
assert(refs.perform==calcs.perform and refs.output==calcs.buildOutput)
local function copy(t)local r={};for k,v in pairs(t or{})do assert(type(v)~="table");r[k]=v end;return r end
local function fields(g)return{gem_id=g.gemId,skill_id=g.skillId,level=g.level,quality=g.quality,enabled=g.enabled,count=g.count,global_1=g.enableGlobal1,global_2=g.enableGlobal2}end
local doc,err=common.xml.ParseXML(occurrenceXml);assert(doc and not err)
local ordinals={};local nextOrdinal=0
local function enumerate(n)
 if type(n)~="table"or not n.elem then return end
 ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1
 for _,c in ipairs(n)do enumerate(c)end
end
enumerate(doc[1])
local skills;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Skills"then assert(not skills);skills=n end end;assert(skills)
local selected=build.skillsTab.activeSkillSetId
local environments={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local origins={};local rows={};local saved={}
local earlier=frostSavedPhysicalReferences
frostSavedPhysicalReferences=frostSavedPhysicalReferences or{}
for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then
 local sid=assert(tonumber(set.attrib.id));local runtimeSet=assert(build.skillsTab.skillSets[sid]);local gi=0
 for _,node in ipairs(set)do if type(node)=="table"and node.elem=="Skill"then
  gi=gi+1;local group=assert(runtimeSet.socketGroupList[gi]);local gemIndex=0
  for _,child in ipairs(node)do if type(child)=="table"and child.elem=="Gem"then
   gemIndex=gemIndex+1;local gem=assert(group.gemList[gemIndex])
   if child.attrib.gemId=="Metadata/Items/Gems/SkillGemFrostBomb"then
    assert(not origins[gem],"distinct saved sources share a physical object")
    assert(gem.gemData and gem.gemData.id==child.attrib.gemId and gem.skillId==child.attrib.skillId)
    local data=gem.gemData;assert(#data.grantedEffectList==1 and #data.additionalGrantedEffects==0)
    assert(data.grantedEffectList[1]==data.grantedEffect and data.grantedEffect.id=="FrostBombPlayer")
    -- Data.processMod sets this flag lazily when its stat-map entry is used.
    -- Disabled sources need not prepare that entry. Observe; never force it.
    assert(data.grantedEffect.hasGlobalEffect==nil or data.grantedEffect.hasGlobalEffect==true)
    assert(child.attrib.count=="1"and gem.count==1,"witness is deliberately restricted to count1")
    if earlier then
     local prior=assert(earlier[ordinals[child]])
     assert(prior.gem==gem and prior.group==group and prior.set==runtimeSet,"recalculation replaced a saved source object")
    else
     frostSavedPhysicalReferences[ordinals[child]]={gem=gem,group=group,set=runtimeSet}
    end
    local row={source_ordinal=ordinals[child],group_source_ordinal=ordinals[node],preset_source_ordinal=ordinals[set],
     preset=sid,group=gi,index=gemIndex,selected=sid==selected,attributes=copy(child.attrib),group_attributes=copy(node.attrib),loaded=fields(gem),
     group_enabled=group.enabled,slot_enabled=group.slotEnabled,
     global_effect_flag_present=data.grantedEffect.hasGlobalEffect~=nil,
     global_effect_flag=data.grantedEffect.hasGlobalEffect==true,MAIN={},CALCS={}}
    origins[gem]=row;rows[#rows+1]=row;saved[#saved+1]={gem=gem,group=group,row=row,set=runtimeSet}
   end
  end end
 end end
end end
local selectors={}
for mode,env in pairs(environments)do
 local main=assert(env.player.mainSkill)
 selectors[mode]={group=env.mainSocketGroup,effect=main.activeEffect.grantedEffect.id,
  stat_set_index=(mode=="CALCS"and main.activeEffect.statSetCalcs or main.activeEffect.statSet).index,
  minion_effect=env.minion and env.minion.mainSkill and env.minion.mainSkill.activeEffect.grantedEffect.id}
 for _,a in ipairs(env.player.activeSkillList)do
  local effect=a.activeEffect
  if effect.grantedEffect.id=="FrostBombPlayer"then
   local row=assert(origins[effect.srcInstance],"Frost effect has no exact saved physical source")
   assert(row.selected and a.socketGroup==build.skillsTab.skillSets[row.preset].socketGroupList[row.group])
   assert(a.actor==env.player and not a.minion and effect.grantedEffect==effect.srcInstance.gemData.grantedEffectList[1])
   assert(effect.grantedEffect.hasGlobalEffect==true)
   local statSet=assert(mode=="CALCS"and effect.statSetCalcs or effect.statSet)
   assert(statSet.index==1 and effect.grantedEffect.statSets[1]==statSet.statSet)
   row[mode][#row[mode]+1]={source_ordinal=row.source_ordinal,exact_physical_object=true,exact_group=true,actor_is_player=true,
    effect=effect.grantedEffect.id,effect_index=1,has_global_effect=effect.grantedEffect.hasGlobalEffect==true,stat_set_index=statSet.index,
    stat_set_id=statSet.statSet.id,stat_set_label=statSet.statSet.label,level=effect.level,quality=effect.quality}
  end
 end
end
for _,s in ipairs(saved)do
 assert(s.set.socketGroupList[s.row.group]==s.group and s.group.gemList[s.row.index]==s.gem)
 for key,value in pairs(s.row.loaded)do assert(fields(s.gem)[key]==value)end
end
assert(refs.load_skill==build.skillsTab.LoadSkill and refs.process_group==build.skillsTab.ProcessSocketGroup and refs.output==calcs.buildOutput)
return{saved=rows,selectors=selectors,selected_preset=selected,original_functions=djinnOriginals.auth,
 output_lifecycle=djinnOriginals.output_lifecycle,source_methods_preserved=true,exact_physical_objects=true,
 output_revision=build.outputRevision,build_flag=build.buildFlag==true,physical_objects_preserved_across_stages=true}
