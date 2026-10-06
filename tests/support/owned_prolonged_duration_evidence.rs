//! Finite offline evidence for contributions on real player Actions. Source
//! precision, reference selection and buff lifetime are not native defaults.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};
const PAIN: &str = "PainOfferingPlayer";
const ICE: &str = "IceNovaPlayer";
const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";
const STAT: &str = "support_more_duration_skill_effect_duration_+%_final";
const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const CONTROLS: [&str; 12] = [
    "pain-tier-i",
    "pain-tier-ii",
    "pain-remove",
    "pain-disable",
    "family-ii-last",
    "family-i-last",
    "pain-raw-level",
    "pain-raw-quality",
    "pain-distinct-occurrences",
    "ice-tier-i",
    "ice-tier-ii",
    "ice-remove",
];
const PARTS: [&str; 2] = [
    "crates/poe-optimizer-pob/tests/support/physical_support_delivery.lua",
    "crates/poe-optimizer-pob/tests/support/prolonged_duration_delivery.lua",
];
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
        v.as_array().expect("array or authenticated empty table")
    }
}
fn optional_empty(v: &Value) {
    assert!(v.is_null() || rows(v).is_empty());
}
fn named<'a>(r: &'a Value, name: &str) -> &'a Value {
    let rows: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn names() -> Value {
    let mut n: Vec<_> = (1..=5).map(|i| format!("original-{i:02}")).collect();
    n.extend(CONTROLS.map(str::to_owned));
    n.extend(["repeat-pain-tier-i".into(), "repeat-ice-tier-i".into()]);
    n.extend((1..=5).map(|i| format!("repeat-original-{i:02}")));
    json!(n)
}
fn project(v: &Value) -> Value {
    let mut v = v.clone();
    for c in v.as_array_mut().unwrap() {
        assert!(
            c["queries"]
                .as_object_mut()
                .unwrap()
                .remove("output")
                .is_some()
        );
    }
    v
}
fn check_definitions(v: &Value, b: &Value) {
    assert_eq!(rows(&v["definitions"]).len(), 2);
    assert_eq!(rows(&b["supports"]).len(), 2);
    for ((d, s), (effect, value, factor)) in rows(&v["definitions"])
        .iter()
        .zip(rows(&b["supports"]))
        .zip([(I, 30, 1.3), (II, 35, 1.35)])
    {
        assert_eq!(d["effect"], effect);
        assert_eq!(s["source_effect"], effect);
        assert_eq!(s["source_duration_more"], value);
        assert_eq!(s["duration_factor"], factor);
        assert_eq!(s["source_mana_multiplier"], 20);
        assert_eq!(s["cost_factor"], 1.2);
        assert_eq!(d["mod_source"], format!("Skill:{effect}"));
        same(
            &d["levels"],
            &json!({"positions":[{"index":1,"value":{"levelRequirement":0,"manaMultiplier":20}}]}),
            "support level declaration",
        );
        same(
            &d["family"],
            &json!({"positions":[{"index":1,"value":"ProlongedDuration"}]}),
            "family",
        );
        same(
            &d["require_types"],
            &json!({"positions":[{"index":1,"value":9}]}),
            "Duration eligibility",
        );
        assert!(rows(&d["exclude_types"]).is_empty());
        assert!(rows(&d["add_types"]).is_empty());
        optional_empty(&d["add_flags"]);
        assert_eq!(rows(&d["stat_sets"]).len(), 1);
        let set = &d["stat_sets"][0];
        same(
            &set["constants"],
            &json!({"positions":[{"index":1,"value":{"positions":[{"index":1,"value":STAT},{"index":2,"value":value}]}}]}),
            "duration constant",
        );
        assert!(rows(&set["stats"]).is_empty());
        optional_empty(&set["quality_stats"]);
        optional_empty(&set["base_mods"]);
        same(
            &set["declared_maps"],
            &json!([{"stat":STAT,"present":true,"value":{"positions":[{"index":1,"value":{"name":"Duration","type":"MORE","source":format!("Skill:{effect}"),"flags":0,"keywordFlags":0}}]}}]),
            "local duration map",
        );
    }
    same(
        &v["global_stat_maps"],
        &json!([{"stat":STAT,"present":false}]),
        "global map absence diagnostic",
    );
    same(
        &v["profile_domain"],
        &json!([PAIN, ICE]),
        "finite physical domain",
    );
    assert_eq!(rows(&v["recipient_definitions"]).len(), 2);
    for (r, (effect, count, cast)) in rows(&v["recipient_definitions"])
        .iter()
        .zip([(PAIN, 1, 0.6), (ICE, 2, 1.0)])
    {
        assert_eq!(r["effect"], effect);
        assert_eq!(r["support"], false);
        assert_eq!(r["cast_time"], cast);
        assert_eq!(r["parts_present"], false);
        assert_eq!(r["minion_list_present"], false);
        assert!(r.get("parts").is_none());
        assert!(r.get("minion_list").is_none());
        assert_eq!(rows(&r["stat_sets"]).len(), count);
        assert_eq!(
            rows(&r["skill_types"]["positions"])
                .iter()
                .filter(|p| p["index"] == 9 && p["value"] == true)
                .count(),
            1
        );
    }
    let constants = rows(&v["recipient_definitions"][0]["stat_sets"][0]["constants"]["positions"]);
    for stat in [
        "base_skill_effect_duration",
        "base_secondary_skill_effect_duration",
    ] {
        assert_eq!(
            constants
                .iter()
                .filter(|r| r["value"]
                    == json!({"positions":[{"index":1,"value":stat},{"index":2,"value":6000}]}))
                .count(),
            1
        );
    }
    same(
        &v["source_cost_precision"],
        &json!({"stat":"SupportManaMultiplier","type":"MORE","digits":4}),
        "source precision diagnostic",
    );
    same(
        &v["methods"],
        &json!([
        {"name":"Sum","path":"Classes/ModStore.lua","first":202,"last":217},
        {"name":"Tabulate","path":"Classes/ModStore.lua","first":345,"last":364},
        {"name":"More","path":"Classes/ModStore.lua","first":261,"last":276},
        {"name":"MoreInternal","path":"Classes/ModDB.lua","first":214,"last":252},
        {"name":"MoreInternal","path":"Classes/ModList.lua","first":164,"last":193},
        {"name":"SumInternal","path":"Classes/ModDB.lua","first":137,"last":161}]),
        "original methods",
    );
}
fn channel(q: &Value, name: &str, winner: Option<&str>, value: u64) {
    let raw = rows(&q["channels"][name]["raw_source_records"]);
    assert_eq!(raw.len(), usize::from(winner.is_some()));
    for r in raw {
        same(
            r,
            &json!({"channel_index":1,"ancestor_depth":1,"source_effect":winner.unwrap(),"record":{
        "name":name,"type":"MORE","value":value,"source":format!("Skill:{}",winner.unwrap()),"flags":0,"keyword_flags":0,"tags":{}}}),
            name,
        );
    }
    let applied: Vec<_> = rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_some())
        .collect();
    assert_eq!(applied.len(), raw.len());
    for (r, s) in applied.iter().zip(raw) {
        same(
            r,
            &json!({"value":value,"record":s["record"],"source_effect":s["source_effect"],"source_record_indices":[1]}),
            "exact source object join",
        );
    }
    for r in rows(&q["channels"][name]["applied"])
        .iter()
        .filter(|r| r.get("source_effect").is_none())
    {
        assert!(rows(&r["source_record_indices"]).is_empty());
    }
}
fn contexts(contexts: &Value, control: &Value, name: &str, projected: bool) {
    let target = if name.starts_with("ice-") || name == "repeat-ice-tier-i" {
        ICE
    } else {
        PAIN
    };
    let expected = if name.ends_with("remove") || name.ends_with("disable") {
        None
    } else if name == "pain-tier-ii" || name == "ice-tier-ii" || name == "family-ii-last" {
        Some(II)
    } else {
        Some(I)
    };
    assert_eq!(control["effect"], target);
    same(&control["winner"], &json!(expected), "control winner");
    let copies = if name == "pain-distinct-occurrences" {
        2
    } else {
        1
    };
    assert_eq!(rows(&control["sources"]).len(), copies);
    assert_eq!(rows(contexts).len(), if copies == 2 { 6 } else { 4 });
    for c in rows(contexts) {
        let effect = c["effect"].as_str().unwrap();
        assert!([PAIN, ICE].contains(&effect));
        let winner = if effect == target {
            expected
        } else if effect == PAIN {
            Some(I)
        } else {
            None
        };
        assert_eq!(c["actor_is_player"], true);
        assert_eq!(c["group"]["source_present"], false);
        assert_eq!(c["source"]["skill_id"], effect);
        assert_eq!(c["source"]["exact_source_instance"], true);
        assert_eq!(c["source"]["enabled"], true);
        let candidates = rows(&c["candidates"]);
        assert_eq!(candidates.len(), usize::from(winner.is_some()));
        for s in candidates {
            assert_eq!(s["effect"], winner.unwrap());
            assert_eq!(s["origin"]["skill_id"], s["effect"]);
            assert_eq!(s["origin"]["saved_attributes"]["skillId"], s["effect"]);
            for (r, k) in [
                (s, "accepted"),
                (s, "exact_definition"),
                (&s["origin"], "exact_source_instance"),
                (&s["origin"], "enabled"),
            ] {
                assert_eq!(r[k], true);
            }
            assert_ne!(s["origin"]["source_ordinal"], c["source"]["source_ordinal"]);
            if effect == target {
                assert_eq!(s["origin"]["position"], control["winner_position"]);
            }
        }
        let q = &c["queries"];
        assert_eq!(q.get("output").is_none(), projected);
        assert_ne!(q["skill_flags"]["disable"], true);
        for k in [
            "cfg_effect_exact",
            "exact_stat_set",
            "original_query_methods",
        ] {
            assert_eq!(q[k], true);
        }
        same(
            &q["store_chain"],
            &json!([
            {"depth":0,"kind":"ModList","base_skill_store":false,"actor_store":false},
            {"depth":1,"kind":"ModList","base_skill_store":true,"actor_store":false},
            {"depth":2,"kind":"ModDB","base_skill_store":false,"actor_store":true}]),
            "actual player action store",
        );
        let d = &q["duration"];
        for k in [
            "no_attached_minion",
            "no_summoning_parent",
            "exact_physical_instance",
            "exact_actor",
            "primary_definition_exact",
        ] {
            assert_eq!(d[k], true);
        }
        assert_eq!(d["has_reservation"], false);
        assert_eq!(d["original_calculation_calls_captured"], false);
        assert_eq!(
            d["query_observation_kind"],
            "diagnostic_original_method_read"
        );
        assert_eq!(
            d["physical_id"],
            format!(
                "Metadata/Items/Gems/SkillGem{}",
                if effect == PAIN {
                    "PainOffering"
                } else {
                    "IceNova"
                }
            )
        );
        same(
            &d["granted_effects"],
            &json!([{"index":1,"effect":effect,"primary":true,"support":false}]),
            "declared primary",
        );
        if effect == PAIN {
            assert_eq!(d["primary_base"], 6);
            assert_eq!(d["secondary_base"], 6);
        }
        assert!(d["effective_level"].as_u64().is_some_and(|n| n > 0));
        assert!(d["effective_quality"].is_number());
        assert_eq!(q["duration_snapshot"]["verified"], true);
        assert_eq!(q["duration_snapshot"]["original_methods_preserved"], true);
        channel(
            q,
            "Duration",
            winner,
            if winner == Some(II) { 35 } else { 30 },
        );
        channel(q, "SupportManaMultiplier", winner, 20);
        for name in ["Speed", "ReservationMultiplier", "ExtraSpirit"] {
            let ch = &q["channels"][name];
            assert!(rows(&ch["raw_source_records"]).is_empty());
            assert!(
                rows(&ch["applied"])
                    .iter()
                    .all(|r| r.get("source_effect").is_none()
                        && rows(&r["source_record_indices"]).is_empty())
            );
        }
    }
    for mode in ["MAIN", "CALCS"] {
        let found: Vec<_> = rows(contexts)
            .iter()
            .filter(|c| c["effect"] == target && c["mode"] == mode)
            .collect();
        assert_eq!(found.len(), copies);
        let mut origins = BTreeSet::new();
        for (c, s) in found.iter().zip(rows(&control["sources"])) {
            for k in ["position", "raw_level", "raw_quality"] {
                assert_eq!(c["source"][k], s[k]);
            }
            assert!(origins.insert(c["source"]["source_ordinal"].as_u64().unwrap()));
        }
        let selected: Vec<_> = found.iter().filter(|c| c["selected"] == true).collect();
        assert_eq!(selected.len(), 1);
        let pick = if mode == "MAIN" { 1 } else { copies };
        assert_eq!(
            control[if mode == "MAIN" {
                "main_occurrence"
            } else {
                "calcs_occurrence"
            }],
            pick
        );
        assert_eq!(
            selected[0]["source"]["position"],
            control["sources"][pick - 1]["position"]
        );
        assert_eq!(selected[0]["queries"]["output_available"], true);
        assert_eq!(
            selected[0]["stat_set_index"],
            if target == ICE && mode == "CALCS" {
                2
            } else {
                1
            }
        );
    }
}
fn report_bytes(path: &Value, len: &Value, sha: &Value) -> Vec<u8> {
    let path = path.as_str().unwrap();
    assert!(
        path.starts_with("runs/")
            && !path.contains("..")
            && !path.contains('\\')
            && !path.contains(':')
    );
    let len = len.as_u64().unwrap();
    assert!(len > 0 && len <= 128 * 1024 * 1024);
    let path = root().join(path);
    assert_eq!(fs::metadata(&path).unwrap().len(), len);
    let bytes = fs::read(path).unwrap();
    assert_eq!(hash(&bytes), sha.as_str().unwrap());
    bytes
}
pub fn check(a: &Value, b: &Value, v: &Value, require_local: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["context_projection"],
        "physical-context-without-final-output-v1"
    );
    same(&v["scope"], &b["scope"], "bound scope");
    same(&v["scope"], &a["scope"], "authoring scope");
    same(
        &v["scope"],
        &json!({"contribution_factors_only":true,"final_cost_formula":false,"final_duration_formula":false,"application_lifetime_transfer":false,"reservation_formula":false,"owner_closure":"partial","receiving_fragment_only":true}),
        "finite authority",
    );
    same(
        &v["declaration"],
        &json!({"source_duration_stat":STAT,"duration_values":[30,35],"source_mana_multiplier":20,"ordinary_cost_factor":1.2}),
        "source declaration",
    );
    check_definitions(v, b);
    let cert = &v["certificate"];
    same(&cert["case_names"], &names(), "exact cases");
    same(&cert["lifecycle_stages"], &json!(STAGES), "lifecycle");
    for k in ["byte_identical_jit_modes", "raw_byte_identical_jit_modes"] {
        assert_eq!(cert[k], true);
    }
    for prefix in ["report", "raw_report"] {
        assert_eq!(rows(&cert[format!("{prefix}_paths")]).len(), 2);
        assert!(
            cert[format!("{prefix}_bytes")]
                .as_u64()
                .is_some_and(|n| n > 0 && n <= 128 * 1024 * 1024)
        );
        let sha = cert[format!("{prefix}_sha256")].as_str().unwrap();
        assert!(sha.len() == 64 && sha.bytes().all(|b| b.is_ascii_hexdigit()));
    }
    assert_eq!(cert["report_bytes"], cert["raw_report_bytes"]);
    assert_eq!(cert["report_sha256"], cert["raw_report_sha256"]);
    let parts: Vec<_> = PARTS
        .iter()
        .map(|p| fs::read(root().join(p)).unwrap())
        .collect();
    same(
        &cert["observer_parts"],
        &json!(
            PARTS
                .iter()
                .zip(&parts)
                .map(|(p, b)| json!({"path":p,"sha256":hash(b)}))
                .collect::<Vec<_>>()
        ),
        "immutable observer parts",
    );
    let observer = format!(
        "local collect = (function()\n{}\nend)()\nreturn (function(collect)\n{}\nend)(collect)",
        std::str::from_utf8(&parts[0]).unwrap(),
        std::str::from_utf8(&parts[1]).unwrap()
    );
    assert_eq!(cert["observer_sha256"], hash(observer.as_bytes()));
    assert_eq!(rows(&cert["original_sources"]).len(), 5);
    for (i, pin) in rows(&cert["original_sources"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        );
        assert_eq!(pin["path"], path);
        assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
    }
    assert_eq!(rows(&v["controls"]).len(), 12);
    for (c, name) in rows(&v["controls"]).iter().zip(CONTROLS) {
        assert_eq!(c["name"], name);
        contexts(&c["contexts"], &c["control"], name, true);
    }
    if !require_local {
        return;
    }
    let mut previous = None;
    for (i, mode) in ["off", "on"].into_iter().enumerate() {
        let path = cert["report_paths"][i].as_str().unwrap();
        let suffix = format!("/source-jit-{mode}.json");
        let dir = path.strip_suffix(&suffix).unwrap();
        assert_eq!(
            cert["raw_report_paths"][i],
            format!("{dir}/source-jit-{mode}.raw.json")
        );
        let bytes = report_bytes(
            &cert["report_paths"][i],
            &cert["report_bytes"],
            &cert["report_sha256"],
        );
        let raw = report_bytes(
            &cert["raw_report_paths"][i],
            &cert["raw_report_bytes"],
            &cert["raw_report_sha256"],
        );
        assert!(bytes == raw, "raw/compared source differs");
        if let Some(p) = previous {
            assert!(bytes == p, "JIT source differs");
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
        same(
            &r["files"],
            &json!(
                rows(&a["source_files"])
                    .iter()
                    .map(|p| json!({"path":p["path"],"sha256":p["sha256"]}))
                    .collect::<Vec<_>>()
            ),
            "source pins",
        );
        for k in [
            "business_wrappers",
            "source_tables_mutated",
            "source_cfg_modified",
            "native_build_parity",
            "native_inventory_authority",
            "final_duration_formula_authority",
            "final_resource_cost_formula_authority",
            "reservation_formula_authority",
            "effect_application_uptime_authority",
            "final_input_assembly_authority",
            "canonical_parity_lifecycle_selected",
            "original_calculation_calls_captured",
        ] {
            assert_eq!(r[k], false);
        }
        assert_eq!(r["numeric_tolerance"], 0);
        same(
            &json!(
                rows(&r["cases"])
                    .iter()
                    .map(|c| &c["name"])
                    .collect::<Vec<_>>()
            ),
            &names(),
            "complete cases",
        );
        for c in rows(&r["cases"]) {
            assert_eq!(c["independent_source_bindings_verified"], true);
            for stage in STAGES {
                let d = &c["states"][stage]["delivery"];
                for k in [
                    "definitions",
                    "recipient_definitions",
                    "profile_domain",
                    "methods",
                    "global_stat_maps",
                    "source_cost_precision",
                ] {
                    same(&d[k], &v[k], k);
                }
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
                assert_eq!(rows(&d["profile_snapshots"]).len(), 2);
                for snapshot in rows(&d["profile_snapshots"]) {
                    assert_eq!(snapshot["verified"], true);
                    assert!(
                        snapshot["tables"]
                            .as_u64()
                            .is_some_and(|n| n > 0 && n <= 200000)
                    );
                    assert!(snapshot["entries"].as_u64().is_some_and(|n| n <= 4000000));
                }
                if !c["control"].is_null() {
                    contexts(
                        &d["contexts"],
                        &c["control"],
                        c["name"].as_str().unwrap(),
                        false,
                    );
                }
            }
        }
        for (a, b) in (1..=5)
            .map(|i| {
                (
                    format!("original-{i:02}"),
                    format!("repeat-original-{i:02}"),
                )
            })
            .chain([
                ("pain-tier-i".into(), "repeat-pain-tier-i".into()),
                ("ice-tier-i".into(), "repeat-ice-tier-i".into()),
            ])
        {
            let a = named(&r, &a);
            let b = named(&r, &b);
            same(&a["xml_sha256"], &b["xml_sha256"], "repeat XML");
            same(&a["states"], &b["states"], "independent source replay");
        }
        for (index, pin) in rows(&cert["original_sources"]).iter().enumerate() {
            same(
                &named(&r, &format!("original-{:02}", index + 1))["xml_sha256"],
                &pin["sha256"],
                "original source hash",
            );
        }
        for c in rows(&v["controls"]) {
            let actual = named(&r, c["name"].as_str().unwrap());
            for k in ["name", "xml_sha256", "control"] {
                same(&actual[k], &c[k], k);
            }
            same(
                &project(&actual["states"]["fresh"]["delivery"]["contexts"]),
                &c["contexts"],
                "committed source projection",
            );
        }
        previous = Some(bytes);
    }
}
