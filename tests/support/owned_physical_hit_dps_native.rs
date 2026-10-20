//! Compose the existing public replay input and the actual producer programs.
//! All fixture-only completions stay here; production owns no such certificate.
use super::source::family as speed;
use super::{armour_native, critical_native, dps, replay::ReplayInput, source};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;

const FINAL: &str = "ordinary-physical-hit-dps";
const RATE: &str = "selected-basic-ordinary-rate";
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        format!("def.{n:016x}"),
    )
    .unwrap()
}
fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn owner() -> SchemaSubject {
    dps::consumer().owners[0].owner.clone()
}
fn declare(i: &mut ReplayInput, d: DefinitionDescriptor) {
    assert!(
        !i.schema
            .definitions
            .iter()
            .any(|v| v.address() == d.address())
    );
    i.rules.owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(d.address()),
        programs: DeclaredSet::complete(vec![]),
    });
    i.schema.definitions.push(d);
}
fn install(i: &mut ReplayInput, p: RuleProgram, stage: &str) {
    let subject = owner();
    assert!(
        !i.rules
            .owners
            .iter()
            .flat_map(|o| &o.programs.members)
            .any(|v| v.id == p.id)
    );
    i.stages.programs.members.push(StagedRuleProgram {
        owner: subject.clone(),
        program: p.id.clone(),
        stage: key(stage),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner: subject.clone(),
            program: p.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == subject)
        .unwrap()
        .programs
        .members
        .push(p);
}
fn freeze(i: &mut ReplayInput, channel: StageChannel, stage: &str) {
    if let Some(old) = i
        .stages
        .frozen_channels
        .iter()
        .find(|f| f.channel == channel)
    {
        assert_eq!(old.stage, key(stage));
    } else {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel,
            stage: key(stage),
        });
    }
}
fn fixture() -> ReplayInput {
    source::check();
    let mut i = armour_native::fixture();
    let prior_rules = i.rules.owners.clone();
    // Exactly the existing Speed extension. No copied calculation or second
    // replay model; its existing application adds one Speed modifier.
    let dependencies: Value = speed::read("dependencies.json");
    let prior: DeclaredSet<EffectApplicationRule> =
        serde_json::from_value(dependencies["prior_application_registry"].clone()).unwrap();
    assert_eq!(
        i.rules.effect_applications.as_ref().unwrap().members,
        prior.members
    );
    assert!(i.rules.effect_applications.as_ref().unwrap().is_complete());
    i.rules.effect_applications = Some(DeclaredSet::complete(speed::applications().members));
    for d in speed::definitions().into_iter().chain(dps::definitions()) {
        declare(&mut i, d);
    }
    i.rules.tables.extend(speed::tables());
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(speed::queries());
    i.stages.stages.push(EvaluationStage {
        id: key(RATE),
        predecessors: vec![key("offering-application"), key("deliver")],
    });
    install(
        &mut i,
        speed::consumer().owners[0].programs.members[0].clone(),
        RATE,
    );
    freeze(
        &mut i,
        StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: def(speed::SPEED),
            contribution: ContributionKind::Add,
        },
        "offering-application",
    );
    freeze(
        &mut i,
        StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: def(0x3214),
        },
        "deliver",
    );
    let b: Value = dps::read("bindings.json");
    i.stages.stages.push(EvaluationStage {
        id: key(FINAL),
        predecessors: b["predecessors"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| key(v.as_str().unwrap()))
            .collect(),
    });
    install(
        &mut i,
        dps::consumer().owners[0].programs.members[0].clone(),
        FINAL,
    );
    for (n, stage) in [
        (0x33b2, "armour-adjusted-physical-averages"),
        (0x33b3, "armour-adjusted-physical-averages"),
        (0x33aa, "ordinary-outgoing-critical"),
        (0x3220, "ordinary-minion-attack-hit-chance"),
        (0x33a3, RATE),
    ] {
        // The hit program is staged separately in the maintained replay.
        let actual = if n == 0x3220 {
            i.stages
                .programs
                .members
                .iter()
                .find(|p| p.program == key(stage))
                .unwrap()
                .stage
                .as_str()
                .to_owned()
        } else {
            stage.to_owned()
        };
        freeze(
            &mut i,
            StageChannel::Stat {
                scope: RuleEntityKind::Action,
                stat: def(n),
            },
            &actual,
        );
    }
    let d: Value = dps::read("dependencies.json");
    for row in d["potential_writers"].as_array().unwrap() {
        let expected: RuleProgram = serde_json::from_value(row["program"].clone()).unwrap();
        let owner: SchemaSubject = serde_json::from_value(row["owner"].clone()).unwrap();
        let actual: Vec<_> = i
            .rules
            .owners
            .iter()
            .filter(|o| o.owner == owner)
            .flat_map(|o| &o.programs.members)
            .filter(|p| p.id == expected.id)
            .collect();
        assert_eq!(actual, vec![&expected], "actual published operand producer");
    }
    for old in prior_rules {
        let actual = i
            .rules
            .owners
            .iter()
            .find(|o| o.owner == old.owner)
            .unwrap();
        assert!(
            old.programs
                .members
                .iter()
                .all(|p| actual.programs.members.contains(p))
        );
        assert_eq!(old.programs.closure, actual.programs.closure);
    }
    i.rebind_test_edit().unwrap();
    i
}
fn values(i: &ReplayInput, r: &SupportEffectsReport) -> Vec<[f64; 3]> {
    critical_native::actions(i)
        .into_iter()
        .map(|a| {
            dps::OUTPUTS.map(|n| {
                match critical_native::value(r, ConcreteEntity::Action(Box::new(a.clone())), n) {
                    EffectValue::Known {
                        value: ParameterValue::Quantity(q),
                    } => {
                        assert_eq!(q.unit(), &def(if n == 0x33ba { dps::UNIT } else { 0x1d3a }));
                        q.value()
                    }
                    other => panic!("physical component {n:x}: {other:?}"),
                }
            })
        })
        .collect()
}
fn program_mut<'a>(i: &'a mut ReplayInput, name: &str) -> &'a mut RuleProgram {
    let matches: Vec<_> = i
        .rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .filter(|p| p.id == key(name))
        .collect();
    assert_eq!(matches.len(), 1);
    matches.into_iter().next().unwrap()
}
fn set_derived(i: &mut ReplayInput, stat: u64, value: f64, unit: u64) {
    let matches: Vec<_> = i
        .rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .filter(|p| {
            p.effects.iter().any(|e| {
                matches!(&e.effect,
            RuleEffectKind::Derive{stat:s,..} if *s==def(stat))
            })
        })
        .collect();
    assert_eq!(matches.len(), 1);
    let p = matches.into_iter().next().unwrap();
    let effect = p
        .effects
        .iter()
        .find(|e| {
            matches!(&e.effect,
        RuleEffectKind::Derive{stat:s,..} if *s==def(stat))
        })
        .unwrap();
    let RuleEffectKind::Derive { value: node, .. } = &effect.effect else {
        unreachable!()
    };
    p.nodes
        .iter_mut()
        .find(|n| &n.id == node)
        .unwrap()
        .expression = RuleExpression::Literal {
        value: quantity(value, unit),
    };
}

#[test]
fn physical_dps_actual_producers_form_source_comparison_without_metric_rebinding() {
    let i = fixture();
    let r = critical_native::evaluate(&i);
    for got in values(&i, &r) {
        for (actual, expected) in
            got.into_iter()
                .zip([448.55974910751837, 448.55974910751837, 388.7517825598493])
        {
            assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
        }
    }
    assert!(
        i.routing
            .outputs
            .iter()
            .flat_map(|o| &o.routes.members)
            .all(|r| !dps::OUTPUTS.iter().any(|n| r.target == def(*n)))
    );
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("component")
    };
    let actual: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(dps::PROGRAM))
        .collect();
    assert_eq!(
        actual.len(),
        6,
        "three exact effects on two independent Actions"
    );
}

#[test]
fn physical_dps_keeps_multiply_then_divide_order_and_post_block_not_accuracy() {
    // Chosen f64 boundaries expose reassociation; values are synthetic native
    // operands, never a substitute for the source capture's input authority.
    for [normal, critical, crit, hit, rate] in [
        [0., 0., 0., 0., 0.],
        [
            2.222222222222222,
            7.777777777777777,
            33.33333333333333,
            77.77777777777777,
            1.234567890123,
        ],
        [3.5, 9.25, 0., 50., 0.5],
        [3.5, 9.25, 100., 0., 3.],
        [1e90, 3e90, 12.3456789012345, 43.210987654321, 0.123456789],
    ] {
        let mut i = fixture();
        for (stat, value, unit) in [
            (0x33b2, normal, 0x1d3a),
            (0x33b3, critical, 0x1d3a),
            (0x33aa, crit, 2),
            (0x3220, hit, 2),
            (0x33a3, rate, 0x1d39),
        ] {
            set_derived(&mut i, stat, value, unit);
        }
        i.rebind_test_edit().unwrap();
        let weighted = normal * (1. - crit / 100.) + critical * crit / 100.;
        let after = weighted * hit / 100.;
        assert_eq!(
            values(&i, &critical_native::evaluate(&i)),
            vec![[weighted, after, after * rate]; 2]
        );
    }
}

#[test]
fn physical_dps_retains_coverage_gaps_and_propagates_unsupported_domains() {
    for mode in 0..3 {
        let mut i = fixture();
        if mode == 0 {
            let c = dps::consumer();
            i.rules
                .owners
                .iter_mut()
                .find(|o| o.owner == owner())
                .unwrap()
                .programs
                .closure = c.owners[0].programs.closure.clone();
        } else if mode == 1 {
            let authored: Value = serde_json::from_slice(
                &std::fs::read(
                    dps::root()
                        .join("data/owned/poe2/3887ae68/enemy-physical-mitigation/consumer.json"),
                )
                .unwrap(),
            )
            .unwrap();
            let q: ContributionQuery =
                serde_json::from_value(authored["queries"][0].clone()).unwrap();
            i.rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .iter_mut()
                .find(|v| v.id == q.id)
                .unwrap()
                .groups[0]
                .members
                .closure = q.groups[0].members.closure.clone();
        } else {
            let p = program_mut(&mut i, "selected-physical-hit");
            p.nodes
                .iter_mut()
                .find(|n| n.id == key("rounded-min"))
                .unwrap()
                .expression = RuleExpression::Literal {
                value: quantity(-1., 0x1d3a),
            };
        }
        i.rebind_test_edit().unwrap();
        let report = critical_native::evaluate(&i);
        if mode == 2 {
            for a in critical_native::actions(&i) {
                assert!(matches!(
                    critical_native::value(&report, ConcreteEntity::Action(Box::new(a)), 0x33ba),
                    EffectValue::UnsupportedDomain { .. }
                ));
            }
        } else {
            assert!(
                !report.gaps.is_empty()
                    || match &report.outcome {
                        SupportEffectsOutcome::Evaluated { effects } => !effects.gaps.is_empty(),
                        _ => true,
                    }
            );
        }
    }
}

#[test]
fn physical_dps_late_operand_writer_is_rejected_even_when_zero_or_inactive() {
    for inactive in [false, true] {
        let mut i = fixture();
        let late = key("synthetic.after-physical-dps");
        i.stages.stages.push(EvaluationStage {
            id: late.clone(),
            predecessors: vec![key(FINAL)],
        });
        install(
            &mut i,
            RuleProgram {
                id: key("synthetic.late-hit"),
                context: RuleEntityKind::Action,
                reads: vec![],
                nodes: vec![
                    RuleNode {
                        id: key("zero"),
                        expression: RuleExpression::Literal {
                            value: quantity(0., 2),
                        },
                    },
                    RuleNode {
                        id: key("inactive"),
                        expression: RuleExpression::Literal {
                            value: ParameterValue::Boolean(false),
                        },
                    },
                ],
                effects: vec![RuleEffect {
                    id: key("write"),
                    when: inactive.then(|| key("inactive")),
                    effect: RuleEffectKind::Derive {
                        entity: RuleEntity::Current,
                        stat: def(0x3220),
                        value: key("zero"),
                    },
                }],
            },
            late.as_str(),
        );
        assert!(
            i.rebind_test_edit()
                .and_then(|_| i.compile().map(|_| ()))
                .is_err()
        );
    }
}

#[test]
fn physical_dps_fresh_reused_parallel_and_changed_identity_reports_agree() {
    let i = fixture();
    let p = i.compile().unwrap();
    let expected = p.evaluate(&mut p.new_scratch()).unwrap();
    let mut scratch = p.new_scratch();
    for _ in 0..3 {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    }
    let parallel: Vec<_> = (0..4)
        .into_par_iter()
        .map(|_| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert!(parallel.iter().all(|r| r == &expected));
    let mut changed = i.clone();
    set_derived(&mut changed, 0x3220, 25., 2);
    changed.rebind_test_edit().unwrap();
    let q = changed.compile().unwrap();
    let report = q.evaluate(&mut scratch).unwrap();
    assert_eq!(report, q.evaluate(&mut q.new_scratch()).unwrap());
    assert_ne!(report, expected);
    for (old, new) in values(&i, &expected)
        .into_iter()
        .zip(values(&changed, &report))
    {
        assert_eq!(new[0], old[0]);
        assert_eq!(new[1], old[1] * 0.25);
        assert_eq!(new[2], old[2] * 0.25);
    }
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
}
