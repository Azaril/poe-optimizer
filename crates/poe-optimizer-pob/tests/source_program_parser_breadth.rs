//! Actual import-time parser inputs from complete builds, followed by isolated
//! native miss/hit replay. Corpus coverage is separate from full-build parity.
#![cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
#[path = "support/source_program_classes.rs"]
mod classes;
#[allow(dead_code)]
#[path = "support/source_program_observation.rs"]
mod observation;
#[path = "support/source_program_parser_capture.rs"]
mod parser_capture;
#[allow(dead_code)]
#[path = "support/configuration_preparation_source.rs"]
mod source;
#[path = "support/source_program_state_watch.rs"]
mod state_watch;
use classes::Primitives;
use mlua::{Function, Lua, LuaSerdeExt, MultiValue, Table, Value};
use poe_optimizer_data::source_program::{SourceProgramExpr, SourceProgramLocation};
use poe_optimizer_engine::{lua_pattern::MatchLimits, source_program::*};
use poe_optimizer_pob::{
    runtime::RuntimeError,
    source_programs::{attach_reserved_string_templates, capture::*},
};
use serde_json::{Value as Json, json};
use sha2::{Digest, Sha256};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{Duration, Instant},
};
const TEST: &str = "all_original_import_parser_inputs_have_native_outcomes";
const MANIFEST: &str = "tests/fixtures/builds/breadth-20260908/index.json";
const OBSERVER_PATH: &str = "tests/support/source_program_import_inputs.lua";
const OBSERVER: &str = include_str!("support/source_program_import_inputs.lua");
const OUTPUT: &str = "POE_PARSER_BREADTH_OUTPUT";
const CHILD: &str = "POE_PARSER_BREADTH_CHILD";
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn limits() -> ProgramLimits {
    ProgramLimits {
        max_steps: 50_000_000,
        max_values: 8_000_000,
        max_bytes: 256 * 1024 * 1024,
        max_tables: 100_000,
        pattern: MatchLimits {
            max_steps: 200_000_000,
            ..Default::default()
        },
        ..Default::default()
    }
}
// Traverse the validated serializable AST for diagnostic selection only. The live
// source adapter, not these JSON locations, authenticates every attached site.
fn table_locations(value: &Json, result: &mut Vec<SourceProgramLocation>) {
    if value
        .get("operation")
        .and_then(|v| v.get("kind"))
        .and_then(Json::as_str)
        == Some("table")
    {
        let expr: SourceProgramExpr = serde_json::from_value(value.clone()).unwrap();
        result.push(expr.location);
    }
    match value {
        Json::Array(a) => {
            for v in a {
                table_locations(v, result)
            }
        }
        Json::Object(a) => {
            for v in a.values() {
                table_locations(v, result)
            }
        }
        _ => {}
    }
}
fn descriptor(lua: &Lua, row: &Table) -> Value {
    let kind: String = row.raw_get("kind").unwrap();
    match kind.as_str() {
        "nil" => Value::Nil,
        "boolean" => Value::Boolean(row.raw_get("value").unwrap()),
        "string" => Value::String(
            lua.create_string(row.raw_get::<mlua::LuaString>("value").unwrap().as_bytes())
                .unwrap(),
        ),
        "number" => Value::Number(row.raw_get("value").unwrap()),
        _ => panic!("unrepresented actual argument {kind}"),
    }
}
fn scalar_json(value: &Value) -> Json {
    match value {
        Value::Nil => json!({"kind":"nil"}),
        Value::Boolean(v) => json!({"kind":"boolean","value":v}),
        Value::Integer(v) => {
            json!({"kind":"number","bits":format!("{:016x}",(*v as f64).to_bits())})
        }
        Value::Number(v) => json!({"kind":"number","bits":format!("{:016x}",v.to_bits())}),
        Value::String(v) => json!({"kind":"string","bytes":v.as_bytes().to_vec()}),
        _ => panic!("actual parser arguments must be represented exactly"),
    }
}
// Check the observation domain before using the existing plain graph comparer.
// Unrepresented behavior remains in the denominator; it never becomes a pass.
fn capture_plain(values: &[Value]) -> Option<ProgramValueGraph> {
    struct Capture {
        graph: ProgramValueGraph,
        seen: BTreeMap<usize, ProgramTableId>,
        nodes: usize,
        bytes: usize,
    }
    impl Capture {
        fn value(&mut self, v: &Value, depth: usize) -> Option<ProgramValue> {
            if self.nodes == 0 || depth > 128 {
                return None;
            }
            self.nodes -= 1;
            Some(match v {
                Value::Nil => ProgramValue::Nil,
                Value::Boolean(v) => ProgramValue::Boolean(*v),
                Value::Integer(v) => ProgramValue::Number(*v as f64),
                Value::Number(v) => ProgramValue::Number(*v),
                Value::String(v) => {
                    self.bytes = self.bytes.checked_sub(v.as_bytes().len())?;
                    ProgramValue::Bytes(v.as_bytes().to_vec())
                }
                Value::Table(t) => {
                    if t.metatable().is_some() {
                        return None;
                    }
                    let pointer = t.to_pointer() as usize;
                    if let Some(id) = self.seen.get(&pointer) {
                        return Some(ProgramValue::Table(*id));
                    }
                    let id = ProgramTableId(self.graph.tables.len() as u32 + 1);
                    self.seen.insert(pointer, id);
                    self.graph.tables.push(ProgramTable::default());
                    // Retain only native values; never collect Lua key/value handles.
                    let mut entries = Vec::new();
                    for (index, row) in t.clone().pairs::<Value, Value>().enumerate() {
                        if index >= 2048 {
                            return None;
                        }
                        let (key, value) = row.ok()?;
                        if !matches!(key, Value::Integer(_) | Value::Number(_) | Value::String(_)) {
                            return None;
                        }
                        entries
                            .push((self.value(&key, depth + 1)?, self.value(&value, depth + 1)?));
                    }
                    self.graph.tables[id.0 as usize - 1].entries = entries;
                    ProgramValue::Table(id)
                }
                _ => return None,
            })
        }
    }
    let mut c = Capture {
        graph: ProgramValueGraph::default(),
        seen: BTreeMap::new(),
        nodes: 8192,
        bytes: 1024 * 1024,
    };
    c.graph.values = values
        .iter()
        .map(|v| c.value(v, 0))
        .collect::<Option<Vec<_>>>()?;
    Some(c.graph)
}
#[cfg(test)]
fn plain(values: &[Value]) -> bool {
    capture_plain(values).is_some()
}
fn source_arguments(lua: &Lua, values: &[ProgramValue]) -> Vec<Value> {
    values
        .iter()
        .map(|v| match v {
            ProgramValue::Nil => Value::Nil,
            ProgramValue::Boolean(v) => Value::Boolean(*v),
            ProgramValue::Number(v) => Value::Number(*v),
            ProgramValue::Bytes(v) => Value::String(lua.create_string(v).unwrap()),
            _ => panic!("non-scalar observed parser argument"),
        })
        .collect()
}
fn output(lua: &Lua) -> Json {
    let build: Table = lua.globals().raw_get("build").unwrap();
    let calcs: Table = build.raw_get("calcsTab").unwrap();
    let data: Table = calcs.raw_get("mainOutput").unwrap();
    let mut scalars = BTreeMap::new();
    let mut omitted = BTreeMap::<String, usize>::new();
    for row in data.pairs::<Value, Value>() {
        let (k, v) = row.unwrap();
        let key = k.as_string().unwrap().to_str().unwrap().to_owned();
        if matches!(
            v,
            Value::Nil
                | Value::Boolean(_)
                | Value::Number(_)
                | Value::Integer(_)
                | Value::String(_)
        ) {
            scalars.insert(key, scalar_json(&v));
        } else {
            *omitted.entry(v.type_name().into()).or_default() += 1;
        }
    }
    json!({"scalars":scalars,"uncompared_nonscalar_counts":omitted})
}
fn input_identity(input: &poe_optimizer_data::source_program::SourceSessionInput) -> String {
    fn number(h: &mut Sha256, value: u64) {
        h.update(value.to_le_bytes());
    }
    fn value(h: &mut Sha256, v: &ProgramValue) {
        match v {
            ProgramValue::Nil => h.update([0]),
            ProgramValue::Boolean(v) => h.update([1, *v as u8]),
            ProgramValue::Number(v) => {
                h.update([2]);
                number(h, v.to_bits());
            }
            ProgramValue::Bytes(v) => {
                h.update([3]);
                number(h, v.len() as u64);
                h.update(v);
            }
            ProgramValue::Table(v) => {
                h.update([4]);
                number(h, v.0 as u64);
            }
            ProgramValue::Callback(v) => {
                h.update([5]);
                number(h, v.0 as u64);
            }
            ProgramValue::Closure(v) => {
                h.update([6]);
                number(h, v.0 as u64);
            }
            ProgramValue::DefinitionTable(v) => {
                h.update([7]);
                number(h, v.0 as u64);
            }
        }
    }
    fn values(h: &mut Sha256, vs: &[ProgramValue]) {
        number(h, vs.len() as u64);
        for v in vs {
            value(h, v);
        }
    }
    let mut h = Sha256::new();
    h.update(b"parser-corpus-input-v1");
    h.update(serde_json::to_vec(input.owner.definitions().unwrap()).unwrap());
    h.update(serde_json::to_vec(&input.owner.closure_prototypes()).unwrap());
    values(&mut h, &input.state.values);
    number(&mut h, input.state.tables.len() as u64);
    for table in &input.state.tables {
        number(&mut h, table.entries.len() as u64);
        for (k, v) in &table.entries {
            value(&mut h, k);
            value(&mut h, v);
        }
    }
    h.update(
        serde_json::to_vec(
            &input
                .coverage
                .iter()
                .map(|(id, c)| (id.0, c))
                .collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    assert!(
        input.class_bindings.is_empty(),
        "parser-only capture has no class instance roots"
    );
    if let Some(traversal) = &input.traversal {
        h.update([1]);
        number(&mut h, traversal.tables.len() as u64);
        for (id, t) in &traversal.tables {
            number(&mut h, id.0 as u64);
            values(&mut h, &t.order);
            h.update([t.raw_length.is_some() as u8]);
            number(&mut h, t.raw_length.unwrap_or(0) as u64);
        }
    } else {
        h.update([0]);
    }
    values(&mut h, &input.cells);
    number(&mut h, input.closures.len() as u64);
    for c in &input.closures {
        number(&mut h, c.prototype.id().0 as u64);
        number(&mut h, c.captures.len() as u64);
        for id in &c.captures {
            number(&mut h, id.0 as u64);
        }
    }
    format!("{:x}", h.finalize())
}
fn bounded_native_graph(graph: &ProgramValueGraph) -> bool {
    fn visit(
        v: &ProgramValue,
        g: &ProgramValueGraph,
        seen: &mut BTreeSet<ProgramTableId>,
        nodes: &mut usize,
        bytes: &mut usize,
        depth: usize,
    ) -> Option<()> {
        if *nodes == 0 || depth > 128 {
            return None;
        }
        *nodes -= 1;
        match v {
            ProgramValue::Nil | ProgramValue::Boolean(_) | ProgramValue::Number(_) => {}
            ProgramValue::Bytes(v) => {
                *bytes = bytes.checked_sub(v.len())?;
            }
            ProgramValue::Table(id) => {
                if !seen.insert(*id) {
                    return Some(());
                }
                let t = g.tables.get(id.0.checked_sub(1)? as usize)?;
                if t.entries.len() > 2048 {
                    return None;
                }
                // Check key sizes before making sorting strings. Walk exactly the
                // canonical visitor's key order: alias order affects DFS depth.
                let mut key_bytes = 0usize;
                for (key, _) in &t.entries {
                    match key {
                        ProgramValue::Number(_) => {}
                        ProgramValue::Bytes(v) => {
                            key_bytes = key_bytes.checked_add(v.len())?;
                        }
                        _ => return None,
                    }
                }
                if key_bytes > *bytes {
                    return None;
                }
                let mut rows = t.entries.iter().collect::<Vec<_>>();
                rows.sort_by_cached_key(|(key, _)| match key {
                    ProgramValue::Bytes(v) => format!("b:{v:?}"),
                    ProgramValue::Number(v) => format!("n:{:016x}", v.to_bits()),
                    _ => unreachable!(),
                });
                for (key, value) in rows {
                    visit(key, g, seen, nodes, bytes, depth + 1)?;
                    visit(value, g, seen, nodes, bytes, depth + 1)?;
                }
            }
            _ => return None,
        }
        Some(())
    }
    if graph.tables.len() > 8192 || graph.values.len() > 8192 {
        return false;
    }
    let mut seen = BTreeSet::new();
    let mut nodes = 8192;
    let mut bytes = 1024 * 1024;
    graph
        .values
        .iter()
        .all(|v| visit(v, graph, &mut seen, &mut nodes, &mut bytes, 0).is_some())
}

struct Prepared {
    observed: ObservedSourceSession,
    compiled: CompiledSourcePrograms,
    witnesses: Vec<ConstructorDiagnosticWitness>,
    report: Json,
}
fn prepare(lua: &Lua, primitives: &Primitives, parser: &Function) -> Prepared {
    let inner = primitives
        .captured_value(parser, "parseMod")
        .as_function()
        .unwrap()
        .clone();
    let target = primitives
        .observer
        .retain_constructor_diagnostic_target(lua, &inner)
        .unwrap();
    let scan = primitives
        .captured_value(&inner, "scan")
        .as_function()
        .unwrap()
        .clone();
    let dictionaries = parser_capture::DICTIONARIES
        .iter()
        .map(|name| {
            (
                name.to_string(),
                primitives
                    .captured_value(&inner, name)
                    .as_table()
                    .unwrap()
                    .clone(),
            )
        })
        .collect();
    let probes: Table = lua
        .load(parser_capture::TEXT)
        .set_name(format!("@{}", parser_capture::PATH))
        .eval()
        .unwrap();
    let captured = parser_capture::capture(
        lua,
        primitives,
        &scan,
        parser,
        &probes,
        &dictionaries,
        true,
        BTreeMap::new(),
    );
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let sources = captured
        .observed
        .owner()
        .source()
        .files
        .keys()
        .map(|p| {
            (
                p.clone(),
                if p == parser_capture::PATH {
                    parser_capture::TEXT.to_owned()
                } else {
                    poe_optimizer_pob::source::read_verified_text(&root, p).unwrap()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let info = inner.info();
    let mut witnesses = Vec::new();
    let mut candidate_sites = Vec::new();
    for program in &captured.lowered.catalog().data().programs {
        if Some(program.provenance.source.line as usize) != info.line_defined
            || Some(program.provenance.source.end_line as usize) != info.last_line_defined
        {
            continue;
        }
        if !info
            .source
            .as_deref()
            .unwrap()
            .ends_with(program.provenance.source.path.trim_start_matches("src/"))
        {
            continue;
        }
        let mut locations = Vec::new();
        table_locations(&serde_json::to_value(program).unwrap(), &mut locations);
        for location in locations {
            let diagnostic = target
                .inspect(
                    &sources,
                    captured.observed.constructor_observations().unwrap(),
                    captured.lowered.catalog(),
                    ConstructorDiagnosticRequest {
                        callback: program.callback,
                        expression: location,
                        continuation_line: None,
                    },
                    ConstructorDiagnosticLimits::default(),
                )
                .unwrap();
            let ConstructorDiagnostic::Observed(w) = diagnostic else {
                continue;
            };
            if w.report().instruction.word & 255 != 53 {
                continue;
            }
            let eligible = !w.report().template_rows.is_empty()
                && w.report().template_rows.iter().all(|r| {
                    matches!(
                        (&r.key, &r.value),
                        (
                            ConstructorTemplateValue::Bytes(_),
                            ConstructorTemplateValue::SelfMarker
                        )
                    )
                });
            candidate_sites.push(json!({"callback":program.callback,"location":location,"self_marker_template":eligible,"diagnostic":w.report()}));
            if eligible {
                match attach_reserved_string_templates(&sources, captured.lowered.clone(), &[&w]) {
                    Ok(_) => {
                        candidate_sites.last_mut().unwrap()["admitted"] = true.into();
                        witnesses.push(*w);
                    }
                    Err(error) => {
                        candidate_sites.last_mut().unwrap()["admitted"] = false.into();
                        candidate_sites.last_mut().unwrap()["admission_error"] =
                            error.to_string().into();
                    }
                }
            }
        }
    }
    assert!(
        !witnesses.is_empty(),
        "actual original parser has reserved templates"
    );
    let admitted = attach_reserved_string_templates(
        &sources,
        captured.lowered.clone(),
        &witnesses.iter().collect::<Vec<_>>(),
    )
    .unwrap();
    let compiled = CompiledSourcePrograms::new(admitted.catalog()).unwrap();
    let input = captured.observed.input();
    let state_sha256 = input_identity(input);
    let report = json!({"state_sha256":state_sha256, "candidate_templates":candidate_sites,
        "admitted_template_count":witnesses.len(), "capture_inventory":captured.inventory});
    Prepared {
        observed: captured.observed,
        compiled,
        witnesses,
        report,
    }
}
fn replay(
    lua: &Lua,
    primitives: &Primitives,
    capture: &Table,
    watch_factory: &state_watch::StateWatchFactory,
) -> Json {
    // This corpus measures reference semantics, not warmed JIT performance.
    lua.load("jit.off(); jit.flush()").exec().unwrap();
    let source_report: Table = capture
        .raw_get::<Function>("report")
        .unwrap()
        .call(())
        .unwrap();
    let parser: Function = capture
        .raw_get::<Function>("parser")
        .unwrap()
        .call(())
        .unwrap();
    let cache: Table = lua
        .globals()
        .raw_get::<Table>("modLib")
        .unwrap()
        .raw_get("parseModCache")
        .unwrap();
    assert_eq!(
        lua.globals().raw_get::<Value>("foo").unwrap(),
        Value::Nil,
        "logging branch is outside this parser protocol"
    );
    let mut prepared = prepare(lua, primitives, &parser);
    let mut watch = watch_factory.watch(&parser, &cache).unwrap();
    let mut generations = vec![prepared.report.clone()];
    let mut changed: Option<String> = None;
    let mut detail_bytes = 0usize;
    let events: Table = source_report.raw_get("events").unwrap();
    let mut occurrences = Vec::new();
    let mut inputs = Vec::<(Vec<ProgramValue>, Vec<usize>)>::new();
    let mut ids = BTreeMap::new();
    for entry in events.sequence_values::<Table>() {
        let e = entry.unwrap();
        let args = vec![
            descriptor(lua, &e.raw_get::<Table>("line").unwrap()),
            descriptor(lua, &e.raw_get::<Table>("combined").unwrap()),
        ];
        let encoded = Json::Array(args.iter().map(scalar_json).collect());
        let identity = serde_json::to_string(&encoded).unwrap();
        let next = inputs.len();
        let index = *ids.entry(identity).or_insert_with(|| {
            inputs.push((observation::capture(&args).values, Vec::new()));
            next
        });
        let ordinal = occurrences.len() + 1;
        assert_eq!(e.raw_get::<usize>("ordinal").unwrap(), ordinal);
        inputs[index].1.push(ordinal);
        // Convert argument descriptors separately to preserve arbitrary byte strings.
        e.raw_set("line", Value::Nil).unwrap();
        e.raw_set("combined", Value::Nil).unwrap();
        let mut event: Json = lua.from_value(Value::Table(e)).unwrap();
        event["input_index"] = index.into();
        occurrences.push(event);
    }
    source_report.raw_set("events", Value::Nil).unwrap();
    let source_metadata: Json = lua.from_value(Value::Table(source_report)).unwrap();
    assert!(!occurrences.is_empty());
    let mut outcomes = Vec::new();
    let mut counts = BTreeMap::<String, usize>::new();
    let before_output = output(lua);
    for (index, (stored_args, occurrences_for_input)) in inputs.iter().enumerate() {
        let args = source_arguments(lua, stored_args);
        if let Some(reason) = changed.take() {
            for w in &prepared.witnesses {
                w.verify_unchanged().unwrap();
            }
            prepared = prepare(lua, primitives, &parser);
            watch = watch_factory.watch(&parser, &cache).unwrap();
            let mut generation = prepared.report.clone();
            generation["reason"] = reason.into();
            generations.push(generation);
        }
        let key = args[0].clone();
        cache.raw_set(key.clone(), Value::Nil).unwrap();
        let (mut session, roots) = prepared
            .compiled
            .session_from_input(prepared.observed.input(), limits())
            .unwrap();
        let root = |name: &str| roots[prepared.observed.root_index(name).unwrap()].clone();
        let native_args = session.borrow(&observation::capture(&args)).unwrap();
        let nil = session
            .borrow(&ProgramValueGraph {
                values: vec![ProgramValue::Nil],
                tables: vec![],
            })
            .unwrap()
            .remove(0);
        session
            .invoke_callable(
                &root("probe.replace"),
                &[root("public.cache"), native_args[0].clone(), nil],
            )
            .unwrap();
        let actual = parser.call::<MultiValue>(MultiValue::from_vec(args.clone()));
        let native = session.invoke_callable(&root("original.parser"), &native_args);
        let mut row = json!({"input_index":index,"arguments":args.iter().map(scalar_json).collect::<Vec<_>>(),"occurrences":occurrences_for_input,"capture_generation":generations.len()-1,"replay_argc":2,"original_arity_observed":false});
        let status = match (actual, native) {
            (Ok(actual), Ok(native)) => {
                let actual = actual.into_vec();
                row["miss_arity"] = json!({"source":actual.len(),"native":native.len()});
                let miss_arity_equal = actual.len() == native.len();
                let actual_row: Value = cache.raw_get(key.clone()).unwrap();
                let native_row = session
                    .invoke_callable(
                        &root("probe.lookup"),
                        &[root("public.cache"), native_args[0].clone()],
                    )
                    .unwrap()
                    .remove(0);
                let mut source_values = vec![actual_row];
                source_values.extend(actual);
                let mut native_values = vec![native_row];
                native_values.extend(native);
                let hit = parser
                    .call::<MultiValue>(MultiValue::from_vec(args.clone()))
                    .unwrap()
                    .into_vec();
                let native_hit = session.invoke_callable(&root("original.parser"), &native_args);
                match native_hit {
                    Err(error) => {
                        row["frontier"] = error_json(prepared.compiled.catalog().owner(), &error);
                        "native_hit_failed"
                    }
                    Ok(native_hit) => {
                        row["hit_arity"] = json!({"source":hit.len(),"native":native_hit.len()});
                        let hit_arity_equal = hit.len() == native_hit.len();
                        source_values.push(cache.raw_get(key.clone()).unwrap());
                        source_values.extend(hit);
                        native_values.extend(
                            session
                                .invoke_callable(
                                    &root("probe.lookup"),
                                    &[root("public.cache"), native_args[0].clone()],
                                )
                                .unwrap(),
                        );
                        native_values.extend(native_hit);
                        if let Some(source_graph) =
                            capture_plain(&source_values).filter(bounded_native_graph)
                        {
                            let expected = observation::canonical(&source_graph);
                            match session.snapshot(&native_values) {
                                Ok(snapshot) => {
                                    let scalar = |v: &ProgramValue| {
                                        matches!(
                                            v,
                                            ProgramValue::Nil
                                                | ProgramValue::Boolean(_)
                                                | ProgramValue::Number(_)
                                                | ProgramValue::Bytes(_)
                                                | ProgramValue::Table(_)
                                        )
                                    };
                                    let graph = snapshot.graph();
                                    let represented = graph.values.iter().all(scalar)
                                        && graph.tables.iter().all(|t| {
                                            t.entries.iter().all(|(k, v)| {
                                                matches!(
                                                    k,
                                                    ProgramValue::Number(_)
                                                        | ProgramValue::Bytes(_)
                                                ) && scalar(v)
                                            })
                                        });
                                    if represented && bounded_native_graph(graph) {
                                        let observed = observation::canonical(graph);
                                        row["reference_history"] = expected.clone();
                                        row["native_history"] = observed.clone();
                                        if expected == observed
                                            && miss_arity_equal
                                            && hit_arity_equal
                                        {
                                            "matched_miss_and_hit"
                                        } else {
                                            "parity_mismatch"
                                        }
                                    } else {
                                        row["comparison_error"] = "native graph is outside the bounded plain comparison domain".into();
                                        "comparison_unsupported"
                                    }
                                }
                                Err(error) => {
                                    row["comparison_error"] =
                                        error_json(prepared.compiled.catalog().owner(), &error);
                                    "comparison_unsupported"
                                }
                            }
                        } else {
                            "comparison_unsupported"
                        }
                    }
                }
            }
            (Ok(actual), Err(error)) => {
                let actual = actual.into_vec();
                row["source_result_count"] = actual.len().into();
                if let Some(graph) = capture_plain(&actual).filter(bounded_native_graph) {
                    row["reference_result"] = observation::canonical(&graph);
                }
                row["frontier"] = error_json(prepared.compiled.catalog().owner(), &error);
                match error.kind {
                    ProgramRuntimeErrorKind::UnsupportedCapability => "native_unsupported",
                    ProgramRuntimeErrorKind::ResourceBound => "native_resource_bound",
                    _ => "unexpected_native_error",
                }
            }
            (Err(source_error), Err(error)) => {
                row["source_error"] = source_error.to_string().into();
                row["frontier"] = error_json(prepared.compiled.catalog().owner(), &error);
                match error.kind {
                    ProgramRuntimeErrorKind::Source => {
                        "source_error_native_source_uncompared_prefix"
                    }
                    ProgramRuntimeErrorKind::UnsupportedCapability => {
                        "source_error_native_unsupported"
                    }
                    ProgramRuntimeErrorKind::ResourceBound => "source_error_native_resource_bound",
                    ProgramRuntimeErrorKind::InvalidInput => "unexpected_native_error",
                }
            }
            (Err(source_error), Ok(_)) => {
                row["source_error"] = source_error.to_string().into();
                "unexpected_native_success"
            }
        };
        watch.restore_cache_bindings().unwrap();
        changed = watch.changed().unwrap();
        row["source_state_changed_after_probe"] = json!(changed);
        // All complete graphs are compared before this output-detail budget. A
        // digest never substitutes for a comparison that did not execute.
        for field in ["reference_result", "reference_history", "native_history"] {
            if let Some(graph) = row.get_mut(field) {
                let bytes = serde_json::to_vec(graph).unwrap();
                let sha256 = hash(&bytes);
                if detail_bytes.saturating_add(bytes.len()) > 16 * 1024 * 1024 {
                    *graph = json!({"sha256":sha256,"serialized_bytes":bytes.len(),"detail_omitted_due_to_report_limit":true});
                } else {
                    detail_bytes += bytes.len();
                }
            }
        }
        row["status"] = status.into();
        row["native_steps"] = session.steps().into();
        row["native_pattern_steps"] = session.pattern_steps().into();
        *counts.entry(status.into()).or_default() += 1;
        outcomes.push(row);
        if (index + 1) % 50 == 0 {
            eprintln!(
                "parser breadth {}/{} inputs {:?}",
                index + 1,
                inputs.len(),
                counts
            );
        }
    }
    assert_eq!(
        output(lua),
        before_output,
        "parser replay changed retained public output"
    );
    for w in &prepared.witnesses {
        w.verify_unchanged().unwrap()
    }
    json!({"schema_version":1,"scope":"actual original import calls; fresh-native cache-miss/hit component replay; source semantic-state guard versions coherent recapture when changed; not original import execution in native",
        "complete_public_parser":false,"complete_native_build":false,"source":source_metadata,"occurrences":occurrences,"input_count":inputs.len(),"occurrence_count":occurrences.len(),"counts":counts,"outcomes":outcomes,"capture_generations":generations,"full_graph_detail_bytes":detail_bytes,"final_source_state_change":changed,"parameter_values_observed_not_original_call_arity":true,"public_output_unchanged_by_replay":true})
}
fn error_json(
    owner: &poe_optimizer_data::source_program::SourceProgramOwner,
    error: &ProgramRuntimeError,
) -> Json {
    json!({"kind":format!("{:?}",error.kind),"message":error.message,"callback":error.callback,"location":error.location,"declaration":error.callback.and_then(|id|owner.callback(id)).map(|c|&c.kind)})
}
fn child(root: &Path, destination: &Path, entry: &Json) {
    let xml_path = root
        .join("tests/fixtures/builds/breadth-20260908")
        .join(entry["xml"].as_str().unwrap());
    let bytes = fs::read(&xml_path).unwrap();
    assert_eq!(hash(&bytes), entry["xml_sha256"]);
    let xml = std::str::from_utf8(&bytes).unwrap();
    let scratch = tempfile::tempdir().unwrap();
    let vendor = root.join("vendor/path-of-building-poe2");
    let control = source::observe_with_hook(
        &vendor,
        scratch.path(),
        xml,
        None,
        false,
        Some(&|lua| Ok(output(lua))),
    )
    .unwrap();
    let primitives = RefCell::new(None);
    let watch_factory = RefCell::new(None);
    let observer = RefCell::new(None::<Table>);
    let capture = RefCell::new(None::<Table>);
    let trace = source::observe_with_build_hook(
        &vendor,
        scratch.path(),
        xml,
        None,
        false,
        Some(&|lua| {
            primitives.replace(Some(Primitives::before_source_with_closures(lua)?));
            watch_factory.replace(Some(state_watch::StateWatchFactory::before_source(lua)?));
            observer.replace(Some(
                lua.load(OBSERVER)
                    .set_name(format!("@{OBSERVER_PATH}"))
                    .eval()?,
            ));
            Ok(())
        }),
        Some(&|_| {
            let c: Table = observer
                .borrow()
                .as_ref()
                .unwrap()
                .raw_get::<Function>("start")?
                .call(())?;
            let finish = c.raw_get("finish")?;
            capture.replace(Some(c));
            Ok(finish)
        }),
        Some(&|lua| -> Result<Json, RuntimeError> {
            assert_eq!(
                output(lua),
                control["additional_observation"],
                "import call hook changed public scalar outputs"
            );
            Ok(replay(
                lua,
                primitives.borrow().as_ref().unwrap(),
                capture.borrow().as_ref().unwrap(),
                watch_factory.borrow().as_ref().unwrap(),
            ))
        }),
    )
    .unwrap();
    let parsed_xml = roxmltree::Document::parse(xml).unwrap();
    let items_section = parsed_xml
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Items"))
        .unwrap();
    let saved_ids = items_section
        .children()
        .filter(|n| n.has_tag_name("Item"))
        .map(|n| n.attribute("id").unwrap().parse::<u64>().unwrap())
        .collect::<BTreeSet<_>>();
    let observed_items = &trace["additional_observation"]["source"]["items"];
    let final_ids = observed_items
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|item| {
            item["final_item_ids"]
                .as_array()
                .map(Vec::as_slice)
                .unwrap_or(&[])
        })
        .map(|id| id.as_u64().unwrap())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        final_ids, saved_ids,
        "actual final item identities must cover every saved item"
    );
    let cases = trace["additional_observation"]["outcomes"]
        .as_array()
        .unwrap();
    let occurrences = trace["additional_observation"]["occurrences"]
        .as_array()
        .unwrap();
    assert_eq!(
        cases.len(),
        trace["additional_observation"]["input_count"]
            .as_u64()
            .unwrap() as usize
    );
    assert_eq!(
        occurrences.len(),
        trace["additional_observation"]["occurrence_count"]
            .as_u64()
            .unwrap() as usize
    );
    let mut accounted = BTreeSet::new();
    for (index, case) in cases.iter().enumerate() {
        assert_eq!(case["input_index"], index);
        for ordinal in case["occurrences"].as_array().unwrap() {
            let ordinal = ordinal.as_u64().unwrap() as usize;
            assert!(accounted.insert(ordinal), "occurrence counted twice");
            assert_eq!(occurrences[ordinal - 1]["input_index"], index);
        }
    }
    assert_eq!(accounted, (1..=occurrences.len()).collect());
    assert_eq!(trace["selected"], control["selected"]);
    assert_eq!(trace["final"], control["final"]);
    assert_eq!(fs::read(xml_path).unwrap(), bytes);
    let report = json!({"input":entry["id"],"xml_sha256":hash(&bytes),"manifest_sha256":hash(&fs::read(root.join(MANIFEST)).unwrap()),"observer_sha256":hash(OBSERVER.as_bytes()),"unhooked_source_control":{"selected":control["selected"],"public_output":control["additional_observation"]},"trace":trace});
    let name = format!("{}.json", entry["id"].as_str().unwrap());
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination.join(name))
        .unwrap();
    f.write_all(&serde_json::to_vec_pretty(&report).unwrap())
        .unwrap();
    let counts = &report["trace"]["additional_observation"]["counts"];
    for status in [
        "parity_mismatch",
        "unexpected_native_error",
        "unexpected_native_success",
        "native_hit_failed",
    ] {
        assert!(
            counts[status].as_u64().unwrap_or(0) == 0,
            "{status}: {counts}"
        );
    }
}
#[test]
fn all_original_import_parser_inputs_have_native_outcomes() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    let manifest: Json = serde_json::from_slice(&fs::read(root.join(MANIFEST)).unwrap()).unwrap();
    let builds = manifest["builds"].as_array().unwrap();
    assert_eq!(builds.len(), 5);
    let scratch = tempfile::tempdir().unwrap();
    let destination = std::env::var_os(OUTPUT)
        .map(PathBuf::from)
        .unwrap_or_else(|| scratch.path().to_owned());
    assert!(destination.is_absolute());
    fs::create_dir_all(&destination).unwrap();
    if let Ok(id) = std::env::var(CHILD) {
        let entry = builds.iter().find(|b| b["id"] == id).unwrap();
        child(&root, &destination, entry);
        return;
    }
    for entry in builds {
        let id = entry["id"].as_str().unwrap();
        let log = fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(destination.join(format!("{id}.log")))
            .unwrap();
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, id)
            .env(OUTPUT, &destination)
            .current_dir(root.join("vendor/path-of-building-poe2/src"))
            .stdout(Stdio::from(log.try_clone().unwrap()))
            .stderr(Stdio::from(log))
            .spawn()
            .unwrap();
        let started = Instant::now();
        loop {
            if let Some(status) = child.try_wait().unwrap() {
                assert!(status.success(), "breadth child {id}; see {destination:?}");
                break;
            }
            if started.elapsed() > Duration::from_secs(1200) {
                child.kill().unwrap();
                child.wait().unwrap();
                panic!("breadth child deadline {id}")
            }
            std::thread::sleep(Duration::from_millis(100));
        }
    }
}

#[test]
fn comparison_guard_rejects_behavior_and_bounds_without_running_it() {
    let lua = Lua::new();
    let table: Table = lua
        .load(
            "local t = setmetatable({}, {__index=function() error('must not run') end}); return t",
        )
        .eval()
        .unwrap();
    assert!(!plain(&[Value::Table(table)]));
    let wide = lua.create_table().unwrap();
    for i in 1..=2049 {
        wide.raw_set(i, i).unwrap();
    }
    assert!(!plain(&[Value::Table(wide)]));
    let deep = lua.create_table().unwrap();
    let mut cursor = deep.clone();
    for _ in 0..130 {
        let next = lua.create_table().unwrap();
        cursor.raw_set(1, next.clone()).unwrap();
        cursor = next;
    }
    assert!(!plain(&[Value::Table(deep)]));
    let cyclic = lua.create_table().unwrap();
    cyclic.raw_set("self", cyclic.clone()).unwrap();
    assert!(plain(&[Value::Table(cyclic)]));
}
#[test]
fn observed_parameter_identity_preserves_nil_false_and_binary_text() {
    let lua = Lua::new();
    let text = Value::String(lua.create_string([0, 255, 65]).unwrap());
    assert_ne!(
        scalar_json(&Value::Nil),
        scalar_json(&Value::Boolean(false))
    );
    assert_eq!(
        scalar_json(&text),
        json!({"kind":"string","bytes":[0,255,65]})
    );
    assert_eq!(
        scalar_json(&Value::Number(-0.0))["bits"],
        "8000000000000000"
    );
}

#[test]
fn streaming_graph_capture_keeps_aliases_without_retaining_lua_row_handles() {
    let lua = Lua::new();
    let table = lua.create_table().unwrap();
    for i in 1..=1500 {
        let child = lua.create_table().unwrap();
        child.raw_set("value", i).unwrap();
        table.raw_set(i, child).unwrap();
    }
    let graph = capture_plain(&[Value::Table(table.clone()), Value::Table(table)]).unwrap();
    assert_eq!(graph.tables.len(), 1501);
    assert_eq!(graph.values[0], graph.values[1]);
}

#[test]
fn native_comparison_preflight_bounds_depth_and_rows() {
    let mut graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default(); 130],
    };
    for i in 0..129 {
        graph.tables[i].entries.push((
            ProgramValue::Number(1.0),
            ProgramValue::Table(ProgramTableId(i as u32 + 2)),
        ));
    }
    assert!(!bounded_native_graph(&graph));
    graph.tables[0].entries.clear();
    assert!(bounded_native_graph(&graph));
    graph.tables[0].entries = (0..2049)
        .map(|v| (ProgramValue::Number(v as f64), ProgramValue::Boolean(true)))
        .collect();
    assert!(!bounded_native_graph(&graph));
}

#[test]
fn comparison_depth_follows_canonical_alias_order() {
    let mut graph = ProgramValueGraph {
        values: vec![ProgramValue::Table(ProgramTableId(1))],
        tables: vec![ProgramTable::default(); 132],
    };
    // Reverse storage order would visit every child shallowly before seeing
    // its incoming chain edge. Canonical key order sees the complete chain.
    graph.tables[0].entries = (2..=132)
        .rev()
        .map(|id| {
            (
                ProgramValue::Number(id as f64),
                ProgramValue::Table(ProgramTableId(id)),
            )
        })
        .collect();
    for id in 2..132 {
        graph.tables[id as usize - 1].entries.push((
            ProgramValue::Number(1.0),
            ProgramValue::Table(ProgramTableId(id + 1)),
        ));
    }
    assert!(!bounded_native_graph(&graph));
}
