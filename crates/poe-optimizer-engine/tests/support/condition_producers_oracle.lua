-- Test construction/observation only. Every query and tag consumer below is the
-- unchanged, authenticated ModStore/ModDB/ModList implementation loaded by the host.
local trace_records, completed_records = {}, {}
local util = require('jit.util')

function condition_oracle_jit(enabled)
	jit.flush()
	if not enabled then jit.off(); return end
	jit.on()
	jit.opt.start('hotloop=2', 'hotexit=2')
	jit.attach(function(trace, func)
		local info = util.funcinfo(func)
		if info and info.source then
			trace_records[trace] = trace_records[trace] or {}
			trace_records[trace][info.source .. ':' .. tostring(info.linedefined)] = true
		end
	end, 'record')
	jit.attach(function(event, trace)
		if event == 'stop' and util.traceinfo(trace) then
			for name in pairs(trace_records[trace] or {}) do completed_records[name] = true end
		end
	end, 'trace')
end

function condition_oracle_traces()
	local out = {}
	for name in pairs(completed_records) do out[#out + 1] = name end
	table.sort(out)
	return out
end

local function modifier(input)
	local out = {}
	for key, value in pairs(input) do if key ~= 'tags' then out[key] = value end end
	for i, tag in ipairs(input.tags or {}) do out[i] = tag end
	return out
end

function condition_oracle_prepare(input)
	local stores, actors = {}, {}
	for i, raw in ipairs(input.stores) do
		local kind = raw.store_type or 'ModDB'
		assert(kind == 'ModDB' or kind == 'ModList')
		stores[i] = new(kind)
		stores[i][kind](stores[i])
		stores[i].conditions = raw.conditions or {}
		for _, mod in ipairs(raw.mods or {}) do stores[i]:AddMod(modifier(mod)) end
	end
	for i, raw in ipairs(input.actors) do
		actors[i] = {
			modDB = stores[raw.store], output = raw.output or {},
			weaponData1 = raw.weapon_one, weaponData2 = raw.weapon_two,
		}
	end
	for i, raw in ipairs(input.actors) do
		for role, index in pairs(raw.links or {}) do actors[i][role] = actors[index] end
	end
	for i, raw in ipairs(input.stores) do
		stores[i].actor = actors[raw.actor]
		stores[i].parent = raw.parent and stores[raw.parent] or false
	end
	local db, cfg = stores[input.root], input.cfg
	local query = input.query
	if query.kind == 'flag' then
		return function() return db:Flag(cfg, unpack(query.names)) end
	elseif query.kind == 'condition' then
		return function() return db:GetCondition(query.variable, cfg, query.no_mod) end
	elseif query.kind == 'sum' then
		return function() return db:Sum(query.operation, cfg, unpack(query.names)) end
	elseif query.kind == 'eval' then
		local mod = modifier(query.mod)
		return function() return db:EvalMod(mod, cfg) end
	end
	error('unknown test query kind')
end

function condition_oracle_run(query, repetitions)
	local value
	for _ = 1, repetitions do value = query() end
	return value
end
