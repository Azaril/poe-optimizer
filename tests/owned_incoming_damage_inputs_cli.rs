//! Real incoming-hit input publication without claiming completed build evaluation.
#[path = "support/owned_incoming_damage_inputs.rs"]
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
    owned_normalize::{ConfigurationInputsPolicy, NormalizationLimits},
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
fn incoming_input_authoring_keeps_prior_numeric_lanes_and_exact_option_identity() {
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
    let receipt = json!(package.receipt());
    for (sidecar_key, receipt_key) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(
            sidecar[sidecar_key], receipt[receipt_key],
            "{sidecar_key} commitment"
        );
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
}
fn compare_original(
    case: usize,
    xml: &[u8],
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    out: &Path,
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
    let ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        inputs,
        option_inputs,
        ..
    } = family::policy()
    else {
        panic!()
    };
    let mut additions: Vec<_> = inputs
        .iter()
        .skip(6)
        .map(|input| {
            json!({
                "input":{"kind":"known","value":input.presence_input},
                "target":{"kind":"enemy"},
                "value":{"kind":"known","value":{"kind":"boolean","value":false}},
            })
        })
        .collect();
    assert_eq!(additions.len(), 8);
    let option = &option_inputs[0];
    additions.push(json!({
        "input":{"kind":"known","value":option.value_input},
        "target":{"kind":"enemy"},
        "value":{"kind":"known","value":{"kind":"option","value":option.constructor_default}},
    }));
    let aa = a["draft"]["scenario_presets"]["members"]
        .as_array()
        .unwrap();
    let bb = b["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(aa.len(), 1);
    assert_eq!(bb.len(), aa.len());
    for (before, after) in aa.iter().zip(bb) {
        assert_eq!(before["id"], after["id"]);
        let before = &before["scenario"]["assumptions"];
        let after = &mut after["scenario"]["assumptions"];
        assert_eq!(before["completion"], after["completion"]);
        assert_eq!(after["completion"]["kind"], "pending");
        let before_members = before["members"].as_array().unwrap();
        let after_members = after["members"].as_array_mut().unwrap();
        assert_eq!(after_members.len(), before_members.len() + additions.len());
        for addition in &additions {
            assert_eq!(
                after_members
                    .iter()
                    .filter(|row| row["input"] == addition["input"])
                    .count(),
                1
            );
            let at = after_members
                .iter()
                .position(|row| row["input"] == addition["input"])
                .unwrap();
            assert_eq!(
                &after_members[at], addition,
                "absence or exact constructor option only"
            );
            after_members.remove(at);
        }
        assert_eq!(
            &*after_members, before_members,
            "all prior inputs retain their original values and relative order"
        );
    }
    assert!(
        a == b,
        "original {case}: unrelated physical inputs, skill/support targets, relationships or allocator changed"
    );
    let old_items = sa["item_texts"].as_array().unwrap();
    let new_item_policy = sb["item_policy"].clone();
    let new_source_policy = sb["item_source_policy"].clone();
    let new_items = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_items.len(), new_items.len());
    for (old, new) in old_items.iter().zip(new_items) {
        assert_eq!(old["attribution"]["policy"], sa["item_source_policy"]);
        assert_eq!(old["attribution"]["item_lines"], sa["item_policy"]);
        assert_eq!(new["attribution"]["policy"], new_source_policy);
        assert_eq!(new["attribution"]["item_lines"], new_item_policy);
        new["attribution"]["policy"] = old["attribution"]["policy"].clone();
        new["attribution"]["item_lines"] = old["attribution"]["item_lines"].clone();
    }
    for field in [
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
        "draft",
    ] {
        sb[field] = sa[field].clone();
    }
    assert!(
        sa == sb,
        "original {case}: source evidence changed outside authenticated dependency commitments"
    );
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
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert!(
        old_issues == new_issues,
        "raw configuration inputs do not retire inventory or readiness obligations"
    );
    let count = new_issues.as_array().unwrap().len();
    assert_eq!(count, [116, 116, 108, 121, 18][case - 1]);
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(
        old_selection, new_selection,
        "saved physical selections and query IDs are preserved"
    );
    json!({"original":case,"selected_before":count,"selected_after":count,
        "added_raw_absence_inputs":8,"added_default_option_inputs":1,
        "selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
#[test]
#[ignore = "requires the checked empty-payload predecessor and passed incoming-damage source witness"]
fn incoming_damage_inputs_publication_preserves_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_INCOMING_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_INCOMING_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    assert_eq!(prior.receipt().query_rows, 110);
    assert_eq!(next.receipt().query_rows, 110);
    for kind in ["mapping", "tree"] {
        let mut bad = next.input().clone();
        if kind == "mapping" {
            let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { mapping_source, .. }) =
                &mut bad.normalization.configuration_inputs
            else {
                panic!()
            };
            *mapping_source = prior.receipt().input;
            bad.tree.as_mut().unwrap().normalization = digest_owned(
                "owned-normalization-policy-v3",
                &bad.normalization,
                TreePolicyLimits::default().max_base_policy_bytes,
            )
            .unwrap();
        } else {
            bad.tree = prior.input().tree.clone();
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {kind} binding is rejected"
        );
    }
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    let published = release::inventory(&package);
    assert_eq!(published, release::inventory(&rebuilt));
    assert_eq!(published.len(), inventory.len());
    for case in 1..=5 {
        let file = format!("queries-original-{case:02}.json");
        assert_eq!(published.get(&file), inventory.get(&file));
        assert_eq!(
            fs::read(package.join(&file)).unwrap(),
            fs::read(prior_path.join(file)).unwrap()
        );
    }
    let mut reports = Vec::new();
    for case in 1..=5 {
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&path).unwrap();
        release::normalize(
            &prior_path,
            &path,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &path,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let mut report = compare_original(case, &xml, &prior, &next, &out);
        report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&xml)));
        reports.push(report);
    }
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":reports,
            "queries":110,"stale_bindings":2,"prior_unchanged":true,"rebuild_byte_identical":true,
            "allocated_definitions":43,"added_programs":1,"added_tables":1,
            "operations_version":"owned-domain-operations-v15","complete_original_builds":0,
        }),
    );
}
