-- The numeric parser, Item constructor, ParseRaw, variant methods and BuildModList
-- execute unchanged authenticated source. Only GUI controls are replaced.
-- Range observation temporarily intercepts the range property after ParseRaw;
-- it restores ordinary tables before any subsequent original item method.
-- Each instrumented corpus run is compared with an uninstrumented run.
local originalNew = new
local class = common.classes.Item
local originalParse, originalBuild = class.ParseRaw, class.BuildModList
local originalParseMod = modLib.parseMod
local originalFormat = itemLib.applyRange
local state
local lists = {'buffModLines','enchantModLines','runeModLines','implicitModLines','explicitModLines'}
local function copy(value, seen)
 if type(value)=='function' then return '<source function; not executed by observer>' end
 if type(value)~='table' then return value end
 seen=seen or {}; if seen[value] then return '<shared/cyclic table>' end
 seen[value]=true; local out={}
 for k,v in pairs(value)do out[copy(k,seen)]=copy(v,seen)end
 seen[value]=nil; return out
end
local function snapshot(item)
 local out={}
 for k,v in pairs(item)do if type(v)~='table' and type(v)~='function' then out[k]=v end end
 for _,k in ipairs({'rawLines','requirements','variantList','versionList','variantGroups','variantGroupSelections','sockets','runes','modMagnitudeMods','classRequirementModLines','weaponData','armourData','flaskData','jewelData','modList'})do out[k]=copy(item[k])end
 for _,k in ipairs(lists)do out[k]=copy(item[k])end
 if item.affixes then for key,value in pairs(data.itemMods)do if value==item.affixes then out.selectedAffixesTableKey=key end end end
 out.hasBase=item.base~=nil
 return out
end
local function restore(item)
 if not state then return end
 for _,entry in ipairs(state.proxies[item] or {})do
  setmetatable(entry.row,entry.oldMeta); rawset(entry.row,'range',entry.value)
 end
 state.proxies[item]=nil
end
local function install(item)
 if not state or not state.instrument then return end
 local proxies={};state.proxies[item]=proxies
 for _,name in ipairs(lists)do
  for index,row in ipairs(item[name] or {})do
   local entry={row=row,value=row.range,oldMeta=getmetatable(row)}
   assert(entry.oldMeta==nil,'original row unexpectedly has a metatable')
   proxies[#proxies+1]=entry;rawset(row,'range',nil)
   setmetatable(row,{
    __index=function(_,key)if key=='range'then return entry.value end end,
    __newindex=function(t,key,value)
     if key=='range'then
      entry.value=value
      state.events[#state.events+1]={kind='range',item=state.ordinals[item],list=name,index=index,value=value}
     else rawset(t,key,value)end
    end,
   })
  end
 end
end
function class:ParseRaw(raw,...)
 if not state then return originalParse(self,raw,...)end
 restore(self)
 if not state.ordinals[self] then
  state.created[#state.created+1]=self;state.ordinals[self]=#state.created
 end
 local event={kind='parse_raw',item=state.ordinals[self],raw=raw,before=snapshot(self),calls={}}
 state.events[#state.events+1]=event
 local previous=state.current;state.current=event
 originalParse(self,raw,...)
 state.current=previous;event.after=snapshot(self)
 install(self)
end
function class:BuildModList(...)
 restore(self)
 if not state then return originalBuild(self,...)end
 local previous=state.building;state.building=true
 local event={kind='build_mod_list',item=state.ordinals[self],before=snapshot(self)}
 state.events[#state.events+1]=event
 local result=originalBuild(self,...)
 event.after=snapshot(self)
 state.building=previous
 return result
end
function modLib.parseMod(text,combined,...)
 local mods,extra=originalParseMod(text,combined,...)
 if state and state.current then
  state.current.calls[#state.current.calls+1]={text=text,combined=combined==true,phase=state.building and 'build_mod_list' or 'parse_raw',has_modifiers=mods~=nil,modifiers=copy(mods),extra=extra}
 end
 return mods,extra
end
local function noop()end
function new(kind,...)
 if kind=='ItemSlotControl'then
  return{ItemSlotControl=function(self,anchor,x,y,tab,name)
   self.slotName=name;self.controls={activate={}};self.jewelSocketList={};self.selItemId=0;return self
  end}
 elseif kind=='Control'then return{Control=function(self)return self end}
 elseif kind=='DropDownControl'then
  return{DropDownControl=function(self)
   self.anchor={};self.value='None'
   self.SelByValue=function(s,value,key)assert(key=='name');s.value=value end
   self.GetSelValue=function(s)return{name=s.value}end
   return self
  end}
 elseif kind=='LabelControl'then return{LabelControl=function(self)return self end}
 end
 return originalNew(kind,...)
end
function item_loading_parse(raw)
 return originalNew('Item'):Item(raw)
end
function item_loading_load(xml,instrument)
 local roots,err=originalXml.ParseXML(xml);assert(roots,err)
 local node=roots[1]
 if node.elem~='Items'then
  local found
  for _,child in ipairs(node)do if type(child)=='table'and child.elem=='Items'then assert(not found);found=child end end
  node=assert(found,'missing Items container')
 end
 state={events={},created={},ordinals={},proxies={},instrument=instrument}
 local tab=setmetatable({items={},itemOrderList={},controls={},tradeQuery={},showStatDifferences=true,ResetUndo=noop,PopulateSlots=noop,build={SyncLoadouts=noop}},{__index=ItemsTabClass})
 original_slot_setup(tab)
 local ok,error=pcall(ItemsTabClass.Load,tab,node,'original-source item loading oracle')
 local result={ok=ok,error=not ok and tostring(error)or nil,events=state.events,items={},order=copy(tab.itemOrderList),sets=copy(tab.itemSets),setOrder=copy(tab.itemSetOrderList),activeSet=tab.activeItemSetId}
 for _,item in ipairs(state.created)do restore(item);result.items[#result.items+1]=snapshot(item)end
 state=nil
 return result
end

-- Independent dependency providers execute the original functions. They are
-- test adapters only and never make the portable library load Lua itself.
function item_loading_parse_dependency(text,combined)
 return originalParseMod(text,combined)
end
function item_loading_format_dependency(text,range,scalar,corrupted)
 return itemLib.applyRange(text,range,scalar,corrupted)
end
function item_loading_unique_dependency(name,title,baseName)
 local item=originalNew('Item')
 item.rarity='UNIQUE';item.name=name;item.title=title;item.baseName=baseName
 local found=item:GetUniqueDBItem()
 return found and found.requirements
end

item_loading_functions={
 constructor=class.Item,parse_raw=originalParse,build_mod_list=originalBuild,
 check_variant=class.CheckModLineVariant,variant_count=class.GetModLineVariantCount,
 normalize=class.NormaliseVariantSelections,load=ItemsTabClass.Load,
}

function item_loading_warm()
 local started,completed={},{}
 local function trace(event,id,func)
  if event=='start'then started[id]=(func==originalParse or func==originalBuild or func==ItemsTabClass.Load)
  elseif event=='stop'and started[id]then completed[id]=true
  elseif event=='abort'then started[id]=nil end
 end
 jit.flush();jit.on();jit.opt.start('hotloop=1','hotexit=1')
 jit.attach(trace,'trace')
 for i=1,96 do originalNew('Item'):Item('Rarity: Normal\nRusted Greathelm\nItem Level: 1\nQuality: 0\n+10 to maximum Life')end
 jit.attach(trace)
 local count=0
 for id in pairs(completed)do if require('jit.util').traceinfo(id)then count=count+1 end end
 return count
end

function itemLib.applyRange(text,range,scalar,corrupted)
 if state and state.current and not state.building then
  state.events[#state.events+1]={kind='format',text=text,range=range,scalar=scalar,corrupted=corrupted,before=snapshot(state.created[state.current.item])}
 end
 return originalFormat(text,range,scalar,corrupted)
end
