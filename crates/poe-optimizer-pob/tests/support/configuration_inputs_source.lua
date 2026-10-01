-- Read-only complete Config lifecycle census. No callback or business method is replaced.
assert(jit.status()==configurationInputsJit,"unexpected source JIT mode")
local function source(f)
 assert(type(f)=="function")
 local i=debug.getinfo(f,"S")
 return {path=i.source:gsub("\\","/"),first=i.linedefined,last=i.lastlinedefined,kind=i.what}
end
local function original(f,path,line)
 local s=source(f)
 assert(s.kind=="Lua"and s.path:sub(-#path)==path and s.first==line,"unexpected source "..s.path..":"..s.first.." expected "..path..":"..line)
 return f,s
end
local function upvalue(f,key)
 for i=1,64 do local k,v=debug.getupvalue(f,i);if not k then break end;if k==key then return v end end
 error("missing original upvalue "..key)
end
local function plain(v,depth,functions)
 if type(v)=="function"then assert(functions,"unexpected function in observed state");return {function_source=source(v)}end
 if type(v)~="table"then
  assert(v==nil or type(v)=="string"or type(v)=="number"or type(v)=="boolean")
  if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end
  return v
 end
 depth=(depth or 0)+1;assert(depth<25)
 local count,maximum,array=0,0,true
 for k in pairs(v)do count=count+1;assert(count<30000);if type(k)=="number"and k>=1 and k==math.floor(k)then maximum=math.max(maximum,k)else array=false end end
 array=array and maximum==count
 local out={};for k,x in pairs(v)do assert(type(k)=="string"or type(k)=="number");local key=array and k or tostring(k);assert(out[key]==nil,"ambiguous snapshot key");out[key]=plain(x,depth,functions)end
 return out
end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="string"or type(v)=="number"or type(v)=="boolean"then r[k]=plain(v)end end;return r end
local class=common.classes.ConfigTab
local calcs=require("Modules.CalcBase")
local methods={
 {"constructor_wrapper",class,"ConfigTab","Modules/Common.lua",167},
 {"load",class,"Load","Classes/ConfigTab.lua",878},
 {"create_set",class,"CreateConfigSet","Classes/ConfigTab.lua",1317},
 {"select_set",class,"SetActiveConfigSet","Classes/ConfigTab.lua",1407},
 {"get_default",class,"GetDefaultState","Classes/ConfigTab.lua",980},
 {"update_controls",class,"UpdateControls","Classes/ConfigTab.lua",1047},
 {"update_level",class,"UpdateLevel","Classes/ConfigTab.lua",1157},
 {"build_mod_list",class,"BuildModList","Classes/ConfigTab.lua",1169},
 {"set_placeholder",common.classes.EditControl,"SetPlaceholder","Classes/EditControl.lua",110},
 {"init_env",calcs,"initEnv","Modules/CalcSetup.lua",717},
 {"defence",calcs,"defence","Modules/CalcDefence.lua",789},
 {"offence",calcs,"offence","Modules/CalcOffence.lua",527},
 {"sum",common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {"round",_G,"round","Modules/Common.lua",722},
 {"launch_error",launch,"ShowErrMsg","Launch.lua",379},
 {"output",common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
}
local constructor,constructorSource=original(upvalue(class.ConfigTab,"originalFunc"),"Classes/ConfigTab.lua",134)
local varList=upvalue(class.BuildModList,"varList")
assert(varList==require("Modules.ConfigOptions"))
local definitions,callbacks={},{}
local function capture_functions(v,path)
 if type(v)=="function"then callbacks[#callbacks+1]={path=path,value=v};return end
 if type(v)=="table"then for k,x in pairs(v)do capture_functions(x,path.."/"..tostring(k))end end
end
for index,row in ipairs(varList)do
 definitions[#definitions+1]={index=index,definition=plain(row,0,true)}
 capture_functions(row,tostring(index))
end
local function same_callbacks(saved)
 assert(#callbacks==#saved)
 -- Traversal is only compared within one Lua state; JSON catalogue ordering is indexed.
 for index,row in ipairs(callbacks)do assert(row.path==saved[index].path and row.value==saved[index].value,"source callback changed "..row.path)end
end
if configurationInputsPhase=="before"then
 local refs,auth={},{};for _,m in ipairs(methods)do refs[m[1]],auth[m[1]]=original(m[2][m[3]],m[4],m[5])end
 auth.constructor_original=constructorSource
 configurationInputsOriginals={refs=refs,auth=auth,constructor=constructor,callbacks=callbacks,var_list=varList,catalogue=definitions}
 return function()
  assert(jit.status()==configurationInputsJit)
  for _,m in ipairs(methods)do assert(m[2][m[3]]==refs[m[1]],"source method changed "..m[1])end
  assert(upvalue(class.ConfigTab,"originalFunc")==constructor and upvalue(class.BuildModList,"varList")==varList)
  configurationInputsOriginals.finished=true
  local titles={};for _,p in ipairs(main.popups)do titles[#titles+1]=p.title end
  local cfg=build.configTab
  return {methods_preserved=true,prompt=launch.promptMsg,main_mode=main.mode,pending_mode=main.newMode,popup_titles=titles,config_present=cfg~=nil,selected_config=cfg and cfg.activeConfigSetId,input=cfg and plain(cfg.input),placeholder=cfg and plain(cfg.placeholder)}
 end
end
assert(configurationInputsPhase=="observe"and configurationInputsOriginals.finished)
same_callbacks(configurationInputsOriginals.callbacks)
assert(equal(definitions,configurationInputsOriginals.catalogue),"source catalogue mutated during load")
local config=assert(build.configTab)
local function mod(m)
 local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end
 return {name=m.name,type=m.type,value=plain(m.value),source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags,all_fields=plain(m,0,true)}
end
local function records(list)local r={};for _,m in ipairs(list)do r[#r+1]=mod(m)end;return r end
local function database(db,names)
 local r,seen={},{};local depth=0
 while db do depth=depth+1;assert(depth<=16 and not seen[db]);seen[db]=true
  local ordered={};for name in pairs(db.mods)do if names[name]then ordered[#ordered+1]=name end end;table.sort(ordered)
  for _,name in ipairs(ordered)do for index,m in ipairs(db.mods[name])do r[#r+1]={depth=depth,index=index,record=mod(m)}end end
  db=db.parent
 end;return r
end
local function delivery(list,db,context)
 local names={};for _,m in ipairs(list)do names[m.name]=true end
 local actual=database(db,names);local used,joins={},{}
 for index,m in ipairs(list)do
  local expected=mod(m);local found
  for i,row in ipairs(actual)do if not used[i]and equal(expected,row.record)then found=i;break end end
  assert(found,context.." missing exact delivered Config record "..index.." "..m.name.." "..tostring(m.source))
  used[found]=true;joins[#joins+1]={config_index=index,database_index=found}
 end
 return {records=actual,joins=joins}
end
local function outputs(env,out)return {output=scalars(out),player=scalars(env.player.output),enemy=scalars(env.enemy.output),minion_available=env.minion~=nil,minion=env.minion and scalars(env.minion.output)}end
local function breakdown(actor)
 local r={available=actor~=nil};if not actor then return r end
 r.entries={};for _,name in ipairs({"Fire","Cold","Lightning","Chaos"})do
  local b=actor.breakdown and actor.breakdown[name.."EffMult"]
  r.entries[name]={available=b~=nil,value=b and plain(b,0,true)}
 end;return r
end
local function snapshot()
 local sets={};for id,set in pairs(config.configSets)do sets[#sets+1]={id=id,title=set.title,input=plain(set.input),placeholder=plain(set.placeholder),custom_blocks=plain(set.customModsList)}end;table.sort(sets,function(a,b)return a.id<b.id end)
 local controlRows={};for key,c in pairs(config.varControls)do controlRows[#controlRows+1]={key=key,scalars=scalars(c),change_source=type(c.changeFunc)=="function"and source(c.changeFunc)or nil}end;table.sort(controlRows,function(a,b)return a.key<b.key end)
 local modes={};for _,mode in ipairs({"MAIN","CALCS"})do
  local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  local out=assert(mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)
  assert(env.configInput==config.input and env.configPlaceholder==config.placeholder)
  local base={};for _,name in ipairs({"FireResist","ColdResist","LightningResist","ChaosResist","Armour","Evasion"})do
   base[name]={all=env.enemyDB:Sum("BASE",nil,name),enemy_config=env.enemyDB:Sum("BASE",{source="EnemyConfig"},name),config=env.enemyDB:Sum("BASE",{source="Config"},name)}
  end
  modes[mode]={input_alias=true,placeholder_alias=true,mode_effective=env.mode_effective,enemy_level=env.enemy.level,main_effect=env.player.mainSkill.activeEffect.grantedEffect.id,main_group=env.mainSocketGroup,
   player_delivery=delivery(config.modList,env.modDB,mode.." Player"),enemy_delivery=delivery(config.enemyModList,env.enemyDB,mode.." Enemy"),
   player_conditions=plain(env.modDB.conditions),enemy_conditions=plain(env.enemyDB.conditions),enemy_base=base,player_resistance_breakdown=breakdown(env.player),minion_resistance_breakdown=breakdown(env.minion),outputs=outputs(env,out)}
 end
 return {selected={config=config.activeConfigSetId,skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec},sets=sets,input=plain(config.input),placeholder=plain(config.placeholder),control_defaults=plain(config.defaultState),controls=controlRows,enemy_level=config.enemyLevel,player_mods=records(config.modList),enemy_mods=records(config.enemyModList),modes=modes}
end
local current=assert(config.configSets[config.activeConfigSetId]);assert(config.input==current.input and config.placeholder==current.placeholder)
local saved=snapshot()
local setRefs={};for id,set in pairs(config.configSets)do setRefs[id]={set=set,input=set.input,placeholder=set.placeholder,blocks=set.customModsList}end
local refs={input=config.input,placeholder=config.placeholder,mod_list=config.modList,enemy_mod_list=config.enemyModList,spec=build.spec,main_env=build.calcsTab.mainEnv,calcs_env=build.calcsTab.calcsEnv,main_output=build.calcsTab.mainOutput,calcs_output=build.calcsTab.calcsOutput}
local doc,err=common.xml.ParseXML(configurationInputsXml);assert(doc and not err)
local ordinal,nextOrdinal={},0
local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinal[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,c in ipairs(n)do enumerate(c)end end;enumerate(doc[1])
local raw={};for _,section in ipairs(doc[1])do if type(section)=="table"and section.elem=="Config"then
 local rows={};for _,set in ipairs(section)do if type(set)=="table"then
  local entries={};for _,entry in ipairs(set)do if type(entry)=="table"then entries[#entries+1]={source=ordinal[entry],xml=plain(entry)}end end
  rows[#rows+1]={source=ordinal[set],element=set.elem,attributes=plain(set.attrib),entries=entries}
 end end
 raw[#raw+1]={source=ordinal[section],attributes=plain(section.attrib),sets=rows}
end end
assert(config.input==refs.input and config.placeholder==refs.placeholder and config.modList==refs.mod_list and config.enemyModList==refs.enemy_mod_list and build.spec==refs.spec)
assert(build.calcsTab.mainEnv==refs.main_env and build.calcsTab.calcsEnv==refs.calcs_env and build.calcsTab.mainOutput==refs.main_output and build.calcsTab.calcsOutput==refs.calcs_output)
for id,r in pairs(setRefs)do local set=config.configSets[id];assert(set==r.set and set.input==r.input and set.placeholder==r.placeholder and set.customModsList==r.blocks)end
assert(equal(saved,snapshot()),"read-only observation changed state/output")
-- The full local preservation check above includes this clock. EditControl
-- stores GetTime() only to animate the cursor; it is not a cross-run input.
for _,row in ipairs(saved.controls)do
 assert(row.scalars.blinkStart==nil or type(row.scalars.blinkStart)=="number")
 row.scalars.blinkStart=nil
end
-- The complete local snapshot above checks the source database's actual order.
-- Cross-run delivery proves only Config-origin records: unrelated producers can
-- insert equal-name records in different orders. Keep actual matched records,
-- their database depth, and multiplicity, without copying expected Config rows.
for _,mode in ipairs({"MAIN","CALCS"})do
 for _,target in ipairs({"player_delivery","enemy_delivery"})do
  local delivered=saved.modes[mode][target]
  local matched,joins,used={},{},{}
  for _,join in ipairs(delivered.joins)do
   assert(not used[join.database_index],"duplicate actual database match")
   used[join.database_index]=true
   local actual=assert(delivered.records[join.database_index])
   matched[#matched+1]={depth=actual.depth,record=actual.record}
   joins[#joins+1]={config_index=join.config_index,matched_index=#matched}
  end
  delivered.records=matched;delivered.joins=joins
 end
end
assert(jit.status()==configurationInputsJit)
for _,m in ipairs(methods)do assert(m[2][m[3]]==configurationInputsOriginals.refs[m[1]])end
local levels={};for level=1,85 do levels[#levels+1]={level=level,armour=assert(data.monsterArmourTable[level]),evasion=assert(data.monsterEvasionTable[level])}end
return {raw=raw,saved=saved,catalogue=definitions,quest_catalogue=plain(data.questRewards),source_data={levels=levels,pinnacle_armour_mean=data.bossStats.PinnacleArmourMean,pinnacle_evasion_mean=data.bossStats.PinnacleEvasionMean,enemy_max_resist=data.misc.EnemyMaxResist,max_resist_cap=data.misc.MaxResistCap,resist_floor=data.misc.ResistFloor},method_sources=configurationInputsOriginals.auth,callback_reference_count=#callbacks,methods_preserved=true,objects_preserved=true,outputs_preserved=true,business_wrappers=false,callback_invocation_trace=false,omitted_ui_clock_fields={"controls.scalars.blinkStart"},delivery_scope="injectively_matched_actual_config_records_in_config_order",unrelated_global_database_order_excluded=true}
