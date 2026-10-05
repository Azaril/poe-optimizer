-- Test-only finite source census. All source reads are raw: observing a lazy
-- stat map must neither initialize metadata nor change a calculation.
return function(source, requests, maximum)
 local work = 0
 local function charge(n)
  work = work + (n or 1)
  assert(work <= maximum, "global-switch census work limit")
 end
 local result = {requests={}, calculation_non_applicability_proved=false,
  unresolved={"extra_stat_scope_unproved"}}
 local function keys(t)
  local out={}
  for k in next,t do charge(type(k)=="string" and #k+1 or 1);out[#out+1]=k end
  table.sort(out,function(a,b)
   if type(a)==type(b) then
    if type(a)=="number" or type(a)=="string" then return a<b end
    return false
   end
   return type(a)<type(b)
  end)
  return out
 end
 local function plain(t)
  return type(t)=="table" and getmetatable(t)==nil
 end
 for _,request in ipairs(requests) do
  charge()
  local row={game_id=request.game_id,variant_id=request.variant_id,effects={},issues={}}
  result.requests[#result.requests+1]=row
  local function issue(path,code)
   charge();row.issues[#row.issues+1]={path=path,code=code}
  end
  local visiting={}
  local function snapshot(value,path,depth)
   charge()
   local kind=type(value)
   if kind=="nil" then return {kind="absent"} end
   if kind=="boolean" or kind=="string" then
    if kind=="string" then charge(#value) end
    return {kind=kind,value=value}
   end
   if kind=="number" then
    if value~=value or value==math.huge or value==-math.huge then
     issue(path,"non-finite-number");return {kind="unresolved"}
    end
    return {kind=kind,value=value}
   end
   if kind~="table" then issue(path,"unreviewed-"..kind);return {kind="unresolved"} end
   if depth>24 or visiting[value] then issue(path,"recursive-or-deep-value");return {kind="unresolved"} end
   if getmetatable(value)~=nil then issue(path,"unreviewed-value-metatable");return {kind="unresolved"} end
   visiting[value]=true
   local entries={}
   for _,k in ipairs(keys(value)) do
    if type(k)~="number" and type(k)~="string" then
     issue(path,"unreviewed-table-key")
    else
     entries[#entries+1]={key_type=type(k),key=k,value=snapshot(rawget(value,k),path.."/"..tostring(k),depth+1)}
    end
   end
   visiting[value]=nil
   return {kind="table",entries=entries}
  end
  local function array(t,path,optional)
   if t==nil and optional then return {} end
   if type(t)~="table" or getmetatable(t)~=nil then issue(path,"unreviewed-array");return nil end
   local ks=keys(t)
   for i,k in ipairs(ks) do
    if k~=i then issue(path,"non-dense-array");return nil end
   end
   return t
  end
  local global_paths={}
  local function inspect_tags(t,path,depth,seen)
   charge()
   if type(t)~="table" then return end
   if depth>24 or seen[t] then return end
   seen[t]=true
   if rawget(t,"type")=="GlobalEffect" then global_paths[#global_paths+1]=path end
   for _,k in ipairs(keys(t)) do
    if type(k)=="string" or type(k)=="number" then
     inspect_tags(rawget(t,k),path.."/"..tostring(k),depth+1,seen)
    end
   end
  end
  local by_game=type(source)=="table" and rawget(source,"gemsByGameId")
  local variants=type(by_game)=="table" and rawget(by_game,request.game_id)
  local gem=type(variants)=="table" and rawget(variants,request.variant_id)
  local skills=type(source)=="table" and rawget(source,"skills")
  local global_map=type(source)=="table" and rawget(source,"skillStatMap")
  local map_meta=type(source)=="table" and rawget(source,"skillStatMapMeta")
  local containers_plain=plain(source) and plain(by_game) and plain(variants)
   and plain(gem) and plain(skills) and plain(global_map)
  local map_authority_plain=plain(map_meta)
  if map_authority_plain then
   local meta_keys=keys(map_meta)
   map_authority_plain=#meta_keys==1 and meta_keys[1]=="__index" and type(rawget(map_meta,"__index"))=="function"
  end
  if not containers_plain then issue("catalog","unreviewed-catalog-container") end
  if not map_authority_plain then issue("catalog","unreviewed-stat-map-metatable") end
  if not containers_plain or not map_authority_plain
   or rawget(gem,"gameId")~=request.game_id or rawget(gem,"variantId")~=request.variant_id then
   issue("catalog","unresolved-catalog-identity")
  else
   row.vaal=rawget(gem,"vaalGem")==true
   if rawget(gem,"vaalGem")~=nil and type(rawget(gem,"vaalGem"))~="boolean" then issue("catalog","unreviewed-vaal-value") end
   if row.vaal then issue("catalog","vaal-switch-consumer") end
   local effects=array(rawget(gem,"grantedEffectList"),"catalog/effects",false)
   if effects and #effects~=#request.effect_list then issue("catalog/effects","effect-inventory-mismatch") end
   for index,effect in ipairs(effects or {}) do
    local path="effects/"..index
    local id=type(effect)=="table" and rawget(effect,"id")
    if not plain(effect) then
     issue(path,"unreviewed-effect-container")
    elseif type(id)~="string" or rawget(skills,id)~=effect or request.effect_list[index]~=id then
     issue(path,"unresolved-effect-identity")
    else
     local e={id=id,index=index,has_global_before=rawget(effect,"hasGlobalEffect")==true,parts={}}
     row.effects[#row.effects+1]=e
     if e.has_global_before then issue(path,"initialized-global-effect") end
     if rawget(effect,"hasGlobalEffect")~=nil and type(rawget(effect,"hasGlobalEffect"))~="boolean" then
      issue(path,"unreviewed-global-effect-value")
     end
     local sets=array(rawget(effect,"statSets"),path.."/statSets",false)
     local parts={effect}
     for _,set in ipairs(sets or {}) do parts[#parts+1]=set end
     local root_names,root_examined={},{}
     local root_map,root_receipt
     for part_index,part in ipairs(parts) do
      local pp=path.."/parts/"..part_index
      if not plain(part) then issue(pp,"unreviewed-stat-set") else
       local p={index=part_index,kind=part_index==1 and "root" or "stat_set",fields={},maps={}}
       e.parts[#e.parts+1]=p
       local names={}
       for _,field in ipairs({"stats","constantStats","qualityStats","altQualityStats"}) do
        local values=array(rawget(part,field),pp.."/"..field,true)
        p.fields[field]=snapshot(rawget(part,field),pp.."/"..field,0)
        for i,value in ipairs(values or {}) do
         local name=field=="stats" and value or type(value)=="table" and rawget(value,1)
         if type(name)~="string" or name=="" then issue(pp.."/"..field.."/"..i,"unreviewed-stat-name")
         else names[name]=true end
        end
       end
       -- Quality belongs to the granted effect even while a stat set is merged.
       for _,field in ipairs({"qualityStats","altQualityStats"}) do
        for _,value in ipairs(array(rawget(effect,field),path.."/"..field,true) or {}) do
         if type(value)=="table" and type(rawget(value,1))=="string" then names[rawget(value,1)]=true end
        end
       end
       -- CalcActiveSkill also looks up active stat-set stats through the root
       -- effect's map. A set-local override cannot prove that root read inert.
       for _,name in ipairs(keys(names)) do root_names[name]=true end
       for _,field in ipairs({"baseMods","qualityMods","levelMods","levels"}) do
        local value=rawget(part,field)
        p.fields[field]=snapshot(value,pp.."/"..field,0)
        inspect_tags(value,pp.."/"..field,0,{})
       end
       local local_map=rawget(part,"statMap")
       if type(local_map)~="table" or getmetatable(local_map)~=map_meta
        or rawget(local_map,"_grantedEffect")~=effect then
        issue(pp.."/statMap","unreviewed-stat-map-authority")
       else
        if part_index==1 then root_map=local_map;root_receipt=p end
        for _,key in ipairs(keys(local_map)) do
         if key~="_grantedEffect" then
          if type(key)~="string" then issue(pp.."/statMap","unreviewed-stat-map-key") else names[key]=true end
         end
        end
        for _,name in ipairs(keys(names)) do
         local local_value=rawget(local_map,name)
         local inherited=local_value==nil
         local map=local_value
         if inherited then map=rawget(global_map,name) end
         local mp=pp.."/statMap/"..name
         p.maps[#p.maps+1]={stat=name,origin=inherited and "global_raw" or "local_raw",value=snapshot(map,mp,0)}
         if part_index==1 then root_examined[name]=true end
         inspect_tags(map,mp,0,{})
        end
       end
      end
     end
     if root_map then
      for _,name in ipairs(keys(root_names)) do
       if not root_examined[name] then
        local local_value=rawget(root_map,name)
        local inherited=local_value==nil
        local map=local_value
        if inherited then map=rawget(global_map,name) end
        local mp=path.."/parts/1/statMap/"..name
        root_receipt.maps[#root_receipt.maps+1]={stat=name,origin=inherited and "global_raw" or "local_raw",
         lookup="active_stat_set_through_root",value=snapshot(map,mp,0)}
        inspect_tags(map,mp,0,{})
       end
      end
     end
     e.has_global_after=rawget(effect,"hasGlobalEffect")==true
     assert(e.has_global_before==e.has_global_after,"census initialized effect metadata")
    end
   end
  end
  table.sort(global_paths)
  row.global_tag_paths=global_paths
  if #global_paths>0 then issue("modifiers","reachable-global-effect-tag") end
  row.declared_scope_status=#row.issues==0 and "complete_without_global_tag" or "refused"
  row.calculation_non_applicability_proved=false
 end
 result.work=work
 return result
end
