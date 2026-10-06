//! Bounded offline evidence for ordinary Amulet recipient applicability.
//! It supplies no native owner closure or inferred minion Gem-level consumer.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const LIMIT: u64 = 16 * 1024 * 1024;
const CASES: [&str; 21] = [
    "original-01",
    "original-02",
    "original-03",
    "original-04",
    "original-05",
    "connected-baseline",
    "allocated-talisman",
    "remove-talisman",
    "baseline-boots-copy-100",
    "talisman-boots-copy-100",
    "remove-copy-supplier",
    "talisman-amulet-copy-100",
    "talisman-absent-amulet",
    "repeat-original-01",
    "repeat-original-02",
    "repeat-original-03",
    "repeat-original-04",
    "repeat-original-05",
    "unhooked-original-05",
    "unhooked-allocated-talisman",
    "unhooked-talisman-boots-copy-100",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|v| v.is_empty()) {
        &[]
    } else {
        v.as_array().expect("bounded source sequence")
    }
}
fn hash(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}
fn same(a: &Value, b: &Value, label: &str) {
    assert!(a == b, "{label} differs");
}
fn text(path: &str) -> String {
    fs::read_to_string(root().join(path))
        .unwrap()
        .replace("\r\n", "\n")
}
fn relative(path: &str) -> &Path {
    let p = Path::new(path);
    assert!(p.components().all(|c| matches!(c, Component::Normal(_))));
    p
}
fn read_checked(record: &Value, path: &str, bytes: &str, sha: &str) -> Vec<u8> {
    let p = root().join(relative(record[path].as_str().unwrap()));
    let n = record[bytes].as_u64().unwrap();
    assert!((1..=LIMIT).contains(&n));
    assert_eq!(fs::metadata(&p).unwrap().len(), n);
    let b = fs::read(p).unwrap();
    assert_eq!(b.len() as u64, n);
    assert_eq!(hash(&b), record[sha]);
    b
}
fn project(report: &Value) -> Value {
    let mut projected = report.clone();
    for case in projected["cases"].as_array_mut().unwrap() {
        for stage in STAGES {
            let state = case["states"][stage].as_object_mut().unwrap();
            state.remove("outputs");
            for env in state
                .get_mut("environments")
                .unwrap()
                .as_array_mut()
                .unwrap()
            {
                env.as_object_mut().unwrap().remove("before_perform");
            }
        }
    }
    projected
}
fn expected(name: &str) -> Option<(bool, bool, i64, i64)> {
    match name {
        "original-05" | "repeat-original-05" | "connected-baseline" | "remove-talisman" => {
            Some((false, false, 0, 22))
        }
        "allocated-talisman" | "remove-copy-supplier" | "talisman-amulet-copy-100" => {
            Some((true, false, 0, 21))
        }
        "baseline-boots-copy-100" => Some((false, false, 100, 23)),
        "talisman-boots-copy-100" => Some((true, false, 100, 22)),
        "talisman-absent-amulet" => Some((true, true, 0, 21)),
        _ => None,
    }
}
fn source_property() -> Value {
    json!({"flags":0,"keywordFlags":0,"name":"GemProperty","type":"LIST",
        "source":"Item:23:New Item, Solar Amulet","sourceSlot":"Amulet",
        "value":{"keyword":"minion","key":"level","value":1,"keyOfScaledMod":"value"}})
}
fn named<'a>(r: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn check_projection(r: &Value) {
    assert_eq!(r["schema_version"], 1);
    assert_eq!(r["numeric_tolerance"], 0);
    assert_eq!(r["business_wrappers"], false);
    same(&r["lifecycle_stages"], &json!(STAGES), "lifecycle");
    for key in [
        "complete_native_owner",
        "intended_gameplay_law",
        "minion_gem_level_consumer_claim",
        "whole_build_closure",
        "roll_legality_authority",
    ] {
        assert_eq!(r["scope"][key], false);
    }
    assert_eq!(rows(&r["cases"]).len(), CASES.len());
    for (i, case) in rows(&r["cases"]).iter().enumerate() {
        assert_eq!(case["name"], CASES[i]);
        assert_eq!(case["instrumented"], i < 18);
        assert_eq!(case["independent_source_bindings_verified"], true);
        for stage in STAGES {
            let state = &case["states"][stage];
            assert_eq!(state["actual_calls"], i < 18);
            for k in ["original_methods_preserved", "hook_removed"] {
                assert_eq!(state[k], true);
            }
            for k in [
                "business_wrappers",
                "source_tables_mutated",
                "diagnostic_requery",
                "roll_legality_authority",
                "native_owner_closure",
                "minion_gem_level_consumer_claim",
            ] {
                assert_eq!(state[k], false);
            }
            assert_eq!(rows(&state["environments"]).len(), 2);
            for (env, mode) in rows(&state["environments"]).iter().zip(["MAIN", "CALCS"]) {
                assert_eq!(env["mode"], mode);
                assert_eq!(env["exact_selected_environment"], true);
                if i >= 18 {
                    continue;
                }
                for offering in rows(&env["offerings"]) {
                    assert_eq!(offering["exact_physical_source"], true);
                    same(
                        &offering["assembly"]["after"],
                        &offering["final"],
                        "prepared input retained",
                    );
                    same(
                        &offering["assembly"]["lookup"],
                        &offering["lookup"],
                        "prepared lookup retained",
                    );
                    for ordinary in rows(&offering["ordinary"]) {
                        for flag in [
                            "before_perform",
                            "original_query",
                            "exact_actor_store",
                            "no_diverted_object_consumed",
                        ] {
                            assert_eq!(ordinary[flag], true);
                        }
                    }
                }
                for (field, line) in [("diverted", 1402), ("copies", 1667)] {
                    for row in rows(&env[field]) {
                        assert_eq!(row["caller_line"], line);
                        for flag in [
                            "exact_source_object",
                            "exact_destination",
                            "actual_item_return",
                            "return_observed",
                        ] {
                            assert_eq!(row[flag], true);
                        }
                        assert_eq!(rows(&row["insertions"]).len(), 1);
                        assert_eq!(row["insertions"][0]["exact_inserted_object"], true);
                    }
                }
                for receipt in rows(&env["receipts"]) {
                    for flag in [
                        "exact_talisman_source",
                        "exact_minion_destination",
                        "original_return",
                        "skill_actor_is_player",
                        "receiver_is_skill_minion",
                        "selected_minion_identity",
                    ] {
                        assert_eq!(receipt[flag], true);
                    }
                    same(
                        &receipt["records"],
                        &env["talisman_records"],
                        "exact minion receipt",
                    );
                    assert_eq!(
                        rows(&receipt["joins"]).len(),
                        rows(&receipt["records"]).len()
                    );
                    for (n, join) in rows(&receipt["joins"]).iter().enumerate() {
                        assert_eq!(join["source_index"], n + 1);
                        assert!(!rows(&join["destination_indices"]).is_empty());
                    }
                }
                let Some((talisman, absent, percent, level)) = expected(CASES[i]) else {
                    continue;
                };
                assert_eq!(env["path"][2]["id"], 39935);
                assert_eq!(env["path"][2]["name"], "Necromantic Talisman");
                assert_eq!(env["path"][2]["allocated"], talisman);
                assert_eq!(env["path"][2]["exact_allocated_object"], true);
                let diverted = talisman && !absent;
                assert_eq!(env["final_amulet"]["absent"] == true, diverted || absent);
                if absent {
                    assert_eq!(env["amulet"]["absent"], true);
                    assert!(env["query"].is_null());
                } else {
                    assert_eq!(env["amulet"]["id"], 23);
                    assert_eq!(env["amulet"]["type"], "Amulet");
                    assert_eq!(env["amulet"]["exact_registered"], true);
                    assert_eq!(env["query"]["result"], percent);
                    for flag in ["original_call", "return_observed", "exact_player_store"] {
                        assert_eq!(env["query"][flag], true);
                    }
                }
                let properties: Vec<_> = rows(&env["talisman_records"])
                    .iter()
                    .filter(|r| r["record"]["name"] == "GemProperty")
                    .collect();
                assert_eq!(properties.len(), usize::from(diverted));
                if diverted {
                    same(
                        &properties[0]["record"],
                        &source_property(),
                        "diverted source property",
                    );
                } else {
                    assert!(rows(&env["diverted"]).is_empty());
                    assert!(rows(&env["receipts"]).is_empty());
                }
                assert_eq!(rows(&env["offerings"]).len(), 1);
                let offering = &env["offerings"][0];
                same(
                    &offering["raw"],
                    &json!({"level":20,"quality":0}),
                    "raw Offering",
                );
                same(
                    &offering["final"],
                    &json!({"level":level,"quality":0}),
                    "computed Offering",
                );
                assert_eq!(rows(&offering["ordinary"]).len(), 1);
                let ordinary = &offering["ordinary"][0];
                let originals = rows(&ordinary["candidates"])
                    .iter()
                    .filter(|c| c["record"] == source_property())
                    .count();
                assert_eq!(originals, usize::from(!diverted && !absent));
                let copies: Vec<_> = rows(&env["copies"])
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c["source_record"]["name"] == "GemProperty")
                    .collect();
                assert_eq!(copies.len(), usize::from(!absent));
                for (index, copy) in copies {
                    same(
                        &copy["source_record"],
                        &source_property(),
                        "copy's original property",
                    );
                    assert_eq!(copy["factor"].as_f64(), Some(percent as f64 / 100.0));
                    assert_eq!(
                        copy["insertions"][0]["record"]["value"]["value"],
                        percent / 100
                    );
                    let joins: Vec<_> = rows(&ordinary["joins"])
                        .iter()
                        .filter(|j| rows(&j["early_copy_indices"]).contains(&json!(index + 1)))
                        .collect();
                    assert_eq!(joins.len(), 1);
                    assert!(rows(&ordinary["matched"]).contains(&joins[0]["candidate_index"]));
                }
            }
        }
    }
    for i in 1..=5 {
        same(
            &named(r, &format!("original-{i:02}"))["states"],
            &named(r, &format!("repeat-original-{i:02}"))["states"],
            "independent original replay",
        );
    }
    for (a, b) in [
        ("connected-baseline", "remove-talisman"),
        ("allocated-talisman", "remove-copy-supplier"),
    ] {
        same(
            &named(r, a)["states"],
            &named(r, b)["states"],
            "removal inverse",
        );
        same(
            &named(r, a)["xml_sha256"],
            &named(r, b)["xml_sha256"],
            "removal input",
        );
    }
}

/// Fast checks use committed projection and source pins. Publication additionally
/// authenticates original report bytes and reconstructs the exact projection.
pub(crate) fn check(v: &Value, verify_external: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert!(serde_json::to_vec(v).unwrap().len() < 2 * 1024 * 1024);
    same(
        &v["scope"],
        &json!({"ordinary_player_recipient_guard":true,"early_copy_unchanged":true,
        "source_minion_store_receipt":true,"native_minion_property_consumer":false,
        "whole_modifier_owner_complete":false,"other_item_diversions_proved":false,
        "focus_dynamic_proof":false,"source_game_intent_resolved":false,"complete_original_builds":0}),
        "scope",
    );
    let projected = &v["report_projection"];
    check_projection(projected);
    same(&project(projected), projected, "projection idempotence");
    let manifest_text = text("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    assert_eq!(hash(manifest_text.as_bytes()), projected["manifest_sha256"]);
    let manifest: Value = serde_json::from_str(&manifest_text).unwrap();
    same(
        &manifest["upstream_revision"],
        &projected["source_revision"],
        "revision",
    );
    for (field, path) in [
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/talisman_property_source.lua",
        ),
        (
            "lifecycle_sha256",
            "crates/poe-optimizer-pob/tests/support/djinn_provider_source.lua",
        ),
    ] {
        assert_eq!(
            projected[field],
            hash(&fs::read(root().join(path)).unwrap())
        );
    }
    let pins = rows(&projected["files"]);
    let unique: BTreeSet<_> = pins.iter().map(|p| p["path"].as_str().unwrap()).collect();
    assert_eq!(unique.len(), pins.len());
    for pin in pins {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let index: Value = serde_json::from_slice(
        &fs::read(root().join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(rows(&projected["original_sources"]).len(), 5);
    for (i, source) in rows(&projected["original_sources"]).iter().enumerate() {
        assert_eq!(
            source["path"],
            format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                i + 1
            )
        );
        let bytes = fs::read(root().join(relative(source["path"].as_str().unwrap()))).unwrap();
        assert_eq!(source["sha256"], hash(&bytes));
        same(
            &source["sha256"],
            &index["builds"][i]["xml_sha256"],
            "unchanged original",
        );
        same(
            &source["sha256"],
            &projected["cases"][i]["xml_sha256"],
            "original case identity",
        );
    }
    let excerpts = [
        (
            "ordinary-amulet-diversion",
            "src/Modules/CalcSetup.lua",
            1397,
            1403,
        ),
        (
            "independent-early-player-copy",
            "src/Modules/CalcSetup.lua",
            1661,
            1669,
        ),
        (
            "player-ordinary-property-consumer",
            "src/Modules/CalcSetup.lua",
            543,
            575,
        ),
        (
            "early-item-grant-collection",
            "src/Modules/CalcSetup.lua",
            1039,
            1048,
        ),
        (
            "early-grants-sockets-only",
            "src/Classes/PassiveSpec.lua",
            1222,
            1276,
        ),
        (
            "generic-grant-keystone-resolution",
            "src/Classes/PassiveSpec.lua",
            1135,
            1158,
        ),
        (
            "later-generic-passive-grants",
            "src/Modules/CalcSetup.lua",
            1603,
            1616,
        ),
        (
            "actual-minion-store-delivery",
            "src/Modules/CalcPerform.lua",
            1094,
            1104,
        ),
        (
            "minion-assembly-before-store-delivery",
            "src/Modules/CalcPerform.lua",
            1203,
            1233,
        ),
        (
            "minion-level-assembly",
            "src/Modules/CalcActiveSkill.lua",
            1116,
            1164,
        ),
        (
            "focus-list-copy-static-only",
            "src/Modules/CalcSetup.lua",
            1473,
            1484,
        ),
        (
            "nonadditive-merge-static-only",
            "src/Classes/ModList.lua",
            80,
            95,
        ),
        (
            "default-talisman-declaration",
            "src/TreeData/0_5/tree.lua",
            92694,
            92713,
        ),
    ];
    assert_eq!(rows(&v["source_excerpts"]).len(), excerpts.len());
    for (excerpt, (id, expected_path, expected_first, expected_last)) in
        rows(&v["source_excerpts"]).iter().zip(excerpts)
    {
        assert_eq!(excerpt["id"], id);
        assert_eq!(excerpt["path"], expected_path);
        assert_eq!(excerpt["first_line"], expected_first);
        assert_eq!(excerpt["last_line"], expected_last);
        let path = excerpt["path"].as_str().unwrap();
        assert!(unique.contains(path));
        let source = text(&format!("vendor/path-of-building-poe2/{path}"));
        let first = excerpt["first_line"].as_u64().unwrap() as usize;
        let last = excerpt["last_line"].as_u64().unwrap() as usize;
        assert!(first > 0 && last >= first && last - first < 128);
        let lines: Vec<_> = source.lines().collect();
        let actual = lines[first - 1..last].join("\n") + "\n";
        assert_eq!(excerpt["text"], actual);
        assert_eq!(excerpt["sha256"], hash(actual.as_bytes()));
    }
    let templates = &v["template_evidence"];
    assert_eq!(
        templates["authoring_source"]["path"],
        "data/owned/poe2/3887ae68/amulet-level-copy/source-vectors.json"
    );
    let prior: Value = serde_json::from_slice(&read_checked(
        &templates["authoring_source"],
        "path",
        "bytes",
        "sha256",
    ))
    .unwrap();
    let report = rows(&prior["reports"])
        .iter()
        .find(|r| r["path"] == templates["report"]["path"])
        .unwrap();
    for field in ["path", "bytes", "sha256"] {
        same(
            &report[field],
            &templates["report"][field],
            "existing template report",
        );
    }
    assert_eq!(rows(&templates["observations"]).len(), 6);
    for (i, observation) in rows(&templates["observations"]).iter().enumerate() {
        assert_eq!(observation["pointer"], format!("/templates/{i}"));
        assert!(rows(&report["observations"]).contains(observation));
        let row = &observation["value"];
        let source = text(&format!(
            "vendor/path-of-building-poe2/{}",
            row["source_base_file"].as_str().unwrap()
        ));
        let lines: Vec<_> = source
            .lines()
            .skip(row["source_base_line"].as_u64().unwrap() as usize - 1)
            .collect();
        assert_eq!(
            lines[0],
            format!(
                "itemBases[\"{}\"] = {{",
                row["source_base"].as_str().unwrap()
            )
        );
        let end = lines.iter().position(|line| *line == "}").unwrap();
        assert_eq!(
            row["source_base_sha256"],
            hash(lines[..=end].join("\n").as_bytes())
        );
        assert!(
            lines[..=end].iter().any(|line| line.trim()
                == format!("type = \"{}\",", row["source_type"].as_str().unwrap()))
        );
    }
    let reports = rows(&v["reports"]);
    assert_eq!(reports.len(), 2);
    for (report, mode) in reports.iter().zip(["off", "on"]) {
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
        for (a, b) in [("bytes", "raw_bytes"), ("sha256", "raw_sha256")] {
            same(&report[a], &report[b], "raw report identity");
        }
        same(&report["bytes"], &reports[0]["bytes"], "JIT bytes");
        same(&report["sha256"], &reports[0]["sha256"], "JIT hash");
    }
    if !verify_external {
        return;
    }
    let placement: Value = serde_json::from_slice(&read_checked(
        &templates["report"],
        "path",
        "bytes",
        "sha256",
    ))
    .unwrap();
    for o in rows(&templates["observations"]) {
        same(
            placement.pointer(o["pointer"].as_str().unwrap()).unwrap(),
            &o["value"],
            "template projection",
        );
    }
    let mut previous = None;
    for report in reports {
        let bytes = read_checked(report, "path", "bytes", "sha256");
        let raw = read_checked(report, "raw_path", "raw_bytes", "raw_sha256");
        assert!(bytes == raw, "raw evidence differs");
        if let Some(old) = &previous {
            assert!(*old == bytes, "JIT evidence differs");
        }
        let full: Value = serde_json::from_slice(&bytes).unwrap();
        same(&project(&full), projected, "full exact report projection");
        for i in 1..=5 {
            same(
                &named(&full, &format!("original-{i:02}"))["states"],
                &named(&full, &format!("repeat-original-{i:02}"))["states"],
                "full independent original replay, including all output snapshots",
            );
        }
        for name in [
            "original-05",
            "allocated-talisman",
            "talisman-boots-copy-100",
        ] {
            let a = named(&full, name);
            let b = named(&full, &format!("unhooked-{name}"));
            for stage in STAGES {
                same(
                    &a["states"][stage]["outputs"],
                    &b["states"][stage]["outputs"],
                    "independent unhooked numerical output",
                );
            }
        }
        previous = Some(bytes);
    }
}
