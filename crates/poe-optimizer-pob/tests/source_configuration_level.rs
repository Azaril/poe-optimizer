//! Complete original UpdateLevel with actual initial/saved lifecycle inputs.
//! This proves one source consumer, not native configuration activation.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_configuration_projection.rs"]
mod projection;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use classes::Primitives;
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_data::source_program::SourceTableKey;
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
const PROBES: &str = include_str!("support/source_configuration_level.lua");
struct Reached {
    graph: ProgramValueGraph,
    coverage: ProgramTableCoverage,
    expected: Option<Json>,
    event_index: usize,
    maximum: f64,
    config: Table,
}
fn observed_result(config: &Table) -> Json {
    let sets: Table = config.raw_get("configSets").unwrap();
    let set: Table = sets
        .raw_get(config.raw_get::<Value>("activeConfigSetId").unwrap())
        .unwrap();
    observation::canonical(&observation::capture(&[
        config.raw_get("enemyLevel").unwrap(),
        Value::Boolean(
            config.raw_get::<Table>("input").unwrap().to_pointer()
                == set.raw_get::<Table>("input").unwrap().to_pointer(),
        ),
        Value::Boolean(
            config.raw_get::<Table>("placeholder").unwrap().to_pointer()
                == set.raw_get::<Table>("placeholder").unwrap().to_pointer(),
        ),
    ]))
}
#[test]
fn original_update_level_matches_live_configuration_inputs_and_continuing_writes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/r2f-configuration-level");
    fs::create_dir_all(&destination).unwrap();
    if let Ok(build) = std::env::var("POE_CONFIG_LEVEL_CHILD") {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        let xml = fs::read_to_string(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
        )))
        .unwrap();
        let primitives = RefCell::new(None);
        let reached = Rc::new(RefCell::new(Vec::<Reached>::new()));
        let result = source::observe_with_hooks(
            &root.join("vendor/path-of-building-poe2"),
            tempfile::tempdir().unwrap().path(),
            &xml,
            None,
            false,
            Some(&|lua| {
                primitives.replace(Some(Primitives::before_source(lua)?));
                let reached = reached.clone();
                lua.globals().set(
                    "_configuration_source_level_observer",
                    lua.create_function(
                        move |lua, (phase, config, event_index): (String, Table, usize)| {
                            if phase == "enter" {
                                let (graph, coverage) = projection::capture(&config);
                                let maximum = lua
                                    .globals()
                                    .get::<Table>("data")?
                                    .get::<Table>("misc")?
                                    .get("MaxEnemyLevel")?;
                                reached.borrow_mut().push(Reached {
                                    graph,
                                    coverage,
                                    expected: None,
                                    event_index,
                                    maximum,
                                    config,
                                });
                            } else {
                                let mut reached = reached.borrow_mut();
                                let row = reached.last_mut().unwrap();
                                assert!(row.expected.is_none());
                                assert_eq!(row.event_index, event_index);
                                assert_eq!(row.config.to_pointer(), config.to_pointer());
                                row.expected = Some(observed_result(&config));
                            }
                            Ok(())
                        },
                    )?,
                )?;
                Ok(())
            }),
            Some(&|lua| {
                observe(
                    lua,
                    primitives.borrow().as_ref().unwrap(),
                    &reached.borrow(),
                )
            }),
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
                "original_update_level_matches_live_configuration_inputs_and_continuing_writes",
                "--nocapture",
            ])
            .env("POE_CONFIG_LEVEL_CHILD", build)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "build {build} failed; see level log");
                break;
            }
            if start.elapsed() > Duration::from_secs(120) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("level child timeout {build}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
fn observe(lua: &Lua, primitives: &Primitives, reached: &[Reached]) -> Result<Json, RuntimeError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::new();
    for path in ["src/Classes/ConfigTab.lua", "src/Modules/Data.lua"] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(&root, path).unwrap(),
        );
    }
    let probes: Table = lua
        .load(PROBES)
        .set_name("@tests/support/source_configuration_level.lua")
        .eval()?;
    texts.insert(
        "tests/support/source_configuration_level.lua".into(),
        PROBES.into(),
    );
    let registry: Table = lua.globals().get::<Table>("common")?.get("classes")?;
    let wrapped: Function = registry.get::<Table>("ConfigTab")?.get("UpdateLevel")?;
    let original = primitives.unwrap(&wrapped, "original");
    let mut callbacks = probes
        .clone()
        .pairs::<String, Function>()
        .map(Result::unwrap)
        .collect::<BTreeMap<_, _>>();
    callbacks.insert("UpdateLevel".into(), original.clone());
    let globals = lua.globals();
    let data: Table = globals.get("data")?;
    let misc: Table = data.get("misc")?;
    let selection = |table, keys: &[&str]| SourceTableSelection {
        table,
        fields: keys.iter().map(|key| (*key).into()).collect(),
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    };
    let (source, source_names) = classes::inventory(lua, &root, &texts);
    let observed = primitives
        .observer
        .observe_with_context(
            lua,
            &texts,
            source,
            &callbacks,
            SourceCaptureContext {
                capture_iteration: false,
                projections: vec![
                    selection(globals.clone(), &["data", "_G"]),
                    selection(data, &["misc"]),
                    selection(misc.clone(), &["MaxEnemyLevel"]),
                ],
                environment: Some(SourceEnvironmentSelection {
                    table: globals,
                    root_name: "original.environment".into(),
                }),
                source_names,
            },
        )
        .unwrap();
    let extracted = lower_from_sources(&texts, observed.owner()).unwrap();
    assert!(
        extracted.unsupported().is_empty(),
        "{:?}",
        extracted.unsupported()
    );
    let compiled = CompiledSourcePrograms::new(extracted.catalog()).unwrap();
    let ids = observed.callbacks();
    let trace: Json = lua.from_value(lua.globals().get("_configuration_source_trace")?)?;
    let events = trace["events"].as_array().unwrap();
    let loaded = events
        .iter()
        .position(|event| event["kind"] == "enter" && event["name"] == "ConfigTab.Load")
        .unwrap()
        + 1;
    assert_eq!(
        reached.len(),
        4,
        "two initial and two saved calls must be observed"
    );
    let mut stages = BTreeMap::<String, usize>::new();
    let maximum: f64 = misc.get("MaxEnemyLevel")?;
    let mut checkpoints = Vec::new();
    for (index, row) in reached.iter().enumerate() {
        assert_eq!(row.maximum.to_bits(), maximum.to_bits());
        let event = &events[row.event_index - 1];
        assert_eq!(event["kind"], "enter");
        assert_eq!(event["name"], "ConfigTab.UpdateLevel");
        let mut stack = Vec::new();
        for event in &events[..row.event_index - 1] {
            if event["kind"] == "enter" {
                stack.push(event["name"].as_str().unwrap());
            }
            if event["kind"] == "exit" {
                assert_eq!(stack.pop(), event["name"].as_str());
            }
        }
        let parent = *stack.last().unwrap();
        assert!(["apply", "ConfigTab.BuildModList"].contains(&parent));
        if parent == "apply" {
            assert_eq!(event["callback"], "enemyIsBoss");
        }
        let phase = if row.event_index < loaded {
            "initial"
        } else {
            "saved"
        };
        *stages.entry(format!("{phase}/{parent}")).or_default() += 1;
        let (mut session, state) = compiled
            .session_with_coverage(&row.graph, &row.coverage, ProgramLimits::default())
            .unwrap();
        assert!(
            session
                .invoke(ids["UpdateLevel"], &state)
                .unwrap()
                .is_empty()
        );
        let result = session.invoke(ids["result"], &state).unwrap();
        let actual = observation::canonical(session.snapshot(&result).unwrap().graph());
        assert_eq!(
            Some(&actual),
            row.expected.as_ref(),
            "actual activation call {index}"
        );
        assert!(
            session.snapshot(&state).is_err(),
            "partial state must not escape as complete"
        );
        for name in ["omitted", "inherited", "call"] {
            let error = session.invoke(ids[name], &state).unwrap_err();
            assert_eq!(
                error.kind,
                ProgramRuntimeErrorKind::UnsupportedCapability,
                "{name}: {error:?}"
            );
        }
        checkpoints.push(json!({"call":index,"event_index":row.event_index,"phase":phase,"parent":parent,"result":actual,"covered_tables":row.coverage.len(),"table_count":row.graph.tables.len()}));
    }
    assert_eq!(stages.len(), 4);
    assert!(stages.values().all(|count| *count == 1));
    // Continue on the actual constructed object after the original build pass.
    // The input values come from this object's current state, not a fixed build.
    let config = &reached.last().unwrap().config;
    let (graph, coverage) = projection::capture(config);
    let (mut session, state) = compiled
        .session_with_coverage(&graph, &coverage, ProgramLimits::default())
        .unwrap();
    let input_table: Table = config.raw_get("input")?;
    let placeholder_table: Table = config.raw_get("placeholder")?;
    let build_table: Table = config.raw_get("build")?;
    let baseline = observed_result(config);
    let saved = vec![
        Value::Table(config.clone()),
        input_table.raw_get("enemyLevel")?,
        placeholder_table.raw_get("enemyLevel")?,
        build_table.raw_get("characterLevel")?,
    ];
    let cases = vec![
        (Value::Nil, Value::Nil, Value::Integer(27)),
        (Value::Integer(44), Value::Integer(55), Value::Integer(66)),
        (Value::Integer(0), Value::Integer(55), Value::Integer(66)),
        (Value::Integer(-1), Value::Integer(-2), Value::Integer(66)),
        (Value::Integer(999), Value::Integer(55), Value::Integer(66)),
        (
            Value::Boolean(false),
            Value::Integer(55),
            Value::Integer(66),
        ),
        (
            Value::Number(f64::NAN),
            Value::Integer(55),
            Value::Integer(66),
        ),
        (Value::Number(f64::INFINITY), Value::Nil, Value::Integer(66)),
        (
            Value::Integer(44),
            Value::Boolean(true),
            Value::Boolean(true),
        ),
        (Value::Boolean(true), Value::Integer(55), Value::Integer(66)),
        (Value::Nil, Value::Boolean(true), Value::Integer(66)),
        (Value::Nil, Value::Nil, Value::Boolean(true)),
        (Value::Nil, Value::Nil, Value::Integer(71)),
    ];
    let mut continuing = Vec::new();
    for (input, placeholder, level) in cases {
        let args = vec![input, placeholder, level];
        let handles = session.borrow(&observation::capture(&args)).unwrap();
        let mut native_args = state.clone();
        native_args.extend(handles);
        session.invoke(ids["set"], &native_args).unwrap();
        let mut source_args = vec![Value::Table(config.clone())];
        source_args.extend(args);
        probes
            .get::<Function>("set")?
            .call::<()>(MultiValue::from_vec(source_args))?;
        let source_result = original.call::<MultiValue>(config.clone());
        let native_result = session.invoke(ids["UpdateLevel"], &state);
        assert_eq!(source_result.is_ok(), native_result.is_ok());
        if let Err(error) = &native_result {
            assert_eq!(
                error.kind,
                ProgramRuntimeErrorKind::Source,
                "expected source error: {error:?}"
            );
        }
        let result = session.invoke(ids["result"], &state).unwrap();
        let native_state = observation::canonical(session.snapshot(&result).unwrap().graph());
        assert_eq!(native_state, observed_result(config));
        continuing.push(json!({"success":source_result.is_ok(),"state":native_state}));
    }
    let source_cycle: bool = probes.get::<Function>("cycle")?.call(config.clone())?;
    assert!(source_cycle);
    let cycle = session.invoke(ids["cycle"], &state).unwrap();
    assert_eq!(
        session.snapshot(&cycle).unwrap().graph().values,
        vec![ProgramValue::Boolean(true)]
    );
    // A missing producer must fail only when its value is actually consumed.
    // Keep the original object's raw value available to the oracle, but omit it
    // explicitly from this native transport instead of supplying a fake nil.
    let (mut partial, mut partial_coverage) = projection::capture(config);
    let ProgramValue::Table(build_id) = partial.tables[0]
        .entries
        .iter()
        .find(|(key, _)| *key == ProgramValue::Bytes(b"build".to_vec()))
        .unwrap()
        .1
    else {
        panic!("build table")
    };
    let build_entries = &mut partial.tables[build_id.0 as usize - 1].entries;
    build_entries.retain(|(key, _)| *key != ProgramValue::Bytes(b"characterLevel".to_vec()));
    partial_coverage
        .get_mut(&build_id)
        .unwrap()
        .unavailable
        .insert(SourceTableKey::Text("characterLevel".into()));
    let (mut partial_session, partial_state) = compiled
        .session_with_coverage(&partial, &partial_coverage, ProgramLimits::default())
        .unwrap();
    let mut partial_cases = Vec::new();
    for (input, expected_success) in [(Value::Integer(44), true), (Value::Nil, false)] {
        let handles = partial_session
            .borrow(&observation::capture(&[input.clone(), Value::Nil]))
            .unwrap();
        let mut args = partial_state.clone();
        args.extend(handles);
        partial_session.invoke(ids["set_levels"], &args).unwrap();
        probes
            .get::<Function>("set_levels")?
            .call::<()>((config.clone(), input, Value::Nil))?;
        original.call::<()>(config.clone())?;
        let before = partial_session
            .invoke(ids["result"], &partial_state)
            .unwrap();
        let before = observation::canonical(partial_session.snapshot(&before).unwrap().graph());
        let native = partial_session.invoke(ids["UpdateLevel"], &partial_state);
        assert_eq!(native.is_ok(), expected_success);
        if let Err(error) = native {
            assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
            let after = partial_session
                .invoke(ids["result"], &partial_state)
                .unwrap();
            assert_eq!(
                observation::canonical(partial_session.snapshot(&after).unwrap().graph()),
                before,
                "unavailable RHS cannot publish an assignment"
            );
        } else {
            let result = partial_session
                .invoke(ids["result"], &partial_state)
                .unwrap();
            assert_eq!(
                observation::canonical(partial_session.snapshot(&result).unwrap().graph()),
                observed_result(config)
            );
        }
        partial_cases
            .push(json!({"success":expected_success,"missing_producer":"build.characterLevel"}));
    }
    probes
        .get::<Function>("set")?
        .call::<()>(MultiValue::from_vec(saved))?;
    original.call::<()>(config.clone())?;
    assert_eq!(observed_result(config), baseline);
    Ok(
        json!({"actual_calls":checkpoints,"continuing_cases":continuing,"partial_input_cases":partial_cases,"programs":extracted.catalog().data().programs.len(),"source":observed.owner().source(),"context":observed.owner().context(),"whole_build_admission":false}),
    )
}
