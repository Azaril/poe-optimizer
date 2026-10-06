-- Optional source evidence for the original pre-Amulet sum. No replacements,
-- diagnostic re-query, invented defaults, or native Lua-compatibility contract.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(assert(f),"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local init,ia=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local sum,sa=original(common.classes.ModStore.Sum,"Classes/ModStore.lua",202)
local sumInternal,si=original(common.classes.ModDB.SumInternal,"Classes/ModDB.lua",137)
local scale,sc=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local add,aa=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local nodeBuilder,na=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200)
local nodeListBuilder,nla=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)
local callback,ca=original(runCallback,"HeadlessWrapper.lua",17)
local methods={initialization=ia,sum=sa,sum_internal=si,scale=sc,add_database=aa,node=na,node_list=nla,callback=ca}
local stat="EffectOfBonusesFromAmulet"
local work=0
local function charge(n)work=work+(n or 1);assert(work<=2000000,"Amulet observer work bound")end
local function keys(t)
 local r={};for k in next,t do charge();assert(type(k)=="string"or type(k)=="number");r[#r+1]=k end
 assert(#r<=32768);table.sort(r,function(a,b)if type(a)~=type(b)then return type(a)<type(b)end;return a<b end);return r
end
local function plain(v,depth,seen)
 charge();if v==nil then return {kind="absent"}end
 local typ=type(v)
 if typ=="number"then assert(v==v and v~=math.huge and v~=-math.huge);return v end
 if typ=="string"or typ=="boolean"then return v end
 assert(typ=="table","unreviewed source value "..typ)
 depth=(depth or 0)+1;assert(depth<=16);seen=seen or{};assert(not seen[v],"cyclic source payload");seen[v]=true
 assert(getmetatable(v)==nil,"nonplain modifier payload")
 local r,positions={},{};for _,k in ipairs(keys(v))do
  if type(k)=="number"then positions[#positions+1]={index=k,value=plain(rawget(v,k),depth,seen)}else r[k]=plain(rawget(v,k),depth,seen)end
 end
 if #positions>0 then r.positions=positions end;seen[v]=nil;return r
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in next,a do if not equal(v,b[k])then return false end end
 for k in next,b do if a[k]==nil then return false end end;return true
end
local function fields(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="string"or type(v)=="boolean"or type(v)=="number")then
  r[k]=type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)and tostring(v)or v
 end end;return r
end
local function relevant(v,seen,depth)
 charge();if type(v)~="table"then return false end
 depth=(depth or 0)+1;assert(depth<=16);seen=seen or{};if seen[v]then return false end;seen[v]=true
 if rawget(v,"name")==stat then return true end
 for _,k in ipairs(keys(v))do if relevant(rawget(v,k),seen,depth)then return true end end;return false
end
local function list(list)
 local r={present=list~=nil,count=0,rows={}}
 for i,m in ipairs(list or{})do charge();assert(i<=16384 and type(m)=="table");r.count=i
  if relevant(m)then r.rows[#r.rows+1]={index=i,record=plain(m)}end
 end;return r
end
local function chain(store)
 local out,seen={},{};while store do
  charge();assert(type(store)=="table"and not seen[store]and #out<32);seen[store]=true
  local parent=rawget(store,"parent");assert(parent==nil or parent==false or type(parent)=="table")
  local mods=assert(rawget(store,"mods"));assert(type(mods)=="table"and getmetatable(mods)==nil)
  local rows={};for i,m in ipairs(rawget(mods,stat)or{})do
   charge();assert(i<=16384);rows[i]={index=i,record=plain(m)}
  end
  out[#out+1]={depth=#out,parent_kind=parent==nil and"absent"or parent==false and"false_sentinel"or"store",rows=rows}
  store=parent
 end;return out
end
local function finite_chain(rows)
 for _,s in ipairs(rows)do for _,r in ipairs(s.rows)do local m=r.record
  assert(m.name==stat and m.type=="INC"and m.flags==0 and m.keywordFlags==0 and m.positions==nil,
   "snapshot witness admits only original untagged unconditional INC candidates")
  assert(type(m.value)=="number")
 end end
end
local api={}
function api.install()
 assert(not debug.gethook() and common.classes.ModStore.Sum==sum and common.classes.ModDB.SumInternal==sumInternal)
 work=0;local enabled=jit.status();local observations,byEnv,nodeReturns,frames,copyFrames={},{},{},{},{}
 local hook_failure
 local function candidates(env)
  local items,nodes={},{ }
  for _,slot in ipairs(keys(env.player.itemList))do local item=env.player.itemList[slot]
   items[#items+1]={slot=slot,id=item.id,type=item.type,source=item.modSource,exact_registered=build.itemsTab.items[item.id]==item,
    base=list(item.baseModList),active=list(item.modList)}
  end
  for _,id in ipairs(keys(env.allocNodes))do local node=env.allocNodes[id];local tree=assert(env.spec.tree.nodes[id])
   assert(env.spec.nodes[id]==node and getmetatable(node)==tree and rawget(tree,"__index")==tree)
   local returned={};for i,row in ipairs(nodeReturns[env]and nodeReturns[env][id]or{})do returned[i]=row end
   local localMods=rawget(node,"modList");local inherited=rawget(tree,"modList")
   assert(node.modList==(localMods or inherited),"unknown effective passive lookup")
   nodes[#nodes+1]={id=id,name=node.dn,effective_lookup=localMods~=nil and"local"or"tree_inherited",
    effective=list(node.modList),original_returns=returned,
    exact_allocated_object=true}
  end
  return {items=items,nodes=nodes,config=list(build.configTab.modList),
   class={id=env.spec.curClassId,name=env.spec.curClassName,ascendancy_id=env.spec.curAscendClassId,ascendancy_name=env.spec.curAscendClassName},
   config_input=plain(env.configInput),config_placeholder=plain(env.configPlaceholder),
   axes={passives=build.treeTab.activeSpec,items=build.itemsTab.activeItemSetId,config=build.configTab.activeConfigSetId,skills=build.skillsTab.activeSkillSetId},
   scope="current selected candidate lists only; arbitrary transforms, alternative suppliers and game legality remain unproved"}
 end
 local function hook(event)
  if event~="call"and event~="return"then return end
  local info=debug.getinfo(2,"fl");local f=info.func
  if f~=sum and f~=sumInternal and f~=scale and f~=add and f~=nodeBuilder then return end
  local v={};for i=1,160 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==sum or f==scale or f==nodeBuilder then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
  if f==nodeBuilder and event=="return"and caller.func==nodeListBuilder and caller.currentline==435 then
   if cv.nodeList~=v.env.allocNodes then return end
   assert(info.currentline==411 and v.env.allocNodes[v.node.id]==v.node)
   nodeReturns[v.env]=nodeReturns[v.env]or{};local previous=nodeReturns[v.env][v.node.id]or{}
   assert(#previous<8);previous[#previous+1]={returned=list(v.modList),include_keystone_mods=plain(v.includeKeystoneMods),
    inc_small_passive_skill=plain(v.incSmallPassiveSkill),original_return=true};nodeReturns[v.env][v.node.id]=previous
  elseif f==sum and event=="call"and caller.func==init and caller.currentline==1662 then
   local env=assert(cv.env);local item=assert(env.player.itemList.Amulet)
   assert(item.type=="Amulet"and build.itemsTab.items[item.id]==item)
   assert(v.self==env.modDB and v.self==env.player.modDB and v.modType=="INC"and v.cfg==nil)
   assert(not byEnv[env],"duplicate capture for one actual environment")
   local before=chain(v.self);finite_chain(before)
   local row={mode=env.mode,item={id=item.id,type=item.type,source=item.modSource,exact_registered=true},
    cfg_absent=true,query_type=v.modType,query_name=stat,caller_line=1662,store_is_player=true,
    before=before,source_candidates=candidates(env),internal={},copies={},bucket_complete=true,untagged_domain=true}
   assert(#observations<64);observations[#observations+1]=row
   byEnv[env]={row=row,store=v.self,env=env};frames[v.self]=byEnv[env]
  elseif f==sumInternal and v.modName==stat then
   local owner=frames[v.context];if not owner then return end
   assert(v.modType=="INC"and v.cfg==nil and v.flags==0 and v.keywordFlags==0 and v.source==nil)
   if event=="return"then
    local depth;local p=owner.store;local n=0;while p do if p==v.self then depth=n;break end;n=n+1;p=p.parent end
    assert(depth~=nil and type(v.result)=="number")
    owner.row.internal[#owner.row.internal+1]={depth=depth,result=v.result,original_return=true}
    if v.self==owner.store then
     assert(equal(owner.row.before,chain(v.self)),"original sum changed its incoming bucket")
     owner.row.result=v.result;owner.row.query_return_observed=true;frames[v.self]=nil
    end
   end
  elseif f==scale and caller.func==init and caller.currentline==1667 then
   local owner=assert(byEnv[cv.env],"copy lacks actual captured query")
   if event=="call"then
    assert(owner.row.query_return_observed and v.scale==owner.row.result/100 and cv.amuletEffectMod==v.scale)
    assert(v.self==owner.store and cv.modDB==v.self and cv.modCopy==v.mod and cv.mod~=v.mod)
    local item=cv.env.player.itemList.Amulet;assert(item.id==owner.row.item.id)
    local positions={};for i,m in ipairs(cv.modList)do charge();if m==cv.mod then positions[#positions+1]=i end end;assert(#positions==1)
    assert(cv.modList==item.modList,"unreviewed Amulet slot-specific list")
    local row={source_index=positions[1],original_record=plain(cv.mod),copy_argument=plain(v.mod),factor=v.scale,
     exact_source_object=true,exact_copy_argument=true,exact_destination=true,insertions={}}
    copyFrames[#copyFrames+1]={owner=owner,row=row,source=cv.mod,argument=v.mod,destination=v.self}
   else
    local r=assert(table.remove(copyFrames));assert(r.owner==owner and r.argument==v.mod)
    assert(equal(r.row.original_record,plain(r.source)),"copy mutated original Amulet record")
    assert(#r.row.insertions==1,"unreviewed original ScaleAddMod insertion count")
    r.row.return_observed=true;r.row.after_bucket=chain(v.self);owner.row.copies[#owner.row.copies+1]=r.row
   end
  elseif f==add and event=="return"and #copyFrames>0 then
   local r=copyFrames[#copyFrames];if v.self~=r.destination then return end
   local bucket=assert(v.self.mods[v.mod.name]);assert(bucket[#bucket]==v.mod)
   r.row.insertions[#r.row.insertions+1]={record=plain(v.mod),same_as_copy_argument=v.mod==r.argument,exact_inserted_object=true}
  end
  end,debug.traceback)
  if not ok then hook_failure=tostring(err);error(hook_failure,0)end
 end
 if amuletSnapshotInstrumented then jit.flush();debug.sethook(hook,"cr")end
 return function()
  if amuletSnapshotInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not hook_failure,hook_failure)
  assert(common.classes.ModStore.Sum==sum and common.classes.ModDB.SumInternal==sumInternal and common.classes.ModStore.ScaleAddMod==scale
   and common.classes.ModDB.AddMod==add and calcs.initEnv==init and calcs.buildModListForNode==nodeBuilder and runCallback==callback)
  assert(jit.status()==enabled)
  api.environments=byEnv
  api.last={queries=observations,methods=methods,work=work,complete_frames=next(frames)==nil and #copyFrames==0,
   original_methods_preserved=true,hook_removed=true,business_wrappers=false,diagnostic_query_substitution=false,
   current_capture_bucket_authority=amuletSnapshotInstrumented,whole_supplier_domain_complete=false,native_inventory_authority=false}
 end
end
function api.observe()
 assert(not debug.gethook() and api.last and api.last.complete_frames)
 local environments={};for _,mode in ipairs({"MAIN","CALCS"})do local env=mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
  local observed=api.environments[env];local index
  if observed then for i,q in ipairs(api.last.queries)do if observed.row==q then assert(not index);index=i end end;assert(index)end
  local item=env.player.itemList.Amulet
  environments[#environments+1]={mode=mode,has_amulet=item~=nil,item_type=item and item.type,item_id=item and item.id,
   query_index=index,actual_selected_env_observed=not amuletSnapshotInstrumented or observed~=nil or item==nil or item.type~="Amulet"}
 end
 return {snapshot=api.last,environments=environments,outputs={MAIN=fields(build.calcsTab.mainOutput),CALCS=fields(build.calcsTab.calcsOutput)}}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision
 local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
