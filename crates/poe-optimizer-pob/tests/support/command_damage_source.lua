-- Bounded original-call evidence. No business function is replaced.
local calcs = require("Modules.CalcBase")
local ids = {25927, 41511, 32847}
local wanted = {[25927]=true,[41511]=true,[32847]=true}
local sources = {}; for _,id in ipairs(ids) do sources["Tree:"..id]=id end
local function original(f,path,line)
 local i=debug.getinfo(f,"S"); assert(i.what=="Lua" and i.source:gsub("\\","/"):sub(-#path)==path and (not line or i.linedefined==line),path); return f
end
local function upvalue(f,key)
 for i=1,128 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==key then return v end end
 error("missing upvalue "..key)
end
local function scalar(v)
 if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return {nonfinite=tostring(v)} end
 return v
end
local function scalars(t)
 local out={};for k,v in pairs(t or {}) do if type(k)=="string" and (type(v)=="number" or type(v)=="string" or type(v)=="boolean") then out[k]=scalar(v) end end;return out
end
local function copy(v,depth)
 if type(v)~="table" then assert(type(v)~="function" and type(v)~="userdata");return scalar(v) end
 depth=(depth or 0)+1;assert(depth<=16);local out,positions,n={},{},0
 for k,x in pairs(v) do n=n+1;assert(n<=4096)
  if type(k)=="number" then positions[#positions+1]={index=k,value=copy(x,depth)} else assert(type(k)=="string");out[k]=copy(x,depth) end
 end
 table.sort(positions,function(a,b)return a.index<b.index end);if #positions>0 then out._positions=positions end;return out
end
local function equal(a,b,seen)
 if type(a)~=type(b) then return false end;if type(a)~="table" then return a==b end
 seen=seen or {};if seen[a] then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a) do if not equal(v,b[k],seen) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function clone(v,seen)
 if type(v)~="table" then return scalar(v) end;seen=seen or {};if seen[v] then return seen[v] end
 local out={};seen[v]=out;for k,x in pairs(v) do out[k]=clone(x,seen) end;return out
end
local function watch(store)
 local saved,seen={},{}
 while store do
  assert(not seen[store] and #saved<16);seen[store]=true
  local owned={};for k,v in pairs(store) do if k~="parent" and k~="actor" then owned[k]=v end end
  saved[#saved+1]={store=store,parent=store.parent,actor=store.actor,meta=getmetatable(store),owned=clone(owned)};store=store.parent
 end
 return function()
  for _,r in ipairs(saved) do
   assert(r.store.parent==r.parent and r.store.actor==r.actor and getmetatable(r.store)==r.meta)
   local owned={};for k,v in pairs(r.store) do if k~="parent" and k~="actor" then owned[k]=v end end
   assert(equal(r.owned,clone(owned)),"diagnostic modified store")
  end
 end
end
local function list(t) local out={};for _,v in ipairs(t or {}) do out[#out+1]=copy(v) end;assert(#out<=512);return out end
local function locals(level)
 local out={};for i=1,192 do local n,v=debug.getlocal(level+1,i);if not n then break end;out[n]=v end;return out
end
local function relevant(active)
 return active and active.actor and active.actor.type=="RaisedSkeletonSniper"
  and (active.activeEffect.grantedEffect.id=="MinionMeleeBow" or active.activeEffect.grantedEffect.id=="GasShotSkeletonSniperMinion")
end
local methods={
 {calcs,"buildModListForNode","Modules/CalcSetup.lua",200},
 {calcs,"initEnv","Modules/CalcSetup.lua",717},
 {calcs,"offence","Modules/CalcOffence.lua",527},
 {calcs,"perform","Modules/CalcPerform.lua",1193},
 {common.classes.ModStore,"Sum","Classes/ModStore.lua",202},
 {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
 {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
 {modLib,"parseMod","Modules/ModParser.lua"},
}
local function sourceOccurrence(active)
 local src=assert(active.activeEffect.srcInstance);local matches={}
 for g,group in ipairs(build.skillsTab.socketGroupList) do for i,gem in ipairs(group.gemList) do if gem==src then
  matches[#matches+1]={group=g,gem=i,group_is_socket_group=group==active.socketGroup,physical_gem=gem.gemData and gem.gemData.gameId,
   gem_id=gem.gemId,source_effect=gem.grantedEffect and gem.grantedEffect.id,enabled=gem.enabled,level=gem.level,quality=gem.quality}
 end end end
 assert(#matches==1);return matches[1]
end
local function ancestry(store,select)
 local out,depth={},0;local seen={}
 while store do
  assert(not seen[store] and depth<16);seen[store]=true
  local function add(mod,pos,name)
   if select(mod) then out[#out+1]={record=copy(mod),depth=depth,position=pos,channel=name,object=mod} end
  end
  if store.mods then
   local names={};for name in pairs(store.mods) do names[#names+1]=name end;table.sort(names)
   for _,name in ipairs(names) do for pos,mod in ipairs(store.mods[name]) do add(mod,pos,name) end end
  else for pos,mod in ipairs(store) do add(mod,pos,mod.name) end end
  store=store.parent;depth=depth+1
 end
 assert(#out<=512);return out
end
local function public(rows)
 local out={};for _,r in ipairs(rows) do out[#out+1]={record=r.record,depth=r.depth,position=r.position,channel=r.channel} end;return out
end
local function selectedRecord(mod) return mod.name=="Damage" and sources[mod.source]~=nil end
local function selectedOuter(mod) return mod.name=="MinionModifier" and mod.value and mod.value.mod and selectedRecord(mod.value.mod) end
local function cfg(active,c)
 assert(c and c.skillGrantedEffect==active.activeEffect.grantedEffect and c.skillGem==active.activeEffect.gemData)
 return {fields=scalars(c),skill_conditions=scalars(c.skillCond),skill_types=copy(c.skillTypes),exact_granted_effect=true,exact_gem=true}
end
local function queried(active,c,names)
 local store=active.skillModList;local checked=watch(store);local oldCfg=cfg(active,c)
 local matches={};local total=store:Sum("INC",c,unpack(names))
 for _,r in ipairs(store:Tabulate("INC",c,unpack(names))) do if selectedRecord(r.mod) then
  matches[#matches+1]={record=copy(r.mod),value=scalar(r.value)}
 end end
 local commandable=not not store:Flag(c,"Condition:CommandableSkill")
 checked();assert(equal(oldCfg,cfg(active,c)))
 return {kind="diagnostic-original-method-queries",names=names,sum=scalar(total),reviewed_records=matches,commandable=commandable,query_state_preserved=true}
end
if commandDamagePhase=="before" then
 local refs={};for i,r in ipairs(methods) do refs[i]=original(r[1][r[2]],r[3],r[4]) end
 local calcDamage=original(upvalue(calcs.offence,"calcDamage"),"Modules/CalcOffence.lua",178)
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local auth={refs=refs,calc_damage=calcDamage,node_returns={},calls={},consumers={},work=0}
 local failure
 local function observe(event)
  if event~="return" then return end
  -- observe -> pcall -> hook -> the original function producing this event.
  local f=debug.getinfo(4,"f").func
  if f~=calcs.buildModListForNode and f~=calcDamage and f~=calcs.offence then return end
  local v=locals(4);auth.work=auth.work+1;assert(auth.work<=300000)
  if f==calcs.buildModListForNode and v.node and wanted[v.node.id] then
   local byEnv=auth.node_returns[v.env] or {};auth.node_returns[v.env]=byEnv
   local rows=byEnv[v.node.id] or {};byEnv[v.node.id]=rows;assert(#rows<64)
   rows[#rows+1]={id=v.node.id,node_is_allocated=v.env.allocNodes[v.node.id]==v.node,modifiers=list(v.modList),
    input_modifiers=list(v.node.modList),local_small_effect=v.localSmallIncEffect,local_notable_effect=v.localNotableIncEffect,
    small_effect=v.incSmallPassiveSkill,include_keystone=v.includeKeystoneMods,node_fields=scalars(v.node)}
  elseif f==calcDamage and relevant(v.activeSkill) then
   local active=v.activeSkill;local rows=auth.calls[active] or {};auth.calls[active]=rows;assert(#rows<256)
   local names={};for _,n in ipairs(v.modNames or {}) do names[#names+1]=n end
   rows[#rows+1]={damage_type=v.damageType,type_flags=v.typeFlags,inc=scalar(v.inc),more=scalar(v.more),summed_min=scalar(v.summedMin),summed_max=scalar(v.summedMax),
    cfg=cfg(active,v.cfg),modifier_names=names,diagnostic=#names>0 and queried(active,v.cfg,names) or nil,original_calc_damage=true}
  elseif f==calcs.offence and relevant(v.activeSkill) then
   local active=v.activeSkill;local rows=auth.consumers[active] or {};auth.consumers[active]=rows;assert(#rows<16)
   local parent=ancestry(v.env.player.modDB,selectedOuter)
   local received=ancestry(active.skillModList,selectedRecord);local joins={}
   for index,r in ipairs(received) do
    local matched={};for pi,p in ipairs(parent) do if p.object.value.mod==r.object then matched[#matched+1]=pi end end
    joins[#joins+1]={received_index=index,parent_indices=matched,exact_nested_object=#matched>0}
   end
   local set=v.env.mode=="CALCS" and active.activeEffect.statSetCalcs or active.activeEffect.statSet
   assert(active.activeEffect.grantedEffect.statSets[set.index]==set.statSet)
   local calls=auth.calls[active] or {};auth.calls[active]=nil
   rows[#rows+1]={mode=v.env.mode,effect=active.activeEffect.grantedEffect.id,stat_set_index=set.index,stat_set_label=set.statSet.label,
    actor_type=active.actor.type,actor_parent_is_player=active.actor.parent==v.env.player,actor_is_selected=active.actor==v.env.minion,
    selected=active.actor.mainSkill==active,source=sourceOccurrence(assert(active.summonSkill)),summoner_owns_actor=active.summonSkill.minion==active.actor,
    parent_records=public(parent),received_records=public(received),joins=joins,cfg=cfg(active,active.skillCfg),
    diagnostic=queried(active,active.skillCfg,{"Damage"}),original_damage_calls=calls,output=scalars(v.output),original_offence=true}
  end
 end
 local function hook(event)
  if failure then return end
  local ok,err=pcall(observe,event);if not ok then failure=debug.traceback(tostring(err)) end
 end
 local enabled=jit.status();jit.flush();debug.sethook(hook,"r");commandDamageAuth=auth
 return function()
  local intact=debug.gethook()==hook;debug.sethook(oldHook,oldMask,oldCount)
  assert(intact and jit.status()==enabled);assert(not failure,failure)
  for i,r in ipairs(methods) do assert(r[1][r[2]]==refs[i]) end
  assert(upvalue(calcs.offence,"calcDamage")==calcDamage);auth.finished=true
 end
end
local auth=assert(commandDamageAuth);assert(auth.finished)
local function environment(env)
 local allocated={};for id in pairs(env.allocNodes) do allocated[#allocated+1]=id end;table.sort(allocated);assert(#allocated<=2048)
 local nodes={};for _,id in ipairs(ids) do
  local default=assert(env.spec.tree.nodes[id]);local effective=env.allocNodes[id]
  local records=auth.node_returns[env] and auth.node_returns[env][id] or {}
  local returned={};for _,r in ipairs(records) do returned[#returned+1]=r end
  local parsed={};for _,text in ipairs(default.sd) do
   local cached=assert(modLib.parseModCache[text],"node line must already be parsed")
   local before=copy(cached)
   local mods,extra=modLib.parseMod(text);assert(equal(before,copy(modLib.parseModCache[text])))
   parsed[#parsed+1]={line=text,modifiers=list(mods),extra=extra,kind="diagnostic-original-parser-cache-hit",cache_preserved=true}
  end
  nodes[#nodes+1]={id=id,allocated=effective~=nil,default_fields=scalars(default),default_stats=list(default.sd),default_modifiers=list(default.modList),
   default_linked_ids=list(default.linkedId),effective_fields=effective and scalars(effective),effective_stats=effective and list(effective.sd),
   effective_is_default=effective==default,effective_modifiers=effective and list(effective.modList),original_node_returns=returned,parser=parsed}
 end
 local recipients={}
 for _,summon in ipairs(env.player.activeSkillList) do if summon.activeEffect.grantedEffect.id=="SummonSkeletalSnipersPlayer" and summon.minion then
  for _,active in ipairs(summon.minion.activeSkillList) do if relevant(active) then
   local observed=auth.consumers[active] or {};local rows={};for _,r in ipairs(observed) do rows[#rows+1]=r end
   recipients[#recipients+1]={effect=active.activeEffect.grantedEffect.id,selected=active.actor.mainSkill==active,
    source=sourceOccurrence(summon),original_offence_calls=rows}
  end end
 end end
 return {mode=env.mode,allocated_nodes=allocated,nodes=nodes,recipients=recipients,output=scalars(env.player.output)}
end
local result={main=environment(build.calcsTab.mainEnv),calcs=environment(build.calcsTab.calcsEnv),
 methods_preserved=true,business_wrappers=false,observer_modifies_game_state=false,native_owner_closure=false,whole_build_parity=false,
 node_projection_scope="Effective/default scalar fields, declared descriptions, adjacency and original returned modifier lists; no complete graph or alternate transform authority",
 query_authority="Original calcDamage/offence execution captured; separate original-method diagnostic queries do not replace consumer calls"}
for i,r in ipairs(methods) do assert(r[1][r[2]]==auth.refs[i]) end
return result
