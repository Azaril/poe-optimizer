//! Public tree publication and normalization from explicit persisted owned inputs.
#[path = "support/owned_bundle_cli.rs"]
mod cli;
use cli::*;
use poe_optimizer_core::{
    owned_draft::{DraftFinalization, DraftLimits, EvaluationSelection, decode_draft},
    owned_project::VariantSelection,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::{SourceEvidenceLimits, SourceEvidenceRow, SourceProjectEvidence},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::{Command, Output},
};

fn extend(cwd: &Path, prior: &Path, catalog: &Path, policy: &Path, out: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(cwd)
        .arg("extend-owned-tree-catalog")
        .arg(prior)
        .arg("--catalog")
        .arg(catalog)
        .arg("--policy")
        .arg(policy)
        .arg("--output")
        .arg(out)
        .output()
        .unwrap()
}
fn same_files(left: &Path, right: &Path, names: &[&str]) {
    for name in names {
        assert!(
            fs::read(left.join(name)).unwrap() == fs::read(right.join(name)).unwrap(),
            "artifact changed: {name}"
        );
    }
}
#[test]
fn real_tree_publication_reproduces_twenty_files_and_reuses_every_tree_identity() {
    let temp = tempfile::tempdir().unwrap();
    let prior = temp.path().join("prior");
    success(publish(
        temp.path(),
        &data().join("import/compiled/mapping.json"),
        &prior,
    ));
    assert_eq!(
        json(prior.join("registry.json"))["entries"]
            .as_array()
            .unwrap()
            .len(),
        2589
    );
    let catalog = data().join("tree/tree-catalog.json");
    let policy = data().join("tree/catalog-policy.json");
    let first = temp.path().join("first");
    let report = success(extend(temp.path(), &prior, &catalog, &policy, &first));
    assert_eq!(
        report["catalog"]["counts"],
        serde_json::json!({"allocated_definitions":4582,"allocated_slots":298,"reused_definitions":0,"reused_slots":0})
    );
    let publication = &report["publication"];
    assert_eq!(publication["calculation"], "not_run");
    assert_eq!(publication["whole_build_parity"], "not_established");
    assert_eq!(publication["source_execution"], false);
    assert_eq!(publication["query_rows"], 110);
    assert_eq!(json(first.join("transition.json")), *publication);
    let actual = bundle(&first);
    let mut expected = bundle(&data().join("current"));
    assert!(expected.remove("README.md").is_some());
    assert_eq!(actual.len(), 20);
    assert_eq!(expected.len(), 20);
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>()
    );
    for (name, bytes) in &actual {
        assert!(
            bytes == &expected[name],
            "persisted current artifact differs: {name}"
        );
    }
    let second = temp.path().join("second");
    let repeated = success(extend(temp.path(), &first, &catalog, &policy, &second));
    assert_eq!(
        repeated["catalog"]["counts"],
        serde_json::json!({"allocated_definitions":0,"allocated_slots":0,"reused_definitions":4582,"reused_slots":298})
    );
    assert_eq!(
        repeated["catalog"]["before_registry"],
        repeated["catalog"]["after_registry"]
    );
    assert_eq!(
        repeated["catalog"]["before_definitions"],
        repeated["catalog"]["after_definitions"]
    );
    same_files(
        &first,
        &second,
        &[
            "recipe.json",
            "registry.json",
            "schema.json",
            "rules.json",
            "routing.json",
            "mapping.json",
            "tree-normalization.json",
            "normalization.json",
            "rewards.json",
            "items.json",
            "item-source.json",
        ],
    );
    assert_eq!(fs::read_dir(&second).unwrap().count(), 20);
    assert!(
        json(second.join("catalog-append.json"))["mappings"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}
fn identity(value: &Value) -> String {
    serde_json::to_string(value).unwrap()
}
fn known(value: &Value) -> &Value {
    assert_eq!(value["kind"], "known");
    &value["value"]
}
fn attribute<'a>(row: &'a SourceEvidenceRow<'_>, name: &str) -> &'a str {
    row.attribute(name).unwrap().decoded().unwrap()
}
fn tokens(value: &str) -> Vec<&str> {
    if value.is_empty() {
        return vec![];
    }
    let tokens: Vec<_> = value.split(',').collect();
    assert!(
        tokens
            .iter()
            .all(|v| !v.is_empty() && v.bytes().all(|c| c.is_ascii_digit()))
    );
    assert_eq!(
        tokens.iter().copied().collect::<BTreeSet<_>>().len(),
        tokens.len()
    );
    tokens
}
#[test]
fn all_five_xml_censuses_match_independent_spec_allocations_choices_roots_and_queries() {
    let temp = tempfile::tempdir().unwrap();
    let current = data().join("current");
    let tree = json(current.join("tree-normalization.json"));
    let content = &tree["content"];
    let source_catalog = json(data().join("tree/tree-catalog.json"));
    let raw_nodes: BTreeMap<_, _> = source_catalog["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|n| (n["key"].as_str().unwrap(), n))
        .collect();
    let roles: BTreeMap<_, _> = content["tokens"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| (r["token"].as_str().unwrap(), &r["role"]))
        .collect();
    let attribute_rules: BTreeMap<_, _> = content["attributes"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (identity(&a["node"]), a))
        .collect();
    let expected_allocations = [128, 467, 125, 151, 462];
    let expected_choices = [25, 112, 21, 46, 131];
    let expected_specs = [1, 6, 1, 1, 7];
    let expected_gems = [52, 153, 57, 59, 157];
    let expected_rewards = [16, 17, 15, 16, 17];
    let mut totals = [0usize; 4];
    for case in 1..=5 {
        let output = temp.path().join(format!("normalized-{case}"));
        let report = success(normalize(temp.path(), &current, case, &output, true));
        assert_eq!(report["normalization_status"], "pending");
        assert_eq!(report["verification"]["artifact_bindings"], "checked");
        assert_eq!(report["verification"]["legality"], "not_checked");
        assert_eq!(report["verification"]["calculation"], "not_run");
        assert_eq!(
            report["counts"]["allocations"],
            expected_allocations[case - 1]
        );
        assert_eq!(report["counts"]["gems"], expected_gems[case - 1]);
        assert_eq!(report["counts"]["rewards"], expected_rewards[case - 1]);
        let session = decode_draft(
            &fs::read(output.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let draft = session.input();
        let d = serde_json::to_value(draft).unwrap();
        let sidecar = json(output.join("sidecar.json"));
        assert_eq!(
            sidecar["tree_policy"],
            json(current.join("transition.json"))["tree"]
        );
        let source = ImportedBuildInstance::from_decoded(
            decode_build(
                &fs::read(root().join(format!(
                    "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
                )))
                .unwrap(),
            )
            .unwrap(),
            draft.allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        assert_eq!(sidecar["source_sha256"], source.source_sha256());
        assert_eq!(report["source"]["sha256"], source.source_sha256());
        let origins = sidecar["origins"].as_array().unwrap();
        assert_eq!(origins.len(), evidence.rows().len());
        for (row, origin) in evidence.rows().iter().zip(origins) {
            assert_eq!(origin["source"]["ordinal"], row.occurrence().id().ordinal());
            assert_eq!(origin["source"]["source_sha256"], source.source_sha256());
        }
        let allocations: BTreeMap<_, _> = d["allocations"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| (identity(&a["id"]), a))
            .collect();
        let presets: BTreeMap<_, _> = d["allocation_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| (identity(&p["id"]), p))
            .collect();
        let specs: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|r| {
                r.occurrence().name() == "Spec"
                    && r.occurrence().parent().is_some_and(|p| {
                        evidence.rows()[p.ordinal() as usize].occurrence().name() == "Tree"
                    })
            })
            .collect();
        assert_eq!(specs.len(), expected_specs[case - 1]);
        assert_eq!(presets.len(), specs.len());
        let mut all_ids = BTreeSet::new();
        let mut case_choices = 0;
        let mut case_roots = 0;
        for spec in specs {
            assert!(!spec.occurrence().has_namespace_context());
            let listed = tokens(attribute(spec, "nodes"));
            let expected_nodes: BTreeSet<_> = listed
                .iter()
                .filter(|k| {
                    matches!(
                        raw_nodes[**k]["kind"]["kind"].as_str(),
                        Some("allocation" | "attribute")
                    )
                })
                .map(|k| identity(&roles[*k]["value"]["node"]))
                .collect();
            let expected_roots: BTreeSet<_> = listed
                .iter()
                .filter(|k| raw_nodes[**k]["kind"]["kind"] == "implicit_root")
                .map(|k| identity(&roles[*k]["value"]["node"]))
                .collect();
            assert_eq!(expected_roots.len(), 2);
            let origin = &origins[spec.occurrence().id().ordinal() as usize];
            let links = origin["links"].as_array().unwrap();
            let preset_links: Vec<_> = links
                .iter()
                .filter(|l| l["kind"] == "allocation_preset")
                .collect();
            assert_eq!(preset_links.len(), 1);
            let character_links: Vec<_> = links
                .iter()
                .filter(|l| l["kind"] == "character_preset")
                .collect();
            assert_eq!(character_links.len(), 1);
            let root_links: Vec<_> = links
                .iter()
                .filter(|l| l["kind"] == "implicit_passive")
                .collect();
            assert_eq!(
                root_links
                    .iter()
                    .map(|l| identity(&l["value"]["node"]))
                    .collect::<BTreeSet<_>>(),
                expected_roots
            );
            assert!(
                root_links
                    .iter()
                    .all(|l| l["value"]["character"] == character_links[0]["value"])
            );
            case_roots += root_links.len();
            let preset = presets[&identity(&preset_links[0]["value"])];
            assert_eq!(preset["allocations"]["completion"]["kind"], "complete");
            let actual: Vec<_> = preset["allocations"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| {
                    assert!(
                        all_ids.insert(identity(id)),
                        "one allocation leaked into two saved Specs"
                    );
                    allocations[&identity(id)]
                })
                .collect();
            assert_eq!(actual.len(), expected_nodes.len());
            assert_eq!(
                actual
                    .iter()
                    .map(|a| identity(known(&a["node"])))
                    .collect::<BTreeSet<_>>(),
                expected_nodes
            );
            let mut expected_choices: BTreeMap<String, BTreeSet<(String, String)>> =
                BTreeMap::new();
            for token in &listed {
                if raw_nodes[*token]["kind"]["kind"] == "attached_choice" {
                    let v = &roles[*token]["value"];
                    expected_choices
                        .entry(identity(&v["parent"]))
                        .or_default()
                        .insert((identity(&v["slot"]), identity(&v["option"])));
                }
            }
            let mut scoped = BTreeSet::new();
            for child in spec.children() {
                let child = evidence.row(*child).unwrap();
                if child.occurrence().name() == "Overrides" {
                    for override_id in child.children() {
                        let override_row = evidence.row(*override_id).unwrap();
                        assert_eq!(override_row.occurrence().name(), "AttributeOverride");
                        for lane in source_catalog["attribute_options"].as_array().unwrap() {
                            let name = lane["key"].as_str().unwrap();
                            if let Some(value) = override_row.attribute(name) {
                                for token in tokens(value.decoded().unwrap()) {
                                    assert!(listed.contains(&token));
                                    assert_eq!(raw_nodes[token]["kind"]["kind"], "attribute");
                                    let node = identity(&roles[token]["value"]["node"]);
                                    let rule = attribute_rules[&node];
                                    let option = &rule["lanes"]
                                        .as_array()
                                        .unwrap()
                                        .iter()
                                        .find(|l| l["attribute"] == name)
                                        .unwrap()["option"];
                                    assert!(
                                        expected_choices
                                            .entry(node)
                                            .or_default()
                                            .insert((identity(&rule["slot"]), identity(option)))
                                    );
                                }
                            }
                        }
                    }
                }
                if let Some(overlay) = content["syntax"]["weapon_overlays"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|o| o["element"] == child.occurrence().name())
                {
                    for token in tokens(attribute(
                        child,
                        overlay["nodes_attribute"].as_str().unwrap(),
                    )) {
                        assert!(
                            scoped.insert(identity(&roles[token]["value"]["node"])),
                            "source overlay overlap"
                        );
                    }
                }
            }
            for allocation in actual {
                let node = identity(known(&allocation["node"]));
                known(&allocation["pool"]);
                assert_eq!(
                    allocation["access"]["kind"], "pending",
                    "access legality is unconverted"
                );
                assert_eq!(
                    known(&allocation["scope"])["kind"],
                    if scoped.contains(&node) {
                        "selected"
                    } else {
                        "shared"
                    }
                );
                assert_eq!(allocation["choices"]["completion"]["kind"], "complete");
                let actual_choices: BTreeSet<_> = allocation["choices"]["members"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| {
                        let value = known(&c["value"]);
                        assert_eq!(value["kind"], "option");
                        (identity(known(&c["slot"])), identity(&value["value"]))
                    })
                    .collect();
                let expected = expected_choices.remove(&node).unwrap_or_default();
                assert_eq!(actual_choices, expected);
                case_choices += actual_choices.len();
            }
            assert!(expected_choices.is_empty());
        }
        assert_eq!(all_ids.len(), allocations.len());
        assert_eq!(case_choices, expected_choices[case - 1]);
        assert_eq!(case_roots, expected_specs[case - 1] * 2);
        let queries = &draft.query_presets.members[0].queries;
        let expected = json(current.join(format!("queries-original-{case:02}.json")));
        assert_eq!(queries.requests.members.len(), 22);
        for (q, e) in queries
            .requests
            .members
            .iter()
            .zip(expected.as_array().unwrap())
        {
            assert_eq!(serde_json::to_value(&q.id).unwrap(), e["id"]);
        }
        let selection = EvaluationSelection {
            build: VariantSelection {
                character: draft.character_presets.members[0].id,
                equipment: draft.equipment_presets.members[0].id,
                allocations: draft.allocation_presets.members[0].id,
                skills: draft.skill_presets.members[0].id,
                choices: draft.choice_presets.members[0].id,
                active_weapon_loadout: draft.weapon_loadouts.members[0],
            },
            scenario: draft.scenario_presets.members[0].id,
            queries: draft.query_presets.members[0].id,
        };
        match session
            .finalize_selection(selection, DraftLimits::default())
            .unwrap()
        {
            DraftFinalization::Pending {
                issues,
                queries: retained,
                ..
            } => {
                assert!(!issues.is_empty());
                assert_eq!(&retained, queries);
            }
            DraftFinalization::Ready(_) => {
                panic!("tree conversion cannot finalize other missing semantics")
            }
        }
        totals[0] += allocations.len();
        totals[1] += case_choices;
        totals[2] += case_roots;
        totals[3] += queries.requests.members.len();
    }
    assert_eq!(totals, [1333, 335, 32, 110]);
}
fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn rehash_manifest(directory: &Path, name: &str) {
    let bytes = fs::read(directory.join(name)).unwrap();
    let path = directory.join("transition.json");
    let mut m = json(&path);
    let row = m["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["file"] == name)
        .unwrap();
    row["bytes"] = bytes.len().into();
    row["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    write_json(&path, &m);
}
#[test]
fn malformed_hash_omission_stale_tree_and_existing_output_never_publish() {
    let temp = tempfile::tempdir().unwrap();
    let prior = temp.path().join("prior");
    fs::create_dir(&prior).unwrap();
    let mut original = bundle(&data().join("current"));
    original.remove("README.md");
    for (name, bytes) in &original {
        fs::write(prior.join(name), bytes).unwrap();
    }
    let catalog = data().join("tree/tree-catalog.json");
    let policy = data().join("tree/catalog-policy.json");
    let out = temp.path().join("not-published");
    let bad = temp.path().join("malformed.json");
    fs::write(&bad, b"{\"schema_version\":1,\"schema_version\":1}").unwrap();
    assert!(
        !extend(temp.path(), &prior, &bad, &policy, &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(prior.join("mapping.json"), b"{}").unwrap();
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(prior.join("mapping.json"), &original["mapping.json"]).unwrap();
    let transition_path = prior.join("transition.json");
    let mut manifest = json(&transition_path);
    manifest["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["file"] == "tree-normalization.json")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("sha256");
    write_json(&transition_path, &manifest);
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(&transition_path, &original["transition.json"]).unwrap();
    let mut manifest = json(&transition_path);
    manifest["artifacts"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["file"] != "tree-normalization.json");
    write_json(&transition_path, &manifest);
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(&transition_path, &original["transition.json"]).unwrap();
    // The manifest must account for all five ordered query files. Adjusting
    // its counts cannot authorize silently dropping a still-present file.
    for reduced_counts in [false, true] {
        let mut manifest = json(&transition_path);
        manifest["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|a| a["file"] != "queries-original-05.json");
        if reduced_counts {
            manifest["query_sets"] = 4.into();
            manifest["query_rows"] = 88.into();
        }
        write_json(&transition_path, &manifest);
        assert!(
            !extend(temp.path(), &prior, &catalog, &policy, &out)
                .status
                .success(),
            "omitted query file was accepted with reduced_counts={reduced_counts}"
        );
        assert!(!out.exists());
        fs::write(&transition_path, &original["transition.json"]).unwrap();
    }
    // This is a valid owned policy-version edit. Rehashing the file alone must
    // not replace the previously declared semantic reward-policy identity.
    let rewards_path = prior.join("rewards.json");
    let mut rewards = json(&rewards_path);
    rewards["version"] = "reviewed-quest-input-metadata-v2".into();
    write_json(&rewards_path, &rewards);
    rehash_manifest(&prior, "rewards.json");
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success(),
        "rehashing a reward edit bypassed manifest.after.rewards"
    );
    assert!(!out.exists());
    fs::write(&rewards_path, &original["rewards.json"]).unwrap();
    fs::write(&transition_path, &original["transition.json"]).unwrap();
    for field in ["items", "item_source"] {
        let mut manifest = json(&transition_path);
        manifest[field] = "00".repeat(32).into();
        write_json(&transition_path, &manifest);
        assert!(
            !extend(temp.path(), &prior, &catalog, &policy, &out)
                .status
                .success(),
            "wrong manifest.{field} identity was accepted"
        );
        assert!(!out.exists());
        fs::write(&transition_path, &original["transition.json"]).unwrap();
    }
    let mut stale = json(prior.join("tree-normalization.json"));
    stale["registry"] = "00".repeat(32).into();
    write_json(&prior.join("tree-normalization.json"), &stale);
    rehash_manifest(&prior, "tree-normalization.json");
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success()
    );
    assert!(!out.exists());
    fs::write(
        prior.join("tree-normalization.json"),
        &original["tree-normalization.json"],
    )
    .unwrap();
    fs::write(&transition_path, &original["transition.json"]).unwrap();
    fs::create_dir(&out).unwrap();
    fs::write(out.join("sentinel"), b"caller-owned").unwrap();
    let before = bundle(&out);
    assert!(
        !extend(temp.path(), &prior, &catalog, &policy, &out)
            .status
            .success()
    );
    assert_eq!(bundle(&out), before);
    assert_eq!(
        fs::read_dir(temp.path()).unwrap().count(),
        3,
        "no staged output leaked"
    );
}
