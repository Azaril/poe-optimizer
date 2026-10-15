//! Optional input/lifecycle witness for the unchanged selected passive radius path.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/json_evidence.rs"]
mod json_evidence;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_pob::{runtime::RuntimeError, source as pinned};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

const TEST: &str = "unchanged_radius_inputs_are_original_and_repeatable";
const CHILD: &str = "POE_MANA_RADIUS_SOURCE_CHILD";
const OUTPUT: &str = "POE_MANA_RADIUS_SOURCE_OUT";
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const COMMON: &str = include_str!("support/owned_minion_physical_damage_source.lua");
const OBSERVER: &str = include_str!("support/mana_radius_observer.lua");
const BOOTSTRAP: &[u8] = include_bytes!("support/configuration_preparation_source.rs");

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn helpers(lua: &Lua) -> Result<Table, RuntimeError> {
    let marker = "local methods={";
    assert_eq!(COMMON.matches(marker).count(), 1);
    let prefix = COMMON.split_once(marker).unwrap().0;
    Ok(lua
        .load(format!(
            "{prefix}\nreturn {{original=original,scalar=scalar,scalars=scalars,offeringOutputSnapshot=offeringOutputSnapshot}}"
        ))
        .set_name("@existing-source-helpers")
        .eval()?)
}
fn equal(a: &Json, b: &Json, label: &str) {
    assert_eq!(json_evidence::first_difference(a, b, "$"), None, "{label}");
}
fn validate_trace(trace: &Json) {
    assert_eq!(trace["kind"], "original-mana-radius-inputs");
    assert_eq!(trace["original_functions_preserved"], true);
    assert_eq!(trace["observer_removed"], true);
    assert_eq!(trace["scope"]["native_coverage_authorized"], false);
    let calls = trace["init_calls"].as_array().unwrap();
    let lists = trace["list_calls"].as_array().unwrap();
    assert!(!calls.is_empty() && !lists.is_empty());
    for (at, call) in calls.iter().enumerate() {
        assert_eq!(call["invocation_id"], at + 1);
        assert_eq!(call["returned"], true);
        assert!(call["env_id"].as_u64().is_some_and(|n| n > 0));
        if let Some(parent) = call["parent_invocation_id"].as_u64() {
            assert!(parent > 0 && parent <= at as u64);
        } else {
            assert_eq!(call["parent_invocation_id"], false);
        }
    }
    for (at, list) in lists.iter().enumerate() {
        assert_eq!(list["list_id"], at + 1);
        assert_eq!(list["returned"], true);
    }
    for family in ["list_calls", "node_calls", "slot_candidates"] {
        for row in trace[family].as_array().unwrap() {
            let id = row["invocation_id"].as_u64().unwrap() as usize;
            assert!(id > 0 && id <= calls.len());
            assert_eq!(row["env_id"], calls[id - 1]["env_id"]);
        }
    }
    for (at, row) in trace["node_calls"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["node_call_id"], at + 1);
        assert_eq!(row["returned"], true);
        if let Some(id) = row["list_id"].as_u64() {
            assert!(id > 0 && id as usize <= lists.len());
            let list = &lists[id as usize - 1];
            assert_eq!(row["invocation_id"], list["invocation_id"]);
            assert_eq!(row["env_id"], list["env_id"]);
        } else {
            assert_eq!(row["list_id"], false);
        }
    }
}

// buildModListForNodeList visits a keyed node inventory via pairs. Compare only
// that counted visit family independently of traversal order. All invocation,
// environment and list bindings, bodies, duplicate counts and input values stay
// exact. Raw visit sequence/IDs remain in the acquisition files. This grants no
// arithmetic reordering and does not normalize slot priority or scalar output.
fn comparison(trace: &Json) -> Json {
    let mut value = trace.clone();
    let rows = value["node_calls"].as_array_mut().unwrap();
    for row in rows.iter_mut() {
        row.as_object_mut().unwrap().remove("node_call_id");
    }
    rows.sort_by_cached_key(|row| serde_json::to_string(row).unwrap());
    value
}

#[test]
fn radius_observer_parses_and_counted_visits_preserve_bindings() {
    Lua::new().load(OBSERVER).into_function().unwrap();
    let a = json!({"node_call_id":1,"invocation_id":1,"env_id":2,"list_id":3,
        "node":{"id":4},"before_radius":{"radius_jewels":[]}});
    let b = json!({"node_call_id":2,"invocation_id":1,"env_id":2,"list_id":3,
        "node":{"id":5},"before_radius":{"radius_jewels":[]}});
    let original = json!({"node_calls":[a.clone(),b.clone()]});
    let mut swapped = json!({"node_calls":[b,a]});
    swapped["node_calls"][0]["node_call_id"] = json!(1);
    swapped["node_calls"][1]["node_call_id"] = json!(2);
    assert_eq!(comparison(&original), comparison(&swapped));
    for field in ["invocation_id", "env_id", "list_id"] {
        let mut changed = swapped.clone();
        changed["node_calls"][0][field] = json!(999);
        assert_ne!(comparison(&original), comparison(&changed));
    }
    let mut missing = swapped.clone();
    missing["node_calls"].as_array_mut().unwrap().pop();
    assert_ne!(comparison(&original), comparison(&missing));
    swapped["node_calls"][0]["before_radius"]["radius_jewels"] = json!([{"item":0}]);
    assert_ne!(comparison(&original), comparison(&swapped));
}

#[test]
#[ignore = "four fresh unchanged original PoB loads; explicit optional source acquisition"]
fn unchanged_radius_inputs_are_original_and_repeatable() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = PathBuf::from(std::env::var_os(OUTPUT).expect("fresh output directory"));
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(label) = std::env::var_os(CHILD) {
        child(&root, &out, label.to_str().unwrap());
        return;
    }
    assert!(!out.exists(), "fresh acquisition directory required");
    fs::create_dir_all(&out).unwrap();
    for (name, bytes) in [
        (
            "acquisition-driver.rs",
            include_bytes!("owned_mana_radius_source.rs").as_slice(),
        ),
        ("acquisition-observer.lua", OBSERVER.as_bytes()),
        ("acquisition-helpers.lua", COMMON.as_bytes()),
        ("acquisition-bootstrap.rs", BOOTSTRAP),
    ] {
        fs::write(out.join(name), bytes).unwrap();
    }
    let labels = ["observed-a", "observed-b", "plain-off", "plain-on"];
    for label in labels {
        let log = fs::File::create(out.join(format!("{label}.log"))).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, label)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(status.success(), "inspect {label}.log");
                break;
            }
            if start.elapsed() > Duration::from_secs(600) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source deadline: {label}.log");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    let loads: Vec<Json> = labels
        .iter()
        .map(|label| {
            serde_json::from_slice(&fs::read(out.join(format!("{label}.json"))).unwrap()).unwrap()
        })
        .collect();
    let a = &loads[0]["additional_observation"];
    let b = &loads[1]["additional_observation"];
    validate_trace(a);
    validate_trace(b);
    equal(
        &comparison(a),
        &comparison(b),
        "independent unchanged radius input traces",
    );
    for (label, output) in [
        ("second observed", &b["snapshot"]),
        ("plain JIT off", &loads[2]["additional_observation"]),
        ("plain JIT on", &loads[3]["additional_observation"]),
    ] {
        equal(&a["snapshot"], output, label);
    }
    let files: Vec<_> = labels
        .iter()
        .map(|label| {
            let file = format!("{label}.json");
            let bytes = fs::read(out.join(&file)).unwrap();
            json!({"file":file,"bytes":bytes.len(),"sha256":hash(&bytes)})
        })
        .collect();
    let report = json!({"schema_version":1,"kind":"original-mana-radius-input-evidence",
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "xml":{"path":XML,"sha256":hash(&fs::read(root.join(XML)).unwrap())},
        "driver_sha256":hash(include_bytes!("owned_mana_radius_source.rs")),
        "observer_sha256":hash(OBSERVER.as_bytes()),"helper_sha256":hash(COMMON.as_bytes()),
        "bootstrap_sha256":hash(BOOTSTRAP),"files":files,
        "complete_loads":4,"instrumented_loads":2,"instrumented_jit_enabled":false,
        "plain_jit_modes":[false,true],"all_scalar_outputs_exact":true,
        "comparison":{"counted_node_visits_only":true,"raw_sequence_retained":true,
            "slot_and_lifecycle_order_exact":true,"native_order_authority":false},
        "counts":{"init_calls":a["init_calls"].as_array().unwrap().len(),
            "node_calls":a["node_calls"].as_array().unwrap().len(),
            "list_calls":a["list_calls"].as_array().unwrap().len(),
            "slot_candidates":a["slot_candidates"].as_array().unwrap().len()},
        "observed":[a,b],"native_coverage_authorized":false});
    fs::write(
        out.join("source.json"),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
}

fn child(root: &Path, out: &Path, label: &str) {
    assert!(matches!(
        label,
        "observed-a" | "observed-b" | "plain-off" | "plain-on"
    ));
    let instrumented = label.starts_with("observed-");
    let jit = label == "plain-on";
    let xml = fs::read_to_string(root.join(XML)).unwrap();
    assert_eq!(
        hash(xml.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let before = |lua: &Lua| -> Result<(), RuntimeError> {
        lua.globals().set("manaRadiusJit", jit)?;
        lua.load("if manaRadiusJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        assert!(!lua.load("return jit.status()").eval::<bool>()?);
        let factory: Function = lua
            .load(OBSERVER)
            .set_name("@mana-radius-observer")
            .eval()?;
        Ok(factory.call(helpers(lua)?)?)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        assert_eq!(lua.load("return jit.status()").eval::<bool>()?, jit);
        let value: Value = if instrumented {
            lua.globals().get("manaRadiusObservation")?
        } else {
            helpers(lua)?
                .get::<Function>("offeringOutputSnapshot")?
                .call(())?
        };
        Ok(lua.from_value(value)?)
    };
    let scratch = tempfile::tempdir().unwrap();
    let observed = source::observe_with_build_hook_unwrapped(
        &root.join("vendor/path-of-building-poe2"),
        scratch.path(),
        &xml,
        None,
        false,
        Some(&before),
        instrumented.then_some(&install),
        Some(&after),
    )
    .unwrap_or_else(|error| panic!("{label}: {error}"));
    assert_eq!(observed["configuration_method_wrappers"], false);
    assert_eq!(observed["source_hash"], pinned::manifest_sha256());
    assert_eq!(observed["original_build_output_available"], true);
    fs::write(
        out.join(format!("{label}.json")),
        serde_json::to_vec(&observed).unwrap(),
    )
    .unwrap();
}
