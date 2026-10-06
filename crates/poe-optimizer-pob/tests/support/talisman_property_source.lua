-- Original-call evidence only: ordinary Amulet diversion, independent early
-- Player copies and actual minion-store receipt. No native/gameplay law here.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(assert(f),"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,name)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end
 error("missing original upvalue "..name)
end
local init,ia=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local perform,pa=original(calcs.perform,"Modules/CalcPerform.lua",1193)
local minionInit,mi=original(upvalue(perform,"initMinionModDB"),"Modules/CalcPerform.lua",1049)
local itemList,il=original(common.classes.Item.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local sum,sa=original(common.classes.ModStore.Sum,"Classes/ModStore.lua",202)
local internal,si=original(common.classes.ModDB.SumInternal,"Classes/ModDB.lua",137)
local scale,sca=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local add,ada=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local listAdd,la=original(common.classes.ModList.AddMod,"Classes/ModList.lua",29)
local addList,al=original(common.classes.ModDB.AddList,"Classes/ModDB.lua",113)
local tabulate,ta=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local apply,aa=original(upvalue(init,"applyGemMods"),"Modules/CalcSetup.lua",550)
local assemble,ba=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local callback,ca=original(runCallback,"HeadlessWrapper.lua",17)
local load=assert(djinnOriginals.refs.load_skill)
local methods={initialization=ia,performance=pa,minion_initialization=mi,item_active_list=il,sum=sa,sum_internal=si,
 scale=sca,add_database=ada,add_list_record=la,add_list_database=al,tabulate=ta,ordinary=aa,assembly=ba,callback=ca}
local function fields(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="string"or type(v)=="boolean"or type(v)=="number")then
  r[k]=type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)and tostring(v)or v
 end end;return r
end
local work=0
local function plain(v,depth,seen)
 work=work+1;assert(work<2000000,"Talisman observer work bound")
 if v==nil then return {kind="absent"}end
 if type(v)=="number"then assert(v==v and v~=math.huge and v~=-math.huge);return v end
 if type(v)=="string"or type(v)=="boolean"then return v end
 assert(type(v)=="table"and getmetatable(v)==nil,"unknown modifier payload")
 depth=(depth or 0)+1;assert(depth<16);seen=seen or{};assert(not seen[v]);seen[v]=true
 local r,p,n={},{},0;for k,x in pairs(v)do n=n+1;assert(n<32768)
  if type(k)=="number"then p[#p+1]={index=k,value=plain(x,depth,seen)}else assert(type(k)=="string");r[k]=plain(x,depth,seen)end
 end
 table.sort(p,function(a,b)return a.index<b.index end);if #p>0 then r.positions=p end;seen[v]=nil;return r
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end
 for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function list(t)local r={};for i,m in ipairs(t or{})do assert(i<4096);r[i]={index=i,record=plain(m)}end;return r end
local function bucket(store,name)
 local out,seen={},{};while store do assert(type(store)=="table"and not seen[store]and #out<32);seen[store]=true
  local parent=rawget(store,"parent");assert(parent==nil or parent==false or type(parent)=="table")
  local rows={};if store.mods then for i,m in ipairs(store.mods[name]or{})do assert(i<4096);rows[#rows+1]={index=i,record=plain(m)}end
  else for i,m in ipairs(store)do assert(i<32768);if m.name==name then rows[#rows+1]={index=i,record=plain(m)}end end end
  out[#out+1]={depth=#out,rows=rows,parent_kind=parent==nil and"absent"or parent==false and"false_sentinel"or"store"};store=parent
 end;return out
end
local function input(e)return {level=e.level,quality=e.quality}end
local function target(e)return e and e.grantedEffect and e.grantedEffect.id=="PainOfferingPlayer"end
local function item(i)return i and {id=i.id,type=i.type,source=i.modSource,exact_registered=build.itemsTab.items[i.id]==i}or{absent=true}end
local api={loaded={}}
function api.install()
 assert(not debug.gethook()and jit.status()==talismanJit);work=0
 local byEnv,queries,ordinary,assemblies,applyFrames,scaleFrames,deliveryFrames,sumFrames={},{},{},{},{},{},{},{}
 local sequence,failure=0
 local function environment(env)
  local r=byEnv[env];if not r then r={env=env,diverted={},copies={},receipts={},item_returns={}};byEnv[env]=r end;return r
 end
 local wanted={[init]=true,[perform]=true,[itemList]=true,[sum]=true,[internal]=true,[scale]=true,[add]=true,[listAdd]=true,[addList]=true,[tabulate]=true,[apply]=true,[assemble]=true,[load]=true}
 local function hook(event)
  if event~="call"and event~="return"then return end
  local f=debug.getinfo(2,"f").func;if not wanted[f]then return end
  local v={};for i=1,192 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==itemList or f==sum or f==scale or f==addList then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
  sequence=sequence+1;assert(sequence<1000000)
  if f==load and event=="return"and v.node.elem=="Skill"then
   local set=v.self.skillSets[v.skillSetId];local group=v.socketGroup;assert(set.socketGroupList[#set.socketGroupList]==group)
   api.loaded[v.skillSetId]=api.loaded[v.skillSetId]or{};local gems={};for i,g in ipairs(group.gemList)do gems[i]=g end
   api.loaded[v.skillSetId][#set.socketGroupList]={group=group,set=set,gems=gems}
  elseif f==itemList and event=="return"and caller.func==init and(caller.currentline==1322 or caller.currentline==1663)and v.self.type=="Amulet"then
   local r=environment(cv.env);assert(cv.env.player.itemList.Amulet==v.self)
   local returned=v.self.modList or v.self.slotModList[v.slotNum]
   r.item_returns[caller.currentline]={object=returned,item=v.self,records=list(returned),exact_return_expression=true}
  elseif f==scale and caller.func==init and(caller.currentline==1402 or caller.currentline==1667)then
   local r=environment(cv.env);local diversion=caller.currentline==1402
   if event=="call"then
    local source=assert(r.item_returns[diversion and 1322 or 1663],"transport lacks actual item method return")
    assert(source.item==cv.env.player.itemList.Amulet)
    local sourceList=diversion and cv.srcList or cv.modList;assert(sourceList==source.object,"unreviewed item-list rewrite")
    local sourceMod=assert(cv.mod);local index;for i,m in ipairs(sourceList)do if m==sourceMod then assert(not index);index=i end end;assert(index)
    assert(v.self==(diversion and cv.env.talismanModList or cv.env.modDB))
    if diversion then assert(v.mod==sourceMod and cv.env.allocNodes[39935]and cv.env.allocNodes[39935].dn=="Necromantic Talisman")
    else assert(v.mod==cv.modCopy and v.mod~=sourceMod and r.query and r.query.return_observed and v.scale==r.query.result/100)end
    local row={source_index=index,source_record=plain(sourceMod),argument=plain(v.mod),factor=v.scale,caller_line=caller.currentline,
     exact_source_object=true,exact_destination=true,actual_item_return=true,insertions={},objects={}}
    scaleFrames[#scaleFrames+1]={owner=r,row=row,source=sourceMod,argument=v.mod,destination=v.self,diversion=diversion}
   else
    local frame=assert(table.remove(scaleFrames));assert(frame.owner==r and frame.argument==v.mod)
    assert(equal(frame.row.source_record,plain(frame.source))and #frame.row.insertions==1)
    frame.row.return_observed=true;local into=frame.diversion and r.diverted or r.copies;into[#into+1]=frame.row
   end
  elseif(f==add or f==listAdd)and event=="return"and #scaleFrames>0 then
   local r=scaleFrames[#scaleFrames];if v.self~=r.destination then return end
   if f==add then assert(v.self.mods[v.mod.name][#v.self.mods[v.mod.name]]==v.mod)else assert(v.self[#v.self]==v.mod)end
   r.row.insertions[#r.row.insertions+1]={record=plain(v.mod),exact_inserted_object=true,same_as_argument=v.mod==r.argument};r.row.objects[#r.row.objects+1]=v.mod
  elseif f==sum and event=="call"and caller.func==init and caller.currentline==1662 then
   local r=environment(cv.env);assert(not r.query and v.self==cv.env.modDB and v.modType=="INC"and v.cfg==nil)
   r.query={before=bucket(v.self,"EffectOfBonusesFromAmulet"),original_call=true,exact_player_store=true};sumFrames[v.self]=r
  elseif f==internal and event=="return"and v.modName=="EffectOfBonusesFromAmulet"then
   local r=sumFrames[v.context];if r and v.self==r.env.modDB then r.query.result=v.result;r.query.return_observed=true;sumFrames[v.context]=nil end
  elseif f==tabulate and event=="return"and v.modType=="LIST"then
   queries[assert(v.result)]={store=v.self,sequence=sequence}
  elseif f==apply and target(v.effect)then
   if event=="call"then
    assert(not applyFrames[v.effect]);local query=assert(queries[v.modList]);local candidates={}
    for i,c in ipairs(v.modList)do assert(c.mod.name=="GemProperty");candidates[i]={index=i,record=plain(c.mod),value=plain(c.value)}end
    applyFrames[v.effect]={effect=v.effect,before=input(v.effect),list=v.modList,query=query,candidates=candidates,sequence=sequence}
   else
    local r=assert(applyFrames[v.effect]);applyFrames[v.effect]=nil;r.after=input(v.effect);r.matched={}
    for i,c in ipairs(r.list)do for _,m in ipairs(v.effect.gemPropertyInfo or{})do if m==c then r.matched[#r.matched+1]=i end end end
    ordinary[#ordinary+1]=r;assert(#ordinary<4096)
   end
  elseif f==assemble and event=="return"and target(v.activeSkill.activeEffect)then
   assert(not assemblies[v.activeSkill]);assemblies[v.activeSkill]={env=v.env,after=input(v.activeSkill.activeEffect),lookup=plain(v.activeSkill.activeEffect.grantedEffectLevel),sequence=sequence}
  elseif f==init and event=="return"then
   local r=environment(v.env);r.initialized=sequence;r.amulet=item(v.env.player.itemList.Amulet)
   r.before_perform=bucket(v.env.modDB,"GemProperty");r.talisman_records=list(v.env.talismanModList)
   r.path={};for _,id in ipairs({61042,44344,39935})do local node=v.env.allocNodes[id];local tree=v.env.spec.tree.nodes[id]
    r.path[#r.path+1]={id=id,allocated=node~=nil,name=tree.dn,exact_allocated_object=node==nil or v.env.spec.nodes[id]==node,
     connections=plain(tree.linkedId)}
   end
   r.class={id=v.env.spec.curClassId,name=v.env.spec.curClassName,ascendancy_id=v.env.spec.curAscendClassId,ascendancy_name=v.env.spec.curAscendClassName}
  elseif f==addList and caller.func==minionInit and caller.currentline==1102 then
   local r=environment(cv.env);assert(v.modList==cv.env.talismanModList and cv.minion==cv.activeSkill.minion and v.self==cv.minion.modDB)
   if event=="call"then
    local row={skill=cv.activeSkill,receiver=cv.minion,list=v.modList,records=list(v.modList),before=bucket(v.self,"GemProperty"),
     selected=cv.activeSkill==cv.env.player.mainSkill,exact_talisman_source=true,exact_minion_destination=true}
    deliveryFrames[#deliveryFrames+1]={owner=r,row=row,destination=v.self}
   else
    local frame=assert(table.remove(deliveryFrames));assert(frame.owner==r and frame.destination==v.self)
    local joins={};for i,m in ipairs(v.modList)do local indices={};for j,x in ipairs(v.self.mods[m.name]or{})do if x==m then indices[#indices+1]=j end end;assert(#indices>0);joins[i]={source_index=i,destination_indices=indices}end
    frame.row.joins=joins;frame.row.after=bucket(v.self,"GemProperty");frame.row.original_return=true;r.receipts[#r.receipts+1]=frame.row
   end
  elseif f==perform and event=="return"then local r=assert(byEnv[v.env]);r.performed=true;r.final_amulet=item(v.env.player.itemList.Amulet)end
  end,debug.traceback)
  if not ok then failure=tostring(err);error(failure,0)end
 end
 if talismanInstrumented then jit.flush();debug.sethook(hook,"cr")end
 return function()
  if talismanInstrumented then assert(debug.gethook()==hook);debug.sethook()end;assert(not failure,failure)
  assert(calcs.initEnv==init and calcs.perform==perform and upvalue(perform,"initMinionModDB")==minionInit and upvalue(init,"applyGemMods")==apply)
  assert(common.classes.ModStore.Sum==sum and common.classes.ModDB.SumInternal==internal and common.classes.ModStore.ScaleAddMod==scale
   and common.classes.ModDB.AddMod==add and common.classes.ModList.AddMod==listAdd and common.classes.ModDB.AddList==addList
   and common.classes.ModStore.Tabulate==tabulate and common.classes.Item.GetActiveModListForSlotNum==itemList and calcs.buildActiveSkillModList==assemble)
  api.state={byEnv=byEnv,ordinary=ordinary,assemblies=assemblies,complete=next(applyFrames)==nil and next(sumFrames)==nil and #scaleFrames==0 and #deliveryFrames==0}
  assert(jit.status()==talismanJit)
 end
end
local function origins()
 local parsed,err=common.xml.ParseXML(talismanXml);assert(parsed and not err)
 local ordinal,n={},0;local function visit(x)if type(x)~="table"or not x.elem then return end;ordinal[x]=n;n=n+1;assert(n<100000);for _,c in ipairs(x)do visit(c)end end;visit(parsed[1])
 local out={};for _,skills in ipairs(parsed[1])do if type(skills)=="table"and skills.elem=="Skills"then
  for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then local sid=tonumber(set.attrib.id);local gi=0
   for _,g in ipairs(set)do if type(g)=="table"and g.elem=="Skill"then gi=gi+1;local saved=api.loaded[sid]and api.loaded[sid][gi];local i=0
    for _,gem in ipairs(g)do if type(gem)=="table"and gem.elem=="Gem"then i=i+1
     if g.attrib.source==nil then local runtime=saved and saved.gems[i]or build.skillsTab.skillSets[sid].socketGroupList[gi].gemList[i]
      assert(runtime.skillId==gem.attrib.skillId);if saved then assert(saved.group.gemList[i]==runtime and saved.set==build.skillsTab.skillSets[sid])end
      out[runtime]={source_ordinal=ordinal[gem],group_source_ordinal=ordinal[g],preset=sid,index=i,attributes=fields(gem.attrib)}
     end
    end end
   end end
  end end
 end end;return out
end
local function transport(rows)
 local out={};for i,r in ipairs(rows)do local row={};for k,v in pairs(r)do if k~="objects"then row[k]=v end end;out[i]=row end;return out
end
function api.observe()
 assert(not debug.gethook()and api.state and api.state.complete and jit.status()==talismanJit)
 local state=api.state;local source=origins();local environments={}
 for _,mode in ipairs({"MAIN","CALCS"})do
  local env=mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv;local r=state.byEnv[env]
  assert(not talismanInstrumented or r and r.initialized and r.performed,"selected environment not captured")
  local row={mode=mode,exact_selected_environment=true,offerings={},receipts={}}
  if r then
   row.path=r.path;row.class=r.class;row.amulet=r.amulet;row.final_amulet=r.final_amulet;row.before_perform=r.before_perform
   row.talisman_records=r.talisman_records;row.diverted=transport(r.diverted);row.copies=transport(r.copies);row.query=r.query
   for _,receipt in ipairs(r.receipts)do
    local effect=receipt.skill.activeEffect;local origin=source[effect.srcInstance]
    row.receipts[#row.receipts+1]={effect=effect.grantedEffect.id,source=origin,source_present=origin~=nil,
     selected=receipt.selected,records=receipt.records,before=receipt.before,after=receipt.after,joins=receipt.joins,
     exact_talisman_source=receipt.exact_talisman_source,exact_minion_destination=receipt.exact_minion_destination,original_return=receipt.original_return,
     skill_actor_is_player=receipt.skill.actor==env.player,receiver_is_skill_minion=receipt.receiver==receipt.skill.minion,
     selected_minion_identity=not receipt.selected or env.minion==receipt.receiver}
   end
  end
  for _,a in ipairs(env.player.activeSkillList)do if target(a.activeEffect)then
   local e=a.activeEffect;local s={source=assert(source[e.srcInstance]),raw=input(e.srcInstance),final=input(e),lookup=plain(e.grantedEffectLevel),
    exact_physical_source=e.gemData==e.srcInstance.gemData,ordinary={},assembly={}}
   if r then
    local assembly=assert(state.assemblies[a]);assert(assembly.env==env and assembly.sequence<r.initialized)
    s.assembly={after=assembly.after,lookup=assembly.lookup,original_call=true,before_perform=true}
    for _,o in ipairs(state.ordinary)do if o.effect==e then
     assert(o.query.store==env.modDB and o.sequence<r.initialized);local joins={}
     for i,candidate in ipairs(o.list)do local copied={};for j,copy in ipairs(r.copies)do for _,m in ipairs(copy.objects)do if candidate.mod==m then copied[#copied+1]=j end end end
      for _,diverted in ipairs(r.diverted)do for _,m in ipairs(diverted.objects)do assert(candidate.mod~=m,"diverted record entered Player ordinary query")end end
      joins[i]={candidate_index=i,early_copy_indices=copied}
     end
     s.ordinary[#s.ordinary+1]={before=o.before,after=o.after,candidates=o.candidates,matched=o.matched,joins=joins,
      original_query=true,exact_actor_store=true,no_diverted_object_consumed=true,before_perform=true}
    end end
   end
   row.offerings[#row.offerings+1]=s
  end end
  environments[#environments+1]=row
 end
 return {methods=methods,environments=environments,outputs={MAIN=fields(build.calcsTab.mainOutput),CALCS=fields(build.calcsTab.calcsOutput)},
  instrumented=talismanInstrumented,actual_calls=talismanInstrumented,original_methods_preserved=true,hook_removed=true,business_wrappers=false,
  source_tables_mutated=false,diagnostic_requery=false,roll_legality_authority=false,native_owner_closure=false,minion_gem_level_consumer_claim=false}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision;local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
