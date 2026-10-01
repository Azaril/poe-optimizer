-- Full pinned functions remain unchanged. Observers retain actual source records.
local function original(f,path,first,last)
 assert(type(f)=="function","missing function "..path..":"..first)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first,"unexpected function "..s..":"..i.linedefined.." expected "..path..":"..first)
 if last then assert(i.lastlinedefined==last,"unexpected end "..path..":"..i.lastlinedefined)end
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,key)for i=1,64 do local k,v=debug.getupvalue(f,i);if not k then break end;if k==key then return v end end;error("missing upvalue "..key)end
local function plain(v,depth)
 if type(v)~="table"then
  assert(type(v)=="nil"or type(v)=="string"or type(v)=="number"or type(v)=="boolean")
  if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end
  return v
 end
 depth=(depth or 0)+1;assert(depth<20);local out={};local count=0
 for k,x in pairs(v)do count=count+1;assert(count<20000);out[k]=plain(x,depth)end;return out
end
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="string"or type(v)=="boolean"or type(v)=="number"then r[k]=plain(v)end end;return r end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list,filter)
 local out={};for _,m in ipairs(list or{})do if not filter or filter(m)then
  local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end
  out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,tags=tags}
 end end;return out
end
local function quest(m)return type(m.source)=="string"and m.source:sub(1,6)=="Quest:"end
local function custom(m)return type(m.source)=="string"and m.source:sub(1,6)=="Custom"end
local function db_records(db,filter)
 local names={};for name in pairs(db.mods)do names[#names+1]=name end;table.sort(names)
 local out={};for _,name in ipairs(names)do for _,m in ipairs(records(db.mods[name],filter))do out[#out+1]=m end end;return out
end
local class=common.classes.ConfigTab
local methods={
 {"constructor_wrapper",class,"ConfigTab","Modules/Common.lua",167,183},
 {"load",class,"Load","Classes/ConfigTab.lua",878,978},
 {"create_set",class,"CreateConfigSet","Classes/ConfigTab.lua",1317,1334},
 {"select_set",class,"SetActiveConfigSet","Classes/ConfigTab.lua",1407},
 {"update_controls",class,"UpdateControls","Classes/ConfigTab.lua",1047,1061},
 {"build_mod_list",class,"BuildModList","Classes/ConfigTab.lua",1169,1241},
 {"dropdown_select",common.classes.DropDownControl,"SelByValue","Classes/DropDownControl.lua",142,156},
 {"launch_frame",launch,"OnFrame","Launch.lua",110},
 {"launch_error",launch,"ShowErrMsg","Launch.lua",379,386},
 {"parse_mod",modLib,"parseMod","Modules/ModParser.lua",7404},
 {"set_source",modLib,"setSource","Modules/ModTools.lua",277,283},
 {"build_output",common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486,517},
}
local constructor,constructorSource=original(upvalue(class.ConfigTab,"originalFunc"),"Classes/ConfigTab.lua",134)
local definitions=require("Modules.ConfigOptions")
local byKey={};for _,d in ipairs(definitions)do if d.var then if d.var:sub(1,5)=="quest"then assert(not byKey[d.var])end;byKey[d.var]=d end end
local rows,excluded,known,callbacks={},{},{},{}
for index,q in ipairs(data.questRewards)do
 local key="quest"..q.Description..q.Area..q.Info
 if q.useConfig==false then assert(not byKey[key]);excluded[#excluded+1]={source_index=index,key=key,quest=plain(q)}
 else
  assert(q.useConfig==true and not known[key]);known[key]=true
  local d=assert(byKey[key]);local first,last
  if q.Stat then assert(not q.Options and d.type=="check"and d.defaultState==true);first,last=82,84
  else assert(q.Options and d.type=="list"and d.defaultIndex==1);first,last=98,103 end
  local callback,where=original(d.apply,"Modules/ConfigOptions.lua",first,last)
  local applyLines,applyWhere=original(upvalue(callback,"applyModsFromString"),"Modules/ConfigOptions.lua",60,66)
  local parseQuest,parseWhere=original(upvalue(applyLines,"questModsRewards"),"Modules/ConfigOptions.lua",40,54)
  assert(upvalue(callback,"source")=="Quest:"..q.Description..": "..q.Area)
  if q.Stat then assert(upvalue(callback,"quest")==q)end
  callbacks[key]=callback
  local options={};if d.list then for _,option in ipairs(d.list)do options[#options+1]=option.val end end
  rows[#rows+1]={key=key,source_index=index,quest=plain(q),widget=d.type,default=d.defaultState,default_index=d.defaultIndex,options=options,callback=where,apply_lines=applyWhere,parse_quest=parseWhere,source=upvalue(callback,"source")}
 end
end
local generated=0
for _,d in ipairs(definitions)do if d.var and d.var:sub(1,5)=="quest"then assert(known[d.var],"unreviewed generated quest control "..d.var);generated=generated+1 end end
assert(#data.questRewards==29 and #rows==17 and #excluded==12 and generated==17)
if rewardPhase=="before"then
 local refs,auth={},{};for _,m in ipairs(methods)do refs[m[1]],auth[m[1]]=original(m[2][m[3]],m[4],m[5],m[6])end
 auth.constructor_original=constructorSource
 rewardOriginals={refs=refs,auth=auth,callbacks=callbacks,constructor=constructor}
 return function()
  for _,m in ipairs(methods)do assert(m[2][m[3]]==refs[m[1]])end
  for key,callback in pairs(callbacks)do assert(byKey[key].apply==callback)end
  assert(upvalue(class.ConfigTab,"originalFunc")==constructor)
  rewardOriginals.finished=true
  return {original_methods_preserved=true,prompt=launch.promptMsg,main_mode=main.mode,pending_mode=main.newMode,config_present=build.configTab~=nil}
 end
end
assert(rewardPhase=="observe"and rewardOriginals.finished)
for _,m in ipairs(methods)do assert(m[2][m[3]]==rewardOriginals.refs[m[1]])end
for key,callback in pairs(callbacks)do assert(callback==rewardOriginals.callbacks[key])end
assert(constructor==rewardOriginals.constructor)
local config=assert(build.configTab)
local function snapshot()
 local controls={};for _,r in ipairs(rows)do
  local control=assert(config.varControls[r.key]);local selection=control.selIndex
  controls[#controls+1]={key=r.key,widget=r.widget,input=plain(config.input[r.key]),input_type=type(config.input[r.key]),placeholder=plain(config.placeholder[r.key]),placeholder_type=type(config.placeholder[r.key]),ui_index=selection,ui_value=selection and plain(control.list[selection].val),ui_state=plain(control.state)}
 end
 local modes={};for _,mode in ipairs({"MAIN","CALCS"})do
  local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  assert(env.configInput==config.input and env.configPlaceholder==config.placeholder)
  modes[mode]={available=true,quest= db_records(env.modDB,quest),custom=db_records(env.modDB,custom),player=scalars(env.player.output),minion_available=env.minion~=nil,minion=env.minion and scalars(env.minion.output),enemy_level=env.enemy.level}
 end
 return {selected={config=config.activeConfigSetId,skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec},controls=controls,input=scalars(config.input),placeholder=scalars(config.placeholder),custom_blocks=plain(config.configSets[config.activeConfigSetId].customModsList),quest=records(config.modList,quest),custom=records(config.modList,custom),enemy_quest=records(config.enemyModList,quest),modes=modes}
end
local saved=snapshot();local input,placeholder=config.input,config.placeholder
local sourceSets={};for id,set in pairs(config.configSets)do sourceSets[#sourceSets+1]={id=id,title=set.title,input=scalars(set.input),placeholder=scalars(set.placeholder),custom_blocks=plain(set.customModsList)}end;table.sort(sourceSets,function(a,b)return a.id<b.id end)
local matrix={}
if rewardMatrix then
 for _,r in ipairs(rows)do
  local d=byKey[r.key];local values=r.widget=="check"and{true}or r.options
  for _,value in ipairs(values)do
   local player=new("ModList"):ModList();local enemy=new("ModList"):ModList()
   local beforeInput=scalars(config.input);local beforePlaceholder=scalars(config.placeholder)
   d.apply(value,player,enemy,build)
   assert(equal(beforeInput,scalars(config.input))and equal(beforePlaceholder,scalars(config.placeholder)))
   matrix[#matrix+1]={key=r.key,value=value,records=records(player),enemy_records=records(enemy)}
  end
 end
end
assert(input==config.input and placeholder==config.placeholder and equal(saved,snapshot()),"isolated callbacks changed selected source state")
local history={}
if rewardSwitch then
 assert(config.activeConfigSetId==1 and config.configSets[2])
 for _,id in ipairs({2,1,2,1})do config:SetActiveConfigSet(id);build.calcsTab:BuildOutput();history[#history+1]=snapshot()end
 assert(equal(saved,snapshot()),"restored source config/output differs after selection history")
end
local raw,err=common.xml.ParseXML(rewardXml);assert(raw and not err)
local rawConfigs={};for _,row in ipairs(raw[1])do if type(row)=="table"and row.elem=="Config"then rawConfigs[#rawConfigs+1]=plain(row)end end
return {dynamic_census=rows,excluded_source_rows=excluded,raw_config=rawConfigs,saved=saved,sets=sourceSets,matrix=matrix,history=history,method_sources=rewardOriginals.auth,original_methods_preserved=true,business_method_wrappers=false,selected_state_and_outputs_preserved=true}
