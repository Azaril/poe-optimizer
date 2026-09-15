//! Occurrence-scoped native values, with no source/import or game-name dispatch.
#[allow(dead_code)]
#[path = "support/owned_plan_fixture.rs"]
mod support;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use support::*;

fn rules_rejected(f: &Fixture, expected: &str) {
    use poe_optimizer_data::owned_schema::{OwnedDefinitionSchemaPackage, OwnedSchemaLimits};
    use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleLimits};
    let schema =
        OwnedDefinitionSchemaPackage::new(f.schema.clone(), OwnedSchemaLimits::default()).unwrap();
    let input = RulePackageInput {
        schema_version: OWNED_RULE_PACKAGE_VERSION,
        namespace: ns(),
        release: key("test"),
        semantics_version: key("test"),
        operations_version: key(OWNED_RULE_OPERATIONS_VERSION),
        definitions: schema.identity().clone(),
        owners: f.owners.clone(),
        tables: f.tables.clone(),
        receivers: f.receivers.clone(),
    };
    let Err(err) = CompiledRulePackage::compile(&input, &schema, RuleLimits::default()) else {
        panic!("invalid scoped rule accepted");
    };
    assert!(err.to_string().contains(expected), "{err}");
}
fn fixture() -> Fixture {
    let mut f = Fixture::new();
    for name in ["local-input", "local-result"] {
        f.schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: def(name),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Integer,
                    targets: vec![RuleEntityKind::Modifier],
                }),
            }));
    }
    f.owner_mut(&modifier_owner()).programs.members.extend([
        RuleProgram {
            id: key("local-producer"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![read(
                "roll",
                RuleReadSource::Parameter {
                    slot: parameter(SlotOwnerDefId::Modifier(def("modifier")), "roll"),
                },
            )],
            nodes: vec![read_node("roll", "roll")],
            effects: vec![derive(
                "local-input",
                RuleEntity::Modifier,
                "local-input",
                "roll",
            )],
        },
        RuleProgram {
            id: key("local-consumer"),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![read(
                "input",
                RuleReadSource::Stat {
                    entity: RuleEntity::Modifier,
                    stat: def("local-input"),
                },
            )],
            nodes: vec![
                read_node("input", "input"),
                literal("one", 1),
                node(
                    "result",
                    RuleExpression::Add {
                        left: key("input"),
                        right: key("one"),
                    },
                ),
            ],
            effects: vec![derive(
                "local-result",
                RuleEntity::Modifier,
                "local-result",
                "result",
            )],
        },
    ]);
    f.build.items[0].modifiers[0].rolls[0].value = integer(25);
    f.build.items[0].modifiers[1].rolls[0].value = integer(35);
    f
}
fn value(report: &OwnedEffectsReport, use_id: u64, modifier_id: u64, stat: &str) -> EffectValue {
    let key = PlanValueKey::Stat {
        entity: ConcreteEntity::Modifier(ProviderKey {
            root: ProviderRoot::ItemModifier {
                equipment_use: occurrence(use_id),
                modifier: occurrence(modifier_id),
            },
            grant_path: vec![],
        }),
        stat: def(stat),
    };
    report
        .values
        .iter()
        .find(|r| r.key == key)
        .unwrap()
        .value
        .clone()
}
fn assert_results(report: &OwnedEffectsReport, first: i64) {
    for use_id in [6, 7] {
        assert_eq!(
            value(report, use_id, 4, "local-input"),
            EffectValue::Known {
                value: integer(first)
            }
        );
        assert_eq!(
            value(report, use_id, 4, "local-result"),
            EffectValue::Known {
                value: integer(first + 1)
            }
        );
        assert_eq!(
            value(report, use_id, 5, "local-result"),
            EffectValue::Known { value: integer(36) }
        );
    }
}

#[test]
fn repeated_modifiers_and_equipment_uses_bind_distinct_dependency_values() {
    let mut f = fixture();
    let first = f.compile().unwrap();
    let mut scratch = first.new_scratch();
    let a = first.evaluate(&mut scratch).unwrap();
    assert!(a.gaps.is_empty(), "{a:?}");
    assert_results(&a, 25);
    f.build.items[0].modifiers[0].rolls[0].value = integer(50);
    f.build.revision = poe_optimizer_core::build_identity::BuildRevision::from_u64(2);
    let second = f.compile().unwrap();
    assert_ne!(first.identity(), second.identity());
    assert_results(&second.evaluate(&mut scratch).unwrap(), 50);
    assert_results(&first.evaluate(&mut scratch).unwrap(), 25);
}

#[test]
fn missing_modifier_stage_withholds_downstream_value() {
    let mut f = fixture();
    f.owner_mut(&modifier_owner())
        .programs
        .members
        .retain(|p| p.id != key("local-producer"));
    let plan = f.compile().unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    for use_id in [6, 7] {
        for modifier in [4, 5] {
            assert!(matches!(
                value(&report, use_id, modifier, "local-result"),
                EffectValue::Unresolved { .. }
            ));
        }
    }
}

#[test]
fn modifier_values_retain_duplicate_producer_and_cycle_errors() {
    let mut duplicate = fixture();
    let mut p = duplicate
        .owner_mut(&modifier_owner())
        .programs
        .members
        .iter()
        .find(|p| p.id == key("local-producer"))
        .unwrap()
        .clone();
    p.id = key("duplicate-producer");
    duplicate
        .owner_mut(&modifier_owner())
        .programs
        .members
        .push(p);
    assert!(duplicate.compile().is_err());
    let mut cycle = fixture();
    let p = cycle
        .owner_mut(&modifier_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("local-producer"))
        .unwrap();
    p.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Modifier,
        stat: def("local-result"),
    };
    assert!(cycle.compile().is_err());
}

#[test]
fn modifier_relative_scope_rejects_wrong_owner_context_and_stat_target() {
    let original = fixture();
    let producer = original
        .owners
        .iter()
        .find(|o| o.owner == modifier_owner())
        .unwrap()
        .programs
        .members
        .iter()
        .find(|p| p.id == key("local-producer"))
        .unwrap()
        .clone();
    for context in [
        RuleEntityKind::Actor,
        RuleEntityKind::Action,
        RuleEntityKind::Modifier,
        RuleEntityKind::Enemy,
        RuleEntityKind::Environment,
    ] {
        let mut f = fixture();
        let p = f
            .owner_mut(&modifier_owner())
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key("local-producer"))
            .unwrap();
        p.context = context;
        rules_rejected(&f, "Modifier");
    }
    // Literal inputs avoid making the owner check pass only because a roll is foreign.
    let mut wrong_owner = fixture();
    let mut p = producer;
    p.reads.clear();
    p.nodes = vec![literal("roll", 2)];
    wrong_owner
        .owner_mut(&item_owner())
        .programs
        .members
        .push(p);
    rules_rejected(&wrong_owner, "relative Modifier requires");
    let mut wrong_target = fixture();
    for d in &mut wrong_target.schema.definitions {
        if let DefinitionDescriptor::Stat(row) = d
            && row.id == def("local-input")
            && let SchemaState::Known(schema) = &mut row.schema
        {
            schema.targets = vec![RuleEntityKind::EquipmentUse];
        }
    }
    rules_rejected(&wrong_target, "stat target/context mismatch");
}
