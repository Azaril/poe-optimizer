//! Current-release count transport, with exact unchanged-corpus preservation.
#[allow(dead_code)]
#[path = "support/owned_ice_nova_count_inputs.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SlotDescriptor};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, configuration_disposition_dependencies, usage_inputs_identity,
    },
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_skill_roles::OwnedGemRoleRow,
    owned_successor::{SuccessorBundleInput, transition_owned_normalization_with_tree_compact},
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

const BEFORE: &str = "2d307fd317100b5282026a38d91c281a937eb850e275664d8c5ddfc9ee2c6560";

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    assert_eq!(prior.receipt().input.to_string(), BEFORE);
    let old = prior.input();
    let dependencies: Value = family::read("dependencies.json");
    let definitions: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    let slots: Vec<SlotDescriptor> = serde_json::from_value(dependencies["slots"].clone()).unwrap();
    let roles: Vec<OwnedGemRoleRow> =
        serde_json::from_value(dependencies["roles"].clone()).unwrap();
    for definition in definitions {
        assert_eq!(
            old.recipe
                .schema
                .definitions
                .iter()
                .filter(|row| **row == definition)
                .count(),
            1
        );
    }
    for slot in slots {
        assert_eq!(
            old.recipe
                .schema
                .slots
                .iter()
                .filter(|row| **row == slot)
                .count(),
            1
        );
    }
    for role in roles {
        assert_eq!(
            old.roles.roles.iter().filter(|row| **row == role).count(),
            1
        );
    }
    let mut normalization = old.normalization.clone();
    family::checked_append(&mut normalization).unwrap();
    let identity = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("current checked physical inventory")
    };
    *usage_inputs = identity;
    let dependencies =
        configuration_disposition_dependencies(&normalization, prior.rewards(), Default::default())
            .unwrap();
    let disposition = normalization.configuration_dispositions.as_mut().unwrap();
    let mut dependency_inverse = dependencies.clone();
    assert_eq!(dependencies.usage_inputs, identity);
    dependency_inverse.usage_inputs = disposition.dependencies.usage_inputs;
    assert_eq!(
        dependency_inverse, disposition.dependencies,
        "the only changed configuration-adapter dependency is usage input"
    );
    disposition.dependencies = dependencies;
    let transition = transition_owned_normalization_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: old.recipe.clone(),
            successor: old.recipe.clone(),
            mapping: old.mapping.clone(),
            roles: old.roles.clone(),
            normalization: old.normalization.clone(),
            rewards: old.rewards.clone(),
            query_sets: old.query_sets.clone(),
            items: old.items.clone(),
            item_source: old.item_source.clone(),
        },
        old.tree.clone().unwrap(),
        normalization.clone(),
        Default::default(),
    )
    .unwrap();
    let mut full = old.clone();
    full.normalization = transition.normalization().clone();
    full.tree = transition.tree().map(|tree| tree.input().clone());
    assert_eq!(full.normalization, normalization);
    assert_eq!(
        full.tree.as_ref().unwrap().content,
        old.tree.as_ref().unwrap().content
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("ice-nova-requested-count").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(
            "owned-ice-nova-count-inputs-v1",
            &read(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("data/owned/poe2/3887ae68/ice-nova-count-inputs/authoring.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    inverse.normalization = old.normalization.clone();
    inverse.tree = old.tree.clone();
    inverse.provenance.pop();
    assert!(
        inverse == *old,
        "only checked import policy/tree commitments and provenance change"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.query_sets(), prior.query_sets());
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

fn compare(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior_path: &Path,
    package: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    let old_dir = out.join(format!("prior-original-{case:02}"));
    let new_dir = out.join(format!("original-{case:02}"));
    let mut before = read(old_dir.join("draft.json"));
    let mut after = read(new_dir.join("draft.json"));
    let mut old_sidecar = read(old_dir.join("sidecar.json"));
    let mut new_sidecar = read(new_dir.join("sidecar.json"));
    preservation::authenticate(&old_sidecar, &old_dir, prior_path, case, prior);
    preservation::authenticate(&new_sidecar, &new_dir, package, case, next);
    let mut selection = selected::selection(xml, &new_dir);
    for value in [
        &mut before,
        &mut after,
        &mut old_sidecar,
        &mut new_sidecar,
        &mut selection,
    ] {
        selected::canonical(value);
    }
    let mut total = 0;
    let mut active = 0;
    let old_presets = before["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap();
    let new_presets = after["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(old_presets.len(), new_presets.len());
    for (old, new) in old_presets.iter().zip(new_presets) {
        assert_eq!(old["id"], new["id"]);
        let is_selected = new["id"] == selection["build"]["skills"];
        let old_rows = old["intent"]["usage"]["members"].as_array().unwrap();
        let rows = new["intent"]["usage"]["members"].as_array_mut().unwrap();
        let expected: Vec<_> = old_rows
            .iter()
            .filter(|row| {
                row["selection"]["policy"]["value"]["key"] == "def.000000000000338e"
                    && row["selection"]["target"]["value"]["value"]["slot"]["value"]["declaration"]
                        ["definition"]["key"]
                        == "def.00000000000007dd"
            })
            .collect();
        for reporting in expected {
            let target = &reporting["selection"]["target"];
            let positions: Vec<_> = rows
                .iter()
                .enumerate()
                .filter_map(|(index, row)| {
                    (row["selection"]["policy"]["value"]["key"] == "def.000000000000326a"
                        && &row["selection"]["target"] == target)
                        .then_some(index)
                })
                .collect();
            assert_eq!(
                positions.len(),
                1,
                "one count for the existing exact physical target"
            );
            assert!(!old_rows.iter().any(|row| {
                row["selection"]["policy"]["value"]["key"] == "def.000000000000326a"
                    && &row["selection"]["target"] == target
            }));
            let added = rows.remove(positions[0]);
            assert_eq!(added["applicability"], "required");
            assert_eq!(
                added["selection"]["parameters"]["completion"],
                json!({"kind":"complete"})
            );
            let parameters = added["selection"]["parameters"]["members"]
                .as_array()
                .unwrap();
            assert_eq!(parameters.len(), 1);
            assert_eq!(
                parameters[0]["slot"]["value"]["slot"]["key"],
                "def.000000000000326b"
            );
            assert_eq!(
                parameters[0]["value"],
                json!({"kind":"known","value":{"kind":"integer","value":1}})
            );
            total += 1;
            active += usize::from(is_selected);
        }
        assert_eq!(
            *rows, *old_rows,
            "all previous preferences and order survive"
        );
    }
    assert_eq!(
        before, after,
        "exact whole-draft inverse; no existing value or obligation changes"
    );
    assert_eq!(total, [0, 0, 0, 0, 4][case - 1]);
    assert_eq!(active, [0, 0, 0, 0, 1][case - 1]);
    // All three changed commitments were authenticated above. Preserve every
    // source/attribution/link/allocator field, including origins for the new use.
    for field in ["policy", "tree_policy", "draft"] {
        new_sidecar[field] = old_sidecar[field].clone();
    }
    assert_eq!(old_sidecar, new_sidecar);
    let old_report = selected::finalize_with_definitions(
        xml,
        &old_dir,
        &out.join(format!("prior-selection-{case:02}.json")),
        &prior_path.join("schema.json"),
    );
    let new_report = selected::finalize_with_definitions(
        xml,
        &new_dir,
        &out.join(format!("selection-{case:02}.json")),
        &package.join("schema.json"),
    );
    let mut old_issues = old_report["finalization"]["issues"].clone();
    let mut new_issues = new_report["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert_eq!(old_issues, new_issues);
    assert_eq!(
        new_issues.as_array().unwrap().len(),
        [107, 118, 110, 124, 5][case - 1]
    );
    write(out.join(format!("report-{case:02}.json")), &new_report);
    json!({"original":case,"all_preset_added_counts":total,"selected_added_counts":active,
        "selected_unresolved":new_issues.as_array().unwrap().len(),"calculation":"not_run"})
}

#[test]
#[ignore = "explicit immutable paths; publishes and checks all five unchanged originals"]
fn publish_ice_nova_counts_preserving_all_other_inputs_and_coverage() {
    let prior_path = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_COUNTS_PRIOR").unwrap(),
    );
    let out = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_COUNTS_OUTPUT").unwrap(),
    );
    assert!(!out.exists(), "immutable evidence output");
    let prior_files = release::inventory(&prior_path);
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
    assert_eq!(
        release::inventory(&package).keys().collect::<Vec<_>>(),
        prior_files.keys().collect::<Vec<_>>()
    );
    let mut originals = Vec::new();
    for case in 1..=5 {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml",
        ));
        let bytes = fs::read(&source).unwrap();
        release::normalize(
            &prior_path,
            &source,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &source,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(compare(
            case,
            &bytes,
            &out,
            &prior_path,
            &package,
            &prior,
            &next,
        ));
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
    assert_eq!(originals[4]["selected_added_counts"], 1);
    assert_eq!(prior_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
            "queries":110,"rebuild_byte_identical":true,"prior_unchanged":true,
            "complete_original_builds":0,"evaluation_bundle_added":false,
            "retired_input_issues":0,"exact_import_inverse":true
        }),
    );
}

// Fresh CLI imports create separate projects with random lineages. Authenticate
// those identities and their derived commitments before comparing build content.
fn canonical_checkpoint_lineage(value: &mut Value, expected: &Value) {
    match value {
        Value::Object(fields) => {
            if let Some(lineage) = fields.get_mut("lineage") {
                assert_eq!(lineage, expected, "one exact project lineage");
                *lineage = json!("canonical-test-lineage");
            }
            for child in fields.values_mut() {
                canonical_checkpoint_lineage(child, expected);
            }
        }
        Value::Array(children) => {
            for child in children {
                canonical_checkpoint_lineage(child, expected);
            }
        }
        _ => {}
    }
}

fn checkpoint_outputs(root: &Path, case: usize, release: &StagedOwnedRelease) -> [Value; 3] {
    let directory = root.join(format!("original-{case:02}"));
    let mut draft = read(directory.join("draft.json"));
    let sidecar_bytes = fs::read(directory.join("sidecar.json")).unwrap();
    let mut sidecar: Value = serde_json::from_slice(&sidecar_bytes).unwrap();
    let mut report = read(directory.join("report.json"));
    preservation::authenticate(&sidecar, &directory, &root.join("package"), case, release);
    let lineage = sidecar["allocator_before"]["lineage"].clone();
    assert!(lineage.is_string());
    assert_eq!(report["allocator_before"], sidecar["allocator_before"]);
    assert_eq!(report["allocator_after"], sidecar["allocator_after"]);
    assert_eq!(report["draft_digest"], sidecar["draft"]);
    assert_eq!(
        report["sidecar_sha256"],
        json!(format!("{:x}", Sha256::digest(&sidecar_bytes)))
    );
    assert_eq!(report["sidecar_bytes"], json!(sidecar_bytes.len()));
    assert_eq!(
        report["output"]["directory"],
        json!(fs::canonicalize(&directory).unwrap())
    );
    // Only authenticated project identity, derived digests/length and actual
    // output location differ. Preserve all values, IDs, origins and diagnostics.
    sidecar["draft"] = Value::Null;
    report["draft_digest"] = Value::Null;
    report["sidecar_sha256"] = Value::Null;
    report["sidecar_bytes"] = Value::Null;
    report["output"]["directory"] = Value::Null;
    for value in [&mut draft, &mut sidecar, &mut report] {
        canonical_checkpoint_lineage(value, &lineage);
    }
    [draft, sidecar, report]
}

#[test]
#[ignore = "explicit immutable checkpoint paths; authenticates all five prior and replayed imports"]
fn replayed_checkpoint_preserves_all_build_content() {
    let baseline = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_COUNTS_BASELINE").unwrap(),
    );
    let replay = std::path::PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ICE_NOVA_COUNTS_REPLAY").unwrap(),
    );
    let baseline_package = baseline.join("package");
    let replay_package = replay.join("package");
    assert_eq!(
        release::inventory(&baseline_package),
        release::inventory(&replay_package),
        "every package file is byte-identical"
    );
    let release = release::load(&replay_package);
    for case in 1..=5 {
        assert_eq!(
            checkpoint_outputs(&baseline, case, &release),
            checkpoint_outputs(&replay, case, &release),
            "Original{case:02}: only authenticated project identity and output metadata may differ"
        );
    }
}
