//! Add count preferences to exact real occurrences without certifying inventories.
#[path = "support/owned_skeletal_counts.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::{build_identity::BuildLineage, owned_content::digest_owned};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::PrimarySkillUsageInput,
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
    ops::Range,
    path::{Path, PathBuf},
    process::Command,
};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn write(p: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(p, serde_json::to_vec(value).unwrap()).unwrap();
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
struct Node {
    ordinal: usize,
    name: String,
    parent: Option<usize>,
    attrs: Vec<(String, String)>,
    range: Range<usize>,
}
impl Node {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}
fn frame(xml: &[u8]) -> Vec<Node> {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([95; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let evidence =
        SourceProjectEvidence::collect(&imported, SourceEvidenceLimits::default()).unwrap();
    evidence
        .rows()
        .iter()
        .map(|r| Node {
            ordinal: r.occurrence().id().ordinal() as usize,
            name: r.occurrence().name().to_owned(),
            parent: r.occurrence().parent().map(|id| id.ordinal() as usize),
            attrs: r
                .attributes()
                .iter()
                .map(|a| (a.origin().name.clone(), a.decoded().unwrap().to_owned()))
                .collect(),
            range: r.occurrence().range(),
        })
        .collect()
}
fn active_set(rows: &[Node]) -> &Node {
    let container = rows.iter().find(|r| r.name == "Skills").unwrap();
    rows.iter()
        .find(|r| {
            r.parent == Some(container.ordinal)
                && r.name == "SkillSet"
                && r.attr("id") == container.attr("activeSkillSet")
        })
        .unwrap()
}
fn matches(row: &Node, rule: &PrimarySkillUsageInput) -> bool {
    row.name == "Gem"
        && row.attr("gemId") == Some(&rule.game_id)
        && row.attr("variantId") == Some(&rule.variant_id)
}
fn target(side: &Value, source: usize, rule: &PrimarySkillUsageInput) -> Value {
    json!({"kind":"skill","value":{"kind":"generated","value":{
        "provider":{"root":{"kind":"skill_use","value":{"kind":"known","value":preservation::link(side,source,"skill")}},"grant_path":{"members":[],"completion":{"kind":"complete"}}},
        "slot":{"kind":"known","value":rule.supply}
    }}})
}
fn preferences<'a>(
    draft: &'a Value,
    side: &Value,
    rows: &[Node],
    source: &Node,
    rule: &PrimarySkillUsageInput,
) -> (&'a Value, &'a Value) {
    let group = &rows[source.parent.unwrap()];
    let set = &rows[group.parent.unwrap()];
    assert_eq!(set.name, "SkillSet");
    assert!(group.attr("source").is_none());
    let preset = preservation::link(side, set.ordinal, "skill_preset");
    for ordinal in [source.ordinal, group.ordinal] {
        assert_eq!(preservation::link(side, ordinal, "skill_preset"), preset);
    }
    let p = preservation::member(draft, "skill_presets", &preset);
    let t = target(side, source.ordinal, rule);
    let matching: Vec<_> = p["usage_preferences"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|u| {
            u["policy"] == json!({"kind":"known","value":rule.policies[0].policy})
                && u["target"] == t
        })
        .collect();
    assert_eq!(matching.len(), 1, "one exact supplied-skill preference");
    assert_eq!(
        p["usage_preferences"]["completion"]["code"],
        "usage-preferences-not-converted"
    );
    (p, matching[0])
}
fn assert_count(pref: &Value, rule: &PrimarySkillUsageInput, value: Option<i64>) {
    if let Some(value) = value {
        assert_eq!(
            pref["parameters"],
            json!({"completion":{"kind":"complete"},"members":[{
                "slot":{"kind":"known","value":rule.policies[0].parameters[0].slot},"value":{"kind":"known","value":{"kind":"integer","value":value}}
            }]})
        );
    } else {
        assert_eq!(
            pref["parameters"]["completion"]["code"],
            "usage-parameters-not-converted"
        );
        assert!(
            pref["parameters"]["members"].as_array().unwrap().is_empty(),
            "no invented fallback"
        );
    }
}

struct Comparison<'a> {
    prior: &'a StagedOwnedRelease,
    next: &'a StagedOwnedRelease,
    prior_path: &'a Path,
    package: &'a Path,
    out: &'a Path,
    rules: &'a [PrimarySkillUsageInput],
}
fn compare_original(case: usize, xml: &[u8], c: &Comparison<'_>) -> Value {
    let source = c.out.join(format!("original-{case:02}.xml"));
    fs::write(&source, xml).unwrap();
    let old = c.out.join(format!("prior-original-{case:02}"));
    let new = c.out.join(format!("original-{case:02}"));
    release::normalize(c.prior_path, &source, case, &old);
    release::normalize(c.package, &source, case, &new);
    let mut a: Value = read(old.join("draft.json"));
    let mut b: Value = read(new.join("draft.json"));
    let mut sa: Value = read(old.join("sidecar.json"));
    let mut sb: Value = read(new.join("sidecar.json"));
    preservation::authenticate(&sa, &old, c.prior_path, case, c.prior);
    preservation::authenticate(&sb, &new, c.package, case, c.next);
    for side in [&sa, &sb] {
        assert_eq!(side["source_sha256"], format!("{:x}", Sha256::digest(xml)));
        assert_eq!(side["source_bytes"], xml.len());
    }
    let rows = frame(xml);
    let mut additions = Vec::new();
    let mut contributing = BTreeSet::new();
    let mut counts = vec![0usize; c.rules.len()];
    // Existing consumers retain their exact records. Include their source row
    // only to authenticate the possible issue/preset link-order exchange.
    for preset in a["draft"]["skill_presets"]["members"].as_array().unwrap() {
        if let Some(usage) = preset.get("usage_preferences") {
            for pref in usage["members"].as_array().unwrap().iter().filter(|p| {
                p["policy"] == json!({"kind":"known","value":c.rules[0].policies[0].policy})
            }) {
                let skill = &pref["target"]["value"]["value"]["provider"]["root"]["value"]["value"];
                let link = json!({"kind":"skill","value":skill});
                let sources: Vec<_> = rows
                    .iter()
                    .filter(|r| {
                        r.name == "Gem"
                            && preservation::origin(&sa, r.ordinal)["links"]
                                .as_array()
                                .unwrap()
                                .contains(&link)
                    })
                    .collect();
                assert_eq!(sources.len(), 1);
                contributing.insert(sources[0].ordinal);
            }
        }
    }
    for (index, rule) in c.rules.iter().enumerate() {
        for row in rows.iter().filter(|r| matches(r, rule)) {
            let group = &rows[row.parent.unwrap()];
            let (preset, pref) = preferences(&b, &sb, &rows, row, rule);
            // Every original saved copy is independently observed at count one.
            assert_eq!(row.attr("count"), Some("1"));
            assert!(group.attr("groupCount").is_none());
            assert_count(pref, rule, Some(1));
            additions.push((preset["id"].clone(), pref.clone()));
            contributing.extend([row.ordinal, group.ordinal]);
            counts[index] += 1;
        }
    }
    assert_eq!(
        counts,
        match case {
            1 => vec![1, 1, 1],
            5 => vec![5, 3, 4],
            _ => vec![0, 0, 0],
        }
    );
    let mut selected_usage = 0;
    for (preset, pref) in &additions {
        let owner = b["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|v| v["id"] == *preset)
            .unwrap();
        let members = owner["usage_preferences"]["members"]
            .as_array_mut()
            .unwrap();
        let before = members.len();
        members.retain(|v| v != pref);
        assert_eq!(before - members.len(), 1);
    }
    if case == 5 {
        let s = preservation::link(&sb, active_set(&rows).ordinal, "skill_preset");
        selected_usage =
            preservation::member(&b, "skill_presets", &s)["usage_preferences"]["members"]
                .as_array()
                .unwrap()
                .len();
        assert_eq!(
            selected_usage, 3,
            "the existing Sniper/Offering/Frost Bomb records survive"
        );
    }
    for v in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(v);
    }
    assert_eq!(
        a["draft"]["allocator"], b["draft"]["allocator"],
        "known counts allocate no new issues"
    );
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    let mut ids = BTreeMap::new();
    identity::correspond(&a, &mut b, &mut ids, "only exact count preferences change");
    let moved = ids
        .iter()
        .filter(|(key, value)| serde_json::from_str::<Value>(key).unwrap() != **value)
        .count();
    assert_eq!(
        moved,
        if case == 5 { 12 } else { 0 },
        "only three earlier usage-issue allocation positions move"
    );
    let actual_moves: BTreeMap<String, String> = ids
        .iter()
        .filter(|(key, value)| serde_json::from_str::<Value>(key).unwrap() != **value)
        .map(|(key, value)| {
            let new: Value = serde_json::from_str(key).unwrap();
            (
                new["local"].as_str().unwrap().into(),
                value["local"].as_str().unwrap().into(),
            )
        })
        .collect();
    let mut expected_moves = BTreeMap::new();
    if case == 5 {
        for first in [0x4f0_u64, 0x503, 0x52b] {
            expected_moves.insert(format!("{first:016x}"), format!("{:016x}", first + 4));
            // The source materializer consumes one temporary issue ID between
            // the physical Gem and SkillUse. Its resolved obligation has no
            // surviving draft or provenance reference, so it is not a live ID
            // correspondence. Keep the allocator watermark check above.
            for old in [first, first + 1, first + 3] {
                expected_moves.insert(format!("{:016x}", old + 1), format!("{old:016x}"));
            }
        }
    }
    assert_eq!(
        actual_moves, expected_moves,
        "source-audited issue timing is the only relocation"
    );
    identity::relocate(&mut sb, &ids);
    for ordinal in contributing {
        // New consumers may insert an existing same-preset issue before its
        // preset link. All non-usage provenance keeps its exact order.
        let parent = if rows[ordinal].name == "Gem" {
            rows[ordinal].parent.unwrap()
        } else {
            ordinal
        };
        let set = rows[parent].parent.unwrap();
        let preset = preservation::link(&sa, set, "skill_preset");
        let issue=preservation::member(&a,"skill_presets",&preset)["usage_preferences"]["completion"]["id"].clone();
        for kind in ["skill_preset", "issue"] {
            let wanted = json!({"kind":kind,"value":if kind=="issue" {&issue} else {&preset}});
            let old_links = preservation::origin_mut(&mut sa, ordinal)["links"]
                .as_array_mut()
                .unwrap();
            let old_count = old_links.iter().filter(|v| **v == wanted).count();
            old_links.retain(|v| *v != wanted);
            let new_links = preservation::origin_mut(&mut sb, ordinal)["links"]
                .as_array_mut()
                .unwrap();
            let new_count = new_links.iter().filter(|v| **v == wanted).count();
            new_links.retain(|v| *v != wanted);
            assert_eq!(
                old_count, 1,
                "existing disposition already proves exact usage provenance"
            );
            assert_eq!(new_count, 1, "no duplicate or lost usage provenance");
        }
    }
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert_eq!(sa, sb, "all unrelated source links and commitments survive");
    let before = selected::finalize(
        xml,
        &old,
        &c.out
            .join(format!("prior-original-{case:02}-selection.json")),
    );
    let after = selected::finalize(
        xml,
        &new,
        &c.out.join(format!("original-{case:02}-selection.json")),
    );
    write(
        c.out
            .join(format!("original-{case:02}-selected-report.json")),
        &after,
    );
    let mut old_issues = before["finalization"]["issues"].clone();
    let mut new_issues = after["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    identity::relocate(&mut new_issues, &ids);
    assert_eq!(
        old_issues, new_issues,
        "count transport closes no inventory"
    );
    assert_eq!(
        new_issues.as_array().unwrap().len(),
        [113, 116, 108, 121, 11][case - 1]
    );
    let mut old_selection = selected::selection(xml, &old);
    let mut new_selection = selected::selection(xml, &new);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    identity::relocate(&mut new_selection, &ids);
    assert_eq!(old_selection, new_selection);
    json!({"original":case,"source_sha256":format!("{:x}",Sha256::digest(xml)),"preferences_added":additions.len(),"family_counts":counts,"allocator_unchanged":true,"relocated_local_ids":moved,"whole_graph_preserved":true,"whole_sidecar_preserved":true,"selected_unresolved":new_issues.as_array().unwrap().len(),"prior_selected_usage":selected_usage})
}

fn change_header(xml: &str, row: &Node, changes: &[(&str, Option<&str>)]) -> String {
    let end = row.range.start + xml[row.range.start..].find('>').unwrap() + 1;
    let mut header = preservation::header(&row.name, &row.attrs, changes);
    if xml[row.range.start..end].ends_with("/>") {
        header.insert(header.len() - 1, '/');
    }
    let mut out = xml.to_owned();
    out.replace_range(row.range.start..end, &header);
    out
}
struct Control {
    name: String,
    family: usize,
    set: String,
    xml: String,
    value: Option<i64>,
    occurrences: usize,
}
fn controls(c: &Comparison<'_>) -> Vec<Value> {
    let path = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    let xml = fs::read_to_string(&path).unwrap();
    let rows = frame(xml.as_bytes());
    let active = active_set(&rows);
    let mut cases = vec![];
    for (index, rule) in c.rules.iter().enumerate() {
        let row = rows
            .iter()
            .find(|r| matches(r, rule) && rows[r.parent.unwrap()].parent == Some(active.ordinal))
            .unwrap();
        let group = &rows[row.parent.unwrap()];
        let with_three = change_header(&xml, row, &[("count", Some("3"))]);
        for (label, body, value) in [
            ("count-three", with_three.clone(), Some(3)),
            (
                "group-zero",
                change_header(&with_three, group, &[("groupCount", Some("0"))]),
                Some(0),
            ),
            (
                "group-four",
                change_header(&with_three, group, &[("groupCount", Some("4"))]),
                Some(4),
            ),
            (
                "malformed-override",
                change_header(&with_three, group, &[("groupCount", Some("bad"))]),
                None,
            ),
        ] {
            cases.push(Control {
                name: format!("family-{index}-{label}"),
                family: index,
                set: active.attr("id").unwrap().into(),
                xml: body,
                value,
                occurrences: 1,
            });
        }
        if index != 0 {
            continue;
        }
        for (label, value) in [
            ("missing", None),
            ("malformed", Some("bad")),
            ("fractional", Some("1.5")),
            ("negative", Some("-1")),
            ("out-of-range", Some("5")),
        ] {
            cases.push(Control {
                name: format!("count-{label}"),
                family: index,
                set: active.attr("id").unwrap().into(),
                xml: change_header(&xml, row, &[("count", value)]),
                value: None,
                occurrences: 1,
            });
        }
        let sibling = rows
            .iter()
            .find(|r| {
                r.name == "Gem" && r.parent == Some(group.ordinal) && r.ordinal != row.ordinal
            })
            .unwrap();
        cases.push(Control {
            name: "unreviewed-companion-tuple".into(),
            family: index,
            set: active.attr("id").unwrap().into(),
            xml: change_header(
                &xml,
                sibling,
                &[("nameSpec", Some("Unreviewed support tuple"))],
            ),
            value: None,
            occurrences: 1,
        });
        for (label, disabled, override_count, expected) in [
            ("duplicate-primary", false, false, None),
            ("disabled-duplicate-primary", true, false, None),
            ("duplicate-primary-group-override", false, true, Some(2)),
        ] {
            let copy = if disabled {
                change_header(&xml, row, &[("enabled", Some("false"))])
            } else {
                xml.clone()
            };
            let copied_rows = frame(copy.as_bytes());
            let entry = &copied_rows[row.ordinal];
            let mut body = xml.clone();
            body.insert_str(row.range.end, &copy[entry.range.clone()]);
            if override_count {
                body = change_header(&body, group, &[("groupCount", Some("2"))]);
            }
            cases.push(Control {
                name: label.into(),
                family: index,
                set: active.attr("id").unwrap().into(),
                xml: body,
                value: expected,
                occurrences: 2,
            });
        }
        let archived = rows
            .iter()
            .find(|r| matches(r, rule) && rows[r.parent.unwrap()].parent != Some(active.ordinal))
            .unwrap();
        let archived_set = &rows[rows[archived.parent.unwrap()].parent.unwrap()];
        cases.push(Control {
            name: "archived-count-edit".into(),
            family: index,
            set: archived_set.attr("id").unwrap().into(),
            xml: change_header(&xml, archived, &[("count", Some("3"))]),
            value: Some(3),
            occurrences: 1,
        });
    }
    let original: Value = read(c.out.join("original-05/draft.json"));
    let original_side: Value = read(c.out.join("original-05/sidecar.json"));
    let mut reports = vec![];
    for case in cases {
        assert_ne!(case.xml, xml);
        let name = format!("control-{}", case.name);
        let source = c.out.join(format!("{name}.xml"));
        fs::write(&source, &case.xml).unwrap();
        let directory = c.out.join(&name);
        release::normalize(c.package, &source, 5, &directory);
        let draft: Value = read(directory.join("draft.json"));
        let side: Value = read(directory.join("sidecar.json"));
        preservation::authenticate(&side, &directory, c.package, 5, c.next);
        assert_eq!(
            side["source_sha256"],
            format!("{:x}", Sha256::digest(case.xml.as_bytes()))
        );
        let changed = frame(case.xml.as_bytes());
        let touched = changed
            .iter()
            .find(|r| r.name == "SkillSet" && r.attr("id") == Some(&case.set))
            .unwrap();
        let rule = &c.rules[case.family];
        let targets: Vec<_> = changed
            .iter()
            .filter(|r| {
                matches(r, rule) && changed[r.parent.unwrap()].parent == Some(touched.ordinal)
            })
            .collect();
        assert_eq!(targets.len(), case.occurrences);
        let mut target_ids = BTreeSet::new();
        for row in &targets {
            let (_, pref) = preferences(&draft, &side, &changed, row, rule);
            assert_count(pref, rule, case.value);
            assert!(
                target_ids.insert(pref["target"].to_string()),
                "repeated definitions stay independent"
            );
        }
        let mut ids = BTreeMap::new();
        let old_set = rows
            .iter()
            .find(|r| r.name == "SkillSet" && r.attr("id") == Some(&case.set))
            .unwrap();
        let old_id = preservation::link(&original_side, old_set.ordinal, "skill_preset");
        let new_id = preservation::link(&side, touched.ordinal, "skill_preset");
        let mut old_usage =
            preservation::member(&original, "skill_presets", &old_id)["usage_preferences"].clone();
        let mut new_usage =
            preservation::member(&draft, "skill_presets", &new_id)["usage_preferences"].clone();
        let old_targets: Vec<_> = rows
            .iter()
            .filter(|r| matches(r, rule) && rows[r.parent.unwrap()].parent == Some(old_set.ordinal))
            .map(|r| target(&original_side, r.ordinal, rule))
            .collect();
        let new_targets: Vec<_> = targets
            .iter()
            .map(|r| target(&side, r.ordinal, rule))
            .collect();
        for (usage, targets) in [
            (&mut old_usage, &old_targets),
            (&mut new_usage, &new_targets),
        ] {
            let members = usage["members"].as_array_mut().unwrap();
            let before = members.len();
            members.retain(|u| !targets.contains(&u["target"]));
            assert_eq!(
                before - members.len(),
                targets.len(),
                "strip only intended target records"
            );
        }
        selected::canonical(&mut old_usage);
        selected::canonical(&mut new_usage);
        identity::correspond(
            &old_usage,
            &mut new_usage,
            &mut ids,
            "same-preset group overrides never leak to other skill occurrences",
        );
        let mut preserved = 0;
        for old_set in rows
            .iter()
            .filter(|r| r.name == "SkillSet" && r.attr("id") != Some(&case.set))
        {
            let new_set = changed
                .iter()
                .find(|r| r.name == "SkillSet" && r.attr("id") == old_set.attr("id"))
                .unwrap();
            let old_id = preservation::link(&original_side, old_set.ordinal, "skill_preset");
            let new_id = preservation::link(&side, new_set.ordinal, "skill_preset");
            let mut a = preservation::member(&original, "skill_presets", &old_id).clone();
            let mut b = preservation::member(&draft, "skill_presets", &new_id).clone();
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            identity::correspond(
                &a,
                &mut b,
                &mut ids,
                "every untouched preset retains its complete saved intent",
            );
            preserved += 1;
        }
        assert_eq!(preserved, 5);
        reports.push(json!({"name":case.name,"target_family":case.family,"source_preset":case.set,"typed_count":case.value,"occurrences":case.occurrences,"untouched_presets_preserved":preserved,"usage_inventory":"pending"}));
    }
    assert_eq!(fs::read_to_string(path).unwrap(), xml);
    reports
}

#[test]
fn count_packet_reuses_the_existing_semantic_consumer() {
    family::check_authored();
}

#[test]
#[ignore = "requires checked membership package and authenticated count/occurrence source reports"]
fn publish_skeletal_counts_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_COUNTS_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKELETAL_COUNTS_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let before = release::inventory(&prior_path);
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
    let after = release::inventory(&package);
    assert_eq!(after.len(), 18);
    assert_eq!(after, release::inventory(&out.join("rebuilt")));
    assert_eq!(
        before
            .iter()
            .filter(|(name, hash)| after.get(*name) != Some(*hash))
            .map(|(name, _)| name.as_str())
            .collect::<Vec<_>>(),
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
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
            "stale {field}"
        );
    }
    let rules = family::usages();
    let c = Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        rules: &rules,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        originals.push(compare_original(case, &xml, &c));
    }
    let controls = controls(&c);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"complete_original_builds":0,"usage_inventory":"pending","reservation_totals":"not_validated"}),
    );
}
