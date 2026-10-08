//! Empty authored payload relationships never imply skill or trigger coverage.
#[path = "support/owned_empty_payload_inventory.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    owned_normalize::NormalizationLimits,
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::TreePolicyLimits,
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
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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
#[test]
fn empty_payload_policy_covers_the_reviewed_catalog_without_new_definitions() {
    family::check_authored();
}
fn commitments(package: &StagedOwnedRelease, case: usize, path: &Path, sidecar: &Value) {
    let draft = decode_draft(
        &fs::read(path.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        sidecar["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &package
        .input()
        .query_sets
        .iter()
        .find(|set| set.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        sidecar["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(package.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes,
            )
            .unwrap()
        )
    );
    assert_eq!(sidecar["tree_policy"], json!(package.receipt().tree));
}

fn compare_drafts(
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    case: usize,
    old: &Path,
    new: &Path,
) -> Vec<Value> {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    commitments(prior, case, old, &sa);
    commitments(next, case, new, &sb);
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    let mut retired = Vec::new();
    let aa = a["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let bb = b["draft"]["skill_presets"]["members"].as_array().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (before, after) in aa.iter_mut().zip(bb) {
        assert_eq!(before["id"], after["id"]);
        let old = &mut before["payload_links"];
        let new = &after["payload_links"];
        if old["completion"] != new["completion"] {
            assert_eq!(old["members"], json!([]));
            assert_eq!(new["members"], json!([]));
            assert_eq!(old["completion"]["kind"], "pending");
            assert_eq!(
                old["completion"]["code"],
                "payload-membership-not-converted"
            );
            assert_eq!(new["completion"], json!({"kind":"complete"}));
            retired.push(old["completion"]["id"].clone());
            old["completion"] = new["completion"].clone();
        }
    }
    assert_eq!(
        a, b,
        "all physical inputs, supports, skills, relationships and allocator watermarks preserved"
    );
    for issue in &retired {
        let mut removed = 0;
        for origin in sa["origins"].as_array_mut().unwrap() {
            let links = origin["links"].as_array_mut().unwrap();
            let before = links.len();
            links.retain(|link| !(link["kind"] == "issue" && &link["value"] == issue));
            removed += before - links.len();
        }
        assert_eq!(removed, 1, "one exact retired source obligation");
    }
    for field in ["policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa, sb, "all other source evidence and identities preserved");
    retired
}

fn compare_original(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let retired = compare_drafts(prior, next, case, &old, &new);
    let before = selected::finalize(
        xml,
        &old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        &new,
        &out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        out.join(format!("original-{case:02}-prior-selected-report.json")),
        &before,
    );
    write(
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut expected = before["finalization"]["issues"].clone();
    let mut actual = after["finalization"]["issues"].clone();
    selected::canonical(&mut expected);
    selected::canonical(&mut actual);
    let selected_retired: Vec<_> = expected
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| retired.contains(&issue["id"]))
        .map(|issue| issue["id"].clone())
        .collect();
    expected
        .as_array_mut()
        .unwrap()
        .retain(|issue| !retired.contains(&issue["id"]));
    assert_eq!(
        expected, actual,
        "only empty-payload inventory issues retire"
    );
    let mut selection = selected::selection(xml, &new);
    let mut previous = selected::selection(xml, &old);
    selected::canonical(&mut selection);
    selected::canonical(&mut previous);
    assert_eq!(selection, previous, "identical saved selection and queries");
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count, [116, 116, 108, 121, 19][case - 1]);
    assert_eq!(after_count, [116, 116, 108, 121, 18][case - 1]);
    assert_eq!(selected_retired.len(), usize::from(case == 5));
    if matches!(case, 1 | 4) {
        assert!(
            retired.is_empty(),
            "unresolved or container rows remain Pending"
        );
    }
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"retired":retired,"selected_retired":selected_retired,"calculation":"not_run"})
}

fn measured_controls(
    prior_path: &Path,
    package: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    out: &Path,
) -> Value {
    let authoring: Value = family::read("authoring.json");
    let evidence_path = root().join(
        authoring["source_validation"]["evidence_json"]
            .as_str()
            .unwrap(),
    );
    let evidence: Value = read(&evidence_path);
    let inputs = evidence_path.parent().unwrap().join("inputs");
    let output = out.join("measured-controls");
    fs::create_dir_all(&output).unwrap();
    let mut reports = Vec::new();
    for (name, case, complete) in [
        ("disabled-meta-group", 4, false),
        ("duplicate-active-row", 5, false),
        ("unknown-gem-selector", 5, false),
        ("selected-contained-tornado", 4, false),
        ("selected-independent-tornado", 4, false),
        ("name-only-meta", 4, false),
        ("repeat-original-05", 5, true),
        ("warm-meta-to-original-05", 5, true),
    ] {
        let observed: Vec<_> = evidence["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["name"] == name)
            .collect();
        assert_eq!(observed.len(), 1);
        assert_eq!(observed[0]["available"], true);
        let xml_path = inputs.join(format!("{name}.xml"));
        let xml = fs::read(&xml_path).unwrap();
        assert_eq!(
            observed[0]["xml_sha256"],
            format!("{:x}", Sha256::digest(&xml)),
            "exact source-tested input {name}"
        );
        let directory = output.join(name);
        fs::create_dir_all(&directory).unwrap();
        let old = directory.join("prior");
        let new = directory.join("next");
        release::normalize(prior_path, &xml_path, case, &old);
        release::normalize(package, &xml_path, case, &new);
        let retired = compare_drafts(prior, next, case, &old, &new);
        let mut selection = selected::selection(&xml, &new);
        let mut old_selection = selected::selection(&xml, &old);
        selected::canonical(&mut selection);
        selected::canonical(&mut old_selection);
        assert_eq!(selection, old_selection);
        let mut draft: Value = read(new.join("draft.json"));
        selected::canonical(&mut draft);
        let preset = draft["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == selection["build"]["skills"])
            .unwrap();
        assert_eq!(preset["payload_links"]["members"], json!([]));
        assert_eq!(
            preset["payload_links"]["completion"]["kind"],
            if complete { "complete" } else { "pending" },
            "{name}"
        );
        if !complete {
            assert_eq!(
                preset["payload_links"]["completion"]["code"],
                "payload-membership-not-converted"
            );
        }
        for field in ["skills", "authored_support_order"] {
            assert_eq!(
                preset[field]["completion"]["kind"], "pending",
                "independent {name} {field}"
            );
        }
        reports.push(json!({"name":name,"xml_sha256":observed[0]["xml_sha256"],"selected_empty_inventory":complete,"retired":retired}));
    }
    json!(reports)
}

#[test]
#[ignore = "requires the checked plain-passive predecessor and fresh complete payload source proof"]
fn empty_payload_inventory_publication_preserves_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PAYLOAD_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_EMPTY_PAYLOAD_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    assert_eq!(
        prior
            .input()
            .query_sets
            .iter()
            .map(|s| s.queries.len())
            .sum::<usize>(),
        110
    );
    for field in ["mapping_source", "roles"] {
        let mut bad = next.input().clone();
        let mut policy = json!(bad.normalization.payload_inventory.as_ref().unwrap());
        policy[field] = json!("00".repeat(32));
        bad.normalization.payload_inventory = Some(serde_json::from_value(policy).unwrap());
        bad.tree.as_mut().unwrap().normalization = digest_owned(
            "owned-normalization-policy-v3",
            &bad.normalization,
            TreePolicyLimits::default().max_base_policy_bytes,
        )
        .unwrap();
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {field} rejects despite rebound outer binding"
        );
    }
    let mut bad = next.input().clone();
    bad.tree = prior.input().tree.clone();
    assert!(
        assemble_owned_release(bad, Default::default()).is_err(),
        "stale full tree normalization binding"
    );
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    let actual = release::inventory(&package);
    assert_eq!(actual, release::inventory(&rebuilt));
    assert_eq!(actual.len(), inventory.len());
    for (name, digest) in &inventory {
        if !matches!(
            name.as_str(),
            "normalization.json" | "tree-normalization.json" | "release.json"
        ) {
            assert_eq!(actual.get(name), Some(digest), "unchanged {name}");
        }
    }
    let mut reports = Vec::new();
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
        reports.push(compare_original(
            case,
            &fs::read(xml).unwrap(),
            &out,
            &prior,
            &next,
        ));
    }
    let controls = measured_controls(&prior_path, &package, &prior, &next, &out);
    write(out.join("measured-controls.json"), &controls);
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"stale_bindings":3,"measured_controls":controls.as_array().unwrap().len(),"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
