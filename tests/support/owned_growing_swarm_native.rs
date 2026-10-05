//! The actual unconditional passive and receivers in the existing two-Sniper
//! component. The sums below check contribution inventories, not cooldown time.
use super::*;
use poe_optimizer_core::build_identity::InstanceAllocator;

fn packet<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(
        &fs::read(
            intrinsic::repository_root()
                .join("data/owned/poe2/3887ae68/growing-swarm")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn migration() -> OwnedReleaseMigrationInput {
    packet("migration.json")
}
fn closure() -> Closure {
    packet("closure.json")
}

fn world(selected: bool) -> World {
    let mut world = super::world(&super::nodes());
    let migration = migration();
    assert!(
        migration.tables.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    let f = &mut world.intrinsic.f;
    f.schema.schema_version = migration.contract.schema_version;
    assert_eq!(migration.schema.len(), 4);
    for entry in migration.schema {
        let SchemaExtensionEntry::Definition(descriptor) = entry else {
            panic!("Growing Swarm adds only four stat definitions")
        };
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|old| old.address() == descriptor.address())
        );
        f.schema.definitions.push(descriptor);
    }
    let closure = closure();
    assert_eq!((closure.definitions.len(), closure.owners.len()), (1, 1));
    let definition = fixture::finite_node(&closure.definitions[0]);
    let DefinitionDescriptor::PassiveNode(entry) = &definition else {
        panic!("passive descriptor")
    };
    assert_eq!(entry.id, def(0x0bc0));
    let SchemaState::Known(schema) = &entry.schema else {
        panic!("known passive")
    };
    assert_eq!(schema.pools.members.len(), 1);
    if selected {
        let mut allocator = InstanceAllocator::from_state(f.build.allocator);
        f.build.allocations.push(Allocation {
            id: allocator.allocate().unwrap(),
            node: entry.id.clone(),
            pool: schema.pools.members[0].clone(),
            scope: LoadoutScope::Shared,
            access: AllocationAccess::Ordinary,
            choices: vec![],
        });
        f.build.allocator = allocator.state();
    }
    assert!(
        !f.schema
            .definitions
            .iter()
            .any(|old| old.address() == definition.address())
    );
    f.schema.definitions.push(definition);
    for owner in migration.owners.into_iter().chain(closure.owners) {
        if let Some(existing) = f.owners.iter_mut().find(|old| old.owner == owner.owner) {
            // Existing finite Action owners retain every original conditional
            // and intrinsic program. Production Partial closure is tested below.
            for program in owner.programs.members {
                if let Some(old) = existing
                    .programs
                    .members
                    .iter()
                    .find(|old| old.id == program.id)
                {
                    assert_eq!(old, &program);
                } else {
                    existing.programs.members.push(program);
                }
            }
        } else {
            assert!(
                owner.programs.is_complete(),
                "new owned producers must be complete"
            );
            f.owners.push(owner);
        }
    }
    f.receivers.members.extend(migration.receivers);
    assert_eq!(f.build.skills.len(), 2);
    assert_eq!(
        f.queries.requests.len(),
        10,
        "no additional parent query or Action is fabricated"
    );
    world
}
fn new_action_program(selected: &ActionSelection) -> OwnedDefinitionKey {
    let subject = SchemaSubject::Slot(SlotAddress::ActionOutput(selected.action.output.clone()));
    let prior = super::migration()
        .owners
        .into_iter()
        .find(|owner| owner.owner == subject)
        .unwrap();
    let owner = migration()
        .owners
        .into_iter()
        .find(|owner| owner.owner == subject)
        .unwrap();
    let added: Vec<_> = owner
        .programs
        .members
        .iter()
        .filter(|program| {
            !prior
                .programs
                .members
                .iter()
                .any(|old| old.id == program.id)
        })
        .collect();
    assert_eq!(added.len(), 1);
    assert_eq!(added[0].id.as_str(), "unconditional-minion-cooldown");
    added[0].id.clone()
}
fn action_effects<'a>(
    report: &'a OwnedEffectsReport,
    selected: &ActionSelection,
) -> Vec<&'a BoundEffectResult> {
    super::contributions(report, &def(0x32ea)).into_iter().filter(|effect| matches!(&effect.target,
        BoundEffectTarget::Contribution { key } if key.entity == ConcreteEntity::Action(Box::new(selected.clone())))).collect()
}
fn check_action(report: &OwnedEffectsReport, selected: &ActionSelection, unconditional: f64) {
    let effects = action_effects(report, selected);
    assert_eq!(
        effects.len(),
        2,
        "independent conditional and unconditional contributions"
    );
    let conditional = super::bindings().programs.action;
    let added = new_action_program(selected);
    let is_gas = selected.action.output == super::output(0x24, 0x25);
    let mut programs = BTreeSet::new();
    for effect in &effects {
        assert!(programs.insert(effect.key.invocation.program.clone()));
        let amount = if effect.key.invocation.program == conditional {
            if is_gas { 72.0 } else { 0.0 }
        } else {
            assert_eq!(effect.key.invocation.program, added);
            unconditional
        };
        assert_eq!(
            effect.key.invocation.origin,
            RuleOrigin::Provider {
                provider: selected.action.provider.clone()
            }
        );
        assert_eq!(
            effect.key.invocation.owner,
            SchemaSubject::Slot(SlotAddress::ActionOutput(selected.action.output.clone()))
        );
        assert_eq!(
            effect.key.invocation.entity,
            ConcreteEntity::Action(Box::new(selected.clone()))
        );
        assert_eq!(
            effect.value,
            EffectValue::Known {
                value: intrinsic::quantity(amount, 2)
            }
        );
        let BoundEffectTarget::Contribution { key } = &effect.target else {
            unreachable!()
        };
        assert_eq!(key.kind, ContributionKind::Increase);
    }
    assert_eq!(
        effects
            .iter()
            .map(|effect| known(&effect.value))
            .sum::<f64>(),
        if is_gas {
            72.0 + unconditional
        } else {
            unconditional
        }
    );
}
fn check(world: &World, report: &OwnedEffectsReport, amount: f64) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    fixture::check(report, 24.0);
    assert_eq!(
        known(value(
            report,
            ConcreteEntity::Actor(ActorKey::Player),
            0x32e8
        )),
        72.0
    );
    for stat in [0x32f4, 0x32f6] {
        assert_eq!(
            value(report, ConcreteEntity::Actor(ActorKey::Player), stat),
            &EffectValue::Known {
                value: intrinsic::quantity(amount, 2)
            }
        );
        let effects = super::contributions(report, &def(stat));
        assert_eq!(effects.len(), usize::from(amount != 0.0));
        for effect in effects {
            let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
                panic!("allocation origin")
            };
            let ProviderRoot::Allocation(id) = provider.root else {
                panic!("physical allocation")
            };
            assert!(provider.grant_path.is_empty());
            let allocation = world
                .intrinsic
                .f
                .build
                .allocations
                .iter()
                .find(|row| row.id == id)
                .unwrap();
            assert_eq!(allocation.node, def(0x0bc0));
            assert_eq!(
                effect.key.invocation.owner,
                SchemaSubject::Definition(allocation.node.address())
            );
            assert_eq!(
                effect.key.invocation.entity,
                ConcreteEntity::Actor(ActorKey::Player)
            );
            assert_eq!(
                effect.key.invocation.program.as_str(),
                "ordinary-minion-area-and-cooldown"
            );
            assert_eq!(
                effect.value,
                EffectValue::Known {
                    value: intrinsic::quantity(20.0, 2)
                }
            );
            let BoundEffectTarget::Contribution { key } = &effect.target else {
                unreachable!()
            };
            assert_eq!(key.entity, ConcreteEntity::Actor(ActorKey::Player));
            assert_eq!(key.kind, ContributionKind::Increase);
        }
    }
    for index in 0..2 {
        for stat in [0x32f5, 0x32f7] {
            assert_eq!(
                fixture::actor_value(report, index, stat),
                &EffectValue::Known {
                    value: intrinsic::quantity(amount, 2)
                }
            );
        }
        assert_eq!(
            fixture::actor_value(report, index, 0x1c),
            &EffectValue::Known {
                value: ParameterValue::Integer(BoundedInteger::new([44, 2][index]).unwrap())
            }
        );
        for selected in super::actions(index) {
            check_action(report, &selected, amount);
        }
    }
    assert_ne!(intrinsic::actor(0), intrinsic::actor(1));
    assert!(
        !report.values.iter().any(|entry| matches!(&entry.key,
        PlanValueKey::Stat { stat, .. } if *stat == def(0x32ea))),
        "no final Action cooldown rate or duration reducer"
    );
}

#[test]
fn both_actual_passive_effects_reach_two_actors_and_all_existing_action_alternatives() {
    let world = world(true);
    check(&world, &world.evaluate(), 20.0);
    let removed = self::world(false);
    check(&removed, &removed.evaluate(), 0.0);
}

#[test]
fn missing_or_partial_passive_and_actual_partial_action_owners_keep_complete_contributor_gate() {
    for partial in [false, true] {
        let mut world = world(true);
        let subject = SchemaSubject::Definition(DefinitionAddress::PassiveNode(def(0x0bc0)));
        if partial {
            world.intrinsic.f.owner_mut(&subject).programs.closure = SchemaClosure::Partial {
                gaps: vec![SchemaGap {
                    subject: subject.clone(),
                    facet: SchemaFacet::GameRules,
                    code: intrinsic::key("fixture-incomplete-growing-swarm"),
                }],
            };
        } else {
            world
                .intrinsic
                .f
                .owners
                .retain(|owner| owner.owner != subject);
        }
        let report = world.evaluate();
        let reason = if partial {
            PlanGapReason::PartialPrograms
        } else {
            PlanGapReason::MissingPrograms
        };
        assert!(
            report
                .gaps
                .iter()
                .any(|gap| gap.subject.as_ref() == Some(&subject) && gap.reason == reason)
        );
        for stat in [0x32f4, 0x32f6] {
            assert!(matches!(
                value(&report, ConcreteEntity::Actor(ActorKey::Player), stat),
                EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    ..
                }
            ));
        }
    }
    let mut world = world(true);
    let owner = migration()
        .owners
        .into_iter()
        .find(|owner| {
            owner.owner == SchemaSubject::Slot(SlotAddress::ActionOutput(super::output(0x24, 0x25)))
        })
        .unwrap();
    assert!(!owner.programs.is_complete());
    world.intrinsic.f.owner_mut(&owner.owner).programs.closure = owner.programs.closure;
    let report = world.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::PartialPrograms)
    );
    for index in 0..2 {
        for selected in super::actions(index) {
            for effect in action_effects(&report, &selected) {
                assert!(matches!(
                    effect.value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    }
                ));
            }
        }
    }
}

#[test]
fn parent_inputs_and_actual_entering_grants_are_required_for_unconditional_delivery() {
    let mut world = world(true);
    world.intrinsic.missing_final_input();
    let report = world.evaluate();
    for index in 0..2 {
        for selected in super::actions(index) {
            let effects = action_effects(&report, &selected);
            assert_eq!(effects.len(), 2);
            for effect in effects {
                assert!(matches!(
                    effect.value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::MissingProducer,
                        ..
                    }
                ));
            }
        }
    }
    let mut world = self::world(true);
    let actor = world
        .intrinsic
        .f
        .owner_mut(&SchemaSubject::Definition(DefinitionAddress::Actor(def(
            0x3091,
        ))));
    let program = actor
        .programs
        .members
        .iter_mut()
        .find(|program| program.id.as_str() == "gas-arrow-supply")
        .unwrap();
    let before = program.effects.len();
    program.effects.retain(|effect| {
        !matches!(&effect.effect,
        RuleEffectKind::ActivateGrant { slot, .. } if slot.slot == def(0x3095))
    });
    assert_eq!(program.effects.len() + 1, before);
    let report = world.evaluate();
    for index in 0..2 {
        for selected in super::actions(index) {
            let effects = action_effects(&report, &selected);
            assert_eq!(effects.len(), 2);
            for effect in effects {
                assert!(matches!(
                    effect.value,
                    EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    }
                ));
            }
        }
    }
}

#[test]
fn disabled_ancestor_cannot_create_area_receivers_or_unconditional_actions() {
    let mut world = world(true);
    world.intrinsic.f.build.skills[1].enabled = false;
    let root = ProviderRoot::SkillUse(world.intrinsic.f.build.skills[1].id);
    let report = world.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|gap| gap.reason == PlanGapReason::UnresolvedTopology)
    );
    assert!(
        !report
            .effects
            .iter()
            .any(|effect| matches!(&effect.key.invocation.entity,
        ConcreteEntity::Action(selected) if selected.action.provider.root == root))
    );
    assert!(!report.values.iter().any(|entry| matches!(&entry.key,
        PlanValueKey::Stat { entity: ConcreteEntity::Actor(actor), .. } if *actor == intrinsic::actor(1))));
    world.intrinsic.f.queries.requests.retain(|query| {
        !matches!(&query.target,
        MetricTarget::Action(selected) if selected.action.provider.root == root)
    });
    let report = world.evaluate();
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for stat in [0x32f5, 0x32f7] {
        assert_eq!(known(fixture::actor_value(&report, 0, stat)), 20.0);
    }
    for selected in super::actions(0) {
        check_action(&report, &selected, 20.0);
    }
}

#[test]
fn removal_and_loadout_candidates_restore_a_b_a_scratch_on_each_rayon_worker() {
    let mut world = world(true);
    let loadouts = world.intrinsic.f.build.weapon_loadouts.clone();
    let node = world
        .intrinsic
        .f
        .build
        .allocations
        .iter_mut()
        .find(|node| node.node == def(0x0bc0))
        .unwrap();
    node.scope = LoadoutScope::Selected {
        loadouts: vec![loadouts[1]],
    };
    let without = world.compile();
    world.intrinsic.f.build.active_weapon_loadout = loadouts[1];
    let with = world.compile();
    let mut scratch = with.new_scratch();
    let a = with.evaluate(&mut scratch).unwrap();
    check(&world, &a, 20.0);
    let b = without.evaluate(&mut scratch).unwrap();
    check(&world, &b, 0.0);
    for _ in 0..3 {
        assert_eq!(with.evaluate(&mut scratch).unwrap(), a);
        assert_eq!(without.evaluate(&mut scratch).unwrap(), b);
        assert_eq!(with.evaluate(&mut scratch).unwrap(), a);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..32).into_par_iter().for_each_init(
                || with.new_scratch(),
                |scratch, _| {
                    assert_eq!(with.evaluate(scratch).unwrap(), a);
                    assert_eq!(without.evaluate(scratch).unwrap(), b);
                    assert_eq!(with.evaluate(scratch).unwrap(), a);
                },
            );
        });
}
