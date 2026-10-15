//! Pure assessment of fixed captured loads; no source invocation or semantic coverage.
use serde_json::{Value as Json, json};
fn rows(value: &Json) -> &[Json] {
    if value.as_object().is_some_and(serde_json::Map::is_empty) {
        return &[];
    }
    value.as_array().unwrap()
}
fn text(value: &Json) {
    assert!(value.as_str().is_some_and(|value| !value.is_empty()));
}
fn scalars(value: &Json, nonempty: bool) {
    let values = value.as_object().expect("captured scalar map");
    assert!(!nonempty || !values.is_empty(), "required scalar output");
    assert!(
        values
            .values()
            .all(|v| v.is_boolean() || v.is_string() || v.is_number())
    );
}
fn output(value: &Json, required: bool) {
    let available = value["available"].as_bool().expect("output availability");
    assert!(!required || available, "required output unavailable");
    scalars(&value["scalars"], available);
    if !available {
        assert!(value["scalars"].as_object().unwrap().is_empty());
    }
}
fn source(value: &Json) {
    assert_eq!(value["source_present"], true);
    let matches = value["matches"]
        .as_array()
        .expect("source occurrence matches");
    assert!(!matches.is_empty());
    for record in matches {
        for name in ["group_index", "gem_index", "raw_level"] {
            assert!(record[name].as_u64().is_some_and(|v| v > 0));
        }
        assert!(record["raw_quality"].is_number());
        text(&record["gem_id"]);
        text(&record["physical_gem_id"]);
        for name in [
            "source_enabled",
            "group_enabled",
            "group_is_active_socket_group",
        ] {
            assert!(record[name].is_boolean());
        }
    }
}
fn validate_snapshot(snapshot: &Json) {
    assert_eq!(snapshot.as_object().expect("MAIN/CALCS snapshot").len(), 2);
    for (key, mode) in [("main", "MAIN"), ("calcs", "CALCS")] {
        let frame = &snapshot[key];
        assert_eq!(frame["mode"], mode);
        assert!(frame["main_group"].as_u64().is_some_and(|v| v > 0));
        output(&frame["player_output"], true);
        output(&frame["cached_output"], true);
        let selected = &frame["player_selected"];
        text(&selected["effect_id"]);
        source(&selected["source_occurrence"]);
        output(&selected["output"], false);
        let matches = frame["selected_minion"]["matches"]
            .as_array()
            .expect("selected minion identity");
        assert_eq!(matches.len(), 1, "one exact selected minion");
        let actors = frame["actors"].as_array().expect("minion actor inventory");
        assert!(!actors.is_empty());
        let mut selected_count = 0;
        let mut ordinals = std::collections::BTreeSet::new();
        for actor in actors {
            let ordinal = actor["ordinal"].as_u64().expect("actor occurrence ordinal");
            assert!(ordinal > 0 && ordinals.insert(ordinal));
            text(&actor["actor_profile"]);
            source(&actor["source_occurrence"]);
            let is_selected = actor["selected"].as_bool().expect("actor selection");
            output(&actor["output"], is_selected);
            let children = actor["children"].as_array().expect("actor skill inventory");
            assert!(!children.is_empty());
            let mut selected_children = 0;
            for (index, child) in children.iter().enumerate() {
                assert_eq!(child["index"], index + 1);
                text(&child["effect_id"]);
                output(&child["output"], false);
                if child["selected"].as_bool().expect("child selection") {
                    selected_children += 1;
                    // This fixed unchanged-build capture includes the selected MAIN
                    // Sniper's raw preconversion snapshot, even though child.output
                    // itself is absent in PoB. Do not substitute absence for it.
                    if is_selected && key == "main" {
                        let preconversion = &child["preconversion"];
                        for name in ["enemy", "environment_skill_data", "skill"] {
                            rows(&preconversion["raw"][name]);
                        }
                        scalars(&preconversion["skill_data"], true);
                        scalars(&preconversion["weapon1"], true);
                        scalars(&preconversion["weapon2"], false);
                    }
                }
            }
            assert_eq!(selected_children, 1);
            if is_selected {
                selected_count += 1;
                assert_eq!(matches[0]["ordinal"], ordinal);
                assert_eq!(matches[0]["effect_id"], selected["effect_id"]);
                assert_eq!(matches[0]["source"], actor["source_occurrence"]);
                assert_eq!(actor["source_occurrence"], selected["source_occurrence"]);
            }
        }
        assert_eq!(selected_count, 1);
    }
}
fn validate_trace(trace: &Json) {
    assert_eq!(trace["kind"], "original-inherited-minion-modifier-delivery");
    assert_eq!(trace["original_functions_preserved"], true);
    assert_eq!(trace["observer_removed"], true);
    assert_eq!(trace["business_methods_replaced"], false);
    assert_eq!(trace["scope"]["native_coverage_authorized"], false);
    let calls = rows(&trace["perform_calls"]);
    assert!(!calls.is_empty());
    for (index, call) in calls.iter().enumerate() {
        assert_eq!(call["perform_id"], index + 1);
        assert_eq!(call["returned"], true);
    }
    let deliveries = rows(&trace["deliveries"]);
    assert!(!deliveries.is_empty());
    for (index, call) in deliveries.iter().enumerate() {
        assert_eq!(call["delivery_id"], index + 1);
        let id = call["perform_id"].as_u64().unwrap() as usize;
        assert!(id > 0 && id <= calls.len());
        assert_eq!(call["env_id"], calls[id - 1]["env_id"]);
        assert_eq!(call["mode"], calls[id - 1]["mode"]);
        assert!(matches!(
            call["line"].as_u64(),
            Some(1854 | 3123 | 3377 | 3384)
        ));
        assert_eq!(call["returned"], true);
        assert_eq!(call["original_list_return_observed"], true);
        assert_eq!(call["original_list_call_count"], 1);
        assert_eq!(call["cfg"]["present"], true);
        for (at, added) in rows(&call["added"]).iter().enumerate() {
            assert_eq!(added["position"], at + 1);
            assert_eq!(added["original_append_observed"], true);
            assert_eq!(
                added["after_count"].as_u64().unwrap(),
                added["before_count"].as_u64().unwrap() + 1
            );
        }
    }
    let gates = rows(&trace["gates"]);
    for gate in gates {
        let id = gate["perform_id"].as_u64().unwrap() as usize;
        assert!(id > 0 && id <= calls.len());
        assert_eq!(gate["env_id"], calls[id - 1]["env_id"]);
        assert_eq!(gate["mode"], calls[id - 1]["mode"]);
    }
    for call in calls {
        // A loop header can have entry, iteration and termination events.
        // Keep all events; only business calls have unique delivery identities.
        for line in [1844, 2037, 3117, 3397, 3406] {
            assert!(
                gates
                    .iter()
                    .any(|r| r["perform_id"] == call["perform_id"] && r["line"] == line),
                "required gate {line} for perform {}",
                call["perform_id"]
            );
        }
    }
    for pool in rows(&trace["life_pool_calls"]) {
        let id = pool["perform_id"].as_u64().unwrap() as usize;
        assert!(id > 0 && id <= calls.len());
        assert_eq!(pool["env_id"], calls[id - 1]["env_id"]);
        assert_eq!(pool["mode"], calls[id - 1]["mode"]);
        assert_eq!(pool["returned"], true);
    }
}

pub fn assess(loads: &[Json]) -> Json {
    assert_eq!(loads.len(), 4, "fixed four complete captures");
    let a = &loads[0]["additional_observation"];
    let b = &loads[1]["additional_observation"];
    validate_trace(a);
    validate_trace(b);
    validate_snapshot(&a["snapshot"]);
    for (label, value) in [
        ("independent observed scalar output", &b["snapshot"]),
        (
            "unhooked JIT-off scalar output",
            &loads[2]["additional_observation"],
        ),
        (
            "unhooked JIT-on scalar output",
            &loads[3]["additional_observation"],
        ),
    ] {
        validate_snapshot(value);
        assert!(&a["snapshot"] == value, "{label} differs");
    }
    json!({"all_scalar_outputs_exact":true,
        "comparison":{"all_trace_fields_exact":a==b,"raw_list_and_addmod_order_retained":true,
            "offline_receiving_proof_required":true,"semantic_receiving_proof_executed":false,
            "native_order_authority":false},
        "counts":{"perform_calls":rows(&a["perform_calls"]).len(),
            "deliveries":rows(&a["deliveries"]).len(),"gates":rows(&a["gates"]).len(),
            "life_pool_calls":rows(&a["life_pool_calls"]).len()}})
}

/// Current fixed capture extends the original witness at two later consumer
/// boundaries. Historical captures remain immutable and use `assess` above.
#[allow(dead_code)] // Also included by the original immutable-capture regression.
pub fn assess_later_auras(loads: &[Json]) -> Json {
    let inherited = assess(loads);
    for load in &loads[..2] {
        let trace = &load["additional_observation"];
        let calls = rows(&trace["perform_calls"]);
        let queries = rows(&trace["later_aura_queries"]);
        let party = rows(&trace["party_aura_gates"]);
        assert_eq!(calls.len(), 18, "complete unchanged perform census");
        assert_eq!(queries.len(), calls.len());
        assert_eq!(party.len(), calls.len());
        assert!(rows(&trace["later_aura_adds"]).is_empty());
        assert_eq!(trace["scope"]["later_aura_original_calls_observed"], true);
        for (index, call) in calls.iter().enumerate() {
            let q = &queries[index];
            let p = &party[index];
            for row in [q, p] {
                for key in ["perform_id", "env_id", "mode"] {
                    assert_eq!(row[key], call[key], "exact original call identity");
                }
            }
            assert_eq!(q["query_id"], index + 1);
            assert_eq!(q["line"], 3219);
            assert_eq!(q["name_from_pinned_callsite"], "AffectedByAuraMod");
            assert_eq!(q["exact_player_store"], true);
            assert_eq!(q["cfg"], json!({"kind":"nil"}));
            assert_eq!(q["caller"]["first_line"], 1193);
            assert_eq!(q["caller"], trace["original_methods"][0]);
            assert!(
                q["caller"]["path"]
                    .as_str()
                    .unwrap()
                    .ends_with("/Modules/CalcPerform.lua")
            );
            assert_eq!(q["returned"], true);
            assert_eq!(q["original_list_return_observed"], true);
            assert!(
                rows(&q["list_return"]).is_empty(),
                "filtered return is nonempty"
            );
            assert_eq!(q["source_stores"], q["source_stores_after"]);
            let stores = rows(&q["source_stores"]);
            assert!(!stores.is_empty(), "raw ancestry is required");
            for (at, store) in stores.iter().enumerate() {
                assert!(store["store_id"].as_u64().is_some_and(|v| v > 0));
                assert!(matches!(store["kind"].as_str(), Some("keyed" | "list")));
                assert!(
                    rows(&store["records"]).is_empty(),
                    "raw candidate requires review even if filtered out"
                );
                assert!(
                    store["names"]
                        .as_object()
                        .unwrap()
                        .get("AffectedByAuraMod")
                        .is_none()
                );
                assert_eq!(
                    store["parent_store_id"],
                    stores
                        .get(at + 1)
                        .map_or(json!(false), |s| s["store_id"].clone())
                );
            }
            assert_eq!(q["recipient_state"], json!({"kind":"nil"}));
            assert_eq!(q["recipient_state_after"], q["recipient_state"]);
            assert_eq!(p["line"], 3204);
            assert_eq!(p["exact_party_aura_owner"], true);
            assert!(p["party_aura"].as_object().unwrap().is_empty());
            assert_eq!(p["extra_aura"], json!({"kind":"nil"}));
            assert_eq!(p["recipient"], q["selected_recipient"]);
        }
    }
    let a = &loads[0]["additional_observation"];
    let b = &loads[1]["additional_observation"];
    for key in ["later_aura_queries", "party_aura_gates", "later_aura_adds"] {
        assert_eq!(a[key], b[key], "independent later aura observations");
    }
    json!({"inherited":inherited,"later_aura":{"original_player_queries":18,"party_gates":18,
        "raw_candidates":0,"returned_candidates":0,"recipient_adds":0,
        "independent_observations_exact":true,"recipient_global_absence_not_repaired":true,
        "future_input_or_native_coverage_authorized":false}})
}
