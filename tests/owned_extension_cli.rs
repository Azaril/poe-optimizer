//! Owned local arithmetic is data; publication does not certify whole item coverage.
#[path = "support/owned_actor_baselines.rs"]
mod actor_baselines;
#[path = "support/owned_canonical_admission.rs"]
mod canonical_admission;
#[path = "support/owned_cold_family.rs"]
mod cold_family;
#[path = "support/owned_elemental_weapons.rs"]
mod elemental_weapons;
#[path = "support/owned_item_headers.rs"]
mod headers;
#[path = "support/owned_item_layouts.rs"]
mod item_layouts;
#[path = "support/owned_item_metadata.rs"]
mod item_metadata;
#[path = "support/owned_item_observations.rs"]
mod item_observations;
#[path = "support/owned_local_modifiers.rs"]
mod local_modifiers;
#[path = "support/owned_local_scaling.rs"]
mod local_scaling;
#[path = "support/owned_modifier_transforms.rs"]
mod modifier_transforms;
#[path = "support/owned_modifier_values.rs"]
mod modifier_values;
#[path = "support/owned_intrinsic_predecessor.rs"]
mod predecessor;
#[path = "support/owned_source_conditions.rs"]
mod source_conditions;
#[path = "support/owned_source_role_migration.rs"]
mod source_role_migration;
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
#[path = "support/owned_weapon_catalyst.rs"]
mod weapon_catalysts;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::FiniteQuantity,
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_schema::*,
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
use support::{bundle, data, json, normalize, success};
fn command(cwd: &Path, name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(cwd).arg(name);
    c
}
fn profiles(cwd: &Path) -> PathBuf {
    let mut prior = predecessor::intrinsic_predecessor(cwd);
    for (name, folder) in [
        ("compile-owned-item-bases", "item-bases"),
        ("compile-owned-weapon-profiles", "weapon-profiles"),
    ] {
        let output = cwd.join(folder);
        let mut c = command(cwd, name);
        c.arg(prior)
            .arg("--catalog")
            .arg(data().join(folder).join("catalog.json"))
            .arg("--policy")
            .arg(data().join(folder).join("policy.json"))
            .arg("--definitions")
            .arg(data().join(folder).join("definitions.json"))
            .arg("--output")
            .arg(&output);
        if folder == "weapon-profiles" {
            c.arg("--base-catalog")
                .arg(data().join("item-bases/catalog.json"));
        }
        success(c.output().unwrap());
        prior = output;
    }
    prior
}
fn run(cwd: &Path, prior: &Path, extension: &Path, output: &Path) -> Output {
    command(cwd, "extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
#[test]
fn local_recipe_data_publishes_evaluates_and_preserves_original_gaps() {
    let temp = tempfile::tempdir().unwrap();
    let cwd = temp.path();
    let prior = profiles(cwd);
    let before = bundle(&prior);
    let extension = data().join("local-weapon-inputs/extension.json");
    let output = cwd.join("local");
    let report = success(run(cwd, &prior, &extension, &output));
    assert_eq!(report["extension"]["allocated_entries"], 8);
    assert_eq!(report["extension"]["refined_subjects"], 337);
    assert_eq!(report["extension"]["appended_receivers"], 4);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["schema_refinement"]["schema_version"],
        3
    );
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let recipe = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry: serde_json::from_value(json(output.join("registry.json"))).unwrap(),
            schema: serde_json::from_value(json(output.join("schema.json"))).unwrap(),
            rules: serde_json::from_value(json(output.join("rules.json"))).unwrap(),
            routing: serde_json::from_value(json(output.join("routing.json"))).unwrap(),
        },
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let mut scratch = compiled.new_scratch();
    // Independently reviewed source Item.lua local arithmetic examples. These facts
    // do not assert that the importer has reconstructed the original rune lifecycle.
    for (program, values, expected) in [
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.4),
                ("quality", 20.0),
                ("quality-effect", 0.0),
                ("increase", 49.0),
            ],
            2.09,
        ),
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.4),
                ("quality", 20.0),
                ("quality-effect", 2.0),
                ("increase", 49.0),
            ],
            2.16,
        ),
        (
            "pre-override-attack-rate",
            vec![
                ("raw", 1.005),
                ("quality", 0.0),
                ("quality-effect", 0.0),
                ("increase", 0.0),
            ],
            1.0,
        ),
        (
            "pre-override-critical-chance",
            vec![
                ("raw", 12.0),
                ("flat", 2.43),
                ("quality", 20.0),
                ("quality-effect", 0.0),
                ("increase", 0.0),
            ],
            14.43,
        ),
        (
            "pre-override-critical-chance",
            vec![
                ("raw", 12.0),
                ("flat", 2.43),
                ("quality", 20.0),
                ("quality-effect", 2.0),
                ("increase", 0.0),
            ],
            15.87,
        ),
        (
            "rounded-physical-minimum",
            vec![
                ("raw", 55.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 0.0),
                ("increase", 203.0),
            ],
            200.0,
        ),
        (
            "rounded-physical-maximum",
            vec![
                ("raw", 91.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 0.0),
                ("increase", 203.0),
            ],
            331.0,
        ),
        (
            "rounded-physical-minimum",
            vec![
                ("raw", 55.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 1.0),
                ("increase", 203.0),
            ],
            167.0,
        ),
        (
            "rounded-physical-maximum",
            vec![
                ("raw", 91.0),
                ("flat", 0.0),
                ("quality", 20.0),
                ("alternate-quality", 1.0),
                ("increase", 203.0),
            ],
            276.0,
        ),
    ] {
        let receiver = recipe
            .rules()
            .input()
            .receivers
            .members
            .iter()
            .find(|r| r.id.as_str() == program)
            .unwrap();
        assert_eq!(receiver.targets.len(), 337);
        let owner = SchemaSubject::Definition(receiver.stat.address());
        let p = &recipe
            .rules()
            .input()
            .owners
            .iter()
            .find(|o| o.owner == owner)
            .unwrap()
            .programs
            .members[0];
        let facts: Vec<_> = values
            .into_iter()
            .map(|(name, value)| {
                let read = p.reads.iter().find(|r| r.id.as_str() == name).unwrap();
                let ComputedValueType::Quantity { unit } = &read.value_type else {
                    panic!()
                };
                RuleFact {
                    read: read.id.clone(),
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(value, unit.clone()).unwrap(),
                    ),
                }
            })
            .collect();
        let result = compiled
            .evaluate(&owner, &p.id, &facts, recipe.schema(), &mut scratch)
            .unwrap();
        assert!(
            matches!(&result.effects[0].disposition,EffectDisposition::Applied{value:ParameterValue::Quantity(v)} if v.value()==expected),
            "{program}: {result:?}"
        );
        let empty = compiled
            .evaluate(&owner, &p.id, &[], recipe.schema(), &mut scratch)
            .unwrap();
        assert!(matches!(
            empty.effects[0].disposition,
            EffectDisposition::Unresolved { .. }
        ));
    }
    // Definitions and rules are project-authored data. Changing a coefficient in
    // a fresh extension changes the calculation without adding a Rust dispatcher.
    let mut changed = json(&extension);
    let node = changed["owners"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|o| o["programs"]["members"][0]["id"] == "pre-override-attack-rate")
        .unwrap()["programs"]["members"][0]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "quality-divisor")
        .unwrap();
    node["expression"]["value"]["value"]["value"] = serde_json::json!(10);
    let changed_path = cwd.join("changed.json");
    fs::write(&changed_path, serde_json::to_vec(&changed).unwrap()).unwrap();
    assert!(
        !run(
            cwd,
            &output,
            &changed_path,
            &cwd.join("cannot-overwrite-rules")
        )
        .status
        .success()
    );
    let base_recipe = assemble_owned_recipe(
        OwnedRecipeInput {
            schema_version: 1,
            registry: serde_json::from_value(json(prior.join("registry.json"))).unwrap(),
            schema: serde_json::from_value(json(prior.join("schema.json"))).unwrap(),
            rules: serde_json::from_value(json(prior.join("rules.json"))).unwrap(),
            routing: serde_json::from_value(json(prior.join("routing.json"))).unwrap(),
        },
        Default::default(),
    )
    .unwrap();
    let changed: OwnedRecipeExtension = serde_json::from_value(changed).unwrap();
    let altered = extend_owned_recipe(&base_recipe, &changed, Default::default()).unwrap();
    let altered = assemble_owned_recipe(altered.successor, Default::default()).unwrap();
    let altered_compiled = CompiledRulePackage::compile(
        altered.rules().input(),
        altered.schema(),
        Default::default(),
    )
    .unwrap();
    let receiver = altered
        .rules()
        .input()
        .receivers
        .members
        .iter()
        .find(|r| r.id.as_str() == "pre-override-attack-rate")
        .unwrap();
    let owner = SchemaSubject::Definition(receiver.stat.address());
    let program = &altered
        .rules()
        .input()
        .owners
        .iter()
        .find(|o| o.owner == owner)
        .unwrap()
        .programs
        .members[0];
    let facts: Vec<_> = [
        ("raw", 1.4),
        ("quality", 20.0),
        ("quality-effect", 2.0),
        ("increase", 49.0),
    ]
    .into_iter()
    .map(|(name, value)| {
        let read = program
            .reads
            .iter()
            .find(|r| r.id.as_str() == name)
            .unwrap();
        let ComputedValueType::Quantity { unit } = &read.value_type else {
            panic!()
        };
        RuleFact {
            read: read.id.clone(),
            value: ParameterValue::Quantity(FiniteQuantity::new(value, unit.clone()).unwrap()),
        }
    })
    .collect();
    let changed_result = altered_compiled
        .evaluate(
            &owner,
            &program.id,
            &facts,
            altered.schema(),
            &mut altered_compiled.new_scratch(),
        )
        .unwrap();
    assert!(
        matches!(&changed_result.effects[0].disposition,EffectDisposition::Applied{value:ParameterValue::Quantity(v)} if v.value()==2.14)
    );
    for case in 1..=5 {
        let draft = cwd.join(format!("original-{case}"));
        assert_eq!(
            success(normalize(cwd, &output, case, &draft, true))["normalization_status"],
            "pending"
        );
        let draft = decode_draft(
            &fs::read(draft.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        assert!(
            draft
                .input()
                .allocations
                .members
                .iter()
                .all(|a| matches!(a.access, DraftAllocationAccess::Pending(_)))
        );
        assert!(
            draft
                .input()
                .items
                .members
                .iter()
                .all(|i| i.to_resolved().is_none())
        );
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(fs::read(output.join(&name)).unwrap(), before[&name]);
    }
    let rerun = cwd.join("rerun");
    let report = success(run(cwd, &output, &extension, &rerun));
    assert_eq!(report["extension"]["allocated_entries"], 0);
    assert_eq!(report["extension"]["refined_subjects"], 0);
    for (name, bytes) in bundle(&output) {
        if !["transition.json", "catalog-append.json"].contains(&name.as_str()) {
            assert_eq!(bytes, fs::read(rerun.join(&name)).unwrap(), "{name}");
        }
    }
    assert_eq!(bundle(&prior), before);
    assert!(!run(cwd, &output, &extension, &output).status.success());
    let headers = headers::check_headers(cwd, &output, recipe.schema());
    let local = local_modifiers::check_local_modifiers(cwd, &headers);
    let actor = actor_baselines::check_actor_baselines(cwd, &local);
    let transforms = modifier_transforms::check_modifier_transforms(cwd, &actor);
    let values = modifier_values::check_modifier_values(cwd, &transforms);
    let canonical = canonical_admission::check_canonical_admission(cwd, &values);
    let catalysts = weapon_catalysts::check_weapon_catalysts(cwd, &canonical);
    let local_scaled = local_scaling::check_local_scaling(cwd, &catalysts);
    let elemental = elemental_weapons::check_elemental_weapons(cwd, &local_scaled);
    let layouts = item_layouts::check_item_layouts(cwd, &elemental);
    let metadata = item_metadata::check_item_metadata(cwd, &layouts);
    let cold = cold_family::check_cold_family(cwd, &metadata);
    let conditions = source_conditions::check_source_conditions(cwd, &cold);
    let source_roles = source_role_migration::check_source_role_migration(cwd, &conditions);
    item_observations::check_item_observations(cwd, &source_roles);
}
