-- Read-only observation of the original preset callback and complete actor output.
local function original(f,path,line)
 local info=debug.getinfo(f,"S");local actual=info.source:gsub("\\","/")
 assert(info.what=="Lua" and actual:sub(-#path)==path and info.linedefined==line,
  "unexpected source "..actual..":"..info.linedefined)
 return f
end
local function scalar(value)
 if type(value)=="number" and (value~=value or value==math.huge or value==-math.huge) then return tostring(value) end
 return value
end
local function output(value)
 local out={scalars={},availability={}};local count=0
 for key,child in pairs(value) do
  count=count+1;assert(count<=5000 and type(key)=="string")
  local kind=type(child);out.availability[key]=kind
  assert(kind=="table" or kind=="number" or kind=="string" or kind=="boolean", "unexpected output type "..kind)
  if kind~="table" then out.scalars[key]=scalar(child) end
 end
 return out
end
local function damage(fields)
 local out={}
 for _,kind in ipairs({"Physical","Lightning","Cold","Fire","Chaos"}) do
  out[kind]=scalar(fields["enemy"..kind.."Damage"])
 end
 return out
end
local function state(build)
 local cfg=build.configTab
 return {boss=cfg.input.presetBossSkills,input_roll=cfg.input.enemyDamageRollRange,
  saved_placeholder_roll=cfg.placeholder.enemyDamageRollRange,
  control_placeholder_roll=cfg.varControls.enemyDamageRollRange.placeholder,
  damage_input=damage(cfg.input),damage_placeholder=damage(cfg.placeholder),
  damage_type=cfg.input.enemyDamageType}
end
local function locals()
 local out={};for index=1,128 do local name,value=debug.getlocal(3,index);if not name then break end;out[name]=value end
 return out
end
local preset,roll
for _,row in ipairs(require("Modules.ConfigOptions")) do
 if row.var=="presetBossSkills" then assert(not preset);preset=row
 elseif row.var=="enemyDamageRollRange" then assert(not roll);roll=row end
end
assert(preset and roll and not roll.apply)
local callback=original(preset.apply,"Modules/ConfigOptions.lua",2189)
local methods={
 {common.classes.ConfigTab,"Load","Classes/ConfigTab.lua",878},
 {common.classes.ConfigTab,"BuildModList","Classes/ConfigTab.lua",1169},
 {common.classes.EditControl,"SetPlaceholder","Classes/EditControl.lua",110},
 {common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
}
if damageRollPhase=="before" then
 local refs={};for index,row in ipairs(methods) do refs[index]=original(row[1][row[2]],row[3],row[4]) end
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local enabled=jit.status();assert(enabled==damageRollJit)
 local auth={refs=refs,callback=callback,calls={}}
 local frame
 local function hook(event,line)
  if debug.getinfo(2,"f").func~=callback then return end
  local vars=locals()
  if event=="call" then
   assert(not frame)
   frame={value=vars.val,guard_observed=false,boss_branch=false,none_branch=false,
    roll_read=false,roll_available=false,before=state(vars.build)}
   debug.sethook(hook,"crl")
  elseif event=="line" then
   assert(frame)
   if line==2190 then frame.guard_observed=true
   elseif line==2191 then frame.boss_branch=true
   elseif line==2203 then frame.roll_read=true
   elseif line==2204 then frame.roll_available=true;frame.roll=vars.rollRangeMult
   elseif line==2262 then frame.none_branch=true end
  elseif event=="return" then
   assert(frame);frame.after=state(vars.build);auth.calls[#auth.calls+1]=frame
   frame=nil;debug.sethook(hook,"cr")
  end
 end
 jit.flush();debug.sethook(hook,"cr");damageRollAuth=auth
 return function()
  assert(debug.gethook()==hook);debug.sethook(oldHook,oldMask,oldCount)
  assert(not frame and jit.status()==enabled)
  for index,row in ipairs(methods) do assert(row[1][row[2]]==refs[index]) end
  auth.finished=true
 end
end
assert(damageRollPhase=="after")
local auth=assert(damageRollAuth);assert(auth.finished and #auth.calls>0 and debug.gethook()==nil)
assert(auth.callback==callback)
for index,row in ipairs(methods) do assert(row[1][row[2]]==auth.refs[index]) end
local function view(env,cached)
 assert(cached==env.player.output)
 local summon=assert(env.player.mainSkill)
 local minion=assert(env.minion)
 assert(summon.minion==minion)
 local selected=assert(minion.mainSkill)
 return {mode=env.mode,player=output(env.player.output),selected_minion=output(minion.output),
  identity={summon=summon.activeEffect.grantedEffect.id,selected=selected.activeEffect.grantedEffect.id,
   main_group=env.mainSocketGroup,selected_summon=true,selected_minion=true},
  input_damage=damage(env.configInput),placeholder_damage=damage(env.configPlaceholder),
  input_roll=env.configInput.enemyDamageRollRange,placeholder_roll=env.configPlaceholder.enemyDamageRollRange,
  boss=env.configInput.presetBossSkills,damage_type=env.configInput.enemyDamageType}
end
local contrast=assert(build.data.bossSkills["Shaper Ball"])
assert(contrast.DamageMultipliers.Cold[2]>0)
return {original_functions_preserved=true,business_method_wrappers=false,hook_restored=true,
 callbacks=auth.calls,final_config=state(build),
 roll_definition={has_apply=roll.apply~=nil,kind=roll.type,default_placeholder=roll.defaultPlaceholderState},
 contrast={key="Shaper Ball",damage_type=contrast.DamageType,cold_base=contrast.DamageMultipliers.Cold[1],cold_range=contrast.DamageMultipliers.Cold[2]},
 selected={items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,
  config=build.configTab.activeConfigSetId,spec=build.treeTab.activeSpec,main_group=build.mainSocketGroup},
 main=view(build.calcsTab.mainEnv,build.calcsTab.mainOutput),calcs=view(build.calcsTab.calcsEnv,build.calcsTab.calcsOutput)}
