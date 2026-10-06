-- Reference-only Focus branch witness. All business methods stay original.
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
local itemList,il=original(common.classes.Item.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local merge,ma=original(common.classes.ModList.MergeMod,"Classes/ModList.lua",75)
local scale,sa=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local scaleList,sl=original(common.classes.ModStore.ScaleAddList,"Classes/ModStore.lua",127)
local listAdd,la=original(common.classes.ModList.AddMod,"Classes/ModList.lua",29)
local addList,al=original(common.classes.ModDB.AddList,"Classes/ModDB.lua",113)
local tabulate,ta=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local apply,aa=original(upvalue(init,"applyGemMods"),"Modules/CalcSetup.lua",550)
local assemble,ba=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local callback,ca=original(runCallback,"HeadlessWrapper.lua",17)
local load=assert(djinnOriginals.refs.load_skill)
local methods={initialization=ia,item_active_list=il,merge=ma,scale=sa,scale_list=sl,list_add=la,
 add_list=al,tabulate=ta,ordinary=aa,assembly=ba,callback=ca}
local function fields(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="string"or type(v)=="boolean"or type(v)=="number")then
  r[k]=type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)and tostring(v)or v
 end end;return r
end
local work=0
local function charge()work=work+1;assert(work<2000000,"Focus observer work bound")end
local function plain(v,depth,seen)
 charge();if v==nil then return {kind="absent"}end
 if type(v)=="number"then assert(v==v and v~=math.huge and v~=-math.huge);return v end
 if type(v)=="string"or type(v)=="boolean"then return v end
 assert(type(v)=="table"and getmetatable(v)==nil,"unknown source payload")
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
local function chain(store,name)
 local out,seen={},{};while store do charge();assert(type(store)=="table"and not seen[store]and #out<32);seen[store]=true
  local parent=rawget(store,"parent");assert(parent==nil or parent==false or type(parent)=="table")
  local rows={};if store.mods then for i,m in ipairs(store.mods[name]or{})do assert(i<16384);rows[#rows+1]={index=i,record=plain(m)}end
  else for i,m in ipairs(store)do assert(i<32768);if m.name==name then rows[#rows+1]={index=i,record=plain(m)}end end end
  out[#out+1]={depth=#out,kind=store.mods and"database"or"list",parent_kind=parent==nil and"absent"or parent==false and"false_sentinel"or"store",rows=rows};store=parent
 end;return out
end
local function input(e)return {level=e.level,quality=e.quality}end
local function target(e)return e and e.grantedEffect and e.grantedEffect.id=="SummonSkeletalSnipersPlayer"end
local function item(slot,i)return {slot=slot,id=i.id,type=i.type,source=i.modSource,exact_registered=build.itemsTab.items[i.id]==i}end
local function records(t)local r={};for i,m in ipairs(t or{})do assert(i<4096);r[i]={index=i,record=plain(m)}end;return r end
local function public_trace(rows)
 local out={};for i,row in ipairs(rows)do local copy={};for k,v in pairs(row)do if k~="sequence"then copy[k]=v end end;out[i]=copy end;return out
end
local function allocation(env)
 local out={};for id,node in pairs(env.allocNodes)do assert(type(id)=="number"and node.id==id);out[#out+1]={id=id,name=node.dn,ascendancy=node.ascendancyName or false,type=node.type}end
 table.sort(out,function(a,b)return a.id<b.id end);return out
end
local api={loaded={}}
function api.install()
 assert(not debug.gethook()and jit.status()==focusJit);work=0
 local byEnv,queries,ordinary,assemblies,applyFrames,mergeFrames,scaleFrames,addFrames={},{},{},{},{},{},{},{}
 local failure,sequence,activeScale
 local function environment(env)local r=byEnv[env];if not r then r={env=env,merges={},scaled={},deliveries={}};byEnv[env]=r end;return r end
 local wanted={[init]=true,[itemList]=true,[merge]=true,[scale]=true,[scaleList]=true,[listAdd]=true,[addList]=true,[tabulate]=true,[apply]=true,[assemble]=true,[load]=true}
 sequence=0
 local function hook(event)
  if event~="call"and event~="return"then return end
  local f=debug.getinfo(2,"f").func;if not wanted[f]then return end
  local v={};for i=1,192 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==itemList or f==merge or f==scaleList or f==addList then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
   sequence=sequence+1;assert(sequence<1000000)
   if f==load and event=="return"and v.node.elem=="Skill"then
    local set=v.self.skillSets[v.skillSetId];local group=v.socketGroup;assert(set.socketGroupList[#set.socketGroupList]==group)
    api.loaded[v.skillSetId]=api.loaded[v.skillSetId]or{};local gems={};for i,g in ipairs(group.gemList)do gems[i]=g end
    api.loaded[v.skillSetId][#set.socketGroupList]={group=group,set=set,gems=gems}
   elseif f==itemList and event=="return"and caller.func==init and caller.currentline==1322 and v.self.type=="Focus"then
    local r=environment(cv.env);assert(not r.item_return and cv.slotName=="Weapon 2"and cv.env.player.itemList[cv.slotName]==v.self)
    local returned=v.self.modList or v.self.slotModList[v.slotNum]
    r.item_return={object=returned,item=v.self,rows=records(returned),slot=cv.slotName,slot_num=v.slotNum,sequence=sequence}
   elseif f==merge and caller.func==init and(caller.currentline==1477 or caller.currentline==1482)and v.mod.name=="GemProperty"then
    local r=environment(cv.env);local ret=assert(r.item_return);assert(cv.srcList==ret.object and cv.item==ret.item and v.self==cv.combinedList)
    if event=="call"then
     local first=caller.currentline==1477;assert((first and v.skipNonAdditive==nil)or(not first and v.skipNonAdditive==true))
     local source=first and cv.srcList or cv.scaledList;local index;for i,m in ipairs(source)do if m==v.mod then assert(not index);index=i end end;assert(index)
     mergeFrames[#mergeFrames+1]={owner=r,list=v.self,argument=v.mod,row={caller_line=caller.currentline,source_index=index,record=plain(v.mod),before=records(v.self),skip_non_additive=not first,sequence=sequence}}
    else
     local frame=assert(table.remove(mergeFrames));assert(frame.owner==r and frame.argument==v.mod and frame.list==v.self)
     frame.row.after=records(v.self);frame.row.original_return=true;frame.row.exact_source_object=true;frame.row.exact_combined_destination=true
     local found=0;for _,m in ipairs(v.self)do if m==v.mod then found=found+1 end end;frame.row.argument_occurrences_after=found
     r.merges[#r.merges+1]=frame.row;r.combined=v.self
    end
   elseif f==scaleList and caller.func==init and caller.currentline==1480 then
    local r=environment(cv.env);assert(v.self==cv.scaledList and v.modList==cv.combinedList and v.scale==cv.scale)
    if event=="call"then assert(not activeScale);r.focus_effect_contributors=chain(cv.nodesModsList,"EffectOfBonusesFromFocus");activeScale={owner=r,destination=v.self,source=v.modList,factor=v.scale}
    else assert(activeScale and activeScale.owner==r);activeScale=nil end
   elseif f==scale and activeScale and v.self==activeScale.destination and v.mod.name=="GemProperty"then
    if event=="call"then
     assert(caller.func==scaleList and v.scale==activeScale.factor)
     local index;for i,m in ipairs(activeScale.source)do if m==v.mod then assert(not index);index=i end end;assert(index)
     scaleFrames[#scaleFrames+1]={owner=activeScale.owner,argument=v.mod,destination=v.self,row={source_index=index,record=plain(v.mod),factor=v.scale,insertions={},sequence=sequence}}
    else
     local frame=assert(table.remove(scaleFrames));assert(frame.argument==v.mod and #frame.row.insertions==1)
     assert(equal(frame.row.record,plain(v.mod)));frame.row.original_return=true;frame.owner.scaled[#frame.owner.scaled+1]=frame.row
    end
   elseif f==listAdd and event=="return"and #scaleFrames>0 then
    local frame=scaleFrames[#scaleFrames];if v.self~=frame.destination then return end
    assert(v.self[#v.self]==v.mod and v.mod~=frame.argument)
    frame.row.insertions[#frame.row.insertions+1]={record=plain(v.mod),exact_inserted_object=true,new_object=true}
   elseif f==addList and caller.func==init and caller.currentline==1484 then
    local r=environment(cv.env);assert(v.self==cv.env.itemModDB and v.modList==cv.combinedList)
    if event=="call"then
     addFrames[#addFrames+1]={owner=r,destination=v.self,list=v.modList,row={before=chain(v.self,"GemProperty"),source=records(v.modList),sequence=sequence}}
    else
     local frame=assert(table.remove(addFrames));assert(frame.owner==r and frame.list==v.modList and frame.destination==v.self)
     frame.row.after=chain(v.self,"GemProperty");frame.row.joins={}
     for i,m in ipairs(v.modList)do if m.name=="GemProperty"then
      local indices={};for j,d in ipairs(v.self.mods.GemProperty or{})do if d==m then indices[#indices+1]=j end end;assert(#indices>0)
      frame.row.joins[#frame.row.joins+1]={source_index=i,destination_indices=indices,record=plain(m)}
     end end
     frame.row.original_return=true;frame.row.exact_destination=true;r.deliveries[#r.deliveries+1]=frame.row
    end
   elseif f==tabulate and event=="return"and v.modType=="LIST"then queries[assert(v.result)]={store=v.self,sequence=sequence}
   elseif f==apply and target(v.effect)then
    if event=="call"then
     assert(not applyFrames[v.effect]);local q=assert(queries[v.modList]);local candidates={}
     for i,c in ipairs(v.modList)do assert(c.mod.name=="GemProperty");candidates[i]={index=i,record=plain(c.mod),value=plain(c.value)}end
     applyFrames[v.effect]={effect=v.effect,list=v.modList,query=q,before=input(v.effect),candidates=candidates,sequence=sequence}
    else
     local r=assert(applyFrames[v.effect]);applyFrames[v.effect]=nil;r.after=input(v.effect);r.matched={}
     for i,c in ipairs(r.list)do for _,m in ipairs(v.effect.gemPropertyInfo or{})do if c==m then r.matched[#r.matched+1]=i end end end
     ordinary[#ordinary+1]=r
    end
   elseif f==assemble and event=="return"and target(v.activeSkill.activeEffect)then
    assert(not assemblies[v.activeSkill]);assemblies[v.activeSkill]={env=v.env,after=input(v.activeSkill.activeEffect),sequence=sequence}
   elseif f==init and event=="return"then local r=environment(v.env);r.initialized=true;r.allocations=allocation(v.env)end
  end,debug.traceback)
  if not ok then failure=tostring(err);error(failure,0)end
 end
 if focusInstrumented then jit.flush();debug.sethook(hook,"cr")end
 return function()
  if focusInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not failure,failure)
  assert(calcs.initEnv==init and common.classes.Item.GetActiveModListForSlotNum==itemList and common.classes.ModList.MergeMod==merge)
  assert(common.classes.ModStore.ScaleAddMod==scale and common.classes.ModStore.ScaleAddList==scaleList and common.classes.ModList.AddMod==listAdd)
  assert(common.classes.ModDB.AddList==addList and common.classes.ModStore.Tabulate==tabulate and upvalue(init,"applyGemMods")==apply and calcs.buildActiveSkillModList==assemble)
  assert(not activeScale and #mergeFrames==0 and #scaleFrames==0 and #addFrames==0 and next(applyFrames)==nil)
  api.state={byEnv=byEnv,ordinary=ordinary,assemblies=assemblies};assert(jit.status()==focusJit)
 end
end
local function origins()
 local parsed,err=common.xml.ParseXML(focusXml);assert(parsed and not err)
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
function api.observe()
 assert(not debug.gethook()and api.state and jit.status()==focusJit)
 local state=api.state;local source=origins();local environments={}
 for _,mode in ipairs({"MAIN","CALCS"})do
  local env=mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv;local r=state.byEnv[env]
  assert(not focusInstrumented or r and r.initialized)
  local row={mode=mode,allocations=allocation(env),skills={},merges={},scaled={},deliveries={},path={}}
  for _,id in ipairs({8305,35880,20701,30265,23265,36891})do local node=env.spec.tree.nodes[id];assert(node)
   row.path[#row.path+1]={id=id,name=node.dn,allocated=env.allocNodes[id]~=nil,connections=plain(node.linkedId),ascendancy=node.ascendancyName or false}
  end
  if r then
   local ret=assert(r.item_return);row.item_return={item=item(ret.slot,ret.item),slot_num=ret.slot_num,records=ret.rows,original_return=true}
   row.merges=public_trace(r.merges);row.scaled=public_trace(r.scaled);row.deliveries=public_trace(r.deliveries);row.focus_effect_contributors=r.focus_effect_contributors
   if #r.merges>0 then assert(#r.merges==2 and #r.scaled==1 and #r.deliveries==1)
    assert(ret.sequence<r.merges[1].sequence and r.merges[1].sequence<r.scaled[1].sequence and r.scaled[1].sequence<r.merges[2].sequence and r.merges[2].sequence<r.deliveries[1].sequence)
   end
  end
  for _,a in ipairs(env.player.activeSkillList)do if target(a.activeEffect)then
   local e=a.activeEffect;local selected=mode=="CALCS"and e.statSetCalcs or e.statSet
   local s={effect=e.grantedEffect.id,source=assert(source[e.srcInstance]),raw=input(e.srcInstance),final=input(e),
    final_lookup=plain(e.grantedEffectLevel),stat_set=selected.index,exact_physical_source=e.gemData==e.srcInstance.gemData,ordinary={},assembly={}}
   if r then
    local ret=assert(r.item_return);for _,o in ipairs(state.ordinary)do if o.effect==e then
     assert(o.query.store==env.modDB and o.sequence>ret.sequence)
     if #r.deliveries>0 then assert(o.sequence>r.deliveries[1].sequence)end
     local joins={};for i,c in ipairs(o.list)do for j,m in ipairs(ret.object)do if m==c.mod then joins[#joins+1]={candidate_index=i,item_index=j}end end end
     s.ordinary[#s.ordinary+1]={before=o.before,after=o.after,candidates=o.candidates,matched=o.matched,item_object_joins=joins,original_query=true,exact_player_store=true}
    end end
    local assembly=assert(state.assemblies[a]);assert(assembly.env==env)
    s.assembly={after=assembly.after,original_call=true}
   end
   row.skills[#row.skills+1]=s
  end end
  environments[#environments+1]=row
 end
 return {methods=methods,environments=environments,outputs={MAIN=fields(build.calcsTab.mainOutput),CALCS=fields(build.calcsTab.calcsOutput)},
  instrumented=focusInstrumented,actual_calls=focusInstrumented,original_methods_preserved=true,hook_removed=true,business_wrappers=false,
  source_tables_mutated=false,diagnostic_requery=false,native_owner_closure=false,intended_gameplay_law=false}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision;local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
