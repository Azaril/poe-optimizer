use super::*;
use crate::source_programs::lower_from_sources;
use poe_optimizer_engine::source_program::{
    CompiledSourcePrograms, ProgramLimits, ProgramValue, ProgramValueGraph,
};
const PATH: &str = "tests/context.lua";
struct Fixture {
    lua: Lua,
    observer: SourceClosureObserver,
    sources: BTreeMap<String, String>,
    source: ItemLoadingSource,
    roots: BTreeMap<String, Function>,
}
fn fixture(text: &str, alias: &str) -> Fixture {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let function = lua.load(text).set_name(format!("@{alias}")).eval().unwrap();
    Fixture {
        lua,
        observer,
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
        roots: [("evaluate".into(), function)].into(),
    }
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
fn environment(f: &Fixture, fields: &[&str]) -> SourceCaptureContext {
    SourceCaptureContext {
        projections: vec![selection(f.lua.globals(), fields)],
        environment: Some(SourceEnvironmentSelection {
            table: f.lua.globals(),
            root_name: "ObservedGlobals".into(),
        }),
        ..SourceCaptureContext::default()
    }
}
fn observe(f: &Fixture, context: SourceCaptureContext) -> Result<ObservedSourceContext> {
    f.observer
        .observe_with_context(&f.lua, &f.sources, f.source.clone(), &f.roots, context)
}
fn evaluate(f: &Fixture, observed: &ObservedSourceContext) -> Vec<ProgramValue> {
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let (mut session, _) = compiled
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let output = session
        .invoke(observed.callbacks()["evaluate"], &[])
        .unwrap();
    session.snapshot(&output).unwrap().graph().values.clone()
}

#[test]
fn projected_raw_inventory_preserves_aliases_cycles_and_omitted_presence() {
    let f = fixture(
        r#"Data = {selected=7, omitted=collectgarbage, [2]=42, [3]=true}
Data.self = Data
Data.other = { child=Data, complete=true }
return function() return Data.selected, Data.self == Data, _G.Data == Data, Data[2], Data.missing end
"#,
        PATH,
    );
    let mut context = environment(&f, &["Data", "_G"]);
    let mut data = selection(
        f.lua.globals().raw_get("Data").unwrap(),
        &["selected", "self", "other", "missing"],
    );
    data.indexed.insert(2);
    context.projections.push(data);
    let observed = observe(&f, context).unwrap();
    let owner = observed.owner();
    let env = owner.bind_environment().unwrap().unwrap();
    let SourceValue::Table(data_id) = env.table().fields["Data"] else {
        panic!("Data")
    };
    assert_eq!(env.table().fields["_G"], SourceValue::Table(env.table_id()));
    assert_eq!(
        owner.table(data_id).unwrap().fields["self"],
        SourceValue::Table(data_id)
    );
    let SourceValue::Table(other_id) = owner.table(data_id).unwrap().fields["other"] else {
        panic!("other")
    };
    assert!(
        owner.table_coverage(other_id).is_none(),
        "unselected nested table is complete"
    );
    assert_eq!(
        owner.table(other_id).unwrap().fields["child"],
        SourceValue::Table(data_id)
    );
    let coverage = owner.table_coverage(data_id).unwrap();
    assert_eq!(coverage.inventory, SourceTableInventory::Complete);
    assert_eq!(
        coverage.unavailable,
        [
            SourceTableKey::Text("omitted".into()),
            SourceTableKey::Integer(3)
        ]
        .into()
    );
    assert!(!owner.table(data_id).unwrap().fields.contains_key("missing"));
    assert_eq!(
        evaluate(&f, &observed),
        vec![
            ProgramValue::Number(7.0),
            ProgramValue::Boolean(true),
            ProgramValue::Boolean(true),
            ProgramValue::Number(42.0),
            ProgramValue::Nil
        ]
    );
}

#[test]
fn explicit_environment_uses_rebound_values_and_preserves_captured_primitives() {
    let f = fixture(
        r#"local originalMin, originalMax = math.min, math.max
math.min = function(a,b) return a + b end
math.max = function(a,b) return a * b end
tostring = nil
return function() return originalMin(5,3), math.min(5,3), originalMax(5,3), math.max(5,3), tostring end
"#,
        PATH,
    );
    assert!(
        f.observer
            .observe(&f.lua, &f.sources, f.source.clone(), &f.roots)
            .is_err()
    );
    let mut context = environment(&f, &["math", "tostring"]);
    context.projections.push(selection(
        f.lua.globals().raw_get("math").unwrap(),
        &["min", "max"],
    ));
    let observed = observe(&f, context).unwrap();
    assert_eq!(
        evaluate(&f, &observed),
        vec![
            ProgramValue::Number(3.0),
            ProgramValue::Number(8.0),
            ProgramValue::Number(5.0),
            ProgramValue::Number(15.0),
            ProgramValue::Nil
        ]
    );
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(
        lowered
            .catalog()
            .data()
            .programs
            .iter()
            .flat_map(|program| &program.bindings)
            .all(|binding| !matches!(
                binding,
                SourceProgramBinding::Intrinsic {
                    source: SourceProgramIntrinsicSource::OriginalGlobal,
                    ..
                }
            ))
    );
}

#[test]
fn environment_root_name_is_not_a_source_binding_and_locals_captures_win() {
    let f = fixture(
        r#"ObservedGlobals = 17
local data = 11
return function()
    local math = 13
    return ObservedGlobals, data, math
end
"#,
        PATH,
    );
    let observed = observe(&f, environment(&f, &["ObservedGlobals"])).unwrap();
    assert_eq!(
        evaluate(&f, &observed),
        vec![
            ProgramValue::Number(17.0),
            ProgramValue::Number(11.0),
            ProgramValue::Number(13.0)
        ]
    );
}

#[test]
fn omitted_global_and_raw_index_fallbacks_remain_runtime_frontiers() {
    for (text, global_fields, projected) in [
        ("return function() return tonumber('3') end\n", vec![], None),
        (
            "Data=setmetatable({present=3},{__index={fallback=4}})\nreturn function() return Data.missing end\n",
            vec!["Data"],
            Some("Data"),
        ),
    ] {
        let f = fixture(text, PATH);
        let mut context = environment(&f, &global_fields);
        if let Some(name) = projected {
            let mut selected = selection(f.lua.globals().raw_get(name).unwrap(), &["present"]);
            selected.allow_index_fallback = true;
            context.projections.push(selected);
        }
        let observed = observe(&f, context).unwrap();
        let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
        assert!(lowered.unsupported().is_empty());
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        let (mut session, _) = compiled
            .session(&ProgramValueGraph::default(), ProgramLimits::default())
            .unwrap();
        assert!(
            session
                .invoke(observed.callbacks()["evaluate"], &[])
                .is_err(),
            "{text}"
        );
    }
}

#[test]
fn raw_index_opt_in_rejects_other_metamethods_and_unselected_nested_proxies() {
    let f = fixture(
        "Data=setmetatable({present=3}, {__index={missing=4}})\nreturn function() return Data.present end\n",
        PATH,
    );
    let data: Table = f.lua.globals().raw_get("Data").unwrap();
    let mut context = environment(&f, &["Data"]);
    context
        .projections
        .push(selection(data.clone(), &["present"]));
    assert!(
        observe(&f, context.clone())
            .unwrap_err()
            .to_string()
            .contains("opt-in")
    );
    context.projections[1].allow_index_fallback = true;
    let observed = observe(&f, context.clone()).unwrap();
    assert_eq!(evaluate(&f, &observed), vec![ProgramValue::Number(3.0)]);
    let SourceValue::Table(data_id) = observed
        .owner()
        .bind_environment()
        .unwrap()
        .unwrap()
        .table()
        .fields["Data"]
    else {
        panic!("data")
    };
    assert_eq!(
        observed
            .owner()
            .table_coverage(data_id)
            .unwrap()
            .index_fallback,
        SourceTableIndexFallback::Unavailable
    );
    let meta = data.metatable().unwrap();
    for operation in [
        "__newindex",
        "__eq",
        "__add",
        "__len",
        "__mode",
        "__metatable",
        "__future",
    ] {
        meta.raw_set(operation, true).unwrap();
        assert!(
            observe(&f, context.clone())
                .unwrap_err()
                .to_string()
                .contains(operation)
        );
        meta.raw_set(operation, Value::Nil).unwrap();
    }
    data.raw_set("nested", data.clone()).unwrap();
    context.projections[1].fields.insert("nested".into());
    assert!(
        observe(&f, context.clone()).is_ok(),
        "an explicitly selected alias is retained"
    );
    let nested = f.lua.create_table().unwrap();
    nested
        .set_metatable(Some(f.lua.create_table().unwrap()))
        .unwrap();
    data.raw_set("nested", nested).unwrap();
    assert!(
        observe(&f, context)
            .unwrap_err()
            .to_string()
            .contains("metatable/proxy")
    );
}

#[test]
fn unsupported_raw_key_domains_fail_even_when_their_values_were_not_selected() {
    for key in ["true", "{}", "1.5", "'bad\\0key'"] {
        let text = format!(
            "Data = {{ [ {key} ] = collectgarbage, selected=3 }}\nreturn function() return Data.selected end\n"
        );
        let f = fixture(&text, PATH);
        let mut context = environment(&f, &["Data"]);
        context.projections.push(selection(
            f.lua.globals().raw_get("Data").unwrap(),
            &["selected"],
        ));
        assert!(observe(&f, context).is_err(), "unsupported key {key}");
    }
}

#[test]
fn explicit_aliases_and_environment_identity_are_checked_before_capture() {
    let f = fixture("return function() return 1 end\n", "actual/chunk.lua");
    let mut context = environment(&f, &[]);
    assert!(observe(&f, context.clone()).is_err());
    context
        .source_names
        .insert("actual/chunk.lua".into(), PATH.into());
    assert!(observe(&f, context.clone()).is_ok());
    context.environment.as_mut().unwrap().table = f.lua.create_table().unwrap();
    assert!(
        observe(&f, context)
            .unwrap_err()
            .to_string()
            .contains("original globals")
    );
}

#[test]
fn projection_counts_and_key_bounds_fail_before_unbounded_graph_growth() {
    let f = fixture("return function() return 1 end\n", PATH);
    let empty = f.lua.create_table().unwrap();
    let context = SourceCaptureContext {
        projections: (0..4097).map(|_| selection(empty.clone(), &[])).collect(),
        ..SourceCaptureContext::default()
    };
    assert!(
        observe(&f, context)
            .unwrap_err()
            .to_string()
            .contains("count bound")
    );
    let mut projection = selection(empty.clone(), &[]);
    projection.indexed = (0..50_001).collect();
    let context = SourceCaptureContext {
        projections: vec![projection],
        ..SourceCaptureContext::default()
    };
    assert!(
        observe(&f, context)
            .unwrap_err()
            .to_string()
            .contains("selected key bound")
    );
    for i in 0..50_001 {
        empty.raw_set(i, true).unwrap();
    }
    let context = SourceCaptureContext {
        projections: vec![selection(empty, &[])],
        ..SourceCaptureContext::default()
    };
    assert!(
        observe(&f, context)
            .unwrap_err()
            .to_string()
            .contains("row bound")
    );
}

#[test]
fn unknown_call_requires_opt_in_and_never_becomes_a_plain_noncallable_table() {
    let f = fixture(
        "Data=setmetatable({present=3}, {__call=collectgarbage})\nreturn function() return Data() end\n",
        PATH,
    );
    let mut context = environment(&f, &["Data"]);
    let mut data = selection(f.lua.globals().raw_get("Data").unwrap(), &["present"]);
    data.allow_index_fallback = true;
    context.projections.push(data);
    assert!(
        observe(&f, context.clone())
            .unwrap_err()
            .to_string()
            .contains("__call")
    );
    context.projections[1].allow_call_fallback = true;
    let observed = observe(&f, context).unwrap();
    let SourceValue::Table(id) = observed
        .owner()
        .bind_environment()
        .unwrap()
        .unwrap()
        .table()
        .fields["Data"]
    else {
        panic!("data")
    };
    assert_eq!(
        observed.owner().table_coverage(id).unwrap().call_fallback,
        SourceTableCallFallback::Unavailable
    );
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(lowered.unsupported().is_empty());
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let (mut session, _) = compiled
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let error = session
        .invoke(observed.callbacks()["evaluate"], &[])
        .unwrap_err();
    assert_eq!(
        error.kind,
        poe_optimizer_engine::source_program::ProgramRuntimeErrorKind::UnsupportedCapability
    );
}

#[test]
fn environment_lookup_happens_before_arguments_and_global_iterators_keep_their_frontier() {
    let text = "return function() return tonumber(math.missing()) end\n";
    let f = fixture(text, PATH);
    let observed = observe(&f, environment(&f, &[])).unwrap();
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
    let (mut session, _) = compiled
        .session(&ProgramValueGraph::default(), ProgramLimits::default())
        .unwrap();
    let error = session
        .invoke(observed.callbacks()["evaluate"], &[])
        .unwrap_err();
    let offset = text.find("tonumber").unwrap() - text.find("function").unwrap();
    assert_eq!(
        error.location.unwrap().start,
        offset as u32,
        "callee lookup precedes math argument lookup"
    );
    let f = fixture(
        "return function() for _,value in ipairs({}) do end end\n",
        PATH,
    );
    let observed = observe(&f, environment(&f, &["ipairs"])).unwrap();
    let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
    assert!(lowered.catalog().data().programs.is_empty());
    assert!(lowered.unsupported()[&observed.callbacks()["evaluate"]].contains("iterator"));
}

#[test]
fn explicit_environment_still_authenticates_the_string_method_contract() {
    let f = fixture(
        "return function() return ('value'):match('value') end\n",
        PATH,
    );
    f.lua.load("string.match = math.min").exec().unwrap();
    assert!(
        observe(&f, environment(&f, &[]))
            .unwrap_err()
            .to_string()
            .contains("string method")
    );
}
