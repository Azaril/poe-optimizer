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
local function plain(v,d)if type(v)~="table"then assert(type(v)~="function"and type(v)~="userdata");return v end;d=(d or 0)+1;assert(d<10);local o={};local n=0;for k,x in pairs(v)do n=n+1;assert(n<512);o[k]=plain(x,d)end;return o end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list)local out={};for _,m in ipairs(list or {})do local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}end;return out end
local scalarFields={"title","name","baseName","type","rarity","crafted","itemLevel","quality","corrupted","doubleCorrupted","mirrored","sanctified","desecrated","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
local function snapshot(item,list)
 local out={base=item.baseName,field_types={},lists={},base_mods=records(item.baseModList),active=records(list)}
 for _,k in ipairs(scalarFields)do out[k]=item[k];out.field_types[k]=type(item[k])end
 for _,k in ipairs({"prefixes","suffixes","requirements","armourData","sockets","runes","variantList","socketedSoulCoreTypes"})do out[k]=plain(item[k]);out.field_types[k]=type(item[k])end
 for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"})do local lines={};for _,line in ipairs(item[category.."ModLines"] or {})do
  local row={line=line.line,records=records(line.modList),field_types={extra=type(line.extra)}}
  for _,k in ipairs({"extra","modTags","variantList","range","corruptedRange","valueScalar","custom","unscalable","disabled","bonded","prefix","suffix","rune","enchant","augmentType","socketedAugmentTypeOverride","socketedSoulCoreType"})do row[k]=plain(line[k])end;row.catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality)
  lines[#lines+1]=row
 end;out.lists[category]=lines end
 return out
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local doc,err=common.xml.ParseXML(solarItemXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};local xmlRows={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local text={};local childNames={};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else childNames[#childNames+1]={name=s.elem,attributes=plain(s.attrib)}end end;local id=assert(tonumber(v.attrib.id));rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),text_nodes=#text,children=childNames}end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},probes={}}
local originals,snapshots={},{}
local function replace(text,old,new)local start=assert(text:find(old,1,true));assert(not text:find(old,start+#old,true));return text:sub(1,start-1)..new..text:sub(start+#old)end
local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
for _,id in ipairs({23})do
 local item=assert(build.itemsTab.items[id]);originals[id]=item
 local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local raw=assert(rawItems[id]);local fresh=construct(raw,id);local before=snapshot(fresh,nil);mods(fresh)
 local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 local requirements={};for _,row in ipairs(env.requirementsTableItems)do if row.sourceItem==item then requirements[#requirements+1]={source=row.source,slot=row.sourceSlot,Str=row.Str,Dex=row.Dex,Int=row.Int}end end
 local slots={};for slot,list in pairs(item.slotModList or {})do slots[#slots+1]={slot=slot,kind="indexed",records=records(list),active=records(active(item,slot,false))}end;table.sort(slots,function(a,b)return a.slot<b.slot end)
 if item.modList then assert(#slots==0);slots[1]={kind="shared-mod-list",records=records(item.modList),active=records(active(item,1,false))}end
 local row={id=id,raw=raw,xml=xmlRows[id],loaded=loaded,fresh_before_build=before,fresh=snapshot(fresh,active(fresh,1,false)),selected_slots=selected,requirements_rows=requirements,slot_lists=slots,probes={},base_facts={name=item.baseName,type=item.base.type,subtype=item.base.subType,implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),armour=plain(item.base.armour),requirements=plain(item.base.req),quality=item.base.quality,socket_limit=item.base.socketLimit,flask=plain(item.base.flask),charm=plain(item.base.charm)}}
 if solarItemControls then
  local spirit="{range:0.5}+(10-15) to Spirit";local minion="+1 to Level of all Minion Skills"
  local cases={
   {"quality-malformed",header(raw,"Quality: Nope")},
   {"quality-duplicate",header(raw,"Quality: 0\nQuality: 20")},
   {"catalyst-style-quality",header(raw,"Quality (Mana Modifiers): 20%")},
   {"catalyst-kind",header(raw,"Catalyst: Neural")},
   {"catalyst-zero",header(raw,"Catalyst: Neural\nCatalystQuality: 0")},
   {"catalyst-amount-only",header(raw,"CatalystQuality: 37")},
   {"catalyst-matching-tag",header(replace(raw,spirit,"{tags:mana}"..spirit),"Catalyst: Neural")},
   {"level-zero",replace(raw,"LevelReq: 30","LevelReq: 0")},
   {"level-high",replace(raw,"LevelReq: 30","LevelReq: 77")},
   {"level-absent",replace(raw,"LevelReq: 30\n","")},
   {"level-alias",replace(raw,"LevelReq: 30","Requires Level: 77")},
   {"level-duplicate",replace(raw,"LevelReq: 30","LevelReq: 30\nLevelReq: 77")},
   {"level-malformed",replace(raw,"LevelReq: 30","LevelReq: Nope")},
   {"crafted-false",replace(raw,"Crafted: true","Crafted: false")},
   {"corrupted",raw.."\nCorrupted"},{"twice-corrupted",raw.."\nTwice Corrupted"},
   {"mirrored",raw.."\nMirrored"},{"desecrated-setter",raw.."\nDesecrated Prefix"},
   {"empty-rune-socket",header(raw,"Sockets: S\nRune: None")},
   {"jewel-socket",header(raw,"Sockets: J")},
   {"unknown-rune",header(raw,"Sockets: S\nRune: OwnedUnknownRune")},
   {"no-implicit-count",replace(raw,"Implicits: 1\n","")},
   {"implicit-zero",replace(raw,"Implicits: 1","Implicits: 0")},
   {"implicit-two",replace(raw,"Implicits: 1","Implicits: 2")},
   {"missing-spirit",replace(raw,spirit.."\n","")},
   {"missing-minion",replace(raw,minion,"")},
   {"duplicate-spirit",replace(raw,spirit,spirit.."\n"..spirit)},
   {"extra-member",raw.."\n+3 to maximum Life"},
   {"unknown-before",replace(raw,spirit,"Owned unknown member\n"..spirit)},
   {"enchant-spirit",replace(raw,spirit,"{enchant}"..spirit)},
   {"reversed-lines",replace(raw,spirit.."\n"..minion,minion.."\n"..spirit)},
   {"class-restricted",raw.."\nRequires Class Witch"}
  }
  for _,case in ipairs(cases)do local probe=construct(case[2],id);local initial=snapshot(probe,nil);mods(probe);local after=snapshot(probe,active(probe,1,false));assert(equal(after,snapshot(probe,active(probe,1,false))));row.probes[#row.probes+1]={name=case[1],raw=case[2],before=initial,after=after}end
  local reused=construct(header(raw,"Quality: 20\nCatalyst: Neural\nCatalystQuality: 37\nSockets: S\nRune: None").."\nCorrupted",id);mods(reused);parse(reused,raw);mods(reused);row.reparsed=snapshot(reused,active(reused,1,false))
  local again=construct(raw,id);mods(again);row.fresh_after_reparse=snapshot(again,active(again,1,false))
 end
 result.items[#result.items+1]=row
end
for _,id in ipairs({23})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true;result.main_output=outputValues
assert(class.NormaliseQuality==normalise and class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and class.BuildModListsForSlots==slotLists and class.BuildModListForSlotNum==slotList and class.CheckModLineVariant==variantCheck and class.GetModLineVariantCount==variantCount and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"##;
