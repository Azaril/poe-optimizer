-- Complete original-source observation. No game method is replaced.
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line,path..":"..line.." actual "..s..":"..tostring(i.linedefined));return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local passive=common.classes.PassiveSpec
local slotClass=common.classes.ItemSlotControl
local calcs=require("Modules.CalcBase")
if jewelPlacementPhase=="before"then
 local constructor=original(passive.PassiveSpec,"Modules/Common.lua",167)
 original(upvalue(constructor,"originalFunc"),"Classes/PassiveSpec.lua",33)
 local priorItems,priorSpecs={},{}
 for _,item in pairs(build.itemsTab.items)do priorItems[item]=true end
 for _,spec in ipairs(build.treeTab.specList)do priorSpecs[spec]=true end
 _jewelPlacement={priorItems=priorItems,priorSpecs=priorSpecs,methods={
  constructor=constructor,init=original(passive.Init,"Classes/PassiveSpec.lua",45),load=original(passive.Load,"Classes/PassiveSpec.lua",117),post=original(passive.PostLoad,"Classes/PassiveSpec.lua",353),import=original(passive.ImportFromNodeList,"Classes/PassiveSpec.lua",358),
  treeLoad=original(build.treeTab.Load,"Classes/TreeTab.lua",492),active=original(build.treeTab.SetActiveSpec,"Classes/TreeTab.lua",540),
  itemsLoad=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193),valid=original(build.itemsTab.IsItemValidForSlot,"Classes/ItemsTab.lua",2603),update=original(build.itemsTab.UpdateSockets,"Classes/ItemsTab.lua",1712),populate=original(build.itemsTab.PopulateSlots,"Classes/ItemsTab.lua",1705),
  slotPopulate=original(slotClass.Populate,"Classes/ItemSlotControl.lua",117),slotSet=original(slotClass.SetSelItemId,"Classes/ItemSlotControl.lua",103),
  itemActive=original(common.classes.Item.GetActiveModListForSlotNum,"Classes/Item.lua",2198),itemBuild=original(common.classes.Item.BuildModList,"Classes/Item.lua",2694),itemParse=original(common.classes.Item.ParseRaw,"Classes/Item.lua",468),
  mainTree=original(main.LoadTree,"Modules/Main.lua",347),output=original(build.calcsTab.BuildOutput,"Classes/CalcsTab.lua",486),
  env=original(calcs.initEnv,"Modules/CalcSetup.lua",717),node=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200),nodes=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)
 }}
 return function()_jewelPlacement.finished=true end
end
local auth=assert(_jewelPlacement);assert(auth.finished)
local function clone(v,seen)
 if type(v)~="table"then return v end;seen=seen or {};if seen[v]then return seen[v]end
 local out={};seen[v]=out;for k,x in pairs(v)do out[k]=clone(x,seen)end;return out
end
local function equal(a,b,seen)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 seen=seen or {};if seen[a]then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a)do if not equal(v,b[k],seen)then return false end end
 for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function plain(v,depth)
 local t=type(v)
 if t=="function"then local i=debug.getinfo(v,"S");return {source_function={source=i.source:gsub("\\","/"),line=i.linedefined,kind=i.what}}end
 if t~="table"then assert(t~="userdata" and t~="thread");return v end
 depth=(depth or 0)+1;assert(depth<24);local out={};local count=0
 for k,x in pairs(v)do count=count+1;assert(count<8192);out[k]=plain(x,depth)end;return out
end
local function keys(t)local out={};for k in pairs(t or {})do out[#out+1]=k end;table.sort(out);return out end
local function records(list)
 local out={};for _,m in ipairs(list or {})do local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end
  out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,source_slot_num=m.sourceSlotNum,tags=tags}
 end;return out
end
local function delivery(env,item)
 local out={};if not item then return out end
 for _,name in ipairs(keys(env.itemModDB.mods))do for _,mod in ipairs(env.itemModDB.mods[name])do
  if mod.source==item.modSource then out[#out+1]=records({mod})[1] end
 end end;return out
end
local function nodeFacts(spec,id)
 local node=spec.nodes[id];local raw=spec.tree.nodes[id]
 local out={id=id,exists=node~=nil,raw_exists=raw~=nil,allocated=spec.allocNodes[id]~=nil}
 if node then
  out.same_allocation_object=spec.allocNodes[id]==node;out.alloc_mode=node.allocMode;out.type=node.type;out.name=node.name
  out.is_jewel_socket=node.isJewelSocket or false;out.contain_jewel_socket=node.containJewelSocket or false;out.sinister=node.sinister or false;out.charm_socket=node.charmSocket or false
  out.expansion_jewel=plain(node.expansionJewel);out.ascendancy_name=node.ascendancyName
 end;return out
end
local function mapRows(map)
 local out={};for _,nodeId in ipairs(keys(map))do out[#out+1]={node=nodeId,item=map[nodeId]}end;return out
end
local function specState(spec)
 local out={version=spec.treeVersion,jewels=mapRows(spec.jewels),allocations={}}
 for _,nodeId in ipairs(keys(spec.allocNodes))do out.allocations[#out.allocations+1]={node=nodeId,mode=spec.allocNodes[nodeId].allocMode}end
 return out
end
local doc,err=common.xml.ParseXML(jewelPlacementXml);assert(doc and not err)
local tree,itemsNode;for _,n in ipairs(doc[1])do if type(n)=="table"then if n.elem=="Tree"then tree=n elseif n.elem=="Items"then itemsNode=n end end end;assert(tree and itemsNode)
local savedSpecs={};for _,n in ipairs(tree)do if type(n)=="table"and n.elem=="Spec"then savedSpecs[#savedSpecs+1]=n end end
local rawItems={};for _,n in ipairs(itemsNode)do if type(n)=="table"and n.elem=="Item"then local text={};for _,v in ipairs(n)do if type(v)=="string"then text[#text+1]=v end end;rawItems[tonumber(n.attrib.id)]=table.concat(text,"\n")end end
local selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local spec=build.spec;assert(spec==build.treeTab.specList[selected.spec]);assert(spec.tree==main.tree[spec.treeVersion])
local env,calcEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local output,calcOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local oldOutput,oldCalcOutput=clone(output),clone(calcOutput)
local oldItems,oldSpecs={},{}
for id,item in pairs(build.itemsTab.items)do assert(not auth.priorItems[item]);oldItems[id]={object=item,raw=item.raw,modList=item.modList,slotModList=item.slotModList}end
for i,s in ipairs(build.treeTab.specList)do assert(not auth.priorSpecs[s]);oldSpecs[i]={object=s,state=specState(s)}end
local result={selected=selected,tree_version=spec.treeVersion,active_weapon_set=build.itemsTab.activeItemSet.useSecondWeaponSet and 2 or 1,assignments={},specs={},eligibility_matrix={},ordinary_nodes={},isolated_load_controls={},isolated_character_load_controls={},main_output={},calcs_output={}}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.main_output[k]=v end end
for k,v in pairs(calcOutput)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.calcs_output[k]=v end end
local ordinary={2491,7960,21984,26196,26725,32763,46882,54127,55190,60735,61419,61834}
for _,id in ipairs(ordinary)do result.ordinary_nodes[#result.ordinary_nodes+1]=nodeFacts(spec,id)end
for index,saved in ipairs(savedSpecs)do
 local actual=build.treeTab.specList[index]
 local summary={index=index,saved_attributes=plain(saved.attrib),loaded=actual and specState(actual),assignments={}}
 for _,container in ipairs(saved)do if type(container)=="table"and container.elem=="Sockets"then
  for _,socket in ipairs(container)do if type(socket)=="table"then
   summary.assignments[#summary.assignments+1]=plain(socket)
   if index==selected.spec and socket.elem=="Socket"then
    local nodeId,itemId=tonumber(socket.attrib.nodeId),tonumber(socket.attrib.itemId)
    local item=itemId and build.itemsTab.items[itemId]
    local slot=nodeId and build.itemsTab.sockets[nodeId]
    local slotName=nodeId and "Jewel "..nodeId
    local row={saved=plain(socket),node=nodeId,item_id=itemId,item_exists=item~=nil,spec_assignment=nodeId and spec.jewels[nodeId],node_facts=nodeId and nodeFacts(spec,nodeId),slot_exists=slot~=nil}
    if slot then row.slot={name=slot.slotName,node=slot.nodeId,selected_item=slot.selItemId,slot_num=slot.slotNum,weapon_set=slot.weaponSet,inactive=slot.inactive}end
    if item then
     row.item={id=item.id,raw=rawItems[itemId],loaded_raw=item.raw,name=item.name,base=item.baseName,type=item.type,rarity=item.rarity,sub_type=item.base.subType,cluster_jewel=plain(item.clusterJewel),source=item.modSource,radius_index=item.jewelRadiusIndex,limit=item.limit}
     row.valid=slotName and auth.methods.valid(build.itemsTab,item,slotName)or false
     if slot then row.active_modifiers=records(item.modList or item.slotModList and item.slotModList[slot.slotNum]);row.active_bonded_state=item.activeBondedState end
     row.receiving={main=slotName and env.player.itemList[slotName]==item or false,calcs=slotName and calcEnv.player.itemList[slotName]==item or false,main_allocated=nodeId and env.allocNodes[nodeId]~=nil or false,calcs_allocated=nodeId and calcEnv.allocNodes[nodeId]~=nil or false}
     row.merged_modifiers={main=delivery(env,item),calcs=delivery(calcEnv,item)}
     for _,candidate in ipairs(ordinary)do result.eligibility_matrix[#result.eligibility_matrix+1]={item=itemId,base=item.baseName,rarity=item.rarity,sub_type=item.base.subType,node=candidate,valid=auth.methods.valid(build.itemsTab,item,"Jewel "..candidate)}end
    end
    result.assignments[#result.assignments+1]=row
   end
  end end
 end end
 result.specs[#result.specs+1]=summary
end
-- Fresh isolated Spec loads use original methods and the actual loaded item
-- collection. They never become the selected Spec or enter the live Spec list.
if jewelPlacementControls then
 local saved=assert(savedSpecs[selected.spec])
 local function modified(kind)
  local xml=clone(saved)
  for i=#xml,1,-1 do if type(xml[i])=="table"and xml[i].elem=="Sockets"then table.remove(xml,i)end end
  if kind~="absent"then
   local sockets={elem="Sockets",attrib={}}
   if kind~="empty"then sockets[1]={elem="Socket",attrib={nodeId="46882",itemId=kind=="zero"and"0"or kind=="missing"and"999999"or"1"}}end
   xml[#xml+1]=sockets
  end
  return xml
 end
 local function fresh(xml)local s=new("PassiveSpec"):PassiveSpec(build,"0_5");local returned=auth.methods.load(s,xml,"owned-isolated-passive-source");return s,returned end
 for _,kind in ipairs({"absent","empty","zero","missing"})do
  local reused=fresh(modified("seed"));local before=specState(reused)
  local returned=auth.methods.load(reused,modified(kind),"owned-reused-passive-source")
  local isolated,freshReturned=fresh(modified(kind))
  result.isolated_load_controls[#result.isolated_load_controls+1]={name=kind,reused_before=before,reused_after=specState(reused),fresh=specState(isolated),returned=returned,fresh_returned=freshReturned,separate_objects=reused~=isolated and reused~=spec and isolated~=spec}
 end
 for _,kind in ipairs({"missing-class","invalid-class","missing-ascendancy","invalid-ascendancy"})do
  local xml=clone(saved)
  local isClass=kind:find("class",1,true)~=nil
  local legacy=isClass and "classId"or"ascendClassId"
  local internal=isClass and "classInternalId"or"ascendancyInternalId"
  xml.attrib[legacy]=kind:find("invalid",1,true)and"-1"or nil
  xml.attrib[internal]=kind:find("invalid",1,true)and(isClass and"999999"or"owned_unknown")or nil
  local isolated=new("PassiveSpec"):PassiveSpec(build,"0_5")
  local succeeded,returned=pcall(auth.methods.load,isolated,xml,"owned-character-passive-source")
  result.isolated_character_load_controls[#result.isolated_character_load_controls+1]={name=kind,saved_attributes=plain(xml.attrib),call_succeeded=succeeded,returned=returned,state=specState(isolated),node=nodeFacts(isolated,46882),saved_assignment=isolated.jewels[46882],separate_object=isolated~=spec}
 end
end
for id,saved in pairs(oldItems)do assert(build.itemsTab.items[id]==saved.object and saved.object.raw==saved.raw and saved.object.modList==saved.modList and saved.object.slotModList==saved.slotModList)end
for _,row in ipairs(result.assignments)do if row.item_exists then local item=build.itemsTab.items[row.item_id]
 assert(equal(row.merged_modifiers.main,delivery(env,item))and equal(row.merged_modifiers.calcs,delivery(calcEnv,item)))
 if row.slot then assert(equal(row.active_modifiers,records(item.modList or item.slotModList and item.slotModList[row.slot.slot_num])))end
end end
for i,saved in ipairs(oldSpecs)do assert(build.treeTab.specList[i]==saved.object and equal(specState(saved.object),saved.state))end
assert(build.spec==spec and env.spec==spec and build.treeTab.activeSpec==selected.spec and build.itemsTab.activeItemSetId==selected.items and build.skillsTab.activeSkillSetId==selected.skills and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
assert(build.calcsTab.mainEnv==env and build.calcsTab.calcsEnv==calcEnv and build.calcsTab.mainOutput==output and build.calcsTab.calcsOutput==calcOutput)
assert(equal(output,oldOutput)and equal(calcOutput,oldCalcOutput))
local m=auth.methods
assert(passive.PassiveSpec==m.constructor and passive.Init==m.init and passive.Load==m.load and passive.PostLoad==m.post and passive.ImportFromNodeList==m.import)
assert(build.treeTab.Load==m.treeLoad and build.treeTab.SetActiveSpec==m.active and build.itemsTab.Load==m.itemsLoad and build.itemsTab.IsItemValidForSlot==m.valid and build.itemsTab.UpdateSockets==m.update and build.itemsTab.PopulateSlots==m.populate)
assert(slotClass.Populate==m.slotPopulate and slotClass.SetSelItemId==m.slotSet and common.classes.Item.GetActiveModListForSlotNum==m.itemActive and common.classes.Item.BuildModList==m.itemBuild and common.classes.Item.ParseRaw==m.itemParse)
assert(main.LoadTree==m.mainTree and build.calcsTab.BuildOutput==m.output and calcs.initEnv==m.env and calcs.buildModListForNode==m.node and calcs.buildModListForNodeList==m.nodes)
result.saved_items_preserved=true;result.saved_specs_preserved=true;result.saved_selections_preserved=true;result.main_output_preserved=true;result.calcs_output_preserved=true;result.original_functions_preserved=true;result.fresh_loaded_objects=true
return result
