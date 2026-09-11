-- Complete source callbacks used by the interpreted LuaJIT/native iterator oracle.
local gmatch, select, type = string.gmatch, select, type
local held
local function factory(...) return gmatch(...) end
local function call(iterator, ...) return iterator(...) end
local function pack(...) return { n = select("#", ...), ... } end
local function packed_call(iterator, ...) return pack(iterator(...)) end
local function collect(subject, pattern)
    local rows = {}
    for first, second, third in subject:gmatch(pattern) do
        rows[#rows + 1] = { first, second, third }
    end
    return rows
end
local function split(iterator)
    local first, rest = {}, {}
    for value in iterator do first[#first + 1] = value break end
    for value in iterator do rest[#rest + 1] = value end
    return first, rest
end
local function identities(first, alias, second)
    local keys = {}
    keys[first] = "first"
    keys[second] = "second"
    return first == alias, first == second, type(first), keys[alias], keys[second]
end
local function wrap(iterator)
    local box = { first = iterator, alias = iterator }
    box[iterator] = "key"
    return box
end
local function alias_call(box) return box.first(), box.alias() end
local function wrapped_key(box) return box[box.first], box.first == box.alias end
local function hold(subject, pattern) held = gmatch(subject, pattern) return held end
local function held_call(...) return held(...) end
local function held_identity(iterator) return held == iterator end
local function effect(side, value) side.count = side.count + 1 return value, nil, "extra" end
local function factory_effect(side, subject, pattern) return gmatch(subject, pattern, effect(side, "ignored")) end
local function call_effect(iterator, side) return iterator(effect(side, "ignored")) end
local function colon_effect(receiver, side) return receiver:gmatch(effect(side, ".")) end
local function side_state(side) return side.count end
local function old_method(receiver, pattern) return "old", receiver.tag, pattern end
local function new_method(receiver, pattern) return "new", receiver.tag, pattern end
local function replace(target, side) side.count = side.count + 1 target.gmatch = new_method return "." end
local function target() return { gmatch = old_method, tag = "receiver" }, { count = 0 } end
local function iterator_target(iterator) return { gmatch = iterator, tag = "receiver" }, { count = 0 } end
local function lookup(target, side) return target:gmatch(replace(target, side)) end
local function target_state(target, side) return target.gmatch == new_method, side.count end
local function overwrite(target) target.gmatch = false end
local function function_read(iterator) return iterator.missing end
local function function_write(iterator) iterator.field = true end
return {
    factory = factory, call = call, packed_call = packed_call, collect = collect,
    split = split, identities = identities, wrap = wrap, alias_call = alias_call,
    wrapped_key = wrapped_key, hold = hold, held_call = held_call,
    held_identity = held_identity, factory_effect = factory_effect,
    call_effect = call_effect, colon_effect = colon_effect, side_state = side_state,
    target = target, iterator_target = iterator_target, lookup = lookup, target_state = target_state, overwrite = overwrite,
    function_read = function_read, function_write = function_write,
}
