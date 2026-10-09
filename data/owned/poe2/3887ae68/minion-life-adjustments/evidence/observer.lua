-- Observe original functions and caller locals; never replace a business method.
local calcs = require("Modules.CalcBase")
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line,path..":"..line);return f
end
local function upvalue(f,key)
 for i=1,128 do local name,value=debug.getupvalue(f,i);if not name then break end;if name==key then return value end end
 error("missing original upvalue "..key)
end
local function scalar(v)
 if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return tostring(v) end;return v
end
local function scalars(t)
 local out={};for k,v in pairs(t or {}) do if type(v)=="number" or type(v)=="string" or type(v)=="boolean" then out[k]=scalar(v) end end;return out
end
local function clone(v,seen)
 if type(v)~="table" then return v end;seen=seen or {};if seen[v] then return seen[v] end
 local out={};seen[v]=out;for k,e in pairs(v) do out[k]=clone(e,seen) end;return out
end
local function equal(a,b,seen)
 if type(a)~=type(b) then return false end;if type(a)~="table" then return a==b end
 seen=seen or {};if seen[a] then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a) do if not equal(v,b[k],seen) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function keys(t) local out={};for k in pairs(t) do out[k]=true end;return out end
local function setRows(t)
 local ids={};for id in pairs(t or {}) do ids[#ids+1]=id end;table.sort(ids)
 local out={};for _,id in ipairs(ids) do out[#out+1]={id=id,value=t[id]} end;return out
end
local function watchStores(stores)
 local seen,saved={},{}
 local function add(store)
  if not store or seen[store] then return end;seen[store]=true
  local owned={};for k,v in pairs(store) do if k~="actor" and k~="parent" then owned[k]=v end end
  saved[#saved+1]={store=store,actor=store.actor,parent=store.parent,meta=getmetatable(store),owned=clone(owned),output=store.actor and clone(store.actor.output)};add(store.parent)
 end
 for _,store in ipairs(stores) do add(store) end
 return function()
  for _,row in ipairs(saved) do
   assert(row.store.actor==row.actor and row.store.parent==row.parent and getmetatable(row.store)==row.meta)
   local owned={};for k,v in pairs(row.store) do if k~="actor" and k~="parent" then owned[k]=v end end
   assert(equal(owned,row.owned),"observer changed modifier store");assert(equal(row.actor and row.actor.output,row.output),"observer changed actor output")
  end
 end
end
local function modRecord(mod)
 local tags={};for _,tag in ipairs(mod) do tags[#tags+1]=clone(tag) end
 -- The optional receiving witness retains mixed-table payloads losslessly.
 -- Keep the earlier report's wire projection unchanged for its existing users.
 local function precise(value,depth)
  if type(value)~="table" then return scalar(value) end
  depth=(depth or 0)+1;assert(depth<16)
  local out,positions,count={},{},0
  for key,entry in pairs(value) do
   count=count+1;assert(count<=4096)
   if type(key)=="number" then positions[#positions+1]={index=key,value=precise(entry,depth)}
   else assert(type(key)=="string");out[key]=precise(entry,depth) end
  end
  table.sort(positions,function(a,b)return a.index<b.index end)
  if #positions>0 then out._positions=positions end
  return out
 end
 local value=type(mod.value)=="table" and (physicalDamageCommandEvidence and precise(mod.value) or clone(mod.value)) or scalar(mod.value)
 return {name=mod.name,type=mod.type,value=value,source=mod.source,flags=mod.flags,keyword_flags=mod.keywordFlags,tags=tags}
end
local function sourceOccurrence(active)
 local source=active and active.activeEffect and active.activeEffect.srcInstance
 if not source then return {source_present=false,matches={}} end
 local matches={}
 for groupIndex,group in ipairs(build.skillsTab.socketGroupList) do
  for gemIndex,gem in ipairs(group.gemList) do
   if gem==source then
    matches[#matches+1]={group_index=groupIndex,gem_index=gemIndex,group_is_active_socket_group=group==active.socketGroup,
     group_enabled=group.enabled,group_label=group.label,source_enabled=gem.enabled,raw_level=gem.level,raw_quality=gem.quality,
     enable_global_1=gem.enableGlobal1,enable_global_2=gem.enableGlobal2,gem_id=gem.gemId,
     effect_id=gem.grantedEffect and gem.grantedEffect.id,physical_gem_id=gem.gemData and gem.gemData.gameId}
   end
  end
 end
 return {source_present=true,matches=matches}
end
local function recipientOccurrence(env,actor)
 local matches={}
 for ordinal,active in ipairs(env.player.activeSkillList) do
  if active.minion==actor then matches[#matches+1]={ordinal=ordinal,effect_id=active.activeEffect.grantedEffect.id,source=sourceOccurrence(active)} end
 end
 return {matches=matches}
end
local function offeringSupports(active)
 local out={}
 for _,support in ipairs(active.supportList or {}) do
  local admitted=false;for _,effect in ipairs(active.effectList or {}) do if effect==support then admitted=true end end
  out[#out+1]={effect_id=support.grantedEffect.id,level=support.level,quality=support.quality,
   modifier_source=support.grantedEffect.modSource,admitted_by_original_effect_list=admitted,
   supports_exact_source=not not (support.isSupporting and support.isSupporting[active.activeEffect.srcInstance]),
   source_occurrence=sourceOccurrence({activeEffect=support,socketGroup=active.socketGroup})}
 end
 return out
end
local function sourceConfig(active,cfg)
 if not cfg then return {present=false} end
 -- Config contains definition graph references and sparse numeric skill-type
 -- sets. Preserve explicit filter inputs and prove the references by identity;
 -- never serialize a deep copy of the source graph or erase numeric keys.
 local tables={skillGem=true,skillGrantedEffect=true,skillTypes=true,skillCond=true}
 for key,value in pairs(cfg) do
  assert(type(key)=="string")
  assert(type(value)~="table" or tables[key],"unreviewed source config table "..key)
 end
 assert(cfg.skillGrantedEffect==active.activeEffect.grantedEffect)
 assert(cfg.skillGem==active.activeEffect.gemData)
 return {present=true,fields=scalars(cfg),skill_conditions=scalars(cfg.skillCond),skill_types=setRows(cfg.skillTypes),
  skill_gem=scalars(cfg.skillGem),skill_gem_tags=scalars(cfg.skillGem and cfg.skillGem.tags),
  granted_effect={id=cfg.skillGrantedEffect.id,name=cfg.skillGrantedEffect.name},
  skill_gem_is_source=true,granted_effect_is_source=true}
end
local function records(store,kind,cfg,...)
 local out={};for _,entry in ipairs(store:Tabulate(kind,cfg,...)) do out[#out+1]={value=scalar(entry.value),mod=modRecord(entry.mod)} end;return out
end
local function rawRecords(store,wanted)
 local out,depth={},0
 while store do
  local names={};for name in pairs(store.mods or {}) do names[#names+1]=name end;table.sort(names)
  local function add(mod)
   if wanted[mod.name] then out[#out+1]={ancestor_depth=depth,mod=modRecord(mod)} end
  end
  for _,name in ipairs(names) do for _,mod in ipairs(store.mods[name]) do add(mod) end end
  for _,mod in ipairs(store) do add(mod) end
  store=store.parent;depth=depth+1
 end
 return out
end
local damageTypes={"Physical","Lightning","Cold","Fire","Chaos"}
local elemental={Lightning=true,Cold=true,Fire=true}
local function scalarChannel(store,kind,cfg,names)
 return {names=names,records=records(store,kind,cfg,unpack(names)),value=kind=="MORE" and store:More(cfg,unpack(names)) or store:Sum(kind,cfg,unpack(names))}
end
-- Optional census of the original Life consumer. Raw records retain zero and
-- every modifier type; Tabulate's own zero filtering is reported separately.
local lifeAdjustmentNames={"Life","ExtraLife","LifeTotal","LifeConvertToEnergyShield","LifeConvertToArmour","LifeConvertToEvasion","ChaosInoculation"}
local inherentLifeFlags={"NoAttributeBonuses","NoStrengthAttributeBonuses","NoStrBonusToLife","DoubledInherentAttributeBonuses","HalvesLifeFromStrength"}
local function lifeAdjustmentWanted()
 local wanted={};for _,name in ipairs(lifeAdjustmentNames) do wanted[name]=true end;return wanted
end
local function inherentLifeInputs(store,output)
 local flags={}
 for _,name in ipairs(inherentLifeFlags) do
  local value=store:Flag(nil,name)
  flags[#flags+1]={name=name,present=value~=nil,value=scalar(value),eligible=records(store,"FLAG",nil,name),raw=rawRecords(store,{[name]=true})}
 end
 return {strength={present=output.Str~=nil,value=scalar(output.Str)},flags=flags,
  -- Raw inputs only: do not imply calcLib.val evaluated its lazy INC/MORE
  -- branches when the original attribute BASE was zero.
  raw_attributes=rawRecords(store,{Str=true,Attributes=true})}
end
local function lifeAdjustmentInputs(actor,vars,auth)
 local store=vars.modDB;assert(store==actor.modDB and vars.output==actor.output)
 assert(actor.mainSkill.summonSkill.minion==actor and actor.mainSkill.summonSkill.actor==actor.parent)
 assert(store~=actor.parent.modDB and store.actor==actor)
 local channels={}
 channels.Life={base=scalarChannel(store,"BASE",nil,{"Life"}),increased=scalarChannel(store,"INC",nil,{"Life"}),more=scalarChannel(store,"MORE",nil,{"Life"}),overrides=records(store,"OVERRIDE",nil,"Life")}
 for _,name in ipairs({"ExtraLife","LifeTotal","LifeConvertToEnergyShield","LifeConvertToArmour","LifeConvertToEvasion"}) do
  channels[name]=scalarChannel(store,"BASE",nil,{name})
 end
 local strength={};local cursor,depth=store,0
 while cursor do
  assert(depth<16)
  for position,mod in ipairs(cursor.mods and cursor.mods.Life or {}) do if mod.source=="Strength" then
   local insertion=auth.life_strength_objects[actor] and auth.life_strength_objects[actor][mod]
   if insertion then assert(equal(modRecord(mod),insertion.record)) end
   local tabulated=0;for _,entry in ipairs(store:Tabulate("BASE",nil,"Life")) do if entry.mod==mod then tabulated=tabulated+1 end end
   strength[#strength+1]={ancestor_depth=depth,position=position,record=modRecord(mod),tabulated_base_occurrences=tabulated,
    original_insertion_observed=insertion~=nil,insertion_index=insertion and insertion.index or nil}
  end end
  cursor=cursor.parent;depth=depth+1
 end
 return {observed_at=97,exact_actor_store=true,store_is_player=false,exact_summoner=true,
  raw_modifiers=rawRecords(store,lifeAdjustmentWanted()),eligible_modifiers=records(store,nil,nil,unpack(lifeAdjustmentNames)),
  tabulate_omits_zero_non_override=true,channels=channels,
  -- This is a supplementary query to the unchanged original Sum method, not a
  -- copied formula or a claim that the capped local contains the original sum.
  conversion_before_cap=scalarChannel(store,"BASE",nil,{"LifeConvertToEnergyShield","LifeConvertToArmour","LifeConvertToEvasion"}),
  channel_queries_are_supplemental=true,original_capped_conversion=vars.conv,
  selected_override={present=vars.override~=nil,value=scalar(vars.override)},
  chaos_inoculation={present=vars.output.ChaosInoculation~=nil,value=scalar(vars.output.ChaosInoculation),
   eligible=records(store,"FLAG",nil,"ChaosInoculation"),raw=rawRecords(store,{ChaosInoculation=true})},
  inherent_inputs=inherentLifeInputs(store,vars.output),strength_records=strength}
end
local function damageInputs(active,cfg,source,output)
 local actor,store=active.actor,active.skillModList
 local enemy=actor.enemy.modDB
 local checked=watchStores({store,actor.modDB,enemy})
 local oldCfg,oldSource,oldOutput=clone(cfg),clone(source),clone(output)
 local wanted={Damage=true,PhysicalDamage=true,AddedDamage=true,AddedPhysicalDamage=true,MinPhysicalDamage=true,MaxPhysicalDamage=true,Gigantic=true,MinionModifier=true,DealNoDamage=true}
 local bases,conversion,gain={},{},{}
 for _,from in ipairs(damageTypes) do
  wanted[from.."Min"]=true;wanted[from.."Max"]=true;wanted["DealNo"..from]=true
  bases[from]={minimum=scalarChannel(store,"BASE",cfg,{from.."Min"}),maximum=scalarChannel(store,"BASE",cfg,{from.."Max"}),enemy_minimum=scalarChannel(enemy,"BASE",cfg,{"Self"..from.."Min"}),enemy_maximum=scalarChannel(enemy,"BASE",cfg,{"Self"..from.."Max"}),added_increased=scalarChannel(store,"INC",cfg,{"Added"..from.."Damage","AddedDamage"}),added_more=scalarChannel(store,"MORE",cfg,{"Added"..from.."Damage","AddedDamage"}),can_deal=not store:Flag(cfg,"DealNo"..from,"DealNoDamage")}
  conversion[from]={};gain[from]={}
  for _,to in ipairs(damageTypes) do
   local global={"DamageConvertTo"..to,from.."DamageConvertTo"..to}
   if elemental[from] then global[#global+1]="ElementalDamageConvertTo"..to end
   if from~="Chaos" then global[#global+1]="NonChaosDamageConvertTo"..to end
   local skill={"SkillDamageConvertTo"..to,"Skill"..from.."DamageConvertTo"..to}
   conversion[from][to]={global=scalarChannel(store,"BASE",active.skillCfg,global),skill=scalarChannel(store,"BASE",active.skillCfg,skill)}
   local globalGain={"DamageAs"..to,"DamageGainAs"..to,from.."DamageAs"..to,from.."DamageGainAs"..to}
   if elemental[from] then globalGain[#globalGain+1]="ElementalDamageAs"..to;globalGain[#globalGain+1]="ElementalDamageGainAs"..to end
   if from~="Chaos" then globalGain[#globalGain+1]="NonChaosDamageAs"..to;globalGain[#globalGain+1]="NonChaosDamageGainAs"..to end
   local skillGain={"SkillDamageGainAs"..to,"Skill"..from.."DamageGainAs"..to}
   if elemental[from] then skillGain[#skillGain+1]="SkillElementalDamageGainAs"..to end
   if from~="Chaos" then skillGain[#skillGain+1]="SkillNonChaosDamageGainAs"..to end
   gain[from][to]={global=scalarChannel(store,"BASE",active.skillCfg,globalGain),skill=scalarChannel(store,"BASE",active.skillCfg,skillGain)}
   for _,list in ipairs({global,skill,globalGain,skillGain}) do for _,name in ipairs(list) do wanted[name]=true end end
  end
 end
 local level=active.activeEffect.grantedEffectLevel
 local result={source=scalars(source),cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),skill_types=setRows(cfg.skillTypes),skill_flags=scalars(active.skillFlags),
  base_coefficient={level_present=level.baseMultiplier~=nil,level=level.baseMultiplier,skill_data_present=active.skillData.baseMultiplier~=nil,skill_data=active.skillData.baseMultiplier},
  skill_data=scalars(active.skillData),granted_effect_level=scalars(level),bases=bases,conversion=conversion,gain=gain,
  conversion_table=clone(active.conversionTable),gain_table=clone(active.gainTable),gain_only_cold=not not store:Flag(active.skillCfg,"DamageGainIsOnlyCold"),
  raw_skill_modifiers=rawRecords(store,wanted),raw_player_minion_modifiers=rawRecords(actor.parent.modDB,{MinionModifier=true}),
  actor_gigantic=not not actor.modDB:Flag(nil,"Gigantic"),gigantic_records=records(actor.modDB,"FLAG",nil,"Gigantic"),
  query_state_preserved=true}
 checked();assert(equal(cfg,oldCfg) and equal(source,oldSource) and equal(output,oldOutput));return result
end
local function commandRecipient(active,env,calls)
 local store,actor,cfg=active.skillModList,active.actor,active.skillCfg
 local checked=watchStores({store,actor.modDB,actor.parent.modDB});local priorCfg=clone(cfg)
 local raw,joins={},{}
 local db,depth=store,0
 while db do
  assert(depth<16)
  local list=db.mods and db.mods.CooldownRecovery or db
  for index,m in ipairs(list or {}) do if m.name=="CooldownRecovery" then
   raw[#raw+1]={ancestor_depth=depth,position=index,mod=modRecord(m)}
   local matches,parent,parentDepth={},actor.parent.modDB,0
   while parent do
    assert(parentDepth<16)
    for position,outer in ipairs(parent.mods.MinionModifier or {}) do
     if outer.value.mod==m then matches[#matches+1]={ancestor_depth=parentDepth,position=position,
      outer=modRecord(outer),exact_inner_object=true} end
    end
    parent=parent.parent;parentDepth=parentDepth+1
   end
   joins[#joins+1]={raw_index=#raw,player_minion_modifiers=matches}
  end end
  db=db.parent;depth=depth+1
 end
 local result={effect_id=active.activeEffect.grantedEffect.id,cfg=sourceConfig(active,cfg),
  commandable=not not store:Flag(cfg,"Condition:CommandableSkill"),
  condition_records=records(store,"FLAG",cfg,"Condition:CommandableSkill"),
  raw_cooldown_modifiers=raw,producer_joins=joins,
  received=scalarChannel(store,"INC",cfg,{"CooldownRecovery"}),
  original_cooldown_calls={},source_query_state_preserved=true,
  actor_is_actual_minion=actor.minionData~=nil,actor_parent_is_player=actor.parent==env.player,
  selected=actor.mainSkill==active,output=scalars(active.output)}
 for _,call in ipairs(calls or {}) do
  assert(call.cfg==cfg and call.skill_data==active.skillData)
  result.original_cooldown_calls[#result.original_cooldown_calls+1]=call.report
 end
 checked();assert(equal(cfg,priorCfg));return result
end
-- Opt-in Actor resource evidence; ordinary physical-damage report shape is unchanged.
local function intrinsicLifeFacts(env,active)
 local actor=assert(active.minion);local profile=assert(env.data.minions.RaisedSkeletonSniper)
 return {actor_level=actor.level,effective_level=active.activeEffect.level,hostile=actor.hostile,
  profile_hostile={present=profile.hostile~=nil,value=profile.hostile},profile_life=profile.life,
  profile_is_loaded=actor.minionData==profile,life_table_is_allied=actor.lifeTable==env.data.monsterAllyLifeTable,
  life_table_is_hostile=actor.lifeTable==env.data.monsterLifeTable,table_value=actor.lifeTable[actor.level],
  exact_source_actor=active.minion==actor,exact_parent=actor.parent==env.player,
  source=sourceOccurrence(active)}
end
local function intrinsicLifeDefinitions(env)
 local table=assert(env.data.monsterAllyLifeTable);local saved=clone(table)
 assert(#table==100);local count=0;for key,value in pairs(table) do
  assert(type(key)=="number" and key==math.floor(key) and key>=1 and key<=100 and type(value)=="number");count=count+1
 end;assert(count==100)
 local profile=assert(env.data.minions.RaisedSkeletonSniper);local before=clone(profile)
 local result={allied_life=clone(table),profile=clone(profile),profile_id="RaisedSkeletonSniper",
  profile_hostile={present=profile.hostile~=nil,value=profile.hostile},minion_levels=clone(env.data.minionLevelTable),
  global_table_identity=table==data.monsterAllyLifeTable,global_profile_identity=profile==data.minions.RaisedSkeletonSniper}
 assert(equal(table,saved) and equal(profile,before));return result
end
local function allocatedIds(env)
 local ids={};for id in pairs(env.spec.allocNodes) do ids[#ids+1]=id end;table.sort(ids);return ids
end
local function lifeProvider(env,mod)
 local id=mod.source and tonumber(mod.source:match("^Tree:(%d+)$"))
 if not id then return {tree_source=false} end
 local node=env.spec.allocNodes[id];local matches={}
 if node then for _,outer in ipairs(node.modList or {}) do
  local inner=outer.name=="MinionModifier" and type(outer.value)=="table" and outer.value.mod
  if inner and inner.name=="Life" and inner.type=="INC" then
   matches[#matches+1]={record=modRecord(inner),exact_delivered_object=inner==mod}
  end
 end end
 return {tree_source=true,node_id=id,allocated=node~=nil,matches=matches}
end
local function benefitSnapshot()
 local function frame(env)
  local actors={}
  for ordinal,summoner in ipairs(env.player.activeSkillList) do
   local actor=summoner.minion
   if actor and actor.type=="RaisedSkeletonSniper" then
    local checked=watchStores({actor.modDB});local before=clone(actor.output)
    actors[#actors+1]={ordinal=ordinal,source=sourceOccurrence(summoner),summon_effect=summoner.activeEffect.grantedEffect.id,
     actor_profile=actor.type,selected=actor==env.minion,output=scalars(actor.output),
     raw_benefit_modifiers=rawRecords(actor.modDB,{Life=true,Damage=true,Gigantic=true}),
     raw_life_adjustments=physicalDamageLifeAdjustmentEvidence and rawRecords(actor.modDB,lifeAdjustmentWanted()) or nil,
     raw_inherent_life_flags=physicalDamageLifeAdjustmentEvidence and rawRecords(actor.modDB,{NoAttributeBonuses=true,NoStrengthAttributeBonuses=true,NoStrBonusToLife=true,DoubledInherentAttributeBonuses=true,HalvesLifeFromStrength=true}) or nil,
     raw_inherent_attributes=physicalDamageLifeAdjustmentEvidence and rawRecords(actor.modDB,{Str=true,Attributes=true}) or nil,
     life_adjustment_identity=physicalDamageLifeAdjustmentEvidence and {output_present=actor.output~=nil,
      strength={present=actor.output~=nil and actor.output.Str~=nil,value=scalar(actor.output and actor.output.Str)},
      exact_actor_store=actor.modDB.actor==actor,store_is_player=actor.modDB==env.player.modDB,
      exact_parent=actor.parent==env.player,exact_summoner=summoner.minion==actor and summoner.actor==env.player} or nil,
     intrinsic_life=physicalDamageIntrinsicLifeEvidence and intrinsicLifeFacts(env,summoner) or nil}
    checked();assert(equal(before,actor.output))
   end
  end
  return {combat=not not env.mode_combat,buffs=not not env.mode_buffs,effective=not not env.mode_effective,
   actors=actors,player_output=scalars(env.player.output),
   allocated_node_ids=physicalDamageLifeDeliveryEvidence and allocatedIds(env) or nil}
 end
 assert(debug.gethook()==nil)
 return {main=frame(build.calcsTab.mainEnv),calcs=frame(build.calcsTab.calcsEnv)}
end
if physicalDamageBenefitEvidence and physicalDamagePhase=="benefit_snapshot" then return benefitSnapshot() end
local function offeringOutputSnapshot()
 -- This projection only reads identities and already-calculated scalar outputs.
 -- It is shared by instrumented and entirely uninstrumented fresh VMs; it does
 -- not query modifier stores, rerun a calculation or mirror any formula.
 assert(debug.gethook()==nil)
 local function output(value) return {available=value~=nil,scalars=scalars(value)} end
 local function frame(env,cached)
  local actors={}
  for ordinal,summoner in ipairs(env.player.activeSkillList) do
   local actor=summoner.minion
   if actor then
    local children={}
    for index,active in ipairs(actor.activeSkillList or {}) do
     children[#children+1]={index=index,effect_id=active.activeEffect.grantedEffect.id,
      selected=actor.mainSkill==active,output=output(active.output)}
    end
    actors[#actors+1]={ordinal=ordinal,source_occurrence=sourceOccurrence(summoner),
     actor_profile=actor.type,selected=actor==env.minion,output=output(actor.output),children=children}
   end
  end
  return {mode=env.mode,main_group=env.mainSocketGroup,selected_minion=env.minion and recipientOccurrence(env,env.minion),
   player_output=output(env.player.output),cached_output=output(cached),
   player_selected={effect_id=env.player.mainSkill.activeEffect.grantedEffect.id,source_occurrence=sourceOccurrence(env.player.mainSkill),output=output(env.player.mainSkill.output)},
   actors=actors}
 end
 return {main=frame(build.calcsTab.mainEnv,build.calcsTab.mainOutput),calcs=frame(build.calcsTab.calcsEnv,build.calcsTab.calcsOutput)}
end
if physicalDamageOfferingEvidence and physicalDamagePhase=="offering_snapshot" then return offeringOutputSnapshot() end
local methods={
 {common.classes.SkillsTab,"LoadSkill","Classes/SkillsTab.lua",303},
 {common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {calcs,"initEnv","Modules/CalcSetup.lua",717},
 {calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
 {calcs,"createMinionSkills","Modules/CalcActiveSkill.lua",1116},
 {calcs,"perform","Modules/CalcPerform.lua",1193},
 {calcs,"offence","Modules/CalcOffence.lua",527},
 {calcLib,"mod","Modules/CalcTools.lua",16},
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"More","Classes/ModStore.lua",261},
 {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModStore,"EvalMod","Classes/ModStore.lua",490},
 {common.classes.ModStore,"ScaleAddMod","Classes/ModStore.lua",82},
 {common.classes.ModStore,"ScaleAddList","Classes/ModStore.lua",127},
 {common.classes.ModDB,"MoreInternal","Classes/ModDB.lua",214},
 {common.classes.ModDB,"MoreInternalMulti","Classes/ModDB.lua",254},
 {common.classes.ModList,"MoreInternal","Classes/ModList.lua",164},
 {common.classes.ModList,"MoreInternalMulti","Classes/ModList.lua",195},
 {modLib,"compareModParams","Modules/ModTools.lua",144},
}
if physicalDamageBenefitEvidence then
 methods[#methods+1]={calcs,"doActorLifeManaSpirit","Modules/CalcDefence.lua",74}
 methods[#methods+1]={calcs,"defence","Modules/CalcDefence.lua",789}
end
if physicalDamageLifeDeliveryEvidence then
 methods[#methods+1]={common.classes.ModStore,"List","Classes/ModStore.lua",321}
 methods[#methods+1]={common.classes.ModDB,"AddMod","Classes/ModDB.lua",31}
end
if physicalDamageLifeAdjustmentEvidence then
 assert(physicalDamageBenefitEvidence and physicalDamageIntrinsicLifeEvidence and physicalDamageLifeDeliveryEvidence)
 methods[#methods+1]={common.classes.ModStore,"Override","Classes/ModStore.lua",301}
 methods[#methods+1]={common.classes.ModStore,"NewMod","Classes/ModStore.lua",142}
end
if physicalDamagePhase=="before" then
 local refs={};for i,row in ipairs(methods) do refs[i]=original(row[1][row[2]],row[3],row[4]) end
 local calcDamage=original(upvalue(calcs.offence,"calcDamage"),"Modules/CalcOffence.lua",178)
 local mergeBuff=original(upvalue(calcs.perform,"mergeBuff"),"Modules/CalcPerform.lua",42)
 local cooldown=physicalDamageCommandEvidence and original(calcSkillCooldown,"Modules/CalcOffence.lua",410)
 local initMinion=physicalDamageIntrinsicLifeEvidence and original(upvalue(calcs.perform,"initMinionModDB"),"Modules/CalcPerform.lua",1049)
 local transferMinion=physicalDamageLifeDeliveryEvidence and original(upvalue(calcs.perform,"addMinionModifiers"),"Modules/CalcPerform.lua",1161)
 local actorAttributes=physicalDamageLifeAdjustmentEvidence and original(upvalue(calcs.perform,"doActorAttribsConditions"),"Modules/CalcPerform.lua",264)
 local priorActors={}
 for _,env in ipairs({build.calcsTab.mainEnv,build.calcsTab.calcsEnv}) do if env then for _,active in ipairs(env.player.activeSkillList) do if active.minion then priorActors[active.minion]=true end end end end
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local auth={refs=refs,calc_damage=calcDamage,merge_buff=mergeBuff,previous_actors=priorActors,captures={},calls={},base_calls={},buff_events={},count=0,line_events=0,
  cooldown=cooldown,cooldown_calls={},command_recipients={},life_calls={},
  init_minion=initMinion,life_selections={},life_initializers={},life_base_objects={},
  transfer_minion=transferMinion,life_transfers={},life_delivered_objects={},
  actor_attributes=actorAttributes,life_strength_insertions={},life_strength_objects={},
  intrinsic_definitions=physicalDamageIntrinsicLifeEvidence and intrinsicLifeDefinitions({data=data}) or nil,
  offering_more_calls={}}
 local pending,lifePending,selectionPending,initPending,deliveryPending
 local moreStack={}
 local baseHookMask=(physicalDamageBenefitEvidence or physicalDamageOfferingEvidence) and "cr" or "r"
 local function lineMask() return baseHookMask.."l" end
 local function relevant(active)
  return not physicalDamageCommandEvidence and active and active.actor and active.actor.minionData and active.activeEffect.grantedEffect.id=="MinionMeleeBow"
 end
 local function commandRelevant(active)
  return active and active.actor and active.actor.type=="RaisedSkeletonSniper"
   and (active.activeEffect.grantedEffect.id=="MinionMeleeBow" or active.activeEffect.grantedEffect.id=="GasShotSkeletonSniperMinion")
 end
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  if physicalDamageLifeAdjustmentEvidence and event=="return" and f==common.classes.ModDB.AddMod then
   local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
   local mod,store=vars.mod,vars.self
   if mod and mod.name=="Life" and mod.type=="BASE" and mod.source=="Strength" and store.actor and store.actor.type=="RaisedSkeletonSniper" then
    local outer,callerLine
    for depth=3,16 do
     local info=debug.getinfo(depth,"fl");if not info then break end
     if info.func==actorAttributes then
      outer={};callerLine=info.currentline
      for i=1,160 do local name,value=debug.getlocal(depth,i);if not name then break end;outer[name]=value end
      break
     end
    end
    assert(outer and (callerLine==504 or callerLine==506))
    local actor,env=assert(outer.actor),assert(outer.env)
    assert(store==outer.modDB and store==actor.modDB and store.actor==actor and actor.output==outer.output)
    assert(actor.parent==env.player and store~=env.player.modDB)
    local active=assert(actor.mainSkill.summonSkill);assert(active.minion==actor and active.actor==env.player)
    local checked=watchStores({store,env.player.modDB});local count=0
    for _,current in ipairs(store.mods.Life or {}) do if current==mod then count=count+1 end end
    assert(count==1)
    local rows=auth.life_strength_insertions[actor] or {};auth.life_strength_insertions[actor]=rows
    local row={index=#rows+1,caller_source="Modules/CalcPerform.lua",original_function_line=264,caller_line=callerLine,
     source=sourceOccurrence(active),mode=env.mode,selected=actor==env.minion,actor_profile=actor.type,
     exact_actor_store=true,store_is_player=false,exact_summoner=true,original_record_preserved=true,
     stored_identity_count=count,record=modRecord(mod),inputs=inherentLifeInputs(store,outer.output),
     inherent_attribute_multiplier=outer.inherentAttributeMultiplier}
    rows[#rows+1]=row;assert(#rows<=8)
    local objects=auth.life_strength_objects[actor] or {};auth.life_strength_objects[actor]=objects
    assert(not objects[mod]);objects[mod]=row;checked()
   end
  end
  if physicalDamageOfferingEvidence and (f==common.classes.ModDB.MoreInternal or f==common.classes.ModList.MoreInternal) then
   local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
   if event=="call" and (vars.modName=="BuffEffect" or vars.modName=="BuffEffectOnSelf" or vars.modName=="Magnitude") then
    local caller,callerLine
    for depth=3,24 do
     local info=debug.getinfo(depth,"fl");if not info then break end
     if info.func==calcs.perform then
      caller={};callerLine=info.currentline
      for i=1,160 do local name,value=debug.getlocal(depth,i);if not name then break end;caller[name]=value end
      break
     end
    end
    -- This is the original Offering -> minion scaling expression, not queries
    -- made by this observer or a different buff/display consumer.
    if caller and callerLine==2147 and caller.activeSkill and caller.activeSkill.activeEffect.grantedEffect.id=="PainOfferingPlayer" then
     local env,active=assert(caller.env),caller.activeSkill
     local domain
     if vars.context==caller.modStore then domain=caller.buff.activeSkillBuff and "source_skill" or "source_actor"
     elseif vars.context==env.minion.modDB then domain="recipient_actor" else error("unknown Offering MORE context") end
     local depth,store=0,vars.context
     while store~=vars.self do store=assert(store.parent);depth=depth+1;assert(depth<16) end
     local own={};for i,mod in ipairs(vars.self.mods and (vars.self.mods[vars.modName] or {}) or vars.self) do
      if mod.name==vars.modName then own[#own+1]={position=i,mod=modRecord(mod)} end
     end
     local row={name=vars.modName,domain=domain,source_occurrence=sourceOccurrence(active),recipient_occurrence=recipientOccurrence(env,env.minion),
      caller_line=callerLine,store_kind=f==common.classes.ModDB.MoreInternal and "ModDB" or "ModList",
      original_function_line=f==common.classes.ModDB.MoreInternal and 214 or 164,
      context_is_source_skill=vars.context==active.skillModList,context_is_recipient_actor=vars.context==env.minion.modDB,
      local_store_is_source_skill=vars.self==active.skillModList,local_store_is_player_actor=vars.self==env.player.modDB,
      local_store_is_recipient_actor=vars.self==env.minion.modDB,source_store_depth=depth,
      flags=vars.flags,keyword_flags=vars.keywordFlags,source_filter=vars.source,cfg=scalars(vars.cfg),
      local_candidates=own,original_steps={}}
     local calls=auth.offering_more_calls[env] or {};auth.offering_more_calls[env]=calls
     calls[#calls+1]=row;assert(#calls<=512)
     moreStack[#moreStack+1]={f=f,store=vars.self,row=row};assert(#moreStack<=16)
     debug.sethook(hook,lineMask())
    end
   else
    local top=moreStack[#moreStack]
    if top and top.f==f and top.store==vars.self then
     local db=f==common.classes.ModDB.MoreInternal
     if event=="line" then
      if (db and line==233) or (not db and (line==172 or line==174)) then
       top.step={mod=modRecord(assert(vars.mod)),product_before=assert(vars.modResult),line=line,evaluated_value=db and vars.value or nil}
      elseif (db and line==234) or (not db and line==176) then
       assert(top.step);top.step.product_after=assert(vars.modResult)
       top.row.original_steps[#top.row.original_steps+1]=top.step;top.step=nil
      elseif (db and line==242) or (not db and line==183) then
       top.row.local_product_before_rounding=assert(vars.modResult);top.row.precision_present=vars.modPrecision~=nil;top.row.precision=vars.modPrecision
      elseif (db and line==248) or (not db and line==189) then
       top.row.local_result_before_parent=assert(vars.result)
      end
     elseif event=="return" then
      assert(not top.step and top.row.local_product_before_rounding and top.row.local_result_before_parent)
      top.row.original_return_result=assert(vars.result);top.row.return_observed=true
      table.remove(moreStack)
      if #moreStack==0 and not pending then debug.sethook(hook,baseHookMask) end
     end
    end
   end
   return
  end
  if physicalDamageLifeDeliveryEvidence then
   if f==transferMinion then
    local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
    local actor=vars.minion
    if actor and actor.type=="RaisedSkeletonSniper" then
     if event=="call" then
      assert(not deliveryPending)
      local caller=debug.getinfo(3,"flS");local outer={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;outer[name]=value end
      if caller.func~=calcs.perform then original(caller.func,"Modules/CalcPerform.lua",3365) end
      local env=assert(outer.env);local active=assert(actor.mainSkill.summonSkill)
      deliveryPending={actor=actor,env=env,store=actor.modDB,source_store=vars.modList,cfg=vars.skillCfg,payloads={},
       row={caller_line=caller.currentline,mode=env.mode,selected=actor==env.minion,actor_profile=actor.type,
        source=sourceOccurrence(active),exact_parent=actor.parent==env.player,exact_summoner=active.minion==actor,
        parent_skill_store=vars.modList==active.skillModList,exact_parent_cfg=vars.skillCfg==active.skillCfg,
        listed_life={},inserted_life={}}}
     elseif event=="return" then
      assert(deliveryPending and deliveryPending.actor==actor and deliveryPending.row.list_return_observed)
      assert(actor.modDB==deliveryPending.store)
      local list=auth.life_transfers[actor] or {};auth.life_transfers[actor]=list;list[#list+1]=deliveryPending.row
      assert(#list<=64);deliveryPending=nil
     end
    end
    return
   elseif event=="return" and (f==common.classes.ModStore.List or f==common.classes.ModDB.AddMod) then
    local caller=debug.getinfo(3,"fl")
    if caller and caller.func==transferMinion and deliveryPending then
     local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
     local outer={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;outer[name]=value end
     local d=deliveryPending;assert(outer.minion==d.actor and outer.modList==d.source_store and outer.skillCfg==d.cfg)
     if f==common.classes.ModStore.List then
      assert(vars.self==d.source_store and vars.cfg==d.cfg and vars.n==1 and caller.currentline==1162 and not d.row.list_return_observed)
      d.row.list_return_observed=true;d.row.list_caller_line=caller.currentline;d.row.all_payload_count=#vars.result
      for index,value in ipairs(vars.result) do
       if value.mod and value.mod.name=="Life" then
        assert(not d.payloads[value]);d.payloads[value]={index=index,mod=value.mod}
        d.row.listed_life[#d.row.listed_life+1]={payload_index=index,recipient_type={present=value.type~=nil,value=value.type},
         record=modRecord(value.mod),provider=lifeProvider(d.env,value.mod)}
       end
      end
     elseif vars.mod.name=="Life" then
      local payload=assert(d.payloads[outer.value]);assert(vars.self==d.store and payload.mod==vars.mod)
      local count=0;for _,mod in ipairs(d.store.mods.Life or {}) do if mod==vars.mod then count=count+1 end end
      assert(count>=1)
      d.row.inserted_life[#d.row.inserted_life+1]={payload_index=payload.index,record=modRecord(vars.mod),
       addmod_caller_line=caller.currentline,exact_list_payload=true,exact_actor_store=d.store.actor==d.actor,
       stored_identity_count=count,provider=lifeProvider(d.env,vars.mod)}
      local objects=auth.life_delivered_objects[d.actor] or {};auth.life_delivered_objects[d.actor]=objects
      objects[vars.mod]=(objects[vars.mod] or 0)+1
     end
    end
    return
   end
  end
  if physicalDamageIntrinsicLifeEvidence then
   if f==calcs.buildActiveSkillModList or f==initMinion then
    local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
    local active,env=vars.activeSkill,vars.env
    if active and active.activeEffect.grantedEffect.id=="SummonSkeletalSnipersPlayer" and not active.actor.minionData then
     if f==calcs.buildActiveSkillModList then
      if event=="call" then
       assert(not selectionPending and not initPending and not pending and not lifePending)
       selectionPending={active=active,env=env,ally_branch_executed=false};debug.sethook(hook,"crl")
      elseif event=="line" and line==961 then
       assert(selectionPending and selectionPending.active==active and vars.minion==active.minion)
       selectionPending.ally_branch_executed=true
      elseif event=="line" and line==963 then
       assert(selectionPending and selectionPending.active==active and vars.minion==active.minion)
       local actor=assert(active.minion);local row=intrinsicLifeFacts(env,active)
       row.observed_at=line;row.ally_branch_executed=selectionPending.ally_branch_executed
       row.skill_data=scalars(active.skillData);row.level_table_value=env.data.minionLevelTable[active.activeEffect.level]
       local list=auth.life_selections[actor] or {};auth.life_selections[actor]=list;list[#list+1]=row
       assert(#list<=4);selectionPending=nil;debug.sethook(hook,baseHookMask)
      elseif event=="return" then assert(not selectionPending) end
     else
      local actor=assert(active.minion)
      if event=="call" then
       assert(not initPending and not selectionPending and not pending and not lifePending)
       local caller=debug.getinfo(3,"flS");if caller.func~=calcs.perform then original(caller.func,"Modules/CalcPerform.lua",3365) end
       initPending={actor=actor,active=active,store=actor.modDB,row={caller_line=caller.currentline,
        source=sourceOccurrence(active),actor_profile=actor.type,mode=env.mode,selected=actor==env.minion}}
       debug.sethook(hook,"crl")
      elseif event=="line" and line==1061 then
       assert(initPending and initPending.actor==actor and vars.minion==actor and actor.modDB==initPending.store)
       local row=initPending.row;row.input=intrinsicLifeFacts(env,active);row.unrounded_base_life=vars.baseLife
       row.unrounded_observed_at=line;row.base_records_before=rawRecords(actor.modDB,{Life=true})
       initPending.prior={};for _,mod in ipairs(actor.modDB.mods.Life or {}) do initPending.prior[mod]=true end
      elseif event=="line" and line==1065 then
       assert(initPending and initPending.actor==actor and initPending.row.input and vars.minion==actor)
       local added={};for _,mod in ipairs(actor.modDB.mods.Life or {}) do if not initPending.prior[mod] then added[#added+1]=mod end end
       assert(#added==1);local mod=added[1];assert(mod.name=="Life" and mod.type=="BASE" and mod.source=="Base")
       initPending.mod=mod;initPending.row.stored_base=modRecord(mod);initPending.row.stored_observed_at=line
       initPending.row.base_life_at_store=vars.baseLife;initPending.row.exact_actor_store=actor.modDB.actor==actor
       auth.life_base_objects[actor]=mod;debug.sethook(hook,baseHookMask)
      elseif event=="return" then
       assert(initPending and initPending.actor==actor and initPending.mod and actor.modDB==initPending.store)
       local found=0;for _,mod in ipairs(actor.modDB.mods.Life or {}) do if mod==initPending.mod then found=found+1 end end
       assert(found==1);initPending.row.original_record_preserved=true
       local list=auth.life_initializers[actor] or {};auth.life_initializers[actor]=list;list[#list+1]=initPending.row
       assert(#list<=4);initPending=nil
      end
     end
    end
    return
   end
  end
  if physicalDamageBenefitEvidence then
   if lifePending and lifePending.returned and f==lifePending.caller and event=="line" then
    local actor,row=lifePending.actor,lifePending.row
    assert(actor.modDB==lifePending.store and actor.output==lifePending.output)
    row.post_return_life=actor.output.Life;row.post_return_observed_at=line
    assert(row.post_return_life==row.return_life)
    local list=auth.life_calls[actor] or {};auth.life_calls[actor]=list;list[#list+1]=row
    assert(#list<=32);lifePending=nil;debug.sethook(hook,baseHookMask)
   end
   if f==calcs.doActorLifeManaSpirit then
    local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
    local actor=vars.actor
    if actor and actor.type=="RaisedSkeletonSniper" then
     if event=="call" then
      assert(not lifePending and not pending)
      local caller=debug.getinfo(3,"flS");assert(caller and caller.what=="Lua")
      if caller.func~=calcs.defence and caller.func~=calcs.perform then original(caller.func,"Modules/CalcPerform.lua",3365) end
      lifePending={actor=actor,store=actor.modDB,output=actor.output,caller=caller.func,
       row={caller_source=caller.func==calcs.defence and "Modules/CalcDefence.lua" or "Modules/CalcPerform.lua",caller_line=caller.currentline,
        actor_profile=actor.type,exact_actor_store=actor.modDB.actor==actor,exact_actor_output=true,
        source=sourceOccurrence(actor.mainSkill.summonSkill),summoner_owns_actor=actor.mainSkill.summonSkill.minion==actor,
        summoner_actor_is_parent=actor.mainSkill.summonSkill.actor==actor.parent}}
      debug.sethook(hook,"crl")
     elseif event=="line" and line==97 and vars.res=="Life" then
      assert(lifePending and lifePending.actor==actor and not lifePending.row.computation)
      assert(vars.modDB==actor.modDB and vars.output==actor.output)
      local checked=watchStores({vars.modDB});local row=lifePending.row
      row.computation={observed_at=line,base=vars.base,extra=vars.extra,total=vars.total,increased=vars.inc,
       more=vars.more,conversion=vars.conv,override_present=vars.override~=nil,override=vars.override,
       life_after_assignment=vars.output.Life,raw_modifiers=rawRecords(vars.modDB,{Life=true,Damage=true,Gigantic=true}),
       eligible_life_more=records(vars.modDB,"MORE",nil,"Life"),gigantic=not not vars.modDB:Flag(nil,"Gigantic"),
       gigantic_records=records(vars.modDB,"FLAG",nil,"Gigantic"),
       life_precision={present=data.highPrecisionMods.Life~=nil,types=scalars(data.highPrecisionMods.Life)}}
      if physicalDamageIntrinsicLifeEvidence then
       local originalBase=assert(auth.life_base_objects[actor]);local matches=0
       for _,entry in ipairs(vars.modDB:Tabulate("BASE",nil,"Life")) do if entry.mod==originalBase then matches=matches+1 end end
       assert(matches==1);row.computation.intrinsic_base={original_record_is_eligible=true,record=modRecord(originalBase),
        eligible_base_records=records(vars.modDB,"BASE",nil,"Life")}
      end
      if physicalDamageLifeDeliveryEvidence then
       local eligible={};local objects=auth.life_delivered_objects[actor] or {}
       for _,entry in ipairs(vars.modDB:Tabulate("INC",nil,"Life")) do
        eligible[#eligible+1]={value=entry.value,record=modRecord(entry.mod),actual_transfer_count=objects[entry.mod] or 0}
       end
       row.computation.life_increase_delivery={eligible=eligible,raw_increase=records(vars.modDB,"INC",nil,"Life")}
      end
      if physicalDamageLifeAdjustmentEvidence then row.computation.adjustments=lifeAdjustmentInputs(actor,vars,auth) end
      checked();assert(actor.output==lifePending.output)
     elseif event=="return" then
      assert(lifePending and lifePending.actor==actor and lifePending.row.computation)
      lifePending.row.return_life=actor.output.Life;lifePending.returned=true
     end
    end
    return
   end
  end
  if f~=calcDamage and f~=calcs.offence and f~=mergeBuff and f~=calcLib.mod and f~=cooldown then return end
  if event~="return" and not (event=="line" and pending and f==calcs.offence) then return end
  local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
  if physicalDamageCommandEvidence and f==cooldown and event=="return" then
   local callerInfo=debug.getinfo(3,"fl")
   if callerInfo and callerInfo.func==calcs.offence then
    local caller={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;caller[name]=value end
    if commandRelevant(caller.activeSkill) then
     assert(vars.skillModList==caller.activeSkill.skillModList and vars.skillCfg==caller.activeSkill.skillCfg)
     local list=auth.cooldown_calls[vars.skillModList] or {};auth.cooldown_calls[vars.skillModList]=list
     list[#list+1]={cfg=vars.skillCfg,skill_data=vars.skillData,report={caller_line=callerInfo.currentline,
      cooldown=vars.cooldown,rounded=vars.rounded,added_cooldown=vars.addedCooldown,no_cooldown_chance=vars.noCooldownChance,
      exact_skill_store=true,exact_cfg=true}}
     assert(#list<=32)
    end
   end
   return
  end
  if physicalDamageCommandEvidence and f==calcs.offence and event=="return" and commandRelevant(vars.activeSkill) then
   local active=vars.activeSkill
   auth.command_recipients[active]=commandRecipient(active,vars.env,auth.cooldown_calls[active.skillModList])
   return
  end
  if f==calcLib.mod then
   local callerInfo=debug.getinfo(3,"fl")
   if callerInfo and callerInfo.func==calcs.offence and callerInfo.currentline==4134 then
    local caller={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;caller[name]=value end
    if relevant(caller.activeSkill) then
     assert(not pending);pending={kind="base",active=caller.activeSkill,cfg=caller.cfg};debug.sethook(hook,lineMask())
    end
   end
   return
  end
  if f==mergeBuff then
   if vars.destKey=="Pain Offering" then
    local caller={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;caller[name]=value end
    assert(debug.getinfo(3,"f").func==calcs.perform)
    local env,active=assert(caller.env),assert(caller.activeSkill)
    local src,dest={},{}
    for _,mod in ipairs(vars.src) do src[#src+1]=modRecord(mod) end
    for _,mod in ipairs(vars.destTable[vars.destKey]) do dest[#dest+1]=modRecord(mod) end
    auth.buff_events[env]=auth.buff_events[env] or {}
    table.insert(auth.buff_events[env],{name=vars.destKey,source_effect=active.activeEffect.grantedEffect.id,
     source_level=active.activeEffect.level,recipient_profile=env.minion and env.minion.type,
     source_occurrence=sourceOccurrence(active),recipient_occurrence=env.minion and recipientOccurrence(env,env.minion),
     minion_destination=vars.destTable==caller.minionBuffs,scaling_increased=caller.inc,scaling_more=caller.more,
     source_modifiers=src,merged_modifiers=dest})
   end
   return
  end
  local active=vars.activeSkill;if not relevant(active) then return end
  if event=="return" and f==calcDamage then
   assert(not pending,"overlapping observed calcDamage call")
   local cfg,store=vars.cfg,active.skillModList
   local checked=watchStores({store});local oldCfg=clone(cfg)
   local row={damage_type=vars.damageType,type_flags=vars.typeFlags,critical=not not cfg.skillCond.CriticalStrike,
    summed_min=vars.summedMin,summed_max=vars.summedMax,add_min=vars.addMin,add_max=vars.addMax,
    modifier_names=clone(vars.modNames),increased_factor=vars.inc,more_factor=vars.more,minimum_more=vars.moreMinDamage,maximum_more=vars.moreMaxDamage,
    increased_records=vars.modNames and records(store,"INC",cfg,unpack(vars.modNames)),more_records=vars.modNames and records(store,"MORE",cfg,unpack(vars.modNames)),
    minimum_more_records=records(store,"MORE",cfg,"Min"..vars.damageType.."Damage"),maximum_more_records=records(store,"MORE",cfg,"Max"..vars.damageType.."Damage"),
    cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),query_state_preserved=true}
   checked();assert(equal(cfg,oldCfg))
   pending={kind="damage",active=active,cfg=cfg,row=row};debug.sethook(hook,lineMask())
  elseif event=="line" then
   auth.line_events=auth.line_events+1;assert(pending.active==active and pending.cfg==vars.cfg)
   if pending.kind=="base" then
    if line==4137 then
     local store,enemy,cfg=vars.skillModList,vars.enemyDB,vars.cfg
     local checked=watchStores({store,enemy});local oldCfg,oldSource,oldOutput=clone(cfg),clone(vars.source),clone(vars.output)
     local kind=vars.damageType
     local row={damage_type=kind,observed_at=line,cfg=scalars(cfg),skill_conditions=scalars(cfg.skillCond),source=scalars(vars.source),
      base_multiplier=vars.baseMultiplier,added_min=vars.addedMin,added_max=vars.addedMax,added_multiplier=vars.addedMult,base_min=vars.baseMin,base_max=vars.baseMax,
      minimum=scalarChannel(store,"BASE",cfg,{kind.."Min"}),maximum=scalarChannel(store,"BASE",cfg,{kind.."Max"}),
      enemy_minimum=scalarChannel(enemy,"BASE",cfg,{"Self"..kind.."Min"}),enemy_maximum=scalarChannel(enemy,"BASE",cfg,{"Self"..kind.."Max"}),
      added_increased=scalarChannel(store,"INC",cfg,{"Added"..kind.."Damage","AddedDamage"}),added_more=scalarChannel(store,"MORE",cfg,{"Added"..kind.."Damage","AddedDamage"}),
      query_state_preserved=true}
     checked();assert(equal(cfg,oldCfg) and equal(vars.source,oldSource) and equal(vars.output,oldOutput))
     auth.base_calls[active]=auth.base_calls[active] or {};table.insert(auth.base_calls[active],row)
     pending=nil;debug.sethook(hook,baseHookMask)
    end
   elseif line==4216 then
    pending.row.returned_min=vars.damageTypeHitMin;pending.row.returned_max=vars.damageTypeHitMax
    pending.row.return_observed_at=line
   elseif line==4257 then
    local row=pending.row;assert(row.return_observed_at==4216)
    assert(vars.damageTypeHitMin==row.returned_min and vars.damageTypeHitMax==row.returned_max)
    row.later_all_mult=vars.allMult;row.general_all_mult=vars.output.allMult;row.all_mult_observed_at=line
    auth.calls[active]=auth.calls[active] or {};table.insert(auth.calls[active],row)
    pending=nil;debug.sethook(hook,baseHookMask)
   end
  elseif event=="return" and f==calcs.offence then
   assert(not pending)
   local passes={}
   for _,pass in ipairs(vars.passList or {}) do
    local inputs=damageInputs(active,pass.cfg,pass.source,pass.output)
    passes[#passes+1]={label=pass.label,inputs=inputs,output=scalars(pass.output)}
   end
   auth.captures[active]={actor=vars.actor,mode=vars.env.mode,passes=passes,damage_calls=auth.calls[active] or {},base_calls=auth.base_calls[active] or {}}
   auth.calls[active]=nil;auth.base_calls[active]=nil;auth.count=auth.count+1
  end
 end
 local enabled=jit.status();jit.flush();assert(jit.status()==enabled);debug.sethook(hook,baseHookMask)
 physicalDamageAuth=auth
 return function()
  assert(debug.gethook()==hook and not pending and not lifePending and not selectionPending and not initPending and not deliveryPending and #moreStack==0);debug.sethook(oldHook,oldMask,oldCount);assert(jit.status()==enabled)
  for i,row in ipairs(methods) do assert(row[1][row[2]]==refs[i]) end
  assert(upvalue(calcs.offence,"calcDamage")==calcDamage and upvalue(calcs.perform,"mergeBuff")==mergeBuff);auth.finished=true
  if physicalDamageCommandEvidence then assert(calcSkillCooldown==cooldown) end
  if physicalDamageIntrinsicLifeEvidence then assert(upvalue(calcs.perform,"initMinionModDB")==initMinion) end
  if physicalDamageLifeDeliveryEvidence then assert(upvalue(calcs.perform,"addMinionModifiers")==transferMinion) end
  if physicalDamageLifeAdjustmentEvidence then assert(upvalue(calcs.perform,"doActorAttribsConditions")==actorAttributes) end
 end
end
local auth=assert(physicalDamageAuth);assert(auth.finished)
local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local mainOutput,calcsOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local savedMain,savedCalcs=clone(mainOutput),clone(calcsOutput)
local savedItems,savedSets,savedSkills,savedConfig=clone(build.itemsTab.items),clone(build.itemsTab.itemSets),clone(build.skillsTab.skillSets),clone(build.configTab.configSets)
local function allocations(spec)
 local out={};for id,node in pairs(spec.allocNodes) do out[id]={object=node,id=node.id,alloc=node.alloc,mode=node.allocMode} end;return out
end
local savedSpecs,savedSpecKeys={},keys(build.treeTab.specList)
for i,spec in ipairs(build.treeTab.specList) do savedSpecs[i]={object=spec,nodes=spec.allocNodes,allocations=allocations(spec),jewels=clone(spec.jewels)} end
local function selected()
 return {items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup}
end
local selection=selected()
local function consumer(env,actor,active)
 local observed=auth.captures[active];if observed then assert(observed.actor==actor and observed.mode==env.mode) end
 return {effect_id=active.activeEffect.grantedEffect.id,effect_name=active.activeEffect.grantedEffect.name,selected=actor.mainSkill==active,
  command_receiving=physicalDamageCommandEvidence and auth.command_recipients[active] or nil,
  summoner_source=active.summonSkill and sourceOccurrence(active.summonSkill),summoner_owns_actor=active.summonSkill and active.summonSkill.minion==actor,
  output=scalars(active.output),flags=scalars(active.skillFlags),passes=observed and observed.passes,damage_calls=observed and observed.damage_calls,base_calls=observed and observed.base_calls}
end
local actorStates={}
local function environment(env)
 local actors,identities,skills={},{},{}
 for ordinal,summoner in ipairs(env.player.activeSkillList) do
  local buffs={}
  for _,buff in ipairs(summoner.buffList or {}) do
   local mods={};for _,mod in ipairs(buff.modList or {}) do mods[#mods+1]=modRecord(mod) end
   local scaling
   if summoner.activeEffect.grantedEffect.id=="PainOfferingPlayer" and env.minion then
    local source=buff.activeSkillBuff and summoner.skillModList or env.modDB
    local cfg=buff.activeSkillBuff and summoner.skillCfg or nil
    local recipient=env.minion.modDB;local checked=watchStores({source,recipient});local oldCfg=clone(cfg)
    scaling={recipient_profile=env.minion.type,source_store_is_skill=buff.activeSkillBuff or false,source_cfg=sourceConfig(summoner,cfg),
     source_flags=scalars(summoner.skillFlags),source_conditions=scalars(source.conditions),recipient_conditions=scalars(recipient.conditions),
     recipient_occurrence=recipientOccurrence(env,env.minion),recipient_hostile=not not env.minion.hostile,
     source_buff_increased=scalarChannel(source,"INC",cfg,{"BuffEffect"}),source_buff_more=scalarChannel(source,"MORE",cfg,{"BuffEffect"}),
     source_magnitude_increased=scalarChannel(source,"INC",cfg,{"Magnitude"}),source_magnitude_more=scalarChannel(source,"MORE",cfg,{"Magnitude"}),
     recipient_increased=scalarChannel(recipient,"INC",nil,{"BuffEffectOnSelf"}),recipient_more=scalarChannel(recipient,"MORE",nil,{"BuffEffectOnSelf"})}
    checked();assert(equal(cfg,oldCfg))
   end
   buffs[#buffs+1]={fields=scalars(buff),modifiers=mods,scaling=scaling}
  end
  skills[#skills+1]={ordinal=ordinal,effect_id=summoner.activeEffect.grantedEffect.id,effective_level=summoner.activeEffect.level,
   source_occurrence=sourceOccurrence(summoner),
   physical_level=summoner.activeEffect.srcInstance and summoner.activeEffect.srcInstance.level,
   skill_data=scalars(summoner.skillData),flags=scalars(summoner.skillFlags),buffs=buffs,
   offering_supports=physicalDamageOfferingEvidence and summoner.activeEffect.grantedEffect.id=="PainOfferingPlayer" and offeringSupports(summoner) or nil}
  local actor=summoner.minion
  if actor then
   assert(not auth.previous_actors[actor] and not identities[actor]);identities[actor]=true
   local children={};for _,active in ipairs(actor.activeSkillList or {}) do children[#children+1]=consumer(env,actor,active) end
   actors[#actors+1]={ordinal=ordinal,summon_effect_id=summoner.activeEffect.grantedEffect.id,effective_level=summoner.activeEffect.level,
    source_occurrence=sourceOccurrence(summoner),
    physical_level=summoner.activeEffect.srcInstance and summoner.activeEffect.srcInstance.level,quality=summoner.activeEffect.quality,
    actor_level=actor.level,actor_profile=actor.type,profile=scalars(actor.minionData),hidden_damage_fixup=actor.hiddenDamageFixup,
    children=children,fresh_actor=true,hostile=not not actor.hostile,is_environment_minion=actor==env.minion,weapon1=scalars(actor.weaponData1),
    gigantic_benefits=physicalDamageBenefitEvidence and actor.type=="RaisedSkeletonSniper" and {
     original_life_calls=auth.life_calls[actor] or {},actor_output_life=actor.output and actor.output.Life,
     exact_parent=actor.parent==env.player,exact_summoner=summoner.minion==actor} or nil,
    intrinsic_life=physicalDamageIntrinsicLifeEvidence and actor.type=="RaisedSkeletonSniper" and {
     table_selections=auth.life_selections[actor] or {},initializers=auth.life_initializers[actor] or {},
     facts=intrinsicLifeFacts(env,summoner)} or nil,
    life_delivery=physicalDamageLifeDeliveryEvidence and actor.type=="RaisedSkeletonSniper" and {
     transfers=auth.life_transfers[actor] or {},allocated_node_ids=allocatedIds(env)} or nil,
    life_adjustments=physicalDamageLifeAdjustmentEvidence and actor.type=="RaisedSkeletonSniper" and {
     original_life_calls=auth.life_calls[actor] or {},strength_insertions=auth.life_strength_insertions[actor] or {},
     actor_output_life=actor.output and actor.output.Life,exact_parent=actor.parent==env.player,exact_summoner=summoner.minion==actor} or nil}
   actorStates[#actorStates+1]={actor=actor,level=actor.level,weapon=actor.weaponData1,state=clone(actor.weaponData1)}
  end
 end
 return {mode=env.mode,main_group=env.mainSocketGroup,selected_minion=env.minion and recipientOccurrence(env,env.minion),effective=not not env.mode_effective,combat=not not env.mode_combat,buffs_enabled=not not env.mode_buffs,actors=actors,skills=skills,
  offering_merge_events=auth.buff_events[env] or {},
  offering_more_calls=physicalDamageOfferingEvidence and (auth.offering_more_calls[env] or {}) or nil,
  player=consumer(env,env.player,env.player.mainSkill),output=scalars(env.player.output)}
end
local config=build.configTab.configSets[build.configTab.activeConfigSetId]
local function offeringDefinition()
 local effect=assert(data.skills.PainOfferingPlayer)
 local gem=assert(data.gemsByGameId["Metadata/Items/Gems/SkillGemPainOffering"].PainOffering)
 assert(gem.grantedEffect==effect and data.gems[data.gemForSkill[effect]]==gem)
 local beforeEffect,beforeGem=clone(effect),clone(gem)
 local sets={}
 for index,set in ipairs(effect.statSets) do
  local levels={};local keys={};for level in pairs(set.levels) do assert(type(level)=="number");keys[#keys+1]=level end;table.sort(keys)
  for _,level in ipairs(keys) do
   local row=set.levels[level];local values,fields={},{}
   for k,v in pairs(row) do
    if type(k)=="number" then assert(k>=1 and k<=#row and k==math.floor(k));values[k]=clone(v) else fields[k]=clone(v) end
   end
   levels[#levels+1]={level=level,values=values,fields=fields,definition_level=clone(assert(effect.levels[level]))}
  end
  local maps={}
  for _,stat in ipairs(set.stats or {}) do
   local map=rawget(set.statMap,stat)
   if map then local mods={};for _,mod in ipairs(map) do mods[#mods+1]={fields=scalars(mod),modifier=modRecord(mod)} end;maps[#maps+1]={stat=stat,modifiers=mods} end
  end
  local mods={};for _,mod in ipairs(set.baseMods or {}) do mods[#mods+1]=modRecord(mod) end
  sets[#sets+1]={index=index,fields=scalars(set),stats=clone(set.stats),levels=levels,stat_map=maps,base_modifiers=mods,constant_stats=clone(set.constantStats)}
 end
 local result={effect_id=effect.id,physical_gem=scalars(gem),physical_tags=scalars(gem.tags),physical_effect_identity=true,
  skill_types=setRows(effect.skillTypes),minion_skill_types=setRows(effect.minionSkillTypes),quality_stats=clone(effect.qualityStats),
  alternate_quality_stats=clone(effect.altQualityStats),stat_sets=sets}
 assert(equal(effect,beforeEffect) and equal(gem,beforeGem));result.source_tables_preserved=true
 return result
end
local precision={}
for _,name in ipairs({"Damage","PhysicalDamage","AddedDamage","AddedPhysicalDamage","MinPhysicalDamage","MaxPhysicalDamage"}) do
 local source=data.highPrecisionMods[name]
 precision[#precision+1]={name=name,present=source~=nil,types=source and scalars(source) or {}}
end
local family={}
-- Reviewed default-node candidates, observed independently of allocation. These
-- IDs are source-test cases, never production dispatch or inferred coverage.
if mainEnv.spec.treeVersion=="0_5" then
 for _,id in ipairs({95,229,752,762,1447,3443,3723,4345,4725,8737,8983,9065,11048,14033,14598,16413,17501,19006,22393,22783,28458,33612,34493,36286,37594,39461,40200,41130,43979,47155,48565,49593,50720,50837,54036,54453,54964,61842,63545,64653,65328}) do
  local raw=assert(mainEnv.spec.tree.nodes[id]);local node=assert(mainEnv.spec.nodes[id]);local mods={}
  for _,mod in ipairs(raw.modList or {}) do mods[#mods+1]=modRecord(mod) end
  family[#family+1]={id=id,name=raw.name,stats=clone(raw.stats),modifiers=mods,allocated=mainEnv.spec.allocNodes[id]~=nil,
   effective_same_definition=node==raw,effective_name=node.name}
 end
end
local result={selected=selection,main=environment(mainEnv),calcs=environment(calcsEnv),main_output=scalars(mainOutput),calcs_output=scalars(calcsOutput),
 offering_definition=offeringDefinition(),
 config={custom_blocks=clone(config.customModsList)},modifier_precision={default=data.defaultHighPrecision,overrides=precision},
 plain_minion_damage_family=family,observed_offence_count=auth.count,bounded_caller_line_events=auth.line_events}
if physicalDamageOfferingEvidence then
 result.offering_output_snapshot=offeringOutputSnapshot()
 local support=assert(data.skills.SupportDanseMacabrePlayer)
 local set=assert(support.statSets[1]);local maps={}
 for _,name in ipairs({"offering_spells_effect_+%_if_consumed_additional_skeleton","support_danse_macabre_offering_skill_damage_+%_final_if_consumed_additional_skeleton"}) do
  local mods={};for _,mod in ipairs(assert(rawget(set.statMap,name))) do mods[#mods+1]=modRecord(mod) end
  maps[#maps+1]={stat=name,modifiers=mods}
 end
 result.offering_support_definition={effect_id=support.id,name=support.name,description=support.description,
  constant_stats=clone(set.constantStats),stat_map=maps}
 local inventory={}
 for _,group in ipairs(build.skillsTab.socketGroupList) do for _,gem in ipairs(group.gemList) do
  if gem.grantedEffect and gem.grantedEffect.id=="SupportDanseMacabrePlayer" then
   inventory[#inventory+1]=sourceOccurrence({activeEffect={srcInstance=gem},socketGroup=group})
  end
 end end
 result.offering_support_inventory=inventory
end
if physicalDamageBenefitEvidence then result.benefit_snapshot=benefitSnapshot() end
if physicalDamageIntrinsicLifeEvidence then
 result.intrinsic_life_definitions=intrinsicLifeDefinitions(mainEnv)
 assert(equal(result.intrinsic_life_definitions,auth.intrinsic_definitions))
 assert(equal(result.intrinsic_life_definitions,intrinsicLifeDefinitions(calcsEnv)))
 assert(upvalue(calcs.perform,"initMinionModDB")==auth.init_minion)
 result.intrinsic_life_methods_preserved=true
end
if physicalDamageLifeDeliveryEvidence then
 assert(upvalue(calcs.perform,"addMinionModifiers")==auth.transfer_minion)
 result.life_delivery_methods_preserved=true
end
if physicalDamageLifeAdjustmentEvidence then
 assert(upvalue(calcs.perform,"doActorAttribsConditions")==auth.actor_attributes)
 result.life_adjustment_methods_preserved=true
end
if physicalDamageCommandEvidence then
 local commandFamily={}
 assert(mainEnv.spec.treeVersion=="0_5")
 -- This is the complete observed selected CooldownRecovery producer census,
 -- not just the four INC8 nodes being considered for a later native family.
 for _,id in ipairs({4345,6077,14598,14945,35645,43979,50837}) do
  local raw=assert(mainEnv.spec.tree.nodes[id]);local node=assert(mainEnv.spec.nodes[id]);local mods={}
  for _,modifier in ipairs(raw.modList or {}) do mods[#mods+1]=modRecord(modifier) end
  commandFamily[#commandFamily+1]={id=id,name=raw.name,string_id=raw.stringId,stats=clone(raw.stats),modifiers=mods,
   allocated=mainEnv.spec.allocNodes[id]~=nil,effective_same_definition=node==raw,effective_name=node.name}
 end
 result.command_cooldown_family=commandFamily
end
for _,row in ipairs(actorStates) do assert(row.actor.level==row.level and row.actor.weaponData1==row.weapon and equal(row.actor.weaponData1,row.state)) end
assert(equal(build.itemsTab.items,savedItems) and equal(build.itemsTab.itemSets,savedSets) and equal(build.skillsTab.skillSets,savedSkills) and equal(build.configTab.configSets,savedConfig))
assert(equal(keys(build.treeTab.specList),savedSpecKeys))
for i,saved in ipairs(savedSpecs) do
 local spec=build.treeTab.specList[i];assert(spec==saved.object and spec.allocNodes==saved.nodes and equal(spec.jewels,saved.jewels))
 local actual=allocations(spec);assert(equal(keys(actual),keys(saved.allocations)))
 for id,node in pairs(actual) do local prior=saved.allocations[id];assert(node.object==prior.object and node.id==prior.id and node.alloc==prior.alloc and node.mode==prior.mode) end
end
assert(equal(mainOutput,savedMain) and equal(calcsOutput,savedCalcs) and equal(selected(),selection))
assert(build.calcsTab.mainEnv==mainEnv and build.calcsTab.calcsEnv==calcsEnv and build.calcsTab.mainOutput==mainOutput and build.calcsTab.calcsOutput==calcsOutput)
for i,row in ipairs(methods) do assert(row[1][row[2]]==auth.refs[i]) end
assert(upvalue(calcs.offence,"calcDamage")==auth.calc_damage and upvalue(calcs.perform,"mergeBuff")==auth.merge_buff)
if physicalDamageCommandEvidence then assert(calcSkillCooldown==auth.cooldown);result.command_receiving_methods_preserved=true end
result.original_functions_preserved=true;result.loaded_state_preserved=true;result.cached_outputs_preserved=true
result.saved_specs_preserved=true;result.fresh_actor_construction=true;result.query_state_preserved=true
result.source_actor_level_mutated=false;result.business_method_wrappers=false
return result
