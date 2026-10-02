//! Publish real Offering data without manufacturing its still-unresolved inputs.
#[path = "support/owned_pain_offering.rs"]
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
    owned_normalize::NormalizationLimits, owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
fn offering_declarations_and_applications_preserve_partial_coverage() {
    family::check_authored();
}
fn check_bindings(p: &StagedOwnedRelease, case: usize, directory: &Path, side: &Value) {
    let receipt = json!(p.receipt());
    for (s, r) in [
        ("mapping", "mapping"),
        ("mapping_source", "mapping_source"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        let expected = if r == "mapping_source" {
            json!(p.mapping().source_identity())
        } else {
            receipt[r].clone()
        };
        assert_eq!(side[s], expected, "authenticated {s}");
    }
    for item in side["item_texts"].as_array().unwrap() {
        assert_eq!(item["attribution"]["policy"], receipt["item_source"]);
        assert_eq!(item["attribution"]["item_lines"], receipt["items"]);
    }
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    assert_eq!(
        side["draft"],
        json!(
            draft
                .digest(DraftLimits::default().input.max_wire_bytes)
                .unwrap()
        )
    );
    let queries = &p
        .input()
        .query_sets
        .iter()
        .find(|q| q.name.as_str() == format!("original-{case:02}"))
        .unwrap()
        .queries;
    assert_eq!(
        side["policy"],
        json!(
            digest_owned(
                "owned-normalization-policy-v3",
                &(p.normalization(), queries),
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
}
#[test]
#[ignore = "requires checked nonphysical-inventory predecessor and fresh complete Offering source witness"]
fn offering_release_preserves_all_five_original_requests() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PAIN_OFFERING_PRIOR").expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PAIN_OFFERING_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(publish(&package, &rebuilt), json!(next.receipt()));
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    assert_eq!(prior.input().query_sets, next.input().query_sets);
    let mut reports = Vec::new();
    for (case, expected_issues) in (1..=5).zip([116, 116, 108, 121, 19]) {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        let mut a: Value = read(old.join("draft.json"));
        let mut b: Value = read(new.join("draft.json"));
        let mut sa: Value = read(old.join("sidecar.json"));
        let mut sb: Value = read(new.join("sidecar.json"));
        check_bindings(&prior, case, &old, &sa);
        check_bindings(&next, case, &new, &sb);
        for v in [&mut a, &mut b, &mut sa, &mut sb] {
            selected::canonical(v);
        }
        assert!(
            a == b,
            "all raw inputs, support assignments, Pending targets, source IDs, gaps and allocator preserved"
        );
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
        let old_items = sa["item_texts"].as_array().unwrap();
        let new_items = sb["item_texts"].as_array_mut().unwrap();
        assert_eq!(old_items.len(), new_items.len());
        for (old, new) in old_items.iter().zip(new_items) {
            for field in ["policy", "item_lines"] {
                new["attribution"][field] = old["attribution"][field].clone();
            }
        }
        assert!(
            sa == sb,
            "all provenance except authenticated dependency bindings preserved"
        );
        let xml = fs::read(xml).unwrap();
        let before = selected::finalize(
            &xml,
            &old,
            &out.join(format!("original-{case:02}-prior-selection.json")),
        );
        let after = selected::finalize(
            &xml,
            &new,
            &out.join(format!("original-{case:02}-selection.json")),
        );
        let mut x = before["finalization"]["issues"].clone();
        let mut y = after["finalization"]["issues"].clone();
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(
            x, y,
            "data publication cannot close unresolved source inputs"
        );
        assert_eq!(y.as_array().unwrap().len(), expected_issues);
        let mut x = selected::selection(&xml, &old);
        let mut y = selected::selection(&xml, &new);
        selected::canonical(&mut x);
        selected::canonical(&mut y);
        assert_eq!(x, y);
        write(
            out.join(format!("original-{case:02}-selected-report.json")),
            &after,
        );
        reports.push(
            json!({"original":case,"selected_issues":expected_issues,"calculation":"not_run"}),
        );
    }
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,
            "originals":reports,"queries":110,"prior_unchanged":true,
            "rebuild_byte_identical":true,"complete_original_builds":0,
            "operations":"owned-domain-operations-v15","application_coverage":"partial",
            "new_definition_range":["def.0000000000003221","def.000000000000322d"],
            "pending_producers":family::read::<Value>("authoring.json")["pending_producers"]
        }),
    );
}
