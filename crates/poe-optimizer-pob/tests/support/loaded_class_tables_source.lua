-- Observe the original loaded tree and class initializer; no copied calculation.
local M = {}
local hooked, expectedJit, hook, finished, armed
local trees, initializers, originals, methods
local function plain(value, depth)
    if type(value) ~= "table" then
        assert(type(value) == "nil" or type(value) == "string" or type(value) == "number" or type(value) == "boolean")
        if type(value) == "number" and (value ~= value or value == math.huge or value == -math.huge) then return tostring(value) end
        return value
    end
    depth = (depth or 0) + 1
    assert(depth < 16 and getmetatable(value) == nil)
    local out, count = {}, 0
    for k, v in pairs(value) do
        count = count + 1; assert(count <= 256)
        assert(type(k) == "number" or type(k) == "string")
        out[k] = plain(v, depth)
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
local function original(f, path, first)
    assert(type(f) == "function")
    local i = debug.getinfo(f, "S")
    assert(i.what == "Lua" and i.source:gsub("\\", "/"):sub(-#path) == path)
    assert(i.linedefined == first)
    return {path=path, first=i.linedefined, last=i.lastlinedefined}
end
local function upvalue(f, name)
    for i = 1, 64 do
        local key, value = debug.getupvalue(f, i)
        if not key then break end
        if key == name then return value end
    end
    error("missing original upvalue " .. name)
end
local function constructor()
    local wrapper = common.classes.PassiveTree.PassiveTree
    original(wrapper, "Modules/Common.lua", 167)
    local implementation = upvalue(wrapper, "originalFunc")
    original(implementation, "Classes/PassiveTree.lua", 59)
    return implementation, wrapper
end
local function presence(value)
    return {present=value ~= nil, kind=type(value), value=plain(value)}
end
local function classRows(tree, raw)
    local rows = {}
    for id, class in pairs(tree.classes) do
        if raw then
            rows[#rows+1] = plain(class)
        else
            assert(id == class.integerId and class.classes == class.ascendancies)
            local fields, inventory, ascendancies = {}, {}, {}
            for k, v in pairs(class) do
                assert(type(k) == "string")
                inventory[#inventory+1] = k
                if k ~= "classes" and k ~= "ascendancies" then fields[k] = plain(v) end
            end
            for index, asc in pairs(class.ascendancies) do
                assert(type(index) == "number")
                ascendancies[#ascendancies+1] = {index=index, fields=plain(asc)}
            end
            table.sort(inventory)
            table.sort(ascendancies, function(a,b) return a.index < b.index end)
            rows[#rows+1] = {id=id, fields=fields, field_inventory=inventory,
                ascendancies=ascendancies, classes_alias_ascendancies=true}
        end
    end
    assert(#rows == 8)
    table.sort(rows, function(a,b) return (raw and a.integerId or a.id) < (raw and b.integerId or b.id) end)
    return rows
end
local function record(mod)
    local tags = {}
    for i, tag in ipairs(mod) do tags[i] = plain(tag) end
    return {name=mod.name, type=mod.type, value=plain(mod.value), source=mod.source,
        flags=mod.flags, keyword_flags=mod.keywordFlags, tags=tags}
end
local function treeEntry(tree)
    for _, row in ipairs(trees) do if row.tree == tree then return row end end
end
function M.start(enable, jitEnabled)
    assert(debug.gethook() == nil and jit.status() == jitEnabled)
    hooked, expectedJit, finished, armed = enable, jitEnabled, false, false
    trees, initializers, originals, methods = {}, {}, {}, {}
    hook = function(_, line)
        if line ~= 100 and line ~= 445 and line ~= 834 then return end
        local info = debug.getinfo(2, "fS")
        local path = info.source:gsub("\\", "/")
        local isTree = path:sub(-#"Classes/PassiveTree.lua") == "Classes/PassiveTree.lua" and info.linedefined == 59
        local isInitializer = armed and path:sub(-#"Modules/CalcSetup.lua") == "Modules/CalcSetup.lua" and info.func == originals.initializer
        if not isTree and not isInitializer then return end
        local locals = {}
        for i = 1, 64 do
            local name, value = debug.getlocal(2, i)
            if not name then break end
            if name == "self" or name == "treeVersion" or name == "env" or name == "modDB" or name == "classStats" or name == "cachedPlayerDB" then locals[name] = value end
        end
        if isTree then
            assert(info.func == constructor())
            local tree = assert(locals.self)
            if tree.treeVersion ~= "0_5" then return end
            if line == 100 then
                assert(not treeEntry(tree) and #trees < 8)
                trees[#trees+1] = {tree=tree, raw=classRows(tree, true),
                    character_data_at_load=presence(tree.characterData)}
            elseif line == 445 then
                local row = assert(treeEntry(tree), "original tree load must precede constructor completion")
                assert(not row.constructed)
                row.constructed = classRows(tree, false)
                row.character_data_at_return = presence(tree.characterData)
            end
        elseif line == 834 then
            local env, db, selected = assert(locals.env), assert(locals.modDB), assert(locals.classStats)
            assert(not locals.cachedPlayerDB and env.player.modDB == db and env.modDB == db)
            local tree = env.spec.tree
            assert(tree.treeVersion == "0_5" and treeEntry(tree))
            assert(#initializers < 64)
            local records, objects = {}, {}
            for _, stat in ipairs({"Str", "Dex", "Int"}) do
                local list = assert(db.mods[stat])
                assert(#list == 1)
                local mod = list[1]
                assert(mod.name == stat and mod.type == "BASE" and mod.source == "Base")
                records[stat], objects[stat] = record(mod), mod
            end
            local alternate = tree.characterData
            initializers[#initializers+1] = {tree=tree, store=db, objects=objects,
                row={mode=env.mode, class_id=env.classId, character_data=presence(alternate),
                    selected_is_classes_row=selected == tree.classes[env.classId],
                    selected_is_character_data_row=type(alternate) == "table" and selected == alternate[env.classId] or false,
                    selected_bases={base_str=selected.base_str, base_dex=selected.base_dex, base_int=selected.base_int},
                    records=records}}
        end
    end
    if enable then debug.sethook(hook, "l") end
end
function M.before_build()
    assert(jit.status() == expectedJit)
    local implementation, wrapper = constructor()
    originals.constructor, originals.wrapper = implementation, wrapper
    methods.constructor = original(implementation, "Classes/PassiveTree.lua", 59)
    methods.constructor_wrapper = original(wrapper, "Modules/Common.lua", 167)
    originals.initializer = require("Modules.CalcBase").initEnv
    methods.initializer = original(originals.initializer, "Modules/CalcSetup.lua", 717)
    originals.new_mod = common.classes.ModStore.NewMod
    methods.new_mod = original(originals.new_mod, "Classes/ModStore.lua", 142)
    originals.add_mod = common.classes.ModDB.AddMod
    methods.add_mod = original(originals.add_mod, "Classes/ModDB.lua", 31)
    originals.cache_copy = specCopy
    methods.cache_copy = original(specCopy, "Modules/Common.lua", 542)
    armed = true
    return function()
        if hooked then assert(debug.gethook() == hook); debug.sethook() end
        assert(debug.gethook() == nil and jit.status() == expectedJit)
        assert(constructor() == originals.constructor and common.classes.PassiveTree.PassiveTree == originals.wrapper)
        assert(require("Modules.CalcBase").initEnv == originals.initializer)
        assert(common.classes.ModStore.NewMod == originals.new_mod and common.classes.ModDB.AddMod == originals.add_mod)
        assert(specCopy == originals.cache_copy)
        finished = true
    end
end
local function scalarOutput(output)
    local scalars, nonScalars = {}, {}
    for k, v in pairs(output) do
        assert(type(k) == "string")
        local t = type(v)
        if t == "number" or t == "boolean" or t == "string" then scalars[k] = plain(v)
        else nonScalars[#nonScalars+1] = {key=k, value_type=t} end
    end
    table.sort(nonScalars, function(a,b) return a.key < b.key end)
    return {scalars=scalars, non_scalar_fields=nonScalars}
end
local function selection()
    return {items=build.itemsTab.activeItemSetId, spec=build.treeTab.activeSpec,
        skills=build.skillsTab.activeSkillSetId, config=build.configTab.activeConfigSetId,
        group=build.mainSocketGroup, character_level=build.characterLevel}
end
function M.observe()
    assert(finished and debug.gethook() == nil and jit.status() == expectedJit)
    local tree = assert(build.spec.tree)
    assert(tree.treeVersion == "0_5")
    local currentClasses, selected, items = classRows(tree, false), selection(), {}
    for id, item in pairs(build.itemsTab.items) do items[id] = {item=item, raw=item.raw} end
    local capturedTree = treeEntry(tree)
    if hooked then
        assert(capturedTree and equal(capturedTree.constructed, currentClasses))
        assert(equal(capturedTree.character_data_at_return, presence(tree.characterData)))
    else assert(#trees == 0 and #initializers == 0) end
    local modes, outputs = {}, {}
    for _, mode in ipairs({"MAIN", "CALCS"}) do
        local env = assert(mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
        assert(env.spec.tree == tree and env.player.modDB == env.modDB)
        local current, seen, depth, found = env.player.modDB, {}, 0, {}
        while current do
            assert(not seen[current]); seen[current] = true
            for _, stat in ipairs({"Str", "Dex", "Int"}) do
                for _, mod in ipairs(current.mods[stat] or {}) do
                    if mod.type == "BASE" and mod.source == "Base" then
                        assert(not found[stat], "exactly one original class Base record per attribute")
                        found[stat] = {object=mod, store=current, depth=depth}
                    end
                end
            end
            current, depth = current.parent, depth + 1
        end
        local records, depths, provenance = {}, {}, {}
        for _, stat in ipairs({"Str", "Dex", "Int"}) do
            local actual = assert(found[stat])
            records[stat], depths[stat] = record(actual.object), actual.depth
            if hooked then
                local creation
                for _, row in ipairs(initializers) do
                    if row.objects[stat] == actual.object then assert(not creation); creation = row end
                end
                assert(creation and creation.tree == tree and creation.row.class_id == env.classId)
                assert(equal(creation.row.records[stat], records[stat]))
                provenance[stat] = {exact_initialized_record=true, initializer_store_is_owner=creation.store == actual.store,
                    selected_is_classes_row=creation.row.selected_is_classes_row,
                    selected_is_character_data_row=creation.row.selected_is_character_data_row,
                    selected_bases=creation.row.selected_bases}
            end
        end
        outputs[mode] = env.player.output
        modes[mode] = {class_id=env.classId, class_name=env.spec.curClassName,
            character_level=env.player.level, records=records, ancestor_depths=depths,
            output=scalarOutput(outputs[mode]), provenance=hooked and provenance or nil}
    end
    local constructorRows, initializerRows = {}, {}
    for _, row in ipairs(trees) do
        constructorRows[#constructorRows+1] = {tree_version=row.tree.treeVersion,
            raw_classes=row.raw, constructed_classes=row.constructed,
            character_data_at_load=row.character_data_at_load,
            character_data_at_return=row.character_data_at_return, exact_selected_tree=row.tree == tree}
    end
    for _, row in ipairs(initializers) do initializerRows[#initializerRows+1] = row.row end
    assert(equal(currentClasses, classRows(tree, false)) and equal(selected, selection()))
    for id, saved in pairs(items) do assert(build.itemsTab.items[id] == saved.item and saved.item.raw == saved.raw) end
    for mode, output in pairs(outputs) do
        local env = mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
        assert(env.player.output == output and equal(modes[mode].output, scalarOutput(output)))
    end
    return {hooked=hooked, methods=methods, selected=selected, tree_version=tree.treeVersion,
        classes=currentClasses, character_data=presence(tree.characterData), modes=modes,
        constructors=constructorRows, initializers=initializerRows,
        evidence={original_constructor=true, original_initializer=true, original_methods_preserved=true,
            exact_selected_tree=hooked, exact_selected_class_row=hooked,
            scalar_outputs_preserved=true, saved_items_preserved=true, saved_selection_preserved=true,
            business_method_wrappers=false, copied_initializer=false, full_output_graph=false,
            class_coverage_closed=false, shared_actor_state_closed=false, complete_build_claim=false}}
end
return M
