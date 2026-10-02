-- Observation only: every actor is constructed by the complete original load.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local info = debug.getinfo(f, "S")
 local actual = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line, path .. ":" .. line)
 return f
end
local function scalar(value)
 if type(value) == "number" and (value ~= value or value == math.huge or value == -math.huge) then return tostring(value) end
 return value
end
local function scalars(t)
 local out = {}
 for key, value in pairs(t or {}) do
  if type(value) == "number" or type(value) == "string" or type(value) == "boolean" then out[key] = scalar(value) end
 end
 return out
end
local function clone(value, seen)
 if type(value) ~= "table" then return value end
 seen = seen or {}; if seen[value] then return seen[value] end
 local out = {}; seen[value] = out
 for key, entry in pairs(value) do out[key] = clone(entry, seen) end
 return out
end
local function equal(a, b, seen)
 if type(a) ~= type(b) then return false end
 if type(a) ~= "table" then return a == b end
 seen = seen or {}; if seen[a] then return seen[a] == b end; seen[a] = b
 for key, value in pairs(a) do if not equal(value, b[key], seen) then return false end end
 for key in pairs(b) do if a[key] == nil then return false end end
 return true
end
local methodRows = {
 { common.classes.SkillsTab, "LoadSkill", "Classes/SkillsTab.lua", 303 },
 { common.classes.SkillsTab, "ProcessSocketGroup", "Classes/SkillsTab.lua", 1242 },
 { common.classes.CalcsTab, "BuildOutput", "Classes/CalcsTab.lua", 486 },
 { calcs, "initEnv", "Modules/CalcSetup.lua", 717 },
 { calcs, "createActiveSkill", "Modules/CalcActiveSkill.lua", 144 },
 { calcs, "buildActiveSkillModList", "Modules/CalcActiveSkill.lua", 426 },
 { calcs, "createMinionSkills", "Modules/CalcActiveSkill.lua", 1116 },
 { calcs, "perform", "Modules/CalcPerform.lua", 1193 },
 { calcs, "offence", "Modules/CalcOffence.lua", 527 },
}
if intrinsicAttackPhase == "before" then
 local methods = {}; for _, row in ipairs(methodRows) do methods[row[2]] = original(row[1][row[2]], row[3], row[4]) end
 local previousActors = {}
 for _, env in ipairs({build.calcsTab.mainEnv, build.calcsTab.calcsEnv}) do
  if env then for _, skill in ipairs(env.player.activeSkillList) do if skill.minion then previousActors[skill.minion] = true end end end
 end
 local oldHook, oldMask, oldCount = debug.gethook(); assert(oldHook == nil, "unexpected existing debug observer")
 local captures, observedReturns = {}, 0
 local function hook(event)
  if event ~= "return" or debug.getinfo(2, "f").func ~= methods.offence then return end
  local vars = {}
  for index = 1, 128 do local name, value = debug.getlocal(2, index); if not name then break end; vars[name] = value end
  local actor, active = vars.actor, vars.activeSkill
  if not actor or not actor.minionData or not active then return end
  local passes = {}
  for _, pass in ipairs(vars.passList or {}) do
   passes[#passes + 1] = {label=pass.label, source=scalars(pass.source), copied_from_actor=pass.source ~= actor.weaponData1, output=scalars(pass.output)}
  end
  captures[active] = {passes=passes, actor=actor, mode=vars.env.mode}
  observedReturns = observedReturns + 1
 end
 -- Existing traces must not bypass a newly installed observation hook. This
 -- flush changes cache warmth only, and both requested JIT modes stay intact.
 local enabled = jit.status(); jit.flush(); assert(jit.status() == enabled)
 debug.sethook(hook, "r")
 intrinsicAttackAuth = {methods=methods, previous_actors=previousActors, captures=captures, jit_enabled=enabled}
 return function()
  assert(debug.gethook() == hook)
  debug.sethook(oldHook, oldMask, oldCount)
  assert(jit.status() == enabled)
  for _, row in ipairs(methodRows) do assert(row[1][row[2]] == methods[row[2]], "business method replaced") end
  intrinsicAttackAuth.finished = true
  intrinsicAttackAuth.observed_returns = observedReturns
 end
end
local auth = assert(intrinsicAttackAuth); assert(auth.finished)
local mainEnv, calcsEnv = build.calcsTab.mainEnv, build.calcsTab.calcsEnv
local mainOutput, calcsOutput = build.calcsTab.mainOutput, build.calcsTab.calcsOutput
local oldMain, oldCalcs = clone(mainOutput), clone(calcsOutput)
local savedItems, savedSets, savedGroups = clone(build.itemsTab.items), clone(build.itemsTab.itemSets), clone(build.skillsTab.skillSets)
local function selected()
 return {items=build.itemsTab.activeItemSetId, skills=build.skillsTab.activeSkillSetId, spec=build.treeTab.activeSpec,
  config=build.configTab.activeConfigSetId, main_group=build.mainSocketGroup}
end
local selection = selected()
local capturedActors = {}
local function captureEnvironment(env)
 local rows, identities = {}, {}
 for ordinal, summoner in ipairs(env.player.activeSkillList) do
  local actor = summoner.minion
  if actor then
   assert(not auth.previous_actors[actor], "stale actor from previous load")
   assert(not identities[actor], "distinct source summoners share one actor")
   identities[actor] = true
   local effect, profile = summoner.activeEffect, actor.minionData
   assert(profile == env.data.minions[actor.type])
   local physical = effect.srcInstance
   local sourceGroup, sourceGem
   for groupIndex, group in ipairs(build.skillsTab.socketGroupList) do
    for gemIndex, gem in ipairs(group.gemList) do if gem == physical then assert(not sourceGroup); sourceGroup=groupIndex; sourceGem=gemIndex end end
   end
   local monsterDamage = effect.grantedEffect.minionList and (effect.grantedEffect.name:match("^Spectre") or effect.grantedEffect.name:match("^Companion")) ~= nil
   local hostileCurve = not not (monsterDamage or profile.hostile)
   local curve = hostileCurve and env.data.monsterDamageTable or env.data.monsterAllyDamageTable
   local children = {}
   for _, child in ipairs(actor.activeSkillList or {}) do
    local observed = auth.captures[child]
    if observed then assert(observed.actor == actor and observed.mode == env.mode) end
    children[#children + 1] = {effect_id=child.activeEffect.grantedEffect.id, effect_name=child.activeEffect.grantedEffect.name, effect_level=child.activeEffect.level,
     actor_level=child.activeEffect.actorLevel, selected=actor.mainSkill == child, flags=scalars(child.skillFlags),
     output=scalars(child.output), consumer=observed and {passes=observed.passes,mode=observed.mode} or nil}
   end
   local policy = {hostile=not not profile.hostile, monster_damage=not not monsterDamage,
    minion_has_item_set=not not effect.grantedEffect.minionHasItemSet, uses_weapon1=not not (actor.uses and actor.uses["Weapon 1"]),
    uses_weapon2=not not (actor.uses and actor.uses["Weapon 2"]), minion_use_bow_and_quiver=not not summoner.skillData.minionUseBowAndQuiver,
    iron_mass=not not env.theIronMass, weapon1_is_player_weapon=actor.weaponData1 == env.player.weaponData1,
    weapon1_from_item=false, alternate_level=not not (summoner.skillData.minionLevelIsEnemyLevel or summoner.skillData.minionLevelIsTriggeredSkillLevel or summoner.skillData.minionLevelIsPlayerLevel or summoner.skillData.minionLevel)}
   for _, item in pairs(build.itemsTab.items) do for _, weapon in pairs(item.weaponData or {}) do if actor.weaponData1 == weapon then policy.weapon1_from_item=true end end end
   local row = {ordinal=ordinal, source_group=sourceGroup, source_gem=sourceGem,
    summon_effect_id=effect.grantedEffect.id, physical_level=physical and physical.level, physical_quality=physical and physical.quality,
    effective_level=effect.level, actor_profile=actor.type, actor_level=actor.level, actor_is_main=env.player.mainSkill == summoner,
    table_actor_level=env.data.minionLevelTable[effect.level],
    profile={attack_time=profile.attackTime, damage_scale=profile.damage, damage_spread=profile.damageSpread,
     crit_chance=profile.critChance, attack_range=profile.attackRange, weapon_type=profile.weaponType1,
     ignore_attack_speed=not not profile.baseDamageIgnoresAttackSpeed, ignore_attack_speed_present=profile.baseDamageIgnoresAttackSpeed ~= nil},
    curve={kind=hostileCurve and "hostile" or "allied", level=actor.level, raw_value=curve[actor.level]},
    weapon1=scalars(actor.weaponData1), weapon2=scalars(actor.weaponData2), hidden_damage_fixup=actor.hiddenDamageFixup,
    policy=policy, children=children, fresh_actor=true}
   capturedActors[#capturedActors + 1] = {actor=actor, weapon=actor.weaponData1, level=actor.level, state=clone(actor.weaponData1)}
   rows[#rows + 1] = row
  end
 end
 return {actors=rows, main_effect=env.player.mainSkill.activeEffect.grantedEffect.id, output=scalars(env.player.output)}
end
assert(auth.observed_returns > 0, "original minion offence was not observed")
local result = {selected=selection, main=captureEnvironment(mainEnv), calcs=captureEnvironment(calcsEnv),
 main_output=scalars(mainOutput), calcs_output=scalars(calcsOutput), original_minion_offence_observed=true}
for _, entry in ipairs(capturedActors) do assert(entry.actor.level == entry.level and entry.actor.weaponData1 == entry.weapon and equal(entry.actor.weaponData1, entry.state)) end
assert(equal(build.itemsTab.items, savedItems) and equal(build.itemsTab.itemSets, savedSets) and equal(build.skillsTab.skillSets, savedGroups))
assert(equal(mainOutput, oldMain) and equal(calcsOutput, oldCalcs) and equal(selected(), selection))
assert(build.calcsTab.mainEnv == mainEnv and build.calcsTab.calcsEnv == calcsEnv and build.calcsTab.mainOutput == mainOutput and build.calcsTab.calcsOutput == calcsOutput)
for _, row in ipairs(methodRows) do assert(row[1][row[2]] == auth.methods[row[2]]) end
result.original_functions_preserved=true; result.loaded_state_preserved=true; result.cached_outputs_preserved=true
result.fresh_actor_construction=true; result.source_actor_level_mutated=false; result.business_method_wrappers=false
return result
