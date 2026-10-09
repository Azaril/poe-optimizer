-- Observe original Life copy records after a complete build. Scalar probes call
-- the original ScaleAddMod in detached stores; no source formula is reproduced.
local function plain(v, depth)
    if type(v) ~= "table" then return v end
    depth = (depth or 0) + 1; assert(depth < 24)
    local out = {}; for k, x in pairs(v) do out[k] = plain(x, depth) end
    return out
end
local function equal(a, b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k, v in pairs(a) do if not equal(v, b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end
    return true
end
local function record(mod)
    local tags = {}; for i, tag in ipairs(mod) do tags[i] = plain(tag) end
    return {name=mod.name, type=mod.type, value=plain(mod.value), source=mod.source,
        flags=mod.flags, keyword_flags=mod.keywordFlags, tags=tags}
end
local function records(mods)
    local out = {}; for i, mod in ipairs(mods or {}) do out[i] = record(mod) end
    return out
end
local function original(f, suffix, line)
    local d = debug.getinfo(f, "S")
    assert(d.what == "Lua" and d.source:gsub("\\", "/"):sub(-#suffix) == suffix)
    assert(d.linedefined == line)
    return {path=suffix, first=d.linedefined, last=d.lastlinedefined}
end
local scale = common.classes.ModStore.ScaleAddMod
local locations = {
    scale=original(scale, "Classes/ModStore.lua", 82),
    add=original(common.classes.ModDB.AddMod, "Classes/ModDB.lua", 31),
    round=original(round, "Modules/Common.lua", 722),
    sum=original(common.classes.ModStore.Sum, "Classes/ModStore.lua", 202),
}
assert(debug.gethook() == nil)
local function chain(db)
    local rows, seen = {}, {}
    while db do
        assert(not seen[db] and #rows < 16); seen[db] = true
        rows[#rows+1] = {life=records(db.mods.Life),
            amulet_effect=records(db.mods.EffectOfBonusesFromAmulet)}
        db = db.parent
    end
    return rows
end
local modes = {}
for _, mode in ipairs({"MAIN", "CALCS"}) do
    local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
    local db = assert(env.player.modDB)
    local before = chain(db)
    local direct, copied = {}, {}
    local item = env.player.itemList.Amulet
    local items = {}
    for slot, selected in pairs(env.player.itemList) do
        items[#items+1] = {slot=slot, base=selected.baseName, type=selected.type, source=selected.modSource}
    end
    table.sort(items, function(a,b) return tostring(a.slot) < tostring(b.slot) end)
    local current, seen = db, {}
    while current do
        assert(not seen[current]); seen[current] = true
        for _, mod in ipairs(current.mods.Life or {}) do
            if mod.type == "BASE" then
                if item and mod.source == item.modSource then direct[#direct+1] = record(mod) end
                if mod.source and mod.source:find("%% Amulet Bonus Effect$") then copied[#copied+1] = record(mod) end
            end
        end
        current = current.parent
    end
    modes[mode] = {read_set=before, direct=direct, copied=copied, items=items,
        amulet_present=item ~= nil, amulet_type=item and item.type,
        final_life=env.player.output.Life,
        amulet_percent=db:Sum("INC", nil, "EffectOfBonusesFromAmulet")}
    assert(equal(before, chain(db)))
end
local probes = {}
local cases = {
    {"zero",17,0}, {"quarter",17,0.25}, {"negative-quarter",17,-0.25},
    {"identity",17,1}, {"below-integer",17,99.99/100},
    {"fractional-source",17.5,0.25}, {"fractional-negative",17.5,-0.25},
    {"fractional-identity",17.5,1}, {"bypass",17,0.25,true},
}
for _, c in ipairs(cases) do
    local mod = {name="Life", type="BASE", value=c[2], source="probe:"..c[1], flags=0, keywordFlags=0}
    if c[4] then mod[1] = {unscalable=true} end
    local before = plain(mod)
    local function run()
        local db = new("ModDB"):ModDB()
        assert(db.ScaleAddMod == scale)
        db:ScaleAddMod(mod, c[3])
        assert(#db.mods.Life == 1 and equal(mod, before))
        return record(db.mods.Life[1])
    end
    local output = run(); assert(equal(output, run()))
    probes[#probes+1] = {name=c[1], input=record(mod), factor=c[3], output=output}
end
assert(scale == common.classes.ModStore.ScaleAddMod and debug.gethook() == nil)
return {modes=modes, probes=probes, methods=locations,
    life_precision_present=data.highPrecisionMods.Life ~= nil and data.highPrecisionMods.Life.BASE ~= nil,
    default_precision=data.defaultHighPrecision, source_methods_replaced=false}
