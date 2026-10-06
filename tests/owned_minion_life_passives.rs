//! Two actual default owners; finite producer checks are not a full Life pool.
#[path = "support/owned_minion_life_passives.rs"]
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
fn two_plain_life_default_owners_have_exact_source_and_declaration_evidence() {
    family::check_authored();
}

#[test]
fn duplicate_or_changed_same_source_records_are_not_default_owner_proof() {
    let a = family::read("authoring.json");
    let b = family::read("bindings.json");
    for duplicate in [false, true] {
        let mut v: Value = family::read("source-vectors.json");
        for report in v["reports"].as_array_mut().unwrap() {
            let rows = report["observations"][0]["value"].as_array_mut().unwrap();
            let index = rows
                .iter()
                .position(|r| r["mod"]["source"] == "Tree:1218")
                .unwrap();
            if duplicate {
                rows.push(rows[index].clone());
            } else {
                rows[index]["mod"]["value"]["mod"]["value"] = json!(11);
            }
        }
        assert!(std::panic::catch_unwind(|| family::check_vectors(&a, &b, &v, false)).is_err());
    }
}

#[test]
#[ignore = "requires exact predecessor and immutable existing source02 reports"]
fn publish_two_life_owners_preserving_all_five_originals() {
    let output = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_PASSIVES_OUTPUT").expect("fresh output"),
    );
    publication::run_with_scope(
        PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_PASSIVES_PRIOR")
                .expect("checked predecessor"),
        ),
        output.clone(),
        &family::data(),
        &[],
        &["source-vectors.json"],
        family::stage,
        publication::PublicationScope {
            closed_existing_rule_owners: 2,
            passive_refinement: true,
            extra: json!({"new_definitions":0,"new_programs":2,"new_receivers":0,"whole_life_result":false,"effective_transforms_proved_absent":false}),
        },
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let bytes =
        std::fs::read(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap();
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
    let bindings: Value = family::read("bindings.json");
    for node in bindings["nodes"].as_array().unwrap() {
        assert_eq!(
            draft["draft"]["allocations"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|a| selected_ids.contains(&a["id"])
                    && a["node"] == json!({"kind":"known","value":node["definition"]}))
                .count(),
            1
        );
    }
}

#[path = "../crates/poe-optimizer-engine/tests/support/plain_minion_damage_fixture.rs"]
mod fixture;
mod native {
    use super::{PathBuf, Value, family, fixture, release};
    use fixture::{Node, World, def, known};
    use poe_optimizer_core::{owned_build::*, owned_rules::*, owned_schema::*};
    use poe_optimizer_engine::owned_plan::*;
    use rayon::prelude::*;
    use std::{collections::BTreeSet, sync::Arc};

    fn world() -> World {
        let path = PathBuf::from(
            std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_PASSIVES_RELEASE")
                .expect("actual published endpoint"),
        );
        let endpoint = release::load(&path);
        family::assert_endpoint(&endpoint);
        let d: Value = family::read("dependencies.json");
        let c: Value = family::read("closure.json");
        let mut owners: Vec<DefinitionRules> =
            serde_json::from_value(d["preserved_life_owners"].clone()).unwrap();
        owners.extend(serde_json::from_value::<Vec<DefinitionRules>>(c["owners"].clone()).unwrap());
        let nodes: Vec<_> = owners
            .iter()
            .map(|owner| {
                let SchemaSubject::Definition(address) = &owner.owner else {
                    panic!("passive definition")
                };
                let descriptor = endpoint
                    .input()
                    .recipe
                    .schema
                    .definitions
                    .iter()
                    .find(|d| d.address() == *address)
                    .unwrap();
                let DefinitionDescriptor::PassiveNode(node) = descriptor else {
                    panic!("passive")
                };
                let SchemaState::Known(schema) = &node.schema else {
                    panic!("known")
                };
                assert_eq!(schema.pools.members.len(), 1);
                Node {
                    node: node.id.clone(),
                    source_id: String::new(),
                    value: 0.0,
                    pool: schema.pools.members[0].clone(),
                }
            })
            .collect();
        assert_eq!(nodes.len(), 6);
        let mut world = World::new(&nodes);
        let f = &mut world.intrinsic.f;
        for owner in owners {
            assert!(owner.programs.is_complete());
            assert_eq!(
                endpoint
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .filter(|o| **o == owner)
                    .count(),
                1
            );
            let SchemaSubject::Definition(address) = &owner.owner else {
                unreachable!()
            };
            let actual = endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .find(|d| d.address() == *address)
                .unwrap();
            // Exact numerical bodies come from the authenticated publication.
            // Only adjacency/unrelated declarations use the shared finite fixture.
            let finite = fixture::finite_node(actual);
            if let Some(row) = f
                .schema
                .definitions
                .iter_mut()
                .find(|d| d.address() == *address)
            {
                *row = finite;
            } else {
                f.schema.definitions.push(finite);
            }
            if let Some(row) = f.owners.iter_mut().find(|o| o.owner == owner.owner) {
                *row = owner;
            } else {
                f.owners.push(owner);
            }
        }
        let stat = endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .find(|d| {
                d.address()
                    == def::<poe_optimizer_core::owned_definitions::StatDefinition>(0x32e5)
                        .address()
            })
            .unwrap()
            .clone();
        assert!(
            !f.schema
                .definitions
                .iter()
                .any(|d| d.address() == stat.address())
        );
        f.schema.definitions.push(stat);
        world
    }
    fn check(report: &OwnedEffectsReport, allocations: &[Allocation]) -> f64 {
        assert!(
            report.gaps.is_empty(),
            "finite source boundary has an unresolved gap"
        );
        let mut actual = BTreeSet::new();
        let mut sum = 0.0;
        for effect in &report.effects {
            if !matches!(&effect.target,BoundEffectTarget::Contribution{key} if key.stat==def(0x32e5))
            {
                continue;
            }
            let RuleOrigin::Provider { provider } = &effect.key.invocation.origin else {
                panic!("allocation origin")
            };
            let ProviderRoot::Allocation(id) = provider.root else {
                panic!("allocation root")
            };
            assert!(provider.grant_path.is_empty());
            assert!(actual.insert(id));
            let allocation = allocations.iter().find(|a| a.id == id).unwrap();
            let expected = if [def(0xaef), def(0x1372)].contains(&allocation.node) {
                10.0
            } else {
                6.0
            };
            assert_eq!(known(&effect.value), expected);
            sum += expected;
            assert_eq!(
                effect.target,
                BoundEffectTarget::Contribution {
                    key: ContributionKey {
                        entity: ConcreteEntity::Actor(ActorKey::Player),
                        stat: def(0x32e5),
                        kind: ContributionKind::Increase
                    }
                }
            );
        }
        assert_eq!(actual, allocations.iter().map(|a| a.id).collect());
        assert!(
            !report
                .values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x32e5))),
            "no implicit scalar reducer or received Life pool"
        );
        sum
    }
    #[test]
    #[ignore = "requires the actual published Life endpoint"]
    fn actual_six_producers_preserve_old_damage_and_each_new_node_removal() {
        let mut world = world();
        let all = world.intrinsic.f.build.allocations.clone();
        let report = world.evaluate();
        assert_eq!(check(&report, &all), 44.0);
        fixture::check(&report, 24.0);
        for node in [def(0xaef), def(0x1372)] {
            world.intrinsic.f.build.allocations =
                all.iter().filter(|a| a.node != node).cloned().collect();
            let report = world.evaluate();
            assert_eq!(check(&report, &world.intrinsic.f.build.allocations), 34.0);
            fixture::check(&report, 24.0);
        }
    }
    #[test]
    #[ignore = "requires the actual published Life endpoint"]
    fn partial_selected_owner_refuses_complete_effects_without_fabricating_life() {
        let mut world = world();
        let d: Value = family::read("dependencies.json");
        let old: Vec<DefinitionRules> = serde_json::from_value(d["owners"].clone()).unwrap();
        *world.intrinsic.f.owner_mut(&old[0].owner) = old[0].clone();
        let report = world.evaluate();
        assert!(
            report
                .gaps
                .iter()
                .any(|g| g.reason == PlanGapReason::PartialPrograms)
        );
        assert!(
            !report
                .values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x32e5)))
        );
        world
            .intrinsic
            .f
            .build
            .allocations
            .retain(|a| a.node != def(0xaef));
        assert_eq!(
            check(&world.evaluate(), &world.intrinsic.f.build.allocations),
            34.0
        );
    }
    #[test]
    #[ignore = "requires the actual published Life endpoint"]
    fn exact_allocation_provenance_survives_scratch_and_parallel_replays() {
        let mut world = world();
        let all = world.intrinsic.f.build.allocations.clone();
        let plan = Arc::new(world.compile());
        world
            .intrinsic
            .f
            .build
            .allocations
            .retain(|a| a.node != def(0x1372));
        let smaller = world.compile();
        let mut scratch = plan.new_scratch();
        let original = plan.evaluate(&mut scratch).unwrap();
        assert_eq!(check(&original, &all), 44.0);
        assert_eq!(
            check(
                &smaller.evaluate(&mut scratch).unwrap(),
                &world.intrinsic.f.build.allocations
            ),
            34.0
        );
        assert!(
            plan.evaluate(&mut scratch).unwrap() == original,
            "same-request A/B/A identity and values"
        );
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let reports: Vec<_> = pool.install(|| {
            (0..16)
                .into_par_iter()
                .map_init(
                    || plan.new_scratch(),
                    |scratch, _| plan.evaluate(scratch).unwrap(),
                )
                .collect()
        });
        assert!(reports.iter().all(|r| *r == original));
    }
}
