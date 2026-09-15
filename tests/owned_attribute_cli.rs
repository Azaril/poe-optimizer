//! The public offline host publishes injected attributes without any default build.
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_definitions::{BoundedInteger, OwnedDefinitionKey, StatDefinition},
    owned_draft::{DraftAllocationAccess, DraftLimits, decode_draft},
    owned_rules::{RuleEffectKind, RuleReadSource},
    owned_schema::{
        ComputedValueType, DefinitionEntry, RuleEntityKind, SchemaDefinitionId, SchemaState,
        SchemaSubject, StatSchema,
    },
};
use poe_optimizer_engine::owned_rules::{CompiledRulePackage, EffectDisposition, RuleFact};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_attribute_recipe::{AttributeLanePolicy, AttributeRecipePolicy},
    owned_mapping::{OwnedIdRegistry, OwnedMappingLimits},
    owned_recipe::assemble_owned_recipe,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_catalog::{TreeCatalogInput, TreeNodeKind},
    owned_tree_policy::TreeNormalizationPackageInput,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68")
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
fn copy_bundle(from: &Path, to: &Path) {
    fs::create_dir(to).unwrap();
    for (name, bytes) in files(from) {
        fs::write(to.join(name), bytes).unwrap();
    }
}
struct Fixture {
    temp: tempfile::TempDir,
    prior: PathBuf,
    catalog: PathBuf,
    policy: PathBuf,
    statistics: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let prior = temp.path().join("prior");
        copy_bundle(&data().join("current"), &prior);
        let catalog = temp.path().join("catalog.json");
        fs::copy(data().join("tree/tree-catalog.json"), &catalog).unwrap();
        let source: TreeCatalogInput = serde_json::from_value(read(&catalog)).unwrap();
        let mut registry = OwnedIdRegistry::new(
            serde_json::from_value(read(prior.join("registry.json"))).unwrap(),
            OwnedMappingLimits::default(),
        )
        .unwrap();
        let mut statistics = vec![];
        let lanes = source
            .attribute_options
            .iter()
            .map(|option| {
                let id = registry.allocate_definition::<StatDefinition>().unwrap();
                statistics.push(DefinitionEntry {
                    id: id.clone(),
                    schema: SchemaState::Known(StatSchema {
                        value: ComputedValueType::Integer,
                        targets: vec![RuleEntityKind::Actor],
                    }),
                });
                AttributeLanePolicy {
                    key: option.key.clone(),
                    expected_stats: option.stats.clone(),
                    stat: id,
                    value: ParameterValue::Integer(BoundedInteger::new(5).unwrap()),
                }
            })
            .collect();
        let policy_value = AttributeRecipePolicy {
            schema_version: 1,
            version: OwnedDefinitionKey::new("fixture-attribute-recipes").unwrap(),
            expected_node_stats: source
                .nodes
                .iter()
                .find(|node| matches!(node.kind, TreeNodeKind::Attribute { .. }))
                .unwrap()
                .stats
                .clone(),
            lanes,
        };
        let policy = temp.path().join("policy.json");
        write(&policy, &policy_value);
        let statistics_path = temp.path().join("statistics.json");
        write(&statistics_path, &statistics);
        Self {
            temp,
            prior,
            catalog,
            policy,
            statistics: statistics_path,
        }
    }
    fn run(&self, prior: &Path, output: &Path) -> Output {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(self.temp.path())
            .arg("compile-owned-attributes")
            .arg(prior)
            .arg("--catalog")
            .arg(&self.catalog)
            .arg("--policy")
            .arg(&self.policy)
            .arg("--statistics")
            .arg(&self.statistics)
            .arg("--output")
            .arg(output)
            .output()
            .unwrap()
    }
    fn reject(&self, prior: &Path, name: &str, expected: &str) {
        let out = self.temp.path().join(name);
        let result = self.run(prior, &out);
        assert!(
            !result.status.success(),
            "invalid input was published: {name}"
        );
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(expected),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!out.exists(), "failed compilation left an output directory");
    }
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn explicit_statistics_publish_and_rerun_preserves_all_semantic_artifacts() {
    let f = Fixture::new();
    let before = files(&f.prior);
    let first = f.temp.path().join("first");
    let report = success(f.run(&f.prior, &first));
    assert_eq!(report["attributes"]["converted_nodes"], 293);
    assert_eq!(report["attributes"]["refined_nodes"], 293);
    assert_eq!(report["publication"]["query_rows"], 110);
    assert_eq!(report["publication"]["source_execution"], false);
    assert_eq!(report["publication"]["calculation"], "not_run");
    assert_eq!(
        report["publication"]["whole_build_parity"],
        "not_established"
    );
    assert_eq!(report["publication"], read(first.join("transition.json")));
    let refinement = &report["publication"]["schema_refinement"];
    assert_eq!(refinement["nodes"].as_array().unwrap().len(), 293);
    assert_eq!(
        refinement["before"],
        read(f.prior.join("transition.json"))["after"]["definitions"]
    );
    assert_eq!(
        refinement["after"],
        report["publication"]["after"]["definitions"]
    );
    let old_registry = read(f.prior.join("registry.json"));
    let new_registry = read(first.join("registry.json"));
    let old_entries = old_registry["entries"].as_array().unwrap();
    let new_entries = new_registry["entries"].as_array().unwrap();
    assert_eq!(&new_entries[..old_entries.len()], old_entries);
    assert_eq!(new_entries.len(), old_entries.len() + 3);
    assert_eq!(
        read(first.join("tree-normalization.json"))["content"],
        read(f.prior.join("tree-normalization.json"))["content"]
    );
    assert_eq!(
        read(first.join("mapping.json"))["entries"],
        read(f.prior.join("mapping.json"))["entries"]
    );
    for i in 1..=5 {
        let name = format!("queries-original-{i:02}.json");
        assert_eq!(fs::read(first.join(&name)).unwrap(), before[&name]);
    }
    let second = f.temp.path().join("second");
    let rerun = success(f.run(&first, &second));
    assert_eq!(rerun["attributes"]["converted_nodes"], 293);
    assert_eq!(rerun["attributes"]["refined_nodes"], 0);
    assert!(rerun["publication"].get("schema_refinement").is_none());
    let first_files = files(&first);
    let second_files = files(&second);
    assert_eq!(first_files.len(), second_files.len());
    for (name, bytes) in &first_files {
        if name != "transition.json" {
            assert_eq!(bytes, &second_files[name], "rerun changed {name}");
        }
    }
    assert_eq!(before, files(&f.prior));
    let sentinel = f.temp.path().join("occupied");
    fs::create_dir(&sentinel).unwrap();
    fs::write(sentinel.join("keep"), b"caller data").unwrap();
    assert!(!f.run(&f.prior, &sentinel).status.success());
    assert_eq!(fs::read(sentinel.join("keep")).unwrap(), b"caller data");
    assert_eq!(fs::read_dir(&sentinel).unwrap().count(), 1);
}

#[test]
fn invalid_statistics_fail_before_publication_and_preserve_inputs() {
    let f = Fixture::new();
    let before = files(&f.prior);
    let original = read(&f.statistics);
    let mut changed = original.clone();
    changed.as_array_mut().unwrap().swap(0, 1);
    write(&f.statistics, &changed);
    f.reject(&f.prior, "unordered", "canonical ID order");
    let mut changed = original.clone();
    let duplicate = changed[0].clone();
    changed.as_array_mut().unwrap().push(duplicate);
    write(&f.statistics, &changed);
    f.reject(&f.prior, "duplicate", "canonical ID order");
    let mut changed = original.clone();
    changed[0]["schema"]["value"]["targets"] = json!(["action"]);
    write(&f.statistics, &changed);
    f.reject(&f.prior, "wrong-target", "known Actor");
    let mut changed = original.clone();
    changed[0]["id"] = changed[1]["id"].clone();
    changed.as_array_mut().unwrap().remove(1);
    write(&f.statistics, &changed);
    f.reject(&f.prior, "skipped-id", "next canonical registry");
    write(&f.statistics, &original);
    let first = f.temp.path().join("first");
    success(f.run(&f.prior, &first));
    let mut changed = original;
    changed[0]["schema"]["value"]["value"] = json!({"kind":"boolean"});
    write(&f.statistics, &changed);
    f.reject(&first, "redefined", "existing descriptor");
    assert_eq!(before, files(&f.prior));
}

#[test]
fn corrupted_artifacts_policy_and_refinement_metadata_are_rejected() {
    let f = Fixture::new();
    let first = f.temp.path().join("first");
    success(f.run(&f.prior, &first));
    let manifest_path = first.join("transition.json");
    let original = read(&manifest_path);
    let mut changed = original.clone();
    changed["schema_refinement"]["before"] = changed["schema_refinement"]["after"].clone();
    write(&manifest_path, &changed);
    f.reject(&first, "stale-refinement", "refinement metadata");
    let mut changed = original.clone();
    let node = changed["schema_refinement"]["nodes"][0].clone();
    changed["schema_refinement"]["nodes"]
        .as_array_mut()
        .unwrap()
        .push(node);
    write(&manifest_path, &changed);
    f.reject(&first, "duplicate-refinement", "refinement nodes");
    write(&manifest_path, &original);
    let catalog = read(&f.catalog);
    let mut changed_catalog = catalog.clone();
    changed_catalog["nodes"].as_array_mut().unwrap().pop();
    write(&f.catalog, &changed_catalog);
    f.reject(&first, "changed-catalog", "catalog differs");
    write(&f.catalog, &catalog);
    let mut policy = read(&f.policy);
    policy["expected_node_stats"] = json!(["unreviewed source"]);
    write(&f.policy, &policy);
    f.reject(&first, "wrong-policy", "attribute");
    let mapping_path = first.join("mapping.json");
    let mut bytes = fs::read(&mapping_path).unwrap();
    bytes.push(b' ');
    fs::write(&mapping_path, bytes).unwrap();
    f.reject(&first, "corrupt-artifact", "hash/size differs");
}

#[test]
fn persisted_inputs_execute_each_original_attribute_choice_without_finalizing_builds() {
    let mut f = Fixture::new();
    f.policy = data().join("attributes/policy.json");
    f.statistics = data().join("attributes/statistics.json");
    let output = f.temp.path().join("published");
    success(f.run(&f.prior, &output));
    let recipe = assemble_owned_recipe(
        serde_json::from_value(read(output.join("recipe.json"))).unwrap(),
        Default::default(),
    )
    .unwrap();
    let compiled =
        CompiledRulePackage::compile(recipe.rules().input(), recipe.schema(), Default::default())
            .unwrap();
    let policy: AttributeRecipePolicy = serde_json::from_value(read(&f.policy)).unwrap();
    let tree: TreeNormalizationPackageInput =
        serde_json::from_value(read(output.join("tree-normalization.json"))).unwrap();
    let mut scratch = compiled.new_scratch();
    let mut selected_choices = 0;
    let mut all_choices = 0;
    for case in 1..=5 {
        let build = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let normalized = f.temp.path().join(format!("normalized-{case}"));
        let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
        command
            .current_dir(f.temp.path())
            .arg("normalize-owned")
            .arg(&build);
        for (flag, file) in [
            ("--policy", "normalization.json"),
            ("--registry", "registry.json"),
            ("--definitions", "schema.json"),
            ("--mapping", "mapping.json"),
            ("--roles", "roles.json"),
            ("--rewards", "rewards.json"),
            ("--items", "items.json"),
            ("--item-source", "item-source.json"),
            ("--tree-policy", "tree-normalization.json"),
        ] {
            command.arg(flag).arg(output.join(file));
        }
        command
            .arg("--queries")
            .arg(output.join(format!("queries-original-{case:02}.json")))
            .arg("--output")
            .arg(&normalized);
        let report = success(command.output().unwrap());
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["calculation"], "not_run");
        let session = decode_draft(
            &fs::read(normalized.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let draft = session.input();
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&fs::read(&build).unwrap()).unwrap(),
            draft.allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        let source_tree = evidence
            .rows()
            .iter()
            .find(|row| row.occurrence().name() == "Tree")
            .unwrap();
        let selected_spec: usize = source_tree
            .attribute("activeSpec")
            .unwrap()
            .decoded()
            .unwrap()
            .parse()
            .unwrap();
        let source_specs = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Spec")
            .count();
        assert_eq!(source_specs, draft.allocation_presets.members.len());
        // Tree normalization retains source Spec order. The active source index
        // selects observations only; it is never used as an owned semantic ID.
        let selected = &draft.allocation_presets.members[selected_spec - 1]
            .allocations
            .members;
        let mut case_count = 0;
        for allocation in &draft.allocations.members {
            let node = allocation.node.to_resolved().unwrap();
            let Some(attribute) = tree.content.attributes.iter().find(|row| row.node == node)
            else {
                continue;
            };
            assert!(matches!(
                allocation.access,
                DraftAllocationAccess::Pending(_)
            ));
            let owner = SchemaSubject::Definition(node.address());
            let programs = recipe
                .rules()
                .input()
                .owners
                .iter()
                .find(|row| row.owner == owner)
                .unwrap();
            assert!(programs.programs.is_complete());
            assert_eq!(programs.programs.members.len(), 1);
            let program = &programs.programs.members[0];
            let RuleReadSource::Choice { slot } = &program.reads[0].source else {
                panic!("expected attribute choice")
            };
            assert_eq!(allocation.choices.members.len(), 1);
            let choice = &allocation.choices.members[0];
            assert_eq!(choice.slot.to_resolved().as_ref(), Some(slot));
            let value = choice.value.to_resolved().unwrap();
            let ParameterValue::Option(option) = &value else {
                panic!("expected owned attribute option")
            };
            let lane_key = &attribute
                .lanes
                .iter()
                .find(|lane| &lane.option == option)
                .unwrap()
                .attribute;
            let expected = policy
                .lanes
                .iter()
                .find(|lane| &lane.key == lane_key)
                .unwrap();
            let evaluated = compiled
                .evaluate(
                    &owner,
                    &program.id,
                    &[RuleFact {
                        read: program.reads[0].id.clone(),
                        value,
                    }],
                    recipe.schema(),
                    &mut scratch,
                )
                .unwrap();
            let applied: Vec<_> = evaluated
                .effects
                .iter()
                .filter(|effect| matches!(effect.disposition, EffectDisposition::Applied { .. }))
                .collect();
            assert_eq!(applied.len(), 1);
            assert_eq!(
                applied[0].disposition,
                EffectDisposition::Applied {
                    value: expected.value.clone()
                }
            );
            let RuleEffectKind::Contribute { stat, .. } = &applied[0].effect else {
                panic!("expected attribute contribution")
            };
            assert_eq!(*stat, expected.stat);
            assert_eq!(
                evaluated
                    .effects
                    .iter()
                    .filter(|effect| matches!(effect.disposition, EffectDisposition::Inactive))
                    .count(),
                2
            );
            all_choices += 1;
            if selected.contains(&allocation.id) {
                selected_choices += 1;
                case_count += 1;
            }
        }
        assert_eq!(case_count, [25, 28, 21, 45, 22][case - 1]);
        assert_eq!(
            draft.query_presets.members[0]
                .queries
                .requests
                .members
                .len(),
            22
        );
    }
    assert_eq!(selected_choices, 141);
    assert_eq!(all_choices, 331);
}
