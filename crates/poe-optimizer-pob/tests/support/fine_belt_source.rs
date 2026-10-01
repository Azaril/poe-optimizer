pub const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local normalise=original(class.NormaliseQuality,"Classes/Item.lua",1805,1813)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local slotLists=original(class.BuildModListsForSlots,"Classes/Item.lua",2671,2680)
local slotList=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local variantCheck=original(class.CheckModLineVariant,"Classes/Item.lua",2289,2318)
local variantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320,2336)
local applyRange=original(itemLib.applyRange,"Modules/ItemTools.lua",130)
local formatValue=original(itemLib.formatValue,"Modules/ItemTools.lua",45,58)
local function plain(v,d)if type(v)~="table"then assert(type(v)~="function"and type(v)~="userdata");return v end;d=(d or 0)+1;assert(d<10);local o={};local n=0;for k,x in pairs(v)do n=n+1;assert(n<512);o[k]=plain(x,d)end;return o end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list,filter)local out={};for _,m in ipairs(list or {})do if not filter or filter(m)then local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,source_slot_num=m.sourceSlotNum,tags=tags}end end;return out end
local scalarFields={"title","name","baseName","type","rarity","crafted","itemLevel","quality","corrupted","doubleCorrupted","mirrored","sanctified","desecrated","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","charmLimit","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
local function snapshot(item,list)
 local out={base=item.baseName,field_types={},lists={},base_mods=records(item.baseModList),active=records(list)}
 for _,k in ipairs(scalarFields)do out[k]=item[k];out.field_types[k]=type(item[k])end
 for _,k in ipairs({"prefixes","suffixes","requirements","armourData","sockets","runes","variantList","socketedSoulCoreTypes"})do out[k]=plain(item[k]);out.field_types[k]=type(item[k])end
 for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"})do local lines={};for _,line in ipairs(item[category.."ModLines"] or {})do
  local row={line=line.line,records=records(line.modList),field_types={extra=type(line.extra)}}
  for _,k in ipairs({"extra","modTags","variantList","range","corruptedRange","valueScalar","custom","unscalable","disabled","desecrated","fractured","bonded","prefix","suffix","rune","enchant","augmentType","socketedAugmentTypeOverride","socketedSoulCoreType"})do row[k]=plain(line[k]);row.field_types[k]=type(line[k])end;row.catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality)
  lines[#lines+1]=row
 end;out.lists[category]=lines end
 return out
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local doc,err=common.xml.ParseXML(fineBeltXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};local xmlRows={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local text={};local childNames={};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else childNames[#childNames+1]={name=s.elem,attributes=plain(s.attrib)}end end;local id=assert(tonumber(v.attrib.id));rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),text_nodes=#text,children=childNames}end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},probes={},scalability={charm=plain(data.modScalability["Has # Charm Slot"]),flask=plain(data.modScalability["Flasks gain # charges per Second"])}}
local originals,snapshots={},{}
local function replace(text,old,new)local start=assert(text:find(old,1,true));assert(not text:find(old,start+#old,true));return text:sub(1,start-1)..new..text:sub(start+#old)end
local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
for _,id in ipairs({27})do
 local item=assert(build.itemsTab.items[id]);originals[id]=item
 local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local raw=assert(rawItems[id]);local fresh=construct(raw,id);local before=snapshot(fresh,nil);mods(fresh)
 local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 local receiving={};for _,set in ipairs(node)do if type(set)=="table"and set.elem=="ItemSet"then for _,slot in ipairs(set)do if type(slot)=="table"and slot.elem=="Slot"and tonumber(slot.attrib.itemId)==id then receiving[#receiving+1]={set=tonumber(set.attrib.id),slot=slot.attrib.name}end end end end
 table.sort(receiving,function(a,b)if a.set~=b.set then return a.set<b.set end;return a.slot<b.slot end)
 local function fromItem(m)return (type(m.source)=="string"and m.source:sub(1,8)=="Item:27:")or(m.name=="CharmLimit"and m.source==item.title)end
 local player={};for _,name in ipairs({"CharmLimit","FlaskChargesGenerated","Life"})do player[name]=records(env.modDB.mods[name],fromItem)end
 local requirements={};for _,row in ipairs(env.requirementsTableItems)do if row.sourceItem==item then requirements[#requirements+1]={source=row.source,slot=row.sourceSlot,Str=row.Str,Dex=row.Dex,Int=row.Int}end end
 local slots={};for slot,list in pairs(item.slotModList or {})do slots[#slots+1]={slot=slot,kind="indexed",records=records(list),active=records(active(item,slot,false))}end;table.sort(slots,function(a,b)return a.slot<b.slot end)
 if item.modList then assert(#slots==0);slots[1]={kind="shared-mod-list",records=records(item.modList),active=records(active(item,1,false))}end
 local row={id=id,raw=raw,xml=xmlRows[id],loaded=loaded,fresh_before_build=before,fresh=snapshot(fresh,active(fresh,1,false)),selected_slots=selected,authored_receiving_uses=receiving,player_records=player,requirements_rows=requirements,slot_lists=slots,probes={},base_facts={name=item.baseName,charm_limit=item.base.charmLimit,type=item.base.type,subtype=item.base.subType,implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),armour=plain(item.base.armour),requirements=plain(item.base.req),quality=item.base.quality,socket_limit=item.base.socketLimit,flask=plain(item.base.flask),charm=plain(item.base.charm)}}
 if fineBeltControls then
  local charm="{tags:charm}{range:0.5}Has (1-3) Charm Slot";local flask="Flasks gain 0.17 charges per Second";local life="+10 to maximum Life"
  local cases={
   {"header-zero",replace(raw,"Charm Slots: 2","Charm Slots: 0")},
   {"header-high",replace(raw,"Charm Slots: 2","Charm Slots: 9")},
   {"header-absent",replace(raw,"Charm Slots: 2\n","")},
   {"header-malformed",replace(raw,"Charm Slots: 2","Charm Slots: Nope")},
   {"header-duplicate",replace(raw,"Charm Slots: 2","Charm Slots: 2\nCharm Slots: 9")},
   {"range-low",replace(raw,charm,"{tags:charm}{range:0}Has (1-3) Charm Slot")},
   {"range-high",replace(raw,charm,"{tags:charm}{range:1}Has (1-3) Charm Slot")},
   {"quality-zero",header(raw,"Quality: 0")},{"quality-twenty",header(raw,"Quality: 20")},
   {"quality-malformed",header(raw,"Quality: Nope")},{"quality-alias",header(raw,"Quality (Life Modifiers): 20%")},
   {"item-level",header(raw,"Item Level: 77")},
   {"catalyst-kind",header(raw,"Catalyst: Flesh")},
   {"catalyst-zero",header(raw,"Catalyst: Flesh\nCatalystQuality: 0")},
   {"catalyst-only-amount",header(raw,"CatalystQuality: 37")},
   {"level-zero",replace(raw,"LevelReq: 62","LevelReq: 0")},
   {"level-absent",replace(raw,"LevelReq: 62\n","")},
   {"crafted-false",replace(raw,"Crafted: true","Crafted: false")},
   {"saved-affix-label",replace(raw,"Prefix: {range:0}IncreasedLife1","Prefix: None")},
   {"corrupted",raw.."\nCorrupted"},{"twice-corrupted",raw.."\nTwice Corrupted"},
   {"empty-rune",header(raw,"Sockets: S\nRune: None")},{"unknown-rune",header(raw,"Sockets: S\nRune: OwnedUnknownRune")},
   {"implicit-one",replace(raw,"Implicits: 2","Implicits: 1")},
   {"implicit-three",replace(raw,"Implicits: 2","Implicits: 3")},
   {"missing-charm",replace(raw,charm.."\n","")},
   {"missing-flask",replace(raw,flask.."\n","")},
   {"unknown-before",replace(raw,charm,"Owned unknown member\n"..charm)},
   {"extra-life",raw.."\n+3 to maximum Life"},
   {"enchant-charm",replace(raw,charm,"{enchant}"..charm)},
   {"implicit-magnitude",raw.."\n50% increased implicit modifier magnitudes"},
   {"explicit-magnitude",raw.."\n50% increased explicit modifier magnitudes"},
   {"charm-tag-magnitude",raw.."\n50% increased charm modifier magnitudes"},
   {"flask-range",replace(raw,flask,"{range:0.5}Flasks gain (0.1-0.3) charges per Second")},
   {"flask-range-magnitude",replace(raw,flask,"{range:0.5}Flasks gain (0.1-0.3) charges per Second").."\n50% increased implicit modifier magnitudes"},
   {"flask-fraction",replace(raw,flask,"Flasks gain 0.175 charges per Second")},
   {"flask-eighth",replace(raw,flask,"Flasks gain 0.125 charges per Second")},
   {"flask-small",replace(raw,flask,"Flasks gain 0.001 charges per Second")},
   {"flask-zero",replace(raw,flask,"Flasks gain 0 charges per Second")},
   {"flask-negative",replace(raw,flask,"Flasks gain -0.17 charges per Second")},
   {"flask-corrupted-range",replace(raw,flask,"{corruptedRange:1.5}"..flask)},
   {"charm-zero",replace(raw,charm,"{tags:charm}{range:0.5}Has (0-0) Charm Slot")},
   {"charm-corrupted-range",replace(raw,charm,"{corruptedRange:1.5}"..charm)}
  }
  for _,case in ipairs(cases)do local probe=construct(case[2],id);local initial=snapshot(probe,nil);mods(probe);local after=snapshot(probe,active(probe,1,false));assert(equal(after,snapshot(probe,active(probe,1,false))));row.probes[#row.probes+1]={name=case[1],raw=case[2],before=initial,after=after}end
  -- Isolated post-parse state contrasts use the same complete BuildModList.
  -- Ranged lines are recomputed; fixed lines retain their parsed numeric cache.
  row.rebuilds={}
  for _,form in ipairs({"fixed","ranged"})do
   local text=raw;if form=="ranged"then text=replace(raw,flask,"{range:0.5}Flasks gain (0.1-0.3) charges per Second")end
   local probe=construct(text,id);mods(probe);local baseline=snapshot(probe,active(probe,1,false));local steps={}
   for _,factor in ipairs({1.5,2,1})do
    for _,line in ipairs(probe.implicitModLines)do line.valueScalar=factor end
    mods(probe);steps[#steps+1]={factor=factor,state=snapshot(probe,active(probe,1,false))}
   end
   row.rebuilds[#row.rebuilds+1]={form=form,baseline=baseline,steps=steps}
  end
  local reused=construct(header(raw,"Item Level: 77\nQuality: 20\nCatalyst: Flesh\nCatalystQuality: 37\nSockets: S\nRune: None").."\nCorrupted",id);mods(reused);parse(reused,raw);mods(reused);row.reparsed=snapshot(reused,active(reused,1,false))
  local again=construct(raw,id);mods(again);row.fresh_after_reparse=snapshot(again,active(again,1,false))
 end
 for _,name in ipairs({"CharmLimit","FlaskChargesGenerated","Life"})do assert(equal(player[name],records(env.modDB.mods[name],fromItem)))end
 result.items[#result.items+1]=row
end
for _,id in ipairs({27})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true;result.main_output=outputValues
assert(class.NormaliseQuality==normalise and class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and class.BuildModListsForSlots==slotLists and class.BuildModListForSlotNum==slotList and class.CheckModLineVariant==variantCheck and class.GetModLineVariantCount==variantCount and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
assert(itemLib.applyRange==applyRange and itemLib.formatValue==formatValue)
return result
"##;
