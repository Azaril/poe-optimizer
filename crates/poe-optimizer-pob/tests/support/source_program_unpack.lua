-- Complete functions; the original C primitive identity is observed before source.
local original, kind = unpack, type
local function captured(...) return original(...) end
local function global(...) return unpack(...) end
local function effect(state) state.count = state.count + 1 return "ignored", nil end
local function extra(list, state) return original(list, 1, 2, effect(state)) end
local function empty(list, state) return original(list, 2, 1, effect(state)) end
local function state(value) return value.count end
local function identity()
    local shared = {}
    local values = {shared, shared, captured}
    local first, second, callback = original(values)
    return first == second, callback == captured, kind(callback), values, first, second
end
local function replacement(...) return "rebound" end
return { captured = captured, global = global, extra = extra, empty = empty,
    state = state, identity = identity, replacement = replacement }
