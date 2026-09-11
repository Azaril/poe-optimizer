-- The evaluator roots are complete parameterized source bodies. The driver below
-- only observes/warms them; it never substitutes their arithmetic.
local floor = math.floor
local functions = {}
function functions.power(a, b) return a ^ b end
function functions.floor(a, b, c) return floor(a, b, c) end
function functions.negative_base(a, b) return -a ^ b end
function functions.negative_exponent(a, b, c) return a ^ -b ^ c end
function functions.right_associative(a, b, c) return a ^ b ^ c end

return functions
