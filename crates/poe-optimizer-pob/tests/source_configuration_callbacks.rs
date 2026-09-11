//! Breadth inventory from all five unchanged originals. Compilable callback
//! bodies and paired prefixes are not complete configuration/build admission.
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
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::{
    runtime::RuntimeError,
    source_programs::{
        capture::{SourceClassCaptureRequest, SourceClassSelection},
        lower_from_sources,
    },
};
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const PROBE: &str = "local function rows(list) local out = {} for i, mod in ipairs(list) do out[i] = mod end return out end return rows";
#[test]
fn all_original_configuration_callbacks_are_inventoried_and_reached_prefixes_match() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/r2e-configuration-callbacks");
    fs::create_dir_all(&destination).unwrap();
    if let Ok(build) = std::env::var("POE_CONFIG_CALLBACK_CHILD") {
        assert!(["01", "02", "03", "04", "05"].contains(&build.as_str()));
        let xml = fs::read_to_string(root.join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{build}.xml"
        )))
        .unwrap();
        let primitives = RefCell::new(None);
        let result = source::observe_with_hooks(
            &root.join("vendor/path-of-building-poe2"),
            tempfile::tempdir().unwrap().path(),
            &xml,
            None,
            false,
            Some(&|lua| {
                primitives.replace(Some(Primitives::before_source(lua)?));
                Ok(())
            }),
            Some(&|lua| observe(lua, primitives.borrow().as_ref().unwrap())),
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
                "all_original_configuration_callbacks_are_inventoried_and_reached_prefixes_match",
                "--nocapture",
            ])
            .env("POE_CONFIG_CALLBACK_CHILD", build)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let start = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(
                    status.success(),
                    "build {build} failed; see its callback log"
                );
                break;
            }
            if start.elapsed() > Duration::from_secs(120) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("callback child timeout {build}");
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}
fn observe(lua: &Lua, primitives: &Primitives) -> Result<Json, RuntimeError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let mut texts = BTreeMap::new();
    for path in [
        "src/Modules/Common.lua",
        "src/Data/Global.lua",
        "src/Modules/ModTools.lua",
        "src/Classes/ModStore.lua",
        "src/Classes/ModList.lua",
        "src/Modules/ConfigOptions.lua",
    ] {
        texts.insert(
            path.into(),
            poe_optimizer_pob::source::read_verified_text(&root, path).unwrap(),
        );
    }
    let probe: Function = lua
        .load(PROBE)
        .set_name("@tests/configuration_callback_rows.lua")
        .eval()?;
    texts.insert("tests/configuration_callback_rows.lua".into(), PROBE.into());
    let registry: Table = lua.globals().get::<Table>("common")?.get("classes")?;
    let _: Table = lua.load("return new('ModList')").eval()?;
    let options: Table = lua
        .globals()
        .get::<Table>("package")?
        .get::<Table>("loaded")?
        .get("Modules.ConfigOptions")?;
    let mut functions = BTreeMap::new();
    let mut options_meta = BTreeMap::new();
    for (i, row) in options.sequence_values::<Table>().enumerate() {
        let row = row?;
        let apply: Value = row.raw_get("apply")?;
        if let Value::Function(apply) = apply {
            assert_eq!(
                apply.info().source.as_deref(),
                Some("@configuration-source-observation.lua")
            );
            let index = i + 1;
            let key = format!("config.apply.{index}");
            functions.insert(key.clone(), primitives.unwrap(&apply, "original"));
            options_meta.insert(index, (key, row.get::<String>("var")?));
        }
    }
    assert_eq!(options_meta.len(), 537);
    functions.insert("probe.rows".into(), probe.clone());
    let (source, source_names) = classes::inventory(lua, &root, &texts);
    let observed = primitives
        .observer
        .observe_classes(
            lua,
            &texts,
            source,
            SourceClassCaptureRequest {
                classes: vec![
                    SourceClassSelection {
                        table: registry.get("ModStore")?,
                        methods: BTreeSet::from(["NewMod".into(), "ReplaceMod".into()]),
                    },
                    SourceClassSelection {
                        table: registry.get("ModList")?,
                        methods: BTreeSet::from([
                            "NewMod".into(),
                            "ReplaceMod".into(),
                            "AddMod".into(),
                            "ReplaceModInternal".into(),
                        ]),
                    },
                ],
                callbacks: functions.clone(),
                // A complete observed table, not a partial global projection.
                definition_roots: BTreeMap::from([(
                    "SkillType".into(),
                    lua.globals().get("SkillType")?,
                )]),
                allocation: primitives.unwrap(&lua.globals().get("new")?, "originalNew"),
                source_names,
            },
        )
        .unwrap();
    let extracted = lower_from_sources(&texts, observed.owner()).unwrap();
    let compiled = CompiledSourcePrograms::new(extracted.catalog()).unwrap();
    let programs = extracted
        .catalog()
        .data()
        .programs
        .iter()
        .map(|program| (program.callback, program))
        .collect::<BTreeMap<_, _>>();
    let mut inventory = Vec::new();
    for (index, (key, var)) in &options_meta {
        let id = observed.callbacks()[key];
        let program = programs.get(&id);
        inventory.push(json!({"index":index,"var":var,"callback_id":id,"body_compiled":program.is_some(),"parameters":program.map(|p|p.parameter_count),"variadic":program.map(|p|p.variadic),"unsupported":extracted.unsupported().get(&id),"descriptor":observed.owner().callback(id).unwrap()}));
    }
    let trace: Json = lua.from_value(lua.globals().get("_configuration_source_trace")?)?;
    let events = trace["events"].as_array().unwrap();
    let raw_inputs: Table = lua.globals().get("_configuration_source_apply_inputs")?;
    let mut started = false;
    let mut passes = Vec::<Vec<&Json>>::new();
    for event in events {
        if event["kind"] == "enter" && event["name"] == "ConfigTab.ConfigTab" {
            started = true;
        }
        if !started {
            continue;
        }
        if event["kind"] == "enter" && event["name"] == "ConfigTab.BuildModList" {
            passes.push(Vec::new());
        }
        if event["kind"] == "enter" && event["name"] == "apply" {
            passes.last_mut().unwrap().push(event);
        }
    }
    assert_eq!(passes.len(), 2);
    let mut reports = Vec::new();
    for calls in passes {
        let (mut session, _) = compiled
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        let class = observed
            .owner()
            .bind_class(observed.class_ids()["ModList"])
            .unwrap();
        let player = session.allocate_instance(&class).unwrap();
        session.invoke_method(&player, "ModList", &[]).unwrap();
        let enemy = session.allocate_instance(&class).unwrap();
        session.invoke_method(&enemy, "ModList", &[]).unwrap();
        let source_player: Table = lua.load("return new('ModList'):ModList()").eval()?;
        let source_enemy: Table = lua.load("return new('ModList'):ModList()").eval()?;
        let mut paired = Vec::new();
        let mut frontier = Json::Null;
        let mut reached = Vec::new();
        for event in &calls {
            let index = event["details"]["index"].as_u64().unwrap() as usize;
            let (key, var) = &options_meta[&index];
            let id = observed.callbacks()[key];
            let program = programs.get(&id);
            reached.push(json!({"index":index,"var":var,"body_compiled":program.is_some(),"unsupported":extracted.unsupported().get(&id)}));
            if !frontier.is_null() {
                continue;
            }
            let Some(program) = program else {
                frontier =
                    json!({"index":index,"var":var,"reason":extracted.unsupported().get(&id)});
                continue;
            };
            if program.parameter_count > 3 || program.variadic {
                frontier = json!({"index":index,"var":var,"reason":"caller build/session state is not bound; whole body is retained"});
                continue;
            }
            let observation_id = event["details"]["inputObservation"].as_u64().unwrap();
            let value: Value = raw_inputs
                .raw_get::<Table>(observation_id)?
                .raw_get("value")?;
            assert!(
                matches!(
                    value,
                    Value::Nil
                        | Value::Boolean(_)
                        | Value::Integer(_)
                        | Value::Number(_)
                        | Value::String(_)
                ),
                "identity-bearing callback inputs need a lifecycle graph observation"
            );
            let mut args = session
                .borrow(&observation::capture(std::slice::from_ref(&value)))
                .unwrap();
            args.push(player.clone());
            args.push(enemy.clone());
            // These nonvariadic <=3-parameter source bodies cannot access the
            // fourth argument. No missing build fields are represented as nil.
            let original: MultiValue = functions[key].call((
                value,
                source_player.clone(),
                source_enemy.clone(),
                lua.globals().get::<Table>("build")?,
            ))?;
            let native = session.invoke(id, &args).unwrap();
            assert_eq!(
                observation::canonical(session.snapshot(&native).unwrap().graph()),
                observation::canonical(&observation::capture(&original.into_vec()))
            );
            let native_rows = session
                .invoke(
                    observed.callbacks()["probe.rows"],
                    std::slice::from_ref(&player),
                )
                .unwrap();
            let native_enemy = session
                .invoke(
                    observed.callbacks()["probe.rows"],
                    std::slice::from_ref(&enemy),
                )
                .unwrap();
            let mut native_values = native_rows;
            native_values.extend(native_enemy);
            let player_rows: Value = probe.call(source_player.clone())?;
            let enemy_rows: Value = probe.call(source_enemy.clone())?;
            let state = observation::canonical(session.snapshot(&native_values).unwrap().graph());
            assert_eq!(
                state,
                observation::canonical(&observation::capture(&[player_rows, enemy_rows])),
                "callback {var}"
            );
            let raw_observation: Table = raw_inputs.raw_get(observation_id)?;
            let rows: Table = raw_observation.raw_get("rows").unwrap_or_else(|_| {
                panic!(
                    "actual-pass rows unavailable: {:?}",
                    raw_observation.raw_get::<Value>("snapshotUnsupported")
                )
            });
            assert_eq!(
                state,
                observation::canonical(&observation::capture(&[
                    rows.raw_get(1)?,
                    rows.raw_get(2)?
                ])),
                "actual activation callback exit {var}"
            );
            paired.push(
                json!({"index":index,"var":var,"state":state,"actual_pass_exit_compared":true}),
            );
        }
        assert!(
            paired.len() >= 22,
            "no prefix paired: {frontier}; compiled {}",
            programs.len()
        );
        assert_eq!(
            frontier["var"], "enemySizePreset",
            "review the reached lifecycle frontier before changing callback coverage"
        );
        reports.push(json!({"reached_count":calls.len(),"reached":reached,"paired_prefix_count":paired.len(),"paired_prefix":paired,"first_frontier":frontier}));
    }
    Ok(
        json!({"source":observed.owner().source(),"definition_roots":observed.owner().roots(),"all_apply_count":inventory.len(),"compiled_apply_count":inventory.iter().filter(|row|row["body_compiled"]==true).count(),"inventory":inventory,"passes":reports,"program_count":programs.len(),"closure_count":observed.owner().callbacks().len(),"extractor_sha256":extracted.implementation_sha256(),"native_complete_builds":0,"scope":"Complete observed apply-closure inventory; body lowering is not executable admission. Paired contiguous callback prefixes use actual pass inputs and original constructors/callbacks, with raw scalar inputs, modifier-array/return graph comparison, and typed modifier rows captured at actual activation callback exits. Caller-dependent activation, UI notifications, parser services and complete builds remain unimplemented."}),
    )
}
