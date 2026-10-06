//! Offline evidence for one pre-copy reduction boundary, not supplier closure.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const CONTROLS: [(&str, f64); 7] = [
    ("allocated-mystic-attunement", 25.0),
    ("remove-mystic-attunement", 0.0),
    ("boots-percent-100", 100.0),
    ("amulet-percent-100", 100.0),
    ("boots-and-passive-125", 125.0),
    ("remove-boots-percent", 25.0),
    ("absent-amulet", 0.0),
];
const OBSERVER: &str = "crates/poe-optimizer-pob/tests/support/amulet_bonus_snapshot_source.lua";
const LIFECYCLE: &str = "crates/poe-optimizer-pob/tests/support/djinn_provider_source.lua";
const STAT: &str = "EffectOfBonusesFromAmulet";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        &[]
    } else {
        v.as_array().expect("explicit source sequence")
    }
}
fn same(a: &Value, b: &Value, label: &str) {
    assert!(a == b, "{label} differs");
}
fn number(v: &Value) -> f64 {
    let n = v.as_f64().expect("source number");
    assert!(n.is_finite());
    n
}
fn case_names() -> Value {
    let mut names: Vec<_> = (1..=5).map(|i| format!("original-{i:02}")).collect();
    names.extend(CONTROLS.map(|(name, _)| name.to_owned()));
    names.extend((1..=5).map(|i| format!("repeat-original-{i:02}")));
    names.extend([
        "unhooked-original-05".to_owned(),
        "unhooked-amulet-percent-100".to_owned(),
    ]);
    json!(names)
}
fn named<'a>(r: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "case {name}");
    found[0]
}
fn record(m: &Value) {
    assert_eq!(m["name"], STAT);
    assert_eq!(m["type"], "INC");
    assert_eq!(m["flags"], 0);
    assert_eq!(m["keywordFlags"], 0);
    assert!(
        m.get("positions").is_none(),
        "unreviewed conditional incoming record"
    );
    number(&m["value"]);
}
fn total(chain: &Value) -> f64 {
    rows(chain)
        .iter()
        .map(|layer| {
            rows(&layer["rows"])
                .iter()
                .map(|r| number(&r["record"]["value"]))
                .sum::<f64>()
        })
        .sum()
}
fn query(q: &Value, expected: Option<f64>, name: &str) {
    for k in [
        "cfg_absent",
        "store_is_player",
        "bucket_complete",
        "untagged_domain",
        "query_return_observed",
    ] {
        assert_eq!(q[k], true, "{k}");
    }
    assert_eq!(q["caller_line"], 1662);
    assert_eq!(q["query_name"], STAT);
    assert_eq!(q["query_type"], "INC");
    let layers = rows(&q["before"]);
    assert!(!layers.is_empty() && layers.len() < 32);
    for (depth, layer) in layers.iter().enumerate() {
        assert_eq!(layer["depth"], depth);
        if depth + 1 < layers.len() {
            assert_eq!(layer["parent_kind"], "store");
        } else {
            assert!(layer["parent_kind"] == "false_sentinel" || layer["parent_kind"] == "absent");
        }
        for (i, r) in rows(&layer["rows"]).iter().enumerate() {
            assert_eq!(r["index"], i + 1);
            record(&r["record"]);
        }
    }
    let actual = number(&q["result"]);
    assert_eq!(actual, total(&q["before"]));
    if let Some(expected) = expected {
        assert_eq!(actual, expected);
    }
    assert_eq!(rows(&q["internal"]).len(), layers.len());
    for depth in 0..layers.len() {
        let found: Vec<_> = rows(&q["internal"])
            .iter()
            .filter(|r| r["depth"] == depth)
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["original_return"], true);
        assert_eq!(
            number(&found[0]["result"]),
            layers[depth..]
                .iter()
                .map(|l| rows(&l["rows"])
                    .iter()
                    .map(|r| number(&r["record"]["value"]))
                    .sum::<f64>())
                .sum::<f64>()
        );
    }
    assert_eq!(q["item"]["type"], "Amulet");
    assert_eq!(q["item"]["exact_registered"], true);
    let items = rows(&q["source_candidates"]["items"]);
    assert!(items.len() <= 64);
    for item in items {
        assert_eq!(item["exact_registered"], true);
    }
    let amulet: Vec<_> = items.iter().filter(|i| i["slot"] == "Amulet").collect();
    assert_eq!(amulet.len(), 1);
    same(&amulet[0]["id"], &q["item"]["id"], "source Amulet identity");
    let nodes = rows(&q["source_candidates"]["nodes"]);
    assert!(nodes.len() <= 4096);
    if matches!(
        name,
        "allocated-mystic-attunement"
            | "remove-mystic-attunement"
            | "boots-and-passive-125"
            | "remove-boots-percent"
    ) {
        same(
            &q["source_candidates"]["class"],
            &json!({"id":8,"name":"Huntress","ascendancy_id":3,"ascendancy_name":"Ritualist"}),
            "actual diagnostic class",
        );
        for id in [36365, 58574, 34785, 3223] {
            assert!(
                nodes.iter().any(|n| n["id"] == id),
                "connected Ritualist path node {id}"
            );
        }
        if name == "remove-mystic-attunement" {
            assert!(nodes.iter().all(|n| n["id"] != 7068));
        }
    }
    for n in nodes {
        assert_eq!(n["exact_allocated_object"], true);
        assert!(n["effective_lookup"] == "local" || n["effective_lookup"] == "tree_inherited");
    }
    if matches!(
        name,
        "allocated-mystic-attunement" | "boots-and-passive-125" | "remove-boots-percent"
    ) {
        let found: Vec<_> = nodes.iter().filter(|n| n["id"] == 7068).collect();
        assert_eq!(found.len(), 1);
        let n = found[0];
        assert_eq!(n["name"], "Mystic Attunement");
        assert!(!rows(&n["original_returns"]).is_empty());
        assert!(rows(&n["original_returns"]).iter().any(|r| {
            rows(&r["returned"]["rows"])
                .iter()
                .any(|m| m["record"]["name"] == STAT && m["record"]["value"] == 25)
        }));
        assert!(layers.iter().any(|l| {
            rows(&l["rows"])
                .iter()
                .any(|r| r["record"]["source"] == "Tree:7068" && r["record"]["value"] == 25)
        }));
    }
    let copies = rows(&q["copies"]);
    assert!(!copies.is_empty() && copies.len() <= 16384);
    for c in copies {
        for k in [
            "exact_source_object",
            "exact_copy_argument",
            "exact_destination",
            "return_observed",
        ] {
            assert_eq!(c[k], true);
        }
        assert_eq!(number(&c["factor"]), actual / 100.0);
        assert_eq!(rows(&c["insertions"]).len(), 1);
        assert_eq!(c["insertions"][0]["exact_inserted_object"], true);
        assert!(c["source_index"].as_u64().unwrap() > 0);
    }
    if name == "amulet-percent-100" {
        let found: Vec<_> = copies
            .iter()
            .filter(|c| c["original_record"]["name"] == STAT)
            .collect();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0]["original_record"]["value"], 100);
        assert_eq!(found[0]["insertions"][0]["record"]["value"], 100);
        assert_eq!(total(&copies.last().unwrap()["after_bucket"]), 200.0);
    }
    if name == "original-05" {
        assert_eq!(actual, 0.0);
        assert!(layers.iter().all(|l| rows(&l["rows"]).is_empty()));
    }
}
fn project(state: &Value) -> Value {
    let mut modes = serde_json::Map::new();
    for e in rows(&state["environments"]) {
        let mode = e["mode"].as_str().unwrap();
        assert!(matches!(mode, "MAIN" | "CALCS"));
        assert_eq!(e["actual_selected_env_observed"], true);
        let q = if let Some(i) = e["query_index"].as_u64() {
            assert!(i > 0);
            let q = &rows(&state["snapshot"]["queries"])[i as usize - 1];
            assert_eq!(q["mode"], mode);
            q.clone()
        } else {
            assert_eq!(e["has_amulet"], false);
            Value::Null
        };
        assert!(modes.insert(mode.to_owned(), q).is_none());
    }
    assert_eq!(modes.len(), 2);
    json!({"environments":state["environments"],"modes":modes})
}
fn observation(name: &str, value: &Value) {
    let expected = if name == "original-05" {
        0.0
    } else {
        CONTROLS.iter().find(|(n, _)| *n == name).unwrap().1
    };
    assert_eq!(rows(&value["environments"]).len(), 2);
    assert_eq!(value["modes"].as_object().unwrap().len(), 2);
    for mode in ["MAIN", "CALCS"] {
        let e: Vec<_> = rows(&value["environments"])
            .iter()
            .filter(|e| e["mode"] == mode)
            .collect();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0]["actual_selected_env_observed"], true);
        if name == "absent-amulet" {
            assert_eq!(e[0]["has_amulet"], false);
            assert!(e[0].get("query_index").is_none());
            assert!(value["modes"][mode].is_null());
        } else {
            assert_eq!(e[0]["has_amulet"], true);
            assert!(e[0]["query_index"].as_u64().unwrap() > 0);
            query(&value["modes"][mode], Some(expected), name);
        }
    }
}
fn checked_report(p: &Value, path_key: &str, bytes_key: &str, hash_key: &str) -> Vec<u8> {
    let path = p[path_key].as_str().unwrap();
    assert!(
        path.starts_with("runs/")
            && !path.contains("..")
            && !path.contains('\\')
            && !path.contains(':')
    );
    let size = p[bytes_key].as_u64().unwrap();
    assert!(size > 0 && size <= 64 * 1024 * 1024);
    let path = root().join(path);
    assert_eq!(fs::metadata(&path).unwrap().len(), size);
    let bytes = fs::read(path).unwrap();
    assert_eq!(hash(&bytes), p[hash_key]);
    bytes
}

/// Fast checks authenticate committed finite vectors. Full publication also
/// authenticates the original immutable raw reports and rederives projections.
pub fn check(v: &Value, full: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["context_projection"],
        "exact-current-environment-pre-amulet-query-v1"
    );
    assert_eq!(v["scope"]["snapshot_aggregation"], true);
    for k in [
        "incoming_contributor_closure",
        "existing_owner_closures_changed",
        "post_copy_values_reused_as_input",
        "complete_build_parity",
    ] {
        assert_eq!(v["scope"][k], false);
    }
    assert!(
        v["scope"]["source_status"]
            .as_str()
            .unwrap()
            .ends_with("-passed")
    );
    same(
        &v["declaration"],
        &json!({"stat":STAT,"type":"INC","cfg":"absent","capture_line":1662,"copy_line":1667,
        "incoming_domain":"untagged INC, flags0, keywordFlags0, finite numeric values","percent_divisor":100,"current_bucket_only":true,
        "arbitrary_supplier_domain":false,"game_legality":false,"native_inventory_closure":false}),
        "finite source declaration",
    );
    let cert = &v["certificate"];
    same(&cert["case_names"], &case_names(), "case inventory");
    same(&cert["lifecycle_stages"], &json!(STAGES), "fixed lifecycle");
    assert_eq!(cert["byte_identical_jit_modes"], true);
    assert_eq!(cert["raw_byte_identical_jit_modes"], true);
    for (key, path) in [
        ("observer_sha256", OBSERVER),
        ("lifecycle_sha256", LIFECYCLE),
    ] {
        assert_eq!(cert[key], hash(&fs::read(root().join(path)).unwrap()));
    }
    let raw =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let normalized = String::from_utf8(raw).unwrap().replace("\r\n", "\n");
    assert_eq!(hash(normalized.as_bytes()), cert["manifest_sha256"]);
    let manifest: Value = serde_json::from_str(&normalized).unwrap();
    same(
        &manifest["upstream_revision"],
        &cert["source_revision"],
        "source revision",
    );
    assert_eq!(
        cert["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    for pin in rows(&cert["files"]) {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|m| m["path"] == pin["path"] && m["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let tree = &cert["tree_pin"];
    assert_eq!(tree["path"], "src/TreeData/0_5/tree.lua");
    assert_eq!(
        rows(&manifest["files"])
            .iter()
            .filter(|m| m["path"] == tree["path"]
                && m["sha256"] == tree["sha256"]
                && m["bytes"] == tree["bytes"])
            .count(),
        1
    );
    for path in [
        "src/Modules/CalcSetup.lua",
        "src/Modules/ModParser.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModDB.lua",
        "src/Data/ModCache.lua",
    ] {
        assert_eq!(
            rows(&cert["files"])
                .iter()
                .filter(|p| p["path"] == path)
                .count(),
            1
        );
    }
    assert_eq!(rows(&cert["original_sources"]).len(), 5);
    for (i, pin) in rows(&cert["original_sources"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        );
        assert_eq!(pin["path"], path);
        assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
    }
    assert_eq!(v["methods"].as_object().unwrap().len(), 8);
    for (key, path, line) in [
        ("initialization", "Modules/CalcSetup.lua", 717),
        ("sum", "Classes/ModStore.lua", 202),
        ("sum_internal", "Classes/ModDB.lua", 137),
        ("scale", "Classes/ModStore.lua", 82),
        ("add_database", "Classes/ModDB.lua", 31),
        ("node", "Modules/CalcSetup.lua", 200),
        ("node_list", "Modules/CalcSetup.lua", 415),
        ("callback", "HeadlessWrapper.lua", 17),
    ] {
        assert_eq!(v["methods"][key]["path"], path);
        assert_eq!(v["methods"][key]["first"], line);
        assert!(v["methods"][key]["last"].as_u64().unwrap() >= line);
    }
    let selected = std::iter::once("original-05").chain(CONTROLS.map(|(n, _)| n));
    assert_eq!(rows(&v["observations"]).len(), 8);
    for (o, name) in rows(&v["observations"]).iter().zip(selected) {
        assert_eq!(o["case"], name);
        assert_eq!(o["stage"], "fresh");
        observation(name, &o["value"]);
    }
    let reports = rows(&v["reports"]);
    assert_eq!(reports.len(), 2);
    for (i, p) in reports.iter().enumerate() {
        assert_eq!(p["mode"], if i == 0 { "off" } else { "on" });
        same(&p["bytes"], &reports[0]["bytes"], "mode byte count");
        same(&p["sha256"], &reports[0]["sha256"], "mode digest");
        same(&p["bytes"], &p["raw_bytes"], "raw bytes");
        same(&p["sha256"], &p["raw_sha256"], "raw digest");
    }
    if !full {
        return;
    }
    let mut previous = None;
    for p in reports {
        let bytes = checked_report(p, "path", "bytes", "sha256");
        let raw = checked_report(p, "raw_path", "raw_bytes", "raw_sha256");
        assert!(bytes == raw, "raw report differs");
        if let Some(previous) = previous {
            assert!(bytes == previous, "JIT report differs");
        }
        let r: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r["schema_version"], 1);
        assert_eq!(r["numeric_tolerance"], 0);
        for k in [
            "source_revision",
            "manifest_sha256",
            "observer_sha256",
            "lifecycle_sha256",
            "files",
            "original_sources",
            "lifecycle_stages",
        ] {
            same(&r[k], &cert[k], k);
        }
        for k in ["business_wrappers", "source_tables_mutated"] {
            assert_eq!(r[k], false);
        }
        same(
            &json!(
                rows(&r["cases"])
                    .iter()
                    .map(|c| &c["name"])
                    .collect::<Vec<_>>()
            ),
            &case_names(),
            "full case names",
        );
        for c in rows(&r["cases"]) {
            for stage in STAGES {
                let s = &c["states"][stage];
                let snapshot = &s["snapshot"];
                same(&snapshot["methods"], &v["methods"], "source methods");
                for k in [
                    "complete_frames",
                    "original_methods_preserved",
                    "hook_removed",
                ] {
                    assert_eq!(snapshot[k], true);
                }
                for k in [
                    "business_wrappers",
                    "diagnostic_query_substitution",
                    "whole_supplier_domain_complete",
                    "native_inventory_authority",
                ] {
                    assert_eq!(snapshot[k], false);
                }
                if c["instrumented"] == false {
                    assert!(rows(&snapshot["queries"]).is_empty());
                    continue;
                }
                for q in rows(&snapshot["queries"]) {
                    query(
                        q,
                        c["control"]["expected_pre_copy_percent"].as_f64(),
                        c["name"].as_str().unwrap(),
                    );
                }
                if c["name"] == "original-05" || CONTROLS.iter().any(|(n, _)| c["name"] == *n) {
                    observation(c["name"].as_str().unwrap(), &project(s));
                }
            }
        }
        for o in rows(&v["observations"]) {
            let c = named(&r, o["case"].as_str().unwrap());
            same(&c["control"], &o["control"], "control projection");
            same(
                &project(&c["states"]["fresh"]),
                &o["value"],
                "exact source projection",
            );
        }
        for i in 1..=5 {
            same(
                &named(&r, &format!("original-{i:02}"))["states"],
                &named(&r, &format!("repeat-original-{i:02}"))["states"],
                "independent replay",
            );
        }
        {
            let (a, b) = ("allocated-mystic-attunement", "remove-boots-percent");
            same(
                &named(&r, a)["xml_sha256"],
                &named(&r, b)["xml_sha256"],
                "supplier removal input",
            );
            same(
                &named(&r, a)["states"],
                &named(&r, b)["states"],
                "supplier removal state",
            );
        }
        let removed = named(&r, "remove-mystic-attunement");
        for name in ["allocated-mystic-attunement", "remove-mystic-attunement"] {
            same(
                &named(&r, name)["control"]["connected_baseline_xml_sha256"],
                &removed["xml_sha256"],
                "connected control baseline",
            );
        }
        assert_eq!(
            removed["control"]["node_removal_restores_connected_baseline"],
            true
        );
        assert_ne!(
            removed["xml_sha256"],
            named(&r, "original-05")["xml_sha256"]
        );
        for name in ["original-05", "amulet-percent-100"] {
            for stage in STAGES {
                same(
                    &named(&r, name)["states"][stage]["outputs"],
                    &named(&r, &format!("unhooked-{name}"))["states"][stage]["outputs"],
                    "unhooked original outputs",
                );
            }
        }
        previous = Some(bytes);
    }
}
