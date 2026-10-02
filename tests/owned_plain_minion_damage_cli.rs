//! Real release publication for static passive producers, without whole-build closure.
#[path = "support/owned_plain_minion_damage.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::owned_draft::{DraftLimits, decode_draft};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
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
fn authored_passives_preserve_partial_coverage_and_exact_source_values() {
    family::check_authored();
    let selected = family::selected_values();
    assert_eq!(selected.len(), 10);
    assert_eq!(selected.values().sum::<f64>(), 68.0);
}
fn check_sidecar_digest(directory: &Path, sidecar: &Value) {
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
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
}
fn selected_passives(directory: &Path, selection: &Value) -> BTreeMap<String, f64> {
    let bindings: family::Bindings = family::read("bindings.json");
    let draft: Value = read(directory.join("draft.json"));
    let draft = &draft["draft"];
    let presets: Vec<_> = draft["allocation_presets"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|p| p["id"] == selection["build"]["allocations"])
        .collect();
    assert_eq!(presets.len(), 1);
    let allocations = &presets[0]["allocations"];
    assert_eq!(allocations["completion"]["kind"], "complete");
    let mut values = BTreeMap::new();
    for id in allocations["members"].as_array().unwrap() {
        let matching: Vec<_> = draft["allocations"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|a| a["id"] == *id)
            .collect();
        assert_eq!(matching.len(), 1);
        let allocation = matching[0];
        if let Some(binding) = bindings
            .nodes
            .iter()
            .find(|b| allocation["node"]["value"] == json!(b.node))
        {
            assert_eq!(allocation["node"]["kind"], "known");
            assert_eq!(
                allocation["pool"],
                json!({"kind":"known","value":binding.pool})
            );
            assert_eq!(
                allocation["scope"],
                json!({"kind":"known","value":{"kind":"shared"}})
            );
            assert_eq!(allocation["access"]["kind"], "ordinary");
            assert!(
                values
                    .insert(binding.source_id.clone(), binding.value)
                    .is_none(),
                "each source node occurs once in selected preset"
            );
        }
    }
    values
}
#[test]
#[ignore = "requires checked Pain Offering release and authenticated original source evidence"]
fn passive_release_preserves_all_five_original_requests() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PLAIN_MINION_DAMAGE_PRIOR")
            .expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_PLAIN_MINION_DAMAGE_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists(), "do not overwrite previous evidence");
    let prior_inventory = release::inventory(&prior_path);
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
    let mut original_five = None;
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
        check_sidecar_digest(&old, &sa);
        check_sidecar_digest(&new, &sb);
        for v in [&mut a, &mut b, &mut sa, &mut sb] {
            selected::canonical(v);
        }
        assert!(
            a == b,
            "all raw fields, unresolved targets, allocations, closures, local IDs and allocator watermark preserved"
        );
        // A new normalization run has its own lineage. No dependency identity
        // changes for this rules-only extension; the verified draft hash is the
        // only non-lineage sidecar field that is expected to differ.
        sb["draft"] = sa["draft"].clone();
        assert!(
            sa == sb,
            "all provenance and normalization dependency bindings preserved"
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
            "numeric producers alone do not resolve import obligations"
        );
        assert_eq!(y.as_array().unwrap().len(), expected_issues);
        let before_selection = selected::selection(&xml, &old);
        let after_selection = selected::selection(&xml, &new);
        if case == 5 {
            let expected = family::selected_values();
            assert_eq!(selected_passives(&old, &before_selection), expected);
            assert_eq!(selected_passives(&new, &after_selection), expected);
            assert_eq!(expected.len(), 10);
            assert_eq!(expected.values().sum::<f64>(), 68.0);
            original_five = Some(expected);
        }
        let mut x = before_selection;
        let mut y = after_selection;
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
    assert_eq!(prior_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,
            "originals":reports,"queries":110,"prior_unchanged":true,
            "rebuild_byte_identical":true,"complete_original_builds":0,
            "operations":"owned-domain-operations-v15","application_coverage":"partial",
            "appended_partial_passive_programs":41,"allocated_definitions":0,
            "original05_source_selected_passives":original_five.unwrap(),
            "original05_source_selected_passive_total":68,
            "scope":"static-passive-producers-only-not-full-build-or-action-damage"
        }),
    );
}
