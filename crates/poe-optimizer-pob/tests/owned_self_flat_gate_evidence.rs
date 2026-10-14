//! Retained original-runtime evidence. These tests authorize no native closure.
use poe_optimizer_pob::source as pinned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

#[path = "support/self_flat_gate_evidence.rs"]
mod gate_evidence;
#[path = "support/json_evidence.rs"]
mod json_evidence;

const PACKET: &str = "data/owned/poe2/3887ae68/self-flat-runtime-gates";
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const XML_SHA: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const REPORT_SHA: &str = "7b3d1971be77acfa1a9166659e7e06df26cf95e669d6b60305e7138940e51640";
const WITNESSES: [&str; 4] = [
    "driver.rs",
    "observer.lua",
    "shared-helper.lua",
    "bootstrap.rs",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn equal(left: &Value, right: &Value) {
    assert_eq!(json_evidence::first_difference(left, right, "$"), None);
}
fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_else(|| {
        assert_eq!(value, &json!({}));
        &[]
    })
}
fn count(state: &Value, key: &str, expected: usize) {
    assert_eq!(rows(&state[key]).len(), expected, "{key}");
}

fn check_state(state: &Value) {
    assert_eq!(
        state["selected"],
        json!({"items":2,"skills":4,"spec":3,"config":1,"main_group":3})
    );
    assert_eq!(state["native_coverage_authorized"], false);
    assert_eq!(state["original_functions_preserved"], true);
    assert_eq!(state["business_methods_replaced"], false);
    assert_eq!(state["query_probes_are_supplemental"], true);
    for (key, n) in [
        ("parent_weapon", 4),
        ("hollow_palm", 18),
        ("flag_calls", 2),
        ("aura_queries", 18),
        ("player_visits", 288),
        ("player_buff_visits", 36),
        ("child_visits", 432),
        ("child_buff_visits", 18),
        ("child_merges", 0),
        ("group_deliveries", 28),
        ("final_raw_stores", 2),
    ] {
        count(state, key, n);
    }
    let empty_flag = |flag: &Value| {
        assert_eq!(flag["query_state_preserved"], true);
        assert_eq!(flag["raw"], json!({}));
        assert_eq!(flag["eligible"], json!({}));
        // The original Flag API returns nil here, not a fabricated false value.
        assert_eq!(flag["supplemental_original_flag"], json!({"present":false}));
    };
    for row in rows(&state["parent_weapon"]) {
        assert_eq!(row["line"], 820);
        assert_eq!(row["actor_is_selected_minion"], true);
        assert_eq!(row["actor_parent_is_player"], true);
        assert_eq!(row["exact_actor_store"], true);
        assert_eq!(row["parent_present"], true);
        assert_eq!(row["parent_main_hand_present"], true);
        assert_eq!(row["original_branch_entered"], false);
        assert_eq!(row["flag"]["name"], "GainMainHandDmgFromParent");
        empty_flag(&row["flag"]);
    }
    for row in rows(&state["hollow_palm"]) {
        assert_eq!(row["line"], 887);
        assert_eq!(row["effect_id"], "MinionMeleeBow");
        assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
        assert_eq!(row["summoner_owns_actor"], true);
        assert_eq!(row["actor_condition"], json!({"present":false}));
        assert_eq!(row["original_branch_entered"], false);
        assert_eq!(row["facebreaker"]["name"], "UseFacebreakerItemDamage");
        assert_eq!(row["hollow_palm"]["name"], "UseHollowPalmDamage");
        empty_flag(&row["facebreaker"]);
        empty_flag(&row["hollow_palm"]);
    }
    for row in rows(&state["flag_calls"]) {
        assert_eq!(
            row["name_from_pinned_callsite"],
            "GainMainHandDmgFromParent"
        );
        assert_eq!(row["original_flag_call_observed"], true);
        assert_eq!(row["line"], 820);
        assert_eq!(row["cfg"], json!({"present":false}));
    }
    for row in rows(&state["aura_queries"]) {
        assert_eq!(row["name"], "ExtraAura");
        assert_eq!(row["line"], 3169);
        for key in [
            "original_consumer_executed",
            "original_list_return_observed",
            "exact_store",
            "store_is_player",
        ] {
            assert_eq!(row[key], true);
        }
        assert_eq!(row["raw"], json!({}));
        assert_eq!(row["returned"], json!({}));
        assert_eq!(row["cfg"], json!({"present":false}));
    }
    for row in rows(&state["player_visits"]) {
        assert_eq!(row["actor_is_player"], true);
        // Archived observer read a missing convenience field. Empty is unavailable
        // diagnostic information, never evidence that effective flags are false.
        assert_eq!(row["flags"], json!({}));
        for buff in rows(&row["buffs"]) {
            assert!(matches!(
                buff["fields"]["type"].as_str(),
                Some("Buff" | "Debuff")
            ));
        }
    }
    let mut child_ids = BTreeSet::new();
    let mut profiles = BTreeSet::new();
    for row in rows(&state["child_visits"]) {
        child_ids.insert(row["effect_id"].as_str().unwrap());
        profiles.insert(row["actor_profile"].as_str().unwrap());
        assert_eq!(row["summoner_owns_actor"], true);
        assert_eq!(row["flags"], json!({}));
        assert_eq!(row["enable"], json!({"present":false}));
        for buff in rows(&row["buffs"]) {
            assert_eq!(buff["fields"]["name"], "Rage");
            assert_eq!(buff["fields"]["type"], "Buff");
            assert_eq!(row["effect_id"], "EnrageSkeletonReaverMinion");
        }
    }
    assert_eq!(child_ids.len(), 16);
    assert_eq!(profiles.len(), 6);
    for row in rows(&state["group_deliveries"]) {
        assert_eq!(row["exact_recipient_store"], true);
        assert!(matches!(
            (row["buff_key"].as_str(), row["recipient"].as_str()),
            (Some("Pain Offering"), Some("selected_minion")) | (Some("Frost Bomb"), Some("enemy"))
        ));
        for modifier in rows(&row["modifiers"]) {
            assert!(matches!(
                modifier["name"].as_str(),
                Some("Speed" | "Damage" | "FireExposure" | "ColdExposure" | "LightningExposure")
            ));
        }
    }
    for row in rows(&state["final_raw_stores"]) {
        assert_eq!(rows(&row["actors"]).len(), 8);
        for actor in rows(&row["actors"]) {
            assert_eq!(actor["raw"], json!({}));
        }
        for modifier in rows(&row["player_raw"]) {
            assert_eq!(modifier["mod"]["name"], "MinionModifier");
        }
    }
}

fn summarize(packet: &Path) -> Value {
    let mut files = Vec::new();
    let mut states = Vec::new();
    let mut metadata = None;
    for mode in ["off", "on"] {
        for label in ["observed", "repeat", "plain"] {
            let name = format!("jit-{mode}-{label}-raw.json");
            let bytes = fs::read(packet.join(&name)).unwrap();
            let mut raw: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(raw["configuration_method_wrappers"], false);
            assert_eq!(raw["original_build_output_available"], true);
            assert_eq!(raw["source_hash"], pinned::manifest_sha256());
            assert!(raw["elapsed_ms"].as_f64().unwrap() > 0.0);
            let state = raw
                .as_object_mut()
                .unwrap()
                .remove("additional_observation")
                .unwrap();
            raw.as_object_mut().unwrap().remove("elapsed_ms");
            if let Some(first) = &metadata {
                equal(first, &raw);
            } else {
                metadata = Some(raw);
            }
            if label != "plain" {
                check_state(&state);
            }
            states.push(state);
            files.push(json!({"path":name,"bytes":bytes.len(),"sha256":hash(&bytes)}));
        }
    }
    for i in [1, 3, 4] {
        equal(
            &gate_evidence::comparison_view(&states[0]),
            &gate_evidence::comparison_view(&states[i]),
        );
    }
    for i in [1, 3, 4] {
        equal(&states[0]["snapshot"], &states[i]["snapshot"]);
    }
    for i in [2, 5] {
        equal(&states[0]["snapshot"], &states[i]);
    }
    let archives: Vec<_> = WITNESSES
        .iter()
        .map(|name| {
            let path = format!("evidence/{name}");
            let bytes = fs::read(packet.join(&path)).unwrap();
            json!({"path":path,"bytes":bytes.len(),"sha256":hash(&bytes)})
        })
        .collect();
    // This is the report written by the original compiled acquisition driver,
    // before the maintained comparator was changed. Bind its witness hashes to
    // the exact archived bytes, rather than treating archives as unauthenticated
    // copies created by the later proof author.
    let acquisition_bytes = fs::read(packet.join("acquisition-jit-on.json")).unwrap();
    let acquisition: Value = serde_json::from_slice(&acquisition_bytes).unwrap();
    equal(&acquisition["state"], &states[3]);
    assert_eq!(acquisition["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(acquisition["xml"]["sha256"], XML_SHA);
    for (index, key) in [
        "driver_sha256",
        "observer_sha256",
        "shared_helpers_file_sha256",
        "bootstrap_sha256",
    ]
    .iter()
    .enumerate()
    {
        assert_eq!(archives[index]["sha256"], acquisition[key]);
    }
    assert_eq!(hash(&fs::read(root().join(XML)).unwrap()), XML_SHA);
    json!({"schema_version":1,"kind":"unchanged-selected-self-flat-runtime-gates", "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "xml":{"path":XML,"sha256":XML_SHA},"raw_runs":files,"archived_witnesses":archives,
        "original_acquisition_report":{"path":"acquisition-jit-on.json","bytes":acquisition_bytes.len(),"sha256":hash(&acquisition_bytes)},"total_fresh_loads":6,
        "instrumented_fresh_loads":4,"instrumented_jit_enabled":false,"plain_controls":{"jit_off":1,"jit_on":1},
        "scope":{"native_coverage_authorized":false,"whole_build_parity":false,"unchanged_build_only":true,"query_probes_are_supplemental":true,
            "actual_list_returns_observed":true,"traversal_flags_available":false,"raw_order_retained":true,"scalar_comparison":"exact across all six loads"},
        "comparison":{"inventory_paths":["group_deliveries[*].modifiers","player_buff_visits[*].buff.modifiers","player_visits[*].buffs[*].modifiers"],
            "policy":"stable stat-name buckets; preserve full records, multiplicity and within-stat order","all_other_observation_fields":"exact",
            "off_repeat_reordered_lists":gate_evidence::reviewed_reordering_count(&states[0],&states[1]),
            "on_repeat_reordered_lists":gate_evidence::reviewed_reordering_count(&states[3],&states[4]),
            "cross_mode_reordered_lists":gate_evidence::reviewed_reordering_count(&states[0],&states[3])},
        "observed_counts":{"parent_weapon_visits":4,"hollow_palm_visits":18,"actual_parent_flag_calls":2,"actual_hollow_flag_calls":0,
            "actual_extra_aura_queries":18,"actual_extra_aura_effect_queries":0,"player_visits":288,"child_visits":432,
            "child_unique_effect_ids":16,"child_unique_profiles":6,"child_buff_visits":18,"child_merges":0,"group_deliveries":28}})
}

#[test]
#[ignore = "offline retention only; reads the already captured six original runs"]
fn retain_six_original_gate_runs() {
    let from = PathBuf::from(
        std::env::var_os("POE_SELF_FLAT_GATE_SOURCE_OUT")
            .expect("existing raw acquisition directory"),
    );
    let packet = root().join(PACKET);
    for mode in ["off", "on"] {
        for label in ["observed", "repeat", "plain"] {
            let name = format!("jit-{mode}-{label}-raw.json");
            fs::copy(from.join(&name), packet.join(&name)).unwrap();
        }
    }
    fs::copy(
        from.join("source-jit-on.json"),
        packet.join("acquisition-jit-on.json"),
    )
    .unwrap();
    let report = serde_json::to_vec_pretty(&summarize(&packet)).unwrap();
    fs::write(packet.join("report.json"), &report).unwrap();
    eprintln!("retained report SHA256 {}", hash(&report));
}

#[test]
fn retained_gate_runs_and_archived_witnesses_are_authenticated() {
    let packet = root().join(PACKET);
    let bytes = fs::read(packet.join("report.json")).unwrap();
    assert_eq!(hash(&bytes), REPORT_SHA);
    equal(
        &serde_json::from_slice(&bytes).unwrap(),
        &summarize(&packet),
    );
}

fn retained_state() -> Value {
    read(root().join(PACKET).join("jit-off-observed-raw.json"))["additional_observation"].clone()
}

#[test]
fn comparison_preserves_same_stat_order_multiplicity_and_records() {
    let mut a = retained_state();
    let mods = a["group_deliveries"][0]["modifiers"]
        .as_array_mut()
        .unwrap();
    let mut same_stat = mods[0].clone();
    same_stat["value"] = json!(71);
    mods.push(same_stat);
    let baseline = gate_evidence::comparison_view(&a);
    let mut changed = a.clone();
    changed["group_deliveries"][0]["modifiers"]
        .as_array_mut()
        .unwrap()
        .swap(0, 2);
    assert_ne!(baseline, gate_evidence::comparison_view(&changed));
    let mut duplicate = a.clone();
    let first = duplicate["group_deliveries"][0]["modifiers"][0].clone();
    duplicate["group_deliveries"][0]["modifiers"]
        .as_array_mut()
        .unwrap()
        .push(first);
    assert_ne!(baseline, gate_evidence::comparison_view(&duplicate));
    let duplicate_baseline = gate_evidence::comparison_view(&duplicate);
    duplicate["group_deliveries"][0]["modifiers"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_ne!(
        duplicate_baseline,
        gate_evidence::comparison_view(&duplicate)
    );
    for key in ["value", "source", "flags", "tags"] {
        let mut changed = a.clone();
        changed["group_deliveries"][0]["modifiers"][0][key] = json!("changed");
        assert_ne!(baseline, gate_evidence::comparison_view(&changed));
    }
    let mut changed = a.clone();
    changed["group_deliveries"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert_ne!(baseline, gate_evidence::comparison_view(&changed));
    let mut reordered = a.clone();
    reordered["group_deliveries"][0]["modifiers"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert_eq!(baseline, gate_evidence::comparison_view(&reordered));
    assert_eq!(a["group_deliveries"][0]["modifiers"][0]["name"], "Speed");
}

#[test]
fn raw_candidates_false_flags_missing_context_and_coverage_claims_are_refused() {
    let original = retained_state();
    let mut explicit_false = original.clone();
    explicit_false["parent_weapon"][0]["flag"]["supplemental_original_flag"] =
        json!({"present":true,"value":false});
    assert!(std::panic::catch_unwind(|| check_state(&explicit_false)).is_err());
    let mut zero_candidate = original.clone();
    zero_candidate["parent_weapon"][0]["flag"]["raw"] = json!([{"ancestor_depth":0,"mod":{
        "name":"GainMainHandDmgFromParent","type":"FLAG","value":0,"source":"mutation-control",
        "flags":0,"keyword_flags":0,"tags":{}}}]);
    assert!(std::panic::catch_unwind(|| check_state(&zero_candidate)).is_err());
    for path in [
        "/native_coverage_authorized",
        "/original_functions_preserved",
        "/parent_weapon/0/exact_actor_store",
        "/parent_weapon/0/original_branch_entered",
        "/hollow_palm/0/actor_condition",
        "/hollow_palm/0/hollow_palm/raw",
        "/aura_queries/0/raw",
        "/aura_queries/0/original_consumer_executed",
        "/parent_weapon/0/flag/supplemental_original_flag",
        "/child_visits/0/enable",
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = json!({"value":0,"present":true});
        assert!(
            std::panic::catch_unwind(|| check_state(&changed)).is_err(),
            "{path}"
        );
    }
    let mut changed = original.clone();
    changed["child_visits"].as_array_mut().unwrap().remove(0);
    assert!(std::panic::catch_unwind(|| check_state(&changed)).is_err());
    let mut changed = original.clone();
    changed["aura_queries"][0]["name"] = json!("ExtraAuraEffect");
    assert!(std::panic::catch_unwind(|| check_state(&changed)).is_err());
    let baseline = gate_evidence::comparison_view(&original);
    for path in [
        "/hollow_palm/0/cfg/fields/flags",
        "/flag_calls/0/gate_index",
        "/group_deliveries/0/recipient",
    ] {
        let mut changed = original.clone();
        *changed.pointer_mut(path).unwrap() = json!("changed");
        assert_ne!(baseline, gate_evidence::comparison_view(&changed));
    }
}
