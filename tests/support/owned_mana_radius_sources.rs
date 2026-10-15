//! Finite original-source radius-provider proof. No runtime coverage authority.
#[allow(dead_code)]
#[path = "owned_empty_passive_socket_intrinsics.rs"]
mod sockets;
#[allow(dead_code)]
#[path = "owned_mana_supplier_sources.rs"]
pub(super) mod supplier;

use flate2::{Compression, GzBuilder, read::GzDecoder};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{Read, Write},
    sync::OnceLock,
};

const MANIFEST: &str = "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675";
const REVISION: &str = "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4";
const XML: &str = "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089";
const REPORT_SHA: &str = "8784718a733d10878da117724b3dff69e97d380c0140bfbab1cc02ffbf11a2da";
const SOURCE_RUN: &str = "runs/owned-mana-radius-source-01";
const LIMIT: usize = 32 * 1024 * 1024;

pub fn read(name: &str) -> Value {
    supplier::read(format!("{}/{name}", supplier::DIR))
}
fn ensure(ok: bool, why: &str) -> Result<(), String> {
    if ok { Ok(()) } else { Err(why.into()) }
}
fn rows<'a>(value: &'a Value, why: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| why.into())
}
fn id(value: &Value, why: &str) -> Result<u64, String> {
    value.as_u64().ok_or_else(|| why.into())
}
fn empty(value: &Value) -> bool {
    value.as_array().is_some_and(Vec::is_empty)
        || value.as_object().is_some_and(serde_json::Map::is_empty)
}
fn digest(value: &Value) -> String {
    supplier::hash(&serde_json::to_vec(value).unwrap())
}

struct Context {
    selected: Value,
    setup: String,
    nodes: BTreeSet<u64>,
    socket_nodes: BTreeSet<u64>,
}
fn context() -> &'static Context {
    static VALUE: OnceLock<Context> = OnceLock::new();
    VALUE.get_or_init(|| {
        let inputs = supplier::load();
        let bindings = supplier::reconcile_selected_inputs(&inputs).unwrap();
        let proof: Value = sockets::read("source-evidence.json");
        sockets::check_source(&proof);
        let acquisition = supplier::acquisition_fixture().unwrap();
        let preset = inputs.draft["draft"]["allocation_presets"]["members"]
            .as_array().unwrap().iter()
            .find(|p| p["id"] == inputs.selection["build"]["allocations"]).unwrap();
        let selected = json!({
            "historical_publication":bindings["publication"],
            "selection":bindings["selection"],
            "passives":bindings["memberships"]["passives"],
            "allocation_members":preset["allocations"],
            "socket_equipment":preset["equipment"],
            "socket_source":proof["contextual_reports"][0]["original_state"],
            "source02_counts":{"environments":acquisition.jit_off["state"]["environments"].as_array().unwrap().len(),
                "node_lists":acquisition.jit_off["state"]["node_lists"].as_array().unwrap().len(),
                "nodes":acquisition.jit_off["state"]["node_construction"].as_array().unwrap().len()},
            "current_native_coverage_authorized":false
        });
        let (nodes, socket_nodes) = review_membership(&selected).unwrap();
        // Both previously authenticated runs contain exactly these occurrences.
        // This is a recipe/input join, not evidence about unobserved invocations.
        for run in [&acquisition.jit_off, &acquisition.jit_on] {
            for env in run["state"]["environments"].as_array().unwrap() {
                let observed: BTreeSet<_> = env["nodes"].as_array().unwrap().iter()
                    .map(|n| n["key"].as_u64().unwrap()).collect();
                assert_eq!(observed, nodes);
                assert_eq!(env["nodes"].as_array().unwrap().len(), nodes.len());
                let saved = env["sockets"]["saved_jewels"]["_positions"].as_array().unwrap();
                assert_eq!(saved.len(), socket_nodes.len());
                assert_eq!(saved.iter().map(|r| r["index"].as_u64().unwrap()).collect::<BTreeSet<_>>(), socket_nodes);
                assert!(saved.iter().all(|r| r["value"] == 0));
                let ui = env["sockets"]["ui_sockets"].as_array().unwrap();
                assert_eq!(ui.len(), socket_nodes.len());
                assert_eq!(ui.iter().map(|r| r["key"].as_u64().unwrap()).collect::<BTreeSet<_>>(), socket_nodes);
                assert!(ui.iter().all(|r| r["value"]["nodeId"] == r["key"] && r["value"]["selItemId"] == 0));
            }
        }
        Context { selected, setup:acquisition.setup, nodes, socket_nodes }
    })
}

pub fn selected() -> &'static Value {
    &context().selected
}

/// Exact saved source membership, with roots kept separate from spendable nodes.
pub fn review_membership(v: &Value) -> Result<(BTreeSet<u64>, BTreeSet<u64>), String> {
    ensure(
        v["historical_publication"]
            == "3ee48944d26b5351ecc267cc2e87637b68a8b6153b7ab2bbc5c906f11e4bd702",
        "historical input identity",
    )?;
    ensure(
        v["socket_equipment"] == json!({"members":[],"completion":{"kind":"complete"}}),
        "selected socket membership is not Complete-empty",
    )?;
    ensure(
        v["allocation_members"]["completion"]["kind"] == "complete",
        "unknown allocation membership",
    )?;
    let passives = rows(&v["passives"], "passive source membership")?;
    ensure(passives.len() == 57, "57 selected source nodes required")?;
    let mut nodes = BTreeSet::new();
    let mut allocations = BTreeSet::new();
    let mut roots = BTreeSet::new();
    for row in passives {
        let source = row["source_token"]
            .as_str()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or("source node token")?;
        ensure(nodes.insert(source), "duplicate selected source node")?;
        ensure(
            row["source_spec"] == json!({"ordinal":135,"source_sha256":XML}),
            "different source Spec",
        )?;
        ensure(
            row["allocation_preset"] == v["selection"]["build"]["allocations"]
                && row["character_preset"] == v["selection"]["build"]["character"],
            "cross-preset passive binding",
        )?;
        match row["binding"]["kind"].as_str() {
            Some("allocation") => {
                ensure(
                    allocations.insert(digest(&row["binding"]["value"]["id"])),
                    "duplicate allocation occurrence",
                )?;
            }
            Some("implicit_root") => {
                ensure(
                    row["binding"]["value"]["character"] == v["selection"]["build"]["character"],
                    "root character differs",
                )?;
                roots.insert(source);
            }
            _ => return Err("unrecognized passive source binding".into()),
        }
    }
    let members = rows(&v["allocation_members"]["members"], "allocation members")?;
    ensure(
        allocations.len() == 55
            && members.len() == 55
            && members.iter().map(digest).collect::<BTreeSet<_>>() == allocations
            && roots == BTreeSet::from([8305, 54447]),
        "55 allocations plus two independent roots",
    )?;
    let state = &v["socket_source"];
    ensure(
        state["active_spec"] == 3
            && state["original_functions_preserved"] == true
            && state["saved_state_preserved"] == true
            && state["hook_removed"] == true,
        "selected source socket state",
    )?;
    let spec = &state["specs"][2];
    ensure(
        spec["index"] == 3
            && spec["constructor_calls"] == 1
            && spec["load_calls"] == 1
            && spec["postload_calls"] == 1
            && spec["source_socket_containers"] == 1
            && spec["source_direct_socket_rows"] == 0
            && spec["source_nested_socket_rows"] == 0
            && spec["jewels_type"] == "table"
            && spec["jewels"] == json!({}),
        "saved sockets are not explicitly empty",
    )?;
    let socket_rows = rows(&spec["zero_socket_nodes"], "socket node census")?;
    let socket_nodes = socket_rows
        .iter()
        .map(|v| id(v, "socket ID"))
        .collect::<Result<BTreeSet<_>, _>>()?;
    ensure(
        socket_nodes.len() == 19 && socket_rows.len() == 19 && socket_nodes.contains(&7960),
        "complete selected socket census",
    )?;
    for n in [4, 5] {
        ensure(
            rows(&state["specs"][n]["jewels"], "occupied sibling sockets")?.len() == 3,
            "occupied sibling control lost",
        )?;
    }
    ensure(
        v["current_native_coverage_authorized"] == false,
        "source membership cannot authorize runtime coverage",
    )?;
    Ok((nodes, socket_nodes))
}

pub fn review_bodies(bodies: &Value) -> Result<(), String> {
    let setup = &context().setup;
    ensure(
        bodies["schema_version"] == 1
            && bodies["kind"] == "selected-radius-construction-source-bodies",
        "body schema",
    )?;
    ensure(
        bodies["source"]
            == json!({"path":"src/Modules/CalcSetup.lua","archive":"CalcSetup.lua.gz","sha256":supplier::hash(setup.as_bytes())}),
        "body source authority",
    )?;
    let spans = [
        ("granted-passive-extra-node-removal", 19, 23),
        ("complete-radius-callback-adapter-and-cache", 120, 186),
        ("complete-node-modifier-constructor", 200, 413),
        ("complete-node-list-and-radius-finalization", 415, 464),
        ("complete-reused-environment-wipe", 466, 541),
        ("fresh-versus-reused-environment-initialization", 717, 801),
        ("complete-item-slot-radius-provider-insertion", 1040, 1214),
        ("equipped-jewel-modifier-delivery-boundary", 1292, 1323),
    ];
    let actual = rows(&bodies["spans"], "body spans")?;
    ensure(actual.len() == spans.len(), "complete body span census")?;
    for (row, (role, first, last)) in actual.iter().zip(spans) {
        let body = setup
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        ensure(
            row["role"] == role
                && row["first"] == first
                && row["last"] == last
                && row["body"] == body
                && row["sha256"] == supplier::hash(body.as_bytes()),
            "source body changed",
        )?;
    }
    let references: Vec<_> = setup
        .lines()
        .enumerate()
        .filter(|(_, s)| s.contains("radiusJewelList") || s.contains("extraRadiusNodeList"))
        .map(|(n, s)| json!({"line":n+1,"text":s}))
        .collect();
    ensure(
        bodies["radius_references"] == json!(references),
        "radius source reference census",
    )?;
    ensure(
        bodies["scope"]
            == json!({"selected_empty_passive_socket_providers_only":true,
        "ordinary_passive_transformations_covered":false,"generated_support_construction_covered":false,
        "native_coverage_authorized":false,"generic_radius_behavior_covered":false}),
        "body proof scope",
    )
}

fn node_ids(v: &Value) -> Result<BTreeSet<u64>, String> {
    let entries = rows(v, "node inventory")?;
    let mut ids = BTreeSet::new();
    for row in entries {
        let key = id(&row["key"], "node key")?;
        ensure(
            row["value"]["present"] == true && row["value"]["id"] == key && ids.insert(key),
            "node occurrence identity",
        )?;
    }
    ensure(
        ids == context().nodes,
        "node membership differs from selected 55+2",
    )?;
    Ok(ids)
}
fn empty_radius(v: &Value, env: &Value) -> Result<(), String> {
    ensure(
        v["environment_present"] == true
            && v["env_id"] == *env
            && v["radius_list_present"] == true
            && v["extra_nodes_present"] == true
            && empty(&v["radius_jewels"])
            && empty(&v["extra_radius_nodes"]),
        "radius state not known empty",
    )
}
fn no_override(v: &Value) -> Result<(), String> {
    ensure(
        v == &json!({"present":false}) || (v["present"] == true && empty(&v["fields"])),
        "override radius/provider input is unreviewed",
    )
}
fn selected_spec(v: &Value) -> Result<(), String> {
    ensure(
        v["present"] == true && v["is_selected"] == true,
        "unselected or unknown spec",
    )?;
    node_ids(&v["allocated_nodes"])?;
    ensure(v["jewels"]["kind"] == "table", "unknown jewel inventory")?;
    let entries = rows(&v["jewels"]["entries"], "jewel entries")?;
    let mut keys = BTreeSet::new();
    for row in entries {
        ensure(
            row["key_type"] == "number" && row["value"] == json!({"kind":"number","value":0}),
            "occupied or unknown selected socket",
        )?;
        ensure(
            keys.insert(id(&row["key"], "jewel node")?),
            "duplicate jewel node",
        )?;
    }
    ensure(
        keys == context().socket_nodes,
        "selected jewel slot membership changed",
    )
}

fn no_radius_item(v: &Value) -> Result<(), String> {
    ensure(
        v["present"] == false
            || (v["present"] == true
                && v["fields"]
                    .as_object()
                    .is_some_and(|f| !f.contains_key("jewelRadiusIndex"))),
        "unreviewed item radius provider",
    )
}

fn slots(v: &Value) -> Result<(), String> {
    ensure(
        v["item_zero"] == json!({"present":false}),
        "items[0] is a provider",
    )?;
    let ordered = rows(&v["ordered"], "complete ordered slots")?;
    let sockets = rows(&v["ui_sockets"], "complete UI socket map")?;
    let map = rows(&v["slot_map"], "complete slot map")?;
    ensure(
        ordered.len() == 113 && map.len() == 113,
        "complete 113-slot input census",
    )?;
    let mut indices = BTreeSet::new();
    let mut socket_ids = BTreeSet::new();
    for row in ordered {
        ensure(
            indices.insert(id(&row["key"], "ordered slot index")?),
            "duplicate ordered slot",
        )?;
        ensure(
            row["is_slot_map_entry"] == true,
            "ordered slot lacks exact map identity",
        )?;
        no_radius_item(&row["selected_item"])?;
        if let Some(node) = row["fields"]["nodeId"].as_u64() {
            ensure(
                socket_ids.insert(node)
                    && row["is_ui_socket"] == true
                    && row["fields"]["selItemId"] == 0
                    && row["selected_item"] == json!({"present":false})
                    && row["spec_item"] == json!({"present":false}),
                "selected socket provider is not absent",
            )?;
            let matched: Vec<_> = sockets.iter().filter(|s| s["key"] == node).collect();
            ensure(
                matched.len() == 1
                    && matched[0]["fields"] == row["fields"]
                    && matched[0]["ordered_indices"] == json!([row["key"]]),
                "UI/ordered socket correspondence",
            )?;
        }
        let matched: Vec<_> = map
            .iter()
            .filter(|s| s["key"] == row["fields"]["slotName"])
            .collect();
        ensure(
            matched.len() == 1 && matched[0]["ordered_indices"] == json!([row["key"]]),
            "slot map/ordered correspondence",
        )?;
    }
    ensure(
        socket_ids == context().socket_nodes && sockets.len() == socket_ids.len(),
        "socket census incomplete",
    )?;
    // Extra named slots may be non-ordered; none may hide a socket provider.
    for row in map {
        let matched: Vec<_> = ordered
            .iter()
            .filter(|s| s["fields"]["slotName"] == row["key"])
            .collect();
        ensure(
            matched.len() == 1 && row["ordered_indices"] == json!([matched[0]["key"]]),
            "hidden or duplicate slot-map entry",
        )?;
        let expected_node = matched[0]["fields"].get("nodeId").map_or_else(
            || json!({"kind":"nil"}),
            |n| json!({"kind":"number","value":n}),
        );
        ensure(
            row["node_id"] == expected_node,
            "slot-map node identity differs",
        )?;
        if row["node_id"]["kind"] != "nil" {
            ensure(
                row["node_id"]["kind"] == "number"
                    && socket_ids.contains(&id(&row["node_id"]["value"], "slot map node")?)
                    && rows(&row["ordered_indices"], "socket map indices")?.len() == 1,
                "hidden socket map entry",
            )?;
        }
    }
    Ok(())
}

/// Source bodies permit reuse, but every retained call supplies no reusable env.
/// Do not generalize this finite proof to accelerated or reused environments.
pub fn review_trace(trace: &Value) -> Result<Value, String> {
    ensure(
        trace["schema_version"] == 1
            && trace["kind"] == "original-mana-radius-inputs"
            && trace["original_functions_preserved"] == true
            && trace["observer_removed"] == true,
        "original observer authority",
    )?;
    ensure(
        trace["scope"]
            == json!({"unchanged_build_only":true,"native_coverage_authorized":false,
        "input_observation_not_universal_absence":true,"modifier_bodies_reused_from_source02":true}),
        "trace scope",
    )?;
    let source_path =
        "@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/CalcSetup.lua";
    let methods: Vec<_> = [(717, 2309), (200, 412), (415, 464)]
        .into_iter()
        .map(|(first, last)| {
            json!({"kind":"function_reference","path":source_path,"what":"Lua",
            "first_line":first,"last_line":last,"execution_not_implied":true})
        })
        .collect();
    ensure(
        trace["original_methods"] == json!(methods),
        "original method identities",
    )?;
    let calls = rows(&trace["init_calls"], "init calls")?;
    let lists = rows(&trace["list_calls"], "node list calls")?;
    let nodes = rows(&trace["node_calls"], "node calls")?;
    let candidates = rows(&trace["slot_candidates"], "item selection candidates")?;
    ensure(
        calls.len() == 21 && lists.len() == 42 && nodes.len() == 2394 && candidates.len() == 2352,
        "complete source invocation census",
    )?;
    let mut modes = BTreeMap::<String, usize>::new();
    for (at, call) in calls.iter().enumerate() {
        let invocation = json!(at + 1);
        ensure(
            call["invocation_id"] == invocation
                && call["env_id"] == invocation
                && call["parent_invocation_id"] == false
                && call["returned"] == true,
            "fresh independent invocation identity",
        )?;
        ensure(call["active_spec"] == 3, "selected active Spec differs")?;
        selected_spec(&call["selected_spec"])?;
        no_override(&call["override"])?;
        ensure(
            call["entry_radius"] == json!({"environment_present":false})
                && call["spec_env"]["env_id"] == false
                && call["spec_env"]["accelerate"] == json!({"kind":"nil"}),
            "reused/accelerated environment is outside proof",
        )?;
        let cached = call["spec_env"]["present"] == true;
        if cached {
            let mut expected = vec![
                json!({"key":"cachedEnemyDB","key_type":"string","value_type":"table"}),
                json!({"key":"cachedPlayerDB","key_type":"string","value_type":"table"}),
            ];
            if at == 20 {
                expected
                    .push(json!({"key":"fullDPSCache","key_type":"string","value_type":"table"}));
            }
            ensure(
                call["spec_env"]["keys"] == json!(expected),
                "unreviewed cached construction input",
            )?;
        } else {
            ensure(
                call["spec_env"]["present"] == false && empty(&call["spec_env"]["keys"]),
                "unknown construction input",
            )?;
        }
        let expected_caller = if cached {
            (252, 251, 439)
        } else if at == 0 || at == 17 {
            (471, 469, 891)
        } else if at == 19 {
            (91, 89, 144)
        } else {
            (443, 442, 466)
        };
        ensure(
            call["caller"]
                == json!({"path":"@Modules/Calcs.lua","what":"Lua",
            "current_line":expected_caller.0,"first_line":expected_caller.1,"last_line":expected_caller.2}),
            "unexpected original caller",
        )?;
        let mode = call["mode"].as_str().ok_or("mode")?;
        let expected_mode = if matches!(at, 1 | 18 | 19 | 20) {
            "CALCULATOR"
        } else if at == 17 {
            "CALCS"
        } else {
            "MAIN"
        };
        ensure(mode == expected_mode, "mode/caller correspondence")?;
        *modes.entry(mode.to_owned()).or_default() += 1;
        if cached {
            ensure(
                call.get("pre_configuration").is_none(),
                "unexpected cached baseline branch",
            )?;
        } else {
            let pre = &call["pre_configuration"];
            empty_radius(&pre["radius"], &invocation)?;
            selected_spec(&pre["spec"])?;
            no_override(&pre["override"])?;
            ensure(
                pre["accelerate"]["kind"] == "table" && empty(&pre["accelerate"]["entries"]),
                "baseline acceleration",
            )?;
        }
        let pass = &call["item_pass"];
        empty_radius(&pass["radius"], &invocation)?;
        node_ids(&pass["allocated_nodes"])?;
        no_override(&pass["override"])?;
        ensure(
            pass["accelerate"]["kind"] == "table" && empty(&pass["accelerate"]["entries"]),
            "item-pass acceleration",
        )?;
        for inventory in [&call["entry_slots"], &pass["slots"], &call["exit_slots"]] {
            slots(inventory)?;
        }
        ensure(
            call["entry_slots"] == pass["slots"] && pass["slots"] == call["exit_slots"],
            "slot inputs changed during invocation",
        )?;
        empty_radius(&call["exit_radius"], &invocation)?;
        node_ids(&call["exit_nodes"])?;
        let ordered = rows(&pass["slots"]["ordered"], "ordered item-pass slots")?;
        let actual: Vec<_> = candidates
            .iter()
            .filter(|c| c["invocation_id"] == invocation)
            .collect();
        ensure(actual.len() == 112, "complete actual item-selection census")?;
        let mut seen = BTreeSet::new();
        for candidate in actual {
            ensure(
                candidate["env_id"] == invocation,
                "item selection environment",
            )?;
            no_override(&candidate["override"])?;
            let indices = rows(&candidate["ordered_indices"], "candidate indices")?;
            ensure(
                indices.len() == 1 && seen.insert(id(&indices[0], "candidate index")?),
                "duplicate or missing ordered candidate identity",
            )?;
            let slot = ordered
                .iter()
                .find(|s| s["key"] == indices[0])
                .ok_or("unmatched candidate slot")?;
            ensure(
                candidate["slot_fields"] == slot["fields"]
                    && candidate["slot_name"] == slot["fields"]["slotName"]
                    && candidate["resolved_item"] == slot["selected_item"],
                "actual source item-selection differs",
            )?;
            no_radius_item(&candidate["resolved_item"])?;
            ensure(
                candidate["allocated"]
                    == json!(
                        slot["fields"]["nodeId"]
                            .as_u64()
                            .is_some_and(|n| context().nodes.contains(&n))
                    ),
                "socket allocation gate",
            )?;
        }
        let skipped: Vec<_> = ordered
            .iter()
            .filter(|s| !seen.contains(&s["key"].as_u64().unwrap()))
            .collect();
        ensure(
            skipped.len() == 1
                && skipped[0]["fields"]["slotName"] == "Ring 3"
                && skipped[0]["selected_item"] == json!({"present":false}),
            "only original early Ring 3 skip is reviewed",
        )?;
        let own_lists: Vec<_> = lists
            .iter()
            .filter(|l| l["invocation_id"] == invocation)
            .collect();
        ensure(
            own_lists.len() == 2,
            "two selected node-list passes required",
        )?;
        for (pass_at, list) in own_lists.iter().enumerate() {
            ensure(
                list["list_id"] == (2 * at + pass_at + 1)
                    && list["env_id"] == invocation
                    && list["parent_list_id"] == false
                    && list["returned"] == true
                    && list["finish_jewels"] == json!({"kind":"boolean","value":true})
                    && list["include_keystone_mods"]
                        == if pass_at == 0 {
                            json!({"kind":"boolean","value":true})
                        } else {
                            json!({"kind":"nil"})
                        },
                "node-list identity/flags",
            )?;
            ensure(
                list["caller"]
                    == json!({"path":source_path,"what":"Lua","current_line":if pass_at==0 {999} else {1637},
                "first_line":717,"last_line":2309}),
                "node-list consumer caller",
            )?;
            node_ids(&list["nodes"])?;
            empty_radius(&list["before_radius"], &invocation)?;
            empty_radius(&list["after_radius"], &invocation)?;
            let visits: Vec<_> = nodes
                .iter()
                .filter(|n| n["list_id"] == list["list_id"])
                .collect();
            ensure(
                visits.len() == 57,
                "each node-list must visit all 57 nodes exactly once",
            )?;
            let mut visited = BTreeSet::new();
            for visit in visits {
                ensure(
                    visit["invocation_id"] == invocation
                        && visit["env_id"] == invocation
                        && visit["returned"] == true
                        && visit["node"]["present"] == true,
                    "node visit authority",
                )?;
                ensure(
                    visited.insert(id(&visit["node"]["id"], "visited node")?),
                    "duplicate node visit",
                )?;
                ensure(
                    visit["caller"]
                        == json!({"path":source_path,"what":"Lua","current_line":435,"first_line":415,"last_line":464}),
                    "node constructor caller",
                )?;
                empty_radius(&visit["before_radius"], &invocation)?;
                empty_radius(&visit["after_radius"], &invocation)?;
            }
            ensure(
                visited == context().nodes,
                "visited nodes differ from source membership",
            )?;
        }
    }
    for (at, row) in nodes.iter().enumerate() {
        ensure(row["node_call_id"] == at + 1, "raw node sequence identity")?;
    }
    Ok(
        json!({"fresh_environments":calls.len(),"reused_environments":0,"nested_init_calls":0,
        "modes":modes,"ordered_slots_per_call":113,"selected_candidates_per_call":112,
        "early_skipped_slot":"Ring 3","empty_socket_slots_per_call":19,
        "selected_nodes":57,"allocation_occurrences":55,"implicit_roots":[8305,54447],
        "node_list_calls":lists.len(),"node_constructor_calls":nodes.len(),
        "radius_provider_branches_reached_with_eligible_item":0,
        "selected_radius_construction_reconciled":true,"native_coverage_authorized":false}),
    )
}

fn comparison(trace: &Value) -> Value {
    let mut value = trace.clone();
    let rows = value["node_calls"].as_array_mut().unwrap();
    for row in rows.iter_mut() {
        row.as_object_mut().unwrap().remove("node_call_id");
    }
    rows.sort_by_cached_key(|r| serde_json::to_string(r).unwrap());
    value
}

fn unpack(name: &str, limit: usize, receipt: &Value) -> Result<Vec<u8>, String> {
    let path = supplier::root()
        .join(supplier::DIR)
        .join(format!("radius-{name}.gz"));
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= 4 * 1024 * 1024,
        "bounded compressed acquisition",
    )?;
    let compressed = fs::read(path).map_err(|e| e.to_string())?;
    let row = &receipt["archives"][name];
    ensure(
        row["compressed_bytes"] == compressed.len()
            && row["compressed_sha256"] == supplier::hash(&compressed),
        "compressed acquisition pin",
    )?;
    let mut bytes = Vec::new();
    GzDecoder::new(compressed.as_slice())
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    ensure(
        bytes.len() <= limit
            && row["bytes"] == bytes.len()
            && row["sha256"] == supplier::hash(&bytes),
        "bounded raw acquisition pin",
    )?;
    Ok(bytes)
}

pub fn acquisition() -> Result<Value, String> {
    let receipt = read("radius-acquisition.json");
    let mut report = receipt["report_metadata"].clone();
    ensure(
        report.get("observed").is_none(),
        "compact acquisition metadata cannot supply traces",
    )?;
    ensure(
        report["kind"] == "original-mana-radius-input-evidence"
            && report["schema_version"] == 1
            && report["source_revision"] == REVISION
            && report["manifest_sha256"] == MANIFEST
            && report["xml"]
                == json!({"path":"tests/fixtures/builds/breadth-20260908/build-05.xml","sha256":XML}),
        "source acquisition identity",
    )?;
    ensure(
        report["complete_loads"] == 4
            && report["instrumented_loads"] == 2
            && report["instrumented_jit_enabled"] == false
            && report["plain_jit_modes"] == json!([false, true])
            && report["all_scalar_outputs_exact"] == true
            && report["native_coverage_authorized"] == false,
        "fixed acquisition protocol",
    )?;
    let manifest =
        fs::read(supplier::root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json"))
            .map_err(|e| e.to_string())?;
    ensure(
        supplier::hash(&manifest) == MANIFEST,
        "pinned source manifest",
    )?;
    for (name, field) in [
        ("acquisition-driver.rs", "driver_sha256"),
        ("acquisition-observer.lua", "observer_sha256"),
        ("acquisition-helpers.lua", "helper_sha256"),
        ("acquisition-bootstrap.rs", "bootstrap_sha256"),
    ] {
        ensure(
            supplier::hash(&unpack(name, 256 * 1024, &receipt)?) == report[field],
            "acquisition witness identity",
        )?;
    }
    let files = rows(&report["files"], "four raw loads")?.to_vec();
    ensure(files.len() == 4, "four raw load census")?;
    let mut observed = Vec::new();
    for (at, (file, label)) in files
        .iter()
        .zip(["observed-a", "observed-b", "plain-off", "plain-on"])
        .enumerate()
    {
        let name = format!("{label}.json");
        let bytes = unpack(&name, LIMIT, &receipt)?;
        ensure(
            file["file"] == name
                && file["bytes"] == bytes.len()
                && file["sha256"] == supplier::hash(&bytes),
            "raw source report identity",
        )?;
        let raw: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        ensure(
            raw["source_hash"] == MANIFEST
                && raw["configuration_method_wrappers"] == false
                && raw["original_build_output_available"] == true,
            "original unwrapped source execution",
        )?;
        if at < 2 {
            observed.push(raw["additional_observation"].clone());
        } else {
            ensure(
                raw["additional_observation"] == observed[0]["snapshot"],
                "plain control captured scalar outputs",
            )?;
        }
    }
    report["observed"] = json!(observed);
    // The original driver wrote canonical serde_json bytes. Reassemble that
    // exact report from its raw observations rather than archive them twice.
    ensure(
        digest(&report) == REPORT_SHA,
        "independent accepted source report pin",
    )?;
    ensure(
        receipt["report_sha256"] == REPORT_SHA,
        "receipt accepted source pin",
    )?;
    Ok(report)
}

pub fn fixture() -> &'static Value {
    static VALUE: OnceLock<Value> = OnceLock::new();
    VALUE.get_or_init(|| acquisition().unwrap())
}

pub fn reconstruct(report: &Value, bodies: &Value, membership: &Value) -> Result<Value, String> {
    review_bodies(bodies)?;
    review_membership(membership)?;
    ensure(
        membership == selected(),
        "historical selected membership authority",
    )?;
    let observed = rows(&report["observed"], "independent traces")?;
    ensure(observed.len() == 2, "two independent hooked traces")?;
    let a = review_trace(&observed[0])?;
    let b = review_trace(&observed[1])?;
    ensure(
        a == b && comparison(&observed[0]) == comparison(&observed[1]),
        "independent counted-node trace equality",
    )?;
    ensure(
        observed[0]["snapshot"] == observed[1]["snapshot"],
        "captured scalar snapshot equality",
    )?;
    // Compare the complete caller-supplied report to authenticated originals;
    // the cache never bypasses checks on mutation entrypoints.
    ensure(
        report == fixture(),
        "supplied acquisition differs from authenticated report",
    )?;
    Ok(
        json!({"schema_version":1,"kind":"selected-passive-radius-source-review",
        "source_report_sha256":REPORT_SHA,"source_revision":REVISION,"manifest_sha256":MANIFEST,
        "source_xml_sha256":XML,"selected_membership_sha256":digest(membership),
        "historical_membership_publication":membership["historical_publication"],
        "body_review_sha256":digest(bodies),"observed":a,
        "evidence_reused":{"source02_environment_count":21,"source02_node_list_count":42,
            "source02_node_constructor_count":2394,"selected_socket_source_spec":3},
        "protocol":{"fresh_loads":4,"hooked_jit_off":2,"plain_jit_modes":[false,true],
            "captured_scalar_snapshots_exact":true,"raw_node_visit_order_retained":true,
            "comparison_only_counts_node_visits":true},
        "scope":{"selected_radius_construction_reconciled":true,"arbitrary_reuse_or_overrides_covered":false,
            "ordinary_passive_transformations_covered":false,"generated_support_construction_covered":false,
            "later_modifier_delivery_covered":false,"generic_radius_behavior_covered":false,
            "native_coverage_authorized":false,"current_native_package_changed":false}}),
    )
}

pub fn check() -> Result<(), String> {
    let report = acquisition()?;
    let result = reconstruct(&report, &read("radius-bodies.json"), selected())?;
    ensure(
        read("radius-review.json") == result,
        "retained radius review differs",
    )
}

pub fn retain() -> Result<(), String> {
    let base = supplier::root().join(SOURCE_RUN);
    let bytes = fs::read(base.join("source.json")).map_err(|e| e.to_string())?;
    ensure(
        supplier::hash(&bytes) == REPORT_SHA,
        "accepted acquisition required",
    )?;
    let mut metadata: Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    ensure(
        serde_json::to_vec(&metadata).unwrap() == bytes,
        "exact canonical acquisition report",
    )?;
    metadata.as_object_mut().unwrap().remove("observed");
    let mut archives = serde_json::Map::new();
    for name in [
        "observed-a.json",
        "observed-b.json",
        "plain-off.json",
        "plain-on.json",
        "acquisition-driver.rs",
        "acquisition-observer.lua",
        "acquisition-helpers.lua",
        "acquisition-bootstrap.rs",
    ] {
        let bytes = fs::read(base.join(name)).map_err(|e| e.to_string())?;
        let mut encoder = GzBuilder::new()
            .mtime(0)
            .write(Vec::new(), Compression::best());
        encoder.write_all(&bytes).map_err(|e| e.to_string())?;
        let compressed = encoder.finish().map_err(|e| e.to_string())?;
        fs::write(
            supplier::root()
                .join(supplier::DIR)
                .join(format!("radius-{name}.gz")),
            &compressed,
        )
        .map_err(|e| e.to_string())?;
        archives.insert(
            name.into(),
            json!({"bytes":bytes.len(),"sha256":supplier::hash(&bytes),
            "compressed_bytes":compressed.len(),"compressed_sha256":supplier::hash(&compressed)}),
        );
    }
    write(
        "radius-acquisition.json",
        &json!({"schema_version":1,"report_sha256":REPORT_SHA,"report_metadata":metadata,"archives":archives}),
    )?;
    let result = reconstruct(&acquisition()?, &read("radius-bodies.json"), selected())?;
    write("radius-review.json", &result)
}
fn write(name: &str, v: &Value) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(v).unwrap();
    bytes.push(b'\n');
    fs::write(supplier::root().join(supplier::DIR).join(name), bytes).map_err(|e| e.to_string())
}
