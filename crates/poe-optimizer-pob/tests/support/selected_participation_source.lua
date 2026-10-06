-- Original execution identities and output snapshots, not native participation rules.
-- The optional loader provenance comes from the existing, removed LoadSkill hook.
local calcs=require("Modules.CalcBase")
local methods={
 {"callback",_G,"runCallback","HeadlessWrapper.lua",17},
 {"frame",build,"OnFrame","Modules/Build.lua",1285},
 {"tab_output",build.calcsTab,"BuildOutput","Classes/CalcsTab.lua",486},
 {"load_skills",common.classes.SkillsTab,"Load","Classes/SkillsTab.lua",407},
 {"load_skill",common.classes.SkillsTab,"LoadSkill","Classes/SkillsTab.lua",303},
 {"process_group",common.classes.SkillsTab,"ProcessSocketGroup","Classes/SkillsTab.lua",1242},
 {"init",calcs,"initEnv","Modules/CalcSetup.lua",717},
 {"create",calcs,"createActiveSkill","Modules/CalcActiveSkill.lua",144},
 {"mods",calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
 {"minion_skills",calcs,"createMinionSkills","Modules/CalcActiveSkill.lua",1116},
 {"perform",calcs,"perform","Modules/CalcPerform.lua",1193},
 {"output",calcs,"buildOutput","Modules/Calcs.lua",469},
}
local refs,auth={},{}
for _,m in ipairs(methods) do
 local fn=assert(m[2][m[3]]);local info=debug.getinfo(fn,"S");local path=info.source:gsub("\\","/")
 assert(info.what=="Lua" and path:sub(-#m[4])==m[4] and info.linedefined==m[5],m[1])
 refs[m[1]]=fn;auth[m[1]]={path=m[4],first=info.linedefined,last=info.lastlinedefined}
end
local function unchanged()
 assert(not debug.gethook() and jit.status()==participationJit)
 for _,m in ipairs(methods) do assert(m[2][m[3]]==refs[m[1]],m[1].." changed") end
end
local function scalar(v)
 if type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) then return tostring(v) end
 return v
end
local function scalars(t)
 local r={};local n=0
 for k,v in pairs(t or {}) do
  n=n+1;assert(n<=10000)
  if type(k)=="string" and (type(v)=="string" or type(v)=="number" or type(v)=="boolean") then r[k]=scalar(v) end
 end
 return r
end
local function present(v) return {present=v~=nil,value=scalar(v)} end
local function same(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not same(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end
 return true
end
local function item_id(item)
 if not item then return nil end
 local id
 for i,v in pairs(build.itemsTab.items) do if v==item then assert(not id);id=i end end
 assert(id,"item pointer must belong to the loaded inventory")
 return id
end
local function group_index(group)
 local index
 for i,g in ipairs(build.skillsTab.socketGroupList) do
  if g==group then assert(not index);index=i end
 end
 return index
end
local function effect_identity(skill)
 if not skill then return {present=false} end
 local effect=assert(skill.activeEffect);local granted=assert(effect.grantedEffect)
 return {present=true,effect=granted.id,level=effect.level,quality=effect.quality,
  stat_set=effect.statSet and effect.statSet.index,stat_set_calcs=effect.statSetCalcs and effect.statSetCalcs.index}
end
local function action(skill,env)
 local group=skill.socketGroup;local effect=assert(skill.activeEffect);local source=effect.srcInstance
 local gi=group_index(group);local gemIndex
 if group then
  assert(gi,"active source group must be the actual selected runtime group")
  for i,g in ipairs(group.gemList) do if g==source then assert(not gemIndex);gemIndex=i end end
  assert(gemIndex,"active effect must reference the exact runtime Gem object")
 end
 local r={identity=effect_identity(skill),is_main=skill==env.player.mainSkill,
  actor_is_player=skill.actor==env.player,group_present=group~=nil,group_index=gi,
  exact_source_gem_index=gemIndex,source_present=source~=nil,
  source=source and scalars(source),group=group and scalars(group),
  source_item_id=group and item_id(group.sourceItem),source_node_id=group and group.sourceNode and group.sourceNode.id,
  source_item_equipped=group and group.sourceItem and env.player.itemList[group.slot]==group.sourceItem or false,
  source_node_allocated=group and group.sourceNode and env.allocNodes[group.sourceNode.id]==group.sourceNode or false,
  output_present=skill.output~=nil,output=scalars(skill.output)}
 if skill.minion then
  local actor=skill.minion;local main=actor.mainSkill;local mainIndex;local children={}
  for i,c in ipairs(actor.activeSkillList or {}) do
   assert(i<=128);assert(c.actor==actor)
   if c==main then assert(not mainIndex);mainIndex=i end
   children[#children+1]={index=i,identity=effect_identity(c),is_main=c==main,
    exact_actor=true,exact_summon=c.summonSkill==skill,output_present=c.output~=nil,output=scalars(c.output)}
  end
  assert(not main or mainIndex,"minion main skill must belong to the exact actor")
  r.minion={type=actor.type,level=actor.level,main=effect_identity(main),main_index=mainIndex,
   selected=env.minion==actor,children=children,output_present=actor.output~=nil,output=scalars(actor.output)}
 end
 return r
end
local function snapshot()
 local modes={}
 for _,mode in ipairs({"MAIN","CALCS"}) do
  local env=assert(mode=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  local output=mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
  local active={};local mainIndex
  for i,skill in ipairs(env.player.activeSkillList) do
   assert(i<=256);active[#active+1]=action(skill,env)
   if skill==env.player.mainSkill then assert(not mainIndex);mainIndex=i end
  end
  assert(mainIndex,"actual main skill must be present in original activeSkillList")
  local main=active[mainIndex]
  local fallback=main.identity.effect=="MeleeUnarmedPlayer" and not main.group_present and not main.source_present
  modes[mode]={selected_group=env.mainSocketGroup,main_index=mainIndex,main=main,
   default_unarmed_without_source=fallback,active=active,output_present=output~=nil,output=scalars(output),
   output_is_player=output==env.player.output,output_is_selected_minion=env.minion~=nil and output==env.minion.output,
   weapon_one_item_id=present(item_id(env.player.itemList["Weapon 1"])),
   node_13289_allocated=env.allocNodes[13289]~=nil}
 end
 return {selection={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,
  spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup,
  calcs_group=build.calcsTab.calcsEnv.mainSocketGroup},modes=modes}
end
local function capture(base)
 unchanged();local before=snapshot();local result={state=before,methods=auth,
  calculation_hook=false,business_wrappers=false,native_participation_authority=false}
 if base then
  assert(base.exact_loader_capture and base.observer_removed_before_evaluation and base.requested_jit_mode_preserved)
  local saved,runtime={},{ }
  for _,g in ipairs(base.saved_groups) do if g.selected then saved[#saved+1]=g end end
  for _,g in ipairs(base.runtime_groups) do if g.selected then runtime[#runtime+1]=g end end
  result.provenance={saved=saved,runtime=runtime,granted_skills=base.granted_skills,
   exact_loader_capture=true,loader_hook_removed_before_calculation=true}
 end
 assert(same(before,snapshot()),"participation observer changed actual identities or outputs")
 unchanged();result.source_methods_preserved=true;result.outputs_preserved=true
 return result
end
local function rebuild()
 unchanged();assert(build.buildFlag==false)
 local revision=build.outputRevision;local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 build.buildFlag=true;refs.callback("OnFrame")
 unchanged();assert(build.buildFlag==false and build.outputRevision==revision+1)
 assert(build.calcsTab.mainEnv~=mainEnv and build.calcsTab.calcsEnv~=calcsEnv)
end
return {capture=capture,rebuild=rebuild}
