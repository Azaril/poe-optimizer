-- Test-only observations and input mutations. No game calculations are replaced.
local probe = {}
function probe.state(self, parent)
    local mods = {}
    for i, mod in ipairs(self) do
        mods[i] = mod
    end
    local proxy = self.ModStore
    return {
        object_alias = self.Object == self,
        proxy_object_alias = proxy._object == self,
        proxy_write_alias = proxy.__newindex == self,
        parent_initialized = self._parentInit[proxy._parent],
        proxy_index_type = type(proxy.__index),
        proxy_call_type = type(proxy.__call),
        proxy_class_name = proxy._className,
        parent_matches = self.parent == parent,
        parent_actor_alias = parent and self.actor == parent.actor or false,
        actor = self.actor,
        multipliers = self.multipliers,
        conditions = self.conditions,
        modifier_count = #self,
        mods = mods,
        forwarded = self.forwarded,
        addmod_type = type(self.AddMod),
    }
end
function probe.set(self, key, value)
    self[key] = value
end
function probe.proxy_write(self, value)
    self.ModStore.forwarded = value
    return self.forwarded, self.ModStore.forwarded
end
function probe.proxy_newmod(self, ...)
    return self.ModStore:NewMod(...)
end
function probe.proxy_read(self, key)
    return self.ModStore[key]
end
function probe.proxy(self)
    return self.ModStore
end
function probe.alias(self, other)
    return self == other
end
function probe.proxy_arrays(self)
    local proxy = self.ModStore
    local before = 0
    for i, mod in ipairs(proxy) do
        before = before + 1
    end
    table.insert(proxy, {name = "ProxyOnly", value = 123})
    return before, #proxy, #self, proxy[1].name, self[1].name
end
function probe.class_frontier(self)
    return self._superParents
end
return probe
