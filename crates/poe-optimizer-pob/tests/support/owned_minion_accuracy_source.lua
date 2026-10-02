-- Read-only observation of complete original loads; no calculation wrappers.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local info = debug.getinfo(f, "S")
 local actual = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line, path .. ":" .. line)
 return f
end
local function scalar(v)
 if type(v) == "number" and (v ~= v or v == math.huge or v == -math.huge) then return tostring(v) end
 return v
end
local function scalars(t)
 local out = {}
 for k, v in pairs(t or {}) do
  if type(v) == "number" or type(v) == "string" or type(v) == "boolean" then out[k] = scalar(v) end
 end
 return out
end
local function clone(v, seen)
 if type(v) ~= "table" then return v end
 seen = seen or {}; if seen[v] then return seen[v] end
 local out = {}; seen[v] = out
 for k, e in pairs(v) do out[k] = clone(e, seen) end
 return out
end
local function equal(a, b, seen)
 if type(a) ~= type(b) then return false end
 if type(a) ~= "table" then return a == b end
 seen = seen or {}; if seen[a] then return seen[a] == b end; seen[a] = b
 for k, v in pairs(a) do if not equal(v, b[k], seen) then return false end end
 for k in pairs(b) do if a[k] == nil then return false end end
 return true
end
local function modRecord(mod)
 local tags = {}; for _, tag in ipairs(mod) do tags[#tags + 1] = clone(tag) end
 return {name=mod.name, type=mod.type, value=scalar(mod.value), source=mod.source,
  flags=mod.flags, keyword_flags=mod.keywordFlags, tags=tags}
end
local function records(store, kind, cfg, name)
 local out = {}
 for _, entry in ipairs(store:Tabulate(kind, cfg, name)) do
  out[#out + 1] = {value=scalar(entry.value), mod=modRecord(entry.mod)}
 end
 return out
end
local function rawRecords(store, kind, name)
 local out = {}
 -- Tabulate intentionally drops zero BASE values. Configuration provenance
 -- instead comes from actual stored records, preserving explicit zero.
 for _, mod in ipairs(store) do
  if mod.type == kind and mod.name == name then out[#out + 1] = {value=scalar(mod.value),mod=modRecord(mod)} end
 end
 return out
end
-- Clone owned fields, preserving actor/parent references as identities. Queries
-- may follow them, so every reachable parent plus actor outputs is checked too.
local function watchStores(stores)
 local seen, saved = {}, {}
 local function add(store)
  if not store or seen[store] then return end; seen[store] = true
  local owned = {}; for k, v in pairs(store) do if k ~= "actor" and k ~= "parent" then owned[k] = v end end
  saved[#saved + 1] = {store=store, actor=store.actor, parent=store.parent, meta=getmetatable(store), owned=clone(owned), output=store.actor and clone(store.actor.output)}
  add(store.parent)
 end
 for _, store in ipairs(stores) do add(store) end
 return function()
  for _, row in ipairs(saved) do
   assert(row.store.actor == row.actor and row.store.parent == row.parent and getmetatable(row.store) == row.meta)
   local owned = {}; for k, v in pairs(row.store) do if k ~= "actor" and k ~= "parent" then owned[k] = v end end
   assert(equal(owned, row.owned), "observation changed modifier store")
   assert(equal(row.actor and row.actor.output, row.output), "observation changed actor output")
  end
 end
end
local methodRows = {
 {common.classes.ConfigTab,"Load","Classes/ConfigTab.lua",878},
 {common.classes.ConfigTab,"BuildModList","Classes/ConfigTab.lua",1169},
 {common.classes.ConfigTab,"CreateConfigSet","Classes/ConfigTab.lua",1317},
 {common.classes.SkillsTab,"LoadSkill","Classes/SkillsTab.lua",303},
 {common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {calcs,"initEnv","Modules/CalcSetup.lua",717},
 {calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
 {calcs,"createMinionSkills","Modules/CalcActiveSkill.lua",1116},
 {calcs,"perform","Modules/CalcPerform.lua",1193},
 {calcs,"offence","Modules/CalcOffence.lua",527},
 {calcs,"hitChance","Modules/CalcDefence.lua",33},
 {calcLib,"mod","Modules/CalcTools.lua",16},
 {calcLib,"mods","Modules/CalcTools.lua",26},
 {calcLib,"val","Modules/CalcTools.lua",50},
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"More","Classes/ModStore.lua",261},
 {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModStore,"EvalMod","Classes/ModStore.lua",490},
}
local function passObservation(vars, pass)
 local env, actor, active = vars.env, vars.actor, vars.activeSkill
 local store, enemy, cfg = vars.skillModList, actor.enemy.modDB, pass.cfg
 local checked = watchStores({store, actor.modDB, enemy, env.modDB, env.player.modDB})
 local oldCfg, oldOutput, oldSource = clone(cfg), clone(pass.output), clone(pass.source)
 local chain, parent = false, store
 while parent do if parent == actor.modDB then chain = true end; parent = parent.parent end
 local accuracy = {
  base=store:Sum("BASE",cfg,"Accuracy"), base_vs_enemy=store:Sum("BASE",cfg,"Accuracy","AccuracyVsEnemy"),
  increased=store:Sum("INC",cfg,"Accuracy"), increased_vs_enemy=store:Sum("INC",cfg,"Accuracy","AccuracyVsEnemy"),
  -- These deliberately preserve the original offence function's argument order.
  more=store:More("MORE",cfg,"Accuracy"), more_vs_enemy=store:More("MORE",cfg,"Accuracy","AccuracyVsEnemy"),
  penalty_multiplier=calcLib.mod(store,cfg,"AccuracyPenalty"), hit_chance_multiplier=calcLib.mod(store,cfg,"HitChance"),
  enemy_evasion_raw=calcLib.val(enemy,"Evasion"), enemy_evasion=math.max(round(calcLib.val(enemy,"Evasion")),0),
  distance_sum=env.modDB:Sum("BASE",nil,"Multiplier:enemyDistance"), skill_distance=cfg.skillDist,
 }
 local flags = {attack=not not vars.isAttack, skill_cannot_be_evaded=not not store:Flag(cfg,"CannotBeEvaded"),
  actor_cannot_be_evaded=not not actor.modDB:Flag(cfg,"CannotBeEvaded"), skill_data_cannot_be_evaded=not not vars.skillData.cannotBeEvaded,
  enemy_cannot_evade=not not enemy:Flag(nil,"CannotEvade"), enemy_cannot_block_attacks=not not enemy:Flag(nil,"CannotBlockAttacks"),
  player_minion_accuracy_equals_accuracy=not not env.player.modDB:Flag(nil,"MinionAccuracyEqualsAccuracy"),
  no_accuracy_distance_penalty=not not actor.modDB:Flag(cfg,"NoAccuracyDistancePenalty"),
  offhand_accuracy_is_main=not not store:Flag(nil,"Condition:OffHandAccuracyIsMainHandAccuracy"),
  hit_chance_can_exceed_100=not not store:Flag(cfg,"Condition:HitChanceCanExceed100")}
 local result = {label=pass.label, cfg=scalars(cfg), skill_conditions=scalars(cfg.skillCond), source=scalars(pass.source),
  output=scalars(pass.output), accuracy=accuracy, flags=flags, mode=env.mode, effective=not not env.mode_effective,
  block={base=enemy:Sum("BASE",cfg,"BlockChance"), reduction=store:Sum("BASE",cfg,"reduceEnemyBlock"),
   base_records=records(enemy,"BASE",cfg,"BlockChance"), reduction_records=records(store,"BASE",cfg,"reduceEnemyBlock"),
   cannot_block_records=records(enemy,"FLAG",nil,"CannotBlockAttacks")},
  flag_records=records(store,"FLAG",cfg,"CannotBeEvaded"), actor_flag_records=records(actor.modDB,"FLAG",cfg,"CannotBeEvaded"),
  inheritance_records=records(env.player.modDB,"FLAG",nil,"MinionAccuracyEqualsAccuracy"),
  skill_inherits_actor_modifiers=chain, query_state_preserved=true}
 checked(); assert(equal(cfg,oldCfg) and equal(pass.output,oldOutput) and equal(pass.source,oldSource))
 return result
end
if minionAccuracyPhase == "before" then
 local methods = {}; for index,row in ipairs(methodRows) do methods[index] = original(row[1][row[2]],row[3],row[4]) end
 local priorActors = {}
 for _, env in ipairs({build.calcsTab.mainEnv, build.calcsTab.calcsEnv}) do
  if env then for _, active in ipairs(env.player.activeSkillList) do if active.minion then priorActors[active.minion] = true end end end
 end
 local oldHook, oldMask, oldCount = debug.gethook(); assert(oldHook == nil)
 local captures, count = {}, 0
 local function hook(event)
  if event ~= "return" or debug.getinfo(2,"f").func ~= calcs.offence then return end
  local vars = {}; for index=1,128 do local name,value = debug.getlocal(2,index); if not name then break end; vars[name] = value end
  if not vars.actor or not vars.activeSkill then return end
  local passes = {}; for _,pass in ipairs(vars.passList or {}) do passes[#passes + 1] = passObservation(vars,pass) end
  captures[vars.activeSkill] = {actor=vars.actor, mode=vars.env.mode, passes=passes}; count=count+1
 end
 local enabled=jit.status(); jit.flush(); assert(jit.status()==enabled); debug.sethook(hook,"r")
 minionAccuracyAuth={methods=methods,previous_actors=priorActors,captures=captures}
 return function()
  assert(debug.gethook()==hook); debug.sethook(oldHook,oldMask,oldCount); assert(jit.status()==enabled)
  for index,row in ipairs(methodRows) do assert(row[1][row[2]]==methods[index]) end
  minionAccuracyAuth.finished=true; minionAccuracyAuth.count=count
 end
end
local auth=assert(minionAccuracyAuth); assert(auth.finished and auth.count>0)
local mainEnv, calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local mainOutput, calcsOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local savedMain,savedCalcs=clone(mainOutput),clone(calcsOutput)
local savedItems,savedSets,savedSkills=clone(build.itemsTab.items),clone(build.itemsTab.itemSets),clone(build.skillsTab.skillSets)
local savedConfig=clone(build.configTab.configSets)
local function keys(t)
 local out={};for k in pairs(t) do out[k]=true end;return out
end
local function allocations(spec)
 local out={}
 for id,node in pairs(spec.allocNodes) do out[id]={object=node,id=node.id,alloc=node.alloc,mode=node.allocMode} end
 return out
end
local savedSpecs,savedSpecKeys={},keys(build.treeTab.specList)
for i,spec in ipairs(build.treeTab.specList) do
 savedSpecs[i]={object=spec,nodes=spec.allocNodes,allocations=allocations(spec),jewels=clone(spec.jewels)}
end
local function selected()
 return {items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,
  config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup}
end
local selection=selected()
local function consumer(env,actor,active)
 local observed=auth.captures[active]
 if observed then assert(observed.actor==actor and observed.mode==env.mode) end
 return {effect_id=active.activeEffect.grantedEffect.id,effect_name=active.activeEffect.grantedEffect.name,
  selected=actor.mainSkill==active,flags=scalars(active.skillFlags),output=scalars(active.output),
  passes=observed and observed.passes or nil}
end
local actorStates={}
local function environment(env)
 local actors,identities={},{}
 for ordinal,summoner in ipairs(env.player.activeSkillList) do
  local actor=summoner.minion
  if actor then
   assert(not auth.previous_actors[actor] and not identities[actor]); identities[actor]=true
   local children={}; for _,active in ipairs(actor.activeSkillList or {}) do children[#children+1]=consumer(env,actor,active) end
   actors[#actors+1]={ordinal=ordinal,summon_effect_id=summoner.activeEffect.grantedEffect.id,
    effective_level=summoner.activeEffect.level,physical_level=summoner.activeEffect.srcInstance and summoner.activeEffect.srcInstance.level,
    actor_level=actor.level,actor_profile=actor.type,children=children,fresh_actor=true,weapon1=scalars(actor.weaponData1)}
   actorStates[#actorStates+1]={actor=actor,level=actor.level,weapon=actor.weaponData1,state=clone(actor.weaponData1)}
  end
 end
 return {main_effect=env.player.mainSkill.activeEffect.grantedEffect.id,mode=env.mode,effective=not not env.mode_effective,
  actors=actors,player=consumer(env,env.player,env.player.mainSkill),output=scalars(env.player.output)}
end
local config=build.configTab.configSets[build.configTab.activeConfigSetId]
local function raw(name)
 return {input_present=config.input[name]~=nil,input=scalar(config.input[name]),placeholder_present=config.placeholder[name]~=nil,placeholder=scalar(config.placeholder[name])}
end
local configCheck=watchStores({build.configTab.modList,build.configTab.enemyModList})
local result={selected=selection,main=environment(mainEnv),calcs=environment(calcsEnv),
 main_output=scalars(mainOutput),calcs_output=scalars(calcsOutput),
 config={enemy_block=raw("enemyBlockChance"),enemy_distance=raw("enemyDistance"),
  block_records=rawRecords(build.configTab.enemyModList,"BASE","BlockChance"),
  distance_records=rawRecords(build.configTab.modList,"BASE","Multiplier:enemyDistance"),
  custom_blocks=clone(config.customModsList)},
 constants={falloff_start=mainEnv.data.misc.AccuracyFalloffStart,falloff_end=mainEnv.data.misc.AccuracyFalloffEnd,max_penalty=mainEnv.data.misc.MaxAccuracyRangePenalty}}
configCheck()
for _,row in ipairs(actorStates) do assert(row.actor.level==row.level and row.actor.weaponData1==row.weapon and equal(row.actor.weaponData1,row.state)) end
assert(equal(build.itemsTab.items,savedItems) and equal(build.itemsTab.itemSets,savedSets) and equal(build.skillsTab.skillSets,savedSkills) and equal(build.configTab.configSets,savedConfig))
assert(equal(keys(build.treeTab.specList),savedSpecKeys))
for i,saved in ipairs(savedSpecs) do
 local spec=build.treeTab.specList[i]
 assert(spec==saved.object and spec.allocNodes==saved.nodes and equal(spec.jewels,saved.jewels))
 local actual=allocations(spec);assert(equal(keys(actual),keys(saved.allocations)))
 for id,node in pairs(actual) do
  local prior=saved.allocations[id]
  assert(node.object==prior.object and node.id==prior.id and node.alloc==prior.alloc and node.mode==prior.mode)
 end
end
assert(equal(mainOutput,savedMain) and equal(calcsOutput,savedCalcs) and equal(selected(),selection))
assert(build.calcsTab.mainEnv==mainEnv and build.calcsTab.calcsEnv==calcsEnv and build.calcsTab.mainOutput==mainOutput and build.calcsTab.calcsOutput==calcsOutput)
for index,row in ipairs(methodRows) do assert(row[1][row[2]]==auth.methods[index]) end
result.original_functions_preserved=true; result.loaded_state_preserved=true; result.cached_outputs_preserved=true
result.fresh_actor_construction=true; result.source_actor_level_mutated=false; result.business_method_wrappers=false
result.query_state_preserved=true
result.saved_specs_preserved=true
return result
