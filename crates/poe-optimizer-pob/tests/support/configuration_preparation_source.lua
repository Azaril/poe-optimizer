-- Observation only: every reached constructor, loader, callback, control setter,
-- modifier operation and calculator remains the original pinned implementation.
local trace = { events = {}, callbacks = {}, diagnostics = {} }
local originalError = launch.ShowErrMsg
launch.ShowErrMsg = function(self, fmt, ...)
    trace.diagnostics[#trace.diagnostics + 1] = string.format(fmt, ...)
    return originalError(self, fmt, ...)
end
local activeCallback, depth = nil, 0
local lifecycle = { constructed = {}, loaded = {} }
local function copy(value, seen, level)
	local kind = type(value)
	if kind ~= "table" then
		if kind == "number" and (value ~= value or value == math.huge or value == -math.huge) then return tostring(value) end
		if kind == "function" or kind == "userdata" or kind == "thread" then return "<" .. kind .. ">" end
		return value
	end
	seen, level = seen or {}, level or 0
	if seen[value] then return "<cycle>" end
	assert(level < 16, "configuration observation depth limit")
	seen[value] = true
	local out, count = {}, 0
	for key, item in pairs(value) do
		count = count + 1
		assert(count < 20000, "configuration observation width limit")
		if type(key) == "string" or type(key) == "number" then out[key] = copy(item, seen, level + 1) end
	end
	seen[value] = nil
	return out
end
local function mods(list)
	if not list then return nil end
	local out = {}
	for index, mod in ipairs(list) do out[index] = copy(mod) end
	return out
end
local function state(config, build)
	build = build or (config and config.build)
	local out = {
		input = config and copy(config.input), placeholder = config and copy(config.placeholder),
		defaultState = config and copy(config.defaultState), active = config and config.activeConfigSetId,
		enemyLevel = config and config.enemyLevel,
		modList = config and mods(config.modList), enemyModList = config and mods(config.enemyModList),
		customMods = config and copy(config.customModsList),
		build = build and {
			characterLevel = build.characterLevel, buildFlag = build.buildFlag,
			data = build.data ~= nil, config = build.configTab ~= nil, items = build.itemsTab ~= nil,
			tree = build.treeTab ~= nil, spec = build.spec ~= nil, skills = build.skillsTab ~= nil,
			calcs = build.calcsTab ~= nil, mainEnv = build.calcsTab and build.calcsTab.mainEnv ~= nil,
			mainOutput = build.calcsTab and build.calcsTab.mainOutput ~= nil,
			outputRevision = build.outputRevision,
			constructed = copy(lifecycle.constructed), loaded = copy(lifecycle.loaded),
		} or nil,
	}
	if config and config.configSetOrderList then
        out.order = {}
        for key, value in pairs(config.configSetOrderList) do out.order[tostring(key)] = value end
    end
    if config and config.configSets then
		out.sets = {}
		local active = config.configSets[config.activeConfigSetId]
		out.input_alias = active and config.input == active.input
		out.placeholder_alias = active and config.placeholder == active.placeholder
		for key, set in pairs(config.configSets) do
			out.sets[tostring(key)] = { input = copy(set.input), placeholder = copy(set.placeholder),
				customModsList = copy(set.customModsList), title = set.title }
		end
	end
	return out
end
local function event(kind, name, config, build, details)
	assert(#trace.events < 20000, "configuration event limit")
	trace.events[#trace.events + 1] = { kind = kind, name = name, depth = depth,
		callback = activeCallback, state = state(config, build), details = details }
end
local installed = {}
local function wrap(class, name, configMethod)
	local original = class[name]
	if type(original) ~= "function" then return end
	class[name] = function(self, ...)
		local args = {...}
		if name == "ConfigTab" then lifecycle = { constructed = {}, loaded = {} } end
		local config = configMethod and self or (self.build and self.build.configTab)
		local build = self.build or (configMethod and name == "ConfigTab" and args[1])
		local first = args[1]
		if type(first) == "table" then first = first.elem or "<table>" end
		local details = { argument = first, init = copy(args[2]), deferSync = copy(args[3]) }
		if name == "ConfigTab" then details = nil end
		event("enter", class._className .. "." .. name, config, build, details)
		depth = depth + 1
		local result = original(self, ...)
		if name == class._className then lifecycle.constructed[name] = true end
		if name == "Load" then lifecycle.loaded[class._className] = true end
		build = self.build or build
		config = configMethod and self or (build and build.configTab)
		depth = depth - 1
		event("exit", class._className .. "." .. name, config, build, details)
		return result
	end
end
local function install(name)
	if installed[name] then return end
	installed[name] = true
	local class = assert(common.classes[name])
	if name == "ConfigTab" then
		for _, method in ipairs({"ConfigTab", "CreateConfigSet", "Load", "SetActiveConfigSet", "UpdateControls", "UpdateLevel", "BuildModList", "ImportCalcSettings"}) do wrap(class, method, true) end
		for index, var in ipairs(require("Modules.ConfigOptions")) do
			if var.apply then
				local original = var.apply
				local info = debug.getinfo(original, "S")
				trace.callbacks[#trace.callbacks + 1] = {index=index, var=var.var, type=var.type,
					source=info.source, first=info.linedefined, last=info.lastlinedefined}
				var.apply = function(value, modList, enemyModList, build)
					local previous = activeCallback
					activeCallback = var.var
					event("enter", "apply", build.configTab, build, {index=index, var=var.var, type=var.type, value=copy(value)})
					depth = depth + 1
					local result = original(value, modList, enemyModList, build)
					depth = depth - 1
					event("exit", "apply", build.configTab, build, {index=index, var=var.var, type=var.type, value=copy(value)})
					activeCallback = previous
					return result
				end
			end
		end
	elseif name == "EditControl" then
		local original = class.SetPlaceholder
		class.SetPlaceholder = function(self, value, notify)
			if activeCallback then trace.events[#trace.events+1] = {kind="control", name="EditControl.SetPlaceholder", callback=activeCallback, depth=depth, details={value=value,notify=notify}} end
			return original(self, value, notify)
		end
	else
		wrap(class, name, false)
		wrap(class, "Load", false)
		wrap(class, "PostLoad", false)
		if name == "CalcsTab" then wrap(class, "BuildOutput", false) end
	end
end
local observed = {ConfigTab=true, ItemsTab=true, TreeTab=true, SkillsTab=true,
	CalcsTab=true, NotesTab=true, PartyTab=true, ImportTab=true, EditControl=true}
local originalNew = new
new = function(name, ...)
	local object = originalNew(name, ...)
	if observed[name] then install(name) end
	return object
end
_configuration_source_trace = trace
_configuration_source_state = state
