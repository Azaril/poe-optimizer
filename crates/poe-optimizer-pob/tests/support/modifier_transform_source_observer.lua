-- Read-only observations over complete original Item functions. No method replacements.
local raw, auth = ...
local class = common.classes.Item
local before_functions = {parse=class.ParseRaw,build=class.BuildModList,
  constructor=class.Item,parser=modLib.parseMod,range=itemLib.applyRange,
  count=class.GetModLineVariantCount,check=class.CheckModLineVariant,new=new}
assert(auth.parse == item_loading_functions.parse_raw)
assert(auth.build == item_loading_functions.build_mod_list)
assert(auth.constructor == class.Item)
assert(auth.check == class.CheckModLineVariant and auth.count == class.GetModLineVariantCount)
local function copy(value, seen)
  assert(type(value)~='function' and type(value)~='userdata')
  if type(value)~='table' then return value end
  seen=seen or {};assert(not seen[value],'unexpected source cycle');seen[value]=true
  local out={};for k,v in pairs(value)do out[copy(k,seen)]=copy(v,seen)end
  seen[value]=nil;return out
end
local categories={'buff','enchant','rune','classRequirement','implicit','explicit'}
local function line_state(item,row)
  local out={line=row.line,mod_tags=copy(row.modTags),value_scalar=row.valueScalar,
    range=row.range,corrupted_range=row.corruptedRange,has_mod_list=row.modList~=nil,
    mod_list=copy(row.modList),extra=row.extra,active=not not auth.check(item,row),
    variant_count=auth.count(item,row)}
  for _,key in ipairs({'disabled','unscalable','crafted','custom','fractured','desecrated',
    'prefix','suffix','implicit','enchant','variantList','versionList','variantGroupList','order'})do
    out[key]=copy(row[key])
  end
  return out
end
local function state(item)
  local result={raw_lines=copy(item.rawLines),capacity=item.charmLimit,
    advanced_copy=item.advancedCopy,crafted=item.crafted,
    versioned_or_grouped=item:UsesVersionedOrGroupedVariants(),variant=item.variant,
    selected_version=item.selectedVersion,variant_group_selections=copy(item.variantGroupSelections),
    lines={},producers={},base_records={},active_records={}}
  local addresses={}
  for _,category in ipairs(categories)do
    local rows={}
    for index,row in ipairs(item[category..'ModLines'] or {})do
      addresses[row]={category=category,index=index}
      rows[#rows+1]=line_state(item,row)
    end
    result.lines[category]=rows
  end
  for index,producer in ipairs(item.modMagnitudeMods or {})do
    local row=assert(producer.sourceLine)
    result.producers[#result.producers+1]={discovery_index=index,
      operation=producer.multiplier and 'multiply' or 'add',quality=producer.quality,
      multiplier=producer.multiplier,mod_type=producer.modType,tags=copy(producer.tags),
      any_tags=copy(producer.anyTags),source_address=copy(addresses[row]),
      source_line=line_state(item,row)}
  end
  for _,record in ipairs(item.baseModList or {})do result.base_records[#result.base_records+1]=copy(record)end
  for _,record in ipairs(item:GetActiveModListForSlotNum(1,false) or {})do result.active_records[#result.active_records+1]=copy(record)end
  return result
end
local constructed = new('Item'):Item(raw)
assert(constructed.baseName=='Fine Belt' and constructed.base.charmLimit==0)
local result={constructed=state(constructed)}
auth.build(constructed)
result.rebuilt=state(constructed)
auth.build(constructed)
result.rebuilt_twice=state(constructed)
-- Independent path uses the exact authenticated ParseRaw on a fresh original Item.
-- The Item constructor has no behavior beyond sanitiseText + ParseRaw; fixtures are ASCII.
local direct = new('Item')
auth.parse(direct,raw)
result.direct_parse=state(direct)
assert(class.ParseRaw==before_functions.parse and class.BuildModList==before_functions.build)
assert(class.Item==before_functions.constructor and modLib.parseMod==before_functions.parser)
assert(itemLib.applyRange==before_functions.range and new==before_functions.new)
assert(class.GetModLineVariantCount==before_functions.count and class.CheckModLineVariant==before_functions.check)
result.original_functions_preserved=true
return result
