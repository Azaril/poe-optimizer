//! No second evaluator: install the authored consumer into the current public
//! component replay, then use normal compilation, scratch and evaluation.
use super::{family, physical_base_family, replay::ReplayInput};
use poe_optimizer_core::{
    owned_build::*, owned_definitions::*, owned_readiness::*, owned_rules::*, owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
use rayon::prelude::*;
use std::fs;

const FINAL: &str = "selected-physical-hit-final";
const CONTROL: &str = "synthetic.selected-actor-condition";
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
fn actor_slot() -> DeclaredSlot<ActorSlotDefId> {
    DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(0x12)),
        slot: def(0x1f),
    }
}
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}
fn install(i: &mut ReplayInput, owner: SchemaSubject, p: RuleProgram, stage: &str) {
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
            owner: owner.clone(),
            program: p.id.clone(),
            phase: ReadinessPhase::Execution,
            role: ReadinessProgramRole::Execution,
            outputs: vec![],
        });
    i.rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owner)
        .unwrap()
        .programs
        .members
        .push(p);
}
fn fixture() -> ReplayInput {
    let mut i = ReplayInput::decode(
        &fs::read(family::root().join("tests/fixtures/owned-sniper-replay.json.gz")).unwrap(),
    );
    let base = physical_base_family::consumer();
    for p in &base.owners[0].programs.members {
        assert_eq!(
            i.rules
                .owners
                .iter()
                .find(|o| o.owner == owner())
                .unwrap()
                .programs
                .members
                .iter()
                .filter(|v| v.id == p.id)
                .collect::<Vec<_>>(),
            vec![p]
        );
    }
    for entry in family::migration().schema {
        let SchemaExtensionEntry::Definition(d) = entry else {
            panic!("Stat only")
        };
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
    i.stages.stages.push(EvaluationStage {
        id: key(FINAL),
        predecessors: vec![
            key("selected-physical-base-final"),
            key("action-minion-damage"),
            key("mixed-minion-damage"),
        ],
    });
    let c = family::consumer();
    for p in c.owners[0].programs.members.clone() {
        install(&mut i, owner(), p, FINAL);
    }
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    for n in family::CONDITIONS {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: def(n),
                contribution: ContributionKind::Flag,
            },
            stage: key("deliver"),
        });
    }
    for (n, scope, stage) in [
        (
            0x339c,
            RuleEntityKind::Action,
            "selected-physical-base-final",
        ),
        (
            0x339d,
            RuleEntityKind::Action,
            "selected-physical-base-final",
        ),
        (0x335c, RuleEntityKind::Action, "action-minion-damage"),
    ] {
        let channel = StageChannel::Stat {
            scope,
            stat: def(n),
        };
        if !i
            .stages
            .frozen_channels
            .iter()
            .any(|f| f.channel == channel)
        {
            i.stages.frozen_channels.push(FrozenStageChannel {
                channel,
                stage: key(stage),
            });
        }
    }
    quality(&mut i, [0., 20.]);
    i.rebind_test_edit().unwrap();
    i
}
fn quality(i: &mut ReplayInput, values: [f64; 2]) {
    let gems: Vec<_> = i
        .build
        .gems
        .iter_mut()
        .filter(|g| g.definition == def(0x11))
        .collect();
    assert_eq!(gems.len(), 2);
    for (g, amount) in gems.into_iter().zip(values) {
        g.quality.as_mut().unwrap().amount = FiniteQuantity::new(amount, def(2)).unwrap();
    }
}
fn evaluate(i: &ReplayInput) -> SupportEffectsReport {
    let p = i.compile().unwrap();
    p.evaluate(&mut p.new_scratch()).unwrap()
}
fn endpoints(i: &ReplayInput, r: &SupportEffectsReport) -> Vec<[f64; 2]> {
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let SupportEffectsOutcome::Evaluated { effects } = &r.outcome else {
        panic!("{:?}", r.outcome)
    };
    assert!(effects.gaps.is_empty(), "{:?}", effects.gaps);
    let mut found = vec![];
    for request in &i.queries.requests {
        let MetricTarget::Action(action) = &request.target else {
            continue;
        };
        let values = family::OUTPUTS.map(|n| {
            effects.values.iter().find(|v| {
                v.key
                    == PlanValueKey::Stat {
                        entity: ConcreteEntity::Action(action.clone()),
                        stat: def(n),
                    }
            })
        });
        if action.action.output.slot != def(0x22) {
            assert!(values.iter().all(|v| v.is_none()));
            continue;
        }
        found.push(values.map(|v| {
            let EffectValue::Known {
                value: ParameterValue::Quantity(v),
            } = &v.expect("exact Basic endpoint").value
            else {
                panic!("unknown endpoint")
            };
            v.value()
        }));
    }
    assert_eq!(found.len(), 2);
    found
}
fn condition(
    i: &mut ReplayInput,
    which: usize,
    value: bool,
    declare: bool,
    inactive: bool,
    stage: &str,
) {
    let id = key(&format!("{CONTROL}-{which}"));
    let p = RuleProgram {
        id: id.clone(),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![
            RuleNode {
                id: key("value"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(value),
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
            id: key("condition"),
            when: inactive.then(|| key("inactive")),
            effect: RuleEffectKind::Contribute {
                entity: RuleEntity::Current,
                stat: def(family::CONDITIONS[which]),
                contribution: ContributionKind::Flag,
                value: key("value"),
            },
        }],
    };
    if declare {
        let q = i
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|q| q.stat == def(family::CONDITIONS[which]))
            .unwrap();
        q.groups[0].members.members.push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner: actor_owner(),
                program: id,
                effect: key("condition"),
                origin: ContributionOrigin::SuppliedActor {
                    slots: vec![actor_slot()],
                },
            }),
            order: None,
        });
    }
    install(i, actor_owner(), p, stage);
}
#[test]
fn retained_source_endpoints_run_on_exact_actions_with_fresh_reused_and_rayon_scratch() {
    let i = fixture();
    let p = i.compile().unwrap();
    let expected = p.evaluate(&mut p.new_scratch()).unwrap();
    assert_eq!(endpoints(&i, &expected), vec![[574., 1068.], [689., 1282.]]);
    let mut scratch = p.new_scratch();
    for _ in 0..3 {
        assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    }
    let parallel: Vec<_> = (0..8)
        .into_par_iter()
        .map(|_| p.evaluate(&mut p.new_scratch()).unwrap())
        .collect();
    assert!(parallel.iter().all(|r| r == &expected));
    let mut other = i.clone();
    quality(&mut other, [1., 0.]);
    other.rebind_test_edit().unwrap();
    assert_eq!(
        endpoints(&other, &evaluate(&other)),
        vec![[579., 1077.], [574., 1068.]]
    );
}
#[test]
fn typed_actor_conditions_stack_once_and_round_the_more_bucket_only_once() {
    for (flags, expected) in [
        ([true, false], [549., 1022.]),
        ([false, true], [517., 961.]),
        ([true, true], [494., 920.]),
    ] {
        let mut i = fixture();
        quality(&mut i, [0., 0.]);
        for (n, value) in flags.into_iter().enumerate() {
            condition(&mut i, n, value, true, false, "deliver");
        }
        i.rebind_test_edit().unwrap();
        assert_eq!(endpoints(&i, &evaluate(&i)), vec![expected; 2]);
    }
    // Rounding the already-rounded 1.24 MORE result would instead produce
    // 1.12 here. The source bucket order gives 0.9 * 1.03 * 1.2 -> 1.11.
    let mut i = fixture();
    quality(&mut i, [3., 3.]);
    condition(&mut i, 1, true, true, false, "deliver");
    i.rebind_test_edit().unwrap();
    assert_eq!(endpoints(&i, &evaluate(&i)), vec![[531., 988.]; 2]);
    let mut i = fixture();
    quality(&mut i, [0., 0.]);
    condition(&mut i, 0, true, true, false, "deliver");
    let mut duplicate = i
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap()
        .programs
        .members
        .iter()
        .find(|p| p.id == key(&format!("{CONTROL}-0")))
        .unwrap()
        .clone();
    duplicate.id = key("synthetic.second-intimidation-origin");
    let q = i
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.stat == def(family::CONDITIONS[0]))
        .unwrap();
    let mut member = q.groups[0].members.members[0].clone();
    let ContributionProducer::ProgramEffect(producer) = &mut member.producer else {
        unreachable!()
    };
    producer.program = duplicate.id.clone();
    q.groups[0].members.members.push(member);
    install(&mut i, actor_owner(), duplicate, "deliver");
    i.rebind_test_edit().unwrap();
    assert_eq!(endpoints(&i, &evaluate(&i)), vec![[549., 1022.]; 2]);
}

#[test]
fn declared_unknown_condition_is_unavailable_and_never_false() {
    let mut i = fixture();
    condition(&mut i, 0, false, true, false, "deliver");
    let missing: StatDefId = DefId::parse(
        GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap(),
        "synthetic.unknown-actor-condition",
    )
    .unwrap();
    let mut descriptor = i
        .schema
        .definitions
        .iter()
        .find(|d| d.address() == def::<StatDefinition>(family::CONDITIONS[0]).address())
        .unwrap()
        .clone();
    let DefinitionDescriptor::Stat(entry) = &mut descriptor else {
        unreachable!()
    };
    entry.id = missing.clone();
    i.schema.definitions.push(descriptor);
    i.rules.owners.push(DefinitionRules {
        owner: SchemaSubject::Definition(missing.address()),
        programs: DeclaredSet::complete(vec![]),
    });
    let p = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == actor_owner())
        .unwrap()
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(&format!("{CONTROL}-0")))
        .unwrap();
    p.reads.push(RuleRead {
        id: key("missing"),
        value_type: ComputedValueType::Boolean,
        source: RuleReadSource::Stat {
            entity: RuleEntity::Current,
            stat: missing.clone(),
        },
    });
    p.nodes
        .iter_mut()
        .find(|n| n.id == key("value"))
        .unwrap()
        .expression = RuleExpression::Read {
        input: key("missing"),
    };
    i.stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::Actor,
            stat: missing,
        },
        stage: key("prepare"),
    });
    i.rebind_test_edit().unwrap();
    let report = evaluate(&i);
    let SupportEffectsOutcome::Evaluated { effects } = report.outcome else {
        panic!("expected explicit missing producer in the component report")
    };
    assert!(effects.values.iter().any(|v| matches!(
        &v.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    )));
    for value in effects.values.iter().filter(|v|matches!(&v.key,PlanValueKey::Stat{stat,..} if family::OUTPUTS.iter().any(|n|*stat==def(*n)))) {
        assert!(matches!(value.value,EffectValue::Unresolved{..}));
    }
}
#[test]
fn unlisted_inactive_false_and_late_condition_suppliers_are_rejected() {
    for which in 0..2 {
        for (declare, inactive, stage) in [
            (false, false, "deliver"),
            (false, true, "deliver"),
            (true, true, FINAL),
        ] {
            let mut i = fixture();
            condition(&mut i, which, false, declare, inactive, stage);
            let error = i
                .rebind_test_edit()
                .and_then(|_| i.compile().map(|_| ()))
                .unwrap_err();
            assert!(!error.is_empty());
        }
    }
}
#[test]
fn partial_condition_or_output_coverage_is_never_neutralized() {
    for which in 0..3 {
        let mut i = fixture();
        let gap = SchemaGap {
            subject: if which < 2 {
                SchemaSubject::Definition(
                    def::<StatDefinition>(family::CONDITIONS[which]).address(),
                )
            } else {
                owner()
            },
            facet: SchemaFacet::GameRules,
            code: key("synthetic-unreviewed-hit-supplier"),
        };
        if which < 2 {
            i.rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .iter_mut()
                .find(|q| q.stat == def(family::CONDITIONS[which]))
                .unwrap()
                .groups[0]
                .members
                .closure = SchemaClosure::Partial { gaps: vec![gap] };
        } else {
            i.rules
                .owners
                .iter_mut()
                .find(|o| o.owner == owner())
                .unwrap()
                .programs
                .closure = family::consumer().owners[0].programs.closure.clone();
        }
        i.rebind_test_edit().unwrap();
        let p = i.compile().unwrap();
        let report = p.evaluate(&mut p.new_scratch()).unwrap();
        assert!(
            !report.gaps.is_empty()
                || match report.outcome {
                    SupportEffectsOutcome::Evaluated { effects } => !effects.gaps.is_empty(),
                    _ => true,
                }
        );
    }
}
