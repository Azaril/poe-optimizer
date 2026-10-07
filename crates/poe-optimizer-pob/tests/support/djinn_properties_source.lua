-- Observations of original Sand property consumers; no business replacements.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(assert(f),"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,name)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end
 error("missing original upvalue "..name)
end
local apply,aa=original(upvalue(calcs.initEnv,"applyGemMods"),"Modules/CalcSetup.lua",550)
local properties,pa=original(upvalue(calcs.buildActiveSkillModList,"getSourceGemPropertyInfo"),"Modules/CalcActiveSkill.lua",236)
local validate,va=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local merge,ma=original(calcs.mergeSkillInstanceMods,"Modules/CalcActiveSkill.lua",116)
local assemble,ba=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local create,ca=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local tabulate,ta=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local scale,sa=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local add,da=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local process=assert(djinnOriginals.refs.process_group)
local load=assert(djinnOriginals.refs.load_skill)
local methods={ordinary=aa,supported=pa,validation=va,merge=ma,assembly=ba,constructor=ca,tabulate=ta,scale=sa,add_database=da}
local SAND="SummonSandDjinnPlayer"
local COMMAND="CommandSandDjinnKnifeThrowPlayer"
local effects={[SAND]=true,[COMMAND]=true,KnifeThrowSandDjinn=true,ExplosiveTeleportSandDjinn=true,HandSlamSandDjinn=true}
local function definition(e)return e and (e.grantedEffect or e.gemData and e.gemData.grantedEffect)end
local function target(e)local d=definition(e);return d and effects[d.id]end
local function scalar(v)
 assert(v==nil or type(v)=="string" or type(v)=="number" or type(v)=="boolean")
 if type(v)=="number" then assert(v==v and v~=math.huge and v~=-math.huge) end
 return v
end
local function fields(t)
 local r={};for k,v in pairs(t or {})do if type(k)=="string" and (type(v)=="string" or type(v)=="number" or type(v)=="boolean")then
  r[k]=type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or v
 end end;return r
end
local function plain(v,depth)
 if type(v)~="table"then return scalar(v)end
 depth=(depth or 0)+1;assert(depth<14);local r,positions,n={},{},0
 for k,x in pairs(v)do n=n+1;assert(n<=4096)
  if type(k)=="number"then positions[#positions+1]={index=k,value=plain(x,depth)}else assert(type(k)=="string");r[k]=plain(x,depth)end
 end
 table.sort(positions,function(a,b)return a.index<b.index end);if #positions>0 then r.positions=positions end;return r
end
local function input(e)return {level=e.level,quality=e.quality,corrupted=e.corrupted,corrupt_level=e.corruptLevel}end
local function mod(m)local r=fields(m);r.value=plain(m.value);r.tags={};for i,t in ipairs(m)do r.tags[i]=plain(t)end;return r end
local function propertyRows(list)
 local r={};for i,v in ipairs(list or {})do assert(i<=2048);r[i]={index=i,value=plain(v.value),mod=mod(assert(v.mod))}end;return r
end
local function chain(store)
 local out,seen={},{};while store do assert(not seen[store] and #out<16);seen[store]=true;local rows={}
  if store.mods then for _,name in ipairs({"GemProperty","SupportedGemProperty","Multiplier:SupportCount"})do
   for i,m in ipairs(store.mods[name] or {})do assert(i<=2048);rows[#rows+1]={index=i,mod=mod(m)}end
  end else for i,m in ipairs(store)do assert(i<=16384);if m.name=="GemProperty" or m.name=="SupportedGemProperty" or m.name=="Multiplier:SupportCount"then rows[#rows+1]={index=i,mod=mod(m)}end end end
  out[#out+1]={depth=#out,kind=store.mods and "database" or "list",rows=rows};store=store.parent
 end;return out
end
local function cfg(c)return {present=c~=nil,fields=fields(c),skill_catalog=c and c.skillGem and c.skillGem.id}end
local function key(sid,source)return tostring(sid).."/"..(source or "manual")end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end
 for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function captureSupports(a,source)
 local out={};for i,s in ipairs(a.supportList)do local admitted=false;for _,e in ipairs(a.effectList)do if e==s then admitted=true end end
  out[i]={instance=s,source=s.srcInstance,index=i,effect=s.grantedEffect.id,hidden=s.grantedEffect.hidden==true,
   prepared=input(s),admitted_to_effect=admitted,is_supporting=source~=nil and s.isSupporting and s.isSupporting[source]==true or false}
 end;return out
end
if djinnPropertiesPhase=="before"then
 assert(not debug.gethook() and jit.status()==djinnJit)
 djinnPropertiesLoaded=djinnPropertiesLoaded or {}
 local auth={ordinary={},supported={},validation={},assemblies={},constructors={},copies={},queries={},query_count=0,
  ordinary_frames={},supported_frames={},validate_frames={},assembly_frames={},scale_frames={},add_frames={},support_merge_frames={}}
 djinnPropertiesAuth=auth
 local wanted={[apply]=true,[properties]=true,[validate]=true,[merge]=true,[assemble]=true,[create]=true,[tabulate]=true,[process]=true,[scale]=true,[add]=true}
 local function hook(event)
  if event~="call" and event~="return"then return end
  local f=debug.getinfo(2,"f").func;if not wanted[f]then return end
  local v={};for i=1,128 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller,cv
  if (f==process or f==scale) and event=="call"then
   caller=debug.getinfo(3,"fSl");cv={};for i=1,128 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end
  end
  local ok,err=xpcall(function()
   if f==process and event=="call" and caller.func==load then
    local group=v.socketGroup;local first=group.gemList[1]
    if first and first.skillId==SAND then
     local k=key(cv.skillSetId,group.source);assert(not djinnPropertiesLoaded[k])
     djinnPropertiesLoaded[k]={source=first,group=group,loaded=input(first),attributes=fields(cv.node[1].attrib),exact_load_call=true}
    end
   elseif f==tabulate and event=="return" and v.modType=="LIST"then
    auth.query_count=auth.query_count+1;assert(auth.query_count<=65536)
    auth.queries[assert(v.result)]={store=v.self,cfg=cfg(v.cfg),chain=chain(v.self)}
   elseif f==apply and target(v.effect)then
    local e=v.effect
    if event=="call"then
     assert(not auth.ordinary_frames[e]);auth.ordinary_frames[e]={effect=e,before=input(e),list=v.modList,candidates=propertyRows(v.modList),query=assert(auth.queries[v.modList],"ordinary result lacks original query")}
    else
     local r=assert(auth.ordinary_frames[e]);auth.ordinary_frames[e]=nil;r.after=input(e);r.matched={};r.rejected={}
     for i,c in ipairs(r.list)do assert(c.mod.name=="GemProperty");local yes=false
      for _,m in ipairs(e.gemPropertyInfo or {})do if m==c then yes=true end end
      local into=yes and r.matched or r.rejected;into[#into+1]=i
     end
     auth.ordinary[#auth.ordinary+1]=r;assert(#auth.ordinary<=2048)
    end
   elseif f==create and event=="return" and target(v.activeEffect)then
    local a=assert(v.activeSkill);assert(a.activeEffect==v.activeEffect and a.actor==v.actor and a.socketGroup==v.socketGroup and a.summonSkill==v.summonSkill)
    auth.constructors[#auth.constructors+1]={skill=a,env=v.env,input=input(a.activeEffect)};assert(#auth.constructors<=2048)
   elseif f==assemble and target(v.activeSkill.activeEffect)then
    local a=v.activeSkill
    if event=="call"then assert(not auth.assembly_frames[a]);auth.assembly_frames[a]={skill=a,env=v.env,before=input(a.activeEffect),merges={}}
    else local r=assert(auth.assembly_frames[a]);auth.assembly_frames[a]=nil;r.after=input(a.activeEffect);auth.assemblies[#auth.assemblies+1]=r;assert(#auth.assemblies<=2048)end
   elseif f==properties and target(v.activeSkill.activeEffect)then
    local a=v.activeSkill;local source=a.activeEffect.srcInstance
    if event=="call"then
     local old=source and v.env.sourceGemPropertyInfo and v.env.sourceGemPropertyInfo[source]
     auth.supported_frames[#auth.supported_frames+1]={skill=a,env=v.env,source=source,before=input(a.activeEffect),previous=old,cache_hit=old~=nil,
      supports=captureSupports(a,source),actor_chain=chain(a.actor.modDB),merges={}}
    else
     local r=assert(table.remove(auth.supported_frames));assert(r.skill==a and r.env==v.env);r.after=input(a.activeEffect)
     if source then
      r.result=assert(v.env.sourceGemPropertyInfo[source]);r.query=assert(auth.queries[r.result],"supported cache lacks original query")
      r.rows=propertyRows(r.result);r.cache_identity_preserved=r.previous==nil or r.previous==r.result
      for _,p in ipairs(r.result)do assert(p.mod.name=="SupportedGemProperty")end
     else assert(not a.activeEffect.gemData);r.rows={}end
     auth.supported[#auth.supported+1]=r;assert(#auth.supported<=4096)
    end
   elseif f==merge then
    if #auth.supported_frames>0 then
     local owner=auth.supported_frames[#auth.supported_frames]
     if event=="call"then auth.support_merge_frames[#auth.support_merge_frames+1]={owner=owner,effect=v.skillEffect,store=v.modList,before=chain(v.modList)}
     else local r=assert(table.remove(auth.support_merge_frames));assert(r.owner==owner and r.effect==v.skillEffect and r.store==v.modList)
      owner.merges[#owner.merges+1]={effect=v.skillEffect.grantedEffect.id,before=r.before,after=chain(v.modList)}end
    elseif event=="call" and target(v.skillEffect)then
     local owner;for a,r in pairs(auth.assembly_frames)do if a.activeEffect==v.skillEffect then assert(not owner);owner=r end end
     assert(owner,"active merge lacks assembly owner");owner.merges[#owner.merges+1]={before=input(v.skillEffect),exact_destination=v.modList==owner.skill.baseSkillModList}
    end
   elseif f==validate and target(v.gemInstance)then
    local e=v.gemInstance
    if event=="call"then assert(not auth.validate_frames[e]);auth.validate_frames[e]={effect=e,before=input(e),before_lookup=definition(e).levels[e.level]~=nil}
    else local r=assert(auth.validate_frames[e]);auth.validate_frames[e]=nil;r.after=input(e);r.after_lookup=definition(e).levels[e.level]~=nil;auth.validation[#auth.validation+1]=r;assert(#auth.validation<=4096)end
   elseif f==scale and v.mod and (v.mod.name=="GemProperty" or v.mod.name=="SupportedGemProperty")then
    if event=="call"then
     local r={object=v.mod,destination=v.self,before=mod(v.mod),scale=v.scale,insertions={},caller={path=caller.source:gsub("\\","/"),line=caller.currentline}}
     if caller.func==calcs.initEnv and caller.currentline==1667 then
      local item=assert(cv.env.player.itemList.Amulet);local found=false;for _,m in ipairs(cv.modList)do if m==cv.mod then found=true end end
      assert(found and cv.modCopy==v.mod and cv.mod~=v.mod and cv.modDB==v.self)
      r.amulet={item=item,original_record=mod(cv.mod),copied_from_actual_list=true,exact_copy_argument=true,exact_destination=true}
     end
     auth.scale_frames[#auth.scale_frames+1]=r
    else local r=assert(table.remove(auth.scale_frames));assert(r.object==v.mod);r.input_after=mod(v.mod);auth.copies[#auth.copies+1]=r;assert(#auth.copies<=4096)end
   elseif f==add and #auth.scale_frames>0 then
    local r=auth.scale_frames[#auth.scale_frames]
    if v.self==r.destination then
     if event=="call"then auth.add_frames[#auth.add_frames+1]={owner=r,object=v.mod,record=mod(v.mod),same_as_input=v.mod==r.object}
     else local a=assert(table.remove(auth.add_frames));assert(a.owner==r and a.object==v.mod and v.self.mods[v.mod.name][#v.self.mods[v.mod.name]]==v.mod);r.insertions[#r.insertions+1]=a end
    end
   end
  end,debug.traceback)
  if not ok then auth.hook_failure=tostring(err);error(auth.hook_failure,0)end
 end
 if djinnPropertiesInstrumented then jit.flush();debug.sethook(hook,"cr")end
 return function()
  if djinnPropertiesInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not auth.hook_failure,auth.hook_failure)
  assert(next(auth.ordinary_frames)==nil and next(auth.validate_frames)==nil and next(auth.assembly_frames)==nil and #auth.supported_frames==0 and #auth.scale_frames==0 and #auth.add_frames==0 and #auth.support_merge_frames==0,"unclosed source observation")
  assert(jit.status()==djinnJit and calcLib.validateGemLevel==validate and calcs.mergeSkillInstanceMods==merge and calcs.buildActiveSkillModList==assemble and calcs.createActiveSkill==create)
  assert(upvalue(calcs.initEnv,"applyGemMods")==apply and upvalue(assemble,"getSourceGemPropertyInfo")==properties)
  assert(common.classes.ModStore.Tabulate==tabulate and common.classes.ModStore.ScaleAddMod==scale and common.classes.ModDB.AddMod==add)
  auth.finished=true
 end
end
assert(djinnPropertiesPhase=="observe" and not debug.gethook() and jit.status()==djinnJit)
local auth=assert(djinnPropertiesAuth);assert(auth.finished)
local parsed,err=common.xml.ParseXML(djinnXml);assert(parsed and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)if type(n)~="table" or not n.elem then return end;ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;assert(nextOrdinal<100000);for _,c in ipairs(n)do enumerate(c)end end
enumerate(parsed[1]);local skills;for _,n in ipairs(parsed[1])do if type(n)=="table" and n.elem=="Skills"then skills=n end end
local saved,byKey={},{}
for _,set in ipairs(assert(skills))do if type(set)=="table" and set.elem=="SkillSet"then
 for _,group in ipairs(set)do if type(group)=="table" and group.elem=="Skill" and group[1] and group[1].attrib.skillId==SAND then
  local k=key(set.attrib.id,group.attrib.source);assert(not byKey[k]);local row={key=k,preset=tonumber(set.attrib.id),group_source=group.attrib.source,group_ordinal=ordinals[group],source_ordinal=ordinals[group[1]],attributes=fields(group[1].attrib),gems={}}
  for i,g in ipairs(group)do if type(g)=="table" and g.elem=="Gem"then row.gems[i]={index=i,source_ordinal=ordinals[g],attributes=fields(g.attrib)}end end
  byKey[k]=row;saved[#saved+1]=row
 end end
end end
local origins,groups={},{}
for sid,set in pairs(build.skillsTab.skillSets)do for _,g in ipairs(set.socketGroupList)do
 if g.gemList[1] and g.gemList[1].skillId==SAND then
  local k=key(sid,g.source);local row={key=k,preset=sid,group_source=g.source,no_supports=g.noSupports==true,selected=sid==build.skillsTab.activeSkillSetId,
   source_node=g.sourceNode and g.sourceNode.id,source_item=g.sourceItem and g.sourceItem.id}
  if byKey[k]then row.saved_source_ordinal=byKey[k].source_ordinal end
  assert(not groups[g]);groups[g]=row
  for i,s in ipairs(g.gemList)do assert(not origins[s]);origins[s]={key=k,index=i,effect=s.skillId,source_ordinal=byKey[k] and byKey[k].gems[i] and byKey[k].gems[i].source_ordinal}end
 end
end end
local function supportsOut(supports)
 local rows={};for _,s in ipairs(supports)do rows[#rows+1]={index=s.index,effect=s.effect,hidden=s.hidden,prepared=s.prepared,
  admitted_to_effect=s.admitted_to_effect,is_supporting=s.is_supporting,source=origins[s.source],source_present=s.source~=nil,source_joined=origins[s.source]~=nil}end;return rows
end
local function population(a)
 local out={present=a.minion~=nil};if not a.minion then return out end
 local m=a.minion;out.level=m.level;out.type=m.type;out.definition=m.minionData and m.minionData.name
 out.table_input=a.activeEffect.level;out.table_output=build.data.minionLevelTable[a.activeEffect.level]
 out.overrides={};for _,k in ipairs({"minionLevelIsEnemyLevel","minionLevelIsTriggeredSkillLevel","minionLevelIsPlayerLevel","minionLevel"})do
  out.overrides[k]={present=a.skillData[k]~=nil,value=scalar(a.skillData[k])}
 end
 out.children={};for _,child in ipairs(m.activeSkillList or {})do local e=child.activeEffect
  out.children[#out.children+1]={effect=e.grantedEffect.id,input=input(e),actor_level=e.actorLevel,source_instance_present=e.srcInstance~=nil,
   catalog_present=e.gemData~=nil,exact_parent=child.summonSkill==a,exact_actor=child.actor==m}
 end;return out
end
local contexts,outcomes,equipped={},{},{}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 equipped[mode]={};local slots={};for slot,item in pairs(env.player.itemList)do if type(slot)=="string" and type(item)=="table" and item.id then slots[#slots+1]=slot end end;table.sort(slots)
 for _,slot in ipairs(slots)do local item=env.player.itemList[slot];equipped[mode][#equipped[mode]+1]={slot=slot,id=item.id,name=item.name,type=item.type,raw_lines=plain(item.rawLines),exact_registered=build.itemsTab.items[item.id]==item}end
 local caches={}
 local function observe(a,parent)
  local e=a.activeEffect;local source=e.srcInstance;local group=assert(groups[(parent or a).socketGroup]);local origin=source and assert(origins[source])
  local row={mode=mode,effect=e.grantedEffect.id,group=group,source=origin,source_instance_present=source~=nil,catalog_present=e.gemData~=nil,
   catalog=e.gemData and {id=e.gemData.id,name=e.gemData.name,tags=fields(e.gemData.tags),req_str=e.gemData.reqStr,req_dex=e.gemData.reqDex,req_int=e.gemData.reqInt,natural_max=e.gemData.naturalMaxLevel},
   raw=source and input(source),final=input(e),parent_present=parent~=nil,actor_is_player=a.actor==env.player,
   ordinary={},supported={},validation={},source_validation={},assembly={},constructor={},population=population(a)}
  if source then assert(a.actor==env.player and source.gemData==e.gemData and source==a.socketGroup.gemList[1]);row.exact_group_source=true
   local loaded=djinnPropertiesLoaded[group.key]
   if loaded then
    assert(equal(loaded.attributes,assert(byKey[group.key]).attributes),"loaded raw source is not the saved source")
    row.loaded={input=loaded.loaded,attributes=loaded.attributes,same_source=loaded.source==source,exact_load_call=loaded.exact_load_call}
   end
   local cache=env.sourceGemPropertyInfo and env.sourceGemPropertyInfo[source]
   row.source_cache_present=cache~=nil
   if cache then assert(not caches[cache] or caches[cache]==source,"distinct sources share cache");caches[cache]=source end
  else assert(parent and a.summonSkill==parent and a.actor==parent.minion)end
  for _,r in ipairs(auth.ordinary)do if r.effect==e then local joins={}
   for j,c in ipairs(r.list)do local ids={};for k,cp in ipairs(auth.copies)do for _,ins in ipairs(cp.insertions)do if ins.object==c.mod then ids[#ids+1]=k end end end;joins[j]=ids end
   row.ordinary[#row.ordinary+1]={before=r.before,after=r.after,candidates=r.candidates,matched=r.matched,rejected=r.rejected,query={cfg=r.query.cfg,chain=r.query.chain},query_store_is_actor=r.query.store==a.actor.modDB,copy_indices=joins}
  end end
  for _,r in ipairs(auth.supported)do if r.skill==a then assert(r.env==env and r.source==source)
   row.supported[#row.supported+1]={before=r.before,after=r.after,cache_hit=r.cache_hit,cache_identity_preserved=r.cache_identity_preserved,
    properties=r.rows,query=r.query and {cfg=r.query.cfg,chain=r.query.chain},query_parent_is_actor=r.query and r.query.store.parent==a.actor.modDB,
    actor_chain=r.actor_chain,supports=supportsOut(r.supports),merges=r.merges,source_instance_present=source~=nil}
  end end
  for _,r in ipairs(auth.validation)do local record={before=r.before,after=r.after,before_lookup=r.before_lookup,after_lookup=r.after_lookup}
   if r.effect==e then row.validation[#row.validation+1]=record end
   if source and r.effect==source then row.source_validation[#row.source_validation+1]=record end
  end
  for _,r in ipairs(auth.assemblies)do if r.skill==a then assert(r.env==env);row.assembly[#row.assembly+1]={before=r.before,after=r.after,merges=r.merges}end end
  for _,r in ipairs(auth.constructors)do if r.skill==a then assert(r.env==env);row.constructor[#row.constructor+1]=r.input end end
  contexts[#contexts+1]=row
  outcomes[#outcomes+1]={mode=mode,effect=row.effect,group=group,raw=row.raw,final=row.final,source_instance_present=row.source_instance_present,catalog=row.catalog,population=row.population,
   source_cache_present=row.source_cache_present,actor_level=e.actorLevel}
 end
 for _,a in ipairs(env.player.activeSkillList)do if a.activeEffect.grantedEffect.id==SAND or a.activeEffect.grantedEffect.id==COMMAND then
  observe(a)
  if a.minion then for _,child in ipairs(a.minion.activeSkillList)do assert(effects[child.activeEffect.grantedEffect.id]);observe(child,a)end end
 end end
end
local function order(a,b)return a.mode.."/"..a.group.key.."/"..a.effect<b.mode.."/"..b.group.key.."/"..b.effect end
table.sort(contexts,order);table.sort(outcomes,order)
local copies={};for i,r in ipairs(auth.copies)do local row={index=i,before=r.before,input_after=r.input_after,scale=r.scale,caller=r.caller,insertions={}}
 for _,s in ipairs(r.insertions)do row.insertions[#row.insertions+1]={record=s.record,same_as_input=s.same_as_input,inserted=true}end
 if r.amulet then local p=r.amulet;row.amulet={id=p.item.id,exact_registered=build.itemsTab.items[p.item.id]==p.item,original_record=p.original_record,
  copied_from_actual_list=p.copied_from_actual_list,exact_copy_argument=p.exact_copy_argument,exact_destination=p.exact_destination}end
 copies[#copies+1]=row
end
local levels={};for _,id in ipairs({SAND,COMMAND,"KnifeThrowSandDjinn","ExplosiveTeleportSandDjinn","HandSlamSandDjinn"})do
 local d=assert(build.data.skills[id]);local domain={};for n in pairs(d.levels)do domain[#domain+1]=n end;table.sort(domain)
 levels[#levels+1]={effect=id,domain=domain,quality_stats=plain(d.qualityStats)}
end
local tableRows={};for i,v in ipairs(build.data.minionLevelTable)do tableRows[#tableRows+1]={input=i,output=v}end
return {methods=methods,saved=saved,contexts=contexts,outcomes=outcomes,copies=copies,equipped=equipped,level_domains=levels,minion_level_table=tableRows,
 instrumented=djinnPropertiesInstrumented,actual_original_calls_observed=djinnPropertiesInstrumented,original_methods_preserved=true,hook_removed=true,
 jit_mode_preserved=true,business_wrappers=false,source_tables_mutated=false,diagnostic_queries_as_consumption=false}
