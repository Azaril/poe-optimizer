-- Apply backend-neutral diagnostic selection/encounter overrides to an imported build.
-- Called only after import has finished, before the final ordinary-frame calculation.
local options = _optimizer_options
local expected_group, expected_player_id, expected_player_gem
local expected_minion_id, expected_minion_action_id
local boss_kinds = { normal = "None", standard = "Boss", pinnacle = "Pinnacle", uber = "Uber" }
local damage_components = {{"physical", "Physical"}, {"fire", "Fire"}, {"cold", "Cold"}, {"lightning", "Lightning"}, {"chaos", "Chaos"}}
local function recalculate()
    local revision = build.outputRevision
    build.buildFlag = true
    runCallback("OnFrame")
    assert(not build.buildFlag and build.outputRevision > revision, "Selection preparation did not recalculate")
end
-- Display lists are shared UI caches and may be replaced by auxiliary MAIN passes.
-- CalcSetup appends group actions to the owning MAIN actor in the same order.
local function group_actions(env, group)
    local result = {}
    for _, action in ipairs(env.player.activeSkillList) do
        if action.socketGroup == group then table.insert(result, action) end
    end
    return result
end
local function assert_hit_category()
    assert(build.configTab.input.enemyDamageType ~= "DamageOverTime",
        "Explicit incoming hit cannot be applied to an imported DamageOverTime encounter")
end
if options.encounter then
    local encounter = options.encounter
    if encounter.enemy_level then
        assert(encounter.enemy_level <= data.misc.MaxEnemyLevel, "Enemy level exceeds the pinned game's supported maximum")
        build.configTab.input.enemyLevel = encounter.enemy_level
    end
    if encounter.boss then
        local kind = assert(boss_kinds[encounter.boss], "Unsupported boss kind")
        build.configTab.input.enemyIsBoss = kind
        -- Boss skill presets consult this control directly when choosing Uber scaling.
        build.configTab.varControls.enemyIsBoss:SelByValue(kind, "val")
    end
    if encounter.incoming_hit then
        for _, pair in ipairs(damage_components) do
            build.configTab.input["enemy" .. pair[2] .. "Damage"] = encounter.incoming_hit[pair[1]]
        end
    end
    build.configTab:BuildModList()
    -- Presets can rewrite the damage category while rebuilding the configuration.
    if encounter.incoming_hit then assert_hit_category() end
end
if options.selection then
    local selected = options.selection
    local group = assert(build.skillsTab.socketGroupList[selected.socket_group], "Requested skill group does not exist")
    assert(group.enabled, "Requested skill group is disabled")
    assert(group.slotEnabled ~= false, "Requested skill group is unavailable in the active weapon set")
    expected_group = group
    build.mainSocketGroup = selected.socket_group
    -- Materialize the actual generated action list, which differs from gem indexes.
    recalculate()
    local skills = group_actions(assert(build.calcsTab.mainEnv), group)
    assert(#skills > 0, "Requested group has no resolved active actions")
    if selected.active_skill then
        assert(skills[selected.active_skill], "Requested active skill index does not exist")
        group.mainActiveSkill = selected.active_skill
        recalculate()
    end
    local env = assert(build.calcsTab.mainEnv)
    local player_skill = assert(env.player.mainSkill, "Requested group has no selected action")
    assert(player_skill.socketGroup == group, "Requested skill group fell back to a different action")
    expected_player_id = player_skill.activeEffect.grantedEffect.id
    expected_player_gem = player_skill.activeEffect.srcInstance
    if selected.minion_skill then
        local minion = assert(env.minion, "Requested action has no selected minion")
        local action = assert(minion.activeSkillList[selected.minion_skill], "Requested minion action index does not exist")
        expected_minion_id = minion.type
        expected_minion_action_id = action.activeEffect.grantedEffect.id
        player_skill.activeEffect.srcInstance.skillMinionSkill = selected.minion_skill
    end
end

-- Called by the host after its final fresh MAIN frame, before exporting any output.
-- Check actual resolved ownership as well as indexes, which upstream may clamp.
function _optimizer_validate_options()
    local env = assert(build.calcsTab.mainEnv, "Missing final MAIN environment")
    assert(build.calcsTab.mainOutput == env.player.output, "Mismatched final MAIN output")
    local encounter = options.encounter
    if encounter then
        if encounter.enemy_level then
            assert(env.enemyLevel == encounter.enemy_level, "Requested enemy level was normalized or replaced")
        end
        if encounter.boss then
            assert(env.configInput.enemyIsBoss == boss_kinds[encounter.boss], "Requested boss kind was replaced")
        end
        if encounter.incoming_hit then
            assert_hit_category()
            for _, pair in ipairs(damage_components) do
                assert(env.configInput["enemy" .. pair[2] .. "Damage"] == encounter.incoming_hit[pair[1]],
                    "Requested incoming damage component was replaced")
            end
        end
    end
    local selected = options.selection
    if not selected then return end
    local group = build.skillsTab.socketGroupList[selected.socket_group]
    assert(group == expected_group and build.mainSocketGroup == selected.socket_group,
        "Requested skill group was normalized or replaced")
    assert(group.enabled and group.slotEnabled ~= false, "Requested skill group became unavailable")
    local player_skill = assert(env.player.mainSkill, "Requested action is absent from final output")
    assert(player_skill.socketGroup == group, "Requested skill group fell back to a different action")
    assert(group_actions(env, group)[group.mainActiveSkill] == player_skill,
        "Final selected action does not belong to the requested group")
    assert(player_skill.activeEffect.grantedEffect.id == expected_player_id
        and player_skill.activeEffect.srcInstance == expected_player_gem,
        "Requested player action was replaced during final calculation")
    if selected.active_skill then
        assert(group.mainActiveSkill == selected.active_skill,
            "Requested active skill index was normalized or replaced")
    end
    if selected.minion_skill then
        local minion = assert(env.minion, "Requested minion is absent from final output")
        local action = assert(minion.mainSkill, "Requested minion action is absent from final output")
        assert(minion.type == expected_minion_id
            and minion.activeSkillList[selected.minion_skill] == action
            and action.activeEffect.grantedEffect.id == expected_minion_action_id
            and action.summonSkill == player_skill
            and player_skill.activeEffect.srcInstance.skillMinionSkill == selected.minion_skill,
            "Requested minion action was normalized or replaced")
    end
end
