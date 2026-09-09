-- Direct calls to retained original ordinary factories, without public cache.
local cases=...
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1','maxtrace=4000')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and info.source=='@src/Modules/ModParser.lua' then
  recorded[id]=recorded[id]or{};recorded[id][info.linedefined]=true
 end
end
local function trace(event,id)
 if event=='stop' then completed[id]=true elseif event=='abort' then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local results,executions={},0
for i,case in ipairs(cases) do
 local callback,result=case.callback
 if case.prefix then
  for iteration=1,128 do result=callback();executions=executions+1 end
 else
  for iteration=1,128 do result=callback(12,'12','34','56','78','90');executions=executions+1 end
 end
 results[i]=copyTable(result)
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed) do
 if util.traceinfo(id) then
  for line in pairs(recorded[id]or{}) do live[#live+1]={id=id,line=line} end
 end
end
return {results=results,executions=executions,live=live}
