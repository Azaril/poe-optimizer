-- Optional source diagnostics only. No inferred absence or native input policy.
return function(requested, stat_keys, lookup_names, control_lines, phase, item_probe)
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
 local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
 local item_build=original(common.classes.Item.BuildModList,"Classes/Item.lua",2694)
 local item_slot=original(common.classes.Item.BuildModListForSlotNum,"Classes/Item.lua",2414)
 local item_active=original(common.classes.Item.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
 local parser_cache,cache_slot,finished
 for index=1,256 do
  charge();local name,value=debug.getupvalue(parser,index)
  if not name then finished=true;break end
  if name=="cache" then assert(not cache_slot);parser_cache=value;cache_slot=index end
 end
 assert(finished and cache_slot and type(parser_cache)=="table" and getmetatable(parser_cache)==nil)
 assert(rawequal(parser_cache,modLib.parseModCache))
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
 local function control_cache(lines)
  local entries={}
  for _,line in ipairs(lines or control_lines) do
   charge(#line);entries[#entries+1]={line=line,entry=present(rawget(parser_cache,line))}
  end
  return entries
 end
 local function same(a,b)
  charge();if type(a)~=type(b) then return false end
  if type(a)~="table" then return a==b end
  for k,v in pairs(a) do if not same(v,b[k]) then return false end end
  for k in pairs(b) do if a[k]==nil then return false end end
  return true
 end
 local function metadata(ids,keys)
  local rows={}
  for _,id in ipairs(ids or requested) do
   charge();local effect=assert(rawget(data.skills,id));local parts={effect}
   for _,part in ipairs(effect.statSets) do parts[#parts+1]=part end
   local row={id=id,name=effect.name,game_id=present(resolve(effect.name,true)),
    has_global_effect=present(rawget(effect,"hasGlobalEffect")),parts={}}
   for index,part in ipairs(parts) do
    assert(getmetatable(part.statMap)==data.skillStatMapMeta)
    assert(rawget(part.statMap,"_grantedEffect")==effect)
    local maps={}
    for _,stat in ipairs(keys or stat_keys) do
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
  native_inventory_authority=false,source_methods_preserved=true,
  parser_original=true,parser_public_cache_identity=true,item_methods_original=true,control_parser_cache=control_cache(),
  item_parser_cache=control_cache(item_probe.control_lines),
  item_metadata=metadata(item_probe.effects,item_probe.stat_keys)}
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
 local function raw_mods(store,name)
  name=name or "ExtraSkillStat"
  local records={};local seen={}
  while store do
   charge();assert(not seen[store]);seen[store]=true
   assert(#records<1024)
   local mods=rawget(store,"mods")
   local candidates=mods and rawget(mods,name) or store
   for _,mod in ipairs(candidates or {}) do
    charge();if mod.name==name then records[#records+1]=plain(mod) end
   end
   store=rawget(store,"parent")
  end
  return records
 end
 local item_store_chain
 local function item_state()
  local item=assert(build.itemsTab.items[item_probe.item_id])
  assert(item.id==item_probe.item_id and item.type=="Boots" and item.slotModList==nil)
  assert(item.BuildModList==item_build and item.BuildModListForSlotNum==item_slot
   and item.GetActiveModListForSlotNum==item_active)
  local selected=assert(build.itemsTab.activeItemSet[item_probe.slot])
  assert(selected.selItemId==item_probe.item_id)
  local function relevant(mod) return mod.name=="ExtraSkill" or mod.name=="ExtraSkillStat" end
  local function local_records(store)
   local records={};local mods=rawget(store,"mods")
   local function add(mod,index,name)
    charge();if relevant(mod) then
     assert(#records<1024)
     local base_index
     for i,base_mod in ipairs(item.baseModList) do
      charge();if rawequal(mod,base_mod) then assert(not base_index);base_index=i end
     end
     records[#records+1]={index=index,bucket=name,record=plain(mod),item_base_record_index=present(base_index)}
    end
   end
   if mods then
    for _,name in ipairs({"ExtraSkill","ExtraSkillStat"}) do
     for i,mod in ipairs(rawget(mods,name) or {}) do add(mod,i,name) end
    end
   else
    for i,mod in ipairs(store) do add(mod,i,"sequence") end
   end
   return records
  end
  local function chain(store,env)
   local result,seen={},{}
   while store do
    charge();assert(type(store)=="table" and #result<32 and not seen[store]);seen[store]=true
    local parent=rawget(store,"parent")
    assert(parent==nil or parent==false or type(parent)=="table")
    -- ModStore.lua:66 uses false for a terminal parent; an extracted ModList sequence
    -- has no raw parent. Only an actual table is a traversable ancestry edge.
    local parent_kind=parent==nil and "absent" or parent==false and "false_sentinel" or "store"
    result[#result+1]={depth=#result,has_parent=type(parent)=="table",parent_kind=parent_kind,
     is_item_base=rawequal(store,item.baseModList),is_item_active=rawequal(store,item.modList),
     is_item_mod_db=env and rawequal(store,env.itemModDB) or false,
     is_player_mod_db=env and rawequal(store,env.player.modDB) or false,
     parent_is_player_mod_db=env and rawequal(parent,env.player.modDB) or false,
     records=local_records(store)}
    store=parent
   end
   return result
  end
  item_store_chain=chain
  local state={item_id=item.id,slot=item_probe.slot,mod_source=item.modSource,
   item_base=chain(assert(item.baseModList)),item_active=chain(assert(item.modList)),
   item_grants=plain(item.grantedSkills),lines={},modes={}}
  for _,line in ipairs(item.explicitModLines) do
   charge()
   for _,text in ipairs(item_probe.control_lines) do
    if line.line==text then state.lines[#state.lines+1]={line=line.line,extra=present(line.extra),
     disabled=present(line.disabled),records=plain(line.modList)} end
   end
  end
  for _,mode in ipairs({"MAIN","CALCS"}) do
   local env=mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
   assert(rawequal(env.player.itemList[item_probe.slot],item))
   local row={equipped_item_exact=true,item_store=chain(env.itemModDB,env),
    player_store=chain(env.player.modDB,env),grants={},receivers={}}
   for _,grant in ipairs(env.grantedSkills) do
    charge();if grant.skillId==item_probe.effect then
     local record={fields=scalars(grant),source_item_exact=rawequal(grant.sourceItem,item),
      source_node_absent=grant.sourceNode==nil,groups={}}
     for _,g in ipairs(build.skillsTab.socketGroupList) do
      charge();local first=g.gemList[1]
      if first and first.skillId==item_probe.effect and g.source==grant.source and g.slot==grant.slotName then
       local base_group
       for _,r in ipairs(base.runtime_groups) do
        if r.selected and rawequal(build.skillsTab.skillSets[r.preset].socketGroupList[r.index],g) then assert(not base_group);base_group=r end
       end
       assert(base_group)
       record.groups[#record.groups+1]={source_item_exact=rawequal(g.sourceItem,item),
        source_node_absent=g.sourceNode==nil,saved_group_present=base_group.saved_group_present,
        saved_source_ordinal=present(base_group.source_ordinal),group_fields=scalars(g),gem_fields=scalars(first)}
      end
     end
     row.grants[#row.grants+1]=record
    end
   end
   for _,skill in ipairs(env.player.activeSkillList) do
    charge();local effect=skill.activeEffect
    if effect.grantedEffect.id==item_probe.effect then
     local cfg=skill.skillCfg;local group=skill.socketGroup;local instance=effect.srcInstance
     assert(skill.skillModList.List==list and skill.skillModList.EvalMod==eval)
     local data_mods={}
     for _,mod in ipairs(raw_mods(skill.skillModList,"SkillData")) do
      if mod.source=="Skill:"..item_probe.effect then data_mods[#data_mods+1]=mod end
     end
     row.receivers[#row.receivers+1]={effect=effect.grantedEffect.id,
      cfg_effect_exact=rawequal(cfg.skillGrantedEffect,effect.grantedEffect),
      catalogue_effect_exact=rawequal(effect.grantedEffect,rawget(data.skills,item_probe.effect)),
      actor_exact=rawequal(skill.actor,env.player),group_source_item_exact=rawequal(group.sourceItem,item),
      source_instance_exact=rawequal(instance,group.gemList[1]),group_fields=scalars(group),
      gem_fields=scalars(instance),cfg=scalars(cfg),store_chain=chain(skill.skillModList,env),
      extra_stats=plain(list(skill.skillModList,cfg,"ExtraSkillStat")),
      emitted_skill_data=data_mods,skill_data=scalars(skill.skillData)}
    end
   end
   state.modes[mode]=row
  end
  return state
 end
 result.item_transport=item_state()
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
     raw_records=raw_mods(skill.skillModList),store_chain=item_store_chain(skill.skillModList,env),
     cfg_effect_exact=rawequal(cfg.skillGrantedEffect,effect.grantedEffect)}
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
 assert(same(result.item_metadata,metadata(item_probe.effects,item_probe.stat_keys)),"observer changed item lazy metadata")
 assert(same(result.item_transport,item_state()),"observer changed item/store/receiver state")
 assert(same(result.item_parser_cache,control_cache(item_probe.control_lines)),"observer changed item parser cache")
 assert(same(result.control_parser_cache,control_cache()),"observer changed parser cache entries")
 local cache_name,cache_value=debug.getupvalue(parser,cache_slot)
 assert(modLib.parseMod==parser and cache_name=="cache" and rawequal(cache_value,parser_cache)
  and rawequal(modLib.parseModCache,parser_cache),"observer changed parser identity")
 assert(same(before,outputs()),"observer changed outputs")
 assert(calcLib.getGameIdFromGemName==resolve and common.classes.ModStore.List==list and common.classes.ModStore.EvalMod==eval)
 assert(common.classes.Item.BuildModList==item_build and common.classes.Item.BuildModListForSlotNum==item_slot
  and common.classes.Item.GetActiveModListForSlotNum==item_active)
 assert(not debug.gethook() and jit.status()==sniperActorJit)
 result.outputs_preserved=true;result.observer_warmed_source_metadata=false;result.work=work
 return result
end
