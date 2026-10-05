-- Numerical observations of retained source objects, using unchanged source queries.
-- This runs after the complete loader/calculator and never changes source cfg or methods.
assert(djinnSupportAuth.finished and debug.gethook()==nil)
local supportIds={"SupportBiddingPlayerTwo","SupportBiddingPlayerThree"}
local sourceIds={};local definitions={}
local function scalar(v)
 assert(v==nil or type(v)=="number"or type(v)=="string"or type(v)=="boolean")
 if type(v)=="number"then assert(v==v and v~=math.huge and v~=-math.huge)end
 return v
end
local function scalars(v)
 local out,positions={},{};for k,x in pairs(v or{})do if type(x)~="table"and type(x)~="function"then
  if type(k)=="number"then positions[#positions+1]={index=k,value=scalar(x)}else assert(type(k)=="string");out[k]=scalar(x)end
 end end
 if #positions>0 then table.sort(positions,function(a,b)return a.index<b.index end);assert(out.numeric_entries==nil);out.numeric_entries=positions end
 return out
end
local function precise(v,depth)
 if type(v)~="table"then return scalar(v)end
 depth=(depth or 0)+1;assert(depth<=12)
 local out,positions,n={},{},0
 for k,x in pairs(v)do n=n+1;assert(n<=4096)
  if type(k)=="number"then positions[#positions+1]={index=k,value=precise(x,depth)}
  else assert(type(k)=="string");out[k]=precise(x,depth)end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 if #positions>0 then out.positions=positions end;return out
end
local function mod(m)
 local tags={};for _,tag in ipairs(m)do tags[#tags+1]=precise(tag)end
 return{name=m.name,type=m.type,value=precise(m.value),source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
end
-- One shallow-edge snapshot for each reachable object over this whole stage.
-- Shared modifier/definition graphs are visited once, not cloned for every child.
-- All fields and object identities are checked; no relevant-record filter weakens
-- mutation detection. Actor/parent store links are preserved as exact boundaries.
local snapshots,seenTables,snapshotEntries={},{},0
local function remember(value,boundary)
 if type(value)~="table"or seenTables[value]then return end
 local row={object=value,meta=getmetatable(value),entries={},count=0};seenTables[value]=row
 snapshots[#snapshots+1]=row;assert(#snapshots<=200000,"immutable snapshot table bound")
 for key,entry in pairs(value)do
  row.count=row.count+1;snapshotEntries=snapshotEntries+1;assert(snapshotEntries<=4000000,"immutable snapshot entry bound")
  row.entries[key]=entry
  if not boundary or(key~="actor"and key~="parent")then remember(entry)end
 end
end
local function watch(store)
 local depth,seen=0,{}
 while store do
  assert(depth<16 and not seen[store]);seen[store]=true;remember(store,true);store=store.parent;depth=depth+1
 end
end
local function verifySnapshots()
 for _,row in ipairs(snapshots)do
  assert(getmetatable(row.object)==row.meta);local count=0
  for key,value in pairs(row.object)do count=count+1;assert(row.entries[key]==value,"numerical observer mutated source object")end
  assert(count==row.count,"numerical observer changed source fields")
 end
end
local methods={};local references={}
for _,v in ipairs({
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"More","Classes/ModStore.lua",261},
 {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModDB,"AddMod","Classes/ModDB.lua",31},
 {common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
 {common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
})do
 local f=v[1][v[2]];local info=debug.getinfo(f,"S");local path=info.source:gsub("\\","/")
 assert(info.what=="Lua"and path:sub(-#v[3])==v[3]and info.linedefined==v[4])
 methods[#methods+1]={name=v[2],path=v[3],first=info.linedefined,last=info.lastlinedefined}
 references[#references+1]={owner=v[1],key=v[2],func=f}
end
for _,id in ipairs(supportIds)do
 local g=assert(build.data.skills[id]);assert(g.support and g.modSource and not sourceIds[g.modSource]);sourceIds[g.modSource]=id
 local sets={};for i,s in ipairs(g.statSets)do
  sets[#sets+1]={index=i,id=s.id,label=s.label,constants=precise(s.constantStats),stats=precise(s.stats),
   levels=precise(s.levels),quality_stats=precise(s.qualityStats),base_mods=precise(s.baseMods),scope=s.statDescriptionScope}
 end
 definitions[#definitions+1]={effect=id,mod_source=g.modSource,levels=precise(g.levels),stat_sets=sets,
  add_types=precise(g.addSkillTypes),exclude_types=precise(g.excludeSkillTypes),family=precise(g.gemFamily)}
end
local function localRecords(store,name)
 local list=store.mods and store.mods[name]or store;local out={}
 for i,m in ipairs(list or{})do if m.name==name and sourceIds[m.source]then
  out[#out+1]={position=i,source_effect=sourceIds[m.source],record=mod(m),object=m}
 end end;return out
end
local function publicRecords(records)
 local out={};for _,r in ipairs(records)do out[#out+1]={position=r.position,source_effect=r.source_effect,record=r.record,
  ancestor_depth=r.ancestor_depth,exact_base_skill_store=r.exact_base_skill_store}end;return out
end
local function parentRecords(active)
 -- CalcPerform gives each active skill a fresh list whose parent is the exact
 -- base list built by CalcActiveSkill. List() reads that ancestry; scanning only
 -- the fresh local list would omit the original nested MinionModifier objects.
 local store,depth,seen=active.skillModList,0,{};local records,chain={},{};local baseCount=0
 assert(active.baseSkillModList)
 while store do
  assert(depth<16 and not seen[store]);seen[store]=true
  local isBase=store==active.baseSkillModList;if isBase then baseCount=baseCount+1 end
  local localRows=localRecords(store,"MinionModifier")
  chain[#chain+1]={ancestor_depth=depth,exact_base_skill_store=isBase,
   kind=store.mods and "ModDB"or "ModList",bidding_records=#localRows}
  for _,r in ipairs(localRows)do
   r.ancestor_depth=depth;r.exact_base_skill_store=isBase;records[#records+1]=r
  end
  store=store.parent;depth=depth+1
 end
 assert(baseCount==1,"parent source base list must be an exact ancestor")
 return records,chain
end
local function origins(effect,group)
 local out={};for i,g in ipairs(group.gemList)do if effect.srcInstance==g then
  out[#out+1]={position=i,skill_id=g.skillId,raw_level=g.level,raw_quality=g.quality,enabled=g.enabled}
 end end;assert(#out==1);return out[1]
end
local function queries(active,parentOuter)
 local store,cfg=active.skillModList,active.skillCfg
 watch(store);remember(cfg);remember(active.output)
 local result={cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),store_conditions=scalars(store.conditions),
  output_available=active.output~=nil,output=scalars(active.output),
  commandable=not not store:Flag(cfg,"Condition:CommandableSkill"),
  cooldown=store:Sum("INC",cfg,"CooldownRecovery"),damage_factor=store:More(cfg,"Damage"),
  cooldown_records={},damage_records={},raw_bidding_records={},producer_joins={}}
 for _,v in ipairs({{"INC","CooldownRecovery","cooldown_records"},{"MORE","Damage","damage_records"}})do
  for _,r in ipairs(store:Tabulate(v[1],cfg,v[2]))do
   result[v[3]][#result[v[3]]+1]={value=scalar(r.value),source_effect=sourceIds[r.mod.source],record=mod(r.mod)}
  end
 end
 local ancestor,depth=store,0
 while ancestor do
  assert(depth<16)
  for _,name in ipairs({"Damage","CooldownRecovery"})do
   for _,r in ipairs(localRecords(ancestor,name))do
    result.raw_bidding_records[#result.raw_bidding_records+1]={ancestor_depth=depth,position=r.position,source_effect=r.source_effect,record=r.record}
    local matches={};for i,p in ipairs(parentOuter or{})do if p.object.value.mod==r.object then matches[#matches+1]=i end end
    result.producer_joins[#result.producer_joins+1]={raw_index=#result.raw_bidding_records,parent_outer_indices=matches,exact_inner_object=#matches>0}
   end
  end
  ancestor=ancestor.parent;depth=depth+1
 end
 result.original_cfg_unchanged=true;result.original_store_unchanged=true;return result
end
local contexts={}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 local prepared={};for _,r in ipairs(djinnSupportAuth.records)do if r.env==env then prepared[r.skill]=r end end
 for _,active in ipairs(env.player.activeSkillList)do
  local auth=prepared[active]
  if auth then
   assert(active.actor==env.player and auth.group==active.socketGroup)
   local group=active.socketGroup;local source=origins(active.activeEffect,group)
   local gi;for i,g in ipairs(build.skillsTab.socketGroupList)do if g==group then assert(not gi);gi=i end end;assert(gi)
   local outer,storeChain=parentRecords(active)
   local row={mode=mode,effect=active.activeEffect.grantedEffect.id,group_index=gi,group_source=group.source,
    group_enabled=group.enabled,source=source,actor_is_player=true,selected=env.player.mainSkill==active,
    candidates={},parent_mod_store_chain=storeChain,parent_minion_modifiers=publicRecords(outer),player_queries=queries(active,{}),children={}}
   for i,s in ipairs(active.supportList)do if sourceIds[s.grantedEffect.modSource]then
    local accepted=false;for _,e in ipairs(active.effectList)do if e==s then assert(not accepted);accepted=true end end
    row.candidates[#row.candidates+1]={retained_position=i,effect=s.grantedEffect.id,accepted=accepted,
     origin=origins(s,group),level=s.level,quality=s.quality,exact_definition=build.data.skills[s.grantedEffect.id]==s.grantedEffect}
   end end
   if active.minion then
    local actor=active.minion;assert(actor.parent==env.player)
    row.minion={type=actor.type,selected=env.minion==actor,mod_db_available=actor.modDB~=nil,output_available=actor.output~=nil}
    for i,child in ipairs(actor.activeSkillList)do
     assert(child.actor==actor and child.summonSkill==active and child.supportList==active.supportList and prepared[child])
     local set=mode=="MAIN"and child.activeEffect.statSet or child.activeEffect.statSetCalcs
     assert(child.activeEffect.grantedEffect.statSets[set.index]==set.statSet)
     row.children[#row.children+1]={index=i,effect=child.activeEffect.grantedEffect.id,selected=actor.mainSkill==child,
      stat_set_index=set.index,stat_set_id=set.statSet.id,stat_set_label=set.statSet.label,
      exact_actor=true,exact_summoner=true,shared_support_list=true,queries=queries(child,outer)}
    end
   end
   contexts[#contexts+1]=row
  end
 end
end
verifySnapshots()
for _,r in ipairs(references)do assert(r.owner[r.key]==r.func)end
return{definitions=definitions,methods=methods,contexts=contexts,original_methods_preserved=true,
 immutable_snapshot={tables=#snapshots,entries=snapshotEntries,verified=true},
 source_cfg_modified=false,source_tables_mutated=false,business_wrappers=false,jit_mode_preserved=jit.status()==djinnJit}
