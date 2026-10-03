//! Source-bound usage publication and preservation of the five unchanged originals.
#[path = "support/owned_skill_usage_inputs.rs"]
mod family;
#[path = "support/owned_canonical_instances.rs"]
mod instances;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{
    build_identity::{InstanceId, SkillUseId},
    owned_build::{
        GeneratedSkillKey, ParameterAssignment, ParameterValue, ProviderKey, ProviderRoot,
        SkillTarget, UsagePolicySelection, UsageTarget,
    },
    owned_content::digest_owned,
    owned_draft::{DraftLimits, DraftSession, decode_draft},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{NormalizationLimits, UsageInputPolicy},
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_tree_policy::TreePolicyLimits,
};
use serde::{Serialize, de::DeserializeOwned};
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
fn usage_authoring_targets_existing_primary_effect_without_count_stat() {
    family::check_authored();
}
fn commitments(package: &StagedOwnedRelease, case: usize, path: &Path, sidecar: &Value) {
    let draft = decode_draft(
        &fs::read(path.join("draft.json")).unwrap(),
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
    for (sidecar_key, receipt_key) in [
        ("mapping", "mapping"),
        ("registry", "registry"),
        ("definitions", "definitions"),
        ("skill_roles", "roles"),
        ("reward_policy", "rewards"),
        ("item_policy", "items"),
        ("item_source_policy", "item_source"),
        ("tree_policy", "tree"),
    ] {
        assert_eq!(sidecar[sidecar_key], receipt[receipt_key], "{sidecar_key}");
    }
    assert_eq!(
        sidecar["mapping_source"],
        json!(package.mapping().source_identity())
    );
}
fn exact_source_preferences(
    xml: &[u8],
    draft: &DraftSession,
    sidecar: &Value,
    policy: &UsageInputPolicy,
) -> Vec<(Value, Value)> {
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 { gems, .. } = policy;
    let rule = &gems[0];
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        draft.input().allocator.lineage(),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let mut expected = vec![];
    let mut origin_links = vec![];
    for row in evidence.rows().iter().filter(|row| {
        row.occurrence().name() == "Gem"
            && row
                .attribute("gemId")
                .is_some_and(|value| value.decoded().unwrap() == rule.game_id)
    }) {
        let origin = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|origin| origin["source"] == json!(row.occurrence().id()))
            .unwrap();
        let ids: Vec<_> = origin["links"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|link| link["kind"] == "skill")
            .collect();
        assert_eq!(ids.len(), 1, "one physical source SkillUse");
        let id: SkillUseId = serde_json::from_value(ids[0]["value"].clone()).unwrap();
        let group = evidence
            .rows()
            .iter()
            .find(|r| Some(r.occurrence().id()) == row.occurrence().parent())
            .unwrap();
        let set = evidence
            .rows()
            .iter()
            .find(|r| Some(r.occurrence().id()) == group.occurrence().parent())
            .unwrap();
        assert_eq!(set.occurrence().name(), "SkillSet");
        let set_origin = sidecar["origins"]
            .as_array()
            .unwrap()
            .iter()
            .find(|origin| origin["source"] == json!(set.occurrence().id()))
            .unwrap();
        let preset_id = &set_origin["links"]
            .as_array()
            .unwrap()
            .iter()
            .find(|link| link["kind"] == "skill_preset")
            .unwrap()["value"];
        let preset = draft
            .input()
            .skill_presets
            .members
            .iter()
            .find(|preset| json!(preset.id) == *preset_id)
            .unwrap();
        assert!(preset.skills.members.contains(&id));
        assert_eq!(
            origin["links"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|link| link["kind"] == "skill_preset" && link["value"] == *preset_id)
                .count(),
            1
        );
        origin_links.push((json!(row.occurrence().id()), preset_id.clone()));
        let enabled = match row.attribute("enableGlobal1").unwrap().decoded().unwrap() {
            "true" => true,
            "false" => false,
            _ => panic!("outside witnessed Boolean tokens"),
        };
        let preference = UsagePolicySelection {
            policy: rule.policy.clone(),
            target: UsageTarget::Skill(SkillTarget::Generated(Box::new(GeneratedSkillKey {
                provider: ProviderKey {
                    root: ProviderRoot::SkillUse(id),
                    grant_path: vec![],
                },
                slot: rule.supply.clone(),
            }))),
            parameters: vec![ParameterAssignment {
                slot: rule.parameters[0].slot.clone(),
                value: ParameterValue::Boolean(enabled),
            }],
        };
        let preferences = &preset.usage_preferences.as_ref().unwrap().members;
        assert_eq!(
            preferences
                .iter()
                .filter(|row| row.to_resolved().as_ref() == Some(&preference))
                .count(),
            1,
            "exact source -> preset -> supplying Skill -> typed input"
        );
        expected.push(preference);
    }
    let actual: Vec<_> = draft
        .input()
        .skill_presets
        .members
        .iter()
        .filter_map(|preset| preset.usage_preferences.as_ref())
        .flat_map(|usage| &usage.members)
        .map(|row| row.to_resolved().unwrap())
        .collect();
    assert_eq!(actual.len(), expected.len());
    for actual in actual {
        assert!(expected.contains(&actual));
    }
    origin_links
}
fn compare_original(
    case: usize,
    xml: &[u8],
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    out: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let old_session = decode_draft(
        &fs::read(old.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let new_session = decode_draft(
        &fs::read(new.join("draft.json")).unwrap(),
        DraftLimits::default(),
    )
    .unwrap();
    let old_lineage = old_session.input().allocator.lineage();
    let new_lineage = new_session.input().allocator.lineage();
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    commitments(prior, case, &old, &sa);
    commitments(next, case, &new, &sb);
    let origin_links = exact_source_preferences(
        xml,
        &new_session,
        &sb,
        next.normalization().usage_inputs.as_ref().unwrap(),
    );
    let Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 { gems, .. }) =
        &next.normalization().usage_inputs
    else {
        panic!()
    };
    let rule = &gems[0];
    let mut added = vec![];
    let mut preferences = 0;
    for preset in b["draft"]["skill_presets"]["members"]
        .as_array_mut()
        .unwrap()
    {
        let Some(usage) = preset.as_object_mut().unwrap().remove("usage_preferences") else {
            continue;
        };
        assert_eq!(usage["completion"]["kind"], "pending");
        let issue: InstanceId = serde_json::from_value(usage["completion"]["id"].clone()).unwrap();
        assert_eq!(issue.lineage(), new_lineage);
        added.push(issue.local());
        let members = usage["members"].as_array().unwrap();
        assert_eq!(members.len(), 1);
        for member in members {
            assert_eq!(
                member["policy"],
                json!({"kind":"known","value":rule.policy})
            );
            assert_eq!(member["parameters"]["completion"]["kind"], "complete");
            assert_eq!(member["parameters"]["members"].as_array().unwrap().len(), 1);
            assert_eq!(
                member["parameters"]["members"][0]["value"],
                json!({"kind":"known","value":{"kind":"boolean","value":true}})
            );
            preferences += 1;
        }
    }
    assert_eq!(preferences, if case == 5 { 3 } else { 0 });
    added.sort_unstable();
    assert!(added.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(
        new_session.input().allocator.last_issued() - old_session.input().allocator.last_issued(),
        added.len() as u64
    );
    let mut removed = 0;
    let mut removed_presets = 0;
    for source in sb["origins"].as_array_mut().unwrap() {
        let source_id = source["source"].clone();
        source["links"].as_array_mut().unwrap().retain(|link| {
            let matches = link["kind"] == "issue"
                && added.contains(
                    &serde_json::from_value::<InstanceId>(link["value"].clone())
                        .unwrap()
                        .local(),
                );
            if matches {
                assert!(
                    origin_links
                        .iter()
                        .any(|(expected, _)| expected == &source_id),
                    "new inventory issue retains exact participating source row"
                );
            }
            removed += usize::from(matches);
            let preset_link = link["kind"] == "skill_preset"
                && origin_links.contains(&(source_id.clone(), link["value"].clone()));
            removed_presets += usize::from(preset_link);
            !matches && !preset_link
        });
    }
    assert_eq!(removed, added.len());
    assert_eq!(removed_presets, preferences);
    instances::canonical_instances(&mut a, old_lineage, &[]);
    instances::canonical_instances(&mut b, new_lineage, &added);
    assert!(
        a == b,
        "original {case}: only the new preferences and their inventory obligations may change"
    );
    let new_item_policy = sb["item_policy"].clone();
    let new_source_policy = sb["item_source_policy"].clone();
    for (previous, current) in sa["item_texts"]
        .as_array()
        .unwrap()
        .iter()
        .zip(sb["item_texts"].as_array_mut().unwrap())
    {
        assert_eq!(previous["attribution"]["item_lines"], sa["item_policy"]);
        assert_eq!(previous["attribution"]["policy"], sa["item_source_policy"]);
        assert_eq!(current["attribution"]["item_lines"], new_item_policy);
        assert_eq!(current["attribution"]["policy"], new_source_policy);
        current["attribution"]["item_lines"] = previous["attribution"]["item_lines"].clone();
        current["attribution"]["policy"] = previous["attribution"]["policy"].clone();
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
    for field in ["allocator_before", "source_allocator"] {
        let mut before = sa.as_object_mut().unwrap().remove(field).unwrap();
        let mut after = sb.as_object_mut().unwrap().remove(field).unwrap();
        instances::canonical_instances(&mut before, old_lineage, &[]);
        instances::canonical_instances(&mut after, new_lineage, &[]);
        assert_eq!(before, after);
    }
    instances::canonical_instances(&mut sa, old_lineage, &[]);
    instances::canonical_instances(&mut sb, new_lineage, &added);
    assert!(
        sa == sb,
        "original {case}: all prior source attribution is retained"
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
    let before_count = old_issues.as_array().unwrap().len();
    let after_count = new_issues.as_array().unwrap().len();
    new_issues.as_array_mut().unwrap().retain(|issue| {
        !added.contains(
            &serde_json::from_value::<InstanceId>(issue["id"].clone())
                .unwrap()
                .local(),
        )
    });
    instances::canonical_instances(&mut old_issues, old_lineage, &[]);
    instances::canonical_instances(&mut new_issues, new_lineage, &added);
    assert_eq!(
        old_issues, new_issues,
        "all pre-existing selected obligations remain"
    );
    assert_eq!(before_count, [116, 116, 108, 121, 18][case - 1]);
    assert_eq!(after_count, before_count + usize::from(case == 5));
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    instances::canonical_instances(&mut old_selection, old_lineage, &[]);
    instances::canonical_instances(&mut new_selection, new_lineage, &added);
    assert_eq!(
        old_selection, new_selection,
        "saved selections and all query identities"
    );
    json!({"original":case,"selected_before":before_count,"selected_after":after_count,"preferences":preferences,
        "added_preset_inventory_obligations":added.len(),"selected_issue_summary":after["selected_issue_summary"],"calculation":"not_run"})
}
#[test]
#[ignore = "requires checked incoming-input predecessor and full active-occurrence source witness"]
fn skill_usage_publication_preserves_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_USAGE_PRIOR").expect("checked predecessor"),
    );
    let out =
        PathBuf::from(std::env::var_os("POE_OPTIMIZER_TEST_USAGE_OUTPUT").expect("fresh output"));
    assert!(!out.exists());
    let inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    for kind in ["definitions", "roles", "scalar_inputs", "tree"] {
        let mut bad = next.input().clone();
        if kind == "tree" {
            bad.tree = prior.input().tree.clone();
        } else {
            let Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
                definitions,
                roles,
                scalar_inputs,
                ..
            }) = &mut bad.normalization.usage_inputs
            else {
                panic!()
            };
            match kind {
                "definitions" => *definitions = prior.assembled().schema().identity().clone(),
                "roles" => *roles = *prior.roles().identity(),
                "scalar_inputs" => *scalar_inputs = prior.receipt().input,
                _ => unreachable!(),
            }
            bad.tree.as_mut().unwrap().normalization = digest_owned(
                "owned-normalization-policy-v3",
                &bad.normalization,
                TreePolicyLimits::default().max_base_policy_bytes,
            )
            .unwrap();
        }
        assert!(
            assemble_owned_release(bad, Default::default()).is_err(),
            "stale {kind} must reject"
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
    let published = release::inventory(&package);
    assert_eq!(published, release::inventory(&rebuilt));
    assert_eq!(published.len(), inventory.len());
    let mut reports = vec![];
    for case in 1..=5 {
        let file = format!("queries-original-{case:02}.json");
        assert_eq!(published.get(&file), inventory.get(&file));
        let path = root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let xml = fs::read(&path).unwrap();
        release::normalize(
            &prior_path,
            &path,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &path,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let mut report = compare_original(case, &xml, &prior, &next, &out);
        report["source_sha256"] = json!(format!("{:x}", Sha256::digest(&xml)));
        reports.push(report);
    }
    assert_eq!(inventory, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":reports,
        "queries":110,"stale_bindings":4,"prior_unchanged":true,"rebuild_byte_identical":true,"allocated_definitions":2,
        "added_programs":1,"added_tables":0,"complete_original_builds":0}),
    );
}
