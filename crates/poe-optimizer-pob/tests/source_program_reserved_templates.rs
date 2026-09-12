//! Original complete source bodies; attachment is explicit and rooted source
//! comparisons do not claim behavior after the original prototype is collected.
#[allow(dead_code)]
#[path = "support/source_program_observation.rs"]
mod observation;
#[allow(dead_code)]
#[path = "support/source_program_warm.rs"]
mod warm;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::{item_loading::ItemLoadingSource, source_program::*};
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{
    SourceProgramExtraction, attach_reserved_string_templates, capture::*,
    lower_observed_closures_and_constructors_from_sources,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_reserved_templates.lua";
const TEXT: &str = include_str!("support/source_program_reserved_templates.lua");
struct Fixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    sources: BTreeMap<String, String>,
    observed: ObservedSourceContext,
    base: SourceProgramExtraction,
    witnesses: Vec<ConstructorDiagnosticWitness>,
}
impl Fixture {
    fn new() -> Self {
        // Isolated trusted fixture host, matching the full reflection source gates.
        let lua = unsafe { Lua::unsafe_new() };
        lua.load("jit.off();jit.flush()").exec().unwrap();
        let observer = SourceClosureObserver::capture_before_source_with_closures(
            &lua,
            SourceTableRuntimeProfile::luajit21_x64_single(),
        )
        .unwrap();
        let exports: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions: BTreeMap<String, Function> = exports.pairs().map(Result::unwrap).collect();
        assert_eq!(functions.len(), 8);
        let targets = ["make", "plain", "ordered", "effect_make"].map(|name| {
            observer
                .retain_constructor_diagnostic_target(&lua, &functions[name])
                .unwrap()
        });
        let sources = BTreeMap::from([(PATH.into(), TEXT.into())]);
        let source = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: BTreeMap::from([(
                PATH.into(),
                format!("{:x}", Sha256::digest(TEXT.as_bytes())),
            )]),
            construction_spans: Default::default(),
            module_order: vec![PATH.into()],
        };
        let observed = observer
            .observe_with_context(
                &lua,
                &sources,
                source,
                &functions,
                SourceCaptureContext {
                    capture_iteration: true,
                    ..Default::default()
                },
            )
            .unwrap();
        let base = lower_observed_closures_and_constructors_from_sources(
            &sources,
            observed.owner(),
            observed.closure_observations().unwrap(),
            observed.constructor_observations().unwrap(),
        )
        .unwrap();
        assert!(base.unsupported().is_empty(), "{:?}", base.unsupported());
        assert_eq!(base.catalog().closure_creations().unwrap().sites.len(), 1);
        assert_eq!(base.catalog().constructors().unwrap().sites.len(), 2);
        assert_eq!(base.constructor_unsupported().len(), 4);
        let witnesses = targets
            .into_iter()
            .zip([
                (
                    "make",
                    r#"{ alpha = a, ["sp ace"] = b, ["\255\000"] = c, ... }"#,
                ),
                ("plain", "{ one = a, two = b, three = c }"),
                ("ordered", "{ left = a, middle = b, right = c }"),
                (
                    "effect_make",
                    "{ first = state, second = state, tail(state, ...) }",
                ),
            ])
            .map(|(target, (name, selected))| {
                let callback = observed.callbacks()[name];
                let program = base.catalog().for_callback(callback).unwrap();
                let span = &program.provenance.source;
                let body = TEXT
                    .split_inclusive('\n')
                    .skip(span.line as usize - 1)
                    .take((span.end_line - span.line + 1) as usize)
                    .collect::<String>();
                let function = &body[program.provenance.function_start as usize
                    ..program.provenance.function_end as usize];
                // Text only selects the request. The production query authenticates
                // the complete lexical inventory and actual Function/TDUP identity.
                let start = function.find(selected).unwrap() as u32;
                let request = ConstructorDiagnosticRequest {
                    callback,
                    expression: SourceProgramLocation {
                        start,
                        end: start + selected.len() as u32,
                    },
                    continuation_line: None,
                };
                match target
                    .inspect(
                        &sources,
                        observed.constructor_observations().unwrap(),
                        base.catalog(),
                        request,
                        Default::default(),
                    )
                    .unwrap()
                {
                    ConstructorDiagnostic::Observed(witness) => *witness,
                    ConstructorDiagnostic::Unavailable(reason) => panic!("{name}: {reason}"),
                }
            })
            .collect();
        Self {
            lua,
            functions,
            sources,
            observed,
            base,
            witnesses,
        }
    }
    fn attach(&self) -> SourceProgramExtraction {
        attach_reserved_string_templates(
            &self.sources,
            self.base.clone(),
            &self.witnesses.iter().collect::<Vec<_>>(),
        )
        .unwrap()
    }
    fn session(&self, attached: bool) -> ProgramSession {
        let extraction = if attached {
            self.attach()
        } else {
            self.base.clone()
        };
        CompiledSourcePrograms::new(extraction.catalog())
            .unwrap()
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap()
            .0
    }
    fn value(&self, bytes: &[u8]) -> Value {
        Value::String(self.lua.create_string(bytes).unwrap())
    }
    fn source(&self, name: &str, arguments: &[Value]) -> Vec<Value> {
        self.functions[name]
            .call::<MultiValue>(MultiValue::from_vec(arguments.to_vec()))
            .unwrap()
            .into_vec()
    }
    fn native(
        &self,
        session: &mut ProgramSession,
        name: &str,
        arguments: &[SessionValue],
    ) -> Vec<SessionValue> {
        session
            .invoke(self.observed.callbacks()[name], arguments)
            .unwrap()
    }
    fn call(
        &self,
        session: &mut ProgramSession,
        name: &str,
        arguments: &[Value],
    ) -> (Vec<Value>, Vec<SessionValue>) {
        let input = import(session, arguments);
        let expected = self.source(name, arguments);
        let actual = self.native(session, name, &input);
        equal(session, &expected, &actual);
        (expected, actual)
    }
    fn order(&self, session: &mut ProgramSession, source: &Value, native: &SessionValue) {
        let mut control = Value::Nil;
        let mut native_control = import(session, &[Value::Nil]).remove(0);
        for _ in 0..8 {
            let expected = self.source("step", &[source.clone(), control.clone()]);
            let actual = self.native(session, "step", &[native.clone(), native_control]);
            equal(session, &expected, &actual);
            if matches!(expected.first(), None | Some(Value::Nil)) {
                return;
            }
            control = expected[0].clone();
            native_control = actual[0].clone();
        }
        panic!("bounded fixture iteration did not terminate");
    }
}
// This local observer supports byte-string keys and preserves aliases. It does
// not infer iteration order from its sorted transport representation.
fn graph(values: &[Value]) -> ProgramValueGraph {
    fn visit(
        value: &Value,
        graph: &mut ProgramValueGraph,
        ids: &mut BTreeMap<usize, ProgramTableId>,
    ) -> ProgramValue {
        match value {
            Value::Nil => ProgramValue::Nil,
            Value::Boolean(value) => ProgramValue::Boolean(*value),
            Value::Integer(value) => ProgramValue::Number(*value as f64),
            Value::Number(value) => ProgramValue::Number(*value),
            Value::String(value) => ProgramValue::Bytes(value.as_bytes().to_vec()),
            Value::Table(table) => {
                let pointer = table.to_pointer() as usize;
                if let Some(id) = ids.get(&pointer) {
                    return ProgramValue::Table(*id);
                }
                let id = ProgramTableId(graph.tables.len() as u32 + 1);
                ids.insert(pointer, id);
                graph.tables.push(ProgramTable::default());
                let mut rows = table
                    .clone()
                    .pairs::<Value, Value>()
                    .map(Result::unwrap)
                    .collect::<Vec<_>>();
                rows.sort_by_key(|(key, _)| match key {
                    Value::String(value) => (1, value.as_bytes().to_vec()),
                    Value::Integer(value) => (0, (*value as f64).to_bits().to_be_bytes().to_vec()),
                    Value::Number(value) => (0, value.to_bits().to_be_bytes().to_vec()),
                    _ => panic!("fixture key kind"),
                });
                let entries = rows
                    .into_iter()
                    .map(|(key, value)| (visit(&key, graph, ids), visit(&value, graph, ids)))
                    .collect();
                graph.tables[id.0 as usize - 1].entries = entries;
                ProgramValue::Table(id)
            }
            _ => panic!("fixture raw graph value kind"),
        }
    }
    let mut graph = ProgramValueGraph::default();
    let mut ids = BTreeMap::new();
    graph.values = values
        .iter()
        .map(|value| visit(value, &mut graph, &mut ids))
        .collect();
    graph
}
fn import(session: &mut ProgramSession, values: &[Value]) -> Vec<SessionValue> {
    session
        .import_with_coverage(&graph(values), &ProgramTableCoverage::new())
        .unwrap()
}
fn equal(session: &mut ProgramSession, source: &[Value], native: &[SessionValue]) {
    assert_eq!(
        observation::canonical(&graph(source)),
        observation::canonical(session.snapshot(native).unwrap().graph())
    );
}
#[test]
fn exact_live_attachment_preserves_facets_and_rejects_foreign_catalog_or_changed_template() {
    let fixture = Fixture::new();
    let attached = fixture.attach();
    assert!(attached.constructor_unsupported().is_empty());
    assert!(attached.unsupported().is_empty());
    assert_eq!(attached.catalog().constructors().unwrap().sites.len(), 6);
    assert_eq!(
        attached.catalog().closure_creations(),
        fixture.base.catalog().closure_creations()
    );
    assert!(!std::ptr::eq(
        attached.catalog().data(),
        fixture.base.catalog().data()
    ));
    assert_eq!(fixture.base.constructor_unsupported().len(), 4);
    let new_catalog = lower_observed_closures_and_constructors_from_sources(
        &fixture.sources,
        fixture.observed.owner(),
        fixture.observed.closure_observations().unwrap(),
        fixture.observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert!(
        attach_reserved_string_templates(&fixture.sources, new_catalog, &[&fixture.witnesses[0]])
            .unwrap_err()
            .to_string()
            .contains("different catalog")
    );
    assert!(
        attach_reserved_string_templates(
            &fixture.sources,
            fixture.base.clone(),
            &[&fixture.witnesses[0], &fixture.witnesses[0]]
        )
        .is_err()
    );
    let witness = &fixture.witnesses[0];
    let template = witness.template().unwrap();
    let ConstructorTemplateValue::Bytes(key) = &witness.report().template_rows[0].key else {
        panic!()
    };
    let key = fixture.lua.create_string(key).unwrap();
    template.raw_set(key.clone(), false).unwrap();
    assert!(
        attach_reserved_string_templates(&fixture.sources, fixture.base.clone(), &[witness])
            .is_err()
    );
    template.raw_set(key, template.clone()).unwrap();
    witness.verify_unchanged().unwrap();
    // Private retained reflection, rather than the currently rebound globals,
    // supplies the proof; original source consumers capture original_next.
    fixture.lua.load("next=function() error('replacement next') end; require('jit.util').funck=function() error('replacement reflection') end").exec().unwrap();
    fixture.attach();
    let mut session = fixture.session(true);
    let arguments = [Value::Integer(1), Value::Boolean(false), Value::Integer(3)];
    let (source, native) = fixture.call(&mut session, "make", &arguments);
    fixture.order(&mut session, &source[0], &native[0]);
}
#[test]
fn reserved_order_survives_deletion_reinsertion_aliases_and_rooted_source_gc() {
    let fixture = Fixture::new();
    let shared = fixture.lua.create_table().unwrap();
    shared.raw_set("child", 9).unwrap();
    let arguments = [
        Value::Table(shared.clone()),
        Value::Table(shared),
        Value::Nil,
    ];
    let mut session = fixture.session(true);
    let (source, native) = fixture.call(&mut session, "make", &arguments);
    fixture.order(&mut session, &source[0], &native[0]);
    for (key, value) in [
        (b"alpha".as_slice(), Value::Nil),
        (b"sp ace".as_slice(), Value::Nil),
        (b"\xff\0".as_slice(), Value::Boolean(false)),
        (b"alpha".as_slice(), Value::Integer(42)),
        (b"sp ace".as_slice(), Value::Integer(17)),
    ] {
        let key = fixture.value(key);
        let mut input = vec![native[0].clone()];
        input.extend(import(&mut session, &[key.clone(), value.clone()]));
        let expected = fixture.source("write", &[source[0].clone(), key, value]);
        let actual = fixture.native(&mut session, "write", &input);
        equal(&mut session, &expected, &actual);
        fixture.lua.gc_collect().unwrap();
        fixture.order(&mut session, &source[0], &native[0]);
    }
    for witness in &fixture.witnesses {
        witness.verify_unchanged().unwrap();
    }
    let expected = fixture.source("copy", &[source[0].clone()]);
    let actual = fixture.native(&mut session, "copy", &[native[0].clone()]);
    equal(&mut session, &expected, &actual);
    // The child creation facet remains executable in the newly built catalog.
    let input = import(&mut session, &[Value::Integer(23)]);
    let child = fixture.native(&mut session, "factory", &input).remove(0);
    let result = session.invoke_callable(&child, &[]).unwrap();
    equal(&mut session, &[Value::Integer(23)], &result);
}
#[test]
fn absence_of_attachment_positive_tail_and_unreserved_mutation_keep_traversal_frontiers() {
    let fixture = Fixture::new();
    let mut baseline = fixture.session(false);
    let (_, unproven) = fixture.call(
        &mut baseline,
        "make",
        &[Value::Integer(1), Value::Boolean(false), Value::Integer(3)],
    );
    let error = baseline
        .invoke(fixture.observed.callbacks()["step"], &[unproven[0].clone()])
        .unwrap_err();
    assert_eq!(error.kind, ProgramRuntimeErrorKind::UnsupportedCapability);
    for tail in [
        vec![],
        vec![Value::Nil],
        vec![Value::Nil, Value::Nil],
        vec![Value::Integer(4)],
    ] {
        let mut session = fixture.session(true);
        let state = fixture.lua.create_table().unwrap();
        state.raw_set("calls", 0).unwrap();
        let mut arguments = vec![Value::Table(state.clone())];
        arguments.extend(tail.clone());
        let input = import(&mut session, &arguments);
        let expected = fixture.source("effect_make", &arguments);
        assert_eq!(state.raw_get::<i64>("calls").unwrap(), 1);
        let actual = session.invoke(fixture.observed.callbacks()["effect_make"], &input);
        if tail.is_empty() {
            let actual = actual.unwrap();
            equal(&mut session, &expected, &actual);
            fixture.order(&mut session, &expected[0], &actual[0]);
        } else {
            let error = actual.unwrap_err();
            assert_eq!(
                error.kind,
                ProgramRuntimeErrorKind::UnsupportedCapability,
                "{error}"
            );
            assert!(error.message.contains("positive tail"), "{error}");
            // Full RHS effects happened before the Unsupported boundary. There
            // is no native result table or guessed bulk write to compare.
            equal(&mut session, &[Value::Table(state)], &input[..1]);
        }
    }
    // An error inside the actual RHS helper must propagate before the positive
    // tail capability boundary. The visible prefix still mutates the same input.
    let mut failing = fixture.session(true);
    let state = fixture.lua.create_table().unwrap();
    state.raw_set("calls", 0).unwrap();
    state.raw_set("fail", true).unwrap();
    let arguments = [Value::Table(state.clone()), Value::Nil];
    let input = import(&mut failing, &arguments);
    let source_error = fixture.functions["effect_make"]
        .call::<MultiValue>(MultiValue::from_vec(arguments.to_vec()))
        .unwrap_err();
    assert!(
        source_error.to_string().contains("arithmetic"),
        "{source_error}"
    );
    let native_error = failing
        .invoke(fixture.observed.callbacks()["effect_make"], &input)
        .unwrap_err();
    assert_eq!(
        native_error.kind,
        ProgramRuntimeErrorKind::Source,
        "{native_error}"
    );
    assert_eq!(state.raw_get::<i64>("calls").unwrap(), 1);
    equal(&mut failing, &[Value::Table(state)], &input[..1]);
    let mut session = fixture.session(true);
    let (source, native) = fixture.call(
        &mut session,
        "plain",
        &[Value::Integer(1), Value::Integer(2), Value::Integer(3)],
    );
    let key = fixture.value(b"new key");
    let mut input = vec![native[0].clone()];
    input.extend(import(&mut session, &[key.clone(), Value::Nil]));
    let expected = fixture.source("write", &[source[0].clone(), key, Value::Nil]);
    let actual = fixture.native(&mut session, "write", &input);
    equal(&mut session, &expected, &actual);
    let error = session
        .invoke(fixture.observed.callbacks()["step"], &[native[0].clone()])
        .unwrap_err();
    assert_eq!(
        error.kind,
        ProgramRuntimeErrorKind::UnsupportedCapability,
        "{error}"
    );
}
#[test]
fn retained_reserved_template_matches_exact_function_warm_traversal() {
    let fixture = Fixture::new();
    let mut session = fixture.session(true);
    let driver = warm::SourceWarmDriver::new(&fixture.lua).unwrap();
    // Only transports the full original result pack; the strict driver requires
    // the exact separately retained original function in completed live traces.
    let wrapper_factory: Function = fixture.lua.load("return function(f) local function pack(...) return { n=select('#',...), ... } end return function(...) return pack(f(...)) end end").eval().unwrap();
    let wrapper: Function = wrapper_factory
        .call(fixture.functions["ordered"].clone())
        .unwrap();
    let seed = [Value::Integer(1), Value::Boolean(false), Value::Integer(3)];
    for (index, arguments) in [
        seed.to_vec(),
        vec![
            fixture.value(b"a"),
            fixture.value(b"b"),
            fixture.value(b"c"),
        ],
        vec![Value::Nil, Value::Integer(2), Value::Boolean(false)],
        vec![Value::Integer(1), Value::Nil, Value::Integer(3)],
        vec![Value::Nil, Value::Nil, Value::Integer(3)],
        vec![Value::Nil, Value::Nil, Value::Nil],
    ]
    .into_iter()
    .enumerate()
    {
        // The latter cases retain a trace from 128 all-present calls and then
        // execute 128 changed-nil calls without flushing between them. The final
        // branch may exit/materialize; only the exact target's live trace is
        // claimed, not compilation of every final path.
        let seed_arguments = (index >= 2).then_some(seed.as_slice());
        let result = driver
            .run_with_target(
                &fixture.lua,
                &wrapper,
                &fixture.functions["ordered"],
                &arguments,
                seed_arguments,
            )
            .unwrap();
        assert!(result.success);
        assert_eq!(result.calls, 128);
        assert_eq!(result.seed_calls, if index >= 2 { 128 } else { 0 });
        assert!(result.target_live_traces > 0 && result.live_traces > 0);
        assert!(result.trace_aborts_complete);
        let _ = (&result.trace_aborts, &result.target_trace_aborts);
        let Value::Table(pack) = result.value else {
            panic!("packed warm result")
        };
        let count: usize = pack.raw_get("n").unwrap();
        let expected = (1..=count)
            .map(|i| pack.raw_get::<Value>(i).unwrap())
            .collect::<Vec<_>>();
        let input = import(&mut session, &arguments);
        let actual = fixture.native(&mut session, "ordered", &input);
        equal(&mut session, &expected, &actual);
    }
}
