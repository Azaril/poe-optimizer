use super::*;
const PATH: &str = "tests/observed-factories.lua";
fn observe(text: &str) -> (Lua, BTreeMap<String, String>, ObservedSourceSession) {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: [(PATH.into(), hash(text.as_bytes()))].into(),
        construction_spans: Default::default(),
        module_order: vec![PATH.into()],
    };
    let observed = observer
        .observe_session(
            &lua,
            &sources,
            source,
            SourceSessionCaptureRequest {
                callbacks: [("root".into(), function)].into(),
                ..Default::default()
            },
        )
        .unwrap();
    (lua, sources, observed)
}
#[test]
fn observes_actual_child_prototypes_without_executing_factory_or_exposing_reflection() {
    let text = "return function(x)\n local function child(y) return x+y end\n return child\nend\n";
    let (lua, sources, observed) = observe(text);
    assert_eq!(observed.input().closures.len(), 1);
    assert!(observed.input().cells.is_empty());
    assert_eq!(
        observed
            .owner()
            .closure_prototypes()
            .unwrap()
            .prototypes
            .len(),
        2
    );
    let evidence = observed.closure_observations().unwrap();
    assert_eq!(evidence.nodes.len(), 2);
    let parent = evidence
        .nodes
        .values()
        .find(|node| !node.children.is_empty())
        .unwrap();
    let child = evidence
        .nodes
        .values()
        .find(|node| node.children.is_empty())
        .unwrap();
    assert_eq!(child.metadata.names, ["x"]);
    assert_eq!(child.metadata.descriptors, [0xc000]);
    assert_eq!(
        parent
            .metadata
            .locals
            .iter()
            .map(|local| local.name.as_str())
            .collect::<Vec<_>>(),
        ["x", "child"]
    );
    let lowered = crate::source_programs::lower_observed_closures_from_sources(
        &sources,
        observed.owner(),
        evidence,
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 2);
    assert_eq!(
        lowered.catalog().closure_creations().unwrap().sites.len(),
        1
    );
    let loaded: Table = lua.named_registry_value("_LOADED").unwrap();
    assert!(matches!(
        loaded.raw_get::<Value>("jit.util").unwrap(),
        Value::Nil
    ));
    assert!(matches!(
        lua.globals().raw_get::<Value>("debug").unwrap(),
        Value::Nil
    ));
}
#[test]
fn same_line_children_and_nested_capture_origins_are_distinct() {
    let text = "return function(x) local a=function() return x end local b=function() return function() return x end end return a,b end";
    let (_, sources, observed) = observe(text);
    assert_eq!(observed.input().closures.len(), 1);
    assert_eq!(
        observed
            .owner()
            .closure_prototypes()
            .unwrap()
            .prototypes
            .len(),
        4
    );
    let lowered = crate::source_programs::lower_observed_closures_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(lowered.catalog().data().programs.len(), 4);
    let sites = &lowered.catalog().closure_creations().unwrap().sites;
    assert_eq!(sites.len(), 3);
    assert_eq!(
        sites
            .iter()
            .filter(|site| matches!(
                site.captures[0],
                SourceProgramCaptureOrigin::ParentCapture { .. }
            ))
            .count(),
        1
    );
}
#[test]
fn bounded_dump_rejects_truncation_and_stripped_debug_without_loading_it() {
    let lua = Lua::new();
    let function: Function = lua
        .load("return function(x) return function() return x end end")
        .eval()
        .unwrap();
    let bytes = dump::write(&lua, &function, MAX_DUMP).unwrap();
    assert!(dump::decode(&bytes).is_ok());
    for length in 0..bytes.len() {
        assert!(dump::decode(&bytes[..length]).is_err(), "{length}");
    }
    let library: Table = lua.globals().raw_get("string").unwrap();
    let dumper: Function = library.raw_get("dump").unwrap();
    let stripped: mlua::LuaString = dumper.call((function.clone(), true)).unwrap();
    assert!(dump::decode(&stripped.as_bytes()).is_err());
    assert!(dump::write(&lua, &function, 8).is_err());
}

#[test]
fn unsupported_child_rejects_complete_parent_transitively_and_owner_evidence_is_exact() {
    let text = "return function() return function() while true do end end end";
    let (_, sources, observed) = observe(text);
    let evidence = observed.closure_observations().unwrap();
    let lowered = crate::source_programs::lower_observed_closures_from_sources(
        &sources,
        observed.owner(),
        evidence,
    )
    .unwrap();
    assert!(lowered.catalog().data().programs.is_empty());
    assert_eq!(lowered.unsupported().len(), 2);
    assert!(
        lowered
            .unsupported()
            .values()
            .any(|reason| reason.contains("created child") && reason.contains("while"))
    );
    let (_, _, other) = observe(text);
    assert!(
        crate::source_programs::lower_observed_closures_from_sources(
            &sources,
            other.owner(),
            evidence
        )
        .is_err()
    );
}
#[test]
fn combined_facets_preserve_original_empty_constructor_proof() {
    let text = "return function(x) local out={} return function() return out,x end end";
    let (_, sources, observed) = observe(text);
    let lowered = crate::source_programs::lower_observed_closures_and_constructors_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert!(
        lowered.unsupported().is_empty(),
        "{:?}",
        lowered.unsupported()
    );
    assert_eq!(
        lowered.catalog().closure_creations().unwrap().sites.len(),
        1
    );
    assert!(lowered.catalog().constructors().is_some());
    // The parent allocation is distinct from its complete nested function body.
    assert!(lowered.constructor_unsupported().is_empty());
    assert_eq!(lowered.catalog().constructors().unwrap().sites.len(), 1);
    let text = "return function() return function() return {} end end";
    let (_, sources, observed) = observe(text);
    let lowered = crate::source_programs::lower_observed_closures_and_constructors_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
        observed.constructor_observations().unwrap(),
    )
    .unwrap();
    assert_eq!(lowered.catalog().constructors().unwrap().sites.len(), 1);
}
#[test]
fn observation_does_not_run_unreachable_factory_effects_and_legacy_opt_in_stays_empty() {
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source(&lua).unwrap();
    assert!(observer.closure_reflection.is_none());
    let text = "local state={count=0}\nreturn function() state.count=state.count+1 return function() return state end end";
    let (_, sources, observed) = observe(text);
    assert_eq!(observed.input().closures.len(), 1);
    assert_eq!(observed.input().state.tables.len(), 1);
    assert!(observed.input().state.tables[0].entries.iter().any(|(key,value)|matches!((key,value),(SourceSessionValue::Bytes(key),SourceSessionValue::Number(value)) if key==b"count" && *value==0.0)));
    let lowered = crate::source_programs::lower_observed_closures_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
    )
    .unwrap();
    assert_eq!(lowered.catalog().data().programs.len(), 2);
}

#[test]
fn original_template_nil_and_dynamic_self_markers_remain_metadata_only() {
    for text in [
        "return function(x) local t={a=nil,b=x,c=false} return function() return t end end",
        "return function(x) local t={a=x,b=nil,c='constant'} return function() return t end end",
    ] {
        let (_, sources, observed) = observe(text);
        assert_eq!(
            observed
                .owner()
                .closure_prototypes()
                .unwrap()
                .prototypes
                .len(),
            2
        );
        let lowered =
            crate::source_programs::lower_observed_closures_and_constructors_from_sources(
                &sources,
                observed.owner(),
                observed.closure_observations().unwrap(),
                observed.constructor_observations().unwrap(),
            )
            .unwrap();
        assert!(
            lowered.unsupported().is_empty(),
            "{:?}",
            lowered.unsupported()
        );
        assert!(lowered.catalog().constructors().unwrap().sites.is_empty());
        assert_eq!(lowered.constructor_unsupported().len(), 1);
    }
}

#[test]
fn warmed_original_loop_normalization_preserves_child_graph_without_disabling_jit() {
    let text = "return function(n) local sum=0 for i=1,n do sum=sum+i end return function() return sum end end";
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let factory: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let reflection = observer.closure_reflection.as_ref().unwrap();
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemSourceSpan {
        path: PATH.into(),
        line: 1,
        end_line: 1,
        sha256: hash(text.as_bytes()),
    };
    let (cold_root, cold, _) = reflection
        .observe(
            &lua,
            &factory,
            &source,
            &sources,
            MAX_TEXT_BYTES,
            MAX_VALUES,
        )
        .unwrap();
    lua.load("require('jit.opt').start('hotloop=1','hotexit=1')")
        .exec()
        .unwrap();
    for n in 20..148 {
        let _: Function = factory.call(n).unwrap();
    }
    let (warm_root, warm, _) = reflection
        .observe(
            &lua,
            &factory,
            &source,
            &sources,
            MAX_TEXT_BYTES,
            MAX_VALUES,
        )
        .unwrap();
    assert_eq!(cold_root, warm_root);
    assert_eq!(
        cold.keys().collect::<Vec<_>>(),
        warm.keys().collect::<Vec<_>>()
    );
    for (pointer, node) in &cold {
        assert_eq!(node.source, warm[pointer].source);
        assert_eq!(node.metadata, warm[pointer].metadata);
        assert_eq!(node.children, warm[pointer].children);
    }
    let node = &warm[&warm_root];
    let mut patched = false;
    for (pc, word) in node.metadata.instructions.iter().enumerate() {
        let values: MultiValue = reflection.bytecode.call((factory.clone(), pc + 1)).unwrap();
        let actual = match values.front().unwrap() {
            Value::Integer(word) => *word as i32 as u32,
            Value::Number(word) => *word as i32 as u32,
            _ => panic!("word"),
        };
        if actual != *word {
            assert!(matches!(actual & 255, 78 | 80 | 81 | 83 | 84 | 86 | 87));
            patched = true;
        }
    }
    assert!(
        patched,
        "the exact original factory must have patched loop bytecode"
    );
    let jit: Table = lua.globals().raw_get("jit").unwrap();
    let status: Function = jit.raw_get("status").unwrap();
    let enabled: MultiValue = status.call(()).unwrap();
    assert!(matches!(enabled.front(), Some(Value::Boolean(true))));
}

#[test]
fn immutable_parent_capture_stays_a_whole_body_frontier_instead_of_copying_cells() {
    let text =
        "local shared=1\nreturn function() return function() shared=shared+1 return shared end end";
    let lua = Lua::new();
    let observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: [(PATH.into(), hash(text.as_bytes()))].into(),
        construction_spans: Default::default(),
        module_order: vec![PATH.into()],
    };
    let observed = observer
        .observe_with_context(
            &lua,
            &sources,
            source,
            &[("root".into(), function)].into(),
            SourceCaptureContext::default(),
        )
        .unwrap();
    let lowered = crate::source_programs::lower_observed_closures_from_sources(
        &sources,
        observed.owner(),
        observed.closure_observations().unwrap(),
    )
    .unwrap();
    assert!(lowered.unsupported()[&observed.callbacks()["root"]].contains("actual session parent"));
}

#[test]
fn exhausted_graph_budgets_reject_before_reflection_and_retained_copy_allocation() {
    use std::{cell::Cell, rc::Rc};
    let lua = Lua::new();
    let mut observer = SourceClosureObserver::capture_before_source_with_closures(
        &lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap();
    let text = "return function(x) return function() return x end end";
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemSourceSpan {
        path: PATH.into(),
        line: 1,
        end_line: 1,
        sha256: hash(text.as_bytes()),
    };
    let calls = Rc::new(Cell::new(0));
    let calls_clone = calls.clone();
    let original = observer.closure_reflection.as_ref().unwrap().info.clone();
    observer.closure_reflection.as_mut().unwrap().info = lua
        .create_function(move |_, _: Value| {
            calls_clone.set(calls_clone.get() + 1);
            Err::<Value, _>(mlua::Error::runtime("unexpected reflection"))
        })
        .unwrap();
    let names = BTreeMap::new();
    macro_rules! make_graph {
        ($observer:expr) => {
            Graph {
                observer: $observer,
                sources: &sources,
                source_names: &names,
                tables: vec![],
                callbacks: vec![],
                seen_tables: Default::default(),
                seen_callbacks: Default::default(),
                intrinsics: Default::default(),
                values: 0,
                text_bytes: 0,
                forbidden_tables: Default::default(),
                forbidden_callbacks: Default::default(),
                forbidden_cells: Default::default(),
                immutable_capture_tables: None,
                session_tables: 0,
                context: Default::default(),
                constructor_observations: Default::default(),
                closure_observations: Default::default(),
            }
        };
    }
    for (values, text_bytes) in [(MAX_VALUES - 1, 0), (0, MAX_TEXT_BYTES - 1)] {
        let mut graph = make_graph!(&observer);
        graph.values = values;
        graph.text_bytes = text_bytes;
        assert!(
            graph
                .observe_closure_creation(SourceCallbackId(1), &function, &source)
                .is_err()
        );
        assert!(graph.closure_observations.nodes.is_empty());
        assert_eq!(graph.values, values);
        assert_eq!(graph.text_bytes, text_bytes);
        assert_eq!(calls.get(), 0);
    }
    observer.closure_reflection.as_mut().unwrap().info = original;
    let mut graph = make_graph!(&observer);
    graph
        .observe_closure_creation(SourceCallbackId(1), &function, &source)
        .unwrap();
    let dump_bytes = dump::write(&lua, &function, MAX_DUMP).unwrap().len();
    let node_text = graph
        .closure_observations
        .nodes
        .values()
        .map(|node| node.source.path.len() + node.source.sha256.len() + node.metadata.sha256.len())
        .sum::<usize>();
    assert_eq!(graph.text_bytes, dump_bytes + node_text);
    let before = graph.text_bytes;
    graph
        .observe_closure_creation(SourceCallbackId(1), &function, &source)
        .unwrap();
    assert_eq!(
        graph.text_bytes,
        before * 2,
        "duplicate observations also retain their cumulative budget charge"
    );
    graph.text_bytes = MAX_TEXT_BYTES - 1;
    let mut prototypes = SourceClosurePrototypes {
        schema_version: SOURCE_CLOSURE_PROTOTYPES_SCHEMA_VERSION,
        prototypes: vec![],
    };
    assert!(graph.finish_closure_creations(&mut prototypes).is_err());
    assert!(graph.callbacks.is_empty());
    assert!(prototypes.prototypes.is_empty());
}
