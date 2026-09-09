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
local inputs={'+7 to strength and dexterity while dual wielding','13% increased damage','Adds 2 to 5 Cold Damage to Attacks','Immune to Freeze and Shock'}
for round=1,128 do
 for _,text in ipairs(inputs)do
  oracle_public_cache[text]=nil
  oracle_public_parse(text)
 end
end
jit.attach(record);jit.attach(trace)
local live={}
for id in pairs(completed)do
 if util.traceinfo(id)then
  for line in pairs(recorded[id]or{})do live[#live+1]={id=id,line=line}end
 end
end
return live
