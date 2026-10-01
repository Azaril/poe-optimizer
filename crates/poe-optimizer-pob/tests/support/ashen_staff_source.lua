
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
local isZeroValueLine=original(itemLib.isZeroValueLine,"Modules/ItemTools.lua",72,74)
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
local calcs=require("Modules.CalcBase")
local init=original(calcs.initEnv,"Modules/CalcSetup.lua",717)
local loadSkill=original(build.skillsTab.LoadSkill,"Classes/SkillsTab.lua",303)
local processGroup=original(build.skillsTab.ProcessSocketGroup,"Classes/SkillsTab.lua",1242)
local createActive=original(calcs.createActiveSkill,"Modules/CalcActiveSkill.lua",144)
local outputMethod=original(calcs.buildOutput,"Modules/Calcs.lua",469)
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then r[k]=v end end;return r end
local function replace(text,old,new)local start=assert(text:find(old,1,true),old);assert(not text:find(old,start+#old,true));return text:sub(1,start-1)..new..text:sub(start+#old)end
local function header(text,value)return replace(text,"Crafted: true",value.."\nCrafted: true")end
local doc,err=common.xml.ParseXML(ashenXml);assert(doc and not err)
local itemsNode,skillsNode;for _,n in ipairs(doc[1])do if type(n)=="table"then if n.elem=="Items"then assert(not itemsNode);itemsNode=n elseif n.elem=="Skills"then assert(not skillsNode);skillsNode=n end end end;assert(itemsNode and skillsNode)
local raw,xmlItem;for _,n in ipairs(itemsNode)do if type(n)=="table"and n.elem=="Item"and n.attrib.id=="28"then assert(not raw);local text,children={},{};for _,v in ipairs(n)do if type(v)=="string"then text[#text+1]=v else children[#children+1]={name=v.elem,attributes=plain(v.attrib)}end end;raw=table.concat(text,"\n");xmlItem={attributes=plain(n.attrib),children=children,text_nodes=#text}end end
local envs={MAIN=assert(build.calcsTab.mainEnv),CALCS=assert(build.calcsTab.calcsEnv)}
local outputs={MAIN=assert(build.calcsTab.mainOutput),CALCS=assert(build.calcsTab.calcsOutput)}
local outputSnapshot={MAIN=scalars(outputs.MAIN),CALCS=scalars(outputs.CALCS)}
local selected={items=build.itemsTab.activeItemSetId,spec=build.treeTab.activeSpec,skills=build.skillsTab.activeSkillSetId,config=build.configTab.activeConfigSetId,group=build.mainSocketGroup,second_weapon_set=build.itemsTab.activeItemSet.useSecondWeaponSet}
local item=build.itemsTab.items[28]
local result={selection=selected,item_available=item~=nil,saved_groups={},runtime_groups={},granted={MAIN={},CALCS={}},actions={MAIN={},CALCS={}},outputs=outputSnapshot,spell_flag=ModFlag.Spell}
for _,set in ipairs(skillsNode)do if type(set)=="table"and set.elem=="SkillSet"then for _,g in ipairs(set)do if type(g)=="table"and g.elem=="Skill"and (g.attrib.source or""):match("^Item:28:")then local gems={};for _,v in ipairs(g)do if type(v)=="table"and v.elem=="Gem"then gems[#gems+1]=plain(v.attrib)end end;result.saved_groups[#result.saved_groups+1]={set=tonumber(set.attrib.id),attributes=plain(g.attrib),gems=gems}end end end end
local function groupSnapshot(g,sid,index)
 local gems={};for _,v in ipairs(g.gemList)do gems[#gems+1]={fields=scalars(v),gem_definition=v.gemData and v.gemData.id,effect=v.grantedEffect and v.grantedEffect.id,gem_effect=v.gemData and v.gemData.grantedEffectId,from_item_type=type(v.fromItem),stat_set=plain(v.statSet),stat_set_calcs=plain(v.statSetCalcs)}end
 return{set=sid,index=index,selected=sid==selected.skills,source=g.source,slot=g.slot,enabled=g.enabled,slot_enabled=g.slotEnabled,source_item_id=g.sourceItem and g.sourceItem.id,source_item_exact=g.sourceItem~=nil and g.sourceItem==item,gems=gems}
end
local savedGroups={};for sid,set in pairs(build.skillsTab.skillSets)do for gi,g in ipairs(set.socketGroupList)do savedGroups[#savedGroups+1]={group=g,set=sid,index=gi,state=groupSnapshot(g,sid,gi)};if (g.source or""):match("^Item:28:")then result.runtime_groups[#result.runtime_groups+1]=groupSnapshot(g,sid,gi)end end end
table.sort(result.runtime_groups,function(a,b)if a.set~=b.set then return a.set<b.set end;return a.index<b.index end)
for mode,env in pairs(envs)do
 for _,g in ipairs(env.grantedSkillsItems or{})do if (g.source or""):match("^Item:28:")then result.granted[mode][#result.granted[mode]+1]={fields=scalars(g),source_item_id=g.sourceItem and g.sourceItem.id,source_item_exact=g.sourceItem~=nil and g.sourceItem==item}end end
 for _,a in ipairs(env.player.activeSkillList or{})do if a.activeEffect.grantedEffect.id=="FireboltPlayer"then
  local e=a.activeEffect;local joins={};for _,g in ipairs(savedGroups)do if g.set==selected.skills then for _,gem in ipairs(g.group.gemList)do if e.srcInstance==gem then joins[#joins+1]={set=g.set,index=g.index,source=g.group.source,slot=g.group.slot,source_item_id=g.group.sourceItem and g.group.sourceItem.id}end end end end
  result.actions[mode][#result.actions[mode]+1]={effect=e.grantedEffect.id,level=e.level,quality=e.quality,actor_is_player=a.actor==env.player,from_item=e.srcInstance.fromItem,source_instance=scalars(e.srcInstance),groups=joins,is_main=env.player.mainSkill==a,selected_stat_set=e.statSet and e.statSet.index,output=a.output and scalars(a.output)}
 end end
end
local loaded
if item then
 assert(raw);loaded=snapshot(item,active(item,1,false));loaded.granted_skills=plain(item.grantedSkills)
 local fresh=construct(raw,28);local before=snapshot(fresh,nil);mods(fresh);local after=snapshot(fresh,active(fresh,1,false));after.granted_skills=plain(fresh.grantedSkills)
 local selectedSlots={};for slot,v in pairs(envs.MAIN.player.itemList)do if v==item then selectedSlots[#selectedSlots+1]=slot end end;table.sort(selectedSlots)
 local receiving={};for _,set in ipairs(itemsNode)do if type(set)=="table"and set.elem=="ItemSet"then for _,slot in ipairs(set)do if type(slot)=="table"and slot.elem=="Slot"and tonumber(slot.attrib.itemId)==28 then receiving[#receiving+1]={set=tonumber(set.attrib.id),slot=slot.attrib.name}end end end end
 local function fromItem(m)return type(m.source)=="string"and m.source:match("^Item:28:")end
 local delivery={};for _,name in ipairs({"Damage","ExtraSkill"})do delivery[name]=records(envs.MAIN.modDB.mods[name],fromItem)end
 result.item={raw=raw,xml=xmlItem,loaded=loaded,fresh_after_parse=before,fresh=after,selected_slots=selectedSlots,receiving=receiving,player_records=delivery,base_facts={name=item.baseName,type=item.base.type,quality=item.base.quality,socket_limit=item.base.socketLimit,implicit=item.base.implicit,implicit_mod_types=plain(item.base.implicitModTypes),requirements=plain(item.base.req),tags=plain(item.base.tags)},probes={}}
 if ashenControls then
  local grant="{range:0.5}Grants Skill: Level (1-20) Firebolt";local spell="128% increased Spell Damage"
  local probes={
   {"grant-low",replace(raw,grant,"{range:0}Grants Skill: Level (1-20) Firebolt")},
   {"grant-high",replace(raw,grant,"{range:1}Grants Skill: Level (1-20) Firebolt")},
   {"grant-quarter",replace(raw,grant,"{range:0.25}Grants Skill: Level (1-20) Firebolt")},
   {"grant-fixed",replace(raw,grant,"Grants Skill: Level 11 Firebolt")},
   {"grant-zero",replace(raw,grant,"Grants Skill: Level 0 Firebolt")},
   {"grant-negative",replace(raw,grant,"Grants Skill: Level -1 Firebolt")},
   {"grant-one",replace(raw,grant,"Grants Skill: Level 1 Firebolt")},
   {"grant-twenty",replace(raw,grant,"Grants Skill: Level 20 Firebolt")},
   {"grant-hundred",replace(raw,grant,"Grants Skill: Level 100 Firebolt")},
   {"grant-corrupted-range",replace(raw,grant,"{corruptedRange:1.5}"..grant)},
   {"grant-range-zero",replace(raw,grant,"{range:0.5}Grants Skill: Level (0-0) Firebolt")},
   {"grant-range-zero-endpoint",replace(raw,grant,"{range:0}Grants Skill: Level (0-1) Firebolt")},
   {"grant-range-half-neighbour",replace(raw,grant,"{range:0.49999999999999994}Grants Skill: Level (0-1) Firebolt")},
   {"grant-range-below-half",replace(raw,grant,"{range:0.4999999999999999}Grants Skill: Level (0-1) Firebolt")},
   {"grant-positive-range-below-half",replace(raw,grant,"{range:0.4999999999999998}Grants Skill: Level (1-2) Firebolt")},
   {"grant-positive-range-half-neighbour",replace(raw,grant,"{range:0.49999999999999994}Grants Skill: Level (1-2) Firebolt")},
   {"grant-missing",replace(raw,grant.."\n","")},
   {"grant-unknown",replace(raw,"Firebolt","OwnedUnknownSkill")},
   {"grant-duplicate",replace(raw,grant,grant.."\n"..grant)},
   {"all-explicit",replace(raw,"Implicits: 1","Implicits: 0")},
   {"all-implicit",replace(raw,"Implicits: 1","Implicits: 2")},
   {"quality-zero",replace(raw,"Quality: 20","Quality: 0")},
   {"quality-absent",replace(raw,"Quality: 20\n","")},
   {"quality-malformed",replace(raw,"Quality: 20","Quality: NaN")},
   {"quality-duplicate",replace(raw,"Quality: 20","Quality: 20\nQuality: 0")},
   {"item-level",header(raw,"Item Level: 77")},
   {"requirement-zero",replace(raw,"LevelReq: 26","LevelReq: 0")},
   {"requirement-absent",replace(raw,"LevelReq: 26\n","")},
   {"crafted-false",replace(raw,"Crafted: true","Crafted: false")},
   {"affix-label-absent",replace(raw,"Prefix: {range:1}SpellDamageOnTwoHandWeapon4","Prefix: None")},
   {"sockets-absent",replace(raw,"Sockets: S S S S\nRune: None\nRune: None\nRune: None\nRune: None\n","")},
   {"unknown-rune",replace(raw,"Sockets: S S S S\nRune: None","Sockets: S S S S\nRune: OwnedUnknownRune")},
   {"corrupted",raw.."\nCorrupted"},
   {"catalyst-untagged",header(raw,"Catalyst: Sibilant\nCatalystQuality: 20")},
   {"catalyst-kind-only",header(raw,"Catalyst: Sibilant")},
   {"catalyst-amount-only",header(raw,"CatalystQuality: 37")},
   {"spell-zero",replace(raw,spell,"0% increased Spell Damage")},
   {"spell-fraction",replace(raw,spell,"128.5% increased Spell Damage")},
   {"spell-negative",replace(raw,spell,"-128% increased Spell Damage")},
   {"explicit-magnitude",raw.."\n50% increased explicit modifier magnitudes"},
   {"implicit-magnitude",raw.."\n50% increased implicit modifier magnitudes"},
   {"tagged-caster",replace(raw,spell,"{tags:caster}"..spell)},
   {"tagged-caster-catalyst",header(replace(raw,spell,"{tags:caster}"..spell),"Catalyst: Sibilant\nCatalystQuality: 20")},
   {"tagged-caster-default",header(replace(raw,spell,"{tags:caster}"..spell),"Catalyst: Sibilant")},
   {"tagged-caster-zero",header(replace(raw,spell,"{tags:caster}"..spell),"Catalyst: Sibilant\nCatalystQuality: 0")},
   {"tagged-caster-wrong",header(replace(raw,spell,"{tags:caster}"..spell),"Catalyst: Flesh\nCatalystQuality: 20")},
   {"spell-corrupted-range",replace(raw,spell,"{corruptedRange:1.5}"..spell)},
   {"spell-desecrated",replace(raw,spell,"{desecrated}"..spell)},
   {"spell-fractured",replace(raw,spell,"{fractured}"..spell)},
   {"spell-unknown-prefix",replace(raw,spell,"Owned unknown prefix "..spell)},
  }
  for _,p in ipairs(probes)do local obj=construct(p[2],28);local parsed=snapshot(obj,nil);mods(obj);local final=snapshot(obj,active(obj,1,false));final.granted_skills=plain(obj.grantedSkills);result.item.probes[#result.item.probes+1]={name=p[1],raw=p[2],parsed=parsed,state=final}end
  local reused=construct(header(raw,"Item Level: 77\nCatalyst: Sibilant\nCatalystQuality: 37").."\nCorrupted",28);mods(reused);parse(reused,raw);mods(reused);result.item.reparsed=snapshot(reused,active(reused,1,false))
  local again=construct(raw,28);mods(again);result.item.fresh_after_reparse=snapshot(again,active(again,1,false))
 end
 assert(build.itemsTab.items[28]==item);local now=snapshot(item,active(item,1,false));now.granted_skills=plain(item.grantedSkills);assert(equal(now,loaded))
end
for _,g in ipairs(savedGroups)do assert(build.skillsTab.skillSets[g.set].socketGroupList[g.index]==g.group);assert(equal(groupSnapshot(g.group,g.set,g.index),g.state))end
assert(build.itemsTab.activeItemSetId==selected.items and build.treeTab.activeSpec==selected.spec and build.skillsTab.activeSkillSetId==selected.skills and build.configTab.activeConfigSetId==selected.config and build.mainSocketGroup==selected.group)
for mode,env in pairs(envs)do assert((mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)==env);assert(equal(scalars(outputs[mode]),outputSnapshot[mode]))end
assert(class.ParseRaw==parse and class.BuildModList==mods and build.itemsTab.Load==load and class.GetActiveModListForSlotNum==active and calcs.initEnv==init and build.skillsTab.LoadSkill==loadSkill and build.skillsTab.ProcessSocketGroup==processGroup and calcs.createActiveSkill==createActive and calcs.buildOutput==outputMethod and itemLib.isZeroValueLine==isZeroValueLine)
result.original_functions_preserved=true;result.saved_instances_preserved=true;result.selected_state_preserved=true;result.outputs_preserved=true
return result
