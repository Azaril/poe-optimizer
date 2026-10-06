//! One shared finite status world, exact authored benefit body and test-owned
//! Product probes. Native products do not claim PoB's final rounded More bucket.
use super::{family, gigantic, release, status_family};
use gigantic::fixture::{self, World, def, intrinsic, known};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(intrinsic::actor_slot()))
}
fn key(name: &str) -> OwnedDefinitionKey {
    intrinsic::key(name)
}
fn named<K: DefinitionDomain>(name: &str) -> DefId<K> {
    DefId::parse(intrinsic::ns(), format!("fixture.gigantic.{name}")).unwrap()
}
fn quantity(n: f64) -> ParameterValue {
    intrinsic::quantity(n, 1)
}
fn product_stat(channel: u64) -> StatDefId {
    named(if channel == 0x330a {
        "life-product"
    } else {
        "damage-product"
    })
}
fn entry<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn scalar(stat: StatDefId, value: ComputedValueType) -> DefinitionDescriptor {
    DefinitionDescriptor::Stat(entry(
        stat,
        StatSchema {
            value,
            targets: vec![RuleEntityKind::Actor],
        },
    ))
}
fn probe(channel: u64) -> RuleProgram {
    let name = if channel == 0x330a {
        "fixture-life-product"
    } else {
        "fixture-damage-product"
    };
    RuleProgram {
        id: key(name),
        context: RuleEntityKind::Actor,
        reads: vec![RuleRead {
            id: key("product"),
            value_type: ComputedValueType::Quantity { unit: def(1) },
            source: RuleReadSource::Contributions {
                entity: RuleEntity::Current,
                stat: def(channel),
                contribution: ContributionKind::Multiply,
                reduction: ContributionReduction::Product,
                empty: quantity(1.0),
            },
        }],
        nodes: vec![RuleNode {
            id: key("product"),
            expression: RuleExpression::Read {
                input: key("product"),
            },
        }],
        effects: vec![RuleEffect {
            id: key("product"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: product_stat(channel),
                value: key("product"),
            },
        }],
    }
}
fn world() -> World {
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GIGANTIC_BENEFITS_RELEASE")
            .expect("actual published benefit endpoint"),
    );
    let endpoint = release::load(&path);
    family::assert_endpoint(&endpoint);
    let mut world = gigantic::from_endpoint(&endpoint);
    let recipe = &endpoint.input().recipe;
    let m: OwnedReleaseMigrationInput = family::read("migration.json");
    let f = &mut world.intrinsic.f;
    for entry in m.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            panic!("two actor Stat definitions")
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
    assert_eq!(m.owners.len(), 1);
    assert!(m.receivers.is_empty());
    let extension = &m.owners[0];
    assert_eq!(extension.owner, actor_owner());
    assert!(!extension.programs.is_complete());
    assert_eq!(extension.programs.members.len(), 1);
    let actual = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(!actual.programs.is_complete());
    for program in &extension.programs.members {
        assert!(actual.programs.members.contains(program));
        assert!(
            !f.owner_mut(&actor_owner())
                .programs
                .members
                .iter()
                .any(|p| p.id == program.id)
        );
        f.owner_mut(&actor_owner())
            .programs
            .members
            .push(program.clone());
    }
    for channel in [0x330a, 0x330b] {
        f.schema.definitions.push(scalar(
            product_stat(channel),
            ComputedValueType::Quantity { unit: def(1) },
        ));
        f.owner_mut(&actor_owner())
            .programs
            .members
            .push(probe(channel));
    }
    world
}
fn effects(report: &OwnedEffectsReport, channel: u64) -> Vec<&BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(
            |e| matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==def(channel)),
        )
        .collect()
}
fn product(report: &OwnedEffectsReport, index: usize, channel: u64) -> &EffectValue {
    &report
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(intrinsic::actor(index)),
                    stat: product_stat(channel),
                }
        })
        .unwrap()
        .value
}
fn check(report: &OwnedEffectsReport, active: bool, actors: &[usize], independent: bool) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    for channel in [0x330a, 0x330b] {
        let rows = effects(report, channel);
        assert_eq!(rows.len(), actors.len() * (1 + usize::from(independent)));
        for index in actors {
            let actor = intrinsic::actor(*index);
            let actual:Vec<_>=rows.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(actor.clone()))).collect();
            let gigantic: Vec<_> = actual
                .iter()
                .filter(|e| e.key.invocation.program.as_str() == "gigantic-life-and-damage")
                .collect();
            assert_eq!(gigantic.len(), 1);
            assert_eq!(gigantic[0].key.invocation.owner, actor_owner());
            let RuleOrigin::Provider { provider } = &gigantic[0].key.invocation.origin else {
                panic!("actual actor provider")
            };
            let ActorKey::Owned(owned) = &actor else {
                unreachable!()
            };
            let mut expected_provider = owned.provider.clone();
            expected_provider
                .grant_path
                .push(intrinsic::slot(SlotOwnerDefId::Skill(def(0x12)), 0x20));
            assert_eq!(provider, &expected_provider);
            assert_eq!(
                gigantic[0].value,
                if active {
                    EffectValue::Known {
                        value: quantity(1.2),
                    }
                } else {
                    EffectValue::Inactive
                }
            );
            for row in actual {
                assert!(
                    matches!(&row.target,BoundEffectTarget::Contribution{key} if key.kind==ContributionKind::Multiply)
                );
            }
            let expected = (if active { 1.2 } else { 1.0 }) * (if independent { 1.2 } else { 1.0 });
            assert_eq!(known(product(report, *index, channel)), expected);
        }
        assert!(!rows.iter().any(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(ActorKey::Player))));
        assert!(
            !report
                .values
                .iter()
                .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(channel))),
            "production contribution channels are not final scalars"
        );
    }
}
fn second_status_source(world: &mut World) {
    let grant = world
        .intrinsic
        .f
        .owner_mut(&gigantic::passive())
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "ordinary-minion-gigantic")
        .unwrap()
        .clone();
    let class = SchemaSubject::Definition(world.intrinsic.f.build.character.class.address());
    world
        .intrinsic
        .f
        .owner_mut(&class)
        .programs
        .members
        .push(grant);
}
fn independent_more(world: &mut World) {
    world
        .intrinsic
        .f
        .owner_mut(&actor_owner())
        .programs
        .members
        .push(RuleProgram {
            id: key("fixture-independent-more"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("factor"),
                expression: RuleExpression::Literal {
                    value: quantity(1.2),
                },
            }],
            effects: [0x330a, 0x330b]
                .into_iter()
                .enumerate()
                .map(|(i, channel)| RuleEffect {
                    id: key(if i == 0 { "life" } else { "damage" }),
                    when: None,
                    effect: RuleEffectKind::Contribute {
                        entity: RuleEntity::Current,
                        stat: def(channel),
                        contribution: ContributionKind::Multiply,
                        value: key("factor"),
                    },
                })
                .collect(),
        });
}
#[test]
#[ignore = "requires actual published Gigantic benefits endpoint"]
fn published_factors_follow_status_once_with_removal_restoration_and_duplicate_grants() {
    let mut world = world();
    let original = world.intrinsic.f.build.allocations.clone();
    let a = world.evaluate();
    check(&a, true, &[0, 1], false);
    fixture::check(&a, 0.0);
    world.intrinsic.f.build.allocations.clear();
    check(&world.evaluate(), false, &[0, 1], false);
    world.intrinsic.f.build.allocations = original;
    assert_eq!(world.evaluate(), a);
    second_status_source(&mut world);
    let duplicate = world.evaluate();
    assert_eq!(effects(&duplicate, 0x3307).len(), 2);
    check(&duplicate, true, &[0, 1], false);
}
#[test]
#[ignore = "requires actual published Gigantic benefits endpoint"]
fn finite_product_combines_an_independent_factor_without_stacking_duplicate_status() {
    let mut world = world();
    independent_more(&mut world);
    second_status_source(&mut world);
    check(&world.evaluate(), true, &[0, 1], true);
    world.intrinsic.f.build.allocations.clear();
    let class = SchemaSubject::Definition(world.intrinsic.f.build.character.class.address());
    world
        .intrinsic
        .f
        .owner_mut(&class)
        .programs
        .members
        .retain(|p| p.id.as_str() != "ordinary-minion-gigantic");
    check(&world.evaluate(), false, &[0, 1], true);
}
fn unresolved(report: &OwnedEffectsReport, reason: PlanGapReason) {
    assert!(
        report.gaps.iter().any(|g| g.reason == reason)
            || report
                .effects
                .iter()
                .any(|e| matches!(&e.value,EffectValue::Unresolved{reason:r,..} if *r==reason)),
        "{:?}",
        report.gaps
    );
    for index in 0..2 {
        for channel in [0x330a, 0x330b] {
            assert!(matches!(
                product(report, index, channel),
                EffectValue::Unresolved { .. }
            ));
            assert!(
                effects(report, channel)
                    .iter()
                    .all(|e| matches!(&e.value, EffectValue::Unresolved { .. })),
                "missing status does not choose inactive/default factors"
            );
        }
    }
}
#[test]
#[ignore = "requires actual published Gigantic benefits endpoint"]
fn missing_boolean_and_partial_input_or_actor_inventories_refuse_products() {
    let mut missing = world();
    missing
        .intrinsic
        .f
        .receivers
        .members
        .retain(|r| r.stat != def(0x3308));
    unresolved(&missing.evaluate(), PlanGapReason::MissingProducer);
    let mut partial = world();
    let dependencies: Value = status_family::read("dependencies.json");
    let old: Vec<DefinitionRules> = serde_json::from_value(dependencies["owners"].clone()).unwrap();
    *partial.intrinsic.f.owner_mut(&gigantic::passive()) = old
        .into_iter()
        .find(|o| o.owner == gigantic::passive())
        .unwrap();
    unresolved(&partial.evaluate(), PlanGapReason::PartialPrograms);
    let mut partial = world();
    let dependencies: Value = family::read("dependencies.json");
    let owners: Vec<DefinitionRules> =
        serde_json::from_value(dependencies["owners"].clone()).unwrap();
    let actual = owners
        .into_iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(!actual.programs.is_complete());
    partial
        .intrinsic
        .f
        .owner_mut(&actor_owner())
        .programs
        .closure = actual.programs.closure;
    unresolved(&partial.evaluate(), PlanGapReason::PartialPrograms);
    let mut partial = world();
    partial.intrinsic.f.receivers.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: gigantic::receiver(),
            facet: SchemaFacet::GameRules,
            code: key("fixture-unreviewed-benefit-receiver"),
        }],
    };
    unresolved(&partial.evaluate(), PlanGapReason::PartialReceivers);
}
#[test]
#[ignore = "requires actual published Gigantic benefits endpoint"]
fn per_root_disable_and_private_parallel_scratch_preserve_factor_provenance() {
    let mut world = world();
    let with = Arc::new(world.compile());
    world.intrinsic.f.build.skills[1].enabled = false;
    let root = ProviderRoot::SkillUse(world.intrinsic.f.build.skills[1].id);
    world
        .intrinsic
        .f
        .queries
        .requests
        .retain(|q| !matches!(&q.target,MetricTarget::Action(a) if a.action.provider.root==root));
    let without = world.compile();
    let mut scratch = with.new_scratch();
    let a = with.evaluate(&mut scratch).unwrap();
    check(&a, true, &[0, 1], false);
    let b = without.evaluate(&mut scratch).unwrap();
    check(&b, true, &[0], false);
    assert_eq!(with.evaluate(&mut scratch).unwrap(), a);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(|| with.new_scratch(), |s, _| with.evaluate(s).unwrap())
            .collect()
    });
    assert!(reports.iter().all(|r| r == &a));
}
// The extra actors have a distinct declaration with no minion profile, owned
// skill or numerical programs. A sentinel proves they were actually created.
fn unrelated_actors(world: &mut World) -> (DeclaredSlot<ActorSlotDefId>, StatDefId) {
    let slot = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(0x12)),
        slot: named("unrelated-actor"),
    };
    let grant = DeclaredSlot {
        declaration: SlotOwnerDefId::Skill(def(0x12)),
        slot: named("unrelated-grant"),
    };
    let sentinel = named("unrelated-present");
    let f = &mut world.intrinsic.f;
    let row = f
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == def::<SkillDefinition>(0x12).address())
        .unwrap();
    let DefinitionDescriptor::Skill(row) = row else {
        unreachable!()
    };
    let SchemaState::Known(schema) = &mut row.schema else {
        unreachable!()
    };
    schema.declarations.actors.members.push(slot.clone());
    schema.declarations.grants.members.push(grant.clone());
    f.schema.slots.push(SlotDescriptor::Actor(entry(
        slot.clone(),
        ActorSlotSchema {
            provider_definition: None,
            skills: DeclaredSet::complete(vec![]),
            outputs: DeclaredSet::complete(vec![]),
        },
    )));
    f.schema.slots.push(SlotDescriptor::Grant(entry(
        grant.clone(),
        GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Actor(slot.clone()),
        },
    )));
    f.schema
        .definitions
        .push(scalar(sentinel.clone(), ComputedValueType::Boolean));
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::Grant(grant.clone())),
        programs: DeclaredSet::complete(vec![]),
    });
    f.owners.push(DefinitionRules {
        owner: SchemaSubject::Slot(SlotAddress::Actor(slot.clone())),
        programs: DeclaredSet::complete(vec![RuleProgram {
            id: key("fixture-unrelated-present"),
            context: RuleEntityKind::Actor,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("true"),
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
                    value: key("true"),
                },
            }],
        }]),
    });
    f.owner_mut(&SchemaSubject::Definition(
        def::<SkillDefinition>(0x12).address(),
    ))
    .programs
    .members
    .push(RuleProgram {
        id: key("fixture-unrelated-supply"),
        context: RuleEntityKind::Actor,
        reads: vec![],
        nodes: vec![RuleNode {
            id: key("true"),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(true),
            },
        }],
        effects: vec![RuleEffect {
            id: key("activate"),
            when: None,
            effect: RuleEffectKind::ActivateGrant {
                slot: grant,
                enabled: key("true"),
            },
        }],
    });
    (slot, sentinel)
}
#[test]
#[ignore = "requires actual published Gigantic benefits endpoint"]
fn exact_actor_slot_excludes_proven_unrelated_actors_and_the_player() {
    let mut world = world();
    let (slot, sentinel) = unrelated_actors(&mut world);
    let report = world.evaluate();
    check(&report, true, &[0, 1], false);
    let sentinels: Vec<_> = report
        .values
        .iter()
        .filter(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==sentinel))
        .collect();
    assert_eq!(sentinels.len(), 2);
    for row in sentinels {
        assert_eq!(
            row.value,
            EffectValue::Known {
                value: ParameterValue::Boolean(true)
            }
        );
        let PlanValueKey::Stat {
            entity: ConcreteEntity::Actor(actor),
            ..
        } = &row.key
        else {
            panic!("owned sentinel")
        };
        let ActorKey::Owned(owned) = actor else {
            panic!("owned sentinel")
        };
        assert_eq!(owned.slot, slot);
        assert!(!report.values.iter().any(|v|matches!(&v.key,PlanValueKey::Stat{entity:ConcreteEntity::Actor(a),stat} if a==actor && *stat==def(0x3308))));
        for channel in [0x330a, 0x330b] {
            assert!(!effects(&report,channel).iter().any(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(actor.clone()))));
        }
    }
}
