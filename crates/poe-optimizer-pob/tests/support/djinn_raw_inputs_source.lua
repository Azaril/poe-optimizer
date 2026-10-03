-- Read original LoadSkill locals at its call to ProcessSocketGroup. The hook
-- never substitutes a business function or changes a source value. It ends
-- after the last saved Djinn group, before calculation preparation.
local effects={SummonSandDjinnPlayer=true,SummonWaterDjinnPlayer=true}
local function scalar(v)
 if type(v)=="number"and(v~=v or v==math.huge or v==-math.huge)then return tostring(v)end
 return v
end
local function scalars(t)local r={};for k,v in pairs(t or{})do if type(v)=="number"or type(v)=="boolean"or type(v)=="string"then r[k]=scalar(v)end end;return r end
local function same(a,b)for k,v in pairs(a)do if b[k]~=v then return false end end;for k,v in pairs(b)do if a[k]~=v then return false end end;return true end
local function input(g)return {level=scalar(g.level),level_type=type(g.level),quality=scalar(g.quality),quality_type=type(g.quality)}end
local function key(sid,effect,source)return tostring(sid).."/"..effect.."/"..(source or"manual")end
local doc,err=common.xml.ParseXML(djinnXml);assert(doc and not err)
local ordinal={};local count=0
local function enumerate(n)if type(n)~="table"or not n.elem then return end;ordinal[n]=count;count=count+1;assert(count<20000);for _,c in ipairs(n)do enumerate(c)end end
enumerate(doc[1])
local expected={};local expectedCount=0
for _,section in ipairs(doc[1])do if type(section)=="table"and section.elem=="Skills"then
 for _,set in ipairs(section)do if type(set)=="table"and set.elem=="SkillSet"then
  for _,g in ipairs(set)do if type(g)=="table"and g.elem=="Skill"then
   for index,gem in ipairs(g)do if type(gem)=="table"and effects[gem.attrib.skillId]then
    assert(index==1,"Djinn source is not first physical occurrence")
    local k=key(tonumber(set.attrib.id),gem.attrib.skillId,g.attrib.source);assert(not expected[k])
    expected[k]={key=k,source=ordinal[gem],attributes=scalars(gem.attrib)};expectedCount=expectedCount+1
   end end
  end end
 end end
end end
assert(expectedCount>0 and expectedCount<=64)
local records,refs={},{}
local load=assert(djinnOriginals.refs.load_skill)
local process=assert(djinnOriginals.refs.process_group)
local previous=debug.gethook();assert(not previous,"unexpected pre-existing debug hook")
local hook
hook=function(event)
 if event~="call"or debug.getinfo(2,"f").func~=process then return end
 if debug.getinfo(3,"f").func~=load then return end
 local locals={};for i=1,32 do local name,value=debug.getlocal(2,i);if not name then break end;locals[name]=value end
 local caller={};for i=1,32 do local name,value=debug.getlocal(3,i);if not name then break end;caller[name]=value end
 local group=assert(locals.socketGroup);local node=assert(caller.node);local sid=assert(caller.skillSetId)
 for index,gem in ipairs(group.gemList)do if effects[gem.skillId]then
  local k=key(sid,gem.skillId,group.source);local row=assert(expected[k]);assert(not refs[k])
  assert(index==1 and same(scalars(node[index].attrib),row.attributes))
  records[#records+1]={key=k,source=row.source,attributes=row.attributes,loaded=input(gem),caller_exact=true,process_exact=true}
  refs[k]={gem=gem,group=group};assert(#records<=expectedCount)
 end end
 if #records==expectedCount then debug.sethook()end
end
debug.sethook(hook,"c")
return function()
 local current=debug.gethook();assert(current==nil or current==hook,"source changed observer hook")
 debug.sethook()
 for _,row in ipairs(records)do
  local ref=refs[row.key];local hits=0
  for sid,set in pairs(build.skillsTab.skillSets)do for _,group in ipairs(set.socketGroupList)do
   for index,gem in ipairs(group.gemList)do if gem==ref.gem then
    hits=hits+1;assert(group==ref.group and index==1)
    assert(row.key==key(sid,gem.skillId,group.source))
   end end
  end end
  assert(hits<=1);row.same_saved_instance=hits==1;row.source_processed=input(ref.gem)
 end
 table.sort(records,function(a,b)return a.source<b.source end)
 local definitions={}
 for _,id in ipairs({"Metadata/Items/Gems/SkillGemAscendancySummonSandDjinn","Metadata/Items/Gems/SkillGemAscendancySummonWaterDjinn"})do
  local gem=assert(build.data.gems[id]);local levels={}
  for level in pairs(gem.grantedEffect.levels)do levels[#levels+1]=level end;table.sort(levels)
  definitions[#definitions+1]={id=id,effect=gem.grantedEffect.id,natural_max_level=gem.naturalMaxLevel,levels=levels}
 end
 djinnRawResult={expected_count=expectedCount,captured_count=#records,hook_removed=debug.gethook()==nil,records=records,definitions=definitions,
  observation_boundary="original LoadSkill -> original ProcessSocketGroup call",business_wrappers=false}
end
