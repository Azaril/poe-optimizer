//! Authored-root list closure changes no members, values, or unrelated obligations.
#[path = "support/owned_skill_membership.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;

use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_release::{StagedOwnedRelease, assemble_owned_release},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    ops::Range,
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
fn authored_membership_packet_has_no_new_mechanics_or_defaults() {
    family::check_authored();
}

struct Node {
    ordinal: usize,
    name: String,
    parent: Option<usize>,
    attrs: Vec<(String, String)>,
    range: Range<usize>,
}
impl Node {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }
}
fn frame(xml: &[u8]) -> Vec<Node> {
    let imported = ImportedBuildInstance::from_decoded(
        decode_build(xml).unwrap(),
        BuildLineage::from_bytes([94; 16]),
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
            parent: r.occurrence().parent().map(|p| p.ordinal() as usize),
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
    let skills = rows.iter().find(|r| r.name == "Skills").unwrap();
    rows.iter()
        .find(|r| {
            r.parent == Some(skills.ordinal)
                && r.name == "SkillSet"
                && r.attr("id") == skills.attr("activeSkillSet")
        })
        .unwrap()
}
fn links<'a>(sidecar: &'a Value, ordinal: usize, kind: &str) -> Vec<&'a Value> {
    preservation::origin(sidecar, ordinal)["links"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|link| link["kind"] == kind)
        .map(|link| &link["value"])
        .collect()
}
fn prove_root_ownership(
    rows: &[Node],
    set: &Node,
    draft: &Value,
    sidecar: &Value,
    members: &Value,
) {
    let mut seen = BTreeSet::new();
    for id in members.as_array().unwrap() {
        assert!(seen.insert(id["local"].as_str().unwrap()));
        let sources: Vec<_> = rows
            .iter()
            .filter(|r| r.name == "Gem" && links(sidecar, r.ordinal, "skill").contains(&id))
            .collect();
        assert_eq!(
            sources.len(),
            1,
            "one exact physical or Direct source per authored root"
        );
        let source = sources[0];
        let group = &rows[source.parent.unwrap()];
        assert_eq!(group.name, "Skill");
        assert_eq!(group.parent, Some(set.ordinal));
        assert!(
            group.attr("source").is_none(),
            "generated representations are not authored roots"
        );
        let skill = preservation::member(draft, "skills", id);
        match skill["source"]["kind"].as_str().unwrap() {
            "gem" => assert_eq!(
                links(sidecar, source.ordinal, "gem"),
                [&skill["source"]["value"]["value"]]
            ),
            "direct" => assert!(
                links(sidecar, source.ordinal, "gem").is_empty(),
                "Direct does not fabricate a physical Gem"
            ),
            other => panic!("unproved authored root kind {other}"),
        }
    }
    for group in rows
        .iter()
        .filter(|r| r.parent == Some(set.ordinal) && r.attr("source").is_some())
    {
        for source in rows.iter().filter(|r| r.parent == Some(group.ordinal)) {
            assert!(links(sidecar, source.ordinal, "gem").is_empty());
            assert!(links(sidecar, source.ordinal, "skill").is_empty());
            assert!(links(sidecar, source.ordinal, "support").is_empty());
        }
    }
}
struct Comparison<'a> {
    prior: &'a StagedOwnedRelease,
    next: &'a StagedOwnedRelease,
    prior_path: &'a Path,
    package: &'a Path,
    out: &'a Path,
}
fn compare(label: &str, case: usize, xml: &[u8], c: &Comparison<'_>) -> Value {
    let source = c.out.join(format!("{label}.xml"));
    fs::write(&source, xml).unwrap();
    let old_dir = c.out.join(format!("prior-{label}"));
    let new_dir = c.out.join(label);
    release::normalize(c.prior_path, &source, case, &old_dir);
    release::normalize(c.package, &source, case, &new_dir);
    let mut a: Value = read(old_dir.join("draft.json"));
    let mut b: Value = read(new_dir.join("draft.json"));
    let mut sa: Value = read(old_dir.join("sidecar.json"));
    let mut sb: Value = read(new_dir.join("sidecar.json"));
    preservation::authenticate(&sa, &old_dir, c.prior_path, case, c.prior);
    preservation::authenticate(&sb, &new_dir, c.package, case, c.next);
    for sidecar in [&sa, &sb] {
        assert_eq!(
            sidecar["source_sha256"],
            format!("{:x}", Sha256::digest(xml))
        );
        assert_eq!(sidecar["source_bytes"], xml.len());
    }
    for value in [&mut a, &mut b, &mut sa, &mut sb] {
        selected::canonical(value);
    }
    assert_eq!(a["draft"]["allocator"], b["draft"]["allocator"]);
    assert_eq!(sa["allocator_after"], sb["allocator_after"]);
    let rows = frame(xml);
    let selected_set = active_set(&rows).ordinal;
    let mut retired = BTreeSet::new();
    let mut completed = Vec::new();
    let mut selected_complete = false;
    for set in rows.iter().filter(|r| r.name == "SkillSet") {
        let id = preservation::link(&sa, set.ordinal, "skill_preset");
        assert_eq!(preservation::link(&sb, set.ordinal, "skill_preset"), id);
        let before = preservation::member(&a, "skill_presets", &id);
        let after = preservation::member(&b, "skill_presets", &id);
        assert_eq!(before["skills"]["members"], after["skills"]["members"]);
        if set.ordinal == selected_set {
            selected_complete = after["skills"]["completion"]["kind"] == "complete";
        }
        if before["skills"]["completion"] == after["skills"]["completion"] {
            continue;
        }
        assert_eq!(before["skills"]["completion"]["kind"], "pending");
        assert_eq!(
            before["skills"]["completion"]["code"],
            "skill-membership-not-converted"
        );
        assert_eq!(after["skills"]["completion"], json!({"kind":"complete"}));
        prove_root_ownership(&rows, set, &b, &sb, &after["skills"]["members"]);
        let issue = before["skills"]["completion"]["id"].clone();
        assert!(retired.insert(issue["local"].as_str().unwrap().to_owned()));
        let expected = json!({"kind":"issue","value":issue});
        let sources: Vec<_> = sa["origins"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["links"].as_array().unwrap().contains(&expected))
            .collect();
        assert_eq!(
            sources.len(),
            1,
            "membership has one exact reserved provenance link"
        );
        assert_eq!(sources[0]["source"]["ordinal"], set.ordinal);
        let links = preservation::origin_mut(&mut sa, set.ordinal)["links"]
            .as_array_mut()
            .unwrap();
        let count = links.len();
        links.retain(|link| *link != expected);
        assert_eq!(count - links.len(), 1);
        let p = a["draft"]["skill_presets"]["members"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|p| p["id"] == id)
            .unwrap();
        p["skills"]["completion"] = json!({"kind":"complete"});
        completed.push(json!({"source_preset":set.attr("id").unwrap(),"source_ordinal":set.ordinal,"root_count":p["skills"]["members"].as_array().unwrap().len(),"selected":set.ordinal==selected_set}));
    }
    assert!(
        a == b,
        "{label}: only exact authored membership completions change; all IDs, values, roots, queries, usage and other issues survive"
    );
    for field in ["draft", "policy", "tree_policy"] {
        sb[field] = sa[field].clone();
    }
    assert!(
        sa == sb,
        "{label}: only authenticated commitments and the exact retired SkillSet issue link change"
    );
    let mut old_selection = selected::selection(xml, &old_dir);
    let mut new_selection = selected::selection(xml, &new_dir);
    selected::canonical(&mut old_selection);
    selected::canonical(&mut new_selection);
    assert_eq!(old_selection, new_selection);
    let old = selected::finalize(
        xml,
        &old_dir,
        &c.out.join(format!("prior-{label}-selected.json")),
    );
    let new = selected::finalize(xml, &new_dir, &c.out.join(format!("{label}-selected.json")));
    for (report, directory) in [(&old, &old_dir), (&new, &new_dir)] {
        let sidecar: Value = read(directory.join("sidecar.json"));
        assert_eq!(report["draft_digest"], sidecar["draft"]);
        assert_eq!(report["finalization"]["draft_digest"], sidecar["draft"]);
    }
    let mut old_issues = old["finalization"]["issues"].clone();
    let mut new_issues = new["finalization"]["issues"].clone();
    selected::canonical(&mut old_issues);
    selected::canonical(&mut new_issues);
    let before = old_issues.as_array().unwrap().len();
    old_issues
        .as_array_mut()
        .unwrap()
        .retain(|issue| !retired.contains(issue["id"]["local"].as_str().unwrap()));
    assert_eq!(
        old_issues, new_issues,
        "all unrelated selected obligations survive"
    );
    json!({"name":label,"completed_presets":completed,"selected_complete":selected_complete,"selected_before":before,"selected_after":new_issues.as_array().unwrap().len(),"exact_local_ids":true,"whole_graph_preserved":true,"whole_sidecar_preserved":true,"source_sha256":format!("{:x}",Sha256::digest(xml))})
}
fn change_header(xml: &str, node: &Node, changes: &[(&str, Option<&str>)]) -> String {
    let start = node.range.start;
    let end = start + xml[start..].find('>').unwrap() + 1;
    let mut header = preservation::header(&node.name, &node.attrs, changes);
    if xml[start..end].ends_with("/>") {
        header.insert(header.len() - 1, '/');
    }
    let mut changed = xml.to_owned();
    changed.replace_range(start..end, &header);
    changed
}
fn controls(c: &Comparison<'_>) -> Vec<Value> {
    let original =
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap();
    let rows = frame(original.as_bytes());
    let active = active_set(&rows).ordinal;
    let inventory = json!(family::policy());
    let firebolt = &inventory["generated_groups"][2];
    assert_eq!(firebolt["source"], "item_grant");
    let generated = rows
        .iter()
        .find(|g| {
            g.name == "Skill"
                && g.parent == Some(active)
                && g.attr("source").is_some_and(|s| s.starts_with("Item:"))
                && rows.iter().any(|gem| {
                    gem.parent == Some(g.ordinal)
                        && gem.name == "Gem"
                        && gem.attr("gemId") == firebolt["game_id"].as_str()
                        && gem.attr("variantId") == firebolt["variant_id"].as_str()
                        && gem.attr("skillId") == firebolt["skill_id"].as_str()
                })
        })
        .unwrap();
    let authored = rows
        .iter()
        .find(|g| g.name == "Skill" && g.parent == Some(active) && g.attr("source").is_none())
        .unwrap();
    let archived = rows
        .iter()
        .find(|r| r.name == "SkillSet" && r.attr("id") == Some("2"))
        .unwrap();
    let archived_generated = rows
        .iter()
        .find(|g| {
            g.name == "Skill" && g.parent == Some(archived.ordinal) && g.attr("source").is_some()
        })
        .unwrap();
    let mut duplicate = original.clone();
    duplicate.insert_str(authored.range.end, &original[authored.range.clone()]);
    let cases = [
        (
            "disabled-authored-group",
            change_header(&original, authored, &[("enabled", Some("false"))]),
            true,
        ),
        ("duplicate-authored-occurrence", duplicate, true),
        (
            "unknown-source",
            change_header(
                &original,
                generated,
                &[("source", Some("Unreviewed:Firebolt"))],
            ),
            false,
        ),
        (
            "empty-source",
            change_header(&original, generated, &[("source", Some(""))]),
            false,
        ),
        (
            "manual-firebolt",
            change_header(&original, generated, &[("source", None)]),
            true,
        ),
        (
            "stale-generated-provider",
            change_header(
                &original,
                generated,
                &[("source", Some("Item:9999:Unmatched staff"))],
            ),
            true,
        ),
        (
            "archived-unknown-source",
            change_header(
                &original,
                archived_generated,
                &[("source", Some("Unreviewed:Archive"))],
            ),
            true,
        ),
    ];
    let mut reports = Vec::new();
    for (name, xml, expected) in cases {
        assert_ne!(xml, original);
        let report = compare(&format!("control-{name}"), 5, xml.as_bytes(), c);
        assert_eq!(report["selected_complete"], expected, "{name}");
        if name == "archived-unknown-source" {
            assert_eq!(report["completed_presets"].as_array().unwrap().len(), 1);
        }
        if matches!(name, "duplicate-authored-occurrence" | "manual-firebolt") {
            let selected = report["completed_presets"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["selected"] == true)
                .unwrap();
            assert_eq!(
                selected["root_count"], 10,
                "the added physical occurrence has an independent authored root"
            );
        }
        if name == "manual-firebolt" {
            // The prior checked role is Physical. Removing source ownership
            // legitimately creates a manual Gem and SkillUse; the source
            // witness separately retains the item-generated runtime effect.
            let edited = frame(xml.as_bytes());
            let row = edited
                .iter()
                .find(|r| {
                    r.name == "Gem"
                        && r.attr("gemId") == firebolt["game_id"].as_str()
                        && r.parent == Some(generated.ordinal)
                })
                .unwrap();
            let dir = c.out.join("control-manual-firebolt");
            let draft: Value = read(dir.join("draft.json"));
            let sidecar: Value = read(dir.join("sidecar.json"));
            let gem_id = preservation::link(&sidecar, row.ordinal, "gem");
            let skill_id = preservation::link(&sidecar, row.ordinal, "skill");
            let gem = preservation::member(&draft, "gems", &gem_id);
            assert_eq!(
                gem["definition"],
                json!({"kind":"known","value":firebolt["gem"]})
            );
            assert_eq!(
                gem["parameters"]["completion"]["code"],
                "gem-parameters-not-converted"
            );
            assert_eq!(
                preservation::member(&draft, "skills", &skill_id)["source"],
                json!({"kind":"gem","value":{"kind":"known","value":gem_id}})
            );
            assert!(edited[row.parent.unwrap()].attr("source").is_none());
        }
        reports.push(report);
    }
    assert_eq!(
        fs::read_to_string(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
            .unwrap(),
        original
    );
    reports
}

#[test]
#[ignore = "requires exact Djinn release and authenticated complete ownership source reports"]
fn publish_authored_skill_membership_preserving_five_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKILL_MEMBERSHIP_PRIOR").expect("exact predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKILL_MEMBERSHIP_OUTPUT")
            .expect("new output directory"),
    );
    assert!(!out.exists());
    let before = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("prior-receipt.json"), prior.receipt());
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
    let changed: Vec<_> = before
        .iter()
        .filter(|(name, hash)| after.get(*name) != Some(*hash))
        .map(|(name, _)| name.as_str())
        .collect();
    assert_eq!(
        changed,
        [
            "normalization.json",
            "release.json",
            "tree-normalization.json"
        ]
    );
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    for query in prior.query_sets() {
        let name = format!("queries-{}.json", query.name.as_str());
        assert_eq!(
            fs::read(prior_path.join(&name)).unwrap(),
            fs::read(package.join(&name)).unwrap()
        );
    }
    for field in ["mapping_source", "roles", "direct_inputs"] {
        let mut stale = json!(next.input());
        stale["normalization"]["skill_inventory"][field] = json!("0".repeat(64));
        assert!(
            assemble_owned_release(serde_json::from_value(stale).unwrap(), Default::default())
                .is_err(),
            "stale {field}"
        );
    }
    let c = Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
    };
    let mut originals = Vec::new();
    for case in 1..=5 {
        let xml = fs::read(root().join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        )))
        .unwrap();
        let report = compare(&format!("original-{case:02}"), case, &xml, &c);
        assert_eq!(
            report["selected_before"],
            [113, 116, 108, 121, 12][case - 1]
        );
        assert_eq!(report["selected_after"], [113, 116, 108, 121, 11][case - 1]);
        assert_eq!(
            report["completed_presets"].as_array().unwrap().len(),
            usize::from(case == 5) * 2
        );
        if case == 5 {
            assert_eq!(
                report["completed_presets"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| p["source_preset"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                ["2", "4"]
            );
            assert_eq!(report["completed_presets"][1]["root_count"], 9);
        }
        originals.push(report);
    }
    let controls = controls(&c);
    assert_eq!(before, release::inventory(&prior_path));
    write(
        out.join("validation.json"),
        &json!({"before":prior.receipt().input,"after":next.receipt().input,"originals":originals,"controls":controls,"queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,"complete_original_builds":0,"usage_inventory":"pending","support_targets":"pending","generated_activation":"pending"}),
    );
}
