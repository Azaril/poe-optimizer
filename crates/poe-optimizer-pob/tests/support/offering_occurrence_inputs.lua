-- Additional read-only inventory after the authenticated full Build lifecycle.
-- The existing occurrence observer owns loading, MAIN/CALCS snapshots and fresh
-- source objects. This observer records actual tables and consumer guard inputs;
-- it neither reproduces calculations nor grants native inventory authority.
local calcs = require("Modules.CalcBase")
local function original(fn, path, line)
    local info = debug.getinfo(fn, "S")
    local source = info.source:gsub("\\", "/")
    assert(info.what == "Lua" and source:sub(-#path) == path and info.linedefined == line)
    return fn
end
local methods = {
    count = original(calcs.getActiveSkillCount, "Modules/CalcDefence.lua", 149),
    reservation = original(calcs.doActorLifeManaSpiritReservation, "Modules/CalcDefence.lua", 177),
    perform = original(calcs.perform, "Modules/CalcPerform.lua", 1193),
    full = original(calcs.calcFullDPS, "Modules/Calcs.lua", 251),
}
local function plain(value, depth)
    depth = depth or 0
    assert(depth < 20, "unexpected recursive Offering metadata")
    if type(value) ~= "table" then
        assert(value == nil or type(value) == "string" or type(value) == "number" or type(value) == "boolean")
        return value
    end
    local result = {}
    for key, item in pairs(value) do
        assert(type(key) == "string" or type(key) == "number")
        result[key] = plain(item, depth + 1)
    end
    return result
end
local function scalars(value)
    local result = {}
    for key, item in pairs(value or {}) do
        if type(item) == "number" or type(item) == "string" or type(item) == "boolean" then
            result[key] = item
        end
    end
    return result
end
local base = assert(offeringOccurrenceBase)
local selected
for _, row in ipairs(base.selected) do
    if row.physical_id == "Metadata/Items/Gems/SkillGemPainOffering" then
        assert(not selected, "ambiguous selected physical Offering")
        selected = row
    end
end
assert(selected)
local group = assert(build.skillsTab.skillSets[base.selection.skills].socketGroupList[selected.group])
local physical = assert(group.gemList[selected.index])
local data = assert(physical.gemData)
assert(data.id == selected.physical_id)
local effectRows, additional = {}, {}
for _, effect in ipairs(data.additionalGrantedEffects or {}) do
    additional[#additional + 1] = effect.id
end
for index, effect in ipairs(data.grantedEffectList) do
    local statSets = {}
    for ordinal, statSet in ipairs(effect.statSets or {}) do
        statSets[#statSets + 1] = {
            ordinal = ordinal, id = statSet.id, label = statSet.label,
            base_flags = scalars(statSet.baseFlags),
        }
    end
    effectRows[#effectRows + 1] = {
        ordinal = index, id = effect.id, name = effect.name,
        is_primary = effect == data.grantedEffect,
        support = effect.support == true, from_tree = effect.fromTree == true,
        hide_from_sidebar = effect.hideFromSideBar == true,
        has_global_effect = effect.hasGlobalEffect == true,
        global_field = "enableGlobal" .. index,
        global_value = physical["enableGlobal" .. index],
        parts = plain(effect.parts), stat_sets = statSets,
    }
end
local result = {
    source_ordinal = selected.source_ordinal,
    physical_id = data.id, variant_id = data.variantId,
    primary_effect = data.grantedEffect.id,
    vaal_gem = data.vaalGem == true,
    granted_effects = effectRows, additional_effects = additional,
    -- These are actual saved and processed tables, including absence of fields.
    saved_attributes = plain(selected.attributes),
    saved_group_attributes = plain(selected.group_attributes),
    loaded = plain(selected.loaded), fresh = plain(selected.fresh),
    group_state = plain(selected.group_state), modes = {},
}
for mode, env in pairs({MAIN = build.calcsTab.mainEnv, CALCS = build.calcsTab.calcsEnv}) do
    local actions, allSkills = {}, {}
    for ordinal, skill in ipairs(env.player.activeSkillList) do
        local effect = skill.activeEffect.grantedEffect
        allSkills[#allSkills + 1] = {
            ordinal = ordinal, effect = effect.id,
            is_main_skill = skill == env.player.mainSkill,
            is_offering_occurrence = skill.activeEffect.srcInstance == physical,
        }
        if skill.activeEffect.srcInstance == physical then
            local effectOrdinal
            for index, declared in ipairs(data.grantedEffectList) do
                if effect == declared then
                    assert(not effectOrdinal)
                    effectOrdinal = index
                end
            end
            assert(effectOrdinal, "constructed effect must be the exact declared table")
            local count, enabled = methods.count(skill)
            local types, skillData = skill.skillTypes, skill.skillData
            local minion = skill.minion
            actions[#actions + 1] = {
                effect = effect.id, declared_ordinal = effectOrdinal,
                physical_source = skill.activeEffect.srcInstance == physical,
                count = count, count_enabled = enabled,
                global_field = "enableGlobal" .. effectOrdinal,
                global_value = physical["enableGlobal" .. effectOrdinal],
                buffs = plain(skill.buffList),
                buff_skill = skill.buffSkill == true,
                minion_buff_skill = skill.minionBuffSkill == true,
                skill_data = scalars(skillData),
                -- Observe source branch inputs, not a synthetic reservation.
                reservation_guard_inputs = {
                    has_reservation = types[SkillType.HasReservation] == true,
                    supported_by_autoexertion = skillData.SupportedByAutoexertion == true,
                    reservation_becomes_cost = types[SkillType.ReservationBecomesCost] == true,
                    summons_totem = types[SkillType.SummonsTotem] == true,
                    ancestral_bond = skill.actor.modDB:Flag(nil, "AncestralBond") == true,
                    multiple_reservation = types[SkillType.MultipleReservation] == true,
                },
                reservation = {
                    life = skillData.LifeReservedBase, mana = skillData.ManaReservedBase,
                    spirit = skillData.SpiritReservedBase,
                },
                umbral_guard_inputs = {
                    attached_minion = minion ~= nil,
                    minion_data = minion ~= nil and minion.minionData ~= nil,
                    minion_limit = minion and minion.minionData and minion.minionData.limit,
                    flag = env.modDB:Flag(nil, "UmbralWell") == true,
                    buff_value = skill.skillModList:Sum("BASE", env.player.mainSkill.skillCfg, "UmbralWellBuffValue"),
                    variant_id = skill.activeEffect.gemData.variantId,
                },
                full_dps_group_included = skill.socketGroup.includeInFullDPS == true,
            }
        end
    end
    result.modes[mode] = {
        actions = actions, active_skill_inventory = allSkills,
        mode_buffs = env.mode_buffs,
        player_affected = env.player.modDB.conditions.AffectedByPainOffering == true,
        selected_minion_present = env.minion ~= nil,
        selected_minion_affected = env.minion ~= nil and env.minion.modDB.conditions.AffectedByPainOffering == true,
        selected_minion_skill = env.minion and env.minion.mainSkill and env.minion.mainSkill.activeEffect.grantedEffect.id,
    }
end
assert(methods.count == calcs.getActiveSkillCount and methods.reservation == calcs.doActorLifeManaSpiritReservation)
assert(methods.perform == calcs.perform and methods.full == calcs.calcFullDPS)
result.source_methods_preserved = true
return result
