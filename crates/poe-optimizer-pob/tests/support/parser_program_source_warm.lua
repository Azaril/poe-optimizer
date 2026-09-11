-- Observation/driver only: f and constructor are retained complete original functions.
-- Pending attempts, aborted IDs, completed traces and flush lifetime are distinct.
local jit, util = require('jit'), require('jit.util')
local pending, completed = {}, {}
local events = {start=0,stop=0,abort=0,flush=0}
local function record(trace, func)
    if pending[trace] then pending[trace][func] = true end
end
local function lifecycle(event, trace)
    if events[event] ~= nil then events[event] = events[event] + 1 end
    if event == 'start' then
        pending[trace], completed[trace] = {}, nil
    elseif event == 'stop' then
        if util.traceinfo(trace) then completed[trace] = pending[trace] end
        pending[trace] = nil
    elseif event == 'abort' then
        pending[trace], completed[trace] = nil, nil
    elseif event == 'flush' then
        pending, completed = {}, {}
    end
end
local function snapshot(required)
    local found, live = {}, 0
    for id, functions in pairs(completed) do
        if util.traceinfo(id) then
            live = live + 1
            for name, func in pairs(required) do
                if functions[func] then found[name] = true end
            end
        end
    end
    return {found=found,live=live,start=events.start,stop=events.stop,abort=events.abort,flush=events.flush}
end
local function begin()
    jit.flush()
    jit.on()
    jit.opt.start('hotloop=2','hotexit=2')
    jit.attach(record,'record')
    jit.attach(lifecycle,'trace')
end
local function finish(required)
    local result=snapshot(required)
    jit.attach(record)
    jit.attach(lifecycle)
    jit.off()
    return result
end
-- A source body is invoked on every iteration; no public-parser cache is involved.
local function run(f,args)
    local value
    for i=1,3000 do value=f(args[1],args[2],args[3],args[4],args[5]) end
    return f(args[1],args[2],args[3],args[4],args[5])
end
for _,f in ipairs({record,lifecycle,snapshot,begin,finish}) do jit.off(f,true) end
return {begin=begin,finish=finish,run=run}
