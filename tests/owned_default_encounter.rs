//! Real selected requests gain one data-defined encounter without invented numerics.
#[path = "support/owned_default_encounter.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_import::owned_normalize::EncounterPolicy;
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
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap()
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
fn authored_encounter_is_an_explicit_identity_with_partial_numerical_coverage() {
    let EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        selector,
        target,
        absent_input_names,
        ..
    } = family::policy();
    assert_eq!(
        absent_input_names,
        ["enemyIsBoss", "presetBossSkills", "enemySizePreset"]
    );
    assert_eq!(selector, family::mapping().source);
    let e = serde_json::to_value(family::extension()).unwrap();
    assert_eq!(e["schema"].as_array().unwrap().len(), 1);
    let definition = &e["schema"][0]["value"]["value"];
    assert_eq!(definition["id"], serde_json::to_value(target).unwrap());
    assert_eq!(definition["schema"]["kind"], "known");
    let schema = &definition["schema"]["value"];
    assert_eq!(schema["enemy_level"], json!({"minimum":1,"maximum":85}));
    assert_eq!(schema["external_inputs"]["closure"]["kind"], "partial");
    assert!(
        schema["external_inputs"]["members"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(e["owners"].as_array().unwrap().len(), 1);
    assert_eq!(e["owners"][0]["programs"]["closure"]["kind"], "partial");
    assert!(
        e["owners"][0]["programs"]["members"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(e["tables"].as_array().unwrap().is_empty());
    assert!(e["receivers"].as_array().unwrap().is_empty());
    let authored = family::authoring();
    let manifest: Value =
        read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    assert_eq!(authored["source_revision"], manifest["upstream_revision"]);
    for pin in authored["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(pin));
    }
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(a["draft"]["allocator"], b["draft"]["allocator"]);
    let old_presets = a["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let new_presets = b["draft"]["scenario_presets"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(old_presets.len(), 1);
    assert_eq!(new_presets.len(), 1);
    let pending = &old_presets[0]["scenario"]["enemy"]["encounter"];
    assert_eq!(pending["kind"], "pending");
    assert_eq!(pending["code"], "encounter-not-converted");
    let retired = pending["id"].clone();
    let EncounterPolicy::PobFreshDefaultConfigEncounterV1 { target, .. } = family::policy();
    let known = json!({"kind":"known","value":target});
    assert_eq!(new_presets[0]["scenario"]["enemy"]["encounter"], known);
    old_presets[0]["scenario"]["enemy"]["encounter"] = known;
    assert!(
        a == b,
        "original {case}: unrelated draft facts or IDs changed"
    );
    let mut retired_links = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let retired_link = link["kind"] == "issue" && link["value"] == retired;
            retired_links += usize::from(retired_link);
            !retired_link
        });
    }
    assert_eq!(retired_links, 1);
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], 15);
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    assert_eq!(sa["mapping_source"], sb["mapping_source"]);
    let old_items = sa["item_texts"].as_array().unwrap();
    let new_item_policy = sb["item_policy"].clone();
    let new_source_policy = sb["item_source_policy"].clone();
    let new_items = sb["item_texts"].as_array_mut().unwrap();
    assert_eq!(old_items.len(), new_items.len());
    for (old, new) in old_items.iter().zip(new_items) {
        // Item attribution embeds the same checked dependency commitments as
        // the outer sidecar. Preserve every other byte of the attribution.
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
        "original {case}: source evidence changed outside checked dependency commitments and the one retired issue"
    );
    // In particular, the previous checkpoint's exact reward inventory survives
    // schema/catalog rebinding and every original reward occurrence is untouched.
    assert_eq!(
        b["draft"]["choice_presets"]["members"][0]["rewards"]["completion"]["kind"],
        "complete"
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
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    let count = old_issues.as_array().unwrap().len();
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|issue| issue["id"] != retired);
    assert_eq!(old_issues, new_issues);
    assert_eq!(count, [125, 126, 118, 155, 22][case - 1]);
    assert_eq!(new_issues.as_array().unwrap().len(), count - 1);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    assert_eq!(first, second);
    json!({"original":case,"before":count,"after":count-1,"retired":retired,
        "selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn probes(package: &Path, out: &Path) -> usize {
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let saved = r#"<Placeholder number="82" name="enemyLevel"/>"#;
    assert_eq!(xml.matches(saved).count(), 1);
    let mut cases = Vec::new();
    for (name, row) in [
        (
            "explicit-pinnacle",
            r#"<Input name="enemyIsBoss" string="Pinnacle"/>"#,
        ),
        (
            "different-boss",
            r#"<Input name="enemyIsBoss" string="Boss"/>"#,
        ),
        (
            "boss-placeholder",
            r#"<Placeholder name="enemyIsBoss" string="Boss"/>"#,
        ),
        (
            "boss-wrong-lane",
            r#"<Input name="enemyIsBoss" boolean="true"/>"#,
        ),
        (
            "named-preset",
            r#"<Input name="presetBossSkills" string="Shaper Ball"/>"#,
        ),
        (
            "preset-placeholder",
            r#"<Placeholder name="presetBossSkills" string="None"/>"#,
        ),
        (
            "changed-size",
            r#"<Input name="enemySizePreset" string="Large"/>"#,
        ),
        (
            "size-placeholder",
            r#"<Placeholder name="enemySizePreset" string="Medium"/>"#,
        ),
        (
            "namespace-input",
            r#"<Input xmlns="urn:other" name="enemyIsBoss" string="Boss"/>"#,
        ),
        (
            "nested-input",
            r#"<Input name="enemyIsBoss" string="Boss"><Child/></Input>"#,
        ),
    ] {
        cases.push((
            name,
            xml.replacen(saved, &format!("{saved}{row}"), 1),
            false,
        ));
    }
    for (name, replacement) in [
        ("missing-level-placeholder", ""),
        (
            "changed-level-placeholder",
            r#"<Placeholder number="7" name="enemyLevel"/>"#,
        ),
        (
            "explicit-level-twenty",
            r#"<Input number="20" name="enemyLevel"/>"#,
        ),
        (
            "unrelated-input",
            r#"<Input string="opaque" name="unreviewedSetting"/>"#,
        ),
    ] {
        cases.push((name, xml.replacen(saved, replacement, 1), true));
    }
    cases.push((
        "duplicate-config-id",
        xml.replacen("</Config>", "<ConfigSet id=\"1\"/></Config>", 1),
        false,
    ));
    let count = cases.len();
    let EncounterPolicy::PobFreshDefaultConfigEncounterV1 { target, .. } = family::policy();
    for (name, changed, known) in cases {
        let path = out.join(format!("probe-{name}.xml"));
        let dir = out.join(format!("probe-{name}"));
        fs::write(&path, changed).unwrap();
        release::normalize(package, &path, 5, &dir);
        let draft: Value = read(dir.join("draft.json"));
        let scenarios = draft["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap();
        assert!(!scenarios.is_empty());
        for preset in scenarios {
            let scenario = &preset["scenario"];
            if known {
                assert_eq!(
                    scenario["enemy"]["encounter"],
                    json!({"kind":"known","value":target}),
                    "{name}"
                );
                // Encounter identity does not stand in for the independent level recipe.
                assert_eq!(scenario["enemy"]["level"]["kind"], "pending", "{name}");
            } else {
                assert_eq!(scenario["enemy"]["encounter"]["kind"], "pending", "{name}");
            }
            assert_eq!(
                scenario["assumptions"]["completion"]["kind"], "pending",
                "{name}"
            );
            assert_eq!(scenario["usage"]["completion"]["kind"], "pending", "{name}");
        }
    }
    count
}

#[test]
#[ignore = "requires checked reward-inventory predecessor and passed source encounter witness"]
fn real_encounter_publication_preserves_other_build_facts_and_reward_inventory() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENCOUNTER_PRIOR").expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let before = release::inventory(&prior_path);
    let next = family::stage(&prior);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENCOUNTER_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let input = out.join("endpoint.json");
    write(&input, next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, output) in [(&input, &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(input, output),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    assert_eq!(release::inventory(&package), release::inventory(&rebuilt));
    for (name, bytes) in next.artifacts() {
        if name.starts_with("queries-") {
            assert_eq!(bytes, fs::read(prior_path.join(name)).unwrap());
        }
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
        reports.push(compare(case, &fs::read(xml).unwrap(), &old, &new, &out));
    }
    let probes = probes(&package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({
            "before":prior.receipt().input,"after":next.receipt().input,
            "definitions":next.receipt().definitions,"registry":next.receipt().registry,
            "queries":next.receipt().query_rows,"provenance":next.input().provenance.len(),
            "originals":reports,"probes":probes,"predecessor_unchanged":true,
            "rebuild_byte_identical":true,"complete_original_builds":0
        }),
    );
}
