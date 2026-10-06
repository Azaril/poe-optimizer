-- Finite reference-only timing proof. Observe original calls without replacing
-- business methods, querying them again, or promoting late LIST quirks to rules.
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
local sum,sa=original(common.classes.ModStore.Sum,"Classes/ModStore.lua",202)
local internal,si=original(common.classes.ModDB.SumInternal,"Classes/ModDB.lua",137)
local listInternal,li=original(common.classes.ModList.SumInternal,"Classes/ModList.lua",125)
local scale,sca=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local add,ada=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local tabulate,ta=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local apply,aa=original(upvalue(init,"applyGemMods"),"Modules/CalcSetup.lua",550)
local assemble,ba=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local callback,ca=original(runCallback,"HeadlessWrapper.lua",17)
local load=assert(djinnOriginals.refs.load_skill)
local methods={initialization=ia,performance=pa,sum=sa,sum_internal=si,list_sum_internal=li,
 scale=sca,add_database=ada,tabulate=ta,ordinary=aa,assembly=ba,callback=ca}
local function fields(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="string"or type(v)=="boolean"or type(v)=="number")then
  r[k]=type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)and tostring(v)or v
 end end;return r
end
local work=0
local function charge()work=work+1;assert(work<2000000,"late slot observer work bound")end
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
local function target(e)return e and e.grantedEffect and(e.grantedEffect.id=="PainOfferingPlayer"or e.grantedEffect.id=="IceNovaPlayer")end
local function item(slot,i)return {slot=slot,id=i.id,type=i.type,source=i.modSource,exact_registered=build.itemsTab.items[i.id]==i}end
local api={loaded={}}
function api.install()
 assert(not debug.gethook()and jit.status()==lateSlotJit);work=0
 local sequence,byEnv,envs,queries,ordinary,assemblies=0,{},{},{},{},{}
 local applyFrames,assemblyFrames,sumFrames,copyFrames={},{},{},{}
 local failure
 local function skills(env)
  local out={};for _,a in ipairs(env.player.activeSkillList)do if target(a.activeEffect)then
   local e=a.activeEffect;out[#out+1]={skill=a,effect=e,source=e.srcInstance,before=input(e),lookup=plain(e.grantedEffectLevel)}
  end end;return out
 end
 local wanted={[perform]=true,[sum]=true,[internal]=true,[listInternal]=true,[scale]=true,[add]=true,[tabulate]=true,[apply]=true,[assemble]=true,[load]=true}
 local function hook(event)
  if event~="call"and event~="return"then return end
  local f=debug.getinfo(2,"f").func;if not wanted[f]then return end
  local v={};for i=1,160 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==sum or f==scale then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
  sequence=sequence+1;assert(sequence<1000000)
  if f==load and event=="return"and v.node.elem=="Skill"then
   local set=v.self.skillSets[v.skillSetId];local group=v.socketGroup;assert(set.socketGroupList[#set.socketGroupList]==group)
   api.loaded[v.skillSetId]=api.loaded[v.skillSetId]or{};local gems={};for i,g in ipairs(group.gemList)do gems[i]=g end
   api.loaded[v.skillSetId][#set.socketGroupList]={group=group,set=set,gems=gems}
  elseif f==tabulate and event=="return"and v.modType=="LIST"then
   assert(v.result);queries[v.result]={store=v.self,sequence=sequence,cfg=fields(v.cfg)}
  elseif f==apply and target(v.effect)then
   if event=="call"then
    assert(not applyFrames[v.effect]);local q=assert(queries[v.modList],"ordinary property lacks original Tabulate return")
    local rows={};for i,r in ipairs(v.modList)do assert(i<2048 and r.mod.name=="GemProperty");rows[i]={index=i,record=plain(r.mod),value=plain(r.value)}end
    applyFrames[v.effect]={effect=v.effect,before=input(v.effect),query=q,list=v.modList,rows=rows,sequence=sequence}
   else
    local r=assert(applyFrames[v.effect]);applyFrames[v.effect]=nil;r.after=input(v.effect);r.matched={}
    for i,c in ipairs(r.list)do for _,m in ipairs(v.effect.gemPropertyInfo or{})do if m==c then r.matched[#r.matched+1]=i end end end
    ordinary[#ordinary+1]=r;assert(#ordinary<4096)
   end
  elseif f==assemble and target(v.activeSkill.activeEffect)then
   if event=="call"then assert(not assemblyFrames[v.activeSkill]);assemblyFrames[v.activeSkill]={env=v.env,before=input(v.activeSkill.activeEffect),sequence=sequence}
   else local r=assert(assemblyFrames[v.activeSkill]);assemblyFrames[v.activeSkill]=nil;r.after=input(v.activeSkill.activeEffect);assemblies[v.activeSkill]=r end
  elseif f==perform then
   if event=="call"then
    assert(not byEnv[v.env],"same environment performed twice in finite witness")
    local r={env=v.env,skills=skills(v.env),start=sequence,queries={},copies={},display={},before_bucket=chain(v.env.modDB,"GemProperty")}
    byEnv[v.env]=r;envs[#envs+1]=r;assert(#envs<128)
   else local r=assert(byEnv[v.env]);r.finish=sequence;r.after_bucket=chain(v.env.modDB,"GemProperty")
    for _,s in ipairs(r.skills)do s.after=input(s.effect);s.after_lookup=plain(s.effect.grantedEffectLevel)end
   end
  elseif f==sum and event=="call"and caller.func==perform and caller.currentline==1491 then
   local r=assert(byEnv[cv.env]);local slot=assert(cv.slot);assert(v.self==cv.env.modDB and v.modType=="INC"and v.cfg==nil)
   local name="EffectOfBonusesFrom"..slot;local q={slot=slot,name=name,item=item(slot,assert(cv.item)),before=chain(v.self,name),internal={},sequence=sequence}
   assert(cv.env.player.itemList[slot]==cv.item and not r.queries[slot]);r.queries[slot]=q
   sumFrames[v.self]={owner=r,row=q,name=name,store=v.self}
  elseif f==sum and event=="call"and caller.func==perform and caller.currentline>=3630 and caller.currentline<=3633 then
   local r=assert(byEnv[cv.env]);local a=cv.mainSkill;if not target(a.activeEffect)then return end
   local names={[3630]="GemLevel",[3631]="GemItemLevel",[3632]="GemSupportLevel",[3633]="GemCorruptionLevel"};local name=assert(names[caller.currentline])
   assert(v.self==a.skillModList and v.cfg==a.skillCfg and v.modType=="BASE")
   local q={name=name,before=chain(v.self,name),original_call=true,exact_skill=true,sequence=sequence};r.display[#r.display+1]=q
   sumFrames[v.self]={owner=r,row=q,name=name,store=v.self}
  elseif(f==internal or f==listInternal)and event=="return"then
   local r=sumFrames[v.context];if not r or r.name~=v.modName then return end
   if v.self==r.store then assert(type(v.result)=="number");r.row.result=v.result;r.row.original_return=true;sumFrames[v.context]=nil end
  elseif f==scale and v.mod and v.mod.name=="GemProperty"and caller.func==perform and caller.currentline==1526 then
   local r=assert(byEnv[cv.env]);local q=assert(r.queries[cv.slot]);assert(q.original_return)
   if event=="call"then
    assert(v.self==cv.modDB and v.self==r.env.modDB and v.mod==cv.modCopy and v.mod~=cv.mod and v.scale==q.result/100)
    local sourceList=cv.item.modList or cv.item.slotModList[2];local found={};for i,m in ipairs(sourceList)do if m==cv.mod then found[#found+1]=i end end;assert(#found==1)
    local row={slot=cv.slot,item=item(cv.slot,cv.item),source_index=found[1],source_record=plain(cv.mod),argument=plain(v.mod),factor=v.scale,
     original_query=true,exact_source_list=true,exact_copy_argument=true,exact_player_destination=true,insertions={},objects={},sequence=sequence}
    copyFrames[#copyFrames+1]={owner=r,row=row,argument=v.mod,source=cv.mod,destination=v.self}
   else
    local frame=assert(table.remove(copyFrames));assert(frame.owner==r and frame.argument==v.mod)
    assert(equal(frame.row.source_record,plain(frame.source))and #frame.row.insertions==1)
    frame.row.return_observed=true;r.copies[#r.copies+1]=frame.row
   end
  elseif f==add and event=="return"and #copyFrames>0 then
   local r=copyFrames[#copyFrames];if v.self~=r.destination then return end
   assert(v.self.mods[v.mod.name][#v.self.mods[v.mod.name]]==v.mod)
   r.row.insertions[#r.row.insertions+1]={record=plain(v.mod),exact_inserted_object=true,same_as_argument=v.mod==r.argument}
   r.row.objects[#r.row.objects+1]=v.mod
  end
  end,debug.traceback)
  if not ok then failure=tostring(err);error(failure,0)end
 end
 if lateSlotInstrumented then jit.flush();debug.sethook(hook,"cr")end
 return function()
  if lateSlotInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not failure,failure)
  assert(calcs.perform==perform and calcs.initEnv==init and calcs.buildActiveSkillModList==assemble and upvalue(init,"applyGemMods")==apply)
  assert(common.classes.ModStore.Sum==sum and common.classes.ModStore.Tabulate==tabulate and common.classes.ModStore.ScaleAddMod==scale
   and common.classes.ModDB.AddMod==add and common.classes.ModDB.SumInternal==internal and common.classes.ModList.SumInternal==listInternal)
  api.state={byEnv=byEnv,ordinary=ordinary,assemblies=assemblies,complete=next(applyFrames)==nil and next(assemblyFrames)==nil and next(sumFrames)==nil and #copyFrames==0}
  assert(jit.status()==lateSlotJit)
 end
end
local function origins()
 local parsed,err=common.xml.ParseXML(lateSlotXml);assert(parsed and not err)
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
 assert(not debug.gethook()and api.state and api.state.complete and jit.status()==lateSlotJit)
 local state=api.state;local source=origins();local environments={}
 for _,mode in ipairs({"MAIN","CALCS"})do
  local env=mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv;local r=state.byEnv[env]
  assert(not lateSlotInstrumented or r and r.finish,"selected original environment was not completed")
  local row={mode=mode,exact_selected_environment=true,queries={},copies={},skills={},display={}}
  if r then
   local slots={};for slot in pairs(r.queries)do slots[#slots+1]=slot end;table.sort(slots)
   for _,slot in ipairs(slots)do local q=r.queries[slot];assert(q.original_return);row.queries[#row.queries+1]={slot=q.slot,name=q.name,item=q.item,before=q.before,result=q.result,original_return=true}end
   for _,c in ipairs(r.copies)do local copy={};for k,v in pairs(c)do if k~="sequence"and k~="objects"then copy[k]=v end end;copy.after_preparation=c.sequence>r.start;row.copies[#row.copies+1]=copy end
   row.before_bucket=r.before_bucket;row.after_bucket=r.after_bucket
   for _,q in ipairs(r.display)do assert(q.original_return);row.display[#row.display+1]={name=q.name,result=q.result,before=q.before,original_call=true,original_return=true,exact_skill=true}end
  end
  for _,a in ipairs(env.player.activeSkillList)do if target(a.activeEffect)then
   local e=a.activeEffect;local selected=mode=="CALCS"and e.statSetCalcs or e.statSet;local physical=assert(source[e.srcInstance])
   local s={effect=e.grantedEffect.id,source=physical,raw=input(e.srcInstance),final=input(e),exact_physical_source=e.gemData==e.srcInstance.gemData,
    final_lookup=plain(e.grantedEffectLevel),root_lookup_present=e.grantedEffect.levels[e.level]~=nil,stat_set=selected.index,
    selected_main=env.player.mainSkill==a,ordinary={},assembly={}}
   if r then
    local found;for _,prepared in ipairs(r.skills)do if prepared.skill==a then assert(not found);found=prepared end end;assert(found and found.effect==e and found.source==e.srcInstance)
    s.prepared=found.before;s.after_perform=found.after;s.prepared_lookup=found.lookup;s.after_lookup=found.after_lookup
    for _,o in ipairs(state.ordinary)do if o.effect==e then assert(o.sequence<r.start and o.query.store==env.modDB)
     for _,candidate in ipairs(o.list)do for _,copy in ipairs(r.copies)do for _,inserted in ipairs(copy.objects)do
      assert(candidate.mod~=inserted,"late copied object entered original prepared input list")
     end end end
     s.ordinary[#s.ordinary+1]={before=o.before,after=o.after,candidates=o.rows,matched=o.matched,original_query=true,exact_actor_store=true,before_perform=true,no_late_copy_object_consumed=true}
    end end
    local assembly=assert(state.assemblies[a]);assert(assembly.env==env and assembly.sequence<r.start)
    s.assembly={before=assembly.before,after=assembly.after,original_call=true,before_perform=true}
   end
   row.skills[#row.skills+1]=s
  end end
  environments[#environments+1]=row
 end
 return {methods=methods,environments=environments,outputs={MAIN=fields(build.calcsTab.mainOutput),CALCS=fields(build.calcsTab.calcsOutput)},
  instrumented=lateSlotInstrumented,actual_calls=lateSlotInstrumented,original_methods_preserved=true,hook_removed=true,business_wrappers=false,
  source_tables_mutated=false,diagnostic_requery=false,roll_legality_authority=false,native_owner_closure=false}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision;local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
