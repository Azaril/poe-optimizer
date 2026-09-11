//! Real unchanged PoB method and constructor consumers on the shared Rust VM.
//! Assertions cover explicit consumed class/state observations, not a complete
//! Lua class graph or complete build evaluation.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/source_program_classes.rs"]
mod class_source;
#[path = "support/source_program_observation.rs"]
mod observation;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
use class_source::{Observed, Primitives};
use mlua::{Function, Lua, MultiValue, Table, Value};
use observation::{canonical, capture};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::runtime::RuntimeError;
use serde_json::{Value as Json, json};
use std::{
    cell::RefCell,
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn original_class_modifier_methods_match_native_persistent_sessions() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let destination = root.join("runs/r2d-method-source");
    fs::create_dir_all(&destination).unwrap();
    if std::env::var_os("POE_METHOD_SOURCE_CHILD").is_some() {
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
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
            Some(&|lua| pair_original(lua, primitives.borrow().as_ref().unwrap())),
        )
        .unwrap();
        fs::write(
            destination.join("result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        return;
    }
    let log = fs::File::create(destination.join("child.log")).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "original_class_modifier_methods_match_native_persistent_sessions",
            "--nocapture",
        ])
        .env("POE_METHOD_SOURCE_CHILD", "1")
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
                "original source child failed; see {}",
                destination.join("child.log").display()
            );
            break;
        }
        if start.elapsed() > Duration::from_secs(120) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("original source child exceeded 120 seconds");
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}
struct Pair<'a> {
    lua: &'a Lua,
    observed: &'a Observed,
    probes: Table,
    call: Function,
    session: ProgramSession,
    checkpoints: Vec<Json>,
    successes: usize,
    failures: usize,
}
impl Pair<'_> {
    fn args(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session.borrow(&capture(values)).unwrap()
    }
    fn probe(&mut self, name: &str, input: &[SessionValue]) -> Vec<SessionValue> {
        self.session
            .invoke(self.observed.callbacks[&format!("probe.{name}")], input)
            .unwrap()
    }
    fn state(
        &mut self,
        label: &str,
        object: &Table,
        native: &SessionValue,
        parent: Option<(&Table, &SessionValue)>,
    ) {
        let mut input = vec![native.clone()];
        let source: MultiValue = if let Some((parent, native_parent)) = parent {
            input.push(native_parent.clone());
            self.probes
                .get::<Function>("state")
                .unwrap()
                .call((object.clone(), parent.clone()))
                .unwrap()
        } else {
            self.probes
                .get::<Function>("state")
                .unwrap()
                .call((object.clone(), false))
                .unwrap()
        };
        if parent.is_none() {
            input.extend(self.args(&[Value::Boolean(false)]));
        }
        let values = self.probe("state", &input);
        let snapshot = self.session.snapshot(&values).unwrap();
        assert!(snapshot.owner().is_same_owner(&self.observed.owner));
        let actual = canonical(snapshot.graph());
        assert_eq!(
            actual,
            canonical(&capture(&source.into_vec())),
            "checkpoint {label}"
        );
        self.checkpoints.push(json!({"label":label,"state":actual,"steps":self.session.steps(),"allocations":{"values":self.session.allocations().values,"tables":self.session.allocations().tables,"bytes":self.session.allocations().bytes}}));
    }
    fn method(
        &mut self,
        object: &Table,
        native: &SessionValue,
        name: &str,
        args: &[Value],
        failure: bool,
    ) {
        let input = self.args(args);
        self.method_values(object, native, name, args, &input, failure);
    }
    #[allow(clippy::too_many_arguments)]
    fn method_values(
        &mut self,
        object: &Table,
        native: &SessionValue,
        name: &str,
        args: &[Value],
        input: &[SessionValue],
        failure: bool,
    ) {
        let mut source_args = vec![
            Value::Table(object.clone()),
            Value::String(self.lua.create_string(name).unwrap()),
        ];
        source_args.extend_from_slice(args);
        let original = self
            .call
            .call::<MultiValue>(MultiValue::from_vec(source_args));
        let native_result = self.session.invoke_method(native, name, input);
        if failure {
            assert!(
                original.is_err(),
                "expected original source failure for {name}"
            );
            assert_eq!(
                native_result.unwrap_err().kind,
                ProgramRuntimeErrorKind::Source,
                "{name}"
            );
            self.failures += 1;
        } else {
            let original = original.unwrap();
            let values = native_result.unwrap();
            assert_eq!(
                canonical(self.session.snapshot(&values).unwrap().graph()),
                canonical(&capture(&original.into_vec())),
                "method {name}"
            );
            self.successes += 1;
        }
    }
    fn set(
        &mut self,
        object: &Table,
        native: &SessionValue,
        key: &str,
        value: Value,
        native_value: Option<ProgramValue>,
    ) {
        let source = self.probes.get::<Function>("set").unwrap();
        source
            .call::<()>((object.clone(), key, value.clone()))
            .unwrap();
        let mut inputs = vec![native.clone()];
        inputs.extend(
            self.session
                .borrow(&ProgramValueGraph {
                    values: vec![ProgramValue::Bytes(key.as_bytes().to_vec())],
                    tables: vec![],
                })
                .unwrap(),
        );
        inputs.extend(if let Some(value) = native_value {
            self.session
                .borrow(&ProgramValueGraph {
                    values: vec![value],
                    tables: vec![],
                })
                .unwrap()
        } else {
            self.args(&[value])
        });
        assert!(self.probe("set", &inputs).is_empty());
    }
}
fn pair_original(lua: &Lua, primitives: &Primitives) -> Result<Json, RuntimeError> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let probes: Table = lua
        .load(include_str!("support/source_program_methods.lua"))
        .set_name("@tests/support/source_program_methods.lua")
        .eval()?;
    let observed = class_source::observe(lua, primitives, &root, &probes);
    let extracted =
        poe_optimizer_pob::source_programs::lower_from_sources(&observed.texts, &observed.owner)
            .unwrap();
    for key in [
        "ModList.NewMod",
        "ModList.ReplaceMod",
        "ModList.AddMod",
        "ModList.ReplaceModInternal",
    ] {
        assert!(
            extracted
                .catalog()
                .data()
                .programs
                .iter()
                .any(|program| program.callback == observed.callbacks[key]),
            "complete original method missing: {key}; {:?}",
            extracted.unsupported()
        );
    }
    let compiled = CompiledSourcePrograms::new(extracted.catalog()).unwrap();
    let (session, _) = compiled
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let call = lua
        .load("return function(receiver,name,...) return receiver[name](receiver,...) end")
        .eval()?;
    let mut pair = Pair {
        lua,
        observed: &observed,
        probes,
        call,
        session,
        checkpoints: vec![],
        successes: 0,
        failures: 0,
    };
    let class = observed
        .owner
        .bind_class(observed.owner.class_id("ModList").unwrap())
        .unwrap();
    let parent: Table = lua.load("return new('ModList')").eval()?;
    let parent_native = pair.session.allocate_instance(&class).unwrap();
    pair.state("allocated parent", &parent, &parent_native, None);
    let returns: Table = parent.get::<Function>("ModList")?.call(parent.clone())?;
    assert_eq!(returns.to_pointer(), parent.to_pointer());
    let native_returns = pair
        .session
        .invoke_method(&parent_native, "ModList", &[])
        .unwrap();
    let aliases = pair.probe("alias", &[parent_native.clone(), native_returns[0].clone()]);
    assert_eq!(
        pair.session.snapshot(&aliases).unwrap().graph().values,
        vec![ProgramValue::Boolean(true)]
    );
    pair.state("constructed parent", &parent, &parent_native, None);
    let child: Table = lua.load("return new('ModList')").eval()?;
    let child_native = pair.session.allocate_instance(&class).unwrap();
    let returned: Table = child
        .get::<Function>("ModList")?
        .call((child.clone(), parent.clone()))?;
    assert_eq!(returned.to_pointer(), child.to_pointer());
    let native_returns = pair
        .session
        .invoke_method(
            &child_native,
            "ModList",
            std::slice::from_ref(&parent_native),
        )
        .unwrap();
    let aliases = pair.probe("alias", &[child_native.clone(), native_returns[0].clone()]);
    assert_eq!(
        pair.session.snapshot(&aliases).unwrap().graph().values,
        vec![ProgramValue::Boolean(true)]
    );
    pair.state(
        "constructed child with shared actor",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    let text = |s: &str| Value::String(lua.create_string(s).unwrap());
    let make = |value: f64| {
        vec![
            text("Strength"),
            text("BASE"),
            Value::Number(value),
            text("SourceClass"),
            Value::Integer(0),
            Value::Integer(0),
        ]
    };
    for value in [10.0, 20.0] {
        pair.method(&parent, &parent_native, "NewMod", &make(value), false);
    }
    pair.method(&child, &child_native, "ReplaceMod", &make(99.0), false);
    pair.state(
        "parent first-match replacement",
        &parent,
        &parent_native,
        None,
    );
    pair.state(
        "parent replacement does not append child",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    pair.method(&child, &child_native, "NewMod", &make(30.0), false);
    pair.method(&child, &child_native, "ReplaceMod", &make(40.0), false);
    let mut different = make(41.0);
    different[4] = Value::Integer(1);
    pair.method(&child, &child_native, "ReplaceMod", &different, false);
    pair.state(
        "local precedence and distinct flags",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    pair.state(
        "parent retained after local replacement",
        &parent,
        &parent_native,
        None,
    );
    // One tag object is supplied twice in one call: captures and snapshots must
    // preserve its identity, including the false and nil vararg positions.
    let tag = lua.create_table()?;
    tag.set("type", "Condition")?;
    tag.set("var", "SourceSharedTag")?;
    let argument_sets = [
        vec![],
        vec![Value::Table(tag.clone())],
        vec![
            text("Tagged"),
            Value::Boolean(false),
            Value::Nil,
            Value::Table(tag.clone()),
            Value::Table(tag.clone()),
        ],
        vec![
            text("Tagged"),
            Value::Integer(8),
            Value::Integer(16),
            Value::Table(tag.clone()),
            Value::Table(tag.clone()),
        ],
    ]
    .into_iter()
    .map(|extra| {
        let mut args = vec![text("Dexterity"), text("BASE"), Value::Integer(12)];
        args.extend(extra);
        args
    })
    .collect::<Vec<_>>();
    // Import the whole external input graph once and retain handles between
    // calls. Fresh imports intentionally cannot infer cross-import identity.
    let flattened = argument_sets.iter().flatten().cloned().collect::<Vec<_>>();
    let native_args = pair.args(&flattened);
    let mut offset = 0;
    for args in &argument_sets {
        pair.method_values(
            &child,
            &child_native,
            "NewMod",
            args,
            &native_args[offset..offset + args.len()],
            false,
        );
        offset += args.len();
    }
    pair.state(
        "createMod varargs tags and aliases",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    pair.method(
        &child,
        &child_native,
        "ReplaceModInternal",
        &[Value::Nil],
        true,
    );
    pair.method(&child, &child_native, "ModList", &[], true);
    pair.state(
        "source failures retain prior state",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    // A parent proxy searches raw object fields then the parent class; it does
    // not fall through to the child's AddMod prototype.
    let result: mlua::Result<MultiValue> =
        pair.probes
            .get::<Function>("proxy_newmod")?
            .call((child.clone(), "Proxy", "BASE", 7));
    assert!(result.is_err());
    let mut args = vec![child_native.clone()];
    args.extend(pair.args(&[text("Proxy"), text("BASE"), Value::Integer(7)]));
    assert_eq!(
        pair.session
            .invoke(observed.callbacks["probe.proxy_newmod"], &args)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    pair.failures += 1;
    let forwarded = lua.create_table()?;
    forwarded.set("label", "shared forwarding")?;
    let original: MultiValue = pair
        .probes
        .get::<Function>("proxy_write")?
        .call((child.clone(), forwarded.clone()))?;
    let mut args = vec![child_native.clone()];
    args.extend(pair.args(&[Value::Table(forwarded)]));
    let result = pair.probe("proxy_write", &args);
    assert_eq!(
        canonical(pair.session.snapshot(&result).unwrap().graph()),
        canonical(&capture(&original.into_vec()))
    );
    assert_eq!(
        child
            .raw_get::<Table>("ModStore")?
            .raw_get::<Value>("forwarded")?,
        Value::Nil
    );
    pair.state(
        "parent proxy forwards missing writes",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    pair.set(&child, &child_native, "AddMod", Value::Boolean(false), None);
    pair.method(&child, &child_native, "NewMod", &make(70.0), true);
    pair.state(
        "false raw field shadows class method",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    let replacement: Function = child.get("ReplaceModInternal")?;
    pair.set(
        &child,
        &child_native,
        "AddMod",
        Value::Function(replacement),
        Some(ProgramValue::Callback(
            observed.callbacks["ModList.ReplaceModInternal"],
        )),
    );
    pair.method(&child, &child_native, "NewMod", &make(80.0), false);
    pair.state(
        "raw callback dispatch replaces first local modifier",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    pair.set(&child, &child_native, "AddMod", Value::Nil, None);
    pair.method(&child, &child_native, "NewMod", &make(90.0), false);
    pair.state(
        "nil restores inherited method lookup",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    for _ in 0..2 {
        let source: MultiValue = pair
            .probes
            .get::<Function>("proxy_arrays")?
            .call(child.clone())?;
        let native = pair.probe("proxy_arrays", std::slice::from_ref(&child_native));
        assert_eq!(
            canonical(pair.session.snapshot(&native).unwrap().graph()),
            canonical(&capture(&source.into_vec()))
        );
    }
    pair.state(
        "proxy raw array operations retain object modifiers",
        &child,
        &child_native,
        Some((&parent, &parent_native)),
    );
    let bad: Table = lua.load("return new('ModList')").eval()?;
    let bad_native = pair.session.allocate_instance(&class).unwrap();
    pair.method(&bad, &bad_native, "ModList", &[Value::Integer(7)], true);
    pair.state(
        "bad constructor retains parent write",
        &bad,
        &bad_native,
        None,
    );
    assert_eq!(bad.raw_get::<i64>("parent")?, 7);
    let proxy: Table = bad.raw_get("ModStore")?;
    let original: mlua::Result<MultiValue> = proxy
        .get::<Function>("__call")?
        .call((proxy.clone(), false));
    assert!(original.is_err());
    let native_proxy = pair
        .probe("proxy", std::slice::from_ref(&bad_native))
        .remove(0);
    let no_self = pair.args(&[Value::Boolean(false)]);
    assert_eq!(
        pair.session
            .invoke_method(&native_proxy, "__call", &no_self)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    pair.failures += 1;
    pair.state(
        "wrong proxy self retains partial constructor state",
        &bad,
        &bad_native,
        None,
    );
    assert_eq!(
        pair.session
            .snapshot(std::slice::from_ref(&child_native))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        pair.session
            .invoke(
                observed.callbacks["probe.class_frontier"],
                std::slice::from_ref(&child_native)
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let (mut independent, _) = compiled
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        independent
            .invoke_method(&child_native, "NewMod", &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    Ok(
        json!({"source_revision":observed.owner.source().upstream_revision,"source_files":observed.owner.source().files,
        "class_descriptors":observed.owner.classes(),"closure_graph":observed.owner.callbacks(),
        "program_count":extracted.catalog().data().programs.len(),"unsupported_programs":extracted.unsupported(),
        "paired_successful_method_calls":pair.successes,"paired_source_failures":pair.failures,"checkpoints":pair.checkpoints,
        "snapshot_scope":"Plain consumed state projections and explicit class/proxy identity checks; behavior-bearing raw snapshots reject. This is not complete class graph or native build parity.",
        "native_complete_builds":0,"extractor_sha256":extracted.implementation_sha256()}),
    )
}
