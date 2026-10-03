-- Separate source-only witness. All reviewed identities and scalar probes are
-- supplied by the Rust test caller; no original business method is replaced.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
    local info = debug.getinfo(f, "S")
    local actual = info.source:gsub("\\", "/")
    assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line,
        "unexpected complete source method: " .. path .. ":" .. tostring(info.linedefined))
    return f
end
local methods = {
    load = original(build.skillsTab.LoadSkill, "Classes/SkillsTab.lua", 303),
    process = original(build.skillsTab.ProcessSocketGroup, "Classes/SkillsTab.lua", 1242),
    validate = original(calcLib.validateGemLevel, "Modules/CalcTools.lua", 60),
    init = original(calcs.initEnv, "Modules/CalcSetup.lua", 717),
    create = original(calcs.createActiveSkill, "Modules/CalcActiveSkill.lua", 144),
    mods = original(calcs.buildActiveSkillModList, "Modules/CalcActiveSkill.lua", 426),
    count = original(calcs.getActiveSkillCount, "Modules/CalcDefence.lua", 149),
    full = original(calcs.calcFullDPS, "Modules/Calcs.lua", 251),
    output = original(calcs.buildOutput, "Modules/Calcs.lua", 469),
}
local function copy(v)
    if type(v) ~= "table" then return v end
    local result = {}
    for k, x in pairs(v) do result[k] = copy(x) end
    return result
end
local function equal(a, b)
    if type(a) ~= type(b) then return false end
    if type(a) ~= "table" then return a == b end
    for k, v in pairs(a) do if not equal(v, b[k]) then return false end end
    for k in pairs(b) do if a[k] == nil then return false end end
    return true
end
local function scalars(t)
    local result = {}
    for k, v in pairs(t or {}) do
        if type(v) == "number" or type(v) == "boolean" or type(v) == "string" then result[k] = v end
    end
    return result
end
local function fields(g)
    return {
        gem_id = g.gemId, skill_id = g.skillId, name = g.nameSpec,
        level = g.level, quality = g.quality, corrupted = g.corrupted, corrupt_level = g.corruptLevel,
        enabled = g.enabled, count = g.count, global_1 = g.enableGlobal1, global_2 = g.enableGlobal2,
        stat_set = copy(g.statSet), stat_set_calcs = copy(g.statSetCalcs),
        skill_part = g.skillPart, skill_part_calcs = g.skillPartCalcs,
        stage = g.skillStageCount, stage_calcs = g.skillStageCountCalcs,
        mine = g.skillMineCount, mine_calcs = g.skillMineCountCalcs,
        minion = g.skillMinion, minion_calcs = g.skillMinionCalcs,
        minion_item_set = g.skillMinionItemSet, minion_item_set_calcs = g.skillMinionItemSetCalcs,
        minion_skill = g.skillMinionSkill, minion_skill_calcs = g.skillMinionSkillCalcs,
        minion_stat_sets = copy(g.skillMinionSkillStatSetIndexLookup),
        minion_stat_sets_calcs = copy(g.skillMinionSkillStatSetIndexLookupCalcs),
    }
end
local function groupFields(g)
    return { enabled = g.enabled, slot_enabled = g.slotEnabled, count = g.groupCount,
        include = g.includeInFullDPS, main = g.mainActiveSkill, calcs = g.mainActiveSkillCalcs,
        source = g.source, slot = g.slot }
end
local function levelKeys(effect)
    local keys = {}
    for key in pairs(effect.levels) do keys[#keys + 1] = key end
    table.sort(keys)
    return keys
end
local function effects(list)
    local ids = {}
    for index, effect in ipairs(list) do
        assert(effect == data.skills[effect.id])
        ids[index] = effect.id
    end
    return ids
end
local function freshTab()
    return setmetatable({ build = build, skillSets = { [1] = { socketGroupList = {} } } },
        { __index = build.skillsTab })
end
local function catalogRow(reviewed)
    local gem = assert(data.gems[reviewed.key])
    assert(gem.id == reviewed.key)
    local unresolved = {}
    for _, reference in ipairs(reviewed.constructed_additional_effects) do
        assert(gem["additionalGrantedEffectId" .. reference.index] == reference.id)
        local found = false
        for _, effect in ipairs(gem.grantedEffectList) do
            if effect.id == reference.id then found = true end
        end
        unresolved[#unresolved + 1] = { index = reference.index, id = reference.id,
            standalone_skill = data.skills[reference.id] ~= nil, in_effect_list = found }
    end
    assert(gem["additionalGrantedEffectId" .. (#unresolved + 1)] == nil)
    return { id = gem.id, primary = gem.grantedEffectId, game_id = gem.gameId, variant_id = gem.variantId,
        primary_data_identity = gem.grantedEffect == data.skills[reviewed.primary_effect_id],
        primary_support = gem.grantedEffect.support == true, primary_from_tree = gem.grantedEffect.fromTree == true,
        natural_max_level = gem.naturalMaxLevel, primary_level_keys = levelKeys(gem.grantedEffect),
        first_stat_set_level_keys = levelKeys(assert(gem.grantedEffect.statSets[1])),
        effects = effects(gem.grantedEffectList), resolved_additional = effects(gem.additionalGrantedEffects),
        unresolved_references = unresolved }
end
local function actions(physical, env)
    local rows = {}
    for _, action in ipairs(env.player.activeSkillList) do
        local effect = action.activeEffect
        if effect.srcInstance == physical then
            local count, enabled = methods.count(action)
            local children = {}
            if action.minion then
                for _, child in ipairs(action.minion.activeSkillList or {}) do
                    children[#children + 1] = { effect = child.activeEffect.grantedEffect.id,
                        summon_parent = child.summonSkill == action,
                        selected = action.minion.mainSkill == child }
                end
            end
            rows[#rows + 1] = { effect = effect.grantedEffect.id, physical_source = true,
                level = effect.level, quality = effect.quality, count = count, count_enabled = enabled,
                is_main_skill = env.player.mainSkill == action,
                multiple_reservation = action.skillTypes[SkillType.MultipleReservation] == true,
                creates_minion = action.skillTypes[SkillType.CreatesMinion] == true,
                minion_present = action.minion ~= nil, minion_skills = children }
        end
    end
    return rows
end
local selection = { skills = build.skillsTab.activeSkillSetId, items = build.itemsTab.activeItemSetId,
    spec = build.treeTab.activeSpec, config = build.configTab.activeConfigSetId, group = build.mainSocketGroup }
local environments = { MAIN = assert(build.calcsTab.mainEnv), CALCS = assert(build.calcsTab.calcsEnv) }
local outputs = { MAIN = assert(build.calcsTab.mainOutput), CALCS = assert(build.calcsTab.calcsOutput) }
local snapshots = { MAIN = scalars(outputs.MAIN), CALCS = scalars(outputs.CALCS) }
local saved = {}
for sid, set in pairs(build.skillsTab.skillSets) do
    for gi, group in ipairs(set.socketGroupList) do
        for i, physical in ipairs(group.gemList) do
            saved[#saved + 1] = { sid = sid, gi = gi, i = i, group = group, gem = physical,
                fields = fields(physical), group_fields = groupFields(group) }
        end
    end
end
local result = { catalog = {}, load_cases = {}, selected = {}, selection = selection, outputs = snapshots,
    actor_outputs = {
        MAIN = { player = scalars(environments.MAIN.player.output),
            minion = environments.MAIN.minion and scalars(environments.MAIN.minion.output) },
        CALCS = { player = scalars(environments.CALCS.player.output),
            minion = environments.CALCS.minion and scalars(environments.CALCS.minion.output) },
    },
}
local selectedObjects = {}
local reviewedByKey = {}
for _, reviewed in ipairs(minionPhysicalReviewed) do
    assert(not reviewedByKey[reviewed.key])
    reviewedByKey[reviewed.key] = reviewed
    result.catalog[#result.catalog + 1] = catalogRow(reviewed)
end

-- Observe the untouched original's actual MAIN/CALCS consumers independently of
-- the fresh physical-scalar probes below, retaining saved minion selectors/maps.
local doc, parseError = common.xml.ParseXML(minionPhysicalXml)
assert(doc and not parseError)
local ordinals, nextOrdinal = {}, 0
local function enumerate(node)
    if type(node) ~= "table" or not node.elem then return end
    ordinals[node], nextOrdinal = nextOrdinal, nextOrdinal + 1
    for _, child in ipairs(node) do enumerate(child) end
end
enumerate(doc[1])
local skills
for _, node in ipairs(doc[1]) do
    if type(node) == "table" and node.elem == "Skills" then assert(not skills); skills = node end
end
assert(skills)
for _, setNode in ipairs(skills) do
    if type(setNode) == "table" and setNode.elem == "SkillSet" and tonumber(setNode.attrib.id) == selection.skills then
        local gi = 0
        for _, node in ipairs(setNode) do
            if type(node) == "table" and node.elem == "Skill" then
                gi = gi + 1
                local group = assert(build.skillsTab.skillSets[selection.skills].socketGroupList[gi])
                for i, child in ipairs(node) do
                    if type(child) == "table" and child.elem == "Gem" then
                        local physical = assert(group.gemList[i])
                        if physical.gemData and reviewedByKey[physical.gemData.id] then
                            local tab = freshTab()
                            methods.load(tab, node, 1)
                            local freshGroup = assert(tab.skillSets[1].socketGroupList[1])
                            local fresh = assert(freshGroup.gemList[i])
                            assert(fresh ~= physical and fresh.gemData == physical.gemData)
                            local row = { source_ordinal = ordinals[child],
                                group = gi, index = i, physical_id = physical.gemData.id,
                                attributes = copy(child.attrib), group_attributes = copy(node.attrib),
                                loaded = fields(physical), fresh = fields(fresh), group_state = groupFields(group),
                                MAIN = actions(physical, environments.MAIN), CALCS = actions(physical, environments.CALCS) }
                            result.selected[#result.selected + 1] = row
                            selectedObjects[#selectedObjects + 1] = { physical = physical, row = row }
                        end
                    end
                end
            end
        end
    end
end
for _, reviewed in ipairs(minionPhysicalReviewed) do
    local gem = assert(data.gems[reviewed.key])
    for _, case in ipairs(minionPhysicalCases) do
        local attributes = copy(case.attributes)
        attributes.gemId, attributes.variantId = gem.gameId, gem.variantId
        local tab = freshTab()
        local ok, err = pcall(methods.load, tab,
            { elem = "Skill", attrib = { enabled = "true" }, { elem = "Gem", attrib = attributes } }, 1)
        local groups = tab.skillSets[1].socketGroupList
        local row = { id = gem.id, label = case.label, attributes = attributes, ok = ok,
            error = not ok and tostring(err) or nil, published_groups = #groups }
        if groups[1] then
            local group = groups[1]
            local physical = assert(group.gemList[1])
            row.physical_gems, row.same_gem_data = #group.gemList, physical.gemData == gem
            row.after, row.effects = fields(physical), effects(gem.grantedEffectList)
            -- Reuse the exact object with the unchanged complete source method.
            methods.process(tab, group)
            assert(group.gemList[1] == physical)
            row.reprocessed = fields(physical)
        end
        result.load_cases[#result.load_cases + 1] = row
    end
end
for _, s in ipairs(saved) do
    local group = build.skillsTab.skillSets[s.sid].socketGroupList[s.gi]
    assert(group == s.group and group.gemList[s.i] == s.gem)
    assert(equal(fields(s.gem), s.fields) and equal(groupFields(group), s.group_fields))
end
assert(build.skillsTab.activeSkillSetId == selection.skills and build.itemsTab.activeItemSetId == selection.items
    and build.treeTab.activeSpec == selection.spec and build.configTab.activeConfigSetId == selection.config
    and build.mainSocketGroup == selection.group)
for mode, env in pairs(environments) do
    assert((mode == "MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv) == env)
    assert((mode == "MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput) == outputs[mode])
    assert(equal(scalars(outputs[mode]), snapshots[mode]))
    assert(equal(scalars(env.player.output), result.actor_outputs[mode].player))
    assert(equal(env.minion and scalars(env.minion.output), result.actor_outputs[mode].minion))
    for _, observed in ipairs(selectedObjects) do
        assert(equal(actions(observed.physical, env), observed.row[mode]))
    end
end
for i, reviewed in ipairs(minionPhysicalReviewed) do assert(equal(catalogRow(reviewed), result.catalog[i])) end
assert(methods.load == build.skillsTab.LoadSkill and methods.process == build.skillsTab.ProcessSocketGroup
    and methods.validate == calcLib.validateGemLevel and methods.init == calcs.initEnv
    and methods.create == calcs.createActiveSkill and methods.mods == calcs.buildActiveSkillModList
    and methods.count == calcs.getActiveSkillCount and methods.full == calcs.calcFullDPS and methods.output == calcs.buildOutput)
result.saved_instances_preserved, result.selected_state_preserved = true, true
result.main_and_calcs_outputs_preserved, result.source_methods_preserved, result.source_catalog_preserved = true, true, true
return result
