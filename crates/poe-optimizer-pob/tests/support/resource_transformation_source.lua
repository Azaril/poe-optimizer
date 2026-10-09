-- Evidence-only observation of the unchanged, complete resource pass.
-- Inputs enter through the ordinary XML/configuration/parser path. No source
-- method, modifier store, resource table or calculation result is replaced.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
    local info = debug.getinfo(f, "S")
    assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-#path) == path
        and info.linedefined == line)
    return f
end
local defence = original(calcs.defence, "Modules/CalcDefence.lua", 789)
local rebuild = original(build.calcsTab.BuildOutput, "Classes/CalcsTab.lua", 486)
local sum = original(common.classes.ModStore.Sum, "Classes/ModStore.lua", 202)
local parser = original(modLib.parseMod, "Modules/ModParser.lua", 7404)
local resources = {"Armour", "Evasion", "EnergyShield", "Life", "Mana", "Ward"}
local function plain(v, depth)
    if type(v) ~= "table" then
        assert(v == nil or type(v) == "number" or type(v) == "string" or type(v) == "boolean")
        return v
    end
    depth = (depth or 0) + 1; assert(depth < 24)
    local result = {}; for k, value in pairs(v) do result[k] = plain(value, depth) end
    return result
end
local function equal(a, b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k, value in pairs(a) do if not equal(value, b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end
    return true
end
local function preserved()
    assert(calcs.defence == defence and build.calcsTab.BuildOutput == rebuild
        and common.classes.ModStore.Sum == sum and modLib.parseMod == parser)
end
local function snapshot()
    local result = {}
    for _, mode in ipairs({"MAIN", "CALCS"}) do
        local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local actor, db = env.player, env.player.modDB
        assert(db == env.modDB and db.actor == actor and db.Sum == sum)
        local row = {resources = {}, stores = {}}
        for _, name in ipairs(resources) do
            local rates = {}
            for _, target in ipairs(resources) do
                rates[target] = {conversion = sum(db, "BASE", nil, name .. "ConvertTo" .. target),
                    gain = sum(db, "BASE", nil, name .. "GainAs" .. target)}
            end
            row.resources[name] = {final = actor.output[name], base = sum(db, "BASE", nil, name),
                extra = sum(db, "BASE", nil, "Extra" .. name),
                total = sum(db, "BASE", nil, name .. "Total"), rates = rates}
        end
        local seen = {}
        while db do
            assert(not seen[db]); seen[db] = true
            local buckets = {}
            for _, name in ipairs(resources) do
                for _, key in ipairs({name, "Extra" .. name, name .. "Total"}) do
                    buckets[key] = plain(db.mods[key] or {})
                end
                for _, target in ipairs(resources) do
                    for _, operation in ipairs({"ConvertTo", "GainAs"}) do
                        local key = name .. operation .. target
                        buckets[key] = plain(db.mods[key] or {})
                    end
                end
            end
            row.stores[#row.stores + 1] = buckets
            db = db.parent
        end
        result[mode] = row
    end
    return result
end
assert(debug.gethook() == nil)
local initial = snapshot()
rebuild(build.calcsTab)
local unobserved = snapshot()
assert(equal(initial, unobserved), "resource state changed after first normal rebuild")

local frames, failure = {}, nil
local function hook(event, line)
    if failure or event ~= "line" or (line ~= 1375 and line ~= 1430) then return end
    if debug.getinfo(2, "f").func ~= defence then return end
    local vars = {}
    for index = 1, 192 do
        local name, value = debug.getlocal(2, index); if not name then break end
        vars[name] = value
    end
    local ok, problem = xpcall(function()
        if vars.actor ~= vars.env.player then return end
        assert(vars.actor.modDB == vars.env.modDB and vars.modDB == vars.actor.modDB)
        local phase = line == 1375 and "before" or "after"
        local current = line == 1375 and vars.source or vars.res
        if current.name ~= "Armour" then return end
        local frame = frames[vars.env]
        if not frame then frame = {actor = vars.actor}; frames[vars.env] = frame end
        assert(frame.actor == vars.actor)
        local state = plain(vars.resourceList)
        -- A call/return within one source line can repeat its line event. It
        -- must expose exactly the same state, not a second transformation.
        if frame[phase] then assert(equal(frame[phase], state), "changed repeated resource checkpoint")
        else frame[phase] = state end
    end, debug.traceback)
    if not ok then failure = problem end
end
local jit_enabled = jit.status()
jit.flush()
debug.sethook(hook, "l")
local ok, problem = xpcall(function() rebuild(build.calcsTab) end, debug.traceback)
local owned_hook = debug.gethook() == hook
debug.sethook()
assert(owned_hook and ok, problem)
assert(not failure, failure)
preserved()
assert(jit.status() == jit_enabled)
local observed = snapshot()
assert(equal(unobserved, observed), "observer changed resource results or records")
local traces = {}
for _, mode in ipairs({"MAIN", "CALCS"}) do
    local env = mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
    local frame = assert(frames[env], "missing exact current Player resource frame")
    assert(frame.actor == env.player and frame.before and frame.after)
    traces[mode] = {before = frame.before, after = frame.after, exact_current_player = true,
        before_line = 1375, after_line = 1430}
end
rebuild(build.calcsTab)
assert(equal(observed, snapshot()), "resource state changed after observer removal")
preserved()
return {modes = observed, traces = traces, original_methods = true,
    initial_and_three_rebuilds_equal = true, uninstrumented_results_equal = true,
    observer_removed = debug.gethook() == nil, native_transformations_claim = false}
