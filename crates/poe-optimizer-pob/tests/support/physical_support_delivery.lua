-- Reusable optional source collector; the profile selects a finite physical
-- recipient/support/stat domain. Rapid's historical named observer remains
-- unchanged for reproducibility of the already-published receipt.
return function(profile)
assert(type(profile)=="table" and getmetatable(profile)==nil)
assert(type(profile.effect)=="string" and type(profile.supports)=="table" and #profile.supports>0 and #profile.supports<=8)
assert(type(profile.stat_names)=="table" and #profile.stat_names<=16)
-- Optional finite source evidence. Original-method queries below are diagnostic
-- reads of retained stores, not intercepted original calculation calls.
assert(djinnSupportAuth.finished and debug.gethook()==nil)
local ids=profile.supports
local channels={{"Speed","INC"},{"SupportManaMultiplier","MORE"},{"ReservationMultiplier","MORE"},{"ExtraSpirit","BASE"}}
local sourceIds,definitions={},{ }
local work=0
local function charge()work=work+1;assert(work<=4000000,"physical support witness work bound")end
local function scalar(v)
 assert(v==nil or type(v)=="number"or type(v)=="string"or type(v)=="boolean")
 if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then
  return{source_number="non_finite",diagnostic=tostring(v)}
 end
 return v
end
local function scalars(t)
 local out,positions={},{};for k,v in pairs(t or{})do charge();if type(v)~="table"and type(v)~="function"then
  if type(k)=="number"then positions[#positions+1]={index=k,value=scalar(v)}
  else assert(type(k)=="string");out[k]=scalar(v)end
 end end
 if #positions>0 then table.sort(positions,function(a,b)return a.index<b.index end);assert(out.numeric_entries==nil);out.numeric_entries=positions end
 return out
end
local function precise(v,depth)
 charge();if type(v)~="table"then return scalar(v)end
 depth=(depth or 0)+1;assert(depth<=12)
 local out,positions,n={},{},0
 for k,x in pairs(v)do n=n+1;assert(n<=4096)
  if type(k)=="number"then positions[#positions+1]={index=k,value=precise(x,depth)}
  else assert(type(k)=="string");out[k]=precise(x,depth)end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 if #positions>0 then assert(out.positions==nil);out.positions=positions end;return out
end
local function mod(m)
 local tags={};for _,tag in ipairs(m)do tags[#tags+1]=precise(tag)end
 return{name=m.name,type=m.type,value=precise(m.value),source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
end
local snapshots,seenTables,entries={},{},0
local function remember(value,boundary)
 if type(value)~="table"or seenTables[value]then return end
 local row={object=value,meta=getmetatable(value),entries={},count=0};seenTables[value]=row
 snapshots[#snapshots+1]=row;assert(#snapshots<=200000)
 for key,entry in pairs(value)do
  row.count=row.count+1;entries=entries+1;assert(entries<=4000000)
  row.entries[key]=entry
  if not boundary or(key~="actor"and key~="parent")then remember(entry)end
 end
end
local function watch(store)
 local seen,depth={},0
 while store do assert(type(store)=="table"and depth<16 and not seen[store]);seen[store]=true
  remember(store,true);store=store.parent;depth=depth+1
 end
end
local methods,references={},{ }
for _,v in ipairs({
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModStore,"More","Classes/ModStore.lua",261},
 {common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
 {common.classes.ModList,"MoreInternal","Classes/ModList.lua",164},
 {common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
})do
 local f=v[1][v[2]];local info=debug.getinfo(f,"S");local path=info.source:gsub("\\","/")
 assert(info.what=="Lua"and path:sub(-#v[3])==v[3]and info.linedefined==v[4])
 methods[#methods+1]={name=v[2],path=v[3],first=info.linedefined,last=info.lastlinedefined}
 references[#references+1]={owner=v[1],key=v[2],func=f}
end
for _,id in ipairs(ids)do
 local g=assert(build.data.skills[id]);assert(g.support and g.modSource and not sourceIds[g.modSource]);sourceIds[g.modSource]=id
 remember(g)
 local sets={};for i,s in ipairs(g.statSets)do
  local maps={};for _,name in ipairs(profile.stat_names)do
   assert(type(name)=="string");local map=rawget(s.statMap,name)
   maps[#maps+1]={stat=name,present=map~=nil,value=precise(map)}
  end
  sets[#sets+1]={index=i,id=s.id,constants=precise(s.constantStats),stats=precise(s.stats),
   levels=precise(s.levels),quality_stats=precise(s.qualityStats),base_mods=precise(s.baseMods),
   declared_maps=maps}
 end
 definitions[#definitions+1]={effect=id,mod_source=g.modSource,levels=precise(g.levels),stat_sets=sets,
  family=precise(g.gemFamily),require_types=precise(g.requireSkillTypes),exclude_types=precise(g.excludeSkillTypes),
  add_types=precise(g.addSkillTypes),add_flags=precise(g.addFlags)}
end
local globalMaps={}
local costPrecision=assert(build.data.highPrecisionMods.SupportManaMultiplier)
remember(costPrecision);assert(costPrecision.MORE==4)
for _,name in ipairs(profile.stat_names)do
 local map=rawget(build.data.skillStatMap,name);remember(map)
 globalMaps[#globalMaps+1]={stat=name,present=map~=nil,value=precise(map)}
end
-- Bind exact saved physical Gem occurrences, including retained family winners.
local doc,err=common.xml.ParseXML(djinnXml);assert(doc and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)if type(n)~="table"or not n.elem then return end
 ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,c in ipairs(n)do enumerate(c)end
end
enumerate(doc[1])
local savedSets={}
for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Skills"then
 for _,set in ipairs(n)do if type(set)=="table"and set.elem=="SkillSet"then savedSets[tonumber(set.attrib.id)]=set end end
end end
local function groupOrigin(group)
 local matches={}
 for sid,set in pairs(build.skillsTab.skillSets)do for gi,g in ipairs(set.socketGroupList)do if g==group then
  assert(g.source==nil,"finite physical support witness expects a physical recipient")
  local saved={}
  for _,r in ipairs(assert(savedSets[sid]))do if type(r)=="table"and r.elem=="Skill"and r.attrib.source==nil then
   local gems={};for _,x in ipairs(r)do if type(x)=="table"and x.elem=="Gem"then gems[#gems+1]=x end end
   if gems[1]and gems[1].attrib.skillId==g.gemList[1].skillId then saved[#saved+1]={group=r,gems=gems}end
  end end
  assert(#saved==1,"ambiguous saved physical recipient group");assert(#saved[1].gems==#g.gemList)
  matches[#matches+1]={preset=sid,index=gi,source_present=false,source_ordinal=ordinals[saved[1].group],saved=saved[1]}
 end end end
 assert(#matches==1);return matches[1]
end
local function origin(effect,group,binding)
 local found={};for i,g in ipairs(group.gemList)do if effect.srcInstance==g then
  local raw=assert(binding.saved.gems[i]);assert(raw.attrib.skillId==g.skillId)
  found[#found+1]={position=i,skill_id=g.skillId,raw_level=g.level,raw_quality=g.quality,enabled=g.enabled,
   source_ordinal=ordinals[raw],saved_attributes=precise(raw.attrib),exact_source_instance=true}
 end end;assert(#found==1);return found[1]
end
local function candidates(active,group,binding)
 local out={};for i,s in ipairs(active.supportList)do if sourceIds[s.grantedEffect.modSource]then
  local accepted=false;for _,e in ipairs(active.effectList)do if e==s then assert(not accepted);accepted=true end end
  out[#out+1]={retained_position=i,effect=s.grantedEffect.id,accepted=accepted,origin=origin(s,group,binding),
   level=s.level,quality=s.quality,exact_definition=build.data.skills[s.grantedEffect.id]==s.grantedEffect}
 end end;return out
end
local function queries(active,mode)
 local store,cfg=active.skillModList,active.skillCfg
 assert(cfg.skillGrantedEffect==active.activeEffect.grantedEffect)
 assert(store.Sum==common.classes.ModStore.Sum and store.Tabulate==common.classes.ModStore.Tabulate and store.More==common.classes.ModStore.More)
 local selected=active.actor.mainSkill==active
 local output=selected and active.actor.output or nil
 local set=mode=="CALCS"and active.activeEffect.statSetCalcs or active.activeEffect.statSet
 local flags=assert(set.skillFlags);assert(active.activeEffect.grantedEffect.statSets[set.index]==set.statSet)
 watch(store);remember(cfg);remember(output);remember(active.skillData);remember(flags)
 local result={cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),skill_types=precise(active.skillTypes),
  skill_flags=scalars(flags),output_available=output~=nil,output=scalars(output),output_is_selected_actor=selected,
  definition_base_flags=scalars(set.statSet.baseFlags),exact_stat_set=true,original_query_methods=true,
  skill_data=scalars(active.skillData),cfg_effect_exact=true,cast_flag_value=ModFlag.Cast,
  cast_flag=AND64(cfg.flags,ModFlag.Cast)~=0,speed_increase=store:Sum("INC",cfg,"Speed"),cost_factor=store:More(cfg,"SupportManaMultiplier"),channels={},store_chain={},
  query_observation_kind="diagnostic_original_method_read",original_calculation_call_captured=false}
 local ancestor,depth,seen=store,0,{}
 local sourceRows={};for _,c in ipairs(channels)do sourceRows[c[1]]={}end
 while ancestor do
  assert(type(ancestor)=="table"and depth<16 and not seen[ancestor]);seen[ancestor]=true
  result.store_chain[#result.store_chain+1]={depth=depth,base_skill_store=ancestor==active.baseSkillModList,
   actor_store=ancestor==active.actor.modDB,kind=ancestor.mods and"ModDB"or"ModList"}
  for _,c in ipairs(channels)do
   local list=ancestor.mods and ancestor.mods[c[1]]or ancestor
   for _,m in ipairs(list or{})do if m.name==c[1]and sourceIds[m.source]then
    local r=sourceRows[c[1]];r[#r+1]={ancestor_depth=depth,source_effect=sourceIds[m.source],record=mod(m),object=m}
   end end
  end
  ancestor=ancestor.parent;depth=depth+1
 end
 for _,c in ipairs(channels)do
  local raw={};for i,r in ipairs(sourceRows[c[1]])do raw[i]={channel_index=i,ancestor_depth=r.ancestor_depth,source_effect=r.source_effect,record=r.record}end
  local applied={};for _,r in ipairs(store:Tabulate(c[2],cfg,c[1]))do
   local joins={};for i,p in ipairs(sourceRows[c[1]])do if p.object==r.mod then joins[#joins+1]=i end end
   applied[#applied+1]={value=scalar(r.value),source_effect=sourceIds[r.mod.source],record=mod(r.mod),source_record_indices=joins}
  end
  result.channels[c[1]]={raw_source_records=raw,applied=applied}
 end
 return result
end
local contexts={}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 for _,active in ipairs(env.player.activeSkillList)do if active.activeEffect.grantedEffect.id==profile.effect then
  assert(active.actor==env.player and active.minion==nil)
  local group=assert(active.socketGroup);local binding=groupOrigin(group)
  local set=mode=="MAIN"and active.activeEffect.statSet or active.activeEffect.statSetCalcs
  contexts[#contexts+1]={mode=mode,effect=active.activeEffect.grantedEffect.id,group={preset=binding.preset,index=binding.index,
   source_present=false,source_ordinal=binding.source_ordinal},source=origin(active.activeEffect,group,binding),
   actor_is_player=true,selected=env.player.mainSkill==active,candidates=candidates(active,group,binding),
   queries=queries(active,mode),stat_set_index=set.index,stat_set_id=set.statSet.id}
 end end
end
for _,row in ipairs(snapshots)do
 assert(getmetatable(row.object)==row.meta);local count=0
 for key,value in pairs(row.object)do count=count+1;assert(row.entries[key]==value,"physical support observer mutation")end
 assert(count==row.count)
end
for _,r in ipairs(references)do assert(r.owner[r.key]==r.func)end
return{definitions=definitions,global_stat_maps=globalMaps,methods=methods,contexts=contexts,original_methods_preserved=true,
 source_cost_precision={stat="SupportManaMultiplier",type="MORE",digits=costPrecision.MORE},
 immutable_snapshot={tables=#snapshots,entries=entries,verified=true},source_cfg_modified=false,
 source_tables_mutated=false,business_wrappers=false,jit_mode_preserved=jit.status()==djinnJit,
 observation_order="per-channel original record order; no cross-channel arithmetic is combined",
 query_observation_kind="diagnostic_original_method_read",original_calculation_calls_captured=false}

end
