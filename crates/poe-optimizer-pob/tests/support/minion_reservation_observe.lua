local calcs=require("Modules.CalcBase")
local capture=assert(reservationCapture);assert(capture.hook_removed and not debug.gethook())
local refs=assert(djinnOriginals.refs)
assert(djinnOriginals.preserved_after_load and refs.output==calcs.buildOutput and refs.perform==calcs.perform)
local function plain(t,depth)
 if type(t)~="table"then assert(t==nil or type(t)=="string"or type(t)=="number"or type(t)=="boolean");return t end
 depth=(depth or 0)+1;assert(depth<16)
 local out={};local count=0;local numeric={};local named=0
 for k,v in pairs(t)do
  count=count+1;assert(count<8192)
  if type(k)=="number"then assert(k==math.floor(k));numeric[#numeric+1]=k
  else assert(type(k)=="string");named=named+1;out[k]=plain(v,depth)end
 end
 table.sort(numeric)
 local sequence=named==0
 for index,key in ipairs(numeric)do if index~=key then sequence=false end end
 if sequence then for _,key in ipairs(numeric)do out[key]=plain(t[key],depth)end
 elseif #numeric>0 then
  -- mlua chooses the array part of mixed Lua tables. Give numeric entries an
  -- explicit named envelope so source level rows never lose either key space.
  assert(t._source_numeric_entries==nil,"source collides with evidence envelope")
  out._source_numeric_entries={}
  for _,key in ipairs(numeric)do out._source_numeric_entries[#out._source_numeric_entries+1]={index=key,value=plain(t[key],depth)}end
 end
 return out
end
local function tabulated(list)
 local out={}
 for _,row in ipairs(list)do
  local mod={};local numeric=0
  for key,value in pairs(row.mod)do
   if type(key)=="string"then mod[key]=plain(value)
   else assert(type(key)=="number"and key>=1 and key==math.floor(key));numeric=numeric+1 end
  end
  assert(row.mod.tags==nil,"source collides with modifier tag envelope")
  mod.tags={};for index,tag in ipairs(row.mod)do mod.tags[index]=plain(tag)end
  assert(#mod.tags==numeric,"sparse modifier tags")
  out[#out+1]={value=plain(row.value),mod=mod}
 end
 return out
end
local function scalars(t)local out={};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then out[k]=v end end;return out end
local reviewed={};for _,gem in ipairs(reservationReviewed)do reviewed[gem.key]=gem end
local parsed,err=common.xml.ParseXML(reservationXml);assert(parsed and not err)
local ordinals={};local nextOrdinal=0
local function enumerate(node)if type(node)~="table"or not node.elem then return end;ordinals[node]=nextOrdinal;nextOrdinal=nextOrdinal+1;assert(nextOrdinal<20000);for _,child in ipairs(node)do enumerate(child)end end
enumerate(parsed[1])
local skills;for _,node in ipairs(parsed[1])do if type(node)=="table"and node.elem=="Skills"then skills=node end end;assert(skills)
local origins,selected,saved={},{},{}
local prior=reservationPhysicalRefs;reservationPhysicalRefs=reservationPhysicalRefs or{}
for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then
 local sid=assert(tonumber(set.attrib.id));local groupIndex=0
 for _,node in ipairs(set)do if type(node)=="table"and node.elem=="Skill"then
  groupIndex=groupIndex+1;local group=assert(build.skillsTab.skillSets[sid].socketGroupList[groupIndex]);local index=0
  for _,child in ipairs(node)do if type(child)=="table"and child.elem=="Gem"then
   index=index+1;local gem=assert(group.gemList[index]);local data=gem.gemData
   if data and reviewed[data.id]then
    assert(not origins[gem]);local source=ordinals[child]
    if prior then assert(prior[source].gem==gem and prior[source].group==group)else reservationPhysicalRefs[source]={gem=gem,group=group}end
    local row={source_ordinal=source,preset=sid,group=groupIndex,index=index,selected=sid==build.skillsTab.activeSkillSetId,attributes=plain(child.attrib),group_attributes=plain(node.attrib),
     loaded={gem_id=gem.gemId,effect=gem.skillId,level=gem.level,quality=gem.quality,count=gem.count,enabled=gem.enabled,group_count=group.groupCount},MAIN={},CALCS={}}
    origins[gem]=row;saved[#saved+1]=row
    if row.selected then selected[#selected+1]={row=row,gem=gem,group=group}end
   end
  end end
 end end
end end
local contexts={}
for mode,env in pairs({MAIN=build.calcsTab.mainEnv,CALCS=build.calcsTab.calcsEnv})do
 local player=env.player;local main=assert(player.mainSkill)
 contexts[mode]={main_group=env.mainSocketGroup,effect=main.activeEffect.grantedEffect.id,
  minion=env.minion and env.minion.type,child=env.minion and env.minion.mainSkill.activeEffect.grantedEffect.id,
  output=scalars(player.output),reserved_spirit_base=player.reserved_SpiritBase,reserved_spirit_percent=player.reserved_SpiritPercent,
  breakdown=player.breakdown and player.breakdown.SpiritReserved and plain(player.breakdown.SpiritReserved.reservations)}
 for _,source in ipairs(selected)do
  for _,skill in ipairs(player.activeSkillList)do if skill.activeEffect.srcInstance==source.gem then
   local effect=skill.activeEffect;local definition=effect.grantedEffect
   assert(skill.actor==player and skill.socketGroup==source.group and definition==source.gem.gemData.grantedEffectList[1])
   local sourceLevel=assert(definition.levels[effect.level])
   local preparedLevel=assert(effect.grantedEffectLevel)
   assert(preparedLevel~=sourceLevel,"source prepares a copied level row")
   local observed=assert(capture.actors[player]and capture.actors[player][skill],"missing original reservation-call observations")
   local count,enabled=calcs.getActiveSkillCount(skill)
   local candidates={};for index,gem in ipairs(source.group.gemList)do
    if gem.gemData and(gem.gemData.grantedEffect==definition or isValueInArray(gem.gemData.additionalGrantedEffects,definition))then
     candidates[#candidates+1]={index=index,source_ordinal=origins[gem]and origins[gem].source_ordinal,count=gem.count,exact_physical_source=gem==source.gem}
    end
   end
   local modifiers={};for _,name in ipairs({"ReservationMultiplier","SpiritReserved","Reserved","SpiritReservationEfficiency","ReservationEfficiency","ExtraSpirit","MinionFreeSpiritCount"})do
    modifiers[name]=tabulated(skill.skillModList:Tabulate(nil,skill.skillCfg,name))
   end
   local statSet=mode=="CALCS"and effect.statSetCalcs or effect.statSet
   assert(definition.statSets[statSet.index]==statSet.statSet)
   local setLevel=statSet.statSet.levels and statSet.statSet.levels[effect.level]
   if not source.gem.noReservation and not(setLevel and setLevel.spiritReservationFlat~=nil)then
    assert(preparedLevel.spiritReservationFlat==sourceLevel.spiritReservationFlat,"ordinary copied flat reservation differs from its source row")
   end
   local accepted={};for _,e in ipairs(skill.effectList)do if e.grantedEffect.support then
    local level=assert(e.grantedEffect.levels[e.level])
    accepted[#accepted+1]={effect=e.grantedEffect.id,level=e.level,quality=e.quality,source_level_row=plain(level),reservation_multiplier=level.reservationMultiplier}
   end end
   local row={source_ordinal=source.row.source_ordinal,physical_source_exact=true,group_exact=true,effect=definition.id,
    raw_level=source.gem.level,final_level=effect.level,final_quality=effect.quality,final_level_row=plain(preparedLevel),stat_set_index=statSet.index,
    level_provenance={source_level_row=plain(sourceLevel),selected_stat_set_level_row=plain(setLevel),prepared_is_copy=true,
     selected_stat_set_flat_override_present=setLevel~=nil and setLevel.spiritReservationFlat~=nil,
     no_reservation_present=source.gem.noReservation~=nil,no_reservation=source.gem.noReservation,no_reservation_active=not not source.gem.noReservation},
    captured=plain(observed),count=count,count_enabled=enabled,count_candidates=candidates,
    modifiers=modifiers,accepted_supports=accepted,
    inputs={reservation_multiplier=skill.skillModList:More(skill.skillCfg,"ReservationMultiplier"),
     extra_spirit=skill.skillModList:Sum("BASE",skill.skillCfg,"ExtraSpirit"),
     reserved_inc=skill.skillModList:Sum("INC",skill.skillCfg,"SpiritReserved","Reserved"),
     reserved_more=skill.skillModList:More(skill.skillCfg,"SpiritReserved","Reserved"),
     efficiency_inc=skill.skillModList:Sum("INC",skill.skillCfg,"SpiritReservationEfficiency","ReservationEfficiency"),
     efficiency_more=skill.skillModList:More(skill.skillCfg,"SpiritReservationEfficiency","ReservationEfficiency"),
     skill_data_flat_present=skill.skillData.spiritReservationFlat~=nil,skill_data_flat=skill.skillData.spiritReservationFlat,
     final_level_flat_present=effect.grantedEffectLevel.spiritReservationFlat~=nil,final_level_flat=effect.grantedEffectLevel.spiritReservationFlat,
     skill_data_percent_present=skill.skillData.spiritReservationPercent~=nil,skill_data_percent=skill.skillData.spiritReservationPercent,
     final_level_percent_present=effect.grantedEffectLevel.spiritReservationPercent~=nil,final_level_percent=effect.grantedEffectLevel.spiritReservationPercent},
    reservation={has_reservation=skill.skillTypes[SkillType.HasReservation]==true,multiple=skill.skillTypes[SkillType.MultipleReservation]==true,
     becomes_cost=skill.skillTypes[SkillType.ReservationBecomesCost]==true,free_count=skill.skillModList:Sum("BASE",skill.skillCfg,"MinionFreeSpiritCount"),
     skill_data=scalars(skill.skillData),spirit_result=skill.skillData.SpiritReservedBase,spirit_result_present=skill.skillData.SpiritReservedBase~=nil,
     forced_flat_present=skill.skillData.SpiritReservationFlatForced~=nil,forced_flat=skill.skillData.SpiritReservationFlatForced,
     forced_percent_present=skill.skillData.SpiritReservationPercentForced~=nil,forced_percent=skill.skillData.SpiritReservationPercentForced,
     mine_count_present=skill.activeMineCount~=nil,mine_count=skill.activeMineCount,stage_count=skill.activeStageCount,
     mana_as_reservation=skill.skillModList:Flag(skill.skillCfg,"ManaCostGainAsReservation")==true,
     spirit_to_life=skill.skillModList:Sum("BASE",skill.skillCfg,"LifeReservePercentPerSpirit")},
    actor=skill.minion and{type=skill.minion.type,level=skill.minion.level,children={}}}
   if skill.minion then for _,child in ipairs(skill.minion.activeSkillList)do row.actor.children[#row.actor.children+1]={effect=child.activeEffect.grantedEffect.id,summon_parent=child.summonSkill==skill,physical_source_present=child.activeEffect.srcInstance~=nil}end end
   source.row[mode][#source.row[mode]+1]=row
  end end
 end
end
assert(refs.load_skill==build.skillsTab.LoadSkill and refs.output==calcs.buildOutput and not debug.gethook())
return{saved=saved,contexts=contexts,selection={skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,items=build.itemsTab.activeItemSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup},
 source_methods_preserved=true,hook_removed=true,physical_objects_preserved=true,original_functions=djinnOriginals.auth,output_lifecycle=djinnOriginals.output_lifecycle,
 output_revision=build.outputRevision,build_flag=build.buildFlag==true}
