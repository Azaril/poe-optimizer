//! Historical publication and canonical-slot laws awaiting direct release migration.
use super::support::{bundle, data, json, success};
use poe_optimizer_core::{
    owned_definitions::ModifierDefId,
    owned_rules::{ContributionKind, RuleProgram, RuleReadSource},
    owned_schema::{SchemaClosure, SchemaDefinitionId, SchemaSubject},
};
use poe_optimizer_import::{
    owned_recipe::{OwnedRecipeInput, StagedOwnedRecipe, assemble_owned_recipe},
    owned_recipe_extension::{OwnedRecipeExtension, extend_owned_recipe},
};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn recipe(path: &Path) -> OwnedRecipeInput {
    OwnedRecipeInput {
        schema_version: 1,
        registry: serde_json::from_value(json(path.join("registry.json"))).unwrap(),
        schema: serde_json::from_value(json(path.join("schema.json"))).unwrap(),
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
fn subject(family: &Value) -> SchemaSubject {
    let modifier: ModifierDefId = serde_json::from_value(family["modifier"].clone()).unwrap();
    SchemaSubject::Definition(modifier.address())
}
struct Component {
    staged: StagedOwnedRecipe,
}
impl Component {
    fn new(input: OwnedRecipeInput) -> Self {
        Self {
            staged: assemble_owned_recipe(input, Default::default()).unwrap(),
        }
    }
    fn program(&self, owner: &SchemaSubject, name: &str) -> &RuleProgram {
        self.staged
            .rules()
            .input()
            .owners
            .iter()
            .find(|o| &o.owner == owner)
            .unwrap()
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == name)
            .unwrap()
    }
}

pub fn check_local_scaling(cwd: &Path, prior: &Path) -> PathBuf {
    let authored = data().join("local-scaling-inputs");
    let authored_bytes = bundle(&authored);
    let extension_path = authored.join("extension.json");
    let extension: OwnedRecipeExtension = serde_json::from_value(json(&extension_path)).unwrap();
    let bindings = json(authored.join("bindings.json"));
    assert_eq!(bindings["families"].as_array().unwrap().len(), 9);
    assert_eq!(bindings["contributions"].as_array().unwrap().len(), 7);
    assert!(extension.schema.is_empty());
    assert!(extension.tables.is_empty());
    assert!(extension.receivers.is_empty());
    assert_eq!(extension.owners.len(), 9);
    assert_eq!(
        extension
            .owners
            .iter()
            .map(|o| o.programs.members.len())
            .sum::<usize>(),
        25
    );
    let before_bytes = bundle(prior);
    let before = recipe(prior);
    let output = cwd.join("local-scaling-successor");
    let report = success(publish(cwd, prior, &extension_path, &output));
    for field in [
        "allocated_entries",
        "refined_subjects",
        "appended_tables",
        "appended_receivers",
    ] {
        assert_eq!(report["extension"][field], 0, "{field}");
    }
    assert_eq!(report["extension"]["appended_programs"], 25);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    let after = recipe(&output);
    assert_eq!(after.registry, before.registry);
    assert_eq!(after.schema, before.schema);
    assert_eq!(after.routing, before.routing);
    assert_eq!(after.rules.tables, before.rules.tables);
    assert_eq!(after.rules.receivers, before.rules.receivers);
    assert_eq!(after.rules.owners.len(), before.rules.owners.len());
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
    for binding in bindings["contributions"].as_array().unwrap() {
        let kind: ContributionKind =
            serde_json::from_value(binding["contribution"].clone()).unwrap();
        assert!(matches!(
            kind,
            ContributionKind::Add | ContributionKind::Increase
        ));
    }
    let component = Component::new(after.clone());
    // Compare with the reviewed canonical cold algorithms after changing only
    // semantic parameter addresses. Named caller facts alone cannot prove that
    // the authored programs read the right Modifier occurrence slots.
    let canonical = json(data().join("modifier-value-inputs/bindings.json"));
    let source = canonical["families"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["family"] == "cold-resistance")
        .unwrap();
    let cold: ModifierDefId = serde_json::from_value(source["canonical"].clone()).unwrap();
    let cold_owner = SchemaSubject::Definition(cold.address());
    for family in bindings["families"].as_array().unwrap() {
        let target = canonical["families"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["family"] == family["family"])
            .unwrap();
        let owner = subject(family);
        let mut expected = component.program(&cold_owner, "catalyst-scalar").clone();
        for read in &mut expected.reads {
            if let RuleReadSource::Parameter { slot } = &mut read.source {
                let property = if read.id.as_str() == "unscalable" {
                    "unscalable"
                } else {
                    read.id.as_str().strip_prefix("property-").unwrap()
                };
                assert_eq!(
                    serde_json::to_value(&*slot).unwrap(),
                    source["property_inputs"][property]
                );
                *slot =
                    serde_json::from_value(target["property_inputs"][property].clone()).unwrap();
            }
        }
        assert_eq!(&expected, component.program(&owner, "catalyst-scalar"));
        assert_eq!(
            component.program(&cold_owner, "ordered-magnitude-scalar"),
            component.program(&owner, "ordered-magnitude-scalar")
        );
    }
    let repeated = extend_owned_recipe(&component.staged, &extension, Default::default()).unwrap();
    assert_eq!(repeated.receipt.appended_programs, 0);
    assert_eq!(repeated.successor, after);
    let published = bundle(&output);
    assert!(
        !publish(cwd, prior, &extension_path, &output)
            .status
            .success()
    );
    assert_eq!(bundle(&output), published);
    let replay = cwd.join("local-scaling-replay");
    assert_eq!(
        success(publish(cwd, prior, &extension_path, &replay)),
        report
    );
    assert_eq!(bundle(&replay), published);
    for case in 1..=5 {
        let name = format!("queries-original-{case:02}.json");
        assert_eq!(published[&name], before_bytes[&name]);
    }
    assert_eq!(bundle(prior), before_bytes);
    assert_eq!(bundle(&authored), authored_bytes);
    output
}
