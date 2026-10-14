//! Fresh original-runtime gates for the unchanged selected build; no native closure.
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

const TEST: &str = "unchanged_self_flat_gates_are_original_and_repeatable";
const CHILD: &str = "POE_SELF_FLAT_GATE_SOURCE_CHILD";
const OUTPUT: &str = "POE_SELF_FLAT_GATE_SOURCE_OUT";
const COMMON: &str = include_str!("support/owned_minion_physical_damage_source.lua");
const OBSERVER: &str = include_str!("support/self_flat_gate_observer.lua");
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn helpers(lua: &Lua) -> Result<Table, RuntimeError> {
    let marker = "local methods={";
    assert_eq!(COMMON.matches(marker).count(), 1);
    let prefix = COMMON.split_once(marker).unwrap().0;
    // Reuse the exact existing observation helpers, without installing the old
    // physical-damage hook or executing any calculation of our own.
    let code = format!(
        "{prefix}\nreturn {{original=original,upvalue=upvalue,scalar=scalar,scalars=scalars,clone=clone,equal=equal,watchStores=watchStores,modRecord=modRecord,sourceOccurrence=sourceOccurrence,recipientOccurrence=recipientOccurrence,sourceConfig=sourceConfig,records=records,rawRecords=rawRecords,offeringOutputSnapshot=offeringOutputSnapshot}}"
    );
    Ok(lua
        .load(code)
        .set_name("@existing-physical-observer-helpers")
        .eval()?)
}
fn equal(left: &Json, right: &Json, label: &str) {
    assert_eq!(
        json_evidence::first_difference(left, right, "$"),
        None,
        "{label}"
    );
}

#[test]
#[ignore = "requires fresh complete original PoB runtime; bounded unchanged-build gate evidence"]
fn unchanged_self_flat_gates_are_original_and_repeatable() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let out = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("runs/owned-self-flat-gate-source-01"));
    let out = if out.is_absolute() {
        out
    } else {
        root.join(out)
    };
    if let Some(mode) = std::env::var_os(CHILD) {
        assert!(mode == "off" || mode == "on");
        run_child(&root, &out, mode == "on");
        return;
    }
    assert!(!out.exists(), "fresh acquisition directory required");
    fs::create_dir_all(&out).unwrap();
    for mode in ["off", "on"] {
        let path = out.join(format!("source-jit-{mode}.log"));
        let log = fs::File::create(&path).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--ignored", "--nocapture"])
            .env(CHILD, mode)
            .env(OUTPUT, &out)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "original source child failed; inspect {}",
                    path.display()
                );
                break;
            }
            if started.elapsed() > Duration::from_secs(600) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("original source child deadline; inspect {}", path.display())
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
    json_evidence::assert_files_equal(
        &out.join("source-jit-off.json"),
        &out.join("source-jit-on.json"),
        "unchanged self-flat gate evidence JIT controls",
    );
}

fn run_child(root: &Path, out: &Path, jit: bool) {
    let xml = fs::read_to_string(root.join(XML)).unwrap();
    assert_eq!(
        hash(xml.as_bytes()),
        "442e048f4bc2d69c05bed2a7cda68580abb5c32f96990ad70f77b8ca614fe089"
    );
    let mut observations = Vec::new();
    for (label, instrumented) in [("observed", true), ("repeat", true), ("plain", false)] {
        eprintln!("self-flat gate original case {label}; plain JIT {jit}");
        let before = |lua: &Lua| -> Result<(), RuntimeError> {
            lua.globals()
                .set("physicalDamagePreconversionEvidence", true)?;
            lua.globals()
                .set("physicalDamagePhase", "self_flat_helpers")?;
            lua.globals().set("selfFlatJit", jit && !instrumented)?;
            lua.load("if selfFlatJit then jit.on() else jit.off();jit.flush() end")
                .exec()?;
            Ok(())
        };
        let install = |lua: &Lua| -> Result<Function, RuntimeError> {
            assert!(!lua.load("return jit.status()").eval::<bool>()?);
            let factory: Function = lua
                .load(OBSERVER)
                .set_name("@self-flat-gate-observer")
                .eval()?;
            Ok(factory.call(helpers(lua)?)?)
        };
        let observe = |lua: &Lua| -> Result<Json, RuntimeError> {
            assert_eq!(
                lua.load("return jit.status()").eval::<bool>()?,
                jit && !instrumented
            );
            let value: Value = if instrumented {
                lua.globals().get("selfFlatGateObservation")?
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
            Some(&observe),
        )
        .unwrap_or_else(|e| panic!("{label}: {e}"));
        fs::write(
            out.join(format!(
                "jit-{}-{label}-raw.json",
                if jit { "on" } else { "off" }
            )),
            serde_json::to_vec(&observed).unwrap(),
        )
        .unwrap();
        assert_eq!(observed["configuration_method_wrappers"], false);
        assert_eq!(observed["original_build_output_available"], true);
        assert_eq!(observed["source_hash"], pinned::manifest_sha256());
        observations.push(observed["additional_observation"].clone());
    }
    equal(
        &observations[0],
        &observations[1],
        "independent fresh original runs",
    );
    equal(
        &observations[0]["snapshot"],
        &observations[2],
        "instrumented versus uninstrumented full scalar outputs",
    );
    let report = json!({"schema_version":1,"kind":"unchanged-selected-self-flat-runtime-gates","source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "xml":{"path":XML,"sha256":hash(xml.as_bytes())},"observer_sha256":hash(OBSERVER.as_bytes()),"shared_helpers_file_sha256":hash(COMMON.as_bytes()),
        "driver_sha256":hash(include_bytes!("owned_self_flat_gate_source.rs")),"bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "complete_loads_per_mode":3,"independent_fresh_observations":2,"plain_controls_per_mode":1,"instrumented_jit_enabled":false,"plain_jit_mode_from_filename":true,
        "scope":{"native_coverage_authorized":false,"whole_build_parity":false,"unchanged_build_only":true,"query_probes_are_supplemental":true,"original_list_returns_observed":true,"normalization":false},
        "state":observations[0]});
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec(&report).unwrap(),
    )
    .unwrap();
    check(&report);
}

fn rows(value: &Json) -> &[Json] {
    value.as_array().map(Vec::as_slice).unwrap_or_else(|| {
        assert_eq!(value, &json!({}));
        &[]
    })
}
fn check(report: &Json) {
    assert_eq!(report["complete_loads_per_mode"], 3);
    assert_eq!(report["scope"]["native_coverage_authorized"], false);
    let state = &report["state"];
    assert_eq!(
        state["selected"],
        json!({"items":2,"skills":4,"spec":3,"config":1,"main_group":3})
    );
    assert_eq!(state["original_functions_preserved"], true);
    assert_eq!(state["business_methods_replaced"], false);
    assert!(
        !rows(&state["parent_weapon"]).is_empty(),
        "must observe original parent-weapon gate"
    );
    assert!(
        !rows(&state["hollow_palm"]).is_empty(),
        "must observe original Hollow Palm gate"
    );
    assert!(
        rows(&state["aura_queries"])
            .iter()
            .any(|r| r["name"] == "ExtraAura"),
        "must observe original ExtraAura consumer"
    );
    assert!(
        !rows(&state["child_visits"]).is_empty(),
        "must observe supplied-child participation"
    );
    for row in rows(&state["parent_weapon"]) {
        assert_eq!(row["exact_actor_store"], true);
        assert_eq!(row["flag"]["query_state_preserved"], true);
    }
    for row in rows(&state["hollow_palm"]) {
        assert_eq!(row["hollow_palm"]["query_state_preserved"], true);
        assert_eq!(row["facebreaker"]["query_state_preserved"], true);
    }
    for row in rows(&state["aura_queries"]) {
        assert_eq!(row["original_list_return_observed"], true);
        assert_eq!(row["exact_store"], true);
    }
}
