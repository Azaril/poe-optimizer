//! Physical input closure and occurrence-specific usage preserve the five saved builds.
#[path = "support/owned_frost_bomb_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::SkillUseId,
    owned_build::{
        GeneratedSkillKey, ParameterAssignment, ParameterValue, ProviderKey, ProviderRoot,
        SkillTarget, UsagePolicySelection, UsageTarget,
    },
    owned_content::digest_owned,
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
fn read<T: DeserializeOwned>(path: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}
fn local(id: &Value) -> String {
    id["local"].as_str().unwrap().to_owned()
}
fn issued(allocator: &Value) -> i64 {
    i64::from_str_radix(allocator["last_issued"].as_str().unwrap(), 16).unwrap()
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
fn frost_input_authoring_keeps_unproved_mechanics_partial() {
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

/// Authenticate every admitted preference against its exact physical source and saved preset.
fn exact_source_preferences(xml: &[u8], directory: &Path, sidecar: &Value) -> Vec<(Value, Value)> {
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
    let origin = |id| {
        sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source"] == json!(id))
            .unwrap()
    };
    let mut expected = vec![];
    let mut links = vec![];
    for row in evidence.rows().iter().filter(|row| {
        row.occurrence().name() == "Gem"
            && row
                .attribute("gemId")
                .is_some_and(|v| v.decoded().unwrap() == rule.game_id)
    }) {
        let source_links = origin(row.occurrence().id())["links"].as_array().unwrap();
        let skill_links: Vec<_> = source_links
            .iter()
            .filter(|link| link["kind"] == "skill")
            .collect();
        let gem_links: Vec<_> = source_links
            .iter()
            .filter(|link| link["kind"] == "gem")
            .collect();
        assert_eq!(skill_links.len(), 1);
        assert_eq!(gem_links.len(), 1);
        let skill: SkillUseId = serde_json::from_value(skill_links[0]["value"].clone()).unwrap();
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
        let presets: Vec<_> = origin(set.occurrence().id())["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["kind"] == "skill_preset")
            .collect();
        assert_eq!(presets.len(), 1);
        let preset_id = &presets[0]["value"];
        let preset = draft
            .input()
            .skill_presets
            .members
            .iter()
            .find(|preset| json!(preset.id) == *preset_id)
            .unwrap();
        assert!(preset.skills.members.contains(&skill));
        assert_eq!(
            source_links
                .iter()
                .filter(|v| v["kind"] == "skill_preset" && v["value"] == *preset_id)
                .count(),
            1
        );
        let enabled = match row.attribute("enableGlobal1").unwrap().decoded().unwrap() {
            "true" => true,
            "false" => false,
            _ => panic!("outside authored Boolean domain"),
        };
        assert_eq!(row.attribute("count").unwrap().decoded().unwrap(), "1");
        let preference = UsagePolicySelection {
            policy: rule.policy.clone(),
            target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(skill),
                    grant_path: vec![],
                },
                slot: rule.supply.clone(),
            }))),
            parameters: vec![ParameterAssignment {
                slot: rule.parameters[0].slot.clone(),
                value: ParameterValue::Boolean(enabled),
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
        links.push((json!(row.occurrence().id()), preset_id.clone()));
    }
    let actual: Vec<_> = draft
        .input()
        .skill_presets
        .members
        .iter()
        .filter_map(|preset| preset.usage_preferences.as_ref())
        .flat_map(|usage| &usage.members)
        .filter_map(|row| row.to_resolved())
        .filter(|row| row.policy == rule.policy)
        .collect();
    assert_eq!(actual.len(), expected.len());
    assert!(actual.iter().all(|row| expected.contains(row)));
    links
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
    let mut source_links = exact_source_preferences(xml, &new, &sb);
    assert_eq!(source_links.len(), [0, 0, 0, 1, 4][case - 1]);
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    for (source, preset) in &mut source_links {
        selected::canonical(source);
        selected::canonical(preset);
    }
    let rule: PrimarySkillUsageInput = family::read("usage.json");
    let mut retired = BTreeSet::new();
    let old_gems = a["draft"]["gems"]["members"].as_array_mut().unwrap();
    let new_gems = b["draft"]["gems"]["members"].as_array_mut().unwrap();
    assert_eq!(old_gems.len(), new_gems.len());
    for (before, after) in old_gems.iter_mut().zip(new_gems) {
        if before["definition"] != json!({"kind":"known","value":rule.gem}) {
            continue;
        }
        for field in ["definition", "level", "quality"] {
            assert_eq!(before[field], after[field]);
        }
        assert_eq!(
            before["parameters"]["members"],
            after["parameters"]["members"]
        );
        assert_eq!(before["parameters"]["members"].as_array().unwrap().len(), 2);
        assert_eq!(
            before["parameters"]["completion"]["code"],
            "gem-parameters-not-converted"
        );
        assert_eq!(
            after["parameters"]["completion"],
            json!({"kind":"complete"})
        );
        assert!(retired.insert(local(&before["parameters"]["completion"]["id"])));
        for row in [before, after] {
            row["parameters"]
                .as_object_mut()
                .unwrap()
                .remove("completion");
        }
    }
    assert_eq!(retired.len(), source_links.len());
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
        members.retain(|v| v["policy"] != json!({"kind":"known","value":rule.policy}));
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
                assert!(
                    new_sources[0].0["ordinal"].as_u64().unwrap()
                        < old_sources[0].0["ordinal"].as_u64().unwrap()
                );
                let previous = sa["origins"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|row| row["source"] == old_sources[0].0)
                    .unwrap();
                assert!(
                    previous["links"]
                        .as_array()
                        .unwrap()
                        .contains(&json!({"kind":"skill_preset","value":before["id"]})),
                    "same saved preset retains its existing inventory obligation"
                );
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
            assert!(added_issues.insert(local(&usage["completion"]["id"])));
            after.as_object_mut().unwrap().remove("usage_preferences");
        }
    }
    assert_eq!(preferences, source_links.len());
    let old_allocator = a["draft"]["allocator"].clone();
    let new_allocator = b["draft"]["allocator"].clone();
    assert_eq!(
        issued(&new_allocator) - issued(&old_allocator),
        added_issues.len() as i64 - retired.len() as i64,
        "only named inventory obligations change allocation"
    );
    b["draft"]["allocator"] = old_allocator.clone();
    let mut removed_old = 0;
    for row in sa["origins"].as_array_mut().unwrap() {
        row["links"].as_array_mut().unwrap().retain(|link| {
            let remove = link["kind"] == "issue" && retired.contains(&local(&link["value"]));
            removed_old += usize::from(remove);
            !remove
        });
    }
    assert_eq!(removed_old, retired.len());
    let mut removed_new = 0;
    let mut removed_links = 0;
    let mut moved = 0;
    for row in sb["origins"].as_array_mut().unwrap() {
        let source = row["source"].clone();
        let previous = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["source"] == source)
            .unwrap();
        row["links"].as_array_mut().unwrap().retain(|link| {
            if link["kind"] == "issue" && added_issues.contains(&local(&link["value"])) {
                removed_new += 1;
                return false;
            }
            if link["kind"] == "skill_preset"
                && source_links.contains(&(source.clone(), link["value"].clone()))
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
    assert_eq!(removed_new, added_issues.len());
    assert_eq!(removed_links, preferences);
    assert_eq!(moved, usage_moves.len());
    for (_, to, index, issue) in &usage_moves {
        let target = sb["origins"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|row| row["source"] == *to)
            .unwrap();
        target["links"]
            .as_array_mut()
            .unwrap()
            .insert(*index, json!({"kind":"issue","value":issue}));
    }
    let mut ids = BTreeMap::new();
    identity::correspond(
        &a,
        &mut b,
        &mut ids,
        "only witnessed physical completion and exact usage records change",
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
    identity::correspond(
        &sa,
        &mut sb,
        &mut ids,
        "all remaining source provenance survives",
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
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|row| !retired.contains(&local(&row["id"])));
    new_issues
        .as_array_mut()
        .unwrap()
        .retain(|row| !added_issues.contains(&local(&row["id"])));
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    identity::relocate(&mut new_issues, &ids);
    assert_eq!(
        old_issues, new_issues,
        "every unrelated selected obligation survives"
    );
    assert_eq!(
        before["finalization"]["issues"].as_array().unwrap().len(),
        [119, 116, 108, 121, 20][case - 1]
    );
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    identity::relocate(&mut new_selection, &ids);
    assert_eq!(
        old_selection, new_selection,
        "saved selections and all query identities remain exact"
    );
    json!({"original":case,"source_sha256":format!("{:x}", Sha256::digest(xml)),"physical_lists_completed":retired.len(),"exact_usage_preferences":preferences,"new_usage_inventories":added_issues.len(),"same_preset_usage_source_relocations":usage_moves.len(),"selected_before":before["finalization"]["issues"].as_array().unwrap().len(),"selected_after":after["finalization"]["issues"].as_array().unwrap().len(),"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}

fn guard_controls(package: &Path, out: &Path) -> Vec<Value> {
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
        .find(|row| row.occurrence().name() == "Skills")
        .unwrap();
    let selected = skills
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
                    .is_some_and(|value| value.decoded().unwrap() == rule.game_id)
            {
                return false;
            }
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
            set.occurrence().name() == "SkillSet"
                && set.attribute("id").unwrap().decoded().unwrap() == selected
        })
        .collect();
    assert_eq!(candidates.len(), 1);
    let row = candidates[0];
    let mut reports = vec![];
    for (name, attribute, changed, expected, complete) in [
        (
            "global-one-false",
            "enableGlobal1",
            Some("false"),
            Some(false),
            true,
        ),
        (
            "global-two-false",
            "enableGlobal2",
            Some("false"),
            Some(true),
            true,
        ),
        ("global-one-missing", "enableGlobal1", None, None, false),
        (
            "global-one-malformed",
            "enableGlobal1",
            Some("bad"),
            None,
            false,
        ),
        ("outside-count-domain", "count", Some("2"), None, false),
    ] {
        let field = row.attribute(attribute).unwrap();
        let mut control = xml.clone();
        if let Some(changed) = changed {
            control.replace_range(field.range(), changed);
        } else {
            let range = row.occurrence().range();
            let fragment = &xml[range.clone()];
            let token = format!("{attribute}=\"{}\"", field.raw());
            assert_eq!(fragment.matches(&token).count(), 1);
            control.replace_range(range, &fragment.replacen(&token, "", 1));
        }
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
        let changed_row = changed_source
            .occurrences()
            .iter()
            .find(|v| v.id().ordinal() == row.occurrence().id().ordinal())
            .unwrap();
        let origin = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|v| v["source"] == json!(changed_row.id()))
            .unwrap();
        let exact_link = |kind| {
            let links: Vec<_> = origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["kind"] == kind)
                .collect();
            assert_eq!(links.len(), 1);
            links[0]["value"].clone()
        };
        let gem_id = exact_link("gem");
        let skill: SkillUseId = serde_json::from_value(exact_link("skill")).unwrap();
        let gem = draft
            .input()
            .gems
            .members
            .iter()
            .find(|v| json!(v.id) == gem_id)
            .unwrap();
        assert_eq!(
            json!(gem.parameters.completion)["kind"] == "complete",
            complete,
            "{name}: physical closure requires the actual typed preference"
        );
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
            .filter(|v| {
                v.policy.to_resolved().as_ref() == Some(&rule.policy)
                    && v.target.to_resolved().as_ref() == Some(&target)
            })
            .collect();
        if let Some(enabled) = expected {
            assert_eq!(preferences.len(), 1);
            assert_eq!(
                preferences[0].to_resolved().unwrap().parameters,
                vec![ParameterAssignment {
                    slot: rule.parameters[0].slot.clone(),
                    value: ParameterValue::Boolean(enabled)
                }]
            );
        } else if attribute == "count" {
            assert!(
                preferences.is_empty(),
                "out-of-domain count cannot produce a usage proof"
            );
        } else {
            assert_eq!(preferences.len(), 1);
            assert!(preferences[0].to_resolved().is_none());
            assert_eq!(
                json!(preferences[0].parameters.completion)["code"],
                "usage-parameters-not-converted"
            );
        }
        let exact_link = |sidecar: &Value, source: Value, kind: &str| {
            let origin = sidecar["origins"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["source"] == source)
                .unwrap();
            let links: Vec<_> = origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["kind"] == kind)
                .collect();
            assert_eq!(links.len(), 1);
            links[0]["value"].clone()
        };
        let mut archived = 0;
        let mut ids = BTreeMap::new();
        for sibling in evidence.rows().iter().filter(|v| {
            v.occurrence().name() == "Gem"
                && v.occurrence().id() != row.occurrence().id()
                && v.attribute("gemId")
                    .is_some_and(|a| a.decoded().unwrap() == rule.game_id)
        }) {
            let group = evidence
                .rows()
                .iter()
                .find(|v| Some(v.occurrence().id()) == sibling.occurrence().parent())
                .unwrap();
            let set = evidence
                .rows()
                .iter()
                .find(|v| Some(v.occurrence().id()) == group.occurrence().parent())
                .unwrap();
            assert_ne!(set.attribute("id").unwrap().decoded().unwrap(), selected);
            let next_source = changed_source
                .occurrences()
                .iter()
                .find(|v| v.id().ordinal() == sibling.occurrence().id().ordinal())
                .unwrap();
            assert_eq!(
                source.source_fragment(sibling.occurrence().id()).unwrap(),
                changed_source.source_fragment(next_source.id()).unwrap()
            );
            let mut pair = vec![];
            for (session, provenance, source) in [
                (
                    &original,
                    &original_sidecar,
                    json!(sibling.occurrence().id()),
                ),
                (&draft, &sidecar, json!(next_source.id())),
            ] {
                let gem = exact_link(provenance, source.clone(), "gem");
                let skill = exact_link(provenance, source.clone(), "skill");
                let preset = exact_link(provenance, source, "skill_preset");
                let mut value = json!({
                    "gem":session.input().gems.members.iter().find(|v| json!(v.id) == gem).unwrap(),
                    "skill":session.input().skills.members.iter().find(|v| json!(v.id) == skill).unwrap(),
                    "preset":session.input().skill_presets.members.iter().find(|v| json!(v.id) == preset).unwrap(),
                });
                selected::canonical(&mut value);
                pair.push(value);
            }
            let mut after = pair.pop().unwrap();
            identity::correspond(
                &pair[0],
                &mut after,
                &mut ids,
                "mutating the selected source preserves each archived physical input, skill and entire preset",
            );
            archived += 1;
        }
        assert_eq!(archived, 3);
        reports.push(json!({"name":name,"physical_complete":complete,"typed_value":expected,"archived_presets_preserved":archived,"source_sha256":format!("{:x}",Sha256::digest(control.as_bytes()))}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), xml);
    reports
}

#[test]
#[ignore = "requires checked Direct-input predecessor and authenticated full Frost source witness"]
fn frost_inputs_preserve_five_originals_and_remaining_obligations() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FROST_INPUT_PRIOR").expect("checked prior"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_FROST_INPUT_OUTPUT").expect("new output"),
    );
    assert!(!out.exists(), "new output only");
    let old_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    for field in ["roles", "catalog", "scalar_inputs", "usage_inputs"] {
        let mut stale = json!(next.input());
        stale["normalization"]["gem_inventory"][field] = json!("0".repeat(64));
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
            "inner {field} cannot be laundered by outer rebinding"
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
    let inventory = release::inventory(&package);
    assert_eq!(inventory, release::inventory(&rebuilt));
    assert_eq!(inventory.len(), 18);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.input().query_sets == prior.input().query_sets);
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
    let controls = guard_controls(&package, &out);
    assert_eq!(old_inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,"controls":controls,"queries":110,"artifacts":18,"stale_binding_rejections":4,"prior_unchanged":true,"rebuild_byte_identical":true,"complete_original_builds":0}),
    );
}
