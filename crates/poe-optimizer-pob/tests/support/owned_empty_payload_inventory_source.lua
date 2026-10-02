-- Read-only extension of the maintained complete-loader saved-row observer.
-- The catalog classification is deliberately sufficient, not an exhaustive model of triggers.
nonphysicalSkillPhase = payloadInventoryPhase
local observed = payloadInventoryBaseObserver()
if payloadInventoryPhase == "before" then return observed end
local auth = assert(nonphysicalSkillAuth)
local data = build.data
local names = {}
for name,id in pairs(SkillType) do if type(id)=="number" then names[id]=name end end
local function types(values, list)
 local out={}
 if list then
  for _,id in ipairs(values or {}) do out[#out+1]=assert(names[id],"unknown source SkillType "..tostring(id)) end
 else
  for id,value in pairs(values or {}) do if value then out[#out+1]=assert(names[id],"unknown source SkillType "..tostring(id)) end end
  table.sort(out)
 end
 return out
end
local function contains(values, name)
 for _,value in ipairs(values) do if value==name then return true end end;return false
end
local runtimeEffects={}
local function effectInfo(effect)
 assert(data.skills[effect.id]==effect,"not the loaded catalog effect")
 runtimeEffects[effect.id]={id=effect.id,name=effect.name,has_global_effect=not not effect.hasGlobalEffect,from_item=not not effect.fromItem}
 local row={id=effect.id,support=not not effect.support,
  skill_types=types(effect.skillTypes),add_skill_types=types(effect.addSkillTypes,true),
  require_skill_types=types(effect.requireSkillTypes,true),exclude_skill_types=types(effect.excludeSkillTypes,true),
  is_trigger=not not effect.isTrigger,triggered=not not effect.triggered,
  hidden=not not effect.hidden,hide_from_side_bar=not not effect.hideFromSideBar,exclusions={}}
 for _,name in ipairs({"Meta","Triggers","Triggered","InbuiltTrigger"}) do
  if contains(row.skill_types,name) then row.exclusions[#row.exclusions+1]="skill_type:"..name end
  if contains(row.add_skill_types,name) then row.exclusions[#row.exclusions+1]="add_skill_type:"..name end
 end
 for _,name in ipairs({"is_trigger","triggered"}) do if row[name] then row.exclusions[#row.exclusions+1]=name end end
 row.non_container=#row.exclusions==0
 return row
end
local catalog,byId={},{}
for id,gem in pairs(data.gems) do
 assert(gem.id==id and gem.grantedEffect==data.skills[gem.grantedEffectId])
 local effects,exclusions={},{}
 local seen={}
 for _,effect in ipairs(gem.grantedEffectList) do
  local row=effectInfo(effect);effects[#effects+1]=row;seen[effect]=true
  for _,reason in ipairs(row.exclusions) do exclusions[#exclusions+1]=effect.id..":"..reason end
 end
 assert(#effects>0 and seen[gem.grantedEffect])
 for _,effect in ipairs(gem.additionalGrantedEffects) do assert(seen[effect]) end
 local additional={};for _,effect in ipairs(gem.additionalGrantedEffects) do additional[#additional+1]=effect.id end
 local declared={}
 for field,id in pairs(gem) do
  if field:match("^additionalGrantedEffectId%d+$") or field:match("^additionalStatSet%d+$") or field=="secondaryGrantedEffectId" then
   local target=data.skills[id]
   declared[#declared+1]={field=field,id=id,resolves_as_effect=target~=nil,
    present_in_constructed_effect_list=target~=nil and seen[target]==true}
  end
 end
 table.sort(declared,function(a,b)return a.field<b.field end)
 local row={gem_id=id,game_id=gem.gameId,variant_id=gem.variantId,primary_effect=gem.grantedEffectId,
  primary_support=not not gem.grantedEffect.support,effects=effects,additional_effects=additional,
  declared_references=declared,selector_resolves_same=data.gemsByGameId[gem.gameId][gem.variantId]==gem,
  non_container=#exclusions==0,exclusions=exclusions}
 catalog[#catalog+1]=row;byId[id]=row
end
table.sort(catalog,function(a,b)return a.gem_id<b.gem_id end)
local nonphysical={}
for _,id in ipairs({"EnemyExplode"}) do
 local effect=assert(data.skills[id]);local row=effectInfo(effect)
 row.gem_mapping_by_object_present=data.gemForSkill[effect]~=nil
 row.gem_mapping_by_id_present=data.gemForSkill[id]~=nil
 nonphysical[#nonphysical+1]=row
end
local function loadedOrigin(gem,group)
 for _,entry in ipairs(auth.loaded) do
  if entry.group==group then
   for index,prior in ipairs(entry.gems) do
    if prior==gem then return {observer_group=entry.state.observer_group,skill_set=entry.state.skill_set,gem_index=index} end
   end
  end
 end
end
local groups={}
for _,entry in ipairs(auth.loaded) do
 local row={observer_group=entry.state.observer_group,skill_set=entry.state.skill_set,source=entry.state.source,
  enabled=entry.state.enabled,raw_group=entry.state.raw_group,gems={},non_support_rows=0,
  all_rows_classified_non_container=true}
 for index,gem in ipairs(entry.state.gems) do
  local definition=gem.gem_data_id and byId[gem.gem_data_id]
  local direct
  if not definition and gem.resolved_effect then direct=effectInfo(assert(data.skills[gem.resolved_effect])) end
  local eligible=definition and definition.non_container or direct and direct.non_container or false
  local support=definition and definition.primary_support or direct and direct.support or false
  if not support then row.non_support_rows=row.non_support_rows+1 end
  if not eligible then row.all_rows_classified_non_container=false end
  row.gems[#row.gems+1]={index=index,raw=entry.state.raw_children[index],resolved=gem,
   classification=definition and "gem" or direct and "direct_effect" or "unresolved",
   non_container=eligible,primary_support=support,effect_count=definition and #definition.effects or direct and 1 or 0}
 end
 -- This describes loaded source semantics only. Strict selector/framing admission is an Import obligation.
 row.loaded_shape_without_container=row.all_rows_classified_non_container and row.non_support_rows<=1
 groups[#groups+1]=row
end
local function environment(env)
 local out={}
 for _,skill in ipairs(env.player.activeSkillList) do
  local effect=skill.activeEffect.grantedEffect
  local supports={};for _,support in ipairs(skill.supportList or {}) do
   supports[#supports+1]={effect=support.grantedEffect.id,is_trigger=not not support.grantedEffect.isTrigger,
    origin=loadedOrigin(support.srcInstance,skill.socketGroup)}
  end
  local trigger=skill.triggeredBy
  local statSet=env.mode=="CALCS" and skill.activeEffect.statSetCalcs or skill.activeEffect.statSet
  local flags=statSet and statSet.skillFlags
  out[#out+1]={effect=effect.id,name=effect.name,selected=skill==env.player.mainSkill,origin=loadedOrigin(skill.activeEffect.srcInstance,skill.socketGroup),
   group_source=skill.socketGroup.source,group_enabled=skill.socketGroup.enabled,
   source_gem_id=skill.activeEffect.srcInstance.gemData and skill.activeEffect.srcInstance.gemData.id,
   trigger_effect=trigger and trigger.grantedEffect.id,
   trigger_origin=trigger and loadedOrigin(trigger.srcInstance,skill.socketGroup),
   legacy_skill_flags_present=skill.skillFlags~=nil,flags_present=flags~=nil,
   disabled=flags and not not flags.disable,supports=supports}
 end
 return out
end
observed.payload_catalog=catalog
observed.nonphysical_catalog=nonphysical
observed.payload_groups=groups
observed.payload_main=environment(build.calcsTab.mainEnv)
observed.payload_calcs=environment(build.calcsTab.calcsEnv)
local runtimeObservations={};for _,row in pairs(runtimeEffects) do runtimeObservations[#runtimeObservations+1]=row end
table.sort(runtimeObservations,function(a,b)return a.id<b.id end)
observed.runtime_effect_observations=runtimeObservations
observed.classification_claim="sufficient empty authored two-SkillUse inventory only; no generated-trigger or action coverage"
return observed
