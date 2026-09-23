//! Published actor components execute natively without closing whole-build gaps.
use super::support::{bundle, data, json, normalize, success};
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, OwnedDefinitionKey, SlotOwnerDefId, StatDefId},
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::{RuleEffectKind, RuleEntity, RuleReadSource},
    owned_schema::{SchemaClosure, SchemaDefinitionId, SchemaSubject, SlotAddress},
};
use poe_optimizer_engine::owned_rules::{
    CompiledRulePackage, EffectDisposition, ProgramEvaluation, RuleFact,
};
use poe_optimizer_import::{
    owned_actor_baseline_recipe::{ActorBaselinePolicy, ActorScalarTarget},
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn compile(cwd: &Path, prior: &Path, catalog: &Path, policy: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("compile-owned-actor-baselines")
        .arg(prior)
        .arg("--catalog")
        .arg(catalog)
        .arg("--policy")
        .arg(policy)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn derived<'a>(result: &'a ProgramEvaluation, stat: &StatDefId) -> &'a EffectDisposition {
    &result
        .effects
        .iter()
        .find(|e| {
            matches!(&e.effect, RuleEffectKind::Derive { entity: RuleEntity::Current, stat: s, .. } if s == stat)
        })
        .unwrap()
        .disposition
}

pub fn check_actor_baselines(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("actor-baselines");
    let catalog = authored.join("catalog.json");
    let policy_path = authored.join("native-policy.json");
    let policy: ActorBaselinePolicy = serde_json::from_value(json(&policy_path)).unwrap();
    let before = bundle(prior);
    // The actor compiler cannot silently allocate missing stat identities.
    let missing = cwd.join("actor-undeclared-targets");
    assert!(
        !compile(cwd, prior, &catalog, &policy_path, &missing)
            .status
            .success()
    );
    assert!(!missing.exists());

    let stage = cwd.join("actor-stat-targets");
    let report = success(
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(cwd)
            .arg("extend-owned-recipe")
            .arg(prior)
            .arg("--extension")
            .arg(authored.join("native-extension.json"))
            .arg("--output")
            .arg(&stage)
            .output()
            .unwrap(),
    );
    assert_eq!(report["extension"]["allocated_entries"], 7);
    assert_eq!(report["extension"]["appended_programs"], 0);
    let staged_bytes = bundle(&stage);
    let staged = recipe(&stage);
    let output = cwd.join("actor-native-successor");
    let report = success(compile(cwd, &stage, &catalog, &policy_path, &output));
    assert_eq!(report["actor_baselines"]["actors"], 1);
    assert_eq!(report["actor_baselines"]["scalar_facts"], 6);
    assert_eq!(report["actor_baselines"]["curve_outputs"], 1);
    assert_eq!(report["actor_baselines"]["level_projections"], 0);
    assert_eq!(
        report["actor_baselines"]["extension"]["appended_programs"],
        1
    );
    assert_eq!(report["actor_baselines"]["extension"]["appended_tables"], 1);
    assert_eq!(
        report["actor_baselines"]["extension"]["allocated_entries"],
        0
    );
    assert_eq!(
        report["actor_baselines"]["remaining_coverage"][0]["child_skills"],
        2
    );
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );

    for file in [
        "registry.json",
        "schema.json",
        "routing.json",
        "tree-normalization.json",
        "items.json",
        "item-source.json",
    ] {
        assert_eq!(
            fs::read(output.join(file)).unwrap(),
            staged_bytes[file],
            "{file}"
        );
    }
    let published = recipe(&output);
    assert_eq!(published.registry, staged.registry);
    assert_eq!(published.schema, staged.schema);
    assert_eq!(published.routing, staged.routing);
    let binding = &policy.bindings[0];
    let actor_owner = SchemaSubject::Slot(SlotAddress::Actor(binding.actor.clone()));
    for previous in &staged.rules.owners {
        let current = published
            .rules
            .owners
            .iter()
            .find(|o| o.owner == previous.owner)
            .unwrap();
        assert_eq!(current.programs.closure, previous.programs.closure);
        for program in &previous.programs.members {
            assert!(current.programs.members.contains(program));
        }
        assert_eq!(
            current.programs.members.len(),
            previous.programs.members.len() + usize::from(current.owner == actor_owner)
        );
    }
    let actor_rules = published
        .rules
        .owners
        .iter()
        .find(|o| o.owner == actor_owner)
        .unwrap();
    assert!(
        matches!(&actor_rules.programs.closure, SchemaClosure::Partial { gaps } if !gaps.is_empty())
    );
    let actor_program = actor_rules
        .programs
        .members
        .iter()
        .find(|p| p.id == binding.program)
        .unwrap()
        .clone();
    assert_eq!(published.rules.receivers, staged.rules.receivers);
    for table in &staged.rules.tables {
        assert!(published.rules.tables.contains(table));
    }
    let checked = assemble_owned_recipe(published, Default::default()).unwrap();
    let compiled = CompiledRulePackage::compile(
        checked.rules().input(),
        checked.schema(),
        Default::default(),
    )
    .unwrap();
    let mut scratch = compiled.new_scratch();
    let curve = &binding.curves[0];
    let curve_read = actor_program.reads.iter().find(|r| {
        matches!(&r.source, RuleReadSource::Stat { entity: RuleEntity::Actor, stat } if stat == &curve.actor_level)
    }).unwrap();
    let SlotOwnerDefId::Skill(parent) = &binding.actor.declaration else {
        panic!("expected an explicitly bound creating skill")
    };
    // Evaluate the already-published parent projection, then feed that exact
    // typed fact to the new actor component. This is component evidence only;
    // the separate resolver tests cover occurrence binding and scheduling.
    for (gem_level, actor_level, damage) in [
        (1, 2, 4.4200000762939),
        (20, 40, 204.03999328613),
        (40, 80, 1776.4300537109),
    ] {
        let parent_result = compiled
            .evaluate(
                &SchemaSubject::Definition(parent.address()),
                &key("ordinary-population-inputs"),
                &[RuleFact {
                    read: key("level"),
                    value: integer(gem_level),
                }],
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        let projected = parent_result
            .effects
            .iter()
            .find_map(|effect| {
                if let RuleEffectKind::ProjectActorStat { actor, stat, .. } = &effect.effect
                    && actor == &binding.actor
                    && stat == &curve.actor_level
                    && let EffectDisposition::Applied { value } = &effect.disposition
                {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(projected, integer(actor_level));
        let facts = [RuleFact {
            read: curve_read.id.clone(),
            value: projected,
        }];
        let result = compiled
            .evaluate(
                &actor_owner,
                &binding.program,
                &facts,
                checked.schema(),
                &mut scratch,
            )
            .unwrap();
        assert_eq!(
            result,
            compiled
                .evaluate(
                    &actor_owner,
                    &binding.program,
                    &facts,
                    checked.schema(),
                    &mut scratch
                )
                .unwrap()
        );
        for (field, expected) in binding.fields.iter().zip([1.5, 1.15, 0.3, 5.0, 80.0]) {
            let ActorScalarTarget::Quantity { stat, unit } = &field.target else {
                panic!("expected numeric source scalar")
            };
            let EffectDisposition::Applied {
                value: ParameterValue::Quantity(value),
            } = derived(&result, stat)
            else {
                panic!("published scalar unavailable")
            };
            assert_eq!(value.value(), expected);
            assert_eq!(value.unit(), unit);
        }
        let ActorScalarTarget::Boolean { stat } = &binding.fields[5].target else {
            panic!("expected explicit source flag")
        };
        assert_eq!(
            derived(&result, stat),
            &EffectDisposition::Applied {
                value: ParameterValue::Boolean(true)
            }
        );
        let EffectDisposition::Applied {
            value: ParameterValue::Quantity(value),
        } = derived(&result, &curve.stat)
        else {
            panic!("published curve unavailable")
        };
        assert_eq!(value.value(), damage);
        assert_eq!(value.unit(), &curve.unit);
    }
    let absent = compiled
        .evaluate(
            &actor_owner,
            &binding.program,
            &[],
            checked.schema(),
            &mut scratch,
        )
        .unwrap();
    assert!(matches!(
        derived(&absent, &curve.stat),
        EffectDisposition::Unresolved { .. }
    ));

    let after = bundle(&output);
    assert!(
        !compile(cwd, &stage, &catalog, &policy_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), after);
    let stale_catalog = cwd.join("stale-actor-catalog.json");
    let mut bytes = fs::read(&catalog).unwrap();
    bytes.push(b' '); // Valid JSON, different exact catalog binding.
    fs::write(&stale_catalog, bytes).unwrap();
    let rejected = cwd.join("actor-stale-catalog");
    assert!(
        !compile(cwd, &stage, &stale_catalog, &policy_path, &rejected)
            .status
            .success()
    );
    assert!(!rejected.exists());
    let mut wrong = policy.clone();
    wrong.bindings[0].profile = "not-in-this-catalog".into();
    let wrong_path = cwd.join("wrong-actor-policy.json");
    fs::write(&wrong_path, serde_json::to_vec(&wrong).unwrap()).unwrap();
    let rejected = cwd.join("actor-wrong-policy");
    assert!(
        !compile(cwd, &stage, &catalog, &wrong_path, &rejected)
            .status
            .success()
    );
    assert!(!rejected.exists());

    let mut rows = 0;
    for case in 1..=5 {
        let queries = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&queries)).unwrap(), before[&queries]);
        rows += json(output.join(&queries)).as_array().unwrap().len();
        let normalized = cwd.join(format!("actor-original-{case}"));
        let report = success(normalize(cwd, &output, case, &normalized, true));
        assert_eq!(report["normalization_status"], "pending");
        let draft = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert!(!draft.input().allocations.members.is_empty());
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
    }
    assert_eq!(rows, 110);
    assert_eq!(bundle(prior), before);
    assert_eq!(bundle(&stage), staged_bytes);
    output
}
