-- Read the original completed attribute bonus and its actual controlling flags.
local names = {"NoAttributeBonuses", "NoIntelligenceAttributeBonuses", "NoIntBonusToMana", "DoubledInherentAttributeBonuses"}
local function plain(v, depth)
    if type(v) ~= "table" then return v end
    depth = (depth or 0) + 1; assert(depth < 32)
    local out = {}; for k,x in pairs(v) do out[k] = plain(x, depth) end; return out
end
local function equal(a,b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end
    return true
end
local function record(m)
    local tags = {}; for i,t in ipairs(m) do tags[i] = plain(t) end
    return {name=m.name,type=m.type,value=m.value,source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
end
local function snapshot(db)
    local rows,seen = {},{}
    while db do
        assert(not seen[db]);seen[db]=true
        local flags = {}
        for _,name in ipairs(names) do
            local mods={};for _,m in ipairs(db.mods[name] or {}) do mods[#mods+1]=record(m) end
            flags[name]=mods
        end
        local int={};for _,m in ipairs(db.mods.Int or {}) do int[#int+1]=record(m) end
        rows[#rows+1]={flags=flags,intelligence=int};db=db.parent
    end
    return rows
end
local flag,sum = common.classes.ModStore.Flag, common.classes.ModStore.Sum
for _,f in ipairs({flag,sum}) do
    local info=debug.getinfo(f,"S")
    assert(info.what=="Lua" and info.source:gsub("\\","/"):sub(-20)=="Classes/ModStore.lua")
end
assert(debug.gethook()==nil)
local out={}
for _,mode in ipairs({"MAIN","CALCS"}) do
    local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
    local db=assert(env.player.modDB)
    assert(db.Flag==flag and db.Sum==sum)
    local before=snapshot(db)
    local intelligence,mana=env.player.output.Int,env.player.output.Mana
    local flags={}
    for _,name in ipairs(names) do
        local value=flag(db,nil,name)
        assert(value==nil or type(value)=="boolean")
        flags[name]={present=value~=nil,value=value,resolved=value==true}
    end
    local amount=sum(db,"BASE",{source="Intelligence"},"Mana")
    assert(amount==sum(db,"BASE",{source="Intelligence"},"Mana"))
    assert(intelligence==env.player.output.Int and mana==env.player.output.Mana)
    assert(equal(before,snapshot(db)))
    out[mode]={intelligence=intelligence,flags=flags,amount=amount,read_set=before}
end
return {modes=out,original_methods=true,repeated_equal=true,input_and_output_preserved=true}
