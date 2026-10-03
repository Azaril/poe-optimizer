-- Exact live actor/ability correspondence after the original full Build load.
-- minion_physical_gem_inputs.lua supplies the existing saved-instance, source
-- catalog, MAIN/CALCS and fresh-LoadSkill preservation observations.
local calcs = require("Modules.CalcBase")
local function original(fn, path, line)
    local info = debug.getinfo(fn, "S")
    local source = info.source:gsub("\\", "/")
    assert(info.what == "Lua" and source:sub(-#path) == path and info.linedefined == line)
    return fn
end
local methods = {
    children = original(calcs.createMinionSkills, "Modules/CalcActiveSkill.lua", 1116),
    count = original(calcs.getActiveSkillCount, "Modules/CalcDefence.lua", 149),
    reservation = original(calcs.doActorLifeManaSpiritReservation, "Modules/CalcDefence.lua", 177),
    perform = original(calcs.perform, "Modules/CalcPerform.lua", 1193),
}
local function plain(value, depth)
    depth = depth or 0
    assert(depth < 20, "recursive minion occurrence metadata")
    if type(value) ~= "table" then
        assert(value == nil or type(value) == "string" or type(value) == "number" or type(value) == "boolean")
        return value
    end
    local result = {}
    for key, item in pairs(value) do result[key] = plain(item, depth + 1) end
    return result
end
local function scalars(value)
    local result = {}
    for key, item in pairs(value or {}) do
        if type(item) == "number" or type(item) == "string" or type(item) == "boolean" then result[key] = item end
    end
    return result
end
local function names(types)
    local result = {}
    for name, id in pairs(SkillType) do if types and types[id] then result[#result + 1] = name end end
    table.sort(result)
    return result
end
local function sets(effect)
    local result = {}
    for index, statSet in ipairs(effect.statSets or {}) do
        result[#result + 1] = { index = index, id = statSet.id, label = statSet.label, base_flags = scalars(statSet.baseFlags) }
    end
    return result
end
local function selectedSet(effect, mode)
    local selected = mode == "CALCS" and effect.statSetCalcs or effect.statSet
    local actual
    for index, declared in ipairs(effect.grantedEffect.statSets) do
        if selected.statSet == declared then assert(not actual); actual = index end
    end
    assert(actual == selected.index, "selected child stat set must be its exact source table")
    return { index = selected.index, declared_table_index = actual, flags = scalars(selected.skillFlags) }
end
local base = assert(minionOccurrenceBase)
local result = { selected = {}, contexts = {} }
local environments = { MAIN = build.calcsTab.mainEnv, CALCS = build.calcsTab.calcsEnv }
for mode, env in pairs(environments) do
    result.contexts[mode] = {
        main_group = env.mainSocketGroup,
        player_effect = env.player.mainSkill.activeEffect.grantedEffect.id,
        selected_actor_type = env.minion and env.minion.type,
        selected_actor_level = env.minion and env.minion.level,
        selected_child = env.minion and env.minion.mainSkill.activeEffect.grantedEffect.id,
        selected_child_stat_set = env.minion and selectedSet(env.minion.mainSkill.activeEffect, mode),
        full_dps = plain((mode == "MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput).SkillDPS),
        reservation_breakdown = env.player.breakdown and env.player.breakdown.SpiritReserved and plain(env.player.breakdown.SpiritReserved.reservations),
    }
end
for _, saved in ipairs(base.selected) do
    local group = assert(build.skillsTab.skillSets[base.selection.skills].socketGroupList[saved.group])
    local physical = assert(group.gemList[saved.index])
    local gem = assert(physical.gemData)
    assert(gem.id == saved.physical_id)
    local declared = {}
    for index, effect in ipairs(gem.grantedEffectList) do
        declared[#declared + 1] = {
            index = index, id = effect.id, is_primary = effect == gem.grantedEffect,
            has_global_effect = effect.hasGlobalEffect == true,
            global_field = "enableGlobal" .. index, global_value = physical["enableGlobal" .. index],
            minion_list = plain(effect.minionList), minion_has_item_set = effect.minionHasItemSet == true,
            stat_sets = sets(effect), parts = plain(effect.parts),
        }
    end
    local row = {
        source_ordinal = saved.source_ordinal, physical_id = gem.id, group = saved.group, index = saved.index,
        effects = declared, vaal_gem = gem.vaalGem == true,
        source_children = {},
    }
    -- Retain the complete source child tree; maps are not reduced to a generic
    -- field allowlist. Their exact per-effect lookup is observed below.
    local doc, err = common.xml.ParseXML(minionPhysicalXml)
    assert(doc and not err)
    local ordinal = -1
    local function visit(node)
        if type(node) ~= "table" or not node.elem then return end
        ordinal = ordinal + 1
        if ordinal == saved.source_ordinal then
            assert(node.elem == "Gem")
            for _, child in ipairs(node) do row.source_children[#row.source_children + 1] = plain(child) end
        end
        for _, child in ipairs(node) do visit(child) end
    end
    visit(doc[1])
    for mode, env in pairs(environments) do
        local actions = {}
        for _, summon in ipairs(env.player.activeSkillList) do
            if summon.activeEffect.srcInstance == physical then
                local effect = summon.activeEffect.grantedEffect
                local effectIndex
                for index, exact in ipairs(gem.grantedEffectList) do if exact == effect then effectIndex = index end end
                assert(effectIndex)
                local count, enabled = methods.count(summon)
                local minion = summon.minion
                local observed = {
                    effect = effect.id, declared_effect_index = effectIndex,
                    physical_source = true, actor_is_player = summon.actor == env.player,
                    is_main_skill = summon == env.player.mainSkill,
                    stat_set = selectedSet(summon.activeEffect, mode),
                    count = count, count_enabled = enabled, full_dps_included = group.includeInFullDPS == true,
                    minion_choices = plain(summon.minionList),
                    actor_selector_field = mode == "CALCS" and summon == env.player.mainSkill and "skillMinionCalcs" or "skillMinion",
                    child_selector_field = mode == "CALCS" and "skillMinionSkillCalcs" or "skillMinionSkill",
                    reservation = {
                        has_reservation = summon.skillTypes[SkillType.HasReservation] == true,
                        multiple_reservation = summon.skillTypes[SkillType.MultipleReservation] == true,
                        becomes_cost = summon.skillTypes[SkillType.ReservationBecomesCost] == true,
                        autoexertion = summon.skillData.SupportedByAutoexertion == true,
                        summons_totem = summon.skillTypes[SkillType.SummonsTotem] == true,
                        ancestral_bond = summon.actor.modDB:Flag(nil, "AncestralBond") == true,
                        free_spirit_count = summon.skillModList:Sum("BASE", summon.skillCfg, "MinionFreeSpiritCount"),
                        life = summon.skillData.LifeReservedBase, mana = summon.skillData.ManaReservedBase,
                        spirit = summon.skillData.SpiritReservedBase,
                    },
                    umbral = { flag = env.modDB:Flag(nil, "UmbralWell") == true,
                        buff_value = summon.skillModList:Sum("BASE", env.player.mainSkill.skillCfg, "UmbralWellBuffValue"),
                        variant_id = summon.activeEffect.gemData.variantId,
                        limit = minion and minion.minionData and minion.minionData.limit },
                }
                if minion then
                    assert(minion.minionData == env.data.minions[minion.type])
                    local children = {}
                    for index, child in ipairs(minion.activeSkillList) do
                        local childEffect = child.activeEffect.grantedEffect
                        assert(child.actor == minion and child.summonSkill == summon)
                        assert(childEffect == env.data.skills[childEffect.id])
                        children[#children + 1] = {
                            index = index, effect = childEffect.id, name = childEffect.name,
                            actor_identity = child.actor == minion, summon_parent = child.summonSkill == summon,
                            source_instance_present = child.activeEffect.srcInstance ~= nil,
                            selected = minion.mainSkill == child,
                            stat_sets = sets(childEffect), stat_set = selectedSet(child.activeEffect, mode),
                            parts = plain(childEffect.parts), types = names(child.skillTypes),
                            level = child.activeEffect.level, quality = child.activeEffect.quality,
                        }
                    end
                    local declaredSkills = {}
                    for index, id in ipairs(minion.minionData.skillList) do
                        declaredSkills[#declaredSkills + 1] = { index = index, id = id, resolved = env.data.skills[id] ~= nil }
                    end
                    observed.actor = {
                        type = minion.type, name = minion.minionData.name, level = minion.level,
                        source_data_identity = minion.minionData == env.data.minions[minion.type],
                        parent_is_player = minion.parent == env.player, enemy_identity = minion.enemy == env.enemy,
                        is_selected_actor = env.minion == minion, item_set_present = minion.itemSet ~= nil,
                        main_child = minion.mainSkill.activeEffect.grantedEffect.id,
                        declared_skills = declaredSkills, children = children,
                        output = scalars(minion.output),
                    }
                end
                actions[#actions + 1] = observed
            end
        end
        row[mode] = actions
    end
    result.selected[#result.selected + 1] = row
end
assert(methods.children == calcs.createMinionSkills and methods.count == calcs.getActiveSkillCount)
assert(methods.reservation == calcs.doActorLifeManaSpiritReservation and methods.perform == calcs.perform)
result.source_methods_preserved = true
return result
