//! Synthetic channel laws exercise the same reduction path as ordinary effects.
use super::fixture as support;
use super::*;
#[path = "owned_empty_support_contribution_fixture.rs"]
mod empty_support;
use poe_optimizer_data::owned_rules::{OwnedRulePackage, RuleStorageLimits};

fn rank(source_rank: u32) -> Option<ContributionOrder> {
    Some(ContributionOrder {
        source_rank,
        program_rank: 0,
        effect_rank: 0,
        slot_ranks: vec![],
    })
}
fn mixed() -> Fixture {
    let mut f = world();
    f.schema
        .definitions
        .retain(|d| !matches!(d, DefinitionDescriptor::Skill(_)));
    f.schema
        .slots
        .retain(|d| !matches!(d, SlotDescriptor::ActionOutput(_)));
    f.owners.retain(|o| {
        !matches!(
            o.owner,
            SchemaSubject::Definition(DefinitionAddress::Skill(_))
                | SchemaSubject::Slot(SlotAddress::ActionOutput(_))
        )
    });
    f.schema
        .definitions
        .push(DefinitionDescriptor::Unit(known_entry(
            def("percent"),
            UnitSchema {
                dimension: UnitDimension::PercentagePoints,
            },
        )));
    let class = f.owner_mut(&class_owner());
    class.programs.members.push(RuleProgram {
        id: key("ordinary"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![literal("amount", 7)],
        effects: vec![effect(
            "ordinary",
            RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def("delivered"),
                contribution: ContributionKind::Add,
                value: key("amount"),
            },
        )],
    });
    class
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key("aggregate-delivered"))
        .unwrap()
        .reads[0]
        .source = RuleReadSource::ContributionQuery {
        entity: RuleEntity::Current,
        query: key("checked"),
        group: key("all"),
    };
    f
}
fn ordinary() -> ContributionMember {
    ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner: class_owner(),
            program: key("ordinary"),
            effect: key("ordinary"),
            origin: ContributionOrigin::Character,
        }),
        order: rank(0),
    }
}
fn install(raw: &mut RulePackageInput) {
    raw.operations_version = key(OWNED_RULE_OPERATIONS_V25);
    let mut groups = std::collections::BTreeMap::new();
    for row in &raw.effect_applications.as_ref().unwrap().members {
        for mapping in &row.stacking {
            let effect = row
                .program
                .effects
                .iter()
                .find(|e| e.id == mapping.effect)
                .unwrap();
            if !matches!(&effect.effect, RuleEffectKind::Contribute { stat, .. } if *stat == def("delivered"))
            {
                continue;
            }
            groups
                .entry((mapping.family.clone(), mapping.modifier.clone()))
                .or_insert_with(Vec::new)
                .push(ApplicationContributionDeclaration {
                    application: row.id.clone(),
                    effect: mapping.effect.clone(),
                });
        }
    }
    let mut members = vec![ordinary()];
    for (i, ((family, modifier), declarations)) in groups.into_iter().enumerate() {
        members.push(ContributionMember {
            producer: ContributionProducer::ApplicationGroup(
                ApplicationGroupContributionProducer {
                    family,
                    modifier,
                    declarations,
                },
            ),
            order: rank(i as u32 + 1),
        });
    }
    raw.contribution_queries = Some(DeclaredSet::complete(vec![ContributionQuery {
        id: key("checked"),
        stat: def("delivered"),
        contribution: ContributionKind::Add,
        groups: vec![ContributionGroup {
            id: key("all"),
            reduction: ContributionReduction::Sum,
            ordering: ContributionOrdering::Ordered,
            empty: Some(integer(0)),
            members: DeclaredSet::complete(members),
        }],
    }]));
}
fn members(raw: &mut RulePackageInput) -> &mut Vec<ContributionMember> {
    &mut raw.contribution_queries.as_mut().unwrap().members[0].groups[0]
        .members
        .members
}
fn app(raw: &mut RulePackageInput) -> &mut ApplicationGroupContributionProducer {
    let ContributionProducer::ApplicationGroup(p) = &mut members(raw)[1].producer else {
        panic!()
    };
    p
}
type CheckedPlan = OwnedSupportEffectPlan<OwnedDefinitionSchemaPackage>;
fn checked_with(
    f: &Fixture,
    rows: DeclaredSet<EffectApplicationRule>,
    limits: PlanLimits,
    edit: impl FnOnce(&mut RulePackageInput),
) -> std::result::Result<CheckedPlan, String> {
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut raw = input(f, &schema, rows);
    edit(&mut raw);
    empty_support::checked_plan(
        f,
        raw,
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("checked-routing"),
            definitions: schema.identity().clone(),
            outputs: f.routes.clone(),
        },
        limits,
    )
    .map_err(|e| e.to_string())
}
fn evaluated(report: SupportEffectsReport) -> OwnedEffectsReport {
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!("{report:?}")
    };
    effects
}
fn checked(f: &Fixture, rows: Vec<EffectApplicationRule>) -> CheckedPlan {
    checked_with(f, DeclaredSet::complete(rows), Default::default(), install).unwrap()
}
fn report(f: &Fixture, rows: Vec<EffectApplicationRule>) -> OwnedEffectsReport {
    let plan = checked(f, rows);
    evaluated(plan.evaluate(&mut plan.new_scratch()).unwrap())
}
fn strength(row: &mut EffectApplicationRule, n: i64) {
    row.program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("strength"))
        .unwrap()
        .expression = RuleExpression::Literal { value: integer(n) };
}
fn partial() -> SchemaClosure {
    SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: summoner_owner(),
            facet: SchemaFacet::GameRules,
            code: key("unproved"),
        }],
    }
}

#[test]
fn checked_groups_mix_with_ordinary_effects_and_preserve_tied_sources() {
    let f = mixed();
    let r = report(&f, vec![application("first"), application("second")]);
    assert_eq!(number(&r, "delivered"), 27);
    let group = r
        .application_groups
        .iter()
        .find(|g| g.key.effect == key("damage"))
        .unwrap();
    assert_eq!(group.candidates.len(), 4);
    assert_eq!(group.co_winners.len(), 2);
    assert!(r.gaps.is_empty());
    let mut independent = application("independent");
    for mapping in &mut independent.stacking {
        mapping.family = key("other-family");
    }
    strength(&mut independent, -3);
    assert_eq!(
        number(
            &report(&f, vec![application("first"), independent]),
            "delivered"
        ),
        24
    );
}

#[test]
fn definition_census_rejects_missing_extra_duplicate_and_wrong_channel_members() {
    let f = mixed();
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut good = input(
        &f,
        &schema,
        DeclaredSet::complete(vec![application("first"), application("second")]),
    );
    install(&mut good);
    OwnedRulePackage::new(good.clone(), &schema, Default::default()).unwrap();
    for case in 0..12 {
        let mut bad = good.clone();
        match case {
            0 => {
                app(&mut bad).declarations.pop();
            }
            1 => app(&mut bad)
                .declarations
                .push(ApplicationContributionDeclaration {
                    application: key("absent"),
                    effect: key("damage"),
                }),
            2 => {
                let duplicate = app(&mut bad).declarations[0].clone();
                app(&mut bad).declarations.push(duplicate);
            }
            3 => app(&mut bad).declarations[0].effect = key("speed"),
            4 => app(&mut bad).family = key("absent"),
            5 => app(&mut bad).modifier = key("speed"),
            6 => {
                let duplicate = members(&mut bad)[1].clone();
                members(&mut bad).push(duplicate);
            }
            7 => {
                members(&mut bad).pop();
            }
            8 => members(&mut bad)[1].order = None,
            9 => members(&mut bad)[1]
                .order
                .as_mut()
                .unwrap()
                .slot_ranks
                .push(ContributionSlotRank {
                    slot: def("weapon"),
                    rank: 0,
                }),
            10 => bad.operations_version = key(OWNED_RULE_OPERATIONS_V24),
            11 => {
                bad.effect_applications.as_mut().unwrap().members[0].targets =
                    vec![EffectApplicationTarget::Enemy];
            }
            _ => unreachable!(),
        }
        assert!(
            OwnedRulePackage::new(bad.clone(), &schema, Default::default()).is_err(),
            "storage case {case}"
        );
        assert!(
            CompiledRulePackage::compile(&bad, &schema, Default::default()).is_err(),
            "compiler case {case}"
        );
    }
    let wire = serde_json::to_value(&good).unwrap();
    let roundtrip: RulePackageInput = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(roundtrip, good);
    let mut old = wire;
    let row = &mut old["contribution_queries"]["members"][0]["groups"][0]["members"]["members"][0];
    let producer = row.as_object_mut().unwrap().remove("producer").unwrap();
    for (k, v) in producer.as_object().unwrap() {
        if k != "kind" {
            row[k] = v.clone();
        }
    }
    assert!(serde_json::from_value::<RulePackageInput>(old).is_err());
}

#[test]
fn census_cannot_hide_unread_inactive_zero_or_unselected_application_declarations() {
    for case in 0..4 {
        let mut f = mixed();
        let mut row = application("unreviewed");
        match case {
            0 => {
                f.owner_mut(&class_owner())
                    .programs
                    .members
                    .retain(|p| p.id != key("aggregate-delivered"));
            }
            1 => {
                row.program
                    .nodes
                    .iter_mut()
                    .find(|n| n.id == key("active"))
                    .unwrap()
                    .expression = RuleExpression::Literal {
                    value: ParameterValue::Boolean(false),
                }
            }
            2 => strength(&mut row, 0),
            3 => {
                f.build.gems.clear();
                f.build.skills.clear();
                f.build.supports.clear();
            }
            _ => unreachable!(),
        }
        let error = checked_with(
            &f,
            DeclaredSet::complete(vec![row]),
            Default::default(),
            |raw| {
                install(raw);
                members(raw).clear();
            },
        )
        .err()
        .unwrap();
        assert!(
            error.contains("potential application contribution"),
            "case {case}: {error}"
        );
    }
}

#[test]
fn inactive_is_absent_unknown_is_not_zero_and_partial_remains_unavailable() {
    let f = mixed();
    let mut inactive = application("a");
    inactive.program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::EffectSource,
        stat: def("missing-strength"),
    };
    inactive
        .program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(false),
    };
    let r = report(&f, vec![inactive.clone()]);
    assert_eq!(number(&r, "delivered"), 7);
    assert!(
        r.application_groups
            .iter()
            .all(|g| g.value == EffectValue::Inactive)
    );
    inactive
        .program
        .nodes
        .iter_mut()
        .find(|n| n.id == key("active"))
        .unwrap()
        .expression = RuleExpression::Literal {
        value: ParameterValue::Boolean(true),
    };
    let r = report(&f, vec![inactive]);
    assert!(matches!(
        value(&r, "delivered"),
        EffectValue::Unresolved { .. }
    ));
    for application_partial in [false, true] {
        let plan = checked_with(
            &f,
            DeclaredSet::complete(vec![application("a")]),
            Default::default(),
            |raw| {
                install(raw);
                if application_partial {
                    raw.effect_applications.as_mut().unwrap().closure = partial();
                } else {
                    raw.contribution_queries.as_mut().unwrap().closure = partial();
                }
            },
        )
        .unwrap();
        let r = plan.evaluate(&mut plan.new_scratch()).unwrap();
        assert!(
            matches!(
                r.outcome,
                SupportEffectsOutcome::Unavailable {
                    cause: EffectValue::Unresolved {
                        reason: PlanGapReason::IncompleteContributors,
                        ..
                    },
                    ..
                }
            ),
            "{r:?}"
        );
    }
}

#[test]
fn recipients_are_separate_order_is_explicit_and_bounds_apply() {
    let f = mixed();
    let mut row = application("a");
    row.targets
        .push(EffectApplicationTarget::OwnedSlot { slot: child_slot() });
    let r = report(&f, vec![row.clone()]);
    assert_eq!(number(&r, "delivered"), 27);
    assert_eq!(r.application_groups.len(), 6);
    let err = checked_with(
        &f,
        DeclaredSet::complete(vec![row.clone()]),
        Default::default(),
        |raw| {
            install(raw);
            members(raw)[1].order = rank(0);
        },
    )
    .err()
    .unwrap();
    assert!(err.contains("semantic positions are tied"), "{err}");
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut raw = input(&f, &schema, DeclaredSet::complete(vec![row]));
    install(&mut raw);
    let usage = OwnedRulePackage::new(raw.clone(), &schema, Default::default())
        .unwrap()
        .resources();
    let limits = RuleStorageLimits {
        max_ordered_work: usage.ordered_work - 1,
        ..Default::default()
    };
    assert!(OwnedRulePackage::new(raw, &schema, limits).is_err());
    assert_ne!(
        RuleOperationsVersion::V24.effect_plan_domain(),
        RuleOperationsVersion::V25.effect_plan_domain()
    );
}

#[test]
fn checked_application_reads_cannot_create_a_cycle_or_hide_an_unlisted_ordinary_writer() {
    let f = mixed();
    let mut row = application("cycle");
    row.program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::Actor,
        stat: def("delivered"),
    };
    let error = checked_with(
        &f,
        DeclaredSet::complete(vec![row]),
        Default::default(),
        install,
    )
    .err()
    .unwrap();
    assert!(error.contains("cycle"), "{error}");
    // The application address cannot absorb an ordinary writer on its channel.
    // This path uses the checked support planner and the shared inventory gate.
    let error = checked_with(
        &f,
        DeclaredSet::complete(vec![application("a")]),
        Default::default(),
        |raw| {
            install(raw);
            members(raw).remove(0);
        },
    )
    .err()
    .unwrap();
    assert!(
        error.contains("actual contribution has no declared membership"),
        "{error}"
    );
}

#[test]
fn query_cannot_read_a_group_before_its_application_stage() {
    use poe_optimizer_core::owned_stages::EvaluationStage;
    let f = mixed();
    let schema = OwnedDefinitionSchemaPackage::new(f.schema.clone(), Default::default()).unwrap();
    let mut raw = input(&f, &schema, DeclaredSet::complete(vec![application("a")]));
    install(&mut raw);
    let error = empty_support::checked_plan_with_stages(
        &f,
        raw,
        ActionRoutingInput {
            schema_version: OWNED_ACTION_ROUTING_VERSION,
            namespace: ns(),
            release: key("checked-routing"),
            definitions: schema.identity().clone(),
            outputs: f.routes.clone(),
        },
        Default::default(),
        |stages| {
            stages.stages.insert(
                1,
                EvaluationStage {
                    id: key("early"),
                    predecessors: vec![key("prepare")],
                },
            );
            stages.stages.last_mut().unwrap().predecessors = vec![key("early")];
            for program in &mut stages.programs.members {
                program.stage = key("early");
            }
            // All ordinary producers precede the read; only applications remain late.
            assert!(
                stages
                    .effect_applications
                    .as_ref()
                    .unwrap()
                    .members
                    .iter()
                    .all(|a| a.stage == key("execute"))
            );
        },
    )
    .err()
    .unwrap();
    assert!(error.to_string().contains("stage"), "{error}");
}

#[test]
fn permutations_and_fresh_reused_a_unknown_b_a_match_on_four_workers() {
    let f = mixed();
    let a = vec![application("first"), application("second")];
    let mut b = a.clone();
    strength(&mut b[0], 41);
    let mut unknown = a.clone();
    unknown[0].program.reads[0].source = RuleReadSource::Stat {
        entity: RuleEntity::EffectSource,
        stat: def("missing-strength"),
    };
    let mut permuted = a.clone();
    permuted.reverse();
    for row in &mut permuted {
        row.stacking.reverse();
        row.program.effects.reverse();
        row.program.nodes.reverse();
    }
    assert_eq!(number(&report(&f, permuted), "delivered"), 27);
    let plans = [
        checked(&f, a.clone()),
        checked(&f, unknown),
        checked(&f, b),
        checked(&f, a),
    ];
    let fresh: Vec<_> = plans
        .iter()
        .map(|p| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert!(matches!(
        value(&evaluated(fresh[1].clone()), "delivered"),
        EffectValue::Unresolved { .. }
    ));
    assert_eq!(number(&evaluated(fresh[2].clone()), "delivered"), 48);
    assert_eq!(fresh[0], fresh[3]);
    let mut scratch = plans[0].new_scratch();
    for (p, expected) in plans.iter().zip(&fresh) {
        assert_eq!(&p.evaluate(&mut scratch).unwrap(), expected);
    }
    rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..16).into_par_iter().for_each_init(
                || plans[0].new_scratch(),
                |scratch, i| {
                    assert_eq!(plans[i % 4].evaluate(scratch).unwrap(), fresh[i % 4]);
                },
            );
        });
}
