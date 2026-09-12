local original_next = next
local original_pairs = pairs
local function make(a, b, c, ...)
    local empty = {}
    return { alpha = a, ["sp ace"] = b, ["\255\000"] = c, ... }, empty
end
local function tail(state, ...)
    state.calls = state.calls + 1
    if state.fail then return nil + 1 end
    return ...
end
local function effect_make(state, ...)
    return { first = state, second = state, tail(state, ...) }
end
local function plain(a, b, c)
    return { one = a, two = b, three = c }
end
local function ordered(a, b, c)
    local t = { left = a, middle = b, right = c }
    local k1, v1 = original_next(t)
    if k1 == nil then return nil end
    local k2, v2 = original_next(t, k1)
    if k2 == nil then return k1, v1, nil end
    local k3, v3 = original_next(t, k2)
    if k3 == nil then return k1, v1, k2, v2, nil end
    local k4 = original_next(t, k3)
    return k1, v1, k2, v2, k3, v3, k4
end
local function step(t, control)
    return original_next(t, control)
end
local function write(t, key, value)
    t[key] = value
    return t
end
local function copy(t)
    local out = {}
    for k, v in original_pairs(t) do out[k] = v end
    return out
end
local function factory(value)
    local function child() return value end
    return child
end
return { make = make, effect_make = effect_make, plain = plain, ordered = ordered, step = step, write = write, copy = copy, factory = factory }
