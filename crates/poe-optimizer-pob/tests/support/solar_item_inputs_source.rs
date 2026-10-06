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

// Opt-in original-call evidence; the historical OBSERVE bytes remain unchanged.
pub const OWNERSHIP_OBSERVER: &str = r##"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)
 local i=debug.getinfo(assert(f),"S");local s=i.source:gsub("\\","/")
 assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==line)
 return f,{path=path,first=i.linedefined,last=i.lastlinedefined}
end
local function upvalue(f,name)
 for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end
 error("missing original upvalue "..name)
end
local class=common.classes.Item
local slot,sl=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local mods,ml=original(class.BuildModList,"Classes/Item.lua",2694)
local localCalc,lc=original(upvalue(slot,"calcLocal"),"Classes/Item.lua",2384)
assert(upvalue(mods,"calcLocal")==localCalc)
local init,ia=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local perform,pa=original(calcs.perform,"Modules/CalcPerform.lua",1193)
local active,aa=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198)
local callback,ca=original(runCallback,"HeadlessWrapper.lua",17)
local function plain(v,depth)
 if v==nil then return {kind="absent"}end
 if type(v)~="table"then assert(type(v)=="number"or type(v)=="string"or type(v)=="boolean");return v end
 depth=(depth or 0)+1;assert(depth<12);local out,n={},0
 for k,x in pairs(v)do n=n+1;assert(n<8192);assert(type(k)=="number"or type(k)=="string");out[k]=plain(x,depth)end;return out
end
local function fields(t)
 local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="number"or type(v)=="string"or type(v)=="boolean")then
  if type(v)~="number"or(v==v and v~=math.huge and v~=-math.huge)then r[k]=v end
 end end;return r
end
local function records(list)
 local out={};for i,m in ipairs(list or{})do assert(i<4096);local tags={};for j,t in ipairs(m)do tags[j]=plain(t)end
 out[i]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}
 end;return out
end
local function target(item)return item and item.id==23 and item.baseName=="Solar Amulet"end
local function itemState(item)
 return {id=item.id,source=item.modSource,base_name=item.baseName,quality=plain(item.quality),crafted_quality=plain(item.craftedQuality),
  requirements=plain(item.requirements),rarity=item.rarity,corrupted=plain(item.corrupted),socket_count=item.itemSocketCount,
  catalyst=plain(item.catalyst),catalyst_quality=plain(item.catalystQuality),granted_skills=plain(item.grantedSkills),
  base_modifiers=records(item.baseModList),active_modifiers=records(item.modList),
  exact_registered=build.itemsTab.items[23]==item,exact_catalogue_base=data.itemBases["Solar Amulet"]==item.base}
end
local function rowState(row,item)
 return {source=row.source,slot=row.sourceSlot,Str=row.Str,Dex=row.Dex,Int=row.Int,exact_source_item=row.sourceItem==item}
end
local function assertOriginals()
 assert(class.BuildModListForSlotNum==slot and class.BuildModList==mods and class.GetActiveModListForSlotNum==active)
 assert(upvalue(slot,"calcLocal")==localCalc and upvalue(mods,"calcLocal")==localCalc)
 assert(calcs.initEnv==init and calcs.perform==perform and runCallback==callback)
end
local api={}
function api.install()
 assert(not debug.gethook()and jit.status()==solarOwnershipJit);assertOriginals()
 local trace={local_calls={},item_builds={},slot_builds={},environments={}}
 local frames,envs,order={},{},{}
 local function environment(env)
  if not envs[env]then envs[env]={env=env,consumers={},positive_branches={},item_returns={}};order[#order+1]=env end;return envs[env]
 end
 local failure,events=nil,0
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  if event=="line"then if f~=perform or(line~=1906 and line~=1914)then return end
  elseif f~=slot and f~=mods and f~=localCalc and f~=init and f~=perform and f~=active then return end
  local v={};for i=1,192 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==localCalc or f==active then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
   events=events+1;assert(events<1000000)
   if f==localCalc and target(cv.self)and((caller.func==slot and caller.currentline==2443)or(caller.func==mods and caller.currentline>=2831 and caller.currentline<=2848))then
    if event=="call"then
     local row={caller_line=caller.currentline,name=v.name,type=v.type,flags=v.flags,before=records(v.modList),item_before=itemState(cv.self),original_call=true}
     frames[#frames+1]={row=row,list=v.modList,item=cv.self}
    elseif event=="return"then
     local frame=assert(table.remove(frames));assert(frame.list==v.modList and frame.item==cv.self)
     frame.row.result=plain(v.result);frame.row.after=records(v.modList);frame.row.return_observed=true;trace.local_calls[#trace.local_calls+1]=frame.row
    end
   elseif f==slot and event=="return"and target(v.self)then
    trace.slot_builds[#trace.slot_builds+1]={slot=v.slotName,item=itemState(v.self),local_crafted_quality=v.craftedQuality,
     return_expression_modifiers=records(v.modList),original_return_expression=true}
   elseif f==mods and event=="return"and target(v.self)then trace.item_builds[#trace.item_builds+1]={item=itemState(v.self),original_return=true}
   elseif f==active and event=="return"and caller.func==init and caller.currentline==1322 and target(v.self)then
    local r=environment(cv.env);assert(cv.env.player.itemList.Amulet==v.self)
    local returned=v.self.modList or v.self.slotModList[v.slotNum]
    r.item_returns[#r.item_returns+1]={records=records(returned),exact_selected_item=true,original_return_expression=true}
   elseif f==init and event=="return"and target(v.env.player.itemList.Amulet)then
    local r=environment(v.env);local item=v.env.player.itemList.Amulet;r.item=item;r.deliveries={}
    local callerInfo=debug.getinfo(caller.func,"S");r.initializer_caller={source=callerInfo.source:gsub("\\","/"),first=callerInfo.linedefined,line=caller.currentline}
    for _,row in ipairs(v.env.requirementsTableItems)do if row.sourceItem==item then
     local index;for i,joined in ipairs(v.env.requirementsTable)do if joined==row then assert(not index);index=i end end;assert(index)
     r.deliveries[#r.deliveries+1]={row=rowState(row,item),merged_index=index,exact_merged_row=true};r.row=row
    end end;assert(#r.deliveries==1);r.initialized=true
   elseif f==perform and event=="line"and v.reqSource and target(v.reqSource.sourceItem)then
    local r=environment(v.env);local item=v.env.player.itemList.Amulet;assert(item==v.reqSource.sourceItem and r.row==v.reqSource and r.initialized)
    if line==1906 then
     r.consumers[#r.consumers+1]={attribute=v.attr,input=plain(v.reqSource[v.attr]),row=rowState(v.reqSource,item),exact_delivered_row=true,
      original_predicate_line=1906,output_before=plain(v.output[v.attr.."RequirementsOnAmulet"])}
    else r.positive_branches[#r.positive_branches+1]={attribute=v.attr,computed=v.req,original_assignment_line=1914}end
   elseif f==perform and event=="return"and target(v.env.player.itemList.Amulet)then environment(v.env).performed=true end
  end,debug.traceback)
  if not ok then failure=tostring(err);error(failure,0)end
 end
 if solarOwnershipInstrumented then jit.flush();debug.sethook(hook,"crl")end
 return function()
  if solarOwnershipInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not failure,failure);assert(#frames==0);assertOriginals();assert(jit.status()==solarOwnershipJit)
  for _,env in ipairs(order)do local r=envs[env];if r.initialized then
   local current=env==build.calcsTab.mainEnv or env==build.calcsTab.calcsEnv
   if current then assert(r.performed,"current selected environment lacks original perform return")end
   trace.environments[#trace.environments+1]={mode=env.mode,item=itemState(r.item),deliveries=r.deliveries,
    consumers=r.consumers,positive_branches=r.positive_branches,item_returns=r.item_returns,initializer_caller=r.initializer_caller,
    exact_current_environment=current,performed=r.performed==true}
  end end;api.trace=trace
 end
end
function api.observe()
 assert(not debug.gethook());assertOriginals();local item=assert(build.itemsTab.items[23]);assert(target(item))
 local selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
 local members={};for _,category in ipairs({"buff","enchant","rune","classRequirement","implicit","explicit"})do
  local lines={};for _,line in ipairs(item[category.."ModLines"]or{})do lines[#lines+1]={line=line.line,records=records(line.modList)}end;members[category]=lines
 end
 local envs={};for _,named in ipairs({{"MAIN",build.calcsTab.mainEnv},{"CALCS",build.calcsTab.calcsEnv}})do
  local env=assert(named[2]);assert(env.player.itemList.Amulet==item);local req={}
  for _,row in ipairs(env.requirementsTableItems)do if row.sourceItem==item then req[#req+1]=rowState(row,item)end end
  envs[#envs+1]={mode=named[1],selected_item_exact=true,requirements_rows=req,outputs=fields(env.player.output),
   amulet_requirement_outputs={Str=plain(env.player.output.StrRequirementsOnAmulet),Dex=plain(env.player.output.DexRequirementsOnAmulet),Int=plain(env.player.output.IntRequirementsOnAmulet)}}
 end
 return {methods={slot=sl,item_build=ml,local_calculation=lc,initialization=ia,performance=pa,active=aa,callback=ca},
  selected=selected,base=plain(item.base),base_catalogue_identity=item.base==data.itemBases["Solar Amulet"],item=itemState(item),members=members,
  environments=envs,trace=api.trace,original_functions_preserved=true,method_wrappers=false,observer_installed=false}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision;local main,calc=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=main and build.calcsTab.calcsEnv~=calc)
end
return api
"##;
