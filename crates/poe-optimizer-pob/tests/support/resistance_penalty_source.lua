-- Read original databases after complete load; exactly two normal rebuilds.
assert(jit.status()==penaltyJit)
local cfg=build.configTab
local function snapshot()
 local result={}
 for _,mode in ipairs({'MAIN','CALCS'})do
  local env=assert(mode=='MAIN'and build.calcsTab.mainEnv or build.calcsTab.calcsEnv)
  assert(env.configInput==cfg.input)
  local stats={}
  for _,name in ipairs({'FireResist','ColdResist','LightningResist','ChaosResist'})do
   local records={}
   local db,depth=env.modDB,0
   while db do
    depth=depth+1;assert(depth<16)
    for _,m in ipairs(db.mods[name]or{})do
     if m.source=='Base'and m.type=='BASE'then
      assert(#m==0 and m.flags==0 and m.keywordFlags==0)
      records[#records+1]=m.value
     end
    end
    db=db.parent
   end
   assert(#records==1,'expected exact base contribution '..name)
   stats[name]=records
  end
  result[mode]=stats
 end
 return result
end
local stages={snapshot()}
for i=1,2 do build.calcsTab:BuildOutput();stages[#stages+1]=snapshot()end
assert(jit.status()==penaltyJit)
return {input=cfg.input.resistancePenalty,placeholder=cfg.placeholder.resistancePenalty,stages=stages}
