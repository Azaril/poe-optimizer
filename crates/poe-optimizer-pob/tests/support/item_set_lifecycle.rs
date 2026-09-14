//! Complete original Load observation; no native/equipment parity is inferred.
#[path = "item_assembly_graph.rs"]
mod graph;
#[path = "item_set_loadout_cases.rs"]
mod loadout_cases;
#[allow(dead_code)]
#[path = "configuration_preparation_source.rs"]
mod source;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    fs,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVER: &str = include_str!("item_set_lifecycle.lua");
const TEST: &str = "all_five_complete_original_item_set_load_lifecycles";
const CHILD: &str = "POE_ITEM_SET_LIFECYCLE_CHILD";
const OUTPUT: &str = "POE_ITEM_SET_LIFECYCLE_OUTPUT";
const LOADOUT_TEST: &str = "all_five_original_loadout_sync_and_lookup_histories";
const LOADOUT_CHILD: &str = "POE_ITEM_SET_LOADOUT_CHILD";
const LOADOUT_OUTPUT: &str = "POE_ITEM_SET_LOADOUT_OUTPUT";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn canonical(table: Table) -> Json {
    graph::canonical(&graph::capture(&[Value::Table(table)]).unwrap()).unwrap()
}
fn scalar(lua: &Lua, value: Value) -> Json {
    assert!(!matches!(
        value,
        Value::Table(_) | Value::Function(_) | Value::UserData(_)
    ));
    lua.from_value(value).unwrap()
}
// Source rune definitions are already projected to names. This key compares
// selected names and multiplicity, not definition identity, effect data, order,
// or selected indices. The independent exact graph retains order and indices.
pub(super) fn rune_control_key(runes: &Table) -> Json {
    let mut result = BTreeMap::new();
    let mut bytes = 0usize;
    for (index, row) in runes.clone().pairs::<String, Table>().enumerate() {
        assert!(index < 4096, "rune slot comparison bound");
        let (slot_name, rune) = row.unwrap();
        bytes = bytes.checked_add(slot_name.len()).unwrap();
        assert!(bytes <= 16 * 1024 * 1024, "rune comparison text bound");
        let list: Table = rune.raw_get("list").unwrap();
        let n = list.raw_len();
        assert!(n > 0 && n <= 4096, "rune choice comparison bound");
        let selected: usize = rune.raw_get("selIndex").unwrap();
        assert!(selected > 0 && selected <= n);
        let mut visited = 0;
        for row in list.clone().pairs::<Value, Value>() {
            row.unwrap();
            visited += 1;
            assert!(visited <= n, "rune choices must be a dense array");
        }
        assert_eq!(visited, n);
        let mut names = BTreeMap::<String, usize>::new();
        let mut selected_name = None;
        for i in 1..=n {
            let row: Table = list.raw_get(i).unwrap();
            let name: mlua::LuaString = row.raw_get("name").unwrap();
            let length = name.as_bytes().len();
            bytes = bytes.checked_add(length).unwrap();
            assert!(
                length <= 262144 && bytes <= 16 * 1024 * 1024,
                "rune comparison text bound"
            );
            let name = name.to_str().unwrap().to_owned();
            if i == selected {
                bytes = bytes.checked_add(length).unwrap();
                assert!(bytes <= 16 * 1024 * 1024, "rune comparison text bound");
                selected_name = Some(name.clone());
            }
            *names.entry(name).or_default() += 1;
        }
        result.insert(
            slot_name,
            json!({"selected_name":selected_name.unwrap(),"name_occurrences":names}),
        );
    }
    json!(result)
}

// This is a separate selected-field comparison, not a normalization of the
// exact graph. It omits slot-to-slot aliases and dropdown indices, and projects
// child controls to names/inactive flags. Rune choices compare selected names
// and occurrence counts, excluding order, indices, aliases and effect data.
// Exact arrays and aliases remain in the independent declared graph/result.
fn control_key(lua: &Lua, state: &Table) -> Json {
    let fixed = lua.create_table().unwrap();
    for row in state.clone().pairs::<Value, Value>() {
        let (key, value) = row.unwrap();
        if matches!(&key,Value::String(s) if s.as_bytes().as_ref()==b"slots" || s.as_bytes().as_ref()==b"runeSlots")
        {
            continue;
        }
        fixed.raw_set(key, value).unwrap();
    }
    let slots: Table = state.raw_get("slots").unwrap();
    let mut slot_rows = BTreeMap::new();
    for row in slots.pairs::<String, Table>() {
        let (name, slot) = row.unwrap();
        let items: Table = slot.raw_get("items").unwrap();
        let labels: Table = slot.raw_get("list").unwrap();
        let n = items.raw_len();
        assert!(n > 0 && n <= 4096);
        assert_eq!(n, labels.raw_len());
        let mut choices = BTreeMap::new();
        for i in 1..=n {
            let id: i64 = items.raw_get(i).unwrap();
            let label: String = labels.raw_get(i).unwrap();
            assert!(choices.insert(id, label).is_none());
        }
        assert_eq!(items.clone().pairs::<Value, Value>().count(), n);
        assert_eq!(labels.clone().pairs::<Value, Value>().count(), n);
        let index: usize = slot.raw_get("selIndex").unwrap();
        assert!(index > 0 && index <= n);
        let mut fields = BTreeMap::new();
        for key in [
            "slotName",
            "nodeId",
            "selItemId",
            "active",
            "inactive",
            "note",
        ] {
            fields.insert(key, scalar(lua, slot.raw_get(key).unwrap()));
        }
        let children: Table = slot.raw_get("jewelSocketList").unwrap();
        let mut child_rows = Vec::new();
        for value in children.sequence_values::<Table>() {
            let child = value.unwrap();
            child_rows.push(json!({"slot":scalar(lua,child.raw_get("slotName").unwrap()),"inactive":scalar(lua,child.raw_get("inactive").unwrap())}));
        }
        let activation = slot.raw_get::<Value>("activate").unwrap();
        let activate = match activation {
            Value::Nil => json!({"present":false}),
            Value::Table(t) => {
                json!({"present":true,"state":scalar(lua,t.raw_get("state").unwrap())})
            }
            other => panic!("activation state {other:?}"),
        };
        slot_rows.insert(name,json!({"fields":fields,"choices_by_item_id":choices,"selected_choice":items.raw_get::<i64>(index).unwrap(),"activate_state":activate,"children":child_rows}));
    }
    json!({"fixed_graph":canonical(fixed),"slot_semantics":slot_rows,"rune_name_semantics":rune_control_key(&state.raw_get::<Table>("runeSlots").unwrap())})
}
// Token values are observer-lifetime diagnostics. Only the top-level identity
// envelope is separated; the remaining joint graph retains spec/loadouts/result
// aliases and exact arrays. This is not a projection of each field independently.
pub(super) fn loadout_snapshot_json(lua: &Lua, snapshot: Table) -> mlua::Result<Json> {
    let joint = lua.create_table()?;
    let mut identity = Json::Null;
    for row in snapshot.pairs::<Value, Value>() {
        let (key, value) = row?;
        if matches!(&key, Value::String(s) if s.as_bytes().as_ref() == b"identity") {
            identity = lua.from_value(value)?;
        } else {
            joint.raw_set(key, value)?;
        }
    }
    Ok(json!({"graph":canonical(joint),"identity":identity}))
}

pub(super) fn loadout_states_json(lua: &Lua, report: &Table) -> mlua::Result<Json> {
    let mut states = Vec::new();
    for row in report
        .raw_get::<Table>("loadout_states")?
        .sequence_values::<Table>()
    {
        let state = row?;
        let mut entry = serde_json::Map::new();
        for row in state.pairs::<String, Value>() {
            let (key, value) = row?;
            if key == "value" {
                let Value::Table(snapshot) = value else {
                    return Err(mlua::Error::RuntimeError(
                        "loadout snapshot is not a table".into(),
                    ));
                };
                entry.insert("snapshot".into(), loadout_snapshot_json(lua, snapshot)?);
            } else {
                // Missing Load ancestry is absent rather than an invented root;
                // root_call_ordinal belongs to this one observer lifetime.
                entry.insert(key, lua.from_value(value)?);
            }
        }
        states.push(Json::Object(entry));
    }
    Ok(states.into())
}

fn host(repo: &Path, directory: &Path, xml: &str, observed: bool) -> Json {
    host_mode(repo, directory, xml, observed, false)
}

// The opt-in shares the original bootstrap and import finish boundary. Direct
// post-import calls get a separate observer lifetime in loadout_cases::run.
fn host_mode(repo: &Path, directory: &Path, xml: &str, observed: bool, loadouts: bool) -> Json {
    fs::create_dir_all(directory).unwrap();
    let module = Rc::new(RefCell::new(None::<Table>));
    let capture = Rc::new(RefCell::new(None::<Table>));
    let before_source = |lua: &Lua| -> Result<(), RuntimeError> {
        *module.borrow_mut() = Some(
            lua.load(OBSERVER)
                .set_name("@item_set_lifecycle.lua")
                .eval()?,
        );
        Ok(())
    };
    let before_build = |lua: &Lua| -> Result<Function, RuntimeError> {
        let start: Function = module.borrow().as_ref().unwrap().raw_get("start")?;
        let active: Table = if loadouts {
            let options = lua.create_table()?;
            options.raw_set("loadouts", true)?;
            start.call((observed, options))?
        } else {
            start.call(observed)?
        };
        let finish = active.raw_get("finish")?;
        *capture.borrow_mut() = Some(active);
        Ok(finish)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        let report: Table = capture.borrow().as_ref().unwrap().raw_get("report")?;
        assert_eq!(
            report.raw_get::<Table>("incomplete_calls")?.raw_len(),
            0,
            "completed source import retained incomplete original calls"
        );
        let mut out = serde_json::Map::new();
        for key in [
            "events",
            "functions",
            "scope",
            "bounds",
            "retained_objects",
            "rows",
            "text_bytes",
        ] {
            out.insert(key.into(), lua.from_value(report.raw_get(key)?)?);
        }
        let mut states = Vec::new();
        for value in report
            .raw_get::<Table>("states")?
            .sequence_values::<Table>()
        {
            let state = value?;
            let mut entry = serde_json::Map::new();
            for key in [
                "event_ordinal",
                "call_ordinal",
                "phase",
                "name",
                "receiver_token",
                "prior_set_token",
            ] {
                entry.insert(key.into(), lua.from_value(state.raw_get(key)?)?);
            }
            entry.insert("graph".into(), canonical(state.raw_get("value")?));
            states.push(Json::Object(entry));
        }
        out.insert("states".into(), states.into());
        let final_state: Table = report.raw_get("finite_post_import")?;
        out.insert(
            "post_import_exact_graph".into(),
            canonical(final_state.clone()),
        );
        out.insert(
            "post_import_control_key".into(),
            control_key(lua, &final_state),
        );
        if loadouts {
            out.insert("loadout_states".into(), loadout_states_json(lua, &report)?);
            out.insert(
                "post_import_loadouts".into(),
                loadout_snapshot_json(lua, report.raw_get("finite_post_loadouts")?)?,
            );
            // Import observation has already finished and removed its hook.
            // These are explicit direct calls against the resulting original
            // owners, not observations of their earlier import-time execution.
            let direct = loadout_cases::run(lua, module.borrow().as_ref().unwrap(), observed)?;
            out.insert("direct_loadouts".into(), direct);
        }
        Ok(Json::Object(out))
    };
    source::observe_with_build_hook_unwrapped(
        &repo.join("vendor/path-of-building-poe2"),
        directory,
        xml,
        None,
        false,
        Some(&before_source),
        Some(&before_build),
        Some(&after),
    )
    .unwrap()
}
fn inspect(report: &Json, xml: &str) -> Json {
    let o = &report["additional_observation"];
    let events = o["events"].as_array().unwrap();
    let functions = o["functions"].as_array().unwrap();
    let load = functions
        .iter()
        .find(|f| f["name"] == "items_load")
        .unwrap();
    assert_eq!(load["first_line"], 1193);
    assert_eq!(load["last_line"], 1320);
    assert!(
        load["source"]
            .as_str()
            .unwrap()
            .replace('\\', "/")
            .ends_with("Classes/ItemsTab.lua")
    );
    let expected = roxmltree::Document::parse(xml)
        .unwrap()
        .root_element()
        .children()
        .filter(|n| n.has_tag_name("Items"))
        .count();
    let calls = events
        .iter()
        .filter(|e| e["event"] == "call" && e["name"] == "items_load")
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), expected);
    let mut counts = BTreeMap::<String, usize>::new();
    let mut populations = BTreeMap::<u64, Vec<String>>::new();
    let mut contexts = BTreeMap::<String, usize>::new();
    let mut completed = Vec::new();
    for e in events {
        if e["event"] != "call" {
            continue;
        }
        *counts
            .entry(e["name"].as_str().unwrap().into())
            .or_default() += 1;
        if e["name"] == "populate_slots" {
            populations.insert(e["ordinal"].as_u64().unwrap(), Vec::new());
        }
        if e["name"] == "populate_slot" && e["direct_populate_slots_caller"] == true {
            populations
                .get_mut(&e["parent_call_ordinal"].as_u64().unwrap())
                .unwrap()
                .push(e["slot"].as_str().unwrap().into());
        }
        if e["name"] == "validity" {
            let c = &e["context"];
            let key = format!(
                "calcsTab={},mainEnv={}",
                c["calcs_tab_present"], c["main_env_present"]
            );
            *contexts.entry(key).or_default() += 1;
        }
    }
    for call in calls {
        let ordinal = call["ordinal"].as_u64().unwrap();
        let end = events
            .iter()
            .find(|e| e["event"] == "return" && e["call_ordinal"] == ordinal)
            .unwrap();
        let last = end["ordinal"].as_u64().unwrap();
        let reset = events
            .iter()
            .filter(|e| {
                e["name"] == "reset_undo"
                    && e["event"] == "call"
                    && e["receiver_token"] == call["receiver_token"]
                    && e["ordinal"].as_u64().unwrap() > ordinal
                    && e["ordinal"].as_u64().unwrap() < last
            })
            .collect::<Vec<_>>();
        assert!(!reset.is_empty());
        let last_reset = reset.last().unwrap()["ordinal"].as_u64().unwrap();
        for sync in events.iter().filter(|e| {
            e["event"] == "return"
                && e["name"] == "sync_loadouts"
                && e["load_call_ordinal"] == ordinal
        }) {
            assert!(sync["ordinal"].as_u64().unwrap() < last_reset);
        }
        let states = o["states"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|s| s["name"] == "items_load" && s["call_ordinal"] == ordinal)
            .collect::<Vec<_>>();
        assert_eq!(states.len(), 2);
        assert_eq!(states[0]["phase"], "before");
        assert_eq!(states[1]["phase"], "after");
        completed.push(json!({"call":ordinal,"return":last,"reset_undo_calls":reset.len(),"sync_returns_before_final_reset":true}));
    }
    assert!(!populations.is_empty());
    assert!(populations.values().all(|v| !v.is_empty()));
    assert!(!contexts.is_empty());
    json!({"complete_loads":completed,"call_counts":counts,"populate_slot_orders":populations.into_values().collect::<Vec<_>>(),"validity_contexts":contexts,"events":events.len(),"states":o["states"].as_array().unwrap().len(),"exact_validity_return_pack_claim":false})
}
fn child(repo: &Path, output: &Path, entry: &Json) {
    let name = entry["xml"].as_str().unwrap();
    let xml = fs::read_to_string(
        repo.join("tests/fixtures/builds/breadth-20260908")
            .join(name),
    )
    .unwrap();
    assert_eq!(hash(xml.as_bytes()), entry["xml_sha256"]);
    let directory = output.join(name);
    fs::create_dir_all(&directory).unwrap();
    let mut reports = Vec::new();
    for (label, observed) in [
        ("control", false),
        ("observed-a", true),
        ("observed-b", true),
    ] {
        let report = host(
            repo,
            &directory.join(format!("{label}-host")),
            &xml,
            observed,
        );
        fs::write(
            directory.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        reports.push(report);
    }
    let control = &reports[0];
    let mut comparisons = Vec::new();
    let mut proofs = Vec::new();
    for (i, label) in [(1, "observed-a"), (2, "observed-b")] {
        let report = &reports[i];
        assert_eq!(control["source_hash"], report["source_hash"]);
        assert_eq!(control["selected"], report["selected"]);
        let exact = control["additional_observation"]["post_import_exact_graph"]
            == report["additional_observation"]["post_import_exact_graph"];
        let keyed = control["additional_observation"]["post_import_control_key"]
            == report["additional_observation"]["post_import_control_key"];
        // A changed final selection is a real observation requiring investigation;
        // retain reports before asserting, including all actual traversal arrays.
        assert!(
            keyed,
            "{name}: keyed selected-field state differs; inspect retained raw orders and graphs"
        );
        comparisons.push(json!({"host":label,"exact_graph_equal":exact,"keyed_selection_control_equal":keyed,"keyed_projection_scope":"fixed graph excluding slots/runeSlots plus selected slot fields, activation presence/state, ID-label choices, child name/inactive rows and rune selected names/name-occurrence counts; slot/rune aliases, dropdown indices/order and rune effect data are not compared here"}));
        proofs.push(inspect(report, &xml));
    }
    let order_equal = proofs[0]["populate_slot_orders"] == proofs[1]["populate_slot_orders"];
    let summary = json!({"xml":name,"xml_sha256":entry["xml_sha256"],"comparisons":comparisons,"observed_host_slot_order_equal":order_equal,"observed":proofs,"scope":{"source_only":true,"complete_original_load_returns":true,"whole_object_graph":false,"native_parity":false,"universal_noninterference":false,"fresh_hosts_same_machine":3}});
    fs::write(
        output.join(format!("{name}.json")),
        serde_json::to_vec_pretty(&summary).unwrap(),
    )
    .unwrap();
}
// Child streams remain on disk as exact evidence. Print bounded diagnostic
// tails only when a child fails so CI annotations include its actual error.
fn child_failure_logs(output: &Path, name: &str) -> String {
    const MAX_BYTES: u64 = 4096;
    let mut diagnostics = String::new();
    for stream in ["stderr", "stdout"] {
        let path = output.join(format!("{name}.{stream}.log"));
        let tail = (|| -> std::io::Result<Vec<u8>> {
            let mut file = fs::File::open(&path)?;
            let length = file.metadata()?.len();
            file.seek(SeekFrom::Start(length.saturating_sub(MAX_BYTES)))?;
            let mut bytes = Vec::with_capacity(MAX_BYTES as usize);
            file.take(MAX_BYTES).read_to_end(&mut bytes)?;
            Ok(bytes)
        })();
        diagnostics.push_str(&format!(
            "\n--- {} (last {MAX_BYTES} bytes) ---\n",
            path.display()
        ));
        match tail {
            Ok(bytes) => diagnostics.push_str(&String::from_utf8_lossy(&bytes)),
            Err(error) => diagnostics.push_str(&format!("could not read child log: {error}")),
        }
    }
    diagnostics
}

pub fn run() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(repo.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let output = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2ad-item-sets-01/source"));
    fs::create_dir_all(&output).unwrap();
    // Export paths read USERPROFILE before the per-host scratch fallback. Give
    // every fresh host in a child the same explicit, test-owned process input.
    let user_profile = output.canonicalize().unwrap();
    if let Ok(name) = std::env::var(CHILD) {
        child(
            &repo,
            &output,
            index["builds"]
                .as_array()
                .unwrap()
                .iter()
                .find(|v| v["xml"] == name)
                .unwrap(),
        );
        return;
    }
    let mut children = Vec::new();
    for entry in index["builds"].as_array().unwrap() {
        let name = entry["xml"].as_str().unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, name)
            .env(OUTPUT, &output)
            .env("USERPROFILE", &user_profile)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(
                fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap(),
            ))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!(
                    "item-set lifecycle child timeout: {name}{}",
                    child_failure_logs(&output, name)
                )
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "item-set lifecycle child failed {name} ({status}); inspect {}{}",
            output.display(),
            child_failure_logs(&output, name)
        );
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({"children":children,"fresh_hosts":15,"original_inputs":5,"source_only":true,"complete_load_boundary":true,"native_parity":false,"scope":"fixed finite Load state, reached context and actual call/traversal ordering; exact arrays preserved"})).unwrap()).unwrap();
}

// These checks apply to both import and direct-call captures, including a
// direct GetSpecList call which is deliberately not a hook root. Dynamic
// callbacks may have multiple owner generations; each must be the original
// declaration and retain the actual named self capture.
fn authenticate_loadout_functions(functions: &Json) {
    let functions = functions.as_array().unwrap();
    for (name, source, first, last) in [
        ("sync_loadouts", "Modules/Build.lua", 637, 777),
        ("activate_loadout", "Modules/Build.lua", 956, 980),
        ("lookup_loadout", "Modules/Build.lua", 899, 954),
        ("get_spec_list", "Classes/TreeTab.lua", 484, 490),
        ("activate_spec", "Classes/TreeTab.lua", 540, 578),
        ("activate_skill_set", "Classes/SkillsTab.lua", 1548, 1573),
        ("activate_config_set", "Classes/ConfigTab.lua", 1407, 1434),
        ("loadout_selection", "Modules/Build.lua", 279, 302),
        ("export_spec_selection", "Classes/ImportTab.lua", 375, 378),
        ("export_skill_selection", "Classes/ImportTab.lua", 379, 381),
        ("export_item_selection", "Classes/ImportTab.lua", 382, 384),
    ] {
        let rows = functions
            .iter()
            .filter(|f| f["name"] == name)
            .collect::<Vec<_>>();
        assert!(!rows.is_empty(), "missing original binding {name}");
        for f in rows {
            assert_eq!(f["first_line"], first, "original {name} start");
            assert_eq!(f["last_line"], last, "original {name} end");
            assert_eq!(
                f["source"].as_str().unwrap().replace('\\', "/"),
                format!("@{source}")
            );
            if name.ends_with("_selection") {
                assert_eq!(f["dynamic_callback"], true);
                assert_eq!(f["owner_capture_observed"], true);
                let own = f["captures"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|v| v["name"] == "self")
                    .collect::<Vec<_>>();
                assert_eq!(own.len(), 1);
                assert_eq!(own[0]["kind"], "table");
                assert_eq!(own[0]["owner_identity"], true);
            }
        }
    }
}

// Inspect every observed call/return and retain both root and optional Load
// ancestry. Hook parameters and state projections do not claim raw return packs.
fn inspect_loadout_import(report: &Json) -> Json {
    inspect_returned_loadout_history(&report["additional_observation"], true)
}

fn inspect_returned_loadout_history(o: &Json, require_sync: bool) -> Json {
    let events = o["events"].as_array().unwrap();
    authenticate_loadout_functions(&o["functions"]);
    let states = o["loadout_states"].as_array().unwrap();
    assert!(!states.is_empty(), "loadout import states are required");
    let mut calls = BTreeMap::new();
    let mut returns = BTreeMap::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let mut roots = Vec::new();
    for e in events {
        if e["event"] == "call" {
            let ordinal = e["ordinal"].as_u64().unwrap();
            assert!(calls.insert(ordinal, e).is_none());
            *counts
                .entry(e["name"].as_str().unwrap().into())
                .or_default() += 1;
            if e["parent_call_ordinal"].is_null() {
                roots.push(json!({"ordinal":ordinal,"name":e["name"],"load_call_ordinal":e["load_call_ordinal"],"root_call_ordinal":e["root_call_ordinal"]}));
            }
        } else {
            assert_eq!(e["event"], "return", "unknown observed event kind");
            let call = e["call_ordinal"].as_u64().unwrap();
            assert!(
                calls.contains_key(&call),
                "return must refer to an earlier observed call"
            );
            assert!(
                returns.insert(call, e).is_none(),
                "duplicate observed return"
            );
        }
    }
    assert_eq!(
        calls.len(),
        returns.len(),
        "every observed import call must return"
    );
    for (&ordinal, call) in &calls {
        let returned = returns.get(&ordinal).expect("missing observed return");
        assert_eq!(returned["name"], call["name"]);
        assert!(returned["ordinal"].as_u64().unwrap() > ordinal);
        if let Some(parent) = call["parent_call_ordinal"].as_u64() {
            assert!(parent < ordinal && calls.contains_key(&parent));
            let parent_end = returns.get(&parent).expect("missing parent return");
            assert!(
                returned["ordinal"].as_u64().unwrap() < parent_end["ordinal"].as_u64().unwrap()
            );
        }
        let root = call["root_call_ordinal"].as_u64().unwrap();
        assert!(root <= ordinal && calls.contains_key(&root));
        assert!(calls[&root]["parent_call_ordinal"].is_null());
        let mut ancestor = ordinal;
        let mut load_seen = call["load_call_ordinal"].is_null();
        for depth in 0..=128 {
            assert!(depth < 128, "source parent chain bound");
            let current = calls[&ancestor];
            assert_eq!(current["root_call_ordinal"], root);
            if call["load_call_ordinal"] == ancestor {
                assert_eq!(current["name"], "items_load");
                load_seen = true;
            }
            if let Some(parent) = current["parent_call_ordinal"].as_u64() {
                ancestor = parent;
            } else {
                assert_eq!(ancestor, root);
                break;
            }
        }
        assert!(
            load_seen,
            "optional Load ancestry must refer to an actual ancestor"
        );
    }
    for state in states {
        assert!(
            state["snapshot"]["graph"].is_object(),
            "every loadout state needs a graph"
        );
        let ordinal = state["call_ordinal"].as_u64().unwrap();
        assert!(calls.contains_key(&ordinal));
    }
    if require_sync {
        assert!(counts.get("sync_loadouts").copied().unwrap_or(0) > 0);
    }
    json!({"call_counts":counts,"roots":roots,"complete_observed_calls":calls.len(),"loadout_state_graphs":states.len(),"hook_result_pack_claim":false})
}

fn loadout_history_key(report: &Json) -> Json {
    let states = report["additional_observation"]["loadout_states"]
        .as_array()
        .unwrap();
    Json::Array(states.iter().map(|s| json!({
        "event_ordinal":s["event_ordinal"],"call_ordinal":s["call_ordinal"],
        "root_call_ordinal":s["root_call_ordinal"],"load_call_ordinal":s["load_call_ordinal"],
        "name":s["name"],"phase":s["phase"],"graph":s["snapshot"]["graph"]
    })).collect())
}

// The direct-call producer keeps hook diagnostics and process-local identities
// outside this key. Every supplied case, including a source error, must retain
// its declared joint graph; absence never compares as successful parity.
pub(super) fn direct_loadout_key(direct: &Json) -> Json {
    let cases = direct["cases"].as_array().unwrap();
    assert!(!cases.is_empty() && cases.len() <= 4096);
    assert!(direct["post_loadouts_exact_graph"].is_object());
    case_graph_keys(cases)
}

fn case_graph_keys(cases: &[Json]) -> Json {
    Json::Array(
        cases
            .iter()
            .map(|case| {
                let object = case.as_object().unwrap();
                let mut key = serde_json::Map::new();
                for field in ["label", "operation", "origin", "argument", "status"] {
                    assert!(object.contains_key(field), "direct case field {field}");
                    key.insert(field.into(), case[field].clone());
                }
                assert!(case["before_snapshot"]["graph"].is_object());
                key.insert(
                    "before_graph".into(),
                    case["before_snapshot"]["graph"].clone(),
                );
                assert!(case["snapshot"]["graph"].is_object());
                key.insert("graph".into(), case["snapshot"]["graph"].clone());
                for field in ["argument_before_snapshot", "argument_after_snapshot"] {
                    if let Some(snapshot) = object.get(field) {
                        assert!(snapshot["graph"].is_object());
                        key.insert(field.into(), snapshot["graph"].clone());
                    }
                }
                if let Some(retained) = object.get("retained_first_result_snapshot") {
                    assert!(retained["graph"].is_object());
                    key.insert(
                        "retained_first_result_graph".into(),
                        retained["graph"].clone(),
                    );
                }
                // These are actual equality booleans, not process-local token IDs.
                if let Some(identity) = object.get("first_result_identity") {
                    key.insert("first_result_identity".into(), identity.clone());
                }
                if let Some(error) = object.get("error") {
                    key.insert("error".into(), error.clone());
                }
                Json::Object(key)
            })
            .collect(),
    )
}

fn activation_key(direct: &Json) -> Json {
    let cases = direct["activation_cases"].as_array().unwrap();
    assert!(!cases.is_empty() && cases.len() <= 7);
    assert!(direct["activation_selection"].is_object());
    assert!(direct["post_activation_exact_graph"].is_object());
    json!({"cases":case_graph_keys(cases),"selection":direct["activation_selection"],
        "final_graph":direct["post_activation_exact_graph"]})
}

fn inspect_activations(direct: &Json) -> Json {
    let cases = direct["activation_cases"].as_array().unwrap();
    let observations = direct["activation_observations"].as_array().unwrap();
    assert_eq!(cases.len(), observations.len());
    assert!(!cases.is_empty() && cases.len() <= 7);
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut reached = BTreeMap::<String, usize>::new();
    let mut histories = Vec::new();
    for (case, observation) in cases.iter().zip(observations) {
        assert_eq!(case["operation"], "SetActiveLoadout");
        authenticate_loadout_functions(&observation["functions"]);
        let status = case["status"]["kind"].as_str().unwrap();
        assert!(matches!(status, "returned" | "source_error"));
        *statuses.entry(status.into()).or_default() += 1;
        if status == "returned" {
            assert_eq!(case["status"]["actual_return_count"], 0);
        }
        if case["argument"]["kind"] == "retained_lookup_result" {
            assert!(case["argument_before_snapshot"]["graph"].is_object());
            assert!(case["argument_after_snapshot"]["graph"].is_object());
        } else {
            assert_eq!(case["argument"]["kind"], "nil");
        }
        let observed = observation["scope"]["observed"].as_bool().unwrap();
        let events = match &observation["events"] {
            Json::Array(events) => events.as_slice(),
            Json::Object(events) if !observed && events.is_empty() => &[],
            _ => panic!("invalid activation event array"),
        };
        if observed {
            let roots = events
                .iter()
                .filter(|e| e["event"] == "call" && e["parent_call_ordinal"].is_null())
                .collect::<Vec<_>>();
            assert_eq!(roots.len(), 1, "standalone activation must be observed");
            assert_eq!(roots[0]["name"], "activate_loadout");
            assert!(roots[0]["load_call_ordinal"].is_null());
        } else {
            assert!(
                events.is_empty(),
                "control host must not contain hook events"
            );
        }
        for event in events.iter().filter(|e| e["event"] == "call") {
            *reached
                .entry(event["name"].as_str().unwrap().into())
                .or_default() += 1;
        }
        if observed && status == "returned" {
            histories.push(inspect_returned_loadout_history(observation, false));
        }
        // Failed calls retain their raw event prefix; they are not certified as
        // complete returned histories by the successful-call ancestry checker.
    }
    json!({"cases":cases.len(),"status_counts":statuses,"reached_call_counts":reached,
        "returned_observed_histories":histories,"selection":direct["activation_selection"],
        "failure_histories_diagnostic_only":true,"native_parity":false})
}

fn inspect_direct_loadouts(direct: &Json) -> Json {
    let cases = direct["cases"].as_array().unwrap();
    let observations = direct["observations"].as_array().unwrap();
    assert_eq!(observations.len(), cases.len());
    for observation in observations {
        authenticate_loadout_functions(&observation["functions"]);
    }
    let syncs = cases
        .iter()
        .filter(|c| c["operation"] == "SyncLoadouts")
        .collect::<Vec<_>>();
    assert_eq!(syncs.len(), 4);
    for (index, case) in syncs.iter().enumerate() {
        assert_eq!(case["status"]["kind"], "returned");
        assert_eq!(case["status"]["actual_return_count"], 4);
        if index > 0 {
            let identity = &case["first_result_identity"];
            assert_eq!(identity["first_n"], 4);
            assert_eq!(identity["current_n"], 4);
            let positions = identity["positions"].as_array().unwrap();
            assert_eq!(positions.len(), 4);
            for position in positions {
                assert_eq!(position["first_is_table"], true);
                assert_eq!(position["current_is_table"], true);
                assert_eq!(
                    position["same_table"], false,
                    "source Sync allocates fresh return tables"
                );
            }
            assert!(case["retained_first_result_snapshot"]["graph"].is_object());
        }
    }
    let specs = cases
        .iter()
        .filter(|c| c["operation"] == "GetSpecList")
        .collect::<Vec<_>>();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0]["status"]["kind"], "returned");
    assert_eq!(specs[0]["status"]["actual_return_count"], 1);
    let counts = &direct["lookup_occurrences"];
    let dropdown = counts["dropdown"].as_u64().unwrap();
    let spec = counts["spec_display"].as_u64().unwrap();
    assert!(dropdown > 0 && spec > 0);
    assert_eq!(counts["absent"], 2);
    let lookups = cases
        .iter()
        .filter(|c| c["operation"] == "GetLoadoutByName")
        .count();
    assert_eq!(lookups as u64, dropdown + spec + 2);
    assert_eq!(cases.len(), lookups + 5);
    let mut statuses = BTreeMap::<String, usize>::new();
    for case in cases {
        let kind = case["status"]["kind"].as_str().unwrap();
        assert!(matches!(kind, "returned" | "source_error"));
        *statuses.entry(kind.into()).or_default() += 1;
    }
    json!({"cases":cases.len(),"status_counts":statuses,"sync_return_packs":4,"fresh_return_tables_against_first":12,"lookup_occurrences":counts,"all_cases_have_joint_graphs":true})
}

fn child_loadouts(repo: &Path, output: &Path, entry: &Json) {
    let name = entry["xml"].as_str().unwrap();
    let xml = fs::read_to_string(
        repo.join("tests/fixtures/builds/breadth-20260908")
            .join(name),
    )
    .unwrap();
    assert_eq!(hash(xml.as_bytes()), entry["xml_sha256"]);
    let directory = output.join(name);
    fs::create_dir_all(&directory).unwrap();
    let mut reports = Vec::new();
    for (label, observed) in [
        ("control", false),
        ("observed-a", true),
        ("observed-b", true),
    ] {
        let report = host_mode(
            repo,
            &directory.join(format!("{label}-host")),
            &xml,
            observed,
            true,
        );
        fs::write(
            directory.join(format!("{label}.json")),
            serde_json::to_vec_pretty(&report).unwrap(),
        )
        .unwrap();
        reports.push(report);
    }
    let control = &reports[0];
    let mut comparisons = Vec::new();
    for (i, label) in [(1, "observed-a"), (2, "observed-b")] {
        let report = &reports[i];
        let c = &control["additional_observation"];
        let o = &report["additional_observation"];
        comparisons.push(json!({
            "host":label,
            "source_hash_equal":control["source_hash"] == report["source_hash"],
            "selected_equal":control["selected"] == report["selected"],
            "import_exact_graph_equal":c["post_import_exact_graph"] == o["post_import_exact_graph"],
            "import_declared_control_equal":c["post_import_control_key"] == o["post_import_control_key"],
            "import_loadout_graph_equal":c["post_import_loadouts"]["graph"] == o["post_import_loadouts"]["graph"],
            "direct_cases_equal":direct_loadout_key(&c["direct_loadouts"]) == direct_loadout_key(&o["direct_loadouts"]),
            "direct_final_graph_equal":c["direct_loadouts"]["post_loadouts_exact_graph"] == o["direct_loadouts"]["post_loadouts_exact_graph"],
            "activation_cases_equal":activation_key(&c["direct_loadouts"]) == activation_key(&o["direct_loadouts"])
        }));
    }
    let history_equal = loadout_history_key(&reports[1]) == loadout_history_key(&reports[2]);
    // Preserve both raw reports and these booleans before any equality assertion.
    let mut summary = json!({"xml":name,"xml_sha256":entry["xml_sha256"],"comparisons":comparisons,"observed_loadout_histories_equal":history_equal,
        "scope":{"source_only":true,"native_parity":false,"original_import_roots_separate_from_direct_calls":true,"identity_tokens_compared_between_hosts":false,"joint_spec_result_aliases_retained":true,"exact_arrays_retained":true,"import_intermediate_history_equality_is_diagnostic":true}});
    let result_path = output.join(format!("{name}.json"));
    fs::write(&result_path, serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
    for row in summary["comparisons"].as_array().unwrap() {
        for key in [
            "source_hash_equal",
            "selected_equal",
            "import_declared_control_equal",
            "import_loadout_graph_equal",
            "direct_cases_equal",
            "direct_final_graph_equal",
            "activation_cases_equal",
        ] {
            assert_eq!(
                row[key], true,
                "{name}: {key}; inspect retained control/observed reports"
            );
        }
    }
    // Debug ordinals and source traversal may differ between fresh hosts.
    // Keep this exact-history comparison diagnostic and prove each host's
    // call/return/parent contract independently below.
    let proofs = [
        inspect_loadout_import(&reports[1]),
        inspect_loadout_import(&reports[2]),
    ];
    let direct_cases = reports[0]["additional_observation"]["direct_loadouts"]["cases"]
        .as_array()
        .unwrap();
    assert!(
        !direct_cases.is_empty(),
        "direct source cases must be represented"
    );
    summary["observed_import"] = json!(proofs);
    summary["complete_items_loads"] =
        json!([inspect(&reports[1], &xml), inspect(&reports[2], &xml)]);
    summary["direct_case_count"] = direct_cases.len().into();
    summary["direct_hosts"] = Json::Array(
        reports
            .iter()
            .map(|report| {
                inspect_direct_loadouts(&report["additional_observation"]["direct_loadouts"])
            })
            .collect(),
    );
    summary["activation_hosts"] = Json::Array(
        reports
            .iter()
            .map(|report| inspect_activations(&report["additional_observation"]["direct_loadouts"]))
            .collect(),
    );
    fs::write(result_path, serde_json::to_vec_pretty(&summary).unwrap()).unwrap();
}

pub fn run_loadouts() {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let index: Json = serde_json::from_slice(
        &fs::read(repo.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
    )
    .unwrap();
    let builds = index["builds"].as_array().unwrap();
    assert_eq!(builds.len(), 5);
    let output = std::env::var_os(LOADOUT_OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| repo.join("runs/r2an-loadout-sync-01/source"));
    fs::create_dir_all(&output).unwrap();
    // Export paths read USERPROFILE before the per-host scratch fallback. Give
    // every fresh host in a child the same explicit, test-owned process input.
    let user_profile = output.canonicalize().unwrap();
    if let Ok(name) = std::env::var(LOADOUT_CHILD) {
        child_loadouts(
            &repo,
            &output,
            builds.iter().find(|v| v["xml"] == name).unwrap(),
        );
        return;
    }
    let mut children = Vec::new();
    let mut total_cases = 0usize;
    let mut total_activation_cases = 0u64;
    let mut reached_activation_calls = BTreeMap::<String, u64>::new();
    for entry in builds {
        let name = entry["xml"].as_str().unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", LOADOUT_TEST, "--nocapture"])
            .env(LOADOUT_CHILD, name)
            .env(LOADOUT_OUTPUT, &output)
            .env("USERPROFILE", &user_profile)
            .current_dir(repo.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(
                fs::File::create(output.join(format!("{name}.stdout.log"))).unwrap(),
            ))
            .stderr(Stdio::from(
                fs::File::create(output.join(format!("{name}.stderr.log"))).unwrap(),
            ))
            .spawn()
            .unwrap();
        let start = Instant::now();
        let status = loop {
            if let Some(status) = process.try_wait().unwrap() {
                break status;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                let _ = process.wait();
                panic!(
                    "loadout lifecycle child timeout: {name}{}",
                    child_failure_logs(&output, name)
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        children.push(json!({"xml":name,"exit":status.code()}));
        fs::write(
            output.join("children.json"),
            serde_json::to_vec_pretty(&children).unwrap(),
        )
        .unwrap();
        assert!(
            status.success(),
            "loadout lifecycle child failed {name} ({status}); inspect {}{}",
            output.display(),
            child_failure_logs(&output, name)
        );
        let summary: Json =
            serde_json::from_slice(&fs::read(output.join(format!("{name}.json"))).unwrap())
                .unwrap();
        total_cases += summary["direct_case_count"].as_u64().unwrap() as usize;
        let hosts = summary["activation_hosts"].as_array().unwrap();
        assert_eq!(hosts.len(), 3);
        total_activation_cases += hosts[0]["cases"].as_u64().unwrap();
        // One observed lane per original avoids double-counting repeat witnesses.
        for (name, count) in hosts[1]["reached_call_counts"].as_object().unwrap() {
            *reached_activation_calls.entry(name.clone()).or_default() += count.as_u64().unwrap();
        }
    }
    fs::write(output.join("summary.json"), serde_json::to_vec_pretty(&json!({"children":children,"original_inputs":5,"fresh_hosts":15,"direct_cases_per_control_lane":total_cases,"activation_cases_per_control_lane":total_activation_cases,"activation_calls_per_observed_lane":reached_activation_calls,"source_only":true,"native_parity":false,"scope":"all original import loadout roots plus separately supplied direct post-import calls; post-import and direct pre/post joint graphs compared; import intermediate histories retained diagnostically with per-host ancestry checks; raw direct result arity and alias ownership; separate retained-lookup activations with original changed-domain callbacks, exact argument/pre/post graphs and successful-call ancestry"})).unwrap()).unwrap();
}

#[test]
fn child_failure_log_diagnostics_preserve_missing_short_and_bounded_tails() {
    let directory = tempfile::tempdir().unwrap();
    let missing = child_failure_logs(directory.path(), "probe");
    assert_eq!(missing.matches("could not read child log:").count(), 2);

    fs::write(
        directory.path().join("probe.stderr.log"),
        b"reached child error\n",
    )
    .unwrap();
    let suffix = b"\nlast child stdout record\n";
    let mut oversized = vec![b'x'; 8192];
    oversized.extend_from_slice(suffix);
    fs::write(directory.path().join("probe.stdout.log"), oversized).unwrap();
    let diagnostics = child_failure_logs(directory.path(), "probe");
    assert!(diagnostics.contains("reached child error\n"));
    assert!(diagnostics.ends_with(std::str::from_utf8(suffix).unwrap()));
    assert!(diagnostics.contains(&"x".repeat(4096 - suffix.len())));
    assert!(!diagnostics.contains(&"x".repeat(4097)));
    assert!(!diagnostics.contains("could not read child log:"));
}
