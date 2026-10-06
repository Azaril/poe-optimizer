pub const OBSERVE: &str = r##"
local function original(f,path,first,last)local i=debug.getinfo(f,"S");local s=i.source:gsub("\\","/");assert(i.what=="Lua"and s:sub(-#path)==path and i.linedefined==first);if last then assert(i.lastlinedefined==last)end;return f end
local function upvalue(f,name)for i=1,64 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local parse=original(class.ParseRaw,"Classes/Item.lua",468,1803)
local mods=original(class.BuildModList,"Classes/Item.lua",2694,2863)
local active=original(class.GetActiveModListForSlotNum,"Classes/Item.lua",2198,2220)
local catalyst=original(upvalue(parse,"getCatalystScalar"),"Classes/Item.lua",32,63)
local load=original(build.itemsTab.Load,"Classes/ItemsTab.lua",1193,1320)
local slotLists=original(class.BuildModListsForSlots,"Classes/Item.lua",2671,2680)
local slotList=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local variantCheck=original(class.CheckModLineVariant,"Classes/Item.lua",2289,2318)
local variantCount=original(class.GetModLineVariantCount,"Classes/Item.lua",2320,2336)
local function plain(v,d)if type(v)~="table"then assert(type(v)~="function"and type(v)~="userdata");return v end;d=(d or 0)+1;assert(d<10);local o={};local n=0;for k,x in pairs(v)do n=n+1;assert(n<512);o[k]=plain(x,d)end;return o end
local function equal(a,b)if type(a)~=type(b)then return false end;if type(a)~="table"then return a==b end;for k,v in pairs(a)do if not equal(v,b[k])then return false end end;for k in pairs(b)do if a[k]==nil then return false end end;return true end
local function records(list)local out={};for _,m in ipairs(list or {})do local tags={};for _,t in ipairs(m)do tags[#tags+1]=plain(t)end;out[#out+1]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}end;return out end
local scalarFields={"title","name","baseName","type","rarity","crafted","itemLevel","quality","corrupted","mirrored","split","catalyst","catalystQuality","itemSocketCount","jewelSocketCount","variant","variantAlt","hasAltVariant","allowDuplicateVariants","classRestriction","socketedAugmentTypeOverride","socketedIdolsUseBondedModifiers","socketedSoulCoreEffectModifier","socketedRuneEffectModifier","socketedAugmentItemEffectModifier"}
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
local doc,err=common.xml.ParseXML(simpleItemXml);assert(doc and not err);local node;for _,v in ipairs(doc[1])do if type(v)=="table"and v.elem=="Items"then assert(not node);node=v end end;assert(node)
local rawItems={};local xmlRows={};for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local text={};local childNames={};for _,s in ipairs(v)do if type(s)=="string"then text[#text+1]=s else childNames[#childNames+1]={name=s.elem,attributes=plain(s.attrib)}end end;local id=assert(tonumber(v.attrib.id));rawItems[id]=table.concat(text,"\n");xmlRows[id]={attributes=plain(v.attrib),text_nodes=#text,children=childNames}end end
local env=build.calcsTab.mainEnv;local output=build.calcsTab.mainOutput;local outputValues={};for k,v in pairs(output)do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then outputValues[k]=v end end
local saved={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup}
local result={selected_items=saved.items,selected_spec=saved.spec,selected_skills=saved.skills,selected_config=saved.config,items={},probes={}}
local originals,snapshots={},{}
local function replace(text,old,new)local start=assert(text:find(old,1,true));assert(not text:find(old,start+#old,true));return text:sub(1,start-1)..new..text:sub(start+#old)end
local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
for _,id in ipairs({21,22})do
 local item=assert(build.itemsTab.items[id]);originals[id]=item
 local loaded=snapshot(item,item.modList or item.slotModList[1]);snapshots[id]=loaded
 local raw=assert(rawItems[id]);local fresh=construct(raw,id);local before=snapshot(fresh,nil);mods(fresh)
 local selected={};for slot,v in pairs(env.player.itemList)do if v==item then selected[#selected+1]=slot end end;table.sort(selected)
 local requirements={};for _,row in ipairs(env.requirementsTableItems)do if row.sourceItem==item then requirements[#requirements+1]={source=row.source,slot=row.sourceSlot,Str=row.Str,Dex=row.Dex,Int=row.Int}end end
 local slots={};for slot,list in pairs(item.slotModList or {})do slots[#slots+1]={slot=slot,kind="indexed",records=records(list),active=records(active(item,slot,false))}end;table.sort(slots,function(a,b)return a.slot<b.slot end)
 if item.modList then assert(#slots==0);slots[1]={kind="shared-mod-list",records=records(item.modList),active=records(active(item,1,false))}end
 local row={id=id,raw=raw,xml=xmlRows[id],loaded=loaded,fresh_before_build=before,fresh=snapshot(fresh,active(fresh,1,false)),selected_slots=selected,requirements_rows=requirements,slot_lists=slots,probes={},base_facts={name=item.baseName,type=item.base.type,subtype=item.base.subType,implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),armour=plain(item.base.armour),requirements=plain(item.base.req),quality=item.base.quality,socket_limit=item.base.socketLimit,flask=plain(item.base.flask),charm=plain(item.base.charm)}}
 if simpleItemControls then
  local needle=id==21 and "+1 to Level of all Minion Skills"or"10% increased Movement Speed"
  local req=id==21 and"LevelReq: 5"or"LevelReq: 80"
  local tag=id==21 and"minion"or"speed";local kind=id==21 and"Necrotic"or"Skittering"
  local cases={
   {"second-member",raw.."\n+3 to maximum Life"},{"unknown-member",raw.."\nOwned witness unknown semantic line"},
   {"header-implicit",replace(raw,"Implicits: 0","Implicits: 1")},
   {"enchant-member",raw.."\n{enchant}+3 to maximum Mana"},
   {"unknown-rune",raw:gsub("Rune: None","Rune: OwnedWitnessUnknownRune",1)},
   {"legacy-sockets",raw:gsub("Rune: None[\r\n]+","")},
   {"level-high",replace(raw,req,"LevelReq: 77")},{"level-absent",replace(raw,req.."\n","")},
   {"quality-zero",replace(raw,"Quality: 20","Quality: 0")},
   {"corrupted",raw.."\nCorrupted"},
   {"catalyst-kind",header(raw,"Catalyst: "..kind)},
   {"catalyst-zero",header(raw,"Catalyst: "..kind.."\nCatalystQuality: 0")},
   {"catalyst-amount-only",header(raw,"CatalystQuality: 37")},
   {"catalyst-matching-tag",header(replace(raw,needle,"{tags:"..tag.."}"..needle),"Catalyst: "..kind)},
   {"catalyst-wrong-tag",header(replace(raw,needle,"{tags:mana}"..needle),"Catalyst: "..kind)}}
  local broad,specific=fresh:GetSocketedAugmentTypes();local names={};for name,rune in pairs(data.itemMods.Runes)do if(rune[broad]and #rune[broad]>0)or(rune[specific]and #rune[specific]>0)then names[#names+1]=name end end;table.sort(names)
  cases[#cases+1]={"occupied-rune",raw:gsub("Rune: None","Rune: "..assert(names[1]),1)}
  if id==21 then
   cases[#cases+1]={"class-restricted",raw.."\nRequires Class Witch"}
   cases[#cases+1]={"different-base",replace(raw,"Iron Crown","Horned Crown")}
   local buffBases={};for name,base in pairs(data.itemBases)do if base.charm and base.charm.buff and #base.charm.buff>0 then buffBases[#buffBases+1]=name end end;table.sort(buffBases)
   cases[#cases+1]={"buff-base","Rarity: NORMAL\n"..assert(buffBases[1])}
  end
  for _,case in ipairs(cases)do local probe=construct(case[2],id);local initial=snapshot(probe,nil);mods(probe);local after=snapshot(probe,active(probe,1,false));assert(equal(after,snapshot(probe,active(probe,1,false))));row.probes[#row.probes+1]={name=case[1],raw=case[2],before=initial,after=after}end
  local reused=construct(header(raw,"Item Level: 77").."\nCorrupted",id);mods(reused);parse(reused,raw);mods(reused);row.reparsed=snapshot(reused,active(reused,1,false))
  local again=construct(raw,id);mods(again);row.fresh_after_reparse=snapshot(again,active(again,1,false))
 end
 result.items[#result.items+1]=row
end
for _,id in ipairs({21,22})do assert(build.itemsTab.items[id]==originals[id]);assert(equal(snapshot(originals[id],originals[id].modList or originals[id].slotModList[1]),snapshots[id]))end;result.saved_items_preserved=true
assert(build.itemsTab.activeItemSetId==saved.items and build.treeTab.activeSpec==saved.spec and build.skillsTab.activeSkillSetId==saved.skills and build.configTab.activeConfigSetId==saved.config and build.mainSocketGroup==saved.group);result.saved_selections_preserved=true
assert(build.calcsTab.mainEnv==env and build.calcsTab.mainOutput==output);for k,v in pairs(outputValues)do assert(output[k]==v)end;result.main_output_preserved=true;result.main_output=outputValues
assert(class.ParseRaw==parse and class.BuildModList==mods and class.GetActiveModListForSlotNum==active and build.itemsTab.Load==load and class.BuildModListsForSlots==slotLists and class.BuildModListForSlotNum==slotList and class.CheckModLineVariant==variantCheck and class.GetModLineVariantCount==variantCount and upvalue(parse,"getCatalystScalar")==catalyst);result.original_functions_preserved=true
return result
"##;

// Opt-in original-call witness; historical OBSERVE stays byte-identical.
pub const LOCAL_DEFENCE_OBSERVER: &str = r##"
local calcs=require("Modules.CalcBase")
local function original(f,path,line)local i=debug.getinfo(assert(f),"S");assert(i.what=="Lua"and i.source:gsub("\\","/"):sub(-#path)==path and i.linedefined==line);return f,{path=path,first=i.linedefined,last=i.lastlinedefined}end
local function upvalue(f,name)for i=1,100 do local n,v=debug.getupvalue(f,i);if not n then break end;if n==name then return v end end;error("missing upvalue "..name)end
local class=common.classes.Item
local slot,sl=original(class.BuildModListForSlotNum,"Classes/Item.lua",2414)
local mods,ml=original(class.BuildModList,"Classes/Item.lua",2694)
local parse,pr=original(class.ParseRaw,"Classes/Item.lua",468)
local normalise,nr=original(class.NormaliseQuality,"Classes/Item.lua",1805)
local localCalc,lc=original(upvalue(slot,"calcLocal"),"Classes/Item.lua",2384)
local getter,gr=original(class.GetArmourDataValue,"Classes/Item.lua",2373)
local list,lr=original(common.classes.ModStore.List,"Classes/ModStore.lua",321)
local defence,dr=original(calcs.defence,"Modules/CalcDefence.lua",789)
local callback,cr=original(runCallback,"HeadlessWrapper.lua",17)
local roundOriginal,rr=original(round,"Modules/Common.lua",722)
local names={[21]="Iron Crown",[22]="Cryptic Leggings"}
local slots={[21]="Helmet",[22]="Boots"}
local function target(i)return i and names[i.id]and i.baseName==names[i.id]end
local function plain(v,d)if v==nil then return {kind="absent"}end;if type(v)~="table"then assert(type(v)=="number"or type(v)=="string"or type(v)=="boolean");return v end;d=(d or 0)+1;assert(d<12);local r,n={},0;for k,x in pairs(v)do n=n+1;assert(n<8192);r[k]=plain(x,d)end;return r end
local function records(t)local r={};for i,m in ipairs(t or{})do assert(i<4096);local tags={};for j,x in ipairs(m)do tags[j]=plain(x)end;r[i]={name=m.name,type=m.type,value=plain(m.value),flags=m.flags,keyword_flags=m.keywordFlags,source=m.source,source_slot=m.sourceSlot,tags=tags}end;return r end
local function fields(t)local r={};for k,v in pairs(t or{})do if type(k)=="string"and(type(v)=="number"or type(v)=="string"or type(v)=="boolean")and(type(v)~="number"or(v==v and v~=math.huge and v~=-math.huge))then r[k]=v end end;return r end
local function itemState(i)return {id=i.id,source=i.modSource,base_name=i.baseName,quality=plain(i.quality),crafted_quality=plain(i.craftedQuality),armour=plain(i.armourData),base=plain(i.base),base_records=records(i.baseModList),active_records=records(i.modList),exact_registered=build.itemsTab.items[i.id]==i,exact_catalogue_base=data.itemBases[names[i.id]]==i.base}end
local function preserved()assert(class.BuildModListForSlotNum==slot and class.BuildModList==mods and class.ParseRaw==parse and class.NormaliseQuality==normalise and class.GetArmourDataValue==getter and common.classes.ModStore.List==list and calcs.defence==defence and runCallback==callback and upvalue(slot,"calcLocal")==localCalc and round==roundOriginal)end
local api={}
function api.install()
 assert(not debug.gethook()and jit.status()==armourDefenceJit);preserved()
 local trace={locals={},assemblies={},overrides={},normalisations={},consumers={},rounds={}}
 local localFrames,normalFrames,getFrames={},{},{}
 local pending,actors,order={},{},{}
 local bindings={}
 local function bind(row,item)bindings[#bindings+1]={row=row,item=item};return row end
 local sequence,failure=0,nil
 local function actorRow(env,actor)
  local r=actors[actor];if not r then r={env=env,actor=actor,reads={}};actors[actor]=r;order[#order+1]=actor end;assert(r.env==env);return r
 end
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  if event=="line"then if not((f==slot and line==2558)or(f==defence and(line==818 or line==827)))then return end
  elseif f~=localCalc and f~=normalise and f~=list and f~=getter and f~=defence and f~=roundOriginal then return end
  local v={};for i=1,192 do local n,x=debug.getlocal(2,i);if not n then break end;v[n]=x end
  local caller=debug.getinfo(3,"fl");local cv={}
  if f==localCalc or f==list or f==getter or f==roundOriginal then for i=1,192 do local n,x=debug.getlocal(3,i);if not n then break end;cv[n]=x end end
  local ok,err=xpcall(function()
   sequence=sequence+1;assert(sequence<1000000)
   if f==localCalc and caller.func==slot and target(cv.self)and(caller.currentline==2443 or(caller.currentline>=2523 and caller.currentline<=2542))then
    if event=="call"then localFrames[#localFrames+1]={list=v.modList,item=cv.self,row={item_id=cv.self.id,source=cv.self.modSource,call_line=caller.currentline,name=v.name,type=v.type,flags=v.flags,before=records(v.modList),sequence=sequence,original_call=true}}
    elseif event=="return"then local frame=assert(table.remove(localFrames));assert(frame.list==v.modList and frame.item==cv.self);frame.row.after=records(v.modList);frame.row.result=plain(v.result);frame.row.original_return=true;trace.locals[#trace.locals+1]=bind(frame.row,frame.item) end
   elseif f==roundOriginal and event=="call"and caller.func==slot and(caller.currentline==2547 or caller.currentline==2551)and target(cv.self)then
    trace.rounds[#trace.rounds+1]=bind({item_id=cv.self.id,source=cv.self.modSource,call_line=caller.currentline,value=v.val,decimal_places=plain(v.dec),original_call=true},cv.self)
   elseif f==slot and event=="line"and target(v.self)then
    local operands={};for _,k in ipairs({"armourBase","armourEvasionBase","evasionBase","evasionEnergyShieldBase","energyShieldBase","armourEnergyShieldBase","wardBase","evasionPerLevel","energyShieldPerLevel","wardPerLevel","armourInc","armourEvasionInc","evasionInc","evasionEnergyShieldInc","energyShieldInc","wardInc","armourEnergyShieldInc","defencesInc","qualityScalar","craftedQuality"})do assert(v[k]~=nil,"missing original local "..k);operands[k]=v[k]end
    trace.assemblies[#trace.assemblies+1]=bind({item_id=v.self.id,source=v.self.modSource,slot=v.slotName,quality=plain(v.self.quality),operands=operands,preoverride=plain(v.armourData),remaining_records=records(v.modList),sequence=sequence,original_assignment_observed=true},v.self)
   elseif f==list and event=="return"and caller.func==slot and caller.currentline==2564 and target(cv.self)then
    assert(v.self==cv.modList);trace.overrides[#trace.overrides+1]=bind({item_id=cv.self.id,result=plain(v.result),original_return=true,exact_local_store=true},cv.self)
   elseif f==normalise and target(v.self)then
    if event=="call"then normalFrames[#normalFrames+1]={item=v.self,before=plain(v.self.quality)}
    elseif event=="return"then local frame=assert(table.remove(normalFrames));assert(frame.item==v.self);trace.normalisations[#trace.normalisations+1]=bind({item_id=v.self.id,before=frame.before,after=plain(v.self.quality),default_quality=main.defaultItemQuality,original_return=true},v.self)end
   elseif f==getter and caller.func==defence and(caller.currentline==817 or caller.currentline==826)and target(v.self)and cv.actor==cv.env.player then
    assert(cv.item==v.self and cv.actor.itemList[slots[v.self.id]]==v.self)
    local r=actorRow(cv.env,cv.actor)
    if event=="call"then getFrames[#getFrames+1]={owner=r,item=v.self,name=v.name,row={item_id=v.self.id,name=v.name,level=v.level,call_line=caller.currentline,stored_armour=plain(v.self.armourData),exact_selected_item=true,original_call=true}}
    elseif event=="return"then local frame=assert(table.remove(getFrames));assert(frame.owner==r and frame.item==v.self and frame.name==v.name);frame.row.original_return=true;pending[cv.actor]=pending[cv.actor]or{};assert(not pending[cv.actor][v.name]);pending[cv.actor][v.name]=frame end
   elseif f==defence and event=="line"and target(v.item)and v.actor==v.env.player then
    local name=line==818 and"EnergyShield"or"Armour";local frame=assert(pending[v.actor]and pending[v.actor][name]);pending[v.actor][name]=nil
    assert(frame.item==v.item and frame.owner==actors[v.actor]);local value=line==818 and v.energyShieldBase or v.armourBase;assert(type(value)=="number")
    frame.row.returned_value=value;frame.row.original_consumer_line=line;frame.row.exact_return_to_consumer=true;frame.owner.reads[#frame.owner.reads+1]=bind(frame.row,frame.item)
   elseif f==defence and event=="return"and v.actor==v.env.player and actors[v.actor]then
    local r=actors[v.actor];r.returned=true;r.outputs={};for _,id in ipairs({21,22})do local s=slots[id];r.outputs[#r.outputs+1]={item_id=id,slot=s,Armour=plain(v.actor.output["ArmourOn"..s]),EnergyShield=plain(v.actor.output["EnergyShieldOn"..s])}end
   end
  end,debug.traceback)
  if not ok then failure=tostring(err);error(failure,0)end
 end
 if armourDefenceInstrumented then jit.flush();debug.sethook(hook,"crl")end
 return function()
  if armourDefenceInstrumented then assert(debug.gethook()==hook);debug.sethook()end
  assert(not failure,failure);assert(#localFrames==0 and #normalFrames==0 and #getFrames==0);for _,v in pairs(pending)do assert(next(v)==nil)end;preserved();assert(jit.status()==armourDefenceJit)
  for _,actor in ipairs(order)do local r=actors[actor];assert(r.returned);trace.consumers[#trace.consumers+1]={mode=r.env.mode,exact_current_environment=r.env==build.calcsTab.mainEnv or r.env==build.calcsTab.calcsEnv,reads=r.reads,outputs=r.outputs,original_defence_return=true}end
  -- Compare actual objects only after the final loaded selection is known. Warm-load
  -- objects and constructor probes remain in the report as non-current diagnostics.
  for _,binding in ipairs(bindings)do local row,item=binding.row,binding.item;local s=assert(slots[item.id])
   row.exact_registered_item=build.itemsTab.items[item.id]==item
   row.exact_main_item=build.calcsTab.mainEnv.player.itemList[s]==item
   row.exact_calcs_item=build.calcsTab.calcsEnv.player.itemList[s]==item
   row.exact_current_item=row.exact_registered_item and row.exact_main_item and row.exact_calcs_item
  end
  api.trace=trace
 end
end
function api.observe()
 assert(not debug.gethook());preserved();local items={}
 for _,id in ipairs({21,22})do local item=assert(build.itemsTab.items[id]);assert(target(item));local s=itemState(item);s.members={}
  for _,category in ipairs({"buff","implicit","explicit","enchant","rune","classRequirement"})do local lines={};for _,line in ipairs(item[category.."ModLines"]or{})do lines[#lines+1]={line=line.line,records=records(line.modList),unparsed=plain(line.extra)}end;s.members[category]=lines end;items[#items+1]=s
 end
 local envs={};for _,named in ipairs({{"MAIN",build.calcsTab.mainEnv},{"CALCS",build.calcsTab.calcsEnv}})do local env=assert(named[2]);local outputs={}
  for _,id in ipairs({21,22})do local s=slots[id];assert(env.player.itemList[s]==build.itemsTab.items[id]);outputs[#outputs+1]={item_id=id,slot=s,Armour=plain(env.player.output["ArmourOn"..s]),EnergyShield=plain(env.player.output["EnergyShieldOn"..s])}end
  envs[#envs+1]={mode=named[1],slot_outputs=outputs,player_outputs=fields(env.player.output),exact_selected_items=true}
 end
 return {items=items,environments=envs,trace=api.trace,selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId},methods={slot=sl,mods=ml,parse=pr,normalise=nr,local_calculation=lc,getter=gr,list=lr,defence=dr,callback=cr,round=rr},original_functions_preserved=true,method_wrappers=false}
end
function api.rebuild()
 local cleanup=api.install();local revision=build.outputRevision;local mainEnv,calcsEnv=build.calcsTab.mainEnv,build.calcsTab.calcsEnv
 local ok,err=pcall(function()build.buildFlag=true;callback("OnFrame")end);local removed,removeErr=pcall(cleanup)
 if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 assert(build.buildFlag==false and build.outputRevision==revision+1 and build.calcsTab.mainEnv~=mainEnv and build.calcsTab.calcsEnv~=calcsEnv)
end
function api.constructor_probes()
 assert(not debug.gethook());local saved=api.observe();local priorTrace=api.trace;local cleanup=api.install();local result={}
 local ok,err=pcall(function()
  local doc,e=common.xml.ParseXML(armourDefenceXml);assert(doc and not e);local raw={}
  for _,node in ipairs(doc[1])do if type(node)=="table"and node.elem=="Items"then for _,v in ipairs(node)do if type(v)=="table"and v.elem=="Item"then local id=tonumber(v.attrib.id);if names[id]then local parts={};for _,text in ipairs(v)do if type(text)=="string"then parts[#parts+1]=text end end;raw[id]=table.concat(parts,"\n")end end end end end
  for _,id in ipairs({21,22})do local direct=new("Item"):Item("");direct.id=id;parse(direct,assert(raw[id]));local before=plain(direct.quality);mods(direct)
   local normalized=new("Item"):Item("");normalized.id=id;parse(normalized,raw[id]);normalise(normalized);mods(normalized)
   result[#result+1]={item_id=id,raw_quality=before,direct=itemState(direct),explicit_normalise=itemState(normalized),default_quality=main.defaultItemQuality,explicit_constructor_controls=true}
  end
 end)
 local removed,removeErr=pcall(cleanup);if not ok then error(err,0)end;if not removed then error(removeErr,0)end
 local trace=api.trace;api.trace=priorTrace;local after=api.observe();for i,x in ipairs(saved.items)do assert(build.itemsTab.items[x.id].quality==after.items[i].quality)end
 return {items=result,trace=trace,selected_items_preserved=true}
end
return api
"##;
