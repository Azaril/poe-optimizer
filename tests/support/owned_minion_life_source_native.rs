//! Reuse the existing intrinsic fixture; its explicit final-parent producer is
//! a component boundary, never an admitted real-build default or scalar Life pool.
use super::{family, gigantic, intrinsic, release};
use intrinsic::{World, actor, actor_slot, def, known, quantity};
use poe_optimizer_core::{owned_build::*, owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_engine::owned_plan::*;
use poe_optimizer_import::{
    owned_release::StagedOwnedRelease, owned_release_migration::OwnedReleaseMigrationInput,
};
use rayon::prelude::*;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};

const PROGRAM: &str = "intrinsic-allied-minion-life";
fn actor_owner() -> SchemaSubject {
    SchemaSubject::Slot(SlotAddress::Actor(actor_slot()))
}
fn endpoint() -> StagedOwnedRelease {
    let path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_MINION_LIFE_SOURCE_RELEASE")
            .expect("actual published intrinsic Life package"),
    );
    let endpoint = release::load(&path);
    family::assert_endpoint(&endpoint);
    endpoint
}
fn install(world: &mut World, endpoint: &StagedOwnedRelease) {
    let recipe = &endpoint.input().recipe;
    // Install the current published population programs. The historical
    // intrinsic fixture predates their facts/requirements split; retaining its
    // combined body would exercise a retired producer in this new component.
    let parent = SchemaSubject::Definition(def::<SkillDefinition>(0x12).address());
    let actual_parent = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == parent)
        .unwrap();
    let actual_level = actual_parent
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "ordinary-population-inputs")
        .unwrap();
    let actual_requirements = actual_parent
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "ordinary-population-requirements")
        .unwrap();
    let population = &mut world.f.owner_mut(&parent).programs.members;
    let previous = population
        .iter_mut()
        .find(|p| p.id == actual_level.id)
        .unwrap();
    *previous = actual_level.clone();
    assert!(!population.iter().any(|p| p.id == actual_requirements.id));
    population.push(actual_requirements.clone());
    for name in ["sniper.actor-level", "sniper.required-character-level"] {
        let table = recipe
            .rules
            .tables
            .iter()
            .find(|t| t.id.as_str() == name)
            .unwrap();
        assert!(world.f.tables.contains(table));
    }
    for address in [
        def::<UnitDefinition>(0x3119).address(),
        def::<StatDefinition>(0x311a).address(),
        def::<StatDefinition>(0x330b).address(),
    ] {
        let actual = recipe
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
            .unwrap();
        if let Some(old) = world
            .f
            .schema
            .definitions
            .iter()
            .find(|d| d.address() == address)
        {
            assert_eq!(old, actual);
        } else {
            world.f.schema.definitions.push(actual.clone());
        }
    }
    let migration: OwnedReleaseMigrationInput = family::read("migration.json");
    assert!(migration.schema.is_empty() && migration.receivers.is_empty());
    assert_eq!((migration.tables.len(), migration.owners.len()), (1, 1));
    for table in migration.tables {
        assert!(recipe.rules.tables.contains(&table));
        assert!(!world.f.tables.iter().any(|t| t.id == table.id));
        world.f.tables.push(table);
    }
    let addition = &migration.owners[0];
    assert_eq!(addition.owner, actor_owner());
    assert!(!addition.programs.is_complete());
    assert_eq!(addition.programs.members.len(), 1);
    let actual = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(!actual.programs.is_complete());
    assert_eq!(addition.programs.members[0].id.as_str(), PROGRAM);
    for program in &addition.programs.members {
        assert!(actual.programs.members.contains(program));
        assert!(
            !world
                .f
                .owner_mut(&actor_owner())
                .programs
                .members
                .iter()
                .any(|p| p.id == program.id)
        );
        world
            .f
            .owner_mut(&actor_owner())
            .programs
            .members
            .push(program.clone());
    }
}
fn world(levels: [u16; 2]) -> World {
    let endpoint = endpoint();
    let mut world = World::new(levels);
    install(&mut world, &endpoint);
    world
}
fn rows(report: &OwnedEffectsReport) -> Vec<&BoundEffectResult> {
    report
        .effects
        .iter()
        .filter(|e| e.key.invocation.program.as_str() == PROGRAM)
        .collect()
}
fn life(report: &OwnedEffectsReport, index: usize) -> &EffectValue {
    let expected = actor(index);
    let rows:Vec<_>=rows(report).into_iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(expected.clone()) && key.stat==def(0x311a) && key.kind==ContributionKind::Add)).collect();
    assert_eq!(rows.len(), 1);
    let row = rows[0];
    assert_eq!(row.key.invocation.owner, actor_owner());
    let RuleOrigin::Provider { provider } = &row.key.invocation.origin else {
        panic!("actual Actor provider")
    };
    let ActorKey::Owned(owned) = expected else {
        unreachable!()
    };
    let mut expected_provider = owned.provider;
    expected_provider
        .grant_path
        .push(intrinsic::slot(SlotOwnerDefId::Skill(def(0x12)), 0x20));
    assert_eq!(provider, &expected_provider);
    &row.value
}
fn check(report: &OwnedEffectsReport, expected: [f64; 2]) {
    assert!(report.gaps.is_empty(), "{:?}", report.gaps);
    assert_eq!(rows(report).len(), 2);
    for (index, value) in expected.into_iter().enumerate() {
        assert_eq!(
            life(report, index),
            &EffectValue::Known {
                value: quantity(value, 0x3119)
            }
        );
    }
    assert!(
        !report
            .values
            .iter()
            .any(|v| matches!(&v.key,PlanValueKey::Stat{stat,..} if *stat==def(0x311a))),
        "a base contribution is not the final scalar Life pool"
    );
    assert!(!rows(report).iter().any(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.entity==ConcreteEntity::Actor(ActorKey::Player))));
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn original_call_controls_match_the_published_program_on_independent_occurrences() {
    let vectors: Value = family::read("source-vectors.json");
    let cases = vectors["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 9);
    for case in cases {
        let level = u16::try_from(case["parent_level"].as_u64().unwrap()).unwrap();
        let world = world([level, level]);
        let report = world.evaluate();
        check(&report, [case["base_life"].as_f64().unwrap(); 2]);
        for index in 0..2 {
            assert_eq!(
                intrinsic::actor_value(&report, index, 0x1c),
                &EffectValue::Known {
                    value: ParameterValue::Integer(
                        BoundedInteger::new(case["actor_level"].as_i64().unwrap()).unwrap()
                    )
                }
            );
        }
    }
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn injected_curve_and_profile_scale_are_used_before_flooring() {
    let mut world = world([22, 1]);
    // Deliberate mathematical counterexample, not a new admitted minion profile.
    let table = world
        .f
        .tables
        .iter_mut()
        .find(|t| t.id.as_str() == "actor.allied-life-by-level")
        .unwrap();
    for row in &mut table.rows {
        *row = quantity(10.99, 0x3119);
    }
    let program = world
        .f
        .owner_mut(&actor_owner())
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == PROGRAM)
        .unwrap();
    let factors:Vec<_>=program.nodes.iter_mut().filter(|n|matches!(&n.expression,RuleExpression::Literal{value:ParameterValue::Quantity(q)} if q.unit()==&def(1))).collect();
    assert_eq!(factors.len(), 1);
    for node in factors {
        node.expression = RuleExpression::Literal {
            value: quantity(1.5, 1),
        };
    }
    check(&world.evaluate(), [16., 16.]); // floor(10.99 * 1.5), not floor(10.99) * 1.5.
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn missing_or_invalid_parent_inputs_and_partial_owners_do_not_default() {
    let mut missing = world([22, 1]);
    missing.missing_final_input();
    let report = missing.evaluate();
    for index in 0..2 {
        assert!(matches!(
            life(&report, index),
            EffectValue::Unresolved { .. }
        ));
    }
    let invalid = world([41, 1]).evaluate();
    assert!(matches!(
        life(&invalid, 0),
        EffectValue::Unresolved {
            reason: PlanGapReason::UpstreamUnavailable,
            ..
        }
    ));
    assert!(matches!(life(&invalid, 1), EffectValue::Known { .. }));
    let mut partial = world([22, 1]);
    let migration: OwnedReleaseMigrationInput = family::read("migration.json");
    partial.f.owner_mut(&actor_owner()).programs.closure =
        migration.owners[0].programs.closure.clone();
    let report = partial.evaluate();
    assert!(
        report
            .gaps
            .iter()
            .any(|g| g.reason == PlanGapReason::PartialPrograms)
    );
    for index in 0..2 {
        assert!(matches!(
            life(&report, index),
            EffectValue::Unresolved { .. }
        ));
    }
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn lookup_domain_has_explicit_boundaries_without_clamping() {
    for level in [0, 1, 100, 101] {
        let mut world = world([22, 1]);
        // Replace only the fixture's level producer to probe the table's domain.
        // The real physical-skill domain remains governed by its separate table.
        let parent = SchemaSubject::Definition(def::<SkillDefinition>(0x12).address());
        let program = world
            .f
            .owner_mut(&parent)
            .programs
            .members
            .iter_mut()
            .find(|p| p.id.as_str() == "ordinary-population-inputs")
            .unwrap();
        program
            .nodes
            .iter_mut()
            .find(|n| n.id.as_str() == "actor-level")
            .unwrap()
            .expression = RuleExpression::Literal {
            value: ParameterValue::Integer(BoundedInteger::new(level).unwrap()),
        };
        let report = world.evaluate();
        for index in 0..2 {
            if level == 0 || level == 101 {
                assert!(!matches!(life(&report, index), EffectValue::Known { .. }));
            } else {
                assert!(known(life(&report, index)) > 0.);
            }
        }
        if level == 0 || level == 101 {
            assert!(report.values.iter().any(|v|matches!(&v.value,EffectValue::UnsupportedValue{value:ParameterValue::Integer(n)} if n.get()==level)));
        }
    }
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn independent_roots_and_reused_parallel_scratch_preserve_life_sources() {
    let mut world = world([22, 1]);
    let plan = Arc::new(world.compile());
    let mut scratch = plan.new_scratch();
    let first = plan.evaluate(&mut scratch).unwrap();
    assert_ne!(life(&first, 0), life(&first, 1));
    let removed_root = ProviderRoot::SkillUse(world.f.build.skills[1].id);
    world.f.build.skills[1].enabled = false;
    world.f.queries.requests.retain(
        |q| !matches!(&q.target,MetricTarget::Action(a) if a.action.provider.root==removed_root),
    );
    let second = world.compile().evaluate(&mut scratch).unwrap();
    assert!(second.gaps.is_empty());
    assert_eq!(rows(&second).len(), 1);
    assert_eq!(life(&first, 0), life(&second, 0));
    assert_eq!(first, plan.evaluate(&mut scratch).unwrap());
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let reports: Vec<_> = pool.install(|| {
        (0..16)
            .into_par_iter()
            .map_init(|| plan.new_scratch(), |s, _| plan.evaluate(s).unwrap())
            .collect()
    });
    assert!(reports.iter().all(|r| r == &first));
}
#[test]
#[ignore = "requires the actual published intrinsic Life endpoint"]
fn migrated_gigantic_uses_the_shared_life_stat_without_an_alias_or_duplicate() {
    let endpoint = endpoint();
    let mut world = gigantic::from_endpoint(&endpoint);
    install(&mut world.intrinsic, &endpoint);
    let replacement: Value = family::read("replacement.json");
    let migrated: RuleProgram = serde_json::from_value(replacement["after"].clone()).unwrap();
    assert_eq!(migrated.id.as_str(), "gigantic-life-and-damage");
    let actual = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner())
        .unwrap();
    assert!(actual.programs.members.contains(&migrated));
    world
        .intrinsic
        .f
        .owner_mut(&actor_owner())
        .programs
        .members
        .push(migrated);
    let first = world.evaluate();
    assert!(first.gaps.is_empty());
    let before = [life(&first, 0).clone(), life(&first, 1).clone()];
    let check_more = |report: &OwnedEffectsReport, active: bool| {
        for (index, base) in before.iter().enumerate() {
            let rows:Vec<_>=report.effects.iter().filter(|e|matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==def(0x311a) && key.kind==ContributionKind::Multiply && key.entity==ConcreteEntity::Actor(actor(index)))).collect();
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0].value,
                if active {
                    EffectValue::Known {
                        value: quantity(1.2, 1),
                    }
                } else {
                    EffectValue::Inactive
                }
            );
            assert_eq!(life(report, index), base);
        }
        assert!(!report.effects.iter().any(
            |e| matches!(&e.target,BoundEffectTarget::Contribution{key} if key.stat==def(0x330a))
        ));
    };
    check_more(&first, true);
    let allocations = world.intrinsic.f.build.allocations.clone();
    world.intrinsic.f.build.allocations.clear();
    check_more(&world.evaluate(), false);
    world.intrinsic.f.build.allocations = allocations;
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
    check_more(&world.evaluate(), true);
    assert!(
        !serde_json::to_string(&endpoint.input().recipe.rules)
            .unwrap()
            .contains("def.000000000000330a")
    );
    // Historical identity persists, but there is no live alias or second writer.
    assert!(
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .any(|d| d.address() == def::<StatDefinition>(0x330a).address())
    );
}
