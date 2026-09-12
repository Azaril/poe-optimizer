local lower, find, sub, select = string.lower, string.find, string.sub, select
local function pack(...) return { n = select("#", ...), ... } end
local function captured_lower(...) return lower(...) end
local function captured_find(...) return find(...) end
local function captured_sub(...) return sub(...) end
local function global_lower(...) return string.lower(...) end
local function global_find(...) return string.find(...) end
local function global_sub(...) return string.sub(...) end
local function method_lower(subject, ...) return subject:lower(...) end
local function method_find(subject, ...) return subject:find(...) end
local function method_sub(subject, ...) return subject:sub(...) end
local function warm_lower(subject)
    local result = pack(lower(subject))
    result.checked = true
    return result
end
local function warm_find(subject, pattern, start, plain)
    local result = pack(find(subject, pattern, start, plain))
    result.checked = true
    return result
end
local function warm_sub(subject, start, finish)
    local result = pack(sub(subject, start, finish))
    result.checked = true
    return result
end
local function effect(side, value)
    side.count = side.count + 1
    return value, nil, "ignored"
end
local function lower_effect(side, subject) return lower(subject, effect(side, "extra")) end
local function find_effect(side, subject, pattern, start, plain)
    return find(subject, pattern, start, plain, effect(side, "extra"))
end
local function sub_effect(side, subject, start, finish)
    return sub(subject, start, finish, effect(side, "extra"))
end
local function method_effect(side, subject) return subject:lower(effect(side, "extra")) end
local function old_method(receiver, value) return "old", receiver.tag, value end
local function new_method(receiver, value) return "new", receiver.tag, value end
local function replace(receiver, side)
    side.count = side.count + 1
    receiver.lower = new_method
    receiver.find = new_method
    receiver.sub = new_method
    return "argument"
end
local function method_order()
    local receiver = { lower = old_method, find = old_method, sub = old_method, tag = "receiver" }
    local side = { count = 0 }
    local first = pack(receiver:lower(replace(receiver, side)))
    receiver.find = old_method
    local second = pack(receiver:find(replace(receiver, side)))
    receiver.sub = old_method
    local third = pack(receiver:sub(replace(receiver, side)))
    return first, second, third, side.count, receiver.lower == new_method
end
local function failed_target(side, target)
    local receiver = { lower = target }
    return receiver:lower(effect(side, "extra"))
end
return {
    captured_lower = captured_lower, captured_find = captured_find, captured_sub = captured_sub,
    global_lower = global_lower, global_find = global_find, global_sub = global_sub,
    method_lower = method_lower, method_find = method_find, method_sub = method_sub,
    warm_lower = warm_lower, warm_find = warm_find, warm_sub = warm_sub,
    lower_effect = lower_effect, find_effect = find_effect, sub_effect = sub_effect,
    method_effect = method_effect, method_order = method_order, failed_target = failed_target,
}
