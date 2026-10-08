-- Original Player Life consumer acquisition uses JIT-off debug observations.
-- Reference execution in either JIT mode has no observer and no method wrappers.
local M = {}
local calcs, methods, refs, where, auth
local nonfinite_marker="__poe_optimizer_nonfinite_number"
local names = {"Life", "ExtraLife", "LifeTotal", "LifeConvertToEnergyShield",
 "LifeConvertToArmour", "LifeConvertToEvasion", "ChaosInoculation",
 "LowLifePercentage", "FullLifePercentage", "Multiplier:Level"}
local function plain(value, depth)
 local kind=type(value)
 if kind~="table" then
  assert(kind=="nil" or kind=="number" or kind=="string" or kind=="boolean")
  if kind=="number" and (value~=value or value==math.huge or value==-math.huge) then
   return {[nonfinite_marker]=value~=value and "nan" or value==math.huge and "positive_infinity" or "negative_infinity"}
  end
  return value
 end
 assert(rawget(value,nonfinite_marker)==nil,"reserved evidence marker present in source table")
 depth=(depth or 0)+1;assert(depth<32)
 local out={};for k,v in pairs(value) do
  assert(type(k)=="string" or type(k)=="number");out[k]=plain(v,depth)
 end;return out
end
-- Expose this evidence-only encoder for ordinary tests without loading PoB.
M.encode_evidence_value=plain
local function optional(value) return {present=value~=nil,kind=type(value),value=plain(value)} end
local function scalar(value)
 local out={};for k,v in pairs(value or {}) do
  if type(v)=="number" or type(v)=="string" or type(v)=="boolean" then out[k]=plain(v) end
 end;return out
end
local function equal(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function original(f,path,first)
 local i=debug.getinfo(f,"S")
 assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path and i.linedefined==first)
 return {path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function identify()
 calcs=require("Modules.CalcBase")
 methods={
  {"life",calcs,"doActorLifeManaSpirit","Modules/CalcDefence.lua",74},
  {"perform",calcs,"perform","Modules/CalcPerform.lua",1193},
  {"defence",calcs,"defence","Modules/CalcDefence.lua",789},
  {"sum",common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
  {"multi",common.classes.ModDB,"SumInternalMulti","Classes/ModDB.lua",185},
  {"more",common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
  {"flag",common.classes.ModDB,"FlagInternal","Classes/ModDB.lua",297},
  {"override",common.classes.ModDB,"OverrideInternal","Classes/ModDB.lua",344},
 }
 refs,where={},{}
 for _,m in ipairs(methods) do refs[m[1]]=m[2][m[3]];where[m[1]]=original(refs[m[1]],m[4],m[5]) end
 refs.round=round;where.round=original(round,"Modules/Common.lua",722)
end
local function preserved()
 for _,m in ipairs(methods) do assert(m[2][m[3]]==refs[m[1]]) end
 assert(round==refs.round)
end
local function stores(db)
 local rows,seen={},{}
 while db do
  assert(not seen[db] and #rows<16);seen[db]=true
  local buckets={};for _,name in ipairs(names) do buckets[name]=plain(db.mods[name] or {}) end
  rows[#rows+1]={buckets=buckets,conditions=scalar(db.conditions),multipliers=scalar(db.multipliers)}
  db=db.parent
 end;return rows
end
local function selected()
 return {items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,
  config=build.configTab.activeConfigSetId,spec=build.treeTab.activeSpec,
  main_group=build.mainSocketGroup,character_level=build.characterLevel}
end
local function snapshot()
 local modes={}
 for _,mode in ipairs({"MAIN","CALCS"}) do
  local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  local actor=assert(env.player);assert(actor.modDB==env.modDB and actor.modDB.actor==actor)
  modes[mode]={class_id=env.classId,level=actor.level,stores=stores(actor.modDB),output=scalar(actor.output),
   availability={life=optional(actor.output.Life),override=optional(actor.output.LifeHasOverride),
    chaos_inoculation=optional(actor.output.ChaosInoculation),full_life=optional(actor.modDB.conditions.FullLife)}}
 end
 return {selected=selected(),modes=modes}
end
local function env_for(actor)
 local result
 for level=3,40 do
  if not debug.getinfo(level,"f") then break end
  for i=1,160 do local n,v=debug.getlocal(level,i);if not n then break end
   if n=="env" and type(v)=="table" and rawget(v,"player")==actor then
    assert(not result or result==v,"ambiguous original Player environment");result=v
   end
  end
 end
 return result
end
function M.begin(expectedJit)
 assert(expectedJit==false and debug.gethook()==nil and jit.status()==false);identify()
 auth={calls={},frames={},finished=false};local failure
 -- Each named local is still in scope at the original line97, after its pinned
 -- assignment and the final Life expression. Reading it is not re-evaluation.
 -- In particular conv is the original post-clamp local, not the raw Sum return.
 local inputs={
  {"low_life_percentage","lowLifePerc",80},{"full_life_percentage","fullLifePerc",82},
  {"base","base",88},{"extra","extra",89},{"total","total",90},
  {"increase","inc",91},{"more","more",92},{"conversion","conv",93},{"override","override",94},
 }
 local function hook(event,line)
  if failure then return end
  local f=debug.getinfo(2,"f").func
  if f~=refs.life and f~=refs.round then return end
  if f==refs.round and event~="call" then return end
  local caller=debug.getinfo(3,"fSl")
  if f==refs.round and (not caller or caller.func~=refs.life or caller.currentline~=96) then return end
  local vars,bindings={},{};for i=1,160 do local n,v=debug.getlocal(2,i);if not n then break end
   vars[n]=v;bindings[n]=true
  end
  local consumer
  if f==refs.round then
   -- Observe only the original final resource expression. Breakdown rounding,
   -- other resources and unrelated callers are outside this acquisition.
   consumer={};for i=1,160 do local n,v=debug.getlocal(3,i);if not n then break end
    consumer[n]=v
   end
   if consumer.res~="Life" then return end
  end
  local env=f==refs.life and event=="call" and env_for(vars.actor) or nil
  local ok,problem=xpcall(function()
   if f==refs.round then
    local frame=assert(auth.frames[#auth.frames],"original Life rounding without consumer frame")
    if frame.row then
     assert(consumer.actor==frame.actor and consumer.modDB==frame.db and consumer.output==frame.output)
     assert(frame.env.player==frame.actor and frame.db.actor==frame.actor)
     assert(bindings.val and bindings.dec,"missing original round parameters")
     assert(type(vars.val)=="number","original Life rounding operand is not numeric")
     frame.rounding_count=frame.rounding_count+1
     assert(frame.rounding_count==1,"repeated original Life rounding call")
     frame.row.rounding={consumer_line=96,invoked=true,exact_consumer_frame=true,
      argument=optional(vars.val),decimal_parameter=optional(vars.dec),result_observed=false}
     -- No original return value is inferred from debug temporaries or from
     -- output.Life: the latter has already passed through the resource minimum.
    end
   elseif event=="call" then
    local frame={}
    if env then
     assert(vars.actor==env.player and vars.actor.modDB==env.modDB and env.modDB.actor==env.player)
     assert(caller and (caller.func==refs.perform or caller.func==refs.defence),"unexpected Player Life caller")
     frame={env=env,actor=vars.actor,db=env.modDB,output=vars.actor.output,rounding_count=0,
      row={mode=env.mode,caller={path=caller.func==refs.perform and "Modules/CalcPerform.lua" or "Modules/CalcDefence.lua",line=caller.currentline},
       selected=selected(),exact_existing_player=true,exact_actor_store=true,skip_breakdown=optional(vars.skipBreakdown),
       entry=stores(env.modDB),consumer_checkpoint_reached=false,
       rounding={consumer_line=96,invoked=false,result_observed=false}}}
    end
    auth.frames[#auth.frames+1]=frame
   else
    local frame=assert(auth.frames[#auth.frames],"original Life event without call")
    if frame.row then
      assert(vars.actor==frame.actor and frame.actor.modDB==frame.db and frame.actor.output==frame.output)
       if event=="line" and line==97 and vars.res=="Life" then
        -- A truthy original override bypasses the expression and round itself.
        -- Preserve this skipped call instead of inventing an operand or zero.
        local skipped=not not vars.override
        assert(frame.rounding_count==(skipped and 0 or 1),"original Life rounding admission mismatch")
        frame.row.rounding.short_circuited_by_override=skipped
        local values={}
        for _,input in ipairs(inputs) do
         assert(bindings[input[2]],"missing original consumer local "..input[2])
         values[input[1]]={assignment_reached=true,assignment_line=input[3],local_name=input[2],value=optional(vars[input[2]])}
        end
        values.chaos_inoculation={assignment_reached=true,assignment_line=85,output_field="ChaosInoculation",
         value=optional(vars.output.ChaosInoculation)}
        local row={line=line,inputs=values,life=vars.output.Life,has_override=vars.output.LifeHasOverride,
         stores=stores(frame.db)}
        -- Repeated notifications at the same source checkpoint carry no second
        -- computation. Every observed value must still agree exactly.
        if frame.row.computation then assert(equal(frame.row.computation,row),"changed repeated consumer checkpoint") end
        frame.row.consumer_checkpoint_reached=true
        frame.row.computation=row
       elseif event=="return" then
        assert(frame.row.consumer_checkpoint_reached and frame.row.computation,"original Life consumer checkpoint not reached")
        frame.row.return_life=frame.output.Life
        frame.row.return_chaos_inoculation=optional(frame.output.ChaosInoculation)
        frame.row.return_full_life=optional(frame.db.conditions.FullLife)
        frame.row.exit=stores(frame.db)
        auth.calls[#auth.calls+1]={env=frame.env,row=frame.row}
       end
    end
    if event=="return" then auth.frames[#auth.frames]=nil end
   end
  end,debug.traceback)
  if not ok then failure=problem end
 end
 debug.sethook(hook,"crl")
 return function()
  local same_hook=debug.gethook()==hook
  debug.sethook()
  assert(same_hook,"original Life observer ownership changed")
  assert(not failure,failure);assert(#auth.frames==0,"unfinished original Life frame")
  preserved();assert(jit.status()==expectedJit);auth.finished=true
 end
end
function M.observe(expectedJit)
 assert(debug.gethook()==nil and jit.status()==expectedJit)
 if not refs then identify() end;preserved()
 local result={snapshot=snapshot(),methods=where,
  instrumentation={consumer_observer_installed=auth~=nil,business_methods_wrapped=false,
   original_methods_preserved=true}}
 if auth then
  assert(auth.finished and #auth.calls>0)
  result.invocations={};result.current_modes={}
  for i,c in ipairs(auth.calls) do result.invocations[i]=c.row end
  for _,mode in ipairs({"MAIN","CALCS"}) do
   local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
   local calls={};for i,c in ipairs(auth.calls) do if c.env==env then calls[#calls+1]=i end end
   assert(#calls>0,"final Player environment was not observed")
   local last=auth.calls[calls[#calls]].row
   assert(last.return_life==env.player.output.Life)
   result.current_modes[mode]={invocations=calls,last=calls[#calls],exact_environment=true}
  end
 end
 return result
end
return M
