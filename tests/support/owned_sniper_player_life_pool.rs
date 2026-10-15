//! Final Life arithmetic over the real joined sources. Only this finite test
//! graph closes the new supplier/order domains; published coverage stays Partial.
//! Custom source witnesses and synthetic transfer/override controls are separate
//! from the unchanged build's actual level, item, Strength and reward producers.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};

const CONTROL: &str = "fixture.player-life-controls";
const FINAL: &str = "fixture.player-life-final";
const CONTROL_PROGRAM: &str = "test-player-life-inputs";
const POOL_PROGRAM: &str = "resolve-player-life";

fn actor_owner() -> SchemaSubject {
    subject(d::<ActorDefinition>(0x332a))
}
fn world() -> World {
    type Component = (
        Vec<DefinitionDescriptor>,
        Vec<DefinitionRules>,
        Vec<StatReceiver>,
        Vec<ContributionQuery>,
    );
    static CURRENT: OnceLock<Component> = OnceLock::new();
    let (definitions, owners, receivers, queries) = CURRENT.get_or_init(|| {
        let endpoint = release::load(&path());
        let previous = life_base_domain_family::previous_rules(&endpoint);
        player_life_pool_family::assert_component_with_reviewed_rules(&endpoint, &previous);
        let recipe = &endpoint.input().recipe;
        let consumer = player_life_pool_family::consumer();
        let definitions = player_life_pool_family::extension()
            .schema
            .into_iter()
            .map(|entry| {
                let poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry::Definition(
                    expected,
                ) = entry
                else {
                    panic!("Life adds only Stat definitions")
                };
                recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == expected.address())
                    .unwrap()
                    .clone()
            })
            .collect();
        (
            definitions,
            consumer
                .owners
                .iter()
                .map(|expected| {
                    recipe
                        .rules
                        .owners
                        .iter()
                        .find(|o| o.owner == expected.owner)
                        .unwrap()
                        .clone()
                })
                .collect(),
            consumer
                .receivers
                .iter()
                .map(|expected| {
                    recipe
                        .rules
                        .receivers
                        .members
                        .iter()
                        .find(|r| r.id == expected.id)
                        .unwrap()
                        .clone()
                })
                .collect(),
            consumer
                .queries
                .iter()
                .map(|expected| {
                    recipe
                        .rules
                        .contribution_queries
                        .as_ref()
                        .unwrap()
                        .members
                        .iter()
                        .find(|q| q.id == expected.id)
                        .unwrap()
                        .clone()
                })
                .collect(),
        )
    });
    let mut w = life_queries_native::world();
    let f = &mut w.sniper.base.source.base.inner;
    for descriptor in definitions {
        assert_eq!(
            f.schema
                .definitions
                .iter()
                .find(|d| d.address() == descriptor.address()),
            Some(descriptor),
            "the current joined schema already contains the published definitions"
        );
    }
    for mut owner in owners.iter().cloned() {
        // Explicitly finite arithmetic/supplier scope, never a production edit.
        owner.programs.closure = SchemaClosure::Complete;
        let existing = f.owner_mut(owner.owner.clone());
        assert!(existing.programs.members.is_empty());
        *existing = owner;
    }
    for receiver in receivers.iter().cloned() {
        assert_eq!(receiver.targets, vec![StatReceiverTarget::Player]);
        assert!(
            !w.sniper
                .receivers
                .members
                .iter()
                .any(|r| r.id == receiver.id)
        );
        w.sniper.receivers.members.push(receiver);
    }
    for mut query in queries.iter().cloned() {
        assert!(
            query
                .groups
                .iter()
                .all(|g| !g.members.is_complete() && g.members.members.is_empty())
        );
        for group in &mut query.groups {
            group.members.closure = SchemaClosure::Complete;
        }
        assert!(
            !w.sniper
                .base
                .contribution_queries
                .members
                .iter()
                .any(|q| q.id == query.id)
        );
        w.sniper.base.contribution_queries.members.push(query);
    }
    w
}

fn configure(stages: &mut EvaluationStagesInput) {
    life_queries_native::configure(stages);
    stages.stages.extend([
        EvaluationStage {
            id: key(CONTROL),
            predecessors: vec![key("life-contributions-ready")],
        },
        EvaluationStage {
            id: key(FINAL),
            predecessors: vec![key(CONTROL), key("life-query-probes")],
        },
    ]);
    let consumers = player_life_pool_family::consumer();
    for row in &mut stages.programs.members {
        if row.program == key(CONTROL_PROGRAM) {
            // Controlled BASE/INC/MORE writers must precede the original freeze.
            row.stage = key("player-inherent-life-contribution");
        } else if consumers
            .owners
            .iter()
            .any(|o| o.owner == row.owner && o.programs.members.iter().any(|p| p.id == row.program))
        {
            assert_eq!(row.program, key(POOL_PROGRAM));
            row.stage = key(FINAL);
        }
    }
    for row in &mut stages.readiness.as_mut().unwrap().programs.members {
        if row.program == key(CONTROL_PROGRAM)
            || consumers.owners.iter().any(|o| {
                o.owner == row.owner && o.programs.members.iter().any(|p| p.id == row.program)
            })
        {
            row.phase = ReadinessPhase::Execution;
            row.role = ReadinessProgramRole::Execution;
            row.outputs.clear();
        }
    }
    for query in consumers.queries {
        stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: query.stat,
                contribution: query.contribution,
            },
            stage: key(CONTROL),
        });
    }
}
fn plan(w: &World) -> shared::Plan {
    w.checked_plan_configured(configure).unwrap()
}
fn report(w: &World) -> SupportEffectsReport {
    let p = plan(w);
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn value(r: &SupportEffectsReport, stat: StatDefId) -> &EffectValue {
    &sniper::offering::effects(r)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: stat.clone(),
                }
        })
        .unwrap()
        .value
}
fn life(r: &SupportEffectsReport, expected: f64) {
    assert_eq!(
        value(r, d(0x311a)),
        &EffectValue::Known {
            value: quantity(expected, &d(0x3119))
        }
    );
    assert!(
        !sniper::offering::effects(r)
            .values
            .iter()
            .any(|v| matches!(&v.key,
        PlanValueKey::Stat { entity, stat } if *stat == d(0x311a)
        && *entity != ConcreteEntity::Actor(ActorKey::Player))),
        "Player-only receiver"
    );
}
fn probe(r: &SupportEffectsReport, name: &str, expected: f64, unit: u64) {
    assert_eq!(
        value(r, def(name)),
        &EffectValue::Known {
            value: quantity(expected, &d(unit))
        }
    );
}
fn robe(w: &mut World, amount: f64) {
    let f = &mut w.sniper.base.source.base.inner;
    let item = f
        .build
        .items
        .iter_mut()
        .find(|i| i.template == d(0x238c))
        .unwrap();
    let modifier = item
        .modifiers
        .iter_mut()
        .find(|m| m.definition == d(0x3100))
        .unwrap();
    modifier
        .rolls
        .iter_mut()
        .find(|r| r.slot.slot == d(0x3101))
        .unwrap()
        .value = quantity(amount, &d(0x295a));
}

#[derive(Clone)]
struct Control {
    stat: u64,
    kind: ContributionKind,
    value: ParameterValue,
    missing: bool,
}
fn numeric(stat: u64, amount: f64, kind: ContributionKind) -> Control {
    Control {
        stat,
        kind,
        value: quantity(
            amount,
            &d(
                if (0x3383..=0x3385).contains(&stat) || kind == ContributionKind::Increase {
                    2
                } else if kind == ContributionKind::Multiply {
                    1
                } else {
                    0x3119
                },
            ),
        ),
        missing: false,
    }
}
fn ci(enabled: bool) -> Control {
    Control {
        stat: 0x3388,
        kind: ContributionKind::Flag,
        value: ParameterValue::Boolean(enabled),
        missing: false,
    }
}
/// These writers are finite test controls, never inferred source admissions.
fn controls(w: &mut World, controls: &[Control]) {
    let mut program = RuleProgram {
        id: key(CONTROL_PROGRAM),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![],
        effects: vec![],
    };
    for (index, control) in controls.iter().enumerate() {
        let node = key(&format!("value-{index}"));
        let effect = key(&format!("source-{index}"));
        let expression = if control.missing {
            let stat: StatDefId = def(&format!("fixture.life-missing-{index}"));
            let ty = match &control.value {
                ParameterValue::Boolean(_) => ComputedValueType::Boolean,
                ParameterValue::Quantity(q) => ComputedValueType::Quantity {
                    unit: q.unit().clone(),
                },
                _ => panic!("finite controls are typed quantities or flags"),
            };
            let f = &mut w.sniper.base.source.base.inner;
            f.schema
                .definitions
                .push(DefinitionDescriptor::Stat(DefinitionEntry {
                    id: stat.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ty.clone(),
                        targets: vec![RuleEntityKind::Actor],
                    }),
                }));
            f.owner_mut(subject(stat.clone()));
            program.reads.push(RuleRead {
                id: node.clone(),
                value_type: ty,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat,
                },
            });
            RuleExpression::Read {
                input: node.clone(),
            }
        } else {
            RuleExpression::Literal {
                value: control.value.clone(),
            }
        };
        program.nodes.push(RuleNode {
            id: node.clone(),
            expression,
        });
        program.effects.push(RuleEffect {
            id: effect.clone(),
            when: None,
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: d(control.stat),
                contribution: control.kind,
                value: node,
            },
        });
        let query = w
            .sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .find(|q| q.stat == d(control.stat) && q.contribution == control.kind)
            .unwrap();
        let group = if control.stat == 0x311a && control.kind == ContributionKind::Add {
            "intrinsic"
        } else if control.stat == 0x311a && control.kind == ContributionKind::Increase {
            "received"
        } else {
            "sources"
        };
        let group = query
            .groups
            .iter_mut()
            .find(|g| g.id == key(group))
            .unwrap();
        group.members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: actor_owner(),
                program: key(CONTROL_PROGRAM),
                effect,
                origin: ContributionOrigin::ExistingActor {
                    application: key("shared-player-initialization"),
                },
            }),
            order: if matches!(
                control.kind,
                ContributionKind::Flag | ContributionKind::Override
            ) {
                None
            } else {
                Some(ContributionOrder {
                    // The intrinsic group already contains this exact shared
                    // Actor at rank zero. Separate this synthetic program,
                    // rather than inventing a second rank for that owner.
                    source_rank: if control.stat == 0x311a && control.kind == ContributionKind::Add
                    {
                        0
                    } else {
                        1
                    },
                    program_rank: 1,
                    effect_rank: index as u32,
                    slot_ranks: vec![],
                })
            },
        });
    }
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(actor_owner())
        .programs
        .members
        .push(program);
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn real_player_sources_reach_final_life_and_change_through_native_inputs() {
    let w = world();
    let a = plan(&w);
    let expected = a.evaluate(&mut a.new_scratch()).unwrap();
    assert!(expected.gaps.is_empty());
    for (name, number, unit) in [
        ("fixture.life-add", 1257., 0x3119),
        ("fixture.life-inc", 5., 2),
        ("fixture.life-more", 1., 1),
    ] {
        probe(&expected, name, number, unit);
    }
    life(&expected, 1320.);
    let mut changed = w.clone();
    changed.sniper.base.source.base.inner.build.character.level = 91;
    robe(&mut changed, 18.);
    let b = plan(&changed);
    let different = b.evaluate(&mut b.new_scratch()).unwrap();
    life(&different, 1308.); // Actual level -12 and raw Life +1, then 5% increase.
    assert_ne!(a.identity(), b.identity());
    let mut scratch = a.new_scratch();
    for (p, expected) in [(&a, &expected), (&b, &different), (&a, &expected)] {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), *expected);
    }
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let results: Vec<_> = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || a.new_scratch(),
                |scratch, n| {
                    let (p, expected) = if n % 2 == 0 {
                        (&a, &expected)
                    } else {
                        (&b, &different)
                    };
                    (p.evaluate(scratch).unwrap(), expected)
                },
            )
            .collect()
    });
    for (actual, expected) in results {
        assert_eq!(&actual, expected);
    }
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn saved_player_life_metric_uses_native_execution_and_preserves_unavailability() {
    use poe_optimizer_core::{owned_draft::MetricRequestDraft, owned_metrics::*};
    use poe_optimizer_data::owned_metrics::OwnedMetricMapping;
    use std::sync::Arc;

    let endpoint = release::load(&path());
    let previous = life_base_domain_family::previous_rules(&endpoint);
    player_life_pool_family::assert_component_with_reviewed_rules(&endpoint, &previous);
    let (draft, _) = imported_selection(&path());
    let requests = draft["draft"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
        .as_array()
        .unwrap();
    let saved = requests
        .iter()
        .map(decode::<MetricRequestDraft>)
        .find(|r| r.id.as_str() == "reference-00")
        .unwrap()
        .to_resolved()
        .unwrap();
    assert_eq!(saved.metric, d(0x312d));
    assert_eq!(saved.target, MetricTarget::Actor(ActorKey::Player));
    let descriptor = endpoint
        .input()
        .recipe
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == saved.metric.address())
        .unwrap()
        .clone();

    for case in 0..3 {
        let mut w = world();
        if case == 1 {
            let actual = player_life_pool_family::consumer().queries.remove(0);
            let id = actual.id.clone();
            *w.sniper
                .base
                .contribution_queries
                .members
                .iter_mut()
                .find(|q| q.id == id)
                .unwrap() = actual;
        } else if case == 2 {
            controls(
                &mut w,
                &[numeric(0x3387, 2_f64.powi(52), ContributionKind::Add)],
            );
        }
        let mut inputs = plan(&w).snapshot();
        // The finite graph keeps only its requested probe metrics. Install the
        // saved Player metric's exact published descriptor for this projection.
        assert!(
            !inputs
                .schema
                .definitions
                .iter()
                .any(|d| d.address() == descriptor.address())
        );
        inputs.schema.definitions.push(descriptor.clone());
        inputs.queries.requests.push(saved.clone());
        inputs.rebind_test_edit().unwrap();
        let effects = inputs.compile().unwrap();
        let mapping = OwnedMetricMapping::new(
            MetricMappingInput {
                schema_version: OWNED_METRIC_MAPPING_VERSION,
                namespace: inputs.schema.namespace.clone(),
                release: key("fixture-final-player-life-metric"),
                definitions: effects.definitions().identity().clone(),
                bindings: vec![MetricStatBinding {
                    metric: saved.metric.clone(),
                    role: MetricBindingRole::PlayerActor,
                    stat: d(0x311a),
                }],
            },
            effects.definitions(),
            Default::default(),
        )
        .unwrap();
        let metrics =
            OwnedSupportMetricPlan::compile(Arc::new(effects), Arc::new(mapping)).unwrap();
        let report = metrics.evaluate(&mut metrics.new_scratch()).unwrap();
        let result = report
            .evaluation
            .results
            .iter()
            .find(|r| r.request.id == saved.id)
            .unwrap();
        assert_eq!(result.request, saved);
        assert_eq!(result.stat, Some(d(0x311a)));
        if case == 0 {
            assert_eq!(
                result.value,
                EffectValue::Known {
                    value: quantity(1320., &d(0x3119))
                }
            );
        } else {
            assert!(!matches!(result.value, EffectValue::Known { .. }));
            if case == 2 {
                assert!(format!("{:?}", result.value).contains("NumericRange"));
            }
        }
    }
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn original_life_vectors_use_the_actual_consumer_and_explicit_source_controls() {
    let source = player_life_pool_family::source_vectors();
    let rows = source["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 11);
    for row in rows {
        let mut w = world();
        let name = row["name"].as_str().unwrap();
        match name {
            "original-05" => {}
            "character-level-91" => w.sniper.base.source.base.inner.build.character.level = 91,
            "selected-robe-life-18" => robe(&mut w, 18.),
            "selected-robe-life-10" => robe(&mut w, 10.),
            "selected-robe-life-9" => robe(&mut w, 9.),
            "selected-robe-life-11" => robe(&mut w, 11.),
            "custom-increased-life" => {
                controls(&mut w, &[numeric(0x311a, 10., ContributionKind::Increase)])
            }
            "custom-more-life" => {
                controls(&mut w, &[numeric(0x311a, 1.1, ContributionKind::Multiply)])
            }
            "custom-chaos-inoculation" => controls(&mut w, &[ci(true)]),
            "custom-zero-base" => {
                controls(&mut w, &[numeric(0x311a, -1257., ContributionKind::Add)])
            }
            "custom-negative-base" => {
                controls(&mut w, &[numeric(0x311a, -1258., ContributionKind::Add)])
            }
            _ => panic!("unreviewed retained control: {name}"),
        }
        assert_eq!(row["synthetic_source_control"], name.starts_with("custom-"));
        let r = report(&w);
        for (name, field, unit) in [
            ("fixture.life-add", "base", 0x3119),
            ("fixture.life-inc", "increase", 2),
            ("fixture.life-more", "more", 1),
        ] {
            probe(&r, name, row["inputs"][field].as_f64().unwrap(), unit);
        }
        life(&r, row["final_life"].as_f64().unwrap());
    }
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn synthetic_transfers_preserve_fractional_values_order_and_conversion_cap() {
    for (conversion, expected) in [(25., 1018.), (-25., 1678.), (150., 28.)] {
        let mut w = world();
        controls(
            &mut w,
            &[
                numeric(0x3383, conversion, ContributionKind::Add),
                numeric(0x3386, 25.25, ContributionKind::Add),
                numeric(0x3387, 1.5, ContributionKind::Add),
            ],
        );
        life(&report(&w), expected);
    }
    let mut combined = world();
    controls(
        &mut combined,
        &[
            numeric(0x3383, 40., ContributionKind::Add),
            numeric(0x3384, 40., ContributionKind::Add),
            numeric(0x3385, 40., ContributionKind::Add),
            numeric(0x3386, 25.25, ContributionKind::Add),
            numeric(0x3387, 1.5, ContributionKind::Add),
        ],
    );
    life(&report(&combined), 28.); // Cap the combined conversion, then retain Extra/Total.
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn synthetic_overrides_include_zero_and_ci_takes_precedence() {
    for (override_value, chaos, expected) in [
        (0., false, 0.),
        (73., false, 73.),
        (-5., false, -5.),
        (0., true, 1.),
        (73., true, 1.),
    ] {
        let mut w = world();
        controls(
            &mut w,
            &[
                numeric(0x311a, override_value, ContributionKind::Override),
                ci(chaos),
            ],
        );
        life(&report(&w), expected);
    }
    let mut agreeing = world();
    controls(
        &mut agreeing,
        &[
            numeric(0x311a, 0., ContributionKind::Override),
            numeric(0x311a, 0., ContributionKind::Override),
        ],
    );
    life(&report(&agreeing), 0.);
    let mut conflicting = world();
    controls(
        &mut conflicting,
        &[
            numeric(0x311a, 0., ContributionKind::Override),
            numeric(0x311a, 2., ContributionKind::Override),
        ],
    );
    assert!(!matches!(
        value(&report(&conflicting), d(0x311a)),
        EffectValue::Known { .. }
    ));
    let mut replaced = world();
    let mut missing_extra = numeric(0x3386, 1., ContributionKind::Add);
    missing_extra.missing = true;
    controls(
        &mut replaced,
        &[
            ci(true),
            numeric(0x311a, 0., ContributionKind::Override),
            numeric(0x311a, 2., ContributionKind::Override),
            missing_extra,
        ],
    );
    life(&report(&replaced), 1.);
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn missing_active_inputs_and_real_partial_authorities_cannot_become_defaults() {
    for mut control in [
        ci(true),
        numeric(0x311a, 0., ContributionKind::Override),
        numeric(0x3386, 1., ContributionKind::Add),
    ] {
        control.missing = true;
        let mut w = world();
        controls(&mut w, &[control]);
        assert!(!matches!(
            value(&report(&w), d(0x311a)),
            EffectValue::Known { .. }
        ));
    }
    for restore_query in [false, true] {
        let mut w = world();
        controls(&mut w, &[ci(true)]);
        if restore_query {
            let actual = player_life_pool_family::consumer().queries.remove(0);
            let id = actual.id.clone();
            *w.sniper
                .base
                .contribution_queries
                .members
                .iter_mut()
                .find(|q| q.id == id)
                .unwrap() = actual;
        } else {
            let actual = player_life_pool_family::consumer()
                .owners
                .into_iter()
                .find(|o| o.owner == subject(d::<StatDefinition>(0x311a)))
                .unwrap();
            w.sniper
                .base
                .source
                .base
                .inner
                .owner_mut(actual.owner)
                .programs
                .closure = actual.programs.closure;
        }
        let p = plan(&w);
        assert!(matches!(
            p.evaluate(&mut p.new_scratch()).unwrap().outcome,
            SupportEffectsOutcome::Unavailable { .. }
        ));
    }
    let mut w = world();
    controls(
        &mut w,
        &[ci(true), numeric(0x311a, 0., ContributionKind::Override)],
    );
    let owner = life_equipment_domain_family::dependencies().owner;
    w.sniper
        .base
        .source
        .base
        .inner
        .owner_mut(owner.owner)
        .programs
        .closure = owner.programs.closure;
    assert!(
        w.checked_plan_configured(configure)
            .is_err_and(|e| e.contains("complete owner programs")),
        "CI and an override cannot waive real item-source preparation coverage"
    );
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn rounding_domain_is_explicit_and_unused_ordinary_branches_are_lazy() {
    let bound = 2_f64.powi(52);
    for operand in [bound.next_down(), bound, bound.next_up(), -f64::MAX] {
        let mut w = world();
        controls(
            &mut w,
            &[
                numeric(0x311a, -1256., ContributionKind::Add), // finite BASE = 1
                numeric(0x311a, -5., ContributionKind::Increase), // finite INC = 0
                numeric(0x3387, operand - 1., ContributionKind::Add),
            ],
        );
        let r = report(&w);
        if operand > bound {
            assert!(format!("{:?}", value(&r, d(0x311a))).contains("NumericRange"));
        } else {
            life(&r, if operand < 0. { 1. } else { bound });
        }
    }
    let mut outside = world();
    controls(
        &mut outside,
        &[numeric(0x3387, 2_f64.powi(52), ContributionKind::Add)],
    );
    let r = report(&outside);
    assert!(format!("{:?}", value(&r, d(0x311a))).contains("NumericRange"));
    for final_control in [numeric(0x311a, 0., ContributionKind::Override), ci(true)] {
        let mut w = world();
        controls(
            &mut w,
            &[
                numeric(0x3387, 2_f64.powi(52), ContributionKind::Add),
                final_control.clone(),
            ],
        );
        life(
            &report(&w),
            if final_control.stat == 0x3388 { 1. } else { 0. },
        );
    }
    let mut w = world();
    let mut unknown = numeric(0x3386, 1., ContributionKind::Add);
    unknown.missing = true;
    controls(
        &mut w,
        &[unknown, numeric(0x311a, 0., ContributionKind::Override)],
    );
    life(&report(&w), 0.);
}

#[test]
#[ignore = "requires current joined Sniper release"]
fn actual_final_consumer_requires_its_frozen_override_and_adjustment_stages() {
    life(&report(&world()), 1320.);
    for stat in [0x311a, 0x3386, 0x3388] {
        let w = world();
        let result = w.checked_plan_configured(|stages| {
            configure(stages);
            if stat == 0x311a {
                stages.frozen_channels.retain(|row| {
                    !matches!(&row.channel,
                    StageChannel::Contributions { stat: id, contribution, .. }
                    if *id == d(stat) && *contribution == ContributionKind::Override)
                });
            } else {
                stages.stages.push(EvaluationStage {
                    id: key("fixture.life-too-late"),
                    predecessors: vec![key(FINAL)],
                });
                let frozen = stages
                    .frozen_channels
                    .iter_mut()
                    .find(|row| {
                        matches!(&row.channel,
                    StageChannel::Contributions { stat: id, .. } if *id == d(stat))
                    })
                    .unwrap();
                frozen.stage = key("fixture.life-too-late");
            }
        });
        let error = result
            .err()
            .expect("invalid frozen contribution stage must refuse");
        assert!(
            error.contains(if stat == 0x311a {
                "numeric selection requires a declared frozen channel"
            } else {
                "frozen channel read occurs before or outside frozen stage"
            }),
            "{error}"
        );
    }
}
