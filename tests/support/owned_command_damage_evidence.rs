//! Shared test-only validation of bounded original Command Damage source evidence.
//! This module does not load Lua or provide native inventory authority.
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

const NODES: [(u64, f64); 3] = [(25927, 20.0), (41511, 15.0), (32847, 20.0)];
const OBSERVER: &str = "crates/poe-optimizer-pob/tests/support/command_damage_source.lua";
const LIFECYCLE: &str =
    "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs";
const MANIFEST: &str = "crates/poe-optimizer-pob/data/pob-source-manifest.json";
const ORIGINAL: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const XML_HASH: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const LIMIT: u64 = 64 * 1024 * 1024;
const CASES: [(&str, usize, usize, &[u64]); 14] = [
    ("original-05", 0, 0, &[]),
    ("selected-1-1", 1, 1, &[]),
    (
        "removed-command-branches-1-1",
        1,
        1,
        &[25927, 41511, 32847, 35560],
    ),
    ("selected-2-1", 2, 1, &[]),
    (
        "removed-command-branches-2-1",
        2,
        1,
        &[25927, 41511, 32847, 35560],
    ),
    ("selected-2-2", 2, 2, &[]),
    (
        "removed-command-branches-2-2",
        2,
        2,
        &[25927, 41511, 32847, 35560],
    ),
    ("selected-2-3", 2, 3, &[]),
    (
        "removed-command-branches-2-3",
        2,
        3,
        &[25927, 41511, 32847, 35560],
    ),
    ("removed-25927-branch", 2, 1, &[25927, 32847]),
    ("removed-41511-branch", 2, 1, &[41511, 35560]),
    ("removed-32847", 2, 1, &[32847]),
    ("original-05-repeat", 0, 0, &[]),
    ("selected-2-3-repeat", 2, 3, &[]),
];
fn rows(v: &Json) -> &[Json] {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        &[]
    } else {
        v.as_array().expect("explicit source sequence")
    }
}
fn number(v: &Json) -> f64 {
    let n = v.as_f64().expect("finite original number");
    assert!(n.is_finite());
    n
}
fn same(a: &Json, b: &Json, label: &str) {
    assert!(a == b, "{label} differs");
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn sha(v: &Json) {
    let s = v.as_str().expect("SHA256 string");
    assert!(
        s.len() == 64
            && s.bytes()
                .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
    );
}
fn envelope(r: &Json) {
    assert_eq!(r["schema_version"], 1);
    assert_eq!(r["case_count"], CASES.len());
    assert_eq!(
        r["manifest_sha256"],
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    assert_eq!(
        r["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(r["original_xml_sha256"], XML_HASH);
    assert_eq!(r["numeric_tolerance"], 0);
    assert_eq!(r["lifecycle"], "one-normal-load-per-fresh-vm");
    for k in [
        "native_owner_closure",
        "native_build_parity",
        "business_wrappers",
    ] {
        assert_eq!(r[k], false, "{k}");
    }
    sha(&r["observer_sha256"]);
    sha(&r["lifecycle_sha256"]);
    assert_eq!(rows(&r["cases"]).len(), CASES.len());
    for (c, (name, child, set, removed)) in rows(&r["cases"]).iter().zip(CASES) {
        assert_eq!(c["name"], name);
        assert_eq!(c["child"], child);
        assert_eq!(c["stat_set"], set);
        same(&c["removed"], &json!(removed), "exact removal control");
        let reviewed: Vec<_> = NODES
            .iter()
            .map(|(id, _)| *id)
            .filter(|id| removed.contains(id))
            .collect();
        same(
            &c["reviewed_sources_removed"],
            &json!(reviewed),
            "reviewed source subset of explicit allocation removal",
        );
        sha(&c["xml_sha256"]);
        if child == 0 {
            assert_eq!(c["xml_sha256"], XML_HASH);
        }
        assert_eq!(c["state"]["methods_preserved"], true);
        for k in [
            "business_wrappers",
            "observer_modifies_game_state",
            "native_owner_closure",
            "whole_build_parity",
        ] {
            assert_eq!(c["state"][k], false, "{name}: {k}");
        }
    }
    same(
        &r["cases"][7]["xml_sha256"],
        &r["cases"][13]["xml_sha256"],
        "selected replay input",
    );
}

pub(crate) fn check_report(report: &Json) {
    envelope(report);
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 14);
    for (a, b) in [(0, 12), (7, 13)] {
        same(
            &cases[a]["state"],
            &cases[b]["state"],
            "fresh independent replay",
        );
    }
    for case in cases {
        let label = case["name"].as_str().unwrap();
        let removed = rows(&case["removed"]);
        for mode in ["main", "calcs"] {
            let env = &case["state"][mode];
            let actual = rows(&env["allocated_nodes"]);
            let expected: Vec<_> = rows(&cases[0]["state"][mode]["allocated_nodes"])
                .iter()
                .filter(|id| !removed.contains(id))
                .cloned()
                .collect();
            assert_eq!(
                actual, expected,
                "{label} {mode}: allocation removal changed additional actual sources"
            );
            for (id, amount) in NODES {
                let node = rows(&env["nodes"]).iter().find(|n| n["id"] == id).unwrap();
                assert_eq!(
                    node["allocated"],
                    !removed.contains(&json!(id)),
                    "{label} {mode} node{id}"
                );
                assert_eq!(
                    node["default_stats"],
                    json!([format!(
                        "Minions deal {}% increased Damage with Command Skills",
                        amount as u64
                    )])
                );
                assert_eq!(rows(&node["parser"]).len(), 1);
                assert_eq!(node["parser"][0]["cache_preserved"], true);
                assert_eq!(rows(&node["parser"][0]["modifiers"]).len(), 1);
                let mut parsed = node["parser"][0]["modifiers"][0].clone();
                let mut default = node["default_modifiers"][0].clone();
                // Parser output has no occurrence source yet. Source assignment
                // belongs to PassiveTree; every other field must already match.
                assert!(parsed["source"].is_null() || parsed["source"] == "");
                assert!(
                    parsed["value"]["mod"]["source"].is_null()
                        || parsed["value"]["mod"]["source"] == ""
                );
                for value in [&mut parsed, &mut default] {
                    value.as_object_mut().unwrap().remove("source");
                    value["value"]["mod"]
                        .as_object_mut()
                        .unwrap()
                        .remove("source");
                }
                assert_eq!(parsed, default, "{label} {mode} parser/source join{id}");
                assert_eq!(rows(&node["default_modifiers"]).len(), 1);
                modifier(&node["default_modifiers"][0], id, amount);
                if node["allocated"] == true {
                    assert!(
                        !rows(&node["original_node_returns"]).is_empty(),
                        "{label} {mode} missing node{id} return"
                    );
                    for r in rows(&node["original_node_returns"]) {
                        assert_eq!(r["node_is_allocated"], true);
                        assert_eq!(rows(&r["modifiers"]).len(), 1);
                        modifier(&r["modifiers"][0], id, amount);
                    }
                } else {
                    assert!(rows(&node["original_node_returns"]).is_empty());
                }
            }
            if case["child"] == 0 && mode == "calcs" {
                continue;
            }
            let wanted_effect = if case["child"] == 2 {
                "GasShotSkeletonSniperMinion"
            } else {
                "MinionMeleeBow"
            };
            let selected: Vec<_> = rows(&env["recipients"])
                .iter()
                .filter(|r| r["selected"] == true && r["effect"] == wanted_effect)
                .collect();
            assert_eq!(selected.len(), 1, "{label} {mode}");
            let calls = rows(&selected[0]["original_offence_calls"]);
            assert!(!calls.is_empty(), "{label} {mode}");
            for call in calls {
                assert_eq!(call["actor_parent_is_player"], true);
                assert_eq!(call["summoner_owns_actor"], true);
                assert_eq!(call["actor_is_selected"], true);
                assert_eq!(
                    call["source"]["physical_gem"],
                    "Metadata/Items/Gems/SkillGemSkeletalSniper"
                );
                if case["child"] != 0 {
                    assert_eq!(call["stat_set_index"], case["stat_set"], "{label} {mode}");
                }
                let expected_count = 3 - rows(&case["reviewed_sources_removed"]).len();
                assert_eq!(
                    rows(&call["parent_records"]).len(),
                    expected_count,
                    "{label} {mode}"
                );
                assert_eq!(
                    rows(&call["received_records"]).len(),
                    expected_count,
                    "{label} {mode}"
                );
                assert_eq!(rows(&call["joins"]).len(), expected_count);
                for (index, join) in rows(&call["joins"]).iter().enumerate() {
                    assert_eq!(join["received_index"], index + 1);
                    assert_eq!(join["exact_nested_object"], true);
                    assert_eq!(rows(&join["parent_indices"]).len(), 1);
                    let parent = rows(&join["parent_indices"])[0].as_u64().unwrap() as usize;
                    assert!((1..=expected_count).contains(&parent));
                    same(
                        &call["received_records"][index]["record"],
                        &call["parent_records"][parent - 1]["record"]["value"]["mod"],
                        "original parent/receiver record identity projection",
                    );
                }
                for (id, amount) in NODES.iter().filter(|(id, _)| !removed.contains(&json!(id))) {
                    let parents: Vec<_> = rows(&call["parent_records"])
                        .iter()
                        .filter(|r| r["record"]["value"]["mod"]["source"] == format!("Tree:{id}"))
                        .collect();
                    assert_eq!(parents.len(), 1, "{label} {mode} exact node source{id}");
                    modifier(&parents[0]["record"], *id, *amount);
                }
                let expected: f64 = NODES
                    .iter()
                    .filter(|(id, _)| !removed.contains(&json!(id)))
                    .map(|(_, v)| v)
                    .sum();
                let command = wanted_effect == "GasShotSkeletonSniperMinion";
                assert_eq!(call["diagnostic"]["commandable"], command);
                let received = rows(&call["diagnostic"]["reviewed_records"]);
                assert_eq!(received.len(), if command { expected_count } else { 0 });
                assert_eq!(
                    received.iter().map(|r| number(&r["value"])).sum::<f64>(),
                    if command { expected } else { 0.0 }
                );
                let consumers: Vec<_> = rows(&call["original_damage_calls"])
                    .iter()
                    .filter(|r| !r["inc"].is_null())
                    .collect();
                assert!(
                    !consumers.is_empty(),
                    "{label} {mode} no nonzero original damage consumer"
                );
                for c in consumers {
                    assert_eq!(c["original_calc_damage"], true);
                    assert_eq!(c["diagnostic"]["query_state_preserved"], true);
                    assert_eq!(
                        number(&c["inc"]),
                        1.0 + number(&c["diagnostic"]["sum"]) / 100.0
                    );
                    let rows = rows(&c["diagnostic"]["reviewed_records"]);
                    assert_eq!(rows.len(), if command { expected_count } else { 0 });
                    assert_eq!(
                        rows.iter().map(|r| number(&r["value"])).sum::<f64>(),
                        if command { expected } else { 0.0 }
                    );
                }
            }
        }
    }
}

fn modifier(v: &Json, id: u64, amount: f64) {
    assert_eq!(v["name"], "MinionModifier");
    assert_eq!(v["type"], "LIST");
    assert_eq!(v["flags"], 0);
    assert_eq!(v["keywordFlags"], 0);
    assert!(v["_positions"].is_null());
    let m = &v["value"]["mod"];
    assert_eq!(m["name"], "Damage");
    assert_eq!(m["type"], "INC");
    assert_eq!(number(&m["value"]), amount);
    assert_eq!(m["source"], format!("Tree:{id}"));
    assert_eq!(m["flags"], 0);
    assert_eq!(m["keywordFlags"], 0);
    assert_eq!(
        m["_positions"],
        json!([{"index":1,"value":{"type":"Condition","var":"CommandableSkill"}}])
    );
}

/// A receipt omits only scalar output snapshots; source inputs, consumer locals,
/// complete selected records, cfg and occurrence joins remain byte-exact values.
pub(crate) fn project_report(report: &Json) -> Json {
    let mut projected = report.clone();
    for case in projected["cases"].as_array_mut().expect("cases") {
        for mode in ["main", "calcs"] {
            let env = case["state"][mode].as_object_mut().expect("environment");
            env.remove("output");
            let recipients = env.get_mut("recipients").expect("recipients");
            if recipients.as_object().is_some_and(|o| o.is_empty()) {
                continue;
            }
            for r in recipients.as_array_mut().expect("recipient sequence") {
                let calls = r.get_mut("original_offence_calls").expect("calls");
                if calls.as_object().is_some_and(|o| o.is_empty()) {
                    continue;
                }
                for c in calls.as_array_mut().expect("offence sequence") {
                    c.as_object_mut().unwrap().remove("output");
                }
            }
        }
    }
    projected
}

fn root() -> PathBuf {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if here.join(MANIFEST).exists() {
        here
    } else {
        here.join("../..").canonicalize().unwrap()
    }
}

fn checked_file(p: &Json, path_key: &str, size_key: &str, hash_key: &str) -> Vec<u8> {
    let relative = Path::new(p[path_key].as_str().expect("report path"));
    assert!(
        relative
            .components()
            .all(|c| matches!(c, Component::Normal(_)))
    );
    assert!(relative.starts_with("runs"));
    let path = root().join(relative);
    let bytes = p[size_key].as_u64().unwrap();
    assert!((1..=LIMIT).contains(&bytes));
    assert_eq!(fs::metadata(&path).unwrap().len(), bytes);
    let raw = fs::read(path).unwrap();
    assert_eq!(raw.len() as u64, bytes);
    assert_eq!(hash(&raw), p[hash_key]);
    raw
}

/// Fast validation uses the bounded authenticated projection plus tracked pins.
/// Full publication additionally reads both original retained reports and proves
/// exact projection correspondence. It never executes or depends on Lua.
pub(crate) fn check(v: &Json, full: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert!(
        serde_json::to_vec(v).unwrap().len() <= 7 * 1024 * 1024 / 2,
        "Command Damage source vectors exceed the publication budget"
    );
    let projected = &v["report_projection"];
    check_report(projected);
    same(
        &project_report(projected),
        projected,
        "idempotent bounded projection",
    );
    let manifest_bytes = fs::read_to_string(root().join(MANIFEST))
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        hash(manifest_bytes.as_bytes()),
        projected["manifest_sha256"]
    );
    let manifest: Json = serde_json::from_str(&manifest_bytes).unwrap();
    same(
        &manifest["upstream_revision"],
        &projected["source_revision"],
        "pinned revision",
    );
    for (key, path) in [
        ("observer_sha256", OBSERVER),
        ("lifecycle_sha256", LIFECYCLE),
    ] {
        assert_eq!(projected[key], hash(&fs::read(root().join(path)).unwrap()));
    }
    assert_eq!(hash(&fs::read(root().join(ORIGINAL)).unwrap()), XML_HASH);
    let index: Json = serde_json::from_slice(
        &fs::read(root().join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(index["builds"][4]["xml_sha256"], XML_HASH);
    let pins = rows(&projected["files"]);
    let unique: std::collections::BTreeSet<_> =
        pins.iter().map(|p| p["path"].as_str().unwrap()).collect();
    assert_eq!(pins.len(), unique.len());
    for p in pins {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|m| m["path"] == p["path"] && m["sha256"] == p["sha256"])
                .count(),
            1
        );
    }
    for required in [
        "src/Modules/CalcSetup.lua",
        "src/Modules/CalcActiveSkill.lua",
        "src/Modules/CalcPerform.lua",
        "src/Modules/CalcOffence.lua",
        "src/Modules/ModParser.lua",
        "src/Modules/ModTools.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModList.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/PassiveTree.lua",
        "src/Classes/PassiveSpec.lua",
        "src/Data/Skills/minion.lua",
        "src/TreeData/0_5/tree.lua",
    ] {
        assert!(unique.contains(required), "missing consumer pin {required}");
    }
    let reports = rows(&v["reports"]);
    assert_eq!(reports.len(), 2);
    for (i, report) in reports.iter().enumerate() {
        let mode = if i == 0 { "off" } else { "on" };
        assert_eq!(report["mode"], mode);
        assert!(
            report["path"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/source-jit-{mode}.json"))
        );
        assert!(
            report["raw_path"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/source-jit-{mode}.raw.json"))
        );
        assert!((1..=LIMIT).contains(&report["bytes"].as_u64().unwrap()));
        sha(&report["sha256"]);
        same(&report["bytes"], &report["raw_bytes"], "unchanged raw size");
        same(
            &report["sha256"],
            &report["raw_sha256"],
            "unchanged raw digest",
        );
        same(&report["bytes"], &reports[0]["bytes"], "JIT size");
        same(&report["sha256"], &reports[0]["sha256"], "JIT digest");
        let observations = rows(&report["observations"]);
        assert_eq!(observations.len(), 5);
        for (o, path) in observations.iter().zip([
            "/schema_version",
            "/manifest_sha256",
            "/original_xml_sha256",
            "/case_count",
            "/numeric_tolerance",
        ]) {
            assert_eq!(o["pointer"], path);
            same(
                &o["value"],
                projected.pointer(path).unwrap(),
                "receipt pointer",
            );
        }
    }
    if !full {
        return;
    }
    let mut previous: Option<Vec<u8>> = None;
    for p in reports {
        let bytes = checked_file(p, "path", "bytes", "sha256");
        let raw = checked_file(p, "raw_path", "raw_bytes", "raw_sha256");
        assert!(bytes == raw, "raw/semantic report bytes differ");
        if let Some(prev) = &previous {
            assert!(*prev == bytes, "JIT report bytes differ");
        }
        let report: Json = serde_json::from_slice(&bytes).unwrap();
        check_report(&report);
        same(
            &project_report(&report),
            projected,
            "full source projection",
        );
        for o in rows(&p["observations"]) {
            same(
                report.pointer(o["pointer"].as_str().unwrap()).unwrap(),
                &o["value"],
                "full report pointer",
            );
        }
        previous = Some(bytes);
    }
}
