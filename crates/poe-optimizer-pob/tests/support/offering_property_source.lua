-- Actual original property call boundaries; no method or source data replacement.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(assert(f),"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua" and p:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,wanted)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==wanted then return v end end
 error("missing original upvalue "..wanted)
end
local apply,aa=original(upvalue(calcs.initEnv,"applyGemMods"),"Modules/CalcSetup.lua",550)
local properties,pa=original(upvalue(calcs.buildActiveSkillModList,"getSourceGemPropertyInfo"),"Modules/CalcActiveSkill.lua",236)
local validate,va=original(calcLib.validateGemLevel,"Modules/CalcTools.lua",60)
local merge,ma=original(calcs.mergeSkillInstanceMods,"Modules/CalcActiveSkill.lua",116)
local assemble,ba=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426)
local tabulate,ta=original(common.classes.ModStore.Tabulate,"Classes/ModStore.lua",345)
local scale,sa=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
local add,da=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
local load=assert(djinnOriginals.refs.load_skill)
local methods={ordinary=aa,supported=pa,validation=va,merge=ma,assembly=ba,tabulate=ta,scale=sa,add_database=da}
local function scalar(v)
 assert(v==nil or type(v)=="string" or type(v)=="boolean" or type(v)=="number")
 if type(v)=="number" then assert(v==v and v~=math.huge and v~=-math.huge) end
 return v
end
local function fields(t)
 local r={};for k,v in pairs(t or {}) do if type(k)=="string" and (type(v)=="string" or type(v)=="number" or type(v)=="boolean") then
  r[k]=type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or v
 end end;return r
end
local function plain(v,depth)
 if type(v)~="table" then return scalar(v) end
 depth=(depth or 0)+1;assert(depth<14);local r,positions,n={},{},0
 for k,x in pairs(v) do n=n+1;assert(n<=2048)
  if type(k)=="number" then positions[#positions+1]={index=k,value=plain(x,depth)} else assert(type(k)=="string");r[k]=plain(x,depth) end
 end
 table.sort(positions,function(a,b)return a.index<b.index end);if #positions>0 then r.positions=positions end;return r
end
local function mod(m)
 local r=fields(m);r.value=plain(m.value);r.tags={};for i,t in ipairs(m) do r.tags[i]=plain(t) end;return r
end
local function input(e)return {level=e.level,quality=e.quality,corrupted=e.corrupted,corrupt_level=e.corruptLevel}end
local function definition(e)return e and (e.grantedEffect or e.gemData and e.gemData.grantedEffect)end
local function target(e)local d=definition(e);return d and d.id=="PainOfferingPlayer"end
local function cfg(c)return {present=c~=nil,fields=fields(c),skill_gem=c and c.skillGem and c.skillGem.id}end
local function listRows(list)
 local r={};for i,v in ipairs(list or {}) do assert(i<=2048);r[i]={index=i,value=plain(v.value),mod=mod(assert(v.mod))} end;return r
end
local function chain(store)
 local out,seen={},{};while store do assert(not seen[store] and #out<16);seen[store]=true;local r={}
  if store.mods then for _,name in ipairs({"GemProperty","SupportedGemProperty","Multiplier:SupportCount"}) do
   for i,m in ipairs(store.mods[name] or {}) do assert(i<=2048);r[#r+1]={index=i,mod=mod(m)} end
  end else for i,m in ipairs(store) do assert(i<=16384);if m.name=="GemProperty" or m.name=="SupportedGemProperty" or m.name=="Multiplier:SupportCount" then r[#r+1]={index=i,mod=mod(m)} end end end
  out[#out+1]={depth=#out,kind=store.mods and "database" or "list",rows=r};store=store.parent
 end;return out
end
if offeringPropertyPhase=="before" then
 assert(not debug.gethook() and jit.status()==offeringPropertyJit)
 offeringPropertyLoaded=offeringPropertyLoaded or {}
 local auth={ordinary={},supported={},validation={},assemblies={},copies={},queries={},query_count=0,
  ordinary_frames={},supported_frames={},validate_frames={},assembly_frames={},scale_frames={},add_frames={},finished=false}
 offeringPropertyAuth=auth
 local wanted={[apply]=true,[properties]=true,[validate]=true,[merge]=true,[assemble]=true,[tabulate]=true,[scale]=true,[add]=true,[load]=true}
 local function hook(event)
  if event~="call" and event~="return" then return end
  local f=debug.getinfo(2,"f").func;if not wanted[f] then return end
  local v={};for i=1,128 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  -- Capture stack-relative data before entering the protected observer body.
  -- A sticky original observer traceback must survive lifecycle cleanup.
  local caller,callerValues
  if f==scale and event=="call" and v.mod and v.mod.name=="GemProperty" then
   caller=debug.getinfo(3,"fSl");callerValues={}
   for i=1,128 do local n,x=debug.getlocal(3,i);if not n then break end;callerValues[n]=x end
  end
  local ok,err=xpcall(function()
  if f==load and event=="return" and v.node.elem=="Skill" then
   local sid=v.skillSetId;local set=v.self.skillSets[sid];local group=v.socketGroup;assert(set.socketGroupList[#set.socketGroupList]==group)
   offeringPropertyLoaded[sid]=offeringPropertyLoaded[sid] or {};local gems={};for i,g in ipairs(group.gemList) do gems[i]=g end
   offeringPropertyLoaded[sid][#set.socketGroupList]={group=group,set=set,gems=gems}
  elseif f==tabulate and event=="return" and v.modType=="LIST" then
   auth.query_count=auth.query_count+1;assert(auth.query_count<=65536)
   auth.queries[assert(v.result)]={store=v.self,cfg=cfg(v.cfg),chain=chain(v.self)}
  elseif f==apply and target(v.effect) then
   local e=v.effect
   if event=="call" then
    assert(not auth.ordinary_frames[e]);local query=assert(auth.queries[v.modList],"ordinary list lacks actual query")
    auth.ordinary_frames[e]={effect=e,before=input(e),list=v.modList,candidates=listRows(v.modList),query=query}
   else
    local r=assert(auth.ordinary_frames[e]);auth.ordinary_frames[e]=nil;r.after=input(e);r.matched={};r.rejected={}
    for i,c in ipairs(r.list) do assert(c.mod.name=="GemProperty");local yes=false
     for _,m in ipairs(e.gemPropertyInfo or {}) do if m==c then yes=true end end
     local into=yes and r.matched or r.rejected;into[#into+1]=i
    end
    auth.ordinary[#auth.ordinary+1]=r;assert(#auth.ordinary<=2048)
   end
  elseif f==assemble and target(v.activeSkill.activeEffect) then
   local a=v.activeSkill
   if event=="call" then assert(not auth.assembly_frames[a]);auth.assembly_frames[a]={skill=a,env=v.env,before=input(a.activeEffect),merges={}}
   else local r=assert(auth.assembly_frames[a]);auth.assembly_frames[a]=nil;r.after=input(a.activeEffect);auth.assemblies[#auth.assemblies+1]=r;assert(#auth.assemblies<=2048) end
  elseif f==properties and target(v.activeSkill.activeEffect) then
   local a=v.activeSkill;local source=assert(a.activeEffect.srcInstance)
   if event=="call" then
    local old=v.env.sourceGemPropertyInfo and v.env.sourceGemPropertyInfo[source];local supports={}
    for i,s in ipairs(a.supportList) do local admitted=false;for _,e in ipairs(a.effectList) do if e==s then admitted=true end end
     supports[i]={instance=s,source=s.srcInstance,index=i,effect=s.grantedEffect.id,prepared=input(s),admitted=admitted,is_supporting=s.isSupporting and s.isSupporting[source]==true}
    end
    auth.supported_frames[#auth.supported_frames+1]={skill=a,env=v.env,source=source,before=input(a.activeEffect),previous=old,cache_hit=old~=nil,supports=supports}
   else
    local r=assert(table.remove(auth.supported_frames));assert(r.skill==a and r.env==v.env)
    r.result=assert(v.env.sourceGemPropertyInfo[source]);r.query=assert(auth.queries[r.result],"supported cache lacks actual query")
    r.rows=listRows(r.result);r.after=input(a.activeEffect);r.cache_identity_preserved=r.previous==nil or r.previous==r.result
    for _,p in ipairs(r.result) do assert(p.mod.name=="SupportedGemProperty") end
    auth.supported[#auth.supported+1]=r;assert(#auth.supported<=2048)
   end
  elseif f==merge and event=="call" and target(v.skillEffect) then
   local owner;for a,r in pairs(auth.assembly_frames) do if a.activeEffect==v.skillEffect then assert(not owner);owner=r end end
   assert(owner,"active merge lacks actual assembly owner")
   owner.merges[#owner.merges+1]={before=input(v.skillEffect),exact_destination=v.modList==owner.skill.baseSkillModList,
    -- CalcFullDPS also constructs CALCULATOR environments. The original
    -- assembly selects CALCS only for that exact mode, and statSet otherwise.
    stat_set_exact=v.statSet==(owner.env.mode=="CALCS" and v.skillEffect.statSetCalcs.statSet or v.skillEffect.statSet.statSet)}
  elseif f==validate and target(v.gemInstance) then
   local e=v.gemInstance
   if event=="call" then assert(not auth.validate_frames[e]);auth.validate_frames[e]={effect=e,before=input(e),before_lookup=definition(e).levels[e.level]~=nil}
   else local r=assert(auth.validate_frames[e]);auth.validate_frames[e]=nil;r.after=input(e);r.after_lookup=definition(e).levels[e.level]~=nil;auth.validation[#auth.validation+1]=r;assert(#auth.validation<=4096) end
  elseif f==scale and v.mod and v.mod.name=="GemProperty" then
   if event=="call" then
    local r={object=v.mod,destination=v.self,before=mod(v.mod),scale=v.scale,insertions={},caller={path=caller.source:gsub("\\","/"),line=caller.currentline}}
    if caller.func==calcs.initEnv and caller.currentline==1667 then
     local cv=callerValues
     local item=assert(cv.env.player.itemList.Amulet);local found=false;for _,m in ipairs(cv.modList) do if m==cv.mod then found=true end end
     assert(found,"Amulet original record is absent from actual source list")
     assert(cv.modCopy==v.mod and cv.mod~=v.mod,"Amulet copy argument lost original object ancestry")
     assert(cv.modDB==v.self,"Amulet copy destination differs from original actor store")
     r.amulet={item=item,original=cv.mod,original_record=mod(cv.mod),copied_from_actual_list=true,exact_copy_argument=true,exact_destination=true}
    end
    auth.scale_frames[#auth.scale_frames+1]=r
   else local r=assert(table.remove(auth.scale_frames));assert(r.object==v.mod);r.input_after=mod(v.mod);auth.copies[#auth.copies+1]=r;assert(#auth.copies<=4096) end
  elseif f==add and #auth.scale_frames>0 then
   local r=auth.scale_frames[#auth.scale_frames]
   if v.self==r.destination then
    if event=="call" then auth.add_frames[#auth.add_frames+1]={owner=r,object=v.mod,record=mod(v.mod),same_as_input=v.mod==r.object}
    else local a=assert(table.remove(auth.add_frames));assert(a.owner==r and a.object==v.mod and v.self.mods[v.mod.name][#v.self.mods[v.mod.name]]==v.mod);r.insertions[#r.insertions+1]=a end
   end
  end
  end,debug.traceback)
  if not ok then auth.hook_failure=tostring(err);error(auth.hook_failure,0) end
 end
 if offeringPropertyInstrumented then jit.flush();debug.sethook(hook,"cr") end
 return function()
  if offeringPropertyInstrumented then assert(debug.gethook()==hook);debug.sethook() end
  assert(not auth.hook_failure,auth.hook_failure)
  local function count(t)local n=0;for _ in pairs(t)do n=n+1 end;return n end
  local outstanding={ordinary=count(auth.ordinary_frames),validation=count(auth.validate_frames),assembly=count(auth.assembly_frames),supported=#auth.supported_frames,scale=#auth.scale_frames,add=#auth.add_frames}
  local pending={};for _,name in ipairs({"ordinary","validation","assembly","supported","scale","add"})do if outstanding[name]~=0 then pending[#pending+1]=name.."="..outstanding[name]end end
  assert(jit.status()==offeringPropertyJit and calcLib.validateGemLevel==validate and calcs.buildActiveSkillModList==assemble and calcs.mergeSkillInstanceMods==merge)
  assert(upvalue(calcs.initEnv,"applyGemMods")==apply and upvalue(assemble,"getSourceGemPropertyInfo")==properties)
  assert(common.classes.ModStore.Tabulate==tabulate and common.classes.ModStore.ScaleAddMod==scale and common.classes.ModDB.AddMod==add)
  -- Initial-load infrastructure must first return an original source failure.
  -- On a successful load the snapshot below refuses any unmatched frame.
  auth.incomplete=#pending>0 and table.concat(pending,", ") or nil
  auth.finished=auth.incomplete==nil
 end
end
assert(offeringPropertyPhase=="observe" and not debug.gethook() and jit.status()==offeringPropertyJit)
local auth=assert(offeringPropertyAuth);assert(auth.finished,"incomplete Offering observer frames: "..tostring(auth.incomplete))
local parsed,err=common.xml.ParseXML(offeringPropertyXml);assert(parsed and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)if type(n)~="table" or not n.elem then return end;ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;assert(nextOrdinal<100000);for _,c in ipairs(n)do enumerate(c)end end
enumerate(parsed[1]);local skills;for _,n in ipairs(parsed[1])do if type(n)=="table" and n.elem=="Skills"then skills=n end end
local origins,saved={},{}
for _,set in ipairs(assert(skills))do if type(set)=="table" and set.elem=="SkillSet"then
 local sid=assert(tonumber(set.attrib.id));local gi=0
 for _,group in ipairs(set)do if type(group)=="table" and group.elem=="Skill"then gi=gi+1;local i=0
  local loaded=offeringPropertyLoaded[sid] and offeringPropertyLoaded[sid][gi]
  for _,g in ipairs(group)do if type(g)=="table" and g.elem=="Gem"then i=i+1
   local runtime=loaded and loaded.gems[i] or build.skillsTab.skillSets[sid].socketGroupList[gi].gemList[i]
   if g.attrib.skillId=="PainOfferingPlayer" or (loaded and loaded.group.gemList[i]==runtime and group.attrib.source==nil)then
    assert(runtime and runtime.skillId==g.attrib.skillId and group.attrib.source==nil)
    if loaded then assert(loaded.group.gemList[i]==runtime and loaded.set==build.skillsTab.skillSets[sid])end
    origins[runtime]={source_ordinal=ordinals[g],group_source_ordinal=ordinals[group],preset=sid,index=i,attributes=fields(g.attrib)}
    if g.attrib.skillId=="PainOfferingPlayer"then saved[#saved+1]=origins[runtime]end
   end
  end end
 end end
end end
local contexts,outcomes={},{}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv);local caches={}
 for _,a in ipairs(env.player.activeSkillList)do if target(a.activeEffect)then
  local e=a.activeEffect;local source=assert(e.srcInstance);local origin=assert(origins[source]);assert(source.gemData==e.gemData and a.actor==env.player)
  local selected=assert(mode=="MAIN" and e.statSet or e.statSetCalcs,"missing selected effect stat set")
  local set=assert(selected.statSet,"missing selected stat-set definition")
  local flags=assert(selected.skillFlags,"missing selected stat-set flags")
  local row={mode=mode,source=origin,raw=input(source),final=input(e),actor_is_player=true,no_attached_minion=a.minion==nil,no_summoning_parent=a.summonSkill==nil,
   exact_source=true,disabled=flags.disable==true,ordinary={},supported={},validation={},source_validation={},assembly={},source_catalog=source.gemData.id,
   final_root_level=plain(e.grantedEffect.levels[e.level]),final_stat_set_level=plain(set.levels[e.level]),final_row=plain(e.grantedEffectLevel)}
  for _,r in ipairs(auth.ordinary)do if r.effect==e then
   local joins={};for j,c in ipairs(r.list)do local ids={};for k,cp in ipairs(auth.copies)do for _,ins in ipairs(cp.insertions)do if ins.object==c.mod then ids[#ids+1]=k end end end;joins[j]=ids end
   row.ordinary[#row.ordinary+1]={before=r.before,after=r.after,candidates=r.candidates,matched=r.matched,rejected=r.rejected,
    query={cfg=r.query.cfg,chain=r.query.chain},query_store_is_actor=r.query.store==a.actor.modDB,copy_indices=joins}
  end end
  for _,r in ipairs(auth.supported)do if r.skill==a then assert(r.env==env and r.source==source)
   local supports={};for _,s in ipairs(r.supports)do supports[#supports+1]={index=s.index,effect=s.effect,prepared=s.prepared,admitted=s.admitted,is_supporting=s.is_supporting==true,source=assert(origins[s.source]),exact_source=true}end
   row.supported[#row.supported+1]={before=r.before,after=r.after,cache_hit=r.cache_hit,cache_identity_preserved=r.cache_identity_preserved,
    properties=r.rows,query={cfg=r.query.cfg,chain=r.query.chain},query_parent_is_actor=r.query.store.parent==a.actor.modDB,supports=supports}
   assert(not caches[r.result] or caches[r.result]==source,"different physical sources shared property cache");caches[r.result]=source
  end end
  for _,r in ipairs(auth.validation)do if r.effect==e then row.validation[#row.validation+1]={before=r.before,after=r.after,before_lookup=r.before_lookup,after_lookup=r.after_lookup}end end
  for _,r in ipairs(auth.validation)do if r.effect==source then row.source_validation[#row.source_validation+1]={before=r.before,after=r.after,before_lookup=r.before_lookup,after_lookup=r.after_lookup}end end
  for _,r in ipairs(auth.assemblies)do if r.skill==a then assert(r.env==env);row.assembly[#row.assembly+1]={before=r.before,after=r.after,merges=r.merges}end end
  row.distinct_source_cache=true;contexts[#contexts+1]=row
  outcomes[#outcomes+1]={mode=mode,source=origin,raw=row.raw,final=row.final,disabled=row.disabled,final_root_level=row.final_root_level,final_stat_set_level=row.final_stat_set_level,final_row=row.final_row}
 end end
end
local copies={};for i,r in ipairs(auth.copies)do local out={index=i,before=r.before,input_after=r.input_after,scale=r.scale,caller=r.caller,insertions={}}
 for _,s in ipairs(r.insertions)do out.insertions[#out.insertions+1]={record=s.record,same_as_input=s.same_as_input,inserted=true}end
 if r.amulet then local p=r.amulet;out.amulet={id=p.item.id,exact_registered=build.itemsTab.items[p.item.id]==p.item,original_record=p.original_record,
  copied_from_actual_list=p.copied_from_actual_list,exact_copy_argument=p.exact_copy_argument,exact_destination=p.exact_destination}end
 copies[#copies+1]=out
end
local equipped={};for _,mode in ipairs({"MAIN","CALCS"})do local env=mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv;equipped[mode]={}
 for _,slot in ipairs({"Helmet","Amulet"})do local item=env.player.itemList[slot];equipped[mode][slot]={present=item~=nil}
  if item then equipped[mode][slot]={present=true,id=item.id,name=item.name,type=item.type,exact_registered=build.itemsTab.items[item.id]==item,raw_lines=copyTable(item.rawLines)}end
 end
end
local def=assert(build.data.skills.PainOfferingPlayer)
return {methods=methods,saved=saved,contexts=contexts,outcomes=outcomes,copies=copies,equipped=equipped,
 definition={effect=def.id,levels=plain(def.levels),stat_set_levels=plain(def.statSets[1].levels),quality_stats=plain(def.statSets[1].qualityStats)},
 instrumented=offeringPropertyInstrumented,actual_original_calls_observed=offeringPropertyInstrumented,
 original_methods_preserved=true,hook_removed=true,jit_mode_preserved=true,business_wrappers=false,source_tables_mutated=false}
