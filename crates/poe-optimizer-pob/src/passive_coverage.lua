-- Observe the already-evaluated live PassiveSpec. Never allocate, replace, or
-- reconstruct the requested tree, and never consult our extracted snapshot.
local spec = assert(build.spec, "No live passive spec")
local tree = assert(spec.tree, "No live passive tree")
local function integer(value, label)
    assert(type(value) == "number" and value >= 0 and value % 1 == 0 and value <= 4294967295, "Invalid passive " .. label)
    return value
end
local function text(value, label)
    assert(type(value) == "string", "Invalid passive " .. label)
    return value
end
local function optional_text(value)
    return type(value) == "string" and value ~= "" and value or nil
end
local function ascendancy(index, descriptor)
    if not index or index == 0 then return nil end
    assert(type(descriptor) == "table", "No live selected ascendancy descriptor")
    return {
        index = integer(index, "ascendancy index"),
        internal_id = optional_text(descriptor.internalId),
        catalog_id = optional_text(descriptor.id),
        name = text(descriptor.name, "ascendancy name"),
        start_node_id = descriptor.startNodeId and integer(descriptor.startNodeId, "ascendancy root"),
    }
end
local class = assert(spec.curClass, "No live selected class")
local selected_class = {
    index = integer(spec.curClassId, "class index"),
    internal_id = integer(class.integerId, "class internal identity"),
    name = text(spec.curClassName, "class name"),
    start_node_id = integer(class.startNodeId, "class root"),
}
local primary = ascendancy(spec.curAscendClassId, spec.curAscendClass)
local secondary = ascendancy(spec.curSecondaryAscendClassId, spec.curSecondaryAscendClass)
-- The reverse map was populated by BuildAllDependsAndPaths at the actual
-- automatic replacement branch. Table identity links evidence to this live node.
local selected_sources = {}
for source_id, node in pairs(spec.switchableNodes or {}) do
    assert(not selected_sources[node], "Ambiguous live passive switch provenance")
    selected_sources[node] = integer(source_id, "selected switch source")
end
local function switch_evidence(node)
    local selected_id = selected_sources[node]
    if not selected_id then return nil end
    local base = tree.nodes[node.id]
    local selected, kind, selector
    if base then
        -- Class-before-ascendancy precedence is checked against the observed map.
        if base.options and base.options[spec.curClassName] and base.options[spec.curClassName].id == selected_id then
            selected, kind, selector = base.options[spec.curClassName], "class", spec.curClassName
        elseif base.options and base.options[spec.curAscendClassName] and base.options[spec.curAscendClassName].id == selected_id then
            selected, kind, selector = base.options[spec.curAscendClassName], "ascendancy", spec.curAscendClassName
        elseif base.id == selected_id then
            selected, kind = base, "base"
        end
    end
    return {
        selected_source_id = selected_id,
        kind = kind or "unclassified",
        selector = selector,
        stats_reference_matches_selected_source = selected ~= nil and node.sd == selected.sd,
        display_name_matches_selected_source = selected ~= nil and node.dn == selected.dn,
    }
end
local nodes = {}
for physical_id, node in pairs(spec.allocNodes) do
    assert(node == spec.nodes[physical_id] and node.id == physical_id and node.alloc, "Inconsistent live passive allocation table")
    local roles = {}
    if physical_id == selected_class.start_node_id then table.insert(roles, "class") end
    if primary and physical_id == primary.start_node_id then table.insert(roles, "ascendancy") end
    if secondary and physical_id == secondary.start_node_id then table.insert(roles, "secondary_ascendancy") end
    local stats = {}
    for i, line in ipairs(node.sd or {}) do stats[i] = text(line, "stat line") end
    assert(node.isFreeAllocate == nil or type(node.isFreeAllocate) == "boolean", "Unexpected live free allocation value")
    table.insert(nodes, {
        physical_node_id = integer(physical_id, "physical node ID"),
        node_type = text(node.type, "node type"),
        name = text(node.name, "inherited name"),
        display_name = text(node.dn, "display name"),
        stats = stats,
        allocation_mode = integer(node.allocMode or 0, "allocation mode"),
        implicit_roots = roles,
        free_allocation = node.isFreeAllocate,
        ascendancy_name = optional_text(node.ascendancyName),
        is_multiple_choice_option = not not node.isMultipleChoiceOption,
        is_granted_passive = not not node.isGrantedPassive,
        is_attribute = not not node.isAttribute,
        is_conquered = not not node.conqueredBy,
        has_hash_override = spec.hashOverrides and spec.hashOverrides[physical_id] ~= nil or false,
        is_switchable = not not node.isSwitchable,
        switch = switch_evidence(node),
    })
end
table.sort(nodes, function(a, b) return a.physical_node_id < b.physical_node_id end)
local ordinary, asc, secondary_asc, sockets, weapon1, weapon2 = spec:CountAllocNodes()
return {
    schema_version = 1,
    tree_version = text(spec.treeVersion, "tree version"),
    class = selected_class,
    ascendancy = primary,
    secondary_ascendancy = secondary,
    allocation_counts = {
        ordinary = integer(ordinary, "ordinary count"),
        ascendancy = integer(asc, "ascendancy count"),
        secondary_ascendancy = integer(secondary_asc, "secondary ascendancy count"),
        sockets = integer(sockets, "socket count"),
        weapon_set_1 = integer(weapon1, "weapon set 1 count"),
        weapon_set_2 = integer(weapon2, "weapon set 2 count"),
    },
    allocated_nodes = nodes,
}
