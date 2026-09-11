use super::*;
use crate::source_programs::lower_from_sources;
use poe_optimizer_engine::source_program::{
    CompiledSourcePrograms, ProgramLimits, ProgramRuntimeErrorKind, ProgramValue, ProgramValueGraph,
};
const PATH: &str = "tests/iteration.lua";
struct Fixture {
    lua: Lua,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    source: ItemLoadingSource,
    function: Function,
}
fn fixture(text: &str) -> Fixture {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    Fixture {
        lua,
        observer,
        sources: [(PATH.into(), text.into())].into(),
        source: ItemLoadingSource {
            upstream_revision: "a".repeat(40),
            files: [(PATH.into(), hash(text.as_bytes()))].into(),
            construction_spans: BTreeMap::new(),
            module_order: vec![PATH.into()],
        },
        function,
    }
}
fn observe(f: &Fixture, context: SourceCaptureContext) -> Result<ObservedSourceContext> {
    f.observer.observe_with_context(
        &f.lua,
        &f.sources,
        f.source.clone(),
        &[("evaluate".into(), f.function.clone())].into(),
        context,
    )
}
fn enabled() -> SourceCaptureContext {
    SourceCaptureContext {
        capture_iteration: true,
        ..SourceCaptureContext::default()
    }
}
fn compile(f: &Fixture, observed: &ObservedSourceContext) -> CompiledSourcePrograms {
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    CompiledSourcePrograms::new(lowered.catalog()).unwrap()
}
#[test]
fn opt_in_preserves_exact_original_next_link_and_raw_definition_order() {
    let f = fixture(
        "local p=pairs\nlocal data={zebra=7,alpha=9,[2]=5,[7]=false}\nreturn function() local out='' for k,v in p(data) do out=out..tostring(k)..':'..tostring(v)..';' end return out end\n",
    );
    let legacy = observe(&f, SourceCaptureContext::default()).unwrap();
    assert!(legacy.owner().context().unwrap().iteration.is_none());
    assert!(
        !legacy
            .owner()
            .definitions()
            .unwrap()
            .intrinsics
            .values()
            .any(|op| *op == SourceProgramIntrinsic::Pairs)
    );
    let observed = observe(&f, enabled()).unwrap();
    let owner = observed.owner();
    let callback = owner.callback(observed.callbacks()["evaluate"]).unwrap();
    let pairs = callback
        .upvalues
        .iter()
        .find_map(|capture| match capture.value {
            SourceValue::Callback(id) => Some(id),
            _ => None,
        })
        .unwrap();
    let table = callback
        .upvalues
        .iter()
        .find_map(|capture| match capture.value {
            SourceValue::Table(id) => Some(id),
            _ => None,
        })
        .unwrap();
    let next = owner.pairs_next_callback(pairs).unwrap();
    assert_eq!(owner.intrinsic(pairs), Some(SourceProgramIntrinsic::Pairs));
    assert_eq!(owner.intrinsic(next), Some(SourceProgramIntrinsic::Next));
    assert!(owner.callback(pairs).unwrap().upvalues.is_empty());
    assert!(owner.callback(next).unwrap().upvalues.is_empty());
    let mut original_table = None;
    for slot in 1..=2 {
        if let Some(value) = upvalues::read(&f.lua, &f.function, slot).unwrap()
            && let Value::Table(table) = value.value
        {
            original_table = Some(table);
        }
    }
    let original_table = original_table.unwrap();
    let mut key = Value::Nil;
    let mut order = Vec::new();
    loop {
        let values: MultiValue = f
            .observer
            .iterator_primitives
            .next
            .call((original_table.clone(), key))
            .unwrap();
        if matches!(values.front(), Some(Value::Nil)) {
            break;
        }
        key = values[0].clone();
        order.push(match &key {
            Value::Integer(value) => SourceTableKey::Integer(*value),
            Value::Number(value) => SourceTableKey::Integer(*value as i64),
            Value::String(value) => SourceTableKey::Text(value.to_str().unwrap().to_string()),
            _ => panic!("key"),
        });
    }
    assert_eq!(owner.table_iteration_order(table).unwrap(), order);
    let native = compile(&f, &observed)
        .execute(
            observed.callbacks()["evaluate"],
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(
        native.graph().values,
        vec![ProgramValue::Bytes(
            f.function.call::<String>(()).unwrap().into_bytes()
        )]
    );
}
#[test]
fn changed_environment_next_does_not_replace_pairs_retained_iterator() {
    let f = fixture(
        "local p=pairs\nnext=function() return 'replacement' end\nreturn function() local iterator=p({}) return iterator==next end\n",
    );
    let globals = f.lua.globals();
    let context = SourceCaptureContext {
        capture_iteration: true,
        projections: vec![SourceTableSelection {
            table: globals.clone(),
            fields: ["next".into()].into(),
            indexed: Default::default(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        }],
        environment: Some(SourceEnvironmentSelection {
            table: globals,
            root_name: "globals".into(),
        }),
        source_names: BTreeMap::new(),
    };
    let observed = observe(&f, context).unwrap();
    let native = compile(&f, &observed)
        .execute(
            observed.callbacks()["evaluate"],
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(native.graph().values, vec![ProgramValue::Boolean(false)]);
    assert!(!f.function.call::<bool>(()).unwrap());
}
#[test]
fn partial_projected_and_live_tables_never_gain_definition_iteration_proof() {
    let f = fixture(
        "Data={first=1,second=2}\nreturn function() local out=0 for k,v in pairs(Data) do out=out+v end return out end\n",
    );
    let globals = f.lua.globals();
    let data: Table = globals.raw_get("Data").unwrap();
    let mut context = enabled();
    context.environment = Some(SourceEnvironmentSelection {
        table: globals.clone(),
        root_name: "globals".into(),
    });
    context.projections = vec![
        SourceTableSelection {
            table: globals,
            fields: ["Data".into(), "pairs".into()].into(),
            indexed: Default::default(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        },
        SourceTableSelection {
            table: data,
            fields: ["first".into()].into(),
            indexed: Default::default(),
            allow_index_fallback: false,
            allow_call_fallback: false,
        },
    ];
    let observed = observe(&f, context).unwrap();
    let owner = observed.owner();
    let SourceValue::Table(table) =
        owner.bind_environment().unwrap().unwrap().table().fields["Data"]
    else {
        panic!("data")
    };
    assert!(owner.table_iteration_order(table).is_none());
    assert_eq!(
        compile(&f, &observed)
            .execute(
                observed.callbacks()["evaluate"],
                &ProgramValueGraph::default(),
                ProgramLimits::default()
            )
            .unwrap_err()
            .kind,
        ProgramRuntimeErrorKind::UnsupportedCapability
    );
    let f = fixture(
        "local data={one=1}\nlocal p=pairs\nreturn function() for k in p(data) do end end\n",
    );
    let observed = f
        .observer
        .observe_session(
            &f.lua,
            &f.sources,
            f.source.clone(),
            SourceSessionCaptureRequest {
                callbacks: [("live".into(), f.function.clone())].into(),
                definitions: enabled(),
                ..SourceSessionCaptureRequest::default()
            },
        )
        .unwrap();
    assert_eq!(observed.input().state.tables.len(), 1);
    assert!(
        observed
            .owner()
            .context()
            .unwrap()
            .iteration
            .as_ref()
            .unwrap()
            .table_order
            .is_empty()
    );
}
#[test]
fn empty_plain_tables_have_explicit_order_but_empty_metatables_do_not() {
    let f = fixture(
        "local p=pairs\nlocal data={}\nreturn function() local count=0 for k in p(data) do count=count+1 end return count end\n",
    );
    let observed = observe(&f, enabled()).unwrap();
    let owner = observed.owner();
    let callback = owner.callback(observed.callbacks()["evaluate"]).unwrap();
    let table = callback
        .upvalues
        .iter()
        .find_map(|capture| match capture.value {
            SourceValue::Table(id) => Some(id),
            _ => None,
        })
        .unwrap();
    assert_eq!(owner.table_iteration_order(table), Some([].as_slice()));
    let native = compile(&f, &observed)
        .execute(
            observed.callbacks()["evaluate"],
            &ProgramValueGraph::default(),
            ProgramLimits::default(),
        )
        .unwrap();
    assert_eq!(native.graph().values, vec![ProgramValue::Number(0.0)]);

    let f = fixture("Data=setmetatable({one=1},{})\nreturn function() return Data.one end\n");
    let globals = f.lua.globals();
    let data: Table = globals.raw_get("Data").unwrap();
    let observed = observe(
        &f,
        SourceCaptureContext {
            capture_iteration: true,
            environment: Some(SourceEnvironmentSelection {
                table: globals.clone(),
                root_name: "globals".into(),
            }),
            projections: vec![
                SourceTableSelection {
                    table: globals,
                    fields: ["Data".into()].into(),
                    indexed: Default::default(),
                    allow_index_fallback: false,
                    allow_call_fallback: false,
                },
                SourceTableSelection {
                    table: data,
                    fields: ["one".into()].into(),
                    indexed: Default::default(),
                    allow_index_fallback: true,
                    allow_call_fallback: false,
                },
            ],
            source_names: BTreeMap::new(),
        },
    )
    .unwrap();
    let owner = observed.owner();
    let SourceValue::Table(table) =
        owner.bind_environment().unwrap().unwrap().table().fields["Data"]
    else {
        panic!("data")
    };
    assert!(owner.table_iteration_order(table).is_none());
}
#[test]
fn mutated_original_pairs_c_capture_is_rejected_before_graph_capture() {
    let f = fixture("return function() return 1 end\n");
    let pairs = f.observer.iterator_primitives.pairs.clone();
    let replacement: Function = f.lua.globals().raw_get("type").unwrap();
    // Test-only mutation of the actual C slot; production observation never sets it.
    unsafe {
        f.lua
            .exec_raw::<()>((pairs.clone(), replacement), |state| {
                mlua::ffi::lua_setupvalue(state, 1, 1);
                mlua::ffi::lua_settop(state, 0);
            })
            .unwrap();
    }
    let error = observe(&f, enabled()).unwrap_err().to_string();
    unsafe {
        f.lua
            .exec_raw::<()>(
                (pairs, f.observer.iterator_primitives.next.clone()),
                |state| {
                    mlua::ffi::lua_setupvalue(state, 1, 1);
                    mlua::ffi::lua_settop(state, 0);
                },
            )
            .unwrap();
    }
    assert!(error.contains("retained next capture changed"), "{error}");
}
