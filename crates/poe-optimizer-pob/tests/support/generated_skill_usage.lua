-- Read-only observations of exact provider-generated source instances.
-- No source business function is replaced and no reporting row is identity-joined.
local base=assert(generatedUsageBase)
local calcs=require("Modules.CalcBase")
local function original(fn,path,line)
 local info=debug.getinfo(fn,"S");local name=info.source:gsub("\\","/")
 assert(info.what=="Lua" and name:sub(-#path)==path and info.linedefined==line)
 return fn
end
local count=original(calcs.getActiveSkillCount,"Modules/CalcDefence.lua",149)
local reservation=original(calcs.doActorLifeManaSpiritReservation,"Modules/CalcDefence.lua",177)
local perform=original(calcs.perform,"Modules/CalcPerform.lua",1193)
local function plain(value,depth)
 depth=depth or 0;assert(depth<24,"generated usage evidence depth")
 if type(value)=="number" and (value~=value or value==math.huge or value==-math.huge) then return tostring(value) end
 if type(value)~="table" then
  assert(value==nil or type(value)=="number" or type(value)=="boolean" or type(value)=="string")
  return value
 end
 local result={};local length=0
 for key,item in pairs(value) do
  length=length+1;assert(length<=10000)
  assert(type(key)=="string" or type(key)=="number")
  result[key]=plain(item,depth+1)
 end
 return result
end
local function scalars(value)
 local result={}
 for key,item in pairs(value or {}) do
  if type(key)=="string" and (type(item)=="number" or type(item)=="boolean" or type(item)=="string") then result[key]=plain(item) end
 end
 return result
end
local function same(a,b)
 if type(a)~=type(b) then return false end
 if type(a)~="table" then return a==b end
 for key,value in pairs(a) do if not same(value,b[key]) then return false end end
 for key in pairs(b) do if a[key]==nil then return false end end
 return true
end
local function present(value) return {present=value~=nil,value=plain(value)} end
local reviewed={SummonSandDjinnPlayer=true,SummonWaterDjinnPlayer=true,FireboltPlayer=true}
local envs={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local function outputs()
 local result={}
 for mode,env in pairs(envs) do
  local output=mode=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
  local pools={}
  for _,pool in ipairs({"Life","Mana","Spirit"}) do
   local breakdown=env.player.breakdown and env.player.breakdown[pool.."Reserved"]
   pools[pool]={reserved=present(env.player.output[pool.."Reserved"]),
    unreserved=present(env.player.output[pool.."Unreserved"]),
    reservations=plain(breakdown and breakdown.reservations)}
  end
  result[mode]={available=output~=nil,scalars=scalars(output),full_dps=plain(output and output.SkillDPS),reservation=pools}
 end
 return result
end
local before=outputs();local result={groups={},outputs=before,full_dps_rows_have_source_identity=false,
 reservation_breakdown_rows_have_source_identity=false,call_boundary_attribution=false}
for _,runtime in ipairs(base.runtime_groups) do
 local set=assert(build.skillsTab.skillSets[runtime.preset]);local group=assert(set.socketGroupList[runtime.index])
 assert(runtime.state.fields.source==group.source)
 local first=group.gemList[1]
 if group.source and first and reviewed[first.skillId] then
  assert(#result.groups<256)
  local saved
  for _,row in ipairs(base.saved_groups) do
   if row.source_ordinal==runtime.source_ordinal then assert(not saved);saved=row end
  end
  local row={preset=runtime.preset,group=runtime.index,selected=runtime.selected,
   source=group.source,source_ordinal=runtime.source_ordinal,
   source_item_id=runtime.state.source_item_id,source_node_id=runtime.state.source_node_id,
   saved_group=saved,group_fields=scalars(group),sources={},MAIN={},CALCS={}}
  for index,instance in ipairs(group.gemList) do
   local observed=assert(runtime.gems[index]);local d=instance.gemData
   local effects={}
   for i,effect in ipairs(d and d.grantedEffectList or {instance.grantedEffect}) do
    assert(effect)
    effects[#effects+1]={index=i,id=effect.id,primary=d and effect==d.grantedEffect or false,
     support=effect.support==true,hide_from_sidebar=effect.hideFromSideBar==true,
     has_global_effect=effect.hasGlobalEffect==true,global_field="enableGlobal"..i,
     global_value=present(instance["enableGlobal"..i])}
   end
   row.sources[#row.sources+1]={index=index,source_ordinal=observed.source_ordinal,
    saved_object_present=observed.saved_object_present,fields=scalars(instance),effects=effects,
    catalog=d and {id=d.id,game_id=d.gameId,variant_id=d.variantId,vaal=d.vaalGem==true}}
  end
  for mode,env in pairs(envs) do
   for _,skill in ipairs(env.player.activeSkillList) do
    if skill.socketGroup==group then
     assert(row.selected and skill.actor==env.player)
     local effect=skill.activeEffect;local instance=assert(effect.srcInstance);local sourceIndex
     for i,source in ipairs(group.gemList) do if source==instance then assert(not sourceIndex);sourceIndex=i end end
     assert(sourceIndex)
     local d=instance.gemData;local effectIndex
     for i,e in ipairs(d and d.grantedEffectList or {instance.grantedEffect}) do
      if e==effect.grantedEffect then assert(not effectIndex);effectIndex=i end
     end
     assert(effectIndex)
     local amount,enabled=count(skill);local types=skill.skillTypes;local pools={}
     for _,pool in ipairs({"Life","Mana","Spirit"}) do
      pools[pool]={flat=present(skill.skillData[pool.."ReservedBase"]),percent=present(skill.skillData[pool.."ReservedPercent"])}
     end
     local actor=skill.minion
     row[mode][#row[mode]+1]={source_index=sourceIndex,effect_index=effectIndex,effect=effect.grantedEffect.id,
      exact_group=true,exact_source_object=true,is_main=env.player.mainSkill==skill,
      source_ordinal=runtime.gems[sourceIndex].source_ordinal,
      helper_count=amount,helper_enabled=enabled,
      prepared_level=effect.level,prepared_quality=effect.quality,
      prepared_level_fields=scalars(effect.grantedEffectLevel),
      global_field="enableGlobal"..effectIndex,global_value=present(instance["enableGlobal"..effectIndex]),
      has_global_effect=effect.grantedEffect.hasGlobalEffect==true,
      full_dps_included=group.includeInFullDPS==true,output_available=skill.output~=nil,
      reservation={has_reservation=types[SkillType.HasReservation]==true,
       multiple_reservation=types[SkillType.MultipleReservation]==true,
       becomes_cost=types[SkillType.ReservationBecomesCost]==true,
       summons_totem=types[SkillType.SummonsTotem]==true,
       ancestral_bond=env.player.modDB:Flag(nil,"AncestralBond")==true,
       autoexertion=skill.skillData.SupportedByAutoexertion==true,
       no_reservation=instance.noReservation==true,actual=pools},
      minion=actor and {type=actor.type,level=actor.level,selected_child=actor.mainSkill and actor.mainSkill.activeEffect.grantedEffect.id},
      source_item_exact=group.sourceItem~=nil and group.sourceItem==env.player.itemList[group.slot],
      source_node_exact=group.sourceNode~=nil and env.allocNodes[group.sourceNode.id]==group.sourceNode}
    end
   end
  end
  result.groups[#result.groups+1]=row
 end
end
assert(same(before,outputs()),"read-only usage observer changed source output")
assert(count==calcs.getActiveSkillCount and reservation==calcs.doActorLifeManaSpiritReservation and perform==calcs.perform)
assert(not debug.gethook() and jit.status()==sniperActorJit)
result.source_methods_preserved=true;result.outputs_preserved=true
result.requested_jit_mode_preserved=true;result.observer_removed_before_evaluation=true
return result
