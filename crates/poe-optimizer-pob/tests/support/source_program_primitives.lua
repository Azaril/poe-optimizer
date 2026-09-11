-- Test-only complete consumers of original LuaJIT primitives and call ordering.
local minimum, maximum = math.min, math.max
local text, match = tostring, string.match
local probe = {}
function probe.minimum(...) return minimum(...) end
function probe.maximum(...) return maximum(...) end
function probe.text(...) return text(...) end
function probe.match(...) return match(...) end
function probe.first(value) return 100 + value end
function probe.second(value) return 200 + value end
function probe.mutate(self, target, replacement)
    self.count = self.count + 1
    target.fn = replacement
    return 2
end
function probe.call(target, side, replacement)
    return target.fn(side:mutate(target, replacement))
end
function probe.state(target, side, expected)
    return side.count, target.fn == expected
end
return probe
