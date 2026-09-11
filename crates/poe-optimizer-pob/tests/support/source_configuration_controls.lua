-- Test-only state observations and producer transitions. Calculations and
-- notifications invoke the complete original SetPlaceholder/changeFunc bodies.
local probes = {}
function probes.result(control, config, var)
    local active = config.configSets[config.activeConfigSetId]
    return control.placeholder, active.placeholder[var], config.build.buildFlag,
        config.input == active.input, config.placeholder == active.placeholder
end
function probes.set_flag(config, value)
    config.build.buildFlag = value
end
function probes.switch(config, id)
    config.activeConfigSetId = id
    config.input = config.configSets[id].input
    config.placeholder = config.configSets[id].placeholder
end
function probes.add_set(config, id)
    config.configSets[id] = {input = {}, placeholder = {}}
end
function probes.call_change(control, text, placeholder)
    return control.changeFunc(text, placeholder)
end
function probes.identities(a, b, c)
    return a.changeFunc == b.changeFunc, a.changeFunc == c.changeFunc
end
function probes.continuing_result(a, b, config, va, vb)
    local active = config.configSets[config.activeConfigSetId]
    return a.placeholder, b.placeholder, active.placeholder[va], active.placeholder[vb],
        active.input[va], active.input[vb], config.build.buildFlag,
        config.input == active.input, config.placeholder == active.placeholder,
        config.activeConfigSetId
end
return probes
