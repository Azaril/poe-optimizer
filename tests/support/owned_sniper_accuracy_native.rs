//! Counterfactual controls on the same item-driven Sniper graph as the source
//! replay. Synthetic contributors have explicit finite membership and do not
//! publish coverage, import custom modifiers or replace any game formula.
use super::*;

fn node(name: &str, expression: RuleExpression) -> RuleNode {
    RuleNode {
        id: key(name),
        expression,
    }
}
fn literal(name: &str, value: ParameterValue) -> RuleNode {
    node(name, RuleExpression::Literal { value })
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    sniper::offering::effects(report)
}
fn close(actual: &EffectValue, expected: f64) {
    let EffectValue::Known {
        value: ParameterValue::Quantity(value),
    } = actual
    else {
        panic!("expected {expected}, got {actual:?}")
    };
    assert!(
        (value.value() - expected).abs() < 1e-10,
        "{actual:?} != {expected}"
    );
}
impl World {
    fn inner_mut(&mut self) -> &mut shared::World {
        &mut self.sniper.base.source.base.inner
    }
    fn accuracy_query_mut(&mut self, stat: u64) -> &mut ContributionQuery {
        let rows: Vec<_> = self
            .sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .filter(|q| q.stat == d(stat))
            .collect();
        assert_eq!(rows.len(), 1);
        rows.into_iter().next().unwrap()
    }
    pub(super) fn set_block(&mut self, present: Option<bool>, raw: Option<f64>) {
        self.block
            .assumptions
            .retain(|a| a.input != d(0x3216) && a.input != d(0x3217));
        if let Some(present) = present {
            self.block.assumptions.push(ExternalAssumption {
                input: d(0x3216),
                target: AssumptionTarget::Enemy,
                value: ParameterValue::Boolean(present),
            });
        }
        if let Some(raw) = raw {
            self.block.assumptions.push(ExternalAssumption {
                input: d(0x3217),
                target: AssumptionTarget::Enemy,
                value: quantity(raw, &d(2)),
            });
        }
    }
    pub(super) fn check_accuracy(
        &self,
        report: &SupportEffectsReport,
        index: usize,
        block: f64,
        hit: f64,
    ) {
        let report = effects(report);
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        assert_eq!(
            self.value(report, index, false, 0x321a),
            &EffectValue::Known {
                value: ParameterValue::Boolean(true),
            }
        );
        close(self.value(report, index, true, 0x321c), 100.);
        close(self.value(report, index, true, 0x321f), block);
        close(self.value(report, index, true, 0x3220), hit);
    }
    fn class_owner(&self) -> SchemaSubject {
        subject(
            self.sniper
                .base
                .source
                .base
                .inner
                .build
                .character
                .class
                .clone(),
        )
    }
    fn counterfactual(
        &mut self,
        name: &str,
        entity: RuleEntity,
        stat: u64,
        kind: ContributionKind,
        value: ParameterValue,
    ) {
        let owner = self.class_owner();
        let members = &mut self.inner_mut().owner_mut(owner.clone()).programs.members;
        assert!(!members.iter().any(|p| p.id == key(name)));
        members.push(RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![literal("value", value)],
            effects: vec![RuleEffect {
                id: key("source"),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity,
                    stat: d(stat),
                    contribution: kind,
                    value: key("value"),
                },
            }],
        });
        if kind == ContributionKind::Flag {
            let query = self.accuracy_query_mut(stat);
            assert_eq!(query.contribution, ContributionKind::Flag);
            assert_eq!(query.groups.len(), 1);
            let group = &mut query.groups[0];
            assert_eq!(group.reduction, ContributionReduction::Any);
            assert_eq!(group.ordering, ContributionOrdering::Unordered);
            group.members.members.push(ContributionMember {
                owner,
                program: key(name),
                effect: key("source"),
                origin: ContributionOrigin::Character,
                order: None,
            });
        }
    }
    pub(super) fn flag(&mut self, name: &str, entity: RuleEntity, stat: u64, value: bool) {
        self.counterfactual(
            name,
            entity,
            stat,
            ContributionKind::Flag,
            ParameterValue::Boolean(value),
        );
    }
    fn counterfactual_mut(&mut self, name: &str) -> &mut RuleProgram {
        let owner = self.class_owner();
        self.inner_mut()
            .owner_mut(owner)
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == key(name))
            .unwrap()
    }
    fn unknown_flag(&mut self, name: &str, entity: RuleEntity, stat: u64) {
        self.flag(name, entity, stat, false);
        let missing = def::<StatDefinition>(&format!("fixture.accuracy-missing.{name}"));
        self.inner_mut()
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: missing.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Actor],
                }),
            }));
        let program = self.counterfactual_mut(name);
        program.reads.push(RuleRead {
            id: key("missing"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Player,
                stat: missing,
            },
        });
        program.nodes[0].expression = RuleExpression::Read {
            input: key("missing"),
        };
    }
    fn action_reduction(&mut self, first: f64, second: f64) {
        // Distinct exact child actors are selected by their computed actor level;
        // no source occurrence identity is used as a gameplay predicate.
        self.inner_mut()
            .owner_mut(subject(d::<SkillDefinition>(0x21)))
            .programs
            .members
            .push(RuleProgram {
                id: key("counterfactual-action-reduction"),
                context: RuleEntityKind::Action,
                reads: vec![RuleRead {
                    id: key("level"),
                    value_type: ComputedValueType::Integer,
                    source: RuleReadSource::Stat {
                        entity: RuleEntity::Actor,
                        stat: d(0x1c),
                    },
                }],
                nodes: vec![
                    node(
                        "level",
                        RuleExpression::Read {
                            input: key("level"),
                        },
                    ),
                    literal("forty-four", integer(44)),
                    node(
                        "first",
                        RuleExpression::Compare {
                            operation: RuleComparison::Equal,
                            left: key("level"),
                            right: key("forty-four"),
                        },
                    ),
                    literal("a", quantity(first, &d(2))),
                    literal("b", quantity(second, &d(2))),
                    node(
                        "amount",
                        RuleExpression::Select {
                            condition: key("first"),
                            when_true: key("a"),
                            when_false: key("b"),
                        },
                    ),
                ],
                effects: vec![RuleEffect {
                    id: key("reduction"),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: d(0x321d),
                        contribution: ContributionKind::Add,
                        value: key("amount"),
                    },
                }],
            });
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn boolean_accuracy_keeps_false_duplicate_sources_and_exact_recipients() {
    let mut w = World::load();
    w.set_block(Some(true), Some(37.));
    w.check_accuracy(&w.evaluate(), 0, 37., 63.); // Complete empty Any is false.
    w.flag("counterfactual-false-a", RuleEntity::Enemy, 0x321e, false);
    w.flag("counterfactual-false-b", RuleEntity::Enemy, 0x321e, false);
    w.check_accuracy(&w.evaluate(), 0, 37., 63.);
    w.flag("counterfactual-true-a", RuleEntity::Enemy, 0x321e, true);
    w.flag("counterfactual-true-b", RuleEntity::Enemy, 0x321e, true);
    let report = w.evaluate();
    for index in 0..2 {
        w.check_accuracy(&report, index, 0., 100.);
    }
    let flags: Vec<_> = effects(&report)
        .effects
        .iter()
        .filter(|row| {
            matches!(&row.target,
        BoundEffectTarget::Contribution { key } if key.stat == d(0x321e))
        })
        .collect();
    assert_eq!(
        flags.len(),
        4,
        "equal Boolean values preserve distinct sources"
    );
    for flag in flags {
        let BoundEffectTarget::Contribution { key } = &flag.target else {
            unreachable!()
        };
        assert_eq!(key.kind, ContributionKind::Flag);
        assert_eq!(key.entity, ConcreteEntity::Enemy);
        assert_eq!(
            flag.key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::Character,
                    grant_path: vec![],
                }
            }
        );
    }
    let mut permuted = w.clone();
    permuted.inner_mut().owners.reverse();
    permuted.accuracy_query_mut(0x321e).groups[0]
        .members
        .members
        .reverse();
    let reordered = permuted.evaluate();
    for index in 0..2 {
        permuted.check_accuracy(&reordered, index, 0., 100.);
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn boolean_accuracy_does_not_short_circuit_unknown_sources_or_invent_inheritance() {
    let mut w = World::load();
    w.set_block(Some(true), Some(37.));
    w.flag("counterfactual-known", RuleEntity::Enemy, 0x321e, true);
    w.unknown_flag("counterfactual-unknown", RuleEntity::Enemy, 0x321e);
    let unknown = w.evaluate();
    for index in 0..2 {
        assert!(matches!(
            w.value(effects(&unknown), index, true, 0x3220),
            EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                ..
            }
        ));
    }
    let p = w.counterfactual_mut("counterfactual-unknown");
    p.nodes
        .push(literal("disabled", ParameterValue::Boolean(false)));
    p.effects[0].when = Some(key("disabled"));
    w.check_accuracy(&w.evaluate(), 0, 0., 100.);

    let mut inherited = World::load();
    inherited.set_block(Some(true), Some(37.));
    inherited.flag(
        "counterfactual-inheritance-false",
        RuleEntity::Player,
        0x3219,
        false,
    );
    inherited.check_accuracy(&inherited.evaluate(), 0, 37., 63.);
    inherited.flag(
        "counterfactual-inheritance-true",
        RuleEntity::Player,
        0x3219,
        true,
    );
    let report = inherited.evaluate();
    for index in 0..2 {
        assert_eq!(
            inherited.value(effects(&report), index, false, 0x321a),
            &EffectValue::Known {
                value: ParameterValue::Boolean(false),
            }
        );
        assert!(!matches!(
            inherited.value(effects(&report), index, true, 0x321c),
            EffectValue::Known { .. }
        ));
        assert!(matches!(
            inherited.value(effects(&report), index, true, 0x3220),
            EffectValue::Unresolved { .. }
        ));
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn boolean_accuracy_rejects_numeric_flags_direct_reads_wrong_recipients_and_missing_members() {
    let mut wrong_value = World::load();
    wrong_value.counterfactual(
        "counterfactual-numeric",
        RuleEntity::Enemy,
        0x321e,
        ContributionKind::Flag,
        integer(1),
    );
    assert!(
        wrong_value
            .checked_plan()
            .err()
            .expect("numeric Flag must fail")
            .contains("Flag requires Boolean stat and contribution value")
    );

    let mut wrong_kind = World::load();
    wrong_kind.counterfactual(
        "counterfactual-add",
        RuleEntity::Enemy,
        0x321e,
        ContributionKind::Add,
        integer(1),
    );
    assert!(
        wrong_kind
            .checked_plan()
            .err()
            .expect("numeric Add on Boolean channel must fail")
            .contains("numeric contribution requires numeric stat")
    );

    let mut wrong_recipient = World::load();
    wrong_recipient.flag("counterfactual-recipient", RuleEntity::Player, 0x321e, true);
    assert!(
        wrong_recipient
            .checked_plan()
            .err()
            .expect("Enemy channel cannot receive Player flag")
            .contains("ordered producer recipient is not admitted by its stat")
    );

    let mut missing = World::load();
    missing.flag("counterfactual-unlisted", RuleEntity::Enemy, 0x321e, false);
    missing.accuracy_query_mut(0x321e).groups[0]
        .members
        .members
        .clear();
    assert!(
        missing
            .checked_plan()
            .err()
            .expect("all concrete flags require membership")
            .contains("actual contribution has no declared membership")
    );

    let mut direct = World::load();
    let program = direct
        .inner_mut()
        .owner_mut(subject(d::<SkillDefinition>(0x21)))
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("ordinary-minion-attack-hit-chance"))
        .unwrap();
    let read = program
        .reads
        .iter_mut()
        .find(|r| r.id == key("cannot-block-flags"))
        .unwrap();
    read.source = RuleReadSource::Contributions {
        entity: RuleEntity::Enemy,
        stat: d(0x321e),
        contribution: ContributionKind::Flag,
        reduction: ContributionReduction::Any,
        empty: ParameterValue::Boolean(false),
    };
    assert!(
        direct
            .checked_plan()
            .err()
            .expect("public direct flag reduction is forbidden")
            .contains("Boolean contributions require checked query membership")
    );
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn boolean_accuracy_partial_inventory_remains_unavailable_even_with_true() {
    let mut w = World::load();
    w.flag("counterfactual-present", RuleEntity::Enemy, 0x321e, true);
    let actual = w
        .actual_accuracy_queries
        .members
        .iter()
        .find(|q| q.stat == d(0x321e))
        .unwrap();
    let closure = actual.groups[0].members.closure.clone();
    assert!(!actual.groups[0].members.is_complete());
    w.accuracy_query_mut(0x321e).groups[0].members.closure = closure;
    let p = w.plan();
    assert_eq!(
        p.gaps(),
        &[PlanGap {
            provider: None,
            subject: Some(subject(d::<StatDefinition>(0x321e))),
            reason: PlanGapReason::IncompleteContributors
        }]
    );
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(matches!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    ));
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn accuracy_block_preserves_aggregate_clamp_order_and_exact_child_action_scope() {
    let mut w = World::load();
    w.sniper.raw(0, 20, 0., 0.); // final22 / actor44
    w.sniper.raw(1, 1, 0., 0.); // final3 / actor6
    w.set_block(Some(true), Some(37.));
    w.counterfactual(
        "counterfactual-additional-block",
        RuleEntity::Enemy,
        0x3218,
        ContributionKind::Add,
        quantity(12.5, &d(2)),
    );
    w.action_reduction(15., 3.);
    let report = w.evaluate();
    w.check_accuracy(&report, 0, 34.5, 65.5);
    w.check_accuracy(&report, 1, 46.5, 53.5);
    let reductions: Vec<_> = effects(&report)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key("counterfactual-action-reduction"))
        .collect();
    assert_eq!(reductions.len(), 2);
    for index in 0..2 {
        let destination = ConcreteEntity::Action(Box::new(w.action(index)));
        assert_eq!(reductions.iter().filter(|row| matches!(&row.target,
            BoundEffectTarget::Contribution { key } if key.entity == destination && key.stat == d(0x321d))).count(), 1);
    }
    for (block, reduction, effective) in [
        (140., 30., 70.),
        (37., 50., 0.),
        (99., -10., 109.),
        (-7., -10., 3.),
        (37.25, 0.125, 37.125),
    ] {
        let mut w = World::load();
        w.set_block(Some(true), Some(block));
        w.action_reduction(reduction, reduction);
        for index in 0..2 {
            w.check_accuracy(&w.evaluate(), index, effective, 100. - effective);
        }
    }
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn accuracy_missing_assumptions_and_intrinsic_producer_never_default_to_known() {
    for (presence, raw) in [(None, None), (Some(true), None)] {
        let mut w = World::load();
        w.set_block(presence, raw);
        let report = w.evaluate();
        for index in 0..2 {
            assert!(matches!(
                w.value(effects(&report), index, true, 0x3220),
                EffectValue::Unresolved { .. }
            ));
        }
    }
    let mut absent = World::load();
    absent.set_block(Some(false), None);
    absent.check_accuracy(&absent.evaluate(), 0, 0., 100.);
    let mut missing = World::load();
    let owner = missing
        .inner_mut()
        .owner_mut(SchemaSubject::Slot(SlotAddress::Actor(actor_slot())));
    let length = owner.programs.members.len();
    owner
        .programs
        .members
        .retain(|p| p.id != key("intrinsic-minion-cannot-be-evaded"));
    assert_eq!(owner.programs.members.len() + 1, length);
    let report = missing.evaluate();
    assert!(matches!(
        missing.value(effects(&report), 0, true, 0x3220),
        EffectValue::Unresolved { .. }
    ));
}

#[test]
#[ignore = "requires POE_OPTIMIZER_TEST_SNIPER_ITEM_ATTACK_RELEASE; finite native integration"]
fn accuracy_flags_restore_reused_scratch_after_unknown_and_match_rayon() {
    let mut a = World::load();
    a.set_block(Some(true), Some(37.));
    a.flag("counterfactual-a", RuleEntity::Enemy, 0x321e, false);
    let pa = a.plan();
    let mut b = a.clone();
    b.flag("counterfactual-b", RuleEntity::Enemy, 0x321e, true);
    let pb = b.plan();
    let mut unknown = b.clone();
    unknown.unknown_flag("counterfactual-missing", RuleEntity::Enemy, 0x321e);
    let pu = unknown.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    a.check_accuracy(&first, 0, 37., 63.);
    let unknown_report = pu.evaluate(&mut scratch).unwrap();
    assert!(matches!(
        unknown.value(effects(&unknown_report), 0, true, 0x3220),
        EffectValue::Unresolved { .. }
    ));
    let changed = pb.evaluate(&mut scratch).unwrap();
    b.check_accuracy(&changed, 0, 0., 100.);
    assert_eq!(first, pa.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map(|i| {
                let p = if i % 2 == 0 { &pa } else { &pb };
                p.evaluate(&mut p.new_scratch()).unwrap()
            })
            .collect()
    });
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(report, if i % 2 == 0 { &first } else { &changed });
    }
}
