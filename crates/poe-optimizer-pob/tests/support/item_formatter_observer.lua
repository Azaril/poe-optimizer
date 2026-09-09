-- Observation only: all calculations below call authenticated, unchanged source.
local base = {
 value = itemLib.formatValue,
 scalar = itemLib.applyValueScalar,
 range = itemLib.applyRange,
 catalyst = itemPolicy.scalar,
}
local function pack(...) return {n=select('#',...),...} end
local function clone(value,seen)
 if type(value)~='table'then return value end
 seen=seen or{};if seen[value]then return seen[value]end
 local out={};seen[value]=out
 for key,child in pairs(value)do out[clone(key,seen)]=clone(child,seen)end
 return out
end
function formatter_observe(operation, args)
 local original = modLib.parseMod
 local calls = {}
 modLib.parseMod = function(text, combined, ...)
  local mods, extra = original(text, combined, ...)
  calls[#calls+1] = {text=text, combined=combined==true, modifiers=clone(mods), extra=extra}
  return mods, extra
 end
 local result = pack(pcall(assert(base[operation]), unpack(args,1,args.n)))
 modLib.parseMod = original
 return {ok=result[1],value=result[1] and result[2] or nil,error=not result[1] and tostring(result[2])or nil,calls=calls}
end
-- Observes original format dispatch. The only replaced callback records arguments
-- and immediately invokes the unchanged original numeric/text formatter.
function formatter_assignment(formats)
 local oldData,oldFormat = data.modScalability,itemLib.formatValue
 local observation
 data.modScalability={['Oracle #']={{isScalable=true,formats=formats}}}
 itemLib.formatValue=function(value,baseScalar,scalar,precision,display,required)
  observation={precision=precision,display_precision=display,if_required=required}
  return oldFormat(value,baseScalar,scalar,precision,display,required)
 end
 local ok,err=pcall(base.range,'Oracle 1.2345',1,1)
 data.modScalability,itemLib.formatValue=oldData,oldFormat
 assert(ok,err);assert(observation,'source did not invoke formatValue')
 return observation
end
function formatter_capture_load(xml)
 local oldRange=itemLib.applyRange
 local calls={}
 itemLib.applyRange=function(...)
  local args=pack(...)
  local result=formatter_observe('range',args)
  calls[#calls+1]={arguments=args,result=result}
  if not result.ok then error(result.error)end
  return result.value
 end
 local ok,loaded=pcall(item_loading_load,xml,false)
 itemLib.applyRange=oldRange
 assert(ok,loaded)
 return {loaded=loaded,formats=calls}
end
-- Completed traces must both record original ItemTools functions and remain live.
function formatter_warm()
 jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1')
 local records,completed={},{}
 local util=require('jit.util')
 local function record(id,func)
  local info=util.funcinfo(func)
  if info and info.source and info.source=='@src/Modules/ItemTools.lua' then
   records[id]=records[id]or{}
   records[id][info.source..':'..tostring(info.linedefined)]=true
  end
 end
 local function trace(event,id)
  if event=='stop'then completed[id]=true
  elseif event=='abort'then records[id]=nil end
 end
 jit.attach(record,'record');jit.attach(trace,'trace')
 -- Its neutral branch is traceable; gsub callback paths may stay interpreted.
 for i=1,192 do base.scalar('Oracle 12 values',1,1,1)end
 for i=1,192 do
  base.value(i/7,1.25,1.3,100,2,true)
  base.scalar('Oracle 12.5 plus 40 values',1.2,1.1,2,1)
  base.range('Adds (10-20) to (30-40) Physical Damage',i%100/100,1.2,1.1)
  base.range('+17.5 to evasion rating',1,1.1,1.1)
 end
 jit.attach(record);jit.attach(trace)
 local out={}
 for id in pairs(completed)do
  if util.traceinfo(id)then
   for name in pairs(records[id]or{})do out[name]=true end
  end
 end
 return out
end

-- Explicit frozen feedback tests the original formatter consumer, not ModParser.
function formatter_feedback(args, modifiers, extra)
 local previous=modLib.parseMod
 modLib.parseMod=function()return modifiers,extra end
 local ok,result=pcall(formatter_observe,'range',args)
 modLib.parseMod=previous
 assert(ok,result)
 return result
end
