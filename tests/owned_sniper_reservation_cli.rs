//! Numeric usage retains source/preset ownership and every unresolved inventory.
#[path = "support/owned_sniper_reservation.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::SkillUseId,
    owned_build::*,
    owned_content::digest_owned,
    owned_definitions::BoundedInteger,
    owned_draft::{DraftLimits, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{NormalizationLimits, PrimarySkillUsageInput},
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
type SourceLinks = Vec<(Value, Value)>;
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn local(value: &Value) -> &str {
    value["local"].as_str().unwrap()
}
fn link(sidecar: &Value, source: &Value, kind: &str) -> Value {
    let links: Vec<_> = sidecar["origins"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| &row["source"] == source)
        .unwrap()["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["kind"] == kind)
        .collect();
    assert_eq!(links.len(), 1, "exact {kind} source link");
    links[0]["value"].clone()
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
fn numeric_count_authoring_retains_unproved_mechanics() {
    family::check_authored();
}

fn commitments(package: &StagedOwnedRelease, case: usize, directory: &Path, sidecar: &Value) {
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
                NormalizationLimits::default().max_policy_bytes
            )
            .unwrap()
        )
    );
    let receipt = json!(package.receipt());
    for (field, binding) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(sidecar[field], receipt[binding], "{field}");
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
    for item in sidecar["item_texts"].as_array().unwrap() {
        assert_eq!(item["attribution"]["item_lines"], sidecar["item_policy"]);
        assert_eq!(item["attribution"]["policy"], sidecar["item_source_policy"]);
    }
}

/// Independently join original XML, imported occurrences and the saved preset.
fn exact_preferences(xml: &[u8], directory: &Path, sidecar: &Value) -> (SourceLinks, SourceLinks) {
    let rule: PrimarySkillUsageInput = family::read("usage.json");
    let draft = decode_draft(
        &fs::read(directory.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        draft.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let mut expected = vec![];
    let mut links = vec![];
    let mut group_links = vec![];
    for row in evidence.rows().iter().filter(|row| {
        row.occurrence().name() == "Gem"
            && row
                .attribute("gemId")
                .is_some_and(|v| v.decoded().unwrap() == rule.game_id)
    }) {
        let group = evidence
            .rows()
            .iter()
            .find(|v| Some(v.occurrence().id()) == row.occurrence().parent())
            .unwrap();
        let set = evidence
            .rows()
            .iter()
            .find(|v| Some(v.occurrence().id()) == group.occurrence().parent())
            .unwrap();
        assert_eq!(set.occurrence().name(), "SkillSet");
        assert!(
            group.attribute("source").is_none(),
            "finite authored source frame"
        );
        let source_id = json!(row.occurrence().id());
        let skill: SkillUseId = serde_json::from_value(link(sidecar, &source_id, "skill")).unwrap();
        let preset_id = link(sidecar, &json!(set.occurrence().id()), "skill_preset");
        assert_eq!(link(sidecar, &source_id, "skill_preset"), preset_id);
        let group_source = json!(group.occurrence().id());
        assert_eq!(link(sidecar, &group_source, "skill_preset"), preset_id);
        if !group_links.contains(&(group_source.clone(), preset_id.clone())) {
            group_links.push((group_source, preset_id.clone()));
        }
        let preset = draft
            .input()
            .skill_presets
            .members
            .iter()
            .find(|v| json!(v.id) == preset_id)
            .unwrap();
        assert!(preset.skills.members.contains(&skill));
        let count: i64 = group
            .attribute("groupCount")
            .or_else(|| row.attribute("count"))
            .unwrap()
            .decoded()
            .unwrap()
            .parse()
            .unwrap();
        let preference = UsagePolicySelection {
            policy: rule.policies[0].policy.clone(),
            target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(skill),
                    grant_path: vec![],
                },
                slot: rule.supply.clone(),
            }))),
            parameters: vec![ParameterAssignment {
                slot: rule.policies[0].parameters[0].slot.clone(),
                value: ParameterValue::Integer(BoundedInteger::new(count).unwrap()),
            }],
        };
        assert_eq!(
            preset
                .usage_preferences
                .as_ref()
                .unwrap()
                .members
                .iter()
                .filter(|v| v.to_resolved().as_ref() == Some(&preference))
                .count(),
            1
        );
        expected.push(preference);
        links.push((source_id, preset_id));
    }
    let actual: Vec<_> = draft
        .input()
        .skill_presets
        .members
        .iter()
        .filter_map(|v| v.usage_preferences.as_ref())
        .flat_map(|v| &v.members)
        .filter_map(|v| v.to_resolved())
        .filter(|v| v.policy == rule.policies[0].policy)
        .collect();
    assert_eq!(actual.len(), expected.len());
    assert!(actual.iter().all(|v| expected.contains(v)));
    (links, group_links)
}

fn issue_sources(sidecar: &Value, issue: &Value) -> Vec<(Value, usize)> {
    let mut sources = vec![];
    for row in sidecar["origins"].as_array().unwrap() {
        for (index, link) in row["links"].as_array().unwrap().iter().enumerate() {
            if link["kind"] == "issue" && &link["value"] == issue {
                sources.push((row["source"].clone(), index));
            }
        }
    }
    sources
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
    let (mut source_links, mut group_links) = exact_preferences(xml, &new, &sb);
    assert_eq!(source_links.len(), [1, 0, 0, 0, 5][case - 1]);
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    for (source, preset) in &mut source_links {
        selected::canonical(source);
        selected::canonical(preset);
    }
    for (source, preset) in &mut group_links {
        selected::canonical(source);
        selected::canonical(preset);
    }
    let rule: PrimarySkillUsageInput = family::read("usage.json");
    let mut added_issues = BTreeSet::new();
    let mut usage_moves = vec![];
    let mut preferences = 0;
    let old_presets = a["draft"]["skill_presets"]["members"].as_array().unwrap();
    let new_presets = b["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap();
    assert_eq!(old_presets.len(), new_presets.len());
    for (before, after) in old_presets.iter().zip(new_presets) {
        let preset_id = after["id"].clone();
        let Some(usage) = after.get_mut("usage_preferences") else {
            continue;
        };
        let members = usage["members"].as_array_mut().unwrap();
        let count = members.len();
        members.retain(|v| v["policy"] != json!({"kind":"known","value":rule.policies[0].policy}));
        let removed = count - members.len();
        preferences += removed;
        if removed == 0 {
            continue;
        }
        assert_eq!(
            removed,
            source_links
                .iter()
                .filter(|(_, preset)| *preset == preset_id)
                .count()
        );
        assert_eq!(
            usage["completion"]["code"],
            "usage-preferences-not-converted"
        );
        if let Some(old_usage) = before.get("usage_preferences") {
            assert_eq!(
                old_usage["completion"]["code"],
                "usage-preferences-not-converted"
            );
            let old_sources = issue_sources(&sa, &old_usage["completion"]["id"]);
            let new_sources = issue_sources(&sb, &usage["completion"]["id"]);
            assert_eq!(old_sources.len(), 1);
            assert_eq!(new_sources.len(), 1);
            if old_sources[0].0 != new_sources[0].0 {
                assert!(source_links.contains(&(new_sources[0].0.clone(), preset_id)));
                usage_moves.push((
                    new_sources[0].0.clone(),
                    old_sources[0].0.clone(),
                    old_sources[0].1,
                    usage["completion"]["id"].clone(),
                ));
            }
        } else {
            assert!(usage["members"].as_array().unwrap().is_empty());
            let sources = issue_sources(&sb, &usage["completion"]["id"]);
            assert_eq!(sources.len(), 1);
            assert!(source_links.contains(&(sources[0].0.clone(), preset_id)));
            assert!(added_issues.insert(local(&usage["completion"]["id"]).to_owned()));
            after.as_object_mut().unwrap().remove("usage_preferences");
        }
    }
    assert_eq!(preferences, source_links.len());
    let old_allocator = a["draft"]["allocator"].clone();
    let new_allocator = b["draft"]["allocator"].clone();
    let issued = |v: &Value| i64::from_str_radix(v["last_issued"].as_str().unwrap(), 16).unwrap();
    assert_eq!(
        issued(&new_allocator) - issued(&old_allocator),
        added_issues.len() as i64
    );
    b["draft"]["allocator"] = old_allocator.clone();
    let mut removed_issues = 0;
    let mut removed_links = 0;
    let mut moved = 0;
    for row in sb["origins"].as_array_mut().unwrap() {
        let source = row["source"].clone();
        let previous = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source"] == source)
            .unwrap();
        row["links"].as_array_mut().unwrap().retain(|link| {
            if link["kind"] == "issue" && added_issues.contains(local(&link["value"])) {
                removed_issues += 1;
                return false;
            }
            if link["kind"] == "skill_preset"
                && (source_links.contains(&(source.clone(), link["value"].clone()))
                    || group_links.contains(&(source.clone(), link["value"].clone())))
                && !previous["links"].as_array().unwrap().contains(link)
            {
                removed_links += 1;
                return false;
            }
            if usage_moves.iter().any(|(from, _, _, issue)| {
                *from == source && link["kind"] == "issue" && link["value"] == *issue
            }) {
                moved += 1;
                return false;
            }
            true
        });
    }
    assert_eq!(removed_issues, added_issues.len());
    let expected_links = source_links
        .iter()
        .chain(&group_links)
        .filter(|(source, preset)| {
            !sa["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| row["source"] == *source)
                .unwrap()["links"]
                .as_array()
                .unwrap()
                .contains(&json!({"kind":"skill_preset","value":preset}))
        })
        .count();
    assert_eq!(removed_links, expected_links);
    assert_eq!(moved, usage_moves.len());
    for (_, to, index, issue) in &usage_moves {
        sb["origins"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["source"] == *to)
            .unwrap()["links"]
            .as_array_mut()
            .unwrap()
            .insert(*index, json!({"kind":"issue","value":issue}));
    }
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "only exact count preferences are added; all physical inventories survive",
    );
    for item in sb["item_texts"].as_array_mut().unwrap() {
        item["attribution"]["item_lines"] = sa["item_policy"].clone();
        item["attribution"]["policy"] = sa["item_source_policy"].clone();
    }
    for field in [
        "draft",
        "policy",
        "mapping",
        "registry",
        "definitions",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "tree_policy",
    ] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa["allocator_after"], old_allocator);
    assert_eq!(sb["allocator_after"], new_allocator);
    sb["allocator_after"] = sa["allocator_after"].clone();
    identity::correspond(&sa, &mut sb, &mut ids, "all unrelated provenance survives");
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
        out.join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    new_issues
        .as_array_mut()
        .unwrap()
        .retain(|row| !added_issues.contains(local(&row["id"])));
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    identity::relocate(&mut new_issues, &ids);
    assert_eq!(
        old_issues, new_issues,
        "numeric transport closes no physical or usage inventory"
    );
    assert_eq!(
        before["finalization"]["issues"].as_array().unwrap().len(),
        [119, 116, 108, 121, 19][case - 1]
    );
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    identity::relocate(&mut new_selection, &ids);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"source_sha256":format!("{:x}",Sha256::digest(xml)),"count_preferences":preferences,"new_usage_inventories":added_issues.len(),"same_preset_usage_source_relocations":usage_moves.len(),"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

#[test]
#[ignore = "requires checked Frost predecessor and full reservation source witness"]
fn numeric_count_preserves_five_originals_and_inventory_obligations() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_RESERVATION_PRIOR").expect("checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SNIPER_RESERVATION_OUTPUT").expect("new output"),
    );
    assert!(!out.exists());
    let old_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    for field in ["roles", "catalog", "scalar_inputs"] {
        let mut stale = json!(next.input());
        stale["normalization"]["usage_inputs"][field] = json!("0".repeat(64));
        let mut stale: poe_optimizer_import::owned_release::OwnedReleaseInput =
            serde_json::from_value(stale).unwrap();
        stale.tree.as_mut().unwrap().normalization = digest_owned(
            "owned-normalization-policy-v3",
            &stale.normalization,
            TreePolicyLimits::default().max_base_policy_bytes,
        )
        .unwrap();
        assert!(
            assemble_owned_release(stale, Default::default()).is_err(),
            "inner {field} cannot be laundered"
        );
    }
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&out.join("rebuilt")));
    assert_eq!(inventory.len(), 18);
    let mut reports = vec![];
    for case in 1..=5 {
        let query = format!("queries-original-{case:02}.json");
        assert_eq!(inventory[&query], old_inventory[&query]);
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
    let controls = count_controls(&package, &out);
    assert_eq!(old_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"controls":controls,"queries":110,"artifacts":18,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}

fn count_controls(package: &Path, out: &Path) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read_to_string(&path).unwrap();
    let original = decode_draft(
        &fs::read(out.join("original-05/draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let original_sidecar: Value = read(out.join("original-05/sidecar.json"));
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        original.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let skills = evidence
        .rows()
        .iter()
        .find(|v| v.occurrence().name() == "Skills")
        .unwrap();
    let selected_set = skills
        .attribute("activeSkillSet")
        .unwrap()
        .decoded()
        .unwrap();
    let rule: PrimarySkillUsageInput = family::read("usage.json");
    let candidates: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| {
            if row.occurrence().name() != "Gem"
                || !row
                    .attribute("gemId")
                    .is_some_and(|v| v.decoded().unwrap() == rule.game_id)
            {
                return false;
            }
            let group = &evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
            let set = &evidence.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
            set.attribute("id").unwrap().decoded().unwrap() == selected_set
        })
        .collect();
    assert_eq!(candidates.len(), 1);
    let row = candidates[0];
    let group = &evidence.rows()[row.occurrence().parent().unwrap().ordinal() as usize];
    let set = &evidence.rows()[group.occurrence().parent().unwrap().ordinal() as usize];
    let selected_preset = link(
        &original_sidecar,
        &json!(set.occurrence().id()),
        "skill_preset",
    );
    let mut reports = vec![];
    for (name, count, group_count, expected) in [
        ("zero", Some("0"), None, Some(0)),
        ("three", Some("3"), None, Some(3)),
        ("group-zero", Some("3"), Some("0"), Some(0)),
        ("group-four", Some("3"), Some("4"), Some(4)),
        ("missing", None, None, None),
        ("malformed", Some("bad"), None, None),
        ("fractional", Some("1.5"), None, None),
        ("outside-domain", Some("5"), None, None),
        ("malformed-override", Some("3"), Some("bad"), None),
        ("nil-override", Some("3"), Some("nil"), None),
    ] {
        let fragment = source.source_fragment(row.occurrence().id()).unwrap();
        let changed = match count {
            Some(value) => fragment.replacen("count=\"1\"", &format!("count=\"{value}\""), 1),
            None => fragment.replacen(" count=\"1\"", "", 1),
        };
        assert_ne!(fragment, changed);
        let old_group = source.source_fragment(group.occurrence().id()).unwrap();
        let mut changed_group = old_group.replacen(fragment, &changed, 1);
        if let Some(value) = group_count {
            assert!(group.attribute("groupCount").is_none());
            changed_group =
                changed_group.replacen("<Skill ", &format!("<Skill groupCount=\"{value}\" "), 1);
        }
        let mut control = xml.clone();
        control.replace_range(group.occurrence().range(), &changed_group);
        let control_path = out.join(format!("control-{name}.xml"));
        fs::write(&control_path, &control).unwrap();
        let directory = out.join(format!("control-{name}"));
        release::normalize(package, &control_path, 5, &directory);
        let sidecar: Value = read(directory.join("sidecar.json"));
        let draft = decode_draft(
            &fs::read(directory.join("draft.json")).unwrap(),
            DraftLimits::default(),
        )
        .unwrap();
        let changed_source = ImportedBuildInstance::from_decoded(
            decode_build(control.as_bytes()).unwrap(),
            draft.input().allocator.lineage(),
            InstanceImportLimits::default(),
        )
        .unwrap();
        let changed_row = &changed_source.occurrences()[row.occurrence().id().ordinal() as usize];
        let skill: SkillUseId =
            serde_json::from_value(link(&sidecar, &json!(changed_row.id()), "skill")).unwrap();
        let presets: Vec<_> = draft
            .input()
            .skill_presets
            .members
            .iter()
            .filter(|v| v.skills.members.contains(&skill))
            .collect();
        assert_eq!(presets.len(), 1);
        let usage = presets[0].usage_preferences.as_ref().unwrap();
        assert_eq!(
            json!(usage.completion)["code"],
            "usage-preferences-not-converted"
        );
        let target = UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
            provider: ProviderKey {
                root: ProviderRoot::SkillUse(skill),
                grant_path: vec![],
            },
            slot: rule.supply.clone(),
        })));
        let preferences: Vec<_> = usage
            .members
            .iter()
            .filter(|v| v.target.to_resolved().as_ref() == Some(&target))
            .collect();
        assert_eq!(preferences.len(), 1, "{name}");
        let resolved = preferences[0].to_resolved();
        match expected {
            Some(value) => assert_eq!(
                resolved.unwrap().parameters,
                vec![ParameterAssignment {
                    slot: rule.policies[0].parameters[0].slot.clone(),
                    value: ParameterValue::Integer(BoundedInteger::new(value).unwrap())
                }],
                "{name}"
            ),
            None => {
                assert!(resolved.is_none(), "{name}: no fallback from invalid input");
                assert_eq!(
                    json!(preferences[0].parameters.completion)["code"],
                    "usage-parameters-not-converted"
                );
            }
        }
        let mut ids = BTreeMap::new();
        let mut before = json!({"gems":original.input().gems, "skills":original.input().skills});
        let mut after = json!({"gems":draft.input().gems, "skills":draft.input().skills});
        selected::canonical(&mut before);
        selected::canonical(&mut after);
        identity::correspond(
            &before,
            &mut after,
            &mut ids,
            "count edits preserve every physical input, SkillUse and inventory",
        );
        let mut archived = 0;
        for old_preset in &original.input().skill_presets.members {
            if json!(old_preset.id) == selected_preset {
                continue;
            }
            let source_set = original_sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|row| {
                    row["links"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"kind":"skill_preset","value":old_preset.id}))
                        && source.occurrences()[row["source"]["ordinal"].as_u64().unwrap() as usize]
                            .name()
                            == "SkillSet"
                })
                .unwrap();
            let next_set = &changed_source.occurrences()
                [source_set["source"]["ordinal"].as_u64().unwrap() as usize];
            let next_id = link(&sidecar, &json!(next_set.id()), "skill_preset");
            let next_preset = draft
                .input()
                .skill_presets
                .members
                .iter()
                .find(|v| json!(v.id) == next_id)
                .unwrap();
            let mut a = json!(old_preset);
            let mut b = json!(next_preset);
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            identity::correspond(
                &a,
                &mut b,
                &mut ids,
                "each archived preset remains whole and independent",
            );
            archived += 1;
        }
        assert_eq!(archived, original.input().skill_presets.members.len() - 1);
        reports.push(json!({"name":name,"typed_count":expected,"archived_presets_preserved":archived,"all_physical_inputs_preserved":true,"source_sha256":format!("{:x}",Sha256::digest(control.as_bytes()))}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), xml);
    reports
}
