jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and info.source=='@src/Modules/ModParser.lua' then
  recorded[id]=recorded[id]or{}
  recorded[id][info.linedefined]=true
 end
end
local function trace(event,id)
 if event=='stop'then completed[id]=true elseif event=='abort'then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local patterns={alpha=1,omega=2,beta=3,['(%d+)']=4}
for i=1,256 do
 oracle_original_scan(i%2==0 and 'ALPHA 123 omega' or 'BETA 456 omega',patterns,true)
 oracle_original_scan(i%2==0 and 'ALPHA 123 omega' or 'BETA 456 omega',patterns,false)
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed)do
 if util.traceinfo(id)then
  for line in pairs(recorded[id]or{})do live[#live+1]={id=id,line=line}end
 end
end
return live
