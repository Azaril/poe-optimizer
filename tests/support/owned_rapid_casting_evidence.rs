//! Source receipts are authoring evidence, never a runtime input or full-build claim.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

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
        v.as_array()
            .expect("source array or authenticated empty table")
    }
}
fn case<'a>(r: &'a Value, name: &str) -> &'a Value {
    let found: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(found.len(), 1);
    found[0]
}

// Final output is retained and authenticated in the complete local report. It
// supplies no authority for this contribution slice and is not duplicated in
// the committed authoring projection.
fn contribution_contexts(contexts: &Value) -> Value {
    let mut contexts = contexts.clone();
    for context in contexts.as_array_mut().unwrap() {
        assert!(
            context["queries"]
                .as_object_mut()
                .unwrap()
                .remove("output")
                .is_some()
        );
    }
    contexts
}

pub fn check(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["context_projection"],
        "physical-context-without-final-output-v1"
    );
    assert_eq!(
        v["declaration"],
        json!({"source_stat":"base_cast_speed_+%","mapped_stat":"Speed",
        "mapped_type":"INC","mapped_flags":16,"raw_values":[15,20],
        "source_mana_multiplier_present":false,"source_reservation_multiplier_present":false})
    );
    let certificate = &v["certificate"];
    assert_eq!(certificate["byte_identical_jit_modes"], true);
    assert_eq!(certificate["raw_byte_identical_jit_modes"], true);
    assert_eq!(
        certificate["lifecycle_stages"],
        json!(["fresh", "rebuilt_once", "rebuilt_twice"])
    );
    assert_eq!(rows(&certificate["case_names"]).len(), 18);
    assert_eq!(rows(&certificate["original_sources"]).len(), 5);
    assert_eq!(rows(&certificate["report_paths"]).len(), 2);
    assert!(
        certificate["report_bytes"]
            .as_u64()
            .is_some_and(|n| n > 0 && n <= 96 * 1024 * 1024)
    );
    assert_eq!(certificate["report_sha256"].as_str().unwrap().len(), 64);
    let observer = root().join("crates/poe-optimizer-pob/tests/support/rapid_casting_delivery.lua");
    assert_eq!(
        certificate["observer_sha256"],
        hash(&fs::read(observer).unwrap())
    );
    assert_eq!(
        v["global_cast_speed_map"],
        json!({"positions":[{"index":1,"value":{
        "flags":16,"keywordFlags":0,"name":"Speed","type":"INC"}}]})
    );
    assert_eq!(rows(&v["definitions"]).len(), 2);
    for (d, s) in rows(&v["definitions"]).iter().zip(rows(&b["supports"])) {
        assert_eq!(d["effect"], s["source_effect"]);
        assert_eq!(
            d["levels"],
            json!({"positions":[{"index":1,"value":{"levelRequirement":0}}]})
        );
        assert_eq!(
            d["family"],
            json!({"positions":[{"index":1,"value":"RapidCasting"}]})
        );
        assert_eq!(
            d["require_types"],
            json!({"positions":[{"index":1,"value":2}]})
        );
        assert_eq!(
            d["exclude_types"],
            json!({"positions":[{"index":1,"value":64},
            {"index":2,"value":106},{"index":3,"value":218}]})
        );
        assert_eq!(rows(&d["stat_sets"]).len(), 1);
        assert_eq!(
            d["stat_sets"][0]["constants"],
            json!({"positions":[{"index":1,"value":{
            "positions":[{"index":1,"value":"base_cast_speed_+%"},{"index":2,"value":s["cast_speed_increase"]}]}}]})
        );
        assert!(rows(&d["stat_sets"][0]["stats"]).is_empty());
        assert!(rows(&d["add_types"]).is_empty());
    }
    let names = [
        "ice-tier-i",
        "ice-tier-ii",
        "ice-remove",
        "ice-disable",
        "family-ii-last",
        "family-i-last",
    ];
    assert_eq!(rows(&v["controls"]).len(), names.len());
    for (control, name) in rows(&v["controls"]).iter().zip(names) {
        assert_eq!(control["name"], name);
        assert_eq!(rows(&control["contexts"]).len(), 2);
        for (c, mode, set) in rows(&control["contexts"])
            .iter()
            .zip(["MAIN", "CALCS"])
            .zip([1, 2])
            .map(|((c, m), s)| (c, m, s))
        {
            assert_eq!(c["mode"], mode);
            assert_eq!(c["stat_set_index"], set);
            assert_eq!(c["effect"], "IceNovaPlayer");
            assert_eq!(c["actor_is_player"], true);
            assert_eq!(c["source"]["exact_source_instance"], true);
            let q = &c["queries"];
            assert!(q.get("output").is_none());
            for k in [
                "cast_flag",
                "cfg_effect_exact",
                "exact_stat_set",
                "original_query_methods",
            ] {
                assert_eq!(q[k], true);
            }
            assert_eq!(q["cast_flag_value"], 16);
            assert_eq!(
                q["query_observation_kind"],
                "diagnostic_original_method_read"
            );
            assert_eq!(q["original_calculation_call_captured"], false);
            assert_eq!(
                q["store_chain"],
                json!([
                {"depth":0,"base_skill_store":false,"actor_store":false,"kind":"ModList"},
                {"depth":1,"base_skill_store":true,"actor_store":false,"kind":"ModList"},
                {"depth":2,"base_skill_store":false,"actor_store":true,"kind":"ModDB"}])
            );
            let winner = &control["control"]["winner"];
            let n = usize::from(!winner.is_null());
            let candidates = rows(&c["candidates"]);
            let speed = rows(&q["channels"]["Speed"]["raw_source_records"]);
            let applied: Vec<_> = rows(&q["channels"]["Speed"]["applied"])
                .iter()
                .filter(|r| r.get("source_effect").is_some())
                .collect();
            assert_eq!((candidates.len(), speed.len(), applied.len()), (n, n, n));
            if n == 1 {
                let s = rows(&b["supports"])
                    .iter()
                    .find(|s| s["source_effect"] == *winner)
                    .unwrap();
                let expected = json!({"name":"Speed","type":"INC","value":s["cast_speed_increase"],
                    "source":format!("Skill:{}",winner.as_str().unwrap()),"flags":16,"keyword_flags":0,"tags":{}});
                assert_eq!(candidates[0]["effect"], *winner);
                assert_eq!(candidates[0]["accepted"], true);
                assert_eq!(candidates[0]["origin"]["exact_source_instance"], true);
                assert_eq!(
                    candidates[0]["origin"]["position"],
                    control["control"]["winner_position"]
                );
                assert_eq!(speed[0]["record"], expected);
                assert_eq!(speed[0]["ancestor_depth"], 1);
                assert_eq!(applied[0]["record"], expected);
                assert_eq!(applied[0]["source_record_indices"], json!([1]));
                assert_eq!(applied[0]["value"], s["cast_speed_increase"]);
            }
            for channel in [
                "SupportManaMultiplier",
                "ReservationMultiplier",
                "ExtraSpirit",
            ] {
                let channel = &q["channels"][channel];
                assert!(rows(&channel["raw_source_records"]).is_empty());
                assert!(
                    rows(&channel["applied"])
                        .iter()
                        .all(|r| r.get("source_effect").is_none()
                            && rows(&r["source_record_indices"]).is_empty())
                );
            }
        }
    }
    if !full {
        return;
    }
    let mut previous = None;
    for path in rows(&certificate["report_paths"]) {
        let path = path.as_str().unwrap();
        assert!(path.starts_with("runs/") && !path.contains(".."));
        let path = root().join(path);
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            certificate["report_bytes"].as_u64().unwrap()
        );
        let bytes = fs::read(path).unwrap();
        assert_eq!(hash(&bytes), certificate["report_sha256"]);
        if let Some(p) = previous {
            assert!(bytes == p, "JIT modes differ");
        }
        let r: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r["source_revision"], a["source_revision"]);
        assert_eq!(r["manifest_sha256"], a["source_manifest_sha256"]);
        assert_eq!(r["observer_sha256"], certificate["observer_sha256"]);
        let pins: Vec<_> = rows(&a["source_files"])
            .iter()
            .map(|p| json!({"path":p["path"],"sha256":p["sha256"]}))
            .collect();
        assert_eq!(r["files"], json!(pins));
        assert_eq!(r["original_sources"], certificate["original_sources"]);
        assert_eq!(r["lifecycle_stages"], certificate["lifecycle_stages"]);
        assert_eq!(r["numeric_tolerance"], 0);
        for k in [
            "business_wrappers",
            "source_tables_mutated",
            "source_cfg_modified",
            "native_build_parity",
            "native_inventory_authority",
            "final_cast_rate_formula_authority",
            "cost_or_reservation_formula_authority",
            "canonical_parity_lifecycle_selected",
            "original_calculation_calls_captured",
        ] {
            assert_eq!(r[k], false);
        }
        assert_eq!(
            json!(
                rows(&r["cases"])
                    .iter()
                    .map(|c| &c["name"])
                    .collect::<Vec<_>>()
            ),
            certificate["case_names"]
        );
        for (index, pin) in rows(&r["original_sources"]).iter().enumerate() {
            let path = pin["path"].as_str().unwrap();
            assert!(path.starts_with("tests/fixtures/builds/") && !path.contains(".."));
            assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
            let name = format!("original-{:02}", index + 1);
            let original = case(&r, &name);
            assert_eq!(original["xml_sha256"], pin["sha256"]);
            assert!(
                original["states"] == case(&r, &format!("repeat-{name}"))["states"],
                "independent {name} replay differs"
            );
        }
        for (i, name) in ["ice-tier-i", "ice-tier-ii"].into_iter().enumerate() {
            assert!(
                case(&r, name)["states"] == case(&r, &format!("repeat-{name}"))["states"],
                "independent {name} replay differs"
            );
            assert_eq!(
                case(&r, name)["states"]["fresh"]["delivery"]["definitions"][i],
                v["definitions"][i]
            );
        }
        for c in rows(&r["cases"]) {
            assert_eq!(c["independent_source_bindings_verified"], true);
            for stage in ["fresh", "rebuilt_once", "rebuilt_twice"] {
                let d = &c["states"][stage]["delivery"];
                for k in ["original_methods_preserved", "jit_mode_preserved"] {
                    assert_eq!(d[k], true);
                }
                assert_eq!(d["immutable_snapshot"]["verified"], true);
                assert_eq!(d["global_cast_speed_map"], v["global_cast_speed_map"]);
            }
        }
        for control in rows(&v["controls"]) {
            let actual = case(&r, control["name"].as_str().unwrap());
            for k in ["name", "xml_sha256", "control"] {
                assert_eq!(actual[k], control[k]);
            }
            assert!(
                contribution_contexts(&actual["states"]["fresh"]["delivery"]["contexts"])
                    == control["contexts"],
                "checked source projection differs"
            );
        }
        previous = Some(bytes);
    }
}
