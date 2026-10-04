-- Observe original call boundaries. This file never replaces a source method.
local calcs = require("Modules.CalcBase")
local function original(f, path, line)
 local info = debug.getinfo(assert(f), "S")
 local actual = info.source:gsub("\\", "/")
 assert(info.what == "Lua" and actual:sub(-#path) == path and info.linedefined == line)
 return f, {path=path, first=info.linedefined, last=info.lastlinedefined}
end
local init, initAuth = original(calcs.initEnv, "Modules/CalcSetup.lua", 717)
local sum, sumAuth = original(common.classes.ModDB.SumInternal, "Classes/ModDB.lua", 137)
local scale, scaleAuth = original(common.classes.ModStore.ScaleAddMod, "Classes/ModStore.lua", 82)
local add, addAuth = original(common.classes.ModDB.AddMod, "Classes/ModDB.lua", 31)
local active, activeAuth = original(common.classes.Item.GetActiveModListForSlotNum, "Classes/Item.lua", 2198)
local nodes, nodesAuth = original(calcs.buildModListForNodeList, "Modules/CalcSetup.lua", 415)
local methods = {init=initAuth, sum=sumAuth, scale=scaleAuth, add=addAuth, active_item=activeAuth, nodes=nodesAuth}
local factor = "EffectOfBonusesFromAmulet"
local names = {factor, "EffectOfBonusesFromRing 1", "EffectOfBonusesFromRing 2", "EffectOfBonusesFromRing 3"}
local wantedNames = {}; for _, name in ipairs(names) do wantedNames[name] = true end
local function scalar(v)
 assert(v == nil or type(v) == "string" or type(v) == "boolean" or type(v) == "number")
 if type(v) == "number" then assert(v == v and v ~= math.huge and v ~= -math.huge) end
 return v
end
local function fields(t)
 local result = {}
 for k, v in pairs(t or {}) do
  if type(k) == "string" and (type(v) == "string" or type(v) == "boolean" or type(v) == "number") then
   result[k] = type(v)=="number" and (v~=v or v==math.huge or v==-math.huge) and tostring(v) or scalar(v)
  end
 end
 return result
end
local function plain(v, depth)
 if type(v) ~= "table" then return scalar(v) end
 depth = (depth or 0) + 1; assert(depth < 12)
 local out, positions, count = {}, {}, 0
 for k, value in pairs(v) do
  count = count + 1; assert(count <= 2048)
  if type(k) == "number" then positions[#positions+1] = {index=k, value=plain(value, depth)}
  else assert(type(k) == "string"); out[k] = plain(value, depth) end
 end
 table.sort(positions, function(a,b) return a.index < b.index end)
 if #positions > 0 then out._positions = positions end
 return out
end
local function mod(m)
 local out = fields(m); out.value = plain(m.value); out.tags = {}
 for i, tag in ipairs(m) do out.tags[i] = plain(tag) end
 return out
end
local function rows(list)
 local out = {}
 for i, m in ipairs(list or {}) do
  assert(i <= 16384)
  if wantedNames[m.name] then out[#out+1] = {position=i, mod=mod(m)} end
 end
 return out
end
local function chain(store)
 local out, seen = {}, {}
 while store do
  assert(not seen[store] and #out < 16); seen[store] = true
  local selected = {}
  if store.mods then
   for _, name in ipairs(names) do
    for i, m in ipairs(store.mods[name] or {}) do
     assert(i <= 2048); selected[#selected+1] = {position=i, mod=mod(m)}
    end
   end
  else selected = rows(store) end
  out[#out+1] = {depth=#out, kind=store.mods and "database" or "list", rows=selected}
  store = store.parent
 end
 return out
end
local function ids(map)
 local out = {}; for id in pairs(map or {}) do assert(type(id) == "number"); out[#out+1] = id end
 assert(#out < 4096); table.sort(out); return out
end
local function equal(a,b)
 if type(a) ~= type(b) then return false end
 if type(a) ~= "table" then return a == b end
 for k,v in pairs(a) do if not equal(v,b[k]) then return false end end
 for k in pairs(b) do if a[k] == nil then return false end end
 return true
end
-- Called only from the debug hook. Stack depths include this helper and hook.
local function init_frame()
 for depth=3,20 do
  local info = debug.getinfo(depth, "fl")
  if not info then break end
  if info.func == init then
   local values = {}
   for i=1,128 do local name,value=debug.getlocal(depth,i); if not name then break end; values[name]=value end
   return values, info.currentline
  end
 end
end
local function preserved()
 assert(calcs.initEnv == init and common.classes.ModDB.SumInternal == sum)
 assert(common.classes.ModStore.ScaleAddMod == scale and common.classes.ModDB.AddMod == add)
 assert(common.classes.Item.GetActiveModListForSlotNum == active and calcs.buildModListForNodeList == nodes)
end

if amuletSnapshotPhase == "before" then
 assert(not debug.gethook() and jit.status() == amuletSnapshotJit)
 local capture = {queries={}, items={}, nodes={}, copies={}, sums={}, scales={}, adds={}, finished=false}
 local watched = {[sum]=true, [scale]=true, [add]=true, [active]=true, [nodes]=true}
 local function source_frame(env)
  local slots = {}; for slot in pairs(env.player.itemList) do slots[#slots+1] = slot end; table.sort(slots)
  local items = {}
  for _,slot in ipairs(slots) do
   local item = env.player.itemList[slot]
   assert(item.id and env.build.itemsTab.items[item.id] == item)
   local itemRows = {}
   for callIndex,call in ipairs(capture.items) do
    if call.env == env and call.item == item then
     itemRows[#itemRows+1] = {call_index=callIndex,line=call.line, slot_number=call.slot_number, can_use_bonded=call.can_use_bonded,
      count=call.count, rows=call.rows, exact_item=true}
    end
   end
   local lines = {}
   for _,kind in ipairs({"implicitModLines","explicitModLines","enchantModLines","runeModLines"}) do
    for index,line in ipairs(item[kind] or {}) do
     local selected=rows(line.modList)
     if #selected>0 then lines[#lines+1]={kind=kind,index=index,line=line.line,rows=selected} end
    end
   end
   items[#items+1] = {slot=slot,id=item.id,name=item.name,base_name=item.baseName,type=item.type,
    mod_source=item.modSource,exact_loaded_item=true,item_socket_count=item.itemSocketCount,
    runes=plain(item.runes or {}),calls=itemRows,source_lines=lines}
  end
  local nodeCalls = {}
  for _,call in ipairs(capture.nodes) do
   if call.env == env then nodeCalls[#nodeCalls+1] = call.report end
  end
  local allocations = {}
  for _,id in ipairs(ids(env.allocNodes)) do
   local node=env.allocNodes[id]
   allocations[#allocations+1]={id=id,exact_spec_node=env.spec.allocNodes[id]==node,
    granted=env.grantedPassives[id]==true,rows=rows(node.modList)}
  end
  -- Record pointer joins before later copy phases can change the databases.
  -- Matching source strings alone are deliberately not treated as ownership.
  local candidateJoins,store,depth={},env.modDB,0
  while store do
   assert(depth<16)
   for position,m in ipairs(store.mods[factor] or {}) do
    local itemCalls,itemDatabase,config={}, {}, {}
    for callIndex,call in ipairs(capture.items) do if call.env==env then
     for index,input in ipairs(call.list) do if input==m then
      itemCalls[#itemCalls+1]={call_index=callIndex,item_id=call.item.id,position=index,exact_object=true}
     end end
    end end
    local db,d=env.itemModDB,0
    while db do
     assert(d<16)
     for index,input in ipairs(db.mods[factor] or {}) do if input==m then
      itemDatabase[#itemDatabase+1]={depth=d,position=index,exact_object=true}
     end end
     db=db.parent;d=d+1
    end
    for index,input in ipairs(env.build.configTab.modList) do if input==m then config[#config+1]=index end end
    candidateJoins[#candidateJoins+1]={depth=depth,position=position,item_calls=itemCalls,
     item_database=itemDatabase,configuration_positions=config}
   end
   store=store.parent;depth=depth+1
  end
  return {items=items,allocations=allocations,granted_ids=ids(env.grantedPassives),node_calls=nodeCalls,
   candidate_joins=candidateJoins,
   config={id=env.build.configTab.activeConfigSetId,input=fields(env.configInput),
    row_count=#env.build.configTab.modList,rows=rows(env.build.configTab.modList)},
   item_database=chain(env.itemModDB),radius_jewel_count=#env.radiusJewelList,
   extra_radius_node_ids=ids(env.extraRadiusNodeList),talisman_list_present=env.talismanModList~=nil,
   weapon_two_present=env.player.itemList["Weapon 2"]~=nil,
   quiver_present=env.player.itemList["Weapon 2"] and env.player.itemList["Weapon 2"].type=="Quiver" or false}
 end
 local function hook(event)
  if event ~= "call" and event ~= "return" then return end
  local f=debug.getinfo(2,"f").func; if not watched[f] then return end
  local v={}; for i=1,128 do local name,value=debug.getlocal(2,i); if not name then break end; v[name]=value end
  if f == active and event == "return" then
   local frame,line=init_frame()
   if frame and frame.env then
    local list=v.self.modList or v.self.slotModList[v.slotNum]
    assert(list)
    capture.items[#capture.items+1]={env=frame.env,item=v.self,list=list,line=line,slot_number=v.slotNum,
     can_use_bonded=v.canUseBonded==true,count=#list,rows=rows(list)}
    assert(#capture.items<=4096)
   end
  elseif f == nodes and event == "return" then
   capture.nodes[#capture.nodes+1]={env=v.env,report={ids=ids(v.nodeList),finish_jewels=v.finishJewels==true,
    include_keystones=v.includeKeystoneMods==true,count=#v.modList,rows=rows(v.modList)}}
   assert(#capture.nodes<=1024)
  elseif f == sum and v.modName == factor and v.modType == "INC" and v.self == v.context then
   if event == "call" then
    local frame,line=init_frame()
    if frame and line==1662 then
     assert(v.cfg==nil and v.source==nil and v.flags==0 and v.keywordFlags==0)
     local env=assert(frame.env); assert(env.modDB==v.self and frame.modDB==v.self)
     assert(not capture.sums[v.self])
     local r={env=env,store=v.self,report={mode=env.mode,line=line,cfg_present=false,flags=v.flags,
      keyword_flags=v.keywordFlags,source_filter_present=false,chain=chain(v.self),frame=source_frame(env)}}
     capture.sums[v.self]=r
    end
   elseif capture.sums[v.self] then
    local r=capture.sums[v.self]; capture.sums[v.self]=nil
    r.report.result=scalar(v.result); r.report.original_return_observed=true
    capture.queries[#capture.queries+1]=r; assert(#capture.queries<=256)
   end
  elseif f == scale then
   if event == "call" then
    local frame,line=init_frame()
    if frame and line==1667 then
     assert(frame.env.modDB==v.self and frame.modCopy==v.mod)
     assert(frame.amuletEffectMod==v.scale)
     local r={env=frame.env,store=v.self,input=v.mod,report={line=line,scale=v.scale,
      original=mod(assert(frame.mod)),input=mod(v.mod),input_is_copy=frame.mod~=v.mod,insertions={}}}
     assert(not capture.scales[v.self]); capture.scales[v.self]=r
    end
   elseif capture.scales[v.self] and capture.scales[v.self].input==v.mod then
    local r=capture.scales[v.self]; capture.scales[v.self]=nil
    r.report.input_preserved=equal(r.report.input,mod(v.mod)); assert(r.report.input_preserved)
    capture.copies[#capture.copies+1]=r; assert(#capture.copies<=8192)
   end
  elseif f == add and capture.scales[v.self] then
   local r=capture.scales[v.self]
   if event == "call" then
    assert(not capture.adds[v.self]); capture.adds[v.self]={row=r,mod=v.mod,before=#(v.self.mods[v.mod.name] or {})}
   else
    local a=assert(capture.adds[v.self]); capture.adds[v.self]=nil
    local list=assert(v.self.mods[a.mod.name]); assert(#list==a.before+1 and list[#list]==a.mod)
    r.report.insertions[#r.report.insertions+1]={mod=mod(a.mod),same_as_input=a.mod==r.input,
     position=#list,exact_destination=true,inserted_object_exact=true}
   end
  end
 end
 debug.sethook(hook,"cr")
 return function()
  debug.sethook(); preserved()
  assert(not next(capture.sums) and not next(capture.scales) and not next(capture.adds),"unfinished original call")
  assert(jit.status()==amuletSnapshotJit)
  capture.finished=true; amuletSnapshotCapture=capture
 end
end

assert(amuletSnapshotPhase=="observe" and not debug.gethook())
assert(jit.status()==amuletSnapshotJit and djinnOriginals.preserved_after_load)
preserved()
local capture=assert(amuletSnapshotCapture); assert(capture.finished)
local doc,err=common.xml.ParseXML(amuletSnapshotXml); assert(doc and not err)
local raw={}; for _,node in ipairs(doc[1]) do if type(node)=="table" then raw[node.elem]=node end end
local selected={skills=build.skillsTab.activeSkillSetId,spec=build.treeTab.activeSpec,
 items=build.itemsTab.activeItemSetId,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup}
assert(tonumber(raw.Skills.attrib.activeSkillSet)==selected.skills)
assert(tonumber(raw.Items.attrib.activeItemSet)==selected.items)
assert(tonumber(raw.Tree.attrib.activeSpec)==selected.spec)
assert(tonumber(raw.Config.attrib.activeConfigSet)==selected.config)
local rawItems,rawSlots={},{}
for _,node in ipairs(raw.Items) do
 if type(node)=="table" and node.elem=="Item" then rawItems[tonumber(node.attrib.id)]=node end
 if type(node)=="table" and node.elem=="ItemSet" and tonumber(node.attrib.id)==selected.items then
  for _,slot in ipairs(node) do
   if type(slot)=="table" and slot.elem=="Slot" then rawSlots[slot.attrib.name]=tonumber(slot.attrib.itemId) end
  end
 end
end
local rawSpec; local index=0
for _,node in ipairs(raw.Tree) do if type(node)=="table" and node.elem=="Spec" then
 index=index+1; if index==selected.spec then rawSpec=node end
end end
assert(rawSpec)
local rawNodes={}; for id in rawSpec.attrib.nodes:gmatch("%d+") do rawNodes[#rawNodes+1]=tonumber(id) end; table.sort(rawNodes)
local modes={}
for _,name in ipairs({"MAIN","CALCS"}) do
 local env=name=="MAIN" and build.calcsTab.mainEnv or build.calcsTab.calcsEnv
 local output=name=="MAIN" and build.calcsTab.mainOutput or build.calcsTab.calcsOutput
 local outputBefore=fields(output)
 local found={}
 for _,r in ipairs(capture.queries) do if r.env==env then found[#found+1]=r end end
 assert(#found==1,"expected one exact final environment pre-Amulet query for "..name..", got "..#found)
 local query=found[1]; local report=query.report; assert(report.mode==name)
 assert(equal(ids(env.allocNodes),rawNodes),"selected original allocations differ")
 for _,item in ipairs(report.frame.items) do
  local saved=assert(rawItems[item.id]); assert(rawSlots[item.slot]==item.id)
  item.source_attributes=fields(saved.attrib); item.exact_saved_slot=true
 end
 local copies,allCopies={},0
 for _,copy in ipairs(capture.copies) do if copy.env==env then
  allCopies=allCopies+1
  if wantedNames[copy.report.input.name] or copy.report.input.name=="GemProperty" then copies[#copies+1]=copy.report end
 end end
 assert(allCopies>0,"original Amulet copy loop was not observed")
 local compact={}
 for _,key in ipairs({"Life","EnergyShield","Spirit","SpiritUnreserved","SpiritReserved","TotalDPS","CombinedDPS"}) do
  compact[key]=outputBefore[key]
 end
 assert(equal(outputBefore,fields(output)),"observation mutated original output")
 modes[name]={snapshot=report,copy_calls=allCopies,relevant_copies=copies,final_chain=chain(env.modDB),
  output=compact,output_preserved=true,exact_final_environment=true}
end
preserved()
return {selected=selected,saved_spec=fields(rawSpec.attrib),saved_node_ids=rawNodes,modes=modes,
 observed_query_count=#capture.queries,methods=methods,methods_preserved=true,hook_removed=true,
 requested_jit_mode_verified=true,business_wrappers=false}
