-- Read the unchanged complete loader state. No business method is replaced.
assert(jit.status()==characterRewardJit,"unexpected source JIT mode")
local function original(f,path,line)
 assert(type(f)=="function","missing original "..path..":"..line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==line,"unexpected original "..s..":"..i.linedefined.." expected "..path..":"..line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,key)
 for i=1,64 do local k,v=debug.getupvalue(f,i);if not k then break end;if k==key then return v end end
 error("missing original upvalue "..key)
end
local function plain(v,depth)
 if type(v)~="table"then
  assert(v==nil or type(v)=="string"or type(v)=="number"or type(v)=="boolean")
  if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end
  return v
 end
 depth=(depth or 0)+1;assert(depth<20);local r={};local count,maximum,array=0,0,true
 for k in pairs(v)do
  count=count+1;assert(count<20000)
  if type(k)=="number"and k>=1 and k==math.floor(k)then maximum=math.max(maximum,k)else array=false end
 end
 array=array and maximum==count
 for k,x in pairs(v)do
  -- JSON arrays retain dense sequence keys. Sparse node-ID maps and XML's
  -- mixed element/attribute tables require object keys; never alter the source.
  assert(type(k)=="string"or type(k)=="number")
  local key=array and k or tostring(k);assert(r[key]==nil,"ambiguous snapshot key")
  r[key]=plain(x,depth)
 end;return r
end
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="string"or type(v)=="number"or type(v)=="boolean"then r[k]=plain(v)end end;return r end
local function equal(a,b)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local calcs=require("Modules.CalcBase")
local methods={
 {"build_init",build,"Init","Modules/Build.lua",34},
 {"build_load_db",build,"LoadDB","Modules/Build.lua",2691},
 {"build_load",build,"Load","Modules/Build.lua",1138},
 {"build_save",build,"Save","Modules/Build.lua",1180},
 {"conversion_popup",build,"OpenConversionPopup","Modules/Build.lua",1543},
 {"progress",build,"EstimatePlayerProgress","Modules/Build.lua",1025},
 {"tree_load",common.classes.TreeTab,"Load","Classes/TreeTab.lua",492},
 {"tree_post_load",common.classes.TreeTab,"PostLoad","Classes/TreeTab.lua",521},
 {"tree_select",common.classes.TreeTab,"SetActiveSpec","Classes/TreeTab.lua",540},
 {"spec_load",common.classes.PassiveSpec,"Load","Classes/PassiveSpec.lua",117},
 {"spec_post_load",common.classes.PassiveSpec,"PostLoad","Classes/PassiveSpec.lua",353},
 {"spec_import",common.classes.PassiveSpec,"ImportFromNodeList","Classes/PassiveSpec.lua",358},
 {"spec_counts",common.classes.PassiveSpec,"CountAllocNodes","Classes/PassiveSpec.lua",1029},
 {"config_load",common.classes.ConfigTab,"Load","Classes/ConfigTab.lua",878},
 {"config_mods",common.classes.ConfigTab,"BuildModList","Classes/ConfigTab.lua",1169},
 {"api_quest_import",common.classes.ImportTab,"ImportQuestRewardConfig","Classes/ImportTab.lua",829},
 {"output",common.classes.CalcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {"init_env",calcs,"initEnv","Modules/CalcSetup.lua",717},
 {"launch_error",launch,"ShowErrMsg","Launch.lua",379},
 {"open_popup",main,"OpenPopup","Modules/Main.lua",1681},
}
-- Observe the exact varList consumed by the original ConfigTab method, rather
-- than loading a second independent catalogue copy.
local varList=upvalue(common.classes.ConfigTab.BuildModList,"varList")
local byKey={};for _,d in ipairs(varList)do if d.var then if d.var:sub(1,5)=="quest"then assert(not byKey[d.var])end;byKey[d.var]=d end end
local questRows,callbacks={},{}
for index,q in ipairs(data.questRewards)do
 local key="quest"..q.Description..q.Area..q.Info;local d=byKey[key]
 local row={source_index=index,key=key,quest=plain(q),config_control_present=d~=nil}
 if q.useConfig then
  assert(d);local line=q.Stat and 82 or 98
  local callback,where=original(d.apply,"Modules/ConfigOptions.lua",line)
  assert(upvalue(callback,"source")=="Quest:"..q.Description..": "..q.Area)
  row.callback=where;row.widget=d.type;row.source=upvalue(callback,"source")
  callbacks[key]=callback
 else assert(q.useConfig==false and not d and q.questPoints)end
 questRows[#questRows+1]=row
end
if characterRewardPhase=="before"then
 local refs,auth={},{};for _,m in ipairs(methods)do refs[m[1]],auth[m[1]]=original(m[2][m[3]],m[4],m[5])end
 assert(liveTargetVersion=="0_1"and legacyTargetVersion=="0_0")
 local versionEntries=0;for _,path in ipairs(_configuration_source_modules)do if path:gsub("\\","/"):sub(-#"GameVersions.lua")=="GameVersions.lua"then versionEntries=versionEntries+1 end end;assert(versionEntries==1)
 characterRewardOriginals={refs=refs,auth=auth,callbacks=callbacks,var_list=varList,live_target=liveTargetVersion,legacy_target=legacyTargetVersion}
 return function()
  assert(jit.status()==characterRewardJit,"source changed global JIT mode")
  for _,m in ipairs(methods)do assert(m[2][m[3]]==refs[m[1]],"source method changed "..m[1])end
  assert(upvalue(common.classes.ConfigTab.BuildModList,"varList")==varList)
  for key,f in pairs(callbacks)do assert(byKey[key].apply==f)end
  assert(liveTargetVersion==characterRewardOriginals.live_target and legacyTargetVersion==characterRewardOriginals.legacy_target)
  characterRewardOriginals.finished=true
  local titles={};for _,popup in ipairs(main.popups)do titles[#titles+1]=popup.title end
  local raw=assert(common.xml.ParseXML(characterRewardXml));local target
  for _,row in ipairs(raw[1])do if type(row)=="table"and row.elem=="Build"then target=row.attrib.targetVersion;break end end
  return {methods_preserved=true,prompt=launch.promptMsg,main_mode=main.mode,pending_mode=main.newMode,popup_count=#main.popups,popup_titles=titles,live_target_version=liveTargetVersion,legacy_target_version=legacyTargetVersion,raw_target_version=target,raw_target_version_type=type(target),loaded_target_version=build.targetVersion,loaded_target_version_type=type(build.targetVersion)}
 end
end
assert(characterRewardPhase=="observe"and characterRewardOriginals.finished)
local function quest_records(list)
 local result={};for _,m in ipairs(list or{})do if type(m.source)=="string"and m.source:sub(1,6)=="Quest:"then
  local tags={};for _,tag in ipairs(m)do tags[#tags+1]=plain(tag)end
  result[#result+1]={name=m.name,type=m.type,value=plain(m.value),source=m.source,flags=m.flags,keyword_flags=m.keywordFlags,tags=tags}
 end end;return result
end
local function db_quests(db)
 local names={};for name in pairs(db.mods)do names[#names+1]=name end;table.sort(names)
 local result={};for _,name in ipairs(names)do for _,m in ipairs(quest_records(db.mods[name]))do result[#result+1]=m end end;return result
end
local function spec_state(spec)
 local allocated={};for id in pairs(spec.allocNodes)do allocated[#allocated+1]=id end;table.sort(allocated)
 return {title=spec.title,tree_version=spec.treeVersion,class_id=spec.curClassId,class_name=spec.curClassName,ascendancy_id=spec.curAscendClassId,ascendancy_name=spec.curAscendClassName,secondary_ascendancy_id=spec.curSecondaryAscendClassId,allocated_ids=allocated,counts={spec:CountAllocNodes()},jewels=plain(spec.jewels)}
end
local function snapshot()
 local specs={};for index,spec in ipairs(build.treeTab.specList)do specs[#specs+1]={index=index,state=spec_state(spec)}end
 local inputs={};for _,q in ipairs(questRows)do if q.config_control_present then
  local value=build.configTab.input[q.key];inputs[#inputs+1]={key=q.key,value=plain(value),value_type=type(value)}
 end end
 local modes={};for _,mode in ipairs({"MAIN","CALCS"})do
  local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  assert(env.configInput==build.configTab.input and env.configPlaceholder==build.configTab.placeholder)
  modes[mode]={quest=db_quests(env.modDB),player=scalars(env.player.output),minion_available=env.minion~=nil,minion=env.minion and scalars(env.minion.output),enemy_level=env.enemy.level,main_effect=env.player.mainSkill.activeEffect.grantedEffect.id,main_group=env.mainSocketGroup}
 end
 return {selected={spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,config=build.configTab.activeConfigSetId},character={target_version=build.targetVersion,level=build.characterLevel,auto=build.characterLevelAutoMode,class_name=build.spec.curClassName,ascendancy_name=build.spec.curAscendClassName},specs=specs,budget={acts=plain(build.acts),max_acts=build.maxActs,max_weapon_sets=build.maxWeaponSets},beasts=plain(build.beastList),spectres=plain(build.spectreList),timeless=plain(build.timelessData),config_inputs=inputs,config_quest=quest_records(build.configTab.modList),enemy_quest=quest_records(build.configTab.enemyModList),modes=modes}
end
local saved=snapshot()
local specList,selectedSpec,config,inputs,placeholders=build.treeTab.specList,build.spec,build.configTab,build.configTab.input,build.configTab.placeholder
local specRefs={};for i,spec in ipairs(specList)do specRefs[i]={spec=spec,allocated=spec.allocNodes,jewels=spec.jewels}end
local mainEnv,calcsEnv,mainOutput,calcsOutput=build.calcsTab.mainEnv,build.calcsTab.calcsEnv,build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local doc,err=common.xml.ParseXML(characterRewardXml);assert(doc and not err)
local ordinal={};local nextOrdinal=0
local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinal[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,child in ipairs(n)do enumerate(child)end end;enumerate(doc[1])
local raw={root={name=doc[1].elem,attributes=plain(doc[1].attrib)},builds={},trees={},legacy_specs={}}
for _,section in ipairs(doc[1])do if type(section)=="table"then
 if section.elem=="Build"then raw.builds[#raw.builds+1]={source=ordinal[section],xml=plain(section)}
 elseif section.elem=="Tree"then
  local specs={};for _,spec in ipairs(section)do if type(spec)=="table"and spec.elem=="Spec"then specs[#specs+1]={source=ordinal[spec],xml=plain(spec)}end end
  raw.trees[#raw.trees+1]={source=ordinal[section],attributes=plain(section.attrib),specs=specs}
 elseif section.elem=="Spec"then raw.legacy_specs[#raw.legacy_specs+1]={source=ordinal[section],xml=plain(section)}end
end end
assert(build.treeTab.specList==specList and build.spec==selectedSpec and build.configTab==config and config.input==inputs and config.placeholder==placeholders)
for i,refs in ipairs(specRefs)do assert(specList[i]==refs.spec and specList[i].allocNodes==refs.allocated and specList[i].jewels==refs.jewels)end
assert(build.calcsTab.mainEnv==mainEnv and build.calcsTab.calcsEnv==calcsEnv and build.calcsTab.mainOutput==mainOutput and build.calcsTab.calcsOutput==calcsOutput)
assert(equal(saved,snapshot()),"observation changed source state, selection or output")
assert(jit.status()==characterRewardJit,"observation changed global JIT mode")
for _,m in ipairs(methods)do assert(m[2][m[3]]==characterRewardOriginals.refs[m[1]])end
assert(varList==characterRewardOriginals.var_list)
for key,f in pairs(callbacks)do assert(f==characterRewardOriginals.callbacks[key])end
return {raw=raw,saved=saved,quest_catalogue=questRows,method_sources=characterRewardOriginals.auth,methods_preserved=true,objects_preserved=true,outputs_and_selection_preserved=true,business_method_wrappers=false,synthetic_reward_collection=false}
