-- Observe the real loaded Player Mana record through original ModStore methods.
-- No initializer, calculation, or modifier value is replaced by this observer.
local function plain(v, depth)
    if type(v) ~= "table" then return v end
    depth = (depth or 0) + 1
    assert(depth < 32)
    local out = {}
    for k, x in pairs(v) do out[k] = plain(x, depth) end
    return out
end
local function scalar_output(t)
    local out = {}
    for k, v in pairs(t) do
        if type(v) == "number" then
            out[k] = (v ~= v or v == math.huge or v == -math.huge) and tostring(v) or v
        elseif type(v) == "boolean" or type(v) == "string" then out[k] = v end
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
local function record(m)
    local tags = {}
    for i, tag in ipairs(m) do tags[i] = plain(tag) end
    return {name=m.name, type=m.type, value=m.value, source=m.source,
        flags=m.flags, keyword_flags=m.keywordFlags, tags=tags}
end
local function snapshot(db)
    local rows, seen = {}, {}
    while db do
        assert(not seen[db]); seen[db] = true
        local mana, levels = {}, {}
        for _, m in ipairs(db.mods.Mana or {}) do mana[#mana+1] = record(m) end
        for _, m in ipairs(db.mods["Multiplier:Level"] or {}) do levels[#levels+1] = record(m) end
        rows[#rows+1] = {mana=mana, level_modifiers=levels, stored_level=db.multipliers.Level}
        db = db.parent
    end
    return rows
end
local methods = {}
for name, line in pairs({EvalMod=490, GetMultiplier=421, Sum=202, Override=301}) do
    local f = common.classes.ModStore[name]
    local info = debug.getinfo(f, "S")
    assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-20) == "Classes/ModStore.lua")
    assert(info.linedefined == line)
    methods[name] = f
end
assert(debug.gethook() == nil)
local modes = {}
for _, mode in ipairs({"MAIN", "CALCS"}) do
    local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
    local db = assert(env.player.modDB)
    for name, f in pairs(methods) do assert(db[name] == f) end
    local candidates, cursor, seen = {}, db, {}
    while cursor do
        assert(not seen[cursor]); seen[cursor] = true
        for _, mod in ipairs(cursor.mods.Mana or {}) do
            if mod.type == "BASE" and mod.source == "Base" then candidates[#candidates+1] = mod end
        end
        cursor = cursor.parent
    end
    assert(#candidates == 1)
    local before, output = snapshot(db), scalar_output(env.player.output)
    local mod = candidates[1]
    local amount = methods.EvalMod(db, mod, nil, {})
    assert(amount == methods.EvalMod(db, mod, nil, {}))
    local total = methods.Sum(db, "BASE", {source="Base"}, "Mana")
    local level = methods.GetMultiplier(db, "Level", nil)
    local extra = methods.Sum(db, "BASE", nil, "Multiplier:Level")
    local override = methods.Override(db, nil, "Multiplier:Level")
    assert(total == amount and equal(before, snapshot(db)) and equal(output, scalar_output(env.player.output)))
    modes[mode] = {class_id=env.classId, class_name=env.spec.curClassName,
        level=env.player.level, record=record(mod), amount=amount, base_source_sum=total,
        multiplier={effective=level, extra=extra, override={present=override~=nil,value=override}},
        read_set=before, final_mana=env.player.output.Mana}
end
return {constant=data.characterConstants.mana_per_level, modes=modes,
    original_methods=true, repeated_equal=true, input_and_output_preserved=true,
    final_pool_claim=false, contributor_closure=false}
