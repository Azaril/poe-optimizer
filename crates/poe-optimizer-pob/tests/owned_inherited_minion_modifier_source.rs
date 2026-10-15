//! Optional temporal capture for the unchanged selected build.
//! Stable scalars are required; receiving semantics require the separate offline proof.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/inherited_minion_capture_assessment.rs"]
mod assessment;
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

const TEST: &str = "unchanged_inherited_minion_modifiers_are_captured_with_stable_scalars";
const CHILD: &str = "POE_INHERITED_MINION_SOURCE_CHILD";
const OUTPUT: &str = "POE_INHERITED_MINION_SOURCE_OUT";
const XML: &str = "tests/fixtures/builds/breadth-20260908/build-05.xml";
const COMMON: &str = include_str!("support/owned_minion_physical_damage_source.lua");
const OBSERVER: &str = include_str!("support/inherited_minion_modifier_observer.lua");
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
            "{prefix}\nreturn {{original=original,upvalue=upvalue,scalar=scalar,scalars=scalars,clone=clone,equal=equal,sourceOccurrence=sourceOccurrence,recipientOccurrence=recipientOccurrence,sourceConfig=sourceConfig,offeringOutputSnapshot=offeringOutputSnapshot}}"
        ))
        .set_name("@existing-source-helpers")
        .eval()?)
}

#[test]
fn inherited_observer_parses_without_executing_source() {
    Lua::new().load(OBSERVER).into_function().unwrap();
}

#[test]
#[ignore = "four fresh unchanged original PoB loads; explicit optional source acquisition"]
fn unchanged_inherited_minion_modifiers_are_captured_with_stable_scalars() {
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
            include_bytes!("owned_inherited_minion_modifier_source.rs").as_slice(),
        ),
        ("acquisition-observer.lua", OBSERVER.as_bytes()),
        ("acquisition-helpers.lua", COMMON.as_bytes()),
        ("acquisition-bootstrap.rs", BOOTSTRAP),
        (
            "acquisition-assessment.rs",
            include_bytes!("support/inherited_minion_capture_assessment.rs"),
        ),
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
    let files: Vec<_> = labels
        .iter()
        .map(|label| {
            let file = format!("{label}.json");
            let bytes = fs::read(out.join(&file)).unwrap();
            json!({"file":file,"bytes":bytes.len(),"sha256":hash(&bytes)})
        })
        .collect();
    let seal = json!({"schema_version":1,"kind":"fixed-inherited-minion-capture-seal",
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "xml":{"path":XML,"sha256":hash(&fs::read(root.join(XML)).unwrap())},
        "driver_sha256":hash(include_bytes!("owned_inherited_minion_modifier_source.rs")),
        "observer_sha256":hash(OBSERVER.as_bytes()),"helper_sha256":hash(COMMON.as_bytes()),
        "bootstrap_sha256":hash(BOOTSTRAP),
        "assessment_sha256":hash(include_bytes!("support/inherited_minion_capture_assessment.rs")),
        "files":files,"offline_receiving_proof_required":true});
    // Seal every raw capture and exact code identity before assessing outputs.
    fs::write(
        out.join("capture-seal.json"),
        serde_json::to_vec(&seal).unwrap(),
    )
    .unwrap();
    let assessed = assessment::assess(&loads);
    let report = json!({"schema_version":1,"kind":"original-inherited-minion-capture-assessment",
        "capture_seal":seal,"complete_loads":4,"instrumented_loads":2,
        "instrumented_jit_enabled":false,"plain_jit_modes":[false,true],
        "assessment":assessed,"native_coverage_authorized":false});
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
        lua.globals().set("inheritedMinionJit", jit)?;
        lua.globals()
            .set("physicalDamagePreconversionEvidence", true)?;
        lua.load("if inheritedMinionJit then jit.on() else jit.off();jit.flush() end")
            .exec()?;
        Ok(())
    };
    let install = |lua: &Lua| -> Result<Function, RuntimeError> {
        assert!(!lua.load("return jit.status()").eval::<bool>()?);
        let factory: Function = lua
            .load(OBSERVER)
            .set_name("@inherited-minion-modifier-observer")
            .eval()?;
        Ok(factory.call(helpers(lua)?)?)
    };
    let after = |lua: &Lua| -> Result<Json, RuntimeError> {
        assert_eq!(lua.load("return jit.status()").eval::<bool>()?, jit);
        let value: Value = if instrumented {
            lua.globals().get("inheritedMinionModifierObservation")?
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
