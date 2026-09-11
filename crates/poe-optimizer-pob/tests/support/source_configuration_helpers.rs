//! Complete original headless helpers and writable modifier identity before parser integration.
use super::*;
fn scalar_cases(lua: &Lua) -> Vec<Vec<Value>> {
    let mut cases = [
        &b""[..],
        b"plain text",
        b"^7red^0white",
        b"^xA1b2C3colour",
        b"^x12345short",
        b"^x1234567tail",
        b"^Xabcdefupper",
        b"^xgggggginvalid",
        b"^^7nested",
        b"^7^xabcdef",
        b"^7line\r\n^x123456line",
        b"^7\0^xABCDEF\xff",
        b"? ^x123456 +10 to Life ?",
        b"^0^1^2^3^4^5^6^7^8^9",
        b"^",
        b"^x",
        b"^x12^73456",
        b"^xabcdef^xABCDEF",
    ]
    .into_iter()
    .map(|bytes| vec![Value::String(lua.create_string(bytes).unwrap())])
    .collect::<Vec<_>>();
    cases.extend([
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Integer(42)],
        vec![Value::Table(lua.create_table().unwrap())],
        vec![
            Value::String(lua.create_string("^7extra").unwrap()),
            Value::Boolean(true),
            Value::Nil,
        ],
    ]);
    cases
}
fn strip(lua: &Lua, captured: &capture::Captured) -> Json {
    let (original, id) = &captured.helpers["original.strip_escapes"];
    assert_eq!(
        original.to_pointer(),
        lua.globals()
            .raw_get::<Function>("StripEscapes")
            .unwrap()
            .to_pointer()
    );
    let info = original.info();
    assert_eq!(info.source.as_deref(), Some("@_SimpleGraphic.def.lua"));
    assert_eq!(
        (info.line_defined, info.last_line_defined),
        (Some(295), Some(298))
    );
    let mut rows = Vec::new();
    for (index, args) in scalar_cases(lua).into_iter().enumerate() {
        let native =
            captured
                .compiled
                .execute(*id, &observation::capture(&args), ProgramLimits::default());
        let actual = original.call::<MultiValue>(MultiValue::from_vec(args));
        match (native, actual) {
            (Ok(native), Ok(actual)) => {
                assert_eq!(actual.len(), 1);
                assert_eq!(
                    observation::canonical(native.graph()),
                    observation::canonical(&observation::capture(&actual.into_vec())),
                    "complete StripEscapes case {index}"
                );
                rows.push(json!({"case":index,"source_error":false}));
            }
            (Err(native), Err(_)) => {
                assert_eq!(
                    native.kind,
                    ProgramRuntimeErrorKind::Source,
                    "StripEscapes case {index}: {native}"
                );
                rows.push(json!({"case":index,"source_error":true}));
            }
            (native, actual) => {
                panic!("StripEscapes case {index}: native={native:?}; actual={actual:?}")
            }
        }
    }
    assert_eq!(rows.len(), 24);
    assert_eq!(
        rows.iter()
            .filter(|row| row["source_error"] == true)
            .count(),
        5
    );
    json!({"cases":rows,"source":"src/_SimpleGraphic.def.lua","first_line":295,"last_line":298,"mode":"original headless Lua function, interpreter"})
}
fn modifier(lua: &Lua, variant: usize) -> Vec<Value> {
    if variant == 8 {
        return vec![Value::Nil];
    }
    if variant == 9 {
        return vec![Value::Integer(4)];
    }
    let outer = lua.create_table().unwrap();
    outer.raw_set("source", "before").unwrap();
    outer.raw_set("name", "helper-fixture").unwrap();
    let nested = lua.create_table().unwrap();
    nested.raw_set("source", "nested-before").unwrap();
    let value = lua.create_table().unwrap();
    match variant {
        0 => outer.raw_set("value", 5).unwrap(),
        1 => (),
        2 => {
            outer.raw_set("value", value.clone()).unwrap();
        }
        3 => {
            value.raw_set("mod", nested.clone()).unwrap();
            outer.raw_set("value", value.clone()).unwrap();
        }
        4 => {
            value.raw_set("mod", false).unwrap();
            outer.raw_set("value", value.clone()).unwrap();
        }
        5 => {
            value.raw_set("mod", true).unwrap();
            outer.raw_set("value", value.clone()).unwrap();
        }
        6 => {
            value.raw_set("mod", 0).unwrap();
            outer.raw_set("value", value.clone()).unwrap();
        }
        7 => {
            value.raw_set("mod", outer.clone()).unwrap();
            outer.raw_set("value", value.clone()).unwrap();
        }
        _ => unreachable!(),
    }
    vec![
        Value::Table(outer),
        Value::Table(nested),
        Value::Table(value),
    ]
}
fn set_source(lua: &Lua, captured: &capture::Captured) -> Json {
    let (original, id) = &captured.helpers["original.set_source"];
    assert_eq!(
        original.to_pointer(),
        lua.globals()
            .raw_get::<Table>("modLib")
            .unwrap()
            .raw_get::<Function>("setSource")
            .unwrap()
            .to_pointer()
    );
    let info = original.info();
    assert_eq!(info.source.as_deref(), Some("@Modules/ModTools.lua"));
    assert_eq!(
        (info.line_defined, info.last_line_defined),
        (Some(277), Some(283))
    );
    let mut rows = Vec::new();
    for variant in 0..10 {
        let roots = modifier(lua, variant);
        let (mut session, handles) = captured
            .compiled
            .session(&observation::capture(&roots), ProgramLimits::default())
            .unwrap();
        for (step, source) in [
            Value::String(lua.create_string("Quest:fixture").unwrap()),
            Value::Nil,
            Value::String(lua.create_string("changed").unwrap()),
            roots[0].clone(),
        ]
        .into_iter()
        .enumerate()
        {
            let source_handle = if step == 3 {
                handles[0].clone()
            } else {
                session
                    .borrow(&observation::capture(std::slice::from_ref(&source)))
                    .unwrap()
                    .remove(0)
            };
            let native = session.invoke(*id, &[handles[0].clone(), source_handle]);
            let actual = original.call::<MultiValue>((roots[0].clone(), source));
            let mut native_state = handles.clone();
            let mut actual_state = roots.clone();
            let error = match (native, actual) {
                (Ok(values), Ok(actual)) => {
                    assert_eq!(values.len(), 1);
                    assert_eq!(actual.len(), 1);
                    assert_eq!(
                        actual.front(),
                        Some(&roots[0]),
                        "setSource returns the exact outer object"
                    );
                    native_state.extend(values);
                    actual_state.extend(actual);
                    false
                }
                (Err(error), Err(_)) => {
                    assert_eq!(
                        error.kind,
                        ProgramRuntimeErrorKind::Source,
                        "setSource {variant}/{step}: {error}"
                    );
                    true
                }
                (native, actual) => {
                    panic!("setSource {variant}/{step}: native={native:?}; actual={actual:?}")
                }
            };
            assert_eq!(error, matches!(variant, 5 | 6 | 8 | 9));
            assert_eq!(
                observation::canonical(session.snapshot(&native_state).unwrap().graph()),
                observation::canonical(&observation::capture(&actual_state)),
                "setSource state/aliases/failure prefix {variant}/{step}"
            );
            rows.push(json!({"variant":variant,"step":step,"source_error":error}));
        }
    }
    assert_eq!(rows.len(), 40);
    json!({"calls":rows,"source":"src/Modules/ModTools.lua","first_line":277,"last_line":283,"reused_sessions":10,"source_errors":16,"mode":"interpreter","private_writable_graphs":true})
}
struct RestoreAppend {
    table: Table,
    key: usize,
    value: Value,
}
impl Drop for RestoreAppend {
    fn drop(&mut self) {
        self.table.raw_set(self.key, self.value.clone()).unwrap();
    }
}
fn insertion(
    lua: &Lua,
    captured: &capture::Captured,
    player: &Table,
    enemy: &Table,
    build: &Table,
) -> Json {
    let source_probe = || {
        captured
            .probe
            .call::<MultiValue>((
                player.clone(),
                enemy.clone(),
                build.clone(),
                captured.names.clone(),
                captured.dropdown_names.clone(),
            ))
            .unwrap()
            .into_vec()
    };
    let before = observation::canonical(&observation::capture(&source_probe()));
    let key = player.raw_len() + 1;
    let old = player.raw_get::<Value>(key).unwrap();
    assert!(matches!(old, Value::Nil));
    let restore = RestoreAppend {
        table: player.clone(),
        key,
        value: old,
    };
    let (mut session, initial) = captured
        .compiled
        .session_from_input(
            captured.observed.input(),
            ProgramLimits {
                max_steps: 5_000_000,
                max_values: 500_000,
                max_bytes: 32 * 1024 * 1024,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    let root = |name: &str| initial[captured.observed.root_index(name).unwrap()].clone();
    let source_roots = modifier(lua, 3);
    let produced = session
        .import_with_coverage(
            &observation::capture(&source_roots),
            &ProgramTableCoverage::new(),
        )
        .unwrap();
    let (set, set_id) = &captured.helpers["original.set_source"];
    let (add, _) = &captured.helpers["original.add_mod"];
    let mut rows = Vec::new();
    for (step, text) in ["Quest:first", "Quest:second"].into_iter().enumerate() {
        let value = Value::String(lua.create_string(text).unwrap());
        let argument = session
            .borrow(&observation::capture(std::slice::from_ref(&value)))
            .unwrap();
        let result = session
            .invoke(*set_id, &[produced[0].clone(), argument[0].clone()])
            .unwrap();
        let actual = set
            .call::<MultiValue>((source_roots[0].clone(), value))
            .unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(actual.len(), 1);
        if step == 0 {
            assert!(
                session
                    .invoke_method(&root("player"), "AddMod", &result)
                    .unwrap()
                    .is_empty()
            );
            assert!(
                add.call::<MultiValue>((player.clone(), actual[0].clone()))
                    .unwrap()
                    .is_empty()
            );
        }
        let mut native = produced.clone();
        native.extend(result);
        native.extend(
            session
                .invoke_callable(
                    &root("probe.state"),
                    &[
                        root("player"),
                        root("enemy"),
                        root("build"),
                        root("names"),
                        root("dropdown_names"),
                    ],
                )
                .unwrap(),
        );
        let mut expected = source_roots.clone();
        expected.extend(actual);
        expected.extend(source_probe());
        assert_eq!(
            observation::canonical(session.snapshot(&native).unwrap().graph()),
            observation::canonical(&observation::capture(&expected)),
            "original setSource and AddMod retain writable nested/list aliases at step {step}"
        );
        rows.push(json!({"step":step,"ordered_state_and_aliases_compared":true}));
    }
    drop(restore);
    assert_eq!(
        observation::canonical(&observation::capture(&source_probe())),
        before,
        "actual modifier list state and aliases restored"
    );
    json!({"steps":rows,"fresh_writable_producer_graph":true,"source_state_restored":true,"parser_service_admission":false})
}
pub fn compare(
    lua: &Lua,
    captured: &capture::Captured,
    player: &Table,
    enemy: &Table,
    build: &Table,
) -> Json {
    let jit: Table = lua.globals().raw_get("jit").unwrap();
    let initially_enabled = jit
        .raw_get::<Function>("status")
        .unwrap()
        .call::<MultiValue>(())
        .unwrap()
        .front()
        == Some(&Value::Boolean(true));
    jit.raw_get::<Function>("off")
        .unwrap()
        .call::<()>(())
        .unwrap();
    jit.raw_get::<Function>("flush")
        .unwrap()
        .call::<()>(())
        .unwrap();
    let result = json!({"strip_escapes":strip(lua,captured),"set_source":set_source(lua,captured),"ordered_insertion":insertion(lua,captured,player,enemy,build),"scope":"Complete original headless StripEscapes and ModTools.setSource; supplied writable modifier fixtures, repeated source writes, aliases, errors and actual ModList.AddMod. No parser service or full quest activation admission."});
    jit.raw_get::<Function>(if initially_enabled { "on" } else { "off" })
        .unwrap()
        .call::<()>(())
        .unwrap();
    result
}
