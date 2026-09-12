-- Original source fixtures for closure-cell lifetimes. No build-specific inputs.
local probes = {}
function probes.siblings()
    local x = 1
    local function get() return x end
    local function set(v) x = v end
    local before = get()
    set(7)
    return before, get()
end
function probes.independent_factories()
    local function make(x)
        return function() return x end, function(v) x = v end
    end
    local a, setA = make(1)
    local b, setB = make(1)
    local alias = a
    setA(7)
    setB(9)
    return a == b, a == alias, a(), b()
end
function probes.forwarded_capture()
    local x = 1
    local function outer()
        return function()
            return function(v) x = v end, function() return x end
        end
    end
    local middle = outer()
    local set, get = middle()
    set(12)
    return x, get()
end
function probes.shadowing_and_initializer()
    local x = 9
    local old = function() return x end
    do
        local x = function() return x end
        local inside = x()
        return old(), inside
    end
end
function probes.recursive_local()
    local function factorial(n)
        if n <= 1 then return 1 end
        return n * factorial(n - 1)
    end
    return factorial(5)
end
function probes.recursive_self_identity()
    local function self() return self end
    return self() == self, self()() == self
end
function probes.loop_numeric_visible()
    local functions = {}
    local shared = 0
    for i = 1, 3 do
        functions[i] = function() return i, shared end
        i = i + 10
        shared = shared + 1
    end
    local a, x = functions[1]()
    local b, y = functions[2]()
    local c, z = functions[3]()
    return a, b, c, x, y, z
end
function probes.loop_generic_visible()
    local values = {4, 5, 6}
    local functions = {}
    for k, v in ipairs(values) do
        functions[k] = function() return k, v end
        v = v + 10
    end
    local a, x = functions[1]()
    local b, y = functions[2]()
    local c, z = functions[3]()
    return a, b, c, x, y, z
end
function probes.loop_body_declarations()
    local functions = {}
    local setters = {}
    for i = 1, 3 do
        local x = i
        functions[i] = function() return x end
        setters[i] = function(v) x = v end
    end
    setters[2](20)
    return functions[1](), functions[2](), functions[3]()
end
function probes.loop_while_declarations()
    local functions = {}
    local i = 0
    while i < 3 do
        i = i + 1
        local x = i
        functions[i] = function() return x, i end
    end
    local a = functions[1]()
    local b = functions[2]()
    return a, b, functions[3]()
end
function probes.loop_repeat_declarations()
    local functions = {}
    local i = 0
    repeat
        i = i + 1
        local x = i
        functions[i] = function() return x end
    until x == 3
    return functions[1](), functions[2](), functions[3]()
end
function probes.break_closes_visible_binding()
    local f
    local outer = 1
    for i = 1, 5 do
        f = function() return i, outer end
        if i == 2 then break end
        outer = outer + 1
    end
    outer = 10
    return f()
end
function probes.return_closes_visible_binding()
    local function make()
        for i = 1, 5 do
            local x = i * 2
            if i == 2 then return function() return i, x end end
        end
    end
    return make()()
end
function probes.error_keeps_escaped_cells()
    local state = {}
    local ok, err = pcall(function()
        local x = 5
        state.get = function() return x end
        state.set = function(v) x = v end
        x = 9
        error("factory sentinel")
    end)
    local before = state.get()
    state.set(11)
    return ok, string.find(err, "factory sentinel", 1, true) ~= nil, before, state.get()
end
function probes.branch_scopes()
    local function make(flag)
        if flag then
            local x = 3
            return function() return x end
        else
            local x = 7
            return function() return x end
        end
    end
    return make(true)(), make(false)()
end
function probes.function_identity_keys()
    local function make() return function() return 1 end end
    local a = make()
    local b = make()
    local values = {}
    values[a] = 7
    values[b] = 11
    return a == b, values[a], values[b], a(), b()
end
function probes.local_capture_reassignment()
    local x = 2
    local function change() x = 10; return 2 end
    local r = x + change()
    return r, x
end
function probes.captured_nil_false_packs()
    local x = nil
    local function get() return x, false, nil end
    local function set(v) x = v end
    local a, b, c = get()
    set(false)
    local d, e, f = get()
    return a, b, c, d, e, f
end
function probes.outer_capture_is_eager()
    local x = 2
    local function change() x = 10; return 2 end
    local function run() return x + change() end
    local result = run()
    return result, x
end
return probes
