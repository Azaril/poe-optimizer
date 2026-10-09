-- Focused read-only Warrior source evidence. The caller either supplies the
-- authenticated saved-object census or takes a separate uninstrumented snapshot.
local base=warriorMembershipBase
local function scalar(t)
 local r={}
 for k,v in pairs(t or {}) do
  if type(k)=="string" and (type(v)=="string" or type(v)=="boolean" or type(v)=="number") then
   r[k]=type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or v
  end
 end
 return r
end
local function present(v) return {present=v~=nil,value=v} end
local result={selection={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId},groups={},modes={}}
local ids={};for sid in pairs(build.skillsTab.skillSets) do ids[#ids+1]=sid end;table.sort(ids)
for _,sid in ipairs(ids) do
 for gi,g in ipairs(build.skillsTab.skillSets[sid].socketGroupList) do
  for si,s in ipairs(g.gemList) do if s.skillId=="SummonSkeletalWarriorsPlayer" then
   local effect=assert(s.gemData and s.gemData.grantedEffect or s.grantedEffect)
   assert(effect.id=="SummonSkeletalWarriorsPlayer")
   local row={preset=sid,group=gi,source_index=si,group_fields=scalar(g),fields=scalar(s),
    actor_main=present(s.skillMinion),actor_calcs=present(s.skillMinionCalcs),
    child_main=present(s.skillMinionSkill),child_calcs=present(s.skillMinionSkillCalcs),
    from_item=effect.fromItem==true,minion_list=effect.minionList,
    source_item_present=g.sourceItem~=nil,source_item_id=g.sourceItem and g.sourceItem.id}
   result.groups[#result.groups+1]=row
  end end
 end
end
for _,mode in ipairs({"MAIN","CALCS"}) do
 local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 local out=mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
 local observed={player_output_available=out~=nil,player_output=scalar(out),warriors={},grants={}}
 for _,grant in ipairs(env.grantedSkills or {}) do if grant.skillId=="SummonSkeletalWarriorsPlayer" then
  observed.grants[#observed.grants+1]={fields=scalar(grant),item_exact=grant.sourceItem~=nil and env.player.itemList[grant.slotName]==grant.sourceItem}
 end end
 for _,skill in ipairs(env.player.activeSkillList) do if skill.activeEffect.grantedEffect.id=="SummonSkeletalWarriorsPlayer" then
  local group=assert(skill.socketGroup);local source=assert(skill.activeEffect.srcInstance)
  local gi,si
  for i,g in ipairs(build.skillsTab.socketGroupList) do if g==group then assert(not gi);gi=i end end
  for i,s in ipairs(group.gemList) do if s==source then assert(not si);si=i end end
  assert(gi and si)
  local actor=skill.minion;local children={}
  for i,child in ipairs(actor and actor.activeSkillList or {}) do
   local effect=child.activeEffect
   children[#children+1]={index=i,id=effect.grantedEffect.id,is_main=actor.mainSkill==child,
    actor_exact=child.actor==actor,output_available=child.output~=nil,output=scalar(child.output),
    stat_set=effect.statSet and effect.statSet.index,stat_set_calcs=effect.statSetCalcs and effect.statSetCalcs.index}
  end
  observed.warriors[#observed.warriors+1]={group=gi,source_index=si,source=group.source,slot=group.slot,
   source_item_exact=group.sourceItem~=nil and group.sourceItem==env.player.itemList[group.slot],
   source_fields=scalar(source),level=skill.activeEffect.level,quality=skill.activeEffect.quality,
   actor_present=actor~=nil,actor_type=actor and actor.type,children=children}
 end end
 result.modes[mode]=observed
end
assert(not debug.gethook())
if base then
 result.saved={}
 for _,row in ipairs(base.saved_groups) do
  for _,gem in ipairs(row.gems) do if gem.source.attributes.skillId=="SummonSkeletalWarriorsPlayer" then
   result.saved[#result.saved+1]={preset=row.preset,source_ordinal=row.source_ordinal,runtime_present=row.runtime_present,
    gem_ordinal=gem.source_ordinal,loaded_object_exact=gem.loaded_object_exact,retained_in_group=gem.retained_in_group,
    saved_group=row.attributes,saved_gem=gem.source.attributes}
  end end
 end
end
return result
