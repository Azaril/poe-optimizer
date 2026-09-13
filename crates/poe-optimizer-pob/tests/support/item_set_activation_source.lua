-- Opt-in pre-call input projection, built from the existing authenticated
-- validity projector. No original function/iterator is called or replaced here.
local validity=...
local globals,ipairs=_G,ipairs
local rawget,rawset,next,type,rawequal,error=rawget,rawset,next,type,rawequal,error
local getmetatable=debug.getmetatable
local huge=math.huge
local function check(ok,message)if not ok then error('activation input observer: '..message,0)end end
return function()
 return {after_populate=true,activation_context=function(tab,event)
  local bound=validity.bind();check(rawequal(bound.receiver,tab),'exact receiver')
  local value=bound.projection()
  local build=rawget(tab,'build');local spec=rawget(build,'spec')
  local calcs=rawget(build,'calcsTab');check(calcs==nil or type(calcs)=='table','calcs type')
  local env=calcs and rawget(calcs,'mainEnv');check(env==nil or type(env)=='table','environment type')
  value.calculation_environment_present=env~=nil
  local count,bytes=0,0
  local function scalar(v)
   count=count+1;check(count<=32768,'supplemental value bound')
   check(type(v)=='nil' or type(v)=='boolean' or type(v)=='string' or (type(v)=='number' and v==v and v>-huge and v<huge),'finite supplemental scalar')
   if type(v)=='string' then bytes=bytes+#v;check(#v<=65536 and bytes<=2*1024*1024,'supplemental text bound') end
   return v
  end
  local class=rawget(rawget(rawget(globals,'common'),'classes'),'Item')
  for key,item in next,bound.items do
   local projected=rawget(value.items,key);check(type(projected)=='table','projected winning item')
   for _,field in ipairs({'id','name','jewelSocketCount'}) do
    local input=rawget(item,field)
    check(input~=nil or getmetatable(item)==nil or rawget(class,field)==nil,'inherited supplemental item input')
    rawset(projected,field,scalar(input))
   end
  end
  value.colors={};local colors=rawget(globals,'colorCodes');check(type(colors)=='table' and getmetatable(colors)==nil,'plain colors')
  for key,color in next,colors do value.colors[scalar(key)]=scalar(color) end
  value.nodeJewels={};local jewels=rawget(spec,'jewels');check(type(jewels)=='table' and getmetatable(jewels)==nil,'plain spec jewels')
  for key,item in next,jewels do check(type(key)=='number','numeric jewel key');value.nodeJewels[scalar(key)]=scalar(item) end
  value.scope.selected_context_after_import=false
  value.scope.actual_activation_entry=true
  value.scope.activation_call_ordinal=event.ordinal
  value.scope.supplemental_item_fields={'id','name','jewelSocketCount'}
  value.scope.actor_flag_values_not_observed=true
  bound.verify();return value
 end}
end
