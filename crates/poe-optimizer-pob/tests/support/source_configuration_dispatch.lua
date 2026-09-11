-- Read-only test observations. No configuration formula or callback is replaced.
local ipairs = ipairs
local function state(player, enemy, build, names, dropdownNames)
    local playerRows, enemyRows, controls = {}, {}, {}
    for i, mod in ipairs(player) do playerRows[i] = mod end
    for i, mod in ipairs(enemy) do enemyRows[i] = mod end
    local config = build.configTab
    local set = config.configSets[config.activeConfigSetId]
    for _, name in ipairs(names) do
        controls[name] = config.varControls[name].placeholder
    end
    local dropdowns = {}
    for _, name in ipairs(dropdownNames) do
        local control = config.varControls[name]
        dropdowns[name] = { selection = control.selIndex, enabled = control.enabled,
            list = control.list, definitionListAlias = control.list == dropdownNames[name] }
    end
    return playerRows, enemyRows, controls, set.input, set.placeholder,
        config.enemyLevel, build.buildFlag,
        config.input == set.input, config.placeholder == set.placeholder,
        config.build == build, dropdowns
end
local function select(build, name, value)
    build.configTab.varControls[name]:SelByValue(value, "val")
end
return {state = state, select = select}
