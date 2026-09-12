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
        capture_iteration: false,
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
        capture_iteration: false,
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
        capture_iteration: false,
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
        capture_iteration: false,
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
fn environment_lookup_precedes_arguments_and_escaped_ipairs_keeps_its_runtime_frontier() {
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
    assert!(error.message.contains("ipairs"));
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

#[test]
fn captured_floor_retains_original_identity_when_environment_floor_is_rebound() {
    let f = fixture(
        r#"local original = math.floor
math.floor = function(value) return value + 100 end
return function() return original(-1.25), math.floor(-1.25) end
"#,
        PATH,
    );
    let mut context = environment(&f, &["math"]);
    context.projections.push(selection(
        f.lua.globals().raw_get("math").unwrap(),
        &["floor"],
    ));
    let observed = observe(&f, context).unwrap();
    assert_eq!(
        evaluate(&f, &observed),
        vec![ProgramValue::Number(-2.0), ProgramValue::Number(98.75)]
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
fn string_primitives_keep_captured_and_method_identity_when_environment_library_rebinds() {
    for (name, expression, method_args, expected) in [
        ("lower", "'AbC'", "", ProgramValue::Bytes(b"abc".to_vec())),
        ("find", "'AbC', 'b'", "'b'", ProgramValue::Number(2.0)),
        ("sub", "'AbC', 2", "2", ProgramValue::Bytes(b"bC".to_vec())),
    ] {
        let text = format!(
            "local original = string.{name}\nlocal function replacement(...) return 'rebound' end\nstring = {{ {name} = replacement }}\nreturn function()\n    return original({expression}), ('AbC'):{name}({method_args}), string.{name}({expression})\nend\n"
        );
        let f = fixture(&text, PATH);
        assert!(observe(&f, SourceCaptureContext::default()).is_err());
        let mut context = environment(&f, &["string"]);
        context.projections.push(selection(
            f.lua.globals().raw_get("string").unwrap(),
            &[name],
        ));
        let observed = observe(&f, context).unwrap();
        let values = evaluate(&f, &observed);
        assert_eq!(
            values,
            vec![
                expected.clone(),
                expected,
                ProgramValue::Bytes(b"rebound".to_vec())
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
}

#[test]
fn explicit_environment_rejects_changed_original_string_methods() {
    for name in ["lower", "find", "sub"] {
        let f = fixture("return function() return true end\n", PATH);
        f.lua
            .load(format!("string.{name} = math.min"))
            .exec()
            .unwrap();
        assert!(
            observe(&f, environment(&f, &[]))
                .unwrap_err()
                .to_string()
                .contains("string method"),
            "{name}"
        );
    }
}

fn bit_operations() -> [(&'static str, SourceProgramIntrinsic, f64); 4] {
    [
        ("band", SourceProgramIntrinsic::BitBand, 3.0),
        ("bor", SourceProgramIntrinsic::BitBor, 7.0),
        ("bxor", SourceProgramIntrinsic::BitBxor, 4.0),
        ("bnot", SourceProgramIntrinsic::BitBnot, -8.0),
    ]
}

#[test]
fn original_bit_captures_keep_exact_intrinsic_identity_across_global_rebinding() {
    for (name, operation, expected) in bit_operations() {
        let text =
            format!("local original = bit.{name}\nreturn function() return original(7,3) end\n");
        let f = fixture(&text, PATH);
        let observed = observe(&f, SourceCaptureContext::default()).unwrap();
        let callback = observed
            .owner()
            .callback(observed.callbacks()["evaluate"])
            .unwrap();
        let SourceValue::Callback(original) = callback.upvalues[0].value else {
            panic!("original bit capture")
        };
        assert_eq!(
            observed.owner().callback(original).unwrap().kind,
            SourceCallbackKind::Builtin {
                symbol: format!("bit.{name}")
            }
        );
        assert_eq!(
            observed
                .owner()
                .definitions()
                .unwrap()
                .intrinsics
                .get(&original),
            Some(&operation)
        );
        let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{name}: {:?}",
            lowered.unsupported()
        );
        assert!(matches!(lowered.catalog().data().programs[0].bindings[0],
            SourceProgramBinding::Intrinsic {
                operation: actual,
                source: SourceProgramIntrinsicSource::Captured { callback: id, upvalue: 0 },
            } if actual == operation && id == original));
        assert_eq!(
            evaluate(&f, &observed),
            vec![ProgramValue::Number(expected)]
        );
        f.lua.load(format!("bit.{name} = math.min")).exec().unwrap();
        assert!(
            observe(&f, SourceCaptureContext::default()).is_err(),
            "field identity {name}"
        );

        let text = format!(
            "local original = bit.{name}\nlocal function replacement(a,b) return a+b end\nbit = {{ {name} = replacement }}\nreturn function() return original == bit.{name}, original(7,3), bit.{name}(7,3) end\n"
        );
        let f = fixture(&text, PATH);
        assert!(
            observe(&f, SourceCaptureContext::default()).is_err(),
            "library identity {name}"
        );
        let mut context = environment(&f, &["bit"]);
        context
            .projections
            .push(selection(f.lua.globals().raw_get("bit").unwrap(), &[name]));
        let observed = observe(&f, context).unwrap();
        let callback = observed
            .owner()
            .callback(observed.callbacks()["evaluate"])
            .unwrap();
        let SourceValue::Callback(original) = callback.upvalues[0].value else {
            panic!("capture")
        };
        assert_eq!(
            observed
                .owner()
                .definitions()
                .unwrap()
                .intrinsics
                .get(&original),
            Some(&operation)
        );
        assert_eq!(
            evaluate(&f, &observed),
            vec![
                ProgramValue::Boolean(false),
                ProgramValue::Number(expected),
                ProgramValue::Number(10.0)
            ]
        );
        let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
        assert!(lowered.catalog().data().programs.iter().all(|program| {
            program.bindings.iter().all(|binding| {
                !matches!(
                    binding,
                    SourceProgramBinding::Intrinsic {
                        source: SourceProgramIntrinsicSource::OriginalGlobal,
                        ..
                    }
                )
            })
        }));
    }
}

#[test]
fn reached_bit_call_preserves_original_argument_effects_and_source_error_stage() {
    use poe_optimizer_engine::source_program::{
        ProgramRuntimeErrorKind, ProgramTable, ProgramTableId,
    };
    for (name, _, expected) in bit_operations() {
        for invalid in [false, true] {
            let result = if invalid { "nil" } else { "7" };
            let text = format!(
                "local function effect(side) side.count=side.count+1; return {result} end\nreturn function(side) return bit.{name}(effect(side),3) end\n"
            );
            let f = fixture(&text, PATH);
            let mut context = environment(&f, &["bit"]);
            context
                .projections
                .push(selection(f.lua.globals().raw_get("bit").unwrap(), &[name]));
            let observed = observe(&f, context).unwrap();
            let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
            assert!(
                lowered.unsupported().is_empty(),
                "{name}: {:?}",
                lowered.unsupported()
            );
            let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
            let input = ProgramValueGraph {
                values: vec![ProgramValue::Table(ProgramTableId(1))],
                tables: vec![ProgramTable {
                    entries: vec![(
                        ProgramValue::Bytes(b"count".to_vec()),
                        ProgramValue::Number(0.0),
                    )],
                }],
            };
            let (mut session, args) = compiled.session(&input, ProgramLimits::default()).unwrap();
            let native = session.invoke(observed.callbacks()["evaluate"], &args);
            let source = f.lua.create_table().unwrap();
            source.raw_set("count", 0).unwrap();
            let original = f.roots["evaluate"].call::<f64>(source.clone());
            if invalid {
                assert_eq!(native.unwrap_err().kind, ProgramRuntimeErrorKind::Source);
                assert!(original.is_err());
            } else {
                assert_eq!(
                    session.snapshot(&native.unwrap()).unwrap().graph().values,
                    vec![ProgramValue::Number(expected)]
                );
                assert_eq!(original.unwrap(), expected);
            }
            assert_eq!(
                session.snapshot(&args).unwrap().graph().tables[0].entries,
                vec![(
                    ProgramValue::Bytes(b"count".to_vec()),
                    ProgramValue::Number(1.0)
                )]
            );
            assert_eq!(source.raw_get::<i32>("count").unwrap(), 1);
        }
    }
}

#[test]
fn bit_environment_omissions_and_hostile_library_shapes_do_not_use_original_global_fallback() {
    use poe_optimizer_engine::source_program::ProgramRuntimeErrorKind;
    for (name, _, _) in bit_operations() {
        let text = format!("return function() return bit.{name}(7,3) end\n");
        let f = fixture(&text, PATH);
        let observed = observe(&f, environment(&f, &[])).unwrap();
        let lowered = lower_from_sources(&f.sources, observed.owner()).unwrap();
        assert!(lowered.unsupported().is_empty());
        let compiled = CompiledSourcePrograms::new(lowered.catalog()).unwrap();
        let error = compiled
            .execute(
                observed.callbacks()["evaluate"],
                &ProgramValueGraph::default(),
                ProgramLimits::default(),
            )
            .unwrap_err();
        assert_eq!(
            error.kind,
            ProgramRuntimeErrorKind::UnsupportedCapability,
            "omitted bit library {name}"
        );
        f.lua
            .load(format!("bit.{name} = function() return 0 end"))
            .exec()
            .unwrap();
        assert!(observe(&f, SourceCaptureContext::default()).is_err());
        assert!(
            SourceClosureObserver::capture_before_source(&f.lua)
                .err()
                .expect("replaced C function")
                .to_string()
                .contains("original C function")
        );
    }
    let lua = Lua::new();
    lua.load("setmetatable(bit,{__index=function() return 0 end})")
        .exec()
        .unwrap();
    assert!(SourceClosureObserver::capture_before_source(&lua).is_err());
}
