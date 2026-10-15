-- Offline construction evidence only. Original methods and callbacks run unchanged.
-- Full raw modifier roots are retained before any eligibility queries; no zero
-- aggregate or function-reference observation constitutes native source closure.
return function(h)
 local calcs=require("Modules.CalcBase")
 local refs={
  {calcs,"initModDB","Modules/CalcSetup.lua",26},
  {calcs,"initEnv","Modules/CalcSetup.lua",717},
  {calcs,"buildModListForNode","Modules/CalcSetup.lua",200},
  {calcs,"buildModListForNodeList","Modules/CalcSetup.lua",415},
  {calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
  {common.classes.ConfigTab,"BuildModList","Classes/ConfigTab.lua",1169},
  {common.classes.ConfigTab,"CreateConfigSet","Classes/ConfigTab.lua",1317},
 }
 for _,r in ipairs(refs) do r[5]=h.original(r[1][r[2]],r[3],r[4]) end
 local function functionRef(f)
  local i=debug.getinfo(f,"S")
  return {kind="callback_reference",what=i.what,path=i.source:gsub("\\","/"),
   first_line=i.linedefined,last_line=i.lastlinedefined,execution_not_implied=true}
 end
 -- Mixed numeric/string keys and repeated references are explicit. A captured
 -- function is a body reference, never a false/absent modifier or executed proof.
 local function raw(v,path,seen,depth)
  local t=type(v)
  if t=="function" then return functionRef(v) end
  if t~="table" then
   assert(t=="nil" or t=="number" or t=="string" or t=="boolean","unsupported raw value "..t)
   return h.scalar(v)
  end
  depth=(depth or 0)+1;assert(depth<32,"bounded raw modifier depth")
  seen=seen or {};path=path or "$"
  if seen[v] then return {kind="table_reference",path=seen[v]} end
  seen[v]=path
  local names,positions={},{ }
  for k in pairs(v) do
   assert(#names+#positions<16384,"bounded raw modifier size")
   if type(k)=="number" then positions[#positions+1]=k
   else assert(type(k)=="string","unsupported raw key");names[#names+1]=k end
  end
  table.sort(names);table.sort(positions)
  local out={}
  for _,k in ipairs(names) do out[k]=raw(v[k],path.."."..k,seen,depth) end
  if #positions>0 then
   out._positions={}
   for _,k in ipairs(positions) do out._positions[#out._positions+1]={index=k,value=raw(v[k],path.."["..k.."]",seen,depth)} end
  end
  return out
 end
 local function records(list)
  local out={};for index,mod in ipairs(list or {}) do out[#out+1]={position=index,body=raw(mod)} end
  return out
 end
 local function store(db)
  local out,seen={},{}
  while db do
   assert(not seen[db] and #out<16,"bounded store ancestry");seen[db]=true
   local buckets={};for name,list in pairs(db.mods or {}) do buckets[name]=records(list) end
   out[#out+1]={buckets=buckets,records=records(db),conditions=raw(db.conditions),multipliers=raw(db.multipliers)}
   db=db.parent
  end
  return out
 end
 local function inventory(t,project)
  local names={};for k in pairs(t or {}) do names[#names+1]=k end
  table.sort(names,function(a,b)return tostring(a)<tostring(b) end)
  local out={};for _,k in ipairs(names) do out[#out+1]={key=k,value=project(t[k],k)} end;return out
 end
 local function effect(e,group)
  return {id=e.grantedEffect and e.grantedEffect.id,fields=h.scalars(e),
   source_instance=h.scalars(e.srcInstance),source=h.sourceOccurrence({activeEffect=e,socketGroup=group}),
   granted_effect_fields=h.scalars(e.grantedEffect)}
 end
 local function skill(env,a)
  local effects,supports={},{}
  for _,e in ipairs(a.effectList or {}) do effects[#effects+1]=effect(e,a.socketGroup) end
  for _,e in ipairs(a.supportList or {}) do supports[#supports+1]=effect(e,a.socketGroup) end
  local buffs={};for k,b in pairs(a.buffList or {}) do buffs[k]={fields=h.scalars(b),records=records(b.modList)} end
  local stat=a.activeEffect and (env.mode=="CALCS" and a.activeEffect.statSetCalcs or a.activeEffect.statSet)
  return {mode=env.mode,actor_profile=a.actor and a.actor.type,actor_is_player=a.actor==env.player,
   actor_parent_is_player=a.actor and a.actor.parent==env.player,fields=h.scalars(a),
   active_effect=effect(assert(a.activeEffect),a.socketGroup),source=h.sourceOccurrence(a),
   summoner=a.summonSkill and h.sourceOccurrence(a.summonSkill),socket_group=h.scalars(a.socketGroup),
   effects=effects,supports=supports,skill_flags=raw(stat and stat.skillFlags),
   skill_data=raw(a.skillData),records=records(a.skillModList),buffs=buffs}
 end
 local function node(n)
  local callbacks={};for k,v in pairs(n) do if type(v)=="function" then callbacks[k]=functionRef(v) end end
  return {fields=h.scalars(n),stats=raw(n.stats),description=raw(n.sd),
   records=records(n.modList),keystone=raw(n.keystoneMod),callbacks=callbacks}
 end
 local function sockets(env)
  local spec=env.spec
  return {saved_jewels=raw(spec.jewels),
   ui_sockets=inventory(env.build.itemsTab.sockets,function(s)return h.scalars(s) end),
   radius_jewels=inventory(env.radiusJewelList,function(r)
    return {fields=h.scalars(r),item_id=r.item and r.item.id,item_fields=h.scalars(r.item),
     records=records(r.item and r.item.modList),nodes=inventory(r.nodes,function(n)return {id=n.id,type=n.type} end),data=raw(r.data)} end),
   extra_radius_nodes=inventory(env.extraRadiusNodeList,node),
   granted_passives=raw(env.grantedPassives)}
 end
 local function items(env)
  return inventory(env.player.itemList,function(item)
   return {fields=h.scalars(item),records=records(item.modList),base_records=records(item.baseModList),
    buff_records=records(item.buffModList),jewel_data=raw(item.jewelData),
    socketed_items=raw(item.sockets),rune_fields=raw(item.runes)}
  end)
 end
 local result={schema_version=1,kind="original-mana-supplier-construction",baseline={},configuration={},
  callback_invocations={},node_construction={},node_lists={},skill_construction={},environments={}}
 local function append(name,row)
  local out=result[name];assert(#out<32768,"bounded supplier observation count");out[#out+1]=row
 end
 local varList=h.upvalue(common.classes.ConfigTab.BuildModList,"varList")
 local callbackMap={};result.configuration_catalogue={}
 for index,v in ipairs(varList) do
  local row={index=index,fields=h.scalars(v),list=raw(v.list),default_state=raw(v.defaultState),
   default_placeholder=raw(v.defaultPlaceholderState),apply=v.apply and functionRef(v.apply)}
  result.configuration_catalogue[#result.configuration_catalogue+1]=row
  if v.apply then callbackMap[v.apply]=true end
 end
 local function config(c)
  local selected=c.configSets[c.activeConfigSetId]
  return {active_set=c.activeConfigSetId,input=raw(selected.input),placeholder=raw(selected.placeholder),
   custom_blocks=raw(selected.customModsList),default_state=raw(c.defaultState),
   player_records=records(c.modList),enemy_records=records(c.enemyModList)}
 end
 local pending,activeFrames={},{};local failure
 assert(debug.gethook()==nil)
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  local interested=event=="line" and (f==calcs.initEnv and line==908
    or f==calcs.buildActiveSkillModList and line==1035)
   or event=="return" and (f==calcs.buildModListForNode or f==calcs.buildModListForNodeList
    or f==calcs.buildActiveSkillModList or f==calcs.initEnv or f==common.classes.ConfigTab.BuildModList)
   or event=="call" and f==calcs.buildActiveSkillModList
   or (event=="call" or event=="return") and callbackMap[f]
  if failure or not interested then return end
  local vars={};for i=1,192 do local n,v=debug.getlocal(2,i);if not n then break end;vars[n]=v end
  local caller=debug.getinfo(3,"fSl");local parent={}
  if event=="call" and callbackMap[f] and caller.func==common.classes.ConfigTab.BuildModList then
   for i=1,128 do local n,v=debug.getlocal(3,i);if not n then break end;parent[n]=v end
  end
  local ok,problem=xpcall(function()
   if event=="call" and f==calcs.buildActiveSkillModList then
    local active=assert(vars.activeSkill)
    assert(not activeFrames[active],"recursive construction of the same exact skill")
    activeFrames[active]={captured=false}
   elseif event=="call" and callbackMap[f] then
    if caller.func~=common.classes.ConfigTab.BuildModList then return end
    local v=assert(parent.varData);local c=assert(parent.self)
    pending[#pending+1]={func=f,tab=c,row={var=v.var,kind=v.type,body=functionRef(f),
     argument_fields=h.scalars(vars),before=config(c)}}
   elseif event=="return" and callbackMap[f] then
    local p=pending[#pending]
    if p and p.func==f then
     pending[#pending]=nil;p.row.after=config(p.tab);append("callback_invocations",p.row)
    end
   elseif f==common.classes.ConfigTab.BuildModList then
    append("configuration",config(assert(vars.self)))
   elseif f==calcs.initEnv and event=="line" then
    local env=assert(vars.env)
    append("baseline",{mode=env.mode,class_id=env.classId,class_fields=h.scalars(vars.classStats),
     class_body=raw(vars.classStats),player_store=store(env.modDB),enemy_store=store(env.enemyDB),
     before_configuration_merge=true})
   elseif f==calcs.buildModListForNode then
    local env=assert(vars.env)
    append("node_construction",{mode=env.mode,node=node(assert(vars.node)),
     include_keystone_mods=vars.includeKeystoneMods,inc_small_passive_skill=vars.incSmallPassiveSkill,
     output_records=records(vars.modList),radius_jewel_count=#env.radiusJewelList})
   elseif f==calcs.buildModListForNodeList then
    local env=assert(vars.env)
    append("node_lists",{mode=env.mode,finish_jewels=vars.finishJewels,include_keystone_mods=vars.includeKeystoneMods,
     nodes=inventory(vars.nodeList,function(n)return {id=n.id,fields=h.scalars(n)} end),
     output_records=records(vars.modList),sockets=sockets(env)})
   elseif f==calcs.buildActiveSkillModList then
    local active=assert(vars.activeSkill);local frame=assert(activeFrames[active])
    -- The while header fires after every routed modifier too. Only its first
    -- visit is a pre-routing inventory; subsequent visits are intermediate state.
    if event=="return" or not frame.captured then
     local row=skill(assert(vars.env),active)
     row.phase=event=="line" and "before-effect-routing" or "after-construction"
     append("skill_construction",row);frame.captured=true
    end
    if event=="return" then activeFrames[active]=nil end
   elseif f==calcs.initEnv then
    local env=assert(vars.env)
    append("environments",{mode=env.mode,class_id=env.classId,
     nodes=inventory(env.allocNodes,node),items=items(env),sockets=sockets(env),
     player_store=store(env.modDB),item_store=store(env.itemModDB),
     party_fields=h.scalars(env.build.partyTab),party_records=records(env.build.partyTab.modList)})
   end
  end,debug.traceback)
  if not ok then failure=problem end
 end
 debug.sethook(hook,"crl")
 return function()
  local ours=debug.gethook()==hook;debug.sethook();assert(ours,"supplier hook replaced")
  assert(not failure,failure);assert(#pending==0,"unmatched original callback return")
  assert(next(activeFrames)==nil,"unmatched original skill construction return")
  for _,r in ipairs(refs) do assert(r[1][r[2]]==r[5],"original supplier function replaced") end
  result.original_methods={}
  for _,r in ipairs(refs) do result.original_methods[#result.original_methods+1]=functionRef(r[5]) end
  result.snapshot=h.offeringOutputSnapshot()
  result.original_functions_preserved=true;result.observer_removed=debug.gethook()==nil
  result.scope={unchanged_build_only=true,native_coverage_authorized=false,callbacks_not_executed_are_unresolved=true,
   captured_callbacks_are_body_references=true,raw_roots_and_routed_results_are_distinct=true,
   initial_construction_is_not_universal_absence=true,unrelated_input_obligations_preserved=true}
  manaSupplierObservation=result
 end
end
