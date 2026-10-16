//! Finite native controls of the actual final owner; no production closure.
use super::{
    equipment, equipment_native, evasion_family, evasion_native, family, override_family,
    replay::ReplayInput,
};
use family::{def, key, subject};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::json;

const STAGE: &str = "player-evasion-override-delivery";
fn quantity(n: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(evasion_family::UNIT)).unwrap())
}
fn missing_stat(name: &str) -> StatDefId {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("synthetic.evasion-override-{name}"),
    )
    .unwrap()
}
fn fixture() -> ReplayInput {
    override_family::check_authored();
    let mut i = equipment_native::fixture();
    let c = override_family::consumer();
    let row = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owner.owner)
        .unwrap();
    assert_eq!(
        row.programs.members,
        equipment::consumer().final_owner.programs.members
    );
    row.programs.members = c.owner.programs.members;
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .push(c.query);
    i.stages.stages.push(EvaluationStage {
        id: key(STAGE),
        predecessors: vec![key(equipment::STAGE)],
    });
    i.stages
        .stages
        .iter_mut()
        .find(|s| s.id == key("player-evasion-final"))
        .unwrap()
        .predecessors
        .push(key(STAGE));
    i.stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: def(evasion_family::CHANNEL),
            contribution: ContributionKind::Override,
        },
        stage: key(STAGE),
    });
    // Declared synthetic missing inputs are test controls, not game inputs or
    // neutral defaults. Their complete empty producers yield MissingProducer.
    for (name, value_type) in [
        (
            "amount",
            json!({"kind":"quantity","value":{"unit":def::<UnitDefinition>(evasion_family::UNIT)}}),
        ),
        ("enabled", json!({"kind":"boolean"})),
    ] {
        let stat = missing_stat(name);
        i.schema.definitions.push(serde_json::from_value(json!({"kind":"stat","value":{"id":stat,"schema":{"kind":"known","value":{"value":value_type,"targets":["actor"]}}}})).unwrap());
        i.rules.owners.push(DefinitionRules {
            owner: subject(stat.clone()),
            programs: DeclaredSet::complete(vec![]),
        });
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat,
            },
            stage: key("deliver"),
        });
    }
    i.rebind_test_edit().unwrap();
    i
}
fn group(i: &mut ReplayInput) -> &mut ContributionGroup {
    &mut i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == override_family::QUERY)
        .unwrap()
        .groups[0]
}
fn source(i: &mut ReplayInput, id: &str, amount: Option<f64>, enabled: Option<bool>) {
    let program = key(&format!("synthetic.evasion-override-{id}"));
    let owner = subject(def::<ActorDefinition>(0x332a));
    let mut reads = vec![];
    let mut nodes = vec![];
    for (name, value, value_type) in [
        (
            "amount",
            amount.map(quantity),
            ComputedValueType::Quantity {
                unit: def(evasion_family::UNIT),
            },
        ),
        (
            "enabled",
            enabled.map(ParameterValue::Boolean),
            ComputedValueType::Boolean,
        ),
    ] {
        let expression = if let Some(value) = value {
            RuleExpression::Literal { value }
        } else {
            reads.push(RuleRead {
                id: key(name),
                value_type,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: missing_stat(name),
                },
            });
            RuleExpression::Read { input: key(name) }
        };
        nodes.push(RuleNode {
            id: key(name),
            expression,
        });
    }
    let p:RuleProgram = serde_json::from_value(json!({"id":program,"context":"actor","reads":reads,"nodes":nodes,"effects":[{"id":"override","when":"enabled","effect":{"kind":"contribute","entity":"current","stat":def::<StatDefinition>(evasion_family::CHANNEL),"contribution":"override","value":"amount"}}]})).unwrap();
    evasion_native::declare(i, owner.clone(), p, STAGE);
    group(i).members.members.push(ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner,
            program,
            effect: key("override"),
            origin: ContributionOrigin::ExistingActor {
                application: key("shared-player-initialization"),
            },
        }),
        order: None,
    });
}
fn rebind(i: &mut ReplayInput) {
    i.rebind_test_edit().unwrap();
}
fn final_value(i: &ReplayInput) -> EffectValue {
    evasion_native::value(&evasion_native::evaluate(i))
}
fn unresolved(i: &ReplayInput, reason: PlanGapReason) {
    assert!(matches!(final_value(i),EffectValue::Unresolved {reason:actual,..} if actual==reason));
}
#[test]
fn override_presence_replaces_only_global_base_before_scaling() {
    let base = fixture();
    evasion_native::known(&evasion_native::evaluate(&base), 10.);
    for (amount, expected) in [(0., 0.), (100., 145.), (-100., 0.)] {
        let mut i = base.clone();
        source(&mut i, "one", Some(amount), Some(true));
        rebind(&mut i);
        evasion_native::known(&evasion_native::evaluate(&i), expected);
    }
    for (amount, expected) in [(None, 119.), (Some(0.), 109.), (Some(100.), 254.)] {
        let mut i = base.clone();
        equipment_native::positive(&mut i, 0x66);
        if let Some(amount) = amount {
            source(&mut i, "one", Some(amount), Some(true));
        }
        rebind(&mut i);
        let r = evasion_native::evaluate(&i);
        evasion_native::known(&r, expected);
        assert_eq!(
            equipment_native::subtotal(&r),
            EffectValue::Known {
                value: quantity(108.75)
            }
        );
    }
}
#[test]
fn equal_conflicting_unknown_and_inactive_sources_keep_selection_semantics() {
    for (values, expected) in [(vec![100., 100.], Some(145.)), (vec![100., 200.], None)] {
        let mut i = fixture();
        for (n, v) in values.into_iter().enumerate() {
            source(&mut i, &n.to_string(), Some(v), Some(true));
        }
        rebind(&mut i);
        if let Some(n) = expected {
            evasion_native::known(&evasion_native::evaluate(&i), n);
        } else {
            unresolved(&i, PlanGapReason::ConflictingContributors);
        }
    }
    for (amount, enabled) in [(None, Some(true)), (Some(100.), None)] {
        let mut i = fixture();
        source(&mut i, "missing", amount, enabled);
        rebind(&mut i);
        unresolved(&i, PlanGapReason::MissingProducer);
    }
    let mut inactive = fixture();
    source(&mut inactive, "inactive", None, Some(false));
    rebind(&mut inactive);
    evasion_native::known(&evasion_native::evaluate(&inactive), 10.);
    assert_eq!(group(&mut inactive).members.members.len(), 1);
}
#[test]
fn override_does_not_bypass_other_missing_operands_or_numeric_domain() {
    let mut domain = fixture();
    source(
        &mut domain,
        "huge",
        Some(4_503_599_627_370_496.),
        Some(true),
    );
    rebind(&mut domain);
    assert!(matches!(
        final_value(&domain),
        EffectValue::UnsupportedDomain { .. }
    ));
    let mut missing = fixture();
    source(&mut missing, "zero", Some(0.), Some(true));
    let final_property = family::consumer()
        .receivers
        .into_iter()
        .find(|r| r.stat == def(equipment::PROPERTY))
        .unwrap();
    missing
        .rules
        .receivers
        .members
        .retain(|r| r.id != final_property.id);
    rebind(&mut missing);
    unresolved(&missing, PlanGapReason::MissingProducer);
    // A selected base also cannot turn an unresolved global factor into zero.
    let mut factor = fixture();
    source(&mut factor, "zero", Some(0.), Some(true));
    let query = factor
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "player-evasion-more")
        .unwrap();
    query.groups[0].members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(def::<StatDefinition>(evasion_family::CHANNEL)),
            facet: SchemaFacet::GameRules,
            code: key("synthetic.unknown-evasion-factor-members"),
        }],
    };
    rebind(&mut factor);
    let p = factor.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
}
#[test]
fn exact_membership_and_production_partial_are_not_replaced_by_observed_zero() {
    let base = fixture();
    for enabled in [true, false] {
        let mut i = base.clone();
        source(&mut i, "unlisted", Some(0.), Some(enabled));
        group(&mut i).members.members.clear();
        rebind(&mut i);
        let error = i
            .compile()
            .err()
            .expect("unlisted active occurrence must fail binding");
        assert!(error.contains("membership"), "{error}");
    }
    let mut partial = base.clone();
    let closure = override_family::consumer().owner.programs.closure;
    partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == override_family::consumer().owner.owner)
        .unwrap()
        .programs
        .closure = closure;
    rebind(&mut partial);
    let p = partial.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut query_partial = base.clone();
    group(&mut query_partial).members.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: subject(def::<StatDefinition>(evasion_family::CHANNEL)),
            facet: SchemaFacet::GameRules,
            code: key("synthetic.unknown-override-members"),
        }],
    };
    rebind(&mut query_partial);
    let p = query_partial.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut duplicate = base;
    source(&mut duplicate, "one", Some(100.), Some(true));
    let member = group(&mut duplicate).members.members[0].clone();
    group(&mut duplicate).members.members.push(member);
    assert!(duplicate.rebind_test_edit().is_err());
}
#[test]
fn override_channel_requires_freeze_before_consumer_and_after_every_writer() {
    let mut i = fixture();
    source(&mut i, "one", Some(100.), Some(true));
    rebind(&mut i);
    let mut late_read = i.clone();
    late_read.stages.stages.push(EvaluationStage {
        id: key("override-too-late"),
        predecessors: vec![key("player-evasion-final")],
    });
    late_read.stages.frozen_channels.iter_mut().find(|f|matches!(&f.channel,StageChannel::Contributions{stat,contribution:ContributionKind::Override,..}if *stat==def(evasion_family::CHANNEL))).unwrap().stage=key("override-too-late");
    assert!(late_read.rebind_test_edit().is_err());
    i.stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key("synthetic.evasion-override-one"))
        .unwrap()
        .stage = key("player-evasion-final");
    assert!(i.rebind_test_edit().is_err());
}
#[test]
fn selection_reorder_scratch_after_failures_and_rayon_are_deterministic() {
    let mut good = fixture();
    source(&mut good, "a", Some(100.), Some(true));
    source(&mut good, "b", Some(100.), Some(true));
    equipment_native::positive(&mut good, 0x66);
    rebind(&mut good);
    let a = good.compile().unwrap();
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    evasion_native::known(&expected, 254.);
    let mut reversed = good.clone();
    group(&mut reversed).members.members.reverse();
    reversed.build.items.reverse();
    reversed.build.equipment.reverse();
    rebind(&mut reversed);
    let b = reversed.compile().unwrap();
    let mut conflict = fixture();
    source(&mut conflict, "a", Some(100.), Some(true));
    source(&mut conflict, "b", Some(200.), Some(true));
    rebind(&mut conflict);
    let conflict = conflict.compile().unwrap();
    let mut missing = fixture();
    source(&mut missing, "missing", None, Some(true));
    rebind(&mut missing);
    let missing = missing.compile().unwrap();
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), expected);
    assert!(matches!(
        evasion_native::value(&conflict.evaluate(&mut scratch).unwrap()),
        EffectValue::Unresolved {
            reason: PlanGapReason::ConflictingContributors,
            ..
        }
    ));
    assert!(matches!(
        evasion_native::value(&missing.evaluate(&mut scratch).unwrap()),
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    ));
    let reordered = b.evaluate(&mut scratch).unwrap();
    assert!(
        reordered == b.evaluate(&mut b.new_scratch()).unwrap(),
        "reordered plan fresh/reused report"
    );
    // Authored member order participates in plan identity even though this
    // unordered selection produces the same exact effects and values.
    assert_ne!(reordered.identity, expected.identity);
    assert_eq!(reordered.gaps, expected.gaps);
    let (
        SupportEffectsOutcome::Evaluated { effects: left },
        SupportEffectsOutcome::Evaluated { effects: right },
    ) = (&reordered.outcome, &expected.outcome)
    else {
        panic!("finite plans evaluated")
    };
    assert!(
        left.gaps == right.gaps
            && left.effects == right.effects
            && left.values == right.values
            && left.application_groups == right.application_groups,
        "all outputs unchanged by unordered member permutation"
    );
    assert!(
        a.evaluate(&mut scratch).unwrap() == expected,
        "original report after conflict/missing/reordered runs"
    );
    let results: Vec<_> = (0..8)
        .into_par_iter()
        .map_init(|| a.new_scratch(), |s, _| a.evaluate(s).unwrap())
        .collect();
    assert!(results.iter().all(|r| r == &expected));
}
