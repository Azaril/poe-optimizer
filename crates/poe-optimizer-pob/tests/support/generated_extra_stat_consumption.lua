-- Source-only, unchanged-input evidence at the actual ExtraSkillStat consumer.
-- No methods are wrapped, no query is repeated, and no result is manufactured.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line)
 return f
end
local merge=original(calcs.mergeSkillInstanceMods,"Modules/CalcActiveSkill.lua",116)
local builder=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local nodeBuilder=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200)
local nodeListBuilder=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)
local initEnv=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local storeClass,dbClass,itemClass=assert(common.classes.ModStore),assert(common.classes.ModDB),assert(common.classes.Item)
local scaleAdd=original(storeClass.ScaleAddMod,"Classes/ModStore.lua",82)
local addMod=original(dbClass.AddMod,"Classes/ModDB.lua",31)
local itemMods=original(itemClass.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local callback=original(runCallback,"HeadlessWrapper.lua",17)
local wanted={};for _,id in ipairs(extraConsumptionEffects) do wanted[id]=true end
local work=0
local function charge() work=work+1;assert(work<=4000000,"consumer observer work bound") end
local function keys(t)
 local result={};for k in next,t do charge()
  if type(k)~="string"and type(k)~="number"then error(debug.traceback("unsupported consumer raw key type "..type(k),2),0)end
  result[#result+1]=k
 end
 assert(#result<=16384);table.sort(result,function(a,b)if type(a)~=type(b)then return type(a)<type(b)end;return a<b end)
 return result
end
local function scalar(v)
 if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then
  return {kind="nonfinite_source_number",diagnostic=tostring(v)}
 end
 if v==nil then return {kind="absent"} end
 assert(type(v)=="number" or type(v)=="string" or type(v)=="boolean")
 return v
end
-- Tagged raw tables preserve sparse/numeric keys without JSON object-key loss.
-- Opaque values are explicit unresolved evidence, not absence or native rules.
local function plain(v,depth,seen)
 charge();local typ=type(v)
 if typ=="function" then
  local i=debug.getinfo(v,"S");local path=i.source:gsub("\\","/")
  path=path:match("(src/.*)$") or path:gsub("^@","")
  return {kind="unresolved_function",source=path,what=i.what,first=i.linedefined,last=i.lastlinedefined}
 end
 if typ~="table" then
  if typ=="userdata" or typ=="thread" then return {kind="unresolved_opaque",source_type=typ}end
  return scalar(v)
 end
 depth=(depth or 0)+1;assert(depth<=16,"consumer raw depth bound")
 seen=seen or{};if seen[v]then return {kind="unresolved_cycle"}end;seen[v]=true
 local fields={};for _,k in ipairs(keys(v))do fields[#fields+1]={key_type=type(k),key=k,value=plain(rawget(v,k),depth,seen)}end
 seen[v]=nil
 return {kind="raw_table",has_metatable=getmetatable(v)~=nil,fields=fields}
end
local function equal(a,b)
 if type(a)~=type(b)then return false end
 if type(a)~="table"then return a==b end
 for k,v in next,a do if not equal(v,b[k])then return false end end
 for k in next,b do if a[k]==nil then return false end end;return true
end
local function scalars(t)
 local r={};for _,k in ipairs(keys(t or{}))do local v=rawget(t,k)
  if type(k)=="string"and(type(v)=="string"or type(v)=="boolean"or type(v)=="number")then r[k]=scalar(v)end
 end;return r
end
local nested={SocketProperty=true,GroupProperty=true,NodeModifier=true,PlayerModifier=true,
 AllyModifier=true,MinionModifier=true,ExtraSkillMod=true,ExtraSkillStat=true,ExtraJewelFunc=true}
local function has_extra(v,seen,depth)
 if type(v)~="table"then return false end
 depth=(depth or 0)+1;assert(depth<=16);seen=seen or{};if seen[v]then return false end;seen[v]=true
 if rawget(v,"name")=="ExtraSkillStat"then return true end
 for _,k in ipairs(keys(v))do if has_extra(rawget(v,k),seen,depth)then return true end end
 return false
end
local function records(store,all)
 if not store then return {present=false,count=0,records={}}end
 assert(type(store)=="table")
 local out,total={},0;local mods=rawget(store,"mods")
 local function add(mod,index,bucket)
  charge();assert(type(mod)=="table");total=total+1;assert(total<=16384)
  if all or nested[mod.name]or has_extra(mod)then
   out[#out+1]={index=index,bucket=bucket,record=plain(mod),name=scalar(mod.name)}
  end
 end
 if mods then
  assert(type(mods)=="table")
  for _,name in ipairs(keys(mods))do for i,mod in ipairs(rawget(mods,name))do add(mod,i,name)end end
 else for i,mod in ipairs(store)do add(mod,i,"sequence")end end
 return {present=true,count=total,records=out,selection=all and"all_local_records"or"extra_stats_and_nested_supplier_records"}
end
local function chain(store,env,actor)
 local out,seen={},{}
 while store do
  charge();assert(type(store)=="table"and not seen[store]and #out<32);seen[store]=true
  local parent=rawget(store,"parent");assert(parent==nil or parent==false or type(parent)=="table")
  out[#out+1]={depth=#out,parent_kind=parent==nil and"absent"or parent==false and"false_sentinel"or"store",
   player=rawequal(store,env.player.modDB),actor=rawequal(store,actor.modDB),item=rawequal(store,env.itemModDB),local_records=records(store,false)}
  store=parent
 end;return out
end
-- PassiveSpec:65-68 uses the exact PassiveTree node as a table __index.
-- Authenticate that finite lookup; local absence is not effective absence.
local function node_inputs(env,node)
 local id=node.id;local tree=env.spec.tree.nodes[id]
 assert(rawequal(env.spec.nodes[id],node)and type(tree)=="table"and tree.id==id)
 assert(rawequal(getmetatable(node),tree)and rawequal(rawget(tree,"__index"),tree))
 local function field(name)
  local localValue=rawget(node,name);local inherited=rawget(tree,name)
  local expected=localValue;if expected==nil then expected=inherited end
  local actual=node[name];assert(rawequal(actual,expected),"unreviewed effective node lookup")
  return actual,{origin=localValue~=nil and"local"or inherited~=nil and"tree_inherited"or"absent",
   local_present=localValue~=nil,effective_present=actual~=nil,lookup_exact=true}
 end
 local mods,modLookup=field("modList");local keystone,keyLookup=field("keystoneMod")
 return {node_id=id,tree_node_id=tree.id,node_type=node.type,spec_node_exact=true,tree_metatable_exact=true,
  modifiers=records(mods,true),modifier_lookup=modLookup,keystone_mod=plain(keystone),keystone_lookup=keyLookup}
end
local function supplier(env,nodeReturns,amuletReturns)
 local items,nodes,overrides={},{},{}
 for _,slot in ipairs(keys(env.player.itemList))do
  local item=env.player.itemList[slot];local selected=build.itemsTab.activeItemSet[slot]
  items[#items+1]={slot=slot,id=item.id,source=scalar(item.modSource),type=scalar(item.type),
   saved_object_exact=rawequal(build.itemsTab.items[item.id],item),selected_item_id=scalar(selected and selected.selItemId),
   base=records(item.baseModList,true),active=records(item.modList,true),slot_lists=plain(rawget(item,"slotModList")),
   grants=plain(rawget(item,"grantedSkills"))}
 end
 for _,id in ipairs(keys(env.allocNodes))do
  local node=env.allocNodes[id]
  local effective=node_inputs(env,node)
  assert(equal(effective,node_inputs(env,node)),"effective node observer mutation")
  local returned=nodeReturns[env]and nodeReturns[env][id]
  assert(returned and #returned>0,"selected node needs actual original return evidence")
  local capturedReturns={};for i,row in ipairs(returned)do capturedReturns[i]=row end
  nodes[#nodes+1]={id=id,node_id=node.id,from_effective_spec=rawequal(env.spec.nodes[id],node),
   modifiers=records(rawget(node,"modList"),true),grants=plain(rawget(node,"grantedSkills")),
   effective_inputs=effective,build_returns=capturedReturns,
   return_order_scope="per-node call order and every modifier-list order exact; cross-node invocation order not captured"}
 end
 -- hashOverrides stores full PassiveSpec node graphs (SwitchAttributeNode:
 -- 2718-2727), not serialized modifier data. Preserve their semantic fields and
 -- effective node correspondence without walking graph adjacency/cache keys.
 for _,id in ipairs(keys(rawget(env.spec,"hashOverrides")or{}))do
  local node=env.spec.hashOverrides[id];local effective=env.spec.nodes[id]
  assert(type(node)=="table"and type(effective)=="table"and node.id==id and effective.id==id)
  overrides[#overrides+1]={id=id,node_id=node.id,name=scalar(node.dn),is_attribute=scalar(node.isAttribute),
   stats=plain(node.sd),modifiers=records(node.modList,true),
   effective_node_id=effective.id,effective_name=scalar(effective.dn),effective_stats=plain(effective.sd),
   effective_modifiers=records(effective.modList,true),allocated=env.allocNodes[id]~=nil,
   allocated_effective_node_exact=rawequal(env.allocNodes[id],effective),
   graph_topology_scope="not captured; no node graph or supplier-completeness authority"}
 end
 local party=build.partyTab
 local amulet={};for i,row in ipairs(amuletReturns[env]or{})do amulet[i]=row end
 return {mode=env.mode,axes={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
  passives=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId},items=items,nodes=nodes,
  item_store=records(env.itemModDB,false),player_store=records(env.player.modDB,false),
  attribute_overrides=overrides,radius_jewels=plain(env.radiusJewelList),amulet_transport=amulet,
  config={input=plain(env.configInput),placeholder=plain(env.configPlaceholder),defaults=plain(build.configTab.defaultState),
   input_exact=rawequal(env.configInput,build.configTab.input),placeholder_exact=rawequal(env.configPlaceholder,build.configTab.placeholder),
   records=records(build.configTab.modList,true),custom=plain(build.configTab.configSets[build.configTab.activeConfigSetId].customModsList)},
  party={actor=records(party.actor.modDB,true),enemy=records(party.enemyModList,true),
   aura=plain(party.actor.Aura),curse=plain(party.actor.Curse),warcry=plain(party.actor.Warcry),link=plain(party.actor.Link),
   export_enabled=scalar(party.enableExportBuffs)},
  unresolved_scope="callback/transform legality, alternative suppliers and dormant presets not certified"}
end
local function source_address(a)
 local group=a.socketGroup;local instance=a.activeEffect.srcInstance
 local matches={}
 for _,sid in ipairs(keys(build.skillsTab.skillSets))do
  local set=build.skillsTab.skillSets[sid]
  for gi,g in ipairs(set.socketGroupList)do if rawequal(g,group)then
   local positions={};for i,gem in ipairs(g.gemList)do if rawequal(gem,instance)then positions[#positions+1]=i end end
   matches[#matches+1]={preset=sid,group=gi,positions=positions}
  end end
 end
 assert(#matches==1 and #matches[1].positions==1,"exact saved source-instance address")
 return {runtime=matches[1],group=scalars(group),instance=scalars(instance),source_present=group.source~=nil,
  source=scalar(group.source),source_item_id=scalar(group.sourceItem and group.sourceItem.id),
  source_node_id=scalar(group.sourceNode and group.sourceNode.id)}
end
-- Compact, opt-in original-call evidence. This deliberately does not retain the
-- broad node/item supplier snapshots used by the unchanged-input witness.
local function focused_install(api)
 local list=original(storeClass.List,"Classes/ModStore.lua",321)
 local eval=original(storeClass.EvalMod,"Classes/ModStore.lua",490)
 local listClass=assert(common.classes.ModList)
 local listInternal=original(listClass.ListInternal,"Classes/ModList.lua",309)
 local dbInternal=original(dbClass.ListInternal,"Classes/ModDB.lua",391)
 local function preserved()
  return calcs.mergeSkillInstanceMods==merge and calcs.buildActiveSkillModList==builder
   and storeClass.List==list and storeClass.EvalMod==eval and listClass.ListInternal==listInternal and dbClass.ListInternal==dbInternal
   and runCallback==callback
 end
 assert(debug.gethook()==nil and preserved())
 local function args_at(level)
  local out={};for i=1,128 do local name,value=debug.getlocal(level+1,i);if not name then break end;out[name]=value end
  return out
 end
 local function djinn(a)
  return a and a.activeEffect and wanted[a.activeEffect.grantedEffect.id]
   and a.activeEffect.grantedEffect.id~="FireboltPlayer"
 end
 local function global_lookup(effect)
  assert(getmetatable(effect)==nil,"reviewed catalogue effects must have plain field lookup")
  local raw=rawget(effect,"hasGlobalEffect");local effective=effect.hasGlobalEffect
  assert(rawequal(raw,effective),"raw absence must equal the actual CalcSetup lookup")
  return {raw=scalar(raw),effective=scalar(effective),has_metatable=false,lookup_exact=true}
 end
 local function candidates(store,env,actor)
  local result,objects,seen={},{},{}
  while store do
   charge();assert(type(store)=="table"and not seen[store]and #result<32);seen[store]=true
   local entries={};local mods=rawget(store,"mods")
   local bucket=store;if mods then bucket=rawget(mods,"ExtraSkillStat")end
   assert(not bucket or #bucket<=16384)
   for i,mod in ipairs(bucket or{})do
    charge();if mod.name=="ExtraSkillStat"then entries[#entries+1]={index=i,record=plain(mod)}end
   end
   local parent=rawget(store,"parent");assert(parent==nil or parent==false or type(parent)=="table")
   objects[#objects+1]=store
   result[#result+1]={depth=#result,representation=mods and"named_bucket"or"sequence",records=entries,
    player=rawequal(store,env.player.modDB),actor=rawequal(store,actor.modDB),item=rawequal(store,env.itemModDB),
    parent_kind=parent==nil and"absent"or parent==false and"false_sentinel"or"store"}
   store=parent
  end
  return result,objects
 end
 work=0;local enabled=jit.status();local rows,builders,queries,merges,internals={},{},{},{},{}
 local callsByEnv,envObjects,envIndices={},{},{};local hook
 hook=function(event)
  if event~="call"and event~="return"then return end
  local info=debug.getinfo(2,"fl");local f=info.func
  if f==builder then
   local args=args_at(2);local a=args.activeSkill;if not djinn(a)then return end
   if event=="call"then
    assert(not builders[a]and #rows<256)
    local ei=envIndices[args.env]
    if not ei then ei=#envObjects+1;assert(ei<=64);envObjects[ei]=args.env;envIndices[args.env]=ei end
    local row={mode=args.env.mode,environment=ei,effect=a.activeEffect.grantedEffect.id,source=source_address(a),
     global_effect_lookup=global_lookup(a.activeEffect.grantedEffect),
     builder_called=true,builder_returned=false,query_called=false,query_returned=false,
     merge_called=false,merge_returned=false,observer_requeried_list=false}
    rows[#rows+1]=row;local frame={row=row,a=a,env=args.env}
    builders[a]=frame;callsByEnv[args.env]=callsByEnv[args.env]or{}
    local calls=callsByEnv[args.env][a]or{};calls[#calls+1]=#rows;callsByEnv[args.env][a]=calls
   else
    local frame=assert(builders[a]);assert(rawequal(frame.env,args.env))
    frame.row.builder_returned=true;frame.row.return_line=info.currentline
    frame.row.disabled=scalar(a.skillFlags and a.skillFlags.disable)
    assert(not queries[a.skillModList]and not merges[a.skillModList]);builders[a]=nil
   end
   return
  end
  if f==list then
   local caller=debug.getinfo(3,"fl");if caller.func~=builder or caller.currentline~=795 then return end
   local parent=args_at(3);local frame=builders[parent.activeSkill];if not frame then return end
   local args=args_at(2);local a=frame.a;local store=a.skillModList
   assert(rawequal(args.self,store)and rawequal(args.cfg,a.skillCfg))
   assert(rawequal(a.skillCfg.skillGrantedEffect,a.activeEffect.grantedEffect))
   if event=="call"then
    assert(not queries[store]and not frame.row.query_called)
    frame.store=store;frame.cfg=args.cfg
    frame.row.query_called=true;frame.row.query_caller_line=caller.currentline
    frame.row.cfg=scalars(args.cfg);frame.row.cfg_flags=plain(args.cfg.skillCond)
    frame.row.cfg_effect_id=args.cfg.skillGrantedEffect.id;frame.row.exact_filter_identity=true
    frame.row.candidates,frame.stores=candidates(store,frame.env,a.actor)
    frame.row.internal_calls={};queries[store]=frame
   else
    assert(rawequal(queries[store],frame)and args.n==1 and type(args.result)=="table")
    frame.extra=args.result;frame.row.query_returned=true;frame.row.query_return_line=info.currentline
    frame.row.returned_stats=plain(args.result);frame.row.returned_count=#args.result
    assert(equal(frame.row.candidates,candidates(store,frame.env,a.actor)),"query changed raw candidates")
    assert(#frame.row.internal_calls==#frame.stores,"every original ancestor ListInternal observed")
    queries[store]=nil
   end
   return
  end
  if f==listInternal or f==dbInternal then
   local args=args_at(2);local frame=queries[args.context]
   if not frame or args.modName~="ExtraSkillStat"then return end
   assert(rawequal(args.cfg,frame.cfg))
   if event=="call"then
    assert(not internals[args.self]);local depth
    for i,store in ipairs(frame.stores)do if rawequal(store,args.self)then assert(not depth);depth=i-1 end end
    assert(depth~=nil)
    local row={depth=depth,flags=scalar(args.flags),keyword_flags=scalar(args.keywordFlags),source_filter=scalar(args.source),
     name=args.modName,exact_context=true,exact_filter=true,result_count_before=#args.result,returned=false}
    frame.row.internal_calls[#frame.row.internal_calls+1]=row
    internals[args.self]={row=row,result=args.result}
   else
    local saved=assert(internals[args.self]);assert(rawequal(saved.result,args.result))
    saved.row.returned=true;saved.row.result_count_after=#args.result;internals[args.self]=nil
   end
   return
  end
  if f~=merge then return end
  local caller=debug.getinfo(3,"fl");if caller.func~=builder or caller.currentline~=795 then return end
  local parent=args_at(3);local frame=builders[parent.activeSkill];if not frame then return end
  local args=args_at(2);local row=frame.row
  assert(rawequal(args.env,frame.env)and rawequal(args.modList,frame.store)
   and rawequal(args.skillEffect,frame.a.activeEffect)and rawequal(args.statSet,parent.activeStatSet))
  assert(row.query_returned and rawequal(args.extraStats,frame.extra))
  assert(equal(row.returned_stats,plain(args.extraStats)),"original list-to-consumer payload changed")
  if event=="call"then
   assert(not merges[frame.store]and not row.merge_called);merges[frame.store]=frame
   row.merge_called=true;row.merge_caller_line=caller.currentline
   row.exact_returned_payload=true;row.admitted_stats=plain(args.extraStats);row.admitted_count=#args.extraStats
  else
   assert(rawequal(merges[frame.store],frame));row.merge_returned=true;row.merge_return_line=info.currentline
   row.payload_unchanged=true;merges[frame.store]=nil
  end
 end
 jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"cr")
 return function()
  assert(debug.gethook()==hook);debug.sethook()
  assert(preserved()and jit.status()==enabled and next(builders)==nil and next(queries)==nil
   and next(merges)==nil and next(internals)==nil,"incomplete compact original call")
  local environments={};for i,env in ipairs(envObjects)do
   environments[i]={index=i,mode=env.mode,final_main=rawequal(env,build.calcsTab.mainEnv),
    final_calcs=rawequal(env,build.calcsTab.calcsEnv)}
  end
  local census={}
  for _,mode in ipairs({"MAIN","CALCS"})do
   local env=mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
   assert(env and env.mode==mode);local groups={}
   for _,group in ipairs(build.skillsTab.socketGroupList)do
    for _,gem in ipairs(group.gemList)do
     if gem.skillId=="SummonSandDjinnPlayer"or gem.skillId=="SummonWaterDjinnPlayer"then
      local source=source_address({socketGroup=group,activeEffect={srcInstance=gem}})
      local effects={};assert(gem.gemData and #gem.gemData.grantedEffectList==2)
      for i,effect in ipairs(gem.gemData.grantedEffectList)do
       local matched,callIndices={},{}
       for ai,a in ipairs(env.player.activeSkillList)do
        if rawequal(a.socketGroup,group)and rawequal(a.activeEffect.srcInstance,gem)
         and rawequal(a.activeEffect.grantedEffect,effect)then
         matched[#matched+1]={index=ai,disabled=scalar(a.skillFlags and a.skillFlags.disable)}
         for _,index in ipairs(callsByEnv[env]and callsByEnv[env][a]or{})do callIndices[#callIndices+1]=index end
        end
       end
       effects[#effects+1]={id=effect.id,index=i,global_field="enableGlobal"..i,global_value=scalar(gem["enableGlobal"..i]),
        has_global_effect=scalar(rawget(effect,"hasGlobalEffect")),global_effect_lookup=global_lookup(effect),
        hide_from_sidebar=scalar(rawget(effect,"hideFromSideBar")),
        support=scalar(rawget(effect,"support")),catalogue_identity=rawequal(data.skills[effect.id],effect),
        active_skill_present=#matched>0,active_skills=matched,builder_call_indices=callIndices,
        consumer_observation=#callIndices>0 and"inspect_original_call_rows"or"not_called"}
      end
      groups[#groups+1]={source=source,effects=effects}
     end
    end
   end
   census[#census+1]={mode=mode,environment=assert(envIndices[env]),groups=groups}
  end
  api.last={mode="manual_djinn_global2_original_admission_v1",calls=rows,environments=environments,census=census,work=work,
   original_functions_preserved=true,hook_removed=true,observer_requeried_list=false,
   native_field_disposition=false,whole_supplier_domain_complete=false}
 end
end
local api={}
function api.install()
 if extraConsumptionFocus then return focused_install(api)end
 assert(debug.gethook()==nil and calcs.mergeSkillInstanceMods==merge and calcs.buildActiveSkillModList==builder
  and calcs.buildModListForNode==nodeBuilder and calcs.buildModListForNodeList==nodeListBuilder
  and calcs.initEnv==initEnv and storeClass.ScaleAddMod==scaleAdd and dbClass.AddMod==addMod
  and itemClass.GetActiveModListForSlotNum==itemMods)
 work=0;local enabled=jit.status();local rows,frames,envs,env_index={}, {}, {},{}
 local nodeReturns,nodeReturnCount={},0
 local amuletFrames,amuletReturns,amuletReturnCount={},{},0
 local hook
 hook=function(event)
  local info=debug.getinfo(2,"fl");local f=info.func
  if f==scaleAdd and(event=="call"or event=="return")then
   local caller=debug.getinfo(3,"fl");if caller.func~=initEnv or caller.currentline~=1667 then return end
   local args={};for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end;args[name]=value end
   local frame=amuletFrames[args.self]
   if event=="return"then
    assert(frame and rawequal(args.mod,frame.argument)and args.scale==frame.row.factor)
    assert(frame.row.add_return_observed and equal(frame.row.scale_argument,plain(args.mod)))
    assert(equal(frame.row.source_record,plain(frame.source)),"original Amulet record changed")
    assert(equal(frame.row.delivered_record,plain(frame.delivered)),"delivered Amulet record changed")
    frame.row.scale_return_observed=true;frame.row.source_record_unchanged=true
    amuletReturns[frame.env]=amuletReturns[frame.env]or{}
    local returned=amuletReturns[frame.env];assert(#returned<8);returned[#returned+1]=frame.row
    amuletReturnCount=amuletReturnCount+1;assert(amuletReturnCount<=128,"Amulet return bound")
    amuletFrames[args.self]=nil;return
   end
   assert(not frame)
   local parent={};for i=1,256 do local name,value=debug.getlocal(3,i);if not name then break end;parent[name]=value end
   local env=assert(parent.env);local item=assert(env.player.itemList.Amulet)
   assert(item.id==23 and item.type=="Amulet"and rawequal(build.itemsTab.items[23],item))
   assert(build.itemsTab.activeItemSet.Amulet.selItemId==23)
   assert(rawequal(args.self,env.modDB)and rawequal(args.self,env.player.modDB)and rawequal(args.self,parent.modDB))
   assert(rawequal(parent.modList,item.modList)and rawequal(parent.modCopy,args.mod))
   assert(args.self.AddMod==addMod and args.self.ScaleAddMod==scaleAdd and item.GetActiveModListForSlotNum==itemMods)
   assert(#parent.modList<=16384)
   local positions={};for i,mod in ipairs(parent.modList)do charge();if rawequal(mod,parent.mod)then positions[#positions+1]=i end end
   assert(#positions==1 and not rawequal(parent.mod,args.mod))
   local function capture()
    return {item_id=item.id,slot="Amulet",mode=env.mode,source_index=positions[1],source_count=#parent.modList,
     caller_line=caller.currentline,factor=scalar(args.scale),round_to_nearest=scalar(args.roundToNearest),
     exact_saved_item=true,exact_selected_slot=true,exact_active_list=true,exact_receiver=true,copy_is_distinct=true,
     source_record=plain(parent.mod),scale_argument=plain(args.mod),original_accessor_preserved=true,
     observer_requeried_item=false}
   end
   local row=capture();assert(equal(row,capture()),"Amulet call observer mutation")
   row.observer_noninterference=true
   amuletFrames[args.self]={row=row,env=env,source=parent.mod,argument=args.mod}
   return
  end
  if f==addMod and(event=="call"or event=="return")then
   local caller=debug.getinfo(3,"fl");if caller.func~=scaleAdd then return end
   local args={};for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end;args[name]=value end
   local frame=amuletFrames[args.self];if not frame then return end
   local bucket=rawget(args.self.mods,args.mod.name)
   if event=="call"then
    assert(not frame.delivered and caller.currentline==117)
    assert(not bucket or #bucket<=16384)
    local before={};for i,mod in ipairs(bucket or{})do charge();before[i]=mod end
    frame.before=before;frame.delivered=args.mod
    frame.row.add_caller_line=caller.currentline;frame.row.bucket=args.mod.name
    frame.row.bucket_count_before=#before;frame.row.delivered_record=plain(args.mod)
    assert(equal(frame.row.delivered_record,plain(args.mod)),"Amulet delivery observer mutation")
   else
    assert(rawequal(frame.delivered,args.mod)and #bucket==#frame.before+1)
    for i,mod in ipairs(frame.before)do charge();assert(rawequal(bucket[i],mod))end
    assert(rawequal(bucket[#bucket],args.mod)and equal(frame.row.delivered_record,plain(args.mod)))
    frame.row.inserted_index=#bucket;frame.row.bucket_count_after=#bucket
    frame.row.inserted_object_exact=true;frame.row.prior_bucket_objects_preserved=true
    frame.row.add_return_observed=true
   end
   return
  end
  if f==nodeBuilder and event=="return"then
   local caller=debug.getinfo(3,"fl");if caller.func~=nodeListBuilder or caller.currentline~=435 then return end
   local args={};for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end;args[name]=value end
   local parent={};for i=1,64 do local name,value=debug.getlocal(3,i);if not name then break end;parent[name]=value end
   local env,node,result=assert(args.env),assert(args.node),assert(args.modList)
   if not rawequal(parent.nodeList,env.allocNodes)then return end
   assert(info.currentline==411 and rawequal(env.allocNodes[node.id],node)and rawequal(parent.env,env))
   nodeReturnCount=nodeReturnCount+1;assert(nodeReturnCount<=4096,"node return evidence bound")
   local function capture()
    return {node_id=node.id,caller_line=caller.currentline,return_line=info.currentline,
     exact_allocated_input=true,original_function_return=true,include_keystone_mods=scalar(args.includeKeystoneMods),
     inc_small_passive_skill=scalar(args.incSmallPassiveSkill),scratch_supplied=args.reuse~=nil,
     returned_reuses_scratch=rawequal(args.reuse,result),effective_inputs=node_inputs(env,node),
     returned_modifiers=records(result,true)}
   end
   local row=capture();assert(equal(row,capture()),"node return observer mutation")
   row.observer_noninterference=true
   nodeReturns[env]=nodeReturns[env]or{};local prior=nodeReturns[env][node.id]or{}
   assert(#prior<8,"per-node return evidence bound");prior[#prior+1]=row;nodeReturns[env][node.id]=prior
   return
  end
  if f~=merge then return end
  if event~="call"and event~="return"then return end
  local caller=debug.getinfo(3,"fl");if caller.func~=builder then return end
  local locals={};for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end;locals[name]=value end
  local effect=locals.skillEffect;if not effect or not wanted[effect.grantedEffect.id]then return end
  -- Support merges at733 are not the extra-stat consumption call.
  if event=="call"and caller.currentline~=795 then return end
  local store=locals.modList
  if event=="return"then
   local frame=frames[store];if not frame then return end
   assert(rawequal(frame.extra,locals.extraStats)and rawequal(frame.effect,effect)and rawequal(frame.env,locals.env))
   assert(equal(frame.row.extra_stats,plain(locals.extraStats)),"original consumer changed supplied extra-stat payloads")
   frame.row.original_return_observed=true;frames[store]=nil;return
  end
  local parent={};for i=1,96 do local name,value=debug.getlocal(3,i);if not name then break end;parent[name]=value end
  local a=assert(parent.activeSkill);local env=assert(locals.env)
  assert(rawequal(a.activeEffect,effect)and rawequal(a.skillModList,store)and rawequal(parent.activeStatSet,locals.statSet))
  assert(type(locals.extraStats)=="table"and rawequal(a.skillCfg.skillGrantedEffect,effect.grantedEffect))
  assert(not frames[store]);assert(#rows<2048)
  local ei=env_index[env]
  if not ei then ei=#envs+1;assert(ei<=32);env_index[env]=ei;envs[ei]=supplier(env,nodeReturns,amuletReturns)end
  local sets={};for i,set in ipairs(effect.grantedEffect.statSets)do if rawequal(set,locals.statSet)then sets[#sets+1]=i end end
  assert(#sets==1)
  local function capture()
   return {mode=env.mode,effect=effect.grantedEffect.id,stat_set_index=sets[1],stat_set_id=scalar(locals.statSet.id),
    caller_line=caller.currentline,environment=ei,actor_is_player=rawequal(a.actor,env.player),
    cfg=scalars(a.skillCfg),cfg_flags=plain(a.skillCfg.skillCond),source=source_address(a),
    extra_stats=plain(locals.extraStats),extra_stats_count=#locals.extraStats,
    ancestry=chain(store,env,a.actor),effect_list=(function()local out={}for _,e in ipairs(a.effectList)do
     out[#out+1]={effect=e.grantedEffect.id,level=e.level,quality=scalar(e.quality),is_support=e.grantedEffect.support==true,
      source_instance=scalars(e.srcInstance)}end;return out end)(),
    exact_effect_cfg=true,exact_caller_objects=true,observer_requeried_list=false}
  end
  local row=capture();assert(equal(row,capture()),"consumer observer mutation")
  row.observer_noninterference=true;rows[#rows+1]=row;frames[store]={row=row,extra=locals.extraStats,effect=effect,env=env}
 end
 jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"cr")
 return function()
  assert(debug.gethook()==hook);debug.sethook()
  assert(calcs.mergeSkillInstanceMods==merge and calcs.buildActiveSkillModList==builder and runCallback==callback
   and calcs.buildModListForNode==nodeBuilder and calcs.buildModListForNodeList==nodeListBuilder
   and calcs.initEnv==initEnv and storeClass.ScaleAddMod==scaleAdd and dbClass.AddMod==addMod
   and itemClass.GetActiveModListForSlotNum==itemMods)
  assert(jit.status()==enabled and next(frames)==nil and next(amuletFrames)==nil,"incomplete consumer call")
  api.last={calls=rows,environments=envs,work=work,node_return_observations=nodeReturnCount,amulet_return_observations=amuletReturnCount,original_functions_preserved=true,hook_removed=true,
   native_field_disposition=false,whole_supplier_domain_complete=false}
 end
end
function api.observe()
 assert(debug.gethook()==nil)
 return {consumer=api.last,outputs={MAIN=scalars(build.calcsTab.mainOutput),CALCS=scalars(build.calcsTab.calcsOutput)}}
end
function api.rebuild(observed)
 local cleanup=observed and api.install()
 local revision=build.outputRevision;local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end)
 if cleanup then cleanup()end
 if not ok then error(err,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1)
 assert(build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
