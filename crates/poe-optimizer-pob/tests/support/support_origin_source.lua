-- Optional, bounded observations of original support discovery. No calculation
-- method is wrapped, replaced or reimplemented. Source identities stay here.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local i = debug.getinfo(f, "S")
 assert(i.what == "Lua" and i.source:gsub("\\", "/"):sub(-#path) == path and i.linedefined == line)
 return f, { path=path, first=i.linedefined, last=i.lastlinedefined }
end
local function upvalue(f, wanted)
 for i=1,100 do local n,v=debug.getupvalue(f,i); if not n then break end; if n==wanted then return v end end
 error("missing original upvalue "..wanted)
end
local init, ia = original(calcs.initEnv, "Modules/CalcSetup.lua", 717)
local process, pa = original(upvalue(init,"processGrantedEffect"), "Modules/CalcSetup.lua", 630)
local create, ca = original(calcs.createActiveSkill, "Modules/CalcActiveSkill.lua", 144)
local list, la = original(common.classes.ModStore.List, "Classes/ModStore.lua", 321)
local tabulate, ta = original(common.classes.ModStore.Tabulate, "Classes/ModStore.lua", 345)
local work = 0
local function charge() work=work+1; assert(work<2000000,"support origin observer work bound") end
local function plain(v, depth)
 charge()
 if v==nil then return {kind="absent"} end
 if type(v)=="number" then
  if v~=v or v==math.huge or v==-math.huge then return {source_number=tostring(v)} end
  return v
 end
 if type(v)=="boolean" or type(v)=="string" then return v end
 assert(type(v)=="table" and getmetatable(v)==nil,"unsupported source record")
 depth=(depth or 0)+1; assert(depth<=12)
 local out, positions, count = {}, {}, 0
 for k,x in pairs(v) do
  count=count+1; assert(count<=16384)
  if type(k)=="number" then positions[#positions+1]={index=k,value=plain(x,depth)}
  else assert(type(k)=="string"); out[k]=plain(x,depth) end
 end
 table.sort(positions,function(a,b)return a.index<b.index end)
 if #positions>0 then assert(out.positions==nil); out.positions=positions end
 return out
end
local function fields(t)
 local out={}
 for k,v in pairs(t or {}) do
  charge()
  if type(k)=="string" and (type(v)=="string" or type(v)=="boolean" or type(v)=="number") then out[k]=plain(v) end
 end
 return out
end
local function chain(store, name)
 local out,seen={},{}
 while store do
  charge(); assert(type(store)=="table" and not seen[store] and #out<32); seen[store]=true
  local rows={}
  for i,m in ipairs(store.mods and store.mods[name] or store) do
   charge(); assert(i<32768)
   if m.name==name then rows[#rows+1]={index=i,record=plain(m)} end
  end
  out[#out+1]={depth=#out,kind=store.mods and "database" or "list",rows=rows}
  store=store.parent
 end
 return out
end
local function effect(e)
 return {id=e.id,support=e.support==true,from_item=plain(e.fromItem),from_tree=plain(e.fromTree)}
end
local function group_index(env,g)
 local found
 for i,row in ipairs(env.build.skillsTab.socketGroupList) do if row==g then assert(not found); found=i end end
 assert(found,"unbound original group"); return found
end
local function gem_index(group,gem)
 local found
 for i,row in ipairs(group.gemList) do if row==gem then assert(not found); found=i end end
 return found
end
local function source(env,gem)
 local found={}
 for gi,g in ipairs(env.build.skillsTab.socketGroupList) do
  local index=gem_index(g,gem)
  if index then found[#found+1]={group=gi,gem=index,group_source=plain(g.source),skill_id=gem.skillId} end
 end
 assert(#found<=1,"ambiguous original Gem occurrence")
 return found[1] or {kind="no_selected_gem_instance"}
end
local function candidates(env,rows)
 local out={}
 for i,s in ipairs(rows) do
  charge(); assert(i<4096)
  out[i]={position=i,effect=effect(s.grantedEffect),source=source(env,s.srcInstance),
   level=plain(s.level),quality=plain(s.quality),enabled=plain(s.enabled),
   superseded=plain(s.superseded)}
 end
 return out
end
local api={}
function api.install()
 assert(debug.gethook()==nil and jit.status()==supportOriginJit); work=0
 local envs,frames={},{}
 local function census(env)
  if not envs[env] then envs[env]={processed={},queries={},constructors={}} end
  return envs[env]
 end
 local wanted={[process]=true,[create]=true,[list]=true,[tabulate]=true}
 local failure
 local function hook(event)
  if event~="call" and event~="return" then return end
  local f=debug.getinfo(2,"f").func
  if not wanted[f] then return end
  local caller=debug.getinfo(3,"fl")
  if caller.func~=init then return end
  local v,cv={},{}
  for i=1,128 do local n,x=debug.getlocal(2,i); if not n then break end; v[n]=x end
  for i=1,192 do local n,x=debug.getlocal(3,i); if not n then break end; cv[n]=x end
  local ok,err=xpcall(function()
   charge()
   local env=assert(cv.env); local row=census(env)
   if f==process and event=="call" then
    local g=v.grantedEffect; local gem=v.gemInstance
    local kind,position="without_gem_data",nil
    if gem.gemData then
     if gem.gemData.grantedEffect==g then kind="primary"
     else kind="additional"; for i,e in ipairs(gem.gemData.additionalGrantedEffects) do if e==g then assert(not position); position=i end end; assert(position) end
    end
    row.processed[#row.processed+1]={source=source(env,gem),effect=g and effect(g) or {kind="absent"},
     branch=kind,additional_position=position,call_line=caller.currentline,exact_environment=v.env==env,
     group=group_index(env,assert(cv.group)),gem_position=v.gemIndex}
   elseif f==create and event=="call" then
    assert(v.env==env and v.actor==env.player and v.socketGroup==cv.group)
    row.constructors[#row.constructors+1]={group=group_index(env,v.socketGroup),source=source(env,v.activeEffect.srcInstance),
     effect=effect(v.activeEffect.grantedEffect),no_supports=plain(v.socketGroup.noSupports),
     group_source=plain(v.socketGroup.source),slot=plain(v.socketGroup.slot),candidates=candidates(env,v.supportList),
     original_call=true,call_line=caller.currentline}
   elseif (f==list and caller.currentline==1934) or (f==tabulate and caller.currentline==1899) then
    local name=f==list and "ExtraSupport" or "LinkedSupport"
    assert(v.self==env.modDB)
    if event=="call" then
     assert(not frames[f]); frames[f]={row=row,name=name,store=v.self,cfg=fields(v.cfg),before=chain(v.self,name),line=caller.currentline}
    else
     local frame=assert(frames[f]); frames[f]=nil
     assert(frame.row==row and frame.store==v.self and type(v.result)=="table")
     row.queries[#row.queries+1]={name=name,cfg=frame.cfg,before=frame.before,result=plain(v.result),
      result_count=#v.result,call_line=frame.line,original_call=true,original_return=true,exact_actor_store=true}
    end
   end
  end,debug.traceback)
  if not ok then failure=err; debug.sethook(); error(err,0) end
 end
 if supportOriginInstrumented then jit.flush(); debug.sethook(hook,"cr") end
 api.envs=envs
 return function()
  if supportOriginInstrumented then if debug.gethook()==hook then debug.sethook() end; assert(not failure,failure); assert(next(frames)==nil) end
  assert(debug.gethook()==nil and jit.status()==supportOriginJit)
  assert(calcs.initEnv==init and calcs.createActiveSkill==create and common.classes.ModStore.List==list and common.classes.ModStore.Tabulate==tabulate)
  assert(upvalue(init,"processGrantedEffect")==process)
 end
end
function api.observe()
 assert(debug.gethook()==nil)
 local groups={}
 for i,g in ipairs(build.skillsTab.socketGroupList) do
  local gems={}
  for j,gem in ipairs(g.gemList) do
   local additional={}
   for k,e in ipairs(gem.gemData and gem.gemData.additionalGrantedEffects or {}) do additional[k]=effect(e) end
   gems[j]={position=j,skill_id=gem.skillId,enabled=gem.enabled,gem_data_present=gem.gemData~=nil,
    primary=effect(assert(gem.gemData and gem.gemData.grantedEffect or gem.grantedEffect)),additional=additional}
  end
  groups[i]={index=i,source=plain(g.source),slot=plain(g.slot),enabled=plain(g.enabled),
   slot_enabled=plain(g.slotEnabled),no_supports=plain(g.noSupports),gems=gems}
 end
 local environments={}
 for _,mode in ipairs({"MAIN","CALCS"}) do
  local env=mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
  local captured=api.envs[env]; assert(not supportOriginInstrumented or captured)
  local skills={}
  for i,a in ipairs(env.player.activeSkillList) do
   skills[i]={effect=effect(a.activeEffect.grantedEffect),group=group_index(env,a.socketGroup),
    source=source(env,a.activeEffect.srcInstance),candidates=candidates(env,a.supportList)}
  end
  environments[#environments+1]={mode=mode,skills=skills,linked_slots=plain(env.crossLinkedSupportGroups),
   calls=captured,main_group=env.mainSocketGroup}
 end
 return {selected={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec},
  groups=groups,environments=environments,outputs={MAIN=fields(build.calcsTab.mainOutput),CALCS=fields(build.calcsTab.calcsOutput)},
  methods={initialization=ia,processing=pa,construction=ca,list=la,tabulate=ta},
  original_methods_preserved=true,hook_removed=true,source_tables_mutated=false,
  diagnostic_requery=false,native_discovery_authority=false}
end
return api
