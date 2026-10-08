//! Real Sniper item/preparation/population programs and intrinsic Actor Life.
//! Coverage is finite and test-only; real support-origin coverage remains open.
#[allow(dead_code)]
#[path = "owned_sniper_final_inputs_fixture.rs"]
mod fixture;
use super::{family, release};
use fixture::{World as Sniper, decode, def, id, key, offering, shared, stat, subject};
use poe_optimizer_core::{
    build_identity::*, owned_build::*, owned_definitions::*, owned_preset_intent::*,
    owned_project::*, owned_readiness::*, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use std::{
    path::PathBuf,
    sync::{Arc, OnceLock},
};

#[derive(Clone)]
struct World {
    sniper: Sniper,
    preferences: Vec<UsagePolicySelection>,
    overrides: Vec<UsagePolicySelection>,
    dormant: Vec<UsagePolicySelection>,
}
impl World {
    fn load() -> Self {
        static BASE: OnceLock<World> = OnceLock::new();
        BASE.get_or_init(||{
            let path=PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_SKILL_PARTICIPATION_RELEASE").expect("published participation package"));
            let endpoint=release::load(&path);family::assert_endpoint(&endpoint);
            let mut sniper=Sniper::load_release(&path);let e:Value=family::read("extension.json");
            for row in e["schema"].as_array().unwrap() {
                match row["kind"].as_str().unwrap() {
                    "definition" => offering::add_definition(&mut sniper.base.source, decode(&row["value"])),
                    "slot" => sniper.base.source.base.inner.schema.slots.push(decode(&row["value"])),
                    _ => unreachable!(),
                }
            }
            sniper.base.source.base.inner.owners.push(family::owner());
            // Retain the real independent count policy and intrinsic Life body.
            let recipe=&endpoint.input().recipe;
            for address in [def::<UsagePolicyDefinition>("def.000000000000326a").address(),stat(0x326c).address(),stat(0x311a).address(),def::<UnitDefinition>("def.0000000000003119").address()] {
                let d=recipe.schema.definitions.iter().find(|d|d.address()==address).unwrap().clone();
                offering::add_definition(&mut sniper.base.source, d);
            }
            let count_owner=subject(def::<UsagePolicyDefinition>("def.000000000000326a"));
            sniper.base.source.base.inner.owners.push(recipe.rules.owners.iter().find(|o|o.owner==count_owner).unwrap().clone());
            sniper.base.source.base.inner.schema.slots.push(recipe.schema.slots.iter().find(|s|matches!(s,SlotDescriptor::Parameter(e) if e.id.slot.key().as_str()=="def.000000000000326b")).unwrap().clone());
            let actor=SchemaSubject::Slot(SlotAddress::Actor(decode(&sniper.bindings["target"]["actor"])));
            let life=recipe.rules.owners.iter().find(|o|o.owner==actor).unwrap().programs.members.iter().find(|p|p.id.as_str()=="intrinsic-allied-minion-life").unwrap().clone();
            let tables:Vec<_>=life.nodes.iter().filter_map(|n|match &n.expression{RuleExpression::LookupIntegerTable{table,..}=>Some(table),_=>None}).collect();
            for table in tables{sniper.base.tables.push(recipe.rules.tables.iter().find(|t|&t.id==table).unwrap().clone());}
            sniper.base.source.base.inner.owner_mut(actor).programs.members.push(life);
            let mut w=Self{sniper,preferences:vec![],overrides:vec![],dormant:vec![]};
            for i in 0..2{w.preferences.push(w.usage(i,true,true));w.preferences.push(w.count(i,1));}w
        }).clone()
    }
    fn usage(&self, i: usize, group: bool, occurrence: bool) -> UsagePolicySelection {
        let b: Value = family::read("bindings.json");
        UsagePolicySelection {
            policy: decode(&b["policy"]),
            target: UsageTarget::Skill(SkillTarget::Generated(Box::new(self.sniper.generated(i)))),
            parameters: vec![
                ParameterAssignment {
                    slot: decode(&b["group"]),
                    value: ParameterValue::Boolean(group),
                },
                ParameterAssignment {
                    slot: decode(&b["occurrence"]),
                    value: ParameterValue::Boolean(occurrence),
                },
            ],
        }
    }
    fn count(&self, i: usize, count: i64) -> UsagePolicySelection {
        let b: Value = family::read("bindings.json");
        UsagePolicySelection {
            policy: decode(&b["count_policy"]),
            target: UsageTarget::Skill(SkillTarget::Generated(Box::new(self.sniper.generated(i)))),
            parameters: vec![ParameterAssignment {
                slot: decode(&b["count_parameter"]),
                value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
            }],
        }
    }
    fn composed(&self) -> std::result::Result<ComposedRequest, String> {
        let intent = |preferences: &[UsagePolicySelection]| SkillPresetIntentV1 {
            schema_version: 1,
            usage: preferences
                .iter()
                .cloned()
                .map(|selection| PresetUsageBinding {
                    selection,
                    applicability: PresetApplicability::Required,
                })
                .collect(),
            generated_inputs: vec![],
        };
        let original = self.sniper.base.source.request();
        let b = original.build().input();
        let limits = OwnedInputLimits::default();
        let selection = VariantSelection {
            character: id(10001),
            equipment: id(10002),
            allocations: id(10003),
            skills: id(10004),
            choices: id(10005),
            active_weapon_loadout: b.active_weapon_loadout,
        };
        let project = BuildProject::new(
            ProjectInput {
                allocator: InstanceAllocatorState::from_parts(b.allocator.lineage(), 10006),
                revision: b.revision,
                game_version: b.game_version.clone(),
                weapon_loadouts: b.weapon_loadouts.clone(),
                items: b.items.clone(),
                gems: b.gems.clone(),
                rewards: b.character.rewards.clone(),
                equipment: b.equipment.clone(),
                allocations: b.allocations.clone(),
                skills: b.skills.clone(),
                supports: b.supports.clone(),
                payload_links: b.payload_links.clone(),
                character_presets: vec![CharacterPreset {
                    id: selection.character,
                    class: b.character.class.clone(),
                    ascendancy: b.character.ascendancy.clone(),
                    level: b.character.level,
                    rewards: b.character.rewards.iter().map(|r| r.id).collect(),
                }],
                equipment_presets: vec![EquipmentPreset {
                    id: selection.equipment,
                    equipment: b.equipment.iter().map(|e| e.id).collect(),
                }],
                allocation_presets: vec![AllocationPreset {
                    id: selection.allocations,
                    allocations: b.allocations.iter().map(|a| a.id).collect(),
                    equipment: vec![],
                }],
                skill_presets: vec![
                    SkillPreset {
                        intent: Some(intent(&self.preferences)),
                        id: selection.skills,
                        skills: b.skills.iter().map(|s| s.id).collect(),
                        supports: b.supports.iter().map(|s| s.id).collect(),
                        authored_support_order: b.authored_support_order.clone(),
                        payload_links: b.payload_links.iter().map(|p| p.id).collect(),
                        usage_preferences: None,
                    },
                    SkillPreset {
                        intent: Some(intent(&self.dormant)),
                        id: id(10006),
                        skills: b.skills.iter().map(|s| s.id).collect(),
                        supports: b.supports.iter().map(|s| s.id).collect(),
                        authored_support_order: b.authored_support_order.clone(),
                        payload_links: b.payload_links.iter().map(|p| p.id).collect(),
                        usage_preferences: None,
                    },
                ],
                choice_presets: vec![ChoicePreset {
                    id: selection.choices,
                    choices: b.choices.clone(),
                    rewards: vec![],
                }],
                saved_variants: vec![],
            },
            limits,
        )
        .unwrap();
        let mut scenario = original.scenario().input().clone();
        scenario.usage = self.overrides.clone();
        let index = OwnedDefinitionSchemaPackage::new(
            self.sniper.base.source.base.inner.schema.clone(),
            Default::default(),
        )
        .map_err(|e| e.to_string())?;
        let proof = prove_project_intent(&index, &project, Default::default())
            .map_err(|e| e.to_string())?;
        compose_request_checked(
            &index,
            &project,
            IntentCompositionInputs {
                selection: &selection,
                inventory: None,
                scenario: ScenarioSpec::new(scenario, limits).map_err(|e| e.to_string())?,
                queries: original.queries().clone(),
            },
            &proof,
            Default::default(),
        )
        .map_err(|e| e.to_string())
    }
    fn checked(&self) -> std::result::Result<shared::Plan, String> {
        let r: Value = family::read("readiness.json");
        self.sniper
            .checked_plan_with_request(self.composed()?.into_request(), |stages| {
                let program: StagedRuleProgram = decode(&r["program"]);
                let destination = stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.owner == program.owner && p.program == program.program)
                    .unwrap();
                *destination = program;
                let ready = stages.readiness.as_mut().unwrap();
                let skill: SkillReadiness = decode(&r["readiness"]["skill_after"]);
                let destination = ready
                    .skills
                    .iter_mut()
                    .find(|s| s.skill == skill.skill)
                    .unwrap();
                assert_eq!(
                    *destination,
                    decode::<SkillReadiness>(&r["readiness"]["skill_before"])
                );
                *destination = skill;
                let p: ReadinessProgram = decode(&r["readiness"]["program"]);
                let destination = ready
                    .programs
                    .members
                    .iter_mut()
                    .find(|row| row.owner == p.owner && row.program == p.program)
                    .unwrap();
                *destination = p;
            })
    }
    fn report(&self) -> SupportEffectsReport {
        let p = self.checked().unwrap();
        p.evaluate(&mut p.new_scratch()).unwrap()
    }
    fn participation<'a>(&self, r: &'a SupportEffectsReport, i: usize) -> Option<&'a EffectValue> {
        let target = ConcreteEntity::Skill(Box::new(SkillTarget::Generated(Box::new(
            self.sniper.generated(i),
        ))));
        offering::effects(r)
            .effects
            .iter()
            .find(|e| {
                e.key.invocation.program.as_str() == "requested-skill-participation"
                    && e.key.invocation.entity == target
            })
            .map(|e| &e.value)
    }
    fn life<'a>(&self, r: &'a SupportEffectsReport, i: usize) -> &'a EffectValue {
        &offering::effects(r)
            .effects
            .iter()
            .find(|e| {
                e.key.invocation.program.as_str() == "intrinsic-allied-minion-life"
                    && e.key.invocation.entity == ConcreteEntity::Actor(self.sniper.actor(i))
            })
            .unwrap()
            .value
    }
}
fn boolean(b: bool) -> EffectValue {
    EffectValue::Known {
        value: ParameterValue::Boolean(b),
    }
}

#[test]
#[ignore = "requires published SKILL_PARTICIPATION_RELEASE"]
fn real_sniper_preparation_survives_inactive_participation_and_actor_execution_is_gated() {
    let mut w = World::load();
    let base = w.report();
    assert!(matches!(w.life(&base, 0), EffectValue::Known { .. }));
    for (group, occurrence) in [(true, true), (false, true), (true, false), (false, false)] {
        w.overrides = vec![w.usage(0, group, occurrence)];
        let r = w.report();
        assert_eq!(w.participation(&r, 0), Some(&boolean(group && occurrence)));
        assert_eq!(w.participation(&r, 1), Some(&boolean(true)));
        assert_eq!(
            w.life(&r, 0),
            if group && occurrence {
                w.life(&base, 0)
            } else {
                &EffectValue::Inactive
            }
        );
        assert_eq!(w.life(&r, 1), w.life(&base, 1));
        // Input assembly remains available; ActorLevel belongs to the later
        // population path and is deliberately not asserted when execution is off.
        for index in 0..2 {
            for field in ["final_level_parameter", "final_quality_parameter"] {
                let wanted = PlanValueKey::SkillParameter {
                    skill: Box::new(w.sniper.generated(index)),
                    parameter: decode(&w.sniper.bindings["target"][field]),
                };
                let rows = |report: &SupportEffectsReport| {
                    offering::effects(report)
                        .effects
                        .iter()
                        .filter(
                            |e| matches!(&e.target,BoundEffectTarget::Value{key}if *key==wanted),
                        )
                        .cloned()
                        .collect::<Vec<_>>()
                };
                let old = rows(&base);
                let new = rows(&r);
                assert_eq!(old.len(), 1);
                assert_eq!(new.len(), 1);
                assert_eq!(new[0].key.invocation.program, key(fixture::ASSEMBLY));
                assert!(matches!(new[0].value, EffectValue::Known { .. }));
                assert_eq!(new[0].value, old[0].value);
            }
        }
    }
}
#[test]
#[ignore = "requires published SKILL_PARTICIPATION_RELEASE"]
fn count_and_dormant_preferences_do_not_redefine_requested_participation() {
    let mut w = World::load();
    let base = w.report();
    w.dormant = vec![w.usage(0, false, false)];
    for count in [0, 2, 4] {
        w.overrides = vec![w.count(0, count)];
        let r = w.report();
        assert_eq!(w.participation(&r, 0), Some(&boolean(true)));
        assert_eq!(w.life(&r, 0), w.life(&base, 0));
    }
    let b: Value = family::read("bindings.json");
    w.preferences.retain(|u| u.policy != decode(&b["policy"]));
    w.overrides.clear();
    let r = w.report();
    assert!(w.participation(&r, 0).is_none());
    assert!(matches!(w.life(&r, 0), EffectValue::Unresolved { .. }));
}
#[test]
#[ignore = "requires published SKILL_PARTICIPATION_RELEASE"]
fn false_preference_cannot_hide_partial_owners_or_revive_disabled_supply() {
    let mut w = World::load();
    w.overrides = vec![w.usage(0, false, false)];
    let mut partial = w.clone();
    let actual = partial.sniper.actual_gem.clone();
    let owner = actual.owner.clone();
    *partial.sniper.base.source.base.inner.owner_mut(owner) = actual;
    let result = partial.checked();
    assert!(
        result.is_err()
            || matches!(
                result
                    .unwrap()
                    .evaluate(&mut w.checked().unwrap().new_scratch())
                    .unwrap()
                    .outcome,
                SupportEffectsOutcome::Unavailable { .. }
            )
    );
    w.sniper
        .base
        .source
        .base
        .inner
        .build
        .skills
        .iter_mut()
        .find(|s| s.id == id(7200))
        .unwrap()
        .enabled = false;
    w.overrides = vec![w.usage(0, true, true)];
    let r = w.report();
    assert!(
        w.participation(&r, 0)
            .is_none_or(|v| *v == EffectValue::Inactive)
    );
    // A disabled physical root supplies no Actor. It differs from a supplied
    // Actor whose execution is gated by a false requested-participation value.
    assert!(
        offering::effects(&r)
            .effects
            .iter()
            .all(|effect| effect.key.invocation.entity != ConcreteEntity::Actor(w.sniper.actor(0)))
    );
    assert!(matches!(w.life(&r, 1), EffectValue::Known { .. }));
}
#[test]
#[ignore = "requires published SKILL_PARTICIPATION_RELEASE"]
fn participation_plans_reuse_scratch_without_cross_occurrence_or_worker_state() {
    let a = World::load();
    let mut b = a.clone();
    b.overrides = vec![b.usage(0, false, true)];
    let plans = [
        Arc::new(a.checked().unwrap()),
        Arc::new(b.checked().unwrap()),
    ];
    let expected: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    let mut scratch = plans[0].new_scratch();
    for i in [0, 1, 0] {
        assert_eq!(plans[i].evaluate(&mut scratch).unwrap(), expected[i]);
    }
    let reports: Vec<_> = (0..12)
        .into_par_iter()
        .map_init(
            || plans[0].new_scratch(),
            |s, i| (i % 2, plans[i % 2].evaluate(s).unwrap()),
        )
        .collect();
    for (i, r) in reports {
        assert_eq!(r, expected[i]);
    }
}

#[test]
#[ignore = "requires published SKILL_PARTICIPATION_RELEASE"]
fn required_usage_inputs_fail_at_current_intent_proof_without_defaults() {
    let w = World::load();
    for malformed in [false, true] {
        let mut bad = w.clone();
        let mut override_ = bad.usage(0, true, true);
        if malformed {
            override_.parameters[0].value =
                ParameterValue::Integer(BoundedInteger::new(1).unwrap());
        } else {
            override_.parameters.remove(0);
        }
        bad.overrides = vec![override_];
        assert!(
            bad.composed().is_err(),
            "required typed current intent must reject missing or non-Boolean input"
        );
    }
    let mut scenario = w.clone();
    scenario.overrides = vec![scenario.usage(0, false, true)];
    let composed = scenario.composed().unwrap();
    assert!(
        composed
            .diagnostics()
            .iter()
            .any(|d| d.disposition == IntentDisposition::Overridden)
    );
    assert_eq!(
        composed
            .request()
            .scenario()
            .input()
            .usage
            .iter()
            .filter(|u| u.policy == scenario.usage(0, false, true).policy
                && u.target == scenario.usage(0, false, true).target)
            .count(),
        1
    );
}
