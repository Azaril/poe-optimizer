-- Original string.find remains unmodified. Record only completed, still-live traces.
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1')
local util=require('jit.util')
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 recorded[id]=recorded[id]or{}
 if info and info.source then recorded[id][info.source..':'..tostring(info.linedefined)]=true end
end
local function trace(event,id)
 if event=='stop'then completed[id]=true elseif event=='abort'then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local count=0
for i=1,256 do
 local a,b=string.find('Alpha123omega','123',i%2+1,true)
 count=count+(a or 0)+(b or 0)
end
for i=1,256 do
 local a,b,c=string.find('Alpha123omega','(%d+)',i%2+1,false)
 count=count+(a or 0)+(b or 0)+#(c or '')
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed)do
 if util.traceinfo(id)then live[#live+1]={id=id,functions=recorded[id]or{}} end
end
return {count=count,live=live,version=jit.version}
