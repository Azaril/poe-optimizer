-- Read-only observation after the original complete load/evaluator. This does
-- not invoke LoadSkill, replace calculations, choose a main skill, or fill any
-- missing per-action output. Saved objects remain exact across rebuilds.
local calcs = require("Modules.CalcBase")
assert(djinnOriginals and djinnOriginals.preserved_after_load)
local refs = djinnOriginals.refs
local function original(f, path, line)
 local info = debug.getinfo(f, "S")
 local name = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and name:sub(-#path) == path and info.linedefined == line)
 return f
end
local count = original(calcs.getActiveSkillCount, "Modules/CalcDefence.lua", 149)
local full = original(calcs.calcFullDPS, "Modules/Calcs.lua", 251)
assert(refs.load_skills == common.classes.SkillsTab.Load and refs.load_skill == common.classes.SkillsTab.LoadSkill)
assert(refs.process_group == common.classes.SkillsTab.ProcessSocketGroup)
assert(refs.init == calcs.initEnv and refs.create == calcs.createActiveSkill and refs.mods == calcs.buildActiveSkillModList)
assert(refs.perform == calcs.perform and refs.output == calcs.buildOutput)
local function scalars(t)
 local r = {}
 for k, v in pairs(t or {}) do
  if type(k) == "string" and (type(v) == "string" or type(v) == "number" or type(v) == "boolean") then r[k] = v end
 end
 return r
end
local function same(a, b)
 for k, v in pairs(a) do
  if type(v) == "table" then if type(b[k]) ~= "table" or not same(v, b[k]) then return false end
  elseif b[k] ~= v then return false end
 end
 for k in pairs(b) do if a[k] == nil then return false end end
 return true
end
local function fields(g)
 return {gem_id=g.gemId,skill_id=g.skillId,level=g.level,quality=g.quality,corrupted=g.corrupted,corrupt_level=g.corruptLevel,
  enabled=g.enabled,count=g.count,global_1=g.enableGlobal1,global_2=g.enableGlobal2,
  stat_sets=scalars(g.statSet),stat_sets_calcs=scalars(g.statSetCalcs)}
end
local function groupFields(g)
 return {enabled=g.enabled,slot_enabled=g.slotEnabled,group_count=g.groupCount,include_in_full_dps=g.includeInFullDPS,
  main=g.mainActiveSkill,calcs=g.mainActiveSkillCalcs,source=g.source,slot=g.slot}
end
local function dpsRows(output)
 local r = {}
 for _, row in ipairs(output.SkillDPS or {}) do
  assert(#r < 256, "unexpected FullDPS inventory size")
  r[#r+1] = scalars(row)
 end
 return r
end
local selection = {skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
 spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local environments = {MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local outputs = {MAIN=assert(build.calcsTab.mainOutput),CALCS=assert(build.calcsTab.calcsOutput)}
local outputSnapshots = {MAIN=scalars(outputs.MAIN),CALCS=scalars(outputs.CALCS)}
local dpsSnapshots = {MAIN=dpsRows(outputs.MAIN),CALCS=dpsRows(outputs.CALCS)}
local doc, err = common.xml.ParseXML(iceOccurrenceXml); assert(doc and not err)
local ordinals, nextOrdinal = {}, 0
local function enumerate(n)
 if type(n) ~= "table" or not n.elem then return end
 assert(nextOrdinal < 100000, "source enumeration exceeds witness bound")
 ordinals[n] = nextOrdinal; nextOrdinal = nextOrdinal + 1
 for _, child in ipairs(n) do enumerate(child) end
end
enumerate(doc[1])
local skills
for _, n in ipairs(doc[1]) do if type(n) == "table" and n.elem == "Skills" then assert(not skills); skills = n end end
assert(skills)
local origins, rows, saved = {}, {}, {}
local earlier = iceSavedPhysicalReferences
iceSavedPhysicalReferences = iceSavedPhysicalReferences or {}
for _, set in ipairs(skills) do if type(set) == "table" and set.elem == "SkillSet" then
 local sid = assert(tonumber(set.attrib.id))
 local runtimeSet = assert(build.skillsTab.skillSets[sid])
 local gi = 0
 for _, node in ipairs(set) do if type(node) == "table" and node.elem == "Skill" then
  gi = gi + 1
  local group, gemIndex = assert(runtimeSet.socketGroupList[gi]), 0
  for _, child in ipairs(node) do if type(child) == "table" and child.elem == "Gem" then
   gemIndex = gemIndex + 1
   local gem = assert(group.gemList[gemIndex])
   if child.attrib.gemId == "Metadata/Items/Gems/SkillGemIceNova" then
    assert(#rows < 256 and not origins[gem], "saved sources must be distinct and bounded")
    assert(gem.gemData and gem.gemData.id == child.attrib.gemId and gem.skillId == child.attrib.skillId)
    local d = gem.gemData
    assert(#d.grantedEffectList == 1 and #d.additionalGrantedEffects == 0)
    assert(d.grantedEffectList[1] == d.grantedEffect and d.grantedEffect.id == "IceNovaPlayer")
    assert(not d.grantedEffect.support and not d.grantedEffect.fromTree and not d.vaalGem)
    if earlier then
     local prior = assert(earlier[ordinals[child]])
     assert(prior.gem == gem and prior.group == group and prior.set == runtimeSet, "rebuild replaced a saved object")
    else iceSavedPhysicalReferences[ordinals[child]] = {gem=gem,group=group,set=runtimeSet} end
    local children = {}
    for _, c in ipairs(child) do if type(c) == "table" and c.elem then
     children[#children+1] = {source_ordinal=ordinals[c],name=c.elem,attributes=scalars(c.attrib)}
    end end
    local row = {source_ordinal=ordinals[child],group_source_ordinal=ordinals[node],preset_source_ordinal=ordinals[set],
     preset=sid,group=gi,index=gemIndex,selected=sid == selection.skills,attributes=scalars(child.attrib),
     children=children,group_attributes=scalars(node.attrib),loaded=fields(gem),group_state=groupFields(group),
     global_effect_flag_present=d.grantedEffect.hasGlobalEffect ~= nil,global_effect_flag=d.grantedEffect.hasGlobalEffect == true,
     MAIN={},CALCS={}}
    origins[gem] = row; rows[#rows+1] = row
    saved[#saved+1] = {gem=gem,group=group,set=runtimeSet,row=row}
   end
  end end
 end end
end end
local selectors = {}
for mode, env in pairs(environments) do
 local main = assert(env.player.mainSkill)
 selectors[mode] = {group=env.mainSocketGroup,effect=main.activeEffect.grantedEffect.id,
  stat_set_index=(mode == "CALCS" and main.activeEffect.statSetCalcs or main.activeEffect.statSet).index,
  minion_effect=env.minion and env.minion.mainSkill and env.minion.mainSkill.activeEffect.grantedEffect.id}
 for _, a in ipairs(env.player.activeSkillList) do
  local effect = a.activeEffect
  if effect.grantedEffect.id == "IceNovaPlayer" then
   local row = assert(origins[effect.srcInstance], "Ice action has no exact saved physical source")
   assert(row.selected and a.socketGroup == build.skillsTab.skillSets[row.preset].socketGroupList[row.group])
   assert(a.actor == env.player and not a.minion and effect.grantedEffect == effect.srcInstance.gemData.grantedEffectList[1])
   local chosen = assert(mode == "CALCS" and effect.statSetCalcs or effect.statSet)
   assert(effect.grantedEffect.statSets[chosen.index] == chosen.statSet, "selection is not a constructed source table")
   local actualCount, enabled = count(a)
   local matches = {}
   -- The original helper matches the effect, not srcInstance. Preserve its
   -- actual answer and every candidate source without inventing ownership.
   for i, gem in ipairs(a.socketGroup.gemList) do
    if gem.gemData and gem.gemData.grantedEffect == effect.grantedEffect then
     local candidate = assert(origins[gem])
     matches[#matches+1] = {source_ordinal=candidate.source_ordinal,index=i,count=gem.count,exact_action_source=gem == effect.srcInstance}
    end
   end
   local flags = a.skillFlags or chosen.skillFlags or {}
   local action = {source_ordinal=row.source_ordinal,exact_physical_object=true,exact_group=true,actor_is_player=true,
    effect=effect.grantedEffect.id,effect_index=1,stat_set_index=chosen.index,stat_set_label=chosen.statSet.label,
    stat_description_scope=chosen.statSet.statDescriptionScope,level=effect.level,quality=effect.quality,
    count=actualCount,count_enabled=enabled,count_candidates=matches,count_group_override=a.socketGroup.groupCount,
    is_main_skill=main == a,disabled=flags.disable == true,has_global_effect=effect.grantedEffect.hasGlobalEffect == true,
    has_parts=effect.grantedEffect.parts ~= nil,output_available=a.output ~= nil,output=scalars(a.output),
    skill_data_available=a.skillData ~= nil,skill_data=scalars(a.skillData),
    umbral_environment_flag=env.modDB and env.modDB:Flag(nil,"UmbralWell") == true,
    umbral_buff_value=a.skillModList and a.skillModList:Sum("BASE",main.skillCfg,"UmbralWellBuffValue")}
   row[mode][#row[mode]+1] = action
  end
 end
end
for _, s in ipairs(saved) do
 assert(s.set.socketGroupList[s.row.group] == s.group and s.group.gemList[s.row.index] == s.gem)
 assert(same(fields(s.gem),s.row.loaded) and same(groupFields(s.group),s.row.group_state))
end
assert(build.skillsTab.activeSkillSetId == selection.skills and build.itemsTab.activeItemSetId == selection.items)
assert(build.treeTab.activeSpec == selection.spec and build.configTab.activeConfigSetId == selection.config and build.mainSocketGroup == selection.group)
for mode, env in pairs(environments) do
 assert((mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv) == env)
 assert((mode == "MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput) == outputs[mode])
 assert(same(scalars(outputs[mode]),outputSnapshots[mode]) and same(dpsRows(outputs[mode]),dpsSnapshots[mode]))
end
assert(refs.load_skill == build.skillsTab.LoadSkill and refs.process_group == build.skillsTab.ProcessSocketGroup)
assert(refs.output == calcs.buildOutput and refs.perform == calcs.perform and count == calcs.getActiveSkillCount and full == calcs.calcFullDPS)
return {saved=rows,selection=selection,selectors=selectors,outputs=outputSnapshots,full_dps=dpsSnapshots,
 output_revision=build.outputRevision,build_flag=build.buildFlag == true,original_functions=djinnOriginals.auth,
 output_lifecycle=djinnOriginals.output_lifecycle,source_methods_preserved=true,exact_physical_objects=true,
 physical_objects_preserved_across_stages=true,saved_inputs_preserved=true,selected_state_preserved=true,reported_outputs_preserved=true}
