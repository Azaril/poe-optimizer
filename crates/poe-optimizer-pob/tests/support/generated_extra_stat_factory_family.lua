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
 local observations={}
 for _,id in ipairs(commands) do
  local entry={id=id,name=data.skills[id].name,name_lookup_present=resolve(data.skills[id].name,true)~=nil,maps={},controls={}}
  -- A key is not an absence proof: each key is looked up on a separate copy.
  for _,key in ipairs(keys) do
   local effect=isolated(id);local map=effect.statSets[1].statMap[key]
   entry.maps[#entry.maps+1]={key=key,mapped=map~=nil,global=effect.hasGlobalEffect==true}
  end
  local function consume(mode,value)
   local effect=isolated(id);local store=new("ModList")
   if mode=="fixed_family" then
    for _,mod in ipairs(all) do store:AddMod(copyTable(mod)) end
   else
    local mod=copyTable(purifying);mod.value.value=value
    if mode=="unscoped" then mod[1]=nil
    else assert(mode=="retargeted");mod[1]={type="SkillId",skillId=id} end
    store:AddMod(mod)
   end
   local extra=list(store,cfg(effect),"ExtraSkillStat")
   local output=new("ModList")
   merge({useAltGemQualityStats=false},output,{grantedEffect=effect,level=1,quality=0},effect.statSets[1],extra)
   local count,observed=0,nil
   for _,mod in ipairs(output) do
    charge();if mod.name=="DamageTakenConsecratedGround" then
     count=count+1;observed=plain(mod)
    end
   end
   return {mode=mode,value=value,delivered=#extra,global=effect.hasGlobalEffect==true,
    mapped=rawget(effect.statSets[1].statMap,global_key)~=nil,global_records=count,record=observed}
  end
  entry.controls[#entry.controls+1]=consume("fixed_family",0)
  for _,mode in ipairs({"unscoped","retargeted"}) do
   entry.controls[#entry.controls+1]=consume(mode,17)
   entry.controls[#entry.controls+1]=consume(mode,0)
  end
  observations[#observations+1]=entry
 end
 for _,id in ipairs(commands) do assert(same(shared[id],snapshot(data.skills[id])),"shared skill warmed") end
 for _,key in ipairs(keys) do assert(same(global_before[key],plain(rawget(data.skillStatMap,key))),"global map changed") end
 assert(parser==modLib.parseMod and list==common.classes.ModStore.List and eval==common.classes.ModStore.EvalMod)
 assert(merge==calcs.mergeSkillInstanceMods and resolve==calcLib.getGameIdFromGemName)
 return {records=records,cached_records=cached_records,commands=observations,work=work,
  shared_metadata_preserved=true,methods_preserved=true,supplier_closure=false,transform_closure=false,
  native_inventory_authority=false,current_original_field_closure=false}
end
