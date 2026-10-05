//! Authoring-only finite source receipt. No native reducer, final cost, or ground-growth authority.
//! The certificate pins both full reports and their raw files. `controls` contains
//! seven named fresh-context projections (only queries.output is removed); the
//! full gate additionally checks every lifecycle stage and independent replay.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const EFFECT: &str = "SupportEncroachingGroundPlayer";
const MAGNIFIED: &str = "SupportMagnifiedAreaPlayerTwo";
const RAPID: &str = "SupportRapidCastingPlayer";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const CONTROLS: [&str; 7] = [
    "ice-focused",
    "ice-remove",
    "ice-disable",
    "duplicate-equal",
    "duplicate-higher",
    "ice-remove-rapid",
    "ice-encroaching-only",
];
const PARTS: [&str; 2] = [
    "crates/poe-optimizer-pob/tests/support/physical_support_delivery.lua",
    "crates/poe-optimizer-pob/tests/support/encroaching_ground_delivery.lua",
];
const GROWTH: [&str; 2] = [
    "support_ground_effect_area_of_effect_+%_final_per_second",
    "support_ground_effect_area_of_effect_+%_final_per_second_max",
];
const MAX_REPORT: u64 = 96 * 1024 * 1024;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn same(a: &Value, b: &Value, label: &str) {
    assert!(a == b, "{label} differs");
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        &[]
    } else {
        v.as_array()
            .expect("source array or authenticated empty table")
    }
}
fn optional_empty(v: &Value) {
    assert!(v.is_null() || rows(v).is_empty());
}
fn case<'a>(r: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1, "case {name}");
    found[0]
}
fn case_names() -> Value {
    let mut names: Vec<_> = (1..=5).map(|i| format!("original-{i:02}")).collect();
    names.extend(
        [
            "ice-focused",
            "ice-remove",
            "ice-disable",
            "duplicate-equal",
            "duplicate-higher",
            "repeat-ice-focused",
            "ice-remove-rapid",
            "ice-encroaching-only",
        ]
        .map(str::to_owned),
    );
    names.extend((1..=5).map(|i| format!("repeat-original-{i:02}")));
    json!(names)
}
fn project(contexts: &Value) -> Value {
    let mut copy = contexts.clone();
    for c in copy.as_array_mut().unwrap() {
        assert!(
            c["queries"]
                .as_object_mut()
                .unwrap()
                .remove("output")
                .is_some()
        );
    }
    copy
}
fn check_maps(maps: &Value) {
    assert_eq!(rows(maps).len(), 2);
    for (row, stat) in rows(maps).iter().zip(GROWTH) {
        same(
            row,
            &json!({"stat":stat,"present":false}),
            "unmapped declaration diagnostic",
        );
    }
}
fn definitions(v: &Value) {
    let defs = rows(&v["definitions"]);
    assert_eq!(defs.len(), 3);
    for (d, effect) in defs.iter().zip([EFFECT, MAGNIFIED, RAPID]) {
        assert_eq!(d["effect"], effect);
        assert_eq!(d["mod_source"], format!("Skill:{effect}"));
        assert_eq!(rows(&d["stat_sets"]).len(), 1);
        check_maps(&d["stat_sets"][0]["declared_maps"]);
    }
    let d = &defs[0];
    let set = &d["stat_sets"][0];
    same(
        &d["levels"],
        &json!({"positions":[{"index":1,"value":{"levelRequirement":0,"manaMultiplier":10}}]}),
        "Encroaching level declaration",
    );
    same(
        &d["family"],
        &json!({"positions":[{"index":1,"value":"EncroachingGround"}]}),
        "family declaration",
    );
    same(
        &d["require_types"],
        &json!({"positions":[{"index":1,"value":167}]}),
        "required type declaration",
    );
    assert!(rows(&d["exclude_types"]).is_empty());
    assert!(rows(&d["add_types"]).is_empty());
    optional_empty(&d["add_flags"]);
    same(
        &set["constants"],
        &json!({"positions":[
            {"index":1,"value":{"positions":[{"index":1,"value":GROWTH[0]},{"index":2,"value":20}]}},
            {"index":2,"value":{"positions":[{"index":1,"value":GROWTH[1]},{"index":2,"value":100}]}}
        ]}),
        "unresolved growth declarations",
    );
    same(
        &set["levels"],
        &json!({"positions":[{"index":1,"value":{"actorLevel":1}}]}),
        "stat-set declaration",
    );
    assert_eq!(set["index"], 1);
    assert!(rows(&set["stats"]).is_empty());
    optional_empty(&set["quality_stats"]);
    optional_empty(&set["base_mods"]);
    // Peer supports authenticate composition/absence. Their native programs
    // remain separately authored, inherited owners.
    assert_eq!(
        defs[1]["levels"]["positions"][0]["value"]["manaMultiplier"],
        30
    );
    same(
        &defs[2]["levels"],
        &json!({"positions":[{"index":1,"value":{"levelRequirement":0}}]}),
        "Rapid has no cost declaration",
    );
    check_maps(&v["global_stat_maps"]);
    same(
        &v["source_cost_precision"],
        &json!({"stat":"SupportManaMultiplier","type":"MORE","digits":4}),
        "original source precision diagnostic",
    );
    same(
        &v["methods"],
        &json!([
            {"name":"Sum","path":"Classes/ModStore.lua","first":202,"last":217},
            {"name":"Tabulate","path":"Classes/ModStore.lua","first":345,"last":364},
            {"name":"More","path":"Classes/ModStore.lua","first":261,"last":276},
            {"name":"MoreInternal","path":"Classes/ModDB.lua","first":214,"last":252},
            {"name":"MoreInternal","path":"Classes/ModList.lua","first":164,"last":193},
            {"name":"SumInternal","path":"Classes/ModDB.lua","first":137,"last":161}
        ]),
        "original method declarations",
    );
}
fn channel(q: &Value, name: &str, expected: &[(&str, i64, u64)], label: &str) {
    let raw = rows(&q["channels"][name]["raw_source_records"]);
    assert_eq!(raw.len(), expected.len(), "{label}/{name}/raw");
    for (index, (r, (effect, value, flags))) in raw.iter().zip(expected).enumerate() {
        same(
            r,
            &json!({"channel_index":index+1,"ancestor_depth":1,"source_effect":effect,
            "record":{"name":name,"type":if name=="Speed" {"INC"}else{"MORE"},"value":value,
            "source":format!("Skill:{effect}"),"flags":flags,"keyword_flags":0,"tags":{}}}),
            label,
        );
    }
    let applied: Vec<_> = rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect();
    assert_eq!(applied.len(), raw.len(), "{label}/{name}/applied");
    for (index, (a, r)) in applied.iter().zip(raw).enumerate() {
        same(
            a,
            &json!({"value":r["record"]["value"],"source_effect":r["source_effect"],"record":r["record"],"source_record_indices":[index+1]}),
            label,
        );
    }
    for a in rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_none())
    {
        assert!(rows(&a["source_record_indices"]).is_empty());
    }
}
fn expected_control(name: &str) -> (bool, bool, bool, bool) {
    match name {
        "ice-focused" | "repeat-ice-focused" | "duplicate-equal" => (true, true, true, false),
        "ice-remove" | "ice-disable" => (false, true, true, false),
        "duplicate-higher" => (true, true, true, true),
        "ice-remove-rapid" => (true, true, false, false),
        "ice-encroaching-only" => (true, false, false, false),
        _ => panic!("unknown control"),
    }
}
fn contexts(contexts: &Value, name: &str, control: &Value, projected: bool) {
    let expected = expected_control(name);
    assert_eq!(rows(contexts).len(), 2);
    assert_eq!(control["effect"], "IceNovaPlayer");
    for (field, value) in [
        ("encroaching", expected.0),
        ("magnified", expected.1),
        ("rapid", expected.2),
    ] {
        assert_eq!(control[field], value);
    }
    assert_eq!(control["main_set"], 1);
    assert_eq!(control["calcs_set"], 2);
    assert_eq!(control["winner_quality"], if expected.3 { 15 } else { 0 });
    if expected.0 {
        assert_eq!(control["winner_position"], if expected.3 { 3 } else { 2 });
    } else {
        assert!(control["winner_position"].is_null());
    }
    for (c, (mode, set)) in rows(contexts).iter().zip([("MAIN", 1), ("CALCS", 2)]) {
        let label = format!("{name}/{mode}");
        assert_eq!(c["mode"], mode);
        assert_eq!(c["stat_set_index"], set);
        assert_eq!(c["effect"], "IceNovaPlayer");
        assert_eq!(c["actor_is_player"], true);
        assert_eq!(c["selected"], true);
        assert_eq!(c["group"]["source_present"], false);
        assert_eq!(c["group"]["preset"], 4);
        assert!(c["group"]["source_ordinal"].is_u64());
        assert_eq!(c["source"]["exact_source_instance"], true);
        assert_eq!(c["source"]["skill_id"], "IceNovaPlayer");
        assert_eq!(c["source"]["position"], 1);
        assert_eq!(c["source"]["enabled"], true);
        let candidates = rows(&c["candidates"]);
        let expected_effects: Vec<_> = [
            (EFFECT, expected.0),
            (MAGNIFIED, expected.1),
            (RAPID, expected.2),
        ]
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(effect, _)| effect)
        .collect();
        assert_eq!(
            candidates
                .iter()
                .map(|x| x["effect"].as_str().unwrap())
                .collect::<Vec<_>>(),
            expected_effects
        );
        let mut origins = BTreeSet::from([c["source"]["source_ordinal"].as_u64().unwrap()]);
        for s in candidates {
            for (r, k) in [
                (s, "accepted"),
                (s, "exact_definition"),
                (&s["origin"], "exact_source_instance"),
                (&s["origin"], "enabled"),
            ] {
                assert_eq!(r[k], true);
            }
            assert!(origins.insert(s["origin"]["source_ordinal"].as_u64().unwrap()));
            assert_eq!(s["origin"]["skill_id"], s["effect"]);
            assert_eq!(s["origin"]["saved_attributes"]["skillId"], s["effect"]);
            if s["effect"] == EFFECT {
                assert_eq!(s["origin"]["position"], control["winner_position"]);
                assert_eq!(s["origin"]["raw_quality"], control["winner_quality"]);
                assert_eq!(s["origin"]["raw_level"], 1);
            }
        }
        let q = &c["queries"];
        assert_eq!(q.get("output").is_none(), projected);
        for k in [
            "cast_flag",
            "cfg_effect_exact",
            "exact_stat_set",
            "original_query_methods",
            "output_available",
            "output_is_selected_actor",
        ] {
            assert_eq!(q[k], true);
        }
        assert_eq!(q["cast_flag_value"], 16);
        assert_eq!(
            q["query_observation_kind"],
            "diagnostic_original_method_read"
        );
        assert_eq!(q["original_calculation_call_captured"], false);
        assert_ne!(q["skill_flags"]["disable"], true);
        same(
            &q["store_chain"],
            &json!([
                {"depth":0,"kind":"ModList","base_skill_store":false,"actor_store":false},
                {"depth":1,"kind":"ModList","base_skill_store":true,"actor_store":false},
                {"depth":2,"kind":"ModDB","base_skill_store":false,"actor_store":true}
            ]),
            &label,
        );
        let mut cost = vec![];
        if expected.0 {
            cost.push((EFFECT, 10, 0));
        }
        if expected.1 {
            cost.push((MAGNIFIED, 30, 0));
        }
        channel(q, "SupportManaMultiplier", &cost, &label);
        let speed = if expected.2 {
            vec![(RAPID, 15, 16)]
        } else {
            vec![]
        };
        channel(q, "Speed", &speed, &label);
        channel(q, "ReservationMultiplier", &[], &label);
        channel(q, "ExtraSpirit", &[], &label);
        let subtotal = match (expected.0, expected.1) {
            (true, true) => 1.43,
            (true, false) => 1.1,
            (false, true) => 1.3,
            (false, false) => 1.0,
        };
        // This is PoB's precision-four diagnostic, not the native product law.
        assert_eq!(q["cost_factor"], subtotal);
    }
}

fn read_report(path: &Value, bytes: &Value, digest: &Value) -> Vec<u8> {
    let path = path.as_str().unwrap();
    assert!(
        path.starts_with("runs/")
            && !path.contains("..")
            && !path.contains('\\')
            && !path.contains(':')
    );
    let bytes = bytes.as_u64().unwrap();
    assert!(bytes > 0 && bytes <= MAX_REPORT);
    let path = root().join(path);
    assert_eq!(fs::metadata(&path).unwrap().len(), bytes);
    let contents = fs::read(path).unwrap();
    assert_eq!(hash(&contents), digest.as_str().unwrap());
    contents
}

pub fn check(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["context_projection"],
        "physical-context-without-final-output-v1"
    );
    same(&v["scope"], &b["scope"], "vector scope");
    same(&v["scope"], &a["scope"], "authoring scope");
    same(
        &v["scope"],
        &json!({"ordinary_cost_contribution_only":true,"final_cost_formula":false,
        "ground_growth_model":false,"reservation_formula":false,"owner_closure":"partial","receiving_fragment_only":true}),
        "finite scope",
    );
    same(
        &v["declaration"],
        &json!({"source_mana_multiplier":10,"ordinary_cost_factor":1.1,
        "ground_growth_per_second":20,"ground_growth_cap":100,"ground_growth_consumer_owned":false,
        "mapped_stat":"SupportManaMultiplier","mapped_type":"MORE","mapped_flags":0,
        "source_reservation_multiplier_present":false,"source_spirit_reservation_flat_present":false}),
        "finite declaration",
    );
    assert_eq!(rows(&b["supports"]).len(), 1);
    assert_eq!(b["supports"][0]["source_effect"], EFFECT);
    assert_eq!(b["supports"][0]["cost_factor"], 1.1);
    assert_eq!(b["supports"][0]["source_mana_multiplier"], 10);
    let cert = &v["certificate"];
    for k in ["byte_identical_jit_modes", "raw_byte_identical_jit_modes"] {
        assert_eq!(cert[k], true);
    }
    same(&cert["lifecycle_stages"], &json!(STAGES), "stages");
    same(&cert["case_names"], &case_names(), "case names");
    assert_eq!(rows(&cert["original_sources"]).len(), 5);
    for (index, pin) in rows(&cert["original_sources"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            index + 1
        );
        assert_eq!(pin["path"], path);
        assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
    }
    for prefix in ["report", "raw_report"] {
        assert_eq!(rows(&cert[format!("{prefix}_paths")]).len(), 2);
        assert!(
            cert[format!("{prefix}_bytes")]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= MAX_REPORT)
        );
        let sha = cert[format!("{prefix}_sha256")].as_str().unwrap();
        assert!(sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit()));
    }
    assert_eq!(cert["report_bytes"], cert["raw_report_bytes"]);
    assert_eq!(cert["report_sha256"], cert["raw_report_sha256"]);
    let mut directories = BTreeSet::new();
    for (index, mode) in ["off", "on"].into_iter().enumerate() {
        let path = cert["report_paths"][index].as_str().unwrap();
        let suffix = format!("/source-jit-{mode}.json");
        let directory = path.strip_suffix(&suffix).unwrap();
        assert!(
            directory.starts_with("runs/")
                && !directory.contains("..")
                && !directory.contains('\\')
                && !directory.contains(':')
        );
        directories.insert(directory);
        assert_eq!(
            cert["raw_report_paths"][index],
            format!("{directory}/source-jit-{mode}.raw.json")
        );
    }
    assert_eq!(directories.len(), 1);
    let parts: Vec<_> = PARTS
        .iter()
        .map(|p| fs::read(root().join(p)).unwrap())
        .collect();
    let expected_parts: Vec<_> = PARTS
        .iter()
        .zip(&parts)
        .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
        .collect();
    same(
        &cert["observer_parts"],
        &json!(expected_parts),
        "observer parts",
    );
    let observer = format!(
        "local collect = (function()\n{}\nend)()\nreturn (function(collect)\n{}\nend)(collect)",
        std::str::from_utf8(&parts[0]).unwrap(),
        std::str::from_utf8(&parts[1]).unwrap()
    );
    assert_eq!(cert["observer_sha256"], hash(observer.as_bytes()));
    definitions(v);
    assert_eq!(rows(&v["controls"]).len(), CONTROLS.len());
    for (control, name) in rows(&v["controls"]).iter().zip(CONTROLS) {
        assert_eq!(control["name"], name);
        assert_eq!(control["xml_sha256"].as_str().unwrap().len(), 64);
        contexts(&control["contexts"], name, &control["control"], true);
    }
    if !full {
        return;
    }
    let mut previous = None;
    for (path, raw_path) in rows(&cert["report_paths"])
        .iter()
        .zip(rows(&cert["raw_report_paths"]))
    {
        let bytes = read_report(path, &cert["report_bytes"], &cert["report_sha256"]);
        let raw = read_report(
            raw_path,
            &cert["raw_report_bytes"],
            &cert["raw_report_sha256"],
        );
        assert!(raw == bytes, "raw/certified report differs");
        if let Some(p) = previous {
            assert!(bytes == p, "JIT reports differ");
        }
        let r: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r["schema_version"], 1);
        assert_eq!(r["source_revision"], a["source_revision"]);
        assert_eq!(r["manifest_sha256"], a["source_manifest_sha256"]);
        for k in [
            "observer_sha256",
            "observer_parts",
            "original_sources",
            "lifecycle_stages",
        ] {
            same(&r[k], &cert[k], k);
        }
        let files: Vec<_> = rows(&a["source_files"])
            .iter()
            .map(|p| json!({"path":p["path"],"sha256":p["sha256"]}))
            .collect();
        same(&r["files"], &json!(files), "source file pins");
        assert_eq!(r["numeric_tolerance"], 0);
        for k in [
            "business_wrappers",
            "source_tables_mutated",
            "source_cfg_modified",
            "native_build_parity",
            "native_inventory_authority",
            "ground_growth_formula_authority",
            "final_resource_cost_formula_authority",
            "reservation_formula_authority",
            "canonical_parity_lifecycle_selected",
            "original_calculation_calls_captured",
        ] {
            assert_eq!(r[k], false, "{k}");
        }
        assert_eq!(
            r["query_observation_kind"],
            "diagnostic_original_method_read"
        );
        same(
            &json!(
                rows(&r["cases"])
                    .iter()
                    .map(|c| &c["name"])
                    .collect::<Vec<_>>()
            ),
            &cert["case_names"],
            "complete cases",
        );
        for (index, pin) in rows(&cert["original_sources"]).iter().enumerate() {
            let path = format!(
                "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                index + 1
            );
            assert_eq!(pin["path"], path);
            assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
            let name = format!("original-{:02}", index + 1);
            let original = case(&r, &name);
            let repeat = case(&r, &format!("repeat-{name}"));
            same(&original["xml_sha256"], &pin["sha256"], "original XML");
            same(&original["xml_sha256"], &repeat["xml_sha256"], "repeat XML");
            same(
                &original["states"],
                &repeat["states"],
                "independent original replay",
            );
        }
        same(
            &case(&r, "ice-focused")["xml_sha256"],
            &case(&r, "repeat-ice-focused")["xml_sha256"],
            "focused repeat XML",
        );
        same(
            &case(&r, "ice-focused")["states"],
            &case(&r, "repeat-ice-focused")["states"],
            "independent focused replay",
        );
        for c in rows(&r["cases"]) {
            assert_eq!(c["independent_source_bindings_verified"], true);
            for stage in STAGES {
                let d = &c["states"][stage]["delivery"];
                for k in ["original_methods_preserved", "jit_mode_preserved"] {
                    assert_eq!(d[k], true);
                }
                for k in [
                    "source_cfg_modified",
                    "source_tables_mutated",
                    "business_wrappers",
                    "original_calculation_calls_captured",
                ] {
                    assert_eq!(d[k], false);
                }
                assert_eq!(d["immutable_snapshot"]["verified"], true);
                assert!(
                    d["immutable_snapshot"]["tables"]
                        .as_u64()
                        .is_some_and(|n| n > 0 && n <= 200000)
                );
                assert!(
                    d["immutable_snapshot"]["entries"]
                        .as_u64()
                        .is_some_and(|n| n <= 4000000)
                );
                for k in [
                    "definitions",
                    "methods",
                    "global_stat_maps",
                    "source_cost_precision",
                ] {
                    same(&d[k], &v[k], k);
                }
                if !c["control"].is_null() {
                    contexts(
                        &d["contexts"],
                        c["name"].as_str().unwrap(),
                        &c["control"],
                        false,
                    );
                }
            }
        }
        for control in rows(&v["controls"]) {
            let actual = case(&r, control["name"].as_str().unwrap());
            for k in ["name", "xml_sha256", "control"] {
                same(&actual[k], &control[k], k);
            }
            same(
                &project(&actual["states"]["fresh"]["delivery"]["contexts"]),
                &control["contexts"],
                "checked source projection",
            );
        }
        previous = Some(bytes);
    }
}
