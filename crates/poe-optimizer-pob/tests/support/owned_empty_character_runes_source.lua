-- Complete unchanged source observation: character RuneSlot entries are not Items.
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==line,path..":"..line.." actual "..s..":"..tostring(i.linedefined));return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local itemsClass=common.classes.ItemsTab
local dropdown=common.classes.DropDownControl
local calcs=require("Modules.CalcBase")
if emptyCharacterRunesPhase=="before"then
 assert(not launch.promptMsg,"source starts without a pending error")
 local constructor=original(itemsClass.ItemsTab,"Modules/Common.lua",167)
 local implementation=original(upvalue(constructor,"originalFunc"),"Classes/ItemsTab.lua",140)
 local priorItems,priorSets,priorSpecs={},{},{}
 for _,item in pairs(build.itemsTab.items)do priorItems[item]=true end
 for _,set in pairs(build.itemsTab.itemSets)do priorSets[set]=true end
 for _,spec in ipairs(build.treeTab.specList)do priorSpecs[spec]=true end
 _emptyCharacterRunes={priorItems=priorItems,priorSets=priorSets,priorSpecs=priorSpecs,slotDefinitions=upvalue(implementation,"characterRuneSlotList"),methods={
  constructor=constructor,load=original(itemsClass.Load,"Classes/ItemsTab.lua",1193),create=original(itemsClass.CreateItemSet,"Classes/ItemsTab.lua",1571),active=original(itemsClass.SetActiveItemSet,"Classes/ItemsTab.lua",1628),populate=original(itemsClass.PopulateSlots,"Classes/ItemsTab.lua",1705),
  select=original(dropdown.SelByValue,"Classes/DropDownControl.lua",142),selected=original(dropdown.GetSelValue,"Classes/DropDownControl.lua",162),
  config=original(common.classes.ConfigTab.Load,"Classes/ConfigTab.lua",878),configMods=original(common.classes.ConfigTab.BuildModList,"Classes/ConfigTab.lua",1169),
  output=original(common.classes.CalcsTab.BuildOutput,"Classes/CalcsTab.lua",486),env=original(calcs.initEnv,"Modules/CalcSetup.lua",717),nodes=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415),addMod=original(common.classes.ModDB.AddMod,"Classes/ModDB.lua",31),
  spec=original(common.classes.PassiveSpec.Load,"Classes/PassiveSpec.lua",117),tree=original(common.classes.TreeTab.Load,"Classes/TreeTab.lua",492),item=original(common.classes.Item.ParseRaw,"Classes/Item.lua",468),frame=original(launch.OnFrame,"Launch.lua",110),error=original(launch.ShowErrMsg,"Launch.lua",379)
 }}
 if emptyCharacterRunesReuse then
  local tab=build.itemsTab
  local function values()local rows={};for _,definition in ipairs(_emptyCharacterRunes.slotDefinitions)do local name=definition[1];rows[#rows+1]={slot=name,stored=tab.activeItemSet[name]and tab.activeItemSet[name].runeName,selected=dropdown.GetSelValue(tab.runeSlots[name]).name}end;return rows end
  local before=values();local doc,err=common.xml.ParseXML(emptyCharacterRunesXml);assert(doc and not err);local items
  for _,node in ipairs(doc[1])do if type(node)=="table"and node.elem=="Items"then assert(not items);items=node end end;assert(items)
  local returned=itemsClass.Load(tab,items,"owned-reused-character-runes-source")
  _emptyCharacterRunes.reusedLoad={before=before,after=values(),same_items_tab=build.itemsTab==tab,returned=returned}
 end
 return function()
  _emptyCharacterRunes.finished=true
  local m=_emptyCharacterRunes.methods
  _emptyCharacterRunesRecordCleanup({source_prompt=launch.promptMsg,abort_save=build.abortSave or false,methods_preserved=itemsClass.Load==m.load and itemsClass.CreateItemSet==m.create and itemsClass.SetActiveItemSet==m.active and launch.OnFrame==m.frame and launch.ShowErrMsg==m.error})
 end
end
local auth=assert(_emptyCharacterRunes);assert(auth.finished)
local function clone(v,seen)
 if type(v)~="table"then return v end;seen=seen or{};if seen[v]then return seen[v]end
 local out={};seen[v]=out;for k,x in pairs(v)do out[k]=clone(x,seen)end;return out
end
local function equal(a,b,seen)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 seen=seen or{};if seen[a]then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a)do if not equal(v,b[k],seen)then return false end end
 for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function plain(v,depth)
 local t=type(v)
 if t=="function"then local i=debug.getinfo(v,"S");return{source_function={source=i.source:gsub("\\","/"),line=i.linedefined,kind=i.what}}end
 if t~="table"then assert(t~="userdata"and t~="thread");return v end
 depth=(depth or 0)+1;assert(depth<24);local out={};local count,arrayKeys,maximum=0,0,0;local stringKeys=true;local sourceKeys={}
 for k in pairs(v)do
  count=count+1;assert(count<8192);sourceKeys[#sourceKeys+1]=k
  local kind=type(k);assert(kind=="string"or kind=="number"or kind=="boolean","unserializable source key")
  if kind~="string"then stringKeys=false end
  if kind=="number"and k>=1 and k==math.floor(k)then arrayKeys=arrayKeys+1;maximum=math.max(maximum,k)end
 end
 if stringKeys or(arrayKeys==count and maximum==count)then
  for k,x in pairs(v)do out[k]=plain(x,depth)end;return out
 end
 -- ItemSets also carry numeric passive-socket keys. Do not coerce those into
 -- strings or drop them merely to satisfy JSON's object-key restriction.
 table.sort(sourceKeys,function(a,b)if type(a)~=type(b)then return type(a)<type(b)end;if type(a)=="number"then return a<b end;return tostring(a)<tostring(b)end)
 for _,k in ipairs(sourceKeys)do out[#out+1]={key={kind=type(k),value=k},value=plain(v[k],depth)}end
 return{source_table_kind="mixed_or_sparse",entries=out}
end
local function keys(t)local out={};for k in pairs(t or{})do out[#out+1]=k end;table.sort(out);return out end
local function records(list)
 local out={};for _,m in ipairs(list or{})do local tags={};for _,tag in ipairs(m)do tags[#tags+1]=plain(tag)end
 out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,tags=tags}
 end;return out
end
local function runeState(rune)return{name=rune.name,slot=rune.slot,lines=plain(rune.lines),mods=records(rune.mods),is_socket_bound=rune.isSocketBound or false,can_socket_in_character=rune.canSocketInChakraSlots or false}end
local function selections()return{items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}end
local tab=build.itemsTab;local selected=selections()
local spec=build.spec;local oldSpecs,oldItems,oldSets={},{},{}
for i,s in ipairs(build.treeTab.specList)do assert(not auth.priorSpecs[s]);oldSpecs[i]={object=s,jewels=clone(s.jewels),allocNodes=s.allocNodes}end
for id,item in pairs(tab.items)do assert(not auth.priorItems[item]);oldItems[id]={object=item,raw=item.raw,modList=item.modList,slotModList=item.slotModList}end
for id,set in pairs(tab.itemSets)do assert(not auth.priorSets[set]);oldSets[id]={object=set,state=clone(set)}end
local env,calcEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local output,calcOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local oldOutput,oldCalcOutput=clone(output),clone(calcOutput)
local doc,err=common.xml.ParseXML(emptyCharacterRunesXml);assert(doc and not err)
local items;for _,node in ipairs(doc[1])do if type(node)=="table"and node.elem=="Items"then assert(not items);items=node end end;assert(items)
local result={selected=selected,slot_definitions=plain(auth.slotDefinitions),slots={},item_sets={},selected_saved_rune_rows={},consumer={},dropdown_controls={},reused_items_tab_load=plain(auth.reusedLoad),main_output={},calcs_output={}}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.main_output[k]=v end end
for k,v in pairs(calcOutput)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.calcs_output[k]=v end end
for _,node in ipairs(items)do if type(node)=="table"and node.elem=="ItemSet"then
 local id=tonumber(node.attrib.id);local row={id=id,saved_attributes=plain(node.attrib),saved_rune_rows={},loaded_state=plain(tab.itemSets[id])}
 for _,child in ipairs(node)do if type(child)=="table"and(child.elem=="RuneSlot"or child.elem:find("RuneSlot",1,true))then row.saved_rune_rows[#row.saved_rune_rows+1]=plain(child)end end
 if id==selected.items then result.selected_saved_rune_rows=plain(row.saved_rune_rows)end
 result.item_sets[#result.item_sets+1]=row
end end
local selectedMods={};local selectedCount=0;local oldDropdown={}
for _,definition in ipairs(auth.slotDefinitions)do
 local name,kind,label=unpack(definition);local slot=assert(tab.runeSlots[name]);local value=assert(auth.methods.selected(slot));local none
 for _,rune in ipairs(slot.list)do if rune.name=="None"then assert(not none);none=rune end end;assert(none and #none.mods==0)
 oldDropdown[name]={object=slot,index=slot.selIndex,value=value,list=slot.list}
 for _,mod in ipairs(value.mods)do selectedMods[mod]=(selectedMods[mod]or 0)+1;selectedCount=selectedCount+1 end
 result.slots[#result.slots+1]={name=name,kind=kind,label=label,stored_rune_name=tab.activeItemSet[name]and tab.activeItemSet[name].runeName,selected=runeState(value),none=runeState(none),is_item_slot=tab.slots[name]~=nil,is_saved_item_reference=tab.activeItemSet[name]and tab.activeItemSet[name].selItemId~=nil or false,control_visible=slot.shown()and true or false,selected_index=slot.selIndex,none_is_first=slot.list[1]==none}
end
local function consumer(e)
 local matched={};local count=0;local runeSources={};local actual={}
 for _,name in ipairs(keys(e.itemModDB.mods))do for _,mod in ipairs(e.itemModDB.mods[name])do
  if type(mod.source)=="string"and mod.source:sub(1,5)=="Rune:"then runeSources[#runeSources+1]=records({mod})[1]end
  if selectedMods[mod]then actual[mod]=(actual[mod]or 0)+1;count=count+1;matched[#matched+1]=records({mod})[1]end
 end end
 local exact=count==selectedCount;for mod,n in pairs(selectedMods)do if actual[mod]~=n then exact=false end end
 return{final_gate=e.modDB:Flag(nil,"SocketRunesOnCharacter")and true or false,character_rune_modifiers=matched,all_rune_source_modifiers=runeSources,selected_modifier_count=selectedCount,received_selected_modifier_count=count,selected_modifier_identity_matches=exact}
end
result.consumer={main=consumer(env),calcs=consumer(calcEnv)}
-- Direct dropdown probes authenticate the relevant no-match behavior without
-- replacing any business method or changing the saved ItemSet entries.
if emptyCharacterRunesControls then
 for _,definition in ipairs(auth.slotDefinitions)do local name=definition[1];local slot=tab.runeSlots[name];local before=oldDropdown[name]
  for _,input in ipairs({"Owned Unknown Rune","","none","None"})do
   auth.methods.select(slot,"Storm Rune","name");local seed=auth.methods.selected(slot);assert(seed.name=="Storm Rune")
   auth.methods.select(slot,input,"name");local after=auth.methods.selected(slot)
   auth.methods.select(slot,before.value.name,"name")
   result.dropdown_controls[#result.dropdown_controls+1]={slot=name,input=input,seed=runeState(seed),after=runeState(after),restored=slot.selIndex==before.index and auth.methods.selected(slot)==before.value}
  end
 end
end
-- Synthetic two-ItemSet builds exercise the real switch method. Their complete
-- MAIN/CALCS consumers were separately constructed with each saved selection.
if emptyCharacterRunesSwitch then
 local transitions={};local other=selected.items==1 and 2 or 1
 for _,id in ipairs({other,selected.items})do
  auth.methods.active(tab,id,true)
  local values={};for _,definition in ipairs(auth.slotDefinitions)do local name=definition[1];values[#values+1]={slot=name,stored=tab.activeItemSet[name].runeName,selected=auth.methods.selected(tab.runeSlots[name]).name}end
  transitions[#transitions+1]={item_set=tab.activeItemSetId,runes=values}
 end
 result.item_set_switch={transitions=transitions,restored_selections=equal(selections(),selected)}
end
for id,old in pairs(oldItems)do assert(tab.items[id]==old.object and old.object.raw==old.raw and old.object.modList==old.modList and old.object.slotModList==old.slotModList)end
for id,old in pairs(oldSets)do assert(tab.itemSets[id]==old.object and equal(old.object,old.state),"item set changed "..id)end
for index,old in ipairs(oldSpecs)do assert(build.treeTab.specList[index]==old.object and old.object.allocNodes==old.allocNodes and equal(old.object.jewels,old.jewels))end
for name,old in pairs(oldDropdown)do local slot=tab.runeSlots[name];assert(slot==old.object and slot.list==old.list and slot.selIndex==old.index and auth.methods.selected(slot)==old.value)end
assert(equal(consumer(env),result.consumer.main)and equal(consumer(calcEnv),result.consumer.calcs))
assert(build.spec==spec and equal(selections(),selected))
assert(build.calcsTab.mainEnv==env and build.calcsTab.calcsEnv==calcEnv and build.calcsTab.mainOutput==output and build.calcsTab.calcsOutput==calcOutput)
assert(equal(output,oldOutput)and equal(calcOutput,oldCalcOutput))
local m=auth.methods
assert(itemsClass.ItemsTab==m.constructor and itemsClass.Load==m.load and itemsClass.CreateItemSet==m.create and itemsClass.SetActiveItemSet==m.active and itemsClass.PopulateSlots==m.populate)
assert(dropdown.SelByValue==m.select and dropdown.GetSelValue==m.selected and common.classes.ConfigTab.Load==m.config and common.classes.ConfigTab.BuildModList==m.configMods)
assert(common.classes.CalcsTab.BuildOutput==m.output and calcs.initEnv==m.env and calcs.buildModListForNodeList==m.nodes and common.classes.ModDB.AddMod==m.addMod)
assert(common.classes.PassiveSpec.Load==m.spec and common.classes.TreeTab.Load==m.tree and common.classes.Item.ParseRaw==m.item)
assert(launch.OnFrame==m.frame and launch.ShowErrMsg==m.error)
result.saved_items_preserved=true;result.saved_specs_preserved=true;result.saved_item_sets_preserved=true;result.saved_selections_preserved=true;result.main_output_preserved=true;result.calcs_output_preserved=true;result.original_functions_preserved=true;result.fresh_loaded_objects=true
return result
