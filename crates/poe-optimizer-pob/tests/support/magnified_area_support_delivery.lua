-- Optional reference evidence: query unchanged stores with their exact original cfg.
-- This observer does not implement area/cost formulas or change admission.
assert(djinnSupportAuth.finished and debug.gethook()==nil)
local ids={"SupportMagnifiedAreaPlayer","SupportMagnifiedAreaPlayerTwo"}
local channels={{"AreaOfEffect","INC"},{"SupportManaMultiplier","MORE"},{"Damage","MORE"}}
local sourceIds,definitions={},{}
local work=0
local function charge()work=work+1;assert(work<=4000000,"Magnified witness work bound")end
local function scalar(v)
 assert(v==nil or type(v)=="number"or type(v)=="string"or type(v)=="boolean")
 if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then
  -- Preserve source diagnostics rather than coercing infinities/NaN to a
  -- numeric default. Reviewed area/cost contributions must still be finite.
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
local methods,references={},{}
for _,v in ipairs({
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"More","Classes/ModStore.lua",261},
 {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
 {common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
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
  local maps={};for _,name in ipairs({"base_skill_area_of_effect_+%","support_increased_area_damage_+%_final"})do
   local entry=rawget(s.statMap,name);maps[#maps+1]={stat=name,present=entry~=nil,entry=precise(entry)}
  end
  sets[#sets+1]={index=i,id=s.id,constants=precise(s.constantStats),stats=precise(s.stats),
   levels=precise(s.levels),quality_stats=precise(s.qualityStats),base_mods=precise(s.baseMods),raw_maps=maps}
 end
 definitions[#definitions+1]={effect=id,mod_source=g.modSource,levels=precise(g.levels),stat_sets=sets,family=precise(g.gemFamily)}
end
-- Exact saved occurrence bindings; labels are not identity. Generated groups
-- keep source identity but do not receive a fabricated saved ordinal.
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
  local saved={}
  for _,r in ipairs(assert(savedSets[sid]))do if type(r)=="table"and r.elem=="Skill"and r.attrib.source==g.source then
   local gems={};for _,x in ipairs(r)do if type(x)=="table"and x.elem=="Gem"then gems[#gems+1]=x end end
   if gems[1]and gems[1].attrib.skillId==g.gemList[1].skillId then saved[#saved+1]={group=r,gems=gems}end
  end end
  assert(#saved<=1,"ambiguous saved Magnified source group")
  if not g.source then assert(#saved==1)end
  if saved[1]then assert(#saved[1].gems==#g.gemList)end
  matches[#matches+1]={preset=sid,index=gi,source=g.source,source_present=g.source~=nil,
   source_ordinal=saved[1]and ordinals[saved[1].group],saved=saved[1]}
 end end end
 assert(#matches==1);return matches[1]
end
local function origin(effect,group,binding)
 local found={};for i,g in ipairs(group.gemList)do if effect.srcInstance==g then
  local raw=binding.saved and binding.saved.gems[i]
  if raw then assert(raw.attrib.skillId==g.skillId)end
  found[#found+1]={position=i,skill_id=g.skillId,raw_level=g.level,raw_quality=g.quality,enabled=g.enabled,
   source_ordinal=raw and ordinals[raw],saved_attributes=raw and precise(raw.attrib)}
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
 assert(store.Sum==common.classes.ModStore.Sum and store.More==common.classes.ModStore.More
  and store.Flag==common.classes.ModStore.Flag and store.Tabulate==common.classes.ModStore.Tabulate)
 local selected=active.actor.mainSkill==active
 local output=selected and active.actor.output or nil
 local set=mode=="CALCS"and active.activeEffect.statSetCalcs or active.activeEffect.statSet
 local flags=assert(set.skillFlags)
 assert(active.activeEffect.grantedEffect.statSets[set.index]==set.statSet)
 watch(store);remember(cfg);remember(output);remember(active.skillData);remember(flags)
 local result={cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),skill_types=precise(active.skillTypes),
  skill_flags=scalars(flags),output_available=output~=nil,output=scalars(output),output_is_selected_actor=selected,
  definition_base_flags=scalars(set.statSet.baseFlags),exact_stat_set=true,original_query_methods=true,
  skill_data=scalars(active.skillData),cfg_effect_exact=true,area_flag_value=ModFlag.Area,
  area_flag=AND64(cfg.flags,ModFlag.Area)~=0,has_no_cost=store:Flag(cfg,"HasNoCost"),
  area_increase=store:Sum("INC",cfg,"AreaOfEffect"),cost_factor=store:More(cfg,"SupportManaMultiplier"),
  damage_factor=store:More(cfg,"Damage"),channels={},store_chain={}}
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
 for _,active in ipairs(env.player.activeSkillList)do
  local auth=djinnSupportAuth.by_skill[active]
  if auth or active.activeEffect.grantedEffect.id=="IceNovaPlayer"then
   assert(active.actor==env.player)
   if auth then assert(auth.env==env and auth.group==active.socketGroup)end
   local group=assert(active.socketGroup);local binding=groupOrigin(group)
   local row={mode=mode,effect=active.activeEffect.grantedEffect.id,group={preset=binding.preset,index=binding.index,
    source=binding.source,source_present=binding.source_present,source_ordinal=binding.source_ordinal},
    source=origin(active.activeEffect,group,binding),actor_is_player=true,selected=env.player.mainSkill==active,
    constructor_observed=auth~=nil,candidates=candidates(active,group,binding),queries=queries(active,mode),children={}}
   local set=mode=="MAIN"and active.activeEffect.statSet or active.activeEffect.statSetCalcs
   row.stat_set_index=set.index;row.stat_set_id=set.statSet.id
   if active.minion then
    local actor=active.minion;assert(actor.parent==env.player)
    row.minion={type=actor.type,selected=env.minion==actor,mod_db_available=actor.modDB~=nil}
    for i,child in ipairs(actor.activeSkillList)do
     assert(child.actor==actor and child.summonSkill==active and child.supportList==active.supportList and djinnSupportAuth.by_skill[child])
     local childSet=mode=="MAIN"and child.activeEffect.statSet or child.activeEffect.statSetCalcs
     assert(child.activeEffect.grantedEffect.statSets[childSet.index]==childSet.statSet)
     row.children[#row.children+1]={index=i,effect=child.activeEffect.grantedEffect.id,selected=actor.mainSkill==child,
      stat_set_index=childSet.index,stat_set_id=childSet.statSet.id,exact_actor=true,exact_summoner=true,
      shared_support_list=true,candidates=candidates(child,group,binding),queries=queries(child,mode)}
    end
   end
   contexts[#contexts+1]=row
  end
 end
end
for _,row in ipairs(snapshots)do
 assert(getmetatable(row.object)==row.meta);local count=0
 for key,value in pairs(row.object)do count=count+1;assert(row.entries[key]==value,"Magnified observer mutation")end
 assert(count==row.count)
end
for _,r in ipairs(references)do assert(r.owner[r.key]==r.func)end
return{definitions=definitions,methods=methods,contexts=contexts,original_methods_preserved=true,
 immutable_snapshot={tables=#snapshots,entries=entries,verified=true},source_cfg_modified=false,
 source_tables_mutated=false,business_wrappers=false,jit_mode_preserved=jit.status()==djinnJit,
 observation_order="per-channel original record order; no cross-channel arithmetic is combined"}
