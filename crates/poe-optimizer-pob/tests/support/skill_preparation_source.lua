-- Test-only UI boundary and observations around unchanged original methods.
-- This harness always runs complete Load, LoadSkill, ProcessSocketGroup,
-- FindSkillGem, SetDisplayGroup, and reached CalcTools helper bodies.
local function noop() end
local function dropdown(list)
 return setmetatable({list=list,selIndex=1},{__index=DropDownClass})
end
local function edit()
 return setmetatable({buf='',ResetUndo=noop,changeFunc=function()error('unexpected UI notification')end},{__index=EditClass})
end
local function scalar_fields(value)
 local result={}
 for key,item in pairs(value)do
  local kind=type(item)
  if kind=='boolean'or kind=='number'or kind=='string'then result[key]=item end
 end
 return result
end
local function load(xml,selection)
 local roots,parse_error=originalXml.ParseXML(xml);assert(roots,parse_error)
 local ordinal=0
 local function stamp(node)
  if type(node)~='table'then return end
  node._source_ordinal=ordinal;ordinal=ordinal+1
  for _,child in ipairs(node)do stamp(child)end
 end
 stamp(roots[1])
 local character_level=1
 for _,node in ipairs(roots[1])do
  if type(node)=='table'and node.elem=='Build'then character_level=tonumber(node.attrib.level)or 1 end
 end
 local build={data=data,characterLevel=character_level,SyncLoadouts=noop}
 local tab=setmetatable({build=build,controls={},ResetUndo=noop},{__index=SkillsTabClass})
 original_skill_defaults(tab)
 local lists=original_control_lists()
 tab.controls.defaultLevel=dropdown(lists.defaultGemLevelList)
 tab.controls.defaultQuality=edit()
 tab.controls.showSupportGemTypes=dropdown(lists.showSupportGemTypeList)
 tab.controls.sortGemsByDPSFieldControl=dropdown(lists.sortGemTypeList)
 tab.controls.sortGemsByDPS={};tab.controls.showLegacyGems={};tab.controls.groupList={}
 tab.controls.groupLabel=edit();tab.controls.groupSlot=dropdown(lists.groupSlotDropList)
 tab.controls.groupEnabled={};tab.controls.includeInFullDPS={};tab.controls.groupCount=edit()
 tab.UpdateGemSlots=function(self)
  self.gemSlots={}
  for index in ipairs(self.displayGroup.gemList)do
   self.gemSlots[index]={nameSpec=edit(),level=edit(),quality=edit(),enabled={},enableGlobal1={},enableGlobal2={},count=edit(),corruptLevel={}}
  end
 end
 local groups,seen,current={},{}
 tab.ProcessSocketGroup=function(self,group)
  local observed=seen[group]
  if not observed then
   assert(current,'unattributed group')
   observed={group=group,node=current,source=current._source_ordinal,attached=false,processing_passes=0}
   table.insert(groups,observed);seen[group]=observed
  end
  observed.processing_passes=observed.processing_passes+1
  -- Observation only: forward complete original processing and preserve errors.
  return SkillsTabClass.ProcessSocketGroup(self,group)
 end
 tab.LoadSkill=function(self,node,set_id)
  current=node
  local ok,err=pcall(SkillsTabClass.LoadSkill,self,node,set_id)
  for _,observed in ipairs(groups)do
   if observed.node==node and ok then observed.attached=true end
  end
  current=nil
  if not ok then error(err,0)end
 end
 local containers={}
 local function observe_load(node)
  local ok,err=pcall(SkillsTabClass.Load,tab,node,'independent skill preparation oracle')
  local fields={}
  for _,name in ipairs({'defaultGemLevel','defaultGemQuality','showSupportGemTypes','sortGemsByDPSField','sortGemsByDPS','showLegacyGems'})do fields[name]=tab[name]end
  table.insert(containers,{source=node._source_ordinal,fields=fields,order=copyTable(tab.skillSetOrderList),active_set_id=(ok or tab.activeSkillSetId~=0)and tab.activeSkillSetId or nil})
  if not ok then error(err,0)end
 end
 local ok,err=pcall(function()
  if roots[1].elem=='Skills'then
   observe_load(roots[1])
  else
   for _,node in ipairs(roots[1])do
    if type(node)=='table'and node.elem=='Skills'then observe_load(node)end
   end
  end
  if selection then tab:SetActiveSkillSet(selection)end
 end)
 local result={containers=containers,success=ok,error=not ok and tostring(err)or nil,groups={},selected_groups={},selected_key=tab.activeSkillSetId}
 for _,observed in ipairs(groups)do
  local record={source=observed.source,attached=observed.attached,processing_passes=observed.processing_passes,fields=scalar_fields(observed.group),gems={}}
  for index,gem in ipairs(observed.group.gemList)do
   local node=observed.node[index]
   table.insert(record.gems,{source=node._source_ordinal,fields=scalar_fields(gem),gem_data=gem.gemData and gem.gemData.id,granted_effect=gem.grantedEffect and gem.grantedEffect.id,stat_set=gem.statSet,stat_set_calcs=gem.statSetCalcs,minion_skill_lookup=gem.skillMinionSkillStatSetIndexLookup,minion_skill_lookup_calcs=gem.skillMinionSkillStatSetIndexLookupCalcs})
  end
  table.insert(result.groups,record)
 end
 if ok then
  for _,group in ipairs(tab.socketGroupList)do table.insert(result.selected_groups,assert(seen[group]).source)end
 end
 return result
end
return{load=load}
