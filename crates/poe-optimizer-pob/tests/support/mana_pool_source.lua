-- Observe the full, unchanged resource calculation. Original ModStore methods
-- supply inputs; original doActorLifeManaSpirit supplies the reference output.
local calcs = require("Modules.CalcBase")
local function plain(v, depth)
    if type(v) ~= "table" then return v end
    depth = (depth or 0) + 1; assert(depth < 32)
    local out = {}; for k,x in pairs(v) do out[k] = plain(x,depth) end; return out
end
local function list(v)
    local out = {}; for i,x in ipairs(v) do out[i] = plain(x) end; return out
end
local function equal(a,b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end; return true
end
local names = {"Mana", "ExtraMana", "ManaTotal", "ManaConvertToEnergyShield", "ManaConvertToArmour", "ManaConvertToEvasion"}
local function snapshot(db)
    local rows, seen = {}, {}
    while db do
        assert(not seen[db]); seen[db] = true
        local row = {}; for _,name in ipairs(names) do row[name] = plain(db.mods[name] or {}) end
        rows[#rows+1] = row; db = db.parent
    end
    return rows
end
local function original(f, suffix, line)
    local d = debug.getinfo(f,"S")
    assert(d.what == "Lua" and d.source:gsub("\\","/"):sub(-#suffix) == suffix and d.linedefined == line)
    return f
end
local sum = original(common.classes.ModStore.Sum,"Classes/ModStore.lua",202)
local more = original(common.classes.ModStore.More,"Classes/ModStore.lua",261)
local override = original(common.classes.ModStore.Override,"Classes/ModStore.lua",301)
local calc = original(calcs.doActorLifeManaSpirit,"Modules/CalcDefence.lua",74)
local parser = original(modLib.parseMod,"Modules/ModParser.lua",7404)
assert(debug.gethook() == nil)
local function inputs(db)
    assert(db.Sum == sum and db.More == more and db.Override == override)
    local o = override(db,nil,"Mana")
    return {base=sum(db,"BASE",nil,"Mana"), extra=sum(db,"BASE",nil,"ExtraMana"),
        total=sum(db,"BASE",nil,"ManaTotal"), increased=sum(db,"INC",nil,"Mana"),
        more=more(db,nil,"Mana"), conversion_sum=sum(db,"BASE",nil,"ManaConvertToEnergyShield","ManaConvertToArmour","ManaConvertToEvasion"),
        override={present=o~=nil,value=o}}
end
local modes = {}
for _,mode in ipairs({"MAIN","CALCS"}) do
    local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
    local actor = env.player; local db = actor.modDB
    local before, reads = snapshot(db), inputs(db)
    local output, has = actor.output.Mana, actor.output.ManaHasOverride
    calc(actor,true)
    assert(actor.output.Mana == output and actor.output.ManaHasOverride == has)
    assert(equal(before,snapshot(db)) and equal(reads,inputs(db)))
    calc(actor,true)
    assert(actor.output.Mana == output and actor.output.ManaHasOverride == has)
    assert(equal(before,snapshot(db)) and equal(reads,inputs(db)))
    local node = assert(env.spec.tree.nodes[51749])
    local description = assert(node.sd[1]); assert(description == "You have no Mana")
    local cached = plain(assert(modLib.parseModCache[description]))
    local mods, extra = parser(description)
    assert(equal(cached,plain(modLib.parseModCache[description])))
    assert(#mods==1 and not extra and mods[1].name=="Mana" and mods[1].type=="OVERRIDE" and mods[1].value==0)
    assert(equal(before,snapshot(db)) and equal(reads,inputs(db)))
    modes[mode] = {inputs=reads,final_mana=output,has_override=has,read_set=before,
        override_source={tree_version="0_5",node_id=51749,allocated=env.allocNodes[51749]~=nil,
            descriptions=list(node.sd),modifiers=list(mods),default_modifiers=list(node.modList),parser_cache_preserved=true}}
end
return {modes=modes,original_methods=true,original_calculation=true,repeated_equal=true,
    mana_inputs_preserved=true,native_final_pool_claim=false}
