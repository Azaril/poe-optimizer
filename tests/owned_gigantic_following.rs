//! Published Gigantic default and receiver bodies in an explicitly finite world.
//! This proves status aggregation and owner-side reservation contributions only.
//! Actor Life/Damage factors, reservation recipients and whole builds stay open.
#[path = "support/owned_gigantic_following.rs"]
mod family;
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_passive_refinement_publication.rs"]
mod passive_publication;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_support_delivery_publication.rs"]
mod publication;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::path::PathBuf;

#[test]
fn gigantic_authors_one_complete_default_with_separate_status_and_efficiency() {
    family::check_authored();
}

#[test]
#[ignore = "requires immutable source reports and checked Gigantic predecessor"]
fn publish_gigantic_preserving_all_five_originals() {
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FOLLOWING_OUTPUT").expect("fresh output"),
    );
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FOLLOWING_PRIOR")
                .expect("checked Minion Life predecessor"),
        ),
        output.clone(),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 1,
            passive_refinement: true,
            extra: json!({"new_definitions":3,"new_programs":3,"new_receivers":1,
                "new_complete_stat_owners":1,"final_damage_claimed":false,
                "final_life_claimed":false,"reservation_delivery_claimed":false,
                "effective_transforms_proved_absent":false}),
        },
    );
    let bytes = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/builds/breadth-20260908/build-05.xml"),
    )
    .unwrap();
    let dir = output.join("original-05");
    let selection = selected::selection(&bytes, &dir);
    let draft: Value =
        serde_json::from_slice(&std::fs::read(dir.join("draft.json")).unwrap()).unwrap();
    let preset = draft["draft"]["allocation_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == selection["build"]["allocations"])
        .unwrap();
    assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
    let selected_ids = preset["allocations"]["members"].as_array().unwrap();
    assert_eq!(
        draft["draft"]["allocations"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| selected_ids.contains(&a["id"])
                && a["node"]["kind"] == "known"
                && a["node"]["value"]["key"] == "def.0000000000001532")
            .count(),
        1
    );
}

#[path = "../crates/poe-optimizer-engine/tests/support/plain_minion_damage_fixture.rs"]
mod fixture;
mod native {
    use super::{PathBuf, Value, family, fixture, release};
    use fixture::{Node, World, def, intrinsic, known};
    use poe_optimizer_core::{
        owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*,
    };
    use poe_optimizer_engine::owned_plan::*;
    use poe_optimizer_import::{
        owned_recipe_extension::SchemaExtensionEntry,
        owned_release_migration::OwnedReleaseMigrationInput,
    };
    use rayon::prelude::*;
    use std::sync::Arc;

    fn passive() -> SchemaSubject {
        SchemaSubject::Definition(def::<PassiveNodeDefinition>(0x1532).address())
    }
    fn receiver() -> SchemaSubject {
        SchemaSubject::Definition(def::<StatDefinition>(0x3308).address())
    }
    fn world() -> World {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_FOLLOWING_RELEASE")
                .expect("actual published Gigantic endpoint"),
        );
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let recipe = &endpoint.input().recipe;
        let descriptor = recipe
            .schema
            .definitions
            .iter()
            .find(|d| SchemaSubject::Definition(d.address()) == passive())
            .unwrap();
        let DefinitionDescriptor::PassiveNode(node) = descriptor else {
            panic!("passive")
        };
        let SchemaState::Known(schema) = &node.schema else {
            panic!("known passive")
        };
        assert_eq!(schema.pools.members.len(), 1);
        let mut world = World::new(&[Node {
            node: node.id.clone(),
            source_id: "46365".into(),
            value: 0.0,
            pool: schema.pools.members[0].clone(),
        }]);
        let f = &mut world.intrinsic.f;
        // Numerical rules remain byte-identical. Only graph adjacency and the
        // unrelated declarations of this existing finite fixture are narrowed.
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|d| d.address() == descriptor.address())
        );
        f.schema.definitions.push(fixture::finite_node(descriptor));
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == passive())
            .unwrap();
        assert!(owner.programs.is_complete());
        assert!(!f.owners.iter().any(|o| o.owner == owner.owner));
        f.owners.push(owner.clone());
        let migration: OwnedReleaseMigrationInput = family::read("migration.json");
        for entry in migration.schema {
            let SchemaExtensionEntry::Definition(row) = entry else {
                panic!("only stat definitions")
            };
            assert!(recipe.schema.definitions.contains(&row));
            assert!(
                !f.schema
                    .definitions
                    .iter()
                    .any(|d| d.address() == row.address())
            );
            f.schema.definitions.push(row);
        }
        for owner in migration.owners {
            assert!(recipe.rules.owners.contains(&owner));
            assert_eq!(owner.owner, receiver());
            assert!(owner.programs.is_complete());
            assert!(!f.owners.iter().any(|o| o.owner == owner.owner));
            f.owners.push(owner);
        }
        assert_eq!(migration.receivers.len(), 1);
        for receiver in migration.receivers {
            assert!(recipe.rules.receivers.members.contains(&receiver));
            f.receivers.members.push(receiver);
        }
        // A test-owned mandatory read makes missing status distinguishable from
        // an explicitly computed false. It supplies no status or game formula.
        f.owner_mut(&SchemaSubject::Slot(SlotAddress::Actor(
            intrinsic::actor_slot(),
        )))
        .programs
        .members
        .push(RuleProgram {
            id: intrinsic::key("fixture-status-read"),
            context: RuleEntityKind::Actor,
            reads: vec![RuleRead {
                id: intrinsic::key("active"),
                value_type: ComputedValueType::Boolean,
                source: RuleReadSource::Stat {
                    entity: RuleEntity::Current,
                    stat: def(0x3308),
                },
            }],
            nodes: vec![RuleNode {
                id: intrinsic::key("active"),
                expression: RuleExpression::Read {
                    input: intrinsic::key("active"),
                },
            }],
            effects: vec![RuleEffect {
                id: intrinsic::key("observe"),
                when: None,
                effect: RuleEffectKind::Requirement {
                    satisfied: intrinsic::key("active"),
                    code: intrinsic::key("fixture-gigantic-active"),
                },
            }],
        });
        world
    }
    fn contribution(report: &OwnedEffectsReport, stat: u64) -> Vec<&BoundEffectResult> {
        report
            .effects
            .iter()
            .filter(|e| {
                matches!(&e.target,
            BoundEffectTarget::Contribution{key} if key.stat==def(stat))
            })
            .collect()
    }
    fn probes(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
        report
            .effects
            .iter()
            .filter(|e| {
                matches!(&e.target,
            BoundEffectTarget::Requirement{code} if code.as_str()=="fixture-gigantic-active")
            })
            .collect()
    }
    fn check(world: &World, report: &OwnedEffectsReport, grants: usize, actors: &[usize]) {
        assert!(report.gaps.is_empty(), "{:?}", report.gaps);
        let status = grants > 0;
        let grants_rows = contribution(report, 0x3307);
        assert_eq!(grants_rows.len(), grants);
        let mut origins = vec![];
        for row in grants_rows {
            assert!(!origins.contains(&row.key.invocation.origin));
            origins.push(row.key.invocation.origin.clone());
            assert_eq!(
                row.value,
                EffectValue::Known {
                    value: ParameterValue::Integer(BoundedInteger::new(1).unwrap())
                }
            );
            assert_eq!(
                row.target,
                BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: def(0x3307),
                        kind: ContributionKind::Add
                    }
                }
            );
        }
        let efficiency = contribution(report, 0x3309);
        assert_eq!(efficiency.len(), world.intrinsic.f.build.allocations.len());
        for row in efficiency {
            assert_eq!(known(&row.value), -25.0);
            assert_eq!(
                row.target,
                BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: def(0x3309),
                        kind: ContributionKind::Increase
                    }
                }
            );
            let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
                panic!("passive origin")
            };
            let ProviderRoot::Allocation(id) = provider.root else {
                panic!("allocation")
            };
            assert_eq!(
                world
                    .intrinsic
                    .f
                    .build
                    .allocations
                    .iter()
                    .filter(|a| a.id == id && a.node == def(0x1532))
                    .count(),
                1
            );
            assert!(provider.grant_path.is_empty());
        }
        let states: Vec<_> = report
            .values
            .iter()
            .filter(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x3308)))
            .collect();
        assert_eq!(states.len(), actors.len());
        for index in actors {
            assert_eq!(
                fixture::actor_value(report, *index, 0x3308),
                &EffectValue::Known {
                    value: ParameterValue::Boolean(status)
                }
            );
        }
        assert!(!report.values.iter().any(|v|matches!(&v.key,
            PlanValueKey::Stat{entity:ConcreteEntity::Actor(ActorKey::Player),stat} if *stat==def(0x3308))));
        assert!(
            !report.values.iter().any(|v| matches!(&v.key,
            PlanValueKey::Stat{stat,..} if [def(0x3307),def(0x3309)].contains(stat))),
            "count and efficiency contributions are not fabricated final scalars"
        );
        assert_eq!(probes(report).len(), actors.len());
        for probe in probes(report) {
            assert_eq!(
                probe.value,
                EffectValue::Known {
                    value: ParameterValue::Boolean(status)
                }
            );
        }
    }
    #[test]
    #[ignore = "requires actual published Gigantic endpoint"]
    fn actual_default_removal_and_restoration_change_status_without_fabricating_pools() {
        let mut world = world();
        let original = world.intrinsic.f.build.allocations.clone();
        let a = world.evaluate();
        check(&world, &a, 1, &[0, 1]);
        fixture::check(&a, 0.0);
        world.intrinsic.f.build.allocations.clear();
        let b = world.evaluate();
        check(&world, &b, 0, &[0, 1]);
        fixture::check(&b, 0.0);
        world.intrinsic.f.build.allocations = original;
        assert_eq!(world.evaluate(), a);
    }
    #[test]
    #[ignore = "requires actual published Gigantic endpoint"]
    fn two_independent_grant_programs_produce_one_boolean_status_per_minion() {
        let mut world = world();
        let grant = world
            .intrinsic
            .f
            .owner_mut(&passive())
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == "ordinary-minion-gigantic")
            .unwrap()
            .clone();
        // Execute the exact published granting body from a second test-only
        // provider. No test writes a count or Boolean into the receiver.
        let class = SchemaSubject::Definition(world.intrinsic.f.build.character.class.address());
        world
            .intrinsic
            .f
            .owner_mut(&class)
            .programs
            .members
            .push(grant);
        let report = world.evaluate();
        check(&world, &report, 2, &[0, 1]);
        let origins: Vec<_> = contribution(&report, 0x3307)
            .iter()
            .map(|e| e.key.invocation.owner.clone())
            .collect();
        assert_eq!(origins.len(), 2);
        for expected in [passive(), class] {
            assert_eq!(
                origins.iter().filter(|owner| **owner == expected).count(),
                1
            );
        }
        world.intrinsic.f.build.allocations.clear();
        check(&world, &world.evaluate(), 1, &[0, 1]);
    }
    #[test]
    #[ignore = "requires actual published Gigantic endpoint"]
    fn independent_minion_occurrences_do_not_grant_status_to_player_or_disabled_root() {
        let mut world = world();
        assert_ne!(intrinsic::actor(0), intrinsic::actor(1));
        world.intrinsic.f.build.skills[1].enabled = false;
        let root = ProviderRoot::SkillUse(world.intrinsic.f.build.skills[1].id);
        world.intrinsic.f.queries.requests.retain(
            |q| !matches!(&q.target,MetricTarget::Action(a) if a.action.provider.root==root),
        );
        check(&world, &world.evaluate(), 1, &[0]);
    }
    fn unresolved_status(report: &OwnedEffectsReport, expected_gap: PlanGapReason) {
        assert!(report.gaps.iter().any(|g|g.reason==expected_gap) || report.effects.iter().any(|e|matches!(&e.value,EffectValue::Unresolved{reason,..} if *reason==expected_gap)),"{:?}",report.gaps);
        let probes = probes(report);
        assert_eq!(probes.len(), 2);
        assert!(
            probes
                .iter()
                .all(|p| matches!(p.value, EffectValue::Unresolved { .. })),
            "status is unavailable, never a false default"
        );
        assert!(!report.values.iter().any(
            |v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x3308))
                && matches!(v.value, EffectValue::Known { .. })
        ));
    }
    #[test]
    #[ignore = "requires actual published Gigantic endpoint"]
    fn partial_or_missing_owner_and_receiver_remain_unavailable() {
        let dependencies: Value = family::read("dependencies.json");
        let old: Vec<DefinitionRules> =
            serde_json::from_value(dependencies["owners"].clone()).unwrap();
        let old = old.into_iter().find(|o| o.owner == passive()).unwrap();
        assert!(!old.programs.is_complete());
        let mut partial = world();
        *partial.intrinsic.f.owner_mut(&passive()) = old;
        unresolved_status(&partial.evaluate(), PlanGapReason::PartialPrograms);
        let mut missing = world();
        missing.intrinsic.f.owners.retain(|o| o.owner != passive());
        unresolved_status(&missing.evaluate(), PlanGapReason::MissingPrograms);
        let mut missing = world();
        missing
            .intrinsic
            .f
            .receivers
            .members
            .retain(|r| r.stat != def(0x3308));
        unresolved_status(&missing.evaluate(), PlanGapReason::MissingProducer);
        let mut partial = world();
        partial.intrinsic.f.receivers.closure = SchemaClosure::Partial {
            gaps: vec![SchemaGap {
                subject: receiver(),
                facet: SchemaFacet::GameRules,
                code: intrinsic::key("fixture-unreviewed-gigantic-receiver"),
            }],
        };
        unresolved_status(&partial.evaluate(), PlanGapReason::PartialReceivers);
    }
    #[test]
    #[ignore = "requires actual published Gigantic endpoint"]
    fn same_request_a_b_a_and_four_worker_replays_preserve_status_and_provenance() {
        let mut world = world();
        let with = Arc::new(world.compile());
        let original = world.intrinsic.f.build.allocations.clone();
        world.intrinsic.f.build.allocations.clear();
        let without = world.compile();
        let mut scratch = with.new_scratch();
        let a = with.evaluate(&mut scratch).unwrap();
        world.intrinsic.f.build.allocations = original.clone();
        check(&world, &a, 1, &[0, 1]);
        let b = without.evaluate(&mut scratch).unwrap();
        world.intrinsic.f.build.allocations.clear();
        check(&world, &b, 0, &[0, 1]);
        assert_eq!(with.evaluate(&mut scratch).unwrap(), a);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let reports: Vec<_> = pool.install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || with.new_scratch(),
                    |scratch, _| with.evaluate(scratch).unwrap(),
                )
                .collect()
        });
        assert!(reports.iter().all(|r| r == &a));
    }
}
