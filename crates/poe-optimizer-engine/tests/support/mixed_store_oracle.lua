-- Test construction/observation only. Every query and tag consumer below is the
-- unchanged, authenticated ModStore/ModDB/ModList implementation loaded by the host.
-- A trace number is an attempt identifier, not permanent provenance: aborted
-- numbers are reused, and a full flush invalidates all previously compiled code.
local trace_records, completed_records = {}, {}
local events = { start = 0, stop = 0, abort = 0, flush = 0 }
local abort_reasons, aborted_functions, aborted_ids = {}, {}, {}
local reused_after_abort = 0
local util = require('jit.util')

local function record_event(trace, func)
	local records = trace_records[trace]
	local info = util.funcinfo(func)
	if records and info and info.source then
		records[info.source .. ':' .. tostring(info.linedefined)] = true
	end
end

local function trace_event(event, trace, func, pc, err)
	if events[event] ~= nil then events[event] = events[event] + 1 end
	if event == 'start' then
		trace_records[trace], completed_records[trace] = {}, nil
	elseif event == 'stop' then
		if util.traceinfo(trace) then
			completed_records[trace] = trace_records[trace]
			if aborted_ids[trace] then reused_after_abort = reused_after_abort + 1 end
		end
		trace_records[trace] = nil
	elseif event == 'abort' then
		local info = util.funcinfo(func)
		local reason = tostring(err) .. ' at ' .. tostring(info.source) .. ':' .. tostring(info.linedefined)
		abort_reasons[reason] = (abort_reasons[reason] or 0) + 1
		for name in pairs(trace_records[trace] or {}) do aborted_functions[name] = true end
		aborted_ids[trace] = true
		-- LuaJIT sends abort before removing its in-progress trace; traceinfo
		-- alone at this instant is therefore insufficient evidence of completion.
		trace_records[trace], completed_records[trace] = nil, nil
	elseif event == 'flush' then
		trace_records, completed_records, aborted_ids = {}, {}, {}
	end
end

function condition_oracle_jit(enabled)
	jit.flush()
	if not enabled then jit.off(); return end
	jit.on()
	jit.opt.start('hotloop=2', 'hotexit=2')
	jit.attach(record_event, 'record')
	jit.attach(trace_event, 'trace')
end

function condition_oracle_traces()
	local names, out = {}, {}
	for trace, records in pairs(completed_records) do
		if util.traceinfo(trace) then
			for name in pairs(records) do names[name] = true end
		end
	end
	for name in pairs(names) do out[#out + 1] = name end
	table.sort(out)
	return out
end

function condition_oracle_trace_state()
	local out = { pending = 0, live = 0, abort_reasons = abort_reasons,
		aborted_functions = {}, reused_after_abort = reused_after_abort }
	for name in pairs(aborted_functions) do out.aborted_functions[#out.aborted_functions + 1] = name end
	table.sort(out.aborted_functions)
	for event, count in pairs(events) do out[event] = count end
	for _ in pairs(trace_records) do out.pending = out.pending + 1 end
	for trace in pairs(completed_records) do
		if util.traceinfo(trace) then out.live = out.live + 1 end
	end
	return out
end

function condition_oracle_flush()
	jit.flush()
end

function condition_oracle_record_limit(limit)
	jit.opt.start('maxrecord=' .. tostring(limit))
end

-- Observation must neither start new traces nor record its own table traversal.
for _, observer in ipairs({record_event, trace_event, condition_oracle_jit,
	condition_oracle_traces, condition_oracle_trace_state, condition_oracle_flush,
	condition_oracle_record_limit}) do jit.off(observer, true) end

local function modifier(input)
	local out = {}
	for key, value in pairs(input) do if key ~= 'tags' then out[key] = value end end
	for i, tag in ipairs(input.tags or {}) do out[i] = tag end
    if input.nonfinite_value == 'nan' then out.value = 0/0
    elseif input.nonfinite_value == 'positive_infinity' then out.value = math.huge
    elseif input.nonfinite_value == 'negative_infinity' then out.value = -math.huge end
	return out
end

function condition_oracle_prepare(input)
	local stores, actors = {}, {}
    data.highPrecisionMods = {}
    for name, places in pairs(input.precision or {}) do data.highPrecisionMods[name] = {MORE=places} end
	for i, raw in ipairs(input.stores) do
		local kind = raw.store_type or 'ModDB'
		assert(kind == 'ModDB' or kind == 'ModList')
		stores[i] = new(kind)
		stores[i][kind](stores[i])
		stores[i].conditions = raw.conditions or {}
		for index, mod in ipairs(raw.mods or {}) do
            local value = modifier(mod)
            value._test_id = mod.id or (tostring(i) .. ':' .. tostring(index))
            stores[i]:AddMod(value)
        end
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
		return function() return db:GetCondition(query.variable, cfg, query.no_mod) end, db, cfg
	elseif query.kind == 'sum' then
		return function() return db:Sum(query.operation, cfg, unpack(query.names)) end, db, cfg
	elseif query.kind == 'more' then
        return function() return db:More(cfg, unpack(query.names)) end, db, cfg
    elseif query.kind == 'override' then
        return function() return db:Override(cfg, unpack(query.names)) end
    elseif query.kind == 'max' then
        return function() return db:Max(cfg, unpack(query.names)) end
    elseif query.kind == 'positive' then
        return function() return db:SumPositiveValues(query.operation, cfg, unpack(query.names)) end
    elseif query.kind == 'tabulate' then
        return function()
            local out = {__mixed_rows=true}
            for _, row in ipairs(db:Tabulate(query.operation, cfg, unpack(query.names))) do
                out[#out+1] = {id=row.mod._test_id, value=row.value}
            end
            return out
        end
    elseif query.kind == 'eval' then
		local mod = modifier(query.mod)
		return function() return db:EvalMod(mod, cfg) end, db, cfg
	end
	error('unknown test query kind')
end

function condition_oracle_run(query, repetitions)
	local value
	for _ = 1, repetitions do value = query() end
	return value
end

-- These independent monomorphic call loops prove the original consumers for
-- their stated inputs. They do not assert that an entire mixed-store tagged
-- chain was compiled together. That chain's numerical parity remains separate.
local function warm_sum(db, cfg, query, repetitions)
	local value
	for _ = 1, repetitions do value = db:Sum(query.operation, cfg, query.names[1]) end
	return value
end

local function warm_more(db, cfg, query, repetitions)
	local value
	for _ = 1, repetitions do value = db:More(cfg, query.names[1]) end
	return value
end

local function warm_condition(db, cfg, query, repetitions)
	local value
	for _ = 1, repetitions do value = db:GetCondition(query.variable, cfg, query.no_mod) end
	return value
end

local function warm_eval(db, cfg, query, repetitions)
	local value
	local mod = modifier(query.mod)
	for _ = 1, repetitions do value = db:EvalMod(mod, cfg) end
	return value
end

function condition_oracle_warm_source(input, repetitions, flush)
	local _, db, cfg = condition_oracle_prepare(input)
	if flush then jit.flush() end
	if input.query.kind == 'sum' or input.query.kind == 'more' then
		assert(#input.query.names == 1, 'fixed-argument control requires exactly one name')
		if input.query.kind == 'sum' then return warm_sum(db, cfg, input.query, repetitions) end
		return warm_more(db, cfg, input.query, repetitions)
	elseif input.query.kind == 'condition' then
		return warm_condition(db, cfg, input.query, repetitions)
	elseif input.query.kind == 'eval' then
		return warm_eval(db, cfg, input.query, repetitions)
	end
	error('unknown warmed source control')
end
jit.off(condition_oracle_warm_source, true)
