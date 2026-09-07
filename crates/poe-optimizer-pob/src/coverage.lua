-- Read-only coverage of the fresh MAIN environment. Never repair unresolved input.
local env = assert(build.calcsTab.mainEnv, "No MAIN calculation environment")
assert(build.calcsTab.mainOutput == env.player.output, "Stale or mismatched MAIN output")
local groups = build.skillsTab.socketGroupList
local function text(value)
    return type(value) == "string" and value or nil
end
local function finite(value)
    if type(value) == "number" and value == value and value ~= math.huge and value ~= -math.huge then
        return value
    end
end
local function index(value)
    value = finite(value)
    return value and value >= 1 and value % 1 == 0 and value or nil
end
local function identifier(value)
    value = finite(value)
    return value and value >= 0 and value % 1 == 0 and value or nil
end
local group_indices, gem_indices = {}, {}
for group_index, group in ipairs(groups) do
    group_indices[group] = group_index
    for gem_index, gem in ipairs(group.gemList) do
        gem_indices[gem] = { group = group_index, gem = gem_index }
    end
end
local function ownership(skill)
    local owner = skill.summonSkill or skill
    local source = owner.activeEffect and owner.activeEffect.srcInstance
    local position = source and gem_indices[source]
    return group_indices[owner.socketGroup] or (position and position.group), position and position.gem
end
local function selection(actor, kind)
    local skill = actor and actor.mainSkill
    if not skill then return nil end
    local effect = skill.activeEffect
    local granted = effect and effect.grantedEffect
    local stat_set = effect and effect.statSet
    local group_index, gem_index = ownership(skill)
    local actor_index
    for i, candidate in ipairs(actor.activeSkillList or {}) do
        if candidate == skill then actor_index = i break end
    end
    return {
        actor = kind,
        skill_id = granted and text(granted.id),
        skill_name = granted and text(granted.name),
        group_index = group_index,
        gem_index = gem_index,
        actor_skill_index = actor_index,
        minion_id = text(actor.type),
        part_index = index(skill.skillPart),
        part_name = text(skill.skillPartName),
        stat_set_index = stat_set and index(stat_set.index),
        stat_set_label = stat_set and stat_set.statSet and text(stat_set.statSet.label),
        -- Final display mode: cooldown/trigger processing can override catalog defaults.
        show_average = not not (skill.skillData and skill.skillData.showAverage),
        synthesized_default_attack = kind == "player" and not skill.socketGroup
            and granted and granted.id == "MeleeUnarmedPlayer" or false,
    }
end
local selected_player = selection(env.player, "player")
local selected_minion = selection(env.minion, "minion")
local parents = {}
for _, skill in ipairs(env.player.activeSkillList or {}) do
    local group_index = ownership(skill)
    if skill.minion and group_index then
        local id = skill.minion.type
        parents[id] = parents[id] or {}
        parents[id][group_index] = true
    end
end
local function parent_groups(minion_id)
    local result = {}
    for group_index in pairs(parents[minion_id] or {}) do table.insert(result, group_index) end
    table.sort(result)
    return result
end
-- Catalog name matches are hints, not import resolution or a supported-mechanics claim.
local action_names = {}
for minion_id, minion in pairs(data.minions) do
    for action_index, skill_id in ipairs(minion.skillList or {}) do
        local effect = data.skills[skill_id]
        if effect and effect.name then
            action_names[effect.name] = action_names[effect.name] or {}
            table.insert(action_names[effect.name], {
                kind = "minion_action",
                minion_id = minion_id,
                minion_name = minion.name or minion_id,
                skill_id = skill_id,
                catalog_action_index = action_index,
                active_parent_groups = parent_groups(minion_id),
                currently_selected_action = selected_minion ~= nil
                    and selected_minion.minion_id == minion_id and selected_minion.skill_id == skill_id,
            })
        end
    end
end
local function sort_candidates(candidates)
    table.sort(candidates, function(a, b)
        if a.minion_id ~= b.minion_id then return a.minion_id < b.minion_id end
        if a.skill_id ~= b.skill_id then return (a.skill_id or "") < (b.skill_id or "") end
        return (a.catalog_action_index or 0) < (b.catalog_action_index or 0)
    end)
end
for _, candidates in pairs(action_names) do sort_candidates(candidates) end
local function resolution_hints(name)
    if not name then return "no_catalog_name_match", {} end
    local spectre_name = name:match("^Spectre:%s*(.+)$")
    if spectre_name then
        local candidates = {}
        for minion_id, minion in pairs(data.spectres) do
            if minion.name == spectre_name then
                table.insert(candidates, {
                    kind = "spectre_monster",
                    minion_id = minion_id,
                    minion_name = minion.name,
                    active_parent_groups = parent_groups(minion_id),
                    currently_selected_action = false,
                })
            end
        end
        sort_candidates(candidates)
        if #candidates > 0 then
            return #candidates > 1 and "ambiguous_spectre_name_match" or "spectre_name_match", candidates
        end
    end
    if action_names[name] then return "minion_action_name_match", action_names[name] end
    return "no_catalog_name_match", {}
end
local result_groups, unresolved, included = {}, 0, 0
for group_index, group in ipairs(groups) do
    local item, node = group.sourceItem, group.sourceNode
    local origin = item and "item_granted" or node and "tree_granted"
        or group.source and "other_generated" or "manual"
    local gems = {}
    for gem_index, gem in ipairs(group.gemList) do
        local catalog = gem.gemData
        local effect = gem.grantedEffect or (catalog and catalog.grantedEffect)
        local resolution, hint, candidates = "empty", "none", {}
        if catalog then
            resolution = "resolved_gem"
        elseif gem.grantedEffect then
            resolution = "resolved_granted_effect"
        elseif gem.gemId or gem.skillId or (gem.nameSpec and gem.nameSpec:match("%S")) then
            resolution = "unresolved"
            unresolved = unresolved + 1
            hint, candidates = resolution_hints(gem.nameSpec)
        end
        table.insert(gems, {
            index = gem_index,
            name = text(gem.nameSpec),
            gem_id = text(gem.gemId),
            gem_game_id = catalog and text(catalog.gameId),
            variant_id = catalog and text(catalog.variantId),
            skill_id = text(gem.skillId),
            enabled = gem.enabled == true,
            count = finite(gem.count),
            level = finite(gem.level),
            quality = finite(gem.quality),
            is_support = effect and effect.support == true,
            resolution = resolution,
            diagnostic = text(gem.errMsg),
            hint = hint,
            related_candidates = candidates,
        })
    end
    if group.includeInFullDPS then included = included + 1 end
    table.insert(result_groups, {
        index = group_index,
        label = text(group.label),
        enabled = group.enabled == true,
        slot = text(group.slot),
        provenance = {
            kind = origin,
            source = text(group.source),
            item_id = item and identifier(item.id),
            item_name = item and text(item.name),
            node_id = node and identifier(node.id),
        },
        include_in_full_dps = group.includeInFullDPS == true,
        group_count = finite(group.groupCount),
        main_active_skill = index(group.mainActiveSkill),
        gems = gems,
    })
end
local full_skills, contributions = {}, {}
for _, skill in ipairs(env.player.activeSkillList or {}) do
    local group_index, gem_index = ownership(skill)
    local effect = skill.activeEffect.grantedEffect
    local count, enabled = build.calcsTab.calcs.getActiveSkillCount(skill)
    table.insert(full_skills, {
        group_index = group_index,
        gem_index = gem_index,
        skill_id = text(effect.id),
        skill_name = text(effect.name),
        included = skill.socketGroup and skill.socketGroup.includeInFullDPS == true or false,
        count = finite(count),
        count_enabled = enabled == true,
    })
end
for _, row in ipairs(env.player.output.SkillDPS or {}) do
    table.insert(contributions, {
        name = text(row.name), dps = finite(row.dps), count = finite(row.count),
        skill_part = text(row.skillPart), trigger = text(row.trigger), source = text(row.source),
    })
end
-- PassiveTree preserves raw connections but excludes edges whose target is missing.
-- Inspect every loaded tree: startup can load a version other than the imported one.
local tree_connections = {}
for version, tree in pairs(main.tree) do
    if type(tree) == "table" and type(tree.nodes) == "table" then
        local node_map = {}
        for _, node in pairs(tree.nodes) do node_map[node.id] = node end
        for _, node in pairs(tree.nodes) do
            for _, connection in pairs(node.connections or {}) do
                if not node_map[connection.id] then
                    local is_build_tree = version == build.spec.treeVersion
                    table.insert(tree_connections, {
                        tree_version = tostring(version),
                        source_node_id = node.id,
                        missing_target_id = connection.id,
                        is_build_tree = is_build_tree,
                        source_allocated = is_build_tree and build.spec.allocNodes[node.id] ~= nil,
                        missing_target_allocated = is_build_tree and build.spec.allocNodes[connection.id] ~= nil,
                    })
                end
            end
        end
    end
end
table.sort(tree_connections, function(a, b)
    if a.tree_version ~= b.tree_version then return a.tree_version < b.tree_version end
    if a.source_node_id ~= b.source_node_id then return a.source_node_id < b.source_node_id end
    return a.missing_target_id < b.missing_target_id
end)
local selected_group = selected_player and selected_player.group_index
return {
    schema_version = 1,
    active_skill_set_id = identifier(build.skillsTab.activeSkillSetId),
    groups = result_groups,
    selected_player = selected_player,
    selected_minion = selected_minion,
    full_dps = {
        included_group_count = included,
        selected_group_included = selected_group and groups[selected_group].includeInFullDPS == true or false,
        active_skills = full_skills,
        reported_contributions = contributions,
    },
    unresolved_entry_count = unresolved,
    tree_connections = tree_connections,
}
