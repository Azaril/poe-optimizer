//! Observations of complete pinned source methods, never replacement recipes.
pub const OBSERVE: &str = r##"
local function original(f,path,first,last)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==first)
 if last then assert(i.lastlinedefined==last) end;return f
end
local function upvalue(f,wanted)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==wanted then return v end end
 error("missing original upvalue "..wanted)
end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local format=original(itemLib.formatValue,"Modules/ItemTools.lua",45,58)
local range=original(itemLib.applyRange,"Modules/ItemTools.lua",130)
local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
local scaleAdd=original(common.classes.ModStore.ScaleAddMod,"Classes/ModStore.lua",82,119)
local function plain(v,d)
 if type(v)~="table" then assert(type(v)~="function" and type(v)~="userdata");return v end
 d=(d or 0)+1;assert(d<10);local out={};local count=0
 for k,x in pairs(v) do count=count+1;assert(count<512);out[k]=plain(x,d) end;return out
end
local function equal(a,b)
 if type(a)~=type(b) then return false end;if type(a)~="table" then return a==b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k]==nil then return false end end;return true
end
local function records(list,filter)
 local out={};for _,m in ipairs(list or {}) do if not filter or filter(m) then
  local tags={};for _,t in ipairs(m) do tags[#tags+1]=plain(t) end
  out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}
 end end;return out
end
local function spirit(m)return m.name=="Spirit" and m.type=="BASE"end
local function snapshot(item,list)
 local out={name=item.name,base=item.baseName,kind=item.type,rarity=item.rarity,item_level=item.itemLevel,quality=item.quality,catalyst=item.catalyst,catalyst_quality=item.catalystQuality,
  field_types={item_level=type(item.itemLevel),quality=type(item.quality),catalyst=type(item.catalyst),catalyst_quality=type(item.catalystQuality)},lists={},active=records(list),base_mods=records(item.baseModList)}
 for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"}) do
  local lines={};for _,line in ipairs(item[category.."ModLines"] or {}) do
   local row={line=line.line,records=records(line.modList),field_types={extra=type(line.extra)},catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality)}
   for _,k in ipairs({"extra","modTags","range","corruptedRange","valueScalar","custom","unscalable","disabled","desecrated","prefix","suffix","bonded","enchant","rune","variantList"}) do row[k]=plain(line[k]) end
   lines[#lines+1]=row
  end;out.lists[category]=lines
 end;return out
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;parse(item,raw);return item end
local function replace(text,old,new)local at=assert(text:find(old,1,true));assert(not text:find(old,at+#old,true));return text:sub(1,at-1)..new..text:sub(at+#old)end
local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
local doc,err=common.xml.ParseXML(spiritXml);assert(doc and not err)
local itemsNode;for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then assert(not itemsNode);itemsNode=n end end;assert(itemsNode)
local raw,xmlRows;for _,n in ipairs(itemsNode) do if type(n)=="table" and n.elem=="Item" and n.attrib.id=="23" then
 assert(not raw);local text={};xmlRows={};for _,v in ipairs(n) do if type(v)=="string" then text[#text+1]=v else xmlRows[#xmlRows+1]={name=v.elem,attributes=plain(v.attrib)} end end;raw=table.concat(text,"\n")
end end;assert(raw)
local needle="{range:0.5}+(10-15) to Spirit";assert(raw:find(needle,1,true))
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local item=assert(build.itemsTab.items[23]);assert(item.baseName=="Solar Amulet")
local loaded=snapshot(item,item.modList or item.slotModList[1]);local selected={}
for slot,value in pairs(env.player.itemList) do if value==item then selected[#selected+1]=slot end end;table.sort(selected)
local playerRecords=records(assert(env.modDB.mods.Spirit),spirit)
local result={selected=saved,raw=raw,xml_children=xmlRows,loaded=loaded,selected_slots=selected,player_spirit=playerRecords,main_output=outputValues,
 base_facts={implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),requirements=plain(item.base.req)},scalability=plain(data.modScalability["# to Spirit"]),default_range=main.defaultItemAffixQuality,probes={},copy_probes={}}
if spiritControls then
 local fresh=construct(raw,23);local before=snapshot(fresh,nil);mods(fresh);result.fresh_before=before;result.fresh=snapshot(fresh,active(fresh,1,false))
 assert(equal(loaded.lists,result.fresh.lists));assert(equal(loaded.base_mods,result.fresh.base_mods))
 local cases={
  {"range-zero","{range:0}+(10-15) to Spirit"},
  {"range-low","{range:0.1}+(10-15) to Spirit"},
  {"range-mid","{range:0.5}+(10-15) to Spirit"},
  {"range-high","{range:0.9}+(10-15) to Spirit"},
  {"range-one","{range:1}+(10-15) to Spirit"},
  {"range-absent","+(10-15) to Spirit"},
  {"zero-range","{range:0.5}+(0-0) to Spirit"},
  {"fractional-bound","{range:0.5}+(10.2-15.2) to Spirit"},
  {"negative-bound","{range:0.5}+(-10-15) to Spirit"},
  {"reversed-bound","{range:0.5}+(15-10) to Spirit"},
  {"untagged-neural",needle,"Catalyst: Neural\nCatalystQuality: 20"},
  {"tagged-neural","{tags:mana}"..needle,"Catalyst: Neural\nCatalystQuality: 20"},
  {"tagged-wrong-catalyst","{tags:mana}"..needle,"Catalyst: Flesh\nCatalystQuality: 20"},
  {"implicit-magnitude",needle,nil,"50% increased implicit modifier magnitudes"},
  {"explicit-magnitude",needle,nil,"50% increased explicit modifier magnitudes"},
  {"corrupted-range","{corruptedRange:1.5}"..needle},
  {"disabled","{disabled}"..needle},
  {"unknown-before","Owned witness unknown line\n"..needle},
  {"enchant","{enchant}"..needle},
  {"item-level-present",needle,"Item Level: 77"}
 }
 for _,case in ipairs(cases) do
  local text=replace(raw,needle,case[2]);if case[3] then text=header(text,case[3]) end;if case[4] then text=text.."\n"..case[4] end
  local current=construct(text,9001);local initial=snapshot(current,nil);mods(current);local after=snapshot(current,active(current,1,false))
  assert(equal(after,snapshot(current,active(current,1,false))))
  result.probes[#result.probes+1]={name=case[1],raw=text,before=initial,after=after}
 end
 local reused=construct(header(raw,"Item Level: 77"),9002);mods(reused);parse(reused,raw);mods(reused)
 result.reused_item_level={field_type=type(reused.itemLevel),value=reused.itemLevel}
 local again=construct(raw,9003);mods(again);result.fresh_item_level={field_type=type(again.itemLevel),value=again.itemLevel}
 local originalRecords=records(active(fresh,1,false),spirit)
 for _,factor in ipairs({0,0.25,0.5,1}) do
  local target=new("ModList"):ModList();for _,m in ipairs(active(fresh,1,false)) do if spirit(m) then scaleAdd(target,m,factor) end end
  result.copy_probes[#result.copy_probes+1]={factor=factor,records=records(target,spirit)}
 end;assert(equal(originalRecords,records(active(fresh,1,false),spirit)))
end
assert(build.itemsTab.items[23]==item and equal(loaded,snapshot(item,item.modList or item.slotModList[1])));result.saved_item_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end;result.main_output_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and upvalue(parse,"getCatalystScalar")==catalyst and itemLib.formatValue==format and itemLib.applyRange==range and modLib.parseMod==parser and common.classes.ModStore.ScaleAddMod==scaleAdd);result.original_functions_preserved=true
return result
"##;
