-- Finite original-parser witness only. This does not close suppliers or transforms.
-- Every lazy lookup below uses an isolated copy; shared skill metadata is never warmed.
return function(lines, commands, keys, global_key, cached_line, first)
 local calcs=require("Modules.CalcBase")
 local work=0
 local function charge(n) work=work+(n or 1);assert(work<=1000000,"factory witness work") end
 local function original(fn,path,line)
  local info=debug.getinfo(fn,"S");local name=info.source:gsub("\\","/")
  assert(info.what=="Lua" and name:sub(-#path)==path and info.linedefined==line)
  return fn
 end
 local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
 local list=original(common.classes.ModStore.List,"Classes/ModStore.lua",321)
 local eval=original(common.classes.ModStore.EvalMod,"Classes/ModStore.lua",490)
 local merge=original(calcs.mergeSkillInstanceMods,"Modules/CalcActiveSkill.lua",116)
 local resolve=original(calcLib.getGameIdFromGemName,"Modules/CalcTools.lua",259)
 local scale_add=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82)
 local copy_list=original(common.classes.ModStore.CopyList,"Classes/ModStore.lua",121)
 local scale_list=original(common.classes.ModStore.ScaleAddList,"Classes/ModStore.lua",127)
 local set_source=original(modLib.setSource,"Modules/ModTools.lua",277)
 local db_add=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31)
 local flat_add=original(common.classes.ModList.AddMod,"Classes/ModList.lua",29)
 local db_list=original(common.classes.ModDB.AddList,"Classes/ModDB.lua",113)
 local flat_list=original(common.classes.ModList.AddList,"Classes/ModList.lua",90)
 local function upvalue(fn,wanted)
  local found,finished
  for index=1,64 do
   charge();local name,value=debug.getupvalue(fn,index)
   if not name then finished=true;break end
   if name==wanted then assert(found==nil);found=value end
  end
  assert(finished and found~=nil,"missing/bounded constructor upvalue: "..wanted)
  return found
 end
 local constructors={}
 for kind,line in pairs({ModList=24,ModDB=25}) do
  local wrapper=original(common.classes[kind][kind],"Modules/Common.lua",167)
  constructors[kind]={wrapper=wrapper,
   body=original(upvalue(wrapper,"originalFunc"),"Classes/"..kind..".lua",line)}
 end
 local function make_store(kind,parent)
  local value=new(kind)
  assert(value[kind]==constructors[kind].wrapper)
  assert(rawequal(value[kind](value,parent),value))
  assert(rawequal(value.parent,parent or false))
  return value
 end
 original(data.skillStatMapMeta.__index,"Modules/Data.lua",914)
 local cache,finished
 for index=1,256 do
  charge();local name,value=debug.getupvalue(parser,index)
  if not name then finished=true;break end
  if name=="cache" then assert(not cache);cache=value end
 end
 assert(finished and rawequal(cache,modLib.parseModCache) and getmetatable(cache)==nil)
 local function plain(value,depth)
  charge();depth=depth or 0;assert(depth<16)
  if type(value)~="table" then
   assert(type(value)=="nil" or type(value)=="boolean" or type(value)=="string" or type(value)=="number")
   if type(value)=="string" then charge(#value) end
   if type(value)=="number" then assert(value==value and math.abs(value)~=math.huge) end
   return value
  end
  assert(getmetatable(value)==nil)
  local result={}
  for key,item in next,value do
   charge();assert(type(key)=="string" or type(key)=="number")
   result[tostring(key)]=plain(item,depth+1)
  end
  return result
 end
 local function same(a,b)
  charge();if type(a)~=type(b) then return false end
  if type(a)~="table" then return a==b end
  for k,v in pairs(a) do if not same(v,b[k]) then return false end end
  for k in pairs(b) do if a[k]==nil then return false end end
  return true
 end
 local function snapshot(effect)
  local result={global=rawget(effect,"hasGlobalEffect"),parts={}}
  for _,part in ipairs(tableConcat({effect},effect.statSets)) do
   charge();assert(getmetatable(part.statMap)==data.skillStatMapMeta)
   assert(rawequal(rawget(part.statMap,"_grantedEffect"),effect))
   local entries={};for key,value in next,part.statMap do
    charge();if key~="_grantedEffect" then entries[key]=plain(value) end
   end
   result.parts[#result.parts+1]=entries
  end
  return result
 end
 local shared={};for _,id in ipairs(commands) do shared[id]=snapshot(assert(data.skills[id])) end
 local global_before={};for _,key in ipairs(keys) do global_before[key]=plain(rawget(data.skillStatMap,key)) end
 local function isolated(id)
  local source=assert(data.skills[id]);local effect=copyTable(source,true)
  assert(rawget(source,"hasGlobalEffect")==nil)
  local function part_copy(part)
   local result=copyTable(part,true);local map={_grantedEffect=effect}
   for key,value in next,part.statMap do
    charge();if key~="_grantedEffect" then map[key]=copyTable(value) end
   end
   result.statMap=setmetatable(map,data.skillStatMapMeta)
   result.baseMods=copyTable(part.baseMods or {})
   return result
  end
  effect.statMap=part_copy(source).statMap;effect.statSets={}
  for _,part in ipairs(source.statSets) do effect.statSets[#effect.statSets+1]=part_copy(part) end
  return effect
 end
 local function cfg(effect) return {skillName=effect.name,skillGrantedEffect=effect,flags=0,keywordFlags=0} end
 local records,all,purifying={}, {},nil
 for _,line in ipairs(lines) do
  charge(#line);if first then assert(rawget(cache,line)==nil,"expected uncached factory input: "..line) end
  local mods,remainder=parser(line);assert(mods and remainder==nil)
  local again,again_remainder=parser(line);assert(again_remainder==nil and same(mods,again))
  local row={line=line,mods={}}
  for _,mod in ipairs(mods) do
   row.mods[#row.mods+1]=plain(mod)
   if mod.name=="ExtraSkillStat" then
    all[#all+1]=mod
    if mod.value.key==global_key then assert(not purifying);purifying=mod end
   end
  end
  records[#records+1]=row
 end
 assert(#all==14 and purifying)
 assert(rawget(cache,cached_line),"canonical Bloodbarrier must use the loaded cache")
 local cached,remainder=parser(cached_line);assert(cached and remainder==nil)
 local cached_records={};for _,mod in ipairs(cached) do cached_records[#cached_records+1]=plain(mod) end
 local function consume(id,store,mode,value)
  local effect=isolated(id)
  local extra=list(store,cfg(effect),"ExtraSkillStat")
  local output=make_store("ModList")
  merge({useAltGemQualityStats=false},output,{grantedEffect=effect,level=1,quality=0},effect.statSets[1],extra)
  local count,observed=0,nil
  for _,mod in ipairs(output) do
   charge();if mod.name=="DamageTakenConsecratedGround" then count=count+1;observed=plain(mod) end
  end
  return {mode=mode,value=value,delivered=#extra,global=effect.hasGlobalEffect==true,
   mapped=rawget(effect.statSets[1].statMap,global_key)~=nil,global_records=count,record=observed}
 end
 local observations={}
 for _,id in ipairs(commands) do
  local entry={id=id,name=data.skills[id].name,name_lookup_present=resolve(data.skills[id].name,true)~=nil,maps={},controls={}}
  -- A key is not an absence proof: each key is looked up on a separate copy.
  for _,key in ipairs(keys) do
   local effect=isolated(id);local map=effect.statSets[1].statMap[key]
   entry.maps[#entry.maps+1]={key=key,mapped=map~=nil,global=effect.hasGlobalEffect==true}
  end
  local function control(mode,value)
   local destination=make_store("ModList")
   if mode=="fixed_family" then
    for _,mod in ipairs(all) do destination:AddMod(copyTable(mod)) end
   else
    local mod=copyTable(purifying);mod.value.value=value
    if mode=="unscoped" then mod[1]=nil
    else assert(mode=="retargeted");mod[1]={type="SkillId",skillId=id} end
    destination:AddMod(mod)
   end
   return consume(id,destination,mode,value)
  end
  entry.controls[#entry.controls+1]=control("fixed_family",0)
  for _,mode in ipairs({"unscoped","retargeted"}) do
   entry.controls[#entry.controls+1]=control(mode,17)
   entry.controls[#entry.controls+1]=control(mode,0)
  end
  observations[#observations+1]=entry
 end
 -- This admitted shape is the complete literal factory output above, not an
 -- arbitrary LIST payload: no nested mod, keyOfScaledMod, or custom metatable.
 -- ScaleAddMod therefore never enters a numeric-value branch for these records.
 local originals=plain(all)
 for _,mod in ipairs(all) do
  assert(mod.name=="ExtraSkillStat" and mod.type=="LIST" and getmetatable(mod)==nil)
  assert(type(mod.value)=="table" and getmetatable(mod.value)==nil)
  assert(mod.value.mod==nil and mod.value.keyOfScaledMod==nil)
 end
 local transports={}
 local function transport(kind,operation,scale,round_nearest)
  local inputs=copyTable(all);local before=plain(inputs);local destination=make_store(kind)
  assert(destination.parent==false)
  if operation=="AddMod" then
   for _,mod in ipairs(inputs) do destination:AddMod(mod) end
  elseif operation=="CopyList" then copy_list(destination,inputs)
  elseif operation=="ScaleAddMod" then
   for _,mod in ipairs(inputs) do scale_add(destination,mod,scale,round_nearest) end
  elseif operation=="ScaleAddList" then scale_list(destination,inputs,scale,round_nearest)
  else
   assert(operation=="setSource_then_copy_scale")
   for _,mod in ipairs(inputs) do assert(rawequal(set_source(mod,"Factory witness:replacement"),mod)) end
   -- Preserve a nonempty source label through the same ordinary transport.
   local copied=make_store(kind);copy_list(copied,inputs)
   local carried=kind=="ModDB" and copied.mods.ExtraSkillStat or copied
   scale_list(destination,carried,0.5,true)
  end
  local actual
  if kind=="ModDB" then
   assert(next(destination.mods)=="ExtraSkillStat" and next(destination.mods,"ExtraSkillStat")==nil)
   actual=assert(destination.mods.ExtraSkillStat)
  else actual=destination end
  assert(#actual==14)
  local row={store=kind,operation=operation,scale=scale,round_to_nearest=round_nearest,records={},commands={}}
  local expected=copyTable(all)
  if operation=="setSource_then_copy_scale" then
   for _,mod in ipairs(expected) do mod.source="Factory witness:replacement" end
   assert(same(plain(inputs),plain(expected)),"setSource changed more than provenance")
  else assert(same(before,plain(inputs)),"transport changed its source record") end
  local positive
  for i,mod in ipairs(actual) do
   assert(same(plain(mod),plain(expected[i])),"ordinary transport changed a complete record")
   row.records[#row.records+1]=plain(mod)
   if mod.value.key==global_key then assert(not positive);positive=mod end
  end
  for _,id in ipairs(commands) do
   -- The actual skill store inherits its supplier. Do not reconstitute a
   -- separate empty store and mistake it for successful transport.
   local inherited=make_store("ModList",destination)
   assert(rawequal(inherited.parent,destination))
   local fixed=consume(id,inherited,"fixed_family",0)
   local changed=copyTable(assert(positive));changed[1]={type="SkillId",skillId=id}
   local control_store=make_store(kind);control_store:AddMod(changed)
   local delivered=consume(id,make_store("ModList",control_store),"retargeted",17)
   row.commands[#row.commands+1]={id=id,fixed=fixed,retargeted=delivered}
  end
  assert(same(originals,plain(all)),"parser-produced records changed")
  row.originals_preserved=true
  transports[#transports+1]=row
 end
 for _,kind in ipairs({"ModList","ModDB"}) do
  transport(kind,"AddMod")
  transport(kind,"CopyList")
  transport(kind,"setSource_then_copy_scale")
  for _,operation in ipairs({"ScaleAddMod","ScaleAddList"}) do
   for _,scale in ipairs({0,0.5,1,2}) do
    for _,nearest in ipairs({false,true}) do transport(kind,operation,scale,nearest) end
   end
  end
 end
 for _,id in ipairs(commands) do assert(same(shared[id],snapshot(data.skills[id])),"shared skill warmed") end
 for _,key in ipairs(keys) do assert(same(global_before[key],plain(rawget(data.skillStatMap,key))),"global map changed") end
 assert(parser==modLib.parseMod and list==common.classes.ModStore.List and eval==common.classes.ModStore.EvalMod)
 assert(merge==calcs.mergeSkillInstanceMods and resolve==calcLib.getGameIdFromGemName)
 assert(scale_add==common.classes.ModStore.ScaleAddMod and copy_list==common.classes.ModStore.CopyList)
 assert(scale_list==common.classes.ModStore.ScaleAddList and set_source==modLib.setSource)
 assert(db_add==common.classes.ModDB.AddMod and flat_add==common.classes.ModList.AddMod)
 assert(db_list==common.classes.ModDB.AddList and flat_list==common.classes.ModList.AddList)
 for kind,constructor in pairs(constructors) do
  assert(constructor.wrapper==common.classes[kind][kind])
  assert(upvalue(constructor.wrapper,"originalFunc")==constructor.body)
 end
 return {records=records,cached_records=cached_records,commands=observations,transports=transports,work=work,
  shared_metadata_preserved=true,methods_preserved=true,supplier_closure=false,transform_closure=false,
  native_inventory_authority=false,current_original_field_closure=false}
end
