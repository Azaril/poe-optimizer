//! EnemyLevel binds the scenario's typed enemy, independently of the executing entity.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod fixture;

use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
use poe_optimizer_engine::{
    owned_plan::*,
    owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact, RuleLimits},
};
use std::sync::Arc;

fn encounter_owner() -> SchemaSubject {
    subject(def::<EncounterDefinition>("encounter"))
}

fn observer(context: RuleEntityKind) -> RuleProgram {
    RuleProgram {
        id: key("observe-enemy-level"),
        context,
        reads: vec![read("enemy-level", RuleReadSource::EnemyLevel)],
        nodes: vec![read_node("enemy-level", "enemy-level")],
        effects: vec![derive(
            "observed",
            RuleEntity::Current,
            "observed-enemy-level",
            "enemy-level",
        )],
    }
}

fn fixture() -> Fixture {
    let mut f = Fixture::new();
    f.build.character.level = 96;
    f.scenario.enemy.level = 20;
    for descriptor in &mut f.schema.definitions {
        if let DefinitionDescriptor::Encounter(entry) = descriptor
            && let SchemaState::Known(schema) = &mut entry.schema
        {
            schema.enemy_level.maximum = BoundedInteger::new(85).unwrap();
        }
    }
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: def("observed-enemy-level"),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Integer,
                targets: vec![
                    RuleEntityKind::Actor,
                    RuleEntityKind::EquipmentUse,
                    RuleEntityKind::Action,
                    RuleEntityKind::Enemy,
                    RuleEntityKind::Environment,
                ],
            }),
        }));
    f.owner_mut(&encounter_owner())
        .programs
        .members
        .push(observer(RuleEntityKind::Enemy));
    f
}

fn schema(f: &Fixture) -> OwnedDefinitionSchemaPackage {
    OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap()
}

fn rules(f: &Fixture, schema: &OwnedDefinitionSchemaPackage) -> RulePackageInput {
    RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("rules"),
        semantics_version: key("test-v1"),
        operations_version: key(OWNED_RULE_OPERATIONS_V14),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
        tables: f.tables.clone(),
        receivers: f.receivers.clone(),
    }
}

fn observation(report: &OwnedEffectsReport, entity: ConcreteEntity) -> &EffectValue {
    let matching: Vec<_> = report
        .values
        .iter()
        .filter(|row| {
            row.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: def("observed-enemy-level"),
                }
        })
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "missing or duplicate observation: {entity:?}"
    );
    &matching[0].value
}

fn assert_level(report: &OwnedEffectsReport, entity: ConcreteEntity, expected: i64) {
    assert_eq!(
        observation(report, entity),
        &EffectValue::Known {
            value: integer(expected)
        }
    );
}

fn observed_effect<'a>(
    report: &'a OwnedEffectsReport,
    owner: &SchemaSubject,
    entity: ConcreteEntity,
) -> &'a EffectValue {
    let matching: Vec<_> = report
        .effects
        .iter()
        .filter(|row| {
            row.key.invocation.owner == *owner
                && row.key.invocation.program == key("observe-enemy-level")
                && row.key.invocation.entity == entity
                && row.key.effect == key("observed")
        })
        .collect();
    assert_eq!(matching.len(), 1, "exact observer effect must be present");
    &matching[0].value
}

#[test]
fn enemy_level_is_request_global_across_legitimate_program_contexts() {
    let mut f = fixture();
    f.add_action_route();
    f.add_generated_actors();
    for (owner, context) in [
        (class_owner(), RuleEntityKind::Actor),
        (item_owner(), RuleEntityKind::EquipmentUse),
        (
            SchemaSubject::Slot(SlotAddress::ActionOutput(output())),
            RuleEntityKind::Action,
        ),
        (
            SchemaSubject::Slot(SlotAddress::Actor(child_slot())),
            RuleEntityKind::Actor,
        ),
    ] {
        f.owner_mut(&owner).programs.members.push(observer(context));
    }
    let mut environment = observer(RuleEntityKind::Environment);
    environment.id = key("observe-enemy-level-environment");
    f.owner_mut(&encounter_owner())
        .programs
        .members
        .push(environment);
    f.scenario.enemy.level = 82;
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for entity in [
        ConcreteEntity::Enemy,
        ConcreteEntity::Environment,
        ConcreteEntity::Actor(ActorKey::Player),
        ConcreteEntity::Actor(child_actor(30)),
        ConcreteEntity::Actor(child_actor(31)),
        ConcreteEntity::EquipmentUse(occurrence(6)),
        ConcreteEntity::EquipmentUse(occurrence(7)),
        ConcreteEntity::Action(Box::new(action())),
    ] {
        assert_level(&report, entity, 82);
    }
    assert_eq!(f.build.character.level, 96);
    assert_eq!(f.build.gems[0].level, 11);
    assert_eq!(f.build.gems[1].level, 20);
    assert_eq!(
        report
            .effects
            .iter()
            .filter(|row| row.key.effect == key("observed"))
            .count(),
        8
    );
}

#[test]
fn enemy_level_requires_v14_integer_and_bounded_reads() {
    let f = fixture();
    let schema = schema(&f);
    let input = rules(&f, &schema);
    CompiledRulePackage::compile(&input, &schema, RuleLimits::default()).unwrap();
    for version in [
        OWNED_RULE_OPERATIONS_V6,
        OWNED_RULE_OPERATIONS_V7,
        OWNED_RULE_OPERATIONS_V8,
        OWNED_RULE_OPERATIONS_V9,
        OWNED_RULE_OPERATIONS_V10,
        OWNED_RULE_OPERATIONS_V11,
        OWNED_RULE_OPERATIONS_V12,
        OWNED_RULE_OPERATIONS_V13,
    ] {
        let mut old = input.clone();
        old.operations_version = key(version);
        assert!(
            CompiledRulePackage::compile(&old, &schema, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("EnemyLevel requires owned-domain-operations-v14"),
            "{version}"
        );
        let mut unchanged = Fixture::new();
        unchanged.scenario.enemy.level = 20;
        let old_plan = unchanged
            .compile_with_operations(PlanLimits::default(), version)
            .unwrap();
        assert_eq!(old_plan.request().scenario().input().enemy.level, 20);
    }
    for value_type in [
        ComputedValueType::Boolean,
        ComputedValueType::Quantity { unit: def("count") },
    ] {
        let mut wrong = input.clone();
        wrong
            .owners
            .iter_mut()
            .find(|owner| owner.owner == encounter_owner())
            .unwrap()
            .programs
            .members[0]
            .reads[0]
            .value_type = value_type;
        assert!(
            CompiledRulePackage::compile(&wrong, &schema, RuleLimits::default())
                .unwrap_err()
                .to_string()
                .contains("declared read type does not match source schema")
        );
    }
    assert!(
        CompiledRulePackage::compile(
            &input,
            &schema,
            RuleLimits {
                max_reads: 0,
                ..RuleLimits::default()
            }
        )
        .is_err()
    );
}

#[test]
fn encounter_range_constrains_rule_facts_and_request_binding() {
    let mut f = fixture();
    let schema = schema(&f);
    let rules =
        CompiledRulePackage::compile(&rules(&f, &schema), &schema, RuleLimits::default()).unwrap();
    let mut scratch = rules.new_scratch();
    for value in [1, 20, 85] {
        let result = rules
            .evaluate(
                &encounter_owner(),
                &key("observe-enemy-level"),
                &[RuleFact {
                    read: key("enemy-level"),
                    value: integer(value),
                }],
                &schema,
                &mut scratch,
            )
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: integer(value)
            }
        );
    }
    for value in [0, 86] {
        assert!(
            rules
                .evaluate(
                    &encounter_owner(),
                    &key("observe-enemy-level"),
                    &[RuleFact {
                        read: key("enemy-level"),
                        value: integer(value),
                    }],
                    &schema,
                    &mut scratch,
                )
                .is_err()
        );
    }
    // A valid structural level can still lie outside this exact Encounter range.
    f.scenario.enemy.level = 86;
    assert!(matches!(f.compile(), Err(PlanError::Invalid(_))));
}

#[test]
fn missing_or_unmapped_encounter_does_not_supply_a_level_to_other_owners() {
    for unmapped in [false, true] {
        let mut f = fixture();
        f.owner_mut(&class_owner())
            .programs
            .members
            .push(observer(RuleEntityKind::Actor));
        f.scenario.enemy.encounter = def("unknown-encounter");
        if unmapped {
            f.schema
                .definitions
                .push(DefinitionDescriptor::Encounter(DefinitionEntry {
                    id: def("unknown-encounter"),
                    schema: SchemaState::Unmapped {
                        gaps: vec![SchemaGap {
                            subject: subject(def::<EncounterDefinition>("unknown-encounter")),
                            facet: SchemaFacet::StaticLinks,
                            code: key("unmapped-encounter"),
                        }],
                    },
                }));
        }
        let plan = f.compile().unwrap();
        let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(matches!(
            observed_effect(
                &report,
                &class_owner(),
                ConcreteEntity::Actor(ActorKey::Player)
            ),
            EffectValue::Unresolved {
                reason: PlanGapReason::SchemaUnresolved,
                ..
            }
        ));
        // The direct read retains its schema cause; final values also require
        // whole-plan incoming membership and cannot bypass that separate gap.
        assert!(matches!(
            observation(&report, ConcreteEntity::Actor(ActorKey::Player)),
            EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            }
        ));
        assert!(
            report
                .gaps
                .iter()
                .any(|gap| gap.reason == PlanGapReason::SchemaUnresolved)
        );
        assert!(
            report
                .effects
                .iter()
                .all(|row| row.key.invocation.owner != encounter_owner())
        );
    }
}

#[test]
fn enemy_level_does_not_activate_other_encounters_or_close_partial_owners() {
    let mut f = fixture();
    let mut second = f
        .schema
        .definitions
        .iter()
        .find_map(|entry| match entry {
            DefinitionDescriptor::Encounter(entry) => Some(entry.clone()),
            _ => None,
        })
        .unwrap();
    second.id = def("other-encounter");
    f.schema
        .definitions
        .push(DefinitionDescriptor::Encounter(second));
    let mut other = f.owner_mut(&encounter_owner()).clone();
    other.owner = subject(def::<EncounterDefinition>("other-encounter"));
    f.owners.push(other);
    f.owner_mut(&encounter_owner()).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: encounter_owner(),
            facet: SchemaFacet::GameRules,
            code: key("other-encounter-mechanics"),
        }],
    };
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_eq!(
        observed_effect(&report, &encounter_owner(), ConcreteEntity::Enemy),
        &EffectValue::Known { value: integer(20) }
    );
    assert!(matches!(
        observation(&report, ConcreteEntity::Enemy),
        EffectValue::Unresolved {
            reason: PlanGapReason::IncompleteContributors,
            ..
        }
    ));
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms)
    );
    assert!(
        report
            .effects
            .iter()
            .filter(|row| row.key.effect == key("observed"))
            .all(|row| row.key.invocation.owner == encounter_owner())
    );

    f.scenario.enemy.encounter = def("other-encounter");
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert_level(&report, ConcreteEntity::Enemy, 20);
    assert!(
        report
            .effects
            .iter()
            .filter(|row| row.key.effect == key("observed"))
            .all(|row| row.key.invocation.owner
                == subject(def::<EncounterDefinition>("other-encounter")))
    );

    f.owner_mut(&subject(def::<EncounterDefinition>("other-encounter")))
        .programs
        .members[0]
        .context = RuleEntityKind::EquipmentUse;
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::UnsupportedContext)
    );
    assert!(
        report
            .effects
            .iter()
            .all(|row| row.key.effect != key("observed"))
    );
}

#[test]
fn level_changes_rebind_identity_scratch_and_parallel_evaluations() {
    let mut f = fixture();
    f.scenario.enemy.level = 82;
    let a = Arc::new(f.compile().unwrap());
    f.scenario.enemy.level = 83;
    let b = Arc::new(f.compile().unwrap());
    assert_ne!(a.bindings().request, b.bindings().request);
    assert_ne!(a.identity(), b.identity());
    assert_eq!(a.bindings().definitions, b.bindings().definitions);
    assert_eq!(a.bindings().rules, b.bindings().rules);
    assert_eq!(a.bindings().routing, b.bindings().routing);
    let mut scratch = a.new_scratch();
    let expected_a = a.evaluate(&mut scratch).unwrap();
    let expected_b = b.evaluate(&mut scratch).unwrap();
    assert_level(&expected_a, ConcreteEntity::Enemy, 82);
    assert_level(&expected_b, ConcreteEntity::Enemy, 83);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected_a);
    let handles: Vec<_> = (0..4)
        .map(|worker| {
            let plan = if worker % 2 == 0 {
                a.clone()
            } else {
                b.clone()
            };
            let expected = if worker % 2 == 0 {
                expected_a.clone()
            } else {
                expected_b.clone()
            };
            std::thread::spawn(move || {
                let mut scratch = plan.new_scratch();
                for _ in 0..8 {
                    assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
                }
            })
        })
        .collect();
    for handle in handles {
        handle.join().unwrap();
    }
}
