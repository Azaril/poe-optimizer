//! The published empty-attribute minion domain, exercised through portable APIs.
//! Synthetic donors below test refusal boundaries, not nonempty attribute parity.
use super::*;
use poe_optimizer_core::owned_schema::*;

const BRIDGE: &str = "contribute-inherent-strength-life";
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn packet(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            root()
                .join("data/owned/poe2/3887ae68/minion-inherent-life")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn source_owner() -> DefinitionRules {
    serde_json::from_value(packet("dependencies.json")["owner_before"].clone()).unwrap()
}
fn owner() -> SchemaSubject {
    serde_json::from_value(packet("bindings.json")["owner"].clone()).unwrap()
}
fn slot() -> DeclaredSlot<ActorSlotDefId> {
    serde_json::from_value(packet("bindings.json")["slot"].clone()).unwrap()
}
fn known(value: ParameterValue) -> EffectValue {
    EffectValue::Known { value }
}
fn integer(n: i64) -> EffectValue {
    known(ParameterValue::Integer(BoundedInteger::new(n).unwrap()))
}
fn life(n: f64) -> EffectValue {
    known(ParameterValue::Quantity(
        FiniteQuantity::new(n, def(0x3119)).unwrap(),
    ))
}
fn bridge_rows<'a>(r: &'a SupportEffectsReport, actor: &ActorKey) -> Vec<&'a BoundEffectResult> {
    effects(r)
        .effects
        .iter()
        .filter(|row| {
            row.key.invocation.program == key(BRIDGE)
                && row.key.invocation.entity == ConcreteEntity::Actor(actor.clone())
        })
        .collect()
}
fn check_player(r: &SupportEffectsReport) {
    assert_eq!(*value(r, &ActorKey::Player, 0x1d2e), integer(27));
    assert_eq!(*value(r, &ActorKey::Player, 0x331a), life(54.));
    let rows = bridge_rows(r, &ActorKey::Player);
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, life(54.));
    assert_eq!(
        rows[0].key.invocation.owner,
        SchemaSubject::Definition(DefinitionAddress::Actor(def(0x332a)))
    );
}
fn no_known_value(r: &SupportEffectsReport, actor: &ActorKey, stat: u64) {
    let row = effects(r).values.iter().find(|row| {
        row.key
            == PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(actor.clone()),
                stat: def(stat),
            }
    });
    assert!(row.is_none_or(|row| matches!(row.value, EffectValue::Unresolved { .. })));
}
fn check_zero(i: &ReplayInput, r: &SupportEffectsReport) {
    let recipients = actors(i);
    let receivers: Vec<StatReceiver> = serde_json::from_value(packet("receivers.json")).unwrap();
    assert_eq!(recipients.len(), 2);
    for actor in recipients {
        for stat in [0x3321, 0x1d2e] {
            assert_eq!(*value(r, &actor, stat), integer(0));
        }
        for stat in 0x3315..=0x3319 {
            assert_eq!(
                *value(r, &actor, stat),
                known(ParameterValue::Boolean(false))
            );
        }
        assert_eq!(*value(r, &actor, 0x331a), life(0.));
        for receiver in &receivers {
            let rows: Vec<_> = effects(r)
                .effects
                .iter()
                .filter(|row| {
                    row.key.invocation.origin
                        == RuleOrigin::Receiver {
                            receiver: receiver.id.clone(),
                            actor: actor.clone(),
                        }
                })
                .collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].key.invocation.entity,
                ConcreteEntity::Actor(actor.clone())
            );
            assert_eq!(rows[0].key.invocation.program, receiver.program);
        }
        // Zero BASE is the authored lazy branch. No minion MORE identity was
        // installed or inherited from the Player to make that branch succeed.
        for stat in [0x3324, 0x3327] {
            no_known_value(r, &actor, stat);
        }
        let rows = bridge_rows(r, &actor);
        assert_eq!(
            rows.len(),
            1,
            "one explicit zero, not absence or duplication"
        );
        let row = rows[0];
        assert_eq!(row.key.invocation.owner, owner());
        assert_eq!(row.value, life(0.));
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: def(0x311a),
                    kind: ContributionKind::Add,
                },
            }
        );
        let ActorKey::Owned(actor) = actor else {
            unreachable!()
        };
        let mut provider = actor.provider;
        provider.grant_path.push(DeclaredSlot {
            declaration: SlotOwnerDefId::Skill(def(0x12)),
            slot: def(0x20),
        });
        assert_eq!(row.key.invocation.origin, RuleOrigin::Provider { provider });
    }
    check_player(r);
}
fn remove_minion_target(i: &mut ReplayInput, id: &str) {
    let receiver = i
        .rules
        .receivers
        .members
        .iter_mut()
        .find(|r| r.id == key(id))
        .unwrap();
    assert_eq!(
        receiver.targets,
        vec![
            StatReceiverTarget::Player,
            StatReceiverTarget::OwnedSlot { slot: slot() }
        ]
    );
    receiver
        .targets
        .retain(|target| *target != StatReceiverTarget::OwnedSlot { slot: slot() });
    assert_eq!(receiver.targets, vec![StatReceiverTarget::Player]);
    i.rebind_test_edit().unwrap();
}
fn unresolved_bridge(i: &ReplayInput, r: &SupportEffectsReport) {
    for actor in actors(i) {
        let rows = bridge_rows(r, &actor);
        assert_eq!(rows.len(), 1);
        assert!(matches!(rows[0].value, EffectValue::Unresolved { .. }));
    }
    check_player(r);
}
fn refused(r: &SupportEffectsReport, reason: PlanGapReason) {
    assert!(matches!(
        r.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                ..
            },
            ..
        }
    ));
    assert!(
        r.gaps.iter().any(|gap| gap.reason == reason),
        "{:?}",
        r.gaps
    );
}

#[test]
fn published_minion_receivers_preserve_player_and_explicit_zero_sources() {
    let i = load();
    let receivers: Vec<StatReceiver> = serde_json::from_value(packet("receivers.json")).unwrap();
    assert_eq!(receivers.len(), 8);
    for receiver in receivers {
        assert_eq!(
            receiver.targets,
            vec![
                StatReceiverTarget::Player,
                StatReceiverTarget::OwnedSlot { slot: slot() }
            ]
        );
        assert_eq!(
            i.rules
                .receivers
                .members
                .iter()
                .filter(|r| **r == receiver)
                .count(),
            1
        );
    }
    let bridge: RuleProgram = serde_json::from_value(packet("program.json")).unwrap();
    for owner in [
        owner(),
        SchemaSubject::Definition(DefinitionAddress::Actor(def(0x332a))),
    ] {
        let actual = i.rules.owners.iter().find(|o| o.owner == owner).unwrap();
        assert_eq!(
            actual
                .programs
                .members
                .iter()
                .filter(|p| **p == bridge)
                .count(),
            1
        );
    }
    let r = run(&i);
    check_zero(&i, &r);
    let mut reordered = i;
    reordered.build.skills.reverse();
    reordered.build.gems.reverse();
    reordered.build.allocations.reverse();
    assert_eq!(run(&reordered), r);
}

#[test]
fn missing_minion_receivers_never_become_zero_or_false() {
    let original = load();
    let receivers: Vec<StatReceiver> = serde_json::from_value(packet("receivers.json")).unwrap();
    for receiver in receivers {
        let mut i = original.clone();
        remove_minion_target(&mut i, receiver.id.as_str());
        let r = run(&i);
        for actor in actors(&i) {
            let row = effects(&r).values.iter().find(|row| {
                row.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Actor(actor.clone()),
                        stat: receiver.stat.clone(),
                    }
            });
            assert!(row.is_none_or(|row| matches!(row.value, EffectValue::Unresolved { .. })));
        }
        if receiver.stat == def(0x3321) {
            // The second pass reads its own BASE channel, not the first result.
            for actor in actors(&i) {
                assert_eq!(*value(&r, &actor, 0x1d2e), integer(0));
                assert_eq!(bridge_rows(&r, &actor)[0].value, life(0.));
            }
            check_player(&r);
        } else {
            unresolved_bridge(&i, &r);
        }
    }
}

// These synthetic potential producers are confined to a negative fixture.
// Giving an effect zero or an inactive condition must not remove its obligation
// to occur in the checked contribution query's source inventory.
fn synthetic_donor(
    i: &mut ReplayInput,
    stat: u64,
    amount: f64,
    active: bool,
) -> OwnedDefinitionKey {
    let subject = owner();
    let owner = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject)
        .unwrap();
    let p = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("finite-actor-baseline"))
        .unwrap();
    let id = key(&format!("synthetic-minion-{stat:x}"));
    p.nodes.push(RuleNode {
        id: id.clone(),
        expression: RuleExpression::Literal {
            value: ParameterValue::Quantity(FiniteQuantity::new(amount, def(0x295a)).unwrap()),
        },
    });
    let condition = key(&format!("synthetic-active-{stat:x}"));
    p.nodes.push(RuleNode {
        id: condition.clone(),
        expression: RuleExpression::Literal {
            value: ParameterValue::Boolean(active),
        },
    });
    p.effects.push(RuleEffect {
        id: id.clone(),
        when: Some(condition),
        effect: RuleEffectKind::Contribute {
            entity: RuleEntity::Current,
            stat: def(stat),
            contribution: ContributionKind::Add,
            value: id.clone(),
        },
    });
    id
}

#[test]
fn unlisted_zero_inactive_and_nonzero_minion_donors_are_rejected() {
    let original = load();
    for (amount, active) in [(0., true), (1., false), (1., true)] {
        let mut i = original.clone();
        synthetic_donor(&mut i, 0x331b, amount, active);
        i.rebind_test_edit().unwrap();
        let error = i
            .compile()
            .err()
            .expect("even a zero or inactive potential donor needs membership");
        assert!(error.contains("no declared membership"), "{error}");
    }
}

#[test]
fn nonempty_minion_base_needs_its_own_more_producer() {
    let mut i = load();
    // This does not admit a new production attribute domain. It establishes
    // that adding checked positive BASE cannot silently obtain Player MORE=1.
    for (stat, query) in [
        (0x331b, "strength-first-base"),
        (0x331e, "strength-second-base"),
    ] {
        let effect = synthetic_donor(&mut i, stat, 1., true);
        let group = &mut i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| q.id == key(query))
            .unwrap()
            .groups[0];
        group.members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: owner(),
                program: key("finite-actor-baseline"),
                effect,
                origin: ContributionOrigin::SuppliedActor {
                    slots: vec![slot()],
                },
            }),
            order: Some(ContributionOrder {
                source_rank: 10_000,
                program_rank: 0,
                effect_rank: 0,
                slot_ranks: vec![],
            }),
        });
    }
    i.rebind_test_edit().unwrap();
    let r = run(&i);
    for actor in actors(&i) {
        for stat in [0x3321, 0x1d2e, 0x331a] {
            assert!(matches!(
                value(&r, &actor, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    unresolved_bridge(&i, &r);
}

#[test]
fn incomplete_minion_authority_and_query_inventories_refuse_evaluation() {
    let original = load();
    let mut partial = original.clone();
    let actual = source_owner();
    assert!(matches!(
        actual.programs.closure,
        SchemaClosure::Partial { .. }
    ));
    partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actual.owner)
        .unwrap()
        .programs
        .closure = actual.programs.closure;
    partial.rebind_test_edit().unwrap();
    refused(&run(&partial), PlanGapReason::PartialPrograms);
    for query in [
        "strength-first-base",
        "strength-second-base",
        "strength-first-increase",
        "inherent-flag-all-inherent-disabled",
    ] {
        let mut partial = original.clone();
        let q = partial
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| q.id == key(query))
            .unwrap();
        q.groups[0].members.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(DefinitionAddress::Stat(q.stat.clone())),
                facet: SchemaFacet::GameRules,
                code: key("unknown-minion-contributor-domain"),
            }],
        };
        partial.rebind_test_edit().unwrap();
        refused(&run(&partial), PlanGapReason::IncompleteContributors);
    }
}

#[test]
fn minion_bridge_and_consumers_require_frozen_prerequisites() {
    let original = load();
    for (owner, program) in [
        (owner(), BRIDGE),
        (
            SchemaSubject::Definition(DefinitionAddress::Stat(def(0x331a))),
            "inherent-strength-life",
        ),
        (
            SchemaSubject::Definition(DefinitionAddress::Stat(def(0x3321))),
            "strength-first-step",
        ),
    ] {
        let mut early = original.clone();
        early
            .stages
            .programs
            .members
            .iter_mut()
            .find(|p| p.owner == owner && p.program == key(program))
            .unwrap()
            .stage = key("facts");
        let error = early
            .rebind_test_edit()
            .expect_err("receiver and bridge reads require their frozen sources");
        assert!(
            error.contains("frozen channel read occurs before or outside frozen stage"),
            "{error}"
        );
    }
}

#[test]
fn minion_zero_unknown_and_changed_builds_recover_across_four_workers() {
    let original = load();
    let mut unknown = original.clone();
    remove_minion_target(&mut unknown, "player-strength-life-disabled");
    let mut changed = original.clone();
    quality(&mut changed, [1., 20.]);
    changed.build.skills.reverse();
    let inputs = [&original, &unknown, &changed];
    let plans = inputs.map(|i| i.compile().unwrap());
    let fresh = plans
        .each_ref()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap());
    check_zero(&original, &fresh[0]);
    unresolved_bridge(&unknown, &fresh[1]);
    check_zero(&changed, &fresh[2]);
    assert_ne!(fresh[0], fresh[1]);
    assert_ne!(
        fresh[0], fresh[2],
        "the B control changes real supported quality consumers"
    );
    let mut scratch = plans[0].new_scratch();
    for n in [0, 1, 2, 0] {
        assert_eq!(plans[n].evaluate(&mut scratch).unwrap(), fresh[n]);
    }
    let reports = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || plans[0].new_scratch(),
                    |s, n| plans[[0, 1, 2, 0][n % 4]].evaluate(s).unwrap(),
                )
                .collect::<Vec<_>>()
        });
    for (n, report) in reports.into_iter().enumerate() {
        assert_eq!(report, fresh[[0, 1, 2, 0][n % 4]]);
    }
}
