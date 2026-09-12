//! Supported unpack source behavior and explicit conversion/layout frontiers.
#[path = "support/source_program_observation.rs"]
mod observation;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_unpack.lua";
const TEXT: &str = include_str!("support/source_program_unpack.lua");
struct Fixture {
    lua: Lua,
    functions: Table,
    compiled: CompiledSourcePrograms,
    callbacks: BTreeMap<String, SourceCallbackId>,
}
impl Fixture {
    fn new(rebind: bool) -> Self {
        let lua = Lua::new();
        lua.load("jit.off(); jit.flush(); assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let functions: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let roots = functions
            .clone()
            .pairs::<String, Function>()
            .map(Result::unwrap)
            .collect();
        let sources = BTreeMap::from([(PATH.into(), TEXT.into())]);
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: [(
                PATH.into(),
                format!("{:x}", Sha256::digest(TEXT.as_bytes())),
            )]
            .into(),
            construction_spans: Default::default(),
            module_order: vec![PATH.into()],
        };
        let context = if rebind {
            lua.globals()
                .raw_set(
                    "unpack",
                    functions.raw_get::<Function>("replacement").unwrap(),
                )
                .unwrap();
            assert!(
                observer
                    .observe_with_context(
                        &lua,
                        &sources,
                        source.clone(),
                        &roots,
                        SourceCaptureContext::default()
                    )
                    .is_err(),
                "no-environment capture must reject rebinding original unpack"
            );
            SourceCaptureContext {
                environment: Some(SourceEnvironmentSelection {
                    table: lua.globals(),
                    root_name: "environment".into(),
                }),
                projections: vec![SourceTableSelection {
                    table: lua.globals(),
                    fields: ["unpack".into()].into(),
                    indexed: Default::default(),
                    allow_index_fallback: false,
                    allow_call_fallback: false,
                }],
                ..SourceCaptureContext::default()
            }
        } else {
            SourceCaptureContext::default()
        };
        let observed = observer
            .observe_with_context(&lua, &sources, source, &roots, context)
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        Self {
            lua,
            functions,
            compiled: CompiledSourcePrograms::new(lowered.catalog()).unwrap(),
            callbacks: observed.callbacks().clone(),
        }
    }
    fn bytes(&self, value: &[u8]) -> Value {
        Value::String(self.lua.create_string(value).unwrap())
    }
    fn table(&self) -> Table {
        let t = self.lua.create_table().unwrap();
        for (key, value) in [
            (-2147483648, self.bytes(b"minimum")),
            (2147483647, self.bytes(b"maximum")),
            (-1, self.bytes(b"negative")),
            (0, self.bytes(b"zero")),
            (1, Value::Boolean(false)),
            (2, self.bytes(b"two")),
            (3, Value::Integer(0)),
        ] {
            t.raw_set(key, value).unwrap();
        }
        t
    }
    fn compare(&self, name: &str, args: &[Value]) -> Vec<Value> {
        let actual = self
            .functions
            .raw_get::<Function>(name)
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()));
        let native = self.compiled.execute(
            self.callbacks[name],
            &observation::capture(args),
            ProgramLimits::default(),
        );
        match (actual, native) {
            (Ok(actual), Ok(native)) => {
                assert_eq!(
                    observation::canonical(native.graph()),
                    observation::canonical(&observation::capture(&actual.clone().into_vec())),
                    "{name} {args:?}"
                );
                actual.into_vec()
            }
            (Err(_), Err(native)) => {
                assert_eq!(
                    native.kind,
                    ProgramRuntimeErrorKind::Source,
                    "source error, never an unsupported substitute: {name} {args:?}: {native}"
                );
                vec![]
            }
            (actual, native) => panic!("{name} {args:?}: source={actual:?}, native={native:?}"),
        }
    }
}
#[test]
fn original_unpack_ranges_conversions_packs_and_rebound_identity() {
    let f = Fixture::new(false);
    let t = Value::Table(f.table());
    let bounds = vec![
        vec![Value::Integer(-1), Value::Integer(3)],
        vec![Value::Integer(2), Value::Integer(1)],
        vec![Value::Number(-1.9), Value::Number(2.9)],
        vec![f.bytes(b" 0x1 "), f.bytes(b"3e0")],
        vec![f.bytes(b"0b10"), f.bytes(b"2")],
        vec![Value::Nil, Value::Integer(3)],
        vec![
            Value::Integer(i64::from(i32::MIN)),
            Value::Integer(i64::from(i32::MIN)),
        ],
        vec![
            Value::Integer(i64::from(i32::MAX)),
            Value::Integer(i64::from(i32::MAX)),
        ],
        vec![Value::Integer(5), Value::Integer(5)],
    ];
    for bounds in bounds {
        let mut args = vec![t.clone()];
        args.extend(bounds);
        f.compare("captured", &args);
    }
    for args in [
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Integer(0)],
        vec![t.clone(), f.bytes(b"not a number"), Value::Integer(0)],
        vec![t.clone(), Value::Integer(0), Value::Boolean(false)],
    ] {
        f.compare("captured", &args);
    }
    let dense = f.lua.create_table().unwrap();
    dense.raw_set(1, false).unwrap();
    dense.raw_set(2, f.bytes(b"a\0\xff")).unwrap();
    for trailing in [
        vec![],
        vec![Value::Nil],
        vec![Value::Nil, Value::Nil],
        vec![Value::Integer(2)],
    ] {
        let mut args = vec![Value::Table(dense.clone())];
        args.extend(trailing);
        f.compare("captured", &args);
    }
    let empty = f.lua.create_table().unwrap();
    assert!(f.compare("global", &[Value::Table(empty)]).is_empty());
    // Source return identity is tested without serializing a function into a fake callback.
    let original = f
        .functions
        .raw_get::<Function>("identity")
        .unwrap()
        .call::<MultiValue>(())
        .unwrap();
    let native = f
        .compiled
        .execute(
            f.callbacks["identity"],
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(original[0], Value::Boolean(true));
    assert_eq!(original[1], Value::Boolean(true));
    assert_eq!(native.graph().values[0], ProgramValue::Boolean(true));
    assert_eq!(native.graph().values[1], ProgramValue::Boolean(true));
    assert_eq!(
        native.graph().values[2],
        ProgramValue::Bytes(b"function".to_vec())
    );
    assert_eq!(native.graph().values[4], native.graph().values[5]);
    let f = Fixture::new(true);
    let dense = f.lua.create_table().unwrap();
    dense.raw_set(1, "original").unwrap();
    assert_eq!(
        f.compare("captured", &[Value::Table(dense.clone())]),
        vec![f.bytes(b"original")]
    );
    assert_eq!(
        f.compare("global", &[Value::Table(dense)]),
        vec![f.bytes(b"rebound")]
    );
}
#[test]
fn extra_argument_effects_and_source_stack_limit_remain_visible() {
    let f = Fixture::new(false);
    for name in ["extra", "empty"] {
        let list = f.lua.create_table().unwrap();
        list.raw_set(1, "first").unwrap();
        list.raw_set(2, false).unwrap();
        let side = f.lua.create_table().unwrap();
        side.raw_set("count", 0).unwrap();
        let roots = [Value::Table(list), Value::Table(side.clone())];
        let (mut session, args) = f
            .compiled
            .session(&observation::capture(&roots), ProgramLimits::default())
            .unwrap();
        let actual = f
            .functions
            .raw_get::<Function>(name)
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(roots.to_vec()))
            .unwrap();
        let native = session.invoke(f.callbacks[name], &args).unwrap();
        assert_eq!(
            observation::canonical(session.snapshot(&native).unwrap().graph()),
            observation::canonical(&observation::capture(&actual.into_vec()))
        );
        let native = session.invoke(f.callbacks["state"], &args[1..]).unwrap();
        assert_eq!(
            session.snapshot(&native).unwrap().graph().values,
            vec![ProgramValue::Number(1.0)]
        );
        assert_eq!(side.raw_get::<i64>("count").unwrap(), 1);
    }
    // Three supplied C arguments leave room for at most7997 result slots.
    for count in [7997, 7998, 8000] {
        let args = [
            Value::Table(f.lua.create_table().unwrap()),
            Value::Integer(1),
            Value::Integer(count),
        ];
        let actual = f
            .functions
            .raw_get::<Function>("captured")
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()));
        let native = f.compiled.execute(
            f.callbacks["captured"],
            &observation::capture(&args),
            ProgramLimits {
                max_results: 9000,
                ..ProgramLimits::default()
            },
        );
        if count == 7997 {
            assert_eq!(actual.unwrap().len(), count as usize);
            assert_eq!(native.unwrap().graph().values.len(), count as usize);
        } else {
            assert!(actual.is_err());
            assert_eq!(native.unwrap_err().kind, ProgramRuntimeErrorKind::Source);
        }
    }
}
#[test]
fn target_dependent_conversions_and_unproved_sparse_length_are_explicit_frontiers() {
    let f = Fixture::new(false);
    let table = Value::Table(f.table());
    let mut records = vec![];
    for value in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        2147483648.0,
        -2147483649.0,
    ] {
        let args = [table.clone(), Value::Number(value), Value::Number(value)];
        let actual = f
            .functions
            .raw_get::<Function>("captured")
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()))
            .unwrap();
        let native = f
            .compiled
            .execute(
                f.callbacks["captured"],
                &observation::capture(&args),
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_eq!(native.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
        assert!(
            native
                .message
                .contains("host-dependent integer argument conversion")
        );
        #[cfg(target_arch = "x86_64")]
        assert_eq!(actual.clone().into_vec(), vec![f.bytes(b"minimum")]);
        records.push(serde_json::json!({"index_bits":format!("{:016x}",value.to_bits()),"source":observation::canonical(&observation::capture(&actual.into_vec())),"native":"unsupported portable integer conversion"}));
    }
    let sparse = f.lua.create_table().unwrap();
    sparse.raw_set(2, "second").unwrap();
    let args = [Value::Table(sparse.clone())];
    assert!(
        f.functions
            .raw_get::<Function>("captured")
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(args.to_vec()))
            .is_ok()
    );
    assert_eq!(
        f.compiled
            .execute(
                f.callbacks["captured"],
                &observation::capture(&args),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        f.compare(
            "captured",
            &[Value::Table(sparse), Value::Integer(1), Value::Integer(2)]
        ),
        vec![Value::Nil, f.bytes(b"second")]
    );
    let destination = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../runs/r2l-unpack-conversion-observations.json");
    std::fs::write(destination, serde_json::to_vec_pretty(&records).unwrap()).unwrap();
}

#[test]
fn raw_reads_ignore_actual_index_behavior_and_argument_effects_precede_errors() {
    let f = Fixture::new(false);
    let table: Table = f
        .lua
        .load("return setmetatable({false}, {__index=function() error('unpack must be raw') end})")
        .eval()
        .unwrap();
    let graph = observation::capture(&[Value::Table(table.clone())]);
    let coverage = ProgramTableCoverage::from([(
        ProgramTableId(1),
        SourceTableCoverage {
            inventory: SourceTableInventory::Complete,
            known_absent: Default::default(),
            unavailable: Default::default(),
            index_fallback: SourceTableIndexFallback::Unavailable,
            call_fallback: SourceTableCallFallback::NonCallable,
        },
    )]);
    let (mut session, roots) = f
        .compiled
        .session_with_coverage(&graph, &coverage, ProgramLimits::default())
        .unwrap();
    let mut args = roots;
    args.extend(
        session
            .borrow(&observation::capture(&[
                Value::Integer(0),
                Value::Integer(2),
            ]))
            .unwrap(),
    );
    let result = session.invoke(f.callbacks["captured"], &args).unwrap();
    let actual = f
        .functions
        .raw_get::<Function>("captured")
        .unwrap()
        .call::<MultiValue>((table, 0, 2))
        .unwrap();
    assert_eq!(
        observation::canonical(session.snapshot(&result).unwrap().graph()),
        observation::canonical(&observation::capture(&actual.into_vec()))
    );

    let side = f.lua.create_table().unwrap();
    side.raw_set("count", 0).unwrap();
    let values = [Value::Boolean(false), Value::Table(side.clone())];
    let (mut session, args) = f
        .compiled
        .session(&observation::capture(&values), ProgramLimits::default())
        .unwrap();
    assert!(
        f.functions
            .raw_get::<Function>("extra")
            .unwrap()
            .call::<MultiValue>(MultiValue::from_vec(values.to_vec()))
            .is_err()
    );
    assert_eq!(
        session
            .invoke(f.callbacks["extra"], &args)
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let native = session.invoke(f.callbacks["state"], &args[1..]).unwrap();
    assert_eq!(
        session.snapshot(&native).unwrap().graph().values,
        vec![ProgramValue::Number(1.0)]
    );
    assert_eq!(side.raw_get::<i64>("count").unwrap(), 1);
}
