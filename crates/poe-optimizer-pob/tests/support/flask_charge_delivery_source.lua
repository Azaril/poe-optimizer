-- Read-only observations of the complete, pinned original build calculation.
-- No modifier injection, function replacement, extracted calculation, or cache repair.
-- CalcsTab retains the loaded module; the full host does not export a calcs global.
local calcs = assert(build.calcsTab.calcs)
assert(calcs == package.loaded["Modules.CalcBase"])
local function original(f, path, first)
  local info = debug.getinfo(f, "S")
  local source = info.source:gsub("\\", "/")
  assert(info.what == "Lua" and source:sub(-#path) == path and info.linedefined == first)
  return f
end
local refs = {
  parse = original(common.classes.Item.ParseRaw, "Classes/Item.lua", 468),
  active = original(common.classes.Item.GetActiveModListForSlotNum, "Classes/Item.lua", 2198),
  load = original(build.itemsTab.Load, "Classes/ItemsTab.lua", 1193),
  init = original(calcs.initEnv, "Modules/CalcSetup.lua", 717),
  perform = original(calcs.perform, "Modules/CalcPerform.lua", 1193),
  output = original(calcs.buildOutput, "Modules/Calcs.lua", 469),
  sum = original(common.classes.ModStore.Sum, "Classes/ModStore.lua", 202),
  sum_internal = original(common.classes.ModDB.SumInternal, "Classes/ModDB.lua", 137),
}
local function plain(v, depth)
  if type(v) ~= "table" then
    assert(type(v) ~= "function" and type(v) ~= "userdata")
    return v
  end
  depth = (depth or 0) + 1
  assert(depth < 12)
  local out, count = {}, 0
  for k, value in pairs(v) do
    count = count + 1
    assert(count < 512)
    out[k] = plain(value, depth)
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
local function scalar_snapshot(output)
  local result = {}
  for k, v in pairs(output) do
    if type(v) == "number" or type(v) == "boolean" or type(v) == "string" then result[k] = v end
  end
  return result
end
local name = "FlaskChargesGenerated"
local item = assert(build.itemsTab.items[27])
assert(item.id == 27 and item.baseName == "Fine Belt")
local item_source = assert(item.modSource)
assert(item_source:sub(1, 8) == "Item:27:")
local function records(list, only_item)
  local result = {}
  for _, mod in ipairs(list or {}) do
    if mod.name == name and (not only_item or mod.source == item_source) then
      local tags = {}
      for _, tag in ipairs(mod) do tags[#tags + 1] = plain(tag) end
      result[#result + 1] = {
        name = mod.name, type = mod.type, value = mod.value,
        source = mod.source, flags = mod.flags, keyword_flags = mod.keywordFlags,
        source_slot = mod.sourceSlot, source_slot_num = mod.sourceSlotNum, tags = tags,
      }
    end
  end
  return result
end
local function store_records(store)
  local result, current, seen, depth = {}, store, {}, 0
  while current do
    assert(not seen[current])
    seen[current], depth = true, depth + 1
    assert(depth <= 8)
    for _, row in ipairs(records(current.mods[name])) do
      row.store_depth = depth
      result[#result + 1] = row
    end
    current = current.parent
  end
  return result
end
local saved = {
  items = build.itemsTab.activeItemSetId, spec = build.treeTab.activeSpec,
  skills = build.skillsTab.activeSkillSetId, config = build.configTab.activeConfigSetId,
  group = build.mainSocketGroup,
}
-- The original getter can rebuild lists when bonded state changes. Require
-- its existing no-rebuild branch before observing this already-calculated item.
local base_list, mod_list, slot_lists = item.baseModList, item.modList, item.slotModList
local bonded_state = item.activeBondedState
local expected_bonded_state = item.socketedIdolsUseBondedModifiers and "idol" or nil
assert(bonded_state == expected_bonded_state,
  "observation would rebuild the loaded item's bonded modifier lists")
local function assert_loaded_identity()
  assert(item.baseModList == base_list and item.modList == mod_list and item.slotModList == slot_lists)
  assert(item.activeBondedState == bonded_state)
end
local loaded = records(refs.active(item, 1, false), true)
assert_loaded_identity()
local result = {
  item_id = 27, item_source = item_source, saved = plain(saved),
  loaded_item_records = loaded, modes = {},
}
for _, mode in ipairs({"MAIN", "CALCS"}) do
  local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  local output = assert(mode == "MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)
  assert(env.mode == mode and env.modDB == env.player.modDB)
  assert(env.player.output == output)
  assert(env.buildBreakdown == true and type(env.player.breakdown) == "table")
  assert(env.modDB.Sum == refs.sum and env.modDB.SumInternal == refs.sum_internal)
  local before = scalar_snapshot(output)
  local player_records = store_records(env.modDB)
  local item_records = store_records(env.itemModDB)
  local selected = env.player.itemList.Belt
  assert(not selected or selected == item)
  local item_total = env.itemModDB:Sum("BASE", nil, name)
  local player_total = env.modDB:Sum("BASE", nil, name)
  local from_belt = env.modDB:Sum("BASE", { source = item_source }, name)
  local item_from_belt = env.itemModDB:Sum("BASE", { source = item_source }, name)
  assert(type(item_total) == "number" and type(player_total) == "number")
  assert(type(output.FlaskChargeGen) == "number")
  assert(output.FlaskChargeGen == player_total)
  result.modes[mode] = {
    mode = mode, build_breakdown = env.buildBreakdown,
    player_breakdown_present = type(env.player.breakdown) == "table",
    selected_belt_id = selected and selected.id,
    selected_flask_1 = env.player.itemList["Flask 1"] and env.player.itemList["Flask 1"].id,
    selected_flask_2 = env.player.itemList["Flask 2"] and env.player.itemList["Flask 2"].id,
    item_records = item_records, player_records = player_records,
    item_total_base = item_total, item_belt_base = item_from_belt,
    player_total_base = player_total, player_belt_base = from_belt,
    output_flask_charge_gen = output.FlaskChargeGen,
  }
  assert(equal(scalar_snapshot(output), before), "read-only observation changed output")
  assert(equal(player_records, store_records(env.modDB)))
  assert(equal(item_records, store_records(env.itemModDB)))
end
assert_loaded_identity()
assert(equal(loaded, records(refs.active(item, 1, false), true)))
assert_loaded_identity()
assert(build.itemsTab.items[27] == item)
assert(build.itemsTab.activeItemSetId == saved.items and build.treeTab.activeSpec == saved.spec)
assert(build.skillsTab.activeSkillSetId == saved.skills and build.configTab.activeConfigSetId == saved.config)
assert(build.mainSocketGroup == saved.group)
assert(build.calcsTab.calcs == calcs and package.loaded["Modules.CalcBase"] == calcs)
assert(common.classes.Item.ParseRaw == refs.parse and common.classes.Item.GetActiveModListForSlotNum == refs.active)
assert(build.itemsTab.Load == refs.load and calcs.initEnv == refs.init)
assert(calcs.perform == refs.perform and calcs.buildOutput == refs.output)
assert(common.classes.ModStore.Sum == refs.sum and common.classes.ModDB.SumInternal == refs.sum_internal)
result.original_functions_preserved = true
result.loaded_item_preserved = true
result.saved_selection_preserved = true
result.outputs_preserved = true
return result
