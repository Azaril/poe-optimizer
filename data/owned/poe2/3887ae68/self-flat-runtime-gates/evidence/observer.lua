-- Focused original-runtime observations. Helpers are the authenticated prefix
-- of the existing physical-damage observer; no business method is replaced.
return function(h)
 local calcs = require("Modules.CalcBase")
 local refs = {
  {calcs,"offence","Modules/CalcOffence.lua",527},
  {calcs,"buildActiveSkillModList","Modules/CalcActiveSkill.lua",426},
  {calcs,"perform","Modules/CalcPerform.lua",1193},
  {common.classes.ModStore,"Flag","Classes/ModStore.lua",281},
  {common.classes.ModStore,"List","Classes/ModStore.lua",321},
  {common.classes.ModStore,"Tabulate","Classes/ModStore.lua",345},
  {common.classes.ModStore,"EvalMod","Classes/ModStore.lua",490},
  {common.classes.ModDB,"FlagInternal","Classes/ModDB.lua",297},
  {common.classes.ModList,"FlagInternal","Classes/ModList.lua",229},
  {common.classes.ModDB,"ListInternal","Classes/ModDB.lua",391},
  {common.classes.ModList,"ListInternal","Classes/ModList.lua",309},
  {common.classes.ModDB,"AddList","Classes/ModDB.lua",113},
 }
 for _,r in ipairs(refs) do r[5]=h.original(r[1][r[2]],r[3],r[4]) end
 local merge = h.original(h.upvalue(calcs.perform,"mergeBuff"),"Modules/CalcPerform.lua",42)
 local result = {parent_weapon={},hollow_palm={},flag_calls={},aura_queries={},player_visits={},player_buff_visits={},child_visits={},child_buff_visits={},child_merges={},group_deliveries={}}
 local parentRows,hollowRows = {},{}
 local function append(name,row)
  local out=result[name];assert(#out<4096,"bounded self-flat observation count")
  row.index=#out+1;out[#out+1]=row;return row
 end
 local function mods(list)
  local out={};for _,mod in ipairs(list or {}) do out[#out+1]=h.modRecord(mod) end;return out
 end
 local function present(value) return {present=value~=nil,value=h.scalar(value)} end
 local function action(env,active)
  local actor=assert(active.actor)
  return {mode=env.mode,effect_id=active.activeEffect.grantedEffect.id,actor_profile=actor.type,
   actor_is_player=actor==env.player,actor_is_selected_minion=actor==env.minion,
   actor_parent_is_player=actor.parent==env.player,
   summoner_owns_actor=active.summonSkill and active.summonSkill.minion==actor or false,
   source=h.sourceOccurrence(active.summonSkill or active)}
 end
 local function selected(active)
  return active and active.actor and active.actor.type=="RaisedSkeletonSniper"
   and active.activeEffect.grantedEffect.id=="MinionMeleeBow"
 end
 local function gate(store,cfg,name)
  local checked=h.watchStores({store});local oldCfg=h.clone(cfg)
  local row={name=name,raw=h.rawRecords(store,{[name]=true}),
   supplemental_original_flag=present(store:Flag(cfg,name)),
   eligible=h.records(store,"FLAG",cfg,name)}
  checked();assert(h.equal(cfg,oldCfg));row.query_state_preserved=true;return row
 end
 local function payloads(values)
  local out={}
  for _,value in ipairs(values) do
   for key,v in pairs(value) do assert(key=="mod" or type(v)~="table","unreviewed Aura payload table") end
   out[#out+1]={fields=h.scalars(value),mod=value.mod and h.modRecord(value.mod)}
  end
  return out
 end
 local function buffRow(buff)
  return {fields=h.scalars(buff),modifiers=mods(buff.modList)}
 end
 local oldHook,oldMask,oldCount=debug.gethook();assert(oldHook==nil)
 local function hook(event,line)
  local f=debug.getinfo(2,"f").func
  local interesting = event=="line" and ((f==calcs.offence and (line==820 or line==821))
   or (f==calcs.buildActiveSkillModList and (line==887 or line==888))
   or (f==calcs.perform and (line==2119 or line==2122 or line==2588 or line==2591)))
   or event=="call" and f==common.classes.ModStore.Flag
   or event=="return" and (f==common.classes.ModStore.List or f==merge or f==common.classes.ModDB.AddList)
  if not interesting then return end
  local vars={};for i=1,192 do local name,value=debug.getlocal(2,i);if not name then break end;vars[name]=value end
  if event=="call" and f==common.classes.ModStore.Flag then
   local caller=debug.getinfo(3,"fl")
   if not caller or not ((caller.func==calcs.offence and caller.currentline==820) or (caller.func==calcs.buildActiveSkillModList and caller.currentline==887)) then return end
   local outer={};for i=1,192 do local name,value=debug.getlocal(3,i);if not name then break end;outer[name]=value end
   if not selected(outer.activeSkill) then return end
   local active,env=outer.activeSkill,outer.env;local row=action(env,active)
   row.line=caller.currentline;row.original_flag_call_observed=true
   if caller.func==calcs.offence then
    assert(vars.self==active.actor.modDB and vars.cfg==nil)
    row.name_from_pinned_callsite="GainMainHandDmgFromParent";row.gate_index=assert(parentRows[active]).index
   else
    assert(vars.self==active.skillModList and (vars.cfg==nil or vars.cfg==active.skillCfg))
    row.name_from_pinned_callsite=vars.cfg==nil and "UseFacebreakerItemDamage" or "UseHollowPalmDamage"
    row.gate_index=assert(hollowRows[active]).index
   end
   row.cfg=h.sourceConfig(active,vars.cfg);append("flag_calls",row)
  elseif f==calcs.offence and selected(vars.activeSkill) then
   local active,env,actor=vars.activeSkill,vars.env,vars.actor
   assert(actor==active.actor and vars.modDB==actor.modDB and vars.skillModList==active.skillModList)
   if line==820 then
    local row=action(env,active)
    row.line=line;row.original_branch_entered=false;row.parent_present=actor.parent~=nil
    row.parent_main_hand_present=actor.parent and actor.parent.itemList["Weapon 1"]~=nil or false
    row.exact_actor_store=true;row.flag=gate(actor.modDB,nil,"GainMainHandDmgFromParent")
    row.parent_weapon=actor.parent and h.scalars(actor.parent.weaponData1)
    parentRows[active]=append("parent_weapon",row)
   else assert(parentRows[active]).original_branch_entered=true end
  elseif f==calcs.buildActiveSkillModList and selected(vars.activeSkill) then
   local active,env=vars.activeSkill,vars.env
   assert(vars.skillModList==active.skillModList)
   if line==887 then
    local row=action(env,active);row.line=line;row.original_branch_entered=false
    row.actor_condition=present(active.actor.modDB.conditions.HollowPalm)
    row.weapon_types=h.scalars(active.activeEffect.grantedEffect.weaponTypes)
    row.cfg=h.sourceConfig(active,active.skillCfg)
    row.facebreaker=gate(active.skillModList,nil,"UseFacebreakerItemDamage")
    row.hollow_palm=gate(active.skillModList,active.skillCfg,"UseHollowPalmDamage")
    hollowRows[active]=append("hollow_palm",row)
   else assert(hollowRows[active]).original_branch_entered=true end
  elseif f==calcs.perform then
   local player=line==2119 or line==2122
   local active=assert(player and vars.activeSkill or vars.activeMinionSkill);local env=assert(vars.env)
   if player then assert(active.actor==env.player) else assert(active.actor==vars.castingMinion and vars.activeSkill.minion==active.actor) end
   if line==2588 or line==2119 then
    local row=action(env,active);row.line=line;row.enable=present(active.skillData.enable)
    row.mode_buffs=not not env.mode_buffs;row.flags=h.scalars(active.skillFlags);row.buffs={}
    for _,buff in ipairs(active.buffList or {}) do row.buffs[#row.buffs+1]=buffRow(buff) end
    append(player and "player_visits" or "child_visits",row)
   else
    local row=action(env,active);row.line=line;row.enable=present(active.skillData.enable)
    row.mode_buffs=not not env.mode_buffs;row.buff=buffRow(assert(vars.buff));append(player and "player_buff_visits" or "child_buff_visits",row)
   end
  else
   local caller=debug.getinfo(3,"fl")
   if not caller or caller.func~=calcs.perform then return end
   local outer={};for i=1,192 do local name,value=debug.getlocal(3,i);if not name then break end;outer[name]=value end
   local env=assert(outer.env)
   if f==common.classes.ModStore.List and (caller.currentline==2266 or caller.currentline==2637 or caller.currentline==3169) then
    local name=caller.currentline==3169 and "ExtraAura" or "ExtraAuraEffect"
    local active=caller.currentline==2637 and outer.activeMinionSkill or outer.activeSkill
    local store=caller.currentline==2637 and outer.activeSkill.minion.modDB or env.player.modDB
    assert(vars.self==store and vars.n==1)
    if name=="ExtraAura" then assert(vars.cfg==nil) else assert(vars.cfg==active.skillCfg) end
    append("aura_queries",{line=caller.currentline,mode=env.mode,name=name,
     original_consumer_executed=true,original_list_return_observed=true,exact_store=true,
     store_is_player=store==env.player.modDB,source=name=="ExtraAuraEffect" and action(env,active) or nil,
     cfg=name=="ExtraAuraEffect" and h.sourceConfig(active,vars.cfg) or {present=false},
     raw=h.rawRecords(store,{[name]=true}),returned=payloads(assert(vars.result))})
   elseif f==merge and outer.activeMinionSkill and caller.currentline>=2585 and caller.currentline<=2795 then
    local destination=vars.destTable==outer.buffs and "player_buffs" or vars.destTable==outer.minionBuffs and "selected_minion_buffs" or vars.destTable==outer.debuffs and "enemy_debuffs"
    assert(destination,"unknown supplied-child merge destination")
    local row=action(env,outer.activeMinionSkill);row.line=caller.currentline;row.destination=destination
    row.buff_key=vars.destKey;row.source_modifiers=mods(vars.src);row.merged_modifiers=mods(vars.destTable[vars.destKey])
    row.selected_recipient=env.minion and h.recipientOccurrence(env,env.minion)
    append("child_merges",row)
   elseif f==common.classes.ModDB.AddList and (caller.currentline==3118 or caller.currentline==3128 or caller.currentline==3132) then
    local group=caller.currentline==3118 and outer.buffs or caller.currentline==3128 and outer.minionBuffs or outer.debuffs
    local names={};for name,value in pairs(group) do if value==vars.modList then names[#names+1]=name end end;table.sort(names);assert(#names==1)
    local expected=caller.currentline==3118 and env.player.modDB or caller.currentline==3128 and env.minion.modDB or outer.enemyDB
    assert(vars.self==expected)
    append("group_deliveries",{line=caller.currentline,mode=env.mode,buff_key=names[1],exact_recipient_store=true,
     recipient=caller.currentline==3118 and "player" or caller.currentline==3128 and "selected_minion" or "enemy",
     selected_recipient=caller.currentline==3128 and h.recipientOccurrence(env,env.minion) or nil,modifiers=mods(vars.modList)})
   end
  end
 end
 debug.sethook(hook,"crl")
 return function()
  assert(debug.gethook()==hook);debug.sethook(oldHook,oldMask,oldCount)
  for _,r in ipairs(refs) do assert(r[1][r[2]]==r[5]) end
  assert(h.upvalue(calcs.perform,"mergeBuff")==merge)
  result.original_functions_preserved=true;result.business_methods_replaced=false
  result.query_probes_are_supplemental=true;result.native_coverage_authorized=false
  result.selected={items=build.itemsTab.activeItemSetId,skills=build.skillsTab.activeSkillSetId,
   spec=build.treeTab.activeSpec,config=build.configTab.activeConfigSetId,main_group=build.mainSocketGroup}
  local wanted={GainMainHandDmgFromParent=true,UseHollowPalmDamage=true,UseFacebreakerItemDamage=true,
   ExtraAura=true,ExtraAuraEffect=true,MinionModifier=true,PhysicalMin=true,PhysicalMax=true}
  result.final_raw_stores={}
  for _,env in ipairs({build.calcsTab.mainEnv,build.calcsTab.calcsEnv}) do
   local actors={}
   for ordinal,summoner in ipairs(env.player.activeSkillList) do
    local actor=summoner.minion
    if actor then
     actors[#actors+1]={ordinal=ordinal,profile=actor.type,source=h.sourceOccurrence(summoner),
      raw=h.rawRecords(actor.modDB,wanted),conditions=h.scalars(actor.modDB.conditions)}
    end
   end
   result.final_raw_stores[#result.final_raw_stores+1]={mode=env.mode,player_raw=h.rawRecords(env.player.modDB,wanted),actors=actors}
  end
  result.snapshot=h.offeringOutputSnapshot()
  selfFlatGateObservation=result
 end
end
