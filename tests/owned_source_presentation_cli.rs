//! Checked presentation dispositions preserve every original native obligation.
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::BuildLineage, owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::SourcePresentationPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/source-presentation-v1")
}
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn hash(path: impl AsRef<Path>) -> String {
    format!("{:x}", Sha256::digest(fs::read(path).unwrap()))
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}

#[test]
fn presentation_authoring_is_pinned_and_has_no_native_rules() {
    let authoring: Value = read(data().join("authoring.json"));
    let policy: SourcePresentationPolicy = read(data().join("policy.json"));
    assert!(matches!(
        policy,
        SourcePresentationPolicy::PobFreshPresentationV1 {
            empty_socket_urls: true,
            calcs_sections: true,
            tree_view: true,
            empty_notes: true,
            cached_build_buffs: true,
            ..
        }
    ));
    for (field, file) in [
        ("policy", "policy.json"),
        ("source_facts", "source-facts.json"),
    ] {
        assert_eq!(authoring["artifact_sha256"][field], hash(data().join(file)));
    }
    for field in ["new_definitions", "new_slots", "new_programs", "new_tables"] {
        assert_eq!(authoring[field], 0);
    }
    assert_eq!(authoring["inventory_retirement"], "none");
    let path = root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    assert_eq!(authoring["source_manifest_sha256"], hash(&path));
    let manifest: Value = read(path);
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    let facts: Value = read(data().join("source-facts.json"));
    assert_eq!(facts["source_revision"], authoring["source_revision"]);
    assert_eq!(facts["runtime_observations"], "none");
    for excerpt in facts["excerpts"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|file| file["path"] == excerpt["file"]["path"]
                    && file["sha256"] == excerpt["file"]["sha256"])
        );
        assert_eq!(
            excerpt["lines"].as_array().unwrap().len() as u64,
            excerpt["last_line"].as_u64().unwrap() - excerpt["first_line"].as_u64().unwrap() + 1
        );
    }
}

fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    presentation_authoring_is_pinned_and_has_no_native_rules();
    let authoring: Value = read(data().join("authoring.json"));
    assert_eq!(authoring["before"], json!(prior.receipt().input));
    let facts: Value = read(data().join("source-facts.json"));
    for excerpt in facts["excerpts"].as_array().unwrap() {
        let source = fs::read_to_string(
            root()
                .join("vendor/path-of-building-poe2")
                .join(excerpt["file"]["path"].as_str().unwrap()),
        )
        .unwrap()
        .replace("\r\n", "\n");
        assert_eq!(
            format!("{:x}", Sha256::digest(source.as_bytes())),
            excerpt["file"]["sha256"]
        );
        let first = excerpt["first_line"].as_u64().unwrap() as usize;
        let last = excerpt["last_line"].as_u64().unwrap() as usize;
        assert_eq!(
            json!(
                source
                    .lines()
                    .skip(first - 1)
                    .take(last - first + 1)
                    .collect::<Vec<_>>()
            ),
            excerpt["lines"]
        );
    }
    let before = prior.input();
    let mut policy = before.normalization.clone();
    let proposed: SourcePresentationPolicy = read(data().join("policy.json"));
    let mut inherited = proposed.clone();
    let SourcePresentationPolicy::PobFreshPresentationV1 {
        cached_build_buffs, ..
    } = &mut inherited;
    assert!(*cached_build_buffs);
    *cached_build_buffs = false;
    assert_eq!(
        policy.source_presentation,
        Some(inherited),
        "the current policy differs only by the new opt-in"
    );
    policy.source_presentation = Some(proposed);
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: before.recipe.clone(),
            successor: before.recipe.clone(),
            mapping: before.mapping.clone(),
            roles: before.roles.clone(),
            normalization: before.normalization.clone(),
            rewards: before.rewards.clone(),
            query_sets: before.query_sets.clone(),
            items: before.items.clone(),
            item_source: before.item_source.clone(),
        },
        before.tree.clone().unwrap(),
        policy.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = before.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, policy);
    let mut tree_inverse = full.tree.clone().unwrap();
    tree_inverse.normalization = before.tree.as_ref().unwrap().normalization;
    assert_eq!(&tree_inverse, before.tree.as_ref().unwrap());
    full.provenance.push(OwnedReleaseProvenance {
        kind: key("reviewed-cached-build-buffs"),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-source-presentation-v1",
            &(authoring, policy.source_presentation),
            1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    inverse.normalization = before.normalization.clone();
    inverse.tree = before.tree.clone();
    inverse.provenance.pop();
    assert_eq!(&inverse, before);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

fn compare(case: usize, xml: &[u8], prior: &Path, out: &Path) -> Value {
    let before_dir = out.join(format!("prior-original-{case:02}"));
    let after_dir = out.join(format!("original-{case:02}"));
    let mut before: Value = read(before_dir.join("draft.json"));
    let mut after: Value = read(after_dir.join("draft.json"));
    selected::canonical(&mut before);
    selected::canonical(&mut after);
    assert_eq!(before, after, "no native input, issue or allocator changes");
    let mut historical: Value = read(
        prior
            .parent()
            .unwrap()
            .join(format!("original-{case:02}/draft.json")),
    );
    selected::canonical(&mut historical);
    assert_eq!(
        before, historical,
        "cached output accounting preserves the checked current baseline"
    );
    let before_side: Value = read(before_dir.join("sidecar.json"));
    let after_side: Value = read(after_dir.join("sidecar.json"));
    assert_eq!(before_side["schema_version"], 21);
    assert_eq!(after_side["schema_version"], 21);
    let mut old_origins = before_side["origins"].clone();
    let mut new_origins = after_side["origins"].clone();
    selected::canonical(&mut old_origins);
    selected::canonical(&mut new_origins);
    let instance = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&instance, SourceEvidenceLimits::default()).unwrap();
    let mut expected = std::collections::BTreeMap::new();
    for row in evidence.rows() {
        let parent = row
            .occurrence()
            .parent()
            .map(|id| &evidence.rows()[id.ordinal() as usize]);
        let parent_name = parent.map(|row| row.occurrence().name());
        let reason = match (row.occurrence().name(), parent_name) {
            ("Buffs", Some("Build")) => {
                assert!(row.children().is_empty());
                assert!(!row.occurrence().has_namespace_context());
                assert!(row.attributes().iter().all(|a| {
                    ["buffList", "combatList", "curseList"].contains(&a.origin().name.as_str())
                        && a.decoded().is_ok()
                }));
                let root = parent.unwrap().occurrence().parent().unwrap();
                assert_eq!(root.ordinal(), 0);
                assert_eq!(evidence.rows()[0].occurrence().name(), "PathOfBuilding2");
                Some("source-cached-build-buffs")
            }
            _ => None,
        };
        if let Some(reason) = reason {
            expected.insert(u64::from(row.occurrence().id().ordinal()), reason);
        }
    }
    assert_eq!(expected.len(), 1);
    assert_eq!(
        old_origins.as_array().unwrap().len(),
        new_origins.as_array().unwrap().len()
    );
    let mut changed = std::collections::BTreeMap::<String, usize>::new();
    for (old, new) in old_origins
        .as_array()
        .unwrap()
        .iter()
        .zip(new_origins.as_array().unwrap())
    {
        if old == new {
            continue;
        }
        assert_eq!(old["source"], new["source"]);
        assert_eq!(old["disposition"]["kind"], "contributes");
        assert!(
            old["links"]
                .as_array()
                .unwrap()
                .iter()
                .all(|link| link["kind"] == "issue")
        );
        assert_eq!(new["disposition"]["kind"], "source_only");
        assert!(new["links"].as_array().unwrap().is_empty());
        let reason = new["disposition"]["value"].as_str().unwrap().to_owned();
        assert_eq!(
            expected.remove(&new["source"]["ordinal"].as_u64().unwrap()),
            Some(reason.as_str())
        );
        *changed.entry(reason).or_default() += 1;
    }
    assert!(
        expected.is_empty(),
        "the one independently censused cached Buffs leaf is accounted for"
    );
    if case == 5 {
        let count = |side: &Value| {
            side["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|origin| {
                    origin["links"].as_array().unwrap().iter().any(|link| {
                        link["kind"] == "issue" && link["value"]["local"] == "00000000000001f2"
                    })
                })
                .count()
        };
        let owned_ranges = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "ModRange")
            .filter(|row| {
                before_side["origins"][row.occurrence().id().ordinal() as usize]["links"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|link| link["kind"] == "item")
            })
            .count();
        assert_eq!(owned_ranges, 101);
        assert_eq!(count(&before_side), 73);
        assert_eq!(count(&after_side), 72);
    }
    let mut old_side = before_side;
    for field in [
        "schema_version",
        "policy",
        "tree_policy",
        "draft",
        "origins",
    ] {
        old_side[field] = after_side[field].clone();
    }
    let mut new_side = after_side;
    selected::canonical(&mut old_side);
    selected::canonical(&mut new_side);
    assert_eq!(
        old_side, new_side,
        "all other source metadata and item provenance survives"
    );
    let mut old_selection = selected::selection(xml, &before_dir);
    let mut new_selection = selected::selection(xml, &after_dir);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    let old_report = selected::finalize_with_definitions(
        xml,
        &before_dir,
        &out.join(format!("before-selection-{case:02}.json")),
        &prior.join("schema.json"),
    );
    let new_report = selected::finalize_with_definitions(
        xml,
        &after_dir,
        &out.join(format!("selection-{case:02}.json")),
        &out.join("package/schema.json"),
    );
    for report in [&old_report, &new_report] {
        assert!(
            report["intent_validation"]["schema_issues"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let mut old_final = old_report["finalization"].clone();
    let mut new_final = new_report["finalization"].clone();
    // Fresh CLI imports allocate independent document lineages. Authenticate
    // each digest against its actual draft receipt before comparing canonical
    // semantic content (whose complete draft equality was already checked).
    assert_eq!(
        old_final["draft_digest"],
        read::<Value>(before_dir.join("sidecar.json"))["draft"]
    );
    assert_eq!(
        new_final["draft_digest"],
        read::<Value>(after_dir.join("sidecar.json"))["draft"]
    );
    old_final.as_object_mut().unwrap().remove("draft_digest");
    new_final.as_object_mut().unwrap().remove("draft_digest");
    selected::canonical(&mut old_final);
    selected::canonical(&mut new_final);
    assert_eq!(old_final, new_final, "every selected obligation survives");
    json!({"original":case,"dispositions":changed,"finalization":new_final,"draft_unchanged":true,"selection_unchanged":true})
}

#[test]
#[ignore = "requires the checked current release and pinned source checkout"]
fn presentation_publication_preserves_all_five_original_requests() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_PRIOR").expect("explicit prior package"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PRESENTATION_OUTPUT").expect("new output directory"),
    );
    assert!(!out.exists());
    let old_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    let mut originals = Vec::new();
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(compare(case, &fs::read(xml).unwrap(), &prior_path, &out));
    }
    assert_eq!(old_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"rebuild_byte_identical":true,"complete_original_builds":0,
            "numerical_rules_unchanged":true,"source_execution":"not_run"
        }),
    );
}
