local args = reference_args
print(jit.version, jit.arch, _VERSION)
local fn, why = loadfile(args[1] .. "/Modules/Main.lua")
assert(fn, why)
local increment, err = loadstring("local count = 0; count += 1; return count")
assert(increment, err)
assert(increment() == 1)
print("Pinned Main.lua and augmented assignment parse successfully")