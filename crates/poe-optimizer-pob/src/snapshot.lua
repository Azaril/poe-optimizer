local function actor_output(actor)
    if not actor then return nil end
    local metrics, non_finite, non_finite_values = {}, {}, {}
    local function capture(key, value)
        if type(key) == "string" and type(value) == "number" then
            if value == value and value ~= math.huge and value ~= -math.huge then
                metrics[key] = value
            else
                table.insert(non_finite, key)
                non_finite_values[key] = value ~= value and "not_a_number" or (value > 0 and "positive_infinity" or "negative_infinity")
            end
        end
    end
    for key, value in pairs(actor.output or {}) do capture(key, value) end
    -- Attack timing is calculated per hand. Preserve its actual source path;
    -- do not substitute aggregate Speed for the pre-cap hand CastRate.
    for _, hand in ipairs({ "MainHand", "OffHand" }) do
        local output = actor.output and actor.output[hand]
        if type(output) == "table" then
            for _, field in ipairs({ "CastRate", "Speed", "Time" }) do
                capture(hand .. "." .. field, output[field])
            end
        end
    end
    table.sort(non_finite)
    local effect = actor.mainSkill and actor.mainSkill.activeEffect
    local granted = effect and effect.grantedEffect
    local flags = effect and effect.statSet and effect.statSet.skillFlags
    return {
        skill_name = granted and granted.name or nil,
        skill_id = granted and granted.id or nil,
        has_hit_damage = flags and flags.hit == true and not flags.disable or false,
        metrics = metrics,
        non_finite_metrics = non_finite,
        non_finite_values = non_finite_values,
    }
end

local env = assert(build.calcsTab.mainEnv, "No MAIN calculation environment")
assert(build.calcsTab.mainOutput == env.player.output, "Stale or mismatched MAIN output")
local nodes = {}
for id in pairs(build.spec.allocNodes) do table.insert(nodes, id) end
table.sort(nodes)
local class = build.spec.tree.classes[build.spec.curClassId]
local ascendancy = class.classes[build.spec.curAscendClassId]
local warnings = {
    "Experimental raw MAIN outputs; metric semantics, build legality and mechanic coverage are not certified.",
    "PoB may normalize imported data during export; this is not a lossless replacement for the source XML.",
}
local included = 0
for group_index, group in ipairs(build.skillsTab.socketGroupList) do
    if group.includeInFullDPS then included = included + 1 end
    for gem_index, gem in ipairs(group.gemList) do
        if not (gem.gemData or gem.grantedEffect) then
            table.insert(warnings, string.format("Unresolved skill in group %d, gem %d: %s",
                group_index, gem_index, gem.nameSpec or "(unnamed)"))
        end
    end
end
if included == 0 then
    table.insert(warnings, "No skill groups are included in Full DPS; that roll-up is not a build damage objective.")
end
return {
    build = {
        level = build.characterLevel,
        class_name = class.name,
        ascendancy_name = ascendancy and ascendancy.name or "None",
        tree_version = build.spec.treeVersion,
        main_socket_group = build.mainSocketGroup,
        allocated_nodes = nodes,
        skill_groups = #build.skillsTab.socketGroupList,
    },
    player = actor_output(env.player),
    minion = actor_output(env.minion),
    warnings = warnings,
    export_xml = assert(build:SaveDB("optimizer-export.xml"), "PoB export failed"),
}
