-- Test-only entry-order observation. Loaded chunks keep their original bytes,
-- arguments, return packs and errors; no game callback or method is replaced.
local entries = {}
_configuration_source_modules = entries
local function entered(source)
    assert(#entries < 20000, "source module observation bound")
    entries[#entries + 1] = source
end
_configuration_source_module_enter = entered
local function observed(chunk)
    local source = debug.getinfo(chunk, "S").source
    return function(...)
        entered(source)
        return chunk(...)
    end
end
local originalLoadfile = loadfile
loadfile = function(...)
    local chunk, message = originalLoadfile(...)
    if not chunk then return chunk, message end
    return observed(chunk)
end
local originalLoader = package.loaders[2]
package.loaders[2] = function(...)
    local loader = originalLoader(...)
    if type(loader) == "function" then return observed(loader) end
    return loader
end
