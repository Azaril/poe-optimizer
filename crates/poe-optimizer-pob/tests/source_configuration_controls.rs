//! Original control notifications with live closure/cell identities. A component
//! oracle, not admission of full configuration activation or constructor lowering.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[path = "support/source_program_observation.rs"]
mod observation;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use classes::Primitives;
use mlua::{Function, Lua, LuaSerdeExt, Table, Value};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::{
    runtime::RuntimeError,
    source_programs::{capture::*, lower_from_sources},
};
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    rc::Rc,
    time::{Duration, Instant},
};
#[path = "support/source_configuration_controls_continuing.rs"]
mod continuing;
const PROBES: &str = include_str!("support/source_configuration_controls.lua");
struct Reached {
    event: usize,
    callback: String,
    var: String,
    notify: bool,
    predicted: Json,
    expected: Option<Json>,
    control: Table,
    config: Table,
    programs: usize,
    cells: usize,
    closures: usize,
}
fn raw_result(control: &Table, config: &Table, var: &str) -> Json {
    let sets: Table = config.raw_get("configSets").unwrap();
    let set: Table = sets
        .raw_get(config.raw_get::<Value>("activeConfigSetId").unwrap())
        .unwrap();
    let placeholder: Table = set.raw_get("placeholder").unwrap();
    let build: Table = config.raw_get("build").unwrap();
    observation::canonical(&observation::capture(&[
        control.raw_get("placeholder").unwrap(),
        placeholder.raw_get(var).unwrap(),
        build.raw_get("buildFlag").unwrap(),
        Value::Boolean(
            config.raw_get::<Table>("input").unwrap().to_pointer()
                == set.raw_get::<Table>("input").unwrap().to_pointer(),
        ),
        Value::Boolean(
            config.raw_get::<Table>("placeholder").unwrap().to_pointer()
                == placeholder.to_pointer(),
        ),
    ]))
}
fn describe_control(primitives: &Primitives, control: &Table) -> (Table, String) {
    let change: Function = control.raw_get("changeFunc").unwrap();
    assert_eq!(
        (change.info().line_defined, change.info().last_line_defined),
        (Some(315), Some(324)),
        "the numeric source family is the declared scope"
    );
    let Value::Table(config) = primitives.captured_value(&change, "self") else {
        panic!("actual ConfigTab capture")
    };
    let Value::Table(option) = primitives.captured_value(&change, "varData") else {
        panic!("actual option capture")
    };
    (config, option.raw_get("var").unwrap())
}
fn capture(
    lua: &Lua,
    primitives: &Primitives,
    config: &Table,
    controls: BTreeMap<String, Table>,
    extra: BTreeMap<String, Value>,
) -> ObservedSourceSession {
    capture_with_fields(
        lua,
        primitives,
        config,
        controls,
        extra,
        &["placeholder", "changeFunc"],
    )
}
fn capture_with_fields(
    lua: &Lua,
    primitives: &Primitives,
    config: &Table,
    controls: BTreeMap<String, Table>,
    extra: BTreeMap<String, Value>,
    control_fields: &[&str],
) -> ObservedSourceSession {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::new();
    for path in [
        "src/Classes/ConfigTab.lua",
        "src/Classes/EditControl.lua",
        "src/Modules/ConfigOptions.lua",
    ] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(&root, path).unwrap(),
        );
    }
    let probes: Table = lua
        .load(PROBES)
        .set_name("@tests/support/source_configuration_controls.lua")
        .eval()
        .unwrap();
    texts.insert(
        "tests/support/source_configuration_controls.lua".into(),
        PROBES.into(),
    );
    let registry: Table = lua
        .globals()
        .get::<Table>("common")
        .unwrap()
        .get("classes")
        .unwrap();
    let wrapped: Function = registry
        .get::<Table>("EditControl")
        .unwrap()
        .get("SetPlaceholder")
        .unwrap();
    let mut callbacks = probes
        .pairs::<String, Function>()
        .map(|row| {
            let (name, function) = row.unwrap();
            (format!("probe.{name}"), function)
        })
        .collect::<BTreeMap<_, _>>();
    callbacks.insert(
        "set_placeholder".into(),
        primitives.unwrap(&wrapped, "original"),
    );
    let selection = |table, names: &[&str], behavior| SourceTableSelection {
        table,
        fields: names.iter().map(|name| (*name).into()).collect(),
        indexed: BTreeSet::new(),
        allow_index_fallback: behavior,
        allow_call_fallback: behavior,
    };
    let globals = lua.globals();
    let options: Table = globals
        .get::<Table>("package")
        .unwrap()
        .get::<Table>("loaded")
        .unwrap()
        .get("Modules.ConfigOptions")
        .unwrap();
    let mut definition_roots = BTreeMap::new();
    let mut definition_projections = vec![selection(
        globals.clone(),
        &["_G", "tostring", "tonumber"],
        false,
    )];
    for (index, row) in options.sequence_values::<Table>().enumerate() {
        let row = row.unwrap();
        if let Ok(kind) = row.raw_get::<String>("type")
            && ["count", "integer", "countAllowZero", "float"].contains(&kind.as_str())
        {
            definition_roots.insert(format!("option.{}", index + 1), row.clone());
            definition_projections.push(selection(row, &["var"], false));
        }
    }
    let build: Table = config.raw_get("build").unwrap();
    let mut state_projections = vec![
        selection(
            config.clone(),
            &[
                "configSets",
                "activeConfigSetId",
                "input",
                "placeholder",
                "build",
            ],
            true,
        ),
        selection(build, &["buildFlag", "configTab"], true),
    ];
    let sets: Table = config.raw_get("configSets").unwrap();
    for (index, row) in sets.pairs::<Value, Table>().enumerate() {
        assert!(index < 10_000);
        state_projections.push(selection(row.unwrap().1, &["input", "placeholder"], false));
    }
    let mut state_roots = BTreeMap::from([("config".into(), Value::Table(config.clone()))]);
    for (name, control) in controls {
        state_projections.push(selection(control.clone(), control_fields, true));
        state_roots.insert(name, Value::Table(control));
    }
    state_roots.extend(extra);
    let (source, source_names) = classes::inventory(lua, &root, &texts);
    let observed = primitives
        .observer
        .observe_session(
            lua,
            &texts,
            source,
            SourceSessionCaptureRequest {
                callbacks,
                state_roots,
                definition_roots,
                definitions: SourceCaptureContext {
                    capture_iteration: false,
                    projections: definition_projections,
                    environment: Some(SourceEnvironmentSelection {
                        table: globals,
                        root_name: "original.environment".into(),
                    }),
                    source_names,
                },
                state_projections,
            },
        )
        .unwrap();
    // Definition data contains no ConfigTab/build snapshot. Captured state is
    // carried only in the separate per-session input graph.
    for table in observed.owner().tables() {
        assert!(
            table
                .fields
                .keys()
                .all(|key| ["var", "_G", "tostring", "tonumber"].contains(&key.as_str()))
        );
    }
    observed
}
fn compile(observed: &ObservedSourceSession) -> CompiledSourcePrograms {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::new();
    for path in [
        "src/Classes/ConfigTab.lua",
        "src/Classes/EditControl.lua",
        "src/Modules/ConfigOptions.lua",
    ] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(&root, path).unwrap(),
        );
    }
    texts.insert(
        "tests/support/source_configuration_controls.lua".into(),
        PROBES.into(),
    );
    let lowered = lower_from_sources(&texts, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    CompiledSourcePrograms::new(lowered.catalog()).unwrap()
}
fn root(observed: &ObservedSourceSession, values: &[SessionValue], name: &str) -> SessionValue {
    values[observed.root_index(name).unwrap()].clone()
}
fn install_observer(
    lua: &Lua,
    primitives: Rc<RefCell<Option<Primitives>>>,
    reached: Rc<RefCell<Vec<Reached>>>,
) -> Result<(), RuntimeError> {
    primitives.replace(Some(Primitives::before_source(lua)?));
    let observer = lua.create_function(
        move |lua,
              (phase, control, text, notify, callback, event): (
            String,
            Table,
            Value,
            Value,
            String,
            usize,
        )| {
            if phase == "enter" {
                let borrowed = primitives.borrow();
                let primitives = borrowed.as_ref().unwrap();
                let (config, var) = describe_control(primitives, &control);
                let observed = capture(
                    lua,
                    primitives,
                    &config,
                    BTreeMap::from([("receiver".into(), control.clone())]),
                    BTreeMap::from([
                        ("text".into(), text),
                        ("notify".into(), notify.clone()),
                        ("var".into(), Value::String(lua.create_string(&var)?)),
                    ]),
                );
                let compiled = compile(&observed);
                let (mut session, values) = compiled
                    .session_from_input(observed.input(), ProgramLimits::default())
                    .unwrap();
                let result = session
                    .invoke_callable(
                        &root(&observed, &values, "set_placeholder"),
                        &[
                            root(&observed, &values, "receiver"),
                            root(&observed, &values, "text"),
                            root(&observed, &values, "notify"),
                        ],
                    )
                    .unwrap();
                assert!(result.is_empty());
                let result = session
                    .invoke_callable(
                        &root(&observed, &values, "probe.result"),
                        &[
                            root(&observed, &values, "receiver"),
                            root(&observed, &values, "config"),
                            root(&observed, &values, "var"),
                        ],
                    )
                    .unwrap();
                let predicted = observation::canonical(session.snapshot(&result).unwrap().graph());
                reached.borrow_mut().push(Reached {
                    event,
                    callback,
                    var,
                    notify: !matches!(notify, Value::Nil | Value::Boolean(false)),
                    predicted,
                    expected: None,
                    control,
                    config,
                    programs: compiled.catalog().data().programs.len(),
                    cells: observed.input().cells.len(),
                    closures: observed.input().closures.len(),
                });
            } else {
                assert_eq!(phase, "exit");
                let mut reached = reached.borrow_mut();
                let row = reached.last_mut().unwrap();
                assert_eq!(row.event, event);
                assert_eq!(row.callback, callback);
                assert_eq!(row.control.to_pointer(), control.to_pointer());
                assert!(row.expected.is_none());
                let expected = raw_result(&control, &row.config, &row.var);
                assert_eq!(
                    row.predicted, expected,
                    "actual source callback {callback}/{}",
                    row.var
                );
                row.expected = Some(expected);
            }
            Ok(())
        },
    )?;
    lua.globals()
        .set("_configuration_source_placeholder_observer", observer)?;
    Ok(())
}
fn summarize(
    lua: &Lua,
    primitives: &Rc<RefCell<Option<Primitives>>>,
    reached: &Rc<RefCell<Vec<Reached>>>,
) -> Result<Json, RuntimeError> {
    let trace: Json = lua.from_value(lua.globals().get("_configuration_source_trace")?)?;
    let events = trace["events"].as_array().unwrap();
    let load = events
        .iter()
        .position(|event| event["kind"] == "enter" && event["name"] == "ConfigTab.Load")
        .unwrap()
        + 1;
    let reached = reached.borrow();
    let mut counts = BTreeMap::new();
    let mut rows = Vec::new();
    for row in reached.iter() {
        let event = &events[row.event - 1];
        assert_eq!(event["kind"], "control");
        assert_eq!(event["name"], "EditControl.SetPlaceholder");
        assert_eq!(event["callback"], row.callback);
        assert!(
            row.expected.is_some(),
            "every entry must have its matching source exit"
        );
        let expected_notify = match row.callback.as_str() {
            "enemySizePreset" => false,
            "enemyIsBoss" => true,
            _ => panic!("new control callback needs explicit oracle scope"),
        };
        assert_eq!(row.notify, expected_notify);
        assert_eq!(event["details"]["notify"], expected_notify);
        let phase = if row.event < load { "initial" } else { "saved" };
        *counts
            .entry(format!("{phase}/{}", row.callback))
            .or_insert(0usize) += 1;
        rows.push(json!({"event":row.event,"phase":phase,"callback":row.callback,"var":row.var,"notify":row.notify,"state":row.expected,"programs":row.programs,"cells":row.cells,"closures":row.closures}));
    }
    // These five immutable fixtures deliberately pin the complete current trace.
    // An upstream change must update the source contract rather than silently
    // shrinking an oracle that only checked the presence of each family.
    assert_eq!(
        counts,
        BTreeMap::from([
            ("initial/enemySizePreset".to_string(), 1usize),
            ("initial/enemyIsBoss".to_string(), 18),
            ("saved/enemySizePreset".to_string(), 1),
            ("saved/enemyIsBoss".to_string(), 18),
        ])
    );
    let actual_events = events
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event["kind"] == "control" && event["name"] == "EditControl.SetPlaceholder"
        })
        .map(|(index, _)| index + 1)
        .collect::<Vec<_>>();
    assert_eq!(actual_events.len(), 38);
    assert_eq!(
        actual_events,
        reached.iter().map(|row| row.event).collect::<Vec<_>>()
    );
    let continuing = continuing::run(lua, primitives.borrow().as_ref().unwrap(), &reached);
    Ok(
        json!({"actual_calls":rows,"stages":counts,"continuing":continuing,"whole_activation_admission":false}),
    )
}
#[test]
fn original_control_notifications_use_session_closures_for_all_five_builds() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = project.join("runs/r2g-configuration-controls");
    fs::create_dir_all(&destination).unwrap();
    if let Ok(build) = std::env::var("POE_CONFIG_CONTROLS_CHILD") {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        let xml = fs::read_to_string(project.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
        )))
        .unwrap();
        let primitives = Rc::new(RefCell::new(None::<Primitives>));
        let reached = Rc::new(RefCell::new(Vec::<Reached>::new()));
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_hooks(
            &project.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&|lua| install_observer(lua, primitives.clone(), reached.clone())),
            Some(&|lua| summarize(lua, &primitives, &reached)),
        )
        .unwrap();
        fs::write(
            destination.join(format!("build-{build}.json")),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    for build in ["01", "02", "03", "04", "05"] {
        let log = fs::File::create(destination.join(format!("build-{build}.log"))).unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "original_control_notifications_use_session_closures_for_all_five_builds",
                "--nocapture",
            ])
            .env("POE_CONFIG_CONTROLS_CHILD", build)
            .current_dir(project.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "build {build} failed; see control log");
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("control child timeout {build}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
