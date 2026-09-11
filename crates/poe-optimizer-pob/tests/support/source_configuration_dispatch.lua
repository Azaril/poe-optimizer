-- Read-only test observations. No configuration formula or callback is replaced.
local ipairs = ipairs
local function state(player, enemy, build, names)
    local playerRows, enemyRows, controls = {}, {}, {}
    for i, mod in ipairs(player) do playerRows[i] = mod end
    for i, mod in ipairs(enemy) do enemyRows[i] = mod end
    local config = build.configTab
    local set = config.configSets[config.activeConfigSetId]
    for _, name in ipairs(names) do
        controls[name] = config.varControls[name].placeholder
    end
    return playerRows, enemyRows, controls, set.input, set.placeholder,
        config.enemyLevel, build.buildFlag,
        config.input == set.input, config.placeholder == set.placeholder,
        config.build == build
end
return {state = state}
