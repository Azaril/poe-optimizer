//! The two authored programs execute through the public staged plan. Existing
//! intrinsic/accuracy routes and item-driven supplied Actor inputs are retained.
//! Synthetic condition/override contributors express test cases, not game facts.
use super::{family, replay::ReplayInput};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::Value;
use std::fs;

const CONTROLS: &str = "synthetic-critical-inputs";
const WEAKNESS: &str = "configured-critical-weakness";
const FINAL: &str = "ordinary-outgoing-critical";
const ENABLED: &str = "synthetic.critical-weakness-enabled";
const ENABLE_PROGRAM: &str = "synthetic.critical-weakness-condition";

fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn named<K: DefinitionDomain>(s: &str) -> DefId<K> {
    DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        s,
    )
    .unwrap()
}
fn def<K: DefinitionDomain>(n: u64) -> DefId<K> {
    named(&format!("def.{n:016x}"))
}
fn quantity(n: f64, unit: u64) -> ParameterValue {
    ParameterValue::Quantity(FiniteQuantity::new(n, def(unit)).unwrap())
}
fn actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(0x12)),
        slot: def(0x1f),
    }
}
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}
fn action_owner() -> SchemaSubject {
    family::consumer().owners[0].owner.clone()
}
fn encounter_owner() -> SchemaSubject {
    SchemaSubject::Definition(def::<EncounterDefinition>(0x31d1).address())
}
fn declare(i: &mut ReplayInput, descriptor: DefinitionDescriptor) {
    let address = descriptor.address();
    assert!(!i.schema.definitions.iter().any(|d| d.address() == address));
    i.schema.definitions.push(descriptor);
    i.rules.owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(address),
        programs: DeclaredSet::complete(vec![]),
    });
}
fn owner_mut<'a>(i: &'a mut ReplayInput, owner: &SchemaSubject) -> &'a mut DefinitionRules {
    i.rules
        .owners
        .iter_mut()
        .find(|o| &o.owner == owner)
        .unwrap()
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
fn query_mut<'a>(i: &'a mut ReplayInput, name: &str) -> &'a mut ContributionQuery {
    let matches: Vec<_> = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .filter(|q| q.id == key(name))
        .collect();
    assert_eq!(matches.len(), 1);
    matches.into_iter().next().unwrap()
}
fn install(i: &mut ReplayInput, owner: SchemaSubject, program: RuleProgram, stage: &str) {
    assert!(
        !owner_mut(i, &owner)
            .programs
            .members
            .iter()
            .any(|p| p.id == program.id)
    );
    i.stages.programs.members.push(StagedRuleProgram {
        owner: owner.clone(),
        program: program.id.clone(),
        stage: key(stage),
    });
    i.stages
        .readiness
        .as_mut()
        .unwrap()
        .programs
        .members
        .push(ReadinessProgram {
            owner: owner.clone(),
            program: program.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    owner_mut(i, &owner).programs.members.push(program);
}
fn freeze(i: &mut ReplayInput, channel: StageChannel, stage: &str) {
    assert!(
        !i.stages
            .frozen_channels
            .iter()
            .any(|f| f.channel == channel)
    );
    i.stages.frozen_channels.push(FrozenStageChannel {
        channel,
        stage: key(stage),
    });
}
fn assumption(i: &mut ReplayInput, input: ExternalInputDefId, value: Option<ParameterValue>) {
    i.scenario.assumptions.retain(|a| a.input != input);
    if let Some(value) = value {
        i.scenario.assumptions.push(ExternalAssumption {
            input,
            target: AssumptionTarget::Actor(ActorKey::ScenarioEnemy),
            value,
        });
    }
}
fn configured_member() -> ContributionMember {
    ContributionMember {
        producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
            owner: encounter_owner(),
            program: key(WEAKNESS),
            effect: key("critical-weakness"),
            origin: ContributionOrigin::Encounter,
        }),
        order: Some(ContributionOrder {
            source_rank: 0,
            program_rank: 0,
            effect_rank: 0,
            slot_ranks: vec![],
        }),
    }
}

pub(crate) fn fixture(enabled: Option<bool>, stacks: Option<f64>) -> ReplayInput {
    let path = family::root().join("tests/fixtures/owned-sniper-replay.json.gz");
    assert!(fs::metadata(&path).unwrap().len() < 256 * 1024);
    let mut i = ReplayInput::decode(&fs::read(path).unwrap());
    assert_eq!(i.scenario.enemy.encounter, def(0x31d1));
    let original_rules = i.rules.clone();
    for d in serde_json::from_value::<Vec<DefinitionDescriptor>>(family::read("definitions.json"))
        .unwrap()
    {
        declare(&mut i, d);
    }
    // Reuse the actual imported Count input's current role schema. Do not copy
    // the broad Encounter inventory into this controlled finite replay.
    let extension: Value = serde_json::from_slice(
        &fs::read(family::root().join(
            "data/owned/poe2/3887ae68/configuration-enemy-critical-weakness-input/extension.json",
        ))
        .unwrap(),
    )
    .unwrap();
    let descriptor: DefinitionDescriptor = serde_json::from_value(
        extension["schema"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["value"]["kind"] == "external_input")
            .unwrap()["value"]
            .clone(),
    )
    .unwrap();
    assert_eq!(
        descriptor.address(),
        def::<ExternalInputDefinition>(0x3390).address()
    );
    declare(&mut i, descriptor);
    declare(
        &mut i,
        DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: named(ENABLED),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: vec![AssumptionTargetKind::ScenarioEnemyActor],
            }),
        }),
    );
    let DefinitionDescriptor::Encounter(entry) = i
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
    assert!(schema.external_inputs.is_complete());
    schema
        .external_inputs
        .members
        .extend([def(0x3390), named(ENABLED)]);

    let baseline = family::read("baseline.json");
    family::review_baseline(&baseline).unwrap();
    let before: RuleProgram = serde_json::from_value(baseline["before"].clone()).unwrap();
    let after: RuleProgram = serde_json::from_value(baseline["after"].clone()).unwrap();
    assert_eq!(
        program_mut(&mut i, "intrinsic-minion-attack-source"),
        &before
    );
    *program_mut(&mut i, "intrinsic-minion-attack-source") = after;
    for (id, predecessor) in [
        (CONTROLS, "deliver"),
        (WEAKNESS, CONTROLS),
        (FINAL, WEAKNESS),
    ] {
        i.stages.stages.push(EvaluationStage {
            id: key(id),
            predecessors: vec![key(predecessor)],
        });
    }
    let consumer = family::consumer();
    assert_eq!(consumer.owners.len(), 2);
    assert_eq!(consumer.queries.len(), 16);
    for owner in &consumer.owners {
        assert!(
            !owner.programs.is_complete(),
            "production owner remains Partial"
        );
        for program in &owner.programs.members {
            let stage = if program.id == key(WEAKNESS) {
                WEAKNESS
            } else {
                assert_eq!(program.id, key(FINAL));
                FINAL
            };
            install(&mut i, owner.owner.clone(), program.clone(), stage);
        }
    }
    for mut q in consumer.queries {
        assert_eq!(q.groups.len(), 1);
        assert!(!q.groups[0].members.is_complete());
        if q.id == key("outgoing-critical-enemy-chance-add") {
            assert_eq!(
                q.groups[0].members.members,
                vec![configured_member()],
                "actual authored Encounter membership is required"
            );
        }
        // This finite fixture declares its own exact sources. The production
        // closures above are retained and exercised separately below.
        q.groups[0].members.closure = SchemaClosure::Complete;
        let d = i
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == q.stat.address())
            .unwrap();
        let DefinitionDescriptor::Stat(DefinitionEntry {
            schema: SchemaState::Known(schema),
            ..
        }) = d
        else {
            unreachable!()
        };
        let scopes = schema.targets.clone();
        for scope in scopes {
            freeze(
                &mut i,
                StageChannel::Contributions {
                    scope,
                    stat: q.stat.clone(),
                    contribution: q.contribution,
                },
                if q.id == key("outgoing-critical-enemy-chance-add") {
                    WEAKNESS
                } else {
                    CONTROLS
                },
            );
        }
        i.rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .push(q);
    }
    for n in [0x3215, 0x321c] {
        let channel = StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: def(n),
        };
        if !i
            .stages
            .frozen_channels
            .iter()
            .any(|f| f.channel == channel)
        {
            freeze(&mut i, channel, "deliver");
        }
    }
    install(
        &mut i,
        encounter_owner(),
        RuleProgram {
            id: key(ENABLE_PROGRAM),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: key("enabled"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::External {
                    entity: RuleEntity::Current,
                    input: named(ENABLED),
                },
            }],
            nodes: vec![RuleNode {
                id: key("enabled"),
                expression: RuleExpression::Read {
                    input: key("enabled"),
                },
            }],
            effects: vec![RuleEffect {
                id: key("enabled"),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def(0x33b0),
                    contribution: ContributionKind::Flag,
                    value: key("enabled"),
                },
            }],
        },
        CONTROLS,
    );
    query_mut(&mut i, "outgoing-critical-weakness-enabled").groups[0]
        .members
        .members
        .push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: encounter_owner(),
                program: key(ENABLE_PROGRAM),
                effect: key("enabled"),
                origin: ContributionOrigin::Encounter,
            }),
            order: None,
        });
    assumption(&mut i, named(ENABLED), enabled.map(ParameterValue::Boolean));
    assumption(&mut i, def(0x3390), stacks.map(|n| quantity(n, 0x295a)));

    // Only the exact reviewed baseline append changes a preexisting producer.
    for owner in original_rules.owners {
        for p in owner.programs.members {
            let actual = owner_mut(&mut i, &owner.owner)
                .programs
                .members
                .iter()
                .find(|v| v.id == p.id)
                .unwrap();
            if p.id == key("intrinsic-minion-attack-source") {
                assert_eq!(
                    actual,
                    &serde_json::from_value::<RuleProgram>(baseline["after"].clone()).unwrap()
                );
            } else {
                assert_eq!(actual, &p);
            }
        }
    }
    i.rebind_test_edit().unwrap();
    i
}

fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    assert!(report.gaps.is_empty(), "support gaps: {:?}", report.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite support preparation refused")
    };
    effects
}
pub(crate) fn evaluate(i: &ReplayInput) -> SupportEffectsReport {
    let plan = i.compile().unwrap();
    plan.evaluate(&mut plan.new_scratch()).unwrap()
}
pub(crate) fn actions(i: &ReplayInput) -> Vec<ActionSelection> {
    let result: Vec<_> = i
        .queries
        .requests
        .iter()
        .filter_map(|q| match &q.target {
            MetricTarget::Action(a) if a.action.output.slot == def(0x22) => {
                Some(a.as_ref().clone())
            }
            _ => None,
        })
        .collect();
    assert_eq!(result.len(), 2);
    assert_ne!(result[0].action.actor, result[1].action.actor);
    result
}
pub(crate) fn value(
    report: &SupportEffectsReport,
    entity: ConcreteEntity,
    stat: u64,
) -> &EffectValue {
    &effects(report)
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: entity.clone(),
                    stat: def(stat),
                }
        })
        .expect("demanded exact endpoint")
        .value
}
fn known(actual: &EffectValue, expected: f64, unit: u64) {
    assert_eq!(
        actual,
        &EffectValue::Known {
            value: quantity(expected, unit)
        }
    );
}
fn endpoints(i: &ReplayInput, report: &SupportEffectsReport, chance: f64, multiplier: f64) {
    assert!(
        effects(report).gaps.is_empty(),
        "effect gaps: {:?}",
        effects(report).gaps
    );
    for action in actions(i) {
        known(
            value(
                report,
                ConcreteEntity::Action(Box::new(action.clone())),
                0x33aa,
            ),
            chance,
            2,
        );
        known(
            value(report, ConcreteEntity::Action(Box::new(action)), 0x33ab),
            multiplier,
            1,
        );
    }
}
fn chance_unresolved(i: &ReplayInput, report: &SupportEffectsReport, reason: PlanGapReason) {
    for action in actions(i) {
        assert!(
            matches!(value(report, ConcreteEntity::Action(Box::new(action.clone())), 0x33aa),
            EffectValue::Unresolved { reason: actual, .. } if *actual == reason)
        );
        known(
            value(report, ConcreteEntity::Action(Box::new(action)), 0x33ab),
            2.,
            1,
        );
    }
}

#[derive(Clone, Copy)]
enum Lane {
    Action,
    Actor,
    Enemy,
    Player,
}
fn lane(i: &ReplayInput, target: Lane) -> (SchemaSubject, RuleEntityKind, ContributionOrigin) {
    match target {
        Lane::Action => (
            action_owner(),
            RuleEntityKind::Action,
            ContributionOrigin::Action {
                authored: false,
                supplies: vec![DeclaredSlot {
                    declaration: SlotOwnerDefId::Actor(def(0x3091)),
                    slot: def(0x3092),
                }],
            },
        ),
        Lane::Actor => (
            actor_owner(),
            RuleEntityKind::Actor,
            ContributionOrigin::SuppliedActor {
                slots: vec![actor_slot()],
            },
        ),
        Lane::Enemy => (
            encounter_owner(),
            RuleEntityKind::Actor,
            ContributionOrigin::Encounter,
        ),
        Lane::Player => (
            SchemaSubject::Definition(i.build.character.class.address()),
            RuleEntityKind::Actor,
            ContributionOrigin::Character,
        ),
    }
}
struct Delivery<'a> {
    inactive: bool,
    declared: bool,
    stage: &'a str,
}
fn contribution(
    i: &mut ReplayInput,
    name: &str,
    query: &str,
    target: Lane,
    amount: f64,
    delivery: Delivery<'_>,
) {
    let (owner, context, origin) = lane(i, target);
    let q = query_mut(i, query);
    let stat = q.stat.clone();
    let kind = q.contribution;
    let order =
        (q.groups[0].ordering == ContributionOrdering::Ordered).then(|| ContributionOrder {
            source_rank: if matches!(target, Lane::Action | Lane::Player) {
                1
            } else {
                0
            },
            program_rank: 100 + q.groups[0].members.members.len() as u32,
            effect_rank: 0,
            slot_ranks: vec![],
        });
    if delivery.declared {
        q.groups[0].members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: owner.clone(),
                program: key(name),
                effect: key("value"),
                origin,
            }),
            order,
        });
    }
    install(
        i,
        owner,
        RuleProgram {
            id: key(name),
            context,
            reads: vec![],
            nodes: vec![
                RuleNode {
                    id: key("value"),
                    expression: RuleExpression::Literal {
                        value: quantity(
                            amount,
                            if kind == ContributionKind::Multiply {
                                1
                            } else {
                                2
                            },
                        ),
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
                when: delivery.inactive.then(|| key("inactive")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat,
                    contribution: kind,
                    value: key("value"),
                },
            }],
        },
        delivery.stage,
    );
}
fn override_value(i: &mut ReplayInput, name: &str, query: &str, target: Lane, amount: f64) {
    contribution(
        i,
        name,
        query,
        target,
        amount,
        Delivery {
            inactive: false,
            declared: true,
            stage: CONTROLS,
        },
    );
}
fn missing_read(i: &mut ReplayInput, program: &str, read: &str) {
    let missing: StatDefId = named(&format!("synthetic.missing-{read}"));
    let p = program_mut(i, program);
    let r = p.reads.iter_mut().find(|r| r.id == key(read)).unwrap();
    let value = r.value_type.clone();
    let scope = p.context;
    r.source = RuleReadSource::Stat {
        entity: RuleEntity::Current,
        stat: missing.clone(),
    };
    declare(
        i,
        DefinitionDescriptor::Stat(DefinitionEntry {
            id: missing.clone(),
            schema: SchemaState::Known(StatSchema {
                value,
                targets: vec![scope],
            }),
        }),
    );
    freeze(
        i,
        StageChannel::Stat {
            scope,
            stat: missing,
        },
        CONTROLS,
    );
}

#[test]
fn real_intrinsic_accuracy_and_encounter_programs_execute_on_exact_recipients() {
    let i = fixture(Some(false), Some(20.));
    let r = evaluate(&i);
    endpoints(&i, &r, 5., 2.);
    let es = effects(&r);
    let weakness: Vec<_> = es
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(WEAKNESS))
        .collect();
    assert_eq!(weakness.len(), 1);
    assert_eq!(weakness[0].key.invocation.origin, RuleOrigin::Encounter);
    assert_eq!(
        weakness[0].key.invocation.entity,
        ConcreteEntity::Actor(ActorKey::ScenarioEnemy)
    );
    assert_eq!(weakness[0].value, EffectValue::Inactive);
    for a in actions(&i) {
        known(
            value(&r, ConcreteEntity::Action(Box::new(a.clone())), 0x3215),
            5.,
            2,
        );
        known(
            value(&r, ConcreteEntity::Action(Box::new(a)), 0x321c),
            100.,
            2,
        );
    }
    let baseline: Vec<_> = es
        .effects
        .iter()
        .filter(|e| {
            e.key.invocation.program == key("intrinsic-minion-attack-source")
                && [key("critical-chance-cap"), key("critical-damage-bonus")]
                    .contains(&e.key.effect)
        })
        .collect();
    assert_eq!(baseline.len(), 4);
    assert!(baseline.iter().all(|e| matches!(
        &e.key.invocation.entity,
        ConcreteEntity::Actor(ActorKey::Owned(_))
    )));
    assert!(!es.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { entity: ConcreteEntity::Actor(_), stat } if *stat == def(0x33aa) || *stat == def(0x33ab))));
}

#[test]
fn configured_weakness_retains_false_unknown_zero_and_clamped_raw_inputs() {
    for (raw, expected, inactive) in [
        (-10., 5., false),
        (0., 5., true),
        (1., 5.5, false),
        (20., 15., false),
        (80., 15., false),
    ] {
        let i = fixture(Some(true), Some(raw));
        let r = evaluate(&i);
        endpoints(&i, &r, expected, 2.);
        let e = effects(&r)
            .effects
            .iter()
            .find(|e| e.key.invocation.program == key(WEAKNESS))
            .unwrap();
        if inactive {
            assert_eq!(e.value, EffectValue::Inactive);
        } else {
            known(&e.value, expected - 5., 2);
        }
    }
    let i = fixture(None, Some(20.));
    chance_unresolved(&i, &evaluate(&i), PlanGapReason::MissingInput);
    let i = fixture(Some(true), None);
    chance_unresolved(&i, &evaluate(&i), PlanGapReason::MissingInput);
    let i = fixture(Some(false), None);
    endpoints(&i, &evaluate(&i), 5., 2.);
}

#[test]
fn current_override_precedence_and_diamond_hundred_bypass_are_lazy() {
    let mut i = fixture(None, None);
    missing_read(&mut i, FINAL, "intrinsic-chance");
    missing_read(&mut i, FINAL, "accuracy-confirmation");
    override_value(
        &mut i,
        "synthetic.actor-diamond",
        "outgoing-critical-chance-override",
        Lane::Actor,
        100.,
    );
    i.rebind_test_edit().unwrap();
    // Unknown ordinary operands still exist, but the exact authored 100 branch
    // does not demand them. This does not close production Diamond coverage.
    let report = evaluate(&i);
    for a in actions(&i) {
        known(
            value(&report, ConcreteEntity::Action(Box::new(a)), 0x33aa),
            100.,
            2,
        );
    }

    let mut i = fixture(Some(true), Some(20.));
    override_value(
        &mut i,
        "synthetic.actor-override",
        "outgoing-critical-chance-override",
        Lane::Actor,
        100.,
    );
    override_value(
        &mut i,
        "synthetic.action-override",
        "outgoing-critical-chance-override",
        Lane::Action,
        40.,
    );
    override_value(
        &mut i,
        "synthetic.base-override",
        "outgoing-critical-base-override",
        Lane::Action,
        12.,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, &evaluate(&i), 12., 2.);
    override_value(
        &mut i,
        "synthetic.conflicting-action",
        "outgoing-critical-chance-override",
        Lane::Action,
        41.,
    );
    i.rebind_test_edit().unwrap();
    chance_unresolved(&i, &evaluate(&i), PlanGapReason::ConflictingContributors);
}

#[test]
fn nonpositive_base_caps_and_present_zero_override_preserve_authored_order() {
    for (base, cap, expected) in [
        (0., 100., 0.),
        (-2., 100., -2.),
        (5., -1., 0.),
        (-2., -3., -3.),
    ] {
        let mut i = fixture(Some(false), Some(20.));
        override_value(
            &mut i,
            "synthetic.base",
            "outgoing-critical-base-override",
            Lane::Action,
            base,
        );
        override_value(
            &mut i,
            "synthetic.cap",
            "outgoing-critical-cap-override",
            Lane::Actor,
            cap,
        );
        i.rebind_test_edit().unwrap();
        endpoints(&i, &evaluate(&i), expected, 2.);
    }
    let mut i = fixture(Some(true), Some(20.));
    override_value(
        &mut i,
        "synthetic.zero-chance",
        "outgoing-critical-chance-override",
        Lane::Action,
        0.,
    );
    override_value(
        &mut i,
        "synthetic.zero-bonus",
        "outgoing-critical-bonus-override",
        Lane::Actor,
        0.,
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, &evaluate(&i), 0., 1.);
}

#[test]
fn missing_intrinsic_and_accuracy_operands_are_not_replaced_by_observed_values() {
    for read in ["intrinsic-chance", "accuracy-confirmation"] {
        let mut i = fixture(Some(false), Some(20.));
        missing_read(&mut i, FINAL, read);
        i.rebind_test_edit().unwrap();
        chance_unresolved(&i, &evaluate(&i), PlanGapReason::MissingProducer);
    }
    let mut i = fixture(Some(false), Some(20.));
    override_value(
        &mut i,
        "synthetic.unknown-override",
        "outgoing-critical-chance-override",
        Lane::Action,
        0.,
    );
    let missing: StatDefId = named("synthetic.unknown-override-value");
    declare(
        &mut i,
        DefinitionDescriptor::Stat(DefinitionEntry {
            id: missing.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Quantity { unit: def(2) },
                targets: vec![RuleEntityKind::Action],
            }),
        }),
    );
    let p = program_mut(&mut i, "synthetic.unknown-override");
    p.reads.push(RuleRead {
        id: key("unknown"),
        value_type: ComputedValueType::Quantity { unit: def(2) },
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: missing.clone(),
        },
    });
    p.nodes[0].expression = RuleExpression::Read {
        input: key("unknown"),
    };
    freeze(
        &mut i,
        StageChannel::Stat {
            scope: RuleEntityKind::Action,
            stat: missing,
        },
        "deliver",
    );
    i.rebind_test_edit().unwrap();
    chance_unresolved(&i, &evaluate(&i), PlanGapReason::MissingProducer);
}

#[test]
fn actor_action_player_and_enemy_contributors_keep_distinct_recipients() {
    let mut i = fixture(Some(false), Some(20.));
    contribution(
        &mut i,
        "synthetic.player-chance",
        "outgoing-critical-chance-add",
        Lane::Player,
        50.,
        Delivery {
            inactive: false,
            declared: true,
            stage: CONTROLS,
        },
    );
    contribution(
        &mut i,
        "synthetic.actor-chance",
        "outgoing-critical-chance-add",
        Lane::Actor,
        2.,
        Delivery {
            inactive: false,
            declared: true,
            stage: CONTROLS,
        },
    );
    contribution(
        &mut i,
        "synthetic.action-chance",
        "outgoing-critical-chance-add",
        Lane::Action,
        3.,
        Delivery {
            inactive: false,
            declared: true,
            stage: CONTROLS,
        },
    );
    contribution(
        &mut i,
        "synthetic.enemy-chance",
        "outgoing-critical-enemy-chance-add",
        Lane::Enemy,
        4.,
        Delivery {
            inactive: false,
            declared: true,
            stage: WEAKNESS,
        },
    );
    i.rebind_test_edit().unwrap();
    endpoints(&i, &evaluate(&i), 14., 2.);
    let mut wrong = i.clone();
    program_mut(&mut wrong, WEAKNESS).effects[0].effect = RuleEffectKind::Contribute {
        entity: RuleEntity::Player,
        stat: def(0x33ae),
        contribution: ContributionKind::Add,
        value: key("self-critical-chance"),
    };
    assert!(
        wrong.rebind_test_edit().is_err(),
        "Encounter cannot acquire Player delivery"
    );
    let mut wrong = i.clone();
    wrong
        .scenario
        .assumptions
        .iter_mut()
        .find(|a| a.input == def(0x3390))
        .unwrap()
        .target = AssumptionTarget::Actor(ActorKey::Player);
    assert!(
        wrong.compile().is_err(),
        "Enemy-only raw input cannot be assigned to Player"
    );
}

#[test]
fn production_partial_owners_and_memberships_refuse_complete_execution() {
    for which in 0..4 {
        let mut i = fixture(Some(false), Some(20.));
        let original = family::consumer();
        if which < 2 {
            owner_mut(&mut i, &original.owners[which].owner)
                .programs
                .closure = original.owners[which].programs.closure.clone();
        } else {
            let name = if which == 2 {
                "outgoing-critical-weakness-enabled"
            } else {
                "outgoing-critical-enemy-chance-add"
            };
            query_mut(&mut i, name).groups[0].members.closure = original
                .queries
                .iter()
                .find(|q| q.id == key(name))
                .unwrap()
                .groups[0]
                .members
                .closure
                .clone();
        }
        i.rebind_test_edit().unwrap();
        let r = evaluate(&i);
        assert!(
            !r.gaps.is_empty()
                || match r.outcome {
                    SupportEffectsOutcome::Evaluated { effects } => !effects.gaps.is_empty(),
                    _ => true,
                },
            "production Partial is not finite test authority"
        );
    }
}

#[test]
fn zero_inactive_unlisted_and_late_potential_sources_cannot_evade_membership_or_freeze() {
    for (declared, inactive, stage) in [
        (false, false, CONTROLS),
        (false, true, CONTROLS),
        (true, true, FINAL),
    ] {
        let mut i = fixture(Some(false), Some(20.));
        contribution(
            &mut i,
            "synthetic.zero",
            "outgoing-critical-chance-add",
            Lane::Action,
            0.,
            Delivery {
                inactive,
                declared,
                stage,
            },
        );
        let error = i.rebind_test_edit().and_then(|_| i.compile().map(|_| ()));
        assert!(
            error.is_err(),
            "potential source must be admitted before its freeze"
        );
    }
    let mut i = fixture(Some(false), Some(20.));
    let frozen = i.stages.frozen_channels.iter_mut().find(|f| matches!(&f.channel,
        StageChannel::Contributions { stat, contribution: ContributionKind::Flag, .. } if *stat == def(0x33b0))).unwrap();
    frozen.stage = key(FINAL);
    assert!(
        i.rebind_test_edit()
            .and_then(|_| i.compile().map(|_| ()))
            .is_err(),
        "Weakness must not read a later condition freeze"
    );
}

#[test]
fn exact_reports_survive_reuse_parallel_workers_and_input_identity_changes() {
    let i = fixture(Some(false), Some(20.));
    let plan = i.compile().unwrap();
    let expected = plan.evaluate(&mut plan.new_scratch()).unwrap();
    endpoints(&i, &expected, 5., 2.);
    let mut scratch = plan.new_scratch();
    for _ in 0..3 {
        assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
    }
    let reports: Vec<_> = (0..4)
        .into_par_iter()
        .map(|_| plan.evaluate(&mut plan.new_scratch()).unwrap())
        .collect();
    assert!(reports.iter().all(|r| r == &expected));
    let mut changed = i.clone();
    assumption(
        &mut changed,
        named(ENABLED),
        Some(ParameterValue::Boolean(true)),
    );
    let changed_plan = changed.compile().unwrap();
    let changed_report = changed_plan.evaluate(&mut scratch).unwrap();
    assert_eq!(
        changed_report,
        changed_plan
            .evaluate(&mut changed_plan.new_scratch())
            .unwrap()
    );
    endpoints(&changed, &changed_report, 15., 2.);
    assert_ne!(
        effects(&expected).identity,
        effects(&changed_report).identity
    );
    assert_eq!(plan.evaluate(&mut scratch).unwrap(), expected);
}
