-- Read-only writer inventory for the cfg Area bit, not final skillFlags/radius.
assert(djinnSupportAuth.finished and debug.gethook()==nil)
local data=build.data
local calcs=require("Modules.CalcBase")
local work=0
local function charge()work=work+1;assert(work<=2000000,"Area census work bound")end
local watched,seen={},{}
local function watch(t)
 if not t or seen[t]then return end
 assert(type(t)=="table");seen[t]=true
 local row={object=t,meta=getmetatable(t),entries={},count=0};watched[#watched+1]=row
 for k,v in pairs(t)do charge();row.entries[k]=v;row.count=row.count+1 end
end
local function keys(t)
 watch(t);local out={};for k in pairs(t or{})do charge();assert(type(k)=="string");out[#out+1]=k end
 table.sort(out);return out
end
local function flags(t)
 local out={};for _,k in ipairs(keys(t))do local v=rawget(t,k);assert(type(v)=="boolean");out[#out+1]={name=k,value=v}end
 return{present=t~=nil,entries=out}
end
local methods,refs={},{}
local function original(owner,key,path,line)
 local f=assert(owner[key]);local i=debug.getinfo(f,"S");local p=i.source:gsub("\\","/")
 assert(i.what=="Lua"and p:sub(-#path)==path and i.linedefined==line)
 methods[#methods+1]={name=key,path=path,first=i.linedefined,last=i.lastlinedefined};refs[#refs+1]={owner,key,f}
end
original(calcs,"createActiveSkill","Modules/CalcActiveSkill.lua",144)
original(calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426)
original(calcLib,"buildSkillInstanceStats","Modules/CalcTools.lua",161)
original(data.skillStatMapMeta,"__index","Modules/Data.lua",914)
watch(data);watch(data.skills);watch(data.skillStatMapMeta)
local function mapInventory(t,owner)
 assert(type(t)=="table");watch(t)
 if owner then assert(getmetatable(t)==data.skillStatMapMeta and rawget(t,"_grantedEffect")==owner)end
 local out={}
 for _,name in ipairs(keys(t))do if name~="_grantedEffect"then
  local map=rawget(t,name);assert(type(map)=="table");watch(map)
  local flag=rawget(map,"skillFlag");assert(flag==nil or type(flag)=="string")
  out[#out+1]={stat=name,skill_flag_present=flag~=nil,skill_flag=flag}
 end end
 return out
end
local catalog,supportCount,nestedAddFlags={},0,{}
for _,id in ipairs(keys(data.skills))do
 local effect=assert(rawget(data.skills,id));watch(effect);assert(effect.id==id)
 local support=effect.support==true;if support then supportCount=supportCount+1 end
 catalog[#catalog+1]={id=id,support=support,add_flags=flags(rawget(effect,"addFlags"))}
 watch(effect.statSets)
 for index,set in ipairs(effect.statSets)do watch(set);charge()
  if rawget(set,"addFlags")~=nil then nestedAddFlags[#nestedAddFlags+1]={effect=id,index=index,add_flags=flags(rawget(set,"addFlags"))}end
 end
end
local requested={"ChilledGroundBurstWaterDjinn","ChilledGroundOasisConvertWaterDjinn",
 "CommandSandDjinnKnifeThrowPlayer","CommandWaterDjinnBubblePlayer","ESRechargeForceRestartWaterDjinn",
 "ExplosiveTeleportSandDjinn","HandSlamSandDjinn","IceNovaPlayer","KnifeThrowSandDjinn",
 "PassiveTriggeredManaWaveWaterDjinn","WaterBubbleWaterDjinn"}
local requestedSet,definitions={},{}
local function statNames(t,paired)
 watch(t);local out={};for i,v in ipairs(t or{})do charge();if paired then watch(v);v=v[1]end
  assert(type(v)=="string");out[#out+1]={index=i,stat=v}
 end;return out
end
for _,id in ipairs(requested)do
 requestedSet[id]=true;local g=assert(rawget(data.skills,id));watch(g);watch(g.statSets)
 local parts={};watch(g.parts)
 for i,part in ipairs(g.parts or{})do
  watch(part);local fields={};for _,k in ipairs(keys(part))do local v=rawget(part,k)
   assert(type(v)=="boolean"or type(v)=="string"or type(v)=="number");fields[#fields+1]={name=k,value=v}
  end;parts[#parts+1]={index=i,fields=fields}
 end
 local sets={}
 for i,s in ipairs(g.statSets)do
  watch(s);sets[#sets+1]={index=i,id=s.id,label=s.label,scope=s.statDescriptionScope,
   base_flags=flags(s.baseFlags),nested_add_flags=flags(rawget(s,"addFlags")),
   stats=statNames(s.stats,false),constants=statNames(s.constantStats,true),local_map=mapInventory(s.statMap,g)}
 end
 definitions[#definitions+1]={effect=id,parts_present=g.parts~=nil,parts=parts,
  quality_stats=statNames(g.qualityStats,true),alt_quality_stats=statNames(g.altQualityStats,true),
  root_map=mapInventory(g.statMap,g),stat_sets=sets,root_fallback_exact=true}
end
local observations={}
local function observe(active,mode,parentIndex,childIndex)
 local effect=active.activeEffect.grantedEffect;if not requestedSet[effect.id]then return end
 assert(data.skills[effect.id]==effect)
 local set=mode=="MAIN"and active.activeEffect.statSet or active.activeEffect.statSetCalcs
 assert(effect.statSets[set.index]==set.statSet);watch(set);watch(set.skillFlags);watch(set.statSet.baseFlags)
 local cfg=active.skillCfg;watch(cfg)
 local r={mode=mode,parent_index=parentIndex,child_index=childIndex,effect=effect.id,stat_set_index=set.index,
  stat_set_id=set.statSet.id,definition_base_area=set.statSet.baseFlags.area==true,
  later_area_present=set.skillFlags.area~=nil,later_area=set.skillFlags.area,
  cfg_available=cfg~=nil,exact_definition=true,exact_stat_set=true}
 if cfg then assert(cfg.skillGrantedEffect==effect);r.cfg_flags=cfg.flags;r.cfg_area=AND64(cfg.flags,ModFlag.Area)~=0 end
 observations[#observations+1]=r
end
for _,mode in ipairs({"MAIN","CALCS"})do
 local env=assert(mode=="MAIN"and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
 for i,active in ipairs(env.player.activeSkillList)do
  observe(active,mode,i,nil)
  if active.minion then for j,child in ipairs(active.minion.activeSkillList)do
   assert(child.summonSkill==active and child.actor==active.minion);observe(child,mode,i,j)
  end end
 end
end
local globalMap=mapInventory(data.skillStatMap,nil)
for _,r in ipairs(watched)do
 assert(getmetatable(r.object)==r.meta);local n=0
 for k,v in pairs(r.object)do charge();n=n+1;assert(r.entries[k]==v,"Area observer mutated source")end
 assert(n==r.count)
end
for _,r in ipairs(refs)do assert(r[1][r[2]]==r[3])end
return{catalog=catalog,support_count=supportCount,effect_count=#catalog,nested_add_flags=nestedAddFlags,global_map=globalMap,definitions=definitions,
 observations=observations,methods=methods,area_flag_value=ModFlag.Area,work=work,
 immutable_snapshot={tables=#watched,verified=true},business_wrappers=false,source_tables_mutated=false,
 source_cfg_modified=false,original_methods_preserved=true,jit_mode_preserved=jit.status()==djinnJit,
 meaning="Area-modifier eligibility at original skillCfg creation; later mutable flags are separate diagnostics"}
