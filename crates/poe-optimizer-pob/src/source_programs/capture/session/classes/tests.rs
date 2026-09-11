use super::*;
use crate::source_programs::lower_from_sources;
use poe_optimizer_engine::source_program::{
    CompiledSourcePrograms, ProgramLimits, ProgramRuntimeErrorKind, ProgramValue, ProgramValueGraph,
};
use std::path::PathBuf;
const COMMON: &str = "src/Modules/Common.lua";
const PATH: &str = "tests/class_session.lua";
const SOURCE: &str = r#"local helper = function(value) return value + 1 end
local Class = newClass("Example")
function Class:Example() self.value = 7; return self end
function Class:Set(value) self.value = helper(value); return self.value end
function Class:Unselected() return 99 end
local object = new("Example"):Example()
local method = Class.Set
local function use(value) return object:Set(value) end
local function alias() return object.Set == method end
local function erase() object.Set = nil; return object.Set == method end
local function plain() return object.Object == object, object.value end
return {object=object,method=method,helper=helper,use=use,alias=alias,erase=erase,plain=plain}
"#;
struct Fixture {
    lua: Lua,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    source: ItemLoadingSource,
    roots: Table,
    class: Table,
    classes: SourceClassCaptureRequest,
    request: SourceSessionCaptureRequest,
}
fn fixture(text: &str) -> Fixture {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let common = crate::source::read_verified_text(&root, COMMON).unwrap();
    let start = common.find("common.classes = { }").unwrap();
    let end = common.find("\nfunction codePointToUTF8").unwrap();
    let prefix = common[..start]
        .split_inclusive('\n')
        .map(|line| {
            if line.starts_with("local pairs =")
                || line.starts_with("local ipairs =")
                || line.starts_with("local s_format =")
                || line.starts_with("common =")
            {
                line.to_string()
            } else {
                "\n".into()
            }
        })
        .collect::<String>();
    lua.load(format!("{prefix}{}", &common[start..end]))
        .set_name(format!("@{COMMON}"))
        .exec()
        .unwrap();
    let roots: Table = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let class: Table = lua
        .globals()
        .get::<Table>("common")
        .unwrap()
        .get::<Table>("classes")
        .unwrap()
        .get("Example")
        .unwrap();
    let sources: BTreeMap<String, String> =
        [(COMMON.into(), common), (PATH.into(), text.into())].into();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: sources
            .iter()
            .map(|(path, text)| (path.clone(), hash(text.as_bytes())))
            .collect(),
        construction_spans: BTreeMap::new(),
        module_order: vec![COMMON.into(), PATH.into()],
    };
    let classes = SourceClassCaptureRequest {
        classes: vec![SourceClassSelection {
            table: class.clone(),
            methods: ["Set".into()].into(),
        }],
        callbacks: BTreeMap::new(),
        definition_roots: BTreeMap::new(),
        allocation: lua.globals().get("new").unwrap(),
        source_names: BTreeMap::new(),
    };
    let request = SourceSessionCaptureRequest {
        callbacks: ["use", "alias", "erase", "plain"]
            .into_iter()
            .filter_map(|name| match roots.raw_get::<Value>(name).unwrap() {
                Value::Function(function) => Some((name.into(), function)),
                _ => None,
            })
            .collect(),
        state_roots: ["object", "method", "helper"]
            .into_iter()
            .filter_map(|name| {
                let value = roots.raw_get::<Value>(name).unwrap();
                if matches!(value, Value::Nil) {
                    None
                } else {
                    Some((name.into(), value))
                }
            })
            .collect(),
        state_projections: vec![SourceTableSelection {
            table: roots.get("object").unwrap(),
            fields: ["Object", "value", "Set", "state"]
                .into_iter()
                .map(str::to_string)
                .collect(),
            indexed: BTreeSet::new(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        }],
        ..SourceSessionCaptureRequest::default()
    };
    Fixture {
        lua,
        observer,
        sources,
        source,
        roots,
        class,
        classes,
        request,
    }
}
fn observe(f: &Fixture, request: SourceSessionCaptureRequest) -> Result<ObservedSourceSession> {
    f.observer.observe_session_with_classes(
        &f.lua,
        &f.sources,
        f.source.clone(),
        request,
        f.classes.clone(),
        vec![f.roots.get("object").unwrap()],
    )
}
fn compile(f: &Fixture, observed: &ObservedSourceSession) -> CompiledSourcePrograms {
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    CompiledSourcePrograms::new(lowered.catalog()).unwrap()
}
#[test]
fn actual_instances_keep_class_lookup_shared_function_aliases_and_live_state() {
    let f = fixture(SOURCE);
    let observed = observe(&f, f.request.clone()).unwrap();
    let input = observed.input();
    let SourceSessionValue::Table(id) = input.state.values[observed.root_index("object").unwrap()]
    else {
        panic!("object")
    };
    assert_eq!(input.class_bindings.len(), 1);
    assert!(
        input.class_bindings[&id]
            .owner()
            .is_same_owner(observed.owner())
    );
    assert_eq!(
        input.coverage[&id].index_fallback,
        SourceTableIndexFallback::ClassResolved
    );
    assert_eq!(
        input.coverage[&id].call_fallback,
        SourceTableCallFallback::Unavailable
    );
    let SourceSessionValue::Callback(method) =
        input.state.values[observed.root_index("method").unwrap()]
    else {
        panic!("shared method alias")
    };
    assert_eq!(
        input.class_bindings[&id].definition().methods["Set"].callback,
        method
    );
    assert!(input.cells.contains(&SourceSessionValue::Callback(method)));
    assert!(matches!(
        input.state.values[observed.root_index("helper").unwrap()],
        SourceSessionValue::Callback(_)
    ));
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(input, ProgramLimits::default())
        .unwrap();
    let args = session
        .borrow(&ProgramValueGraph {
            values: vec![ProgramValue::Number(19.0)],
            tables: vec![],
        })
        .unwrap();
    let output = session
        .invoke_callable(&roots[observed.root_index("use").unwrap()], &args)
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Number(20.0)]
    );
    let output = session
        .invoke_callable(&roots[observed.root_index("alias").unwrap()], &[])
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Boolean(true)]
    );
    let output = session
        .invoke_callable(&roots[observed.root_index("plain").unwrap()], &[])
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Boolean(true), ProgramValue::Number(20.0)]
    );
    assert_eq!(
        f.roots
            .get::<Table>("object")
            .unwrap()
            .get::<i32>("value")
            .unwrap(),
        7,
        "capture/import never runs constructor or mutates source"
    );
}
#[test]
fn raw_overrides_unavailable_fields_and_deletion_precede_class_lookup() {
    let f = fixture(SOURCE);
    let object: Table = f.roots.get("object").unwrap();
    object.raw_set("Set", false).unwrap();
    let observed = observe(&f, f.request.clone()).unwrap();
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session
            .invoke_callable(&roots[observed.root_index("use").unwrap()], &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
    let output = session
        .invoke_callable(&roots[observed.root_index("erase").unwrap()], &[])
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Boolean(true)]
    );
    let mut request = f.request.clone();
    request.state_projections[0].fields.remove("Set");
    let observed = observe(&f, request).unwrap();
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    assert_eq!(
        session
            .invoke_callable(&roots[observed.root_index("use").unwrap()], &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
}
#[test]
fn class_binding_never_synthesizes_allocation_fields_and_respects_absent_call() {
    let f = fixture(SOURCE);
    let object: Table = f.roots.get("object").unwrap();
    object.raw_set("Object", Value::Nil).unwrap();
    f.class.raw_set("__call", Value::Nil).unwrap();
    let observed = observe(&f, f.request.clone()).unwrap();
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    let output = session
        .invoke_callable(&roots[observed.root_index("plain").unwrap()], &[])
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Boolean(false), ProgramValue::Number(7.0)]
    );
    assert_eq!(
        session
            .invoke_callable(&roots[observed.root_index("object").unwrap()], &[])
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::Source
    );
}
#[test]
fn incompatible_metatables_effective_operations_and_classification_fail_closed() {
    let f = fixture(SOURCE);
    let object: Table = f.roots.get("object").unwrap();
    let mut request = f.request.clone();
    request
        .callbacks
        .insert("explicit_method".into(), f.roots.get("method").unwrap());
    assert!(
        observe(&f, request)
            .unwrap_err()
            .to_string()
            .contains("also a selected shared class function")
    );
    f.class.raw_set("__newindex", false).unwrap();
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("__newindex")
    );
    f.class.raw_set("__newindex", Value::Nil).unwrap();
    f.class
        .raw_set("__index", f.lua.create_table().unwrap())
        .unwrap();
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("self index")
    );
    f.class.raw_set("__index", f.class.clone()).unwrap();
    let copy = f.lua.create_table().unwrap();
    for entry in f.class.pairs::<Value, Value>() {
        let (key, value) = entry.unwrap();
        copy.raw_set(key, value).unwrap();
    }
    object.set_metatable(Some(copy)).unwrap();
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("not a selected original class table")
    );
}
#[test]
fn selected_shared_methods_cannot_freeze_live_tables_or_shared_cells() {
    let text = r#"local state = {value=0}
local count = 0
local Class = newClass("Example")
function Class:Example() self.state=state; return self end
function Class:Set() state.value=state.value+1; return state.value end
local object=new("Example"):Example()
local function use() return object:Set() end
return {object=object,use=use}
"#;
    let f = fixture(text);
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("live session table")
    );
    let text = r#"local count = 0
local Class = newClass("Example")
function Class:Example() return self end
function Class:Set() count=count+1; return count end
local object=new("Example"):Example()
local function use() return count end
return {object=object,use=use}
"#;
    let f = fixture(text);
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("live session capture cell")
    );
}
#[test]
fn floor_is_observed_opaque_without_native_execution_admission() {
    let text = r#"local floor = math.floor
local Class = newClass("Example")
function Class:Example() return self end
function Class:Set(value) return floor(value) end
local object=new("Example"):Example()
return {object=object}
"#;
    let f = fixture(text);
    let observed = observe(&f, f.request.clone()).unwrap();
    let (index,_)=observed.owner().callbacks().iter().enumerate().find(|(_,callback)|matches!(&callback.kind,SourceCallbackKind::Builtin{symbol} if symbol=="math.floor")).unwrap();
    assert!(
        observed
            .owner()
            .intrinsic(SourceCallbackId(index as u32 + 1))
            .is_none()
    );
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    assert!(
        session
            .invoke_method(&roots[observed.root_index("object").unwrap()], "Set", &[])
            .is_err()
    );
    f.lua
        .globals()
        .get::<Table>("math")
        .unwrap()
        .raw_set("floor", f.class.raw_get::<Function>("Set").unwrap())
        .unwrap();
    assert!(observe(&f, f.request.clone()).is_err());
}

#[test]
fn hidden_shared_capture_requires_positive_immutable_ownership() {
    let text = r#"local state = {value=23}
local Class = newClass("Example")
function Class:Example() return self end
function Class:Set() return state.value end
local object=new("Example"):Example()
return {object=object, definitions={nested=state}}
"#;
    let f = fixture(text);
    assert!(
        observe(&f, f.request.clone())
            .unwrap_err()
            .to_string()
            .contains("not explicitly classified immutable")
    );
    let definitions: Table = f.roots.get("definitions").unwrap();
    let mut request = f.request.clone();
    request
        .definition_roots
        .insert("constants".into(), definitions.clone());
    let observed = observe(&f, request.clone()).unwrap();
    let library = compile(&f, &observed);
    let (mut session, roots) = library
        .session_from_input(observed.input(), ProgramLimits::default())
        .unwrap();
    let output = session
        .invoke_method(&roots[observed.root_index("object").unwrap()], "Set", &[])
        .unwrap();
    assert_eq!(
        session.snapshot(&output).unwrap().graph().values,
        vec![ProgramValue::Number(23.0)]
    );
    request.definitions.projections.push(SourceTableSelection {
        table: definitions,
        fields: BTreeSet::new(),
        indexed: BTreeSet::new(),
        allow_index_fallback: false,
        allow_call_fallback: false,
    });
    assert!(
        observe(&f, request)
            .unwrap_err()
            .to_string()
            .contains("not explicitly classified immutable"),
        "omitted table fields cannot classify hidden captures"
    );
}
#[test]
fn late_definition_expansion_respects_reserved_live_table_capacity() {
    let f = fixture(SOURCE);
    let source_names = BTreeMap::new();
    let mut graph = Graph {
        observer: &f.observer,
        sources: &f.sources,
        source_names: &source_names,
        tables: vec![],
        callbacks: vec![],
        seen_tables: BTreeMap::new(),
        seen_callbacks: BTreeMap::new(),
        intrinsics: BTreeMap::new(),
        values: 0,
        text_bytes: 0,
        forbidden_tables: BTreeSet::new(),
        forbidden_callbacks: BTreeSet::new(),
        forbidden_cells: BTreeSet::new(),
        context: SourceProgramContext::default(),
        immutable_capture_tables: None,
        session_tables: MAX_TABLES - 1,
    };
    let table = f.lua.create_table().unwrap();
    table
        .raw_set("nested", f.lua.create_table().unwrap())
        .unwrap();
    assert!(
        graph
            .value(Value::Table(table), 0)
            .unwrap_err()
            .to_string()
            .contains("combined table count bound")
    );
    assert_eq!(
        graph.tables.len(),
        1,
        "reject before allocating beyond combined budget"
    );
}
