-- Observe complete original constructors and DB reads/writes. Arithmetic, parser,
-- Item methods and the Main construction loop remain the authenticated source.
-- Alternate modes override only iteration over the original unique-group tables;
-- they exercise unspecified pairs traversal orders, not a sorted production loop.
local originalPairs, originalPairsYield = pairs, pairsYield
local class = common.classes.Item
local originalItem, originalLookup = class.Item, class.GetUniqueDBItem
local originalParse = modLib.parseMod
local active
local function packedNumber(value)
 if value==nil then return {kind='nil'} end
 assert(type(value)=='number','unexpected source requirement type')
 return {kind='number',value=value}
end
local function snapshot(item)
 local req=item.requirements or {}
 return {name=item.name,title=item.title,base_name=item.baseName,
  has_base=item.base~=nil,natural=packedNumber(req.naturalLevel),level=packedNumber(req.level)}
end
local function keyIterator(t,reverse,rotate)
 local keys={}
 for k in originalPairs(t)do keys[#keys+1]=k end
 if reverse then
  for i=1,math.floor(#keys/2)do keys[i],keys[#keys-i+1]=keys[#keys-i+1],keys[i]end
 end
 if rotate and #keys>1 then local first=table.remove(keys,1);keys[#keys+1]=first end
 local i=0
 return function()i=i+1;local k=keys[i];if k~=nil then return k,t[k]end end
end
function pairsYield(t)
 if not active or t~=data.uniques or active.mode=='control' then return originalPairsYield(t)end
 local iterator
 if active.mode=='actual' then iterator=originalPairsYield(t)
 else iterator=keyIterator(t,true,active.mode=='rotate')end
 return function()
  local k,v=iterator()
  if k~=nil then active.group=k end
  return k,v
 end
end
function pairs(t)
 if not active or not active.groups[t] or active.mode=='control' then return originalPairs(t)end
 local iterator,state,key
 if active.mode=='reverse-all' or active.mode=='rotate' then
  iterator=keyIterator(t,true,active.mode=='rotate')
 else iterator,state,key=originalPairs(t)end
 return function()
  local k,v=iterator(state,key);key=k
  if k~=nil then active.index=k end
  return k,v
 end
end
function class:Item(raw,rarity,highQuality)
 if not active or active.mode=='control' then return originalItem(self,raw,rarity,highQuality)end
 assert(not active.current,'nested Item constructor was not audited')
 assert(rarity=='UNIQUE' and highQuality==true,'original constructor options changed')
 local record={group=active.group,index=active.index,raw=raw,reads={},lookups={}}
 active.current=record
 local result=originalItem(self,raw,rarity,highQuality)
 record.result=snapshot(self)
 active.records[#active.records+1]=record
 active.by_item[self]=record
 active.current=nil
 return result
end
function class:GetUniqueDBItem()
 if not active or active.mode=='control' then return originalLookup(self)end
 local record=assert(active.current,'unique lookup outside original constructor')
 local actual=main.uniqueDB.list
 local potential={}
 main.uniqueDB.list=setmetatable({}, {__index=function(_,key)
  potential[#potential+1]=key
  return nil
 end})
 -- The unchanged lookup is read-only. An all-miss observation exposes every
 -- potential key, including fallback keys hidden by an actual exact hit.
 local ok,err=pcall(originalLookup,self)
 main.uniqueDB.list=actual
 assert(ok,err)
 record.lookups[#record.lookups+1]={name=self.name,title=self.title,
  base_name=self.baseName,potential=potential}
 return originalLookup(self)
end
function modLib.parseMod(line,...)
 if active then
  active.parse_calls=active.parse_calls+1
  active.parse_inputs[line]=(active.parse_inputs[line] or 0)+1
 end
 return originalParse(line,...)
end
function runUniqueConstruction(mode)
 assert(not active,'construction already active')
 local state={mode=mode,groups={},records={},by_item={},parse_inputs={},parse_calls=0,
  insertions={},backing={},overwrites=0}
 for group,items in originalPairs(data.uniques)do state.groups[items]=group end
 active=state
 local db
 if mode=='control' then db=state.backing else
  db=setmetatable({}, {
   __index=function(_,key)
    local value=state.backing[key]
    local record=assert(state.current,'DB read outside constructor')
    record.reads[#record.reads+1]={key=key,found=value~=nil}
    return value
   end,
   __newindex=function(_,key,value)
    local record=assert(state.by_item[value],'insertion without observed constructor')
    local prior=state.backing[key]
    if prior~=nil then state.overwrites=state.overwrites+1 end
    state.insertions[#state.insertions+1]={key=key,group=record.group,index=record.index,
     overwrite=prior~=nil}
    record.inserted_key=key
    state.backing[key]=value
   end,
  })
 end
 main.uniqueDB={list=db,loading=true}
 local thread=coroutine.create(originalUniqueLoad)
 local resumes=0
 repeat
  resumes=resumes+1;assert(resumes<=16384,'original construction resume bound')
  local ok,err=coroutine.resume(thread)
  if not ok then active=nil;error(err)end
 until coroutine.status(thread)=='dead'
 assert(not main.uniqueDB.loading,'original loading marker not removed')
 local entries={}
 for key,item in originalPairs(state.backing)do entries[key]=snapshot(item)end
 active=nil
 return {mode=mode,records=state.records,entries=entries,insertions=state.insertions,
  overwrites=state.overwrites,parse_calls=state.parse_calls,parse_inputs=state.parse_inputs,
  resumes=resumes,complete=true}
end
