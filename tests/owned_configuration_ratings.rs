//! Real configuration input publication; no whole-build evaluation is admitted yet.
#[path = "support/owned_configuration_ratings.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_import::owned_normalize::ConfigurationInputsPolicy;
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
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap()
}
fn publish(input: &Path, out: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(out)
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
fn raw_override_profile_and_native_consumers_share_exact_input_identities() {
    let ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
        encounter, inputs, ..
    } = family::policy();
    let native: Value = family::read("native-inputs.json");
    assert_eq!(native["encounter"], json!(encounter));
    assert_eq!(inputs.len(), 6);
    for (raw, actual) in inputs
        .iter()
        .skip(4)
        .zip(native["inputs"].as_array().unwrap())
    {
        assert_eq!(json!(raw.source_name), actual["source_name"]);
        assert_eq!(json!(raw.presence_input), actual["presence_input"]);
        assert_eq!(json!(raw.value_input), actual["value_input"]);
        assert_eq!(json!(raw.recipe)["missing"]["kind"], "pending");
    }
    let manifest = read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    let authoring = family::authoring();
    assert_eq!(authoring["source_revision"], manifest["upstream_revision"]);
    for pin in authoring["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
    assert_eq!(authoring["expected_query_rows"], 110);
}

fn compare_original(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a = read(old.join("draft.json"));
    let mut b = read(new.join("draft.json"));
    let mut sa = read(old.join("sidecar.json"));
    let mut sb = read(new.join("sidecar.json"));
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    let old_sets = a["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let new_sets = b["draft"]["scenario_presets"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(old_sets.len(), 1);
    assert_eq!(new_sets.len(), 1);
    let old_inputs = &mut old_sets[0]["scenario"]["assumptions"];
    let new_inputs = &new_sets[0]["scenario"]["assumptions"];
    assert_eq!(old_inputs["members"].as_array().unwrap().len(), 4);
    assert_eq!(old_inputs["completion"], new_inputs["completion"]);
    assert_eq!(new_inputs["completion"]["kind"], "pending");
    let ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 { inputs, .. } =
        family::policy();
    let expected: Vec<_> = inputs
        .iter()
        .map(|input| {
            json!({
                "input":{"kind":"known","value":input.presence_input},
                "target":{"kind":"enemy"},
                "value":{"kind":"known","value":{"kind":"boolean","value":false}}
            })
        })
        .collect();
    assert_eq!(old_inputs["members"], json!(&expected[..4]));
    assert_eq!(
        new_inputs["members"],
        json!(expected),
        "actual raw absence, no manufactured raw values"
    );
    old_inputs["members"] = new_inputs["members"].clone();
    assert!(
        a == b,
        "original {case}: unrelated draft facts, IDs or allocator changed"
    );
    let new_item_policy = sb["item_policy"].clone();
    let new_source_policy = sb["item_source_policy"].clone();
    let old_items = sa["item_texts"].as_array().unwrap();
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
        "original {case}: source evidence changed outside checked dependency commitments"
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
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    assert_eq!(
        old_issues, new_issues,
        "component inputs cannot retire a whole inventory obligation"
    );
    let count = new_issues.as_array().unwrap().len();
    assert_eq!(
        json!(count),
        family::authoring()["prior_selected_issues"][case - 1]
    );
    let mut old_selection = selected::selection(xml, old);
    let mut new_selection = selected::selection(xml, new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"selected_issues":count,"known_raw_presence_inputs":6,
        "known_raw_quantity_inputs":0,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

#[test]
#[ignore = "requires exact configuration-resistance predecessor and passed rating source witness"]
fn real_configuration_ratings_inputs_preserve_all_original_requests() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_RATING_PRIOR").expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let prior_files = release::inventory(&prior_path);
    let next = family::stage(&prior);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CONFIGURATION_RATING_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let endpoint = out.join("endpoint.json");
    write(&endpoint, next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, output) in [(&endpoint, &package), (&package, &rebuilt)] {
        assert_eq!(publish(input, output), json!(next.receipt()));
    }
    let after_files = release::inventory(&package);
    assert_eq!(after_files, release::inventory(&rebuilt));
    assert_eq!(prior_files.len(), after_files.len());
    for row in family::authoring()["query_files"].as_array().unwrap() {
        let file = row["file"].as_str().unwrap();
        let bytes = fs::read(package.join(file)).unwrap();
        assert_eq!(bytes, fs::read(prior_path.join(file)).unwrap());
        assert_eq!(
            json!(format!("{:x}", Sha256::digest(&bytes))),
            row["sha256"]
        );
    }
    let mut reports = vec![];
    for case in 1..=5 {
        let xml = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let old = out.join(format!("prior-original-{case:02}"));
        let new = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &old);
        release::normalize(&package, &xml, case, &new);
        reports.push(compare_original(
            case,
            &fs::read(xml).unwrap(),
            &old,
            &new,
            &out,
        ));
    }
    assert_eq!(next.receipt().query_rows, 110);
    // Exercise the published profile on actual source syntax as well as the
    // unchanged originals. A wrong lane cannot become an absent override.
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let native: Value = family::read("native-inputs.json");
    let cold = &native["inputs"][0];
    for (label, attribute, expected_count) in
        [("explicit-zero", "number", 2), ("wrong-lane", "string", 0)]
    {
        let path = out.join(format!("probe-{label}.xml"));
        assert_eq!(original.matches("</ConfigSet>").count(), 1);
        fs::write(
            &path,
            original.replacen(
                "</ConfigSet>",
                &format!("<Input name=\"enemyArmour\" {attribute}=\"0\"/></ConfigSet>"),
                1,
            ),
        )
        .unwrap();
        let normalized = out.join(format!("probe-{label}"));
        release::normalize(&package, &path, 5, &normalized);
        let draft = read(normalized.join("draft.json"));
        let list = &draft["draft"]["scenario_presets"]["members"][0]["scenario"]["assumptions"];
        assert_eq!(list["completion"]["kind"], "pending");
        let cold_rows: Vec<_> = list["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| {
                row["input"]["value"] == cold["presence_input"]
                    || row["input"]["value"] == cold["value_input"]
            })
            .collect();
        assert_eq!(cold_rows.len(), expected_count);
        if expected_count == 2 {
            assert_eq!(cold_rows[0]["input"]["value"], cold["presence_input"]);
            assert_eq!(
                cold_rows[0]["value"]["value"],
                json!({"kind":"boolean","value":true})
            );
            assert_eq!(cold_rows[1]["input"]["value"], cold["value_input"]);
            assert_eq!(
                cold_rows[1]["value"]["value"],
                json!({"kind":"quantity","value":{"value":0.0,"unit":native["unit"]}})
            );
        }
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,
        "after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,
        "queries":next.receipt().query_rows,"provenance":next.input().provenance.len(),"originals":reports,
        "source_probes":["explicit-zero","wrong-lane"],
        "predecessor_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
