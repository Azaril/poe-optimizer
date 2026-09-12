-- Complete callable source functions for native factory/cell differential tests.
-- Tables and domain inputs enter through arguments; factories allocate only closures.
local exports = {}
function exports.pair(x)
    local function get() return x end
    local function set(value) x = value end
    return get, set
end
function exports.forward(x)
    local function middle()
        return function(value) x = value end, function() return x end
    end
    return middle, function() return x end
end
function exports.recursive()
    local function factorial(n)
        if n == 0 then return factorial end
        if n <= 1 then return 1 end
        return n * factorial(n - 1)
    end
    return factorial
end
function exports.numeric_loop(out)
    for i = 1, 3 do
        local x = i
        out[i] = function(value)
            if value ~= nil then x = value end
            return x, i
        end
        i = i + 10
    end
end
function exports.generic_loop(out, input)
    local function step(_, index)
        index = index + 1
        if index <= 3 then return index, input[index] end
    end
    for key, value in step, nil, 0 do
        out[key] = function(replacement)
            if replacement ~= nil then value = replacement end
            return key, value
        end
        value = value + 10
    end
end
function exports.shared_outer(out)
    local x = 0
    for i = 1, 3 do
        out[i] = function(delta)
            if delta ~= nil then x = x + delta end
            return x
        end
    end
    x = 10
end
function exports.break_loop(out)
    local outer = 1
    for i = 1, 5 do
        out.get = function() return i, outer end
        if i == 2 then break end
        outer = outer + 1
    end
    outer = 10
end
function exports.return_loop()
    for i = 1, 5 do
        local x = i * 2
        if i == 2 then return function() return i, x end end
    end
end
function exports.branch(flag)
    if flag then
        local x = 3
        return function() return x end
    else
        local x = 7
        return function() return x end
    end
end
function exports.fail_escape(state)
    local x = 5
    state.get = function() return x end
    state.set = function(value) x = value end
    x = 9
    local missing = nil
    return missing.value
end
function exports.binary(state)
    local x = state.initial
    state.get = function() return x end
    local function change()
        x = state.changed
        state.effects = state.effects + 1
        return state.right
    end
    return x + change()
end
function exports.binary_computed(state)
    local x = state.initial
    state.get = function() return x end
    local function change()
        x = state.changed
        state.effects = state.effects + 1
        return state.right
    end
    return (x + 0) + change()
end
function exports.capture_binary()
    local x = 2
    local function change() x = 10; return 2 end
    local function run() return x + change() end
    local result = run()
    return result, x
end
function exports.addresses(a, b)
    local t = a
    local key = "old"
    local function change() t = b; key = "new"; return 7 end
    t[key] = change()
    return t == b, key, a.old, b.new
end
function exports.address_hazard(a, b)
    local t = a
    local key = "old"
    local function change() t = b; key = "new"; return 7, "written" end
    t[key], key = change()
    return t == b, key, a.old, b.old, b.new
end
function exports.read(state, key) return state[key] end
function exports.equal(a, b) return a == b end
return exports
