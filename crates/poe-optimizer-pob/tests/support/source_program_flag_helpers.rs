//! Complete original Global.lua helpers over the initialized build's real flags.
//! This is differential test instrumentation, not a native mask implementation.
use super::*;
use poe_optimizer_data::source_program::SourceCallbackKind;

pub(super) const NAMES: &[&str] = &["OR64", "AND64", "XOR64", "NOT64"];
const PACKED: &str = r#"
local function pack(...) return { n = select('#', ...), ... } end
return function(original, rows, ...)
    local result = pack(original(...))
    local index = rows.calls + 1
    rows.calls = index
    rows[index] = result
    return result
end
"#;
fn canonical(graph: &ProgramValueGraph) -> Json {
    fn normalize(value: &mut Json) {
        match value {
            Json::Object(object) => {
                if let Some(Json::String(bits)) = object.get_mut("number_bits")
                    && f64::from_bits(u64::from_str_radix(bits, 16).unwrap()).is_nan()
                {
                    *bits = "nan_class".into();
                }
                for value in object.values_mut() {
                    normalize(value);
                }
            }
            Json::Array(values) => {
                for value in values {
                    normalize(value);
                }
            }
            _ => {}
        }
    }
    let mut value = observation::canonical(graph);
    normalize(&mut value);
    value
}
fn source_pack(values: &[Value]) -> Json {
    canonical(&observation::capture(values))
}
fn compare(pair: &mut Pair, actual: &[Value], native: &[SessionValue], label: &str) -> Json {
    assert_eq!(actual.len(), native.len(), "flag helper full pack: {label}");
    let expected = source_pack(actual);
    let snapshot = pair.session.snapshot(native).unwrap();
    let observed = canonical(snapshot.graph());
    assert_eq!(observed, expected, "flag helper result: {label}");
    observed
}
fn masks(lua: &Lua) -> Vec<(String, f64)> {
    let mut values = BTreeMap::new();
    for name in ["ModFlag", "KeywordFlag"] {
        let table: Table = lua.globals().raw_get(name).unwrap();
        let mut count = 0;
        for entry in table.pairs::<String, Value>() {
            let (key, value) = entry.unwrap();
            let number = match value {
                Value::Integer(value) => value as f64,
                Value::Number(value) => value,
                _ => panic!("actual {name}.{key} is not a numeric flag"),
            };
            assert!(number.is_finite(), "actual {name}.{key} must be finite");
            values.insert(format!("{name}.{key}"), number);
            count += 1;
            assert!(count <= 256, "bounded actual flag inventory");
        }
        assert!(count > 0, "initialized flag table {name}");
    }
    values.into_iter().collect()
}
fn cases(lua: &Lua, masks: &[(String, f64)]) -> Vec<(String, Vec<Value>)> {
    let number = Value::Number;
    let text = |s: &str| Value::String(lua.create_string(s).unwrap());
    let passthrough = lua.create_table().unwrap();
    passthrough
        .raw_set("input", "flag helper passthrough")
        .unwrap();
    let mut cases = vec![
        ("zero_arguments".into(), vec![]),
        ("nil_only".into(), vec![Value::Nil]),
        ("false_only".into(), vec![Value::Boolean(false)]),
        ("true_only".into(), vec![Value::Boolean(true)]),
        ("text_passthrough".into(), vec![text("unchanged")]),
        ("table_passthrough".into(), vec![Value::Table(passthrough)]),
        (
            "numeric_text_pair".into(),
            vec![text("4294967297"), number(3.0)],
        ),
        (
            "bad_text_pair".into(),
            vec![text("not a number"), number(1.0)],
        ),
        ("nil_first_pair".into(), vec![Value::Nil, number(1.0)]),
        (
            "false_first_pair".into(),
            vec![Value::Boolean(false), number(1.0)],
        ),
        (
            "nil_second_short_circuit".into(),
            vec![number(7.0), Value::Nil, Value::Boolean(false)],
        ),
        (
            "nil_third_short_circuit".into(),
            vec![number(1.0), number(2.0), Value::Nil, text("ignored")],
        ),
        (
            "bad_third".into(),
            vec![number(1.0), number(2.0), Value::Boolean(false)],
        ),
        (
            "eight_numbers".into(),
            (0..8).map(|i| number(f64::from(i))).collect(),
        ),
        (
            "ninth_ignored".into(),
            (0..8)
                .map(|i| number(f64::from(i)))
                .chain([Value::Boolean(false)])
                .collect(),
        ),
    ];
    let boundaries = [
        0.0,
        -0.0,
        1.0,
        -1.0,
        1.5,
        -1.5,
        2147483647.0,
        2147483648.0,
        4294967295.0,
        4294967296.0,
        4294967297.0,
        4503599627370497.0,
        9007199254740991.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
    ];
    for (index, value) in boundaries.into_iter().enumerate() {
        cases.push((format!("boundary_single_{index}"), vec![number(value)]));
        cases.push((
            format!("boundary_pair_{index}"),
            vec![number(value), number(4294967297.0)],
        ));
    }
    for (index, (name, value)) in masks.iter().enumerate() {
        cases.push((format!("data_single_{name}"), vec![number(*value)]));
        cases.push((
            format!("data_pair_{name}"),
            vec![number(*value), number(masks[(index + 1) % masks.len()].1)],
        ));
    }
    for (index, chunk) in masks.chunks(8).enumerate() {
        cases.push((
            format!("data_many_{index}"),
            chunk.iter().map(|(_, value)| number(*value)).collect(),
        ));
    }
    cases
}
fn original(lua: &Lua, pair: &Pair, name: &str) -> (Function, Json) {
    let function: Function = lua.globals().raw_get(name).unwrap();
    let info = function.info();
    assert_eq!(info.what, "Lua");
    assert!(info.source.as_deref().unwrap().ends_with("Data/Global.lua"));
    let input = pair.observed.input();
    let ProgramValue::Closure(id) =
        input.state.values[pair.observed.root_index(&format!("flag.{name}")).unwrap()]
    else {
        panic!("observed actual flag helper closure");
    };
    let instance = &input.closures[id.0 as usize - 1];
    let prototype = pair
        .observed
        .owner()
        .resolve_closure_prototype(&instance.prototype)
        .unwrap();
    let callback = pair.observed.owner().callback(prototype.callback).unwrap();
    let SourceCallbackKind::Lua { source } = &callback.kind else {
        panic!("original Lua helper")
    };
    assert!(source.path.ends_with("Data/Global.lua"));
    assert_eq!(info.line_defined, Some(source.line as usize));
    assert_eq!(info.last_line_defined, Some(source.end_line as usize));
    (
        function,
        json!({"callback":prototype.callback,"declaration":callback.kind,"root":format!("flag.{name}")}),
    )
}
pub(super) fn run(lua: &Lua, parent: &Pair) -> Json {
    lua.load("jit.off();jit.flush();assert(not jit.status())")
        .exec()
        .unwrap();
    let actual_masks = masks(lua);
    let source_cases = cases(lua, &actual_masks);
    assert!(
        source_cases.len() <= 1200,
        "bounded complete helper input matrix"
    );
    let parent_steps = parent.session.steps();
    let (session, roots) = parent
        .compiled
        .session_from_input(parent.observed.input(), limits())
        .unwrap();
    let mut pair = Pair {
        copy_table: parent.copy_table.clone(),
        observed: parent.observed.clone(),
        compiled: parent.compiled.clone(),
        session,
        roots,
    };
    let mut cold = Vec::new();
    let mut declarations = BTreeMap::new();
    let mut functions = BTreeMap::new();
    let mut success_count = 0;
    let mut source_error_count = 0;
    for name in NAMES {
        let (function, declaration) = original(lua, &pair, name);
        declarations.insert(*name, declaration);
        for (label, arguments) in &source_cases {
            let label = format!("{name}/{label}");
            let native_args = pair.args(arguments);
            let actual = function.call::<MultiValue>(MultiValue::from_vec(arguments.clone()));
            let native = pair.call(&format!("flag.{name}"), &native_args);
            let outcome = match (actual, native) {
                (Ok(actual), Ok(native)) => {
                    let actual = actual.into_vec();
                    let comparison = compare(&mut pair, &actual, &native, &label);
                    for (index, value) in actual.iter().enumerate() {
                        if let Value::Table(_) = value {
                            let argument = arguments
                                .iter()
                                .position(|input| input == value)
                                .expect("actual helper table passthrough");
                            let different = pair
                                .call(
                                    "probe.distinct",
                                    &[native[index].clone(), native_args[argument].clone()],
                                )
                                .unwrap();
                            assert!(
                                !pair.boolean(&different),
                                "native passthrough identity: {label}"
                            );
                        }
                    }
                    success_count += 1;
                    json!({"result":comparison})
                }
                (Err(actual), Err(native)) => {
                    assert_eq!(
                        native.kind,
                        ProgramRuntimeErrorKind::Source,
                        "{label}: original{actual}; native{native}"
                    );
                    source_error_count += 1;
                    json!({"source_error":actual.to_string(),"native_error":native.message})
                }
                (actual, native) => panic!(
                    "flag helper success/error mismatch {label}: original{actual:?}; native{native:?}"
                ),
            };
            cold.push(json!({"helper":name,"case":label,"arguments":source_pack(arguments),"outcome":outcome}));
        }
        functions.insert(*name, function);
    }
    let wrapper: Function = lua
        .load(PACKED)
        .set_name("@tests/support/source_program_flag_helpers_wrapper.lua")
        .eval()
        .unwrap();
    let driver = warm::SourceWarmDriver::new(lua).unwrap();
    let derived = (0..8)
        .map(|index| actual_masks[index * (actual_masks.len() - 1) / 7].1)
        .collect::<Vec<_>>();
    let vectors = [
        vec![0.0, 0.0],
        vec![1.0, 2.0],
        vec![2147483647.0, 2147483648.0],
        vec![4294967297.0, 4294967299.0],
        vec![4503599627370497.0, 1099511627785.0],
        derived,
    ];
    let mut warmed = Vec::new();
    for name in NAMES {
        let function = &functions[name];
        for (index, vector) in vectors.iter().enumerate() {
            let arguments = vector
                .iter()
                .copied()
                .map(Value::Number)
                .collect::<Vec<_>>();
            let native_args = pair.args(&arguments);
            let rows = lua.create_table().unwrap();
            rows.raw_set("calls", 0).unwrap();
            let mut source_args = vec![
                Value::Function(function.clone()),
                Value::Table(rows.clone()),
            ];
            source_args.extend(arguments.clone());
            let evidence = driver
                .run_with_target(lua, &wrapper, function, &source_args, None)
                .unwrap_or_else(|error| panic!("actual helper warm {name}/{index}: {error}"));
            assert!(evidence.success);
            assert_eq!(evidence.calls, 128);
            assert_eq!(evidence.seed_calls, 0);
            assert!(evidence.target_live_traces > 0);
            assert_eq!(rows.raw_get::<usize>("calls").unwrap(), 128);
            assert_eq!(evidence.value, rows.raw_get::<Value>(128).unwrap());
            let mut results = Vec::new();
            for invocation in 1..=128 {
                let pack: Table = rows.raw_get(invocation).unwrap();
                let count: usize = pack.raw_get("n").unwrap();
                assert!(count <= 8, "bounded actual helper pack");
                let actual = (1..=count)
                    .map(|slot| pack.raw_get::<Value>(slot).unwrap())
                    .collect::<Vec<_>>();
                let native = pair
                    .call(&format!("flag.{name}"), &native_args)
                    .unwrap_or_else(|error| {
                        panic!("native warm {name}/{index}/{invocation}: {error}")
                    });
                results.push(compare(
                    &mut pair,
                    &actual,
                    &native,
                    &format!("warm{name}/{index}/{invocation}"),
                ));
            }
            warmed.push(json!({"helper":name,"vector":index,"arguments":source_pack(&arguments),"calls":128,"seed_calls":0,
                "actual_helper_target_traces":evidence.target_live_traces,"live_traces":evidence.live_traces,"result_packs":results}));
        }
    }
    for (name, function) in functions {
        assert_eq!(
            lua.globals().raw_get::<Function>(name).unwrap(),
            function,
            "original helper identity retained"
        );
    }
    assert_eq!(masks(lua), actual_masks, "real flag data unchanged");
    assert_eq!(
        parent.session.steps(),
        parent_steps,
        "helper session is private"
    );
    lua.load("jit.off();jit.flush()").exec().unwrap();
    json!({"scope":"complete unchanged Global.lua helpers; original closures/cells and projected math/bit libraries in the initialized owner",
        "declarations":declarations,"data_masks":actual_masks.into_iter().map(|(name,value)|json!({"name":name,"value_bits":format!("{:016x}",value.to_bits())})).collect::<Vec<_>>(),
        "cold_case_count":cold.len(),"cold_successes":success_count,"cold_source_errors":source_error_count,"cold_cases":cold,
        "warm_vectors":warmed,"warm_vector_count":24,"warm_calls":3072,"warm_seed_calls":0,
        "comparison":"full packs, exact finite/infinite/signed-zero bits and aliases; NaN class only normalized",
        "warm_scope":"generic test wrapper records every full pack; exact original helper is required in completed still-live traces; no helper body is copied or replaced",
        "private_session":true,"source_flags_and_function_identities_retained":true,"steps":pair.session.steps(),"pattern_steps":pair.session.pattern_steps()})
}
