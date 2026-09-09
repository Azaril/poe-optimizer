-- Test-owned driver: direct calls to retained, unchanged original factories.
-- No public parser or cache is consulted inside the loop.
local cases=...
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1','maxtrace=4000')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and (info.source=='@src/Modules/ModParser.lua' or info.source=='@src/Modules/ModTools.lua') then
  recorded[id]=recorded[id]or{}
  recorded[id][info.source..':'..info.linedefined]={source=info.source,line=info.linedefined}
 end
end
local function trace(event,id)
 if event=='stop' then completed[id]=true elseif event=='abort' then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local results,executions={},0
for i,callback in ipairs(cases) do
 local result
 for iteration=1,256 do
  result=callback(12,'12','34','56','78','90')
  executions=executions+1
 end
 results[i]=copyTable(result)
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed) do
 if util.traceinfo(id) then
  for _,entry in pairs(recorded[id]or{}) do
   live[#live+1]={id=id,source=entry.source,line=entry.line}
  end
 end
end
return {results=results,executions=executions,live=live}
