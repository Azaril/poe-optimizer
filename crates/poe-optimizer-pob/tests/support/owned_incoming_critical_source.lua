-- Observe original critical calculations during complete Build loading. No method replacement.
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
 depth=(depth or 0)+1; assert(depth<=24)
 local out,count,maximum,array={},0,0,true
 for key in pairs(value) do
  count=count+1;assert(count<=30000)
  if type(key)=="number" and key>=1 and key==math.floor(key) then maximum=math.max(maximum,key) else array=false end
 end
 array=array and count==maximum
 for key,child in pairs(value) do
  assert(type(key)=="string" or type(key)=="number")
  out[array and key or tostring(key)]=plain(child,depth)
 end
 return out
end
local function scalars(value)
 local out={}
 for key,child in pairs(value or {}) do
  if type(child)=="number" or type(child)=="string" or type(child)=="boolean" then out[key]=plain(child) end
 end
 return out
end
local function membership(db,name)
 local out,seen,depth={},{},0
 while db do
  depth=depth+1;assert(depth<=16 and not seen[db]);seen[db]=true
  for index,mod in ipairs(db.mods[name] or {}) do out[#out+1]={depth=depth,index=index,record=plain(mod)} end
  db=db.parent
 end
 return out
end
local function belongs(db,root)
 for depth=0,15 do
  if root==db then return true end
  if not root then return false end
  root=root.parent
 end
 error("mod DB parent depth exceeded")
end
local function locals()
 local vars={}
 for index=1,128 do local name,value=debug.getlocal(3,index);if not name then break end;vars[name]=value end
 return vars
end
local function query(db,name)
 return {queried=false,returned=false,result_available=false,bucket_membership=membership(db,name)}
end
local methods={
 {calcs,"buildDefenceEstimations","Modules/CalcDefence.lua",2207},
 {common.classes.ModDB,"SumInternal","Classes/ModDB.lua",137},
 {common.classes.ModDB,"FlagInternal","Classes/ModDB.lua",297},
 {common.classes.ModDB,"OverrideInternal","Classes/ModDB.lua",344},
 {common.classes.ConfigTab,"Load","Classes/ConfigTab.lua",878},
 {common.classes.ConfigTab,"BuildModList","Classes/ConfigTab.lua",1169},
 {common.classes.EditControl,"SetPlaceholder","Classes/EditControl.lua",110},
 {common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
}
local boss
for _,row in ipairs(require("Modules.ConfigOptions")) do if row.var=="enemyIsBoss" then assert(not boss);boss=row.apply end end
assert(boss)
local constructor
for index=1,128 do
 local name,value=debug.getupvalue(common.classes.ConfigTab.ConfigTab,index)
 if not name then break end
 if name=="originalFunc" then constructor=original(value,"Classes/ConfigTab.lua",134) end
end
assert(constructor)
if incomingCriticalPhase=="before" then
 local refs={};for index,row in ipairs(methods) do refs[index]=original(row[1][row[2]],row[3],row[4]) end
 original(boss,"Modules/ConfigOptions.lua",1982)
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local enabled=jit.status();assert(enabled==incomingCriticalJit)
 local auth={refs=refs,boss=boss,views={},reductions={},callbacks={},constructors={},load_entries={},defence_calls=0}
 local frame
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  if f~=refs[1] and f~=refs[2] and f~=refs[3] and f~=refs[4] and f~=refs[5] and f~=boss and f~=constructor then return end
  local vars=locals()
  if f==constructor and event=="return" then
   auth.constructors[#auth.constructors+1]={input=scalars(vars.self.input),placeholder=scalars(vars.self.placeholder),default_state=scalars(vars.self.defaultState)}
  elseif f==refs[5] and event=="call" then
   auth.load_entries[#auth.load_entries+1]={input=scalars(vars.self.input),placeholder=scalars(vars.self.placeholder)}
  elseif f==boss and event=="return" then
   local cfg=vars.build.configTab
   auth.callbacks[#auth.callbacks+1]={value=vars.val,input=scalars(cfg.input),placeholder=scalars(cfg.placeholder)}
  elseif f==refs[1] then
   if event=="call" and vars.actor==vars.env.player then
    assert(not frame)
    local actor,enemy=vars.actor.modDB,vars.actor.enemy.modDB
    frame={env=vars.env,actor=vars.actor,critical=false,hit_branch=false,queries={
     never_crit=query(enemy,"NeverCrit"),always_crit=query(enemy,"AlwaysCrit"),unlucky_crit=query(actor,"EnemyUnluckyCrit"),
     override_chance=query(actor,"enemyCritChance"),actor_chance_increase=query(actor,"EnemyCritChance"),
     enemy_chance_increase=query(enemy,"CritChance"),enemy_bonus_base=query(enemy,"CritMultiplier"),enemy_bonus_increase=query(enemy,"CritMultiplier")}}
    debug.sethook(hook,"crl")
   elseif event=="line" and frame and vars.actor==frame.actor then
    if line==2268 then frame.critical=true;frame.hit_branch=true
    elseif line==2273 then frame.chance=vars.enemyCritChance
    elseif line==2274 then frame.damage_bonus=vars.enemyCritDamage
    elseif line==2275 then frame.critical=false end
   elseif event=="return" and frame and vars.actor==frame.actor then
    local output=frame.actor.output
    auth.defence_calls=auth.defence_calls+1
    auth.views[frame.env]={mode=frame.env.mode,category=vars.damageCategoryConfig,hit_branch=frame.hit_branch,
     input=scalars(frame.env.configInput),placeholder=scalars(frame.env.configPlaceholder),queries=frame.queries,
     configured_evade=plain(output.ConfiguredEvadeChance),extra_damage_reduction=plain(output.CritExtraDamageReduction),
     chance=plain(output.EnemyCritChance),damage_bonus=plain(frame.damage_bonus),critical_effect=plain(output.EnemyCritEffect),
     local_chance=plain(frame.chance),reduction_query=auth.reductions[frame.actor.modDB],
     total_in=plain(output.totalEnemyDamageIn),total_damage=plain(output.totalEnemyDamage),
     evasion=plain(output.Evasion),actor=frame.actor}
    frame=nil;debug.sethook(hook,"cr")
   end
  elseif f==refs[2] and event=="return" and vars.modType=="BASE" and vars.modName=="ReduceCritExtraDamage" then
   auth.reductions[vars.self]={queried=true,result_available=true,result=vars.result,cfg=plain(vars.cfg),
    flags=vars.flags,keyword_flags=vars.keywordFlags,bucket_membership=membership(vars.self,vars.modName)}
  elseif frame and frame.critical then
   local root,name
   if f==refs[3] then
    if vars.modName=="NeverCrit" then root=frame.actor.enemy.modDB;name="never_crit"
    elseif vars.modName=="AlwaysCrit" then root=frame.actor.enemy.modDB;name="always_crit"
    elseif vars.modName=="EnemyUnluckyCrit" then root=frame.actor.modDB;name="unlucky_crit" end
   elseif f==refs[4] and vars.modName=="enemyCritChance" then root=frame.actor.modDB;name="override_chance"
   elseif f==refs[2] then
    if vars.modName=="EnemyCritChance" and vars.modType=="INC" then root=frame.actor.modDB;name="actor_chance_increase"
    elseif vars.modName=="CritChance" and vars.modType=="INC" then root=frame.actor.enemy.modDB;name="enemy_chance_increase"
    elseif vars.modName=="CritMultiplier" and vars.modType=="BASE" then root=frame.actor.enemy.modDB;name="enemy_bonus_base"
    elseif vars.modName=="CritMultiplier" and vars.modType=="INC" then root=frame.actor.enemy.modDB;name="enemy_bonus_increase" end
   end
   if root and belongs(vars.self,root) then
    local row=assert(frame.queries[name])
    if event=="call" and vars.self==root then
     row.queried=true;row.cfg=plain(vars.cfg);row.flags=vars.flags;row.keyword_flags=vars.keywordFlags
     -- FLAG's absent return is nil in PoB; evidence exposes its Boolean truth,
     -- while queried/returned preserve whether the original call occurred.
     if f==refs[3] then row.result_available=true;row.result=false;row.result_semantics="boolean_truth" end
    elseif event=="line" and f==refs[3] and (line==306 or line==309) then
     row.result_available=true;row.result=true;row.winner=plain(vars.mod);row.return_line=line
    elseif event=="line" and f==refs[4] and (line==353 or line==356) then
     row.result_available=true;row.result=plain(line==353 and vars.value or vars.mod.value);row.winner=plain(vars.mod);row.return_line=line
    elseif event=="return" and vars.self==root then
     row.returned=true
     if f==refs[2] then row.result_available=true;row.result=plain(vars.result) end
    end
   end
  end
 end
 jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"cr")
 incomingCriticalAuth=auth
 return function()
  assert(debug.gethook()==hook);debug.sethook(oldHook,oldMask,oldCount)
  auth.incomplete=frame~=nil
  assert(jit.status()==enabled)
  for index,row in ipairs(methods) do assert(row[1][row[2]]==refs[index]) end
  auth.finished=true
 end
end
assert(incomingCriticalPhase=="after")
local auth=assert(incomingCriticalAuth);assert(auth.finished and not auth.incomplete and #auth.callbacks>0)
for index,row in ipairs(methods) do assert(row[1][row[2]]==auth.refs[index]) end
assert(auth.boss==boss and debug.gethook()==nil)
local function view(env,output)
 local captured=assert(auth.views[env],"missing original defence invocation")
 assert(captured.actor==env.player)
 for _,name in ipairs({"EnemyCritChance","EnemyCritEffect","ConfiguredEvadeChance","CritExtraDamageReduction"}) do
  assert(output[name]==env.player.output[name],"cached critical output differs")
 end
 local result={}
 for name,value in pairs(captured) do if name~="actor" then result[name]=value end end
 if result.hit_branch then assert(result.chance==result.local_chance and result.damage_bonus~=nil) end
 return result
end
return {business_method_wrappers=false,original_functions_preserved=true,hook_restored=true,
 defence_calls=auth.defence_calls,constructors=auth.constructors,load_entries=auth.load_entries,callbacks=auth.callbacks,
 default_critical_bonus=build.data.monsterConstants.base_critical_hit_damage_bonus,hit_flag=ModFlag.Hit,
 selected={items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,spec=build.treeTab.activeSpec,main_group=build.mainSocketGroup},
 main=view(build.calcsTab.mainEnv,build.calcsTab.mainOutput),calcs=view(build.calcsTab.calcsEnv,build.calcsTab.calcsOutput)}
