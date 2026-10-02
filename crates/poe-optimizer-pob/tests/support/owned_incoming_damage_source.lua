-- Read-only observation of the complete original Build lifecycle. No game method is replaced.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local info = debug.getinfo(f, "S")
 local actual = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line,
  "unexpected source "..actual..":"..info.linedefined.." expected "..path..":"..line)
 return f
end
local function plain(value, depth)
 local kind = type(value)
 if kind == "number" then
  return (value ~= value or value == math.huge or value == -math.huge) and tostring(value) or value
 end
 if kind ~= "table" then
  assert(kind == "nil" or kind == "string" or kind == "boolean", "unexpected evidence type "..kind)
  return value
 end
 depth = (depth or 0) + 1; assert(depth <= 24)
 local out, count, maximum, array = {}, 0, 0, true
 for key in pairs(value) do
  count=count+1;assert(count<=30000)
  if type(key)=="number" and key>=1 and key==math.floor(key) then maximum=math.max(maximum,key) else array=false end
 end
 array=array and count==maximum
 for key, child in pairs(value) do
  assert(type(key) == "string" or type(key) == "number")
  out[array and key or tostring(key)] = plain(child, depth)
 end
 return out
end
local function scalars(value)
 local out = {}
 for key, child in pairs(value or {}) do
  if type(child) == "number" or type(child) == "string" or type(child) == "boolean" then out[key] = plain(child) end
 end
 return out
end
local function clone(value, seen)
 if type(value) ~= "table" then return value end
 seen = seen or {}; if seen[value] then return seen[value] end
 local out = {}; seen[value] = out
 for key, child in pairs(value) do out[key] = clone(child, seen) end
 return out
end
local function equal(a, b, seen)
 if type(a) ~= type(b) then return false end
 if type(a) == "number" and a ~= a then return b ~= b end
 if type(a) ~= "table" then return a == b end
 seen = seen or {}; if seen[a] then return seen[a] == b end; seen[a] = b
 for key, value in pairs(a) do if not equal(value, b[key], seen) then return false end end
 for key in pairs(b) do if a[key] == nil then return false end end
 return true
end
local damageTypes = {"Physical", "Lightning", "Cold", "Fire", "Chaos"}
local statNames = {}
for _, kind in ipairs(damageTypes) do statNames[kind.."Min"] = true; statNames[kind.."Max"] = true end
local boss
for _, row in ipairs(require("Modules.ConfigOptions")) do if row.var == "enemyIsBoss" then assert(not boss); boss = row.apply end end
assert(boss)
local constructor
for index=1,128 do
 local name,value=debug.getupvalue(common.classes.ConfigTab.ConfigTab,index)
 if not name then break end
 if name=="originalFunc" then constructor=original(value,"Classes/ConfigTab.lua",134) end
end
assert(constructor)
local methods = {
 {calcs, "buildDefenceEstimations", "Modules/CalcDefence.lua", 2207},
 {common.classes.ConfigTab, "Load", "Classes/ConfigTab.lua", 878},
 {common.classes.ConfigTab, "UpdateLevel", "Classes/ConfigTab.lua", 1157},
 {common.classes.ConfigTab, "BuildModList", "Classes/ConfigTab.lua", 1169},
 {common.classes.EditControl, "SetPlaceholder", "Classes/EditControl.lua", 110},
 {common.classes.CalcsTab, "BuildOutput", "Classes/CalcsTab.lua", 486},
 {common.classes.ModDB, "SumInternal", "Classes/ModDB.lua", 137},
 {common.classes.ModStore, "Sum", "Classes/ModStore.lua", 202},
 {_G, "round", "Modules/Common.lua", 722},
}
local function membership(db, name)
 local out, seen, depth = {}, {}, 0
 while db do
  depth = depth + 1; assert(depth <= 16 and not seen[db]); seen[db] = true
  for index, mod in ipairs(db.mods[name] or {}) do
   out[#out+1] = {depth=depth,index=index,record=plain(mod)}
  end
  db = db.parent
 end
 return out
end
if incomingDamagePhase == "before" then
 local refs = {}; for index, row in ipairs(methods) do refs[index] = original(row[1][row[2]], row[3], row[4]) end
 original(boss, "Modules/ConfigOptions.lua", 1982)
 local oldHook, oldMask, oldCount = debug.gethook(); assert(oldHook == nil)
 local enabled = jit.status(); assert(enabled == incomingDamageJit)
 local auth = {refs=refs,boss=boss,views={},callbacks={},constructors={},load_entries={},defence_calls=0}
 local defenceFrame, callbackFrames = nil, {}
 local function hook(event, line)
  local f = debug.getinfo(2, "f").func
  if f ~= refs[1] and f ~= refs[2] and f ~= refs[7] and f ~= boss and f ~= constructor then return end
  local vars = {}
  for index=1,128 do local name,value = debug.getlocal(2,index); if not name then break end; vars[name] = value end
  if f == constructor and event=="return" then
   auth.constructors[#auth.constructors+1]={input=scalars(vars.self.input),placeholder=scalars(vars.self.placeholder),default_state=scalars(vars.self.defaultState)}
  elseif f == refs[2] and event=="call" then
   auth.load_entries[#auth.load_entries+1]={input=scalars(vars.self.input),placeholder=scalars(vars.self.placeholder),default_state=scalars(vars.self.defaultState)}
  elseif f == boss then
   if event == "call" then
    callbackFrames[#callbackFrames+1] = {value=vars.val,build=vars.build}
    debug.sethook(hook, "crl")
   elseif event == "line" and line == 2085 then
    local frame = assert(callbackFrames[#callbackFrames]); assert(frame.value == "Pinnacle")
    frame.defaults = {level=vars.defaultLevel,damage=vars.defaultDamage,
     monster_damage=build.data.monsterDamageTable[vars.defaultLevel],
     multiplier=build.data.misc.pinnacleBossDPSMult,penetration=build.data.misc.pinnacleBossPen}
   elseif event == "return" then
    local frame = assert(table.remove(callbackFrames))
    if frame.value == "Pinnacle" then
     assert(frame.defaults)
     local cfg = frame.build.configTab
     auth.callbacks[#auth.callbacks+1] = {defaults=frame.defaults,enemy_level=cfg.enemyLevel,
      active=cfg.activeConfigSetId,input=scalars(cfg.input),placeholder=scalars(cfg.placeholder)}
    end
    if not defenceFrame and #callbackFrames == 0 then debug.sethook(hook, "cr") end
   end
  elseif f == refs[1] then
   if event == "call" and vars.actor == vars.env.player then
    assert(not defenceFrame)
    defenceFrame = {env=vars.env,actor=vars.actor,rows={},queries={}}
    debug.sethook(hook, "crl")
   elseif event == "line" and defenceFrame and vars.actor == defenceFrame.actor and line == 2339 then
    local frame = defenceFrame
    local minimum, maximum = assert(frame.queries[vars.damageType.."Min"]), assert(frame.queries[vars.damageType.."Max"])
    assert(vars.enemyCfg.keywordFlags == NOT64(KeywordFlag.MatchAll))
    frame.rows[#frame.rows+1] = {damage_type=vars.damageType,source=vars.sourceStr,
     raw_input=plain(vars.env.configInput["enemy"..vars.damageType.."Damage"]),
     current_placeholder=plain(vars.env.configPlaceholder["enemy"..vars.damageType.."Damage"]),
     minimum=minimum,maximum=maximum,preconversion_amount=vars.enemyDamage,
     total_before=vars.output.totalEnemyDamageIn,penetration=vars.enemyPen,
     query_cfg=plain(vars.enemyCfg)}
    frame.queries = {}
   elseif event == "return" and defenceFrame and vars.actor == defenceFrame.actor then
    local frame = defenceFrame; auth.defence_calls = auth.defence_calls + 1
    auth.views[frame.env] = {mode=frame.env.mode,category=vars.damageCategoryConfig,
     rows=frame.rows,total_in=frame.actor.output.totalEnemyDamageIn,
     config_input=scalars(frame.env.configInput),config_placeholder=scalars(frame.env.configPlaceholder),
     output=scalars(frame.actor.output),actor=frame.actor}
    defenceFrame = nil
    if #callbackFrames == 0 then debug.sethook(hook, "cr") end
   end
  elseif event == "return" and defenceFrame and vars.self == defenceFrame.actor.enemy.modDB
   and vars.modType == "BASE" and statNames[vars.modName]
   and vars.cfg and vars.cfg.keywordFlags == NOT64(KeywordFlag.MatchAll) then
   assert(type(vars.result) == "number")
   defenceFrame.queries[vars.modName] = {name=vars.modName,operation=vars.modType,
    cfg=plain(vars.cfg),flags=vars.flags,keyword_flags=vars.keywordFlags,
    result=vars.result,bucket_membership=membership(vars.self,vars.modName)}
  end
 end
 jit.flush(); assert(jit.status() == enabled); debug.sethook(hook, "cr")
 incomingDamageAuth = auth
 return function()
  assert(debug.gethook() == hook)
  debug.sethook(oldHook, oldMask, oldCount)
  auth.incomplete = defenceFrame ~= nil or #callbackFrames ~= 0
  assert(jit.status() == enabled)
  for index,row in ipairs(methods) do assert(row[1][row[2]] == refs[index]) end
  auth.finished = true
 end
end
assert(incomingDamagePhase == "after")
local auth = assert(incomingDamageAuth); assert(auth.finished and not auth.incomplete and #auth.callbacks > 0)
for index,row in ipairs(methods) do assert(row[1][row[2]] == auth.refs[index]) end
assert(auth.boss == boss)
local tabs = {build.itemsTab.items,build.itemsTab.itemSets,build.skillsTab.skillSets,build.configTab.configSets}
local saved = {}; for index,value in ipairs(tabs) do saved[index] = clone(value) end
local mainOutput, calcsOutput = build.calcsTab.mainOutput, build.calcsTab.calcsOutput
local savedMain, savedCalcs = clone(mainOutput), clone(calcsOutput)
local specs = {}
for index,spec in ipairs(build.treeTab.specList) do
 specs[index] = {object=spec,nodes=spec.allocNodes,allocations={}}
 for id,node in pairs(spec.allocNodes) do specs[index].allocations[id] = {object=node,alloc=node.alloc,mode=node.allocMode} end
end
local selection = {items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,
 config=build.configTab.activeConfigSetId,spec=build.treeTab.activeSpec,main_group=build.mainSocketGroup}
local function view(env, output)
 local captured = assert(auth.views[env], "missing final original defence invocation")
 assert(captured.actor == env.player and captured.total_in == env.player.output.totalEnemyDamageIn)
 local level = math.max(build.configTab.enemyLevel, 82)
 local damage = round(build.data.monsterDamageTable[level] * 1.5 * build.data.misc.pinnacleBossDPSMult)
 local expected = {Physical=damage,Lightning=damage,Cold=damage,Fire=damage,Chaos=round(damage/2.5)}
 assert(env.configInput.enemyIsBoss == "Pinnacle")
 for _,kind in ipairs(damageTypes) do assert(env.configPlaceholder["enemy"..kind.."Damage"] == expected[kind]) end
 for _,kind in ipairs({"Lightning","Cold","Fire"}) do assert(env.configPlaceholder["enemy"..kind.."Pen"] == build.data.misc.pinnacleBossPen) end
 local total = 0
 for index,row in ipairs(captured.rows) do
  assert(row.damage_type == damageTypes[index] and row.total_before == total)
  local value = tonumber(env.configInput["enemy"..row.damage_type.."Damage"])
  if value == nil then value = tonumber(env.configPlaceholder["enemy"..row.damage_type.."Damage"]) or 0 end
  assert(row.preconversion_amount == value + (row.minimum.result + row.maximum.result) / 2)
  total = total + row.preconversion_amount
  assert(env.player.output[row.damage_type.."EnemyPen"] == row.penetration)
 end
 if captured.category == "DamageOverTime" then assert(#captured.rows == 0 and captured.total_in == 0)
 else assert(#captured.rows == 5 and total == captured.total_in) end
 return {mode=captured.mode,category=captured.category,rows=captured.rows,total_in=captured.total_in,
  input=captured.config_input,placeholder=captured.config_placeholder,enemy_level=build.configTab.enemyLevel,
  default_level=level,monster_damage=build.data.monsterDamageTable[level],default_damage=damage,
  default_chaos=expected.Chaos,default_pen=build.data.misc.pinnacleBossPen,
  output=scalars(output),player=scalars(env.player.output),enemy=scalars(env.enemy.output)}
end
local out = {business_method_wrappers=false,selected=selection,callbacks=auth.callbacks,
 constructors=auth.constructors,load_entries=auth.load_entries,
 defence_calls=auth.defence_calls,input=scalars(build.configTab.input),placeholder=scalars(build.configTab.placeholder),
 max_enemy_level=build.data.misc.MaxEnemyLevel,pinnacle_multiplier=build.data.misc.pinnacleBossDPSMult,
 monster_damage_table=plain(build.data.monsterDamageTable),pinnacle_pen=build.data.misc.pinnacleBossPen,
 damage_type_order=damageTypes,match_all=KeywordFlag.MatchAll,enemy_keyword_flags=NOT64(KeywordFlag.MatchAll),
 main=view(build.calcsTab.mainEnv,mainOutput),calcs=view(build.calcsTab.calcsEnv,calcsOutput)}
for index,value in ipairs(tabs) do assert(equal(saved[index],value)) end
assert(mainOutput == build.calcsTab.mainOutput and calcsOutput == build.calcsTab.calcsOutput)
assert(equal(savedMain,mainOutput) and equal(savedCalcs,calcsOutput))
for index,entry in ipairs(specs) do
 local spec = build.treeTab.specList[index]; assert(spec == entry.object and spec.allocNodes == entry.nodes)
 local count = 0
 for id,node in pairs(spec.allocNodes) do
  count = count+1; local prior = assert(entry.allocations[id]); assert(node == prior.object and node.alloc == prior.alloc and node.allocMode == prior.mode)
 end
 local priorCount = 0; for _ in pairs(entry.allocations) do priorCount=priorCount+1 end; assert(count == priorCount)
end
assert(#specs == #build.treeTab.specList and debug.gethook() == nil)
out.original_functions_preserved=true; out.loaded_state_preserved=true
out.saved_specs_preserved=true; out.cached_outputs_preserved=true; out.hook_restored=true
return out
