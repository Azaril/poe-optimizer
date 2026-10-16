//! Published pre-conversion endpoints in the real item/preparation/Action graph.
//! Positive flat controls explicitly add synthetic declared Action suppliers;
//! they do not import additional affixes or complete the real request.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry, owned_release::StagedOwnedRelease,
};

const FINAL: &str = "selected-physical-base-final";
const CONTROL: &str = "synthetic.selected-physical-flat";
const CHANNELS: [u64; 2] = [0x339a, 0x339b];
const OUTPUTS: [u64; 2] = [0x339c, 0x339d];

fn owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::ActionOutput(basic_output()))
}

pub(super) fn install(w: &mut sniper::World, endpoint: &StagedOwnedRelease) {
    physical_base_family::assert_component(endpoint);
    let inner = &mut w.base.source.base.inner;
    for entry in physical_base_family::migration().schema {
        let SchemaExtensionEntry::Definition(descriptor) = entry else {
            panic!("physical base adds Stat definitions only")
        };
        if let Some(existing) = inner
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == descriptor.address())
        {
            assert_eq!(existing, &descriptor);
        } else {
            inner.schema.definitions.push(descriptor.clone());
        }
        inner.owner_mut(SchemaSubject::Definition(descriptor.address()));
    }
    let authored = physical_base_family::consumer();
    assert_eq!(authored.owners.len(), 1);
    assert_eq!(authored.owners[0].owner, owner());
    for row in authored.owners {
        // This finite joined world already explicitly closes its selected Basic
        // owner. The packet's actual Partial owner is tested separately below.
        let target = inner.owner_mut(row.owner);
        for program in row.programs.members {
            assert!(!target.programs.members.iter().any(|p| p.id == program.id));
            target.programs.members.push(program);
        }
    }
    assert_eq!(authored.queries.len(), 2);
    for query in authored.queries {
        assert!(matches!(
            query.groups[0].members.closure,
            SchemaClosure::Complete
        ));
        assert!(query.groups[0].members.members.is_empty());
        assert!(
            !w.base
                .contribution_queries
                .members
                .iter()
                .any(|q| q.id == query.id)
        );
        w.base.contribution_queries.members.push(query);
    }
}

pub(super) fn configure(s: &mut EvaluationStagesInput) {
    s.stages.push(EvaluationStage {
        id: key(FINAL),
        predecessors: vec![key("combined-added-attack-damage"), key("deliver")],
    });
    for row in &mut s.programs.members {
        if row.program == key(physical_base_family::PROGRAM) {
            row.stage = key(FINAL);
        } else if row.program == key(CONTROL) {
            row.stage = key("deliver");
        }
    }
    for stat in CHANNELS {
        s.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Action,
                stat: d(stat),
                contribution: ContributionKind::Add,
            },
            stage: key("deliver"),
        });
    }
    for stat in [0x3212, 0x3213] {
        let channel = StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: d(stat),
        };
        if !s.frozen_channels.iter().any(|row| row.channel == channel) {
            s.frozen_channels.push(FrozenStageChannel {
                channel,
                stage: s.routing_stage.clone(),
            });
        }
    }
}

fn number(w: &World, r: &SupportEffectsReport, index: usize, stat: u64) -> f64 {
    let EffectValue::Known {
        value: ParameterValue::Quantity(value),
    } = w.value(sniper::offering::effects(r), index, true, stat)
    else {
        panic!("expected known Action stat {stat:x}")
    };
    value.value()
}

fn check(w: &World, r: &SupportEffectsReport, expected: [[f64; 2]; 2]) {
    assert!(r.gaps.is_empty());
    let effects = sniper::offering::effects(r);
    assert!(effects.gaps.is_empty());
    let emitted: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(physical_base_family::PROGRAM))
        .collect();
    assert_eq!(
        emitted.len(),
        4,
        "two endpoints for each exact Basic occurrence only"
    );
    for (index, values) in expected.into_iter().enumerate() {
        for (stat, expected) in OUTPUTS.into_iter().zip(values) {
            assert_eq!(
                w.value(effects, index, true, stat),
                &EffectValue::Known {
                    value: quantity(expected, &d(0x1d3a))
                }
            );
        }
    }
    assert_ne!(w.action(0), w.action(1));
    assert!(effects.values.iter().filter(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if OUTPUTS.map(d::<StatDefinition>).contains(stat)))
        .all(|v| matches!(&v.key, PlanValueKey::Stat { entity: ConcreteEntity::Action(a), .. }
            if **a == w.action(0) || **a == w.action(1))));
}

/// Explicit finite control, not an admitted item/configuration producer.
fn flat(w: &mut World, amounts: [f64; 2], active: bool, declared: bool) {
    let p = RuleProgram {
        id: key(CONTROL),
        context: RuleEntityKind::Action,
        reads: vec![],
        nodes: vec![
            RuleNode {
                id: key("active"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(active),
                },
            },
            RuleNode {
                id: key("minimum"),
                expression: RuleExpression::Literal {
                    value: quantity(amounts[0], &d(0x1d3a)),
                },
            },
            RuleNode {
                id: key("maximum"),
                expression: RuleExpression::Literal {
                    value: quantity(amounts[1], &d(0x1d3a)),
                },
            },
        ],
        effects: CHANNELS
            .into_iter()
            .zip(["minimum", "maximum"])
            .map(|(stat, name)| RuleEffect {
                id: key(name),
                when: Some(key("active")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: d(stat),
                    contribution: ContributionKind::Add,
                    value: key(name),
                },
            })
            .collect(),
    };
    let programs = &mut w
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(owner())
        .programs
        .members;
    assert!(!programs.iter().any(|p| p.id == key(CONTROL)));
    programs.push(p);
    if declared {
        for (stat, effect) in CHANNELS.into_iter().zip(["minimum", "maximum"]) {
            let query = w
                .sniper
                .base
                .contribution_queries
                .members
                .iter_mut()
                .find(|q| q.stat == d(stat))
                .unwrap();
            query.groups[0].members.members.push(ContributionMember {
                producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                    owner: owner(),
                    program: key(CONTROL),
                    effect: key(effect),
                    origin: ContributionOrigin::Action {
                        authored: false,
                        supplies: vec![slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3092)],
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
}

fn missing_values(w: &World, read: &str, missing: [bool; 2]) {
    let p = w.checked_plan().unwrap();
    let r = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(r.gaps.is_empty());
    let effects = sniper::offering::effects(&r);
    for occurrence in 0..2 {
        for (index, stat) in OUTPUTS.into_iter().enumerate() {
            let expected = if missing[index] {
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingProducer,
                    read: Some(key(read)),
                }
            } else {
                EffectValue::Known {
                    value: quantity([208., 387.][index], &d(0x1d3a)),
                }
            };
            assert_eq!(w.value(effects, occurrence, true, stat), &expected);
        }
    }
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_source_vectors_use_actual_intrinsic_routes_and_checked_factor() {
    let neutral = World::load();
    let mut positive = neutral.clone();
    flat(&mut positive, [3., 7.], true, true);
    let worlds = [&neutral, &positive];
    let reports = [neutral.evaluate(), positive.evaluate()];
    check(&neutral, &reports[0], [[208., 387.]; 2]);
    check(&positive, &reports[1], [[211.45, 395.05]; 2]);
    let source = physical_base_family::source_vectors();
    let mut observed = 0;
    for row in source["vectors"].as_array().unwrap() {
        if row["observed"] != true {
            assert!(row.get("base_call").is_none());
            continue;
        }
        observed += 1;
        let c = &row["base_call"];
        assert_eq!(row["child_effect"], "MinionMeleeBow");
        assert_eq!(c["damage_type"], "Physical");
        assert_eq!(c["observed_at"], 4137);
        assert_eq!(c["query_state_preserved"], true);
        assert!(c["source"].get("PhysicalBonusMin").is_none());
        assert!(c["source"].get("PhysicalBonusMax").is_none());
        assert_eq!(c["enemy_minimum"]["value"], 0.);
        assert_eq!(c["enemy_maximum"]["value"], 0.);
        assert_eq!(c["base_multiplier"], 1.);
        let index = usize::from(c["minimum"]["value"] == 3.);
        assert_eq!(c["minimum"]["value"], [0., 3.][index]);
        assert_eq!(c["maximum"]["value"], [0., 7.][index]);
        for occurrence in 0..2 {
            for (stat, field) in [(0x3212, "PhysicalMin"), (0x3213, "PhysicalMax")] {
                assert_eq!(
                    number(worlds[index], &reports[index], occurrence, stat),
                    c["source"][field].as_f64().unwrap()
                );
            }
            assert_eq!(
                number(worlds[index], &reports[index], occurrence, 0x336e),
                c["added_multiplier"].as_f64().unwrap()
            );
        }
        check(
            worlds[index],
            &reports[index],
            [[
                c["base_min"].as_f64().unwrap(),
                c["base_max"].as_f64().unwrap(),
            ]; 2],
        );
    }
    assert_eq!(observed, 13);
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_declared_signed_fractional_zero_and_inactive_suppliers() {
    for amounts in [[3.5, 6.], [-5., -6.], [-0., 0.], [-180.75, -336.5]] {
        let mut w = World::load();
        flat(&mut w, amounts, true, true);
        let expected = [
            (208. + amounts[0] * 1.15) * 1.,
            (387. + amounts[1] * 1.15) * 1.,
        ];
        check(&w, &w.evaluate(), [expected; 2]);
    }
    let mut w = World::load();
    flat(&mut w, [100., 200.], false, true);
    let r = w.evaluate();
    check(&w, &r, [[208., 387.]; 2]);
    let controls: Vec<_> = sniper::offering::effects(&r)
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(CONTROL))
        .collect();
    assert_eq!(controls.len(), 4);
    assert!(controls.iter().all(|e| e.value == EffectValue::Inactive));
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_closed_membership_refuses_zero_inactive_and_application_writers() {
    for (amount, active) in [(0., true), (10., false)] {
        let mut w = World::load();
        flat(&mut w, [amount; 2], active, false);
        assert!(
            w.checked_plan().is_err(),
            "unlisted writers cannot hide behind their value or activation"
        );
    }
    let mut w = World::load();
    let app = w
        .sniper
        .base
        .effect_applications
        .members
        .first_mut()
        .unwrap();
    let mut changed = 0;
    for effect in &mut app.program.effects {
        if let RuleEffectKind::Contribute {
            stat, contribution, ..
        } = &mut effect.effect
        {
            *stat = d(0x339a);
            *contribution = ContributionKind::Add;
            changed += 1;
        }
    }
    assert!(changed > 0);
    assert!(
        w.checked_plan().is_err(),
        "Actor application writers cannot silently enter this Action-only domain"
    );
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_missing_inputs_and_partial_queries_remain_unavailable() {
    for stat in [0x3212, 0x3213] {
        let mut w = World::load();
        let routes = &mut w
            .sniper
            .base
            .action_routes
            .iter_mut()
            .find(|r| r.output == basic_output())
            .unwrap()
            .routes
            .members;
        let before = routes.len();
        routes.retain(|r| r.target != d(stat));
        assert_eq!(routes.len() + 1, before);
        missing_values(
            &w,
            if stat == 0x3212 {
                "source-min"
            } else {
                "source-max"
            },
            [stat == 0x3212, stat == 0x3213],
        );
    }
    let mut missing = World::load();
    let programs = &mut missing
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(owner())
        .programs
        .members;
    let before = programs.len();
    programs.retain(|p| p.id != key(combined_added_damage_family::PROGRAM));
    assert_eq!(programs.len() + 1, before);
    missing_values(&missing, "added-multiplier", [true; 2]);
    for stat in CHANNELS {
        let mut w = World::load();
        w.sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .find(|q| q.stat == d(stat))
            .unwrap()
            .groups[0]
            .members
            .closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: subject(d::<StatDefinition>(stat)),
                facet: SchemaFacet::GameRules,
                code: key("fixture-unreviewed-flat-supplier"),
            }],
        };
        let p = w.checked_plan().unwrap();
        assert!(
            p.gaps()
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors)
        );
        assert_eq!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Unavailable {
                cause: EffectValue::Unresolved {
                    reason: PlanGapReason::IncompleteContributors,
                    read: None
                },
                input: None,
            }
        );
    }
    let mut actual = World::load();
    actual
        .sniper
        .base
        .source
        .base
        .inner
        .owner_mut(owner())
        .programs
        .closure = physical_base_family::consumer().owners[0]
        .programs
        .closure
        .clone();
    let p = actual.checked_plan().unwrap();
    assert!(
        !p.gaps().is_empty(),
        "the real Basic owner is still Partial"
    );
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_declared_unknown_flat_does_not_become_zero() {
    let mut w = World::load();
    flat(&mut w, [0.; 2], true, true);
    let unknown = def::<StatDefinition>("fixture.selected-physical-missing");
    let inner = &mut w.sniper.base.source.base.inner;
    inner
        .schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: unknown.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: d(0x1d3a) },
                targets: vec![RuleEntityKind::Action],
            }),
        }));
    inner.owner_mut(subject(unknown.clone()));
    let p = inner
        .owner_mut(owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(CONTROL))
        .unwrap();
    p.reads.push(RuleRead {
        id: key("missing"),
        value_type: ComputedValueType::Quantity { unit: d(0x1d3a) },
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: unknown,
        },
    });
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("minimum"))
        .unwrap()
        .expression = RuleExpression::Read {
        input: key("missing"),
    };
    missing_values(&w, "missing", [true, false]);
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_requires_both_exact_flat_freezes_before_final_consumer() {
    for stat in CHANNELS {
        let w = World::load();
        let error = w.checked_plan_configured(|stages| {
            stages.stages.push(EvaluationStage { id: key("fixture.physical-too-late"), predecessors: vec![key(FINAL)] });
            let frozen = stages.frozen_channels.iter_mut().find(|row| matches!(&row.channel, StageChannel::Contributions { scope: RuleEntityKind::Action, stat: id, contribution: ContributionKind::Add } if *id == d(stat))).unwrap();
            frozen.stage = key("fixture.physical-too-late");
        }).err().expect("late Action flat channel must refuse");
        assert!(
            error.contains("frozen channel read occurs before or outside frozen stage"),
            "{error}"
        );
    }
    let mut w = World::load();
    flat(&mut w, [0.; 2], false, true);
    let error = w
        .checked_plan_configured(|stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|row| row.program == key(CONTROL))
                .unwrap()
                .stage = key(FINAL);
        })
        .err()
        .expect("a potential supplier cannot write after the declared freeze");
    assert!(
        error.contains("potential writer occurs after or outside frozen stage"),
        "{error}"
    );
}

#[test]
#[ignore = "requires current selected-physical-base release; joined native graph"]
fn physical_base_exact_occurrences_fresh_reused_and_rayon_agree() {
    let a = World::load();
    let mut b = a.clone();
    b.sniper.raw(0, 1, 13., 0.);
    flat(&mut b, [3.5, 6.], true, true);
    let pa = a.plan();
    let pb = b.plan();
    let ra = pa.evaluate(&mut pa.new_scratch()).unwrap();
    let rb = pb.evaluate(&mut pb.new_scratch()).unwrap();
    check(&a, &ra, [[208., 387.]; 2]);
    assert_ne!(number(&b, &rb, 0, 0x339c), number(&b, &rb, 1, 0x339c));
    let mut reordered = b.clone();
    reordered
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .skills
        .reverse();
    reordered.sniper.base.source.base.inner.build.gems.reverse();
    assert_eq!(reordered.evaluate(), rb);
    assert_ne!(pa.identity(), pb.identity());
    let mut scratch = pa.new_scratch();
    for (p, expected) in [(&pa, &ra), (&pb, &rb), (&pa, &ra)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *expected);
    }
    let results: Vec<_> = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap()
        .install(|| {
            (0..12)
                .into_par_iter()
                .map_init(
                    || pa.new_scratch(),
                    |scratch, i| {
                        let (p, expected) = if i % 2 == 0 { (&pa, &ra) } else { (&pb, &rb) };
                        (p.evaluate(scratch).unwrap(), expected)
                    },
                )
                .collect()
        });
    for (actual, expected) in results {
        assert_eq!(actual, *expected);
    }
}
