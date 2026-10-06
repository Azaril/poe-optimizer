-- Bounded observation of original Player Life initialization and EvalMod.
-- Source records are observed; this file contains no copy of the Life formula.
local M = {}
local originals, where, captured, hooked, finished
local function original(f, path, first)
    assert(type(f) == "function", "missing original " .. path)
    local info = debug.getinfo(f, "S")
    assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-#path) == path)
    assert(info.linedefined == first, "unexpected original line " .. path .. ":" .. info.linedefined)
    return {path = path, first = info.linedefined, last = info.lastlinedefined}
end
local function plain(value, depth)
    if type(value) ~= "table" then
        assert(type(value) == "nil" or type(value) == "string" or type(value) == "number" or type(value) == "boolean")
        if type(value) == "number" and (value ~= value or value == math.huge or value == -math.huge) then return tostring(value) end
        return value
    end
    depth = (depth or 0) + 1
    assert(depth < 32)
    local out = {}
    for k, v in pairs(value) do out[k] = plain(v, depth) end
    return out
end
local function equal(a, b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k, v in pairs(a) do if not equal(v, b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end
    return true
end
local function scalar(t)
    local out = {}
    for k, v in pairs(t) do
        if type(v) == "number" or type(v) == "string" or type(v) == "boolean" then out[k] = plain(v) end
    end
    return out
end
local function record(mod)
    local tags = {}
    for i, tag in ipairs(mod) do tags[i] = plain(tag) end
    return {name = mod.name, type = mod.type, value = plain(mod.value), source = mod.source,
        flags = mod.flags, keyword_flags = mod.keywordFlags, tags = tags}
end
local function methods()
    local calcs = require("Modules.CalcBase")
    return {
        {"initializer", calcs, "initEnv", "Modules/CalcSetup.lua", 717},
        {"eval_mod", common.classes.ModStore, "EvalMod", "Classes/ModStore.lua", 490},
        {"multiplier", common.classes.ModStore, "GetMultiplier", "Classes/ModStore.lua", 421},
        {"new_mod", common.classes.ModStore, "NewMod", "Classes/ModStore.lua", 142},
        {"sum", common.classes.ModStore, "Sum", "Classes/ModStore.lua", 202},
        {"override", common.classes.ModStore, "Override", "Classes/ModStore.lua", 301},
        {"add_mod", common.classes.ModDB, "AddMod", "Classes/ModDB.lua", 31},
        {"sum_internal", common.classes.ModDB, "SumInternal", "Classes/ModDB.lua", 137},
    }
end
function M.begin(enable, expectedJit)
    assert(debug.gethook() == nil and jit.status() == expectedJit)
    originals, where, captured, hooked, finished = {}, {}, {}, enable, false
    for _, m in ipairs(methods()) do originals[m[1]], where[m[1]] = m[2][m[3]], original(m[2][m[3]], m[4], m[5]) end
    where.cache_copy = original(specCopy, "Modules/Common.lua", 542)
    originals.cache_copy = specCopy
    local function hook(_, line)
        if line ~= 837 then return end
        if debug.getinfo(2, "f").func ~= originals.initializer then return end
        local locals = {}
        for i = 1, 64 do
            local name, value = debug.getlocal(2, i)
            if not name then break end
            if name == "env" or name == "modDB" or name == "build" or name == "cachedPlayerDB" then locals[name] = value end
        end
        local env, db, input = assert(locals.env), assert(locals.modDB), assert(locals.build)
        assert(not locals.cachedPlayerDB and env.modDB == db and env.player.modDB == db)
        local mods = assert(db.mods.Life)
        assert(#mods == 1)
        local mod = mods[1]
        assert(mod.name == "Life" and mod.type == "BASE" and mod.source == "Base")
        captured[#captured + 1] = {store = db, mod = mod, record = record(mod),
            character_level = input.characterLevel, stored_level = db.multipliers.Level,
            constant = data.characterConstants.life_per_level, class_id = env.classId, mode = env.mode}
    end
    if enable then debug.sethook(hook, "l") end
    return function()
        if enable then assert(debug.gethook() == hook); debug.sethook() end
        assert(debug.gethook() == nil and jit.status() == expectedJit)
        for _, m in ipairs(methods()) do assert(m[2][m[3]] == originals[m[1]]) end
        assert(specCopy == originals.cache_copy)
        finished = true
    end
end
local function chain(db)
    local rows, seen = {}, {}
    while db do
        assert(not seen[db]); seen[db] = true
        local life, level = {}, {}
        for _, mod in ipairs(db.mods.Life or {}) do life[#life + 1] = record(mod) end
        for _, mod in ipairs(db.mods["Multiplier:Level"] or {}) do level[#level + 1] = record(mod) end
        rows[#rows + 1] = {life = life, multiplier_level = level,
            stored_level = {present = db.multipliers.Level ~= nil, value = db.multipliers.Level}}
        db = db.parent
    end
    return rows
end
local function selected()
    return {items = build.itemsTab.activeItemSetId, spec = build.treeTab.activeSpec,
        skills = build.skillsTab.activeSkillSetId, config = build.configTab.activeConfigSetId,
        group = build.mainSocketGroup, character_level = build.characterLevel}
end
function M.observe(expectedJit)
    assert(finished and debug.gethook() == nil and jit.status() == expectedJit)
    local initializers = {}
    for _, row in ipairs(captured) do
        initializers[#initializers + 1] = {record = row.record, character_level = row.character_level,
            stored_level = row.stored_level, constant = row.constant, class_id = row.class_id, mode = row.mode}
    end
    if hooked then assert(#initializers > 0) else assert(#initializers == 0) end
    local saved = selected()
    local allItems = {}
    for id, item in pairs(build.itemsTab.items) do allItems[id] = {item = item, raw = item.raw} end
    local modes = {}
    for _, mode in ipairs({"MAIN", "CALCS"}) do
        local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local db = assert(env.player.modDB)
        assert(env.modDB == db and db.actor == env.player)
        assert(db.EvalMod == originals.eval_mod and db.GetMultiplier == originals.multiplier)
        assert(db.Sum == originals.sum and db.Override == originals.override)
        local before, output = chain(db), scalar(env.player.output)
        local candidates, current, seen, depth = {}, db, {}, 0
        while current do
            assert(not seen[current]); seen[current] = true
            for _, mod in ipairs(current.mods.Life or {}) do
                if mod.source == "Base" and mod.type == "BASE" then candidates[#candidates + 1] = {mod = mod, store = current, depth = depth} end
            end
            current, depth = current.parent, depth + 1
        end
        assert(#candidates == 1)
        local chosen, creation = candidates[1], nil
        if hooked then
            for _, row in ipairs(captured) do
                if row.mod == chosen.mod then
                    assert(creation == nil, "unique original initializer for the exact record")
                    creation = row
                end
            end
            assert(creation, "actual Life record is the original initialized object")
            assert(equal(creation.record, record(chosen.mod)))
        end
        local first = originals.eval_mod(db, chosen.mod, nil, {})
        local second = originals.eval_mod(db, chosen.mod, nil, {})
        assert(type(first) == "number" and first == second)
        local multiplier = originals.multiplier(db, "Level", nil)
        local added = originals.sum(db, "BASE", nil, "Multiplier:Level")
        local override = originals.override(db, nil, "Multiplier:Level")
        local sourceTotal = originals.sum(db, "BASE", {source = "Base"}, "Life")
        assert(sourceTotal == first)
        assert(equal(before, chain(db)) and equal(output, scalar(env.player.output)))
        modes[mode] = {class_id = env.classId, class_name = env.spec.curClassName,
            character_level = env.player.level, record = record(chosen.mod), ancestor_depth = chosen.depth,
            effective_value = first, base_source_sum = sourceTotal,
            multiplier = {effective = multiplier, extra_base = added,
                override = {present = override ~= nil, value = override}, read_set = before},
            player_output = output,
            provenance = hooked and {exact_initialized_record = true,
                initializer_store_is_owner = creation.store == chosen.store,
                initialized_character_level = creation.character_level, initialized_stored_level = creation.stored_level,
                initialized_constant = creation.constant, initializer_class_id = creation.class_id} or nil}
    end
    assert(equal(saved, selected()))
    for id, savedItem in pairs(allItems) do assert(build.itemsTab.items[id] == savedItem.item and savedItem.item.raw == savedItem.raw) end
    local classes = {}
    for id, class in pairs(build.spec.tree.classes) do
        classes[#classes + 1] = {id = id, name = class.name, integer_id = class.integerId,
            base_str = class.base_str, base_dex = class.base_dex, base_int = class.base_int}
    end
    table.sort(classes, function(a,b) return a.id < b.id end)
    for _, m in ipairs(methods()) do assert(m[2][m[3]] == originals[m[1]]) end
    return {methods = where, selected = saved, classes = classes, constant = data.characterConstants.life_per_level,
        modes = modes, initializers = initializers, hooked = hooked,
        evidence = {original_methods_preserved = true, direct_original_eval = true,
            repeated_eval_equal = true, relevant_read_set_preserved = true,
            saved_items_preserved = true, saved_selection_preserved = true,
            cached_outputs_preserved = true, business_method_wrappers = false,
            final_life_claim = false, contributor_closure = false, multiplier_compatibility_admitted = false}}
end
return M
