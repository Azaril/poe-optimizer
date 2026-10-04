-- Independent calls to the original pure stat assembler. No source table,
-- activeEffect, calculation method, or build setting is changed by these probes.
assert(djinnOriginals and djinnOriginals.preserved_after_load)
assert(not debug.gethook() and jit.status() == iceOccurrenceJit)
local assemble = assert(calcLib.buildSkillInstanceStats)
local info = debug.getinfo(assemble, "S")
local path = info.source:gsub("\\", "/")
assert(info.what == "Lua" and path:sub(-#"Modules/CalcTools.lua") == "Modules/CalcTools.lua" and info.linedefined == 161)
iceIntrinsicFunction = iceIntrinsicFunction or assemble
assert(iceIntrinsicFunction == assemble)
local effect = assert(data.skills.IceNovaPlayer)
local gem = assert(data.gems["Metadata/Items/Gems/SkillGemIceNova"])
assert(gem.grantedEffect == effect and #gem.grantedEffectList == 1 and gem.grantedEffectList[1] == effect)
assert(#effect.statSets == 2)

local function finite(v)
 assert(type(v) == "number" and v == v and v ~= math.huge and v ~= -math.huge)
 return v
end
local function scalars(t)
 local out,n = {},0
 for k,v in pairs(t or {}) do
  n=n+1;assert(n<=128)
  if type(k)=="string" and (type(v)=="number" or type(v)=="string" or type(v)=="boolean") then
   if type(v)=="number" then finite(v) end
   out[k]=v
  end
 end
 return out
end
local function dense(t, max)
 local n=0
 for k in pairs(t or {}) do assert(type(k)=="number" and k%1==0 and k>=1 and k<=max);n=n+1 end
 assert(n==#(t or {}))
 return n
end
local function same(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not same(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end
 return true
end
local function stats(t)
 local out,n={},0
 for k,v in pairs(t) do
  n=n+1;assert(n<=64 and type(k)=="string" and #k<=256)
  out[k]=finite(v)
 end
 return out
end
local function entries(t)
 local out={};dense(t,128)
 for index,row in ipairs(t or {}) do
  assert(type(row[1])=="string" and #row[1]<=256)
  out[#out+1]={index=index,stat=row[1],value_present=row[2]~=nil,value=row[2] and finite(row[2])}
 end
 return out
end
local function level_row(row)
 local positions={}
 for k,v in pairs(row) do
  if type(k)=="number" then
   assert(k%1==0 and k>=1 and k<=128)
   positions[#positions+1]={index=k,value=finite(v)}
  end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 local interpolation={};dense(row.statInterpolation,128)
 for i,v in ipairs(row.statInterpolation or {}) do interpolation[#interpolation+1]={index=i,value=finite(v)} end
 return {positions=positions,fields=scalars(row),interpolation=interpolation,cost=row.cost and scalars(row.cost)}
end
local function snapshot()
 assert(dense(effect.levels,64)==40 and dense(effect.statSets,64)==2)
 local out={effect=effect.id,cast_time=effect.castTime,quality=entries(effect.qualityStats),alt_quality=entries(effect.altQualityStats),common_levels={},stat_sets={}}
 for level,row in ipairs(effect.levels) do out.common_levels[#out.common_levels+1]={level=level,row=level_row(row)} end
 for index,set in ipairs(effect.statSets) do
  local names={};dense(set.stats,128)
  for i,name in ipairs(set.stats) do assert(type(name)=="string");names[#names+1]={index=i,stat=name} end
  assert(dense(set.levels,64)==40)
  local levels={}
  for level,row in ipairs(set.levels) do levels[#levels+1]={level=level,row=level_row(row)} end
  out.stat_sets[#out.stat_sets+1]={index=index,label=set.label,scope=set.statDescriptionScope,
   base_effectiveness=set.baseEffectiveness,incremental_effectiveness=set.incrementalEffectiveness,
   damage_incremental_effectiveness=set.damageIncrementalEffectiveness,
   stats=names,constants=entries(set.constantStats),levels=levels}
 end
 return out
end
local before=snapshot()
local function invoke(name,index,instance,alt,missingSet)
 local input=scalars(instance)
 local ok,value=pcall(assemble,instance,effect,not missingSet and effect.statSets[index] or nil,alt)
 assert(same(input,scalars(instance)),"pure helper changed caller input")
 local result={name=name,stat_set=index,level_present=instance.level~=nil,quality_present=instance.quality~=nil,
  instance=input,include_alt_quality=alt,constructed_stat_set=not missingSet,success=ok}
 if ok then result.stats=stats(value) else result.error=tostring(value);assert(#result.error<=4096) end
 return result
end
local admitted,boundaries={},{}
for index=1,2 do
 for level=1,40 do
  admitted[#admitted+1]=invoke("level-"..level,index,{level=level,quality=0},false)
 end
 for _,control in ipairs({{"below-domain",0},{"above-domain",41},{"negative-level",-1},{"fractional-level",1.5},{"text-level","invalid"},{"missing-level"}}) do
  boundaries[#boundaries+1]=invoke(control[1],index,{level=control[2],quality=0},false)
 end
 for _,quality in ipairs({-1,0,0.5,1.5,20.5}) do
  for _,alt in ipairs({false,true}) do
   boundaries[#boundaries+1]=invoke("quality-"..quality.."-alt-"..tostring(alt),index,{level=17,quality=quality},alt)
  end
 end
 boundaries[#boundaries+1]=invoke("missing-quality",index,{level=17},false)
 boundaries[#boundaries+1]=invoke("missing-stat-set",index,{level=17,quality=0},false,true)
end
local actual={}
for _,mode in ipairs({"MAIN","CALCS"}) do
 local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 for _,action in ipairs(env.player.activeSkillList) do
  local instance=action.activeEffect
  if instance.grantedEffect==effect then
   local source=assert(instance.srcInstance)
   local ordinal,ref
   for key,candidate in pairs(iceSavedPhysicalReferences or {}) do
    if candidate.gem==source then assert(not ordinal);ordinal,ref=key,candidate end
   end
   assert(ordinal and action.socketGroup==ref.group and action.actor==env.player)
   local selected=assert(mode=="MAIN" and instance.statSet or instance.statSetCalcs)
   assert(selected.statSet==effect.statSets[selected.index])
   local inputs={level=instance.level,quality=instance.quality,actorLevel=instance.actorLevel}
   local result=invoke("prepared-occurrence",selected.index,instance,env.useAltGemQualityStats==true)
   assert(result.success and same(inputs,{level=instance.level,quality=instance.quality,actorLevel=instance.actorLevel}))
   actual[#actual+1]={mode=mode,source_ordinal=ordinal,physical_source_exact=true,constructed_table_exact=true,
    selected=selected.index,prepared=inputs,loaded=scalars(source),stats=result.stats,
    include_alt_quality=env.useAltGemQualityStats==true,skill_data=scalars(action.skillData),
    prepared_level_row=instance.grantedEffectLevel and level_row(instance.grantedEffectLevel),
    source_level_row=effect.levels[instance.level] and level_row(effect.levels[instance.level]),
    selected_level_row=selected.statSet.levels[instance.level] and level_row(selected.statSet.levels[instance.level]),
    output_available=action.output~=nil,is_main_skill=action==env.player.mainSkill}
  end
 end
end
assert(#admitted==80 and #boundaries==36 and #actual<=32)
assert(same(before,snapshot()),"original helper inputs were modified")
assert(calcLib.buildSkillInstanceStats==assemble and not debug.gethook() and jit.status()==iceOccurrenceJit)
return {function_identity={path="src/Modules/CalcTools.lua",first=info.linedefined,last=info.lastlinedefined},
 constructed=before,admitted=admitted,boundaries=boundaries,actual=actual,
 original_function_preserved=true,constructed_inputs_preserved=true,caller_inputs_preserved=true,
 requested_jit_verified=true,helper_probe_is_final_input_authority=false}
