use super::*;
const PATH: &str = "src/observed.lua";
fn source(text: &str) -> (BTreeMap<String, String>, ItemLoadingSource) {
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemLoadingSource {
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
    };
    (sources, source)
}
fn load(lua: &Lua, text: &str) -> Function {
    lua.load(text).set_name(format!("@{PATH}")).eval().unwrap()
}
#[test]
fn original_closure_captures_preserve_callback_and_table_aliases_cycles_and_spans() {
    let lua = Lua::new();
    let loaded: Table = lua.named_registry_value("_LOADED").unwrap();
    assert!(matches!(
        loaded.raw_get::<Value>("debug").unwrap(),
        Value::Nil
    ));
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    assert!(matches!(
        loaded.raw_get::<Value>("debug").unwrap(),
        Value::Nil
    ));
    assert!(matches!(
        lua.globals().raw_get::<Value>("debug").unwrap(),
        Value::Nil
    ));
    let text = "local shared={}; shared.self=shared\nlocal select=select\nlocal function helper(...) return select('#',...),shared end\nreturn function(...) return helper(...),shared end\n";
    let function = load(&lua, text);
    let (sources, source) = source(text);
    let roots = [("one".into(), function.clone()), ("alias".into(), function)].into();
    let observed = observer.observe(&lua, &sources, source, &roots).unwrap();
    assert_eq!(observed.callbacks()["one"], observed.callbacks()["alias"]);
    let definitions = observed.definitions();
    assert_eq!(definitions.callbacks.len(), 3);
    assert_eq!(definitions.tables.len(), 1);
    assert_eq!(
        definitions.tables[0].fields["self"],
        SourceValue::Table(SourceTableId(1))
    );
    assert_eq!(
        definitions.intrinsics.values().copied().collect::<Vec<_>>(),
        vec![SourceProgramIntrinsic::Select]
    );
    let owner = SourceProgramOwner::new(definitions.clone()).unwrap();
    let lowered = crate::source_programs::lower_from_sources(&sources, &owner).unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 2);
}
#[test]
fn all_captures_are_observed_even_in_unexecuted_branches_and_unknown_values_fail() {
    for (text, expected) in [
        (
            "local proxy=setmetatable({}, {})\nreturn function() if false then return proxy end end\n",
            "metatable/proxy",
        ),
        (
            "local unknown=tostring\nreturn function() if false then return unknown(1) end end\n",
            "builtin has no observed",
        ),
        (
            "local invalid=string.char(255)\nreturn function() return invalid end\n",
            "utf-8",
        ),
    ] {
        let lua = Lua::new();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let function = load(&lua, text);
        let (sources, source) = source(text);
        let error = observer
            .observe(&lua, &sources, source, &[("root".into(), function)].into())
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}
#[test]
fn foreign_hosts_replaced_primitives_source_inventory_and_environments_are_rejected() {
    for edit in [
        "type=tostring",
        "select=tonumber",
        "table.insert=tostring",
        "setmetatable(_G,{})",
        "getmetatable('').__index={}",
        "getmetatable('').__add=function() return 1 end",
    ] {
        let lua = Lua::new();
        let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
        let text = "return function() return 1 end\n";
        let function = load(&lua, text);
        let (sources, source) = source(text);
        lua.load(edit).exec().unwrap();
        assert!(
            observer
                .observe(&lua, &sources, source, &[("root".into(), function)].into())
                .is_err(),
            "{edit}"
        );
    }
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let text = "return function() return 1 end\n";
    let function = load(&lua, text);
    let (mut sources, metadata) = source(text);
    let roots = [("root".into(), function.clone())].into();
    assert!(
        observer
            .observe(&Lua::new(), &sources, metadata.clone(), &roots)
            .is_err()
    );
    sources.insert(PATH.into(), text.replace('1', "2"));
    assert!(
        observer
            .observe(&lua, &sources, metadata.clone(), &roots)
            .is_err()
    );
    let (sources, metadata) = source(text);
    function
        .set_environment(lua.create_table().unwrap())
        .unwrap();
    assert!(
        observer
            .observe(&lua, &sources, metadata, &roots)
            .unwrap_err()
            .to_string()
            .contains("non-original global environment")
    );
}
#[test]
fn captured_graph_recursion_is_bounded_before_model_allocation() {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    let text = "local root={}; local at=root; for _=1,66 do local next={}; at.next=next; at=next end\nreturn function() return root end\n";
    let function = load(&lua, text);
    let (sources, metadata) = source(text);
    assert!(
        observer
            .observe(
                &lua,
                &sources,
                metadata,
                &[("root".into(), function)].into()
            )
            .unwrap_err()
            .to_string()
            .contains("resource bound")
    );
}
