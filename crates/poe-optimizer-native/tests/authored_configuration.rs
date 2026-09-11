#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{SelectionRequest, ViewRequest},
    evaluation::EvaluationErrorKind,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{CompiledGameData, configuration::*};
use std::sync::{Arc, OnceLock};
fn data() -> &'static Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| CompiledGameData::bundled().unwrap())
}
fn import(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([114; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn prepare(xml: &str) -> PreparedConfiguration {
    let build = import(xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_configuration(
        &build,
        &view,
        data(),
        ConfigurationPreparationLimits::default(),
    )
    .unwrap()
}
fn doc(body: &str) -> String {
    format!("<PathOfBuilding2>{body}</PathOfBuilding2>")
}
fn set(body: &str) -> String {
    doc(&format!(
        "<Config><ConfigSet id='1'>{body}</ConfigSet></Config>"
    ))
}
fn text(value: &ConfigurationValue) -> &str {
    value.text().unwrap()
}

#[test]
fn all_five_originals_prepare_exact_authored_prefix_without_effective_configuration() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    for (n, inputs) in [(1, 53), (2, 52), (3, 46), (4, 45), (5, 45)] {
        let xml = std::fs::read_to_string(root.join(format!("build-{n:02}.xml"))).unwrap();
        let stage = prepare(&xml);
        let report = stage.report();
        assert_eq!(report.status, ConfigurationPrefixStatus::Prepared);
        assert_eq!(report.sets.len(), 1);
        assert_eq!(report.default_state.len(), 563);
        let config = &report.sets[0];
        assert_eq!(config.inputs.len(), inputs);
        assert_eq!(config.placeholders.len(), 34);
        assert_eq!(text(&config.inputs["enemyIsBoss"]), "Pinnacle");
        assert!(!config.inputs.contains_key("enemyLevel"));
        assert_eq!(config.placeholders["enemyLevel"].number(), Some(82.0));
        assert_eq!(config.migration_passes, 1);
        assert_eq!(config.blocks.len(), 1);
        assert!(
            matches!(&config.blocks[0].text,ConfigurationBlockText::Value{value} if value.text()==Some(""))
        );
        assert_eq!(
            report.continuation.as_ref().unwrap().stage,
            ConfigurationContinuationStage::UpdateControls
        );
        assert!(report.diagnostics.is_empty());
        assert!(!report.frontiers.is_empty());
    }
}
#[test]
fn scalar_precedence_deletions_nonfinite_values_and_placeholder_destination_match_source() {
    let stage = prepare(&set(
        "<Input name='x' string='before'/><Input name='x' number='garbage' string='ignored'/><Input name='falseish' boolean='TRUE'/><Input name='nan' number='nan'/><Input name='hex' number='0x10'/><Input name='enemyIsBoss' string='uBeR atziri'/><Placeholder name='enemyIsBoss' string='Shaper'/><Placeholder name='customNumber' number='inf'/><Input name='raw' string='line1\nline2'/>",
    ));
    let r = stage.report();
    assert_eq!(r.status, ConfigurationPrefixStatus::Prepared);
    let s = &r.sets[0];
    assert!(!s.inputs.contains_key("x"));
    assert_eq!(text(&s.inputs["raw"]), "line1\nline2");
    assert!(matches!(
        s.inputs["falseish"],
        ConfigurationValue::Boolean(false)
    ));
    assert!(s.inputs["nan"].number().unwrap().is_nan());
    assert_eq!(s.inputs["hex"].number(), Some(16.0));
    assert_eq!(text(&s.inputs["enemyIsBoss"]), "Shaper");
    assert!(!s.placeholders.contains_key("enemyIsBoss"));
    assert!(
        s.placeholders["customNumber"]
            .number()
            .unwrap()
            .is_infinite()
    );
    assert!(r.writes.iter().any(|w| w.key == "x" && w.value.is_none()));
}
#[test]
fn ignored_record_errors_do_not_stop_loading_later_valid_records() {
    let stage = prepare(&set(
        "<Input number='2'/><Input name='missing'/><Placeholder name='missingPlaceholder' boolean='true'/><Input name='following' number='32'/><Unknown ignored='yes'/>",
    ));
    let r = stage.report();
    assert_eq!(r.status, ConfigurationPrefixStatus::Prepared);
    assert_eq!(r.diagnostics.len(), 3);
    assert_eq!(r.sets[0].inputs["following"].number(), Some(32.0));
    assert!(r.failure.is_none());
}
#[test]
fn migrations_preserve_source_types_and_first_block_array_entry() {
    let stage = prepare(&set(
        "<Input name='enemyIsBoss' string='sHaPeR'/><Input name='presetBossSkills' string='Uber Atziri Flameblast'/><Input name='customMods' number='12'/>",
    ));
    let r = stage.report();
    assert_eq!(text(&r.sets[0].inputs["enemyIsBoss"]), "Pinnacle");
    assert_eq!(
        text(&r.sets[0].inputs["presetBossSkills"]),
        "Atziri Flameblast"
    );
    assert!(!r.sets[0].inputs.contains_key("customMods"));
    assert!(
        matches!(&r.sets[0].blocks[0].text,ConfigurationBlockText::Value{value} if value.number()==Some(12.0))
    );
    assert!(matches!(
        r.sets[0].blocks[0].origin,
        ConfigurationBlockOrigin::LegacyInput { source: Some(_) }
    ));
    let stage = prepare(&set(
        "<CustomModifierBlock enabled='TRUE'><Future/>tail</CustomModifierBlock>",
    ));
    assert!(!stage.report().sets[0].blocks[0].enabled);
    assert!(matches!(
        stage.report().sets[0].blocks[0].text,
        ConfigurationBlockText::Element { .. }
    ));
    assert_eq!(stage.report().status, ConfigurationPrefixStatus::Prepared);
}
#[test]
fn source_order_holes_skip_migration_and_duplicate_keys_retain_overwritten_sets() {
    let stage = prepare(&doc(
        "<Config activeConfigSet='2'><ConfigSet id='7'><Input name='customMods' string='first'/></ConfigSet><Input name='legacy' string='separate'/><ConfigSet id='2'><Input name='customMods' string='not yet migrated'/></ConfigSet></Config>",
    ));
    let r = stage.report();
    assert_eq!(r.order.len(), 3);
    assert!(r.order[1].is_none());
    let later = r.sets.iter().find(|s| s.key.value() == 2.0).unwrap();
    assert_eq!(later.migration_passes, 0);
    assert_eq!(text(&later.inputs["customMods"]), "not yet migrated");
    assert!(later.blocks.is_empty());
    let stage = prepare(&doc(
        "<Config><ConfigSet id='2.5'><Input name='x' number='1'/></ConfigSet><ConfigSet id='2.5'><Input name='x' number='2'/></ConfigSet></Config>",
    ));
    let r = stage.report();
    assert_eq!(r.sets.len(), 2);
    assert!(!r.sets[0].winner);
    assert!(r.sets[1].winner);
    assert_eq!(r.sets[1].migration_passes, 2);
    assert_eq!(r.sets[1].inputs["x"].number(), Some(2.0));
}
#[test]
fn nil_and_nan_keys_fail_with_registered_prefix_and_repeated_sections_stay_unexecuted() {
    let stage = prepare(&doc("<Config><ConfigSet id='2'/><ConfigSet/></Config>"));
    let r = stage.report();
    assert_eq!(r.status, ConfigurationPrefixStatus::SourceFailure);
    assert_eq!(r.sets.len(), 2);
    assert!(r.failure.as_ref().unwrap().message.contains("nil"));
    let stage = prepare(&doc("<Config><ConfigSet id='nan'/></Config>"));
    assert_eq!(
        stage.report().status,
        ConfigurationPrefixStatus::SourceFailure
    );
    assert!(stage.report().sets.is_empty());
    let stage = prepare(&doc(
        "<Config><ConfigSet id='1'><Input name='x' number='1'/></ConfigSet></Config><Config><ConfigSet id='2'><Input name='x' number='2'/></ConfigSet></Config>",
    ));
    let r = stage.report();
    assert_eq!(r.sets.len(), 1);
    assert_eq!(r.sets[0].inputs["x"].number(), Some(1.0));
    assert_eq!(
        r.continuation.as_ref().unwrap().unexecuted_containers.len(),
        1
    );
    assert_ne!(
        serde_json::to_value(r.active_set).unwrap(),
        serde_json::to_value(r.view_selected_set).unwrap()
    );
}
#[test]
fn constructor_prefix_binding_isolation_parallel_reuse_and_resource_failure_are_explicit() {
    let missing = prepare(&doc(""));
    assert_eq!(
        missing.report().continuation.as_ref().unwrap().stage,
        ConfigurationContinuationStage::InitialBuildModList
    );
    assert_eq!(missing.report().sets[0].title, None);
    let xml = doc("<Config><ConfigSet id='1'/><ConfigSet id='2'/></Config>");
    let build = import(&xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let prepared = prepare_authored_configuration(
        &build,
        &view,
        data(),
        ConfigurationPreparationLimits::default(),
    )
    .unwrap();
    prepared
        .validate_binding(&build.clone(), &view, data())
        .unwrap();
    let foreign = import(&xml);
    let foreign_view = resolve_view(
        &foreign,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        prepared
            .validate_binding(&foreign, &foreign_view, data())
            .is_err()
    );
    let second = build
        .instances()
        .iter()
        .filter_map(|b| {
            if let AuthoredInstanceId::ConfigSet(id) = b.instance() {
                Some(id)
            } else {
                None
            }
        })
        .nth(1)
        .unwrap();
    let another_view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest {
            configuration: SelectionRequest::Instance(second),
            ..Default::default()
        },
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        prepared
            .validate_binding(&build, &another_view, data())
            .is_err()
    );
    // Creating a second set must reserve its own default assignments. The
    // budget below fits the UI defaults plus exactly one CreateConfigSet pass.
    let definitions = data().snapshot().configuration().definitions();
    let one_set_fields: usize = definitions
        .iter()
        .map(|definition| {
            usize::from(definition.defaults.input.is_some())
                + usize::from(definition.defaults.placeholder.is_some())
                + usize::from(definition.defaults.option_index.is_some())
        })
        .sum();
    let limited = prepare_authored_configuration(
        &build,
        &view,
        data(),
        ConfigurationPreparationLimits {
            max_fields: definitions.len() + one_set_fields,
            ..Default::default()
        },
    )
    .err()
    .unwrap();
    assert_eq!(limited.kind, EvaluationErrorKind::InvalidRequest);
    assert!(limited.message.contains("default fields"));
    let expected = serde_json::to_vec(prepared.report()).unwrap();
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let build = build.clone();
            std::thread::spawn(move || {
                let view = resolve_view(
                    &build,
                    data().snapshot(),
                    &ViewRequest::default(),
                    ResolveLimits::default(),
                )
                .unwrap();
                serde_json::to_vec(
                    prepare_authored_configuration(
                        &build,
                        &view,
                        data(),
                        ConfigurationPreparationLimits::default(),
                    )
                    .unwrap()
                    .report(),
                )
                .unwrap()
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), expected);
    }
    for limits in [
        ConfigurationPreparationLimits {
            max_sets: 0,
            ..Default::default()
        },
        ConfigurationPreparationLimits {
            max_fields: 0,
            ..Default::default()
        },
        ConfigurationPreparationLimits {
            max_text_bytes: 0,
            ..Default::default()
        },
    ] {
        let result = prepare_authored_configuration(&build, &view, data(), limits);
        assert!(matches!(result,Err(ref error)if error.kind==EvaluationErrorKind::InvalidRequest));
    }
}
