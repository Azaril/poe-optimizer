-- Optional source diagnostics only. No inferred absence or native input policy.
return function(requested, stat_keys, lookup_names, phase)
 local work=0
 local function charge(n) work=work+(n or 1);assert(work<=1000000,"extra-stat witness work") end
 local function original(fn,path,line)
  local info=debug.getinfo(fn,"S");local name=info.source:gsub("\\","/")
  assert(info.what=="Lua" and name:sub(-#path)==path and info.linedefined==line)
  return fn
 end
 local resolve=original(calcLib.getGameIdFromGemName,"Modules/CalcTools.lua",259)
 local list=original(common.classes.ModStore.List,"Classes/ModStore.lua",321)
 local eval=original(common.classes.ModStore.EvalMod,"Classes/ModStore.lua",490)
 local function plain(value,depth)
  charge();depth=depth or 0;assert(depth<16)
  local kind=type(value)
  if kind~="table" then
   assert(kind=="nil" or kind=="boolean" or kind=="string" or kind=="number")
   if kind=="string" then charge(#value) end
   if kind=="number" and (value~=value or value==math.huge or value==-math.huge) then
    return {source_number="non_finite",diagnostic=tostring(value)}
   end
   return value
  end
  assert(getmetatable(value)==nil)
  -- JSON cannot represent Lua's mixed numeric/string table keys. Keep dense
  -- sequences as arrays and encode mixed-map numeric keys explicitly as text.
  local count,largest,sequence=0,0,true
  for key in next,value do
   charge();count=count+1
   if type(key)~="number" or key<1 or key%1~=0 then sequence=false
   else largest=math.max(largest,key) end
  end
  sequence=sequence and count>0 and largest==count
  local result={}
  for key,item in next,value do
   charge();assert(type(key)=="number" or type(key)=="string")
   result[not sequence and type(key)=="number" and tostring(key) or key]=plain(item,depth+1)
  end
  return result
 end
 local function present(value) return {present=value~=nil,value=plain(value)} end
 local function same(a,b)
  charge();if type(a)~=type(b) then return false end
  if type(a)~="table" then return a==b end
  for k,v in pairs(a) do if not same(v,b[k]) then return false end end
  for k in pairs(b) do if a[k]==nil then return false end end
  return true
 end
 local function metadata()
  local rows={}
  for _,id in ipairs(requested) do
   charge();local effect=assert(rawget(data.skills,id));local parts={effect}
   for _,part in ipairs(effect.statSets) do parts[#parts+1]=part end
   local row={id=id,name=effect.name,game_id=present(resolve(effect.name,true)),
    has_global_effect=present(rawget(effect,"hasGlobalEffect")),parts={}}
   for index,part in ipairs(parts) do
    assert(getmetatable(part.statMap)==data.skillStatMapMeta)
    assert(rawget(part.statMap,"_grantedEffect")==effect)
    local maps={}
    for _,stat in ipairs(stat_keys) do
     maps[#maps+1]={stat=stat,local_entry=present(rawget(part.statMap,stat)),
      global_entry=present(rawget(data.skillStatMap,stat))}
    end
    row.parts[#row.parts+1]={index=index,maps=maps}
   end
   rows[#rows+1]=row
  end
  return rows
 end
 local result={metadata=metadata(),lookups={},extra_stat_scope_proved=false,
  native_inventory_authority=false,source_methods_preserved=true}
 for _,name in ipairs(lookup_names) do
  result.lookups[#result.lookups+1]={name=name,game_id=present(resolve(name,true))}
 end
 if phase=="before_build" then result.work=work;return result end
 assert(phase=="stage" and djinnOriginals.preserved_after_load and sniperLoaderHookRemoved)
 local base=assert(extraStatBase)
 local wanted={};for _,id in ipairs(requested) do wanted[id]=true end
 local function scalars(t)
  local r={};for k,v in pairs(t) do
   charge();if type(k)=="string" and (type(v)=="string" or type(v)=="number" or type(v)=="boolean") then
    r[k]=plain(v)
   end
  end;return r
 end
 local function outputs()
  return {MAIN=scalars(build.calcsTab.mainOutput),CALCS=scalars(build.calcsTab.calcsOutput)}
 end
 local before=outputs();result.outputs=before;result.groups={};result.modes={}
 local function raw_mods(store)
  local records={};local seen={}
  while store do
   charge();assert(not seen[store]);seen[store]=true
   assert(#records<1024)
   local mods=rawget(store,"mods")
   local candidates=mods and rawget(mods,"ExtraSkillStat") or store
   for _,mod in ipairs(candidates or {}) do
    charge();if mod.name=="ExtraSkillStat" then records[#records+1]=plain(mod) end
   end
   store=rawget(store,"parent")
  end
  return records
 end
 for _,mode in ipairs({"MAIN","CALCS"}) do
  local env=mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
  local row={player_records=raw_mods(env.player.modDB),actions={}}
  for _,skill in ipairs(env.player.activeSkillList) do
   charge();local effect=skill.activeEffect;local id=effect.grantedEffect.id
   if wanted[id] then
    assert(skill.actor==env.player and skill.skillModList.List==list)
    assert(skill.skillModList.EvalMod==eval)
    local cfg=skill.skillCfg;local group,gem_index
    for _,r in ipairs(base.runtime_groups) do
     if r.selected then
      local g=build.skillsTab.skillSets[r.preset].socketGroupList[r.index]
      if g==skill.socketGroup then
       assert(not group);group=r
       for i,gem in ipairs(g.gemList) do if gem==effect.srcInstance then assert(not gem_index);gem_index=i end end
      end
     end
    end
    assert(group and gem_index and cfg.skillGrantedEffect==effect.grantedEffect)
    row.actions[#row.actions+1]={effect=id,source_ordinal=group.source_ordinal,
     gem_source_ordinal=group.gems[gem_index].source_ordinal,source=group.state.fields.source,
     cfg={skill_name=cfg.skillName,game_id=present(resolve(cfg.skillName,true)),
      -- Diagnostic value of the actual non-summon includeTransfigured branch
      -- in ModStore.lua:895. A missing cfg resolution becomes an empty string;
      -- the tag-side missing resolution remains nil and therefore cannot match.
      include_transfigured_match_game_id=cfg and cfg.skillName and resolve(cfg.skillName,true) or "",
      effect_id=cfg.skillGrantedEffect.id,summon_skill_name=present(cfg.summonSkillName)},
     extra_stats=plain(list(skill.skillModList,cfg,"ExtraSkillStat")),
     raw_records=raw_mods(skill.skillModList)}
   end
  end
  result.modes[mode]=row
 end
 for _,runtime in ipairs(base.runtime_groups) do
  if runtime.selected then
   local group=build.skillsTab.skillSets[runtime.preset].socketGroupList[runtime.index]
   local first=group.gemList[1]
   if first and wanted[first.skillId] then
    local row={source_ordinal=runtime.source_ordinal,source=group.source,effects={}}
    for i,effect in ipairs(first.gemData.grantedEffectList) do
     row.effects[#row.effects+1]={id=effect.id,global_field="enableGlobal"..i,
      global_value=present(first["enableGlobal"..i]),has_global_effect=present(rawget(effect,"hasGlobalEffect"))}
    end
    result.groups[#result.groups+1]=row
   end
  end
 end
 result.custom_blocks=plain(build.configTab.configSets[build.configTab.activeConfigSetId].customModsList)
 result.parser_cache={}
 for _,block in ipairs(result.custom_blocks) do
  if block.enabled and block.text~="" then
   result.parser_cache[#result.parser_cache+1]={line=block.text,entry=present(rawget(modLib.parseModCache,block.text))}
  end
 end
 assert(same(result.metadata,metadata()),"observer changed lazy metadata")
 assert(same(before,outputs()),"observer changed outputs")
 assert(calcLib.getGameIdFromGemName==resolve and common.classes.ModStore.List==list and common.classes.ModStore.EvalMod==eval)
 assert(not debug.gethook() and jit.status()==sniperActorJit)
 result.outputs_preserved=true;result.observer_warmed_source_metadata=false;result.work=work
 return result
end
