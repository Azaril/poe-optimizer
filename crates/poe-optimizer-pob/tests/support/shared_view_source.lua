-- Independent selector observations; source bodies remain unmodified.
-- Item ParseRaw always creates a synthetic base, and BuildModList is inert.
-- Skill processing, tree allocation, config defaults/modifiers and UI are inert.
-- These boundaries prove saved SET selection only, never inventory preparation.
local function noop()end
local function control()
 return {SelByValue=function(self,value,key)self.value=value end,
 GetSelValueByKey=function(self,key)return self.value end,SetText=noop}
end
local function list(n)local out={};for i=1,n do out[i]={}end;return out end
local messages
launch={ShowErrMsg=function(_,fmt,...)messages[#messages+1]=string.format(fmt,...)end}
main={OpenMessagePopup=function(_,title,message)messages[#messages+1]=title..':'..message end}
ConPrintf=noop
copyTable=function(t)return t end
data={powerStatList={},setJewelRadiiGlobally=noop}
new=function(kind)
 if kind=='Item'then return{Item=function(self,raw)
  self.jewelSocketCount=0
  self.buffModLines={};self.enchantModLines={};self.implicitModLines={};self.explicitModLines={}
  self.ParseRaw=function(item,text)
   item.base={};item.title='synthetic parse boundary';item.raw=text
   item.buffModLines=list(2);item.enchantModLines=list(3);item.implicitModLines=list(5);item.explicitModLines=list(512)
  end
  self.BuildModList=noop
  return self
 end}end
 assert(kind=='PassiveSpec')
 return{PassiveSpec=function(_,build,version)
  return setmetatable({build=build,treeVersion=version,jewels={},nodeNotes={},hashOverrides={},
   tree={classIntegerIdMap={},internalAscendNameMap={}},
   ResetUndo=noop,SetWindowTitleWithBuildClass=noop,ImportFromNodeList=noop,DecodeURL=noop,SwitchAttributeNode=noop},
   {__index=PassiveSpecClass})
 end}
end
local function create()
 local b={data={gemsByGameId={},skills={},gemForSkill={}},SyncLoadouts=noop,UpdateClassDropdowns=noop}
 local s=setmetatable({build=b,controls={defaultLevel=control(),defaultQuality=control(),sortGemsByDPS={},showLegacyGems={},showSupportGemTypes=control(),sortGemsByDPSFieldControl=control(),groupList={}},ProcessSocketGroup=noop,SetDisplayGroup=noop,ResetUndo=noop},{__index=SkillsTabClass})
 local i=setmetatable({build=b,items={},itemOrderList={},slots={},runeSlots={},controls={specSelect={}},tradeQuery={},PopulateSlots=noop,ResetUndo=noop},{__index=ItemsTabClass})
 b.itemsTab=i
 local c=setmetatable({build=b,defaultState={},UpdateControls=noop,BuildModList=noop,ResetUndo=noop},{__index=ConfigTabClass})
 local t=setmetatable({build=b,controls={versionSelect=control()}},{__index=TreeTabClass})
 return{Skills=s,Items=i,Config=c,Tree=t}
end
local fields={Skills={'activeSkillSetId','skillSetOrderList','skillSets'},Items={'activeItemSetId','itemSetOrderList','itemSets'},Config={'activeConfigSetId','configSetOrderList','configSets'},Tree={'activeSpec',nil,'specList'}}
local function snapshot(tab,kind,ok,err,node)
 local f=fields[kind];local key=tab[f[1]];local records=tab[f[3]]
 local selected=records and key and records[key]
 local out={ok=ok,error=not ok and tostring(err)or nil,key=key,order={},selected_title=selected and selected.title,
  selected_use_second_weapon_set=selected and selected.useSecondWeaponSet,source_container_ordinal=node._source_ordinal,sets={},registered_keys={}}
 for id in pairs(records or{})do out.registered_keys[#out.registered_keys+1]=id end
 table.sort(out.registered_keys)
 if f[2]then for _,id in ipairs(tab[f[2]]or{})do out.order[#out.order+1]=id end
 else for index in ipairs(records or{})do out.order[#out.order+1]=index end end
 for _,id in ipairs(out.order)do local set=records[id];out.sets[#out.sets+1]={key=id,title=set and set.title}end
 if kind=='Skills'then out.groups=selected and #selected.socketGroupList or nil end
 return out
end
local function observe(text)
 messages={}
 local roots,err=originalXml.ParseXML(text);assert(roots,err);local root=roots[1]
 local ordinal=-1
 local function mark(node)
  ordinal=ordinal+1;node._source_ordinal=ordinal
  for _,child in ipairs(node)do if type(child)=='table'then mark(child)end end
 end
 mark(root)
 local tabs=create();local out={scope='original independent saved set selectors with explicit inert preparation boundaries'}
 -- Each source container is loaded in its own domain in XML order. This is not
 -- Build.LoadDB ordering, cross-tab post-load behavior or loadout synchronization.
 for _,node in ipairs(root)do
  local kind=type(node)=='table'and(node.elem=='Spec'and'Tree'or node.elem)
  if tabs[kind]then
   local ok,result=pcall(tabs[kind].Load,tabs[kind],node,'shared-view-source.xml')
   out[kind]=snapshot(tabs[kind],kind,ok,result,node)
  end
 end
 out.messages=messages
 return out
end
return{observe=observe}
