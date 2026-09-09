-- Direct retained original bodies. Public cache is never consulted in these loops.
local cases=...
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1','maxtrace=4000')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and (info.source=='@src/Modules/ModParser.lua' or info.source=='@src/Modules/ModTools.lua') then
  recorded[id]=recorded[id]or{};recorded[id][info.source..':'..info.linedefined]={source=info.source,line=info.linedefined}
 end
end
local function trace(event,id)
 if event=='stop' then completed[id]=true elseif event=='abort' then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local results,executions={},0
for i,case in ipairs(cases)do
 local callback,args,result=case.callback,case.args
 -- The original nonvariadic bodies inspect fixed parameters only; explicit nil
 -- trailing arguments do not alter their values or evaluate synthetic expressions.
 for iteration=1,128 do
  result=callback(args[1],args[2],args[3],args[4],args[5],args[6])
  executions=executions+1
 end
 results[i]=copyTable(result)
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed)do
 if util.traceinfo(id)then
  for _,entry in pairs(recorded[id]or{})do live[#live+1]={id=id,line=entry.line,source=entry.source}end
 end
end
return {results=results,executions=executions,live=live}