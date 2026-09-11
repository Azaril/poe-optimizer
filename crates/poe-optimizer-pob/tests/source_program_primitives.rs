//! Differential coverage for the primitive/call dependencies of configuration.
#[path = "support/source_program_observation.rs"]
mod observation;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::SourceClosureObserver, lower_from_sources};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const TEXT: &str = include_str!("support/source_program_primitives.lua");
fn programs(
    lua: &Lua,
) -> (
    Table,
    CompiledSourcePrograms,
    BTreeMap<String, SourceCallbackId>,
) {
    let observer = SourceClosureObserver::capture_before_source(lua).unwrap();
    let path = "tests/support/source_program_primitives.lua";
    let functions: Table = lua.load(TEXT).set_name(format!("@{path}")).eval().unwrap();
    let roots = functions
        .clone()
        .pairs::<String, Function>()
        .map(Result::unwrap)
        .collect::<BTreeMap<_, _>>();
    let sources = BTreeMap::from([(path.into(), TEXT.into())]);
    let provenance = ItemLoadingSource {
        upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
        files: BTreeMap::from([(
            path.into(),
            format!("{:x}", Sha256::digest(TEXT.as_bytes())),
        )]),
        construction_spans: BTreeMap::new(),
        module_order: vec![path.into()],
    };
    let observed = observer.observe(lua, &sources, provenance, &roots).unwrap();
    let (definitions, callbacks) = observed.into_parts();
    let owner = SourceProgramOwner::new(definitions).unwrap();
    let lowered = lower_from_sources(&sources, &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    (
        functions,
        CompiledSourcePrograms::new(lowered.catalog()).unwrap(),
        callbacks,
    )
}
fn comparable(mut graph: ProgramValueGraph) -> serde_json::Value {
    // Lua numeric equality cannot distinguish NaN payloads; retain signed zero
    // and every finite value bit-for-bit, normalizing only NaN payload bits.
    for value in graph
        .values
        .iter_mut()
        .chain(graph.tables.iter_mut().flat_map(|table| {
            table
                .entries
                .iter_mut()
                .flat_map(|(key, value)| [key, value])
        }))
    {
        if let ProgramValue::Number(value) = value
            && value.is_nan()
        {
            *value = f64::NAN;
        }
    }
    observation::canonical(&graph)
}
#[test]
fn original_luajit_primitives_match_native_scalar_and_pattern_results() {
    let lua = Lua::new();
    let (functions, compiled, callbacks) = programs(&lua);
    let bytes = |value: &[u8]| Value::String(lua.create_string(value).unwrap());
    let numeric = vec![
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Integer(4)],
        vec![bytes(b"-2.5"), Value::Number(8.0), Value::Integer(-4)],
        vec![Value::Number(0.0), Value::Number(-0.0)],
        vec![Value::Number(-0.0), Value::Number(0.0)],
        vec![Value::Number(f64::NAN), Value::Integer(2)],
        vec![Value::Integer(2), Value::Number(f64::NAN)],
        vec![
            Value::Number(f64::INFINITY),
            Value::Number(f64::NEG_INFINITY),
        ],
        vec![Value::Integer(2), bytes(b"oops")],
        vec![Value::Integer(2), Value::Nil],
    ];
    let text = vec![
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Boolean(true)],
        vec![bytes(b"a\0\xffb")],
        vec![Value::Number(-0.0)],
        vec![Value::Number(1.234567890123456)],
        vec![Value::Number(f64::NAN)],
        vec![Value::Number(f64::INFINITY)],
        vec![Value::Number(f64::NEG_INFINITY)],
        vec![Value::Integer(2), Value::Boolean(false)],
    ];
    let patterns = vec![
        vec![],
        vec![bytes(b"a"), Value::Nil],
        vec![bytes(b" a42 "), bytes(b"^%s*(.-)%s*$")],
        vec![bytes(b"a42"), bytes(b"(%a+)(%d+)")],
        vec![bytes(b"a42"), bytes(b"()%d+")],
        vec![bytes(b"none"), bytes(b"%d+")],
        vec![bytes(b"abc"), bytes(b"."), Value::Integer(-1)],
        vec![bytes(b"abc"), bytes(b"."), Value::Integer(0)],
        vec![bytes(b"abc"), bytes(b"."), Value::Integer(99)],
        vec![bytes(b""), bytes(b"()")],
        vec![bytes(b"a"), bytes(b"(")],
        vec![Value::Integer(123), bytes(b"%d+")],
    ];
    let mut count = 0;
    for (name, cases) in [
        ("minimum", numeric.clone()),
        ("maximum", numeric),
        ("text", text),
        ("match", patterns),
    ] {
        for args in cases {
            let source = functions
                .get::<Function>(name)
                .unwrap()
                .call::<MultiValue>(MultiValue::from_vec(args.clone()));
            let native = compiled.execute(
                callbacks[name],
                &observation::capture(&args),
                ProgramLimits::default(),
            );
            match source {
                Ok(values) => assert_eq!(
                    comparable(native.unwrap().graph().clone()),
                    comparable(observation::capture(&values.into_vec())),
                    "{name} {args:?}"
                ),
                Err(_) => assert_eq!(
                    native.unwrap_err().kind,
                    ProgramRuntimeErrorKind::Source,
                    "{name} {args:?}"
                ),
            }
            count += 1;
        }
    }
    assert_eq!(count, 47);
    // Address-dependent table formatting is explicitly outside the portable
    // value contract; do not invent a Lua pointer string for a Rust table.
    let table = lua.create_table().unwrap();
    assert!(
        functions
            .get::<Function>("text")
            .unwrap()
            .call::<mlua::LuaString>(table.clone())
            .is_ok()
    );
    assert_eq!(
        compiled
            .execute(
                callbacks["text"],
                &observation::capture(&[Value::Table(table)]),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn source_function_value_lookup_precedes_arguments_and_noncallable_failure_follows_them() {
    let lua = Lua::new();
    let (functions, compiled, callbacks) = programs(&lua);
    for callable in [true, false] {
        let target = lua.create_table().unwrap();
        target
            .set(
                "fn",
                if callable {
                    Value::Function(functions.get("first").unwrap())
                } else {
                    Value::Boolean(false)
                },
            )
            .unwrap();
        let side = lua.create_table().unwrap();
        side.set("count", 0).unwrap();
        side.set("mutate", functions.get::<Function>("mutate").unwrap())
            .unwrap();
        let initial = ProgramValueGraph {
            values: vec![
                ProgramValue::Table(ProgramTableId(1)),
                ProgramValue::Table(ProgramTableId(2)),
                ProgramValue::Callback(callbacks["second"]),
            ],
            tables: vec![
                ProgramTable {
                    entries: vec![(
                        ProgramValue::Bytes(b"fn".to_vec()),
                        if callable {
                            ProgramValue::Callback(callbacks["first"])
                        } else {
                            ProgramValue::Boolean(false)
                        },
                    )],
                },
                ProgramTable {
                    entries: vec![
                        (
                            ProgramValue::Bytes(b"count".to_vec()),
                            ProgramValue::Number(0.0),
                        ),
                        (
                            ProgramValue::Bytes(b"mutate".to_vec()),
                            ProgramValue::Callback(callbacks["mutate"]),
                        ),
                    ],
                },
            ],
        };
        let (mut session, values) = compiled
            .session(&initial, ProgramLimits::default())
            .unwrap();
        let source = functions
            .get::<Function>("call")
            .unwrap()
            .call::<MultiValue>((
                target.clone(),
                side.clone(),
                functions.get::<Function>("second").unwrap(),
            ));
        let native = session.invoke(callbacks["call"], &values);
        if callable {
            assert_eq!(
                comparable(session.snapshot(&native.unwrap()).unwrap().graph().clone()),
                comparable(observation::capture(&source.unwrap().into_vec()))
            );
        } else {
            assert!(source.is_err());
            assert_eq!(native.unwrap_err().kind, ProgramRuntimeErrorKind::Source);
        }
        let source: MultiValue = functions
            .get::<Function>("state")
            .unwrap()
            .call((
                target.clone(),
                side.clone(),
                functions.get::<Function>("second").unwrap(),
            ))
            .unwrap();
        let state = session.invoke(callbacks["state"], &values).unwrap();
        assert_eq!(
            comparable(session.snapshot(&state).unwrap().graph().clone()),
            comparable(observation::capture(&source.into_vec()))
        );
        assert_eq!(side.get::<i64>("count").unwrap(), 1);
        let nil = session
            .borrow(&ProgramValueGraph {
                values: vec![ProgramValue::Nil],
                tables: vec![],
            })
            .unwrap()
            .remove(0);
        assert!(
            functions
                .get::<Function>("call")
                .unwrap()
                .call::<MultiValue>((
                    Value::Nil,
                    side.clone(),
                    functions.get::<Function>("second").unwrap()
                ))
                .is_err()
        );
        assert_eq!(
            session
                .invoke(
                    callbacks["call"],
                    &[nil, values[1].clone(), values[2].clone()]
                )
                .unwrap_err()
                .kind,
            ProgramRuntimeErrorKind::Source
        );
        assert_eq!(side.get::<i64>("count").unwrap(), 1);
        let state = session.invoke(callbacks["state"], &values).unwrap();
        assert_eq!(
            session.snapshot(&state).unwrap().graph().values,
            vec![ProgramValue::Number(1.0), ProgramValue::Boolean(true)]
        );
    }
}
