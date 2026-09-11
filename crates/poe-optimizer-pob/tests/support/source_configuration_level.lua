-- Test probes observe/mutate state; the calculation always calls the complete
-- original ConfigTab.UpdateLevel body. No substitute enemy-level recipe.
local probes = {}
function probes.result(self)
    local active = self.configSets[self.activeConfigSetId]
    return self.enemyLevel, self.input == active.input, self.placeholder == active.placeholder
end
function probes.set(self, input, placeholder, level)
    self.input.enemyLevel = input
    self.placeholder.enemyLevel = placeholder
    self.build.characterLevel = level
end
function probes.set_levels(self, input, placeholder)
    self.input.enemyLevel = input
    self.placeholder.enemyLevel = placeholder
end
function probes.cycle(self)
    return self.build.configTab == self
end
function probes.omitted(self)
    return self.controls
end
function probes.inherited(self)
    return self.UpdateLevel
end
function probes.call(self)
    return self()
end
return probes
