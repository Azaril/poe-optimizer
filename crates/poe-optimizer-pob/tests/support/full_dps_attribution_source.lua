-- Test-only observations of original Full DPS writes. No formula, method or
-- upvalue is replaced. Pointer joins precede serialization of source identities.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line)
 return f
end
local function upvalue(f,wanted)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==wanted then return v end end
 error("missing original upvalue "..wanted)
end
local function scalar(v)
 if type(v)=="number" and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end
 return v
end
local function scalars(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="number"or type(v)=="boolean"or type(v)=="string")then r[k]=scalar(v)end end;return r
end
local function plain(t,depth)
 if type(t)~="table"then return scalar(t)end
 depth=(depth or 0)+1;assert(depth<=16);local r={};local n=0
 for k,v in pairs(t)do n=n+1;assert(n<=4096);r[k]=plain(v,depth)end;return r
end
local function locals_at(level)
 local r={};for i=1,128 do local n,v=debug.getlocal(level+1,i);if not n then break end;r[n]=v end;return r
end
local function attrs_equal(a,b)
 for k,v in pairs(a)do assert(b[k]==v,"source attributes changed")end
 for k,v in pairs(b)do assert(a[k]==v,"source attributes changed")end
end
local function outputs_and_selection()
 local outputs={}
 for _,mode in ipairs({"MAIN","CALCS"})do
  local out=mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
  outputs[mode]={full_dps=out.FullDPS,full_dot_dps=out.FullDotDPS,skill_dps=plain(out.SkillDPS),scalars=scalars(out)}
 end
 return outputs,{skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,
  config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
end
if fullDpsPhase=="output_only"then
 assert(not debug.gethook()and jit.status()==fullDpsJit)
 local outputs,selection=outputs_and_selection();return {outputs=outputs,selection=selection}
end
if fullDpsPhase=="install"then
 assert(not debug.gethook() and jit.status()==fullDpsJit)
 local auth=fullDpsAuth
 if not auth then
  local doc,err=common.xml.ParseXML(fullDpsXml);assert(doc and not err)
  local ordinal,nextOrdinal={},0
  local function enumerate(n)
   if type(n)~="table"or not n.elem then return end
   assert(nextOrdinal<100000);ordinal[n]=nextOrdinal;nextOrdinal=nextOrdinal+1
   for _,child in ipairs(n)do enumerate(child)end
  end
  enumerate(doc[1]);local saved={}
  for _,skills in ipairs(doc[1])do if type(skills)=="table"and skills.elem=="Skills"then
   for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then
    local id=assert(tonumber(set.attrib.id));assert(not saved[id]);saved[id]={}
    for _,group in ipairs(set)do if type(group)=="table"and group.elem=="Skill"then
     local row={preset=id,source_ordinal=ordinal[group],attributes=scalars(group.attrib),gems={}}
     for _,gem in ipairs(group)do if type(gem)=="table"then row.gems[#row.gems+1]={source_ordinal=ordinal[gem],attributes=scalars(gem.attrib)}end end
     saved[id][#saved[id]+1]=row
    end end
   end end
  end end
  auth={saved=saved,groups={},gems={},loaded={},stages={}}
  fullDpsAuth=auth
 end
 local stage={calls={},load_captures=0};auth.stages[#auth.stages+1]=stage
 local stack={};local frames={};local events,lineEvents,captures,merges=0,0,0,0
 local methods={};local loadSkill=original(common.classes.SkillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
 local loadSkills=original(common.classes.SkillsTab.Load,"Classes/SkillsTab.lua",407)
 local function identity(skill,env)
  local effect=assert(skill.activeEffect);local source=effect.srcInstance;local group=skill.socketGroup
  local saved=source and auth.gems[source];local groupSaved=group and auth.groups[group]
  local itemId
  if group and group.sourceItem then for id,item in pairs(build.itemsTab.items)do if item==group.sourceItem then assert(not itemId);itemId=id end end;assert(itemId)end
  local selected=env.mode=="CALCS"and effect.statSetCalcs or effect.statSet
  local setIndex=selected and selected.index
  local set=selected and selected.statSet
  local setExact=set~=nil and effect.grantedEffect.statSets[setIndex]==set
  assert(setExact,"harvested action lacks an exact prepared stat-set declaration")
  return {source_ordinal=saved and saved.source_ordinal,group_source_ordinal=groupSaved and groupSaved.source_ordinal,
   preset=groupSaved and groupSaved.preset,source_object_joined=saved~=nil,group_object_joined=groupSaved~=nil,
   source=group and group.source,slot=group and group.slot,source_item_id=itemId,
   source_node_id=group and group.sourceNode and group.sourceNode.id,
   effect=effect.grantedEffect.id,stat_set=setIndex,stat_set_present=set~=nil,stat_set_declaration_exact=setExact,
   actor_is_player=skill.actor==env.player,actor_type=skill.actor.type,
   source_fields=scalars(source),group_fields=scalars(group)}
 end
 local function actor_identity(call,v,outputTable)
  local env=assert(v.usedEnv);local active=assert(v.activeSkill);local actor,kind
  if outputTable==env.player.output then actor=env.player;kind="player"
  elseif env.minion and outputTable==env.minion.output then actor=env.minion;kind="minion"
  elseif active.mirage and outputTable==active.mirage.output then actor=active.mirage;kind="mirage"end
  assert(actor,"harvested output has no exact actor")
  local action=actor.mainSkill
  return {kind=kind,actor_type=actor.type,source=identity(active,env),
   action=action and identity(action,env),actor_is_source_minion=active.minion==actor,
   source_skill_is_actor_main=active==action,exact_output_object=true}
 end
 local aggregateLines={ [395]="bleedDPS",[399]="corruptingBloodDPS",[403]="igniteDPS",
  [407]="burningGroundDPS",[411]="poisonDPS",[415]="causticGroundDPS",[419]="impaleDPS",
  [423]="decayDPS",[427]="dotDPS",[434]="cullingDPS" }
 local function mutation(call,frame,v,line)
  local full=v.fullDPS;if not full then return end
  local current=scalars(full)
  if frame.previous then
   local fields={};for field in pairs(current)do fields[#fields+1]=field end;table.sort(fields)
   for _,field in ipairs(fields)do
    local value=current[field]
    if frame.previous[field]~=value then
     local event={field=field,before=frame.previous[field],after=value,line=frame.line,actor=frame.actor}
     call.mutations[#call.mutations+1]=event;assert(#call.mutations<=8192)
     if frame.actor then
      call.contributors[field]=call.contributors[field]or{};table.insert(call.contributors[field],#call.mutations)
      if frame.line==182 or frame.line==190 then call.winners[field]=frame.actor end
     end
    end
   end
  end
  for i,row in ipairs(full.skills)do if not call.rowObjects[row]then
   local record={index=i,value=plain(row),line=frame.line}
   if frame.actor then record.actor=frame.actor;record.attribution="exact_harvested_actor"
   else
    local field=aggregateLines[frame.line];record.aggregate_field=field
    if field then record.attribution="observed_aggregate_write";record.winner=call.winners[field];record.contributors=plain(call.contributors[field]or{})
    else record.attribution="unresolved_aggregate_write"end
   end
   call.rows[#call.rows+1]=record;call.rowObjects[row]=record;assert(#call.rows<=4096)
  end end
  frame.previous=current;frame.line=line
  frame.actor=v.actor and call.actorRows[v.actor]or nil
 end
 local function hook(event,line)
  events=events+1;assert(events<=50000000,"Full DPS observer work bound")
  local functionInfo=debug.getinfo(2,"fS");local fn=functionInfo.func
  local fullFn=calcs.calcFullDPS
  local isFull=fn==fullFn and auth.loaded_complete
  local current=stack[#stack]
  local isMerge=current and fn==current.mergeFn
  if event=="call"and isFull then
   original(fullFn,"Modules/Calcs.lua",251)
   local merge=original(upvalue(fullFn,"mergeFullDPSPass"),"Modules/Calcs.lua",170)
   local capture=original(upvalue(fullFn,"captureFields"),"Modules/Calcs.lua",149)
   local v=locals_at(2);local outer="other"
   for level=3,32 do local info=debug.getinfo(level,"f");if not info then break end
    if info.func==calcs.buildOutput then outer=assert(locals_at(level).mode);break end
    if info.func==calcs.getMiscCalculator then outer="misc_calculator";break end
   end
   current={mode=v.mode,outer=outer,passes={},rows={},mutations={},comparisons={},winners={},contributors={},
    actorRows={},rowObjects={},captured={},mergeFn=merge,captureFn=capture,fullFn=fullFn,frame={}}
   stage.calls[#stage.calls+1]=current;stack[#stack+1]=current;assert(#stage.calls<=512 and #stack<=16)
   methods[fullFn]={merge=merge,capture=capture};debug.sethook(hook,"crl")
  elseif event=="call"and isMerge then
   merges=merges+1;assert(merges<=8192)
   local v=locals_at(2);local caller=locals_at(3);assert(caller.activeSkill)
   local pass={source=identity(caller.activeSkill,assert(caller.usedEnv or caller.fullEnv)),actors={}}
   for _,actor in ipairs(assert(v.pass).actors)do
    local observed=assert(current.captured[actor.out],"merge output lacks capture identity")
    local record={identity=observed.identity,values=plain(actor.out),count=actor.count,dot_scale=actor.dotScale}
    pass.actors[#pass.actors+1]=record
    current.actorRows[actor]={pass=#current.passes+1,actor=#pass.actors}
   end
   current.passes[#current.passes+1]=pass
   frames[#frames+1]={previous=scalars(v.fullDPS),line=170};debug.sethook(hook,"crl")
  elseif event=="return"and current and fn==current.captureFn then
   captures=captures+1;assert(captures<=16384)
   local v=locals_at(2);local caller=locals_at(3)
   assert(v.captured and not current.captured[v.captured])
   current.captured[v.captured]={identity=actor_identity(current,caller,v.output)}
  elseif event=="line"and(isFull or isMerge)then
   lineEvents=lineEvents+1;assert(lineEvents<=262144,"Full DPS line bound")
   local v=locals_at(2);local frame=isMerge and assert(frames[#frames])or current.frame
   mutation(current,frame,v,line)
   if isMerge and line==181 then
    current.comparisons[#current.comparisons+1]={actor=assert(current.actorRows[v.actor]),field=v.stat.target,
     candidate=scalar(v.value),prior=scalar(v.fullDPS[v.stat.target]),replacement_entered=false}
    frame.comparison=current.comparisons[#current.comparisons];assert(#current.comparisons<=8192)
   elseif isMerge and line==182 then assert(frame.comparison);frame.comparison.replacement_entered=true end
  elseif event=="return"and isMerge then
   mutation(current,assert(frames[#frames]),locals_at(2),198);frames[#frames]=nil
  elseif event=="return"and isFull then
   local v=locals_at(2);mutation(current,current.frame,v,439)
   current.result=plain(assert(v.fullDPS));current.sources=plain(v.sources)
   -- Build.lua subsequently sorts the shared MAIN reporting array in place.
   -- Keep its exact objects privately; calculation order is already snapshotted.
   current.resultSkills=v.fullDPS.skills
   current.cache_present=v.cacheStore~=nil;current.finished=true;stack[#stack]=nil
  elseif event=="return"and fn==loadSkills then
   auth.loaded_complete=true
  elseif event=="return"and fn==loadSkill then
   local v=locals_at(2)
   if v.node.elem=="Skill"then
    local sid=assert(v.skillSetId);local set=assert(v.self.skillSets[sid]);local index=#set.socketGroupList
    local group=assert(set.socketGroupList[index]);assert(group==v.socketGroup)
    local row=assert(auth.saved[sid][index]);attrs_equal(row.attributes,v.node.attrib)
    assert(not auth.groups[group]);auth.groups[group]=row;auth.loaded[#auth.loaded+1]=row
    assert(#group.gemList==#row.gems)
    for i,gem in ipairs(group.gemList)do assert(not auth.gems[gem]);auth.gems[gem]=row.gems[i]end
    stage.load_captures=stage.load_captures+1;assert(stage.load_captures<=4096)
   end
  end
  -- Line observation is confined to the two authenticated reporting functions;
  -- expensive nested calculation bodies retain only bounded call/return hooks.
  -- LuaJIT fast C calls (including ipairs) do not guarantee a paired return
  -- hook. Their bodies have no Lua source lines to suppress. Changing the mask
  -- for such a call would leave the reporting loop unobserved until a later
  -- Lua return, losing the actual contributor and max-branch attribution.
  if (event=="call"or event=="return")and functionInfo.what~="C"then
   local target=event=="call"and fn or(debug.getinfo(3,"f")or{}).func
   local c=stack[#stack];local wanted=c and(target==c.fullFn or target==c.mergeFn)
   debug.sethook(hook,wanted and"crl"or"cr")
  end
 end
 jit.flush();assert(jit.status()==fullDpsJit);debug.sethook(hook,"cr")
 return function()
  local present=debug.gethook();debug.sethook()
  assert(present==hook,"Full DPS observer hook changed")
  assert(#stack==0 and #frames==0,"unfinished Full DPS observation")
  assert(jit.status()==fullDpsJit and common.classes.SkillsTab.LoadSkill==loadSkill)
  assert(methods[calcs.calcFullDPS] and #stage.calls>=3,"missing original reporting calls")
  for f,m in pairs(methods)do assert(upvalue(f,"mergeFullDPSPass")==m.merge and upvalue(f,"captureFields")==m.capture)end
  stage.finished=true
 end
end
assert(fullDpsPhase=="observe"and not debug.gethook()and jit.status()==fullDpsJit)
local stage=assert(fullDpsAuth.stages[#fullDpsAuth.stages]);assert(stage.finished)
local calls={}
for _,c in ipairs(stage.calls)do
 assert(c.finished)
 local finalOrder,finalArrayExact
 if c.outer=="MAIN"or c.outer=="CALCS"then
  local output=c.outer=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
  assert(output.SkillDPS==c.resultSkills,"display replaced the exact reporting array")
  finalOrder={};local seen={}
  for index,row in ipairs(output.SkillDPS)do
   assert(index<=4096)
   local recorded=assert(c.rowObjects[row],"display row lacks an observed calculation object")
   assert(not seen[row],"display duplicates a calculation row object");seen[row]=true
   finalOrder[#finalOrder+1]={output_index=index,calculation_index=recorded.index,
    exact_row_object=true,value=plain(row)}
  end
  assert(#finalOrder==#c.rows,"display omitted an observed calculation row object")
  finalArrayExact=true
 end
 calls[#calls+1]={mode=c.mode,outer=c.outer,passes=c.passes,rows=c.rows,mutations=c.mutations,
  comparisons=c.comparisons,winners=c.winners,contributors=c.contributors,result=c.result,sources=c.sources,
  cache_present=c.cache_present,finished=true,final_order=finalOrder,final_array_object_exact=finalArrayExact}
end
local outputs,selection=outputs_and_selection()
return {calls=calls,outputs=outputs,loaded_sources=fullDpsAuth.loaded,original_functions_preserved=true,
 hook_removed=true,business_wrappers=false,formulas_reimplemented=false,
 selection=selection}
