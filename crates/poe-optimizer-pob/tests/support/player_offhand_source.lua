-- Observe original off-hand branches and parsed item identities. No copied formula.
local M = {}
local originals, where, calls, parsed, pending, hooked, finished
local names = {"UsingShield", "UsingFocus", "OffHandIsEmpty"}
local function original(f, path, first)
    assert(type(f)=="function", "missing original "..path)
    local i=debug.getinfo(f,"S")
    assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path)
    assert(i.linedefined==first,"unexpected original line "..path..":"..i.linedefined)
    return {path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,wanted)
    local found
    for i=1,128 do local n,v=debug.getupvalue(f,i);if not n then break end
        if n==wanted then assert(not found);found=v end end
    return assert(found,"missing original upvalue "..wanted)
end
local function plain(v,depth)
    if type(v)~="table" then
        assert(type(v)=="nil" or type(v)=="string" or type(v)=="number" or type(v)=="boolean")
        if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return tostring(v) end
        return v
    end
    depth=(depth or 0)+1;assert(depth<32)
    local out={};for k,x in pairs(v) do assert(type(k)=="string" or type(k)=="number");out[k]=plain(x,depth) end;return out
end
local function same(a,b)
    if type(a)~=type(b) then return false end
    if type(a)~="table" then return a==b end
    for k,v in pairs(a) do if not same(v,b[k]) then return false end end
    for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function present(v) return {present=v~=nil,kind=type(v),value=plain(v)} end
local function scalar_output(t)
    local values,excluded={},{}
    for k,v in pairs(t) do
        assert(type(k)=="string")
        if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then values[k]=plain(v)
        else excluded[#excluded+1]={key=k,kind=type(v)} end
    end
    table.sort(excluded,function(a,b)return a.key<b.key end)
    return {scalars=values,excluded=excluded}
end
local function raw_conditions(db)
    local out={};for _,name in ipairs(names) do out[name]=present(rawget(db.conditions,name)) end;return out
end
local function record(mod)
    local tags={};for i,t in ipairs(mod) do tags[i]=plain(t) end
    return {name=mod.name,type=mod.type,value=plain(mod.value),source=mod.source,
        flags=mod.flags,keyword_flags=mod.keywordFlags,tags=tags}
end
local function ancestry(db)
    local out,seen={},{}
    while db do
        assert(not seen[db]);seen[db]=true
        local row={conditions=raw_conditions(db),modifiers={}}
        for _,name in ipairs(names) do
            local mods={};for _,mod in ipairs(db.mods["Condition:"..name] or {}) do mods[#mods+1]=record(mod) end
            row.modifiers[name]=mods
        end
        out[#out+1]=row;db=db.parent
    end
    return out
end
local function methods()
    return {{"perform",require("Modules.CalcBase"),"perform","Modules/CalcPerform.lua",1193},
        {"parse_raw",common.classes.Item,"ParseRaw","Classes/Item.lua",468},
        {"load_items",common.classes.ItemsTab,"Load","Classes/ItemsTab.lua",1193},
        {"condition",common.classes.ModStore,"GetCondition","Classes/ModStore.lua",409},
        {"initializer",require("Modules.CalcBase"),"initEnv","Modules/CalcSetup.lua",717}}
end
function M.begin(enable,expectedJit)
    assert(not debug.gethook() and jit.status()==expectedJit)
    originals,where,calls,parsed,pending,hooked,finished={},{},{},{},{},enable,false
    for _,m in ipairs(methods()) do originals[m[1]]=m[2][m[3]];where[m[1]]=original(m[2][m[3]],m[4],m[5]) end
    originals.actor=upvalue(originals.perform,"doActorAttribsConditions")
    where.actor=original(originals.actor,"Modules/CalcPerform.lua",264)
    local failure
    local function hook(_,line)
        if failure or (line~=273 and line~=280 and line~=1079) then return end
        local f=debug.getinfo(2,"f").func
        if f~=originals.actor and f~=originals.parse_raw then return end
        local locals={}
        for i=1,80 do local n,v=debug.getlocal(2,i);if not n then break end
            if n=="env" or n=="actor" or n=="self" or n=="base" or n=="baseName" then locals[n]=v end end
        local ok,problem=xpcall(function()
            if f==originals.parse_raw then
                if line~=1079 then return end
                local item,base,name=assert(locals.self),assert(locals.base),assert(locals.baseName)
                assert(item.base==base and item.type==base.type and item.baseName==name and data.itemBases[name]==base)
                parsed[#parsed+1]={item=item,base=base,name=name,type=item.type}
                return
            end
            local env,actor=assert(locals.env),assert(locals.actor)
            if actor~=env.player then return end
            local db=assert(actor.modDB);assert(env.modDB==db)
            if line==273 then
                assert(not pending[db],"duplicate branch entry")
                pending[db]={env=env,actor=actor,db=db,item=actor.itemList["Weapon 2"],mode=env.mode,before=raw_conditions(db)}
            elseif line==280 then
                local p=assert(pending[db],"missing original off-hand entry")
                assert(p.env==env and p.actor==actor and p.item==actor.itemList["Weapon 2"])
                p.after=raw_conditions(db);calls[#calls+1]=p;pending[db]=nil
            end
        end,debug.traceback)
        if not ok then failure=problem end
    end
    if hooked then debug.sethook(hook,"l") end
    return function()
        if hooked then assert(debug.gethook()==hook);debug.sethook() end
        assert(not failure,failure);assert(next(pending)==nil)
        assert(not debug.gethook() and jit.status()==expectedJit)
        for _,m in ipairs(methods()) do assert(m[2][m[3]]==originals[m[1]]) end
        assert(upvalue(originals.perform,"doActorAttribsConditions")==originals.actor)
        finished=true
    end
end
local function item_id(item)
    if not item then return nil end
    local found;for id,v in pairs(build.itemsTab.items) do if v==item then assert(not found);found=id end end
    return found
end
local function item_snapshot(item)
    if not item then return {present=false} end
    local base=assert(item.base);local name=assert(item.baseName)
    local modifiers={}
    for _,mod in ipairs(item.baseModList or {}) do
        for _,tag in ipairs(mod) do if tag.type=="DisablesItem" then modifiers[#modifiers+1]=record(mod);break end end
    end
    return {present=true,source_item_id=present(item_id(item)),base_name=name,type=item.type,base_type=base.type,
        exact_catalogue_base=base==data.itemBases[name],raw=item.raw,disables_item=modifiers}
end
local function selection()
    return {items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,
        skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,
        use_second_weapon_set=build.itemsTab.activeItemSet.useSecondWeaponSet}
end
function M.observe(expectedJit)
    assert(finished and not debug.gethook() and jit.status()==expectedJit)
    local saved=selection();local inventory={}
    for id,item in pairs(build.itemsTab.items) do inventory[id]={item=item,raw=item.raw} end
    local function saved_slot(hand)
        local name=hand..(saved.use_second_weapon_set and " Swap" or "")
        local slot=assert(build.itemsTab.slots[name]);local entry=assert(build.itemsTab.activeItemSet[name])
        assert(slot.selItemId==entry.selItemId)
        return {name=name,selected_item_id=entry.selItemId,weapon_set=slot.weaponSet,
            active=present(entry.active),runtime_active=present(slot.active),item=item_snapshot(build.itemsTab.items[entry.selItemId])}
    end
    local savedOne,savedSlot=saved_slot("Weapon 1"),saved_slot("Weapon 2")
    local selectedItem=build.itemsTab.items[savedSlot.selected_item_id]
    local modes={}
    for _,mode in ipairs({"MAIN","CALCS"}) do
        local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        local actor=assert(env.player);local db=assert(actor.modDB);assert(env.modDB==db)
        local item=actor.itemList["Weapon 2"]
        local output=assert(actor.output);local beforeOutput=scalar_output(output);local before=ancestry(db)
        local effective={};for _,name in ipairs(names) do effective[name]=present(originals.condition(db,name,nil)) end
        assert(same(before,ancestry(db)) and same(beforeOutput,scalar_output(output)))
        local p
        if hooked then
            for i,c in ipairs(calls) do if c.env==env and c.actor==actor then
                assert(not p,"one original branch on the final Player environment")
                p={invocation=i,before=c.before,after=c.after,exact_actor=true,exact_store=c.db==db,
                    exact_prepared_item=c.item==item}
            end end
            assert(p and p.exact_store and p.exact_prepared_item)
            if selectedItem then
                local matches={}
                for i,c in ipairs(parsed) do if c.item==selectedItem and c.base==selectedItem.base then
                    assert(c.type==selectedItem.type and c.name==selectedItem.baseName)
                    matches[#matches+1]={index=i,base_name=c.name,type=c.type,exact_item=true,exact_base=true}
                end end
                assert(#matches>0,"saved selected item lacks original ParseRaw type assignment")
                p.selected_item_parses=matches
            end
        end
        modes[mode]={saved_main_hand=savedOne,saved_slot=savedSlot,prepared_item=item_snapshot(item),selected_to_prepared_same_object=selectedItem==item,
            saved_occupied=selectedItem~=nil,prepared_occupied=item~=nil,
            conditions=raw_conditions(db),condition_ancestry=before,effective_conditions=effective,
            player_output=beforeOutput,provenance=p}
        assert(actor.output==output and actor.modDB==db and actor.itemList["Weapon 2"]==item)
    end
    local catalogue={};for name,base in pairs(data.itemBases) do
        assert(type(name)=="string" and type(base.type)=="string")
        catalogue[#catalogue+1]={name=name,type=base.type,sub_type=base.subType}
    end
    table.sort(catalogue,function(a,b)return a.name<b.name end)
    local invocations={};for i,c in ipairs(calls) do invocations[i]={mode=c.mode,
        item=item_snapshot(c.item),before=c.before,after=c.after,
        exact_final_main=c.env==build.calcsTab.mainEnv,exact_final_calcs=c.env==build.calcsTab.calcsEnv} end
    assert(same(saved,selection()))
    for id,v in pairs(inventory) do assert(build.itemsTab.items[id]==v.item and v.item.raw==v.raw) end
    for _,m in ipairs(methods()) do assert(m[2][m[3]]==originals[m[1]]) end
    return {methods=where,selection=saved,catalogue=catalogue,modes=modes,invocations=invocations,hooked=hooked,
        evidence={original_branch=true,original_item_type_assignment=true,business_method_wrappers=false,
            scalar_outputs_preserved=true,full_output_graph=false,non_scalar_output_fields_excluded=true,
            original_methods_preserved=true,saved_items_preserved=true,saved_selection_preserved=true,
            condition_read_set_preserved=true,effective_condition_closure=false,
            item_filtering_or_substitution_native_admission=false,unarmed_or_unencumbered=false}}
end
return M
