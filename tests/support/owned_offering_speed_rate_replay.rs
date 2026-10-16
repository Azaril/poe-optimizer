//! Execute the authored application and timing recipe in the maintained finite
//! component graph. Its explicit completions do not complete an imported build.
use super::{family, replay::ReplayInput};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::fs;

const FINAL: &str = "selected-basic-ordinary-rate";
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
fn owner() -> SchemaSubject {
    family::consumer().owners[0].owner.clone()
}
fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn fixture() -> ReplayInput {
    let path = family::root().join("tests/fixtures/owned-sniper-replay.json.gz");
    assert!(fs::metadata(&path).unwrap().len() < 256 * 1024);
    let mut i = ReplayInput::decode(&fs::read(path).unwrap());
    let dependencies: Value = family::read("dependencies.json");
    let prior: DeclaredSet<EffectApplicationRule> =
        serde_json::from_value(dependencies["prior_application_registry"].clone()).unwrap();
    assert_eq!(
        i.rules.effect_applications.as_ref().unwrap().members,
        prior.members
    );
    assert!(i.rules.effect_applications.as_ref().unwrap().is_complete());
    i.rules.effect_applications = Some(DeclaredSet::complete(family::applications().members));
    for d in family::definitions() {
        assert!(
            !i.schema
                .definitions
                .iter()
                .any(|old| old.address() == d.address())
        );
        i.rules.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(d.address()),
            programs: DeclaredSet::complete(vec![]),
        });
        i.schema.definitions.push(d);
    }
    i.rules.tables.extend(family::tables());
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(family::queries());
    i.stages.stages.push(EvaluationStage {
        id: key(FINAL),
        predecessors: vec![key("offering-application"), key("deliver")],
    });
    let p = family::consumer().owners[0].programs.members[0].clone();
    i.stages.programs.members.push(StagedRuleProgram {
        owner: owner(),
        program: p.id.clone(),
        stage: key(FINAL),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner: owner(),
            program: p.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner())
        .unwrap()
        .programs
        .members
        .push(p);
    i.stages.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: def(family::SPEED),
                contribution: ContributionKind::Add,
            },
            stage: key("offering-application"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Action,
                stat: def(0x3214),
            },
            stage: key("deliver"),
        },
    ]);
    i.rebind_test_edit().unwrap();
    i
}
fn evaluate(i: &ReplayInput) -> SupportEffectsReport {
    let p = i.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn rates(i: &ReplayInput, report: &SupportEffectsReport) -> Vec<EffectValue> {
    assert!(report.gaps.is_empty(), "component report has gaps");
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("component unavailable")
    };
    i.queries
        .requests
        .iter()
        .filter_map(|request| {
            let MetricTarget::Action(a) = &request.target else {
                return None;
            };
            let found = effects.values.iter().find(|v| {
                v.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Action(a.clone()),
                        stat: def(family::RATE),
                    }
            });
            if a.action.output.slot != def(0x22) {
                assert!(found.is_none());
                return None;
            }
            Some(found.expect("exact Basic endpoint").value.clone())
        })
        .collect()
}
fn known_rates(i: &ReplayInput, report: &SupportEffectsReport, expected: f64) {
    let values = rates(i, report);
    assert_eq!(values.len(), 2);
    for value in values {
        assert_eq!(
            value,
            EffectValue::Known {
                value: quantity(expected, 0x1d39)
            }
        );
    }
}
fn groups<'a>(
    report: &'a SupportEffectsReport,
    modifier: &str,
) -> Vec<&'a EffectApplicationGroupResult> {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("component unavailable")
    };
    effects.application_groups.iter().filter(|g| matches!(&g.key.invocation.origin, RuleOrigin::EffectApplicationGroup { family, modifier: m, .. } if *family == key("pain-offering-buff") && *m == key(modifier))).collect()
}
fn speed(report: &SupportEffectsReport, expected: f64, winners: usize) {
    let groups = groups(report, "attack-cast-speed-increase");
    assert_eq!(groups.len(), 2);
    for g in groups {
        assert_eq!(
            g.value,
            EffectValue::Known {
                value: quantity(expected, 2)
            }
        );
        assert_eq!(g.candidates.len(), 2);
        assert_eq!(g.co_winners.len(), winners);
        let RuleOrigin::EffectApplicationGroup {
            recipient: ConcreteEntity::Actor(actor),
            ..
        } = &g.key.invocation.origin
        else {
            panic!("exact Actor recipient")
        };
        for c in &g.candidates {
            assert!(
                matches!(&c.invocation.origin, RuleOrigin::EffectApplication { application, source: ConcreteEntity::Skill(_), recipient: ConcreteEntity::Actor(a) } if *application == key(family::APPLICATION) && a == actor)
            );
        }
    }
}
fn levels(i: &mut ReplayInput, values: [u16; 2]) {
    let gems: Vec<_> = i
        .build
        .gems
        .iter_mut()
        .filter(|g| g.definition == def(0x86b))
        .collect();
    assert_eq!(gems.len(), 2);
    for (g, level) in gems.into_iter().zip(values) {
        g.level = level;
    }
}
fn active(i: &mut ReplayInput, values: [bool; 2]) {
    let rows: Vec<_> = i
        .scenario
        .usage
        .iter_mut()
        .filter(|u| u.policy == def(0x3259))
        .collect();
    assert_eq!(rows.len(), 2);
    for (row, value) in rows.into_iter().zip(values) {
        row.parameters[0].value = ParameterValue::Boolean(value);
    }
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
fn scale_control(i: &mut ReplayInput, name: &str, value: f64, unit: u64) {
    let p = program_mut(i, name);
    assert_eq!(p.nodes.len(), 1);
    assert!(matches!(p.nodes[0].expression, RuleExpression::Read { .. }));
    // A synthetic upstream value tests the actual application. It grants no
    // source supplier coverage and does not edit the application formula.
    p.nodes[0].expression = RuleExpression::Literal {
        value: quantity(value, unit),
    };
    p.reads.clear();
}
fn source_rate(case: &str) -> f64 {
    let v: Value = family::read("source-vectors.json");
    v["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case"] == case && v["mode"] == "main")
        .unwrap()["basic_passes"][0]["speed"]
        .as_f64()
        .unwrap()
}

#[test]
fn actual_offering_table_and_ordinary_timing_match_retained_level_controls() {
    let base = fixture();
    for (levels_value, case, increase, winners) in [
        ([20, 20], "original-05", 30., 2),
        ([1, 1], "offering-level-1", 21., 2),
        ([30, 20], "offering-higher-first", 35., 1),
        ([20, 30], "offering-higher-last", 35., 1),
    ] {
        let mut i = base.clone();
        levels(&mut i, levels_value);
        i.rebind_test_edit().unwrap();
        let r = evaluate(&i);
        speed(&r, increase, winners);
        known_rates(&i, &r, source_rate(case));
    }
    let mut disabled = base;
    active(&mut disabled, [false, false]);
    disabled.rebind_test_edit().unwrap();
    let r = evaluate(&disabled);
    known_rates(&disabled, &r, source_rate("offering-disabled"));
    assert!(
        groups(&r, "attack-cast-speed-increase")
            .iter()
            .all(|g| g.value == EffectValue::Inactive)
    );
}

#[test]
fn actual_speed_application_uses_source_and_recipient_scaling() {
    for (source, recipient, case, amount) in [
        (50., 0., "offering-source-buff-effect", 45.),
        (0., 25., "offering-recipient-buff-effect", 37.),
        (50., 25., "offering-both-buff-effects", 52.),
    ] {
        let mut i = fixture();
        scale_control(&mut i, "source-empty-buff-effect-increase", source, 2);
        scale_control(&mut i, "recipient-empty-buff-effect-increase", recipient, 2);
        i.rebind_test_edit().unwrap();
        let r = evaluate(&i);
        speed(&r, amount, 2);
        known_rates(&i, &r, source_rate(case));
    }
}

#[test]
fn speed_maximum_is_independent_from_damage_winner() {
    let mut i = fixture();
    levels(&mut i, [1, 20]);
    let p = program_mut(&mut i, "source-empty-magnitude");
    p.reads = vec![RuleRead {
        id: key("level"),
        value_type: ComputedValueType::Integer,
        source: RuleReadSource::Parameter {
            slot: DeclaredSlot {
                declaration: SlotOwnerDefId::Skill(def(0x2a2)),
                slot: def(0x3223),
            },
        },
    }];
    p.nodes = serde_json::from_value(json!([
        {"id":"level","expression":{"kind":"read","input":"level"}},
        {"id":"three","expression":{"kind":"literal","value":{"kind":"integer","value":3}}},
        {"id":"low","expression":{"kind":"compare","operation":"equal","left":"level","right":"three"}},
        {"id":"double","expression":{"kind":"literal","value":quantity(2.,1)}},
        {"id":"one","expression":{"kind":"literal","value":quantity(1.,1)}},
        {"id":"resolved","expression":{"kind":"select","condition":"low","when_true":"double","when_false":"one"}}
    ])).unwrap();
    i.rebind_test_edit().unwrap();
    let r = evaluate(&i);
    speed(&r, 42., 1);
    let damage = groups(&r, "unconditional-damage-increase");
    let speed = groups(&r, "attack-cast-speed-increase");
    assert_eq!(damage.len(), 2);
    for (d, s) in damage.into_iter().zip(speed) {
        assert_eq!(
            d.value,
            EffectValue::Known {
                value: quantity(62., 2)
            }
        );
        assert_eq!(d.co_winners.len(), 1);
        assert_ne!(
            d.co_winners[0].invocation.origin,
            s.co_winners[0].invocation.origin
        );
    }
    known_rates(&i, &r, 1. / (1.5 / 1.42));
}

#[test]
fn missing_inputs_and_inactive_sources_do_not_supply_neutral_rates() {
    for name in [
        "source-empty-buff-effect-increase",
        "source-empty-buff-effect-more",
        "source-empty-magnitude",
        "skill-effect-activation",
        "intrinsic-minion-attack-source",
    ] {
        let mut i = fixture();
        for o in &mut i.rules.owners {
            o.programs.members.retain(|p| p.id != key(name));
        }
        i.stages.programs.members.retain(|p| p.program != key(name));
        i.stages
            .readiness
            .as_mut()
            .unwrap()
            .programs
            .members
            .retain(|p| p.program != key(name));
        i.rebind_test_edit().unwrap();
        let r = evaluate(&i);
        assert!(
            rates(&i, &r)
                .iter()
                .all(|v| matches!(v, EffectValue::Unresolved { .. })),
            "missing {name}"
        );
        if !matches!(
            name,
            "skill-effect-activation" | "intrinsic-minion-attack-source"
        ) {
            active(&mut i, [false, false]);
            i.rebind_test_edit().unwrap();
            known_rates(&i, &evaluate(&i), source_rate("offering-disabled"));
        }
    }
}

#[test]
fn late_channels_and_partial_application_membership_stay_blocking() {
    let base = fixture();
    for stat in [family::SPEED, 0x3214] {
        let mut i = base.clone();
        i.stages.stages.push(EvaluationStage {
            id: key("synthetic.too-late"),
            predecessors: vec![key(FINAL)],
        });
        i.stages
            .frozen_channels
            .iter_mut()
            .find(|c| match &c.channel {
                StageChannel::Contributions { stat: s, .. }
                | StageChannel::Stat { stat: s, .. } => *s == def(stat),
                _ => false,
            })
            .unwrap()
            .stage = key("synthetic.too-late");
        assert!(
            i.rebind_test_edit()
                .and_then(|_| i.compile().map(|_| ()))
                .is_err()
        );
    }
    let mut partial = base.clone();
    partial.rules.effect_applications.as_mut().unwrap().closure = family::applications().closure;
    partial.rebind_test_edit().unwrap();
    let plan = partial.compile().unwrap();
    assert!(
        plan.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialEffectApplications)
    );
    let mut query_partial = base.clone();
    query_partial
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(family::SPEED))
        .unwrap()
        .groups[0]
        .members
        .closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(def::<StatDefinition>(family::SPEED).address()),
            facet: SchemaFacet::GameRules,
            code: key("synthetic-unknown-speed-supplier"),
        }],
    };
    query_partial.rebind_test_edit().unwrap();
    let p = query_partial.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut absent = base;
    absent.rules.effect_applications.as_mut().unwrap().members[0]
        .program
        .effects
        .pop();
    absent.rules.effect_applications.as_mut().unwrap().members[0]
        .stacking
        .pop();
    assert!(absent.rebind_test_edit().is_err());
}

#[test]
fn real_tick_cap_and_out_of_table_levels_are_not_rounded_to_nearby_defaults() {
    let mut i = fixture();
    scale_control(&mut i, "source-empty-buff-effect-increase", 50_000., 2);
    i.rebind_test_edit().unwrap();
    let r = evaluate(&i);
    // Synthetic magnitude input, actual authored 1/0.033 server-tick cap.
    known_rates(&i, &r, 1. / 0.033);
    let mut unsupported = fixture();
    levels(&mut unsupported, [40, 40]);
    unsupported.rebind_test_edit().unwrap();
    assert!(
        rates(&unsupported, &evaluate(&unsupported))
            .iter()
            .all(|v| matches!(v, EffectValue::Unresolved { .. }))
    );
    active(&mut unsupported, [false, false]);
    unsupported.rebind_test_edit().unwrap();
    // Out-of-domain source preparation remains unavailable even when a later
    // usage preference is false; activation cannot repair that earlier failure.
    assert!(
        rates(&unsupported, &evaluate(&unsupported))
            .iter()
            .all(|v| matches!(v, EffectValue::Unresolved { .. }))
    );
}

#[test]
fn rates_and_application_identity_survive_reordering_reuse_and_rayon() {
    let a = fixture();
    let mut b = a.clone();
    active(&mut b, [false, false]);
    b.rebind_test_edit().unwrap();
    let pa = a.compile().unwrap();
    let pb = b.compile().unwrap();
    let mut scratch = pa.new_scratch();
    let ra = pa.evaluate(&mut scratch).unwrap();
    let rb = pb.evaluate(&mut scratch).unwrap();
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), ra);
    known_rates(&a, &ra, source_rate("original-05"));
    known_rates(&b, &rb, source_rate("offering-disabled"));
    let mut reversed = a;
    reversed.build.skills.reverse();
    reversed.build.gems.reverse();
    reversed.scenario.usage.reverse();
    reversed.rebind_test_edit().unwrap();
    assert_eq!(evaluate(&reversed), ra);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let actual = pool.install(|| {
        (0..12)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |s, n| {
                    if n % 2 == 0 {
                        pa.evaluate(s).unwrap()
                    } else {
                        pb.evaluate(s).unwrap()
                    }
                },
            )
            .collect::<Vec<_>>()
    });
    for (n, r) in actual.iter().enumerate() {
        assert_eq!(r, if n % 2 == 0 { &ra } else { &rb });
    }
}
