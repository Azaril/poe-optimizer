-- Additional read-only fields for the shared original-loader/actor observer.
-- Source objects are reference evidence, never native physical Gem ownership.
return function(rows,saved,origins,envs,effects,scalars,statSets,selectedSet,fields,groupFields)
 local doc,err=common.xml.ParseXML(sniperActorXml);assert(doc and not err)
 local ordinals,n={},0
 local function visit(node)
  if type(node)~="table" or not node.elem then return end
  assert(n<100000);ordinals[node]=n;n=n+1
  for _,child in ipairs(node) do visit(child) end
 end
 visit(doc[1])
 local rawOrigins={}
 for _,section in ipairs(doc[1]) do if type(section)=="table" and section.elem=="Skills" then
  for _,set in ipairs(section) do if type(set)=="table" and set.elem=="SkillSet" then
   local sid=assert(tonumber(set.attrib.id));local gi=0
   for _,group in ipairs(set) do if type(group)=="table" and group.elem=="Skill" then
    gi=gi+1;local captured=assert(sniperLoadedGroups[sid][gi]);local index=0
    for _,gem in ipairs(group) do if type(gem)=="table" and gem.elem=="Gem" then
     index=index+1;local object=captured.gems[index]
     if object then
      assert(not rawOrigins[object])
      rawOrigins[object]={source_ordinal=ordinals[gem],preset=sid,source_group_index=gi,index=index,
       group_source_ordinal=ordinals[group],group_source=group.attrib.source,attributes=scalars(gem.attrib)}
     end
    end end
   end end
  end end
 end end
 local function supports(skill)
  local result={candidates={},accepted_indices={},rejected_indices={}}
  for i,support in ipairs(skill.supportList or {}) do
   local accepted=false
   for _,effect in ipairs(skill.effectList) do if effect==support then assert(not accepted);accepted=true end end
   local origin=rawOrigins[support.srcInstance]
   result.candidates[#result.candidates+1]={index=i,effect=support.grantedEffect.id,level=support.level,
    quality=support.quality,accepted=accepted,source_instance_present=support.srcInstance~=nil,
    exact_saved_origin=origin~=nil,origin=origin}
   local indices=accepted and result.accepted_indices or result.rejected_indices;indices[#indices+1]=i
  end
  for i,effect in ipairs(skill.effectList) do if i>1 and effect.grantedEffect.support then
   local found=false;for _,candidate in ipairs(skill.supportList) do if candidate==effect then found=true end end
   assert(found,"accepted support must retain its exact candidate")
  end end
  return result
 end
 local groups={}
 local setIds={};for id in pairs(build.skillsTab.skillSets) do setIds[#setIds+1]=id end;table.sort(setIds)
 for _,sid in ipairs(setIds) do for index,group in ipairs(build.skillsTab.skillSets[sid].socketGroupList) do
  local relevant=false
  for _,gem in ipairs(group.gemList) do if effects[gem.skillId] then relevant=true end end
  if relevant then
   assert(#groups<256)
   local g={preset=sid,group=index,selected=sid==build.skillsTab.activeSkillSetId,
    group_state=groupFields(group),no_supports=group.noSupports,source_node_id=group.sourceNode and group.sourceNode.id,
    sources={},MAIN={},CALCS={}}
   for i,gem in ipairs(group.gemList) do if effects[gem.skillId] then
    local d=assert(gem.gemData);assert(d.grantedEffect.fromTree==true)
    local list={};for j,effect in ipairs(d.grantedEffectList) do list[#list+1]={index=j,id=effect.id,stat_sets=statSets(effect)} end
    g.sources[#g.sources+1]={index=i,loaded=fields(gem),catalog_key=d.id,game_id=d.gameId,
     from_tree=true,manual_source_ordinal=origins[gem] and origins[gem].source_ordinal,
     saved_origin=rawOrigins[gem],effects=list}
   end end
   for mode,env in pairs(envs) do
    for _,skill in ipairs(env.player.activeSkillList) do
     local effect=skill.activeEffect;local source=effect.srcInstance
     if skill.socketGroup==group and source then
      local slot
      for i,gem in ipairs(group.gemList) do if gem==source and effects[gem.skillId] then assert(not slot);slot=i end end
      if slot then
       assert(g.selected and skill.actor==env.player)
       local d=assert(source.gemData);local ei
       for i,e in ipairs(d.grantedEffectList) do if e==effect.grantedEffect then assert(not ei);ei=i end end
       assert(ei)
       local row={source_index=slot,effect_index=ei,effect=effect.grantedEffect.id,
        exact_source_object=true,exact_group=true,source_kind=origins[source] and "manual_direct" or "allocated",
        manual_source_ordinal=origins[source] and origins[source].source_ordinal,
        saved_origin=rawOrigins[source],source_node_id=group.sourceNode and group.sourceNode.id,
        source_node_exact=group.sourceNode~=nil and env.allocNodes[group.sourceNode.id]==group.sourceNode,
        is_main_skill=env.player.mainSkill==skill,level=effect.level,quality=effect.quality,
        stat_set=selectedSet(effect,mode),supports=supports(skill),minion_present=skill.minion~=nil,
        output_available=skill.output~=nil}
       if origins[source] then assert(not group.sourceNode and not group.noSupports)
       else assert(group.sourceNode and row.source_node_exact and group.noSupports==true) end
       if skill.minion then
        row.minion={type=skill.minion.type,children={}}
        for i,child in ipairs(skill.minion.activeSkillList) do
         assert(child.actor==skill.minion and child.summonSkill==skill and child.supportList==skill.supportList)
         row.minion.children[#row.minion.children+1]={index=i,effect=child.activeEffect.grantedEffect.id,
          exact_actor=true,exact_summon=true,support_list_same_parent=true,
          level=child.activeEffect.level,quality=child.activeEffect.quality,
          stat_set=selectedSet(child.activeEffect,mode),supports=supports(child)}
        end
       end
       g[mode][#g[mode]+1]=row
      end
     end
    end
   end
   groups[#groups+1]=g
  end
 end end
 for _,entry in ipairs(saved) do
  assert(rawOrigins[entry.gem] and rawOrigins[entry.gem].source_ordinal==entry.row.source_ordinal)
 end
 return {runtime_groups=groups,exact_manual_source_joins=true,allocated_sources_are_separate=true,
  native_source_membership_authority=false,native_supported_property_authority=false}
end
