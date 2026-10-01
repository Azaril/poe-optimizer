-- Read-only observations of original methods. All mutations below target fresh Items.
local function original(f,path,line)
 local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua" and s:sub(-#path)==path and i.linedefined==line,path..":"..line.." actual "..s..":"..tostring(i.linedefined));return f
end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local calcs=require("Modules.CalcBase")
if rubySourcePhase=="before"then
 local constructor=original(class.Item,"Modules/Common.lua",167)
 original(upvalue(constructor,"originalFunc"),"Classes/Item.lua",93)
 local methods={Item=constructor,ParseRaw=original(class.ParseRaw,"Classes/Item.lua",468),NormaliseQuality=original(class.NormaliseQuality,"Classes/Item.lua",1805),BuildModList=original(class.BuildModList,"Classes/Item.lua",2694),GetActiveModListForSlotNum=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198),BuildModListsForSlots=original(class.BuildModListsForSlots,"Classes/Item.lua",2671),BuildModListForSlotNum=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414),CheckModLineVariant=original(class.CheckModLineVariant,"Classes/Item.lua",2289),GetModLineVariantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320)}
 local prior={};for _,item in pairs(build.itemsTab.items)do prior[item]=true end
 _rubySource={methods=methods,prior=prior,load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193),catalyst=original(upvalue(methods.ParseRaw,"getCatalystScalar"),"Classes/Item.lua",32),format=original(itemLib.formatValue,"Modules/ItemTools.lua",45),range=original(itemLib.applyRange,"Modules/ItemTools.lua",130),parser=original(modLib.parseMod,"Modules/ModParser.lua",7404),init=original(calcs.initEnv,"Modules/CalcSetup.lua",717),node=original(calcs.buildModListForNode,"Modules/CalcSetup.lua",200),nodes=original(calcs.buildModListForNodeList,"Modules/CalcSetup.lua",415)}
 return function()_rubySource.finished=true end
end
local auth=assert(_rubySource);assert(auth.finished)
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
local scalarFields={"title","name","baseName","type","rarity","crafted","advancedCopy","itemLevel","quality","corrupted","doubleCorrupted","mirrored","sanctified","desecrated","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
local categories={"buff","enchant","rune","classRequirement","implicit","explicit"}
local function snapshot(item,list)
 local out={field_types={},lists={},base_mods=records(item.baseModList),active=records(list)}
 for _,k in ipairs(scalarFields)do out[k]=item[k];out.field_types[k]=type(item[k])end
 for _,k in ipairs({"prefixes","suffixes","requirements","sockets","runes","variantList","socketedSoulCoreTypes","modMagnitudeMods"})do out[k]=plain(item[k]);out.field_types[k]=type(item[k])end
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
local doc,err=common.xml.ParseXML(rubySourceXml);assert(doc and not err)
local itemsNode,tree;for _,n in ipairs(doc[1])do if type(n)=="table"then if n.elem=="Items"then assert(not itemsNode);itemsNode=n elseif n.elem=="Tree"then assert(not tree);tree=n end end end;assert(itemsNode and tree)
local rawItems,xmlRows={},{};for _,v in ipairs(itemsNode)do if type(v)=="table"and v.elem=="Item"then local text,children={},{};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else children[#children+1]=plain(s)end end;local id=assert(tonumber(v.attrib.id));assert(not rawItems[id]);rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),children=children,text_nodes=#text}end end
local selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local env=build.calcsTab.mainEnv;local calcEnv=build.calcsTab.calcsEnv;local output=build.calcsTab.mainOutput;local calcOutput=build.calcsTab.calcsOutput
local outputs={main=clone(output),calcs=clone(calcOutput)};local savedItems={};for id,item in pairs(build.itemsTab.items)do assert(not auth.prior[item]);savedItems[id]={object=item,raw=item.raw}end
local spec=build.spec;assert(spec==build.treeTab.specList[selected.spec] and env.spec==spec)
local result={selected=selected,items={},numeric_cases={},fresh_loaded_items=true,main_output={}}
for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then result.main_output[k]=v end end
if rubySourceTarget then
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
 if rubySourceControls then
  local line="14% increased Fire Damage"
  local cases={
   {"amount-zero",replace(raw,line,"0% increased Fire Damage"),0},{"amount-one",replace(raw,line,"1% increased Fire Damage"),1},{"amount-high",replace(raw,line,"1000000% increased Fire Damage"),1000000},{"amount-fraction",replace(raw,line,"14.5% increased Fire Damage"),14.5},{"amount-negative",replace(raw,line,"-14% increased Fire Damage"),-14},
   {"untagged-catalyst-twenty",header(raw,"Catalyst: Xoph's\nCatalystQuality: 20"),14},
   {"tagged-fire-catalyst-twenty",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Xoph's\nCatalystQuality: 20"),14},
   {"tagged-fire-wrong-catalyst",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Flesh\nCatalystQuality: 20"),14},
   {"tagged-fire-catalyst-zero",header(replace(raw,line,"{tags:fire}"..line),"Catalyst: Xoph's\nCatalystQuality: 0"),14},
   {"explicit-magnitude",raw.."\n50% increased explicit modifier magnitudes",14},
   {"crafted-explicit-magnitude",header(raw,"Crafted: true").."\n50% increased explicit modifier magnitudes",14},
   {"crafted-implicit-magnitude-contrast",header(raw,"Crafted: true").."\n50% increased implicit modifier magnitudes",14},
   {"crafted-implicit-category",header(replace(raw,"Implicits: 0","Implicits: 1"),"Crafted: true").."\n50% increased explicit modifier magnitudes",14},
   {"implicit-magnitude-contrast",raw.."\n50% increased implicit modifier magnitudes",14},
   {"implicit-category",replace(raw,"Implicits: 0","Implicits: 1").."\n50% increased explicit modifier magnitudes",14},
   {"enchant-category",replace(raw,line,"{enchant}"..line),14},
   {"corrupted-range",replace(raw,line,"{corruptedRange:1.5}"..line),14},
   {"unscalable",header(replace(raw,line,"{unscalable}{tags:fire}"..line),"Catalyst: Xoph's\nCatalystQuality: 20"),14},
   {"disabled",replace(raw,line,"{disabled}"..line),14},{"variant",replace(raw,line,"{variant:2}"..line),14},
   {"fractured",replace(raw,line,"{fractured}"..line),14},{"desecrated",replace(raw,line,"{desecrated}"..line),14},
   {"quality-zero",header(raw,"Quality: 0"),14},{"quality-twenty",header(raw,"Quality: 20"),14},{"quality-malformed",header(raw,"Quality: Nope"),14},{"quality-duplicate",header(raw,"Quality: 0\nQuality: 20"),14},{"quality-alias",header(raw,"Quality (Fire Modifiers): 20%"),14},{"catalyst-quality-only",header(raw,"CatalystQuality: 20"),14},{"unknown-member",raw.."\nOwned unknown Ruby member",14}
  }
  for _,case in ipairs(cases)do
   local probe=construct(case[2],id);local before=snapshot(probe,nil);local after=finish(probe)
   row.probes[#row.probes+1]={name=case[1],raw=case[2],before=before,after=after}
   local found={};for _,category in ipairs(categories)do for _,member in ipairs(after.lists[category])do if member.line:find("increased Fire Damage",1,true)then found[#found+1]=member end end end;assert(#found==1)
   local fire={};for _,record in ipairs(after.active)do if record.name=="FireDamage"and record.type=="INC"then fire[#fire+1]=record end end
   result.numeric_cases[#result.numeric_cases+1]={name=case[1],raw_amount=case[3],category=found[1].category,ordinal=found[1].ordinal,properties=found[1].modTags,unscalable=not not found[1].unscalable,corrupted_factor=found[1].corruptedRange or 1,magnitude_factor=found[1].valueScalar or 1,catalyst=probe.catalyst,catalyst_quality=probe.catalystQuality,catalyst_factor=found[1].catalyst_factor,formatted=found[1].formatted,parsed=found[1].records,active=fire,extra=found[1].extra}
  end
  local reused=construct(header(raw,"Quality: 20\nCatalyst: Xoph's\nCatalystQuality: 20").."\nCorrupted",id);finish(reused);auth.methods.ParseRaw(reused,raw);row.reparsed=finish(reused)
  local again=construct(raw,id);row.fresh_after_reparse=finish(again)
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
result.scalability=plain(data.modScalability["#% increased Fire Damage"])
return result
