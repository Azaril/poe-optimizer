-- Test-only boundaries around unchanged XML / ItemsTab / PassiveSpec methods.
-- ParseRaw is an event recorder with synthetic list sizes, NOT an item parser.
-- Therefore this oracle proves load instructions and ownership, not item validity,
-- modifier parsing, range-to-real-affix binding, equipped legality, or numerics.
local function noop() end
local function parse(text)
 local roots,err=originalXml.ParseXML(text);assert(roots,err);return roots[1]
end
local function children(node,name)
 local out={};for _,v in ipairs(node)do if type(v)=='table'and v.elem==name then out[#out+1]=v end end;return out
end
local events,created
local function emit(value)events[#events+1]=value end
local function makeList(item,name,size)
 local list={}
 for index=1,size do
  local range
  list[index]=setmetatable({}, {
   __index=function(_,key)if key=='range'then return range end end,
   __newindex=function(_,key,value)
    assert(key=='range','unexpected synthetic modifier write')
    range=value;emit({kind='range',item=item.ordinal,list=name,index=index,value=value,parse_ordinal=item.parseCount})
   end,
  })
 end
 return list
end
function new(kind)
 if kind=='Item'then
  return {Item=function(self,raw)
   assert(raw=='');self.ordinal=#created+1;self.parseCount=0;self.jewelSocketCount=0
   created[#created+1]=self
   function self:ParseRaw(text)
    self.parseCount=self.parseCount+1;self.raw=text;self.base={};self.title='synthetic test boundary'
    emit({kind='parse_raw',item=self.ordinal,text=text,parse_ordinal=self.parseCount})
    self.buffModLines=makeList(self,'buff',2)
    self.enchantModLines=makeList(self,'enchant',3)
    self.implicitModLines=makeList(self,'implicit',5)
    self.explicitModLines=makeList(self,'explicit',512)
    self.runeModLines={}
   end
   function self:BuildModList()emit({kind='build_mod_list',item=self.ordinal})end
   function self:BuildAndParseRaw()error('numerical item save boundary must not execute')end
   -- Real Item construction supplies empty lists even before the first raw string.
   self.buffModLines={};self.enchantModLines={};self.implicitModLines={};self.explicitModLines={};self.runeModLines={}
   return self
  end}
 elseif kind=='ItemSlotControl'then
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
 error('unimplemented GUI or numerical class '..tostring(kind))
end
local function load(node)
 events={};created={}
 local tab=setmetatable({items={},itemOrderList={},controls={},tradeQuery={},showStatDifferences=true,ResetUndo=noop,PopulateSlots=noop,build={SyncLoadouts=noop}},{__index=ItemsTabClass})
 original_slot_setup(tab)
 local ok,err=pcall(ItemsTabClass.Load,tab,node,'independent item-load oracle')
 return{tab=tab,events=events,created=created,ok=ok,error=not ok and tostring(err)or nil,scope='synthetic Item boundary; original ordered load and set methods'}
end
local function load_spec(node,items)
 local messages={}
 launch={ShowErrMsg=function(_,fmt,...)messages[#messages+1]=string.format(fmt,...)end}
 local spec={jewels={},nodeNotes={},hashOverrides={},treeVersion='test',build={itemsTab={items=items}},ResetUndo=noop,SwitchAttributeNode=noop,ImportFromNodeList=noop,DecodeURL=noop}
 -- Neutral test-only mappings are not class/ascendancy or allocation validation.
 spec.tree={classIntegerIdMap=setmetatable({},{__index=function()return 0 end}),internalAscendNameMap=setmetatable({},{__index=function()return{ascendClassId=0}end})}
 local ok,result=pcall(PassiveSpecClass.Load,spec,node,'independent jewel-load oracle')
 return{spec=spec,messages=messages,ok=ok,result=result,scope='original saved jewel reference consumer; no tree import or item numerical validity'}
end
return{parse=parse,children=children,load=load,load_spec=load_spec}
