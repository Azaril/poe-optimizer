//! Empty character reward selections are independent of Config rewards and budgets.
#[path = "support/owned_character_reward_inventory.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::CharacterRewardInventoryPolicy,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
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
fn authored_character_only_profile_has_exact_source_and_no_reward_definitions() {
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 {
        mapping_source,
        target_version,
        tree_version,
    } = family::policy();
    assert_eq!(target_version, "0_1");
    assert_eq!(tree_version, "0_5");
    let encounter: Value =
        read(root().join("data/owned/poe2/3887ae68/default-encounter/policy.json"));
    assert_eq!(
        serde_json::to_value(mapping_source).unwrap(),
        encounter["mapping_source"]
    );
    let authored = family::authoring();
    let manifest: Value =
        read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"));
    assert_eq!(authored["source_revision"], manifest["upstream_revision"]);
    assert_eq!(authored["tree_version"], tree_version);
    assert_eq!(authored["source_target_version"], target_version);
    let pins = authored["source_files"].as_array().unwrap();
    assert!(!pins.is_empty());
    let mut seen = BTreeSet::new();
    for pin in pins {
        assert!(seen.insert(pin["path"].as_str().unwrap()));
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p == pin)
        );
    }
    for needed in [
        "src/Modules/Build.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Classes/TreeTab.lua",
        "src/GameVersions.lua",
    ] {
        assert!(seen.contains(needed));
    }
    assert_eq!(authored["expected_query_rows"], 110);
    assert_eq!(authored["query_files"].as_array().unwrap().len(), 5);
    let encoded = serde_json::to_value(family::policy()).unwrap();
    assert_eq!(encoded.as_object().unwrap().len(), 4);
    assert_eq!(encoded["kind"], "pob_fresh_character_only_empty_v1");
}

struct Changes {
    retired: Vec<Value>,
    retired_links: usize,
    character_presets: usize,
    reward_members: usize,
}

// Compare normalized copies of the same source. The only allowed semantic change
// is each witnessed empty character-list completion and its exact issue links.
fn compare_inputs(old: &Path, new: &Path, complete: bool) -> Changes {
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(a["draft"]["allocator"], b["draft"]["allocator"]);
    let old_presets = a["draft"]["character_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let new_presets = b["draft"]["character_presets"]["members"]
        .as_array()
        .unwrap();
    assert!(!old_presets.is_empty());
    assert_eq!(old_presets.len(), new_presets.len());
    let character_presets = old_presets.len();
    let mut retired = vec![];
    for (prior, next) in old_presets.iter_mut().zip(new_presets) {
        assert_eq!(prior["id"], next["id"]);
        assert_eq!(prior["rewards"]["members"], json!([]));
        let completion = &prior["rewards"]["completion"];
        assert_eq!(completion["kind"], "pending");
        assert_eq!(completion["code"], "character-rewards-not-converted");
        if complete {
            assert_eq!(next["rewards"]["completion"], json!({"kind":"complete"}));
            let id = completion["id"].clone();
            assert!(!retired.contains(&id));
            retired.push(id.clone());
            // Each retired issue has one exact Spec owner link. Historical
            // fallback fan-out may also link unrelated rows to that same issue.
            let owners: Vec<_> = sa["origins"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| {
                    let links = row["links"].as_array().unwrap();
                    links.contains(&json!({"kind":"character_preset","value":prior["id"]}))
                })
                .collect();
            assert_eq!(owners.len(), 1);
            assert!(
                owners[0]["links"]
                    .as_array()
                    .unwrap()
                    .contains(&json!({"kind":"issue","value":id}))
            );
            prior["rewards"]["completion"] = json!({"kind":"complete"});
        } else {
            assert_eq!(next["rewards"]["completion"], *completion);
        }
    }
    let reward_members = a["draft"]["rewards"]["members"].as_array().unwrap().len();
    assert!(
        a == b,
        "unrelated draft facts, IDs, ordering or allocator changed"
    );
    let mut retired_links = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&link["value"]);
            retired_links += usize::from(remove);
            !remove
        });
    }
    assert!(retired_links >= retired.len());
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], 15);
    for field in ["policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    assert!(
        sa == sb,
        "source evidence changed beyond exact retired issue links"
    );
    Changes {
        retired,
        retired_links,
        character_presets,
        reward_members,
    }
}

fn compare(case: usize, xml: &[u8], old: &Path, new: &Path, out: &Path) -> Value {
    let changes = compare_inputs(old, new, true);
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([83; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let tree = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Tree")
        .unwrap();
    let specs: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|r| {
            r.occurrence().name() == "Spec"
                && r.occurrence().parent() == Some(tree.occurrence().id())
        })
        .collect();
    assert_eq!(changes.character_presets, specs.len());
    assert_eq!(changes.retired.len(), specs.len());
    let sidecar: Value = read(new.join("sidecar.json"));
    let mut linked = BTreeSet::new();
    for spec in &specs {
        let row = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["source"] == serde_json::to_value(spec.occurrence().id()).unwrap())
            .unwrap();
        let targets: Vec<_> = row["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|l| l["kind"] == "character_preset")
            .collect();
        assert_eq!(targets.len(), 1);
        assert!(linked.insert(targets[0]["value"].to_string()));
    }
    let draft: Value = read(new.join("draft.json"));
    let choices = draft["draft"]["choice_presets"]["members"]
        .as_array()
        .unwrap();
    assert_eq!(choices.len(), 1);
    let config_members = choices[0]["rewards"]["members"].as_array().unwrap().len();
    assert_eq!(config_members, [16, 17, 15, 16, 17][case - 1]);
    assert_eq!(changes.reward_members, config_members);
    assert_eq!(
        choices[0]["rewards"]["completion"],
        json!({"kind":"complete"})
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
    let prior_count = old_issues.as_array().unwrap().len();
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|i| !changes.retired.contains(&i["id"]));
    assert_eq!(old_issues, new_issues);
    let next_count = new_issues.as_array().unwrap().len();
    assert_eq!(prior_count - next_count, 1);
    assert_eq!(
        json!(prior_count),
        family::authoring()["prior_selected_issues"][case - 1]
    );
    let mut prior_selection = selected::selection(xml, old);
    let mut next_selection = selected::selection(xml, new);
    selected::canonical(&mut prior_selection);
    selected::canonical(&mut next_selection);
    assert_eq!(prior_selection, next_selection);
    json!({"original":case,"before":prior_count,"after":next_count,
        "character_presets_completed":changes.character_presets,"retired":changes.retired,
        "retired_origin_links":changes.retired_links,"config_rewards_preserved":config_members,
        "selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn replace_once(text: &str, old: &str, new: &str) -> String {
    assert_eq!(text.matches(old).count(), 1, "source anchor: {old}");
    text.replacen(old, new, 1)
}
fn probes(prior: &Path, package: &Path, out: &Path) -> usize {
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let first_stat = r#"<PlayerStat stat="ActiveMinionLimit" value="0"/>"#;
    assert_eq!(xml.matches(first_stat).count(), 2);
    let cases = vec![
        (
            "cached-stat",
            xml.replacen(
                first_stat,
                r#"<PlayerStat stat="ActiveMinionLimit" value="42"/>"#,
                1,
            ),
            true,
        ),
        (
            "config-unrelated",
            replace_once(
                &xml,
                "</ConfigSet>",
                "<Input name=\"unreviewed-character-test\" string=\"unknown\"/></ConfigSet>",
            ),
            true,
        ),
        (
            "config-none",
            replace_once(
                &xml,
                r#"<Input string="+5% to Lightning Resistance" name="questAct 4Halls Of The DeadTawhoa&apos;s Test"/>"#,
                r#"<Input string="None" name="questAct 4Halls Of The DeadTawhoa&apos;s Test"/>"#,
            ),
            true,
        ),
        (
            "unknown-build-child",
            replace_once(&xml, "</Build>", "<Reward id=\"unreviewed\"/></Build>"),
            false,
        ),
        (
            "unknown-spec-child",
            xml.replacen("</Spec>", "<Reward id=\"unreviewed\"/></Spec>", 1),
            false,
        ),
        (
            "auto-level",
            replace_once(
                &xml,
                "characterLevelAutoMode=\"false\"",
                "characterLevelAutoMode=\"true\"",
            ),
            false,
        ),
        (
            "missing-auto-level",
            replace_once(&xml, " characterLevelAutoMode=\"false\"", ""),
            false,
        ),
        (
            "namespace-build",
            xml.replacen("<Build ", "<Build xmlns=\"urn:unreviewed\" ", 1),
            false,
        ),
        (
            "duplicate-build",
            replace_once(&xml, "</Build>", "</Build><Build/>"),
            false,
        ),
        (
            "duplicate-tree",
            replace_once(&xml, "</Tree>", "</Tree><Tree activeSpec=\"1\"/>"),
            false,
        ),
        (
            "legacy-root-spec",
            replace_once(&xml, "</PathOfBuilding2>", "<Spec/></PathOfBuilding2>"),
            false,
        ),
        (
            "unknown-tree-version",
            xml.replacen("treeVersion=\"0_5\"", "treeVersion=\"unreviewed\"", 1),
            false,
        ),
        (
            "unknown-target-version",
            replace_once(
                &xml,
                "targetVersion=\"0_1\"",
                "targetVersion=\"unreviewed\"",
            ),
            false,
        ),
        (
            "invalid-active-spec",
            replace_once(&xml, "<Tree activeSpec=\"3\">", "<Tree activeSpec=\"0\">"),
            false,
        ),
        (
            "mixed-build-text",
            replace_once(&xml, "</Build>", "unreviewed</Build>"),
            false,
        ),
        (
            "nested-cached-stat",
            xml.replacen(
                first_stat,
                r#"<PlayerStat stat="ActiveMinionLimit" value="0"><Reward/></PlayerStat>"#,
                1,
            ),
            false,
        ),
    ];
    let count = cases.len();
    for (name, changed, complete) in cases {
        assert_ne!(changed, xml, "{name}");
        let path = out.join(format!("probe-{name}.xml"));
        fs::write(&path, changed).unwrap();
        let old = out.join(format!("probe-{name}-prior"));
        let new = out.join(format!("probe-{name}"));
        release::normalize(prior, &path, 5, &old);
        release::normalize(package, &path, 5, &new);
        let change = compare_inputs(&old, &new, complete);
        assert_eq!(change.retired.is_empty(), !complete, "{name}");
    }
    count
}

#[test]
#[ignore = "requires exact checked encounter predecessor and passed full-source character witness"]
fn real_character_reward_inventory_preserves_all_other_original_facts() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CHARACTER_REWARD_INVENTORY_PRIOR")
            .expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let before = release::inventory(&prior_path);
    let next = family::stage(&prior);
    family::assert_stale_source_rejected(&next);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_CHARACTER_REWARD_INVENTORY_OUTPUT")
            .expect("fresh output"),
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
    let after = release::inventory(&package);
    assert_eq!(after, release::inventory(&rebuilt));
    assert_eq!(before.len(), after.len());
    for (file, hash) in &before {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&file.as_str())
        {
            assert_eq!(after.get(file), Some(hash), "unchanged artifact {file}");
        }
    }
    for row in family::authoring()["query_files"].as_array().unwrap() {
        let name = row["file"].as_str().unwrap();
        let bytes = fs::read(package.join(name)).unwrap();
        assert_eq!(bytes, fs::read(prior_path.join(name)).unwrap());
        assert_eq!(
            json!(format!("{:x}", Sha256::digest(&bytes))),
            row["sha256"]
        );
        assert_eq!(json!(bytes.len()), row["bytes"]);
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
    let full_completions: u64 = reports
        .iter()
        .map(|r| r["character_presets_completed"].as_u64().unwrap())
        .sum();
    let config_rewards: u64 = reports
        .iter()
        .map(|r| r["config_rewards_preserved"].as_u64().unwrap())
        .sum();
    assert_eq!(full_completions, 16);
    assert_eq!(config_rewards, 81);
    assert_eq!(next.receipt().query_rows, 110);
    let probes = probes(&prior_path, &package, &out);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,
        "definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":next.receipt().query_rows,
        "provenance":next.input().provenance.len(),"originals":reports,"full_character_completions":full_completions,
        "config_rewards_preserved":config_rewards,"probes":probes,"predecessor_unchanged":true,
        "rebuild_byte_identical":true,"stale_source_rejected":true,"complete_original_builds":0}),
    );
}
