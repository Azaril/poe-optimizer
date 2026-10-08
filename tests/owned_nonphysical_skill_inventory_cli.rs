//! Nonphysical source rows do not become invented Gems or erase support children.
#[path = "support/owned_nonphysical_skill_inventory.rs"]
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
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, v: &impl Serialize) {
    fs::write(p, serde_json::to_vec(v).unwrap()).unwrap();
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
fn nonphysical_skill_inventory_policy_is_bound_to_source_and_reviewed_roles() {
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
        .find(|s| s.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        sidecar["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(package.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
    assert_eq!(sidecar["tree_policy"], json!(package.receipt().tree));
}
fn compare(
    case: usize,
    xml: &[u8],
    out: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    commitments(prior, case, &old, &sa);
    commitments(next, case, &new, &sb);
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    let mut retired = Vec::new();
    let mut completed = Vec::new();
    let aa = a["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let bb = b["draft"]["skill_presets"]["members"].as_array().unwrap();
    assert_eq!(aa.len(), bb.len());
    for (before, after) in aa.iter_mut().zip(bb) {
        assert_eq!(before["id"], after["id"]);
        let c = &mut before["supports"]["completion"];
        if c != &after["supports"]["completion"] {
            assert_eq!(c["kind"], "pending");
            assert_eq!(c["code"], "support-membership-not-converted");
            assert_eq!(after["supports"]["completion"], json!({"kind":"complete"}));
            retired.push(c["id"].clone());
            *c = after["supports"]["completion"].clone();
            completed.push(json!({"preset":before["id"], "physical_supports":before["supports"]["members"].as_array().unwrap().len()}));
        }
    }
    assert_eq!(
        a, b,
        "only proved inventory completion changes; every raw value, target, ID and watermark preserved"
    );
    for issue in &retired {
        let mut removed = 0;
        for origin in sa["origins"].as_array_mut().unwrap() {
            let links = origin["links"].as_array_mut().unwrap();
            let before = links.len();
            links.retain(|l| !(l["kind"] == "issue" && &l["value"] == issue));
            removed += before - links.len();
        }
        assert_eq!(removed, 1, "one exact source obligation");
    }
    for field in ["policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa, sb, "all other source provenance unchanged");
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
    expected
        .as_array_mut()
        .unwrap()
        .retain(|i| !retired.contains(&i["id"]));
    assert_eq!(expected, actual, "only physical-inventory issues retire");
    let mut selection = selected::selection(xml, &new);
    let mut previous = selected::selection(xml, &old);
    selected::canonical(&mut selection);
    selected::canonical(&mut previous);
    assert_eq!(selection, previous, "unchanged saved selection");
    if case == 5 {
        let preset = b["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == selection["build"]["skills"])
            .unwrap();
        let members = preset["supports"]["members"].as_array().unwrap();
        assert_eq!(members.len(), 16);
        let targets = b["draft"]["supports"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| members.contains(&s["id"]) && s["target"]["kind"] == "pending")
            .count();
        assert_eq!(
            targets, 6,
            "inventory proof preserves exact unresolved targets"
        );
        assert_eq!(
            preset["authored_support_order"]["completion"]["kind"],
            "pending"
        );
    }
    let before_count = before["finalization"]["issues"].as_array().unwrap().len();
    let after_count = after["finalization"]["issues"].as_array().unwrap().len();
    assert_eq!(before_count, [116, 116, 109, 121, 19][case - 1]);
    assert_eq!(after_count, [116, 116, 108, 121, 19][case - 1]);
    if case == 3 {
        assert_eq!(retired.len(), 1);
        assert_eq!(retired[0]["local"], "0000000000000085");
    } else {
        assert!(
            retired.is_empty(),
            "only the reviewed Original03 inventory changes"
        );
    }
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"completed":completed,"retired":retired,"calculation":"not_run"})
}

fn measured_selector_controls(
    prior_path: &Path,
    package: &Path,
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    out: &Path,
) -> Value {
    let authoring: Value =
        read(root().join("data/owned/poe2/3887ae68/nonphysical-skill-inventory/authoring.json"));
    assert_eq!(authoring["source_validation"]["status"], "passed");
    let evidence_path = root().join(
        authoring["source_validation"]["evidence_json"]
            .as_str()
            .unwrap(),
    );
    let evidence: Value = read(&evidence_path);
    let inputs = evidence_path.parent().unwrap().join("inputs");
    let controls = out.join("selector-controls");
    fs::create_dir_all(&controls).unwrap();
    let mut reports = Vec::new();
    for (name, complete, children) in [
        ("known-effect-misleading-name", true, 62),
        ("duplicate-saved-group", true, 63),
        ("unknown-effect-blank-name", false, 62),
        ("unknown-effect-support-name", false, 62),
        ("invalid-gem-id-blocks-effect", false, 62),
        ("empty-gem-id-blocks-effect", false, 62),
        ("invalid-gem-id-support-name", false, 62),
        ("physical-gem-id-wins", false, 62),
        ("append-physical-support", false, 63),
        ("unexpected-child-support", false, 63),
    ] {
        let matches: Vec<_> = evidence["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|case| case["name"] == name)
            .collect();
        assert_eq!(matches.len(), 1, "one measured source case {name}");
        let reference = matches[0];
        assert_eq!(reference["available"], true, "{name}");
        assert_eq!(reference["xml_selected"]["children"], children, "{name}");
        let xml = inputs.join(format!("{name}.xml"));
        let bytes = fs::read(&xml).unwrap();
        assert_eq!(
            reference["xml_sha256"],
            format!("{:x}", Sha256::digest(&bytes)),
            "actual source-tested XML {name}"
        );
        let case_out = controls.join(name);
        fs::create_dir_all(&case_out).unwrap();
        let old = case_out.join("prior");
        let new = case_out.join("next");
        release::normalize(prior_path, &xml, 3, &old);
        release::normalize(package, &xml, 3, &new);
        let mut a: Value = read(old.join("draft.json"));
        let mut b: Value = read(new.join("draft.json"));
        let mut sa: Value = read(old.join("sidecar.json"));
        let mut sb: Value = read(new.join("sidecar.json"));
        commitments(prior, 3, &old, &sa);
        commitments(next, 3, &new, &sb);
        for value in [&mut a, &mut b, &mut sa, &mut sb] {
            selected::canonical(value);
        }
        let mut selection = selected::selection(&bytes, &new);
        let mut before_selection = selected::selection(&bytes, &old);
        selected::canonical(&mut selection);
        selected::canonical(&mut before_selection);
        assert_eq!(selection, before_selection, "same saved request {name}");
        let preset_id = &selection["build"]["skills"];
        let before_preset = a["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|preset| &preset["id"] == preset_id)
            .unwrap();
        let after_preset = b["draft"]["skill_presets"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|preset| &preset["id"] == preset_id)
            .unwrap();
        assert_eq!(before_preset["supports"]["completion"]["kind"], "pending");
        assert_eq!(
            before_preset["supports"]["completion"]["code"],
            "support-membership-not-converted"
        );
        let retired = if complete {
            assert_eq!(
                after_preset["supports"]["completion"],
                json!({"kind":"complete"})
            );
            let id = before_preset["supports"]["completion"]["id"].clone();
            before_preset["supports"]["completion"] = json!({"kind":"complete"});
            let mut removed = 0;
            for origin in sa["origins"].as_array_mut().unwrap() {
                let links = origin["links"].as_array_mut().unwrap();
                let before = links.len();
                links.retain(|link| !(link["kind"] == "issue" && link["value"] == id));
                removed += before - links.len();
            }
            assert_eq!(removed, 1, "one exact retired source link {name}");
            Some(id)
        } else {
            assert_eq!(
                before_preset["supports"]["completion"], after_preset["supports"]["completion"],
                "unsupported selectors or support children cannot prove absence {name}"
            );
            None
        };
        for field in ["skills", "payload_links", "authored_support_order"] {
            assert_eq!(
                after_preset[field]["completion"]["kind"], "pending",
                "{name} {field}"
            );
        }
        assert_eq!(
            after_preset["supports"]["members"]
                .as_array()
                .unwrap()
                .len(),
            48
        );
        for (field, count) in [("gems", 57), ("skills", 9), ("supports", 48)] {
            assert_eq!(
                b["draft"][field]["members"].as_array().unwrap().len(),
                count,
                "no invented physical instances {name} {field}"
            );
        }
        assert_eq!(
            b["draft"]["supports"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["target"]["kind"] == "pending")
                .count(),
            10,
            "unresolved targets remain independent {name}"
        );
        assert_eq!(a, b, "exact same-XML facts and watermark {name}");
        for field in ["policy", "tree_policy", "draft"] {
            sb[field] = sa[field].clone();
        }
        assert_eq!(sa, sb, "exact same-XML provenance {name}");
        reports.push(json!({"case":name,"xml_sha256":reference["xml_sha256"],"inventory_complete":complete,"retired":retired}));
    }
    assert_eq!(reports.len(), 10);
    json!(reports)
}

#[test]
#[ignore = "requires the checked hit-chance predecessor and fresh nonphysical source proof"]
fn nonphysical_source_census_preserves_five_originals_and_unresolved_targets() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_NONPHYSICAL_SKILL_PRIOR")
            .expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_NONPHYSICAL_SKILL_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    for field in ["mapping_source", "roles"] {
        let mut bad = next.input().clone();
        let mut policy = json!(bad.normalization.support_origin_order.as_ref().unwrap());
        policy[field] = json!("00".repeat(32));
        bad.normalization.support_origin_order = Some(serde_json::from_value(policy).unwrap());
        bad.tree.as_mut().unwrap().normalization = digest_owned(
            "owned-normalization-policy-v3",
            &bad.normalization,
            TreePolicyLimits::default().max_base_policy_bytes,
        )
        .unwrap();
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {field} rejects despite rebound outer commitment"
        );
    }
    let mut bad = next.input().clone();
    bad.tree = prior.input().tree.clone();
    assert!(
        assemble_owned_release(bad, Default::default()).is_err(),
        "stale tree normalization binding"
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
        reports.push(compare(case, &fs::read(xml).unwrap(), &out, &prior, &next));
    }
    let controls = measured_selector_controls(&prior_path, &package, &prior, &next, &out);
    write(out.join("selector-controls.json"), &controls);
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"measured_selector_controls":10,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
