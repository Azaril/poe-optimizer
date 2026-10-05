-- Supplemental semantic snapshots only. Never constructs an action, resolves a
-- stat-map entry, changes an input, or replaces an original calculation method.
assert(djinnOriginals and djinnOriginals.preserved_after_load)
assert(jit.status() == occurrenceJit, "requested JIT mode changed")
local work = 0
local function charge(n) work=work+(n or 1);assert(work<=2000000,"semantic snapshot work bound") end
local function scalar(v)
 if type(v)=="number" then
  if v~=v then return {non_finite="nan"} end
  if v==math.huge then return {non_finite="positive_infinity"} end
  if v==-math.huge then return {non_finite="negative_infinity"} end
 end
 return v
end
local function scalars(t)
 local r={};for k,v in pairs(t or{})do charge();if type(v)=="number"or type(v)=="string"or type(v)=="boolean"then r[k]=scalar(v)end end;return r
end
local function plain(v,depth)
 charge();if type(v)~="table"then assert(v==nil or type(v)=="number"or type(v)=="boolean"or type(v)=="string");return scalar(v)end
 depth=(depth or 0)+1;assert(depth<=16,"semantic value depth");local r={};for k,x in pairs(v)do assert(type(k)=="string"or type(k)=="number");r[k]=plain(x,depth)end;return r
end
local origins={}
for sid,set in pairs(build.skillsTab.skillSets)do for gi,group in ipairs(set.socketGroupList)do for i,g in ipairs(group.gemList)do
 charge();assert(not origins[g],"saved physical source has ambiguous occurrence")
 origins[g]={preset=sid,group=gi,index=i,generated_source=group.source,slot=group.slot,gem_id=g.gemId,skill_id=g.skillId}
end end end
local count=0
local function action(a,env)
 count=count+1;assert(count<=8192,"semantic action bound")
 local e=assert(a.activeEffect);local d=assert(e.grantedEffect)
 local s=assert(env.mode=="CALCS"and e.statSetCalcs or e.statSet)
 assert(d.statSets[s.index]==s.statSet,"selected constructed stat set identity changed")
 local origin=e.srcInstance and origins[e.srcInstance]
 local parent=a.summonSkill and a.summonSkill.activeEffect
 return {effect=d.id,level=e.level,quality=e.quality,source_instance_present=e.srcInstance~=nil,
  source=plain(origin),summon_source=parent and plain(origins[parent.srcInstance]),summon_effect=parent and parent.grantedEffect.id,
  stat_set_index=s.index,stat_set_id=s.statSet.id,stat_set_label=s.statSet.label,
  skill_part=a.skillPart,active_stage=a.activeStageCount,active_mines=a.activeMineCount,
  skill_flags=scalars(a.skillFlags),skill_data=scalars(a.skillData),
  output_available=a.output~=nil,output=scalars(a.output)}
end
local function actor(a,env,depth,seen)
 charge();assert(depth<=16 and not seen[a],"cyclic actor ownership");seen[a]=true
 local out={type=a.type,level=a.level,output_available=a.output~=nil,output=scalars(a.output),
  main_available=a.mainSkill~=nil,main=a.mainSkill and action(a.mainSkill,env),actions={}}
 for index,s in ipairs(a.activeSkillList or{})do
  charge();assert(s.actor==a,"action actor identity changed")
  local row={index=index,selected=s==a.mainSkill,action=action(s,env)}
  if s.minion then
   assert(s.minion.parent==a,"minion parent identity changed")
   row.minion=actor(s.minion,env,depth+1,seen)
  end
  out.actions[#out.actions+1]=row
 end
 seen[a]=nil;return out
end
local out={selection={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
 spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup},modes={}}
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 local output=assert(mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)
 out.modes[mode]={mode=env.mode,main_group=env.mainSocketGroup,output=scalars(output),
  full_dps=plain(output.SkillDPS),player=actor(env.player,env,1,{}),
  selected_minion_available=env.minion~=nil,selected_minion=env.minion and actor(env.minion,env,1,{})}
end
assert(build.buildFlag==false and jit.status()==occurrenceJit)
return out
