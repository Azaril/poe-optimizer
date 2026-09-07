local env = assert(build.calcsTab.mainEnv)
local function scalar_map(source)
    local result = {}
    for key, value in pairs(source or {}) do
        if type(key) == "string" then
            local kind = type(value)
            if kind == "boolean" or kind == "string" then
                result[key] = value
            elseif kind == "number" then
                assert(value == value and value ~= math.huge and value ~= -math.huge,
                    "Nonfinite numeric encounter metadata: " .. key)
                result[key] = value
            else
                result[key] = "<" .. kind .. ">"
            end
        end
    end
    return result
end
local function conditions(source)
    local result = {}
    for key, value in pairs(source or {}) do
        if type(key) == "string" and type(value) == "boolean" then result[key] = value end
    end
    return result
end
return {
    requested = _optimizer_options,
    calculation_mode = env.mode,
    enemy_level = env.enemyLevel,
    config_inputs = scalar_map(env.configInput),
    config_placeholders = scalar_map(env.configPlaceholder),
    player_conditions = conditions(env.player.modDB.conditions),
    enemy_conditions = conditions(env.enemy.modDB.conditions),
}
