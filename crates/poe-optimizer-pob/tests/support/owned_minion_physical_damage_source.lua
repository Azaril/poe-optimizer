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
 return {name=mod.name,type=mod.type,value=type(mod.value)=="table" and clone(mod.value) or scalar(mod.value),source=mod.source,flags=mod.flags,keyword_flags=mod.keywordFlags,tags=tags}
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
if physicalDamagePhase=="before" then
 local refs={};for i,row in ipairs(methods) do refs[i]=original(row[1][row[2]],row[3],row[4]) end
 local calcDamage=original(upvalue(calcs.offence,"calcDamage"),"Modules/CalcOffence.lua",178)
 local mergeBuff=original(upvalue(calcs.perform,"mergeBuff"),"Modules/CalcPerform.lua",42)
 local priorActors={}
 for _,env in ipairs({build.calcsTab.mainEnv,build.calcsTab.calcsEnv}) do if env then for _,active in ipairs(env.player.activeSkillList) do if active.minion then priorActors[active.minion]=true end end end end
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local auth={refs=refs,calc_damage=calcDamage,merge_buff=mergeBuff,previous_actors=priorActors,captures={},calls={},base_calls={},buff_events={},count=0,line_events=0}
 local pending
 local function relevant(active)
  return active and active.actor and active.actor.minionData and active.activeEffect.grantedEffect.id=="MinionMeleeBow"
 end
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  if f~=calcDamage and f~=calcs.offence and f~=mergeBuff and f~=calcLib.mod then return end
  if event~="return" and not (event=="line" and pending and f==calcs.offence) then return end
  local vars={};for i=1,160 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
  if f==calcLib.mod then
   local callerInfo=debug.getinfo(3,"fl")
   if callerInfo and callerInfo.func==calcs.offence and callerInfo.currentline==4134 then
    local caller={};for i=1,160 do local name,value=debug.getlocal(3,i);if not name then break end;caller[name]=value end
    if relevant(caller.activeSkill) then
     assert(not pending);pending={kind="base",active=caller.activeSkill,cfg=caller.cfg};debug.sethook(hook,"rl")
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
   pending={kind="damage",active=active,cfg=cfg,row=row};debug.sethook(hook,"rl")
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
     pending=nil;debug.sethook(hook,"r")
    end
   elseif line==4216 then
    pending.row.returned_min=vars.damageTypeHitMin;pending.row.returned_max=vars.damageTypeHitMax
    pending.row.return_observed_at=line
   elseif line==4257 then
    local row=pending.row;assert(row.return_observed_at==4216)
    assert(vars.damageTypeHitMin==row.returned_min and vars.damageTypeHitMax==row.returned_max)
    row.later_all_mult=vars.allMult;row.general_all_mult=vars.output.allMult;row.all_mult_observed_at=line
    auth.calls[active]=auth.calls[active] or {};table.insert(auth.calls[active],row)
    pending=nil;debug.sethook(hook,"r")
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
 local enabled=jit.status();jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"r")
 physicalDamageAuth=auth
 return function()
  assert(debug.gethook()==hook and not pending);debug.sethook(oldHook,oldMask,oldCount);assert(jit.status()==enabled)
  for i,row in ipairs(methods) do assert(row[1][row[2]]==refs[i]) end
  assert(upvalue(calcs.offence,"calcDamage")==calcDamage and upvalue(calcs.perform,"mergeBuff")==mergeBuff);auth.finished=true
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
   skill_data=scalars(summoner.skillData),flags=scalars(summoner.skillFlags),buffs=buffs}
  local actor=summoner.minion
  if actor then
   assert(not auth.previous_actors[actor] and not identities[actor]);identities[actor]=true
   local children={};for _,active in ipairs(actor.activeSkillList or {}) do children[#children+1]=consumer(env,actor,active) end
   actors[#actors+1]={ordinal=ordinal,summon_effect_id=summoner.activeEffect.grantedEffect.id,effective_level=summoner.activeEffect.level,
    source_occurrence=sourceOccurrence(summoner),
    physical_level=summoner.activeEffect.srcInstance and summoner.activeEffect.srcInstance.level,quality=summoner.activeEffect.quality,
    actor_level=actor.level,actor_profile=actor.type,profile=scalars(actor.minionData),hidden_damage_fixup=actor.hiddenDamageFixup,
    children=children,fresh_actor=true,hostile=not not actor.hostile,is_environment_minion=actor==env.minion,weapon1=scalars(actor.weaponData1)}
   actorStates[#actorStates+1]={actor=actor,level=actor.level,weapon=actor.weaponData1,state=clone(actor.weaponData1)}
  end
 end
 return {mode=env.mode,main_group=env.mainSocketGroup,selected_minion=env.minion and recipientOccurrence(env,env.minion),effective=not not env.mode_effective,combat=not not env.mode_combat,buffs_enabled=not not env.mode_buffs,actors=actors,skills=skills,
  offering_merge_events=auth.buff_events[env] or {},
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
result.original_functions_preserved=true;result.loaded_state_preserved=true;result.cached_outputs_preserved=true
result.saved_specs_preserved=true;result.fresh_actor_construction=true;result.query_state_preserved=true
result.source_actor_level_mutated=false;result.business_method_wrappers=false
return result
