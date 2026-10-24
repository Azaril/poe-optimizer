-- Fresh normal-load observations only. No source method or input is replaced.
assert(jit.status() == outsideConfigJit)
local calcs = require("Modules.CalcBase")
local function original(f, path, first)
 assert(type(f) == "function")
 local info = debug.getinfo(f, "S")
 assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-#path) == path and info.linedefined == first, path)
 return f, {path=path, first=info.linedefined, last=info.lastlinedefined}
end
local methods = {
 {build, "Init", "Modules/Build.lua", 34},
 {build, "Load", "Modules/Build.lua", 1138},
 {common.classes.CalcsTab, "Load", "Classes/CalcsTab.lua", 199},
 {common.classes.CalcsTab, "BuildOutput", "Classes/CalcsTab.lua", 486},
 {calcs, "initEnv", "Modules/CalcSetup.lua", 717},
}
if outsideConfigPhase == "before" then
 local refs, sources = {}, {}
 for i, m in ipairs(methods) do refs[i], sources[i] = original(m[1][m[2]], m[3], m[4]) end
 outsideConfigAuth = {refs=refs, sources=sources}
 return function()
  assert(jit.status() == outsideConfigJit)
  for i, m in ipairs(methods) do assert(m[1][m[2]] == refs[i], "changed source method") end
  outsideConfigAuth.finished = true
 end
end
local auth = assert(outsideConfigAuth)
assert(outsideConfigPhase == "observe" and auth.finished)
for i, m in ipairs(methods) do assert(m[1][m[2]] == auth.refs[i]) end
local work = 0
local function plain(value, depth)
 work = work + 1; assert(work <= 200000)
 local kind = type(value)
 if kind == "number" then
  if value ~= value or value == math.huge or value == -math.huge then return {nonfinite=tostring(value)} end
  return value
 end
 if kind ~= "table" then assert(kind == "nil" or kind == "string" or kind == "boolean", kind); return value end
 depth = (depth or 0) + 1; assert(depth <= 20)
 local out, size = {}, 0
 for k, v in pairs(value) do
  size = size + 1; assert(size <= 30000)
  assert(type(k) == "string" or type(k) == "number")
  local key = tostring(k); assert(out[key] == nil)
  out[key] = plain(v, depth)
 end
 return out
end
local function scalars(value)
 local out = {}
 for k, v in pairs(value or {}) do
  if type(v) == "number" or type(v) == "string" or type(v) == "boolean" then out[tostring(k)] = plain(v) end
 end
 return out
end
local function equal(a, b)
 if type(a) ~= type(b) then return false end
 if type(a) ~= "table" then return a == b end
 for k, v in pairs(a) do if not equal(v, b[k]) then return false end end
 for k in pairs(b) do if a[k] == nil then return false end end
 return true
end
local function selected(active, mode)
 if not active then return {available=false} end
 local effect = assert(active.activeEffect)
 local set = mode == "CALCS" and effect.statSetCalcs or effect.statSet
 local groups, origins = {}, {}
 for g, group in ipairs(build.skillsTab.socketGroupList) do
  if group == active.socketGroup then groups[#groups+1] = g end
  for i, gem in ipairs(group.gemList) do
   if gem == effect.srcInstance then origins[#origins+1] = {group=g, gem=i, gem_id=gem.gemId, enabled=gem.enabled, level=gem.level, quality=gem.quality} end
  end
 end
 return {available=true, effect=effect.grantedEffect.id, actor_type=active.actor and active.actor.type,
  stat_set=set and set.index, socket_groups=groups, source_occurrences=origins}
end
local function snapshot()
 local cfg = assert(build.configTab)
 local modes = {}
 for _, mode in ipairs({"MAIN", "CALCS"}) do
  local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  local output = assert(mode == "MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)
  assert(env.configInput == cfg.input and env.configPlaceholder == cfg.placeholder)
  modes[mode] = {main_group=env.mainSocketGroup, skill=selected(env.player.mainSkill, mode),
   minion_skill=selected(env.minion and env.minion.mainSkill, mode),
   buffs=env.mode_buffs, combat=env.mode_combat, effective=env.mode_effective,
   config_alias=true, output=scalars(output), player=scalars(env.player.output),
   enemy=scalars(env.enemy.output), minion_available=env.minion~=nil,
   minion=env.minion and scalars(env.minion.output),
   player_conditions=plain(env.modDB.conditions), enemy_conditions=plain(env.enemyDB.conditions)}
 end
 local sets = {}
 for id, set in pairs(cfg.configSets) do sets[tostring(id)] = {input=plain(set.input), placeholder=plain(set.placeholder), custom_blocks=plain(set.customModsList)} end
 local sections = {}
 for _, section in ipairs(build.calcsTab.sectionList) do
  local children = {}; for _, sub in ipairs(section.subSection) do children[#children+1] = {id=sub.id, collapsed=sub.collapsed} end
  sections[#sections+1] = {id=section.id, children=children}
 end
 return {selected={config=cfg.activeConfigSetId, skills=build.skillsTab.activeSkillSetId, items=build.itemsTab.activeItemSetId, spec=build.treeTab.activeSpec, main_group=build.mainSocketGroup},
  calcs_input=plain(build.calcsTab.input), timeless=plain(build.timelessData), sections=sections,
  config={input=plain(cfg.input), placeholder=plain(cfg.placeholder), sets=sets}, modes=modes}
end
local refs = {config=build.configTab, config_input=build.configTab.input, config_placeholder=build.configTab.placeholder,
 calcs_input=build.calcsTab.input, timeless=build.timelessData, main_env=build.calcsTab.mainEnv,
 calcs_env=build.calcsTab.calcsEnv, main_output=build.calcsTab.mainOutput, calcs_output=build.calcsTab.calcsOutput}
local saved = snapshot()
assert(equal(saved, snapshot()), "read-only snapshot changed selected state or outputs")
assert(build.configTab==refs.config and build.configTab.input==refs.config_input and build.configTab.placeholder==refs.config_placeholder)
assert(build.calcsTab.input==refs.calcs_input and build.timelessData==refs.timeless)
assert(build.calcsTab.mainEnv==refs.main_env and build.calcsTab.calcsEnv==refs.calcs_env)
assert(build.calcsTab.mainOutput==refs.main_output and build.calcsTab.calcsOutput==refs.calcs_output)
assert(jit.status() == outsideConfigJit)
for i, m in ipairs(methods) do assert(m[1][m[2]] == auth.refs[i]) end
return {saved=saved, methods=auth.sources, methods_preserved=true, objects_preserved=true,
 output_scope="all primitive output fields on both environments and their Player/Enemy/selected Minion",
 native_effect_coverage=false, source_inventory_complete=false, observation_work=work}
