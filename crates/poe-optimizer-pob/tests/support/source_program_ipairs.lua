local saved_ipairs, saved_type = ipairs, type
local saved_aux = saved_ipairs({})
local api = {}
function api.factory(...) return saved_ipairs(...) end
function api.direct(...) return saved_aux(...) end
function api.same(a, b) return a == b end
function api.kind(a) return saved_type(a) end
function api.read(t, key) return t[key] end
function api.write(t, key, value) t[key] = value end
function api.walk(t, out)
    for key, value in saved_ipairs(t) do out[key] = value end
    return out
end
function api.walk_mutating(t, out)
    for key, value in saved_ipairs(t) do
        out[key] = value
        if key == 1 then t[2] = false end
        if key == 2 then t[3] = nil end
    end
    return out, t
end
local function bump(state)
    state.effects = state.effects + 1
    return state.value
end
function api.factory_effect(state, t)
    return saved_ipairs(t, bump(state))
end
function api.aux_effect(state, t, control)
    return saved_aux(t, control, bump(state))
end
function api.stored(t, state)
    state.step, state.subject, state.control = saved_ipairs(t)
    return state
end
function api.resume(state)
    local key, value = state.step(state.subject, state.control)
    state.control = key
    return key, value
end
function api.return_originals() return saved_ipairs, saved_aux end
function api.rebound(t) return ipairs(t) end
function api.replacement(t) return "rebound", t, 73 end
function api.record_direct(t, control, out)
    local index = out.calls + 1
    local first, second = saved_aux(t, control)
    local count = select('#', saved_aux(t, control))
    out.calls = index
    out.counts[index] = count
    out.firsts[index], out.seconds[index] = first, second
    return out
end
function api.record_factory(t, out)
    local index = out.calls + 1
    local step, state, control = saved_ipairs(t)
    local count = select('#', saved_ipairs(t))
    out.calls = index
    out.counts[index] = count
    out.firsts[index] = step == saved_aux
    out.seconds[index] = state == t
    out.controls[index] = control
    return out
end
return api
