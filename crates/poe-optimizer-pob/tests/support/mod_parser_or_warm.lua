jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and info.source=='@src/Data/Global.lua' then
  recorded[id]=recorded[id]or{}
  recorded[id][info.linedefined]=true
 end
end
local function trace(event,id)
 if event=='stop'then completed[id]=true elseif event=='abort'then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local results={}
for round=1,3 do
 for i,pair in ipairs(oracle_or_cases) do
  results[#results+1]={i,OR64(pair[1],pair[2])}
 end
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed)do
 if util.traceinfo(id)then
  for line in pairs(recorded[id]or{})do live[#live+1]={id=id,line=line}end
 end
end
return {results=results,live=live}
