//! Exact upstream support preparation/effect observations for real original builds.
//! This optional oracle does not grant native coverage or emulate support routing.
#![cfg(not(target_arch = "wasm32"))]
use mlua::Table;
#[path = "support/item_loading_runtime.rs"]
#[allow(dead_code)]
mod runtime;

fn oracle() -> runtime::Oracle {
    let oracle = runtime::Oracle::new();
    oracle
        .lua
        .load(
            r#"
-- The reused item's host resolves require() only under runtime/lua. CalcActiveSkill
-- requires one src module: load its authenticated original body via LoadModule and
-- temporarily supply that exact returned table for this one module name.
local originalRequire=require
sourceSupportCalcs=LoadModule("Modules/CalcBase")
require=function(name)
 if name=="Modules.CalcBase" then return sourceSupportCalcs end
 return originalRequire(name)
end
LoadModule("Modules/CalcActiveSkill")
require=originalRequire
-- Inputs to complete original createActiveSkill; no applicability is implemented here.
function sourceSupportInstance(id)
 local effect = assert(data.skills[id], id)
 local gem = data.gemForSkill[effect]
 return {grantedEffect=effect, gemData=gem and data.gems[gem],
  level=1, quality=0, actorLevel=90, statSet={index=1}, statSetCalcs={index=1}}
end
function sourceSupportActive(id, supportIds, summonSkill)
 local effects={}
 for _,supportId in ipairs(supportIds) do
  table.insert(effects,sourceSupportInstance(supportId))
 end
 local actor={enemy={}}
 actor.enemy.player=actor
 return sourceSupportCalcs.createActiveSkill(
  sourceSupportInstance(id),effects,{mode="MAIN"},actor,nil,summonSkill)
end
function sourceSupportMods(id)
 local list=new("ModList"):ModList()
 sourceSupportCalcs.mergeSkillInstanceMods({},list,sourceSupportInstance(id))
 return list
end
"#,
        )
        .set_name("@support-source-observation-inputs")
        .exec()
        .unwrap();
    oracle
}

#[test]
fn original_support_ids_resolve_by_game_id_and_variant_without_name_aliases() {
    let oracle = oracle();
    for (number, xml) in [
        (
            1,
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-01.xml"),
        ),
        (
            2,
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-02.xml"),
        ),
        (
            5,
            include_str!("../../../tests/fixtures/builds/breadth-20260908/build-05.xml"),
        ),
    ] {
        oracle.lua.globals().set("inputXml", xml).unwrap();
        oracle.lua.globals().set("originalNumber", number).unwrap();
        let observation: Table = oracle
            .lua
            .load(
                r#"
local roots,err=originalXml.ParseXML(inputXml);assert(roots,err)
local expected = originalNumber==1 and "SupportMeatShieldPlayerTwo"
 or originalNumber==2 and "SupportElementalArmamentPlayerTwo"
 or "SupportFeedingFrenzyPlayer"
local count,selectedCount,selectedTwister=0,0,0
local function walk(node,selectedSet,currentSet,target)
 if type(node)~="table" then return end
 if node.elem=="Skills" then selectedSet=node.attrib.activeSkillSet end
 if node.elem=="SkillSet" then currentSet=node.attrib.id end
 if node.elem=="Skill" then
  for _,child in ipairs(node) do
   if type(child)=="table" and child.elem=="Gem" then target=child.attrib.skillId;break end
  end
 end
 if node.elem=="Gem" and node.attrib.skillId==expected then
  local a=node.attrib
  local gem=assert(assert(data.gemsByGameId[a.gemId])[a.variantId])
  assert(gem.grantedEffectId==a.skillId and gem.grantedEffect.id==a.skillId)
  assert(gem.name==a.nameSpec and a.enabled=="true")
  count=count+1
  if currentSet==selectedSet then
   selectedCount=selectedCount+1
   if target=="TwisterPlayer" then selectedTwister=selectedTwister+1 end
  end
  if originalNumber==2 then
   assert(a.gemId=="Metadata/Items/Gems/SupportGemPrimalArmamentTwo")
   assert(a.variantId=="ElementalArmamentSupportTwo")
   assert(a.nameSpec=="Elemental Armament II")
   assert(gem.id=="Metadata/Items/Gems/SkillGemElementalArmamentSupportTwo")
  end
 end
 for _,child in ipairs(node) do walk(child,selectedSet,currentSet,target) end
end
walk(roots[1])
return {count=count,selected=selectedCount,twister=selectedTwister}
"#,
            )
            .set_name("@original-support-identity-observation")
            .eval()
            .unwrap();
        let expected = match number {
            1 => (1, 1, 0),
            2 => (7, 1, 1),
            5 => (2, 0, 0),
            _ => unreachable!(),
        };
        assert_eq!(observation.get::<u32>("count").unwrap(), expected.0);
        assert_eq!(observation.get::<u32>("selected").unwrap(), expected.1);
        assert_eq!(observation.get::<u32>("twister").unwrap(), expected.2);
    }
}

#[test]
fn upstream_activation_distinguishes_attacks_minion_types_and_exclusions() {
    let oracle = oracle();
    oracle
        .lua
        .load(
            r#"
local elemental="SupportElementalArmamentPlayerTwo"
local meat="SupportMeatShieldPlayerTwo"
local frenzy="SupportFeedingFrenzyPlayer"
local function has(skill,id)
 for _,effect in ipairs(skill.effectList) do
  if effect.grantedEffect.id==id then return true end
 end
 return false
end
for _,warm in ipairs({false,true}) do
 if warm then jit.on() else jit.off();jit.flush() end
 local twister=sourceSupportActive("TwisterPlayer",{elemental,meat,frenzy})
 assert(#twister.effectList==2 and has(twister,elemental))
 assert(not has(twister,meat) and not has(twister,frenzy))
 local spark=sourceSupportActive("SparkPlayer",{elemental,meat,frenzy})
 assert(#spark.effectList==1)
 local cleric=sourceSupportActive("SummonSkeletalClericsPlayer",{meat,frenzy})
 assert(#cleric.effectList==3 and has(cleric,meat) and has(cleric,frenzy))
 local wolf=sourceSupportActive("WolfPackPlayer",{elemental,meat,frenzy})
 assert(not wolf.skillTypes[SkillType.Attack])
 assert(wolf.minionSkillTypes[SkillType.Attack])
 assert(#wolf.effectList==4 and has(wolf,elemental))
 -- Requirements include a summoner's declared minion types; exclusions use the
 -- summoner's own types. Exercise complete original createActiveSkill for a child.
 local child=sourceSupportActive("MeleeAtAnimationSpeed",{elemental,meat,frenzy},wolf)
 assert(#child.effectList==4 and child.summonSkill==wolf)
 local raven=sourceSupportActive("SummonSpiralingConspiracyPlayer",{meat,frenzy})
 assert(raven.skillTypes[SkillType.MinionsAreUndamagable])
 assert(#raven.effectList==1)
 -- A real source pair: Arcane Surge contributes Duration to Firebolt, enabling
 -- Prolonged Duration even when that support was initially rejected.
 local duration="ProlongedDurationSupportPlayer"
 local surge="SupportArcaneSurgePlayer"
 local noDuration=sourceSupportActive("FireboltPlayer",{duration})
 assert(not noDuration.skillTypes[SkillType.Duration] and #noDuration.effectList==1)
 for _,supports in ipairs({{duration,surge},{surge,duration}}) do
  local expanded=sourceSupportActive("FireboltPlayer",supports)
  assert(expanded.skillTypes[SkillType.Duration] and #expanded.effectList==3)
  assert(has(expanded,duration) and has(expanded,surge))
 end
 -- Added types can also invalidate another previously eligible support. This
 -- pair uses distinct gem families; preparation is not assumed monotone.
 local brain="SupportBrutusBrainPlayer"
 for _,supports in ipairs({{frenzy,brain},{brain,frenzy}}) do
  local invulnerable=sourceSupportActive("WolfPackPlayer",supports)
  assert(invulnerable.skillTypes[SkillType.MinionsAreUndamagable])
  assert(#invulnerable.effectList==2 and has(invulnerable,brain))
  assert(not has(invulnerable,frenzy))
 end
 -- An eligible attachment is a local effect-list outcome, not global admission.
 assert(#sourceSupportActive("TwisterPlayer",{}).effectList==1)
end
"#,
        )
        .set_name("@source-support-applicability-assertions")
        .exec()
        .unwrap();
}

#[test]
fn original_modifier_merge_retains_attack_filter_and_minion_wrapper_identity() {
    let oracle = oracle();
    oracle
        .lua
        .load(
            r#"
local elemental="SupportElementalArmamentPlayerTwo"
local list=sourceSupportMods(elemental)
assert(#list==1)
assert(list[1].name=="ElementalDamage" and list[1].type=="MORE")
assert(list[1].value==25 and list[1].keywordFlags==KeywordFlag.Attack)
assert(list[1].source=="Skill:"..elemental)
assert(list:More({keywordFlags=KeywordFlag.Attack},"ElementalDamage")==1.25)
assert(list:More({keywordFlags=KeywordFlag.Spell},"ElementalDamage")==1)
assert(data.skills[elemental].levels[1].manaMultiplier==20)
-- The actual modifier list retains typed nested minion effects. Do not flatten
-- these into player Damage or claim that this observes final minion routing.
for _,case in ipairs({
 {id="SupportMeatShieldPlayerTwo",damage=-40,taken=-40},
 {id="SupportFeedingFrenzyPlayer",damage=30,taken=20},
}) do
 local mods=sourceSupportMods(case.id)
 assert(#mods==2)
 assert(mods:More(nil,"Damage")==1 and mods:More(nil,"DamageTaken")==1)
 local nested={}
 for _,mod in ipairs(mods) do
  assert(mod.name=="MinionModifier" and mod.type=="LIST")
  assert(mod.source=="Skill:"..case.id)
  local child=assert(mod.value.mod)
  assert(child.type=="MORE")
  assert(not nested[child.name]);nested[child.name]=child.value
 end
 assert(nested.Damage==case.damage and nested.DamageTaken==case.taken)
end
"#,
        )
        .set_name("@source-support-effect-assertions")
        .exec()
        .unwrap();
}
#[test]
fn original_retry_algorithm_has_a_sparse_rejection_list_order_boundary() {
    let oracle = oracle();
    oracle
        .lua
        .load(
            r#"
-- Synthetic algorithm witnesses, not game definitions or native content. All
-- require/add pairs below are explicit test inputs to unchanged upstream code.
local function synthetic(id,requires,adds)
 local effect=sourceSupportInstance("SupportElementalArmamentPlayerTwo")
 -- Loaded definitions contain cyclic cross-links. Retain original nested data;
 -- only the explicit synthetic top-level identity/type vectors are replaced.
 local original=effect.grantedEffect
 effect.grantedEffect={}
 for key,value in pairs(original) do effect.grantedEffect[key]=value end
 effect.grantedEffect.id=id
 effect.grantedEffect.requireSkillTypes={requires}
 effect.grantedEffect.addSkillTypes={adds}
 effect.grantedEffect.excludeSkillTypes={}
 effect.gemData=nil
 return effect
end
local function run(order)
 local definitions={
  first=synthetic("test-first",SkillType.Duration,SkillType.GeneratesRemnants),
  delayed=synthetic("test-delayed",SkillType.CreatesFissure,SkillType.ConsumesRage),
  producer=synthetic("test-producer",SkillType.Duration,SkillType.CreatesFissure),
  consumer=synthetic("test-consumer",SkillType.ConsumesRage,SkillType.Area),
  seed=synthetic("test-seed",SkillType.Spell,SkillType.Duration),
 }
 local supports={}
 for _,id in ipairs(order) do table.insert(supports,definitions[id]) end
 return sourceSupportCalcs.createActiveSkill(sourceSupportInstance("FireboltPlayer"),
  supports,{mode="MAIN"},{},nil,nil)
end
local sparse=run({"first","delayed","producer","consumer","seed"})
local contiguous=run({"producer","delayed","first","consumer","seed"})
-- In the sparse case, retry index 1 succeeds and becomes nil. The later producer
-- enables 'delayed', but the next ipairs pass stops at that nil before revisiting
-- it. Final effect-list admission can include delayed without applying its type.
assert(sparse.skillTypes[SkillType.Duration] and sparse.skillTypes[SkillType.CreatesFissure])
assert(not sparse.skillTypes[SkillType.ConsumesRage])
assert(contiguous.skillTypes[SkillType.ConsumesRage])
local function contains(skill,id)
 for _,effect in ipairs(skill.effectList) do
  if effect.grantedEffect.id==id then return true end
 end
 return false
end
assert(contains(sparse,"test-delayed") and not contains(sparse,"test-consumer"))
assert(contains(contiguous,"test-delayed") and contains(contiguous,"test-consumer"))
assert(#sparse.effectList==5 and #contiguous.effectList==6)
"#,
        )
        .set_name("@synthetic-input-original-support-retry-boundary")
        .exec()
        .unwrap();
}
