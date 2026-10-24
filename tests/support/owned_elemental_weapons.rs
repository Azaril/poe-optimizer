//! Historical publication and wire-budget laws awaiting direct release migration.
use super::support::{bundle, data, json, success};
use poe_optimizer_core::{owned_content::MAX_OWNED_CONTENT_BYTES, owned_schema::SchemaClosure};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, RuleLimits};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: poe_optimizer_data::owned_schema::decode_schema_input(
            &std::fs::read(path.join("schema.json")).unwrap(),
            Default::default(),
        )
        .unwrap(),
        rules: serde_json::from_value(json(path.join("rules.json"))).unwrap(),
        routing: serde_json::from_value(json(path.join("routing.json"))).unwrap(),
    }
}
fn publish(cwd: &Path, prior: &Path, extension: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-recipe")
        .arg(prior)
        .arg("--extension")
        .arg(extension)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
pub fn check_elemental_weapons(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("elemental-weapon-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    assert_eq!(extension.schema.len(), 9);
    assert_eq!(extension.receivers.len(), 8);
    assert_eq!(extension.owners.len(), 12);
    assert_eq!(bindings["channels"].as_array().unwrap().len(), 8);
    assert_eq!(
        bindings["shared_local_elemental"]["stat"]["key"],
        "def.0000000000002929"
    );
    assert_eq!(bindings["shared_local_elemental"]["contribution"], "add");
    assert!(extension.tables.is_empty() && extension.operations_version.is_none());
    let before = recipe(prior);
    let before_bytes = bundle(prior);
    let output = cwd.join("elemental-weapon-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    assert_eq!(report["extension"]["allocated_entries"], 9);
    assert_eq!(report["extension"]["appended_receivers"], 8);
    assert_eq!(report["extension"]["appended_programs"], 16);
    assert_eq!(report["extension"]["refined_subjects"], 0);
    assert_eq!(report["extension"]["appended_tables"], 0);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(
        after.registry.last_issued.get(),
        before.registry.last_issued.get() + 9
    );
    assert!(
        before
            .registry
            .entries
            .iter()
            .all(|entry| after.registry.entries.contains(entry))
    );
    assert_eq!(after.schema.slots, before.schema.slots);
    assert!(
        before
            .schema
            .definitions
            .iter()
            .all(|entry| after.schema.definitions.contains(entry))
    );
    let mut rebound_routing = before.routing.clone();
    rebound_routing.definitions = after.routing.definitions.clone();
    assert_eq!(after.routing, rebound_routing);
    assert_eq!(after.rules.tables, before.rules.tables);
    assert_eq!(
        after.rules.receivers.closure,
        before.rules.receivers.closure
    );
    assert!(
        before.rules.receivers.members.iter().all(|receiver| after
            .rules
            .receivers
            .members
            .contains(receiver))
    );
    for old in &before.rules.owners {
        let new = after
            .rules
            .owners
            .iter()
            .find(|o| o.owner == old.owner)
            .unwrap();
        assert_eq!(new.programs.closure, old.programs.closure);
        assert!(
            old.programs
                .members
                .iter()
                .all(|p| new.programs.members.contains(p))
        );
        let addition = extension.owners.iter().find(|o| o.owner == old.owner);
        assert_eq!(
            new.programs.members.len(),
            old.programs.members.len() + addition.map_or(0, |o| o.programs.members.len())
        );
        if addition.is_some() {
            assert!(matches!(
                new.programs.closure,
                SchemaClosure::Partial { .. }
            ));
        }
    }
    // The genuine successor crosses the old serialized-size ceiling while
    // remaining inside the default bound and all unchanged structural guards.
    let staged = assemble_owned_recipe(after.clone(), Default::default()).unwrap();
    let old_wire_limit = 8 * 1024 * 1024;
    let wire_bytes = serde_json::to_vec(staged.rules().input()).unwrap().len();
    assert!(wire_bytes > old_wire_limit);
    assert_eq!(
        RuleLimits::default().max_wire_bytes,
        MAX_OWNED_CONTENT_BYTES
    );
    assert!(wire_bytes <= RuleLimits::default().max_wire_bytes);
    let error = CompiledRulePackage::compile(
        staged.rules().input(),
        staged.schema(),
        RuleLimits {
            max_wire_bytes: old_wire_limit,
            ..Default::default()
        },
    )
    .expect_err("the genuine successor must exceed the explicit old wire cap");
    assert_eq!(error.path, "wire");
    assert_eq!(
        error.message,
        format!("canonical content exceeds {old_wire_limit} bytes")
    );
    let replay = extend_owned_recipe(&staged, &extension, Default::default()).unwrap();
    assert_eq!(replay.receipt.allocated_entries, 0);
    assert_eq!(replay.receipt.appended_programs, 0);
    assert_eq!(replay.receipt.appended_receivers, 0);
    assert_eq!(replay.successor, after);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(published[&name], before_bytes[&name]);
    }
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
