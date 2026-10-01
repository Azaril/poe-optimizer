-- Observe complete original source methods; no business function is replaced.
local function original(f,path,first,last)
 assert(type(f)=="function","missing original function "..path..":"..first)
 local info=debug.getinfo(f,"S");local source=info.source:gsub("\\","/")
 assert(info.what=="Lua" and source:sub(-#path)==path and info.linedefined==first,"unexpected original function "..source..":"..info.linedefined.."; expected "..path..":"..first)
 if last then assert(info.lastlinedefined==last,"unexpected function end "..path..":"..info.lastlinedefined.."; expected "..last) end
 return f,{path=path,first=info.linedefined,last=info.lastlinedefined}
end
local function upvalue(f,key)for i=1,64 do local k,v=debug.getupvalue(f,i);if not k then break end;if k==key then return v end end;error("missing source upvalue "..key)end
local function scalar(v)
 if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return tostring(v) end
 assert(v==nil or type(v)=="number" or type(v)=="string" or type(v)=="boolean")
 return v
end
local function scalars(t)local out={};for k,v in pairs(t or{})do if type(v)~="table" and type(v)~="function" then out[k]=scalar(v) end end;return out end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local class=common.classes.ConfigTab
local methods={
 {"constructor_wrapper",class,"ConfigTab","Modules/Common.lua",167,183},
 {"load",class,"Load","Classes/ConfigTab.lua",878,978},
 {"create_set",class,"CreateConfigSet","Classes/ConfigTab.lua",1317,1334},
 {"select_set",class,"SetActiveConfigSet","Classes/ConfigTab.lua",1407},
 {"update_controls",class,"UpdateControls","Classes/ConfigTab.lua",1047,1061},
 {"update_level",class,"UpdateLevel","Classes/ConfigTab.lua",1157,1167},
 {"build_mod_list",class,"BuildModList","Classes/ConfigTab.lua",1169,1241},
 {"set_placeholder",common.classes.EditControl,"SetPlaceholder","Classes/EditControl.lua",110,115},
 {"launch_frame",launch,"OnFrame","Launch.lua",110},
 {"launch_error",launch,"ShowErrMsg","Launch.lua",379,386},
}
local definitions=require("Modules.ConfigOptions")
local boss,level;for _,row in ipairs(definitions)do if row.var=="enemyIsBoss"then assert(not boss);boss=row elseif row.var=="enemyLevel"then assert(not level);level=row end end;assert(boss and level)
local callback,callbackSource=original(boss.apply,"Modules/ConfigOptions.lua",1982,2139)
local calcs=require("Modules.CalcBase")
local init,initSource=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
-- The pinned class system wraps constructors itself; authenticate both layers.
local constructor,constructorSource=original(upvalue(class.ConfigTab,"originalFunc"),"Classes/ConfigTab.lua",134)
if enemyLevelPhase=="before"then
 local refs,auth={},{}
 for _,m in ipairs(methods)do refs[m[1]],auth[m[1]]=original(m[2][m[3]],m[4],m[5],m[6])end
 auth.constructor_original=constructorSource
 enemyLevelOriginals={refs=refs,auth=auth,boss=callback,init=init,constructor=constructor}
 return function()
  for _,m in ipairs(methods)do assert(m[2][m[3]]==refs[m[1]])end
  assert(boss.apply==callback and calcs.initEnv==init)
  assert(upvalue(class.ConfigTab,"originalFunc")==constructor)
  enemyLevelOriginals.unchanged_after_load=true
  -- Launch catches original OnFrame errors into its own prompt. Read it before
  -- the shared complete-build harness rejects an unfinished source load.
  local cfg=build.configTab
  return {original_methods_preserved=true,prompt=launch.promptMsg,main_mode=main.mode,pending_mode=main.newMode,popup_count=#main.popups,config_present=cfg~=nil,selected_config=cfg and cfg.activeConfigSetId,input=cfg and scalars(cfg.input),placeholder=cfg and scalars(cfg.placeholder),config_level=cfg and scalar(cfg.enemyLevel)}
 end
end
assert(enemyLevelPhase=="observe" and enemyLevelOriginals.unchanged_after_load)
for _,m in ipairs(methods)do assert(m[2][m[3]]==enemyLevelOriginals.refs[m[1]])end
assert(callback==enemyLevelOriginals.boss and init==enemyLevelOriginals.init)
assert(constructor==enemyLevelOriginals.constructor)
local config=assert(build.configTab)
local _,levelCallbackSource=original(config.varControls.enemyLevel.changeFunc,"Classes/ConfigTab.lua",315,324)
local selected=config.activeConfigSetId
local current=assert(config.configSets[selected])
assert(config.input==current.input and config.placeholder==current.placeholder)
local setCopies,setRefs={},{}
for id,set in pairs(config.configSets)do
 setRefs[id]=set
 setCopies[#setCopies+1]={id=id,title=set.title,input=scalars(set.input),placeholder=scalars(set.placeholder)}
end
table.sort(setCopies,function(a,b)return a.id<b.id end)
local inputs,placeholders=scalars(config.input),scalars(config.placeholder)
local doc,err=common.xml.ParseXML(enemyLevelXml);assert(doc and not err)
local raw={};for _,section in ipairs(doc[1])do if type(section)=="table"and section.elem=="Config"then
 local sets={};for _,set in ipairs(section)do if type(set)=="table"then
  local entries={};for _,entry in ipairs(set)do if type(entry)=="table"then
   if entry.attrib.name=="enemyLevel"or entry.attrib.name=="enemyIsBoss"then entries[#entries+1]={element=entry.elem,attributes=scalars(entry.attrib)}end
  end end
  sets[#sets+1]={element=set.elem,attributes=scalars(set.attrib),entries=entries}
 end end
 raw[#raw+1]={attributes=scalars(section.attrib),sets=sets}
end end
local modes={}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 local output=assert(mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)
 local enemy=assert(env.enemy)
 assert(env.configInput==config.input and env.configPlaceholder==config.placeholder)
 assert(enemy.modDB==env.enemyDB and env.enemyDB.actor==enemy and env.player.enemy==enemy)
 modes[mode]={available=true,enemy_level=scalar(env.enemyLevel),actor_level=scalar(enemy.level),actor_level_type=type(enemy.level),input_alias=true,placeholder_alias=true,enemy_actor_alias=true,outputs_available=type(output)=="table"}
end
local cap=assert(data.misc.MaxEnemyLevel)
local callbackLevel=config.enemyLevel
-- These are source data reads, not substitute callback or enemy-level evaluation.
local sourceData={maximum_enemy_level=cap,monster_damage_at_actor_level=data.monsterDamageTable[callbackLevel],monster_damage_at_82=data.monsterDamageTable[82],monster_armour_at_actor_level=data.monsterArmourTable[callbackLevel],monster_armour_at_82=data.monsterArmourTable[82],pinnacle_damage_multiplier=data.misc.pinnacleBossDPSMult,pinnacle_armour_mean=data.bossStats.PinnacleArmourMean}
local opts={};for _,v in ipairs(boss.list)do opts[#opts+1]={value=v.val,label=v.label}end
local result={raw_config=raw,selected={config=selected,items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec},character_level=build.characterLevel,input=inputs,placeholder=placeholders,sets=setCopies,config_level=config.enemyLevel,config_level_type=type(config.enemyLevel),modes=modes,definitions={boss_default_index=boss.defaultIndex,boss_default_value=boss.list[boss.defaultIndex].val,boss_options=opts,level_type=level.type},source_data=sourceData,method_sources=enemyLevelOriginals.auth,callback_source=callbackSource,level_callback_source=levelCallbackSource,init_source=initSource,original_methods_preserved=true,method_wrappers=false}
assert(config.activeConfigSetId==selected and equal(inputs,scalars(config.input))and equal(placeholders,scalars(config.placeholder)))
for _,saved in ipairs(setCopies)do local set=config.configSets[saved.id];assert(set==setRefs[saved.id]and equal(saved.input,scalars(set.input))and equal(saved.placeholder,scalars(set.placeholder)))end
result.saved_sets_preserved=true
return result
