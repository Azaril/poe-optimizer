//! Complete original Load observation; no native/equipment parity is inferred.
#[path = "item_assembly_graph.rs"]
mod graph;
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
    path::{Path, PathBuf},
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
const OBSERVER: &str = include_str!("item_set_lifecycle.lua");
const TEST: &str = "all_five_complete_original_item_set_load_lifecycles";
const CHILD: &str = "POE_ITEM_SET_LIFECYCLE_CHILD";
const OUTPUT: &str = "POE_ITEM_SET_LIFECYCLE_OUTPUT";
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
fn host(repo: &Path, directory: &Path, xml: &str, observed: bool) -> Json {
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
    let before_build = |_lua: &Lua| -> Result<Function, RuntimeError> {
        let active: Table = module
            .borrow()
            .as_ref()
            .unwrap()
            .raw_get::<Function>("start")?
            .call(observed)?;
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
                panic!("item-set lifecycle child timeout: {name}")
            }
            std::thread::sleep(Duration::from_millis(50));
        };
        assert!(
            status.success(),
            "item-set lifecycle child failed {name}; inspect {}",
            output.display()
        );
        children.push(json!({"xml":name,"exit":status.code()}));
    }
    fs::write(output.join("summary.json"),serde_json::to_vec_pretty(&json!({"children":children,"fresh_hosts":15,"original_inputs":5,"source_only":true,"complete_load_boundary":true,"native_parity":false,"scope":"fixed finite Load state, reached context and actual call/traversal ordering; exact arrays preserved"})).unwrap()).unwrap();
}
