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
local function same(a, b)
 if type(a) ~= type(b) then return false end
 if type(a) ~= "table" then return a == b end
 for k, v in pairs(a) do if not same(v, b[k]) then return false end end
 for k in pairs(b) do if a[k] == nil then return false end end
 return true
end
-- Observe only ordinary identities and scalar inputs. Never invoke a selection,
-- rebuild, source parser or UI callback to manufacture the suspected outcome.
local function loadout_snapshot()
 local work = 0
 local function charge() work = work + 1; assert(work <= 1000000) end
 local function fields(value, keys)
  local out = {}
  for _, key in ipairs(keys) do
   charge(); local v = value and value[key]
   if v ~= nil then
    assert(type(v) == "number" or type(v) == "string" or type(v) == "boolean", key)
    if type(v) == "number" then assert(v == v and v ~= math.huge and v ~= -math.huge) end
    out[key] = v
   end
  end
  return out
 end
 local items, tree, skills, config = build.itemsTab, build.treeTab, build.skillsTab, build.configTab
 local function matches(object, collection)
  local out = {}
  if object then
   for key, candidate in pairs(collection or {}) do
    charge(); assert(type(key) == "string" or type(key) == "number")
    if object == candidate then out[tostring(key)] = true end
   end
  end
  return out
 end
 local function provider(value)
  return {item_present=value.sourceItem~=nil, node_present=value.sourceNode~=nil,
   item_ids=matches(value.sourceItem, items and items.items),
   allocated_node_ids=matches(value.sourceNode, build.spec and build.spec.allocNodes),
   node_id=value.sourceNode and value.sourceNode.id}
 end
 local selected = {items=items and items.activeItemSetId, skills=skills and skills.activeSkillSetId,
  config=config and config.activeConfigSetId, spec=tree and tree.activeSpec, main_group=build.mainSocketGroup}
 local titles = {items=items and items.activeItemSet and items.activeItemSet.title,
  skills=skills and skills.skillSets[skills.activeSkillSetId] and skills.skillSets[skills.activeSkillSetId].title,
  config=config and config.configSets[config.activeConfigSetId] and config.configSets[config.activeConfigSetId].title,
  spec=build.spec and build.spec.title}
 local slots = {}
 for key, slot in pairs(items and items.slots or {}) do
  charge(); slots[tostring(key)] = fields(slot, {"selItemId", "active", "nodeId"})
 end
 local groups = {}
 for index, group in ipairs(skills and skills.socketGroupList or {}) do
  charge(); local gems = {}
  for i, gem in ipairs(group.gemList) do
   charge(); gems[i] = fields(gem, {"gemId", "variantId", "skillId", "nameSpec", "level", "quality", "enabled", "fromItem", "enableGlobal1", "enableGlobal2"})
  end
  groups[index] = {index=index, saved=fields(group, {"source", "slot", "enabled", "label", "mainActiveSkill", "mainActiveSkillCalcs"}),
   provider=provider(group), gems=gems}
 end
 local modes = {}
 for _, mode in ipairs({"MAIN", "CALCS"}) do
  local env = build.calcsTab and (mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  if env then
   local grants = {}
   for i, grant in ipairs(env.grantedSkills or {}) do
    charge(); grants[i] = {index=i, saved=fields(grant, {"source", "slotName", "skillId", "nameSpec", "level", "noSupports", "noReservation"}), provider=provider(grant)}
   end
   local main = env.player and env.player.mainSkill
   local main_groups = matches(main and main.socketGroup, skills and skills.socketGroupList)
   local effect = main and main.activeEffect
   modes[mode] = {main_group=env.mainSocketGroup, grants=grants, main_group_identity=main_groups,
    main_effect=effect and effect.grantedEffect.id, main_input=fields(effect and effect.srcInstance, {"skillId", "level", "quality", "gemId"})}
  end
 end
 local dropdown = build.controls and build.controls.buildLoadouts
 return {selected=selected, titles=titles, equipment_slots=slots, groups=groups, modes=modes,
  build_flag=build.buildFlag, loadout_index=build.activeLoadout,
  dropdown=dropdown and {selected=dropdown.selIndex, value=dropdown.list[dropdown.selIndex]}, observation_work=work}
end
local function loadout_objects()
 return {items=build.itemsTab, tree=build.treeTab, spec=build.spec, skills=build.skillsTab, config=build.configTab,
  groups=build.skillsTab and build.skillsTab.socketGroupList,
  main=build.calcsTab and build.calcsTab.mainEnv, calcs=build.calcsTab and build.calcsTab.calcsEnv}
end
local function same_objects(refs)
 local current = loadout_objects()
 for key, value in pairs(refs) do assert(current[key] == value, "changed loadout object: "..key) end
 for key, value in pairs(current) do assert(refs[key] == value, "new loadout object: "..key) end
end
if outsideConfigLoadouts then
 local extra = {
  {build, "SyncLoadouts", "Modules/Build.lua", 637},
  {build, "GetLoadoutByName", "Modules/Build.lua", 899},
  {build, "SetActiveLoadout", "Modules/Build.lua", 956},
  {common.classes.DropDownControl, "SetSel", "Classes/DropDownControl.lua", 166},
  {common.classes.SkillsTab, "Load", "Classes/SkillsTab.lua", 407},
  {common.classes.SkillsTab, "SetActiveSkillSet", "Classes/SkillsTab.lua", 1548},
  {common.classes.ItemsTab, "Load", "Classes/ItemsTab.lua", 1193},
  {common.classes.ItemsTab, "SetActiveItemSet", "Classes/ItemsTab.lua", 1628},
  {common.classes.TreeTab, "Load", "Classes/TreeTab.lua", 492},
  {common.classes.TreeTab, "SetActiveSpec", "Classes/TreeTab.lua", 540},
  {common.classes.ConfigTab, "Load", "Classes/ConfigTab.lua", 878},
  {common.classes.ConfigTab, "SetActiveConfigSet", "Classes/ConfigTab.lua", 1407},
 }
 for _, entry in ipairs(extra) do methods[#methods+1] = entry end
end
if outsideConfigPhase == "before" then
 local refs, sources = {}, {}
 for i, m in ipairs(methods) do refs[i], sources[i] = original(m[1][m[2]], m[3], m[4]) end
 outsideConfigAuth = {refs=refs, sources=sources}
 if outsideConfigLoadouts then outsideConfigAuth.before_load = loadout_snapshot() end
 return function()
  assert(jit.status() == outsideConfigJit)
  for i, m in ipairs(methods) do assert(m[1][m[2]] == refs[i], "changed source method") end
  if outsideConfigLoadouts then
   outsideConfigAuth.load_objects = loadout_objects()
   outsideConfigAuth.load_return = loadout_snapshot()
   same_objects(outsideConfigAuth.load_objects)
  end
  outsideConfigAuth.finished = true
 end
end
local auth = assert(outsideConfigAuth)
assert(outsideConfigPhase == "observe" and auth.finished)
for i, m in ipairs(methods) do assert(m[1][m[2]] == auth.refs[i]) end
if outsideConfigLoadouts then
 same_objects(auth.load_objects)
 local observed = loadout_snapshot()
 assert(same(auth.load_return, observed), "state changed between original loader return and observation")
 assert(same(observed, loadout_snapshot()), "read-only loadout observation changed source state")
 same_objects(auth.load_objects)
 return {before_load=auth.before_load, load_return=auth.load_return, saved=observed, methods=auth.sources,
  methods_preserved=true, objects_preserved=true, native_effect_coverage=false, source_inventory_complete=false,
  selection_correspondence_proven=false, observation_scope="actual original loader return; no rebuild or selection invocation"}
end
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
