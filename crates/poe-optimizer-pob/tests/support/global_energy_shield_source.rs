//! Original-source observations only. Parsed Global scope and source property
//! tags are separate facts; neither provides whole-item coverage authority.
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
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local format=original(itemLib.formatValue,"Modules/ItemTools.lua",45,58)
local range=original(itemLib.applyRange,"Modules/ItemTools.lua",130)
local parser=original(modLib.parseMod,"Modules/ModParser.lua",7404)
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
local function records(list,all)
 local out={};for _,m in ipairs(list or {}) do if all or m.name=="EnergyShield" then
  local tags={};for _,t in ipairs(m) do tags[#tags+1]=plain(t) end
  out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}
 end end;return out
end
local function snapshot(item,list,successor)
 local out={name=item.name,base=item.baseName,kind=item.type,rarity=item.rarity,quality=item.quality,catalyst=item.catalyst,catalyst_quality=item.catalystQuality,
  lists={},active=records(list),base_mods=records(item.baseModList),armour_data=plain(item.armourData)}
 for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"}) do
  local lines={};for _,line in ipairs(item[category.."ModLines"] or {}) do
   local row={line=line.line,records=records(line.modList,line.line==successor),field_types={extra=type(line.extra)},catalyst_factor=catalyst(item.catalyst,line,item.catalystQuality)}
   for _,k in ipairs({"extra","modTags","range","corruptedRange","valueScalar","unscalable","disabled","bonded","enchant","rune"}) do row[k]=plain(line[k]) end
   lines[#lines+1]=row
  end;out.lists[category]=lines
 end;return out
end
local function construct(raw,id,successor)
 local item=new("Item"):Item("");item.id=id;parse(item,raw);local before=snapshot(item,nil,successor);mods(item)
 local after=snapshot(item,active(item,1,false),successor)
 assert(equal(after,snapshot(item,active(item,1,false),successor)))
 return item,before,after
end
local doc,err=common.xml.ParseXML(globalEsXml);assert(doc and not err)
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local outputValues={};for k,v in pairs(output) do if type(v)=="number" or type(v)=="boolean" or type(v)=="string" then outputValues[k]=v end end
local result={selected=saved,main_output=outputValues,player_energy_shield=records(env.modDB.mods.EnergyShield),
 scalability=plain(data.modScalability["#% increased maximum Energy Shield"]),cases={},constructors={},formats={},direct_parses={}}
local preserved={}
for _,wanted in ipairs(globalEsTargets) do
 local raw,xmlRows
 for _,n in ipairs(doc[1]) do if type(n)=="table" and n.elem=="Items" then
  for _,entry in ipairs(n) do if type(entry)=="table" and entry.elem=="Item" and entry.attrib.id==tostring(wanted.item_id) then
   assert(not raw);local text={};xmlRows={}
   for _,child in ipairs(entry) do if type(child)=="string" then text[#text+1]=child else xmlRows[#xmlRows+1]={name=child.elem,attributes=plain(child.attrib)} end end
   raw=table.concat(text,"\n")
  end end
 end end
 assert(raw and raw:find(wanted.line,1,true) and raw:find(wanted.next_line,1,true))
 local item=assert(build.itemsTab.items[wanted.item_id]);assert(item.baseName==wanted.base)
 local loaded=snapshot(item,item.modList or item.slotModList and item.slotModList[1],wanted.next_line)
 local selected={};for slot,value in pairs(env.player.itemList) do if value==item then selected[#selected+1]=slot end end;table.sort(selected)
 local fresh,before,after=construct(raw,wanted.item_id,wanted.next_line)
 local player={};for _,m in ipairs(result.player_energy_shield) do if m.source==fresh.modSource then player[#player+1]=m end end
 result.cases[#result.cases+1]={item_id=wanted.item_id,line=wanted.line,next_line=wanted.next_line,base=wanted.base,player_participating=wanted.player_participating,
  raw=raw,xml_children=xmlRows,loaded=loaded,fresh_before=before,fresh=after,selected_slots=selected,source_player_records=player,
  base_facts={has_buff=not not(item.base.flask and item.base.flask.buff or item.base.charm and item.base.charm.buff)}}
 preserved[#preserved+1]={item=item,id=wanted.item_id,loaded=loaded,successor=wanted.next_line}
end
local tail="+7 to maximum Life"
local function probe(name,line,header,extra,base)
 local raw="Rarity: RARE\nGlobal Energy Shield witness\n"..(base or "Gold Amulet").."\nQuality: 0\n"..(header or "").."\nImplicits: 0\n"..line.."\n"..tail
 if extra then raw=raw.."\n"..extra end
 local item,before,after=construct(raw,9001,tail)
 result.constructors[#result.constructors+1]={name=name,raw=raw,before=before,after=after}
end
for _,amount in ipairs({0,1,16,20,44,1000000,1000001}) do probe("integer-"..amount,amount.."% increased maximum Energy Shield") end
probe("leading-zero","0000044% increased maximum Energy Shield")
probe("untagged-carapace","44% increased maximum Energy Shield","Catalyst: Carapace\nCatalystQuality: 20")
probe("untagged-negative-quality","44% increased maximum Energy Shield","Catalyst: Carapace\nCatalystQuality: -200")
probe("tagged-carapace","{tags:energyshield}44% increased maximum Energy Shield","Catalyst: Carapace\nCatalystQuality: 20")
probe("decimal","10.5% increased maximum Energy Shield")
probe("reduced","11% reduced maximum Energy Shield")
probe("negative","-11% increased maximum Energy Shield")
probe("ranged","(10-20)% increased maximum Energy Shield")
probe("corrupted-range","{corruptedRange:1.5}11% increased maximum Energy Shield")
probe("disabled","{disabled}44% increased maximum Energy Shield")
probe("unknown-predecessor","Owned unknown predecessor\n44% increased maximum Energy Shield")
probe("ordinary-magnitude","25% increased maximum Energy Shield",nil,"20% increased explicit modifier magnitudes")
probe("crafted-magnitude","25% increased maximum Energy Shield","Crafted: true","20% increased explicit modifier magnitudes")
probe("crafted-cancellation-up-down","25% increased maximum Energy Shield","Crafted: true","20% increased explicit modifier magnitudes\n20% reduced explicit modifier magnitudes")
probe("crafted-cancellation-down-up","25% increased maximum Energy Shield","Crafted: true","20% reduced explicit modifier magnitudes\n20% increased explicit modifier magnitudes")
probe("armour-local","11% increased Energy Shield",nil,nil,"Adherent Cuffs")
probe("armour-global","11% increased maximum Energy Shield",nil,nil,"Adherent Cuffs")
for _,entry in ipairs({
 {"quantize-before-magnitude","10.5",1.2,1},
 {"magnitude-truncation","11",0.5,1},
 {"corruption-rounding","11",1,0.5},
 {"all-stages","10.5",1.2,0.5},
 {"source-quality-grouping","1000",(100+0.7)/100,1},
 {"reassociated-quality-contrast","1000",1+0.7/100,1},
}) do
 local line=entry[2].."% increased maximum Energy Shield"
 local formatted=range(line,1,entry[3],entry[4]);local parsed,extra=parser(formatted)
 result.formats[#result.formats+1]={name=entry[1],line=line,magnitude=entry[3],corrupted_base=entry[4],formatted=formatted,records=records(parsed,true),extra=extra,extra_type=type(extra)}
end
for _,line in ipairs({"44% increased maximum Energy Shield","11% increased Energy Shield","11% reduced maximum Energy Shield","-11% increased maximum Energy Shield","10.5% increased maximum Energy Shield"}) do
 local parsed,extra=parser(line)
 result.direct_parses[#result.direct_parses+1]={line=line,records=records(parsed,true),extra=extra,extra_type=type(extra)}
end
for _,p in ipairs(preserved) do
 assert(build.itemsTab.items[p.id]==p.item and equal(p.loaded,snapshot(p.item,p.item.modList or p.item.slotModList and p.item.slotModList[1],p.successor)))
end
result.observed_item_fields_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group)
result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues) do assert(output[k]==v) end
assert(equal(result.player_energy_shield,records(env.modDB.mods.EnergyShield)))
result.main_scalar_output_preserved=true
result.player_contributions_preserved=true
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and upvalue(parse,"getCatalystScalar")==catalyst and itemLib.formatValue==format and itemLib.applyRange==range and modLib.parseMod==parser)
result.original_functions_preserved=true
return result
"##;
