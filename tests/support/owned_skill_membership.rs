//! Checked normalization-only authored SkillPreset membership publication.
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{SkillInventoryPolicy, direct_skill_inputs_identity},
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/skill-membership")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn policy() -> SkillInventoryPolicy {
    read("policy.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let input = json!(policy());
    assert_eq!(input["kind"], "pob_fresh_authored_roots_v1");
    for field in ["mapping_source", "roles", "direct_inputs"] {
        assert_eq!(input[field], a[field]);
    }
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["reviewed_generated_tuples"], 3);
    for field in [
        "new_definitions",
        "new_slots",
        "new_rules",
        "new_tables",
        "new_scalar_values",
    ] {
        assert_eq!(a[field], 0);
    }
    assert_eq!(a["preserved_query_rows"], 110);
    for field in ["generated_activation", "usage_inventory", "support_targets"] {
        assert_eq!(a[field], "pending");
    }
    assert_eq!(a["native_mechanics_coverage"], "partial");
    assert_eq!(a["authored_membership"], "reviewed_intrinsic_only");
    assert_eq!(a["source_validation"]["status"], "passed");
    let bytes = fs::read(data().join("policy.json")).unwrap();
    assert!(!bytes.contains(&b'\r'));
    assert_eq!(
        a["artifact_sha256"]["policy"],
        format!("{:x}", Sha256::digest(bytes))
    );
    assert!(
        !fs::read(data().join("authoring.json"))
            .unwrap()
            .contains(&b'\r')
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest_bytes))
    );
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(pin.as_object().unwrap().len(), 2);
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let catalog: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/skill-identities.json")).unwrap(),
    )
    .unwrap();
    let checked = poe_optimizer_data::skill_identities::SkillIdentityCatalog::new(
        serde_json::from_value(catalog.clone()).unwrap(),
    )
    .unwrap();
    assert_eq!(
        json!(
            digest_owned(
                "owned-skill-source-catalog-v1",
                checked.data(),
                64 * 1024 * 1024
            )
            .unwrap()
        ),
        a["catalog"]
    );
    let rows = input["generated_groups"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    for (row, (source, skill_id, gem_key)) in rows.iter().zip([
        (
            "tree_allocation",
            "SummonSandDjinnPlayer",
            "def.0000000000000656",
        ),
        (
            "tree_allocation",
            "SummonWaterDjinnPlayer",
            "def.0000000000000657",
        ),
        ("item_grant", "FireboltPlayer", "def.0000000000000778"),
    ]) {
        assert_eq!(row["source"], source);
        assert_eq!(row["skill_id"], skill_id);
        assert_eq!(row["gem"]["key"], gem_key);
        let matches: Vec<_> = catalog["gems"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|gem| {
                gem["game_id"] == row["game_id"] && gem["variant_id"] == row["variant_id"]
            })
            .collect();
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0]["primary_effect_id"], row["skill_id"]);
        assert_eq!(matches[0]["name"], row["name_spec"]);
    }
}

fn rows(value: &Value) -> &[Value] {
    if let Some(rows) = value.as_array() {
        rows
    } else {
        assert!(value.as_object().is_some_and(|object| object.is_empty()));
        &[]
    }
}
fn case<'a>(report: &'a Value, name: &str) -> &'a Value {
    let matches: Vec<_> = rows(&report["cases"])
        .iter()
        .filter(|entry| entry["name"] == name)
        .collect();
    assert_eq!(matches.len(), 1, "exact executed source case {name}");
    matches[0]
}
fn source_proof(a: &Value) {
    let proof = &a["source_validation"];
    assert_eq!(proof["status"], "passed");
    let mut reports = Vec::new();
    for (path, size, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let bytes = fs::read(root().join(proof[path].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, proof[size].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), proof[hash]);
        reports.push(bytes);
    }
    assert!(
        reports[0] == reports[1],
        "both JIT modes preserve every observation"
    );
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(report["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(report["source_revision"], a["source_revision"]);
    assert_eq!(report["files"], a["source_files"]);
    assert_eq!(rows(&report["files"]).len(), 14);
    for field in [
        "business_wrappers",
        "native_inventory_authority",
        "native_build_parity",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(report[field], false);
    }
    assert_eq!(rows(&report["original_sources"]).len(), 5);
    let entries = rows(&report["cases"]);
    assert_eq!(entries.len(), 34);
    assert_eq!(proof["cases"], 34);
    let mut names = BTreeSet::new();
    for entry in entries {
        assert!(names.insert(entry["name"].as_str().unwrap()));
        assert_eq!(entry["source_hash"], a["source_manifest_sha256"]);
        assert_eq!(
            entry["source_identity"]["source_sha256"],
            entry["xml_sha256"]
        );
        assert_eq!(entry["source_identity"]["instance_import_schema"], 1);
        assert!(entry["source_identity"]["source_bytes"].as_u64().unwrap() > 0);
        let joins = rows(&entry["source_joins"]);
        let mut joined = BTreeSet::new();
        for join in joins {
            assert_eq!(join["source"]["source_sha256"], entry["xml_sha256"]);
            assert_eq!(join["source"]["ordinal"], join["source_ordinal"]);
            assert!(joined.insert(join["source_ordinal"].as_u64().unwrap()));
        }
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let state = &entry["states"][stage];
            for flag in [
                "exact_loader_capture",
                "observer_removed_before_evaluation",
                "requested_jit_mode_preserved",
            ] {
                assert_eq!(state[flag], true);
            }
            assert_eq!(state["business_wrappers"], false);
            assert_eq!(rows(&state["saved_groups"]).len(), joins.len());
            for group in rows(&state["saved_groups"]) {
                assert_eq!(group["loaded_object_exact"], true);
                let join = joins
                    .iter()
                    .find(|j| j["source_ordinal"] == group["source_ordinal"])
                    .unwrap();
                assert_eq!(join["preset"], group["preset"]);
                if group["attributes"].get("source").is_none() || group["selected"] == false {
                    assert_eq!(group["runtime_present"], true);
                }
                for gem in rows(&group["gems"]) {
                    assert_eq!(gem["loaded_object_exact"], true);
                }
            }
            for mode in ["MAIN", "CALCS"] {
                assert_eq!(state["outputs"][mode]["available"], true);
            }
        }
    }
    let mut activated = 0;
    for original in 1..=5 {
        let path = format!("tests/fixtures/builds/breadth-20260908/build-{original:02}.xml");
        let bytes = fs::read(root().join(&path)).unwrap();
        let sha = format!("{:x}", Sha256::digest(&bytes));
        assert_eq!(
            report["original_sources"][original - 1],
            json!({"path":path,"sha256":sha})
        );
        let original_case = case(&report, &format!("original-{original:02}"));
        assert_eq!(original_case["xml_sha256"], sha);
        let imported = ImportedBuildInstance::from_decoded(
            decode_build(&bytes).unwrap(),
            BuildLineage::from_bytes([97; 16]),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let evidence =
            SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
        assert_eq!(original_case["source_identity"], json!(evidence.identity()));
        let source_rows = evidence.rows();
        let skills = source_rows
            .iter()
            .find(|r| r.occurrence().name() == "Skills")
            .unwrap();
        let active = skills
            .attribute("activeSkillSet")
            .unwrap()
            .decoded()
            .unwrap();
        for preset in source_rows
            .iter()
            .filter(|r| r.occurrence().name() == "SkillSet")
        {
            let id = preset.attribute("id").unwrap().decoded().unwrap();
            let observed = if id == active {
                original_case
            } else {
                case(
                    &report,
                    &format!("original-{original:02}-activate-preset-{id}"),
                )
            };
            for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
                assert_eq!(
                    observed["states"][stage]["selection"]["skills"],
                    id.parse::<u64>().unwrap()
                );
            }
            activated += 1;
        }
        for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
            let groups = rows(&original_case["states"][stage]["saved_groups"]);
            assert_eq!(
                groups.len(),
                source_rows
                    .iter()
                    .filter(|r| r.occurrence().name() == "Skill")
                    .count()
            );
            for group in groups {
                let row = &source_rows[group["source_ordinal"].as_u64().unwrap() as usize];
                assert_eq!(row.occurrence().name(), "Skill");
                assert_eq!(
                    group["attributes"].as_object().unwrap().len(),
                    row.attributes().len()
                );
                for attribute in row.attributes() {
                    assert_eq!(
                        group["attributes"][&attribute.origin().name],
                        attribute.decoded().unwrap()
                    );
                }
                for gem in rows(&group["gems"]) {
                    let row = &source_rows[gem["source_ordinal"].as_u64().unwrap() as usize];
                    assert_eq!(
                        row.occurrence().parent().unwrap().ordinal() as u64,
                        group["source_ordinal"].as_u64().unwrap()
                    );
                    assert_eq!(
                        gem["source"]["attributes"].as_object().unwrap().len(),
                        row.attributes().len()
                    );
                    for attribute in row.attributes() {
                        assert_eq!(
                            gem["source"]["attributes"][&attribute.origin().name],
                            attribute.decoded().unwrap()
                        );
                    }
                }
            }
        }
    }
    assert_eq!(activated, 15);
    let baseline = case(&report, "original-05");
    assert!(baseline["states"] == case(&report, "repeat-original-05")["states"]);
    let p = json!(policy());
    for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
        let state = &baseline["states"][stage];
        let saved: Vec<_> = rows(&state["saved_groups"])
            .iter()
            .filter(|g| g["selected"] == true)
            .collect();
        assert_eq!(saved.len(), 12);
        assert_eq!(
            saved
                .iter()
                .filter(|g| g["attributes"].get("source").is_none())
                .count(),
            9
        );
        for tuple in rows(&p["generated_groups"]) {
            let matching: Vec<_> = saved
                .iter()
                .filter(|g| {
                    rows(&g["gems"]).iter().any(|gem| {
                        gem["source"]["attributes"]["skillId"] == tuple["skill_id"]
                            && g["attributes"].get("source").is_some()
                    })
                })
                .collect();
            assert_eq!(matching.len(), 1);
            let group = matching[0];
            let gem = &rows(&group["gems"])[0];
            for (field, authored) in [
                ("gemId", "game_id"),
                ("variantId", "variant_id"),
                ("skillId", "skill_id"),
                ("nameSpec", "name_spec"),
            ] {
                assert_eq!(gem["source"]["attributes"][field], tuple[authored]);
            }
            assert_eq!(gem["state"]["catalog"]["game_id"], tuple["game_id"]);
            assert_eq!(gem["state"]["catalog"]["primary_effect"], tuple["skill_id"]);
            for mode in ["MAIN", "CALCS"] {
                let grants: Vec<_> = rows(&state["granted_skills"][mode])
                    .iter()
                    .filter(|g| g["fields"]["skillId"] == tuple["skill_id"])
                    .collect();
                assert_eq!(grants.len(), 1);
                assert_eq!(grants[0]["fields"]["source"], group["attributes"]["source"]);
                let joined = rows(&grants[0]["matched_groups"]);
                assert_eq!(joined.len(), 1);
                assert_eq!(joined[0]["source_ordinal"], group["source_ordinal"]);
                assert_eq!(joined[0]["group_source_item_exact"], true);
                assert_eq!(joined[0]["group_source_node_exact"], true);
            }
        }
        for (name, expected_runtime, expected_saved) in [
            ("remove-generated-firebolt", 1, 0),
            ("duplicate-generated-firebolt", 1, 2),
            ("remove-granting-item-leave-saved-group", 0, 1),
            ("unequip-granting-staff-leave-saved-group", 0, 1),
            ("manual-firebolt-with-staff", 2, 1),
            ("manual-firebolt-without-staff", 1, 1),
            ("empty-source-firebolt-with-staff", 1, 1),
            ("empty-source-firebolt-without-staff", 0, 1),
        ] {
            let state = &case(&report, name)["states"][stage];
            let count = |field: &str| {
                rows(&state[field])
                    .iter()
                    .filter(|g| {
                        g["selected"] == true
                            && rows(&g["gems"]).iter().any(|gem| {
                                gem["state"]["catalog"]["primary_effect"] == "FireboltPlayer"
                            })
                    })
                    .count()
            };
            assert_eq!(count("runtime_groups"), expected_runtime, "{name} {stage}");
            assert_eq!(count("saved_groups"), expected_saved, "{name} {stage}");
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    source_proof(&a);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "mapping",
        "roles",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[field], a[field], "exact predecessor {field}");
    }
    let b = prior.input();
    assert!(b.normalization.skill_inventory.is_none());
    assert_eq!(
        json!(prior.mapping().source_identity()),
        a["mapping_source"]
    );
    assert_eq!(
        json!(
            direct_skill_inputs_identity(
                b.normalization.direct_skill_inputs.as_ref().unwrap(),
                Default::default()
            )
            .unwrap()
        ),
        a["direct_inputs"]
    );
    let mut normalization = b.normalization.clone();
    normalization.skill_inventory = Some(policy());
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor: b.recipe.clone(),
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        b.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = b.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert!(
        full.normalization == normalization,
        "checked policy transition is exact"
    );
    assert!(full.tree.as_ref().unwrap().content == b.tree.as_ref().unwrap().content);
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("authored-skill-membership").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-authored-skill-membership-v1",
            &(a, policy()),
            2 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization = b.normalization.clone();
    restored.tree = b.tree.clone();
    restored.provenance.pop();
    assert!(
        restored == *b,
        "only membership policy, dependent tree commitment and provenance change"
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
