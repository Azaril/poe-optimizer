//! Actual authored programs and saved metric in a finite controlled replay.
//! Explicit test completions never alter the published Partial owners.
use super::{evasion_family as family, replay::ReplayInput};
use poe_optimizer_core::{
    build_identity::InstanceAllocator,
    owned_build::*,
    owned_definitions::*,
    owned_draft::{MetricRequestDraft, RewardDraft},
    owned_metrics::*,
    owned_readiness::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_data::owned_metrics::OwnedMetricMapping;
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::{fs, sync::Arc};
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
fn quantity(v: f64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(v, def(family::UNIT)).unwrap())
}
pub(crate) fn declare(i: &mut ReplayInput, owner: SchemaSubject, p: RuleProgram, stage: &str) {
    let existing = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap();
    assert!(!existing.programs.members.iter().any(|old| old.id == p.id));
    i.stages.programs.members.push(StagedRuleProgram {
        owner: owner.clone(),
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
            owner,
            program: p.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    existing.programs.members.push(p);
}
pub(crate) fn fixture() -> ReplayInput {
    let path = family::root().join("tests/fixtures/owned-sniper-replay.json.gz");
    assert!(fs::metadata(&path).unwrap().len() < 256 * 1024);
    let mut i = ReplayInput::decode(&fs::read(path).unwrap());
    let d: Value = family::read("dependencies.json");
    let c = family::consumer();
    for desc in d["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .chain(d["donor_definitions"].as_array().unwrap())
        .cloned()
        .chain(family::definitions().into_iter().map(|d| json!(d)))
    {
        let desc: DefinitionDescriptor = serde_json::from_value(desc).unwrap();
        if let Some(old) = i
            .schema
            .definitions
            .iter()
            .find(|old| old.address() == desc.address())
        {
            assert_eq!(old, &desc);
        } else {
            i.schema.definitions.push(desc.clone());
        }
        if !i
            .rules
            .owners
            .iter()
            .any(|o| o.owner == SchemaSubject::Definition(desc.address()))
        {
            i.rules.owners.push(DefinitionRules {
                owner: SchemaSubject::Definition(desc.address()),
                programs: DeclaredSet::complete(vec![]),
            });
        }
    }
    let shared: SchemaSubject = serde_json::from_value(d["shared_owner"]["owner"].clone()).unwrap();
    declare(&mut i, shared, c.intrinsic, "deliver");
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    let mut count = 0;
    for value in d["selected_snapshot"]["rewards"]["members"]
        .as_array()
        .unwrap()
    {
        let reward: RewardDraft = serde_json::from_value(value.clone()).unwrap();
        let Some(mut reward) = reward.to_resolved() else {
            continue;
        };
        if ![def::<RewardDefinition>(0x41), def(0x5a)].contains(&reward.definition) {
            continue;
        }
        assert!(
            d["selected_snapshot"]["choice_presets"]["members"][0]["rewards"]["members"]
                .as_array()
                .unwrap()
                .contains(&value["id"])
        );
        reward.id = allocator.allocate().unwrap();
        i.build.character.rewards.push(reward);
        count += 1;
    }
    assert_eq!(count, 2);
    i.build.allocator = allocator.state();
    for declaration in d["source_support_domains"].as_array().unwrap() {
        let declaration = serde_json::from_value(declaration.clone()).unwrap();
        assert!(
            !i.rules
                .support_discovery
                .as_ref()
                .unwrap()
                .providers
                .contains(&declaration)
        );
        i.rules
            .support_discovery
            .as_mut()
            .unwrap()
            .providers
            .push(declaration);
    }
    let mut reward_owners = vec![];
    for o in d["source_owners"].as_array().unwrap() {
        let o: DefinitionRules = serde_json::from_value(o.clone()).unwrap();
        assert!(o.programs.is_complete());
        reward_owners.push(o.owner.clone());
        for p in o.programs.members {
            declare(&mut i, o.owner.clone(), p, "deliver");
        }
    }
    for mut q in c.queries {
        // The published census retains all65 potential INC owners. This finite
        // graph contains the two actual quest occurrences and no passive INC.
        if q.id.as_str() == "player-evasion-increase" {
            q.groups[0].members.members.retain(|m|matches!(&m.producer,ContributionProducer::ProgramEffect(p) if reward_owners.contains(&p.owner)));
            assert_eq!(q.groups[0].members.members.len(), 2);
        }
        i.rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .push(q);
    }
    i.rules.receivers.members.push(c.receiver);
    i.stages.stages.push(EvaluationStage {
        id: key("player-evasion-final"),
        predecessors: vec![key("deliver")],
    });
    declare(
        &mut i,
        c.owner.owner,
        c.owner.programs.members[0].clone(),
        "player-evasion-final",
    );
    for contribution in [
        ContributionKind::Add,
        ContributionKind::Increase,
        ContributionKind::Multiply,
    ] {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: def(family::CHANNEL),
                contribution,
            },
            stage: key("deliver"),
        });
    }
    i.rebind_test_edit().unwrap();
    i
}
pub(crate) fn evaluate(i: &ReplayInput) -> SupportEffectsReport {
    let p = i.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
pub(crate) fn value(r: &SupportEffectsReport) -> EffectValue {
    assert!(r.gaps.is_empty(), "finite graph gaps: {:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("finite graph unavailable")
    };
    effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(family::FINAL),
                }
        })
        .unwrap()
        .value
        .clone()
}
pub(crate) fn known(r: &SupportEffectsReport, n: f64) {
    assert_eq!(value(r), EffectValue::Known { value: quantity(n) });
}
#[test]
fn actual_two_reward_occurrences_drive_player_evasion() {
    let original = fixture();
    known(&evaluate(&original), 10.);
    for (remove, expected) in [(0x41, 8.), (0x5a, 9.)] {
        let mut i = original.clone();
        i.build
            .character
            .rewards
            .retain(|r| r.definition != def(remove));
        i.rebind_test_edit().unwrap();
        known(&evaluate(&i), expected);
    }
    let mut i = original;
    i.build
        .character
        .rewards
        .retain(|r| ![def::<RewardDefinition>(0x41), def(0x5a)].contains(&r.definition));
    i.rebind_test_edit().unwrap();
    known(&evaluate(&i), 7.);
}
#[test]
fn authored_final_rounding_guard_refuses_adjacent_outside_domain() {
    for (base, expected) in [
        (-7., Some(0.)),
        (0.4999999999999999, Some(0.)),
        (0.49999999999999994, None),
        (0.5, Some(1.)),
        (4_503_599_627_370_496., Some(4_503_599_627_370_496.)),
        (4_503_599_627_370_497., None),
    ] {
        let mut i = fixture();
        i.build
            .character
            .rewards
            .retain(|r| ![def::<RewardDefinition>(0x41), def(0x5a)].contains(&r.definition));
        // Synthetic numeric input control of the real final consumer. This is
        // not a claim that the source intrinsic constant can vary.
        let p = i
            .rules
            .owners
            .iter_mut()
            .flat_map(|o| &mut o.programs.members)
            .find(|p| p.id.as_str() == "intrinsic-player-evasion")
            .unwrap();
        p.nodes[0].expression = RuleExpression::Literal {
            value: quantity(base),
        };
        i.rebind_test_edit().unwrap();
        let report = evaluate(&i);
        if let Some(expected) = expected {
            known(&report, expected);
        } else {
            let expected_node = if base < 0.5 {
                "checked-below-half"
            } else {
                "checked-above-half"
            };
            assert!(
                matches!(value(&report), EffectValue::UnsupportedDomain { node, domain: poe_optimizer_engine::owned_rules::UnsupportedDomainCause::NumericRange { value, .. } } if node.as_str()==expected_node && value==quantity(base))
            );
        }
    }
}
#[test]
fn published_partial_owner_and_late_channel_remain_unavailable() {
    let base = fixture();
    let mut partial = base.clone();
    let c = family::consumer();
    partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owner.owner)
        .unwrap()
        .programs
        .closure = c.owner.programs.closure;
    partial.rebind_test_edit().unwrap();
    let p = partial.compile().unwrap();
    let report = p.evaluate(&mut p.new_scratch()).unwrap();
    assert!(
        matches!(report.outcome, SupportEffectsOutcome::Unavailable { .. }),
        "production owner is not silently completed"
    );
    let mut late = base;
    late.stages.stages.push(EvaluationStage {
        id: key("evasion-too-late"),
        predecessors: vec![key("player-evasion-final")],
    });
    late.stages.frozen_channels.iter_mut().find(|f|matches!(&f.channel,StageChannel::Contributions{stat,contribution:ContributionKind::Add,..} if *stat==def(family::CHANNEL))).unwrap().stage=key("evasion-too-late");
    assert!(late.rebind_test_edit().is_err());
}
#[test]
fn finite_evasion_reorder_scratch_and_rayon_are_deterministic() {
    let a = fixture();
    let mut b = a.clone();
    b.build.character.rewards.reverse();
    b.rebind_test_edit().unwrap();
    let pa = a.compile().unwrap();
    let pb = b.compile().unwrap();
    let mut scratch = pa.new_scratch();
    let expected = pa.evaluate(&mut scratch).unwrap();
    known(&expected, 10.);
    assert_eq!(pb.evaluate(&mut scratch).unwrap(), expected);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), expected);
    let actual: Vec<_> = (0..8)
        .into_par_iter()
        .map_init(|| pa.new_scratch(), |s, _| pa.evaluate(s).unwrap())
        .collect();
    assert!(actual.iter().all(|r| r == &expected));
}
#[test]
fn actual_saved_reference_19_binds_metric_313e_to_player_evasion() {
    let d: Value = family::read("dependencies.json");
    let raw =
        d["selected_snapshot"]["query_presets"]["members"][0]["queries"]["requests"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|q| q["id"] == "reference-19")
            .unwrap();
    let saved: MetricRequestDraft = serde_json::from_value(raw.clone()).unwrap();
    let saved = saved.to_resolved().unwrap();
    assert_eq!(saved.metric, def(0x313e));
    assert_eq!(saved.target, MetricTarget::Actor(ActorKey::Player));
    let mut i = fixture();
    i.queries.requests.push(saved.clone());
    i.rebind_test_edit().unwrap();
    let effects = i.compile().unwrap();
    let mapping = OwnedMetricMapping::new(
        MetricMappingInput {
            schema_version: OWNED_METRIC_MAPPING_VERSION,
            namespace: i.schema.namespace.clone(),
            release: key("finite-player-evasion"),
            definitions: effects.definitions().identity().clone(),
            bindings: vec![MetricStatBinding {
                metric: saved.metric.clone(),
                role: MetricBindingRole::PlayerActor,
                stat: def(family::FINAL),
            }],
        },
        effects.definitions(),
        Default::default(),
    )
    .unwrap();
    let plan = OwnedSupportMetricPlan::compile(Arc::new(effects), Arc::new(mapping)).unwrap();
    let report = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let result = report
        .evaluation
        .results
        .iter()
        .find(|r| r.request.id == saved.id)
        .unwrap();
    assert_eq!(result.request, saved);
    assert_eq!(result.stat, Some(def(family::FINAL)));
    assert_eq!(
        result.value,
        EffectValue::Known {
            value: quantity(10.)
        }
    );
}
