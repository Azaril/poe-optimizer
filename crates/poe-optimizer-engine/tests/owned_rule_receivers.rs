#[path = "../../poe-optimizer-data/tests/support/owned_rule_receiver_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_engine::owned_rules::*;
#[test]
fn stat_owned_receiver_programs_use_the_existing_typed_executor() {
    let s = schema();
    let i = input(&s);
    let l = RuleLimits::default();
    let compiled = CompiledRulePackage::compile(&i, &s, l).unwrap();
    let mut scratch = compiled.new_scratch();
    for (program, expected) in [("player", 7.0), ("owned", 11.0), ("player", 7.0)] {
        let result = compiled
            .evaluate(&owner(), &key(program), &[], &s, &mut scratch)
            .unwrap();
        assert_eq!(result.effects.len(), 1);
        assert_eq!(
            result.effects[0].disposition,
            EffectDisposition::Applied {
                value: value(expected)
            }
        );
    }
    let mut shuffled = i.clone();
    shuffled.receivers.members.reverse();
    shuffled.receivers.members[0].targets.reverse();
    assert_eq!(
        compiled.identity(),
        CompiledRulePackage::compile(&shuffled, &s, l)
            .unwrap()
            .identity()
    );
    assert_eq!(compiled.input().receivers.members[0].targets.len(), 2);
    // This component evaluates one formula. It does not instantiate the two
    // declared slots or establish activation/global contributor completeness.
    let mut partial = i.clone();
    partial.receivers.closure = SchemaClosure::Partial {
        gaps: vec![gap("not-converted")],
    };
    let partial = CompiledRulePackage::compile(&partial, &s, l).unwrap();
    assert!(!partial.input().receivers.is_complete());
    assert_ne!(compiled.identity(), partial.identity());
    let mut empty = i;
    empty.receivers.members.clear();
    assert!(
        CompiledRulePackage::compile(&empty, &s, l)
            .unwrap()
            .input()
            .receivers
            .members
            .is_empty()
    );
}
#[test]
fn receiver_topology_and_unused_registry_rows_are_validated_before_execution() {
    let s = schema();
    let l = RuleLimits::default();
    for case in 0..8 {
        let mut i = input(&s);
        match case {
            0 => i.receivers.members.push(i.receivers.members[0].clone()),
            1 => i.receivers.members[1].program = key("player"),
            2 => i.receivers.members[1]
                .targets
                .push(ActorReceiverTarget::OwnedSlot {
                    slot: slot("first"),
                }),
            3 => i.receivers.members[1].targets.clear(),
            4 => i.receivers.members[1].program = key("missing"),
            5 => {
                i.receivers.members[1].targets = vec![ActorReceiverTarget::OwnedSlot {
                    slot: slot("missing"),
                }]
            }
            6 => i.owners[0].programs.members[1].context = RuleEntityKind::Action,
            _ => {
                if let RuleEffectKind::Derive { entity, .. } =
                    &mut i.owners[0].programs.members[1].effects[0].effect
                {
                    *entity = RuleEntity::Player;
                }
            }
        }
        assert!(
            CompiledRulePackage::compile(&i, &s, l).is_err(),
            "case {case}"
        );
    }
    let mut i = input(&s);
    let extra = i.owners[0].programs.members[0].effects[0].clone();
    i.owners[0].programs.members[0].effects.push(RuleEffect {
        id: key("extra"),
        ..extra
    });
    assert!(CompiledRulePackage::compile(&i, &s, l).is_err());
    let mut i = input(&s);
    if let RuleEffectKind::Derive { entity, .. } =
        &mut i.owners[0].programs.members[0].effects[0].effect
    {
        *entity = RuleEntity::Actor;
    }
    assert!(CompiledRulePackage::compile(&i, &s, l).is_ok());
}
#[test]
fn stat_owners_do_not_gain_another_owners_authored_ports_or_wrong_units() {
    let s = schema();
    let l = RuleLimits::default();
    for source in [
        RuleReadSource::Parameter { slot: parameter() },
        RuleReadSource::ItemLevel,
        RuleReadSource::GemLevel,
    ] {
        let mut i = input(&s);
        let p = &mut i.owners[0].programs.members[0];
        p.reads.push(RuleRead {
            id: key("forbidden"),
            value_type: if matches!(&source, RuleReadSource::Parameter { .. }) {
                ComputedValueType::Quantity { unit: id("points") }
            } else {
                ComputedValueType::Integer
            },
            source,
        });
        assert!(CompiledRulePackage::compile(&i, &s, l).is_err());
    }
    let mut i = input(&s);
    i.owners[0].programs.members[0].nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Quantity(FiniteQuantity::new(7.0, id("factor")).unwrap()),
    };
    assert!(CompiledRulePackage::compile(&i, &s, l).is_err());
    let mut raw = schema_input();
    if let DefinitionDescriptor::Stat(entry) = &mut raw.definitions[2]
        && let SchemaState::Known(stat) = &mut entry.schema
    {
        stat.targets = vec![RuleEntityKind::Action];
    }
    let wrong = OwnedDefinitionSchemaPackage::new(raw, OwnedSchemaLimits::default()).unwrap();
    assert!(CompiledRulePackage::compile(&input(&wrong), &wrong, l).is_err());
}
#[test]
fn receiver_guards_and_missing_stat_inputs_keep_existing_lazy_semantics() {
    let s = schema();
    let mut i = input(&s);
    let p = &mut i.owners[0].programs.members[0];
    p.reads.push(RuleRead {
        id: key("missing"),
        value_type: ComputedValueType::Quantity { unit: id("points") },
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: id("final"),
        },
    });
    p.nodes[0].expression = RuleExpression::Read {
        input: key("missing"),
    };
    p.nodes.push(RuleNode {
        id: key("guard"),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(false),
        },
    });
    p.effects[0].when = Some(key("guard"));
    let c = CompiledRulePackage::compile(&i, &s, RuleLimits::default()).unwrap();
    let mut scratch = c.new_scratch();
    assert_eq!(
        c.evaluate(&owner(), &key("player"), &[], &s, &mut scratch)
            .unwrap()
            .effects[0]
            .disposition,
        EffectDisposition::Inactive
    );
    i.owners[0].programs.members[0].effects[0].when = None;
    let c = CompiledRulePackage::compile(&i, &s, RuleLimits::default()).unwrap();
    assert_eq!(
        c.evaluate(&owner(), &key("player"), &[], &s, &mut scratch)
            .unwrap()
            .effects[0]
            .disposition,
        EffectDisposition::Unresolved {
            input: key("missing")
        }
    );
    // Cross-program/final-stat cycles belong to occurrence planning. Cycles
    // inside the expression DAG still reject through the ordinary compiler.
    i.owners[0].programs.members[0].nodes[0].expression = RuleExpression::Add {
        left: key("value"),
        right: key("value"),
    };
    assert!(CompiledRulePackage::compile(&i, &s, RuleLimits::default()).is_err());
}
#[test]
fn receiver_counts_targets_gaps_and_lookup_work_are_bounded() {
    let s = schema();
    let l = RuleLimits::default();
    let mut i = input(&s);
    i.receivers.closure = SchemaClosure::Partial {
        gaps: vec![gap("receiver-gap")],
    };
    i.owners[0].programs.closure = SchemaClosure::Partial {
        gaps: vec![gap("program-gap")],
    };
    for tight in [
        RuleLimits {
            max_receivers: 1,
            ..l
        },
        RuleLimits {
            max_receiver_targets: 2,
            ..l
        },
        RuleLimits { max_gaps: 1, ..l },
        RuleLimits { max_work: 1, ..l },
        RuleLimits {
            max_wire_bytes: 1,
            ..l
        },
    ] {
        assert!(CompiledRulePackage::compile(&i, &s, tight).is_err());
    }
    i.receivers.members.clear();
    i.receivers.closure = SchemaClosure::Partial {
        gaps: vec![gap("a"), gap("b")],
    };
    assert!(CompiledRulePackage::compile(&i, &s, RuleLimits { max_gaps: 1, ..l }).is_err());
    for bad in [
        vec![],
        vec![gap("same"), gap("same")],
        vec![SchemaGap {
            subject: SchemaSubject::Definition(DefinitionAddress::Stat(id("missing"))),
            ..gap("missing")
        }],
    ] {
        i.receivers.closure = SchemaClosure::Partial { gaps: bad };
        assert!(CompiledRulePackage::compile(&i, &s, l).is_err());
    }
}
