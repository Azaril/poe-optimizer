-- Bounded original class-start construction witness; no copied parser or native rule.
return function(execute, expectedJit)
    assert(debug.gethook() == nil and jit.status() == expectedJit)
    local function original(f, path, first)
        local i = debug.getinfo(f, "S")
        assert(i.what == "Lua" and i.source:gsub("\\", "/"):sub(-#path) == path)
        assert(i.linedefined == first)
        return {path=path, first=i.linedefined, last=i.lastlinedefined}
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
            count = count + 1; assert(count < 4096)
            assert(type(k) == "number" or type(k) == "string")
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
    local class = common.classes.PassiveTree
    local wrapper, processor = class.PassiveTree, class.ProcessStats
    local implementation = upvalue(wrapper, "originalFunc")
    local methods = {
        constructor_wrapper=original(wrapper, "Modules/Common.lua", 167),
        constructor=original(implementation, "Classes/PassiveTree.lua", 59),
        process_stats=original(processor, "Classes/PassiveTree.lua", 448),
        process_node=original(class.ProcessNode, "Classes/PassiveTree.lua", 528),
        select_class=original(common.classes.PassiveSpec.SelectClass, "Classes/PassiveSpec.lua", 655),
        reconnect=original(common.classes.PassiveSpec.ReconnectNodeToClassStart, "Classes/PassiveSpec.lua", 2067),
        build_node_mods=original(require("Modules.CalcBase").buildModListForNode, "Modules/CalcSetup.lua", 200),
    }
    local rawTree = LoadModule("TreeData/0_5/tree")
    local raw = plain(assert(rawTree.nodes[54447]))
    assert(raw.skill == 54447 and #raw.stats == 0)
    assert(raw.classesStart[1] == "Witch" and raw.classesStart[2] == "Sorceress" and #raw.classesStart == 2)
    local spec = build.spec
    local tree = spec.tree
    assert(tree.treeVersion == "0_5" and spec.curClassId == 7)
    local root, selectedRoot = assert(tree.nodes[54447]), assert(spec.nodes[54447])
    assert(getmetatable(selectedRoot) == root and root.__index == root)
    assert(selectedRoot.alloc and spec.allocNodes[54447] == selectedRoot)
    assert(selectedRoot.modList == root.modList)
    local function modList(list)
        assert(getmetatable(list) == common.classes.ModList)
        local out = {records={}, fields={}, own_keys={}}
        for k, v in pairs(list) do
            if type(k) == "number" then
                assert(k >= 1 and k <= #list and math.floor(k) == k)
                out.records[k] = plain(v)
            else
                assert(type(k) == "string")
                out.own_keys[#out.own_keys+1] = {key=k, value_type=type(v)}
                if k == "Object" then
                    assert(v == list)
                elseif k == "ModStore" then
                    assert(type(v) == "table" and getmetatable(v) == v)
                    assert(v._object == list and v._parent == common.classes.ModStore and v._className == "ModList")
                elseif k == "_parentInit" then
                    assert(type(v) == "table" and v[common.classes.ModStore] == true)
                    for parent, initialized in pairs(v) do
                        assert(parent == common.classes.ModStore and initialized == true)
                    end
                else
                    assert(k == "parent" or k == "actor" or k == "multipliers" or k == "conditions")
                    out.fields[k] = plain(v)
                end
            end
        end
        table.sort(out.own_keys, function(a,b) return a.key < b.key end)
        assert(out.fields.parent == false and #out.records == #list)
        out.metatable = "ModList"
        out.parent_constructor_verified = true
        return out
    end
    local declarationNames = {
        "isJewelSocket", "containJewelSocket", "charmSocket", "expansionJewel", "noRadius",
        "isAttribute", "isSwitchable", "options", "isMastery", "masteryEffects",
        "isKeystone", "keystoneMod", "isAscendancyStart", "ascendancyName", "isProxy",
        "isOnlyImage", "recipe", "unknown", "extra", "grantedSkill", "grantedSkills",
        "grantedPassive", "grantedPassives", "socket", "sockets", "skillId",
    }
    local function nodeProjection(node, ownerTree)
        assert(node.type == "ClassStart" and node.id == 54447 and node.sd == node.stats)
        local fields, inventory, excluded = {}, {}, {}
        for k, v in pairs(node) do
            assert(type(k) == "string")
            inventory[#inventory+1] = {key=k, value_type=type(v)}
            if k == "__index" then
                assert(v == node); excluded[#excluded+1] = {key=k, reason="self_identity_verified"}
            elseif k == "modList" then
                assert(v == node.modList)
            elseif k == "group" then
                assert(v == ownerTree.groups[raw.group])
                -- Geometry is a constructor-owned object, not an intrinsic declaration.
                excluded[#excluded+1] = {key=k, reason="geometry_group", source_group=raw.group}
            elseif k == "overlay" or k == "targetSize" then
                excluded[#excluded+1] = {key=k, reason="render_geometry"}
            else
                fields[k] = plain(v)
            end
        end
        table.sort(inventory, function(a,b) return a.key < b.key end)
        table.sort(excluded, function(a,b) return a.key < b.key end)
        local declarations = {}
        for _, name in ipairs(declarationNames) do
            local v = node[name]
            declarations[#declarations+1] = {name=name, present=v ~= nil, value=plain(v)}
        end
        return {id=node.id, type=node.type, default_mod_count=#node.modList,
            fields=fields, field_inventory=inventory, excluded=excluded,
            declaration_fields=declarations, default_modifiers=modList(node.modList)}
    end
    local before = nodeProjection(root, tree)
    local classes = {}
    for _, id in ipairs({1, 7}) do
        local c = assert(tree.classes[id])
        assert(c.startNodeId == 54447 and tree.nodes[c.startNodeId] == root)
        assert(tree.classStartNodeNameMap[c.name] == 54447)
        classes[#classes+1] = {class_id=id, name=c.name, start_node_id=c.startNodeId,
            same_root=true, base_str=c.base_str, base_dex=c.base_dex, base_int=c.base_int}
    end
    local neighbors = {}
    for _, id in ipairs(root.linkedId) do
        local node = assert(tree.nodes[id])
        local flags = {}
        for _, mod in ipairs(node.modList) do
            if mod.name == "Condition:ConnectedToWitchStart" or mod.name == "Condition:ConnectedToSorceressStart" then
                flags[#flags+1] = plain(mod)
            end
        end
        table.sort(flags, function(a,b) return a.name < b.name end)
        neighbors[#neighbors+1] = {container_node_id=id, node_type=node.type, connection_flags=flags,
            distinct_from_root=node ~= root and node.modList ~= root.modList}
    end
    table.sort(neighbors, function(a,b) return a.container_node_id < b.container_node_id end)
    local function selection()
        return {items=build.itemsTab.activeItemSetId, spec=build.treeTab.activeSpec,
            skills=build.skillsTab.activeSkillSetId, config=build.configTab.activeConfigSetId,
            group=build.mainSocketGroup}
    end
    local selected = selection()
    local outputs, snapshots = {}, {}
    for _, mode in ipairs({"MAIN", "CALCS"}) do
        local env = mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
        assert(env.spec == spec and env.allocNodes[54447] == selectedRoot)
        outputs[mode] = env.player.output
        local scalars = {}
        for k,v in pairs(outputs[mode]) do
            if type(v) == "number" or type(v) == "boolean" or type(v) == "string" then scalars[k] = v end
        end
        snapshots[mode] = scalars
    end
    local probe
    if execute then
        local freshTree = new("PassiveTree"):PassiveTree("0_5")
        local freshRoot = assert(freshTree.nodes[54447])
        assert(freshRoot ~= root and freshRoot.modList ~= root.modList)
        probe = nodeProjection(freshRoot, freshTree)
        assert(equal(probe, before))
        for _, id in ipairs({1,7}) do assert(freshTree.classes[id].startNodeId == 54447) end
    end
    assert(equal(before, nodeProjection(root, tree)) and equal(selected, selection()))
    for mode, output in pairs(outputs) do
        local env = mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
        assert(env.player.output == output and env.spec == spec)
        local after = {}
        for k,v in pairs(output) do
            if type(v) == "number" or type(v) == "boolean" or type(v) == "string" then after[k] = v end
        end
        assert(equal(snapshots[mode], after))
    end
    assert(class.PassiveTree == wrapper and class.ProcessStats == processor)
    assert(upvalue(class.PassiveTree, "originalFunc") == implementation and jit.status() == expectedJit)
    return {executed=execute, raw=raw, constructed=before, constructor_probe=probe,
        classes=classes, neighbor_connection_flags=neighbors, methods=methods, selected=selected,
        selected_root={source_id=54447, class_id=7, tree_prototype_identity=true,
            allocated_object_identity=true, default_modifier_object_identity=true},
        player_scalar_outputs=snapshots,
        evidence={original_constructor=true, original_process_stats=true, original_methods_preserved=true,
            exact_raw_descriptor=true, complete_intrinsic_modifier_fields=true,
            selected_default_root_unchanged=true, cached_scalar_outputs_preserved=true,
            both_class_roots_identical=true, saved_selection_preserved=true,
            class_owner_closed=false, universal_player_initialization_closed=false,
            external_transformations_closed=false, all_same_source_labels_owned_by_root=false,
            complete_build_claim=false}}
end
