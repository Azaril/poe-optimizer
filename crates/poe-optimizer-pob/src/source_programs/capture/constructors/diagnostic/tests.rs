use super::*;
use crate::source_programs::{
    SourceProgramExtraction, lower_observed_closures_and_constructors_from_sources,
};
const PATH: &str = "tests/constructor-diagnostic.lua";
const TEXT: &str = "return function(value)\n local empty = {}\n local row = { name = value, absent = nil, flag = false }\n return row, empty\nend\n";
fn observer(lua: &Lua) -> SourceClosureObserver {
    SourceClosureObserver::capture_before_source_with_closures(
        lua,
        SourceTableRuntimeProfile::luajit21_x64_single(),
    )
    .unwrap()
}
fn capture(
    lua: &Lua,
    observer: &SourceClosureObserver,
    function: &Function,
    text: &str,
) -> (
    BTreeMap<String, String>,
    ObservedSourceContext,
    SourceProgramExtraction,
) {
    let sources = [(PATH.into(), text.into())].into();
    let source = ItemLoadingSource {
        upstream_revision: "a".repeat(40),
        files: [(PATH.into(), hash(text.as_bytes()))].into(),
        construction_spans: BTreeMap::new(),
        module_order: vec![PATH.into()],
    };
    let observed = observer
        .observe_with_context(
            lua,
            &sources,
            source,
            &[("root".into(), function.clone())].into(),
            SourceCaptureContext::default(),
        )
        .unwrap();
    let lowered = lower_observed_closures_and_constructors_from_sources(
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
    (sources, observed, lowered)
}
fn request(
    text: &str,
    observed: &ObservedSourceContext,
    snippet: &str,
    line: Option<u32>,
) -> ConstructorDiagnosticRequest {
    let function = &text[text.find("function").unwrap()..];
    let start = function.find(snippet).unwrap() as u32;
    ConstructorDiagnosticRequest {
        callback: observed.callbacks()["root"],
        expression: SourceProgramLocation {
            start,
            end: start + snippet.len() as u32,
        },
        continuation_line: line,
    }
}
fn query(
    target: &ConstructorDiagnosticTarget,
    sources: &BTreeMap<String, String>,
    observed: &ObservedSourceContext,
    lowered: &SourceProgramExtraction,
    request: ConstructorDiagnosticRequest,
) -> ConstructorDiagnosticWitness {
    match target
        .inspect(
            sources,
            observed.constructor_observations().unwrap(),
            lowered.catalog(),
            request,
            Default::default(),
        )
        .unwrap()
    {
        ConstructorDiagnostic::Observed(witness) => *witness,
        ConstructorDiagnostic::Unavailable(reason) => panic!("{reason}"),
    }
}
#[test]
fn exact_registered_function_template_self_markers_and_raw_order_are_diagnostic_only() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let function: Function = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let target = observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    let (sources, observed, lowered) = capture(&lua, &observer, &function, TEXT);
    let before = lowered.catalog().data().clone();
    let witness = query(
        &target,
        &sources,
        &observed,
        &lowered,
        request(
            TEXT,
            &observed,
            "{ name = value, absent = nil, flag = false }",
            Some(4),
        ),
    );
    assert_eq!(witness.function().to_pointer(), function.to_pointer());
    assert!(std::ptr::eq(
        witness.catalog().data(),
        lowered.catalog().data()
    ));
    assert_eq!(witness.report().instruction.word & 255, TDUP);
    assert_eq!(
        witness.report().template_constant_index,
        Some(-1 - (witness.report().instruction.word >> 16) as i32)
    );
    let rows = &witness.report().template_rows;
    for name in ["name", "absent"] {
        assert!(rows.iter().any(|row| row.key
            == ConstructorTemplateValue::Bytes(name.as_bytes().to_vec())
            && row.value == ConstructorTemplateValue::SelfMarker));
    }
    assert!(rows.iter().any(
        |row| row.key == ConstructorTemplateValue::Bytes(b"flag".to_vec())
            && row.value == ConstructorTemplateValue::Boolean(false)
    ));
    assert!(witness.report().continuation_pc.is_some());
    assert!(!witness.report().post_store_proof_unavailable.is_empty());
    witness.verify_unchanged().unwrap();
    assert!(
        crate::source_programs::attach_reserved_string_templates(
            &sources,
            lowered.clone(),
            &[&witness]
        )
        .unwrap_err()
        .to_string()
        .contains("exact self markers"),
        "a real TDUP with a constant false row is outside the self-marker family"
    );
    assert_eq!(lowered.catalog().data(), &before);
    assert!(lowered.constructor_unsupported()[&observed.callbacks()["root"]].contains("keyed"));
    assert!(
        lowered
            .catalog()
            .constructors()
            .unwrap()
            .sites
            .iter()
            .all(|site| site.expression != witness.report().expression)
    );
    let report = serde_json::to_value(witness.report()).unwrap();
    assert!(report.get("function").is_none());
    assert!(report.get("template").is_none());
    let empty = query(
        &target,
        &sources,
        &observed,
        &lowered,
        request(TEXT, &observed, "{}", None),
    );
    assert_eq!(empty.report().instruction.word & 255, TNEW);
    assert!(empty.template().is_none());
    assert!(empty.report().template_rows.is_empty());
}
#[test]
fn same_source_different_function_late_ticket_foreign_observer_and_host_are_rejected() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let function: Function = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let different: Function = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let wrong = observer
        .retain_constructor_diagnostic_target(&lua, &different)
        .unwrap();
    let (sources, observed, lowered) = capture(&lua, &observer, &function, TEXT);
    let request = request(TEXT, &observed, "{}", None);
    assert!(
        wrong
            .inspect(
                &sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                request,
                Default::default()
            )
            .is_err()
    );
    let late = observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    assert!(
        late.inspect(
            &sources,
            observed.constructor_observations().unwrap(),
            lowered.catalog(),
            request,
            Default::default()
        )
        .is_err()
    );
    let other_observer = self::observer(&lua);
    let other = other_observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    assert!(
        other
            .inspect(
                &sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                request,
                Default::default()
            )
            .is_err()
    );
    let foreign = Lua::new();
    let foreign_function: Function = foreign.load(TEXT).eval().unwrap();
    assert!(
        observer
            .retain_constructor_diagnostic_target(&foreign, &foreign_function)
            .is_err()
    );
    assert!(
        observer
            .retain_constructor_diagnostic_target(&lua, &foreign_function)
            .is_err()
    );
}
#[test]
fn template_changes_fail_recheck_and_limits_do_not_publish_partial_witnesses() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let function: Function = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let target = observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    let (sources, observed, lowered) = capture(&lua, &observer, &function, TEXT);
    let request = request(
        TEXT,
        &observed,
        "{ name = value, absent = nil, flag = false }",
        Some(4),
    );
    let witness = query(&target, &sources, &observed, &lowered, request);
    let template = witness.template().unwrap();
    template.raw_set("flag", true).unwrap();
    assert!(witness.verify_unchanged().is_err());
    template.raw_set("flag", false).unwrap();
    witness.verify_unchanged().unwrap();
    for limits in [
        ConstructorDiagnosticLimits {
            max_template_rows: 0,
            ..Default::default()
        },
        ConstructorDiagnosticLimits {
            max_text_bytes: 0,
            ..Default::default()
        },
        ConstructorDiagnosticLimits {
            max_window_instructions: 1,
            ..Default::default()
        },
        ConstructorDiagnosticLimits {
            max_control_flow_instructions: 0,
            ..Default::default()
        },
    ] {
        assert!(
            target
                .inspect(
                    &sources,
                    observed.constructor_observations().unwrap(),
                    lowered.catalog(),
                    request,
                    limits
                )
                .is_err()
        );
    }
    let absent = ConstructorDiagnosticRequest {
        expression: SourceProgramLocation { start: 0, end: 1 },
        ..request
    };
    assert!(matches!(
        target
            .inspect(
                &sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                absent,
                Default::default()
            )
            .unwrap(),
        ConstructorDiagnostic::Unavailable(_)
    ));
    let late_line = ConstructorDiagnosticRequest {
        continuation_line: Some(9999),
        ..request
    };
    assert!(matches!(
        target
            .inspect(
                &sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                late_line,
                Default::default()
            )
            .unwrap(),
        ConstructorDiagnostic::Unavailable(_)
    ));
    let mut changed_sources = sources.clone();
    changed_sources.get_mut(PATH).unwrap().push(' ');
    assert!(
        target
            .inspect(
                &changed_sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                request,
                Default::default()
            )
            .is_err()
    );
}
#[test]
fn legacy_observation_has_no_target_bindings_and_target_roots_are_bounded() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let function: Function = lua.load(TEXT).set_name(format!("@{PATH}")).eval().unwrap();
    let (_, observed, _) = capture(&lua, &observer, &function, TEXT);
    assert!(observer.constructor_diagnostic_targets.borrow().is_empty());
    assert!(
        observed
            .constructor_observations()
            .unwrap()
            .diagnostic_bindings
            .is_empty()
    );
    let mut tickets = Vec::new();
    for _ in 0..MAX_TARGETS {
        tickets.push(
            observer
                .retain_constructor_diagnostic_target(&lua, &function)
                .unwrap(),
        );
    }
    assert!(
        observer
            .retain_constructor_diagnostic_target(&lua, &function)
            .is_err()
    );
    tickets.pop();
    assert!(
        observer
            .retain_constructor_diagnostic_target(&lua, &function)
            .is_ok()
    );
    tickets.clear();
    assert!(
        observer
            .retain_constructor_diagnostic_target(&lua, &function)
            .is_ok()
    );
}
#[test]
fn diagnostic_uses_retained_primitives_and_excludes_nested_function_tables() {
    let text = "return function(value)\n local child = function() return { inner = false } end\n local row = { outer = value }\n return row, child\nend\n";
    let lua = Lua::new();
    let observer = observer(&lua);
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let target = observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    let (sources, observed, lowered) = capture(&lua, &observer, &function, text);
    lua.globals().raw_set("next", false).unwrap();
    lua.globals().raw_set("jit", false).unwrap();
    let witness = query(
        &target,
        &sources,
        &observed,
        &lowered,
        request(text, &observed, "{ outer = value }", Some(4)),
    );
    assert_eq!(witness.report().template_rows.len(), 1);
    assert_eq!(
        witness.report().template_rows[0].key,
        ConstructorTemplateValue::Bytes(b"outer".to_vec())
    );
    witness.verify_unchanged().unwrap();
    let inner = request(text, &observed, "{ inner = false }", None);
    assert!(matches!(
        target
            .inspect(
                &sources,
                observed.constructor_observations().unwrap(),
                lowered.catalog(),
                inner,
                Default::default()
            )
            .unwrap(),
        ConstructorDiagnostic::Unavailable(_)
    ));
}

#[test]
fn global_name_query_uses_original_constant_not_environment_or_rebound_reflection() {
    let lua = Lua::new();
    let observer = observer(&lua);
    let text = "return function(value)\n local row = { name = value }\n return type(row)\nend\n";
    let function: Function = lua.load(text).set_name(format!("@{PATH}")).eval().unwrap();
    let target = observer
        .retain_constructor_diagnostic_target(&lua, &function)
        .unwrap();
    let (sources, observed, lowered) = capture(&lua, &observer, &function, text);
    let witness = query(
        &target,
        &sources,
        &observed,
        &lowered,
        request(text, &observed, "{ name = value }", Some(3)),
    );
    let get = witness
        .report()
        .instruction_window
        .iter()
        .find(|i| i.word & 255 == 54)
        .unwrap();
    assert_eq!(witness.global_name(get.pc, 4).unwrap(), b"type");
    assert!(witness.global_name(get.pc, 3).is_err());
    assert!(witness.global_name(get.pc, usize::MAX).is_err());
    assert!(
        witness
            .global_name(witness.report().instruction.pc, 256)
            .is_err()
    );
    assert!(witness.global_name(0, 256).is_err());
    lua.load("type=function() error('not executed') end; require('jit.util').funck=function() error('not used') end").exec().unwrap();
    assert_eq!(witness.global_name(get.pc, 4).unwrap(), b"type");
    witness.verify_unchanged().unwrap();
}
