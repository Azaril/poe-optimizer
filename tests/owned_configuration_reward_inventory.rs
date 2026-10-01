//! Real selected requests close only their independently proved reward inventory.
#[path = "support/owned_configuration_reward_inventory.rs"]
mod family;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::ConfigurationRewardInventoryPolicy,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
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
fn authored_inventory_enumerates_the_entire_reviewed_control_family() {
    let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. } =
        family::policy();
    let facts: Value =
        read(root().join("data/owned/poe2/3887ae68/import/reward-source-facts.json"));
    let reviewed: BTreeSet<_> = facts["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["config_key"].as_str().unwrap())
        .collect();
    let actual: BTreeSet<_> = controls.iter().map(|c| c.selector.name.as_str()).collect();
    assert_eq!(actual, reviewed);
    assert_eq!(actual.len(), 17);
    assert_eq!(
        controls
            .iter()
            .map(|c| &c.recipe)
            .collect::<BTreeSet<_>>()
            .len(),
        17
    );
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
        selected::canonical(v);
    }
    assert_eq!(a["draft"]["allocator"], b["draft"]["allocator"]);
    let old_presets = a["draft"]["choice_presets"]["members"]
        .as_array_mut()
        .unwrap();
    let new_presets = b["draft"]["choice_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(old_presets.len(), 1);
    assert_eq!(new_presets.len(), 1);
    let prior = &old_presets[0];
    let completion = &prior["rewards"]["completion"];
    assert_eq!(completion["kind"], "pending");
    assert_eq!(completion["code"], "configuration-rewards-not-converted");
    let retired = completion["id"].clone();
    let preset = prior["id"].clone();
    let roles_issue = prior["choices"]["completion"]["id"].clone();
    let member_count = prior["rewards"]["members"].as_array().unwrap().len();
    assert_eq!(member_count, [16, 17, 15, 16, 17][case - 1]);
    assert_eq!(
        new_presets[0]["rewards"]["completion"],
        json!({"kind":"complete"})
    );
    old_presets[0]["rewards"]["completion"] = json!({"kind":"complete"});
    assert!(
        a == b,
        "original {case}: unrelated draft facts, IDs or order changed"
    );

    // Retiring inventory authority must not discard the other meaning of a source row.
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([79; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    let ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 { controls, .. } =
        family::policy();
    let names: BTreeSet<_> = controls.iter().map(|c| c.selector.name.as_str()).collect();
    let config = [390, 427, 195, 373, 423][case - 1];
    let mut removed = 0;
    let mut additions = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        let ordinal = row["source"]["ordinal"].as_u64().unwrap() as usize;
        let source = &evidence.rows()[ordinal];
        let links = row["links"].as_array_mut().unwrap();
        let had_issue = links
            .iter()
            .any(|l| l["kind"] == "issue" && l["value"] == retired);
        links.retain(|l| {
            let drop = l["kind"] == "issue" && l["value"] == retired;
            removed += usize::from(drop);
            !drop
        });
        if ordinal != config && had_issue {
            assert_eq!(
                source.occurrence().parent().unwrap().ordinal() as usize,
                config
            );
            let reviewed = source.occurrence().name() == "Input"
                && source
                    .attribute("name")
                    .and_then(|a| a.decoded().ok())
                    .is_some_and(|n| names.contains(n));
            if reviewed || links.is_empty() {
                let target = if reviewed {
                    json!({"kind":"choice_preset","value":preset})
                } else {
                    json!({"kind":"issue","value":roles_issue})
                };
                if !links.contains(&target) {
                    links.push(target);
                    additions += 1;
                }
            }
        }
    }
    assert!(removed > 1);
    assert_eq!(sa["schema_version"], 15);
    assert_eq!(sb["schema_version"], 15);
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    for field in ["policy", "tree_policy", "draft"] {
        sb[field] = sa[field].clone();
    }
    // Origin links are sets; source row order and all other fields remain exact.
    for sidecar in [&mut sa, &mut sb] {
        for row in sidecar["origins"].as_array_mut().unwrap() {
            row["links"]
                .as_array_mut()
                .unwrap()
                .sort_by_key(Value::to_string);
        }
    }
    assert!(
        sa == sb,
        "original {case}: unrelated source evidence or provenance changed"
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
    assert_eq!(old_issues, new_issues);
    assert_eq!(count, [126, 127, 119, 156, 23][case - 1]);
    assert_eq!(new_issues.as_array().unwrap().len(), count - 1);
    let mut first = selected::selection(xml, old);
    let mut second = selected::selection(xml, new);
    selected::canonical(&mut first);
    selected::canonical(&mut second);
    assert_eq!(first, second);
    json!({"original":case,"before":count,"after":count-1,"retired":retired,
        "config_source":config,"reward_members":member_count,"retired_origin_links":removed,
        "retained_scope_links_added":additions,"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn probes(package: &Path, out: &Path) -> usize {
    let xml =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let saved = r#"<Input string="+5% to Lightning Resistance" name="questAct 4Halls Of The DeadTawhoa&apos;s Test"/>"#;
    assert_eq!(xml.matches(saved).count(), 1);
    let name = "questAct 4Halls Of The DeadTawhoa&apos;s Test";
    let mut variants = vec![
        (
            "unlisted-parsable",
            format!(r#"<Input string="+100 to maximum Life" name="{name}"/>"#),
            false,
            None,
        ),
        (
            "unknown-option",
            format!(r#"<Input string="opaque option" name="{name}"/>"#),
            false,
            None,
        ),
        (
            "wrong-lane",
            format!(r#"<Input number="1" name="{name}"/>"#),
            false,
            None,
        ),
        (
            "placeholder-alias",
            saved.replace("<Input", "<Placeholder"),
            false,
            None,
        ),
        ("duplicate-input", format!("{saved}{saved}"), false, None),
        (
            "conflicting-encoding",
            saved.replace("/>", " number=\"1\"/>"),
            false,
            None,
        ),
        (
            "namespace-input",
            saved.replace("/>", " xmlns=\"urn:other\"/>"),
            false,
            None,
        ),
        (
            "nested-input",
            saved.replace("/>", "><Child/></Input>"),
            false,
            None,
        ),
        (
            "explicit-none",
            format!(r#"<Input string="None" name="{name}"/>"#),
            true,
            Some(16),
        ),
        ("missing-list-default", String::new(), true, Some(16)),
        (
            "false-check",
            format!(r#"{saved}<Input boolean="false" name="questAct 1ClearfellBeira"/>"#),
            true,
            Some(16),
        ),
        (
            "malformed-check",
            format!(r#"{saved}<Input boolean="invalid" name="questAct 1ClearfellBeira"/>"#),
            false,
            None,
        ),
        (
            "unrelated-input",
            format!(r#"{saved}<Input string="unreviewed" name="unknownControl"/>"#),
            true,
            Some(17),
        ),
        (
            "custom-modifier",
            format!(
                r#"{saved}<CustomModifierBlock title="test" enabled="true">+100 to maximum Life</CustomModifierBlock>"#
            ),
            true,
            Some(17),
        ),
    ];
    variants.push(("duplicate-config-id", saved.into(), false, None));
    let count = variants.len();
    for (name, replacement, complete, expected_count) in variants {
        let mut changed = xml.replacen(saved, &replacement, 1);
        if name == "duplicate-config-id" {
            changed = changed.replacen("</Config>", "<ConfigSet id=\"1\"/></Config>", 1);
        }
        let path = out.join(format!("probe-{name}.xml"));
        let dir = out.join(format!("probe-{name}"));
        fs::write(&path, &changed).unwrap();
        release::normalize(package, &path, 5, &dir);
        let draft: Value = read(dir.join("draft.json"));
        let choices = draft["draft"]["choice_presets"]["members"]
            .as_array()
            .unwrap();
        assert!(!choices.is_empty());
        for choice in choices {
            assert_eq!(
                choice["rewards"]["completion"]["kind"],
                if complete { "complete" } else { "pending" },
                "{name}"
            );
            assert_eq!(choice["choices"]["completion"]["kind"], "pending", "{name}");
            if let Some(expected) = expected_count {
                assert_eq!(
                    choice["rewards"]["members"].as_array().unwrap().len(),
                    expected,
                    "{name}"
                );
            }
        }
        assert_eq!(
            draft["draft"]["rewards"]["completion"]["kind"], "pending",
            "{name}"
        );
        if matches!(
            name,
            "explicit-none" | "false-check" | "unrelated-input" | "custom-modifier"
        ) {
            assert_eq!(choices.len(), 1);
            let sidecar: Value = read(dir.join("sidecar.json"));
            let imported = ImportedBuildInstance::from_decoded(
                decode_build(changed.as_bytes()).unwrap(),
                BuildLineage::from_bytes([80; 16]),
                InstanceImportLimits::default(),
            )
            .unwrap();
            let evidence =
                SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
            let source = evidence
                .rows()
                .iter()
                .find(|r| {
                    if name == "custom-modifier" {
                        r.occurrence().name() == "CustomModifierBlock"
                            && r.attribute("title").and_then(|a| a.decoded().ok()) == Some("test")
                    } else {
                        let key = match name {
                            "explicit-none" => "questAct 4Halls Of The DeadTawhoa's Test",
                            "false-check" => "questAct 1ClearfellBeira",
                            "unrelated-input" => "unknownControl",
                            _ => unreachable!(),
                        };
                        r.occurrence().name() == "Input"
                            && r.attribute("name").and_then(|a| a.decoded().ok()) == Some(key)
                    }
                })
                .unwrap();
            let ordinal = source.occurrence().id().ordinal() as usize;
            let target = if matches!(name, "explicit-none" | "false-check") {
                json!({"kind":"choice_preset","value":choices[0]["id"]})
            } else {
                json!({"kind":"issue","value":choices[0]["choices"]["completion"]["id"]})
            };
            assert_eq!(
                sidecar["origins"][ordinal]["links"],
                json!([target]),
                "{name}: None is scoped evidence; unrelated semantics retain only their local obligation"
            );
        }
    }
    count
}

#[test]
#[ignore = "requires the checked enemy-level predecessor and passed full-source reward witness"]
fn real_reward_inventory_publication_preserves_all_other_original_facts() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_REWARD_INVENTORY_PRIOR").expect("explicit prior"),
    );
    let prior = release::load(&prior_path);
    let before = release::inventory(&prior_path);
    let next = family::stage(&prior);
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_REWARD_INVENTORY_OUTPUT").expect("fresh output"),
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
        &json!({"before":prior.receipt().input,"after":next.receipt().input,
        "definitions":next.receipt().definitions,"registry":next.receipt().registry,"queries":next.receipt().query_rows,
        "provenance":next.input().provenance.len(),"originals":reports,"probes":probes,"predecessor_unchanged":true,
        "rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
