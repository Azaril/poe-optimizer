//! Real query-backed source scalars on two physical Offering occurrences. This
//! finite graph does not close the actual Skill/global inventories. Synthetic
//! contributors below test binding/arithmetic; they are not admitted game rules.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;

const RESOLVE: &str = "source-buff-effect";
const COUNTERFACTUAL: &str = "counterfactual-source-buff-";
#[derive(Clone)]
pub(super) struct Census {
    owner_closure: SchemaClosure,
    registry_closure: SchemaClosure,
}
fn owner() -> SchemaSubject {
    subject(d::<SkillDefinition>(0x2a2))
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn entity(index: usize) -> ConcreteEntity {
    ConcreteEntity::Skill(Box::new(SkillTarget::Generated(Box::new(
        GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(id(23 + index as u64)),
                grant_path: vec![],
            },
            slot: slot(SlotOwnerDefId::Gem(d(0x86b)), 0x3221),
        },
    ))))
}
pub(super) fn install(w: &mut sniper::World, endpoint: &StagedOwnedRelease) -> Census {
    buff_source_family::assert_component(endpoint);
    let deps = buff_source_family::dependencies();
    let inner = &mut w.base.source.base.inner;
    assert_eq!(inner.operations, key(OWNED_RULE_OPERATIONS_V24));
    for descriptor in deps.definitions {
        if let Some(existing) = inner
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == descriptor.address())
        {
            assert_eq!(*existing, descriptor);
        } else {
            inner.owner_mut(SchemaSubject::Definition(descriptor.address()));
            inner.schema.definitions.push(descriptor);
        }
    }
    let selected = inner.owner_mut(owner());
    assert!(selected.programs.is_complete());
    for row in buff_source_family::programs() {
        assert_eq!(row.owner, owner());
        // The existing physical-source loader already retains these programs.
        // Authenticate them rather than adding a second copy of each writer.
        assert_eq!(
            selected
                .programs
                .members
                .iter()
                .filter(|p| p.id == row.program.id)
                .collect::<Vec<_>>(),
            [&row.program]
        );
    }
    for query in buff_source_family::queries() {
        assert!(
            !w.base
                .contribution_queries
                .members
                .iter()
                .any(|q| q.id == query.id)
        );
        w.base.contribution_queries.members.push(query);
    }
    Census {
        owner_closure: deps.owner.programs.closure,
        registry_closure: deps.query_registry_closure,
    }
}
pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.push(EvaluationStage {
        id: key(RESOLVE),
        predecessors: vec![key("deliver")],
    });
    for row in &mut stages.programs.members {
        if row.owner == owner() && buff_source_family::PROGRAMS.contains(&row.program.as_str()) {
            row.stage = key(RESOLVE);
        }
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if row.owner == owner()
            && (buff_source_family::PROGRAMS.contains(&row.program.as_str())
                || row.program.as_str().starts_with(COUNTERFACTUAL))
        {
            row.phase = ReadinessPhase::Execution;
            row.role = ReadinessProgramRole::Execution;
            row.outputs.clear();
        }
    }
    for query in buff_source_family::queries() {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Skill,
                stat: query.stat,
                contribution: query.contribution,
            },
            stage: key("deliver"),
        });
    }
    for stat in [0x3228, 0x3229, 0x322a] {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Skill,
                stat: d(stat),
            },
            stage: key(RESOLVE),
        });
    }
}
fn value(report: &SupportEffectsReport, index: usize, stat: u64) -> &EffectValue {
    &sniper::offering::effects(report)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: entity(index),
                    stat: d(stat),
                }
        })
        .unwrap()
        .value
}
fn check_values(report: &SupportEffectsReport, expected: [f64; 3]) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let effects = sniper::offering::effects(report);
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    for ((stat, unit), amount) in [(0x3228, 2), (0x3229, 1), (0x322a, 1)]
        .into_iter()
        .zip(expected)
    {
        for index in 0..2 {
            assert_eq!(
                value(report, index, stat),
                &EffectValue::Known {
                    value: quantity(amount, &d(unit))
                }
            );
            let writes: Vec<_> = effects
                .effects
                .iter()
                .filter(|e| {
                    e.target
                        == BoundEffectTarget::Value {
                            key: PlanValueKey::Stat {
                                entity: entity(index),
                                stat: d(stat),
                            },
                        }
                })
                .collect();
            assert_eq!(
                writes.len(),
                1,
                "one writer for each exact Skill; copies cannot collapse"
            );
            assert_eq!(writes[0].key.invocation.owner, owner());
            assert_eq!(writes[0].key.invocation.entity, entity(index));
        }
        assert_eq!(
            effects
                .values
                .iter()
                .filter(|v| matches!(&v.key, PlanValueKey::Stat {stat:s,..} if *s == d(stat)))
                .count(),
            2
        );
    }
}
fn query_mut(w: &mut World, index: usize) -> &mut ContributionQuery {
    let query = &buff_source_family::queries()[index];
    w.sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.id == query.id)
        .unwrap()
}
fn potential(w: &mut World, index: usize, amount: Option<f64>, enabled: bool, admit: bool) {
    let query = buff_source_family::queries().remove(index);
    let unit = d(if query.contribution == ContributionKind::Increase {
        2
    } else {
        1
    });
    let program = key(&format!("{COUNTERFACTUAL}{index}"));
    let mut reads = vec![];
    let expression = if let Some(amount) = amount {
        RuleExpression::Literal {
            value: quantity(amount, &unit),
        }
    } else {
        let missing: StatDefId = def("counterfactual.missing-source-buff-input");
        inner(w)
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: missing.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Quantity { unit: unit.clone() },
                    targets: vec![RuleEntityKind::Skill],
                }),
            }));
        reads.push(RuleRead {
            id: key("missing"),
            value_type: ComputedValueType::Quantity { unit },
            source: RuleReadSource::Stat {
                entity: RuleEntity::Current,
                stat: missing,
            },
        });
        RuleExpression::Read {
            input: key("missing"),
        }
    };
    inner(w)
        .owner_mut(owner())
        .programs
        .members
        .push(RuleProgram {
            id: program.clone(),
            context: RuleEntityKind::Skill,
            reads,
            nodes: vec![
                RuleNode {
                    id: key("amount"),
                    expression,
                },
                RuleNode {
                    id: key("enabled"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(enabled),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("potential"),
                when: Some(key("enabled")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: query.stat,
                    contribution: query.contribution,
                    value: key("amount"),
                },
            }],
        });
    if admit {
        query_mut(w, index).groups[0]
            .members
            .members
            .push(ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: owner(),
                    program,
                    effect: key("potential"),
                    origin: ContributionOrigin::Skill {
                        authored: false,
                        supplies: vec![slot(SlotOwnerDefId::Gem(d(0x86b)), 0x3221)],
                    },
                }),
                order: Some(ContributionOrder {
                    source_rank: 0,
                    program_rank: 0,
                    effect_rank: 0,
                    slot_ranks: vec![],
                }),
            });
    }
}
fn unavailable(r: &SupportEffectsReport) {
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
}

#[test]
#[ignore = "requires published source queries and joined Sniper release"]
fn source_scaling_resolves_on_two_independent_item_prepared_offerings() {
    buff_source_family::check_source(false);
    let w = World::load();
    check_values(&w.evaluate(), [0., 1., 1.]);
    let mut different = w.clone();
    different.sniper.base.raw(4, 1, 0.);
    let r = different.evaluate();
    sniper::offering::check(&different.sniper.base, &r, [22, 3], [0., 0.]);
    check_values(&r, [0., 1., 1.]);
}

#[test]
#[ignore = "requires published source queries; unsupported origins reject before values"]
fn source_empty_domains_refuse_neutral_inactive_unknown_and_nonempty_writers() {
    for index in 0..4 {
        for amount in [Some(0.), Some(1.), Some(50.), None] {
            for enabled in [false, true] {
                let mut w = World::load();
                potential(&mut w, index, amount, enabled, false);
                let error = w
                    .checked_plan()
                    .err()
                    .expect("unregistered potential must fail");
                assert!(
                    error.contains("actual contribution has no declared membership"),
                    "{error}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires published source queries; exact supplied membership counterfactuals"]
fn source_arithmetic_uses_checked_queries_and_keeps_copies_separate() {
    // Values come from retained source controls, but these synthetic members do
    // not stand in for imported custom modifiers or authorize their origins.
    let proof = source_evidence::read();
    for name in [
        "offering-source-buff-effect",
        "offering-positive-half-tie",
        "offering-negative-half-tie",
        "offering-combined-more",
        "offering-magnitude",
    ] {
        let vector = proof["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["case"] == name)
            .unwrap();
        let scaling = &vector["invocations"][0]["source_scaling"];
        let values = [
            "source_buff_increased",
            "source_buff_more",
            "source_magnitude_increased",
            "source_magnitude_more",
        ]
        .map(|f| scaling[f]["value"].as_f64().unwrap());
        let mut w = World::load();
        for (i, amount) in values.into_iter().enumerate() {
            potential(&mut w, i, Some(amount), true, true);
        }
        check_values(
            &w.evaluate(),
            [values[0], values[1], (1. + values[2] / 100.) * values[3]],
        );
    }
}

#[test]
#[ignore = "requires published source queries; unresolved inventory and value controls"]
fn source_partial_domains_and_missing_values_never_become_neutral_factors() {
    let original = World::load();
    let mut w = original.clone();
    inner(&mut w).owner_mut(owner()).programs.closure = original.source_buffs.owner_closure.clone();
    // This Skill also owns preparation facts. Restoring its actual Partial
    // inventory must fail the existing early-readiness admission even before
    // an execution plan could report an unresolved value.
    let error = w
        .checked_plan()
        .err()
        .expect("Partial preparation owner must reject");
    assert!(
        error.contains("early readiness needs an early phase and complete owner programs"),
        "{error}"
    );
    for index in 0..4 {
        let mut w = original.clone();
        let query_stat = query_mut(&mut w, index).stat.clone();
        query_mut(&mut w, index).groups[0].members.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: subject(query_stat),
                facet: SchemaFacet::GameRules,
                code: key("counterfactual-source-coverage"),
            }],
        };
        let p = w.plan();
        assert!(
            p.gaps()
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors)
        );
        unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
        let mut w = original.clone();
        potential(&mut w, index, None, true, true);
        let r = w.evaluate();
        let stat = if index < 2 {
            0x3228 + index as u64
        } else {
            0x322a
        };
        for copy in 0..2 {
            assert!(matches!(
                value(&r, copy, stat),
                EffectValue::Unresolved { .. }
            ));
        }
    }
}

#[test]
#[ignore = "requires published source queries; stage order and parallel determinism"]
fn source_queries_require_frozen_inputs_and_preserve_a_unknown_b_a_parallel_replays() {
    let a = World::load();
    let error = a
        .checked_plan_configured(|s| {
            s.programs
                .members
                .iter_mut()
                .find(|r| r.program == key(buff_source_family::PROGRAMS[0]))
                .unwrap()
                .stage = key("facts")
        })
        .err()
        .expect("cannot read before the contribution freeze");
    assert!(
        error.contains("stage") || error.contains("frozen"),
        "{error}"
    );
    let mut b = a.clone();
    b.sniper.base.raw(4, 1, 0.);
    let mut u = a.clone();
    u.sniper.base.contribution_queries.closure = a.source_buffs.registry_closure.clone();
    let (pa, pb, pu) = (a.plan(), b.plan(), u.plan());
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    unavailable(&pu.evaluate(&mut scratch).unwrap());
    let rb = pb.evaluate(&mut scratch).unwrap();
    assert_ne!(ra, rb);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    check_values(&ra, [0., 1., 1.]);
    check_values(&rb, [0., 1., 1.]);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, i| match i % 3 {
                    0 => pa.evaluate(s).unwrap(),
                    1 => pu.evaluate(s).unwrap(),
                    _ => pb.evaluate(s).unwrap(),
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, r) in reports.iter().enumerate() {
        match i % 3 {
            0 => assert_eq!(*r, ra),
            1 => unavailable(r),
            _ => assert_eq!(*r, rb),
        }
    }
}
