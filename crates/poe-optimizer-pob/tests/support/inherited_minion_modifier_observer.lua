-- Observe original List/AddMod calls. No modifier query or game method is replaced.
return function(h)
 local calcs=require("Modules.CalcBase")
 local deliver=h.original(h.upvalue(calcs.perform,"addMinionModifiers"),"Modules/CalcPerform.lua",1161)
 local refs={
  {calcs,"perform","Modules/CalcPerform.lua",1193},
  {common.classes.ModStore,"List","Classes/ModStore.lua",321},
  {common.classes.ModDB,"AddMod","Classes/ModDB.lua",31},
  {common.classes.ModList,"ListInternal","Classes/ModList.lua",309},
  {common.classes.ModDB,"ListInternal","Classes/ModDB.lua",391},
 }
 for _,r in ipairs(refs) do r[5]=h.original(r[1][r[2]],r[3],r[4]) end
 local function functionRef(f)
  local i=debug.getinfo(f,"S")
  return {path=i.source:gsub("\\","/"),first_line=i.linedefined,last_line=i.lastlinedefined,what=i.what}
 end
 local function raw(v,seen,depth)
  local kind=type(v)
  if kind=="nil" then return {kind="nil"} end
  if kind=="function" then local r=functionRef(v);r.kind="function_reference";return r end
  if kind~="table" then assert(kind=="number" or kind=="string" or kind=="boolean");return h.scalar(v) end
  depth=(depth or 0)+1;assert(depth<24,"bounded raw depth");seen=seen or {}
  assert(not seen[v],"cyclic modifier body");seen[v]=true
  local out,positions,count={},{},0
  for key,value in pairs(v) do
   count=count+1;assert(count<=8192,"bounded raw body")
   if type(key)=="number" then positions[#positions+1]={index=key,value=raw(value,seen,depth)}
   else assert(type(key)=="string");out[key]=raw(value,seen,depth) end
  end
  table.sort(positions,function(a,b)return a.index<b.index end)
  if #positions>0 then out._positions=positions end
  seen[v]=nil;return out
 end
 local storeIds,envIds,storeCount,envCount={},{},0,0
 local function storeId(store)
  if not store then return false end
  if not storeIds[store] then storeCount=storeCount+1;storeIds[store]=storeCount end
  return storeIds[store]
 end
 local function envId(env)
  if not envIds[env] then envCount=envCount+1;envIds[env]=envCount end
  return envIds[env]
 end
 local function candidates(store,wanted)
  wanted=wanted or {MinionModifier=true}
  local out,live,seen={},{},{}
  while store do
   assert(not seen[store] and #out<16,"bounded store ancestry");seen[store]=true
   local row={store_id=storeId(store),parent_store_id=storeId(store.parent),kind=store.mods and "keyed" or "list",records={},names={}}
   local function add(mod,position,bucket)
    row.names[mod.name]=(row.names[mod.name] or 0)+1
    if wanted[mod.name] then
     local entry={position=position,bucket=bucket,body=raw(mod)}
     row.records[#row.records+1]=entry
     live[#live+1]={store_id=row.store_id,position=position,bucket=bucket,mod=mod}
    end
   end
   if store.mods then
    local names={};for name in pairs(store.mods) do names[#names+1]=name end;table.sort(names)
    for _,name in ipairs(names) do for i,mod in ipairs(store.mods[name]) do add(mod,i,name) end end
   else for i,mod in ipairs(store) do add(mod,i,false) end end
   out[#out+1]=row;store=store.parent
  end
  return out,live
 end
 local function recipient(env,actor)
  if not actor then return {present=false} end
  local children={}
  for _,a in ipairs(actor.activeSkillList or {}) do
   children[#children+1]={effect_id=a.activeEffect.grantedEffect.id,owns_actor=a.actor==actor,
    summoner_owns_actor=a.summonSkill and a.summonSkill.minion==actor or false}
  end
  return {present=true,profile=actor.type,parent_is_player=actor.parent==env.player,
   is_selected_minion=actor==env.minion,source=h.recipientOccurrence(env,actor),children=children}
 end
 local function actorSkill(env,active)
  return {effect_id=active.activeEffect.grantedEffect.id,source=h.sourceOccurrence(active),
   actor_is_player=active.actor==env.player,minion=recipient(env,active.minion)}
 end
 local function groups(lists)
  local keys={};for name in pairs(lists or {}) do keys[#keys+1]=name end;table.sort(keys)
  local out={}
  for _,name in ipairs(keys) do
   local records={};for i,mod in ipairs(lists[name]) do records[#records+1]={position=i,body=raw(mod)} end
   out[#out+1]={key=name,store_id=storeId(lists[name]),records=records,parent_store_id=storeId(lists[name].parent)}
  end
  return out
 end
 local result={schema_version=1,kind="original-inherited-minion-modifier-delivery",perform_calls={},deliveries={},gates={},life_pool_calls={}}
 local performFrames,deliveryFrames,listFrames,addFrames,poolFrames={},{},{},{},{}
 local failure
 local function append(name,row)
  local list=result[name];assert(#list<32768,"bounded observation count");list[#list+1]=row
 end
 assert(debug.gethook()==nil)
 local function hook(event,line)
  local info=debug.getinfo(2,"fS");local f=info.func
  local pool=info.what=="Lua" and info.linedefined==3365 and info.source:gsub("\\","/"):sub(-#"Modules/CalcPerform.lua")=="Modules/CalcPerform.lua"
  local interesting=(event=="call" or event=="return") and (f==calcs.perform or f==deliver or f==common.classes.ModStore.List or f==common.classes.ModDB.AddMod or pool)
   or event=="line" and ((f==calcs.perform and (line==1844 or line==2037 or line==2038 or line==3117 or line==3397 or line==3398 or line==3406 or line==3407)) or (pool and (line==3372 or line==3373 or line==3374 or line==3383)))
  if failure or not interesting then return end
  local vars={};for i=1,192 do local n,v=debug.getlocal(2,i);if not n then break end;vars[n]=v end
  local caller=debug.getinfo(3,"fSl");local outer={}
  if event=="call" and f==deliver then
   for i=1,192 do local n,v=debug.getlocal(3,i);if not n then break end;outer[n]=v end
  end
  local ok,problem=xpcall(function()
   if f==calcs.perform then
    if event=="call" then
     local env=assert(vars.env);local row={perform_id=#result.perform_calls+1,env_id=envId(env),mode=env.mode,returned=false}
     append("perform_calls",row);performFrames[#performFrames+1]={row=row,env=env,pool_gates={}}
    elseif event=="return" then
     local frame=assert(performFrames[#performFrames]);assert(frame.env==vars.env)
     frame.row.returned=true;performFrames[#performFrames]=nil
    else
     local frame=assert(performFrames[#performFrames]);assert(frame.env==vars.env)
     if line==2038 then
      assert(frame.spectre_gate,"Spectre gate must be observed first").branch_entered=true
     elseif line==3398 or line==3407 then
      assert(frame.pool_gates[line-1],"pool gate must be observed first").branch_entered=true
     else
      local row={perform_id=frame.row.perform_id,env_id=envId(vars.env),line=line,mode=vars.env.mode}
      if line==1844 then row.recipient=recipient(vars.env,vars.env.minion);row.hostile=raw(vars.env.minion and vars.env.minion.hostile)
      elseif line==2037 then
       row.has_active_spectre=raw(vars.hasActiveSpectreSkill);row.skills={};row.branch_entered=false
       for ordinal,active in ipairs(vars.env.player.activeSkillList) do
        local set=vars.env.mode=="CALCS" and active.activeEffect.statSetCalcs or active.activeEffect.statSet
        if vars.env.mode=="CALCS" then set=active.activeEffect.statSetCalcs end
        local flags=set and set.skillFlags
        row.skills[#row.skills+1]={ordinal=ordinal,source=actorSkill(vars.env,active),effective_flags=raw(flags)}
       end
       frame.spectre_gate=row
      elseif line==3117 then row.buffs=groups(vars.buffs);row.recipient=recipient(vars.env,vars.env.minion)
      else
       local wanted=line==3397 and {TakenFromSpectresBeforeYou=true,TotalSpectreLife=true} or {TakenFromCompanionBeforeYou=true,TakenFromCompanionBeforeYouFromDeflected=true,TotalCompanionLife=true}
       assert(vars.modDB==vars.env.modDB);row.candidates=candidates(vars.modDB,wanted);row.branch_entered=false
       frame.pool_gates[line]=row
      end
      append("gates",row)
     end
    end
   elseif pool then
    local frame=assert(performFrames[#performFrames]);assert(frame.env==vars.env)
    if event=="call" then
     local row={pool_id=#result.life_pool_calls+1,perform_id=frame.row.perform_id,env_id=envId(vars.env),mode=vars.env.mode,include_skill=functionRef(vars.includeSkill),buffs=groups(vars.buffs),candidates={},returned=false}
     append("life_pool_calls",row);poolFrames[#poolFrames+1]={row=row,env=vars.env}
    elseif event=="return" then
     local p=assert(poolFrames[#poolFrames]);assert(p.env==vars.env);p.row.returned=true;poolFrames[#poolFrames]=nil
    else
     local p=assert(poolFrames[#poolFrames]);local active=assert(vars.activeSkill)
     if line==3372 then
      if not p.active or p.active~=active then
       local row={source=actorSkill(vars.env,active),flags=raw(vars.skillFlags),seen=raw(vars.minion and vars.seenMinions[vars.minion]),included_branch_entered=false,nonselected_branch_entered=false}
       p.row.candidates[#p.row.candidates+1]=row;p.current=row;p.active=active
      end
     elseif line==3373 then assert(p.active==active);p.current.included_branch_entered=true
     elseif line==3374 then assert(p.active==active);p.current.nonselected_branch_entered=vars.minion~=vars.env.minion
     else assert(p.active==active);p.current.buff_loop=groups(vars.buffs) end
    end
   elseif f==deliver then
    if event=="call" then
     local frame=assert(performFrames[#performFrames]);local env=frame.env
     assert(outer.env==env and (caller.currentline==1854 or caller.currentline==3123 or caller.currentline==3377 or caller.currentline==3384),"unreviewed MinionModifier caller")
     local inPerform=caller.currentline==1854 or caller.currentline==3123
     if inPerform then assert(caller.func==calcs.perform,"original perform caller")
     else assert(caller.linedefined==3365 and poolFrames[#poolFrames] and poolFrames[#poolFrames].env==env,"original Life-pool caller") end
     local active=(caller.currentline==1854 or caller.currentline==3123) and env.player.mainSkill or outer.activeSkill
     assert(active and active.minion==vars.minion and vars.skillCfg==active.skillCfg,"exact source/recipient relation")
     local fromSkill=caller.currentline==1854 or caller.currentline==3377
     assert(not fromSkill or vars.modList==active.skillModList,"exact source Skill store")
     if not fromSkill then assert(vars.modList==outer.modList,"exact buff store") end
     local stores,live=candidates(vars.modList)
     local row={delivery_id=#result.deliveries+1,perform_id=frame.row.perform_id,env_id=envId(env),mode=env.mode,line=caller.currentline,
      source=actorSkill(env,active),recipient=recipient(env,vars.minion),cfg=h.sourceConfig(active,vars.skillCfg),
      caller=functionRef(caller.func),pool_id=not inPerform and poolFrames[#poolFrames].row.pool_id or false,
      from_skill=fromSkill,source_stores=stores,recipient_store_id=storeId(vars.minion.modDB),list_return={},added={},returned=false}
     append("deliveries",row);deliveryFrames[#deliveryFrames+1]={row=row,store=vars.modList,minion=vars.minion,live=live,cfg=vars.skillCfg}
    else
     local frame=assert(deliveryFrames[#deliveryFrames]);assert(frame.store==vars.modList and frame.minion==vars.minion and frame.row.original_list_return_observed)
     frame.row.source_stores_after=candidates(vars.modList);frame.row.returned=true;deliveryFrames[#deliveryFrames]=nil
    end
   elseif f==common.classes.ModStore.List and caller and caller.func==deliver then
    local frame=assert(deliveryFrames[#deliveryFrames]);assert(vars.self==frame.store and vars.cfg==frame.cfg)
    if event=="call" then
     frame.row.original_list_call_count=(frame.row.original_list_call_count or 0)+1
     assert(frame.row.original_list_call_count==1,"one original List per delivery");listFrames[#listFrames+1]=frame
    else
     assert(listFrames[#listFrames]==frame and caller.currentline==1162 and vars.n==1)
     assert(not frame.row.original_list_return_observed,"duplicate original List return")
     frame.return_values=assert(vars.result)
     for index,value in ipairs(assert(vars.result)) do
      local matches={};for _,candidate in ipairs(frame.live) do
       if candidate.mod.value==value then matches[#matches+1]={store_id=candidate.store_id,position=candidate.position,bucket=candidate.bucket} end
      end
      frame.row.list_return[#frame.row.list_return+1]={position=index,body=raw(value),exact_raw_value_candidates=matches}
     end
     frame.row.original_list_return_observed=true;listFrames[#listFrames]=nil
    end
   elseif f==common.classes.ModDB.AddMod and caller and caller.func==deliver then
    local frame=assert(deliveryFrames[#deliveryFrames]);assert(vars.self==frame.minion.modDB and caller.currentline==1164,"exact receiving AddMod")
    if event=="call" then
     local matched={};for index,value in ipairs(assert(frame.return_values)) do
      if value.mod==vars.mod then matched[#matched+1]=index end
     end
     local row={position=#frame.row.added+1,body=raw(vars.mod),returned_value_matches=matched,before_count=#(vars.self.mods[vars.mod.name] or {})}
     frame.row.added[#frame.row.added+1]=row;addFrames[#addFrames+1]={row=row,store=vars.self,mod=vars.mod}
    else
     local added=assert(addFrames[#addFrames]);assert(added.store==vars.self and added.mod==vars.mod)
     local bucket=assert(vars.self.mods[vars.mod.name]);assert(#bucket==added.row.before_count+1 and bucket[#bucket]==vars.mod,"original append identity")
     added.row.after_count=#bucket;added.row.original_append_observed=true;addFrames[#addFrames]=nil
    end
   end
  end,debug.traceback)
  if not ok then failure=problem end
 end
 debug.sethook(hook,"crl")
 return function()
  assert(debug.gethook()==hook);debug.sethook();assert(not failure,failure)
  assert(#performFrames==0 and #deliveryFrames==0 and #listFrames==0 and #addFrames==0 and #poolFrames==0,"unmatched observed call")
  for _,r in ipairs(refs) do assert(r[1][r[2]]==r[5]) end
  assert(h.upvalue(calcs.perform,"addMinionModifiers")==deliver)
  result.original_methods={};for _,r in ipairs(refs) do result.original_methods[#result.original_methods+1]=functionRef(r[5]) end
  result.original_methods[#result.original_methods+1]=functionRef(deliver)
  result.original_functions_preserved=true;result.observer_removed=true;result.business_methods_replaced=false
  result.snapshot=h.offeringOutputSnapshot()
  result.scope={native_coverage_authorized=false,all_minion_store_writers_accounted=false,
   original_list_and_addmod_observed=true,queries_added=false,raw_parent_chains_retained=true,
   direct_spectre_and_hostile_enemy_writes_accounted=false}
  inheritedMinionModifierObservation=result
 end
end
