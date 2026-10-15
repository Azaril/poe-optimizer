//! Exact source-backed additions to reporting intent; no aggregate execution.
use super::{family, preservation, release, selected};
use poe_optimizer_core::{build_identity::BuildLineage, owned_draft::DraftUsageTarget};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::StagedOwnedRelease,
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};
#[path = "owned_identity_correspondence.rs"]
mod identity;

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write(path: impl AsRef<Path>, value: &impl serde::Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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

#[derive(Clone)]
struct Node {
    ordinal: usize,
    name: String,
    parent: Option<usize>,
    attributes: BTreeMap<String, String>,
}
fn frame(xml: &[u8]) -> Vec<Node> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([97; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    evidence
        .rows()
        .iter()
        .map(|row| {
            let mut attributes = BTreeMap::new();
            for a in row.attributes() {
                assert!(
                    attributes
                        .insert(a.origin().name.clone(), a.decoded().unwrap().to_owned())
                        .is_none()
                );
            }
            Node {
                ordinal: row.occurrence().id().ordinal() as usize,
                name: row.occurrence().name().into(),
                parent: row.occurrence().parent().map(|id| id.ordinal() as usize),
                attributes,
            }
        })
        .collect()
}
fn attr<'a>(row: &'a Node, key: &str) -> Option<&'a str> {
    row.attributes.get(key).map(String::as_str)
}
fn links<'a>(sidecar: &'a Value, ordinal: usize, kind: &str) -> Vec<&'a Value> {
    preservation::origin(sidecar, ordinal)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|v| v["kind"] == kind)
        .map(|v| &v["value"])
        .collect()
}
fn matches_frame(node: &Node, allowed: &Value, guards: &Value) -> bool {
    node.attributes
        .keys()
        .all(|name| allowed.as_array().unwrap().contains(&json!(name)))
        && guards.as_array().unwrap().iter().all(|g| {
            let actual = match attr(node, g["attribute"].as_str().unwrap()) {
                Some(value) => json!({"kind":"text","value":value}),
                None => json!({"kind":"missing"}),
            };
            g["allowed"].as_array().unwrap().contains(&actual)
        })
}
fn identity_matches(node: &Node, identity: &Value) -> bool {
    [
        ("gemId", "game_id"),
        ("variantId", "variant_id"),
        ("skillId", "skill_id"),
        ("nameSpec", "name_spec"),
    ]
    .iter()
    .all(|(source, field)| attr(node, source) == identity[*field].as_str())
}
fn generated_matches(group: &Node, provider: &Value) -> bool {
    match provider["kind"].as_str().unwrap() {
        "tree_allocation" => {
            attr(group, "source")
                == Some(format!("Tree:{}", provider["source_node_id"].as_str().unwrap()).as_str())
        }
        "item_modifier" => attr(group, "source")
            .and_then(|v| v.strip_prefix("Item:"))
            .and_then(|v| v.split_once(':'))
            .is_some_and(|(id, name)| {
                id.parse::<u64>().is_ok_and(|id| id > 0)
                    && Some(name) == provider["source_name"].as_str()
                    && attr(group, "slot").is_some()
            }),
        _ => panic!("unreviewed generated provider"),
    }
}
fn physical_target(sidecar: &Value, ordinal: usize, rule: &Value) -> Option<Value> {
    let ids = links(sidecar, ordinal, "skill");
    let [id] = ids.as_slice() else {
        return None;
    };
    Some(json!({"kind":"skill","value":{"kind":"generated","value":{
        "provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":id}},
            "grant_path":{"members":[],"completion":{"kind":"complete"}}},
        "slot":{"kind":"known","value":rule["supply"]}
    }}}))
}
fn occurrence_target(
    sidecar: &Value,
    ordinal: usize,
    preset: &Value,
    rule: &Value,
) -> Option<Value> {
    match rule["target"]["kind"].as_str().unwrap() {
        "authored_direct" => {
            let ids = links(sidecar, ordinal, "skill");
            let [id] = ids.as_slice() else {
                return None;
            };
            Some(
                json!({"kind":"skill","value":{"kind":"authored","value":{"kind":"known","value":id}}}),
            )
        }
        "generated" => {
            let rows = links(sidecar, ordinal, "generated_skill_input");
            let [row] = rows.as_slice() else {
                return None;
            };
            assert_eq!(row["skill_preset"], *preset);
            // Public typed draft conversion preserves the exact provider, grant
            // path and slot. It does not discover or fabricate a provider.
            let target: poe_optimizer_core::owned_build::UsageTarget =
                serde_json::from_value(json!({"kind":"skill","value":{
                    "kind":"generated","value":row["target"]
                }}))
                .unwrap();
            let draft = DraftUsageTarget::from(target);
            Some(json!(draft))
        }
        _ => panic!("unreviewed occurrence kind"),
    }
}
#[derive(Clone)]
struct Addition {
    preset: Value,
    expected: Value,
    source: usize,
    group: usize,
    set: usize,
    literal: String,
    family: &'static str,
}
fn additions(xml: &[u8], draft: &Value, sidecar: &Value) -> Vec<Addition> {
    let rows = frame(xml);
    let changes: Value = family::read("changes.json");
    let usage = json!(family::usage());
    let mut expected = vec![];
    for node in rows.iter().filter(|v| v.name == "Gem") {
        let group = &rows[node.parent.unwrap()];
        if group.name != "Skill" {
            continue;
        }
        let set = &rows[group.parent.unwrap()];
        assert_eq!(set.name, "SkillSet");
        let presets = links(sidecar, set.ordinal, "skill_preset");
        let [preset_id] = presets.as_slice() else {
            continue;
        };
        let preset = preservation::member(draft, "skill_presets", preset_id);
        let prior_preferences = preset["intent"]["usage"]["members"].as_array().unwrap();
        for family in ["physical", "occurrences"] {
            for change in changes[family].as_array().unwrap() {
                let rule = &change["before"];
                let (identity, source_allowed) = if family == "physical" {
                    (rule, attr(group, "source").is_none())
                } else if rule["target"]["kind"] == "generated" {
                    let identity = &rule["target"]["correspondence"];
                    (identity, generated_matches(group, &identity["provider"]))
                } else {
                    (&rule["target"], attr(group, "source").is_none())
                };
                if !source_allowed
                    || !identity_matches(node, identity)
                    || !matches_frame(node, &rule["attributes"], &rule["guards"])
                    || !matches_frame(group, &rule["group_attributes"], &rule["group_guards"])
                {
                    continue;
                }
                let target = if family == "physical" {
                    physical_target(sidecar, node.ordinal, rule)
                } else {
                    occurrence_target(sidecar, node.ordinal, preset_id, rule)
                };
                let Some(target) = target else {
                    continue;
                };
                // Existing import receipts are the independent eligibility
                // authority. All previous policies must bind this same target.
                let mut applicability = None;
                for policy in rule["policies"].as_array().unwrap() {
                    let found: Vec<_> = prior_preferences
                        .iter()
                        .filter(|p| {
                            p["selection"]["policy"]
                                == json!({"kind":"known","value":policy["policy"]})
                                && p["selection"]["target"] == target
                        })
                        .collect();
                    assert_eq!(
                        found.len(),
                        1,
                        "previous exact usage target at source {}",
                        node.ordinal
                    );
                    if let Some(old) = &applicability {
                        assert_eq!(old, &found[0]["applicability"]);
                    }
                    applicability = Some(found[0]["applicability"].clone());
                }
                let literal = attr(group, "includeInFullDPS")
                    .expect("unchanged originals have an explicit reviewed literal");
                let boolean = match literal {
                    "true" => true,
                    "false" | "nil" => false,
                    _ => panic!("unchanged original has unreviewed reporting literal"),
                };
                let expected_row = json!({"selection":{
                    "policy":{"kind":"known","value":usage["policy"]},"target":target,
                    "parameters":{"completion":{"kind":"complete"},"members":[{
                        "slot":{"kind":"known","value":usage["parameters"][0]["slot"]},
                        "value":{"kind":"known","value":{"kind":"boolean","value":boolean}}
                    }]}
                },"applicability":applicability.unwrap()});
                assert!(
                    !expected
                        .iter()
                        .any(|a: &Addition| a.preset == **preset_id && a.expected == expected_row),
                    "duplicate source-to-reporting target"
                );
                expected.push(Addition {
                    preset: (*preset_id).clone(),
                    expected: expected_row,
                    source: node.ordinal,
                    group: group.ordinal,
                    set: set.ordinal,
                    literal: literal.into(),
                    family,
                });
            }
        }
    }
    expected
}
fn strip_additions(draft: &mut Value, expected: &[Addition]) {
    for row in expected {
        let presets = draft["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap();
        let matching: Vec<_> = presets
            .iter_mut()
            .filter(|p| p["id"] == row.preset)
            .collect();
        let [preset] = matching.as_slice() else {
            panic!("exact reporting preset");
        };
        let members = preset["intent"]["usage"]["members"].as_array().unwrap();
        let found: Vec<_> = members
            .iter()
            .enumerate()
            .filter(|(_, p)| **p == row.expected)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(
            found.len(),
            1,
            "exact source-backed reporting preference {}",
            row.source
        );
        let preset = presets.iter_mut().find(|p| p["id"] == row.preset).unwrap();
        preset["intent"]["usage"]["members"]
            .as_array_mut()
            .unwrap()
            .remove(found[0]);
    }
    for preset in draft["draft"]["skill_presets"]["members"]
        .as_array()
        .unwrap()
    {
        assert!(
            !preset["intent"]["usage"]["members"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["selection"]["policy"]["value"]["key"] == family::POLICY),
            "unexpected reporting preference remains"
        );
    }
}
fn metadata_lineage(value: &mut Value, old: &Value, new: &Value) {
    assert_eq!(
        &value["lineage"], new,
        "exact fresh-import metadata lineage"
    );
    value["lineage"] = old.clone();
}
fn exact_ids(ids: &BTreeMap<String, Value>, old: &Value, new: &Value) {
    for (fresh, previous) in ids {
        let fresh: Value = serde_json::from_str(fresh).unwrap();
        assert_eq!(fresh["local"], previous["local"], "local IDs must not move");
        assert_eq!(&fresh["lineage"], new);
        assert_eq!(&previous["lineage"], old);
    }
}
fn compare(
    case: usize,
    xml: &[u8],
    prior: &StagedOwnedRelease,
    next: &StagedOwnedRelease,
    prior_path: &Path,
    package: &Path,
    out: &Path,
) -> Value {
    let old = out.join(format!("prior-original-{case:02}"));
    let new = out.join(format!("original-{case:02}"));
    let a = read(old.join("draft.json"));
    let mut b = read(new.join("draft.json"));
    let sa = read(old.join("sidecar.json"));
    let mut sb = read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, prior_path, case, prior);
    preservation::authenticate(&sb, &new, package, case, next);
    for sidecar in [&sa, &sb] {
        assert_eq!(
            sidecar["source_sha256"],
            format!("{:x}", Sha256::digest(xml))
        );
        assert_eq!(sidecar["source_bytes"], xml.len());
    }
    let old_lineage = a["draft"]["allocator"]["lineage"].clone();
    let new_lineage = b["draft"]["allocator"]["lineage"].clone();
    assert_eq!(sa["allocator_after"], a["draft"]["allocator"]);
    assert_eq!(sb["allocator_after"], b["draft"]["allocator"]);
    assert_eq!(
        a["draft"]["allocator"]["last_issued"],
        b["draft"]["allocator"]["last_issued"]
    );
    let old_expected = additions(xml, &a, &sa);
    let expected = additions(xml, &b, &sb);
    strip_additions(&mut b, &expected);
    b["draft"]["allocator"] = a["draft"]["allocator"].clone();
    let mut ids = BTreeMap::new();
    identity::correspond(&a, &mut b, &mut ids, "whole draft reporting inverse");
    exact_ids(&ids, &old_lineage, &new_lineage);
    assert!(
        a == b,
        "whole draft inverse: only exact reporting preferences may differ (original {case})"
    );
    // These complete commitments were authenticated above, independently on
    // both endpoints. No origin, source field, issue or link is filtered.
    for field in [
        "definitions",
        "mapping",
        "registry",
        "skill_roles",
        "reward_policy",
        "item_policy",
        "item_source_policy",
        "draft",
        "policy",
        "tree_policy",
        "allocator_after",
    ] {
        sb[field] = sa[field].clone();
    }
    for row in sb["item_texts"].as_array_mut().unwrap() {
        metadata_lineage(
            &mut row["attribution"]["source"],
            &old_lineage,
            &new_lineage,
        );
        metadata_lineage(
            &mut row["attribution"]["source"]["allocator"],
            &old_lineage,
            &new_lineage,
        );
        row["attribution"]["item_lines"] = json!(prior.receipt().items);
        row["attribution"]["policy"] = json!(prior.receipt().item_source);
    }
    for field in ["allocator_before", "source_allocator"] {
        metadata_lineage(&mut sb[field], &old_lineage, &new_lineage);
        assert_eq!(sb[field], sa[field]);
    }
    identity::correspond(&sa, &mut sb, &mut ids, "whole sidecar reporting inverse");
    exact_ids(&ids, &old_lineage, &new_lineage);
    assert!(
        sa == sb,
        "whole sidecar inverse: all source dispositions and links remain exact (original {case})"
    );
    assert_eq!(old_expected.len(), expected.len());
    for (old, new) in old_expected.iter().zip(&expected) {
        assert_eq!(
            (old.source, old.group, old.set, &old.literal, old.family),
            (new.source, new.group, new.set, &new.literal, new.family)
        );
        let mut actual = json!({"preset":new.preset,"binding":new.expected});
        identity::relocate(&mut actual, &ids);
        assert_eq!(actual, json!({"preset":old.preset,"binding":old.expected}));
    }
    let x = selected::selection(xml, &old);
    let mut y = selected::selection(xml, &new);
    identity::relocate(&mut y, &ids);
    assert_eq!(x, y);
    let before = selected::finalize_with_definitions(
        xml,
        &old,
        &out.join(format!("prior-selected-{case:02}.json")),
        &prior_path.join("schema.json"),
    );
    let after = selected::finalize_with_definitions(
        xml,
        &new,
        &out.join(format!("selected-{case:02}.json")),
        &package.join("schema.json"),
    );
    for report in [&before, &after] {
        assert_eq!(report["intent_validation"]["schema_issues"], json!([]));
    }
    let x = before["finalization"]["issues"].clone();
    let mut y = after["finalization"]["issues"].clone();
    identity::relocate(&mut y, &ids);
    assert_eq!(x, y);
    assert_eq!(
        x.as_array().unwrap().len(),
        [106, 117, 109, 123, 4][case - 1]
    );
    json!({"original":case,"new_reporting_preferences":expected.len(),"selected_issues":x.as_array().unwrap().len(),
        "exact_local_ids":true,"all_presets_preserved":true,"all_prior_preferences_preserved":true,
        "retired_issues":0,"source_origins_unchanged":true,
        "source_bindings":expected.iter().map(|a|json!({"source":a.source,"group":a.group,"set":a.set,
            "preset":a.preset,"literal":a.literal,"family":a.family,"binding":a.expected})).collect::<Vec<_>>()})
}
pub fn run(prior_path: PathBuf, out: PathBuf) {
    assert!(!out.exists(), "evidence is immutable");
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files.len(), 18);
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        prior_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    for query in prior.query_sets() {
        let name = format!("queries-{}.json", query.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&name)).unwrap(),
            fs::read(package.join(name)).unwrap()
        );
    }
    assert_eq!(
        next.receipt().provenance.len(),
        prior.receipt().provenance.len() + 1
    );
    assert_eq!(
        next.receipt().provenance[..prior.receipt().provenance.len()],
        prior.receipt().provenance
    );
    let mut originals = vec![];
    for case in 1..=5 {
        let source = family::root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        release::normalize(
            &prior_path,
            &source,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &source,
            case,
            &out.join(format!("original-{case:02}")),
        );
        originals.push(compare(
            case,
            &bytes,
            &prior,
            &next,
            &prior_path,
            &package,
            &out,
        ));
        assert_eq!(fs::read(source).unwrap(), bytes);
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,
        "artifacts":18,"queries":110,"rebuild_byte_identical":true,"prior_unchanged":true,"originals":originals,
        "new_policy":family::POLICY,"new_definitions":2,"physical_rule_appends":7,"occurrence_rule_appends":6,
        "closed_prior_owners":0,"retired_input_issues":0,"complete_original_builds":0,
        "aggregate_metric_binding":false,"evaluation_bundle_added":false,"ordinary_gameplay_effects_added":false}),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inverse_requires_exact_value_target_policy_multiplicity_and_prior_records() {
        let row = Addition {
            preset: json!(1),
            expected: json!({"selection":{"policy":{"value":{"key":family::POLICY}},
            "target":"exact-occurrence","parameters":{"value":false}},"applicability":"required"}),
            source: 2,
            group: 1,
            set: 0,
            literal: "nil".into(),
            family: "physical",
        };
        let prior = json!({"draft":{"skill_presets":{"members":[{"id":1,"intent":{"usage":{"members":[{"old":"value"}]}}}]}}});
        let mut after = prior.clone();
        after["draft"]["skill_presets"]["members"][0]["intent"]["usage"]["members"]
            .as_array_mut()
            .unwrap()
            .push(row.expected.clone());
        let mut inverse = after.clone();
        strip_additions(&mut inverse, std::slice::from_ref(&row));
        assert_eq!(inverse, prior);
        for case in 0..6 {
            let mut changed = after.clone();
            let members =
                changed["draft"]["skill_presets"]["members"][0]["intent"]["usage"]["members"]
                    .as_array_mut()
                    .unwrap();
            match case {
                0 => members[1]["selection"]["parameters"]["value"] = json!(true),
                1 => members[1]["selection"]["target"] = json!("other-occurrence"),
                2 => members[1]["selection"]["policy"]["value"]["key"] = json!("other"),
                3 => members.push(row.expected.clone()),
                4 => {
                    members.pop();
                }
                _ => members[0]["old"] = json!("changed"),
            }
            assert!(
                std::panic::catch_unwind(|| {
                    let mut changed = changed.clone();
                    strip_additions(&mut changed, std::slice::from_ref(&row));
                    assert_eq!(changed, prior);
                })
                .is_err()
            );
        }
    }
}
