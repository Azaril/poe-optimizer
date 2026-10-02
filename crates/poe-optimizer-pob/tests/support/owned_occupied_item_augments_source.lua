-- Complete pinned-source observations. Probes mutate only freshly allocated Items.
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==line,path..":"..line.." actual "..s..":"..i.linedefined);return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item;local calcs=require("Modules.CalcBase")
if occupiedAugmentPhase=="before"then
 local constructor=original(class.Item,"Modules/Common.lua",167);original(upvalue(constructor,"originalFunc"),"Classes/Item.lua",93)
 local m={Item=constructor,ParseRaw=original(class.ParseRaw,"Classes/Item.lua",468),UpdateRunes=original(class.UpdateRunes,"Classes/Item.lua",2106),ApplySocketedRuneDisplayScalars=original(class.ApplySocketedRuneDisplayScalars,"Classes/Item.lua",2180),GetSocketedAugmentTypes=original(class.GetSocketedAugmentTypes,"Classes/Item.lua",2338),BuildModList=original(class.BuildModList,"Classes/Item.lua",2694),GetActiveModListForSlotNum=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198),BuildModListsForSlots=original(class.BuildModListsForSlots,"Classes/Item.lua",2671),BuildModListForSlotNum=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414),CheckModLineVariant=original(class.CheckModLineVariant,"Classes/Item.lua",2289),GetModLineVariantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320)}
 local prior={};for _,item in pairs(build.itemsTab.items)do prior[item]=true end
 _occupiedAugmentAuth={methods=m,prior=prior,load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193),parser=original(modLib.parseMod,"Modules/ModParser.lua",7404),init=original(calcs.initEnv,"Modules/CalcSetup.lua",717),nodes=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415),scale=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82),setSource=original(modLib.setSource,"Modules/ModTools.lua",277),output=original(common.classes.CalcsTab.BuildOutput,"Classes/CalcsTab.lua",486)}
 return function()_occupiedAugmentAuth.finished=true end
end
local auth=assert(_occupiedAugmentAuth);assert(auth.finished)
local function clone(v,seen)
 if type(v)~="table"then return v end;seen=seen or{};if seen[v]then return seen[v]end
 local o={};seen[v]=o;for k,x in pairs(v)do o[k]=clone(x,seen)end;return o
end
local function equal(a,b,seen)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 seen=seen or{};if seen[a]then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a)do if not equal(v,b[k],seen)then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function plain(v,depth)
 local t=type(v)
 if t=="function"then local i=debug.getinfo(v,"S");return{source_function={source=i.source:gsub("\\","/"),line=i.linedefined,kind=i.what}}end
 if t~="table"then assert(t~="userdata"and t~="thread");return v end
 depth=(depth or 0)+1;assert(depth<24);local o,ks={},{};local stringKeys=true;local n,m=0,0
 for k in pairs(v)do ks[#ks+1]=k;assert(#ks<8192);local kind=type(k);assert(kind=="string"or kind=="number"or kind=="boolean");if kind~="string"then stringKeys=false end;if kind=="number"and k>=1 and k==math.floor(k)then n=n+1;m=math.max(m,k)end end
 if stringKeys or(n==#ks and m==#ks)then for k,x in pairs(v)do o[k]=plain(x,depth)end;return o end
 table.sort(ks,function(a,b)if type(a)~=type(b)then return type(a)<type(b)end;if type(a)=="number"then return a<b end;return tostring(a)<tostring(b)end)
 for _,k in ipairs(ks)do o[#o+1]={key={kind=type(k),value=k},value=plain(v[k],depth)}end;return{source_table_kind="mixed_or_sparse",entries=o}
end
local function keys(t)local o={};for k in pairs(t or{})do o[#o+1]=k end;table.sort(o);return o end
local function records(list)
 local o={};for _,m in ipairs(list or{})do local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;o[#o+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot_num=m.sourceSlotNum,tags=tags}end;return o
end
local function lines(list)
 local o={};for i,l in ipairs(list or{})do local r={ordinal=i,line=l.line,mods=records(l.modList),bonded_mods=records(l.bondedModList)}
  for _,k in ipairs({"extra","order","rune","enchant","disabled","bonded","augmentType","socketedRuneEffectAlreadyApplied","displayValueScalar","valueScalar","variantList"})do r[k]=plain(l[k])end;o[#o+1]=r
 end;return o
end
local categories={"buff","enchant","rune","classRequirement","implicit","explicit"}
local function snapshot(item)
 local broad,specific=auth.methods.GetSocketedAugmentTypes(item)
 local o={id=item.id,name=item.name,base=item.baseName,type=item.type,raw=item.raw,raw_lines=plain(item.rawLines),runes=plain(item.runes),sockets=plain(item.sockets),socket_count=item.itemSocketCount,jewel_count=item.jewelSocketCount,requirements=plain(item.requirements),broad=broad,specific=specific,base_socket_limit=item.base.socketLimit,base_tags=plain(item.base.tags),base_subtype=item.base.subType,base_weapon=plain(item.base.weapon),base_armour=plain(item.base.armour),lists={},base_mods=records(item.baseModList),mod_list=records(item.modList),slot_mod_lists={},weapon_data=plain(item.weaponData),armour_data=plain(item.armourData),unknown_headers={}}
 for _,category in ipairs(categories)do o.lists[category]=lines(item[category.."ModLines"])end
 for i,list in pairs(item.slotModList or{})do o.slot_mod_lists[tostring(i)]=records(list)end
 for i,name in ipairs(item.runes)do if name~="None"and not data.itemMods.Runes[name]then o.unknown_headers[#o.unknown_headers+1]={ordinal=i,name=name}end end
 for _,k in ipairs({"activeBondedState","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier","socketedAugmentTypeOverride","socketedSoulCoreTypes"})do o[k]=plain(item[k])end
 return o
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;auth.methods.ParseRaw(item,raw);auth.methods.BuildModList(item);return item end
local function branches(item)
 local states={}
 for _,enabled in ipairs({false,true,false})do local active=auth.methods.GetActiveModListForSlotNum(item,1,enabled);states[#states+1]={global_bonded=enabled,active_state=item.activeBondedState,active=records(active),weapon_data=plain(item.weaponData),armour_data=plain(item.armourData)}end
 assert(equal(states[1],states[3]),"Bonded cache did not restore")
 return states
end
local function replace(text,old,new)local i=assert(text:find(old,1,true),old);return text:sub(1,i-1)..new..text:sub(i+#old)end
local function selected()return{items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}end
local selection=selected();local tab=build.itemsTab;local spec=build.spec
local env,calcEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
local output,calcOutput=build.calcsTab.mainOutput,build.calcsTab.calcsOutput
local oldOutput,oldCalcOutput=clone(output),clone(calcOutput)
local savedItems,savedSets,savedSpecs={},{},{}
for id,item in pairs(tab.items)do assert(not auth.prior[item]);savedItems[id]={object=item,raw=item.raw,base=item.baseModList,mods=item.modList,slots=item.slotModList,state=snapshot(item)}end
for id,set in pairs(tab.itemSets)do savedSets[id]={object=set,state=clone(set)}end
local function allocations(s)
 local out={};for id,node in pairs(s.allocNodes)do out[id]={object=node,id=node.id,alloc=node.alloc,mode=node.allocMode}end;return out
end
for i,s in ipairs(build.treeTab.specList)do savedSpecs[i]={object=s,nodes=s.allocNodes,allocations=allocations(s),jewels=clone(s.jewels)}end
local savedItemKeys,savedSetKeys,savedSpecKeys=keys(tab.items),keys(tab.itemSets),keys(build.treeTab.specList)
local doc,err=common.xml.ParseXML(occupiedAugmentXml);assert(doc and not err)
local items;for _,n in ipairs(doc[1])do if type(n)=="table"and n.elem=="Items"then assert(not items);items=n end end;assert(items)
local raws,xmlItems,selectedSet={},{},nil
for _,n in ipairs(items)do if type(n)=="table"then
 if n.elem=="Item"then local text={};for _,s in ipairs(n)do if type(s)=="string"then text[#text+1]=s end end;local id=assert(tonumber(n.attrib.id));assert(not raws[id]);raws[id]=table.concat(text,"\n");xmlItems[id]=plain(n)
 elseif n.elem=="ItemSet"and tonumber(n.attrib.id)==selection.items then assert(not selectedSet);selectedSet=n end
end end;assert(selectedSet)
local function consumer(e,item)
 local slots={};for slot,object in pairs(e.player.itemList)do if object==item then slots[#slots+1]=slot end end;table.sort(slots)
 local delivered={};for _,name in ipairs(keys(e.itemModDB.mods))do for _,m in ipairs(e.itemModDB.mods[name])do if m.source==item.modSource then delivered[#delivered+1]=records({m})[1]end end end
 local identities={}
 for _,slotName in ipairs(slots)do
  local slot=assert(tab.slots[slotName]);local active=item.modList or(item.slotModList and item.slotModList[slot.slotNum])or{}
  local identity=0;for _,m in ipairs(active)do for _,actual in ipairs(e.itemModDB.mods[m.name]or{})do if actual==m then identity=identity+1 end end end
  identities[#identities+1]={slot=slotName,slot_num=slot.slotNum,active_count=#active,identity_count=identity}
 end
 return{actual_item_slots=slots,delivered=delivered,receiving_identity_diagnostics=identities,multipliers=plain(e.itemModDB.multipliers)}
end
local result={selected=selection,selected_saved_item_set=plain(selectedSet),hosts={},main_output={},calcs_output={},probes={},family_probes={}}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.main_output[k]=v end end
for k,v in pairs(calcOutput)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.calcs_output[k]=v end end
for _,slot in ipairs(selectedSet)do if type(slot)=="table"and slot.elem=="Slot"then
 local id=tonumber(slot.attrib.itemId);local raw=raws[id];local occupied=false
 if raw then for line in raw:gmatch("[^\n]+")do local name=line:match("^%s*Rune: (.-)%s*$");if name and name~="None"then occupied=true end end end
 -- Header-free inputs can acquire occupied selections through ParseRaw inference.
 -- Discover those from the actual loaded item instead of omitting the control.
 if raw and tab.items[id] then for _,name in ipairs(tab.items[id].runes or{})do if name~="None"then occupied=true end end end
 if occupied then
  local item=assert(tab.items[id]);local sourceSlot=assert(tab.slots[slot.attrib.name]);assert(sourceSlot.selItemId==id)
  local fresh=construct(raw,id);local freshState=snapshot(fresh);local firstBranches=branches(fresh)
  local rebuilt=construct(raw,id);assert(equal(freshState,snapshot(rebuilt)),"fresh item differs")
  assert(equal(lines(item.runeModLines),lines(rebuilt.runeModLines)),"loaded/reconstructed rune lines differ")
  local actual={main=consumer(env,item),calcs=consumer(calcEnv,item)}
  result.hosts[#result.hosts+1]={id=id,slot=slot.attrib.name,saved_slot=plain(slot),saved_item=xmlItems[id],raw=raw,loaded=snapshot(item),fresh=freshState,branches=firstBranches,delivery=actual,fresh_reconstruction_equal=true}
 end
end end
-- The complete original-03 body armour supplies real duplicate Rune and Bonded
-- descriptions. No synthetic catalog entries or parser substitutions are used.
if occupiedAugmentControls then
 local id=14;local raw=assert(raws[id]);assert(tab.items[id].baseName=="Sleek Jacket")
 local normal="{enchant}{rune}36% increased Armour, Evasion and Energy Shield"
 local bonded="{enchant}{rune}Bonded: +40 to maximum Life"
 local both="Rune: Greater Iron Rune\nRune: Greater Iron Rune"
 local cases={
  {"original",raw},{"changed-saved-value",replace(raw,normal,"{enchant}{rune}99% increased Armour, Evasion and Energy Shield")},
  {"removed-saved-normal",replace(raw,normal.."\n","")},
  {"disabled-normal",replace(raw,normal,"{disabled}"..normal)},
  {"disabled-normal-other-value",replace(raw,normal,"{disabled}{enchant}{rune}99% increased Armour, Evasion and Energy Shield")},
  {"disabled-bonded",replace(raw,bonded,"{disabled}"..bonded)},
  {"duplicate-disabled-normal",replace(raw,normal,"{disabled}"..normal.."\n{disabled}"..normal)},
  {"known-surplus",replace(raw,both,both.."\nRune: Greater Iron Rune")},
  {"unknown-surplus",replace(replace(raw,both,both.."\nRune: Owned Unknown Rune"),normal,"{enchant}{rune}99% increased Armour, Evasion and Energy Shield")},
  {"unknown-active",replace(replace(raw,both,"Rune: Owned Unknown Rune\nRune: Greater Iron Rune"),normal,"{enchant}{rune}99% increased Armour, Evasion and Energy Shield")},
  {"missing-all-headers",replace(raw,both.."\n","")},
  {"missing-one-header",replace(raw,both,"Rune: Greater Iron Rune")},
  {"empty-header",replace(raw,both,"Rune: \nRune: Greater Iron Rune")},
  {"lowercase-header",replace(raw,both,"rune: Greater Iron Rune\nrune: Greater Iron Rune")},
  {"empty-selection",replace(raw,both,"Rune: None\nRune: None")},
  {"duplicate-socket-header",replace(raw,"Sockets: S S","Sockets: S\nSockets: S")},
  {"socket-markers",replace(raw,"Sockets: S S","Sockets: S s J X S")},
  {"missing-sockets",replace(raw,"Sockets: S S\n","")},
  {"extra-effect-thirteen",raw.."\n13% increased Effect of Socketed Runes"},
  {"disabled-extra-effect-thirteen",replace(raw,normal,"{disabled}"..normal).."\n13% increased Effect of Socketed Runes"},
  {"single-extra-effect-thirteen",replace(replace(raw,both,"Rune: Greater Iron Rune"),"Sockets: S S","Sockets: S").."\n13% increased Effect of Socketed Runes"},
  {"weapon-double-extra-effect-twenty-five",assert(raws[17]).."\n25% increased Effect of Socketed Runes",17},
  {"weapon-single-extra-effect-twenty-five",replace(replace(assert(raws[17]),both,"Rune: Greater Iron Rune"),"Sockets: S S","Sockets: S").."\n25% increased Effect of Socketed Runes",17},
  {"weapon-no-runes",replace(assert(raws[17]),both,"Rune: None\nRune: None"),17}
 }
 for _,case in ipairs(cases)do
  local ok,value=pcall(function()local item=construct(case[2],case[3]or id);return{name=case[1],raw=case[2],available=true,state=snapshot(item),branches=branches(item)}end)
  if not ok then value={name=case[1],raw=case[2],available=false,error=tostring(value)}end
  result.probes[#result.probes+1]=value
 end
 local reused=construct(cases[19][2],id);local before=snapshot(reused);auth.methods.ParseRaw(reused,raw);auth.methods.BuildModList(reused)
 local fresh=construct(raw,id);result.reused_item={before=before,after=snapshot(reused),fresh=snapshot(fresh),rune_state_equal=equal(lines(reused.runeModLines),lines(fresh.runeModLines)),headers_equal=equal(reused.runes,fresh.runes)}
end
if occupiedAugmentFamilyControls then
 local idol=assert(raws[11]);local mixed=assert(raws[13]);assert(tab.items[11].baseName=="Dastard Armour")
 local cases={
  {"idol-removed",11,replace(idol,"Rune: Fox Idol","Rune: None")},
  {"idol-rune-effect25",11,idol.."\n25% increased Effect of Socketed Runes"},
  {"idol-all-effect25",11,idol.."\n25% increased Effect of Socketed Augment Items"},
  {"mixed-soulcore-effect25",13,mixed.."\n25% increased Effect of Socketed Soul Cores"},
  {"mixed-rune-effect25",13,mixed.."\n25% increased Effect of Socketed Runes"},
  {"mixed-all-and-family-effect25",13,mixed.."\n25% increased Effect of Socketed Augment Items\n25% increased Effect of Socketed Runes\n25% increased Effect of Socketed Soul Cores"}
 }
 for _,case in ipairs(cases)do
  local ok,value=pcall(function()local item=construct(case[3],case[2]);return{name=case[1],id=case[2],raw=case[3],available=true,state=snapshot(item),branches=branches(item)}end)
  if not ok then value={name=case[1],id=case[2],raw=case[3],available=false,error=tostring(value)}end
  result.family_probes[#result.family_probes+1]=value
 end
end
assert(equal(keys(tab.items),savedItemKeys)and equal(keys(tab.itemSets),savedSetKeys)and equal(keys(build.treeTab.specList),savedSpecKeys))
for id,saved in pairs(savedItems)do local item=tab.items[id];assert(item==saved.object and item.raw==saved.raw and item.baseModList==saved.base and item.modList==saved.mods and item.slotModList==saved.slots);assert(equal(snapshot(item),saved.state),"loaded item changed "..id)end
for id,saved in pairs(savedSets)do assert(tab.itemSets[id]==saved.object and equal(saved.object,saved.state))end
for i,saved in ipairs(savedSpecs)do local s=build.treeTab.specList[i];assert(s==saved.object and s.allocNodes==saved.nodes and equal(s.jewels,saved.jewels))
 local actual=allocations(s);assert(equal(keys(actual),keys(saved.allocations)))
 for id,before in pairs(saved.allocations)do local after=actual[id];assert(after.object==before.object and after.id==before.id and after.alloc==before.alloc and after.mode==before.mode)end
end
for _,host in ipairs(result.hosts)do local item=tab.items[host.id];assert(equal(consumer(env,item),host.delivery.main)and equal(consumer(calcEnv,item),host.delivery.calcs))end
assert(build.spec==spec and equal(selected(),selection))
assert(build.calcsTab.mainEnv==env and build.calcsTab.calcsEnv==calcEnv and build.calcsTab.mainOutput==output and build.calcsTab.calcsOutput==calcOutput)
assert(equal(output,oldOutput)and equal(calcOutput,oldCalcOutput))
for name,f in pairs(auth.methods)do assert(class[name]==f)end
assert(tab.Load==auth.load and modLib.parseMod==auth.parser and calcs.initEnv==auth.init and calcs.buildModListForNodeList==auth.nodes and common.classes.ModStore.ScaleAddMod==auth.scale and modLib.setSource==auth.setSource and common.classes.CalcsTab.BuildOutput==auth.output)
result.saved_items_preserved=true;result.saved_item_sets_preserved=true;result.saved_specs_preserved=true;result.saved_selections_preserved=true;result.main_output_preserved=true;result.calcs_output_preserved=true;result.original_functions_preserved=true;result.fresh_loaded_items=true
return result
