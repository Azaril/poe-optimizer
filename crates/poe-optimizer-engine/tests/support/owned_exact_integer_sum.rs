//! Exact unordered sums use existing Reward-to-Player authority, not Skill payer delivery.
use super::*;
use poe_optimizer_core::build_identity::InstanceAllocator;
use std::collections::BTreeSet;

fn reward() -> RewardDefId {
    def("reward-a")
}
fn input(name: &str) -> DeclaredSlot<ParameterSlotDefId> {
    parameter(SlotOwnerDefId::Reward(reward()), name)
}
fn world(values: &[Option<i64>]) -> World {
    let mut w = World::new();
    for owner in [
        shared(),
        supplied(),
        slot_owner(),
        subject(reward()),
        subject(def::<RewardDefinition>("reward-b")),
    ] {
        w.f.owner_mut(&owner).programs.members.clear();
    }
    for row in &mut w.f.schema.definitions {
        if let DefinitionDescriptor::Reward(row) = row
            && row.id == reward()
        {
            let SchemaState::Known(schema) = &mut row.schema else {
                unreachable!()
            };
            schema.declarations.parameters =
                DeclaredSet::complete(vec![input("amount"), input("enabled")]);
        }
    }
    for (name, value) in [
        (
            "amount",
            ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(BoundedInteger::MIN).unwrap(),
                maximum: BoundedInteger::new(BoundedInteger::MAX).unwrap(),
            }),
        ),
        ("enabled", ValueSchema::Boolean),
    ] {
        w.f.schema.slots.push(SlotDescriptor::Parameter(known(
            input(name),
            ParameterSlotSchema {
                skill_input: None,
                value,
                presence: SlotPresence::OptionalOnce,
                sites: vec![ParameterSite::RewardParameter],
            },
        )));
    }
    let mut p = producer(RuleReadSource::Parameter {
        slot: input("amount"),
    });
    p.reads.push(RuleRead {
        id: key("enabled"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Parameter {
            slot: input("enabled"),
        },
    });
    p.nodes.push(read_node("enabled", "enabled"));
    p.effects[0].when = Some(key("enabled"));
    w.f.owner_mut(&subject(reward())).programs.members = vec![p];
    let mut allocator = InstanceAllocator::from_state(w.f.build.allocator);
    w.f.build.character.rewards = values
        .iter()
        .map(|amount| {
            let mut parameters = vec![ParameterAssignment {
                slot: input("enabled"),
                value: ParameterValue::Boolean(true),
            }];
            if let Some(amount) = amount {
                parameters.push(ParameterAssignment {
                    slot: input("amount"),
                    value: integer(*amount),
                });
            }
            RewardSelection {
                id: allocator.allocate().unwrap(),
                definition: reward(),
                parameters,
            }
        })
        .collect();
    w.f.build.allocator = allocator.state();
    let group = &mut w.queries.members[0].groups[0];
    group.reduction = ContributionReduction::ExactIntegerSum;
    group.ordering = ContributionOrdering::Unordered;
    let mut m = member(subject(reward()), ContributionOrigin::Reward, 0);
    m.order = None;
    group.members = DeclaredSet::complete(vec![m]);
    w
}
fn plan_with(
    w: &World,
    limits: PlanLimits,
    edit: impl FnOnce(&mut RulePackageInput),
) -> Result<OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>> {
    let schema = w.schema();
    let mut rules = w.input(&schema);
    rules.operations_version = key(OWNED_RULE_OPERATIONS_V27);
    edit(&mut rules);
    empty_support::checked_plan(
        &w.f,
        rules,
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("routes"),
            definitions: schema.identity().clone(),
            outputs: vec![],
        },
        limits,
    )
}
fn plan(w: &World) -> OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage> {
    plan_with(w, Default::default(), |_| {}).unwrap()
}
fn evaluated(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("expected evaluated finite domain: {:?}", report.outcome)
    };
    effects
}
fn report(w: &World) -> OwnedEffectsReport {
    let p = plan(w);
    evaluated(&p.evaluate(&mut p.new_scratch()).unwrap()).clone()
}
fn set_enabled(w: &mut World, index: usize, enabled: Option<bool>) {
    let parameters = &mut w.f.build.character.rewards[index].parameters;
    parameters.retain(|p| p.slot != input("enabled"));
    if let Some(enabled) = enabled {
        parameters.push(ParameterAssignment {
            slot: input("enabled"),
            value: ParameterValue::Boolean(enabled),
        });
    }
}

#[test]
fn repeated_reward_effects_sum_exactly_without_occurrence_ranks() {
    let values = [
        Some(BoundedInteger::MAX),
        Some(BoundedInteger::MAX),
        Some(-BoundedInteger::MAX),
        Some(-BoundedInteger::MAX),
        Some(7),
        Some(7),
    ];
    for shift in 0..values.len() {
        let mut values = values;
        values.rotate_left(shift);
        let mut w = world(&values);
        if shift % 2 == 1 {
            w.f.build.character.rewards.reverse();
        }
        let r = report(&w);
        assert_values(&r, 14, 0, 0);
        let effects: Vec<_> = r
            .effects
            .iter()
            .filter(|e| e.key.invocation.owner == subject(reward()))
            .collect();
        assert_eq!(effects.len(), values.len());
        let sources: BTreeSet<_> = effects
            .iter()
            .map(|e| {
                let RuleOrigin::Provider { provider } = &e.key.invocation.origin else {
                    panic!("expected exact Reward provider")
                };
                assert!(provider.grant_path.is_empty());
                let ProviderRoot::Reward(id) = provider.root else {
                    panic!("expected exact Reward occurrence")
                };
                id
            })
            .collect();
        assert_eq!(sources.len(), values.len());
        assert!(effects.iter().all(|e| {
            e.key.invocation.program == key("produce")
                && e.key.effect == key("add")
                && e.key.invocation.entity == ConcreteEntity::Actor(ActorKey::Player)
        }));
    }
}

#[test]
fn exact_sum_bounds_apply_to_final_total_and_empty_or_inactive_is_zero() {
    for (values, expected) in [
        (vec![], 0),
        (vec![Some(0)], 0),
        (vec![Some(BoundedInteger::MAX)], BoundedInteger::MAX),
        (vec![Some(BoundedInteger::MIN)], BoundedInteger::MIN),
    ] {
        assert_values(&report(&world(&values)), expected, 0, 0);
    }
    for values in [
        [Some(BoundedInteger::MAX), Some(1)],
        [Some(BoundedInteger::MIN), Some(-1)],
    ] {
        let r = report(&world(&values));
        assert!(matches!(
            value(&r, ActorKey::Player),
            EffectValue::NumericalError {
                reason: poe_optimizer_engine::owned_rules::NumericalFailure::IntegerOverflow,
                ..
            }
        ));
    }
    let mut w = world(&[None, None]);
    set_enabled(&mut w, 0, Some(false));
    set_enabled(&mut w, 1, Some(false));
    let r = report(&w);
    assert_values(&r, 0, 0, 0);
    assert!(
        r.effects
            .iter()
            .filter(|e| e.key.invocation.owner == subject(reward()))
            .all(|e| e.value == EffectValue::Inactive)
    );
}

#[test]
fn every_active_failure_is_checked_before_exact_sum_is_returned() {
    let values = [Some(BoundedInteger::MAX), Some(1), None, Some(0)];
    let mut expected = None;
    for shift in 0..values.len() {
        let mut values = values;
        values.rotate_left(shift);
        let mut w = world(&values);
        // A distinct unknown activation remains an active uncertainty. Neither
        // the known zero nor a would-be overflow may hide either missing input.
        let zero = values.iter().position(|v| *v == Some(0)).unwrap();
        set_enabled(&mut w, zero, None);
        let r = report(&w);
        let actual = value(&r, ActorKey::Player).clone();
        assert!(matches!(actual, EffectValue::Unresolved { .. }));
        if let Some(expected) = &expected {
            assert_eq!(&actual, expected);
        } else {
            expected = Some(actual);
        }
    }
}

#[test]
fn exact_sum_preserves_membership_coverage_and_plan_limits() {
    let mut w = world(&[Some(4), Some(4)]);
    assert_values(&report(&w), 8, 0, 0);
    assert!(matches!(
        plan_with(
            &w,
            PlanLimits {
                max_effects: 2,
                ..Default::default()
            },
            |_| {}
        ),
        Err(PlanError::Limit("effects"))
    ));
    w.queries.members[0].groups[0].members.closure =
        partial(subject(def::<StatDefinition>("channel")));
    let p = plan(&w);
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    w.queries.members[0].groups[0].members = DeclaredSet::complete(vec![]);
    assert!(
        plan_with(&w, Default::default(), |_| {})
            .is_err_and(|e| { e.to_string().contains("no declared membership") })
    );
}

#[test]
fn exact_sum_requires_checked_query_and_does_not_relax_ordinary_sum() {
    let w = world(&[Some(4), Some(4)]);
    assert!(
        plan_with(&w, Default::default(), |rules| {
            let p = &mut rules
                .owners
                .iter_mut()
                .find(|o| o.owner == subject(def::<StatDefinition>("total")))
                .unwrap()
                .programs
                .members[0];
            p.reads[0].source = RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: def("channel"),
                contribution: ContributionKind::Add,
                reduction: ContributionReduction::ExactIntegerSum,
                empty: integer(0),
            };
        })
        .is_err_and(|e| e.to_string().contains("checked"))
    );
    assert!(
        plan_with(&w, Default::default(), |rules| {
            rules.contribution_queries.as_mut().unwrap().members[0].groups[0].reduction =
                ContributionReduction::Sum;
        })
        .is_err_and(|e| e.to_string().contains("semantic ordering"))
    );
}

#[test]
fn exact_sum_is_stable_across_scratch_reuse_parallel_workers_and_changed_values() {
    let a = plan(&world(&[
        Some(BoundedInteger::MAX),
        Some(5),
        Some(-BoundedInteger::MAX),
    ]));
    let b = plan(&world(&[
        Some(BoundedInteger::MAX),
        Some(9),
        Some(-BoundedInteger::MAX),
    ]));
    let first = a.evaluate(&mut a.new_scratch()).unwrap();
    assert_values(evaluated(&first), 5, 0, 0);
    let mut scratch = a.new_scratch();
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let changed = b.evaluate(&mut scratch).unwrap();
    assert_values(evaluated(&changed), 9, 0, 0);
    assert_eq!(changed, b.evaluate(&mut b.new_scratch()).unwrap());
    assert_ne!(first.identity, changed.identity);
    assert_eq!(a.evaluate(&mut scratch).unwrap(), first);
    let parallel: Vec<_> = (0..16)
        .into_par_iter()
        .map(|_| a.evaluate(&mut a.new_scratch()).unwrap())
        .collect();
    assert!(parallel.iter().all(|r| r == &first));
}
