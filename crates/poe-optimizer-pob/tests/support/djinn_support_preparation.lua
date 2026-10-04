-- Read-only call/return observations of the original support preparation phase.
-- Definition types, per-call mutable types, parent types and final types are separate.
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local create,createAuth=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local canSupport,canAuth=original(calcLib.canGrantedEffectSupportActiveSkill,"Modules/CalcTools.lua",108)
local supportIds={"SupportBiddingPlayerTwo","SupportMagnifiedAreaPlayer","SupportMusterPlayer","SupportChillingIcePlayer"}
local supports={};for _,id in ipairs(supportIds)do supports[id]=true end
local effects={SummonSandDjinnPlayer=true,CommandSandDjinnKnifeThrowPlayer=true,
 KnifeThrowSandDjinn=true,ExplosiveTeleportSandDjinn=true,HandSlamSandDjinn=true,
 SummonWaterDjinnPlayer=true,CommandWaterDjinnBubblePlayer=true,
 WaterBubbleWaterDjinn=true,ChilledGroundBurstWaterDjinn=true,ESRechargeForceRestartWaterDjinn=true,
 ChilledGroundOasisConvertWaterDjinn=true,PassiveTriggeredManaWaveWaterDjinn=true}
local typeNames={};for name,id in pairs(SkillType)do if type(id)=="number"then assert(not typeNames[id]);typeNames[id]=name end end
local function scalar(v)
 assert(v==nil or type(v)=="number"or type(v)=="boolean"or type(v)=="string")
 if type(v)=="number"then assert(v==v and v~=math.huge and v~=-math.huge)end
 return v
end
local function plain(v,depth)
 if type(v)~="table"then return scalar(v)end
 depth=(depth or 0)+1;assert(depth<12);local out={};local n=0
 for k,x in pairs(v)do n=n+1;assert(n<4096);out[k]=plain(x,depth)end;return out
end
local function typeSet(t)
 local out={};for id,value in pairs(t or{})do assert(type(id)=="number"and typeNames[id]);out[#out+1]={id=id,name=typeNames[id],value=scalar(value)}end
 table.sort(out,function(a,b)return a.id<b.id end);return {present=t~=nil,values=out}
end
local function expression(t)
 local out={};for i,id in ipairs(t or{})do assert(typeNames[id]);out[i]={id=id,name=typeNames[id]}end
 return {present=t~=nil,values=out}
end
local function flags(a)
 local e=a.activeEffect;local g=e.grantedEffect;local s=e.srcInstance
 return {cannot_be_supported=g.cannotBeSupported,gem_data_present=e.gemData~=nil,
  effect_from_item=g.fromItem,effect_from_tree=g.fromTree,mod_source=g.modSource,
  source_instance_present=s~=nil,source_from_item=s and s.fromItem,
  actor_enemy_present=a.actor.enemy~=nil,actor_is_enemy_player=a.actor.enemy and a.actor.enemy.player==a.actor}
end
local function types(a)
 return {skill_types=typeSet(a.skillTypes),minion_types=typeSet(a.minionSkillTypes)}
end
local function definition(g)
 return {effect=g.id,skill_types=typeSet(g.skillTypes),minion_types=typeSet(g.minionSkillTypes),
  require_types=expression(g.requireSkillTypes),exclude_types=expression(g.excludeSkillTypes),
  add_types=expression(g.addSkillTypes),family_present=g.gemFamily~=nil,family=plain(g.gemFamily),
  support=g.support,support_gems_only=g.supportGemsOnly,ignore_minion_types=g.ignoreMinionTypes,
  from_item=g.fromItem,is_trigger=g.isTrigger,add_flags=plain(g.addFlags)}
end
if djinnSupportPhase=="before"then
 local prior,mask,count=debug.gethook();assert(prior==nil)
 local enabled=jit.status();assert(enabled==djinnJit)
 local auth={records={},by_skill={},frames={},create_calls=0,predicate_calls=0,refs={create,canSupport}}
 local function hook(event)
  local f=debug.getinfo(2,"f").func
  if f~=create and f~=canSupport then return end
  if event~="call"and event~="return"then return end
  local v={};for i=1,64 do local name,value=debug.getlocal(2,i);if not name then break end;v[name]=value end
  if f==create then
   local e=v.activeEffect;if not e or not effects[e.grantedEffect.id]then return end
   if event=="call"then
    assert(not auth.frames[e]);auth.create_calls=auth.create_calls+1;assert(auth.create_calls<=1024)
    auth.frames[e]={effect=e,env=v.env,actor=v.actor,group=v.socketGroup,parent=v.summonSkill,
     support_list=v.supportList,initial_definition=definition(e.grantedEffect),calls={}}
   else
    local a=assert(v.activeSkill);local r=assert(auth.frames[e]);auth.frames[e]=nil
    assert(a.activeEffect==e and a.actor==r.actor and a.socketGroup==r.group and a.summonSkill==r.parent and a.supportList==r.support_list)
    r.skill=a;r.constructor_types=types(a);r.constructor_flags=flags(a)
    r.constructor_accepted={};for i,s in ipairs(a.supportList)do for _,accepted in ipairs(a.effectList)do if s==accepted then r.constructor_accepted[#r.constructor_accepted+1]=i end end end
    auth.records[#auth.records+1]=r;assert(not auth.by_skill[a]);auth.by_skill[a]=r
   end
  elseif event=="call"then
   local a=v.activeSkill;local g=v.grantedEffect
   if not a or not effects[a.activeEffect.grantedEffect.id]or not supports[g.id]then return end
   local r=auth.frames[a.activeEffect]
   -- Only the actual calls from this constructor are preparation observations.
   if not r then return end
   assert(a.actor==r.actor and a.socketGroup==r.group and a.summonSkill==r.parent and a.supportList==r.support_list)
   auth.predicate_calls=auth.predicate_calls+1;assert(auth.predicate_calls<=16384)
   local positions={};for i,s in ipairs(a.supportList)do if s.grantedEffect==g then positions[#positions+1]=i end end;assert(#positions>0)
   r.calls[#r.calls+1]={support=g.id,candidate_positions=positions,mutable_types=types(a),
    parent_present=a.summonSkill~=nil,parent_prepared_types=a.summonSkill and types(a.summonSkill),
    effective_type_owner=a.summonSkill and"summoner"or"self",flags=flags(a),
    exact_actor=true,exact_parent=true,exact_support_definition=true}
  end
 end
 jit.flush();assert(jit.status()==enabled);debug.sethook(hook,"cr")
 djinnSupportAuth=auth
 return function()
  assert(debug.gethook()==hook);debug.sethook(prior,mask,count)
  assert(next(auth.frames)==nil,"unclosed original constructor observation")
  assert(jit.status()==enabled and calcs.createActiveSkill==create and calcLib.canGrantedEffectSupportActiveSkill==canSupport)
  auth.finished=true;auth.hook_removed=true;auth.jit_mode_preserved=true
 end
end
assert(djinnSupportPhase=="observe")
local auth=assert(djinnSupportAuth);assert(auth.finished and auth.hook_removed and debug.gethook()==nil)
assert(calcs.createActiveSkill==auth.refs[1]and calcLib.canGrantedEffectSupportActiveSkill==auth.refs[2])
local doc,err=common.xml.ParseXML(djinnXml);assert(doc and not err)
local ordinals,nextOrdinal={},0
local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinals[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,c in ipairs(n)do enumerate(c)end end;enumerate(doc[1])
local raw={};local skills;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Skills"then skills=n end end;assert(skills)
local function key(sid,effect,source)return tostring(sid).."/"..effect.."/"..(source or"manual")end
for _,set in ipairs(skills)do if type(set)=="table"and set.elem=="SkillSet"then
 for _,g in ipairs(set)do if type(g)=="table"and g.elem=="Skill"then
  local gems={};for _,gem in ipairs(g)do if type(gem)=="table"and gem.elem=="Gem"then gems[#gems+1]=gem end end
  if gems[1]and(effects[gems[1].attrib.skillId])then local k=key(set.attrib.id,gems[1].attrib.skillId,g.attrib.source);assert(not raw[k]);raw[k]={group=g,gems=gems}end
 end end
end end
local origins,groupOrigins={},{}
for sid,set in pairs(build.skillsTab.skillSets)do for gi,g in ipairs(set.socketGroupList)do
 local first=g.gemList[1];if first and effects[first.skillId]then
  local k=key(sid,first.skillId,g.source);local source=raw[k]
  groupOrigins[g]={key=k,preset=sid,index=gi,source=g.source,source_present=g.source~=nil,source_ordinal=source and ordinals[source.group]}
  if source then assert(#g.gemList==#source.gems)end
  for i,gem in ipairs(g.gemList)do
   local saved=source and source.gems[i];if saved then assert(saved.attrib.skillId==gem.skillId)end
   assert(not origins[gem]);origins[gem]={group=k,index=i,source_ordinal=saved and ordinals[saved],
    effect=gem.skillId,saved_attributes=saved and plain(saved.attrib),enabled=gem.enabled}
  end
 end
end end
local used={};local contexts={};local serial=0
local function context(a,env,parent)
 local r=assert(auth.by_skill[a],"retained Djinn skill has no original constructor observation")
 assert(r.env==env and r.actor==a.actor and r.group==a.socketGroup and r.parent==a.summonSkill and r.effect==a.activeEffect)
 assert(not used[a]);used[a]=true;serial=serial+1
 local sourceOwner=a.summonSkill or a
 local row={index=serial,mode=env.mode,effect=a.activeEffect.grantedEffect.id,
  group=plain(assert(groupOrigins[sourceOwner.socketGroup])),source=plain(origins[a.activeEffect.srcInstance]),
  socket_group_present=a.socketGroup~=nil,source_owner_is_parent=a.summonSkill~=nil,
  source_instance_present=a.activeEffect.srcInstance~=nil,parent_context=parent,
  actor_is_player=a.actor==env.player,parent_present=a.summonSkill~=nil,
  initial_definition=r.initial_definition,constructor_types=r.constructor_types,
  constructor_flags=r.constructor_flags,calls=r.calls,constructor_accepted=r.constructor_accepted,
  final_types=types(a),final_flags=flags(a),candidates={},accepted_indices={},
  exact_constructor_object=true,exact_actor=true,exact_group=true,exact_parent=true}
 if parent then assert(a.actor==a.summonSkill.minion and a.actor.parent==env.player and a.supportList==a.summonSkill.supportList)end
 for i,s in ipairs(a.supportList)do
  local accepted=false;for _,e in ipairs(a.effectList)do if s==e then assert(not accepted);accepted=true end end
  local origin=origins[s.srcInstance];assert(origin,"candidate support lacks exact source occurrence")
  row.candidates[#row.candidates+1]={index=i,effect=s.grantedEffect.id,source=plain(origin),accepted=accepted,
   exact_definition=s.grantedEffect==build.data.skills[s.grantedEffect.id],level=s.level,quality=s.quality}
  if accepted then row.accepted_indices[#row.accepted_indices+1]=i end
 end
 assert(#row.accepted_indices==#row.constructor_accepted)
 for i,index in ipairs(row.accepted_indices)do assert(index==row.constructor_accepted[i])end
 contexts[#contexts+1]=row;return row.index
end
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 for _,a in ipairs(env.player.activeSkillList)do if effects[a.activeEffect.grantedEffect.id]then
  local index=context(a,env,nil)
  if a.minion then for _,child in ipairs(a.minion.activeSkillList)do assert(effects[child.activeEffect.grantedEffect.id]);context(child,env,index)end end
 end end
end
local defs={};for _,id in ipairs(supportIds)do local g=assert(build.data.skills[id]);defs[#defs+1]=definition(g)end
local retained=0;for _ in pairs(used)do retained=retained+1 end
return {methods={create=createAuth,can_support=canAuth},definitions=defs,contexts=contexts,
 observed_constructor_calls=auth.create_calls,observed_predicate_calls=auth.predicate_calls,
 retained_contexts=retained,unretained_constructor_calls=auth.create_calls-retained,
 original_methods_preserved=true,hook_removed=auth.hook_removed,jit_mode_preserved=auth.jit_mode_preserved,
 business_wrappers=false,source_tables_mutated=false}
