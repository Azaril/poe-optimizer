//! Real physical inventory closure preserves every remaining input/query obligation.
#[path = "support/owned_active_gem_inventory.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_import::owned_release::assemble_owned_release;
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

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
fn active_inventory_authoring_binds_single_physical_effect_and_retains_usage_obligations() {
    family::check_authored();
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    selected::canonical(&mut a);
    selected::canonical(&mut b);
    selected::canonical(&mut sa);
    selected::canonical(&mut sb);
    let pa = a["draft"]["allocator"].clone();
    let na = b["draft"]["allocator"].clone();
    let domain: Vec<_> = family::inventory()
        .into_iter()
        .map(|row| json!(row.physical.gem))
        .collect();
    let aa = a["draft"]["gems"]["members"].as_array_mut().unwrap();
    let bb = b["draft"]["gems"]["members"].as_array_mut().unwrap();
    assert_eq!(aa.len(), bb.len());
    let mut retired = BTreeSet::new();
    for (x, y) in aa.iter_mut().zip(bb) {
        if x["definition"]["kind"] == "known" && domain.contains(&x["definition"]["value"]) {
            for field in ["definition", "level", "quality"] {
                assert_eq!(x[field], y[field]);
            }
            assert_eq!(x["parameters"]["members"], y["parameters"]["members"]);
            assert_eq!(x["parameters"]["members"].as_array().unwrap().len(), 2);
            assert_eq!(
                x["parameters"]["completion"]["code"],
                "gem-parameters-not-converted"
            );
            assert_eq!(y["parameters"]["completion"], json!({"kind":"complete"}));
            assert!(
                retired.insert(
                    x["parameters"]["completion"]["id"]["local"]
                        .as_str()
                        .unwrap()
                        .to_owned()
                )
            );
            for row in [x, y] {
                row["parameters"]
                    .as_object_mut()
                    .unwrap()
                    .remove("completion");
            }
        }
    }
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&pa) - issued(&na),
        retired.len() as u64,
        "no phantom retired issue IDs"
    );
    b["draft"]["allocator"] = pa.clone();
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "only physical list completion changes",
    );
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue"
                && link["value"]["local"]
                    .as_str()
                    .is_some_and(|id| retired.contains(id));
            removed += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed, retired.len());
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], pa);
    assert_eq!(sb["allocator_after"], na);
    sb["allocator_after"] = sa["allocator_after"].clone();
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "source observations and retained attribution survive",
    );
    let before = selected::finalize(
        xml,
        old,
        &out.join(format!("original-{case:02}-prior-selection.json")),
    );
    let after = selected::finalize(
        xml,
        new,
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
    let mut x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    selected::canonical(&mut x);
    selected::canonical(&mut y);
    identity::relocate(&mut y, &ids);
    let count = x.as_array().unwrap().len();
    x.as_array_mut()
        .unwrap()
        .retain(|v| !retired.contains(v["id"]["local"].as_str().unwrap()));
    assert_eq!(x, y, "every other selected obligation survives");
    assert_eq!(count, [116, 116, 108, 121, 19][case - 1]);
    let mut old_selection = selected::selection(xml, old);
    let mut new_selection = selected::selection(xml, new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    identity::relocate(&mut new_selection, &ids);
    assert_eq!(
        old_selection, new_selection,
        "original MAIN/CALCS query selection"
    );
    if case == 5 {
        assert_eq!(
            count - y.as_array().unwrap().len(),
            1,
            "Offering physical inventory only"
        );
        let presets = b["draft"]["skill_presets"]["members"].as_array().unwrap();
        assert!(
            presets
                .iter()
                .any(|p| p["usage_preferences"]["completion"]["kind"] == "pending")
        );
    }
    json!({"original":case,"physical_lists_completed":retired.len(),"selected_before":count,"selected_after":y.as_array().unwrap().len(),"selected_issue_summary":after["selected_issue_summary"]})
}

#[test]
#[ignore = "requires the exact checked minion-physical predecessor and source witness"]
fn active_physical_inventory_preserves_five_originals_and_pending_usage() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ACTIVE_INVENTORY_PRIOR").expect("explicit prior"),
    );
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    let temp = tempfile::tempdir().unwrap();
    let out = std::env::var_os("POE_OPTIMIZER_TEST_ACTIVE_INVENTORY_OUTPUT")
        .map(PathBuf::from)
        .unwrap_or_else(|| temp.path().join("publication"));
    assert!(!out.exists(), "new output directory only");
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, output) in [(&out.join("endpoint.json"), &package), (&package, &rebuilt)] {
        assert_eq!(publish(input, output), json!(next.receipt()));
    }
    let after = release::inventory(&package);
    assert_eq!(after, release::inventory(&rebuilt));
    for (name, hash) in &before {
        if ![
            "release.json",
            "normalization.json",
            "tree-normalization.json",
        ]
        .contains(&name.as_str())
        {
            assert_eq!(after.get(name), Some(hash), "unchanged artifact {name}");
        }
    }
    for field in ["roles", "catalog", "scalar_inputs", "usage_inputs"] {
        let mut bad = json!(next.input());
        bad["normalization"]["gem_inventory"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(bad).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"queries":110,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0,"stale_binding_rejections":4}),
    );
}
