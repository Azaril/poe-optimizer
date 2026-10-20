//! Finite composition authority is test-only; production condition coverage stays Partial.
use crate::{critical_native, family, replay::ReplayInput};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use std::fs;

const PROGRAM: &str = "configured-critical-weakness-activation";
const QUERY: &str = "outgoing-critical-weakness-enabled";
const SYNTHETIC: &str = "synthetic.critical-weakness-enabled";
const SYNTHETIC_PROGRAM: &str = "synthetic.critical-weakness-condition";

fn key(name: &str) -> OwnedDefinitionKey {
    name.parse().unwrap()
}
fn named<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        name,
    )
    .unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    named(&format!("def.{n:016x}"))
}
fn read(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(family::root().join(format!(
            "data/owned/poe2/3887ae68/configuration-critical-weakness-activation/{name}"
        )))
        .unwrap(),
    )
    .unwrap()
}
fn fixture(enabled: Option<bool>, count: Option<f64>) -> ReplayInput {
    let mut input = critical_native::fixture(None, count);
    let packet = read("consumer.json");
    let actual: RuleProgram = serde_json::from_value(packet["program"].clone()).unwrap();
    assert_eq!(actual.id, key(PROGRAM));
    let member: ContributionMember = serde_json::from_value(packet["member"].clone()).unwrap();
    let ContributionProducer::ProgramEffect(producer) = &member.producer else {
        panic!("real configured source is an Encounter program");
    };
    assert_eq!(producer.origin, ContributionOrigin::Encounter);
    let descriptors: Vec<DefinitionDescriptor> =
        serde_json::from_value(read("definitions.json")).unwrap();
    assert_eq!(descriptors.len(), 1);
    assert_eq!(
        descriptors[0].address(),
        def::<ExternalInputDefinition>(0x33b1).address()
    );

    let synthetic = named::<ExternalInputDefinition>(SYNTHETIC).address();
    let descriptor = input
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == synthetic)
        .unwrap();
    *descriptor = descriptors[0].clone();
    let owner = input
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == SchemaSubject::Definition(synthetic.clone()))
        .unwrap();
    assert!(owner.programs.members.is_empty());
    owner.owner = SchemaSubject::Definition(descriptors[0].address());
    let DefinitionDescriptor::Encounter(entry) = input
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == def::<EncounterDefinition>(0x31d1).address())
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut entry.schema else {
        unreachable!()
    };
    let slot = schema
        .external_inputs
        .members
        .iter_mut()
        .find(|id| **id == named(SYNTHETIC))
        .unwrap();
    *slot = def(0x33b1);

    let owner = input
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == producer.owner)
        .unwrap();
    let old = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(SYNTHETIC_PROGRAM))
        .unwrap();
    *old = actual;
    let stage = input
        .stages
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key(SYNTHETIC_PROGRAM))
        .unwrap();
    assert_eq!(stage.owner, producer.owner);
    stage.program = key(PROGRAM);
    let ready = input
        .stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.program == key(SYNTHETIC_PROGRAM))
        .unwrap();
    assert_eq!(ready.owner, producer.owner);
    ready.program = key(PROGRAM);
    let query = input
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == key(QUERY))
        .unwrap();
    assert_eq!(query.groups[0].members.members.len(), 1);
    query.groups[0].members.members = vec![member];
    if let Some(value) = enabled {
        input.scenario.assumptions.push(ExternalAssumption {
            input: def(0x33b1),
            target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
            value: ParameterValue::Boolean(value),
        });
    }
    let serialized = serde_json::to_string(&input).unwrap();
    assert!(!serialized.contains(SYNTHETIC));
    assert!(!serialized.contains(SYNTHETIC_PROGRAM));
    input.rebind_test_edit().unwrap();
    input
}
fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(report.gaps.is_empty());
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite controlled composition should execute");
    };
    effects
}
fn chance(input: &ReplayInput, report: &SupportEffectsReport, expected: Option<f64>) {
    for action in critical_native::actions(input) {
        let actual =
            critical_native::value(report, ConcreteEntity::Action(Box::new(action)), 0x33aa);
        if let Some(n) = expected {
            assert_eq!(
                actual,
                &EffectValue::Known {
                    value: ParameterValue::Quantity(FiniteQuantity::new(n, def(2)).unwrap())
                }
            );
        } else {
            assert!(
                !matches!(actual, EffectValue::Known { .. }),
                "missing is not false"
            );
        }
    }
}

#[test]
fn actual_configured_flag_feeds_the_conditional_numeric_producer() {
    for (enabled, count, expected) in [
        (Some(false), Some(20.), Some(5.)),
        (Some(true), Some(20.), Some(15.)),
        (Some(true), Some(0.), Some(5.)),
        (Some(true), Some(80.), Some(15.)),
        (Some(false), None, Some(5.)),
        (Some(true), None, None),
        (None, Some(20.), None),
    ] {
        let input = fixture(enabled, count);
        let report = critical_native::evaluate(&input);
        chance(&input, &report, expected);
        let rows: Vec<_> = effects(&report)
            .effects
            .iter()
            .filter(|e| e.key.invocation.program == key(PROGRAM))
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].key.invocation.origin, RuleOrigin::Encounter);
        assert_eq!(
            rows[0].key.invocation.entity,
            ConcreteEntity::Actor(ActorKey::ScenarioEnemy)
        );
        match enabled {
            Some(false) => assert_eq!(rows[0].value, EffectValue::Inactive),
            Some(true) => assert_eq!(
                rows[0].value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }
            ),
            None => assert!(!matches!(
                rows[0].value,
                EffectValue::Known { .. } | EffectValue::Inactive
            )),
        }
    }
}

#[test]
fn configured_source_requires_enemy_authority_membership_and_pre_freeze_stage() {
    for mutation in 0..3 {
        let mut input = fixture(Some(false), Some(20.));
        match mutation {
            0 => {
                input
                    .scenario
                    .assumptions
                    .iter_mut()
                    .find(|a| a.input == def(0x33b1))
                    .unwrap()
                    .target = AssumptionTarget::Actor(ActorKey::Player)
            }
            1 => input
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .iter_mut()
                .find(|q| q.id == key(QUERY))
                .unwrap()
                .groups[0]
                .members
                .members
                .clear(),
            _ => {
                input
                    .stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.program == key(PROGRAM))
                    .unwrap()
                    .stage = key("ordinary-outgoing-critical")
            }
        }
        assert!(
            input.rebind_test_edit().is_err() || input.compile().is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn configured_conditions_preserve_production_partial_coverage() {
    let mut input = fixture(Some(false), Some(20.));
    let original = family::consumer();
    input
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == key(QUERY))
        .unwrap()
        .groups[0]
        .members
        .closure = original
        .queries
        .iter()
        .find(|q| q.id == key(QUERY))
        .unwrap()
        .groups[0]
        .members
        .closure
        .clone();
    input.rebind_test_edit().unwrap();
    let report = critical_native::evaluate(&input);
    assert!(
        !report.gaps.is_empty()
            || match &report.outcome {
                SupportEffectsOutcome::Evaluated { effects } => !effects.gaps.is_empty(),
                _ => true,
            }
    );
}

#[test]
fn configured_input_changes_are_deterministic_across_reused_and_parallel_workers() {
    let input = fixture(Some(false), Some(20.));
    let plan = input.compile().unwrap();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    let mut scratch = plan.new_scratch();
    for enabled in [Some(true), None, Some(false)] {
        let changed = fixture(enabled, Some(20.));
        let changed_plan = changed.compile().unwrap();
        let fresh = changed_plan
            .evaluate(&mut changed_plan.new_scratch())
            .unwrap();
        assert_eq!(changed_plan.evaluate(&mut scratch).unwrap(), fresh);
        let parallel: Vec<_> = (0..4)
            .into_par_iter()
            .map(|_| {
                changed_plan
                    .evaluate(&mut changed_plan.new_scratch())
                    .unwrap()
            })
            .collect();
        assert!(parallel.iter().all(|r| r == &fresh));
        chance(&changed, &fresh, enabled.map(|b| if b { 15. } else { 5. }));
    }
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
}
