//! Five unchanged selected requests gain only a source-proved enemy level.
#[path = "support/owned_enemy_level.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_import::{
    owned_normalize::EnemyLevelPolicy,
    owned_value_policy::{DuplicatePolicy, MissingValuePolicy, ValueLane},
};
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
fn authored_level_profile_is_explicit_and_has_no_missing_value_default() {
    let EnemyLevelPolicy::PobFreshDefaultConfigLevelV1 {
        absent_input_names,
        placeholder,
        expected_level,
        ..
    } = family::policy();
    assert_eq!(expected_level, 82);
    assert_eq!(absent_input_names, vec!["enemyLevel", "enemyIsBoss"]);
    assert_eq!(placeholder.tiers.len(), 1);
    assert_eq!(placeholder.tiers[0].selectors.len(), 1);
    assert_eq!(
        placeholder.tiers[0].selectors[0].lane,
        ValueLane::PlaceholderNumber
    );
    assert_eq!(placeholder.tiers[0].duplicates, DuplicatePolicy::Reject);
    assert_eq!(placeholder.missing, MissingValuePolicy::Pending);
    assert!(placeholder.numeric_aliases.is_empty());
    let authored = family::authoring();
    let manifest: Value =
        read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    assert_eq!(authored["source_revision"], manifest["upstream_revision"]);
    for pin in authored["source_files"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
        );
    }
}
fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v)
    }
    let before_allocator = a["draft"]["allocator"].clone();
    let after_allocator = b["draft"]["allocator"].clone();
    let issued = |v: &Value| u64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(issued(&before_allocator), issued(&after_allocator) + 1);
    b["draft"]["allocator"] = before_allocator.clone();
    let old_presets = a["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let new_presets = b["draft"]["scenario_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(old_presets.len(), 1);
    assert_eq!(new_presets.len(), 1);
    let pending = &old_presets[0]["scenario"]["enemy"]["level"];
    assert_eq!(pending["kind"], "pending");
    assert_eq!(pending["code"], "enemy-level-not-converted");
    let retired = pending["id"].clone();
    let new_scenario = new_presets[0]["id"].clone();
    assert_eq!(
        new_presets[0]["scenario"]["enemy"]["level"],
        json!({"kind":"known","value":82})
    );
    for preset in [&mut old_presets[0], &mut new_presets[0]] {
        preset["scenario"]["enemy"]
            .as_object_mut()
            .unwrap()
            .remove("level");
    }
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "one independently known enemy level; every other canonical fact preserved",
    );
    let mut removed = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let drop = link["kind"] == "issue" && link["value"] == retired;
            removed += usize::from(drop);
            !drop
        });
    }
    assert_eq!(removed, 1);
    let ordinal = [411, 458, 211, 390, 438][case - 1];
    let row = sb["origins"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["source"]["ordinal"] == ordinal)
        .unwrap();
    let links = row["links"].as_array_mut().unwrap();
    let pos = links
        .iter()
        .position(|l| l["kind"] == "scenario_preset" && l["value"] == new_scenario)
        .expect("exact placeholder supplies exact scenario");
    links.remove(pos);
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], 15);
    assert_eq!(sb["allocator_after"], after_allocator);
    for field in ["allocator_after", "policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "all other source evidence and origin links",
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
        .retain(|i| i["id"] != retired);
    identity::relocate(&mut new_issues, &ids);
    assert_eq!(old_issues, new_issues);
    assert_eq!(count, [127, 128, 120, 157, 24][case - 1]);
    assert_eq!(new_issues.as_array().unwrap().len(), count - 1);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    identity::relocate(&mut second, &ids);
    assert_eq!(first, second);
    json!({"original":case,"before":count,"after":count-1,"retired":retired,"placeholder_ordinal":ordinal,"level":82,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
fn probes(package: &Path, out: &Path) -> usize {
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let saved = "<Placeholder number=\"82\" name=\"enemyLevel\"/>";
    assert_eq!(xml.matches(saved).count(), 1);
    let mut count = 0;
    for (name, replacement) in [
        (
            "different-placeholder",
            "<Placeholder number=\"81\" name=\"enemyLevel\"/>",
        ),
        ("missing-placeholder", ""),
        (
            "zero-placeholder",
            "<Placeholder number=\"0\" name=\"enemyLevel\"/>",
        ),
        (
            "fractional-spelling",
            "<Placeholder number=\"82.0\" name=\"enemyLevel\"/>",
        ),
        (
            "malformed-number",
            "<Placeholder number=\"oops\" name=\"enemyLevel\"/>",
        ),
        (
            "explicit-level",
            "<Input number=\"82\" name=\"enemyLevel\"/><Placeholder number=\"82\" name=\"enemyLevel\"/>",
        ),
        (
            "boss-override",
            "<Input string=\"Pinnacle\" name=\"enemyIsBoss\"/><Placeholder number=\"82\" name=\"enemyLevel\"/>",
        ),
        (
            "string-placeholder",
            "<Placeholder string=\"82\" name=\"enemyLevel\"/>",
        ),
        (
            "boolean-placeholder",
            "<Placeholder boolean=\"true\" name=\"enemyLevel\"/>",
        ),
        (
            "conflicting-encoding",
            "<Placeholder number=\"82\" string=\"20\" name=\"enemyLevel\"/>",
        ),
        (
            "duplicate-placeholder",
            "<Placeholder number=\"82\" name=\"enemyLevel\"/><Placeholder number=\"82\" name=\"enemyLevel\"/>",
        ),
        (
            "namespace-placeholder",
            "<Placeholder xmlns=\"urn:other\" number=\"82\" name=\"enemyLevel\"/>",
        ),
    ] {
        let changed = xml.replacen(saved, replacement, 1);
        let path = out.join(format!("probe-{name}.xml"));
        let dir = out.join(format!("probe-{name}"));
        fs::write(&path, changed).unwrap();
        release::normalize(package, &path, 5, &dir);
        let draft: Value = read(dir.join("draft.json"));
        let scenarios = draft["draft"]["scenario_presets"]["members"]
            .as_array()
            .unwrap();
        assert!(!scenarios.is_empty());
        assert!(
            scenarios
                .iter()
                .all(|s| s["scenario"]["enemy"]["level"]["kind"] == "pending"),
            "{name}"
        );
        count += 1;
    }
    count
}
#[test]
#[ignore = "requires the checked Ashen Staff predecessor and passed complete-source witness"]
fn real_enemy_level_publication_preserves_every_other_original_fact() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_LEVEL_PRIOR").expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let before = release::inventory(&prior_path);
    let next = family::stage(&prior);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_ENEMY_LEVEL_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    fs::create_dir_all(&out).unwrap();
    let input = out.join("endpoint.json");
    write(&input, next.input());
    let package = out.join("package");
    let rebuilt = out.join("rebuilt");
    for (input, out) in [(&input, &package), (&package, &rebuilt)] {
        assert_eq!(
            publish(input, out),
            serde_json::to_value(next.receipt()).unwrap()
        );
    }
    let after = release::inventory(&package);
    assert_eq!(after, release::inventory(&rebuilt));
    for (file, hash) in &before {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&file.as_str())
        {
            assert_eq!(after.get(file), Some(hash), "unchanged artifact: {file}");
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
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":next.receipt().query_rows,"provenance":next.input().provenance.len(),"originals":reports,"probes":probes,"predecessor_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
