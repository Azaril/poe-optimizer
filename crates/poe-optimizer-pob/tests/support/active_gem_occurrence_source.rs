//! Read-only observations after the authenticated, complete source lifecycle.
pub const OBSERVE: &str = r##"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line);return f
end
local methods={
 load=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303),
 process=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242),
 init=original(calcs.initEnv,"Modules/CalcSetup.lua",717),
 create=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144),
 mods=original(calcs.buildActiveSkillModList,"Modules/CalcActiveSkill.lua",426),
 count=original(calcs.getActiveSkillCount,"Modules/CalcDefence.lua",149),
 full=original(calcs.calcFullDPS,"Modules/Calcs.lua",251),
 output=original(calcs.buildOutput,"Modules/Calcs.lua",469),
}
local function copy(v)if type(v)~="table"then return v end;local r={};for k,x in pairs(v)do r[k]=copy(x)end;return r end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then r[k]=v end end;return r end
local function fields(g)
 return{gem_id=g.gemId,skill_id=g.skillId,name=g.nameSpec,level=g.level,quality=g.quality,
 corrupted=g.corrupted,corrupt_level=g.corruptLevel,enabled=g.enabled,count=g.count,
 global_1=g.enableGlobal1,global_2=g.enableGlobal2,stat_set=copy(g.statSet),stat_set_calcs=copy(g.statSetCalcs),
 skill_part=g.skillPart,skill_part_calcs=g.skillPartCalcs,stage=g.skillStageCount,stage_calcs=g.skillStageCountCalcs,
 mine=g.skillMineCount,mine_calcs=g.skillMineCountCalcs,minion=g.skillMinion,minion_calcs=g.skillMinionCalcs,
 minion_stat_sets=copy(g.skillMinionSkillStatSetIndexLookup),minion_stat_sets_calcs=copy(g.skillMinionSkillStatSetIndexLookupCalcs)}
end
local function groupFields(g)return{enabled=g.enabled,slot_enabled=g.slotEnabled,group_count=g.groupCount,include=g.includeInFullDPS,
 main=g.mainActiveSkill,calcs=g.mainActiveSkillCalcs,source=g.source,slot=g.slot}end
local function fresh(node)
 local tab=setmetatable({build=build,skillSets={[1]={socketGroupList={}}}},{__index=build.skillsTab})
 methods.load(tab,node,1);assert(#tab.skillSets[1].socketGroupList==1);return tab.skillSets[1].socketGroupList[1]
end
local function names(t)local r={};for name,id in pairs(SkillType)do if t and t[id]then r[#r+1]=name end end;table.sort(r);return r end
local selected={skills=build.skillsTab.activeSkillSetId,items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local environments={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local outputs={MAIN=assert(build.calcsTab.mainOutput),CALCS=assert(build.calcsTab.calcsOutput)}
local snapshots={MAIN=scalars(outputs.MAIN),CALCS=scalars(outputs.CALCS)}
local saved={};for sid,set in pairs(build.skillsTab.skillSets)do for gi,group in ipairs(set.socketGroupList)do for i,g in ipairs(group.gemList)do
 saved[#saved+1]={sid=sid,gi=gi,i=i,group=group,gem=g,fields=fields(g),group_fields=groupFields(group)}
end end end
local reviewed={};for _,key in ipairs(occurrenceReviewed)do reviewed[key]=true end
local function action(a,env,physical)
 local effect=a.activeEffect;local d=effect.grantedEffect;local set=env.mode=="CALCS"and effect.statSetCalcs or effect.statSet
 local flags=a.skillFlags or set.skillFlags;local skillData=a.skillData or{}
 local inventory={};local matched
 for i,row in ipairs(d.statSets or{})do inventory[#inventory+1]={index=i,id=row.id,label=row.label,base_flags=scalars(row.baseFlags)};if set.statSet==row then matched=i end end
 assert(matched==set.index,"selected stat set must be the actual declared table")
 local count,enabled=methods.count(a)
 local umbralFlag;local umbralBuff
 if env.modDB then umbralFlag=env.modDB:Flag(nil,"UmbralWell")==true end
 if a.skillModList and env.player.mainSkill and env.player.mainSkill.skillCfg then
  umbralBuff=a.skillModList:Sum("BASE",env.player.mainSkill.skillCfg,"UmbralWellBuffValue")
 end
 local child={};if a.minion then for _,s in ipairs(a.minion.activeSkillList or{})do child[#child+1]={effect=s.activeEffect.grantedEffect.id,summon_parent=s.summonSkill==a,types=names(s.skillTypes)}end end
 return{effect=d.id,name=d.name,physical_source=effect.srcInstance==physical,level=effect.level,quality=effect.quality,
 stat_sets=inventory,selected_index=set.index,selected_table_index=matched,part=a.skillPart,part_name=a.skillPartName,
 declared_parts=copy(d.parts),initial_types=names(d.skillTypes),final_types=names(a.skillTypes),minion_types=names(a.minionSkillTypes),
 has_global_effect=d.hasGlobalEffect==true,count=count,count_enabled=enabled,disable=flags.disable==true,
 instance_flags_present=a.skillFlags~=nil,selected_stat_set_flags_present=set.skillFlags~=nil,
 skill_data_present=a.skillData~=nil,skill_types_present=a.skillTypes~=nil,minion_types_present=a.minionSkillTypes~=nil,
 is_main_skill=env.player.mainSkill==a,actor_is_player=a.actor==env.player,
 multiple_reservation=a.skillTypes[SkillType.MultipleReservation]==true,
 umbral_environment_db_present=env.modDB~=nil,umbral_environment_flag=umbralFlag,
 umbral_buff_value=umbralBuff,umbral_skill_mod_list_present=a.skillModList~=nil,
 umbral_main_skill_cfg_present=env.player.mainSkill~=nil and env.player.mainSkill.skillCfg~=nil,
 minion_limit_present=a.minion~=nil and a.minion.minionData~=nil and a.minion.minionData.limit~=nil,
 creates_minion=a.skillTypes[SkillType.CreatesMinion]==true,minion_present=a.minion~=nil,minion_skills=child,
 reservation={life=skillData.LifeReservedBase,mana=skillData.ManaReservedBase,spirit=skillData.SpiritReservedBase},
 active_stage=a.activeStageCount,active_mines=a.activeMineCount}
end
local function actions(physical,env)
 local rows={};for _,a in ipairs(env.player.activeSkillList)do if a.activeEffect.srcInstance==physical then rows[#rows+1]=action(a,env,physical)end end;return rows
end
local doc,err=common.xml.ParseXML(occurrenceXml);assert(doc and not err)
local ordinal={};local nextOrdinal=0;local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinal[n]=nextOrdinal;nextOrdinal=nextOrdinal+1;for _,c in ipairs(n)do enumerate(c)end end;enumerate(doc[1])
local skills;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Skills"then assert(not skills);skills=n end end;assert(skills)
local result={selected={},fresh_probes={},selection=selected,outputs=snapshots,
 actor_outputs={MAIN={player=scalars(environments.MAIN.player.output),minion=environments.MAIN.minion and scalars(environments.MAIN.minion.output)},
 CALCS={player=scalars(environments.CALCS.player.output),minion=environments.CALCS.minion and scalars(environments.CALCS.minion.output)}},
 full_dps={MAIN=copy(outputs.MAIN.SkillDPS),CALCS=copy(outputs.CALCS.SkillDPS)}}
local probeNode
for _,setNode in ipairs(skills)do if type(setNode)=="table"and setNode.elem=="SkillSet"and tonumber(setNode.attrib.id)==selected.skills then
 local gi=0;for _,node in ipairs(setNode)do if type(node)=="table"and node.elem=="Skill"then
 gi=gi+1;local group=assert(build.skillsTab.skillSets[selected.skills].socketGroupList[gi])
 if not node.attrib.source or node.attrib.source==""then
  local isolated=fresh(node);assert(isolated~=group)
  for i,child in ipairs(node)do if type(child)=="table"and child.elem=="Gem"then
   local physical=assert(group.gemList[i]);local isolatedGem=assert(isolated.gemList[i]);local d=physical.gemData
   local eligible=d and reviewed[d.id];for name in pairs(child.attrib)do if name:find("Minion",1,true)then eligible=false end end
   -- Child maps are deliberately represented in full control runs, not silently ignored.
   if eligible then
    assert(not d.grantedEffect.support and not d.grantedEffect.fromTree and #d.grantedEffectList==1 and #d.additionalGrantedEffects==0)
    assert(isolatedGem~=physical and isolatedGem.gemData==d)
    local row={source_ordinal=ordinal[child],group=gi,index=i,physical_id=d.id,attributes=copy(child.attrib),group_attributes=copy(node.attrib),
     loaded=fields(physical),fresh=fields(isolatedGem),group_state=groupFields(group),MAIN=actions(physical,environments.MAIN),CALCS=actions(physical,environments.CALCS)}
    result.selected[#result.selected+1]=row
    if d.grantedEffect.id=="FrostBombPlayer"then probeNode=copy(node)end
   end
  end end
 end
 end end
end end
if occurrenceOriginal==5 then
 assert(probeNode);local target=1
 for i,n in ipairs(probeNode)do if n.attrib.skillId=="FrostBombPlayer"then target=i end end
 local function probe(name,edit)
  local n=copy(probeNode);edit(n[target],n);local g=fresh(n)
  result.fresh_probes[#result.fresh_probes+1]={name=name,gem=fields(g.gemList[target]),group=groupFields(g)}
 end
 probe("count-absent",function(g)g.attrib.count=nil end)
 probe("count-malformed",function(g)g.attrib.count="bad"end)
 probe("count-zero",function(g)g.attrib.count="0"end)
 probe("global-absent",function(g)g.attrib.enableGlobal1=nil;g.attrib.enableGlobal2=nil end)
 probe("global-false",function(g)g.attrib.enableGlobal1="false";g.attrib.enableGlobal2="false"end)
 probe("legacy-statset",function(g)g.attrib.statSetIndex="7";g.attrib.statSetIndexCalcs="9"end)
 probe("effect-statset-maps",function(g)g[#g+1]={elem="StatSetIndex",attrib={grantedEffect="FrostBombPlayer",index="1"}};g[#g+1]={elem="StatSetCalcsIndex",attrib={grantedEffect="FrostBombPlayer",index="2"}}end)
 probe("first-gem-parent-override",function(g,n)n[1].attrib.skillPart="3";n.attrib.skillPart="7"end)
end
for _,s in ipairs(saved)do local g=build.skillsTab.skillSets[s.sid].socketGroupList[s.gi];assert(g==s.group and g.gemList[s.i]==s.gem and equal(fields(s.gem),s.fields)and equal(groupFields(g),s.group_fields))end
assert(build.skillsTab.activeSkillSetId==selected.skills and build.itemsTab.activeItemSetId==selected.items and build.treeTab.activeSpec==selected.spec and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
for mode,env in pairs(environments)do assert((mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)==env);assert((mode=="MAIN"and build.calcsTab.mainOutput or build.calcsTab.calcsOutput)==outputs[mode]);assert(equal(scalars(outputs[mode]),snapshots[mode]))end
assert(methods.load==build.skillsTab.LoadSkill and methods.process==build.skillsTab.ProcessSocketGroup and methods.init==calcs.initEnv and methods.create==calcs.createActiveSkill and methods.mods==calcs.buildActiveSkillModList and methods.count==calcs.getActiveSkillCount and methods.full==calcs.calcFullDPS and methods.output==calcs.buildOutput)
result.saved_instances_preserved=true;result.selected_state_preserved=true;result.main_and_calcs_outputs_preserved=true;result.source_methods_preserved=true;result.fresh_objects=true
return result
"##;
