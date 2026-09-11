//! Continuing actual configuration callback sequences with inherited class dispatch.
//! This oracle does not execute the enclosing BuildModList loop or parser service.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/source_configuration_dispatch_capture.rs"]
mod capture;
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_round.rs"]
mod rounding;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "support/source_program_warm.rs"]
mod warm;
use classes::Primitives;
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
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
struct Pass {
    captured: capture::Captured,
    session: ProgramSession,
    values: Vec<SessionValue>,
    player: Table,
    enemy: Table,
    build: Table,
    rows: Vec<Json>,
    frontier: Option<Json>,
}
impl Pass {
    fn root(&self, name: &str) -> SessionValue {
        self.values[self.captured.observed.root_index(name).unwrap()].clone()
    }
    fn predict(&mut self, index: usize, value: &Value) -> Result<Json, ProgramRuntimeError> {
        let mut args = self
            .session
            .borrow(&observation::capture(std::slice::from_ref(value)))?;
        args.extend([self.root("player"), self.root("enemy"), self.root("build")]);
        let returns = self
            .session
            .invoke_callable(&self.root(&format!("apply.{index}")), &args)?;
        assert!(
            returns.is_empty(),
            "original config applies return no values"
        );
        self.native_state()
    }
    fn native_state(&mut self) -> Result<Json, ProgramRuntimeError> {
        let state = self.session.invoke_callable(
            &self.root("probe.state"),
            &[
                self.root("player"),
                self.root("enemy"),
                self.root("build"),
                self.root("names"),
            ],
        )?;
        Ok(observation::canonical(
            self.session.snapshot(&state)?.graph(),
        ))
    }
    fn actual(&self) -> Json {
        let actual: MultiValue = self
            .captured
            .probe
            .call((
                self.player.clone(),
                self.enemy.clone(),
                self.build.clone(),
                self.captured.names.clone(),
            ))
            .unwrap();
        observation::canonical(&observation::capture(&actual.into_vec()))
    }
}
fn install(
    lua: &Lua,
    primitives: Rc<RefCell<Option<Primitives>>>,
    passes: Rc<RefCell<Vec<Pass>>>,
) -> Result<(), RuntimeError> {
    primitives.replace(Some(Primitives::before_source(lua)?));
    let observer = lua.create_function(move |lua, (phase, index, var, original, value, player, enemy, build, event): (String, usize, String, Function, Value, Table, Table, Table, usize)| {
        assert!(matches!(value, Value::Nil | Value::Boolean(_) | Value::Integer(_) | Value::Number(_) | Value::String(_)), "identity-bearing apply arguments require coherent input binding");
        let mut passes = passes.borrow_mut();
        if phase == "enter" {
            let changed = passes.last().is_none_or(|pass| pass.player.to_pointer() != player.to_pointer());
            if changed {
                let captured = capture::capture(lua, primitives.borrow().as_ref().unwrap(), &player, &enemy, &build);
                let (session, values) = captured.compiled.session_from_input(captured.observed.input(), ProgramLimits { max_steps: 5_000_000, max_values: 500_000, max_bytes: 32*1024*1024, ..ProgramLimits::default() }).unwrap();
                passes.push(Pass {captured, session, values, player:player.clone(), enemy:enemy.clone(), build:build.clone(), rows:Vec::new(), frontier:None});
            }
            let pass = passes.last_mut().unwrap();
            assert_eq!(pass.enemy.to_pointer(), enemy.to_pointer());
            assert_eq!(pass.build.to_pointer(), build.to_pointer());
            assert_eq!(pass.captured.functions[&index].to_pointer(), original.to_pointer(), "exact captured callback identity");
            let mut row = json!({"event":event,"index":index,"var":var,"source_first_line":original.info().line_defined,"source_last_line":original.info().last_line_defined,"paired":false,"exited":false});
            if pass.frontier.is_none() {
                assert_eq!(pass.native_state().unwrap(), pass.actual(), "continuing source entry {var}, event {event}");
                row["entry_state_compared"] = json!(true);
                match pass.predict(index, &value) {
                    Ok(state) => { row["predicted"] = state; row["paired"] = json!(true); }
                    Err(error) => {
                        assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability, "review new source frontier {var}: {error}");
                        assert_eq!(var, "presetBossSkills", "review new source frontier: {error}");
                        assert_eq!(error.message, "session closure has no compiled program");
                        let input = pass.captured.observed.input();
                        let ProgramValue::Closure(closure) = input.state.values[pass.captured.observed.root_index(&format!("apply.{index}")).unwrap()] else { panic!("actual apply must be a session closure") };
                        let callback = input.closures[closure.0 as usize - 1].prototype.definition().callback;
                        let lowering_reason = pass.captured.unsupported[callback.0.to_string()].clone();
                        assert_eq!(lowering_reason, "generic-for helper iterator is unsupported");
                        assert_eq!(pass.native_state().unwrap(), pass.actual(), "uncompiled body must leave source-visible entry state unchanged");
                        let frontier = json!({"event":event,"index":index,"var":var,"reason":error.to_string(),"callback":callback,"lowering_reason":lowering_reason,"argument":observation::canonical(&observation::capture(std::slice::from_ref(&value))),"entry_state_unchanged":true});
                        row["frontier"] = frontier.clone();
                        pass.frontier = Some(frontier);
                    }
                }
            }
            pass.rows.push(row);
        } else {
            assert_eq!(phase, "exit");
            let pass = passes.last_mut().unwrap();
            assert_eq!(pass.captured.functions[&index].to_pointer(), original.to_pointer(), "exact callback exit identity");
            let actual = (pass.rows.last().unwrap()["paired"] == true).then(|| pass.actual());
            let row = pass.rows.last_mut().unwrap();
            assert_eq!(row["event"], event);
            assert_eq!(row["index"], index);
            assert_eq!(row["var"], var);
            assert_eq!(row["exited"], false);
            if let Some(actual) = actual {
                assert_eq!(row["predicted"], actual, "actual continuing callback {var}, event {event}");
                row["state"] = actual;
                row.as_object_mut().unwrap().remove("predicted");
            }
            row["exited"] = json!(true);
        }
        Ok(())
    })?;
    lua.globals()
        .set("_configuration_source_apply_observer", observer)?;
    Ok(())
}
fn summarize(
    lua: &Lua,
    passes: &Rc<RefCell<Vec<Pass>>>,
    build: &str,
) -> Result<Json, RuntimeError> {
    let trace: Json = lua.from_value(lua.globals().get("_configuration_source_trace")?)?;
    let events = trace["events"].as_array().unwrap();
    let passes = passes.borrow();
    assert_eq!(passes.len(), 2, "initial and saved callback sequences");
    let mut reports = Vec::new();
    let mut observed_events = Vec::new();
    for (index, pass) in passes.iter().enumerate() {
        for row in &pass.rows {
            assert_eq!(row["exited"], true);
            let event = row["event"].as_u64().unwrap() as usize;
            let actual = &events[event - 1];
            assert_eq!(actual["kind"], "enter");
            assert_eq!(actual["name"], "apply");
            assert_eq!(actual["details"]["var"], row["var"]);
            assert_eq!(actual["details"]["index"], row["index"]);
            observed_events.push(event);
        }
        let paired = pass
            .rows
            .iter()
            .take_while(|row| row["paired"] == true)
            .collect::<Vec<_>>();
        let expected_saved = match build {
            "01" | "02" => 52,
            "03" => 46,
            "04" | "05" => 45,
            _ => unreachable!(),
        };
        assert_eq!(
            paired.len(),
            if index == 0 { 44 } else { expected_saved },
            "review continuing coverage for build {build}, pass {index}: {:?}",
            pass.frontier
        );
        assert_eq!(pass.frontier.as_ref().unwrap()["var"], "presetBossSkills");
        assert!(paired.iter().any(|row| row["var"] == "enemyIsBoss"));
        assert!(
            paired.iter().any(|row| row["var"] == "enemySizePreset"),
            "size preset remains blocked: {:?}",
            pass.frontier
        );
        reports.push(json!({"phase":if index==0 {"initial"} else {"saved"},"paired_prefix_count":paired.len(),"reached_count":pass.rows.len(),"calls":pass.rows,"first_frontier":pass.frontier,"class_bindings":pass.captured.observed.input().class_bindings.len(),"session_closures":pass.captured.observed.input().closures.len(),"capture_cells":pass.captured.observed.input().cells.len(),"unsupported_bodies":pass.captured.unsupported}));
    }
    let actual_events = events
        .iter()
        .enumerate()
        .filter(|(_, e)| e["kind"] == "enter" && e["name"] == "apply")
        .map(|(i, _)| i + 1)
        .collect::<Vec<_>>();
    assert_eq!(
        observed_events, actual_events,
        "every actual apply entry has one matching exit record"
    );
    let round_parity = rounding::compare(
        lua,
        &passes[0].captured.compiled,
        &passes[0].captured.original_round,
        passes[0].captured.round_id,
    );
    Ok(
        json!({"round_parity":round_parity,"passes":reports,"scope":"Original callback bodies, actual continuing state and inherited methods, compared at each actual callback exit. The enclosing activation loop, parser services, constructors and full build evaluation are not admitted.","native_complete_builds":0,"whole_activation_admission":false}),
    )
}
#[test]
fn original_configuration_callbacks_continue_through_inherited_control_dispatch() {
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = project.join("runs/r2i-configuration-dispatch");
    fs::create_dir_all(&destination).unwrap();
    if let Ok(build) = std::env::var("POE_CONFIG_DISPATCH_CHILD") {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        let xml = fs::read_to_string(project.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
        )))
        .unwrap();
        let primitives = Rc::new(RefCell::new(None));
        let passes = Rc::new(RefCell::new(Vec::new()));
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe_with_hooks(
            &project.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            None,
            false,
            Some(&|lua| install(lua, primitives.clone(), passes.clone())),
            Some(&|lua| summarize(lua, &passes, &build)),
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
                "original_configuration_callbacks_continue_through_inherited_control_dispatch",
                "--nocapture",
            ])
            .env("POE_CONFIG_DISPATCH_CHILD", build)
            .current_dir(project.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "build {build} failed; see dispatch log");
                break;
            }
            if start.elapsed() > Duration::from_secs(180) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("dispatch child timeout {build}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
