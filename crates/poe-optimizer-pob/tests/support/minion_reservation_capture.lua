-- Nonmutating call-boundary observations of the original reservation function.
-- No copied arithmetic supplies any captured intermediate or result.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local name=info.source:gsub("\\","/")
 assert(info.what=="Lua"and name:sub(-#path)==path and info.linedefined==line)
 return f
end
local reservation=original(calcs.doActorLifeManaSpiritReservation,"Modules/CalcDefence.lua",177)
local count=original(calcs.getActiveSkillCount,"Modules/CalcDefence.lua",149)
local rounding=original(round,"Modules/Common.lua",722)
local flooring=original(floor,"Modules/Common.lua",734)
local maximum=math.max
assert(not debug.gethook(),"unexpected existing debug hook")
local function locals(level)
 local out={};for index=1,80 do local name,value=debug.getlocal(level+1,index);if not name then break end;if name:sub(1,1)~="("then out[name]=value end end;return out
end
local function scalars(t)local out={};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then out[k]=v end end;return out end
local capture={actors={},records=0,invocations=0}
reservationCapture=capture
local selected={};for _,gem in ipairs(reservationReviewed)do selected[gem.primary_effect_id]=true end
local hook
hook=function(event)
 if event~="call"then return end
 local current=debug.getinfo(2,"f").func
 if current==reservation then
  capture.invocations=capture.invocations+1;assert(capture.invocations<=16384,"reservation invocation work bound")
  local actor=assert(locals(2).actor);capture.actors[actor]={};return
 end
 if current~=rounding and current~=flooring and current~=count and current~=maximum then return end
 local caller=debug.getinfo(3,"f");if not caller or caller.func~=reservation then return end
 local state=locals(3);local skill=state.activeSkill
 if not skill or not selected[skill.activeEffect.grantedEffect.id]then return end
 local actor=assert(state.actor);local actorRows=assert(capture.actors[actor])
 local row=actorRows[skill]
 if not row then
  capture.records=capture.records+1;assert(capture.records<=16384,"reservation capture work bound")
  row={pools={}};actorRows[skill]=row
 end
 if current==flooring then
  local args=locals(2);row.multiplier_floor={argument=args.val,decimals=args.dec};return
 end
 local name=state.name;if name~="Spirit"then return end
 local pool=row.pools[name]or{};row.pools[name]=pool
 if current==rounding then
  local args=locals(2)
  if args.dec==0 then
   pool.flat_round={argument=args.val,decimals=args.dec,base_flat_times_multiplier=state.baseFlatVal,values=scalars(state.values),multiplier=state.mult}
  elseif args.dec==2 then
   pool.percent_round={argument=args.val,decimals=args.dec,values=scalars(state.values),multiplier=state.mult}
  end
 elseif current==count then
  pool.before_count={values=scalars(state.values),multiplier=state.mult,exact_count_function=true}
 elseif current==maximum and state.minionFreeSpiritCount~=nil and state.activeSkillCount~=nil then
  pool.count_application={count=state.activeSkillCount,free=state.minionFreeSpiritCount,values=scalars(state.values)}
 end
end
debug.sethook(hook,"c")
return function()
 assert(debug.gethook()==hook,"reservation observer hook was replaced")
 debug.sethook()
 assert(calcs.doActorLifeManaSpiritReservation==reservation and calcs.getActiveSkillCount==count and round==rounding and floor==flooring and math.max==maximum)
 capture.hook_removed=debug.gethook()==nil
end
