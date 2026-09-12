local band, bor, bxor, bnot = bit.band, bit.bor, bit.bxor, bit.bnot
local api = {}
function api.modulo(a, b) return a % b end
function api.precedence(a, b, c) return a + b % c * 2, a % b % c, -a % b, a ^ 2 % b end
function api.band(...) return band(...) end
function api.bor(...) return bor(...) end
function api.bxor(...) return bxor(...) end
function api.bnot(...) return bnot(...) end
function api.rebound(...) return bit.bor(...) end
function api.replacement(...) return 'replacement', ... end
function api.same(a, b) return a == b end
function api.originals() return band, bor, bxor, bnot end
function api.write(t, k, v) t[k] = v end
local function bump(state, value)
    state.effects = state.effects + 1
    state.left = state.replacement
    return value
end
function api.modulo_effect(state, a, b) return a % bump(state, b) end
function api.modulo_indexed(state, b) return state.left % bump(state, b) end
function api.bit_effect(state, a, b) return band(a, bump(state, b)) end
function api.ignored_bnot_effect(state, a, b) return bnot(a, bump(state, b)) end
return api
