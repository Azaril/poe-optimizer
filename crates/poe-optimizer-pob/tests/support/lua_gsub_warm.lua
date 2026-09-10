-- Original string.gsub remains the retained C function throughout. Completed
-- live traces establish warmed Lua hosts only, not tracing of C gsub internals.
local original,cases=...
assert(string.gsub==original)
jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1','maxtrace=1000')
local util=require('jit.util')
local primitive=util.funcinfo(original)
assert(primitive.ffid and primitive.ffid>0)
local completed,recorded={},{}
local function record(id,func)
 local info=util.funcinfo(func)
 if info and info.source=='@test-only-gsub-warm' then
  recorded[id]=recorded[id]or{}
  recorded[id][info.linedefined]={source=info.source,line=info.linedefined}
 end
end
local function trace(event,id)
 if event=='stop'then completed[id]=true elseif event=='abort'then recorded[id]=nil end
end
jit.attach(record,'record');jit.attach(trace,'trace')
local function drive(case)
 local outputs,checksum={},0
 for iteration=1,256 do
  local at=(iteration-1)%2+1
  local bytes,count=original(case.subjects[at],case.pattern,case.replacement,case.maximum)
  outputs[at]={bytes=bytes,substitutions=count}
  checksum=checksum+#bytes+count
 end
 return outputs,checksum
end
local live={}
local function collect(case)
 for id in pairs(completed)do
  if util.traceinfo(id)then
   for _,row in pairs(recorded[id]or{})do
    live[#live+1]={case=case,id=id,source=row.source,line=row.line}
   end
  end
 end
end
jit.off(collect,true)
local outputs,checksum,executions={},0,0
for index,case in ipairs(cases)do
 jit.flush();completed,recorded={},{}
 local count
 outputs[index],count=drive(case)
 checksum=checksum+count;executions=executions+256
 collect(index)
end
jit.attach(record);jit.attach(trace)
return{outputs=outputs,executions=executions,checksum=checksum,live=live,
 original_binding_unchanged=string.gsub==original,primitive_ffid=primitive.ffid,
 claims_c_internals_traced=false,version=jit.version}
