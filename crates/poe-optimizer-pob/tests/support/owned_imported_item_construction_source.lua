-- Read-only observations of original methods. All mutations below target fresh Items.
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line,path..":"..line.." actual "..s..":"..tostring(i.linedefined));return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local calcs=require("Modules.CalcBase")
if importedSourcePhase=="before"then
 local constructor=original(class.Item,"Modules/Common.lua",167)
 original(upvalue(constructor,"originalFunc"),"Classes/Item.lua",93)
 local methods={Item=constructor,ParseRaw=original(class.ParseRaw,"Classes/Item.lua",468),NormaliseQuality=original(class.NormaliseQuality,"Classes/Item.lua",1805),BuildModList=original(class.BuildModList,"Classes/Item.lua",2694),GetActiveModListForSlotNum=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198),BuildModListsForSlots=original(class.BuildModListsForSlots,"Classes/Item.lua",2671),BuildModListForSlotNum=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414),CheckModLineVariant=original(class.CheckModLineVariant,"Classes/Item.lua",2289),GetModLineVariantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320)}
 local prior={};for _,item in pairs(build.itemsTab.items)do prior[item]=true end
 _importedSource={methods=methods,prior=prior,load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193),catalyst=original(upvalue(methods.ParseRaw,"getCatalystScalar"),"Classes/Item.lua",32),format=original(itemLib.formatValue,"Modules/ItemTools.lua",45),range=original(itemLib.applyRange,"Modules/ItemTools.lua",130),parser=original(modLib.parseMod,"Modules/ModParser.lua",7404),init=original(calcs.initEnv,"Modules/CalcSetup.lua",717),node=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200),nodes=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)}
 return function()_importedSource.finished=true end
end
local auth=assert(_importedSource);assert(auth.finished)
local function plain(v,d)
 if type(v)~="table"then assert(type(v)~="function" and type(v)~="userdata");return v end
 d=(d or 0)+1;assert(d<24);local o={};local count=0;for k,x in pairs(v)do count=count+1;assert(count<8192);o[k]=plain(x,d)end;return o
end
local function clone(v,seen)
 if type(v)~="table"then return v end;seen=seen or {};if seen[v]then return seen[v]end
 local result={};seen[v]=result;for k,x in pairs(v)do result[k]=clone(x,seen)end;return result
end
local function equal(a,b,seen)
 if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end
 seen=seen or {};if seen[a]then return seen[a]==b end;seen[a]=b
 for k,v in pairs(a)do if not equal(v,b[k],seen)then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true
end
local function records(list,filter)
 local result={};for _,m in ipairs(list or {})do if not filter or filter(m)then local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;result[#result+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,source_slot_num=m.sourceSlotNum,tags=tags}end end;return result
end
local scalarFields={"title","name","uniqueID","baseName","type","rarity","crafted","advancedCopy","itemLevel","quality","corrupted","doubleCorrupted","mirrored","sanctified","desecrated","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
local categories={"buff","enchant","rune","classRequirement","implicit","explicit"}
local function snapshot(item,list)
 local out={field_types={},lists={},base_mods=records(item.baseModList),active=records(list)}
 for _,k in ipairs(scalarFields)do out[k]=item[k];out.field_types[k]=type(item[k])end
 for _,k in ipairs({"rawLines","prefixes","suffixes","requirements","sockets","runes","variantList","socketedSoulCoreTypes","modMagnitudeMods"})do out[k]=plain(item[k]);out.field_types[k]=type(item[k])end
 for _,category in ipairs(categories)do local lines={};for ordinal,line in ipairs(item[category.."ModLines"] or {})do
  local row={line=line.line,ordinal=ordinal,category=category,records=records(line.modList),field_types={extra=type(line.extra)}}
  for _,k in ipairs({"extra","modTags","variantList","range","corruptedRange","valueScalar","custom","unscalable","disabled","desecrated","fractured","bonded","prefix","suffix","rune","enchant","crafted","mutated","augmentType","socketedAugmentTypeOverride","socketedSoulCoreType"})do row[k]=plain(line[k]);row.field_types[k]=type(line[k])end
  row.catalyst_factor=auth.catalyst(item.catalyst,line,item.catalystQuality)
  row.formatted=itemLib.applyRange(line.line,line.range or 1,line.valueScalar,line.corruptedRange)
  lines[#lines+1]=row
 end;out.lists[category]=lines end;return out
end
local function construct(raw,id)local item=new("Item"):Item("");item.id=id;auth.methods.ParseRaw(item,raw);return item end
local function finish(item)auth.methods.BuildModList(item);return snapshot(item,auth.methods.GetActiveModListForSlotNum(item,1,false))end
local function replace(text,old,new)local i=assert(text:find(old,1,true));assert(not text:find(old,i+#old,true));return text:sub(1,i-1)..new..text:sub(i+#old)end
local function header(text,value)return replace(text,"Item Level: 55",value.."\nItem Level: 55")end
local doc,err=common.xml.ParseXML(importedSourceXml);assert(doc and not err)
local itemsNode,tree;for _,n in ipairs(doc[1])do if type(n)=="table"then if n.elem=="Items"then assert(not itemsNode);itemsNode=n elseif n.elem=="Tree"then assert(not tree);tree=n end end end;assert(itemsNode and tree)
local rawItems,xmlRows={},{};for _,v in ipairs(itemsNode)do if type(v)=="table"and v.elem=="Item"then local text,children={},{};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else children[#children+1]=plain(s)end end;local id=assert(tonumber(v.attrib.id));assert(not rawItems[id]);rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),children=children,text_nodes=#text}end end
local selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local env=build.calcsTab.mainEnv;local calcEnv=build.calcsTab.calcsEnv;local output=build.calcsTab.mainOutput;local calcOutput=build.calcsTab.calcsOutput
local outputs={main=clone(output),calcs=clone(calcOutput)};local savedItems={};for id,item in pairs(build.itemsTab.items)do assert(not auth.prior[item]);savedItems[id]={object=item,raw=item.raw}end
local spec=build.spec;assert(spec==build.treeTab.specList[selected.spec] and env.spec==spec)
local result={selected=selected,items={},fresh_loaded_items=true,main_output={}}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.main_output[k]=v end end
if importedSourceTarget then
 local id=1;local nodeId=46882;local item=assert(build.itemsTab.items[id]);local raw=assert(rawItems[id]);assert(item.baseName=="Ruby")
 local specs={};for _,n in ipairs(tree)do if type(n)=="table"and n.elem=="Spec"then specs[#specs+1]=n end end
 local savedSpec=assert(specs[selected.spec]);local jewelMatches={};for _,n in ipairs(savedSpec)do if type(n)=="table"and n.elem=="Sockets"then for _,s in ipairs(n)do if type(s)=="table"and s.elem=="Socket"and tonumber(s.attrib.nodeId)==nodeId then jewelMatches[#jewelMatches+1]=plain(s)end end end end
 assert(#jewelMatches==1 and tonumber(jewelMatches[1].attrib.itemId)==id)
 assert(spec.jewels[nodeId]==id and spec.allocNodes[nodeId] and env.allocNodes[nodeId])
 local loaded=snapshot(item,auth.methods.GetActiveModListForSlotNum(item,1,false));local fresh=construct(raw,id);local initial=snapshot(fresh,nil);local rebuilt=finish(fresh)
 local receiving={};for slot,value in pairs(env.player.itemList)do if value==item then receiving[#receiving+1]=slot end end;table.sort(receiving);assert(#receiving==1)
 local function delivery(environment)
  local out={};local db=environment.modDB;local depth=0
  while db do for _,m in ipairs(db.mods.FireDamage or {})do if m.source==item.modSource then local r=records({m})[1];r.depth=depth;out[#out+1]=r end end;db=db.parent;depth=depth+1;assert(depth<8)end
  return out
 end
 local row={id=id,node=nodeId,raw=raw,xml=xmlRows[id],saved_socket=jewelMatches[1],loaded=loaded,fresh_before_build=initial,fresh=rebuilt,selected_slots=receiving,delivery={main=delivery(env),calcs=delivery(calcEnv)},base_facts={name=item.baseName,type=item.base.type,tags=plain(item.base.tags),implicit=plain(item.base.implicit),implicit_mod_types=plain(item.base.implicitModTypes),requirements=plain(item.base.req),quality=item.base.quality,quality_type=type(item.base.quality)},probes={}}
 -- Loaded XML can restore ModRange; fixed-line semantics and ordinary fields must match a fresh parse.
 local comparison=clone(loaded);local freshComparison=clone(rebuilt)
 for _,state in ipairs({comparison,freshComparison})do for _,category in ipairs(categories)do for _,line in ipairs(state.lists[category])do line.range=nil;line.field_types.range=nil end end end
 assert(equal(comparison,freshComparison),"fresh Ruby reconstruction changed a non-range field")
 result.loaded_fresh_equivalent_except_legacy_range=true
 if importedSourceControls then
  local line="14% increased Fire Damage"
  local unique=assert(raw:match("Unique ID: ([^\n]+)"))
  local uniqueLine="Unique ID: "..unique
  local alternate=string.rep("a",64)
  local cases={
   {"identity-alternate",replace(raw,uniqueLine,"Unique ID: "..alternate)},
   {"identity-missing",replace(raw,uniqueLine.."\n","")},
   {"identity-empty",replace(raw,uniqueLine,"Unique ID: ")},
   {"identity-duplicate",replace(raw,uniqueLine,uniqueLine.."\nUnique ID: "..alternate)},
   {"identity-malformed",replace(raw,uniqueLine,"Unique ID: not-hex")},
   {"identity-alias",replace(raw,uniqueLine,"Unique Id: "..unique)},
   {"levelreq-positive",replace(raw,"LevelReq: 0","LevelReq: 17")},
   {"levelreq-missing",replace(raw,"LevelReq: 0\n","")},
   {"levelreq-malformed",replace(raw,"LevelReq: 0","LevelReq: Nope")},
   {"levelreq-duplicate",replace(raw,"LevelReq: 0","LevelReq: 0\nLevelReq: 17")},
   {"item-level-zero",replace(raw,"Item Level: 55","Item Level: 0")},
   {"item-level-positive",replace(raw,"Item Level: 55","Item Level: 90")},
   {"item-level-missing",replace(raw,"Item Level: 55\n","")},
   {"item-level-malformed",replace(raw,"Item Level: 55","Item Level: Nope")},
   {"item-level-duplicate",replace(raw,"Item Level: 55","Item Level: 55\nItem Level: 90")},
   {"rarity-magic",replace(raw,"Rarity: RARE","Rarity: MAGIC")},
   {"corrupted",raw.."\nCorrupted"},{"twice-corrupted",raw.."\nTwice Corrupted"},
   {"mirrored",raw.."\nMirrored"},{"unidentified",header(raw,"Unidentified")},
   {"sockets-empty",header(raw,"Sockets: ")},{"sockets-s",header(raw,"Sockets: S")},
   {"sockets-s-empty-rune",header(raw,"Sockets: S\nRune: None")},
   {"sockets-j",header(raw,"Sockets: J")},
   {"sockets-s-unknown-rune",header(raw,"Sockets: S\nRune: OwnedUnknownRune")},
   {"crafted-true",header(raw,"Crafted: true")},{"crafted-false",header(raw,"Crafted: false")},
   {"affix-prefix-none",header(raw,"Prefix: None")},
   {"advanced-range",replace(raw,line,"{range:0.5}"..line)},
   {"advanced-header",replace(raw,line,"{ Prefix Modifier \"OwnedProbe\" - Fire }\n"..line)},
   {"quality-zero",header(raw,"Quality: 0")},{"quality-twenty",header(raw,"Quality: 20")},
   {"quality-malformed",header(raw,"Quality: Nope")},{"quality-duplicate",header(raw,"Quality: 0\nQuality: 20")},
   {"quality-alias",header(raw,"Quality (Fire Modifiers): 20%")},
   {"catalyst-both",header(raw,"Catalyst: Xoph's\nCatalystQuality: 20")},
   {"tagged-catalyst-absent-amount",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Xoph's")},
   {"tagged-catalyst-explicit-twenty",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Xoph's\nCatalystQuality: 20")},
   {"tagged-catalyst-explicit-zero",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Xoph's\nCatalystQuality: 0")},
   {"catalyst-only",header(raw,"Catalyst: Xoph's")},{"catalyst-quality-only",header(raw,"CatalystQuality: 20")},
   {"unknown-member",raw.."\nOwned unknown Ruby member"},
   {"replacement-base",header(raw,"Emerald")},
   {"reminder-block",replace(raw,line,"(Owned reminder\n"..line.."\n)")},
   {"member-duplicate",raw.."\n"..line},
   {"disabled",replace(raw,line,"{disabled}"..line)},
   {"variant",replace(raw,line,"{variant:2}"..line)},
   {"tagged-fire",replace(raw,line,"{tags:fire}"..line)},
   {"range-member",replace(raw,line,"(10-20)% increased Fire Damage")}
  }
  for _,case in ipairs(cases)do
   local ok,value=pcall(function()
    local probe=construct(case[2],id);local before=snapshot(probe,nil);local after=finish(probe)
    return {name=case[1],raw=case[2],available=true,before=before,after=after}
   end)
   if not ok then value={name=case[1],raw=case[2],available=false,error=tostring(value)}end
   row.probes[#row.probes+1]=value
  end
  -- Reused objects deliberately retain fields not reset by ParseRaw. They are
  -- observations of rejected construction, never input-default authority.
  local seed=header(raw,"Crafted: true\nQuality: 20\nCatalyst: Xoph's\nCatalystQuality: 37\nSockets: S\nRune: None").."\nCorrupted\nMirrored"
  local reused=construct(seed,id);row.reuse_seed=finish(reused)
  local missingIdentity=replace(raw,uniqueLine.."\n","")
  auth.methods.ParseRaw(reused,missingIdentity);row.reparsed_without_identity=finish(reused)
  row.fresh_without_identity=finish(construct(missingIdentity,id))
  auth.methods.ParseRaw(reused,raw);row.reparsed_original=finish(reused)
  row.fresh_again=finish(construct(raw,id))
 end
 assert(equal(loaded,snapshot(item,auth.methods.GetActiveModListForSlotNum(item,1,false))))
 assert(equal(row.delivery.main,delivery(env)) and equal(row.delivery.calcs,delivery(calcEnv)))
 result.items[1]=row
end
assert(build.spec==spec and build.itemsTab.activeItemSetId==selected.items and build.treeTab.activeSpec==selected.spec and build.skillsTab.activeSkillSetId==selected.skills and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
for id,saved in pairs(savedItems)do assert(build.itemsTab.items[id]==saved.object and saved.object.raw==saved.raw)end
assert(build.calcsTab.mainEnv==env and build.calcsTab.calcsEnv==calcEnv and build.calcsTab.mainOutput==output and build.calcsTab.calcsOutput==calcOutput)
assert(equal(outputs.main,output)and equal(outputs.calcs,calcOutput))
for name,f in pairs(auth.methods)do assert(class[name]==f)end
assert(build.itemsTab.Load==auth.load and itemLib.formatValue==auth.format and itemLib.applyRange==auth.range and modLib.parseMod==auth.parser and upvalue(class.ParseRaw,"getCatalystScalar")==auth.catalyst and calcs.initEnv==auth.init and calcs.buildModListForNode==auth.node and calcs.buildModListForNodeList==auth.nodes)
result.saved_items_preserved=true;result.saved_selections_preserved=true;result.main_output_preserved=true;result.calcs_output_preserved=true;result.original_functions_preserved=true
result.catalyst_scalar_source={path="src/Classes/Item.lua",line=debug.getinfo(auth.catalyst,"S").linedefined}
return result
