//! Opt-in elapsed lifecycle investigation on actual initialized original parser state.
//! No successful uncached-parser, full-build native, or allocator/RSS claim.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[path = "support/profile_source_session_history.rs"]
mod history;
#[path = "support/profile_source_session_native.rs"]
mod native;
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_parser_capture.rs"]
mod parser_capture;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use classes::Primitives;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::source_program::{SourceCallbackId, SourceValue};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
use serde::Serialize;
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    hint::black_box,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const TEST: &str = "profile_original_source_session_lifecycles";
const CHILD: &str = "POE_A1_SESSION_CHILD";
const OUTPUT: &str = "POE_A1_SESSION_OUTPUT";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn json_hash(value: &impl Serialize) -> String {
    hash(&serde_json::to_vec(value).unwrap())
}
fn limits() -> ProgramLimits {
    ProgramLimits {
        max_steps: 50_000_000,
        max_values: 8_000_000,
        max_bytes: 256 * 1024 * 1024,
        max_tables: 100_000,
        pattern: MatchLimits {
            max_steps: 200_000_000,
            ..MatchLimits::default()
        },
        ..ProgramLimits::default()
    }
}
#[derive(Serialize)]
struct Phase {
    name: String,
    elapsed_ms: f64,
}
fn timed<T>(phases: &mut Vec<Phase>, name: &str, f: impl FnOnce() -> T) -> T {
    let start = Instant::now();
    let result = black_box(f());
    let elapsed_ms = start.elapsed().as_secs_f64() * 1000.0;
    phases.push(Phase {
        name: name.into(),
        elapsed_ms,
    });
    result
}
struct Corpus {
    capture: parser_capture::CapturedParser,
    history: history::History,
    copy_callback: SourceCallbackId,
    identity: Json,
}
// A global keeps this userdata reachable until the actual mlua host is destroyed.
// No source function is replaced; the projected environment excludes this key.
struct HostDrop(Arc<AtomicBool>);
impl mlua::UserData for HostDrop {}
impl Drop for HostDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

fn acquire(lua: &Lua, primitives: &Primitives) -> Corpus {
    let jit: Table = lua.globals().raw_get("jit").unwrap();
    jit.raw_get::<Function>("off")
        .unwrap()
        .call::<()>(())
        .unwrap();
    jit.raw_get::<Function>("flush")
        .unwrap()
        .call::<()>(())
        .unwrap();
    let modlib: Table = lua.globals().raw_get("modLib").unwrap();
    let parser: Function = modlib.raw_get("parseMod").unwrap();
    let cache: Table = modlib.raw_get("parseModCache").unwrap();
    assert_eq!(
        primitives.captured_value(&parser, "cache"),
        Value::Table(cache.clone())
    );
    let inner = primitives
        .captured_value(&parser, "parseMod")
        .as_function()
        .unwrap()
        .clone();
    let scan = primitives
        .captured_value(&inner, "scan")
        .as_function()
        .unwrap()
        .clone();
    let dictionaries = parser_capture::DICTIONARIES
        .iter()
        .map(|name| {
            (
                (*name).to_owned(),
                primitives
                    .captured_value(&inner, name)
                    .as_table()
                    .unwrap()
                    .clone(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let probes: Table = lua
        .load(parser_capture::TEXT)
        .set_name(format!("@{}", parser_capture::PATH))
        .eval()
        .unwrap();
    // Capture precedes every derived history mutation. No synthetic dictionaries
    // or prepopulated successful cache entries enter this initial artifact.
    let capture = parser_capture::capture(
        lua,
        primitives,
        &scan,
        &parser,
        &probes,
        &dictionaries,
        true,
        BTreeMap::new(),
    );
    let owner = capture.observed.owner();
    let environment = owner
        .roots()
        .iter()
        .find(|r| r.name == "Environment")
        .unwrap();
    let SourceValue::Callback(copy_callback) =
        owner.table(environment.table).unwrap().fields["copyTable"]
    else {
        panic!("actual immutable copyTable binding");
    };
    let copy: Function = lua.globals().raw_get("copyTable").unwrap();
    let declaration = serde_json::to_value(&owner.callback(copy_callback).unwrap().kind).unwrap();
    assert_eq!(declaration["source"]["path"], "src/Modules/Common.lua");
    assert_eq!(
        declaration["source"]["line"].as_u64(),
        copy.info().line_defined.map(|v| v as u64)
    );
    assert_eq!(
        declaration["source"]["end_line"].as_u64(),
        copy.info().last_line_defined.map(|v| v as u64)
    );
    let identity = json!({
        "source":owner.source(), "definitions_sha256":json_hash(&owner.definitions()),
        "context_sha256":json_hash(&owner.context()), "classes_sha256":json_hash(&owner.classes()),
        "prototypes_sha256":json_hash(&owner.closure_prototypes()),
        "programs_sha256":json_hash(capture.lowered.catalog().data()),
        "constructors_sha256":json_hash(&capture.lowered.catalog().constructors()),
        "creations_sha256":json_hash(&capture.lowered.catalog().closure_creations()),
        "lowerer_sha256":capture.lowered.implementation_sha256(),
        "copy_callback":copy_callback,"copy_declaration":declaration,
        "input_tables":capture.observed.input().state.tables.len(),
        "input_entries":capture.observed.input().state.tables.iter().map(|t|t.entries.len()).sum::<usize>(),
        "input_cells":capture.observed.input().cells.len(),"input_closures":capture.observed.input().closures.len(),
        "injected_dictionary_fixtures":false,"complete_state_serialization":false,
    });
    let history = history::record(lua, &parser, &probes, &cache);
    assert_eq!(
        primitives.captured_value(&parser, "cache"),
        Value::Table(cache.clone())
    );
    assert_eq!(modlib.raw_get::<Table>("parseModCache").unwrap(), cache);
    assert_eq!(
        lua.globals().raw_get::<Function>("copyTable").unwrap(),
        copy
    );
    Corpus {
        capture,
        history,
        copy_callback,
        identity,
    }
}

fn child(project: &Path, destination: &Path, build: &str) {
    fn owned_only<T: Send + Sync>() {}
    owned_only::<Corpus>(); // Supplement the explicit source-host destruction witness.
    let xml = fs::read_to_string(project.join(format!(
        "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
    )))
    .unwrap();
    let host_dropped = Arc::new(AtomicBool::new(false));
    let (corpus, source_report) = {
        let primitives = RefCell::new(None);
        let retained = RefCell::new(None);
        let scratch = tempfile::tempdir().unwrap();
        let source_report = source::observe_with_hooks(
            &project.join("vendor/path-of-building-poe2"), scratch.path(), &xml, None, false,
            Some(&|lua| {
                lua.globals().raw_set("_a1_native_lifecycle_host_drop",
                    lua.create_userdata(HostDrop(host_dropped.clone()))?)?;
                primitives.replace(Some(Primitives::before_source_with_closures(lua)?));
                Ok(())
            }),
            Some(&|lua| {
                let corpus = acquire(lua, primitives.borrow().as_ref().unwrap());
                let report = json!({"inventory":corpus.capture.inventory,
                    "identity":corpus.identity,"successful_public_calls":corpus.history.successful_public_calls});
                retained.replace(Some(corpus));
                assert!(!host_dropped.load(Ordering::SeqCst));
                let marker: mlua::AnyUserData = lua.globals()
                    .raw_get("_a1_native_lifecycle_host_drop")?;
                assert!(Arc::ptr_eq(&marker.borrow::<HostDrop>()?.0, &host_dropped));
                Ok(report)
            }),
        ).unwrap();
        drop(primitives.into_inner());
        (retained.into_inner().unwrap(), source_report)
    };
    assert!(
        host_dropped.load(Ordering::SeqCst),
        "all source host handles must drop before native timing"
    );
    // Native acquisition is cold only relative to this first compiled session;
    // source observation/lowering already affected the allocator and OS caches.
    let native = native::run(corpus);
    let engine_sources = poe_optimizer_engine::modifier_parser::implementation_sources()
        .map(|text| text.replace("\r\n", "\n"));
    let report = json!({"schema_version":1,"status":"passed","build":build,
        "xml_sha256":hash(xml.as_bytes()),"source":source_report,"native":native,
        "host_destroyed_before_native_timing":true,"marker_live_at_end_of_source_hook":true,
        "binary_links_lua":true,"source_history_mode":"interpreter; jit.off and jit.flush before capture/history",
        "engine_sources_sha256":json_hash(&engine_sources),
        "engine_hash_encoding":"JSON ordered implementation_sources with CRLF normalized to LF",
        "harness_sha256":hash(include_bytes!("profile_source_sessions.rs")),
        "capture_helper_sha256":hash(include_bytes!("support/source_program_parser_capture.rs")),
        "history_helper_sha256":hash(include_bytes!("support/profile_source_session_history.rs")),
        "native_helper_sha256":hash(include_bytes!("support/profile_source_session_native.rs")),
        "os":std::env::consts::OS,"arch":std::env::consts::ARCH,"debug_assertions":cfg!(debug_assertions)});
    fs::write(
        destination.join(format!("build-{build}.json")),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "developer-only elapsed lifecycle measurement; run explicitly in release mode"]
fn profile_original_source_session_lifecycles() {
    if cfg!(debug_assertions) {
        panic!("release mode required for the measurement protocol");
    }
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| project.join("runs/a1-source-session-lifecycle"));
    if let Ok(build) = std::env::var(CHILD) {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        child(&project, &destination, &build);
        return;
    }
    fs::create_dir_all(project.join("runs")).unwrap();
    assert_eq!(
        destination.parent().unwrap().canonicalize().unwrap(),
        project.join("runs").canonicalize().unwrap(),
        "choose a fresh direct child of runs for retained artifacts"
    );
    fs::create_dir(&destination).expect("fresh output directory; preserve earlier measurements");
    let destination = destination.canonicalize().unwrap();
    for build in ["01", "02", "03", "04", "05"] {
        let log = fs::File::create(destination.join(format!("build-{build}.log"))).unwrap();
        let mut process = Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                TEST,
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, build)
            .env(OUTPUT, &destination)
            .current_dir(project.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = process.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "source-session build {build}; inspect {destination:?}"
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(300) {
                process.kill().unwrap();
                process.wait().unwrap();
                panic!("source-session child timeout {build}; inspect {destination:?}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
