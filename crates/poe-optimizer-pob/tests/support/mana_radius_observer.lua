-- Optional original-source input/lifecycle observation. No game function is
-- replaced, no modifier query is added, and no native coverage is authorized.
return function(h)
 local calcs=require("Modules.CalcBase")
 local refs={
  {calcs,"initEnv","Modules/CalcSetup.lua",717},
  {calcs,"buildModListForNode","Modules/CalcSetup.lua",200},
  {calcs,"buildModListForNodeList","Modules/CalcSetup.lua",415},
 }
 for _,r in ipairs(refs) do r[5]=h.original(r[1][r[2]],r[3],r[4]) end
 local function functionRef(f)
  local i=debug.getinfo(f,"S")
  return {kind="function_reference",path=i.source:gsub("\\","/"),what=i.what,
   first_line=i.linedefined,last_line=i.lastlinedefined,execution_not_implied=true}
 end
 local function sortedKeys(t)
  local keys={}
  for k in pairs(t or {}) do
   assert(type(k)=="number" or type(k)=="string","unsupported inventory key")
   assert(#keys<16384,"bounded inventory")
   keys[#keys+1]=k
  end
  table.sort(keys,function(a,b)
   if type(a)~=type(b) then return type(a)<type(b) end
   return a<b
  end)
  return keys
 end
 local function raw(v,seen,path,depth)
  local kind=type(v)
  if kind=="nil" then return {kind="nil"} end
  if kind=="function" then return functionRef(v) end
  if kind~="table" then
   assert(kind=="number" or kind=="string" or kind=="boolean","unsupported raw primitive")
   return {kind=kind,value=h.scalar(v)}
  end
  seen=seen or {};path=path or "$";depth=(depth or 0)+1
  assert(depth<24,"bounded raw depth")
  if seen[v] then return {kind="table_reference",path=seen[v]} end
  seen[v]=path
  local out={kind="table",entries={}}
  for _,k in ipairs(sortedKeys(v)) do
   out.entries[#out.entries+1]={key=k,key_type=type(k),value=raw(v[k],seen,path.."["..tostring(k).."]",depth)}
  end
  return out
 end
 local function keysAndTypes(t)
  local out={}
  for _,k in ipairs(sortedKeys(t)) do out[#out+1]={key=k,key_type=type(k),value_type=type(t[k])} end
  return out
 end
 local function item(i)
  if not i then return {present=false} end
  return {present=true,fields=h.scalars(i),base_fields=h.scalars(i.base),
   jewel_data=raw(i.jewelData),mod_source=raw(i.modSource),raw=raw(i.raw)}
 end
 local function node(n)
  if not n then return {present=false} end
  return {present=true,id=n.id,type=n.type,fields=h.scalars(n)}
 end
 local function nodes(t)
  local out={}
  for _,k in ipairs(sortedKeys(t)) do out[#out+1]={key=k,value=node(t[k])} end
  return out
 end
 local function spec(s,build)
  if not s then return {present=false} end
  return {present=true,is_selected=s==build.spec,fields=h.scalars(s),
   jewels=raw(s.jewels),allocated_nodes=nodes(s.allocNodes)}
 end
 local envIds,envCount={},0
 local function envId(env)
  if not env then return false end
  if not envIds[env] then envCount=envCount+1;envIds[env]=envCount end
  return envIds[env]
 end
 local function radius(env)
  if not env then return {environment_present=false} end
  local out={environment_present=true,env_id=envId(env),
   radius_list_present=env.radiusJewelList~=nil,extra_nodes_present=env.extraRadiusNodeList~=nil,
   radius_jewels={},extra_radius_nodes=nodes(env.extraRadiusNodeList)}
  for _,k in ipairs(sortedKeys(env.radiusJewelList)) do
   local r=env.radiusJewelList[k]
   out.radius_jewels[#out.radius_jewels+1]={key=k,fields=h.scalars(r),keys=keysAndTypes(r),
    item=item(r.item),nodes=nodes(r.nodes),attributes=raw(r.attributes),data=raw(r.data),func=raw(r.func)}
  end
  return out
 end
 local function override(v,build)
  if not v then return {present=false} end
  local out={present=true,fields={}}
  for _,k in ipairs(sortedKeys(v)) do
   local value
   if k=="spec" then value=spec(v[k],build)
   elseif k=="repItem" then value=item(v[k])
   elseif k=="addNodes" or k=="removeNodes" then
    value={kind="node_selection",entries={}}
    for n,enabled in pairs(v[k]) do
     assert(type(n)=="table" or type(n)=="number","unknown node override key")
     value.entries[#value.entries+1]={key=type(n)=="table" and node(n) or n,key_type=type(n),value=raw(enabled)}
    end
    table.sort(value.entries,function(a,b)return tostring(type(a.key)=="table" and a.key.id or a.key)<tostring(type(b.key)=="table" and b.key.id or b.key) end)
   else value=raw(v[k]) end
   out.fields[#out.fields+1]={key=k,key_type=type(k),value_type=type(v[k]),value=value}
  end
  return out
 end
 local function slots(build,env)
  local out={ordered={},ui_sockets={},slot_map={},item_zero=item(build.itemsTab.items[0])}
  for _,k in ipairs(sortedKeys(build.itemsTab.orderedSlots)) do
   local s=build.itemsTab.orderedSlots[k]
   out.ordered[#out.ordered+1]={key=k,fields=h.scalars(s),keys=keysAndTypes(s),
    is_slot_map_entry=build.itemsTab.slots[s.slotName]==s,
    is_ui_socket=s.nodeId~=nil and build.itemsTab.sockets[s.nodeId]==s,
    selected_item=item(build.itemsTab.items[s.selItemId]),
    spec_item=env and s.nodeId and item(build.itemsTab.items[env.spec.jewels[s.nodeId]]) or {present=false}}
  end
  for _,k in ipairs(sortedKeys(build.itemsTab.sockets)) do
   local s=build.itemsTab.sockets[k];local indices={}
   for _,at in ipairs(sortedKeys(build.itemsTab.orderedSlots)) do if build.itemsTab.orderedSlots[at]==s then indices[#indices+1]=at end end
   out.ui_sockets[#out.ui_sockets+1]={key=k,fields=h.scalars(s),ordered_indices=indices}
  end
  for _,k in ipairs(sortedKeys(build.itemsTab.slots)) do
   local s=build.itemsTab.slots[k];local indices={}
   for _,at in ipairs(sortedKeys(build.itemsTab.orderedSlots)) do if build.itemsTab.orderedSlots[at]==s then indices[#indices+1]=at end end
   out.slot_map[#out.slot_map+1]={key=k,node_id=raw(s.nodeId),ordered_indices=indices}
  end
  return out
 end
 local result={schema_version=1,kind="original-mana-radius-inputs",init_calls={},node_calls={},list_calls={},slot_candidates={}}
 local frames,nodeFrames,listFrames={},{},{}
 local failure
 local function append(name,row)
  local rows=result[name];assert(#rows<32768,"bounded observation count");rows[#rows+1]=row
 end
 local function caller(info)
  return {path=info.source:gsub("\\","/"),what=info.what,current_line=info.currentline,
   first_line=info.linedefined,last_line=info.lastlinedefined}
 end
 local function frameFor(env)
  local f=frames[#frames];assert(f,"node operation outside observed initEnv")
  if f.env then assert(f.env==env,"node operation belongs to a different environment") end
  f.env=env;return f
 end
 assert(debug.gethook()==nil)
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  local interested=(event=="call" or event=="return") and
   (f==calcs.initEnv or f==calcs.buildModListForNode or f==calcs.buildModListForNodeList)
   or event=="line" and f==calcs.initEnv and (line==908 or line==1072 or line==1087)
  if failure or not interested then return end
  local vars={};for i=1,192 do local n,v=debug.getlocal(2,i);if not n then break end;vars[n]=v end
  local from=debug.getinfo(3,"Sln")
  local ok,problem=xpcall(function()
   if f==calcs.initEnv then
    if event=="call" then
     local input=vars.specEnv
     local row={invocation_id=#result.init_calls+1,parent_invocation_id=frames[#frames] and frames[#frames].row.invocation_id or false,
      caller=caller(from),mode=vars.mode,override=override(vars.override,vars.build),
      selected_spec=spec(vars.build.spec,vars.build),active_spec=vars.build.treeTab.activeSpec,
      spec_env={present=input~=nil,keys=keysAndTypes(input),env_id=envId(input and input.env),
       accelerate=raw(input and input.accelerate)},entry_radius=radius(input and input.env),
      entry_slots=slots(vars.build,input and input.env),returned=false}
     append("init_calls",row);frames[#frames+1]={row=row,env=input and input.env}
    elseif event=="line" then
     local frame=frameFor(assert(vars.env));local row=frame.row
     row.env_id=envId(vars.env)
     if line==908 then
      assert(not row.pre_configuration,"repeated initialization baseline")
      row.pre_configuration={radius=radius(vars.env),spec=spec(vars.env.spec,vars.build),
       override=override(vars.override,vars.build),accelerate=raw(vars.accelerate)}
     elseif line==1072 then
      if not row.item_pass then row.item_pass={radius=radius(vars.env),slots=slots(vars.build,vars.env),
       allocated_nodes=nodes(vars.env.allocNodes),override=override(vars.override,vars.build),accelerate=raw(vars.accelerate)} end
     else
      local s=assert(vars.slot);local indices={}
      for _,at in ipairs(sortedKeys(vars.build.itemsTab.orderedSlots)) do if vars.build.itemsTab.orderedSlots[at]==s then indices[#indices+1]=at end end
      append("slot_candidates",{invocation_id=row.invocation_id,env_id=envId(vars.env),ordered_indices=indices,
       slot_fields=h.scalars(s),slot_name=vars.slotName,resolved_item=item(vars.item),
       allocated=vars.env.allocNodes[s.nodeId]~=nil,override=override(vars.override,vars.build)})
     end
    else
     local frame=frames[#frames];assert(frame and not frame.row.returned,"unmatched initEnv return")
     local env=assert(vars.env);assert(not frame.env or frame.env==env,"initEnv identity changed")
     frame.row.env_id=envId(env);frame.row.exit_radius=radius(env);frame.row.exit_slots=slots(vars.build,env)
     frame.row.exit_nodes=nodes(env.allocNodes);frame.row.returned=true
     frames[#frames]=nil
    end
   elseif f==calcs.buildModListForNodeList then
    if event=="call" then
     local env=assert(vars.env);local frame=frameFor(env)
     local row={list_id=#result.list_calls+1,invocation_id=frame.row.invocation_id,env_id=envId(env),
      parent_list_id=listFrames[#listFrames] and listFrames[#listFrames].row.list_id or false,
      caller=caller(from),nodes=nodes(vars.nodeList),finish_jewels=raw(vars.finishJewels),
      include_keystone_mods=raw(vars.includeKeystoneMods),before_radius=radius(env),returned=false}
     append("list_calls",row);listFrames[#listFrames+1]={row=row,env=env}
    else
     local frame=listFrames[#listFrames];assert(frame and frame.env==vars.env,"unmatched node-list return")
     frame.row.after_radius=radius(vars.env);frame.row.returned=true;listFrames[#listFrames]=nil
    end
   else
    if event=="call" then
     local env=assert(vars.env);local frame=frameFor(env);local list=listFrames[#listFrames]
     local row={node_call_id=#result.node_calls+1,invocation_id=frame.row.invocation_id,env_id=envId(env),
      list_id=list and list.env==env and list.row.list_id or false,caller=caller(from),node=node(vars.node),
      before_radius=radius(env),returned=false}
     append("node_calls",row);nodeFrames[#nodeFrames+1]={row=row,env=env}
    else
     local frame=nodeFrames[#nodeFrames];assert(frame and frame.env==vars.env,"unmatched node return")
     frame.row.after_radius=radius(vars.env);frame.row.returned=true;nodeFrames[#nodeFrames]=nil
    end
   end
  end,debug.traceback)
  if not ok then failure=problem end
 end
 debug.sethook(hook,"crl")
 return function()
  local ours=debug.gethook()==hook;debug.sethook();assert(ours,"radius observer replaced")
  assert(not failure,failure);assert(#frames==0 and #nodeFrames==0 and #listFrames==0,"unmatched source invocation")
  for _,r in ipairs(refs) do assert(r[1][r[2]]==r[5],"original source function replaced") end
  result.original_methods={};for _,r in ipairs(refs) do result.original_methods[#result.original_methods+1]=functionRef(r[5]) end
  result.original_functions_preserved=true;result.observer_removed=debug.gethook()==nil
  result.snapshot=h.offeringOutputSnapshot()
  result.scope={unchanged_build_only=true,native_coverage_authorized=false,
   input_observation_not_universal_absence=true,modifier_bodies_reused_from_source02=true}
  _G.manaRadiusObservation=result
 end
end
