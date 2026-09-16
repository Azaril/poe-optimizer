//! Exact policy replacement uses the same checked compact publication boundary.
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-import/tests/support/owned_compact_fixture.rs"]
mod compact_fixture;
#[allow(dead_code)]
#[path = "support/owned_bundle_cli.rs"]
mod support;
use poe_optimizer_core::{
    owned_definitions::{OwnedDefinitionKey, UnitDefinition},
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_item_lines::{ItemLinePolicyInput, OwnedItemLinePolicy},
    owned_item_source::{ItemSourceDialect, ItemSourceLayoutPolicy, ItemSourceLayoutPolicyInput},
    owned_recipe::assemble_owned_recipe,
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry, extend_owned_recipe},
    owned_successor::{
        CatalogItemPolicyMode, StagedSuccessorBundle, transition_owned_catalog_with_tree_compact,
    },
};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};
use support::{bundle, json, root, success};

fn baseline() -> &'static StagedSuccessorBundle {
    static BASELINE: OnceLock<StagedSuccessorBundle> = OnceLock::new();
    BASELINE.get_or_init(|| {
        let input = compact_fixture::input(&root());
        let mut append = compact_fixture::append(&input);
        append.item_policies = CatalogItemPolicyMode::SuppliedSuccessor;
        let tree = compact_fixture::tree(&input);
        transition_owned_catalog_with_tree_compact(input, append, tree, Default::default()).unwrap()
    })
}
fn write(path: &Path, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
struct Fixture {
    temp: tempfile::TempDir,
    prior: PathBuf,
    extension: PathBuf,
    items: ItemLinePolicyInput,
    source: ItemSourceLayoutPolicyInput,
}
impl Fixture {
    fn new(flags: bool) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let prior = temp.path().join("prior");
        fs::create_dir(&prior).unwrap();
        let base = baseline();
        for (name, bytes) in base.artifacts() {
            fs::write(prior.join(name), bytes).unwrap();
        }
        let mut registry = base.assembled().registry().clone();
        let id = registry.allocate_definition::<UnitDefinition>().unwrap();
        let mut unit = base
            .recipe()
            .schema
            .definitions
            .iter()
            .find_map(|d| match d {
                DefinitionDescriptor::Unit(unit) => Some(unit.clone()),
                _ => None,
            })
            .unwrap();
        unit.id = id;
        let extension = OwnedRecipeExtension {
            schema_version: 1,
            version: OwnedDefinitionKey::new("paired-policy-extension").unwrap(),
            schema: vec![SchemaExtensionEntry::Definition(
                DefinitionDescriptor::Unit(unit),
            )],
            operations_version: None,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
        };
        let extended =
            extend_owned_recipe(base.assembled(), &extension, Default::default()).unwrap();
        let extended = assemble_owned_recipe(extended.successor, Default::default()).unwrap();
        let extension_path = temp.path().join("extension.json");
        write(&extension_path, &extension);
        let mut items = base.items().input().clone();
        items.definitions = extended.schema().identity().clone();
        items.version = OwnedDefinitionKey::new("explicit-successor-items").unwrap();
        let lines =
            OwnedItemLinePolicy::new(items.clone(), extended.schema(), Default::default()).unwrap();
        let mut source = base.item_source().input().clone();
        source.item_lines = *lines.identity();
        source.version = OwnedDefinitionKey::new("explicit-successor-source").unwrap();
        if flags {
            source.schema_version = 4;
            source.dialect = ItemSourceDialect::PobExportedSingleTextFlagsV1 {
                flag_bindings: vec![],
            };
        }
        // This fixture changes the wire contract without claiming new flag coverage.
        ItemSourceLayoutPolicy::new(
            source.clone(),
            &lines,
            extended.schema(),
            Default::default(),
        )
        .unwrap();
        Self {
            temp,
            prior,
            extension: extension_path,
            items,
            source,
        }
    }
    fn command(&self, prior: &Path, destination: &Path) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
        command
            .current_dir(self.temp.path())
            .arg("extend-owned-recipe")
            .arg(prior)
            .arg("--extension")
            .arg(&self.extension)
            .arg("--output")
            .arg(destination);
        command
    }
    fn paired(&self, prior: &Path, destination: &Path) -> Output {
        let items = self.temp.path().join("supplied-items.json");
        let source = self.temp.path().join("supplied-source.json");
        write(&items, &self.items);
        write(&source, &self.source);
        self.command(prior, destination)
            .arg("--items")
            .arg(items)
            .arg("--item-source")
            .arg(source)
            .output()
            .unwrap()
    }
}

#[test]
fn both_policy_flags_are_required_together_before_any_input_is_read() {
    let temp = tempfile::tempdir().unwrap();
    for (flag, missing) in [("--items", "--item-source"), ("--item-source", "--items")] {
        let destination = temp.path().join("not-created");
        let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(temp.path())
            .arg("extend-owned-recipe")
            .arg("missing-prior")
            .arg("--extension")
            .arg("missing-extension")
            .arg(flag)
            .arg("missing-policy")
            .arg("--output")
            .arg(&destination)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(String::from_utf8_lossy(&output.stderr).contains(missing));
        assert!(!destination.exists());
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn exact_v3_policy_pair_preserves_selected_history_queries_tree_and_prior() {
    let fixture = Fixture::new(false);
    let before = bundle(&fixture.prior);
    let destination = fixture.temp.path().join("successor");
    let report = success(fixture.paired(&fixture.prior, &destination));
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "explicit_successor_bound_inputs"
    );
    assert_eq!(report["extension"]["allocated_entries"], 1);
    assert_eq!(
        report["publication"]["before"],
        json(fixture.prior.join("transition.json"))["after"]
    );
    assert_eq!(
        json(destination.join("items.json")),
        serde_json::to_value(&fixture.items).unwrap()
    );
    assert_eq!(
        json(destination.join("item-source.json")),
        serde_json::to_value(&fixture.source).unwrap()
    );
    assert!(!destination.join("recipe.json").exists());
    assert_eq!(
        json(destination.join("tree-normalization.json"))["content"],
        json(fixture.prior.join("tree-normalization.json"))["content"]
    );
    for (name, bytes) in &before {
        if name.starts_with("queries-") {
            assert_eq!(fs::read(destination.join(name)).unwrap(), *bytes);
        }
    }
    assert_eq!(bundle(&fixture.prior), before);
    let published = bundle(&destination);
    assert!(
        !fixture
            .paired(&fixture.prior, &destination)
            .status
            .success()
    );
    assert_eq!(bundle(&destination), published);
    assert_eq!(bundle(&fixture.prior), before);
    let rerun = fixture.temp.path().join("again");
    let rerun_report = success(fixture.paired(&destination, &rerun));
    assert_eq!(rerun_report["extension"]["allocated_entries"], 0);
    assert_eq!(
        rerun_report["publication"]["before"],
        report["publication"]["after"]
    );
    assert_eq!(
        rerun_report["publication"]["after"],
        report["publication"]["after"]
    );
}

#[test]
fn exact_v4_source_policy_is_valid_and_omitted_flags_keep_prior_rebind_behavior() {
    let fixture = Fixture::new(true);
    let destination = fixture.temp.path().join("v4");
    success(fixture.paired(&fixture.prior, &destination));
    assert_eq!(
        json(destination.join("item-source.json"))["schema_version"],
        4
    );
    assert_eq!(
        json(destination.join("item-source.json")),
        serde_json::to_value(&fixture.source).unwrap()
    );
    let carried = fixture.temp.path().join("carried");
    let report = success(fixture.command(&fixture.prior, &carried).output().unwrap());
    assert_eq!(
        report["publication"]["item_policy_mode"],
        "validated_prior_binding_rebind"
    );
    let mut expected_items = baseline().items().input().clone();
    expected_items.definitions = fixture.items.definitions.clone();
    let actual_items = json(carried.join("items.json"));
    assert_eq!(actual_items, serde_json::to_value(expected_items).unwrap());
    let mut expected_source = baseline().item_source().input().clone();
    expected_source.item_lines =
        serde_json::from_value(report["publication"]["items"].clone()).unwrap();
    assert_eq!(
        json(carried.join("item-source.json")),
        serde_json::to_value(expected_source).unwrap()
    );
}

#[test]
fn supplied_stale_bindings_and_mismatched_source_versions_are_never_rewritten() {
    let mut fixture = Fixture::new(false);
    let before = bundle(&fixture.prior);
    let items = fixture.items.clone();
    let source = fixture.source.clone();
    for case in 0..4 {
        fixture.items = items.clone();
        fixture.source = source.clone();
        match case {
            0 => fixture.items.definitions = baseline().items().input().definitions.clone(),
            1 => fixture.source.item_lines = *baseline().items().identity(),
            2 => fixture.source.schema_version = 4, // v4 requires its explicit dialect.
            3 => {
                fixture.source.dialect = ItemSourceDialect::PobExportedSingleTextFlagsV1 {
                    flag_bindings: vec![],
                }
            }
            _ => unreachable!(),
        }
        let destination = fixture.temp.path().join(format!("rejected-{case}"));
        let output = fixture.paired(&fixture.prior, &destination);
        assert!(!output.status.success(), "case {case}");
        assert!(!destination.exists());
        assert_eq!(bundle(&fixture.prior), before);
    }
}
