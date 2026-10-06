-- Test-only calls to the bound original method; no copied placement algorithm.
return function(bound, execute, expectedJit)
    assert(debug.gethook() == nil and jit.status() == expectedJit)
    local function original(f, path, first, last)
        local i = debug.getinfo(f, "S")
        assert(i.what == "Lua" and i.source:gsub("\\", "/"):sub(-#path) == path)
        assert(i.linedefined == first and (not last or i.lastlinedefined == last))
        return {path = path, first = i.linedefined, last = i.lastlinedefined}
    end
    local function upvalue(f, name)
        for n = 1, 100 do
            local k, v = debug.getupvalue(f, n)
            if not k then break end
            if k == name then return v end
        end
        error("missing original upvalue " .. name)
    end
    local function plain(v, depth)
        if type(v) ~= "table" then
            assert(type(v) == "nil" or type(v) == "number" or type(v) == "string" or type(v) == "boolean")
            return v
        end
        depth = (depth or 0) + 1
        assert(depth < 20 and getmetatable(v) == nil)
        local out, count = {}, 0
        for k, x in pairs(v) do
            count = count + 1; assert(count < 65536)
            out[k] = plain(x, depth)
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
    local function outcome(...)
        local n = select("#", ...)
        assert(n <= 1)
        local v = ...
        assert(type(v) == "nil" or type(v) == "boolean")
        return {return_count = n, kind = n == 0 and "none" or type(v), value = v}
    end
    local tab, method = bound.receiver, bound.target
    local class = common.classes.ItemsTab
    local ctor, loader = class.ItemsTab, class.Load
    local constructorWrapperInfo = original(ctor, "Modules/Common.lua", 167, 183)
    local implementation = upvalue(ctor, "originalFunc")
    local methodInfo = original(method, "Classes/ItemsTab.lua", 2603, 2687)
    local constructorInfo = original(implementation, "Classes/ItemsTab.lua", 140)
    local loadInfo = original(loader, "Classes/ItemsTab.lua", 1193, 1320)
    local baseSlots = upvalue(implementation, "baseSlots")
    local originalBaseSlots = plain(baseSlots)
    local slots, setIds = plain(bound.slots), plain(bound.set_ids)
    table.sort(slots); table.sort(setIds)
    local slotSet = {}
    for _, name in ipairs(slots) do
        assert(not slotSet[name] and tab.slots[name]); slotSet[name] = true
    end
    for _, name in ipairs(baseSlots) do assert(slotSet[name]) end
    local item = assert(bound.items[22])
    assert(item.id == 22 and item.baseName == "Cryptic Leggings" and item.type == "Boots")
    assert(item.base == data.itemBases["Cryptic Leggings"])
    local snapshot = {id = item.id, base_name = item.baseName, item_type = item.type,
        rarity = item.rarity, raw = item.raw, base = plain(item.base),
        sockets = plain(item.sockets), runes = plain(item.runes)}
    local savedItems, raw = {}, {}
    for id, entry in pairs(bound.items) do savedItems[id] = entry; raw[id] = entry.raw end
    local env, output = build.calcsTab.mainEnv, build.calcsTab.mainOutput
    local function scalarOutput()
        local out = {}
        for k, v in pairs(output) do
            if type(v) == "number" or type(v) == "boolean" or type(v) == "string" then out[k] = v end
        end
        return out
    end
    local outputBefore = scalarOutput()
    local function selection()
        return {items = tab.activeItemSetId, spec = build.treeTab.activeSpec,
            skills = build.skillsTab.activeSkillSetId, config = build.configTab.activeConfigSetId,
            group = build.mainSocketGroup}
    end
    local selected = selection()
    local uses = {}
    for _, id in ipairs(setIds) do
        for name, entry in pairs(bound.sets[id]) do
            if type(entry) == "table" and entry.selItemId == item.id then
                assert(slotSet[name]); uses[#uses + 1] = {set = id, slot = name}
            end
        end
    end
    table.sort(uses, function(a,b) return a.set == b.set and a.slot < b.slot or a.set < b.set end)
    assert(env.player.itemList.Boots == item and build.calcsTab.calcsEnv.player.itemList.Boots == item)
    local flag = env.modDB.Flag
    local flagInfo = debug.getinfo(flag, "S")
    assert(flagInfo.what == "Lua" and flagInfo.source:gsub("\\", "/"):sub(-#"Classes/ModStore.lua") == "Classes/ModStore.lua")
    local actualFlags = {}
    for _, name in ipairs({"GiantsBlood", "InstrumentsOfPower", "LordOfTheWilds"}) do
        actualFlags[name] = outcome(flag(env.modDB, nil, name))
    end
    local flagCases = {{id = "actual"}}
    for mask = 0, 7 do
        flagCases[#flagCases + 1] = {id = "mask-" .. mask, values = {
            giantsBlood = mask % 2 == 1,
            instrumentsOfPower = math.floor(mask / 2) % 2 == 1,
            lordOfTheWilds = math.floor(mask / 4) % 2 == 1}}
    end
    local contexts = {{id = "active-default"}}
    for _, id in ipairs(setIds) do contexts[#contexts + 1] = {id = "saved-" .. id, set = bound.sets[id]} end
    local before = bound.projection()
    local calls = {}
    if execute then
        assert(#contexts * #slots * #flagCases < 10000)
        for _, context in ipairs(contexts) do
            for _, name in ipairs(slots) do
                for _, flags in ipairs(flagCases) do
                    local value = outcome(method(tab, item, name, context.set, flags.values))
                    assert(equal(value, outcome(method(tab, item, name, context.set, flags.values))))
                    -- Rejection may be false, nil or no return for different
                    -- weapon partners/flags. Retain each raw pack independently.
                    assert((value.kind == "boolean" and value.value == true) == (name == "Boots"))
                    calls[#calls + 1] = {set = context.id, slot = name, flags = flags.id, outcome = value}
                end
            end
        end
    end
    -- The original parser accepts numeric suffixes. An unregistered string is
    -- not an equipment destination and cannot extend the checked slot catalogue.
    local boundary
    if execute then
        assert(not slotSet["Boots 1"])
        boundary = {slot = "Boots 1", registered = false,
            outcome = outcome(method(tab, item, "Boots 1", nil, nil)), native_admission = false}
    end
    bound.verify()
    assert(class.ItemsTab == ctor and class.Load == loader and class.IsItemValidForSlot == method)
    assert(upvalue(ctor, "originalFunc") == implementation and upvalue(implementation, "baseSlots") == baseSlots)
    assert(equal(baseSlots, originalBaseSlots) and env.modDB.Flag == flag)
    assert(build.calcsTab.mainEnv == env and build.calcsTab.mainOutput == output)
    assert(equal(outputBefore, scalarOutput()) and equal(selected, selection()))
    for id, entry in pairs(savedItems) do assert(bound.items[id] == entry and entry.raw == raw[id]) end
    for id, entry in pairs(bound.items) do assert(savedItems[id] == entry) end
    assert(item.base == data.itemBases["Cryptic Leggings"] and equal(item.base, snapshot.base))
    assert(equal(item.sockets, snapshot.sockets) and equal(item.runes, snapshot.runes))
    assert(debug.gethook() == nil and jit.status() == expectedJit)
    return {method = methodInfo, constructor = constructorInfo,
        constructor_wrapper = constructorWrapperInfo, loader = loadInfo,
        context_before = before, context_after = bound.projection(),
        registered_slots = slots, base_slots = originalBaseSlots, set_ids = setIds,
        flag_cases = flagCases, actual_flags = actualFlags, item = snapshot,
        selected = selected, selected_uses = uses, calls = calls, boundary = boundary,
        main_output = outputBefore, executed = execute,
        evidence = {exact_catalogue_base = true, exact_registered_item = true,
            exact_selected_main_and_calcs_item = true, original_functions_preserved = true,
            saved_items_preserved = true, saved_selections_preserved = true,
            main_output_preserved = true, repeated_calls_equal = true,
            call_hook = false, method_wrappers = false, native_owner_coverage = false,
            numerical_parity = false, socket_configuration_admission = false}}
end
