-- Finite profile plus one additional channel over the immutable physical-source
-- collector. These are diagnostic original-method reads, not gameplay wrappers.
local ids={"ProlongedDurationSupportPlayer","ProlongedDurationSupportPlayerTwo"}
local stat="support_more_duration_skill_effect_duration_+%_final"
local work=0
local function charge()work=work+1;assert(work<=4000000,"duration observer work bound")end
local function precise(v,depth)
 charge();if type(v)~="table"then assert(v==nil or type(v)=="number"or type(v)=="string"or type(v)=="boolean");return v end
 depth=(depth or 0)+1;assert(depth<=12)
 local out,positions,n={},{},0
 for k,x in pairs(v)do n=n+1;assert(n<=4096)
  if type(k)=="number"then positions[#positions+1]={index=k,value=precise(x,depth)}
  else assert(type(k)=="string");out[k]=precise(x,depth)end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 if #positions>0 then assert(out.positions==nil);out.positions=positions end
 return out
end
local function mod(m)
 local tags={};for _,tag in ipairs(m)do tags[#tags+1]=precise(tag)end
 return{name=m.name,type=m.type,value=precise(m.value),source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
end
local function equal(a,b,depth)
 charge();if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 depth=(depth or 0)+1;assert(depth<=16)
 local n,m=0,0;for k,v in pairs(a)do n=n+1;if not equal(v,b[k],depth)then return false end end
 for _ in pairs(b)do m=m+1 end;return n==m
end
local sourceIds={};for _,id in ipairs(ids)do sourceIds[assert(build.data.skills[id]).modSource]=id end
local function supplement(context)
 local env=assert(context.mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 local group=assert(build.skillsTab.skillSets[context.group.preset].socketGroupList[context.group.index])
 local physical=assert(group.gemList[context.source.position]);assert(physical.skillId==context.effect)
 local active
 for _,candidate in ipairs(env.player.activeSkillList)do
  if candidate.socketGroup==group and candidate.activeEffect.srcInstance==physical and candidate.activeEffect.grantedEffect.id==context.effect then
   assert(not active,"ambiguous exact physical action");active=candidate
  end
 end
 assert(active and active.actor==env.player and active.minion==nil and active.summonSkill==nil)
 local gem=assert(active.activeEffect.gemData)
 assert(gem.grantedEffect==active.activeEffect.grantedEffect)
 local granted={};for index,effect in ipairs(gem.grantedEffectList)do
  granted[#granted+1]={index=index,effect=effect.id,primary=effect==gem.grantedEffect,support=effect.support==true}
 end
 local store,cfg=active.skillModList,active.skillCfg
 assert(cfg.skillGrantedEffect==active.activeEffect.grantedEffect)
 local methods={Sum=store.Sum,More=store.More,Tabulate=store.Tabulate}
 for name,f in pairs(methods)do assert(f==common.classes.ModStore[name])end
 -- Snapshot the exact stores and all nested record/cfg values touched by the
 -- added queries. Parent traversal is explicit; actor back-links are boundaries.
 local snapshots,seen,entries={},{},0
 local function remember(value,boundary)
  if type(value)~="table"or seen[value]then return end
  local row={object=value,meta=getmetatable(value),entries={},count=0};seen[value]=true
  snapshots[#snapshots+1]=row;assert(#snapshots<=200000)
  for k,v in pairs(value)do
   row.count=row.count+1;entries=entries+1;assert(entries<=4000000);row.entries[k]=v
   if not boundary or(k~="actor"and k~="parent")then remember(v)end
  end
 end
 local ancestor,depth,visited=store,0,{}
 local sources,raw={},{ }
 while ancestor do
  assert(type(ancestor)=="table"and depth<16 and not visited[ancestor]);visited[ancestor]=true
  remember(ancestor,true)
  for _,m in ipairs(ancestor.mods and ancestor.mods.Duration or ancestor)do
   if m.name=="Duration"and sourceIds[m.source]then
    sources[#sources+1]=m
    raw[#raw+1]={channel_index=#raw+1,ancestor_depth=depth,source_effect=sourceIds[m.source],record=mod(m)}
   end
  end
  ancestor=ancestor.parent;depth=depth+1
 end
 remember(cfg);remember(active.skillData);remember(active.actor.output)
 local applied={}
 for _,r in ipairs(store:Tabulate("MORE",cfg,"Duration"))do
  local joins={};for index,m in ipairs(sources)do if m==r.mod then joins[#joins+1]=index end end
  applied[#applied+1]={value=r.value,source_effect=sourceIds[r.mod.source],record=mod(r.mod),source_record_indices=joins}
 end
 local q=context.queries
 q.channels.Duration={raw_source_records=raw,applied=applied}
 q.duration={increased=store:Sum("INC",cfg,"Duration"),more=store:More(cfg,"Duration"),
  primary_base=active.skillData.duration,secondary_base=active.skillData.durationSecondary,
  effective_level=active.activeEffect.level,effective_quality=active.activeEffect.quality,
  physical_id=gem.id,variant_id=gem.variantId,primary_definition_exact=true,granted_effects=granted,
  no_attached_minion=active.minion==nil,no_summoning_parent=active.summonSkill==nil,
  exact_physical_instance=true,exact_actor=true,has_reservation=active.skillTypes[SkillType.HasReservation]==true,
  query_observation_kind="diagnostic_original_method_read",original_calculation_calls_captured=false}
 for _,row in ipairs(snapshots)do
  assert(getmetatable(row.object)==row.meta);local count=0
  for k,v in pairs(row.object)do count=count+1;assert(row.entries[k]==v,"duration observer mutation")end
  assert(count==row.count)
 end
 for name,f in pairs(methods)do assert(store[name]==f and common.classes.ModStore[name]==f)end
 q.duration_snapshot={tables=#snapshots,entries=entries,verified=true,original_methods_preserved=true}
end
local results={}
for _,effect in ipairs({"PainOfferingPlayer","IceNovaPlayer"})do
 results[#results+1]=collect({effect=effect,supports=ids,stat_names={stat}})
end
local result=results[1]
result.profile_snapshots={result.immutable_snapshot,results[2].immutable_snapshot}
for _,field in ipairs({"definitions","global_stat_maps","methods","source_cost_precision"})do assert(equal(result[field],results[2][field]))end
for _,c in ipairs(results[2].contexts)do result.contexts[#result.contexts+1]=c end
for _,c in ipairs(result.contexts)do supplement(c)end
for _,r in ipairs(results)do
 assert(r.original_methods_preserved and r.jit_mode_preserved and r.immutable_snapshot.verified)
 assert(not r.source_cfg_modified and not r.source_tables_mutated and not r.business_wrappers)
end
result.profile_domain={"PainOfferingPlayer","IceNovaPlayer"}
result.recipient_definitions={}
for _,id in ipairs(result.profile_domain)do
 local effect=assert(build.data.skills[id]);local sets={}
 for index,set in ipairs(effect.statSets)do
  sets[#sets+1]={index=index,label=set.label,constants=precise(set.constantStats),stats=precise(set.stats),
   base_flags=precise(set.baseFlags)}
 end
 result.recipient_definitions[#result.recipient_definitions+1]={effect=id,support=effect.support==true,
  cast_time=effect.castTime,skill_types=precise(effect.skillTypes),minion_skill_types=precise(effect.minionSkillTypes),
  parts_present=effect.parts~=nil,parts=precise(effect.parts),minion_list_present=effect.minionList~=nil,
  minion_list=precise(effect.minionList),stat_sets=sets}
end
return result
