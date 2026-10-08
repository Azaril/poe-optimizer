//! Actual final attributes feed checked Boolean reducers and inherent Life.
//! The original selection contains neither flag-producing passive. Counterfactual
//! selections below cover their published flag bodies only, not their remaining
//! mechanics, ascendancy legality, final Life aggregation, or complete builds.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use sha2::{Digest, Sha256};
use std::path::Path;

const FLAGS: &str = "inherent-attribute-flags";
const LIFE: &str = "inherent-strength-life";
const SOURCE_FLAGS: [&str; 5] = [
    "NoAttributeBonuses",
    "NoStrengthAttributeBonuses",
    "NoStrBonusToLife",
    "DoubledInherentAttributeBonuses",
    "HalvesLifeFromStrength",
];
#[derive(Clone)]
pub(super) struct Census {
    queries_before: Vec<ContributionQuery>,
    actual_registry: SchemaClosure,
    actual_donors: Vec<DefinitionRules>,
    original: Value,
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}
fn owner(stat: u64) -> SchemaSubject {
    subject(d::<StatDefinition>(stat))
}
fn passive(index: usize) -> PassiveNodeDefId {
    d([0x10ac, 0x18d9][index])
}
pub(super) fn install(
    sniper: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
) -> Census {
    attribute_flag_family::assert_component(endpoint);
    let source = attribute_flag_family::checked_source();
    let original = source["native_cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap()
        .clone();
    assert_eq!(original["strength"], 27);
    assert_eq!(original["inherent_life"], 54);
    for name in SOURCE_FLAGS {
        assert_eq!(original["flags"][name], false);
    }
    let xml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let observed = source["projections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "original-05")
        .unwrap();
    assert_eq!(
        format!("{:x}", Sha256::digest(xml.as_bytes())),
        observed["xml_sha256"]
    );
    let allocations = passive_damage_evidence::normalized_selected_allocations(package, &xml);
    assert_eq!(allocations.len(), 55);
    assert!(
        allocations
            .iter()
            .all(|a| !(0..2).any(|i| a.node.to_resolved() == Some(passive(i))))
    );
    let recipe = &endpoint.input().recipe;
    let f = &mut sniper.base.source.base.inner;
    let mut addresses: Vec<_> = (0..6)
        .map(|i| d::<StatDefinition>(0x3315 + i).address())
        .collect();
    addresses.extend((0..2).map(|i| passive(i).address()));
    addresses.extend([
        d::<PointPoolDefinition>(0x1bf0).address(),
        d::<PointPoolDefinition>(0x1bf1).address(),
    ]);
    for address in addresses {
        let mut actual = recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == address)
            .unwrap()
            .clone();
        if let DefinitionDescriptor::PassiveNode(DefinitionEntry {
            schema: SchemaState::Known(s),
            ..
        }) = &mut actual
        {
            // Only these two published flag bodies are exercised by the controls.
            // Empty declaration closure here is finite test authority, never data.
            s.adjacent = DeclaredSet::complete(vec![]);
            macro_rules! empty {
                ($field:ident) => {{
                    assert!(s.declarations.$field.members.is_empty());
                    s.declarations.$field.closure = SchemaClosure::Complete;
                }};
            }
            empty!(parameters);
            empty!(choices);
            empty!(grants);
            empty!(actors);
            empty!(skill_grants);
            empty!(outputs);
            empty!(sockets);
        }
        if let Some(existing) = f.schema.definitions.iter().find(|r| r.address() == address) {
            assert_eq!(existing, &actual);
        } else {
            f.schema.definitions.push(actual);
        }
        f.owner_mut(SchemaSubject::Definition(address));
    }
    let mut actual_donors = Vec::new();
    let subjects = (0..6)
        .map(|i| owner(0x3315 + i))
        .chain((0..2).map(|i| subject(passive(i))));
    for subject in subjects {
        let actual = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == subject)
            .unwrap();
        let mut finite = actual.clone();
        if (0..2).any(|i| subject == super::subject(passive(i))) {
            assert!(!actual.programs.is_complete());
            actual_donors.push(actual.clone());
            finite.programs.closure = SchemaClosure::Complete;
        } else {
            assert!(actual.programs.is_complete());
        }
        let selected = f.owner_mut(subject);
        assert!(selected.programs.members.is_empty());
        *selected = finite;
    }
    assert_eq!(actual_donors.len(), 2);
    let receivers: Vec<_> = recipe
        .rules
        .receivers
        .members
        .iter()
        .filter(|r| (0..6).any(|i| r.stat == d(0x3315 + i)))
        .cloned()
        .collect();
    assert_eq!(receivers.len(), 6);
    for receiver in receivers {
        assert_eq!(receiver.targets, vec![StatReceiverTarget::Player]);
        assert!(!sniper.receivers.members.iter().any(|r| r.id == receiver.id));
        sniper.receivers.members.push(receiver);
    }
    let registry = recipe.rules.contribution_queries.as_ref().unwrap();
    assert!(!registry.is_complete());
    let replacements = attribute_flag_family::replacements();
    assert_eq!(replacements.len(), 5);
    let mut queries_before = Vec::new();
    for q in replacements {
        assert!(registry.members.contains(&q.after));
        assert_eq!(q.after.groups.len(), 1);
        assert!(q.after.groups[0].members.is_complete());
        assert!(
            !sniper
                .base
                .contribution_queries
                .members
                .iter()
                .any(|existing| existing.id == q.after.id)
        );
        sniper.base.contribution_queries.members.push(q.after);
        queries_before.push(q.before);
    }
    Census {
        queries_before,
        actual_registry: registry.closure.clone(),
        actual_donors,
        original,
    }
}
pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(FLAGS),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(LIFE),
            predecessors: vec![key(FLAGS), key("attribute-second-pass")],
        },
    ]);
    for row in &mut stages.programs.members {
        if (0..5).any(|i| row.owner == owner(0x3315 + i)) {
            row.stage = key(FLAGS);
        }
        if row.owner == owner(0x331a) {
            row.stage = key(LIFE);
        }
    }
    for i in 0..5 {
        stages.frozen_channels.extend([
            FrozenStageChannel {
                channel: StageChannel::Contributions {
                    scope: RuleEntityKind::Actor,
                    stat: d(0x3315 + i),
                    contribution: ContributionKind::Flag,
                },
                stage: key("deliver"),
            },
            FrozenStageChannel {
                channel: StageChannel::Stat {
                    scope: RuleEntityKind::Actor,
                    stat: d(0x3315 + i),
                },
                stage: key(FLAGS),
            },
        ]);
    }
    stages.frozen_channels.push(FrozenStageChannel {
        channel: StageChannel::Stat {
            scope: RuleEntityKind::Actor,
            stat: d(0x1d2e),
        },
        stage: key("attribute-second-pass"),
    });
}
fn value(report: &SupportEffectsReport, stat: u64) -> &EffectValue {
    let rows: Vec<_> = sniper::offering::effects(report)
        .values
        .iter()
        .filter(|r| {
            r.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(stat),
                }
        })
        .collect();
    assert_eq!(rows.len(), 1);
    &rows[0].value
}
fn check(report: &SupportEffectsReport, amount: f64, halved: bool, doubled: bool) {
    let effects = sniper::offering::effects(report);
    for (i, expected) in [false, false, false, doubled, halved]
        .into_iter()
        .enumerate()
    {
        assert_eq!(
            value(report, 0x3315 + i as u64),
            &EffectValue::Known {
                value: ParameterValue::Boolean(expected)
            }
        );
    }
    assert_eq!(
        value(report, 0x331a),
        &EffectValue::Known {
            value: quantity(amount, &d(0x3119))
        }
    );
    let rows: Vec<_> = effects
        .effects
        .iter()
        .filter(|r| r.key.invocation.program == key(LIFE))
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].key.invocation.owner, owner(0x331a));
    assert_eq!(
        rows[0].key.invocation.entity,
        ConcreteEntity::Actor(ActorKey::Player)
    );
    assert_eq!(
        rows[0].key.invocation.origin,
        RuleOrigin::Receiver {
            receiver: key("player-inherent-strength-life"),
            actor: ActorKey::Player
        }
    );
    assert_eq!(
        rows[0].target,
        BoundEffectTarget::Value {
            key: PlanValueKey::Stat {
                entity: ConcreteEntity::Actor(ActorKey::Player),
                stat: d(0x331a)
            }
        }
    );
    assert!(
        !effects
            .values
            .iter()
            .any(|r| matches!(&r.key,PlanValueKey::Stat {stat,..} if *stat==d(0x311a))),
        "inherent amount is not final Player/minion Life"
    );
    assert!(effects.values.iter().filter(|r| matches!(&r.key,PlanValueKey::Stat {stat,..} if (0..6).any(|i| *stat==d(0x3315+i))))
        .all(|r| matches!(&r.key,PlanValueKey::Stat {entity:ConcreteEntity::Actor(ActorKey::Player),..})),"Player-only receivers do not leak into Sniper Actors");
}
fn select(w: &mut World, halved: bool, doubled: bool) {
    inner(w)
        .build
        .allocations
        .retain(|a| !(0..2).any(|i| a.node == passive(i)));
    for (i, enabled) in [halved, doubled].into_iter().enumerate() {
        if enabled {
            inner(w).build.allocations.push(Allocation {
                id: id(7700 + i as u64),
                node: passive(i),
                pool: d([0x1bf0, 0x1bf1][i]),
                scope: LoadoutScope::Shared,
                access: AllocationAccess::Ordinary,
                choices: vec![],
            });
        }
    }
}
fn unavailable(plan: &shared::Plan) {
    assert_eq!(
        plan.evaluate(&mut plan.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None
        }
    );
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn original_attributes_and_checked_empty_flags_reach_inherent_life_without_supplied_inputs() {
    let w = World::load();
    let report = w.evaluate();
    check(
        &report,
        w.inherent_life.original["inherent_life"].as_f64().unwrap(),
        false,
        false,
    );
    assert_eq!(
        value(&report, 0x1d2e),
        &EffectValue::Known {
            value: ParameterValue::Integer(
                BoundedInteger::new(w.inherent_life.original["strength"].as_i64().unwrap())
                    .unwrap()
            )
        }
    );
    assert!(!sniper::offering::effects(&report).effects.iter().any(|r| matches!(&r.target,BoundEffectTarget::Contribution {key} if key.kind==ContributionKind::Flag && (0..5).any(|i| key.stat==d(0x3315+i)))),"absence comes from complete checked groups; no false producer is invented");
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn actual_flag_bodies_preserve_occurrences_and_combine_once_with_real_final_strength() {
    for (half, double, expected) in [
        (false, false, 54.),
        (true, false, 27.),
        (false, true, 108.),
        (true, true, 54.),
    ] {
        let mut w = World::load();
        select(&mut w, half, double);
        let report = w.evaluate();
        check(&report, expected, half, double);
        let rows:Vec<_>=sniper::offering::effects(&report).effects.iter().filter(|r| matches!(&r.target,BoundEffectTarget::Contribution {key} if key.kind==ContributionKind::Flag && (0..5).any(|i| key.stat==d(0x3315+i)))).collect();
        assert_eq!(rows.len(), usize::from(half) + usize::from(double));
        for row in rows {
            let index = usize::from(row.key.invocation.owner == subject(passive(1)));
            assert_eq!(row.key.invocation.owner, subject(passive(index)));
            assert_eq!(
                row.key.invocation.origin,
                RuleOrigin::Provider {
                    provider: ProviderKey {
                        root: ProviderRoot::Allocation(id(7700 + index as u64)),
                        grant_path: vec![]
                    }
                }
            );
            assert_eq!(
                row.value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(true)
                }
            );
        }
    }
    let mut duplicate = World::load();
    select(&mut duplicate, true, false);
    let mut allocation = inner(&mut duplicate)
        .build
        .allocations
        .iter()
        .find(|a| a.node == passive(0))
        .unwrap()
        .clone();
    allocation.id = id(7702);
    inner(&mut duplicate).build.allocations.push(allocation);
    let report = duplicate.evaluate();
    check(&report, 27., true, false);
    assert_eq!(
        sniper::offering::effects(&report)
            .effects
            .iter()
            .filter(|r| r.key.invocation.program == key("strength-life-halving"))
            .count(),
        2,
        "Any keeps two origins without doubling the mechanical flag"
    );
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn inherent_life_retains_query_and_actual_passive_coverage_refusals() {
    let baseline = World::load();
    for i in 0..5 {
        let mut w = baseline.clone();
        select(&mut w, true, true);
        let actual = w.inherent_life.queries_before[i].clone();
        let id = actual.id.clone();
        *w.sniper
            .base
            .contribution_queries
            .members
            .iter_mut()
            .find(|q| q.id == id)
            .unwrap() = actual;
        let p = w.plan();
        assert!(
            p.gaps()
                .iter()
                .any(|g| g.reason == PlanGapReason::IncompleteContributors)
        );
        unavailable(&p);
    }
    for i in 0..2 {
        let mut w = baseline.clone();
        select(&mut w, true, true);
        let actual = w.inherent_life.actual_donors[i].clone();
        let subject = actual.owner.clone();
        *inner(&mut w).owner_mut(subject.clone()) = actual;
        let p = w.plan();
        assert!(
            p.gaps()
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms
                    && g.subject.as_ref() == Some(&subject))
        );
        unavailable(&p);
    }
    let mut w = baseline.clone();
    w.sniper.base.contribution_queries.closure = w.inherent_life.actual_registry.clone();
    unavailable(&w.plan());
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn unknown_inactive_false_contributors_cannot_masquerade_as_an_empty_flag_domain() {
    for i in 0..5 {
        let mut w = World::load();
        let class = inner(&mut w).build.character.class.clone();
        inner(&mut w)
            .owner_mut(subject(class))
            .programs
            .members
            .push(RuleProgram {
                id: key("counterfactual-unlisted-inherent-flag"),
                context: RuleEntityKind::Actor,
                reads: vec![],
                nodes: vec![RuleNode {
                    id: key("false"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(false),
                    },
                }],
                effects: vec![RuleEffect {
                    id: key("flag"),
                    when: Some(key("false")),
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Player,
                        stat: d(0x3315 + i),
                        contribution: ContributionKind::Flag,
                        value: key("false"),
                    },
                }],
            });
        let error = w
            .checked_plan()
            .err()
            .expect("unknown potential contributor must fail before activation");
        assert!(
            error.contains("actual contribution has no declared membership"),
            "{error}"
        );
    }
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn missing_actual_strength_or_flag_receiver_and_wrong_stage_never_supply_life() {
    for (stat, read) in [(0x1d2e, "strength"), (0x3318, "inherent-doubled")] {
        let mut w = World::load();
        w.sniper.receivers.members.retain(|r| r.stat != d(stat));
        let report = w.evaluate();
        assert_eq!(
            value(&report, 0x331a),
            &EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(key(read))
            }
        );
    }
    let w = World::load();
    let error = w
        .checked_plan_configured(|stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|r| r.owner == owner(0x331a))
                .unwrap()
                .stage = key("deliver");
        })
        .err()
        .expect("inherent Life cannot precede final attributes and flags");
    assert!(
        error.contains("stage") || error.contains("frozen"),
        "{error}"
    );
}

#[test]
#[ignore = "requires current Sniper release with bounded inherent flag membership"]
fn inherent_life_restores_actual_inputs_with_reused_and_parallel_scratch() {
    let original = World::load();
    let p = original.plan();
    let mut scratch = p.new_scratch();
    let first = p.evaluate(&mut scratch).unwrap();
    check(&first, 54., false, false);
    let mut changed = original.clone();
    select(&mut changed, true, false);
    let q = changed.plan();
    check(&q.evaluate(&mut scratch).unwrap(), 27., true, false);
    assert_eq!(p.evaluate(&mut scratch).unwrap(), first);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    pool.install(|| {
        (0..8).into_par_iter().for_each_init(
            || p.new_scratch(),
            |s, _| assert_eq!(p.evaluate(s).unwrap(), first),
        )
    });
}
