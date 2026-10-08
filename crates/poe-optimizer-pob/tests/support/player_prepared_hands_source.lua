-- Optional extension of player_offhand_source.lua. Original methods are never replaced.
-- Capture actual prepared profiles and original condition-write branches, not a formula.
return function(base)
local M = {}
local hooked, finished, calls, pending, original, where
local condition_names = {"Unarmed", "Unencumbered", "HollowPalm"}
local function plain(v, depth)
    if type(v) ~= "table" then
        assert(type(v)=="nil" or type(v)=="string" or type(v)=="number" or type(v)=="boolean")
        if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return tostring(v) end
        return v
    end
    depth=(depth or 0)+1; assert(depth<32)
    local out={}; for k,x in pairs(v) do
        assert(type(k)=="string" or type(k)=="number"); out[k]=plain(x,depth)
    end; return out
end
local function present(v) return {present=v~=nil,kind=type(v),value=plain(v)} end
local function equal(a,b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end; return true
end
local function identify(f,path,line)
    local i=debug.getinfo(f,"S")
    assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path and i.linedefined==line)
    return {path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function actor_function(f)
    for i=1,128 do local n,v=debug.getupvalue(f,i)
        if not n then break end; if n=="doActorAttribsConditions" then return v end
    end; error("missing original actor condition function")
end
local function item(v)
    if not v then return {present=false} end
    local id; for k,x in pairs(build.itemsTab.items) do if x==v then assert(not id);id=k end end
    assert(v.base and v.baseName and v.base==data.itemBases[v.baseName])
    return {present=true,source_item_id=present(id),base_name=v.baseName,type=v.type,
        exact_catalogue_base=true,raw=v.raw,weapon_data_present=v.weaponData~=nil}
end
local function conditions(db)
    local out={}; for _,name in ipairs(condition_names) do out[name]=present(rawget(db.conditions,name)) end
    return out
end
local function snapshot(env)
    local actor=assert(env.player); local db=assert(actor.modDB); assert(db==env.modDB)
    local one,two=actor.itemList["Weapon 1"],actor.itemList["Weapon 2"]
    local intrinsic=assert(env.data.unarmedWeaponData[env.classId])
    local names={"DisableWeapons","CanAttackAsOneHandMaceUnarmed","UseFacebreakerItemDamage","Keystone"}
    local ancestry,seen={},{}; local current=db
    while current do
        assert(not seen[current]);seen[current]=true
        local mods={}; for _,name in ipairs(names) do mods[name]=plain(current.mods[name] or {}) end
        ancestry[#ancestry+1]={conditions=conditions(current),modifiers=mods};current=current.parent
    end
    return {class_id=present(env.classId),prepared_main=item(one),prepared_offhand=item(two),
        prepared_gloves=item(actor.itemList.Gloves),legacy_player_gloves=present(rawget(actor,"Gloves")),
        primary=present(actor.weaponData1),secondary=present(actor.weaponData2),
        intrinsic_catalogue=plain(intrinsic),
        primary_is_item_profile=one~=nil and one.weaponData~=nil and actor.weaponData1==one.weaponData[1],
        secondary_is_item_profile=two~=nil and two.weaponData~=nil and actor.weaponData2==two.weaponData[2],
        primary_is_catalogue_object=actor.weaponData1==intrinsic,
        primary_equals_catalogue=equal(actor.weaponData1,intrinsic),
        conditions=conditions(db),ancestry=ancestry}
end
local lines={ [1853]=true,[1854]=true,[1861]=true,[1867]=true,[1873]=true,[1877]=true,[1889]=true,
    [280]=true,[281]=true,[283]=true,[319]=true,
    [3229]=true,[3230]=true,[3231]=true,[3233]=true,[3240]=true }
function M.begin(enable,expected_jit)
    local finish_base=base.begin(false,expected_jit)
    hooked,finished,calls,pending=enable,false,{},{}
    local calcs=require("Modules.CalcBase")
    original={initializer=calcs.initEnv,perform=calcs.perform}
    original.actor=actor_function(original.perform)
    where={initializer=identify(original.initializer,"Modules/CalcSetup.lua",717),
        perform=identify(original.perform,"Modules/CalcPerform.lua",1193),
        actor=identify(original.actor,"Modules/CalcPerform.lua",264)}
    local failure, next_invocation = nil, 0
    local function hook(event,line)
        if failure or (event=="line" and not lines[line]) then return end
        local f=debug.getinfo(2,"f").func
        local kind=f==original.initializer and "initialization" or f==original.actor and "conditions" or f==original.perform and "late_disable"
        if not kind then return end
        -- A line event is not a function invocation: compound expressions can
        -- resume at the same line after a nested call. Track original frames,
        -- preserve those revisits, and never identify a call by shared ModDB.
        if event~="line" then
            local ok,problem=xpcall(function()
                pending[f]=pending[f] or {}
                local stack=pending[f]
                if event=="call" then
                    next_invocation=next_invocation+1
                    stack[#stack+1]={invocation=next_invocation}
                elseif event=="return" then
                    local frame=assert(stack[#stack],"original return without call")
                    assert(not frame.branch,"unfinished original branch at return")
                    stack[#stack]=nil
                else
                    error("unexpected original hook event "..event)
                end
            end,debug.traceback)
            if not ok then failure=problem end
            return
        end
        local locals={};for i=1,100 do local n,v=debug.getlocal(2,i);if not n then break end
            if n=="env" or n=="actor" then locals[n]=v end end
        local ok,problem=xpcall(function()
            local env=assert(locals.env)
            if kind=="conditions" and locals.actor~=env.player then return end
            local first,last=1853,1889
            if kind=="conditions" then first,last=280,319 end
            if kind=="late_disable" then first,last=3229,3240 end
            -- Relevant line numbers from other original functions are not this interval.
            if line<first or line>last then return end
            local key=assert(env.player.modDB)
            local stack=assert(pending[f],"original line without call")
            local frame=assert(stack[#stack],"original frame missing")
            if line==first then
                assert(not frame.closed,"re-entered completed original branch")
                local state=snapshot(env)
                if frame.branch then
                    local p=frame.branch
                    assert(p.env==env and p.actor==env.player and p.store==key)
                    assert(#p.events==0,"entry line revisited after branch progression")
                    p.entry_events[#p.entry_events+1]={line=line,state=state}
                else
                    frame.branch={env=env,actor=env.player,store=key,kind=kind,mode=env.mode,
                        source_invocation=frame.invocation,first=first,last=last,before=state,
                        entry_events={{line=line,state=state}},events={}}
                end
            elseif line==last then
                local p=assert(frame.branch,"missing original branch entry")
                assert(p.env==env and p.actor==env.player and p.store==key)
                p.after=snapshot(env);calls[#calls+1]=p;frame.branch=nil;frame.closed=true
            else
                local p=assert(frame.branch,"missing original branch entry")
                assert(p.env==env and p.actor==env.player and p.store==key)
                p.events[#p.events+1]={line=line,state=snapshot(env)}
            end
        end,debug.traceback)
        if not ok then failure=problem end
    end
    if hooked then debug.sethook(hook,"crl") end
    return function()
        if hooked then assert(debug.gethook()==hook);debug.sethook() end
        assert(not failure,failure)
        for _,group in pairs(pending) do assert(next(group)==nil,"unfinished original branch") end
        assert(calcs.initEnv==original.initializer and calcs.perform==original.perform)
        assert(actor_function(calcs.perform)==original.actor)
        finish_base();finished=true
    end
end
function M.observe(expected_jit)
    assert(finished and not debug.gethook() and jit.status()==expected_jit)
    local result=base.observe(expected_jit)
    local modes={};for _,mode in ipairs({"MAIN","CALCS"}) do
        local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local row={final=snapshot(env)}
        if hooked then
            local provenance={}
            for i,c in ipairs(calls) do if c.env==env and c.actor==env.player then
                assert(not provenance[c.kind],"duplicate original branch on final environment")
                assert(c.store==env.modDB)
                provenance[c.kind]={invocation=i,exact_actor=true,exact_store=true}
            end end
            assert(provenance.initialization and provenance.conditions and provenance.late_disable)
            row.provenance=provenance
        end
        modes[mode]=row
    end
    local invocations={};for i,c in ipairs(calls) do
        invocations[i]={kind=c.kind,mode=c.mode,source_invocation=c.source_invocation,
            first=c.first,last=c.last,before=c.before,after=c.after,entry_events=c.entry_events,events=c.events,
            exact_final_main=c.env==build.calcsTab.mainEnv,exact_final_calcs=c.env==build.calcsTab.calcsEnv}
    end
    result.prepared_hands={methods=where,modes=modes,invocations=invocations,hooked=hooked,
        evidence={actual_original_profile_assignment=true,actual_original_condition_branches=true,
            separate_late_disable_branch=true,late_disable_activated=false,
            scalar_outputs_preserved=true,business_method_wrappers=false,
            effective_condition_closure=false,full_native_build_parity=false}}
    return result
end
return M
end
