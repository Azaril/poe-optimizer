-- Observe original complete Player attribute/bonus work; no formula is copied.
local M = {}
local originals, where, captured, pending, hooked, finished
local names = {"NoAttributeBonuses", "NoStrengthAttributeBonuses", "NoStrBonusToLife",
    "DoubledInherentAttributeBonuses", "HalvesLifeFromStrength"}
local function original(f, path, first)
    assert(type(f) == "function", "missing original " .. path)
    local info = debug.getinfo(f, "S")
    assert(info.what == "Lua" and info.source:gsub("\\", "/"):sub(-#path) == path)
    assert(info.linedefined == first, "unexpected original line " .. path .. ":" .. info.linedefined)
    return {path = path, first = info.linedefined, last = info.lastlinedefined}
end
local function upvalue(f, wanted)
    local found
    for i = 1, 128 do
        local name, value = debug.getupvalue(f, i)
        if not name then break end
        if name == wanted then assert(found == nil); found = value end
    end
    return assert(found, "missing original upvalue " .. wanted)
end
local function plain(value, depth)
    if type(value) ~= "table" then
        assert(type(value) == "nil" or type(value) == "string" or type(value) == "number" or type(value) == "boolean")
        if type(value) == "number" and (value ~= value or value == math.huge or value == -math.huge) then return tostring(value) end
        return value
    end
    depth = (depth or 0) + 1; assert(depth < 32)
    local out = {}; for k, v in pairs(value) do out[k] = plain(v, depth) end; return out
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
    for k,v in pairs(t) do if type(v)=="number" or type(v)=="string" or type(v)=="boolean" then out[k]=plain(v) end end
    return out
end
local function record(mod)
    local tags = {}; for i, tag in ipairs(mod) do tags[i] = plain(tag) end
    return {name=mod.name,type=mod.type,value=plain(mod.value),source=mod.source,
        flags=mod.flags,keyword_flags=mod.keywordFlags,tags=tags}
end
local function flags(db)
    local out = {}
    for _, name in ipairs(names) do
        local raw = originals.flag(db,nil,name)
        assert(raw == nil or type(raw)=="boolean")
        out[name]={present=raw~=nil,value=raw,resolved=raw==true}
    end
    return out
end
local function chain(db)
    local out, seen = {}, {}
    while db do
        assert(not seen[db]);seen[db]=true
        local row={life={},flags={}}
        for _,mod in ipairs(db.mods.Life or {}) do row.life[#row.life+1]=record(mod) end
        for _,name in ipairs(names) do
            row.flags[name]={}
            for _,mod in ipairs(db.mods[name] or {}) do row.flags[name][#row.flags[name]+1]=record(mod) end
        end
        out[#out+1]=row;db=db.parent
    end
    return out
end
local function strength_records(db)
    local rows, seen = {}, {}
    while db do
        assert(not seen[db]);seen[db]=true
        for _,mod in ipairs(db.mods.Life or {}) do
            if mod.source=="Strength" then
                assert(mod.name=="Life" and mod.type=="BASE")
                rows[#rows+1]={store=db,mod=mod,record=record(mod)}
            end
        end
        db=db.parent
    end
    return rows
end
local function methods()
    return {{"perform",require("Modules.CalcBase"),"perform","Modules/CalcPerform.lua",1193},
        {"flag",common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
        {"sum",common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
        {"new_mod",common.classes.ModStore,"NewMod","Classes/ModStore.lua",142},
        {"eval_mod",common.classes.ModStore,"EvalMod","Classes/ModStore.lua",490},
        {"add_mod",common.classes.ModDB,"AddMod","Classes/ModDB.lua",31},
        {"flag_internal",common.classes.ModDB,"FlagInternal","Classes/ModDB.lua",297}}
end
function M.begin(enable,expectedJit)
    assert(debug.gethook()==nil and jit.status()==expectedJit)
    originals,where,captured,pending,hooked,finished={},{},{},{},enable,false
    for _,m in ipairs(methods()) do originals[m[1]],where[m[1]]=m[2][m[3]],original(m[2][m[3]],m[4],m[5]) end
    originals.actor=upvalue(originals.perform,"doActorAttribsConditions")
    originals.attributes=upvalue(originals.actor,"calculateAttributes")
    where.actor=original(originals.actor,"Modules/CalcPerform.lua",264)
    where.attributes=original(originals.attributes,"Modules/CalcPerform.lua",233)
    local failure
    local function hook(event,line)
        if failure then return end
        if debug.getinfo(2,"f").func~=originals.actor then return end
        if event=="line" and line~=496 and line~=522 then return end
        local locals={}
        for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end
            if name=="env" or name=="actor" then locals[name]=value end end
        local ok,problem=xpcall(function()
            local env,actor=assert(locals.env),assert(locals.actor)
            if actor~=env.player then return end
            local db,output=assert(actor.modDB),assert(actor.output)
            assert(env.modDB==db)
            if event=="call" then
                assert(not pending[db],"nested original Player attribute invocation")
                pending[db]={env=env,actor=actor,db=db,mode=env.mode,phase="entry"}
            elseif event=="line" and line==496 then
                local frame=assert(pending[db],"missing original entry before bonus")
                assert(frame.env==env and frame.actor==actor)
                assert(type(output.Str)=="number" and output.Str>=0 and output.Str==math.floor(output.Str))
                local before=chain(db);local resolved=flags(db);assert(equal(before,chain(db)))
                if frame.phase=="before" then
                    -- A line hook may revisit the call-site line. It is still
                    -- the same original invocation, with unchanged inputs.
                    assert(frame.strength==output.Str and equal(frame.before,before) and equal(frame.flags,resolved))
                else
                    assert(frame.phase=="entry","unexpected original bonus phase")
                    frame.phase="before";frame.strength=output.Str;frame.flags=resolved
                    frame.before=before;frame.prior=strength_records(db)
                end
            elseif event=="line" and line==522 then
                local frame=assert(pending[db],"missing original entry after bonus")
                assert(frame.env==env and frame.actor==actor and frame.strength==output.Str)
                assert(equal(frame.flags,flags(db)))
                local after=strength_records(db);local added={}
                for _,r in ipairs(after) do
                    local existing=false;for _,prior in ipairs(frame.prior) do if r.mod==prior.mod then existing=true end end
                    if not existing then added[#added+1]=r end
                end
                assert(#added<=1,"multiple Strength records emitted by one Player stage")
                local readset=chain(db)
                if frame.phase=="after" then
                    assert(equal(frame.after,readset) and #frame.added==#added)
                    for i,r in ipairs(added) do assert(frame.added[i].mod==r.mod) end
                else
                    assert(frame.phase=="before","bonus end without original start")
                    frame.phase="after";frame.added=added;frame.after=readset
                end
            elseif event=="return" then
                local frame=assert(pending[db],"missing original Player invocation at return")
                assert(frame.phase=="after","original Player function returned without both bonus boundaries")
                assert(frame.strength==output.Str and equal(frame.flags,flags(db)))
                pending[db]=nil;captured[#captured+1]=frame
            else
                assert(event=="line","unexpected original Player hook event "..event)
            end
        end,debug.traceback)
        if not ok then failure=problem end
    end
    if enable then jit.flush();debug.sethook(hook,"crl") end
    return function()
        if enable then assert(debug.gethook()==hook);debug.sethook() end
        assert(not failure,failure)
        assert(debug.gethook()==nil and next(pending)==nil and jit.status()==expectedJit)
        for _,m in ipairs(methods()) do assert(m[2][m[3]]==originals[m[1]]) end
        assert(upvalue(originals.perform,"doActorAttribsConditions")==originals.actor)
        assert(upvalue(originals.actor,"calculateAttributes")==originals.attributes)
        finished=true
    end
end
local function selected()
    return {items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,
        config=build.configTab.activeConfigSetId,group=build.mainSocketGroup,character_level=build.characterLevel}
end
function M.observe(expectedJit)
    assert(finished and debug.gethook()==nil and jit.status()==expectedJit)
    local saved=selected();local items={}
    for id,item in pairs(build.itemsTab.items) do items[id]={item=item,raw=item.raw} end
    local modes={}
    for _,mode in ipairs({"MAIN","CALCS"}) do
        local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local actor=assert(env.player);local db=assert(actor.modDB)
        local before,output=chain(db),scalar(actor.output);local actual=strength_records(db)
        assert(#actual<=1)
        local amount=originals.sum(db,"BASE",{source="Strength"},"Life")
        local repeated=originals.sum(db,"BASE",{source="Strength"},"Life")
        assert(type(amount)=="number" and amount==repeated)
        local resolved=flags(db);local chosen
        if hooked then
            for _,frame in ipairs(captured) do
                if frame.env==env and frame.actor==actor and frame.db==db then
                    if #actual==0 and #frame.added==0 or #actual==1 and #frame.added==1 and actual[1].mod==frame.added[1].mod then chosen=frame end
                end
            end
            assert(chosen,"actual Player Strength bonus must bind to original stage")
            assert(chosen.strength==actor.output.Str and equal(chosen.flags,resolved))
        end
        local records={};for _,r in ipairs(actual) do
            local value=originals.eval_mod(db,r.mod,nil,{})
            assert(value==amount and equal(record(r.mod),r.record));records[#records+1]=r.record
        end
        assert(equal(before,chain(db)) and equal(output,scalar(actor.output)))
        modes[mode]={class_id=env.classId,class_name=env.spec.curClassName,character_level=actor.level,
            strength=actor.output.Str,flags=resolved,records=records,inherent_life=amount,read_set=before,player_output=output,
            provenance=hooked and {original_actor_function=true,original_attribute_function=true,
                before_line=496,after_line=522,exact_emitted_record=#actual==1,emitted_count=#chosen.added,
                stage_strength=chosen.strength,stage_flags=chosen.flags,before=chosen.before,after=chosen.after} or nil}
    end
    assert(equal(saved,selected()))
    for id,row in pairs(items) do assert(build.itemsTab.items[id]==row.item and row.item.raw==row.raw) end
    for _,m in ipairs(methods()) do assert(m[2][m[3]]==originals[m[1]]) end
    return {methods=where,selected=saved,hooked=hooked,modes=modes,
        evidence={original_methods_preserved=true,original_complete_player_stage=true,repeated_sum_equal=true,
            relevant_read_set_preserved=true,cached_outputs_preserved=true,saved_items_preserved=true,saved_selection_preserved=true,
            business_method_wrappers=false,final_life_claim=false,attribute_contributor_closure=false,flag_producer_closure=false}}
end
return M
