//! Real support modifier components against complete authenticated PoB functions.
//! Empty surroundings and explicit finite receivers do not prove whole-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/owned_support_output_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_support_output_runtime.rs"]
mod source;
use mlua::Table;
use poe_optimizer_core::owned_build::SupportReceiverKey;
use poe_optimizer_engine::owned_plan::{ConcreteEntity, RuleOrigin};
use std::collections::BTreeSet;

fn facts(observed: &Table, receiver: native::Receiver) -> native::Facts {
    let types: Table = observed.get("types").unwrap();
    native::Facts {
        types: native::TYPES.map(|name| types.get(name).unwrap()),
        minion_types_present: observed.get("minionTypesPresent").unwrap(),
        percent: observed.get("percent").unwrap(),
        receiver,
    }
}

#[test]
fn original_twister_and_cleric_modifier_channels_are_separate() {
    for warm in [false, true] {
        let oracle = source::oracle(warm);
        oracle
            .lua
            .load(
                r#"
local twister=outputObserve("TwisterPlayer",{"SupportElementalArmamentPlayerTwo"})
assert(twister.active.skillTypes[SkillType.Attack])
assert(bit.band(twister.active.skillCfg.keywordFlags,KeywordFlag.Attack)~=0)
assert(twister.active.skillModList:More(twister.active.skillCfg,"ElementalDamage")==1.25)
assert(twister.active.skillModList:More({keywordFlags=KeywordFlag.Spell},"ElementalDamage")==1)
local cleric=outputCleric({"SupportMeatShieldPlayerTwo"})
local minion=cleric.active.minion
assert(minion.type=="RaisedSkeletonCleric")
assert(cleric.active.skillModList:More(cleric.active.skillCfg,"DamageTaken")==1)
assert(cleric.active.skillModList:More(cleric.active.skillCfg,"Damage")==1)
assert(minion.modDB:More(nil,"Damage")==0.6)
assert(minion.modDB:More(nil,"DamageTaken")==0.6)
assert(#minion.activeSkillList==1)
local child=minion.activeSkillList[1]
assert(child.activeEffect.grantedEffect.id=="HealSkeletonClericMinion")
assert(child.supportList==cleric.active.supportList)
assert(child.skillModList:More(child.skillCfg,"Damage")==0.6)
assert(child.skillModList:More(child.skillCfg,"DamageTaken")==0.6)
assert(bit.band(child.skillCfg.keywordFlags,KeywordFlag.Attack)==0)
local unsupported=outputCleric({})
assert(unsupported.active.minion.modDB:More(nil,"Damage")==1)
assert(unsupported.active.minion.modDB:More(nil,"DamageTaken")==1)
"#,
            )
            .set_name("@real-support-output-source-components")
            .exec()
            .unwrap();
    }
}

#[test]
fn real_armament_action_factor_matches_original_final_type_keyword_filter() {
    for warm in [false, true] {
        let oracle = source::oracle(warm);
        for controlled in [false, true] {
            oracle
                .lua
                .globals()
                .set("controlledTypeContrast", controlled)
                .unwrap();
            let observed:Table=oracle.lua.load(r#"
local types=copyTable(data.skills.TwisterPlayer.skillTypes)
if controlledTypeContrast then
 -- Explicit synthetic contrast: remain supportable via CrossbowAmmoSkill, but
 -- query this receiving action without Attack. This is not a real Twister mode.
 types[SkillType.Attack]=nil
 types[SkillType.CrossbowAmmoSkill]=true
 types[SkillType.Spell]=true
end
local result=outputObserve("TwisterPlayer",{"SupportElementalArmamentPlayerTwo"},types)
assert(#result.active.effectList==2)
return {
 percent=outputConstants("SupportElementalArmamentPlayerTwo")["support_attack_skills_elemental_damage_+%_final"],
 types={attack=not not types[SkillType.Attack],ammo=not not types[SkillType.CrossbowAmmoSkill],
  minion=not not types[SkillType.CreatesMinion],undamageable=not not types[SkillType.MinionsAreUndamagable]},
 minionTypesPresent=false,
 factor=result.active.skillModList:More(result.active.skillCfg,"ElementalDamage")}
"#).set_name("@real-armament-and-controlled-type-contrast").eval().unwrap();
            let report = native::evaluate(facts(&observed, native::Receiver::Action));
            let expected: f64 = observed.get("factor").unwrap();
            let receivers: BTreeSet<_> = report
                .effects
                .iter()
                .filter_map(|effect| {
                    let RuleOrigin::SupportApplication { application } =
                        &effect.key.invocation.origin
                    else {
                        return None;
                    };
                    let SupportReceiverKey::Action(action) = &application.receiver else {
                        return None;
                    };
                    Some(ConcreteEntity::Action(action.clone()))
                })
                .collect();
            assert_eq!(receivers.len(), 2, "two controlled output selections");
            let factors = native::factors(&report, "elemental-damage");
            for receiver in &receivers {
                let matching: Vec<_> = factors
                    .iter()
                    .filter(|(entity, _)| entity == receiver)
                    .collect();
                assert_eq!(matching.len(), 1, "one final factor for {receiver:?}");
                assert_eq!(matching[0].1, expected);
            }
            assert!(
                factors
                    .iter()
                    .any(|(entity, value)| { !receivers.contains(entity) && *value == 1.0 }),
                "an unrelated receiver supplies an explicit identity control"
            );
            for (entity, value) in factors {
                assert_eq!(
                    value,
                    if receivers.contains(&entity) {
                        expected
                    } else {
                        1.0
                    },
                    "{entity:?}"
                );
            }
        }
    }
}

#[test]
fn real_meat_shield_factors_transfer_once_to_exact_child_actor() {
    for warm in [false, true] {
        let oracle = source::oracle(warm);
        let observed:Table=oracle.lua.load(r#"
local result=outputCleric({"SupportMeatShieldPlayerTwo"})
local raw=data.skills.SummonSkeletalClericsPlayer
local constants=outputConstants("SupportMeatShieldPlayerTwo")
return {
 percent=constants["support_minion_defensive_stance_minion_damage_taken_+%_final"],
 damagePercent=constants["support_meat_shield_minion_damage_+%_final"],
 types={attack=not not raw.skillTypes[SkillType.Attack],ammo=not not raw.skillTypes[SkillType.CrossbowAmmoSkill],
  minion=not not raw.skillTypes[SkillType.CreatesMinion],undamageable=not not raw.skillTypes[SkillType.MinionsAreUndamagable]},
 minionTypesPresent=raw.minionSkillTypes~=nil,
 damage=result.active.minion.modDB:More(nil,"Damage"),
 damageTaken=result.active.minion.modDB:More(nil,"DamageTaken"),
 parentDamage=result.env.player.modDB:More(nil,"Damage")}
"#).set_name("@real-meat-shield-transferred-actor-factors").eval().unwrap();
        assert_eq!(
            observed.get::<f64>("percent").unwrap(),
            observed.get::<f64>("damagePercent").unwrap()
        );
        let report = native::evaluate(facts(&observed, native::Receiver::SummonedActor));
        for (stat, source_key) in [("damage", "damage"), ("damage-taken", "damageTaken")] {
            let expected: f64 = observed.get(source_key).unwrap();
            let values = native::factors(&report, stat);
            assert!(
                values.iter().any(
                    |(entity, value)| *entity == native::expected_child() && *value == expected
                )
            );
            assert!(
                values
                    .iter()
                    .any(|(entity, value)| *entity == native::parent()
                        && *value == observed.get::<f64>("parentDamage").unwrap())
            );
            for (entity, value) in values {
                assert_eq!(
                    value,
                    if entity == native::expected_child() {
                        expected
                    } else {
                        1.0
                    },
                    "{entity:?}"
                );
            }
        }
    }
}
