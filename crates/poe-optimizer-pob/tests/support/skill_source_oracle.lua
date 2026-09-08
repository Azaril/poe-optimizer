-- Test-only UI boundary around unchanged original SkillsTab methods.
-- pre_process captures LoadSkill state and explicitly skips ProcessSocketGroup;
-- processed invokes the complete original ProcessSocketGroup + CalcTools helpers.
local function noop()end
local function dropdown(list)
 return setmetatable({list=list,selIndex=1},{__index=DropDownClass})
end
local function edit()
 return setmetatable({buf='',ResetUndo=noop,changeFunc=function()error('unexpected UI notification')end},{__index=EditClass})
end
local function node(xml)
 local roots,err=originalXml.ParseXML(xml);assert(roots,err)
 if roots[1].elem=='Skills'then return roots[1]end
 for _,child in ipairs(roots[1])do if type(child)=='table'and child.elem=='Skills'then return child end end
 return{elem='Skills',attrib={}}
end
local function load(xml,processed,selectedData)
 local build={data=selectedData or data,characterLevel=80,SyncLoadouts=noop}
 local tab=setmetatable({build=build,controls={},observed_before_process={},ResetUndo=noop},{__index=SkillsTabClass})
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
 local seen={}

 tab.ProcessSocketGroup=function(self,group)
  -- LoadSkill has not attached gemData/effect cycles at this boundary.
  if not seen[group]then table.insert(self.observed_before_process,copyTable(group));seen[group]=true end
  if processed then SkillsTabClass.ProcessSocketGroup(self,group)end
 end
 tab:Load(node(xml),'independent source oracle')
 return tab
end
local function save(tab)
 local root={elem='Skills',attrib={}};tab:Save(root)
 local xml,err=originalXml.ComposeXML(root);assert(xml,err)
 return root,xml
end
return{node=node,load=load,save=save}
