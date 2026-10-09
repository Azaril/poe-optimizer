//! Actual selected Gigantic Following on the same item-driven Sniper graph.
//! This joins the flag and individual Life/Damage factors, not composed MORE,
//! final pools, reservation delivery or a complete tree/build.
use super::*;
use poe_optimizer_core::owned_stages::{EvaluationStage, FrozenStageChannel, StageChannel};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use sha2::{Digest, Sha256};
use std::path::Path;

const GRANT: &str = "ordinary-minion-gigantic";
const EFFICIENCY: &str = "ordinary-minion-reservation-efficiency";
const STATUS: &str = "received-minion-gigantic";
const BENEFITS: &str = "gigantic-life-and-damage";
const INTRINSIC_LIFE: &str = "intrinsic-allied-minion-life";
const RECEIVER: &str = "sniper-received-minion-gigantic";
const STATUS_STAGE: &str = "gigantic-status";
const BENEFIT_STAGE: &str = "gigantic-benefits";

#[derive(Clone)]
pub(super) struct Census {
    original: Allocation,
    selected: Allocation,
    query: ContributionQuery,
}
fn passive() -> SchemaSubject {
    subject(d::<PassiveNodeDefinition>(0x1532))
}
fn status_owner() -> SchemaSubject {
    subject(d::<StatDefinition>(0x3308))
}
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}
fn inner(w: &mut World) -> &mut shared::World {
    &mut w.sniper.base.source.base.inner
}

pub(super) fn install(
    sniper: &mut sniper::World,
    endpoint: &StagedOwnedRelease,
    package: &Path,
) -> Census {
    gigantic_family::assert_component(endpoint);
    let recipe = &endpoint.input().recipe;
    let xml = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let hashes = gigantic_family::source_xml_hashes();
    assert_eq!(format!("{:x}", Sha256::digest(xml.as_bytes())), hashes[0]);
    let allocations = passive_damage_evidence::normalized_selected_allocations(package, &xml);
    assert_eq!(allocations.len(), 55);
    let actual: Vec<_> = allocations
        .iter()
        .filter(|a| a.node.to_resolved() == Some(d(0x1532)))
        .collect();
    assert_eq!(actual.len(), 1);
    let original = actual[0]
        .to_resolved()
        .expect("the actual selected Gigantic allocation is resolved");
    assert_eq!(original.access, AllocationAccess::Ordinary);
    assert_eq!(original.scope, LoadoutScope::Shared);
    assert!(original.choices.is_empty());
    let removed = passive_damage_evidence::remove_saved_node(&xml, "46365");
    assert_eq!(
        format!("{:x}", Sha256::digest(removed.as_bytes())),
        hashes[1]
    );
    let without = passive_damage_evidence::normalized_selected_allocations(package, &removed);
    assert_eq!(without.len(), 54);
    assert!(
        !without
            .iter()
            .any(|a| a.node.to_resolved() == Some(d(0x1532)))
    );
    // Preserve all selected records in the shared import. This finite graph
    // chooses only this family's exact occurrence and does not repair others.
    let f = &mut sniper.base.source.base.inner;
    let mut selected = original.clone();
    selected.id = id(7400);
    assert_ne!(
        selected.id, original.id,
        "explicit fixture-lineage remap only"
    );
    assert!(
        !f.build
            .allocations
            .iter()
            .any(|a| a.id == selected.id || a.node == selected.node)
    );
    for address in [
        original.pool.address(),
        d::<StatDefinition>(0x3307).address(),
        d::<StatDefinition>(0x3308).address(),
        d::<StatDefinition>(0x3309).address(),
        d::<StatDefinition>(0x330b).address(),
        d::<StatDefinition>(0x311a).address(),
        d::<UnitDefinition>(0x3119).address(),
    ] {
        let actual = recipe
            .schema
            .definitions
            .iter()
            .find(|r| r.address() == address)
            .unwrap();
        if let Some(existing) = f.schema.definitions.iter().find(|r| r.address() == address) {
            assert_eq!(existing, actual);
        } else {
            f.schema.definitions.push(actual.clone());
        }
        f.owner_mut(SchemaSubject::Definition(address));
    }
    let actual = recipe
        .schema
        .definitions
        .iter()
        .find(|r| r.address() == original.node.address())
        .unwrap();
    let mut descriptor = actual.clone();
    let DefinitionDescriptor::PassiveNode(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = &mut descriptor
    else {
        panic!("known actual passive")
    };
    assert!(schema.pools.is_complete());
    assert_eq!(schema.pools.members, std::slice::from_ref(&original.pool));
    // Numerical integration deliberately does not claim connected-tree legality.
    schema.adjacent = DeclaredSet::complete(vec![]);
    assert!(
        !f.schema
            .definitions
            .iter()
            .any(|r| r.address() == descriptor.address())
    );
    f.schema.definitions.push(descriptor);
    for subject in [passive(), status_owner()] {
        let actual = recipe
            .rules
            .owners
            .iter()
            .find(|r| r.owner == subject)
            .unwrap();
        assert!(actual.programs.is_complete());
        let selected = f.owner_mut(subject);
        assert!(selected.programs.members.is_empty());
        *selected = actual.clone();
    }
    let actual_actor = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(!actual_actor.programs.is_complete());
    let benefits: Vec<_> = actual_actor
        .programs
        .members
        .iter()
        .filter(|p| p.id == key(BENEFITS))
        .collect();
    assert_eq!(benefits.len(), 1);
    let channels: Vec<_> = benefits[0]
        .effects
        .iter()
        .map(|e| match &e.effect {
            RuleEffectKind::Contribute {
                stat,
                contribution: ContributionKind::Multiply,
                ..
            } => stat.clone(),
            _ => panic!("actual paired Gigantic factors"),
        })
        .collect();
    assert_eq!(
        channels,
        [d(0x311a), d(0x330b)],
        "canonical Life, no retired330a alias"
    );
    let selected_actor = f.owner_mut(actor_owner());
    assert!(
        !selected_actor
            .programs
            .members
            .iter()
            .any(|p| p.id == key(BENEFITS))
    );
    selected_actor.programs.members.push(benefits[0].clone());
    // Reuse the actual intrinsic Life producer and curve so removal/duplicate
    // controls retain the canonical-Life regression from the retired fixture.
    // This is an Add contribution, not a final Life pool or a supplied value.
    let life_packet = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/minion-life-source");
    let life_authoring: Value = shared::read(life_packet.join("authoring.json"));
    for artifact in ["migration", "source-vectors"] {
        let bytes = std::fs::read(life_packet.join(format!("{artifact}.json"))).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(&bytes)),
            life_authoring["artifact_sha256"][artifact]
        );
    }
    let life_migration: poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput =
        shared::read(life_packet.join("migration.json"));
    let life = actual_actor
        .programs
        .members
        .iter()
        .find(|p| p.id == key(INTRINSIC_LIFE))
        .unwrap();
    assert!(
        life_migration
            .owners
            .iter()
            .any(|o| o.owner == actor_owner() && o.programs.members.contains(life))
    );
    assert!(
        !selected_actor
            .programs
            .members
            .iter()
            .any(|p| p.id == life.id)
    );
    selected_actor.programs.members.push(life.clone());
    let table = recipe
        .rules
        .tables
        .iter()
        .find(|t| t.id == key("actor.allied-life-by-level"))
        .unwrap();
    assert!(life_migration.tables.contains(table));
    assert!(!sniper.base.tables.iter().any(|t| t.id == table.id));
    sniper.base.tables.push(table.clone());
    assert!(
        !serde_json::to_string(&recipe.rules)
            .unwrap()
            .contains("def.000000000000330a"),
        "the full current rules have no retired Life alias"
    );
    f.build.allocations.push(selected.clone());
    let receivers: Vec<_> = recipe
        .rules
        .receivers
        .members
        .iter()
        .filter(|r| r.id == key(RECEIVER))
        .collect();
    assert_eq!(receivers.len(), 1);
    assert!(
        !sniper
            .receivers
            .members
            .iter()
            .any(|r| r.id == key(RECEIVER))
    );
    sniper.receivers.members.push(receivers[0].clone());
    let queries = gigantic_family::queries();
    assert_eq!(queries.len(), 1);
    let query = queries[0].clone();
    assert_eq!(query.stat, d(0x3307));
    assert_eq!(query.groups.len(), 1);
    assert!(!query.groups[0].members.is_complete());
    assert_eq!(query.groups[0].members.members.len(), 1);
    assert!(
        recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .members
            .contains(&query)
    );
    assert!(
        !sniper
            .base
            .contribution_queries
            .members
            .iter()
            .any(|q| q.id == query.id)
    );
    let mut finite = query.clone();
    // Exactly the real selected passive in this finite domain. Complete build
    // discovery is still Partial in the published query and Actor inventory.
    finite.groups[0].members.closure = SchemaClosure::Complete;
    sniper.base.contribution_queries.members.push(finite);
    Census {
        original,
        selected,
        query,
    }
}

pub(super) fn configure(stages: &mut EvaluationStagesInput) {
    stages.stages.extend([
        EvaluationStage {
            id: key(STATUS_STAGE),
            predecessors: vec![key("deliver")],
        },
        EvaluationStage {
            id: key(BENEFIT_STAGE),
            predecessors: vec![key(STATUS_STAGE)],
        },
    ]);
    for row in &mut stages.programs.members {
        if row.owner == status_owner() && row.program == key(STATUS) {
            row.stage = key(STATUS_STAGE);
        }
        if row.owner == actor_owner() && row.program == key(BENEFITS) {
            row.stage = key(BENEFIT_STAGE);
        }
    }
    stages.frozen_channels.extend([
        FrozenStageChannel {
            channel: StageChannel::Contributions {
                scope: RuleEntityKind::Actor,
                stat: d(0x3307),
                contribution: ContributionKind::Flag,
            },
            stage: key("deliver"),
        },
        FrozenStageChannel {
            channel: StageChannel::Stat {
                scope: RuleEntityKind::Actor,
                stat: d(0x3308),
            },
            stage: key(STATUS_STAGE),
        },
    ]);
}

fn effects(report: &SupportEffectsReport) -> &OwnedEffectsReport {
    sniper::offering::effects(report)
}
fn grant_rows(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
    report.effects.iter().filter(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == d(0x3307))).collect()
}
fn benefit_rows(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(BENEFITS))
        .collect()
}
fn intrinsic_life_rows(report: &OwnedEffectsReport) -> Vec<BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(INTRINSIC_LIFE))
        .cloned()
        .collect()
}
fn remove(w: &mut World) {
    let selected = w.gigantic.selected.id;
    let before = inner(w).build.allocations.len();
    inner(w).build.allocations.retain(|a| a.id != selected);
    assert_eq!(inner(w).build.allocations.len() + 1, before);
}
fn check(
    w: &World,
    report: &SupportEffectsReport,
    actual_selected: bool,
    active: bool,
    grants: usize,
) {
    check_actors(w, report, actual_selected, active, grants, &[0, 1]);
}
fn check_actors(
    w: &World,
    report: &SupportEffectsReport,
    actual_selected: bool,
    active: bool,
    grants: usize,
    recipients: &[usize],
) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    let r = effects(report);
    assert!(r.gaps.is_empty(), "{:?}", r.gaps);
    let selected: Vec<_> = w
        .sniper
        .base
        .source
        .base
        .inner
        .build
        .allocations
        .iter()
        .filter(|a| a.node == d(0x1532))
        .collect();
    assert_eq!(selected.len(), usize::from(actual_selected));
    if actual_selected {
        assert_eq!(selected[0], &w.gigantic.selected);
    }
    let actual: Vec<_> = grant_rows(r)
        .into_iter()
        .filter(|e| e.key.invocation.owner == passive())
        .collect();
    assert_eq!(actual.len(), usize::from(actual_selected));
    for row in actual {
        assert_eq!(row.key.invocation.program, key(GRANT));
        assert_eq!(
            row.key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::Allocation(w.gigantic.selected.id),
                    grant_path: vec![],
                }
            }
        );
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x3307),
                    kind: ContributionKind::Flag,
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
    assert_eq!(grant_rows(r).len(), grants);
    let reservation: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(EFFICIENCY))
        .collect();
    assert_eq!(reservation.len(), usize::from(actual_selected));
    for row in reservation {
        assert_eq!(row.key.invocation.owner, passive());
        assert_eq!(
            row.target,
            BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: d(0x3309),
                    kind: ContributionKind::Increase,
                }
            }
        );
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: quantity(-25., &d(2))
            }
        );
    }
    let statuses: Vec<_> = r
        .effects
        .iter()
        .filter(|e| e.key.invocation.program == key(STATUS))
        .collect();
    assert_eq!(statuses.len(), recipients.len());
    let benefits = benefit_rows(r);
    assert_eq!(benefits.len(), recipients.len() * 2);
    let life = intrinsic_life_rows(r);
    assert_eq!(life.len(), recipients.len());
    for &index in recipients {
        let actor = w.sniper.actor(index);
        let base: Vec<_> = life
            .iter()
            .filter(|e| {
                e.target
                    == BoundEffectTarget::Contribution {
                        key: ContributionKey {
                            entity: ConcreteEntity::Actor(actor.clone()),
                            stat: d(0x311a),
                            kind: ContributionKind::Add,
                        },
                    }
            })
            .collect();
        assert_eq!(base.len(), 1);
        assert!(
            matches!(&base[0].value, EffectValue::Known { value: ParameterValue::Quantity(value) } if value.unit() == &d(0x3119) && value.value() > 0.)
        );
        let rows: Vec<_> = statuses
            .iter()
            .filter(|e| {
                e.key.invocation.origin
                    == RuleOrigin::Receiver {
                        receiver: key(RECEIVER),
                        actor: actor.clone(),
                    }
            })
            .collect();
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows[0].target,
            BoundEffectTarget::Value {
                key: PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: d(0x3308),
                }
            }
        );
        assert_eq!(
            w.value(r, index, false, 0x3308),
            &EffectValue::Known {
                value: ParameterValue::Boolean(active)
            }
        );
        let mut provider = w.action(index).action.provider;
        assert_eq!(
            provider.grant_path.pop(),
            Some(slot(SlotOwnerDefId::Actor(d(0x3091)), 0x3093))
        );
        for stat in [0x311a, 0x330b] {
            let expected = BoundEffectTarget::Contribution {
                key: ContributionKey {
                    entity: ConcreteEntity::Actor(actor.clone()),
                    stat: d(stat),
                    kind: ContributionKind::Multiply,
                },
            };
            let rows: Vec<_> = benefits.iter().filter(|e| e.target == expected).collect();
            assert_eq!(
                rows.len(),
                1,
                "benefit runs once per exact Actor, not once per granting source"
            );
            assert_eq!(rows[0].key.invocation.owner, actor_owner());
            assert_eq!(
                rows[0].key.invocation.origin,
                RuleOrigin::Provider {
                    provider: provider.clone()
                }
            );
            assert_eq!(
                rows[0].value,
                if active {
                    EffectValue::Known {
                        value: quantity(1.2, &d(1)),
                    }
                } else {
                    EffectValue::Inactive
                }
            );
        }
    }
    assert!(!r.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { entity: ConcreteEntity::Actor(ActorKey::Player), stat } if *stat == d(0x3308))));
    assert!(
        !r.values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == d(0x330b))),
        "individual factor is not a final multiplier"
    );
    assert!(!r.effects.iter().any(|e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.stat == d(0x330a))), "retired Life channel stays absent");
    assert!(!r.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == d(0x311a) || *stat == d(0x330a))),
        "base Add and status Multiply are not a final Life pool or an alias");
}
fn query_mut(w: &mut World) -> &mut ContributionQuery {
    w.sniper
        .base
        .contribution_queries
        .members
        .iter_mut()
        .find(|q| q.stat == d(0x3307))
        .unwrap()
}
fn extra_flag(w: &mut World, name: &str, value: Option<bool>, enabled: bool) {
    let owner = subject(inner(w).build.character.class.clone());
    let mut reads = vec![];
    let expression = if let Some(value) = value {
        RuleExpression::Literal {
            value: ParameterValue::Boolean(value),
        }
    } else {
        let missing = def::<StatDefinition>(&format!("counterfactual.gigantic-missing.{name}"));
        inner(w)
            .schema
            .definitions
            .push(DefinitionDescriptor::Stat(DefinitionEntry {
                id: missing.clone(),
                schema: SchemaState::Known(StatSchema {
                    value: ComputedValueType::Boolean,
                    targets: vec![RuleEntityKind::Actor],
                }),
            }));
        reads.push(RuleRead {
            id: key("missing"),
            value_type: ComputedValueType::Boolean,
            source: RuleReadSource::Stat {
                entity: RuleEntity::Player,
                stat: missing,
            },
        });
        RuleExpression::Read {
            input: key("missing"),
        }
    };
    inner(w)
        .owner_mut(owner.clone())
        .programs
        .members
        .push(RuleProgram {
            id: key(name),
            context: RuleEntityKind::Actor,
            reads,
            nodes: vec![
                RuleNode {
                    id: key("flag"),
                    expression,
                },
                RuleNode {
                    id: key("enabled"),
                    expression: RuleExpression::Literal {
                        value: ParameterValue::Boolean(enabled),
                    },
                },
            ],
            effects: vec![RuleEffect {
                id: key("grant"),
                when: Some(key("enabled")),
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Player,
                    stat: d(0x3307),
                    contribution: ContributionKind::Flag,
                    value: key("flag"),
                },
            }],
        });
    query_mut(w).groups[0]
        .members
        .members
        .push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner,
                program: key(name),
                effect: key("grant"),
                origin: ContributionOrigin::Character,
            }),
            order: None,
        });
}
fn unavailable(report: &SupportEffectsReport) {
    assert_eq!(
        report.outcome,
        SupportEffectsOutcome::Unavailable {
            cause: EffectValue::Unresolved {
                reason: PlanGapReason::IncompleteContributors,
                read: None
            },
            input: None,
        }
    );
}

#[test]
#[ignore = "requires current Gigantic Boolean release and retained source evidence"]
fn gigantic_actual_allocation_and_removal_reach_both_item_driven_actors() {
    let w = World::load();
    assert_ne!(w.gigantic.original.id, w.gigantic.selected.id);
    let mut remapped = w.gigantic.original.clone();
    remapped.id = w.gigantic.selected.id;
    assert_eq!(remapped, w.gigantic.selected);
    let first = w.evaluate();
    check(&w, &first, true, true, 1);
    let mut without = w.clone();
    remove(&mut without);
    let removed = without.evaluate();
    check(&without, &removed, false, false, 0);
    assert_eq!(
        intrinsic_life_rows(effects(&removed)),
        intrinsic_life_rows(effects(&first))
    );
    assert_eq!(w.evaluate(), first);
}

#[test]
#[ignore = "requires current Gigantic Boolean release; explicit finite source controls"]
fn gigantic_boolean_duplicates_do_not_duplicate_benefits_and_unknown_is_not_false() {
    let mut w = World::load();
    let original_life = intrinsic_life_rows(effects(&w.evaluate()));
    extra_flag(&mut w, "counterfactual-gigantic-true", Some(true), true);
    extra_flag(&mut w, "counterfactual-gigantic-false", Some(false), true);
    let report = w.evaluate();
    check(&w, &report, true, true, 3);
    assert_eq!(intrinsic_life_rows(effects(&report)), original_life);
    let synthetic: Vec<_> = grant_rows(effects(&report))
        .into_iter()
        .filter(|e| {
            e.key
                .invocation
                .program
                .as_str()
                .starts_with("counterfactual-")
        })
        .collect();
    assert_eq!(synthetic.len(), 2);
    for row in synthetic {
        assert_eq!(
            row.key.invocation.origin,
            RuleOrigin::Provider {
                provider: ProviderKey {
                    root: ProviderRoot::Character,
                    grant_path: vec![]
                }
            }
        );
    }
    let mut unknown = World::load();
    extra_flag(&mut unknown, "counterfactual-gigantic-unknown", None, true);
    let report = unknown.evaluate();
    for index in 0..2 {
        assert_eq!(
            unknown.value(effects(&report), index, false, 0x3308),
            &EffectValue::Unresolved {
                reason: PlanGapReason::MissingProducer,
                read: Some(key("missing"))
            }
        );
    }
    assert!(benefit_rows(effects(&report)).iter().all(|e| matches!(
        e.value,
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            ..
        }
    )));
    let mut inactive = World::load();
    extra_flag(
        &mut inactive,
        "counterfactual-gigantic-inactive-unknown",
        None,
        false,
    );
    check(&inactive, &inactive.evaluate(), true, true, 2);
    remove(&mut inactive);
    check(&inactive, &inactive.evaluate(), false, false, 1);
}

#[test]
#[ignore = "requires current Gigantic Boolean release; missing and Partial coverage"]
fn gigantic_unreviewed_or_missing_inputs_cannot_supply_status_or_factors() {
    let original = World::load();
    let mut query = original.clone();
    query_mut(&mut query).groups[0].members.closure =
        original.gigantic.query.groups[0].members.closure.clone();
    let p = query.plan();
    assert_eq!(
        p.gaps(),
        &[PlanGap {
            provider: None,
            subject: Some(subject(d::<StatDefinition>(0x3307))),
            reason: PlanGapReason::IncompleteContributors
        }]
    );
    unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    let mut missing = original.clone();
    missing
        .sniper
        .receivers
        .members
        .retain(|r| r.id != key(RECEIVER));
    let report = missing.evaluate();
    assert!(
        !effects(&report)
            .values
            .iter()
            .any(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == d(0x3308)))
    );
    assert_eq!(benefit_rows(effects(&report)).len(), 4);
    assert!(benefit_rows(effects(&report)).iter().all(|e| e.value
        == EffectValue::Unresolved {
            reason: PlanGapReason::MissingProducer,
            read: Some(key("active"))
        }));
    let mut omitted = original.clone();
    inner(&mut omitted).owners.retain(|o| o.owner != passive());
    // The query still declares this exact producer. A dangling membership is
    // invalid package structure, not an empty grant inventory or a false flag.
    assert_eq!(
        omitted
            .checked_plan()
            .err()
            .expect("a declared member must retain its producer"),
        "invalid owned rule package structure: ordered member references an unknown producer program"
    );
    let mut partial = original.clone();
    inner(&mut partial).owner_mut(passive()).programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: passive(),
            facet: SchemaFacet::GameRules,
            code: key("counterfactual-unreviewed-gigantic"),
        }],
    };
    let p = partial.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.subject.as_ref() == Some(&passive())
                && g.reason == PlanGapReason::PartialPrograms)
    );
    unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    let mut actor = original.clone();
    inner(&mut actor).owner_mut(actor_owner()).programs.closure =
        original.actual_actor_coverage.clone();
    let p = actor.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.subject.as_ref() == Some(&actor_owner())
                && g.reason == PlanGapReason::PartialPrograms)
    );
    unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    let mut status = original.clone();
    inner(&mut status)
        .owner_mut(status_owner())
        .programs
        .closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: status_owner(),
            facet: SchemaFacet::GameRules,
            code: key("counterfactual-unreviewed-gigantic-status"),
        }],
    };
    let p = status.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.subject.as_ref() == Some(&status_owner())
                && g.reason == PlanGapReason::PartialPrograms)
    );
    unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
    let mut receivers = original.clone();
    receivers.sniper.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: status_owner(),
            facet: SchemaFacet::GameRules,
            code: key("counterfactual-unreviewed-gigantic-recipients"),
        }],
    };
    let p = receivers.plan();
    assert!(
        p.gaps()
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialReceivers)
    );
    unavailable(&p.evaluate(&mut p.new_scratch()).unwrap());
}

#[test]
#[ignore = "requires current Gigantic Boolean release; typed and stage authority controls"]
fn gigantic_numeric_flag_unlisted_source_and_early_consumers_are_rejected() {
    let original = World::load();
    let mut numeric = original.clone();
    let p = inner(&mut numeric)
        .owner_mut(passive())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == key(GRANT))
        .unwrap();
    p.nodes[0].expression = RuleExpression::Literal {
        value: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
    };
    let error = numeric
        .checked_plan()
        .err()
        .expect("numeric sentinel must not enter typed Flag");
    assert!(
        error.contains("Flag requires Boolean stat and contribution value"),
        "{error}"
    );
    let mut unlisted = original.clone();
    extra_flag(
        &mut unlisted,
        "counterfactual-unlisted-gigantic",
        Some(true),
        false,
    );
    query_mut(&mut unlisted).groups[0]
        .members
        .members
        .retain(|m| {
            m.producer.as_program_effect().unwrap().program
                != key("counterfactual-unlisted-gigantic")
        });
    let error = unlisted.checked_plan().err().unwrap();
    assert!(
        error.contains("actual contribution has no declared membership"),
        "{error}"
    );
    for program in [STATUS, BENEFITS] {
        let error = original
            .checked_plan_configured(|stages| {
                stages
                    .programs
                    .members
                    .iter_mut()
                    .find(|p| p.program == key(program))
                    .unwrap()
                    .stage = key("facts");
            })
            .err()
            .expect("consumer cannot precede its real input/freeze");
        assert!(
            error.contains("stage") || error.contains("frozen"),
            "{error}"
        );
    }
}

fn unrelated(w: &mut World) -> (DeclaredSlot<ActorSlotDefId>, StatDefId) {
    let actor: DeclaredSlot<ActorSlotDefId> = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(d(0x12)),
        slot: def("counterfactual.gigantic-unrelated-actor"),
    };
    let grant: DeclaredSlot<GrantSlotDefId> = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(d(0x12)),
        slot: def("counterfactual.gigantic-unrelated-grant"),
    };
    let sentinel: StatDefId = def("counterfactual.gigantic-unrelated-present");
    let f = inner(w);
    let descriptor = f
        .schema
        .definitions
        .iter_mut()
        .find(|row| row.address() == d::<SkillDefinition>(0x12).address())
        .unwrap();
    let DefinitionDescriptor::Skill(DefinitionEntry {
        schema: SchemaState::Known(schema),
        ..
    }) = descriptor
    else {
        panic!("actual parent Skill")
    };
    schema.declarations.actors.members.push(actor.clone());
    schema.declarations.grants.members.push(grant.clone());
    f.schema.slots.extend([
        SlotDescriptor::Actor(DefinitionEntry {
            id: actor.clone(),
            schema: SchemaState::Known(ActorSlotSchema {
                provider_definition: None,
                skills: DeclaredSet::complete(vec![]),
                outputs: DeclaredSet::complete(vec![]),
            }),
        }),
        SlotDescriptor::Grant(DefinitionEntry {
            id: grant.clone(),
            schema: SchemaState::Known(GrantSlotSchema {
                provider_roles: vec![ProviderRole::SkillUse],
                target: GrantTarget::Actor(actor.clone()),
            }),
        }),
    ]);
    f.schema
        .definitions
        .push(DefinitionDescriptor::Stat(DefinitionEntry {
            id: sentinel.clone(),
            schema: SchemaState::Known(StatSchema {
                value: ComputedValueType::Boolean,
                targets: vec![RuleEntityKind::Actor],
            }),
        }));
    f.owner_mut(SchemaSubject::Slot(SlotAddress::Grant(grant.clone())));
    f.owner_mut(SchemaSubject::Slot(SlotAddress::Actor(actor.clone())))
        .programs
        .members
        .push(RuleProgram {
            id: key("counterfactual-unrelated-present"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("present"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                },
            }],
            effects: vec![RuleEffect {
                id: key("present"),
                when: None,
                effect: RuleEffectKind::Derive {
                    entity: RuleEntity::Current,
                    stat: sentinel.clone(),
                    value: key("present"),
                },
            }],
        });
    f.owner_mut(subject(d::<SkillDefinition>(0x12)))
        .programs
        .members
        .push(RuleProgram {
            id: key("counterfactual-unrelated-supply"),
            context: RuleEntityKind::Skill,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("active"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Boolean(true),
                },
            }],
            effects: vec![RuleEffect {
                id: key("activate"),
                when: None,
                effect: RuleEffectKind::ActivateGrant {
                    slot: grant,
                    enabled: key("active"),
                },
            }],
        });
    (actor, sentinel)
}

#[test]
#[ignore = "requires current Gigantic Boolean release; exact Actor recipient scope"]
fn gigantic_excludes_proven_unrelated_actor_slots_and_the_player() {
    let mut w = World::load();
    let (slot, sentinel) = unrelated(&mut w);
    let report = w.evaluate();
    check(&w, &report, true, true, 1);
    let r = effects(&report);
    let sentinels: Vec<_> = r
        .values
        .iter()
        .filter(|v| matches!(&v.key, PlanValueKey::Stat { stat, .. } if *stat == sentinel))
        .collect();
    assert_eq!(
        sentinels.len(),
        2,
        "the unrelated actors were actually instantiated"
    );
    for row in sentinels {
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
        let PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(ActorKey::Owned(actor)),
            ..
        } = &row.key
        else {
            panic!("real unrelated owned Actor")
        };
        assert_eq!(actor.slot, slot);
        let entity = ConcreteEntity::Actor(ActorKey::Owned(actor.clone()));
        assert!(!r.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { entity: e, stat } if e == &entity && *stat == d(0x3308))));
        assert!(!benefit_rows(r).iter().any(
            |e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == entity)
        ));
    }
}

#[test]
#[ignore = "requires current Gigantic Boolean release; actual selection and scratch determinism"]
fn gigantic_real_selection_and_independent_actor_inputs_survive_reuse_and_rayon() {
    let a = World::load();
    let mut b = a.clone();
    remove(&mut b);
    b.sniper.raw(1, 1, 20., 0.);
    let pa = a.plan();
    let pb = b.plan();
    let mut scratch = pa.new_scratch();
    let first = pa.evaluate(&mut scratch).unwrap();
    check(&a, &first, true, true, 1);
    let second = pb.evaluate(&mut scratch).unwrap();
    check(&b, &second, false, false, 0);
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), first);
    assert_eq!(pb.evaluate(&mut pb.new_scratch()).unwrap(), second);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(
                || pa.new_scratch(),
                |scratch, i| {
                    if i % 2 == 0 {
                        pa.evaluate(scratch).unwrap()
                    } else {
                        pb.evaluate(scratch).unwrap()
                    }
                },
            )
            .collect::<Vec<_>>()
    });
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(report, if i % 2 == 0 { &first } else { &second });
    }
}

#[test]
#[ignore = "requires current Gigantic Boolean release; explicit disabled-root query selection"]
fn gigantic_disabled_root_is_absent_when_only_the_survivor_is_requested() {
    let a = World::load();
    let mut b = a.clone();
    let disabled = id(7201);
    inner(&mut b)
        .build
        .skills
        .iter_mut()
        .find(|s| s.id == disabled)
        .unwrap()
        .enabled = false;
    let root = ProviderRoot::SkillUse(disabled);
    // The default request intentionally retains this unavailable child and is
    // tested by the parent. This control explicitly asks only for the survivor.
    let pb = b.checked_plan_selecting(|_| {}, |requests| {
        requests.retain(|q| !matches!(&q.target, MetricTarget::Action(action) if action.action.provider.root == root));
    }).unwrap();
    assert!(pb.gaps().is_empty(), "{:?}", pb.gaps());
    let pa = a.plan();
    let mut scratch = pa.new_scratch();
    let original = pa.evaluate(&mut scratch).unwrap();
    check(&a, &original, true, true, 1);
    let disabled_report = pb.evaluate(&mut scratch).unwrap();
    check_actors(&b, &disabled_report, true, true, 1, &[0]);
    let r = effects(&disabled_report);
    let absent = ConcreteEntity::Actor(b.sniper.actor(1));
    assert!(!r.values.iter().any(|v| matches!(&v.key, PlanValueKey::Stat { entity, stat } if entity == &absent && *stat == d(0x3308))));
    assert!(!benefit_rows(r).iter().any(
        |e| matches!(&e.target, BoundEffectTarget::Contribution { key } if key.entity == absent)
    ));
    assert_eq!(pa.evaluate(&mut scratch).unwrap(), original);
}
