//! Complete source callbacks against interpreted LuaJIT, with continuing private sessions.
#[path = "support/source_program_observation.rs"]
mod observation;
use mlua::{Function, Lua, MultiValue, Table, Value};
use poe_optimizer_data::item_loading::ItemLoadingSource;
use poe_optimizer_engine::source_program::*;
use poe_optimizer_pob::source_programs::{capture::*, lower_from_sources};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
const PATH: &str = "tests/support/source_program_iterators.lua";
const TEXT: &str = include_str!("support/source_program_iterators.lua");
struct Fixture {
    lua: Lua,
    functions: BTreeMap<String, Function>,
    observed: ObservedSourceSession,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    provenance: ItemLoadingSource,
    compiled: CompiledSourcePrograms,
}
impl Fixture {
    fn new() -> Self {
        let lua = Lua::new();
        lua.load("jit.off(); jit.flush(); assert(not jit.status())")
            .exec()
            .unwrap();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let table: Table = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
        let functions = table
            .pairs::<String, Function>()
            .map(Result::unwrap)
            .collect::<BTreeMap<_, _>>();
        let sources: BTreeMap<String, String> = [(PATH.into(), TEXT.into())].into();
        let provenance = ItemLoadingSource {
            upstream_revision: poe_optimizer_pob::source::UPSTREAM_REVISION.into(),
            files: [(
                PATH.into(),
                format!("{:x}", Sha256::digest(TEXT.as_bytes())),
            )]
            .into(),
            construction_spans: BTreeMap::new(),
            module_order: vec![PATH.into()],
        };
        let observed = observer
            .observe_session(
                &lua,
                &sources,
                provenance.clone(),
                SourceSessionCaptureRequest {
                    callbacks: functions.clone(),
                    ..SourceSessionCaptureRequest::default()
                },
            )
            .unwrap();
        let lowered = lower_from_sources(&sources, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        Self {
            lua,
            functions,
            observed,
            observer,
            sources,
            provenance,
            compiled,
        }
    }
    fn pair(&self) -> Pair<'_> {
        let (session, roots) = self
            .compiled
            .session_from_input(self.observed.input(), ProgramLimits::default())
            .unwrap();
        Pair {
            fixture: self,
            session,
            roots,
        }
    }
    fn bytes(&self, bytes: &[u8]) -> Value {
        Value::String(self.lua.create_string(bytes).unwrap())
    }
}
struct Pair<'a> {
    fixture: &'a Fixture,
    session: ProgramSession,
    roots: Vec<SessionValue>,
}
impl Pair<'_> {
    fn root(&self, name: &str) -> SessionValue {
        self.roots[self.fixture.observed.root_index(name).unwrap()].clone()
    }
    fn import(&mut self, values: &[Value]) -> Vec<SessionValue> {
        self.session
            .import_with_coverage(&observation::capture(values), &ProgramTableCoverage::new())
            .unwrap()
    }
    fn native(
        &mut self,
        name: &str,
        args: &[SessionValue],
    ) -> Result<Vec<SessionValue>, ProgramRuntimeError> {
        self.session.invoke_callable(&self.root(name), args)
    }
    fn source(&self, name: &str, args: &[Value]) -> mlua::Result<MultiValue> {
        self.fixture.functions[name].call(MultiValue::from_vec(args.to_vec()))
    }
    fn compare(
        &mut self,
        name: &str,
        source: &[Value],
        native: &[SessionValue],
    ) -> (Vec<Value>, Vec<SessionValue>) {
        let original = self.source(name, source).unwrap();
        let actual = self.native(name, native).unwrap();
        assert_eq!(
            observation::canonical(self.session.snapshot(&actual).unwrap().graph()),
            observation::canonical(&observation::capture(&original.clone().into_vec())),
            "{name} {source:?}"
        );
        (original.into_vec(), actual)
    }
    fn failure(&mut self, name: &str, source: &[Value], native: &[SessionValue]) {
        assert!(
            self.source(name, source).is_err(),
            "source must fail: {name}"
        );
        assert_eq!(
            self.native(name, native).unwrap_err().kind,
            ProgramRuntimeErrorKind::Source,
            "native source error, never an unsupported stand-in: {name}"
        );
    }
    fn factory(&mut self, args: &[Value]) -> (Value, SessionValue) {
        let native_args = self.import(args);
        let original = self.source("factory", args).unwrap();
        let actual = self.native("factory", &native_args).unwrap();
        assert_eq!(original.len(), 1);
        assert_eq!(actual.len(), 1);
        assert!(matches!(original[0], Value::Function(_)));
        (original[0].clone(), actual[0].clone())
    }
}
#[test]
fn complete_source_factories_calls_and_generic_loops_match_original_result_packs() {
    let f = Fixture::new();
    let cases: &[(&[u8], &[u8], usize)] = &[
        (b"a1 b22", b"(%a+)(%d+)", 4),
        (b"a1 b22", b"()(%a+)(%d+)", 4),
        (b"a\0\xffb", b".", 6),
        (b"a^b^^", b"^", 5),
        (b"abc", b"", 6),
        (b"", b"", 3),
        (b"none", b"%d+", 3),
        (b"a\nb\n", b"[^\n]+", 4),
        (b"(ab)(c)", b"%b()", 4),
        (b"aab", b"(a*)", 5),
    ];
    for &(subject, pattern, calls) in cases {
        let mut pair = f.pair();
        let args = [f.bytes(subject), f.bytes(pattern)];
        let (source, native) = pair.factory(&args);
        for _ in 0..calls {
            pair.compare(
                "packed_call",
                std::slice::from_ref(&source),
                std::slice::from_ref(&native),
            );
        }
        let (original, actual) = pair.compare(
            "call",
            std::slice::from_ref(&source),
            std::slice::from_ref(&native),
        );
        assert!(original.is_empty());
        assert!(actual.is_empty(), "exhaustion returns zero values");
        let native_args = pair.import(&args);
        pair.compare("collect", &args, &native_args);
    }
    for args in [
        vec![Value::Integer(123), f.bytes(b"%d")],
        vec![f.bytes(b"1212"), Value::Integer(12)],
        vec![Value::Number(-0.0), f.bytes(b".")],
    ] {
        let mut pair = f.pair();
        let (source, native) = pair.factory(&args);
        for _ in 0..6 {
            pair.compare(
                "call",
                std::slice::from_ref(&source),
                std::slice::from_ref(&native),
            );
        }
    }
    for args in [
        vec![],
        vec![f.bytes(b"a")],
        vec![Value::Nil, f.bytes(b".")],
        vec![Value::Boolean(false), f.bytes(b".")],
        vec![f.bytes(b"a"), Value::Boolean(true)],
    ] {
        let mut pair = f.pair();
        let native = pair.import(&args);
        pair.failure("factory", &args, &native);
    }
}
#[test]
fn iterator_aliases_keys_shared_capture_cells_and_foreign_sessions_keep_identity() {
    let f = Fixture::new();
    let mut pair = f.pair();
    let args = [f.bytes(b"abc"), f.bytes(b".")];
    let (first, one) = pair.factory(&args);
    let (second, two) = pair.factory(&args);
    let (values, _) = pair.compare(
        "identities",
        &[first.clone(), first.clone(), second.clone()],
        &[one.clone(), one.clone(), two.clone()],
    );
    assert_eq!(values[0], Value::Boolean(true));
    assert_eq!(values[1], Value::Boolean(false));
    assert_eq!(values[2], f.bytes(b"function"));
    let boxes = pair.source("wrap", std::slice::from_ref(&first)).unwrap();
    let native_boxes = pair.native("wrap", std::slice::from_ref(&one)).unwrap();
    assert_eq!(
        pair.session.snapshot(&native_boxes).unwrap_err().kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    assert_eq!(
        pair.session
            .snapshot(std::slice::from_ref(&one))
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    pair.compare("wrapped_key", &boxes.clone().into_vec(), &native_boxes);
    let (values, _) = pair.compare("alias_call", &boxes.into_vec(), &native_boxes);
    assert_eq!(values, vec![f.bytes(b"a"), f.bytes(b"b")]);
    let (values, _) = pair.compare(
        "call",
        std::slice::from_ref(&first),
        std::slice::from_ref(&one),
    );
    assert_eq!(values, vec![f.bytes(b"c")]);
    pair.compare(
        "split",
        std::slice::from_ref(&second),
        std::slice::from_ref(&two),
    );
    let native_args = pair.import(&args);
    let held = pair.source("hold", &args).unwrap();
    let native_held = pair.native("hold", &native_args).unwrap();
    pair.compare("held_identity", &held.into_vec(), &native_held);
    let (values, _) = pair.compare("held_call", &[], &[]);
    assert_eq!(values, vec![f.bytes(b"a")]);
    let mut other = f.pair();
    assert_eq!(
        other.session.invoke_callable(&one, &[]).unwrap_err().kind,
        ProgramRuntimeErrorKind::InvalidInput
    );
    let (_foreign_source, separate) = other.factory(&args);
    let next = other.session.invoke_callable(&separate, &[]).unwrap();
    assert_eq!(
        other.session.snapshot(&next).unwrap().graph().values,
        vec![ProgramValue::Bytes(b"a".to_vec())]
    );
    let (values, _) = pair.compare("held_call", &[], &[]);
    assert_eq!(values, vec![f.bytes(b"b")]);
    for name in ["function_read", "function_write"] {
        pair.failure(
            name,
            std::slice::from_ref(&first),
            std::slice::from_ref(&one),
        );
    }
}
#[test]
fn argument_effects_receiver_lookup_and_delayed_pattern_failures_match_source_prefixes() {
    let f = Fixture::new();
    for pattern in [b"(".as_slice(), b"%", b"%b(", b"%f", b"[", b"%1"] {
        let mut pair = f.pair();
        let side = f.lua.create_table().unwrap();
        side.raw_set("count", 0).unwrap();
        let args = [Value::Table(side.clone()), f.bytes(b"a"), f.bytes(pattern)];
        let native_args = pair.import(&args);
        let original = pair.source("factory_effect", &args).unwrap();
        let actual = pair.native("factory_effect", &native_args).unwrap();
        assert_eq!(
            (original.len(), actual.len()),
            (1, 1),
            "syntax error is deferred"
        );
        for count in 2..=3 {
            pair.failure(
                "call_effect",
                &[original[0].clone(), args[0].clone()],
                &[actual[0].clone(), native_args[0].clone()],
            );
            let (values, _) = pair.compare(
                "side_state",
                std::slice::from_ref(&args[0]),
                std::slice::from_ref(&native_args[0]),
            );
            assert_eq!(
                values,
                vec![Value::Integer(count)],
                "argument effects survive each failed attempt"
            );
        }
    }
    for receiver in [Value::Integer(123), Value::Boolean(false), Value::Nil] {
        let mut pair = f.pair();
        let side = f.lua.create_table().unwrap();
        side.raw_set("count", 0).unwrap();
        let args = [receiver, Value::Table(side)];
        let native = pair.import(&args);
        pair.failure("colon_effect", &args, &native);
        let (values, _) = pair.compare("side_state", &args[1..], &native[1..]);
        assert_eq!(values, vec![Value::Integer(0)]);
    }
    let mut pair = f.pair();
    let targets = pair.source("target", &[]).unwrap().into_vec();
    let native = pair.native("target", &[]).unwrap();
    let (values, _) = pair.compare("lookup", &targets, &native);
    assert_eq!(
        values,
        vec![f.bytes(b"old"), f.bytes(b"receiver"), f.bytes(b".")]
    );
    pair.compare("target_state", &targets, &native);
    pair.compare("overwrite", &targets[..1], &native[..1]);
    pair.failure("lookup", &targets, &native);
    let (values, _) = pair.compare("target_state", &targets, &native);
    assert_eq!(values, vec![Value::Boolean(true), Value::Integer(2)]);
}

#[test]
fn successful_ignored_arguments_iterator_methods_and_failed_capture_retries_keep_cursor() {
    let f = Fixture::new();
    let mut pair = f.pair();
    let side = f.lua.create_table().unwrap();
    side.raw_set("count", 0).unwrap();
    let args = [Value::Table(side), f.bytes(b"ab"), f.bytes(b".")];
    let native_args = pair.import(&args);
    let original = pair.source("factory_effect", &args).unwrap();
    let actual = pair.native("factory_effect", &native_args).unwrap();
    assert_eq!((original.len(), actual.len()), (1, 1));
    let (values, _) = pair.compare(
        "call_effect",
        &[original[0].clone(), args[0].clone()],
        &[actual[0].clone(), native_args[0].clone()],
    );
    assert_eq!(values, vec![f.bytes(b"a")]);
    let (values, _) = pair.compare("side_state", &args[..1], &native_args[..1]);
    assert_eq!(values, vec![Value::Integer(2)]);
    let source_targets = pair
        .source("iterator_target", &original.into_vec())
        .unwrap()
        .into_vec();
    let native_targets = pair.native("iterator_target", &actual).unwrap();
    let (values, _) = pair.compare("lookup", &source_targets, &native_targets);
    assert_eq!(values, vec![f.bytes(b"b")]);
    pair.compare("target_state", &source_targets, &native_targets);

    // Subject conversion fails only after every extra argument expression runs.
    let mut pair = f.pair();
    let side = f.lua.create_table().unwrap();
    side.raw_set("count", 0).unwrap();
    let args = [Value::Table(side), Value::Boolean(false), f.bytes(b".")];
    let native_args = pair.import(&args);
    pair.failure("factory_effect", &args, &native_args);
    let (values, _) = pair.compare("side_state", &args[..1], &native_args[..1]);
    assert_eq!(values, vec![Value::Integer(1)]);

    // LuaJIT commits a successful match position before reporting an unfinished
    // capture. Each retry advances; syntax failures while matching do not.
    let mut pair = f.pair();
    let (original, actual) = pair.factory(&[f.bytes(b"ab"), f.bytes(b"(.")]);
    for _ in 0..2 {
        pair.failure(
            "call",
            std::slice::from_ref(&original),
            std::slice::from_ref(&actual),
        );
    }
    for _ in 0..2 {
        let (values, native) = pair.compare(
            "call",
            std::slice::from_ref(&original),
            std::slice::from_ref(&actual),
        );
        assert!(values.is_empty());
        assert!(native.is_empty());
    }
    // An unreachable malformed suffix does not make an empty search fail.
    let (original, actual) = pair.factory(&[f.bytes(b""), f.bytes(b"a(")]);
    let (values, native) = pair.compare(
        "call",
        std::slice::from_ref(&original),
        std::slice::from_ref(&actual),
    );
    assert!(values.is_empty());
    assert!(native.is_empty());
}

#[test]
fn already_created_lua_iterator_capture_is_an_explicit_observation_frontier() {
    let f = Fixture::new();
    let iterator: Function = f.functions["factory"]
        .call((f.bytes(b"ab"), f.bytes(b".")))
        .unwrap();
    let error = f
        .observer
        .observe_session(
            &f.lua,
            &f.sources,
            f.provenance.clone(),
            SourceSessionCaptureRequest {
                state_roots: [("already.created".into(), Value::Function(iterator))].into(),
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap_err();
    assert!(
        error.to_string().contains("no observed primitive identity"),
        "{error}"
    );
}
