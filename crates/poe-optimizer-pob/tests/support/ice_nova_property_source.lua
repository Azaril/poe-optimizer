-- Original call-boundary evidence, without replacements or property evaluation.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local info = debug.getinfo(assert(f), "S")
 local actual = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line)
 return f, {path=path, first=info.linedefined, last=info.lastlinedefined}
end
local function upvalue(f, wanted)
 for index=1,100 do
  local name,value=debug.getupvalue(f,index)
  if not name then break end
  if name==wanted then return value end
 end
 error("missing authenticated upvalue "..wanted)
end
local apply,applyAuth=original(upvalue(calcs.initEnv,"applyGemMods"),"Modules/CalcSetup.lua",550)
local properties,propertyAuth=original(upvalue(calcs.buildActiveSkillModList,"getSourceGemPropertyInfo"),"Modules/CalcActiveSkill.lua",236)
local validate,validateAuth=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local merge,mergeAuth=original(calcs.mergeSkillInstanceMods,"Modules/CalcActiveSkill.lua",116)
local create,createAuth=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local tabulate,tabulateAuth=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local scale,scaleAuth=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local addDb,addDbAuth=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local addList,addListAuth=original(common.classes.ModList.AddMod,"Classes/ModList.lua",29)
local choose,chooseAuth=original(upvalue(calcs.initEnv,"addBestSupport"),"Modules/CalcSetup.lua",586)
local loadSkill=assert(djinnOriginals.refs.load_skill)
local methods={external=applyAuth,properties=propertyAuth,validate=validateAuth,merge=mergeAuth,
 create=createAuth,tabulate=tabulateAuth,scale=scaleAuth,choose=chooseAuth,add_database=addDbAuth,add_list=addListAuth}
local controlCatalog={
 {"Metadata/Items/Gems/SkillGemUhtredExodusSupport","UhtredExodusSupport","SupportUhtredExodusPlayer","Uhtred's Exodus"},
 {"Metadata/Items/Gems/SkillGemMultishotSupport","MultishotSupport","SupportMultishotPlayer","Multishot I"},
 {"Metadata/Items/Gems/SkillGemUnleashSupport","UnleashSupport","SupportUnleashPlayer","Unleash"},
 {"Metadata/Items/Gems/SkillGemSalvoSupport","SalvoSupport","SupportSalvoPlayer","Salvo"}}
local controlEffects={}
for _,tuple in ipairs(controlCatalog) do controlEffects[tuple[3]]=tuple[1] end
local function scalar(v)
 assert(v==nil or type(v)=="number" or type(v)=="string" or type(v)=="boolean")
 if type(v)=="number" then assert(v==v and v~=math.huge and v~=-math.huge) end
 return v
end
local function scalars(t)
 local out={}
 for k,v in pairs(t or {}) do
  if type(k)=="string" and (type(v)=="number" or type(v)=="string" or type(v)=="boolean") then
   out[k]=type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or v
  end
 end
 return out
end
-- Preserve named fields and positional tags in mixed Lua tables independently.
local function plain(v,depth)
 if type(v)~="table" then return scalar(v) end
 depth=(depth or 0)+1;assert(depth<14)
 local named,positions,n={},{},0
 for k,x in pairs(v) do
  n=n+1;assert(n<=2048)
  if type(k)=="number" then positions[#positions+1]={index=k,value=plain(x,depth)}
  else assert(type(k)=="string");named[k]=plain(x,depth) end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 if #positions>0 then named._positions=positions end
 return named
end
local function mod(m)
 local out=scalars(m);out.value=plain(m.value);out.tags={}
 for i,t in ipairs(m) do out.tags[i]=plain(t) end
 return out
end
local function propertyRows(list)
 local out={}
 for i,row in ipairs(list or {}) do
  assert(i<=1024)
  out[i]={index=i,value=plain(row.value),mod=mod(assert(row.mod))}
 end
 return out
end
local propertyNames={GemProperty=true,SupportedGemProperty=true,["Multiplier:SupportCount"]=true}
local function storeChain(store)
 local out,seen={},{}
 while store do
  assert(not seen[store] and #out<16);seen[store]=true
  local rows={}
  if store.mods then
   for _,name in ipairs({"GemProperty","SupportedGemProperty","Multiplier:SupportCount"}) do
    for i,m in ipairs(store.mods[name] or {}) do rows[#rows+1]={name=name,index=i,mod=mod(m)} end
   end
  else
   for i,m in ipairs(store) do
    if propertyNames[m.name] then rows[#rows+1]={name=m.name,index=i,mod=mod(m)} end
   end
  end
  assert(#rows<=2048)
  out[#out+1]={kind=store.mods and "database" or "list",rows=rows,
   multipliers=scalars(store.multipliers),conditions=scalars(store.conditions)}
  store=store.parent
 end
 return out
end
local manualEffects={SummonSandDjinnPlayer=true,CommandSandDjinnKnifeThrowPlayer=true,
 SummonWaterDjinnPlayer=true,CommandWaterDjinnBubblePlayer=true}
local function target(e)
 if not e or not e.grantedEffect then return false end
 if e.grantedEffect.id=="IceNovaPlayer" or e.grantedEffect.id=="TwisterPlayer" then return true end
 local group=icePropertySourceGroups and icePropertySourceGroups[e.srcInstance]
 return manualEffects[e.grantedEffect.id] and group and group.source==nil
end
local function input(e) return {level=e.level,quality=e.quality,corrupted=e.corrupted,corrupt_level=e.corruptLevel} end
local function cfg(c)
 if not c then return {present=false} end
 return {present=true,fields=scalars(c),skill_gem=c.skillGem and c.skillGem.id,
  skill_gem_tags=c.skillGem and scalars(c.skillGem.tags)}
end

if icePropertyPhase=="before" then
 assert(not debug.gethook() and jit.status()==icePropertyJit)
 local auth={external={},cache={},validation={},constructors={},copies={},queries={},query_count=0,selections={},
  apply_frames={},cache_frames={},validate_frames={},merge_frames={},scale_frames={},add_frames={},choose_frames={},finished=false}
 icePropertyLoaded=icePropertyLoaded or {}
 icePropertySourceGroups=icePropertySourceGroups or {}
 local wanted={[apply]=true,[properties]=true,[validate]=true,[merge]=true,[create]=true,
  [tabulate]=true,[scale]=true,[loadSkill]=true,[choose]=true,[addDb]=true,[addList]=true}
 local function supportList(list)
  local out={}
  for i,s in ipairs(list) do
   out[i]={instance=s,source=s.srcInstance,index=i,effect=s.grantedEffect.id,
    prepared=input(s),superseded=s.superseded==true}
  end
  return out
 end
 local function hook(event)
  if event~="call" and event~="return" then return end
  local f=debug.getinfo(2,"f").func
  if not wanted[f] then return end
  local v={}
  for i=1,128 do local name,value=debug.getlocal(2,i);if not name then break end;v[name]=value end
  if f==loadSkill and event=="return" then
   local node,self,sid=assert(v.node),assert(v.self),assert(v.skillSetId)
   if node.elem~="Skill" then return end
   local set=assert(self.skillSets[sid]);local index=#set.socketGroupList
   local group=assert(set.socketGroupList[index]);assert(group==v.socketGroup)
   icePropertyLoaded[sid]=icePropertyLoaded[sid] or {}
   assert(not icePropertyLoaded[sid][index])
   icePropertyLoaded[sid][index]={group=group,set=set,gems={}}
   for i,g in ipairs(group.gemList) do
    icePropertyLoaded[sid][index].gems[i]=g;icePropertySourceGroups[g]=group
   end
  elseif f==tabulate and event=="return" and v.modType=="LIST" then
   local result=assert(v.result)
   -- Keep the exact result reference, not unrelated potentially functional values.
   auth.query_count=auth.query_count+1;assert(auth.query_count<=65536)
   auth.queries[result]={cfg=cfg(v.cfg),store=v.self,chain=storeChain(v.self)}
  elseif f==apply and target(v.effect) then
   local e=v.effect
   if event=="call" then
    assert(not auth.apply_frames[e])
    local r={effect=e,before=input(e),candidates=propertyRows(v.modList),list=v.modList,
     query=auth.queries[v.modList]}
    assert(r.query,"external candidates must come from an observed original query")
    auth.apply_frames[e]=r
   else
    local r=assert(auth.apply_frames[e]);auth.apply_frames[e]=nil;r.after=input(e);r.matched={};r.rejected={}
    for i,candidate in ipairs(r.list) do
     assert(candidate.mod.name=="GemProperty")
     local found=false
     for _,matched in ipairs(e.gemPropertyInfo or {}) do if matched==candidate then found=true end end
     local destination=found and r.matched or r.rejected;destination[#destination+1]=i
    end
    auth.external[#auth.external+1]=r;assert(#auth.external<=2048)
   end
  elseif f==create and event=="return" and target(v.activeEffect) then
   local a=assert(v.activeSkill)
   assert(a.activeEffect==v.activeEffect and a.actor==v.actor and a.socketGroup==v.socketGroup)
   auth.constructors[#auth.constructors+1]={skill=a,env=v.env,before=input(a.activeEffect)}
   assert(#auth.constructors<=2048)
  elseif f==properties and target(v.activeSkill and v.activeSkill.activeEffect) then
   local a=v.activeSkill;local source=assert(a.activeEffect.srcInstance)
   if event=="call" then
    local old=v.env.sourceGemPropertyInfo and v.env.sourceGemPropertyInfo[source]
    local r={skill=a,env=v.env,source=source,cache_hit=old~=nil,previous=old,
     before=input(a.activeEffect),actor_chain=storeChain(a.actor.modDB),supports={},merges={}}
    for i,s in ipairs(a.supportList) do
     local admitted=false
     for _,effect in ipairs(a.effectList) do if effect==s then admitted=true end end
     r.supports[i]={instance=s,source=s.srcInstance,index=i,effect=s.grantedEffect.id,
      hidden=s.grantedEffect.hidden==true,is_supporting=s.isSupporting and s.isSupporting[source]==true,
      admitted_to_effect=admitted,prepared=input(s)}
    end
    auth.cache_frames[#auth.cache_frames+1]=r
   else
    local r=assert(table.remove(auth.cache_frames));assert(r.skill==a and r.env==v.env)
    local result=assert(v.env.sourceGemPropertyInfo[source]);r.result=result;r.rows=propertyRows(result)
    for _,row in ipairs(result) do assert(row.mod.name=="SupportedGemProperty") end
    r.after=input(a.activeEffect);r.cache_identity_preserved=r.previous==nil or r.previous==result
    r.query=auth.queries[result]
    assert(r.query,"cached properties must refer to an original Tabulate result")
    auth.cache[#auth.cache+1]=r;assert(#auth.cache<=2048)
   end
  elseif f==merge and #auth.cache_frames>0 then
   local owner=auth.cache_frames[#auth.cache_frames]
   if event=="call" then
    local r={owner=owner,effect=v.skillEffect,list=v.modList,before=storeChain(v.modList)}
    auth.merge_frames[#auth.merge_frames+1]=r
   else
    local r=assert(table.remove(auth.merge_frames));assert(r.owner==owner and r.effect==v.skillEffect and r.list==v.modList)
    owner.merges[#owner.merges+1]={effect=r.effect,before=r.before,after=storeChain(v.modList)}
   end
  elseif f==validate and target(v.gemInstance) then
   local e=v.gemInstance
   if event=="call" then
    assert(not auth.validate_frames[e]);auth.validate_frames[e]={effect=e,before=input(e)}
   else
    local r=assert(auth.validate_frames[e]);auth.validate_frames[e]=nil;r.after=input(e)
    auth.validation[#auth.validation+1]=r;assert(#auth.validation<=4096)
   end
  elseif f==choose and (v.supportEffect.grantedEffect.id=="SupportUhtredExodusPlayer"
   or v.supportEffect.grantedEffect.id=="SupportSalvoPlayer"
   or v.supportEffect.grantedEffect.id=="SupportMultishotPlayer"
   or v.supportEffect.grantedEffect.id=="SupportUnleashPlayer") then
   if event=="call" then
    auth.choose_frames[#auth.choose_frames+1]={incoming=v.supportEffect,source=v.supportEffect.srcInstance,
     mode=v.mode,prepared=input(v.supportEffect),before=supportList(v.appliedSupportList)}
   else
    local r=assert(table.remove(auth.choose_frames));assert(r.incoming==v.supportEffect and r.mode==v.mode)
    r.after=supportList(v.appliedSupportList);auth.selections[#auth.selections+1]=r
    assert(#auth.selections<=1024)
   end
  elseif (f==addDb or f==addList) and #auth.scale_frames>0 then
   local owner=auth.scale_frames[#auth.scale_frames]
   if v.self==owner.destination then
    if event=="call" then
     auth.add_frames[#auth.add_frames+1]={owner=owner,object=v.mod,destination=v.self,
      inserted=mod(v.mod),same_as_input=v.mod==owner.object}
    else
     local r=assert(table.remove(auth.add_frames));assert(r.owner==owner and r.object==v.mod and r.destination==v.self)
     local list=f==addDb and assert(v.self.mods[v.mod.name]) or v.self
     assert(list[#list]==v.mod,"original AddMod did not append the observed object")
     owner.insertions[#owner.insertions+1]={mod=r.inserted,same_as_input=r.same_as_input,exact_destination=true,inserted=true}
    end
   end
  elseif f==scale and v.mod and propertyNames[v.mod.name] then
   if event=="call" then
    local caller=debug.getinfo(3,"Sl")
    auth.scale_frames[#auth.scale_frames+1]={object=v.mod,destination=v.self,before=mod(v.mod),scale=v.scale,insertions={},
     caller={path=caller.source:gsub("\\","/"),line=caller.currentline}}
   else
    local r=assert(table.remove(auth.scale_frames));assert(r.object==v.mod)
    r.input_after=mod(v.mod);auth.copies[#auth.copies+1]=r;assert(#auth.copies<=2048)
   end
  end
 end
 jit.flush();debug.sethook(hook,"cr");icePropertyAuth=auth
 return function()
  assert(debug.gethook()==hook);debug.sethook()
  assert(next(auth.apply_frames)==nil and next(auth.validate_frames)==nil and #auth.cache_frames==0
   and #auth.merge_frames==0 and #auth.scale_frames==0 and #auth.add_frames==0 and #auth.choose_frames==0)
  assert(jit.status()==icePropertyJit and calcLib.validateGemLevel==validate and calcs.createActiveSkill==create)
  assert(upvalue(calcs.initEnv,"applyGemMods")==apply and upvalue(calcs.buildActiveSkillModList,"getSourceGemPropertyInfo")==properties)
  assert(common.classes.ModStore.Tabulate==tabulate and common.classes.ModStore.ScaleAddMod==scale)
  assert(common.classes.ModDB.AddMod==addDb and common.classes.ModList.AddMod==addList)
  auth.finished=true
 end
end

assert(icePropertyPhase=="observe" and not debug.gethook() and jit.status()==icePropertyJit)
local auth=assert(icePropertyAuth);assert(auth.finished)
local doc,err=common.xml.ParseXML(icePropertyXml);assert(doc and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)
 if type(n)~="table" or not n.elem then return end
 ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;assert(nextOrdinal<100000)
 for _,c in ipairs(n) do enumerate(c) end
end
enumerate(doc[1])
local skills
for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Skills" then skills=n end end
local origins,originOwners,saved,changedGenerated,loadedControls={},{},{},{},{}
for _,set in ipairs(assert(skills)) do if type(set)=="table" and set.elem=="SkillSet" then
 local sid=assert(tonumber(set.attrib.id));local gi=0
 for _,group in ipairs(set) do if type(group)=="table" and group.elem=="Skill" then
  gi=gi+1;local loaded=assert(icePropertyLoaded[sid][gi]);local index=0
  assert(loaded.set==build.skillsTab.skillSets[sid])
  for _,node in ipairs(group) do if type(node)=="table" and node.elem=="Gem" then
   index=index+1;local gem=assert(loaded.gems[index])
   local sameObject=loaded.group.gemList[index]==gem
   local observed=node.attrib.skillId=="IceNovaPlayer" or node.attrib.skillId=="TwisterPlayer"
    or (manualEffects[node.attrib.skillId] and group.attrib.source==nil)
   if not sameObject then
    -- Original generated-group producers can replace their saved cache objects.
    -- Preserve that fact without joining a replacement to the saved occurrence.
    assert(group.attrib.source~=nil and not observed,"an observed authored source object was replaced")
    changedGenerated[#changedGenerated+1]={source_ordinal=ordinals[node],group_source=group.attrib.source,
     saved_effect=node.attrib.skillId,loaded_effect=gem.skillId,
     current_effect=loaded.group.gemList[index] and loaded.group.gemList[index].skillId,
     exact_saved_object=false,excluded_from_property_authority=true}
   end
   origins[gem]={source_ordinal=ordinals[node],preset=sid,index=index,group_source_ordinal=ordinals[group]}
   originOwners[gem]={loaded=loaded,index=index,effect=node.attrib.skillId}
   if controlEffects[node.attrib.skillId] then
    local catalog=assert(build.data.gems[controlEffects[node.attrib.skillId]])
    local byGameId=build.data.gemsByGameId[node.attrib.gemId]
    loadedControls[#loadedControls+1]={source_ordinal=ordinals[node],group_source_ordinal=ordinals[group],preset=sid,
     selected=sid==build.skillsTab.activeSkillSetId,attributes=scalars(node.attrib),
     loaded_effect=gem.skillId,loaded_catalog=gem.gemData and gem.gemData.id,
     exact_saved_object=sameObject,exact_catalog=gem.gemData==catalog,
     exact_game_id=byGameId~=nil and byGameId[node.attrib.variantId]==catalog,
     expected_catalog=catalog.id,expected_game_id=catalog.gameId,expected_variant=catalog.variantId}
   end
   if observed then
    assert(sameObject and node.attrib.skillId==gem.skillId)
    local runtimeIndex
    for i,g in ipairs(loaded.set.socketGroupList) do if g==loaded.group then assert(not runtimeIndex);runtimeIndex=i end end
    saved[#saved+1]={source_ordinal=ordinals[node],preset=sid,source_group_index=gi,runtime_group=runtimeIndex,
     attributes=scalars(node.attrib),group_attributes=scalars(group.attrib),loaded=scalars(gem),
     exact_saved_object=true,selected=sid==build.skillsTab.activeSkillSetId,
     source_classification=manualEffects[gem.skillId] and "manual_nonphysical_correspondence" or "physical_gem"}
   end
  end end
 end end
end end
local function sourceOrigin(instance)
 local owner=assert(originOwners[instance],"observed property input lacks a saved source owner")
 assert(owner.loaded.group.gemList[owner.index]==instance and instance.skillId==owner.effect,
  "observed property input lost its exact saved object")
 return assert(origins[instance])
end
local environments={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local contexts={}
for _,mode in ipairs({"MAIN","CALCS"}) do
 local env=environments[mode];local distinctCaches,cacheKeys={},{};local nextCacheKey=0
 for _,a in ipairs(env.player.activeSkillList) do if target(a.activeEffect) then
  local source=assert(a.activeEffect.srcInstance);local origin=sourceOrigin(source);local e=a.activeEffect
  local row={mode=mode,source=origin,effect=e.grantedEffect.id,actor_is_player=a.actor==env.player,
   source_catalog=assert(source.gemData).id,disabled=a.skillFlags and a.skillFlags.disable==true,
   actor_parent_present=a.actor.parent~=nil,slot=a.slotName,raw=input(source),final=input(e),
   source_classification=manualEffects[e.grantedEffect.id] and "manual_nonphysical_correspondence" or "physical_gem",
   stat_set=(mode=="MAIN" and e.statSet or e.statSetCalcs).index,
   external={},cache={},validation={},constructor={},source_effects={},actor_chain=storeChain(a.actor.modDB)}
  for i,g in ipairs(assert(source.gemData.grantedEffectList)) do row.source_effects[i]={index=i,effect=g.id,support=g.support==true} end
  for _,r in ipairs(auth.external) do if r.effect==e then
   row.external[#row.external+1]={before=r.before,after=r.after,candidates=r.candidates,matched=r.matched,rejected=r.rejected,
    query={cfg=r.query.cfg,chain=r.query.chain},query_store_is_actor=r.query.store==a.actor.modDB}
  end end
  for _,r in ipairs(auth.constructors) do if r.skill==a then
   assert(r.env==env);row.constructor[#row.constructor+1]={before=r.before,exact_actor=true,exact_source=true}
  end end
  local cacheTables={}
  for eventIndex,r in ipairs(auth.cache) do if r.skill==a then
   assert(r.source==source and r.env==env)
   local cr={event_index=eventIndex,cache_hit=r.cache_hit,cache_identity_preserved=r.cache_identity_preserved,before=r.before,after=r.after,
    actor_chain=r.actor_chain,properties=r.rows,query={cfg=r.query.cfg,chain=r.query.chain},supports={},merges={},
    query_parent_is_actor=r.query.store.parent==a.actor.modDB}
   for i,s in ipairs(r.supports) do
    local sameInstance
    for j=1,i-1 do if r.supports[j].instance==s.instance then sameInstance=j;break end end
    cr.supports[#cr.supports+1]={index=s.index,effect=s.effect,source=sourceOrigin(s.source),hidden=s.hidden,
     is_supporting=s.is_supporting==true,admitted_to_effect=s.admitted_to_effect,
     prepared=s.prepared,exact_source=true,same_instance_as=sameInstance}
   end
   for _,m in ipairs(r.merges) do
    local positions={}
    for _,s in ipairs(r.supports) do if s.instance==m.effect then
     assert(s.is_supporting);positions[#positions+1]=s.index
    end end
    assert(#positions>0)
    cr.merges[#cr.merges+1]={candidate_positions=positions,effect=m.effect.grantedEffect.id,before=m.before,after=m.after}
   end
   cacheTables[r.result]=true;row.cache[#row.cache+1]=cr
  end end
  local n=0;for _ in pairs(cacheTables) do n=n+1 end;row.cache_table_count=n
  for _,r in ipairs(auth.validation) do if r.effect==e then row.validation[#row.validation+1]={before=r.before,after=r.after} end end
  local cache=env.sourceGemPropertyInfo[source];row.cache_present=cache~=nil
  row.cache_is_observed=cache~=nil and cacheTables[cache]==true
  if cache then
   assert(not distinctCaches[cache] or distinctCaches[cache]==source,"distinct source instances shared a cache entry")
   distinctCaches[cache]=source
   if not cacheKeys[cache] then nextCacheKey=nextCacheKey+1;cacheKeys[cache]=nextCacheKey end
   row.cache_key=cacheKeys[cache]
  end
  row.distinct_source_cache=true
  row.final_row_available=e.grantedEffectLevel~=nil;row.final_row=scalars(e.grantedEffectLevel)
  contexts[#contexts+1]=row
 end end
end
local copies={}
for _,r in ipairs(auth.copies) do copies[#copies+1]={before=r.before,input_after=r.input_after,
 insertions=r.insertions,scale=r.scale,caller=r.caller} end
local selections={}
local function selectedRows(list)
 local out={}
 for _,s in ipairs(list) do
  out[#out+1]={index=s.index,effect=s.effect,source=sourceOrigin(s.source),prepared=s.prepared,superseded=s.superseded}
 end
 return out
end
for _,r in ipairs(auth.selections) do
 selections[#selections+1]={mode=r.mode,effect=r.incoming.grantedEffect.id,source=sourceOrigin(r.source),prepared=r.prepared,
  before=selectedRows(r.before),after=selectedRows(r.after)}
end
local outputs,amulets={},{}
for _,mode in ipairs({"MAIN","CALCS"}) do
 local output=mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
 outputs[mode]=scalars(assert(output))
 local item=environments[mode].player.itemList.Amulet
 amulets[mode]={present=item~=nil}
 if item then
  amulets[mode]={present=true,id=item.id,name=item.name,title=item.title,base=item.baseName,type=item.type,
   exact_registered=build.itemsTab.items[item.id]==item,raw_lines=copyTable(item.rawLines)}
 end
end
local catalog={}
for _,tuple in ipairs(controlCatalog) do
 local gem=assert(build.data.gems[tuple[1]])
 assert(gem.id==tuple[1] and gem.variantId==tuple[2] and gem.grantedEffect.id==tuple[3] and gem.name==tuple[4])
 catalog[#catalog+1]={key=gem.id,game_id=gem.gameId,variant=gem.variantId,effect=gem.grantedEffect.id,
  name=gem.name,support=gem.grantedEffect.support==true,source_families=plain(gem.grantedEffect.gemFamily)}
end
return {methods=methods,catalog=catalog,saved=saved,contexts=contexts,copies=copies,outputs=outputs,selections=selections,
 loaded_controls=loadedControls,equipped_amulets=amulets,
 excluded_changed_generated_sources=changedGenerated,
 selected={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec},
 observed_external_calls=#auth.external,observed_cache_calls=#auth.cache,observed_validation_calls=#auth.validation,
 original_methods_preserved=true,exact_loaded_objects_preserved=true,hook_removed=true,jit_mode_preserved=true,
 source_tables_mutated=false,business_wrappers=false}
