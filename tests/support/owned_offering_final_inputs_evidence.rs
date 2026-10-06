//! Offline source receipts for finite physical input assembly. No runtime Lua,
//! recovery law, complete contributor inventory or complete-build authority.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const I: &str = "ProlongedDurationSupportPlayer";
const II: &str = "ProlongedDurationSupportPlayerTwo";
const OBSERVER: &str = "crates/poe-optimizer-pob/tests/support/offering_property_source.lua";
const LIFECYCLE: &str = "crates/poe-optimizer-pob/tests/support/djinn_provider_source.lua";
const SELECTED: [&str; 13] = [
    "original-05",
    "focused-tier-i",
    "archived-tier-ii",
    "support-remove",
    "support-disable",
    "raw-level",
    "raw-quality",
    "distinct-offering-copies",
    "fractional-raw-level-diagnostic",
    "helmet-level-remove",
    "amulet-level-remove",
    "amulet-copy-bonus",
    "both-level-remove",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn rows(v: &Value) -> &[Value] {
    if v.as_object().is_some_and(|o| o.is_empty()) {
        &[]
    } else {
        v.as_array().expect("source array or explicit empty table")
    }
}
fn same(a: &Value, b: &Value, label: &str) {
    assert!(a == b, "{label} differs");
}
fn case<'a>(r: &'a Value, name: &str) -> &'a Value {
    let a: Vec<_> = rows(&r["cases"])
        .iter()
        .filter(|c| c["name"] == name)
        .collect();
    assert_eq!(a.len(), 1, "source case {name}");
    a[0]
}
fn case_names() -> Value {
    let mut names: Vec<_> = (1..=5).map(|i| format!("original-{i:02}")).collect();
    names.extend(
        [
            "focused-tier-i",
            "archived-tier-ii",
            "support-remove",
            "support-disable",
            "raw-level",
            "raw-quality",
            "distinct-offering-copies",
            "fractional-raw-level-diagnostic",
            "repeat-focused-tier-i",
            "repeat-distinct-offering-copies",
            "helmet-level-remove",
            "amulet-level-remove",
            "amulet-copy-bonus",
            "both-level-remove",
        ]
        .map(str::to_owned),
    );
    names.extend((1..=5).map(|i| format!("repeat-original-{i:02}")));
    names.extend([
        "unhooked-original-05".to_owned(),
        "unhooked-distinct-offering-copies".to_owned(),
    ]);
    json!(names)
}
fn project(s: &Value) -> Value {
    let mut s = s.clone();
    let m = s.as_object_mut().unwrap();
    assert!(m.remove("definition").is_some());
    assert!(m.remove("methods").is_some());
    s
}
fn point(table: &Value, index: u64) -> &Value {
    let found: Vec<_> = rows(&table["positions"])
        .iter()
        .filter(|p| p["index"] == index)
        .collect();
    assert_eq!(found.len(), 1);
    &found[0]["value"]
}
fn definitions(v: &Value, b: &Value) {
    let d = &v["definition"];
    assert_eq!(d["effect"], "PainOfferingPlayer");
    for key in ["levels", "stat_set_levels"] {
        assert_eq!(rows(&d[key]["positions"]).len(), 40);
        for (i, p) in rows(&d[key]["positions"]).iter().enumerate() {
            assert_eq!(p["index"], i + 1);
            assert!(p["value"].is_object());
        }
    }
    assert_eq!(point(&d["levels"], 22)["cost"]["Mana"], 122);
    assert_eq!(point(point(&d["stat_set_levels"], 22), 2), &json!(62));
    same(
        &b["reviewed_level_domain"],
        &json!({"minimum":1,"maximum":40,"integer_required":true,"outside":"no-projected-value"}),
        "finite integral lookup",
    );
    assert_eq!(rows(&b["supports"]).len(), 2);
    for (support, effect) in rows(&b["supports"]).iter().zip([I, II]) {
        assert_eq!(support["source_effect"], effect);
        assert!(rows(&support["property_programs"]).is_empty());
    }
    for (name, path, first) in [
        ("ordinary", "Modules/CalcSetup.lua", 550),
        ("supported", "Modules/CalcActiveSkill.lua", 236),
        ("validation", "Modules/CalcTools.lua", 60),
        ("merge", "Modules/CalcActiveSkill.lua", 116),
        ("assembly", "Modules/CalcActiveSkill.lua", 426),
        ("tabulate", "Classes/ModStore.lua", 345),
        ("scale", "Classes/ModStore.lua", 82),
        ("add_database", "Classes/ModDB.lua", 31),
    ] {
        let m = &v["methods"][name];
        assert_eq!(m["path"], path);
        assert_eq!(m["first"], first);
        assert!(m["last"].as_u64().unwrap() >= first);
    }
    assert_eq!(v["methods"].as_object().unwrap().len(), 8);
}
fn context(c: &Value, s: &Value, definition: &Value) {
    for key in [
        "actor_is_player",
        "exact_source",
        "no_attached_minion",
        "no_summoning_parent",
        "distinct_source_cache",
    ] {
        assert_eq!(c[key], true);
    }
    assert_eq!(
        c["source_catalog"],
        "Metadata/Items/Gems/SkillGemPainOffering"
    );
    assert_eq!(c["disabled"], false);
    let level = c["final"]["level"]
        .as_u64()
        .expect("observed integral final level");
    assert!((1..=40).contains(&level));
    same(
        &c["final_root_level"],
        point(&definition["levels"], level),
        "actual root lookup",
    );
    same(
        &c["final_stat_set_level"],
        point(&definition["stat_set_levels"], level),
        "actual stat-set lookup",
    );
    let mut final_row = c["final_root_level"].clone();
    for (k, value) in c["final_stat_set_level"].as_object().unwrap() {
        final_row[k] = value.clone();
    }
    same(&c["final_row"], &final_row, "original merged level row");
    let ordinary = rows(&c["ordinary"]);
    assert_eq!(ordinary.len(), 1);
    let ordinary = &ordinary[0];
    assert_eq!(ordinary["query_store_is_actor"], true);
    let mut delta = [0.0; 2];
    let mut partition = BTreeSet::new();
    for (key, admitted) in [("matched", true), ("rejected", false)] {
        for index in rows(&ordinary[key]) {
            let i = index.as_u64().unwrap() as usize;
            assert!(i > 0 && partition.insert(i));
            let row = &ordinary["candidates"][i - 1];
            assert_eq!(row["mod"]["name"], "GemProperty");
            same(&row["value"], &row["mod"]["value"], "actual query payload");
            if admitted {
                let channel = match row["value"]["key"].as_str().unwrap() {
                    "level" => 0,
                    "quality" => 1,
                    other => panic!("unreviewed ordinary property {other}"),
                };
                delta[channel] += row["value"]["value"].as_f64().unwrap();
            }
            for index in rows(&ordinary["copy_indices"][i - 1]) {
                let copy = &s["copies"][index.as_u64().unwrap() as usize - 1];
                assert!(
                    rows(&copy["insertions"])
                        .iter()
                        .any(|r| r["record"] == row["mod"]),
                    "actual copied object correspondence"
                );
            }
        }
    }
    assert_eq!(partition.len(), rows(&ordinary["candidates"]).len());
    for (i, key) in ["level", "quality"].into_iter().enumerate() {
        assert_eq!(
            ordinary["after"][key].as_f64().unwrap(),
            ordinary["before"][key].as_f64().unwrap() + delta[i]
        );
    }
    let supported = rows(&c["supported"]);
    assert!(!supported.is_empty());
    for call in supported {
        assert_eq!(call["query_parent_is_actor"], true);
        assert_eq!(call["cache_identity_preserved"], true);
        same(
            &call["before"],
            &call["after"],
            "property query is not application",
        );
        assert!(
            rows(&call["properties"]).is_empty(),
            "finite Offering component admits no additional supported-property producer"
        );
        for support in rows(&call["supports"]) {
            assert_eq!(support["exact_source"], true);
            assert_ne!(
                support["source"]["source_ordinal"],
                c["source"]["source_ordinal"]
            );
        }
    }
    let assemblies = rows(&c["assembly"]);
    assert_eq!(assemblies.len(), 1);
    let a = &assemblies[0];
    same(&a["before"], &ordinary["after"], "ordinary to assembly");
    same(&a["after"], &c["final"], "assembly to final");
    let merges = rows(&a["merges"]);
    assert_eq!(merges.len(), 1);
    assert_eq!(merges[0]["exact_destination"], true);
    assert_eq!(merges[0]["stat_set_exact"], true);
    same(
        &merges[0]["before"],
        &a["before"],
        "empty supported-property application",
    );
    assert!(!rows(&c["validation"]).is_empty());
    for validation in rows(&c["validation"]) {
        assert_eq!(validation["before_lookup"], true);
        assert_eq!(validation["after_lookup"], true);
        same(
            &validation["before"],
            &validation["after"],
            "finite integral level requires no source recovery",
        );
    }
}
fn observation(name: &str, s: &Value, control: &Value, definition: &Value, b: &Value) {
    for key in [
        "instrumented",
        "actual_original_calls_observed",
        "original_methods_preserved",
        "hook_removed",
        "jit_mode_preserved",
    ] {
        assert_eq!(s[key], true);
    }
    for key in ["business_wrappers", "source_tables_mutated"] {
        assert_eq!(s[key], false);
    }
    for c in rows(&s["contexts"]) {
        context(c, s, definition);
    }
    let (levels, qualities, winner): (&[u64], &[u64], Option<&str>) = match name {
        "raw-level" => (&[7], &[0], Some(I)),
        "raw-quality" => (&[22], &[13], Some(I)),
        "distinct-offering-copies" | "repeat-distinct-offering-copies" => {
            (&[22, 7], &[0, 13], Some(I))
        }
        "helmet-level-remove" | "amulet-level-remove" => (&[21], &[0], Some(I)),
        "both-level-remove" => (&[20], &[0], Some(I)),
        "amulet-copy-bonus" => (&[23], &[0], Some(I)),
        "support-remove" | "support-disable" => (&[22], &[0], None),
        "archived-tier-ii" => (&[22], &[0], Some(II)),
        _ => (&[22], &[0], Some(I)),
    };
    for mode in ["MAIN", "CALCS"] {
        let contexts: Vec<_> = rows(&s["contexts"])
            .iter()
            .filter(|c| c["mode"] == mode)
            .collect();
        assert_eq!(contexts.len(), levels.len());
        let mut origins = BTreeSet::new();
        for (i, c) in contexts.iter().enumerate() {
            assert_eq!(c["final"]["level"], levels[i]);
            assert_eq!(c["final"]["quality"], qualities[i]);
            assert!(origins.insert(c["source"]["source_ordinal"].as_u64().unwrap()));
            let candidates: Vec<_> = rows(&c["supported"][0]["supports"])
                .iter()
                .filter(|p| {
                    p["is_supporting"] == true
                        && [Some(I), Some(II)].contains(&p["effect"].as_str())
                })
                .collect();
            assert_eq!(candidates.len(), usize::from(winner.is_some()));
            if let Some(p) = candidates.first() {
                assert_eq!(p["effect"], winner.unwrap());
                assert_eq!(p["admitted"], true);
            }
        }
    }
    if name == "fractional-raw-level-diagnostic" {
        assert!(
            rows(&s["contexts"])
                .iter()
                .any(|c| rows(&c["source_validation"])
                    .iter()
                    .any(|v| v["before_lookup"] == false && v["before"] != v["after"])),
            "original loader recovery remains diagnostic"
        );
    }
    if name == "original-05" {
        assert!(control.is_null());
        for c in rows(&s["contexts"]) {
            let mode = c["mode"].as_str().unwrap();
            assert_eq!(c["raw"]["level"], 20);
            assert_eq!(c["raw"]["quality"], 0);
            let ordinary = &c["ordinary"][0];
            assert_eq!(rows(&ordinary["matched"]).len(), 3);
            assert!(rows(&ordinary["rejected"]).is_empty());
            for (slot, key) in [("Helmet", "helmet"), ("Amulet", "amulet")] {
                let item = &s["equipped"][mode][slot];
                assert_eq!(item["present"], true);
                assert_eq!(item["exact_registered"], true);
                same(
                    &item["id"],
                    &b["ordinary_items"]["source_item_ids"][key],
                    "original item binding",
                );
                let prefix = format!("Item:{}:", item["id"].as_u64().unwrap());
                let records: Vec<_> = rows(&ordinary["candidates"])
                    .iter()
                    .filter(|r| {
                        r["mod"]["source"]
                            .as_str()
                            .is_some_and(|p| p.starts_with(&prefix))
                    })
                    .collect();
                assert_eq!(records.len(), 1);
                let m = &records[0]["mod"];
                assert_eq!(m["sourceSlot"], slot);
                same(
                    &m["value"],
                    &json!({"key":"level","keyOfScaledMod":"value","keyword":"minion","value":1}),
                    "equipped minion property",
                );
            }
            let zeros: Vec<_> = rows(&ordinary["candidates"])
                .iter()
                .enumerate()
                .filter(|(_, r)| r["value"]["value"] == 0)
                .collect();
            assert_eq!(zeros.len(), 1);
            let joins = rows(&ordinary["copy_indices"][zeros[0].0]);
            assert_eq!(joins.len(), 1);
            let copy = &s["copies"][joins[0].as_u64().unwrap() as usize - 1];
            assert_eq!(copy["scale"], 0);
            assert_eq!(copy["amulet"]["id"], s["equipped"][mode]["Amulet"]["id"]);
            for key in [
                "exact_registered",
                "copied_from_actual_list",
                "exact_copy_argument",
                "exact_destination",
            ] {
                assert_eq!(copy["amulet"][key], true);
            }
            same(
                &copy["before"],
                &copy["input_after"],
                "copy source unchanged",
            );
        }
    }
}
fn report(path: &Value, len: &Value, sha: &Value) -> Vec<u8> {
    let path = path.as_str().unwrap();
    assert!(
        path.starts_with("runs/")
            && !path.contains("..")
            && !path.contains('\\')
            && !path.contains(':')
    );
    let length = len.as_u64().unwrap();
    assert!(length > 0 && length <= 128 * 1024 * 1024);
    let path = root().join(path);
    assert_eq!(fs::metadata(&path).unwrap().len(), length);
    let bytes = fs::read(path).unwrap();
    assert_eq!(hash(&bytes), sha.as_str().unwrap());
    bytes
}

/// Committed vectors contain one definition/method inventory and thirteen fresh
/// stage projections with only those two repeated fields removed. Full receipt
/// validation preserves every raw report byte and all repeat/JIT comparisons.
pub fn check(a: &Value, b: &Value, v: &Value, require_local: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "passed");
    assert_eq!(
        v["context_projection"],
        "property-consumption-contexts-and-copy-joins-v1"
    );
    same(&v["scope"], &b["scope"], "bound evidence scope");
    same(&v["scope"], &a["scope"], "authoring scope");
    for k in [
        "evaluation_bundle",
        "full_owner_closure",
        "full_build_parity",
    ] {
        assert_eq!(v["scope"][k], false);
    }
    assert_eq!(v["scope"]["source_recovery_fallback"], "not-authored");
    assert_eq!(v["scope"]["ordinary_contributor_inventory"], "partial");
    assert_eq!(
        v["scope"]["pre_amulet_snapshot"],
        "required-unresolved-input"
    );
    same(
        &v["declaration"],
        &json!({"ordinary_level_query":"GemProperty","supported_property_query":"SupportedGemProperty","physical_effect":"PainOfferingPlayer","reviewed_lookup_domain":(1..=40).collect::<Vec<_>>(),"natural_maximum_fallback_authored":false}),
        "finite declaration",
    );
    definitions(v, b);
    let cert = &v["certificate"];
    same(&cert["case_names"], &case_names(), "exact source cases");
    same(&cert["lifecycle_stages"], &json!(STAGES), "fixed lifecycle");
    assert_eq!(cert["byte_identical_jit_modes"], true);
    assert_eq!(cert["raw_byte_identical_jit_modes"], true);
    assert_eq!(
        cert["observer_sha256"],
        hash(&fs::read(root().join(OBSERVER)).unwrap())
    );
    assert_eq!(
        cert["lifecycle_sha256"],
        hash(&fs::read(root().join(LIFECYCLE)).unwrap())
    );
    assert_eq!(rows(&cert["original_sources"]).len(), 5);
    for (i, pin) in rows(&cert["original_sources"]).iter().enumerate() {
        let path = format!(
            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
            i + 1
        );
        assert_eq!(pin["path"], path);
        assert_eq!(pin["sha256"], hash(&fs::read(root().join(path)).unwrap()));
    }
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let normalized = String::from_utf8(manifest_bytes)
        .unwrap()
        .replace("\r\n", "\n");
    assert_eq!(
        hash(normalized.as_bytes()),
        a["source_manifest_sha256"].as_str().unwrap()
    );
    let manifest: Value = serde_json::from_str(&normalized).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    for pin in rows(&a["source_files"]) {
        assert_eq!(
            rows(&manifest["files"])
                .iter()
                .filter(|m| m["path"] == pin["path"]
                    && m["sha256"] == pin["sha256"]
                    && m["bytes"] == pin["bytes"])
                .count(),
            1
        );
    }
    assert_eq!(rows(&v["observations"]).len(), SELECTED.len());
    for (o, name) in rows(&v["observations"]).iter().zip(SELECTED) {
        assert_eq!(o["case"], name);
        assert_eq!(o["stage"], "fresh");
        observation(name, &o["value"], &o["control"], &v["definition"], b);
    }
    let reports = rows(&v["reports"]);
    assert_eq!(reports.len(), 2);
    for (i, p) in reports.iter().enumerate() {
        assert_eq!(p["mode"], if i == 0 { "off" } else { "on" });
        same(&p["bytes"], &reports[0]["bytes"], "JIT report bytes");
        same(&p["sha256"], &reports[0]["sha256"], "JIT report digest");
        same(&p["bytes"], &p["raw_bytes"], "raw report bytes");
        same(&p["sha256"], &p["raw_sha256"], "raw report digest");
    }
    if !require_local {
        return;
    }
    let mut previous = None;
    for p in reports {
        let bytes = report(&p["path"], &p["bytes"], &p["sha256"]);
        let raw = report(&p["raw_path"], &p["raw_bytes"], &p["raw_sha256"]);
        assert!(bytes == raw, "raw report differs");
        if let Some(previous) = previous {
            assert!(bytes == previous, "JIT report differs");
        }
        let r: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(r["schema_version"], 1);
        assert_eq!(r["source_revision"], a["source_revision"]);
        assert_eq!(r["manifest_sha256"], a["source_manifest_sha256"]);
        for k in [
            "observer_sha256",
            "lifecycle_sha256",
            "original_sources",
            "lifecycle_stages",
        ] {
            same(&r[k], &cert[k], k);
        }
        same(
            &json!(
                rows(&r["cases"])
                    .iter()
                    .map(|c| &c["name"])
                    .collect::<Vec<_>>()
            ),
            &case_names(),
            "full case inventory",
        );
        for pin in rows(&r["files"]) {
            assert_eq!(
                rows(&manifest["files"])
                    .iter()
                    .filter(|m| m["path"] == pin["path"] && m["sha256"] == pin["sha256"])
                    .count(),
                1
            );
        }
        for pin in rows(&a["source_files"]) {
            assert_eq!(
                rows(&r["files"])
                    .iter()
                    .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                    .count(),
                1
            );
        }
        for k in [
            "business_wrappers",
            "source_tables_mutated",
            "diagnostic_queries_as_consumption",
            "native_build_parity",
            "native_inventory_authority",
            "fallback_semantics_authority",
            "game_level_cap_authority",
            "fractional_game_domain_authority",
            "curve_approximation_authority",
        ] {
            assert_eq!(r[k], false);
        }
        assert_eq!(r["numeric_tolerance"], 0);
        for c in rows(&r["cases"]) {
            assert_eq!(c["independent_source_bindings_verified"], true);
            for stage in STAGES {
                let s = &c["states"][stage];
                same(
                    &s["definition"],
                    &v["definition"],
                    "stable source definition",
                );
                same(&s["methods"], &v["methods"], "original methods");
                for k in [
                    "original_methods_preserved",
                    "hook_removed",
                    "jit_mode_preserved",
                ] {
                    assert_eq!(s[k], true);
                }
                for k in ["business_wrappers", "source_tables_mutated"] {
                    assert_eq!(s[k], false);
                }
                if c["instrumented"] == true {
                    for context_row in rows(&s["contexts"]) {
                        context(context_row, s, &v["definition"]);
                    }
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
                ("focused-tier-i".into(), "repeat-focused-tier-i".into()),
                (
                    "distinct-offering-copies".into(),
                    "repeat-distinct-offering-copies".into(),
                ),
            ])
        {
            same(
                &case(&r, &a)["xml_sha256"],
                &case(&r, &b)["xml_sha256"],
                "repeat source",
            );
            same(
                &case(&r, &a)["states"],
                &case(&r, &b)["states"],
                "independent source replay",
            );
        }
        for name in ["original-05", "distinct-offering-copies"] {
            let h = case(&r, name);
            let u = case(&r, &format!("unhooked-{name}"));
            assert_eq!(u["instrumented"], false);
            same(&h["xml_sha256"], &u["xml_sha256"], "unhooked input");
            for stage in STAGES {
                same(
                    &h["states"][stage]["outcomes"],
                    &u["states"][stage]["outcomes"],
                    "unhooked outcome",
                );
            }
        }
        for (i, pin) in rows(&cert["original_sources"]).iter().enumerate() {
            same(
                &case(&r, &format!("original-{:02}", i + 1))["xml_sha256"],
                &pin["sha256"],
                "original input bytes",
            );
        }
        for o in rows(&v["observations"]) {
            let c = case(&r, o["case"].as_str().unwrap());
            same(&c["xml_sha256"], &o["xml_sha256"], "projected input");
            same(&c["control"], &o["control"], "projected control");
            same(
                &project(&c["states"]["fresh"]),
                &o["value"],
                "exact committed projection",
            );
        }
        previous = Some(bytes);
    }
}
