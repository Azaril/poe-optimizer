local next, pairs = next, pairs
-- Complete callable constructors and consumers for native differential tests.
local unpack = unpack
local api = {}
function api.empty() return {} end
function api.pair(a, b) return {a, b} end
function api.triple(a, b, c) return {a, b, c} end
function api.tail(input) return {unpack(input, 1, input.n)} end
function api.prefix(a, input) return {a, unpack(input, 1, input.n)} end
function api.prefix_two(a, b, input) return {a, b, unpack(input, 1, input.n)} end
function api.single_tail(a, input) return {a, (unpack(input, 1, input.n))} end
function api.separator(a, input) return {a, unpack(input, 1, input.n),} end
function api.nested(a, b) return {{a, b}, {b, a}} end
function api.branch(flag, a, b)
    if flag then return {a, b} else return {b, a} end
end
function api.effect(state)
    local x = state.initial
    local function change()
        x = state.changed
        state.effects = state.effects + 1
        return state.right, nil, false
    end
    return {x, change()}
end
function api.effect_single(state)
    local x = state.initial
    local function change()
        x = state.changed
        state.effects = state.effects + 1
        return state.right, nil, false
    end
    return {x, (change())}
end
function api.failed_tail(state)
    local function change()
        state.effects = state.effects + 1
        local missing = nil
        return missing.value
    end
    return {state.first, change()}
end
function api.walk(table, output)
    local count = 0
    for key, value in pairs(table) do
        count = count + 1
        output[count] = key
        count = count + 1
        output[count] = value
    end
    return count
end
function api.default_unpack(table) return unpack(table) end
function api.explicit_unpack(table, first, last) return unpack(table, first, last) end
function api.next_value(table, control) return next(table, control) end
function api.read(table, key) return table[key] end
function api.write(table, key, value) table[key] = value end
return api
