use super::*;
use crate::source_programs::lower_from_sources;
use poe_optimizer_engine::source_program::{
    CompiledSourcePrograms, ProgramLimits, ProgramSession, ProgramValue, SessionValue,
};
const PATH: &str = "tests/session.lua";
struct Fixture {
    lua: Lua,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    source: ItemLoadingSource,
    exports: Table,
}
fn fixture(text: &str) -> Fixture {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let exports = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    Fixture {
        lua,
        observer,
        exports,
        sources: [(PATH.into(), text.into())].into(),
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [(PATH.into(), hash(text.as_bytes()))].into(),
            construction_spans: [(
                "module".into(),
                ItemSourceSpan {
                    path: PATH.into(),
                    line: 1,
                    end_line: text.lines().count() as u32,
                    sha256: hash(text.as_bytes()),
                },
            )]
            .into(),
            module_order: vec![PATH.into()],
        },
    }
}
fn request(f: &Fixture, callbacks: &[&str], state: &[&str]) -> SourceSessionCaptureRequest {
    SourceSessionCaptureRequest {
        callbacks: callbacks
            .iter()
            .map(|name| ((*name).into(), f.exports.raw_get(*name).unwrap()))
            .collect(),
        state_roots: state
            .iter()
            .map(|name| ((*name).into(), f.exports.raw_get(*name).unwrap()))
            .collect(),
        ..SourceSessionCaptureRequest::default()
    }
}
fn observe(f: &Fixture, request: SourceSessionCaptureRequest) -> Result<ObservedSourceSession> {
    f.observer
        .observe_session(&f.lua, &f.sources, f.source.clone(), request)
}
fn compile(f: &Fixture, observed: &ObservedSourceSession) -> CompiledSourcePrograms {
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    CompiledSourcePrograms::new(lowered.catalog()).unwrap()
}
fn invoke(
    session: &mut ProgramSession,
    callable: &SessionValue,
    args: &[SessionValue],
) -> Vec<ProgramValue> {
    let output = session.invoke_callable(callable, args).unwrap();
    session.snapshot(&output).unwrap().graph().values.clone()
}
fn selection(table: Table, fields: &[&str]) -> SourceTableSelection {
    SourceTableSelection {
        table,
        fields: fields.iter().map(|key| (*key).into()).collect(),
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    }
}
#[test]
fn actual_shared_scalar_cells_and_equal_distinct_cells_have_separate_identity() {
    let f = fixture(
        r#"local shared = 0
local function increment() shared = shared + 1; return shared end
local function read() return shared end
local independent = 0
local function separate() return independent end
return {increment=increment, read=read, separate=separate}
"#,
    );
    let observed = observe(&f, request(&f, &["increment", "read", "separate"], &[])).unwrap();
    let input = observed.input();
    assert_eq!(
        input.cells,
        vec![
            SourceSessionValue::Number(0.0),
            SourceSessionValue::Number(0.0)
        ]
    );
    assert_eq!(input.closures[0].captures, input.closures[1].captures);
    assert_ne!(input.closures[0].captures, input.closures[2].captures);
    assert!(observed.owner().definitions().unwrap().tables.is_empty());
    for callback in &observed.owner().definitions().unwrap().callbacks {
        assert!(
            callback
                .upvalues
                .iter()
                .all(|capture| matches!(capture.value, SourceValue::LiveCapture {}))
        );
    }
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(input, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("increment").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(1.0)]
    );
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("read").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(1.0)]
    );
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("separate").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(0.0)]
    );
    let lua_read: Function = f.exports.raw_get("read").unwrap();
    assert_eq!(
        lua_read.call::<f64>(()).unwrap(),
        0.0,
        "native session never mutates source Lua state"
    );
}
#[test]
fn construction_instances_share_only_source_prototype_and_retain_table_aliases() {
    let f = fixture(
        r#"local state = {value=5}
local function make(target)
    return function() target.value = target.value + 1; return target.value end
end
local first, second = make(state), make(state)
local function probe() return state.value, state.first == state.second end
state.first, state.second, state.self = first, second, state
return {first=first, second=second, probe=probe, state=state}
"#,
    );
    let observed = observe(&f, request(&f, &["first", "second", "probe"], &["state"])).unwrap();
    let input = observed.input();
    assert_eq!(input.closures.len(), 3);
    let first = &input.closures[0];
    let second = &input.closures[1]; // table traversal discovers second while capturing first
    assert_eq!(first.prototype.id(), second.prototype.id());
    assert_ne!(first.captures, second.captures);
    assert_eq!(
        input.cells[first.captures[0].0 as usize - 1],
        input.cells[second.captures[0].0 as usize - 1]
    );
    assert_eq!(input.state.tables.len(), 1);
    assert!(observed.owner().definitions().unwrap().tables.is_empty());
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(input, ProgramLimits::default())
        .unwrap();
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("first").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(6.0)]
    );
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("second").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(7.0)]
    );
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("probe").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(7.0), ProgramValue::Boolean(false)]
    );
}
#[test]
fn immutable_definition_cells_stay_separate_from_live_state_and_primitive_identity() {
    let f = fixture(
        r#"local definition = {var="mana", omitted=collectgarbage}
local state = {value=12}
local convert = tonumber
local function apply(text) state[definition.var] = convert(text); return state[definition.var] end
return {apply=apply, definition=definition, state=state}
"#,
    );
    let definition: Table = f.exports.raw_get("definition").unwrap();
    let mut req = request(&f, &["apply"], &["state"]);
    req.definition_roots
        .insert("Definition".into(), definition.clone());
    req.definitions
        .projections
        .push(selection(definition, &["var"]));
    let observed = observe(&f, req).unwrap();
    let input = observed.input();
    assert_eq!(input.state.tables.len(), 1);
    assert_eq!(observed.owner().definitions().unwrap().tables.len(), 1);
    assert!(
        input
            .cells
            .iter()
            .any(|value| matches!(value, SourceSessionValue::DefinitionTable(_)))
    );
    assert!(
        input
            .cells
            .iter()
            .any(|value| matches!(value, SourceSessionValue::Callback(_)))
    );
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(input, ProgramLimits::default())
        .unwrap();
    let args = session
        .borrow(&SourceSessionValueGraph {
            values: vec![ProgramValue::Bytes(b"27".to_vec())],
            tables: vec![],
        })
        .unwrap();
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("apply").unwrap()],
            &args
        ),
        vec![ProgramValue::Number(27.0)]
    );
}
#[test]
fn projections_retain_omitted_fields_and_unsupported_metatables_fail_closed() {
    let f = fixture(
        r#"local state = {known=7, hidden=collectgarbage}
local function read() return state.known end
local function hidden() return state.hidden end
local function missing() return state.missing end
return {read=read, hidden=hidden, missing=missing, state=state}
"#,
    );
    let state: Table = f.exports.raw_get("state").unwrap();
    let index = f.lua.create_table().unwrap();
    let mt = f.lua.create_table().unwrap();
    mt.raw_set("__index", index).unwrap();
    state.set_metatable(Some(mt.clone())).unwrap();
    let mut req = request(&f, &["read", "hidden", "missing"], &["state"]);
    assert!(observe(&f, req.clone()).is_err());
    let mut projected = selection(state, &["known"]);
    projected.allow_index_fallback = true;
    req.state_projections.push(projected);
    let observed = observe(&f, req.clone()).unwrap();
    let coverage = observed.input().coverage.values().next().unwrap();
    assert!(
        coverage
            .unavailable
            .contains(&SourceTableKey::Text("hidden".into()))
    );
    assert_eq!(
        coverage.index_fallback,
        SourceTableIndexFallback::Unavailable
    );
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("read").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(7.0)]
    );
    for name in ["hidden", "missing"] {
        assert!(
            session
                .invoke_callable(&roots[observed.root_index(name).unwrap()], &[])
                .is_err()
        );
    }
    mt.raw_set("__newindex", f.lua.create_table().unwrap())
        .unwrap();
    assert!(observe(&f, req).is_err());
}
#[test]
fn definition_capture_cannot_smuggle_live_state_or_session_closures_into_owner() {
    let f = fixture(
        r#"local state = {value=1}
local function read() return state.value end
local function make(target)
    return function() return target.value end
end
local definitions = {callback=make(state)}
return {read=read, definitions=definitions, state=state}
"#,
    );
    let mut req = request(&f, &["read"], &["state"]);
    req.definition_roots.insert(
        "Definitions".into(),
        f.exports.raw_get("definitions").unwrap(),
    );
    assert!(
        observe(&f, req.clone())
            .unwrap_err()
            .to_string()
            .contains("immutable definition reaches a live session table")
    );
    let state: Table = f.exports.raw_get("state").unwrap();
    req.definition_roots.clear();
    req.definition_roots.insert("State".into(), state);
    assert!(
        observe(&f, req)
            .unwrap_err()
            .to_string()
            .contains("also classified immutable")
    );
    let mut req = request(&f, &["read"], &["state"]);
    let definitions: Table = f.exports.raw_get("definitions").unwrap();
    definitions
        .raw_set("callback", f.exports.raw_get::<Function>("read").unwrap())
        .unwrap();
    req.definition_roots
        .insert("Definitions".into(), definitions);
    assert!(
        observe(&f, req)
            .unwrap_err()
            .to_string()
            .contains("immutable definition reaches a live session closure")
    );
}
#[test]
fn mixed_live_capture_assignments_compile_while_immutable_captures_stay_rejected() {
    let f = fixture(
        r#"local value = 0
local function mixed(x) value, x = 1, 2; return value end
local function single() value = 3; return value end
return {mixed=mixed, single=single}
"#,
    );
    let observed = observe(&f, request(&f, &["mixed", "single"], &[])).unwrap();
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(lowered.unsupported().is_empty());
    assert_eq!(lowered.catalog().data().programs.len(), 2);
    let callbacks = [(
        "single".into(),
        f.exports.raw_get::<Function>("single").unwrap(),
    )]
    .into();
    let immutable = f
        .observer
        .observe(&f.lua, &f.sources, f.source.clone(), &callbacks)
        .unwrap();
    let owner = SourceProgramOwner::new(immutable.definitions().clone()).unwrap();
    let lowered = lower_from_sources(&f.sources, &owner).unwrap();
    assert_eq!(lowered.unsupported().len(), 1);
    assert!(
        lowered
            .unsupported()
            .values()
            .next()
            .unwrap()
            .contains("declared live session cell")
    );
}

#[test]
fn shared_owner_source_is_independent_of_observed_build_values() {
    let text = r#"local state = {value=1}
local definition = {field="value"}
local function run() state[definition.field] = state[definition.field] + 1; return state[definition.field] end
return {run=run, state=state, definition=definition}
"#;
    let first = fixture(text);
    let second = fixture(text);
    let state: Table = second.exports.raw_get("state").unwrap();
    state.raw_set("value", 9001).unwrap();
    let observations = [&first, &second].map(|f| {
        let mut req = request(f, &["run"], &["state"]);
        req.definition_roots.insert(
            "Definition".into(),
            f.exports.raw_get("definition").unwrap(),
        );
        observe(f, req).unwrap()
    });
    assert_eq!(
        observations[0].owner().definitions(),
        observations[1].owner().definitions()
    );
    assert_ne!(observations[0].input().state, observations[1].input().state);
    let library = compile(&first, &observations[0]);
    assert!(
        library
            .session_from_input(observations[1].input(), ProgramLimits::default())
            .is_err(),
        "equal content does not authorize a foreign owner"
    );
    let (mut a, roots_a) = library
        .session_from_input(observations[0].input(), ProgramLimits::default())
        .unwrap();
    let (mut b, _) = library
        .session_from_input(observations[0].input(), ProgramLimits::default())
        .unwrap();
    assert!(
        b.invoke_callable(&roots_a[observations[0].root_index("run").unwrap()], &[])
            .is_err()
    );
    assert_eq!(
        invoke(
            &mut a,
            &roots_a[observations[0].root_index("run").unwrap()],
            &[]
        ),
        vec![ProgramValue::Number(2.0)]
    );
}
#[test]
fn original_environment_and_primitive_identity_survive_session_capture() {
    let f = fixture(
        r#"local original = tostring
local state = {value=0}
local function rebound(x) return x + 1 end
tostring = rebound
local function read() return original(3), tostring(3), _G.tostring == tostring end
return {read=read, state=state}
"#,
    );
    let mut req = request(&f, &["read"], &["state"]);
    req.definitions = SourceCaptureContext {
        capture_iteration: false,
        projections: vec![selection(f.lua.globals(), &["tostring", "_G"])],
        environment: Some(SourceEnvironmentSelection {
            table: f.lua.globals(),
            root_name: "Globals".into(),
        }),
        ..SourceCaptureContext::default()
    };
    let observed = observe(&f, req.clone()).unwrap();
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        invoke(
            &mut session,
            &roots[observed.root_index("read").unwrap()],
            &[]
        ),
        vec![
            ProgramValue::Bytes(b"3".to_vec()),
            ProgramValue::Number(4.0),
            ProgramValue::Boolean(true)
        ]
    );
    let read: Function = f.exports.raw_get("read").unwrap();
    read.set_environment(f.lua.create_table().unwrap()).unwrap();
    assert!(
        observe(&f, req)
            .unwrap_err()
            .to_string()
            .contains("non-original global environment")
    );
}
#[test]
fn unknown_captures_and_keys_are_rejected_even_when_projected_out() {
    let f = fixture(
        r#"local unsafe = collectgarbage
local function run() return unsafe end
return {run=run, state={}}
"#,
    );
    assert!(observe(&f, request(&f, &["run"], &[])).is_err());
    let state: Table = f.exports.raw_get("state").unwrap();
    state.raw_set(false, 3).unwrap();
    let mut req = request(&f, &[], &["state"]);
    req.state_projections.push(selection(state, &[]));
    assert!(observe(&f, req).unwrap_err().to_string().contains("key"));
}

#[test]
fn immutable_helper_cannot_bake_shared_live_scalar_cell_but_distinct_equal_cell_is_valid() {
    let f = fixture(
        r#"local shared, independent = 0, 0
local function increment() shared = shared + 1; return shared end
local function shared_read() return shared end
local function separate_read() return independent end
return {increment=increment, definitions={shared_read=shared_read, separate_read=separate_read}}
"#,
    );
    let definitions: Table = f.exports.raw_get("definitions").unwrap();
    let mut req = request(&f, &["increment"], &[]);
    req.definition_roots
        .insert("Definitions".into(), definitions.clone());
    assert!(
        observe(&f, req.clone())
            .unwrap_err()
            .to_string()
            .contains("shares a live session capture cell")
    );
    definitions.raw_set("shared_read", Value::Nil).unwrap();
    let observed = observe(&f, req).unwrap();
    let independent = observed
        .owner()
        .callbacks()
        .iter()
        .find(|callback| {
            callback
                .upvalues
                .iter()
                .any(|value| value.name == "independent")
        })
        .unwrap();
    assert_eq!(independent.upvalues[0].value, SourceValue::Number(0.0));
    compile(&f, &observed);
}

#[path = "tests/traversal.rs"]
mod traversal;

#[path = "tests/arena.rs"]
mod arena;
