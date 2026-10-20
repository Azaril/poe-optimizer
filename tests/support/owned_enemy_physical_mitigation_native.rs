//! Actual current hit/critical/configured-Armour programs feed the authored graph.
//! Only the controlled fixture closes memberships; production remains Partial.
use super::{critical_native, family, replay::ReplayInput};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde::Deserialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;

const PROGRAM: &str = "ordinary-armour-adjusted-physical-averages";
const ARMOUR: &str = "configured-enemy-armour-base";
const ARMOUR_STAGE: &str = "configured-enemy-armour";
const HIT_STAGE: &str = "selected-physical-hit-final";
const FINAL: &str = "armour-adjusted-physical-averages";
const DAMAGE: u64 = 0x1d3a;
const RATING: u64 = 0x29ee;
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
fn data(name: &str) -> Value {
    let path = family::root()
        .join("data/owned/poe2/3887ae68/enemy-physical-mitigation")
        .join(name);
    assert!(fs::metadata(&path).unwrap().len() <= 2 * 1024 * 1024);
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Consumer {
    owners: Vec<DefinitionRules>,
    queries: Vec<ContributionQuery>,
}
fn consumer() -> Consumer {
    serde_json::from_value(data("consumer.json")).unwrap()
}
fn owner() -> SchemaSubject {
    consumer().owners[0].owner.clone()
}
fn encounter() -> SchemaSubject {
    SchemaSubject::Definition(def::<EncounterDefinition>(0x31d1).address())
}
fn owner_mut<'a>(i: &'a mut ReplayInput, subject: &SchemaSubject) -> &'a mut DefinitionRules {
    i.rules
        .owners
        .iter_mut()
        .find(|o| &o.owner == subject)
        .unwrap()
}
fn program_mut<'a>(i: &'a mut ReplayInput, name: &str) -> &'a mut RuleProgram {
    let rows: Vec<_> = i
        .rules
        .owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .filter(|p| p.id == key(name))
        .collect();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}
fn query_mut<'a>(i: &'a mut ReplayInput, name: &str) -> &'a mut ContributionQuery {
    let rows: Vec<_> = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .filter(|q| q.id == key(name))
        .collect();
    assert_eq!(rows.len(), 1);
    rows.into_iter().next().unwrap()
}
fn declare(i: &mut ReplayInput, d: DefinitionDescriptor) {
    let address = d.address();
    if let Some(current) = i.schema.definitions.iter().find(|v| v.address() == address) {
        assert_eq!(current, &d, "exact reused descriptor");
    } else {
        i.schema.definitions.push(d);
        i.rules.owners.push(DefinitionRules {
            owner: SchemaSubject::Definition(address),
            programs: DeclaredSet::complete(vec![]),
        });
    }
}
fn install(i: &mut ReplayInput, subject: SchemaSubject, p: RuleProgram, stage: &str) {
    assert!(
        !owner_mut(i, &subject)
            .programs
            .members
            .iter()
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
    owner_mut(i, &subject).programs.members.push(p);
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
fn assumption(i: &mut ReplayInput, n: u64, value: Option<ParameterValue>) {
    i.scenario.assumptions.retain(|a| a.input != def(n));
    if let Some(value) = value {
        i.scenario.assumptions.push(ExternalAssumption {
            input: def(n),
            target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
            value,
        });
    }
}
fn artifact(pin: &Value) -> Value {
    let path = family::root().join(pin["path"].as_str().unwrap());
    assert!(fs::metadata(&path).unwrap().len() <= 2 * 1024 * 1024);
    let bytes = fs::read(path).unwrap();
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(format!("{:x}", Sha256::digest(&bytes)), pin["sha256"]);
    serde_json::from_slice(&bytes).unwrap()
}
pub(crate) fn fixture() -> ReplayInput {
    let mut i = critical_native::fixture(Some(false), Some(20.));
    let old = i.rules.owners.clone();
    let d = data("dependencies.json");
    assert_eq!(d["scope"]["complete_mitigation"], false);
    assert_eq!(d["scope"]["source_supplier_complete"], false);
    let source = artifact(&d["source_authority"][0]);
    let rating = artifact(&source["existing_armour_producer"][0]);
    let rating_owners: Vec<DefinitionRules> =
        serde_json::from_value(rating["owners"].clone()).unwrap();
    let configured: Vec<_> = rating_owners
        .iter()
        .filter(|o| o.owner == encounter())
        .flat_map(|o| &o.programs.members)
        .filter(|p| p.id == key(ARMOUR))
        .collect();
    assert_eq!(configured.len(), 1);
    assert_eq!(
        configured[0],
        &serde_json::from_value::<RuleProgram>(d["configured_armour"]["program"].clone()).unwrap(),
        "retain the exact existing configured-Armour producer"
    );
    // The authored source spells whole quantities as JSON integers, whereas
    // release assembly serializes their typed f64 values with a decimal point.
    // Compare the entire domain object, including identity, units and rows.
    let rating_tables: Vec<IntegerRuleTable> =
        serde_json::from_value(rating["tables"].clone()).unwrap();
    let dependency_tables: Vec<IntegerRuleTable> =
        serde_json::from_value(d["tables"].clone()).unwrap();
    // The rating packet also owns Evasion; this component requires exactly the
    // one Armour table, independent of the dependency packet's declared list.
    let expected_tables: Vec<_> = rating_tables
        .into_iter()
        .filter(|t| t.id == key("monster-armour-by-level"))
        .collect();
    assert_eq!(expected_tables.len(), 1, "unique source Armour table");
    assert_eq!(
        dependency_tables, expected_tables,
        "exact required table inventory and typed contents"
    );
    let hit_migration = artifact(&d["physical_hit_artifacts"][0]);
    let hit: Consumer = serde_json::from_value(artifact(&d["physical_hit_artifacts"][1])).unwrap();
    assert_eq!(hit.owners[0].owner, owner());
    for v in hit_migration["schema"].as_array().unwrap() {
        assert_eq!(v["kind"], "definition");
        declare(&mut i, serde_json::from_value(v["value"].clone()).unwrap());
    }
    for v in data("definitions.json").as_array().unwrap() {
        declare(&mut i, serde_json::from_value(v.clone()).unwrap());
    }
    for v in d["definitions"].as_array().unwrap() {
        let descriptor: DefinitionDescriptor = serde_json::from_value(v.clone()).unwrap();
        if descriptor.address() != def::<EncounterDefinition>(0x31d1).address() {
            declare(&mut i, descriptor);
        }
    }
    let DefinitionDescriptor::Encounter(entry) = i
        .schema
        .definitions
        .iter_mut()
        .find(|v| v.address() == def::<EncounterDefinition>(0x31d1).address())
        .unwrap()
    else {
        unreachable!()
    };
    let SchemaState::Known(e) = &mut entry.schema else {
        unreachable!()
    };
    assert!(e.external_inputs.is_complete());
    for n in [0x31de, 0x31df] {
        assert!(!e.external_inputs.members.contains(&def(n)));
        e.external_inputs.members.push(def(n));
    }
    for (name, predecessors) in [
        (ARMOUR_STAGE, vec!["deliver"]),
        (
            HIT_STAGE,
            vec![
                "selected-physical-base-final",
                "action-minion-damage",
                "mixed-minion-damage",
            ],
        ),
        (
            FINAL,
            vec![ARMOUR_STAGE, HIT_STAGE, "ordinary-outgoing-critical"],
        ),
    ] {
        assert!(!i.stages.stages.iter().any(|s| s.id == key(name)));
        i.stages.stages.push(EvaluationStage {
            id: key(name),
            predecessors: predecessors.into_iter().map(key).collect(),
        });
    }
    for p in hit.owners[0].programs.members.clone() {
        install(&mut i, owner(), p, HIT_STAGE);
    }
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(hit.queries);
    for n in [0x339e, 0x339f] {
        freeze(
            &mut i,
            StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: def(n),
                contribution: ContributionKind::Flag,
            },
            "deliver",
        );
    }
    for (n, stage) in [
        (0x339c, "selected-physical-base-final"),
        (0x339d, "selected-physical-base-final"),
        (0x335c, "action-minion-damage"),
        (0x33a0, HIT_STAGE),
        (0x33a1, HIT_STAGE),
        (0x33ab, "ordinary-outgoing-critical"),
    ] {
        freeze(
            &mut i,
            StageChannel::Stat {
                scope: RuleEntityKind::Action,
                stat: def(n),
            },
            stage,
        );
    }
    let armour: RuleProgram =
        serde_json::from_value(d["configured_armour"]["program"].clone()).unwrap();
    assert_eq!(armour.id, key(ARMOUR));
    assert_eq!(
        serde_json::from_value::<SchemaSubject>(d["configured_armour"]["owner"].clone()).unwrap(),
        encounter()
    );
    install(&mut i, encounter(), armour, ARMOUR_STAGE);
    for v in d["tables"].as_array().unwrap() {
        let table: IntegerRuleTable = serde_json::from_value(v.clone()).unwrap();
        if let Some(existing) = i.rules.tables.iter().find(|t| t.id == table.id) {
            assert_eq!(existing, &table);
        } else {
            i.rules.tables.push(table);
        }
    }
    let c = consumer();
    assert_eq!(c.owners.len(), 1);
    assert_eq!(c.owners[0].programs.members.len(), 1);
    assert_eq!(c.owners[0].programs.members[0].id, key(PROGRAM));
    assert_eq!(c.queries.len(), 4);
    assert!(!c.owners[0].programs.is_complete());
    for p in c.owners[0].programs.members.clone() {
        install(&mut i, owner(), p, FINAL);
    }
    for mut q in c.queries {
        assert!(!q.groups[0].members.is_complete());
        if q.contribution == ContributionKind::Add {
            assert_eq!(q.groups[0].members.members.len(), 1);
            let ContributionProducer::ProgramEffect(p) = &q.groups[0].members.members[0].producer
            else {
                panic!("configured Armour must name its exact Encounter program")
            };
            assert_eq!(p.owner, encounter());
            assert_eq!(p.program, key(ARMOUR));
            assert_eq!(p.origin, ContributionOrigin::Encounter);
        } else {
            assert!(q.groups[0].members.members.is_empty());
        }
        q.groups[0].members.closure = SchemaClosure::Complete;
        freeze(
            &mut i,
            StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: q.stat.clone(),
                contribution: q.contribution,
            },
            ARMOUR_STAGE,
        );
        i.rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .push(q);
    }
    assumption(&mut i, 0x31de, Some(ParameterValue::Boolean(false)));
    assumption(&mut i, 0x31df, None);
    // These are independent, explicit fixture inputs. No observed Armour is used.
    i.scenario.enemy.level = 82;
    for gem in i
        .build
        .gems
        .iter_mut()
        .filter(|g| g.definition == def(0x11))
    {
        gem.quality.as_mut().unwrap().amount = FiniteQuantity::new(0., def(2)).unwrap();
    }
    for o in old {
        for p in o.programs.members {
            assert_eq!(
                owner_mut(&mut i, &o.owner)
                    .programs
                    .members
                    .iter()
                    .find(|v| v.id == p.id)
                    .unwrap(),
                &p
            );
        }
    }
    for writer in d["potential_writers"].as_array().unwrap() {
        let subject: SchemaSubject = serde_json::from_value(writer["owner"].clone()).unwrap();
        let expected: RuleProgram = serde_json::from_value(writer["program"].clone()).unwrap();
        let actual: Vec<_> = owner_mut(&mut i, &subject)
            .programs
            .members
            .iter()
            .filter(|p| p.id == expected.id)
            .collect();
        assert_eq!(
            actual,
            vec![&expected],
            "retain the exact upstream producer"
        );
    }
    i.rebind_test_edit().unwrap();
    i
}
fn values(i: &ReplayInput, r: &SupportEffectsReport) -> Vec<[f64; 2]> {
    critical_native::actions(i)
        .into_iter()
        .map(|a| {
            [0x33b2, 0x33b3].map(|n| {
                match critical_native::value(r, ConcreteEntity::Action(Box::new(a.clone())), n) {
                    EffectValue::Known {
                        value: ParameterValue::Quantity(q),
                    } => {
                        assert_eq!(q.unit(), &def(DAMAGE));
                        q.value()
                    }
                    other => panic!("exact armour endpoint {n:x} was {other:?}"),
                }
            })
        })
        .collect()
}
fn endpoints(i: &ReplayInput, normal: f64, critical: f64) {
    let report = critical_native::evaluate(i);
    for pair in values(i, &report) {
        for (actual, expected) in pair.into_iter().zip([normal, critical]) {
            assert!(
                (actual - expected).abs() <= 1e-10 * expected.abs().max(1.),
                "{actual} != {expected}"
            );
        }
    }
}
fn raw_armour(i: &mut ReplayInput, value: Option<f64>) {
    assumption(i, 0x31de, Some(ParameterValue::Boolean(true)));
    assumption(i, 0x31df, value.map(|v| quantity(v, RATING)));
}
fn synthetic_hit(i: &mut ReplayInput, min: f64, max: f64) {
    let p = program_mut(i, "selected-physical-hit");
    for (name, value) in [("rounded-min", min), ("rounded-max", max)] {
        p.nodes
            .iter_mut()
            .find(|n| n.id == key(name))
            .unwrap()
            .expression = RuleExpression::Literal {
            value: quantity(value, DAMAGE),
        };
    }
}
fn supplier(
    i: &mut ReplayInput,
    name: &str,
    kind: ContributionKind,
    amount: f64,
    declared: bool,
    inactive: bool,
    stage: &str,
) {
    let qname = match kind {
        ContributionKind::Add => "enemy-armour-add",
        ContributionKind::Increase => "enemy-armour-increase",
        ContributionKind::Multiply => "enemy-armour-multiply",
        ContributionKind::Override => "enemy-armour-override",
        _ => unreachable!(),
    };
    let q = query_mut(i, qname);
    if declared {
        let order = (kind != ContributionKind::Override).then(|| ContributionOrder {
            source_rank: 0,
            program_rank: 100 + q.groups[0].members.members.len() as u32,
            effect_rank: 0,
            slot_ranks: vec![],
        });
        q.groups[0].members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: encounter(),
                program: key(name),
                effect: key("value"),
                origin: ContributionOrigin::Encounter,
            }),
            order,
        });
    }
    let unit = match kind {
        ContributionKind::Increase => 2,
        ContributionKind::Multiply => 1,
        _ => RATING,
    };
    install(
        i,
        encounter(),
        RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: quantity(amount, unit),
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
                id: key("value"),
                when: inactive.then(|| key("inactive")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def(0x31e0),
                    contribution: kind,
                    value: key("value"),
                },
            }],
        },
        stage,
    );
}

#[test]
fn armour_actual_table_hit_and_critical_producers_reproduce_separate_source_averages() {
    let d = data("dependencies.json");
    assert_eq!(
        d["source_authority"][0]["sha256"],
        "d462af9bf8a253fa295e7a108145c3972181fbdcf7f1bbd15693391942570ef1"
    );
    assert_eq!(
        d["source_authority"][1]["sha256"],
        "fb675c15fe80240d1a5388d534ebdcb45154970f1ab1cc94e1af6fafae4934df"
    );
    artifact(&d["source_authority"][0]);
    let vectors = artifact(&d["source_authority"][1]);
    let rows: Vec<_> = vectors["rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["case"] == "original-05" && v["mode"] == "main")
        .collect();
    assert_eq!(rows.len(), 1);
    let i = fixture();
    let report = critical_native::evaluate(&i);
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite preparation")
    };
    let configured: Vec<_> = effects
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(ARMOUR))
        .collect();
    assert_eq!(configured.len(), 1);
    assert_eq!(
        configured[0].value,
        EffectValue::Known {
            value: quantity(8063., RATING)
        }
    );
    for pair in values(&i, &report) {
        for (actual, field) in pair
            .into_iter()
            .zip(["PhysicalHitAverage", "PhysicalCritAverage"])
        {
            let expected = rows[0]["output"][field].as_f64().unwrap();
            assert!((actual - expected).abs() < 1e-10);
        }
        assert_ne!(
            pair[1],
            pair[0] * 2.,
            "critical damage must meet armour independently"
        );
    }
    let actions = critical_native::actions(&i);
    assert_ne!(actions[0].action.actor, actions[1].action.actor);
    assert!(effects.values.iter().filter(|v|matches!(&v.key,PlanValueKey::Stat{stat,..}if *stat==def(0x33b2)||*stat==def(0x33b3))).all(|v|matches!(&v.key,PlanValueKey::Stat{entity:ConcreteEntity::Action(_),..})));
}

#[test]
fn armour_raw_zero_negative_and_level_inputs_are_executed_with_the_actual_producer() {
    for amount in [0., -500.] {
        let mut i = fixture();
        raw_armour(&mut i, Some(amount));
        endpoints(&i, 821., 1642.);
    }
    let mut i = fixture();
    i.scenario.enemy.level = 1;
    endpoints(&i, 414.2081976279727, 1101.2392272188865);
    let mut i = fixture();
    i.scenario.enemy.level = 85;
    let r = critical_native::evaluate(&i);
    assert!(
        values(&i, &r)
            .iter()
            .all(|v| v[0] < 414.2081976279727 && v[1] < 1101.2392272188865)
    );
    let mut i = fixture();
    raw_armour(&mut i, None);
    let r = critical_native::evaluate(&i);
    for a in critical_native::actions(&i) {
        for n in [0x33b2, 0x33b3] {
            assert!(matches!(
                critical_native::value(&r, ConcreteEntity::Action(Box::new(a.clone())), n),
                EffectValue::Unresolved {
                    reason: PlanGapReason::MissingInput,
                    ..
                }
            ));
        }
    }
    // Changing only the exact assumption target never makes Player a source.
    let mut i = fixture();
    raw_armour(&mut i, Some(0.));
    i.scenario
        .assumptions
        .iter_mut()
        .find(|a| a.input == def(0x31df))
        .unwrap()
        .target = AssumptionTarget::Actor(ActorKey::Player);
    assert!(i.compile().is_err());
}

#[test]
fn armour_caps_zero_denominator_negative_override_and_agreement_remain_distinct() {
    let mut i = fixture();
    raw_armour(&mut i, Some(1_000_000.));
    endpoints(&i, 205.25, 410.5);
    let mut i = fixture();
    raw_armour(&mut i, Some(0.));
    synthetic_hit(&mut i, 0., 0.);
    i.rebind_test_edit().unwrap();
    endpoints(&i, 0., 0.);
    let mut i = fixture();
    raw_armour(&mut i, None);
    supplier(
        &mut i,
        "synthetic.armour-zero-override",
        ContributionKind::Override,
        0.,
        true,
        false,
        ARMOUR_STAGE,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, 821., 1642.);
    let mut i = fixture();
    supplier(
        &mut i,
        "synthetic.negative-armour",
        ContributionKind::Override,
        -8210.,
        true,
        false,
        ARMOUR_STAGE,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, 1231.5, 2189.333333333333);
    supplier(
        &mut i,
        "synthetic.agreeing-armour",
        ContributionKind::Override,
        -8210.,
        true,
        false,
        ARMOUR_STAGE,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, 1231.5, 2189.333333333333);
    supplier(
        &mut i,
        "synthetic.conflicting-armour",
        ContributionKind::Override,
        -1.,
        true,
        false,
        ARMOUR_STAGE,
    );
    i.rebind_test_edit().unwrap();
    let r = critical_native::evaluate(&i);
    for a in critical_native::actions(&i) {
        assert!(matches!(
            critical_native::value(&r, ConcreteEntity::Action(Box::new(a)), 0x33b2),
            EffectValue::Unresolved {
                reason: PlanGapReason::ConflictingContributors,
                ..
            }
        ));
    }
}

#[test]
fn armour_increase_more_and_critical_endpoint_order_are_native_synthetic_controls() {
    let mut i = fixture();
    raw_armour(&mut i, Some(100.));
    synthetic_hit(&mut i, 10., 30.);
    supplier(
        &mut i,
        "synthetic.armour-increase",
        ContributionKind::Increase,
        100.,
        true,
        false,
        ARMOUR_STAGE,
    );
    supplier(
        &mut i,
        "synthetic.armour-more",
        ContributionKind::Multiply,
        2.,
        true,
        false,
        ARMOUR_STAGE,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, 6.666666666666668, 20.);
    let mut i = fixture();
    synthetic_hit(&mut i, -1., 30.);
    i.rebind_test_edit().unwrap();
    let r = critical_native::evaluate(&i);
    for a in critical_native::actions(&i) {
        assert!(matches!(
            critical_native::value(&r, ConcreteEntity::Action(Box::new(a)), 0x33b2),
            EffectValue::UnsupportedDomain { .. }
        ));
    }
}

#[test]
fn armour_partial_membership_unlisted_zero_inactive_and_late_sources_refuse() {
    for which in 0..5 {
        let mut i = fixture();
        let c = consumer();
        if which == 0 {
            owner_mut(&mut i, &owner()).programs.closure = c.owners[0].programs.closure.clone();
        } else {
            let q = &c.queries[which - 1];
            query_mut(&mut i, q.id.as_str()).groups[0].members.closure =
                q.groups[0].members.closure.clone();
        }
        i.rebind_test_edit().unwrap();
        let r = critical_native::evaluate(&i);
        assert!(
            !r.gaps.is_empty()
                || match r.outcome {
                    SupportEffectsOutcome::Evaluated { effects } => !effects.gaps.is_empty(),
                    _ => true,
                }
        );
    }
    for (declared, inactive, stage) in [
        (false, false, ARMOUR_STAGE),
        (false, true, ARMOUR_STAGE),
        (true, true, FINAL),
    ] {
        let mut i = fixture();
        supplier(
            &mut i,
            "synthetic.unadmitted-armour",
            ContributionKind::Add,
            0.,
            declared,
            inactive,
            stage,
        );
        assert!(
            i.rebind_test_edit()
                .and_then(|_| i.compile().map(|_| ()))
                .is_err()
        );
    }
    let mut i = fixture();
    // A channel can freeze at its consumer's stage: the graph orders the exact
    // producers before the read. A strictly later freeze cannot satisfy it.
    let frozen = i
        .stages
        .frozen_channels
        .iter_mut()
        .find(|f| {
            matches!(&f.channel, StageChannel::Contributions {
            stat, contribution: ContributionKind::Add, ..
        } if *stat == def(0x31e0))
        })
        .unwrap();
    frozen.stage = key(FINAL);
    i.rebind_test_edit().unwrap();
    endpoints(&i, 414.2081976279727, 1101.2392272188865);
    let later = key("synthetic.after-armour-consumer");
    i.stages.stages.push(EvaluationStage {
        id: later.clone(),
        predecessors: vec![key(FINAL)],
    });
    i.stages
        .frozen_channels
        .iter_mut()
        .find(|f| {
            matches!(&f.channel, StageChannel::Contributions {
            stat, contribution: ContributionKind::Add, ..
        } if *stat == def(0x31e0))
        })
        .unwrap()
        .stage = later;
    assert!(
        i.rebind_test_edit()
            .and_then(|_| i.compile().map(|_| ()))
            .is_err()
    );
}

#[test]
fn armour_exact_reports_reuse_and_parallel_workers_preserve_changed_input_identity() {
    let i = fixture();
    let p = i.compile().unwrap();
    let expected = p.evaluate(&mut p.new_scratch()).unwrap();
    let mut scratch = p.new_scratch();
    for _ in 0..3 {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    }
    let reports: Vec<_> = (0..4)
        .into_par_iter()
        .map(|_| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|r| r == &expected));
    let mut changed = i.clone();
    raw_armour(&mut changed, Some(0.));
    let q = changed.compile().unwrap();
    let actual = q.evaluate(&mut scratch).unwrap();
    assert_eq!(actual, q.evaluate(&mut q.new_scratch()).unwrap());
    assert_ne!(actual, expected);
    assert_eq!(values(&changed, &actual), vec![[821., 1642.]; 2]);
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
}
