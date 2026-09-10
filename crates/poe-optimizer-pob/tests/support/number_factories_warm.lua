-- Direct retained source bodies and actual original C-function identity.
-- The public parser cache is not used. Dynamic cases vary raw inputs inside the loop.
local cases=...
local original_tonumber=tonumber
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1','maxtrace=4000')
local util=require('jit.util')
local completed,recorded,numeric={},{},{}
local function record(id,func)
 if func==original_tonumber then numeric[id]=true end
 local info=util.funcinfo(func)
 if info and (info.source=='@src/Modules/ModParser.lua' or info.source=='@src/Modules/ModTools.lua') then
  recorded[id]=recorded[id]or{};recorded[id][info.source..':'..info.linedefined]={source=info.source,line=info.linedefined}
 end
end
local function trace(event,id)
 if event=='stop' then completed[id]=true elseif event=='abort' then recorded[id]=nil;numeric[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local live,tonumber_live={},0
local function collect_live()
 for id in pairs(completed)do
  if util.traceinfo(id)then
   if numeric[id]then tonumber_live=tonumber_live+1 end
   for _,entry in pairs(recorded[id]or{})do live[#live+1]={id=id,line=entry.line,source=entry.source}end
  end
 end
end
jit.off(collect_live,true)
local results,dynamic,executions={},{},0
for i,case in ipairs(cases)do
 -- Independent cases avoid exhausting polymorphic side traces at one hot call.
 -- Every recorded prototype is required to be in a completed live trace here,
 -- before the next case flushes; simultaneous residency is not claimed.
 jit.flush();completed,recorded,numeric={},{},{}
 local callback,result=case.callback
 if case.inputs then
  local inputs=case.inputs;local outputs={};local count=#inputs
  for iteration=1,case.iterations do
   local at=(iteration-1)%count+1;local args=inputs[at]
   result=callback(args[1],args[2],args[3],args[4],args[5],args[6])
   outputs[at]=copyTable(result);executions=executions+1
  end
  dynamic[i]=outputs
 else
  local args=case.args
  for iteration=1,128 do
   result=callback(args[1],args[2],args[3],args[4],args[5],args[6]);executions=executions+1
  end
 end
 results[i]=copyTable(result)
 collect_live()
end
jit.attach(record);jit.attach(trace)
assert(tonumber==original_tonumber,'primitive binding changed')
return{results=results,dynamic=dynamic,executions=executions,live=live,tonumber_live=tonumber_live}
