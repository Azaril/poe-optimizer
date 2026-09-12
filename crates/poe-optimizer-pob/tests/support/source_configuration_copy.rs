//! Complete actual Common.copyTable, with explicit source-observed live table facts.
//! Fixture copies and selected actual cache rows are component gates, not a parser service.
use super::*;
const PATH: &str = "tests/support/source_configuration_copy.lua";
const TEXT: &str = include_str!("source_configuration_copy.lua");
struct Pair<'a> {
    lua: &'a Lua,
    probes: Table,
    observed: ObservedSourceSession,
    session: ProgramSession,
    handles: Vec<SessionValue>,
}
impl Pair<'_> {
    fn root(&self, name: &str) -> SessionValue {
        self.handles[self.observed.root_index(name).unwrap()].clone()
    }
    fn native(
        &mut self,
        name: &str,
        args: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session
            .invoke_callable(&self.root(&format!("probe.{name}")), args)
    }
    fn source(&self, name: &str, args: &[Value]) -> mlua::Result<MultiValue> {
        self.probes
            .raw_get::<Function>(name)
            .unwrap()
            .call(MultiValue::from_vec(args.to_vec()))
    }
    fn args(&mut self, args: &[Value]) -> Vec<SessionValue> {
        self.session.borrow(&observation::capture(args)).unwrap()
    }
    fn compare(
        &mut self,
        name: &str,
        source: &[Value],
        native: &[SessionValue],
    ) -> (Vec<Value>, Vec<SessionValue>) {
        let actual = self.source(name, source).unwrap().into_vec();
        let expected = self.native(name, native).unwrap();
        assert_eq!(
            observation::canonical(self.session.snapshot(&expected).unwrap().graph()),
            observation::canonical(&observation::capture(&actual)),
            "original copy consumer {name}"
        );
        (actual, expected)
    }
}
fn fixtures(lua: &Lua) -> BTreeMap<String, Table> {
    let mut tables = BTreeMap::new();
    for (name, expression) in [
        ("empty", "{}"),
        ("one", "{7}"),
        ("dense", "{false, 3, 'a' .. string.char(0, 255)}"),
        ("second", "{nil, 'unparsed'}"),
        ("second_false", "{nil, false}"),
        ("second_empty", "{nil, ''}"),
        ("third", "{[3]='third'}"),
        ("zero_second", "{[0]=false, [2]='second'}"),
        ("sparse_runs", "{[1]='first', [3]='third'}"),
        (
            "holes",
            "{[1]='first', [3]='third', [-2]='negative', [0]=false, text=9}",
        ),
        (
            "scalars",
            "{a=-0.0, b=1/0, c=-1/0, d=0/0, e=false, f='a'..string.char(0,255)}",
        ),
        ("nested", "{left={n=2}, right={n=3}, [1]={n=4}}"),
    ] {
        tables.insert(
            format!("fixture.{name}"),
            lua.load(format!("return {expression}")).eval().unwrap(),
        );
    }
    let nested = lua.create_table().unwrap();
    nested.raw_set("value", 11).unwrap();
    let aliases = lua.create_table().unwrap();
    aliases.raw_set("left", nested.clone()).unwrap();
    aliases.raw_set("right", nested.clone()).unwrap();
    aliases.raw_set(1, nested).unwrap();
    tables.insert("fixture.aliases".into(), aliases);
    let functions = lua.create_table().unwrap();
    functions
        .raw_set("f", lua.globals().raw_get::<Function>("type").unwrap())
        .unwrap();
    tables.insert("fixture.functions".into(), functions);
    tables
}
/// Selection is for a bounded oracle only. The real parser service must preserve
/// every supported cache entry, including function-bearing outputs and history.
fn plain_row(value: &Value, depth: usize, remaining: &mut usize) -> bool {
    if depth > 8 || *remaining == 0 {
        return false;
    }
    *remaining -= 1;
    match value {
        Value::Nil
        | Value::Boolean(_)
        | Value::Integer(_)
        | Value::Number(_)
        | Value::String(_) => true,
        Value::Table(table) if table.metatable().is_none() => {
            table.clone().pairs::<Value, Value>().all(|entry| {
                let Ok((key, value)) = entry else {
                    return false;
                };
                matches!(key, Value::String(_) | Value::Integer(_) | Value::Number(_))
                    && plain_row(&value, depth + 1, remaining)
            })
        }
        _ => false,
    }
}
fn observe<'a>(
    lua: &'a Lua,
    primitives: &Primitives,
    probes: Table,
    tables: &BTreeMap<String, Table>,
) -> Pair<'a> {
    let vendor =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let common =
        poe_optimizer_pob::source::read_verified_text(&vendor, "src/Modules/Common.lua").unwrap();
    let texts = BTreeMap::from([
        ("src/Modules/Common.lua".into(), common),
        (PATH.into(), TEXT.into()),
    ]);
    let (provenance, source_names) = classes::inventory(lua, &vendor, &texts);
    let functions = probes
        .clone()
        .pairs::<String, Function>()
        .map(Result::unwrap)
        .map(|(name, f)| (format!("probe.{name}"), f))
        .collect();
    let globals = lua.globals();
    let observed = primitives
        .observer
        .observe_session(
            lua,
            &texts,
            provenance,
            SourceSessionCaptureRequest {
                callbacks: functions,
                state_roots: tables
                    .iter()
                    .map(|(name, table)| (name.clone(), Value::Table(table.clone())))
                    .collect(),
                definitions: SourceCaptureContext {
                    capture_iteration: true,
                    environment: Some(SourceEnvironmentSelection {
                        table: globals.clone(),
                        root_name: "Environment".into(),
                    }),
                    projections: vec![SourceTableSelection {
                        table: globals,
                        fields: ["copyTable".into()].into(),
                        indexed: BTreeSet::new(),
                        allow_index_fallback: false,
                        allow_call_fallback: false,
                    }],
                    source_names,
                },
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap();
    let without_constructors = lower_from_sources(&texts, observed.owner()).unwrap();
    let unproven = CompiledSourcePrograms::new(without_constructors.catalog()).unwrap();
    let (mut unproven_session, unproven_roots) = unproven
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    let copied = unproven_session
        .invoke_callable(
            &unproven_roots[observed.root_index("probe.copy").unwrap()],
            &[unproven_roots[observed.root_index("fixture.second").unwrap()].clone()],
        )
        .unwrap();
    let error = unproven_session
        .invoke_callable(
            &unproven_roots[observed.root_index("probe.unpack").unwrap()],
            &copied,
        )
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    let lowered = poe_optimizer_pob::source_programs::lower_observed_from_sources(
        &texts,
        observed.owner(),
        observed
            .constructor_observations()
            .expect("opted-in constructor observation"),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "copy source lowering: {:?}",
        lowered.unsupported()
    );
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let (session, handles) = compiled
        .session_from_input(
            observed.input(),
            ProgramLimits {
                max_values: 500_000,
                max_bytes: 32 * 1024 * 1024,
                ..ProgramLimits::default()
            },
        )
        .unwrap();
    Pair {
        lua,
        probes,
        observed,
        session,
        handles,
    }
}
pub fn run(lua: &Lua, primitives: &Primitives) -> Json {
    let jit: Table = lua.globals().raw_get("jit").unwrap();
    let enabled = jit
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
    let original: Function = lua.globals().raw_get("copyTable").unwrap();
    assert_eq!(
        original.info().source.as_deref(),
        Some("@Modules/Common.lua")
    );
    assert_eq!(
        (
            original.info().line_defined,
            original.info().last_line_defined
        ),
        (Some(495), Some(505))
    );
    let probes: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let mut tables = fixtures(lua);
    let cache: Table = lua
        .globals()
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseModCache")
        .unwrap();
    // Keep at most eight Lua values rooted while selecting lexicographic keys.
    // A full Vec of the real cache can exceed mlua's auxiliary-reference stack.
    let mut selected: Vec<(String, Value)> = Vec::new();
    for row in cache.pairs::<String, Value>() {
        let (line, value) = row.unwrap();
        if selected.len() == 8 && line >= selected[7].0 {
            continue;
        }
        if plain_row(&value, 0, &mut 200) {
            let index = selected.partition_point(|(key, _)| key < &line);
            selected.insert(index, (line, value));
            selected.truncate(8);
        }
    }
    assert_eq!(
        selected.len(),
        8,
        "actual loaded parser cache has sufficient plain result rows"
    );
    for (index, (_, value)) in selected.iter().enumerate() {
        let Value::Table(table) = value else {
            panic!("source public cache row must be a table");
        };
        tables.insert(format!("cache.{index}"), table.clone());
    }
    let mut pair = observe(lua, primitives, probes, &tables);
    let mut rows = Vec::new();
    let mut copied = BTreeMap::new();
    for (name, table) in &tables {
        for shallow in [false, true] {
            let mut args = vec![pair.root(name)];
            args.extend(pair.args(&[Value::Boolean(shallow)]));
            if name == "fixture.functions" {
                let original_copy = pair
                    .source(
                        "copy",
                        &[Value::Table(table.clone()), Value::Boolean(shallow)],
                    )
                    .unwrap()
                    .into_vec();
                let native_copy = pair.native("copy", &args).unwrap();
                assert_eq!(original_copy.len(), 1);
                assert_eq!(native_copy.len(), 1);
                pair.compare(
                    "identity",
                    &[Value::Table(table.clone()), original_copy[0].clone()],
                    &[pair.root(name), native_copy[0].clone()],
                );
            } else {
                let actual = pair
                    .source(
                        "copy",
                        &[Value::Table(table.clone()), Value::Boolean(shallow)],
                    )
                    .unwrap()
                    .into_vec();
                let native = pair.native("copy", &args).unwrap();
                assert_eq!(actual.len(), 1);
                assert_eq!(native.len(), 1);
                // Include the input root so shared/split aliases survive normalization.
                assert_eq!(
                    observation::canonical(
                        pair.session
                            .snapshot(&[pair.root(name), native[0].clone()])
                            .unwrap()
                            .graph()
                    ),
                    observation::canonical(&observation::capture(&[
                        Value::Table(table.clone()),
                        actual[0].clone()
                    ])),
                    "copy {name}, shallow={shallow}"
                );
                if !shallow {
                    copied.insert(name.clone(), (actual[0].clone(), native[0].clone()));
                }
            }
            rows.push(json!({"name":name,"shallow":shallow,"source_copy_complete":true}));
        }
    }
    // The same live input can change values while retaining observed key layout.
    let target = tables["fixture.one"].clone();
    let mut replace = vec![pair.root("fixture.one")];
    replace.extend(pair.args(&[Value::Integer(1), Value::Integer(21)]));
    pair.compare(
        "replace",
        &[
            Value::Table(target.clone()),
            Value::Integer(1),
            Value::Integer(21),
        ],
        &replace,
    );
    pair.compare(
        "copy",
        &[Value::Table(target.clone())],
        &[pair.root("fixture.one")],
    );
    let mut lengths = Vec::new();
    for name in [
        "fixture.empty",
        "fixture.one",
        "fixture.dense",
        "fixture.second",
        "fixture.second_false",
        "fixture.second_empty",
        "fixture.third",
        "fixture.zero_second",
        "fixture.sparse_runs",
        "fixture.holes",
    ] {
        let table = Value::Table(tables[name].clone());
        pair.compare("length", std::slice::from_ref(&table), &[pair.root(name)]);
        pair.compare("unpack", std::slice::from_ref(&table), &[pair.root(name)]);
        let (source_copy, native_copy) = copied.get(name).unwrap();
        let actual = pair
            .source("unpack", std::slice::from_ref(source_copy))
            .unwrap()
            .into_vec();
        let native = pair.native("unpack", std::slice::from_ref(native_copy));
        let admitted = match native {
            Ok(result) => {
                assert_eq!(
                    observation::canonical(pair.session.snapshot(&result).unwrap().graph()),
                    observation::canonical(&observation::capture(&actual))
                );
                true
            }
            Err(error) => {
                assert_eq!(
                    error.kind,
                    ProgramRuntimeErrorKind::UnsupportedCapability,
                    "unproved copied hash layout or JIT-sensitive length is explicit"
                );
                assert!(matches!(
                    name,
                    "fixture.holes"
                        | "fixture.third"
                        | "fixture.zero_second"
                        | "fixture.sparse_runs"
                ));
                false
            }
        };
        assert_eq!(
            admitted,
            !matches!(
                name,
                "fixture.holes" | "fixture.third" | "fixture.zero_second" | "fixture.sparse_runs"
            ),
            "a coincidentally matching source sample cannot establish copied-table layout"
        );
        let range = pair.args(&[Value::Integer(1), Value::Integer(3)]);
        pair.compare(
            "unpack",
            &[source_copy.clone(), Value::Integer(1), Value::Integer(3)],
            &[native_copy.clone(), range[0].clone(), range[1].clone()],
        );
        lengths.push(json!({"name":name,"observed_input_default_unpack":true,"source_copy_result_count":actual.len(),"native_copy_default_unpack":admitted,"explicit_copy_range":true}));
    }
    // Warm the actual Common.copyTable callback, not a replacement copy kernel.
    // Each resulting graph retains the source's deep-copy alias behavior. The
    // packed-unpack probe preserves every result and explicit cardinality.
    let driver = warm::SourceWarmDriver::new(lua).unwrap();
    let mut warmed = Vec::new();
    for name in [
        "fixture.empty",
        "fixture.one",
        "fixture.dense",
        "fixture.second",
        "fixture.second_false",
        "fixture.second_empty",
        "fixture.third",
        "fixture.aliases",
    ] {
        let input = Value::Table(tables[name].clone());
        let actual = driver
            .run(lua, &original, std::slice::from_ref(&input), None)
            .unwrap_or_else(|error| panic!("warm original copy {name}: {error}"));
        assert!(actual.success);
        let result = pair.native("copy", &[pair.root(name)]).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            observation::canonical(
                pair.session
                    .snapshot(&[pair.root(name), result[0].clone()])
                    .unwrap()
                    .graph()
            ),
            observation::canonical(&observation::capture(&[input, actual.value.clone()])),
            "warmed original copy {name}"
        );
        let mut row = json!({"name":name,"copy_calls":actual.calls,"copy_target_live_traces":actual.target_live_traces});
        if !matches!(name, "fixture.third" | "fixture.aliases") {
            let unpacked = driver
                .run(
                    lua,
                    &pair.probes.raw_get::<Function>("packed_unpack").unwrap(),
                    &[actual.value],
                    None,
                )
                .unwrap_or_else(|error| panic!("warm packed unpack {name}: {error}"));
            assert!(unpacked.success);
            let native = pair.native("packed_unpack", &result).unwrap();
            assert_eq!(
                observation::canonical(pair.session.snapshot(&native).unwrap().graph()),
                observation::canonical(&observation::capture(&[unpacked.value])),
                "warmed default unpack {name}"
            );
            row["unpack_calls"] = json!(unpacked.calls);
            row["unpack_target_live_traces"] = json!(unpacked.target_live_traces);
        }
        warmed.push(row);
    }
    for args in [
        vec![],
        vec![Value::Nil],
        vec![Value::Boolean(false)],
        vec![Value::Integer(4)],
    ] {
        let native = pair.args(&args);
        let before = pair.session.allocations();
        assert!(pair.source("copy", &args).is_err());
        let error = pair.native("copy", &native).unwrap_err();
        assert_eq!(error.kind, ProgramRuntimeErrorKind::Source);
        assert!(
            pair.session.allocations().tables > before.tables,
            "fresh output allocation precedes original pairs failure"
        );
    }
    // Structural writes remain valid, but cannot retain unproved physical layout.
    let mut replace = vec![pair.root("fixture.one")];
    let key = Value::String(pair.lua.create_string("added").unwrap());
    replace.extend(pair.args(&[key.clone(), Value::Integer(8)]));
    pair.compare(
        "replace",
        &[Value::Table(target.clone()), key, Value::Integer(8)],
        &replace,
    );
    assert!(pair.source("copy", &[Value::Table(target)]).is_ok());
    let error = pair
        .native("copy", &[pair.root("fixture.one")])
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    jit.raw_get::<Function>(if enabled { "on" } else { "off" })
        .unwrap()
        .call::<()>(())
        .unwrap();
    json!({"source":"src/Modules/Common.lua","source_first_line":495,"source_last_line":505,"copies":rows,"warm_cases":warmed,"constructor_evidence_required":true,"actual_cache_keys":selected.iter().map(|(key,_)|key).collect::<Vec<_>>(),"lengths":lengths,"source_error_cases":4,"live_value_replacement":true,"structural_write_invalidates_traversal":true,"parser_service_admission":false,"scope":"Actual original copyTable over observed writable fixture and selected existing cache rows. Copies split repeated nested aliases; shallow copies/functions preserve identity. Actual empty constructor allocation admits copied second-slot rows; mixed-hash layout, JIT-sensitive multiple length boundaries and structural writes to observed input remain explicit frontiers."})
}
