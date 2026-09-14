//! Runtime owners survive incomplete preparation; reports remain diagnostics.
#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{ViewRequest, WeaponStateRequest},
    evaluation::*,
    options::EvaluationOptions,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    item_sets::ItemSetReadLimits,
    selected_view::{NumericValue, ResolveLimits, SetOrigin, resolve_view},
};
use poe_optimizer_native::{
    CompiledGameData, HostClock, IncompletePreparation, NativeBackend, PreparationOutcome,
    configuration::{ConfigurationActivation, ConfigurationReadLimits},
    skills::{SkillSetActivation, SkillSetReadLimits},
};
use std::{path::PathBuf, sync::Arc};

fn original(index: u8) -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
        "../../tests/fixtures/builds/breadth-20260908/build-{index:02}.xml"
    )))
    .unwrap()
}
fn import(xml: &str, lineage: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn incomplete(outcome: PreparationOutcome) -> IncompletePreparation {
    match outcome {
        PreparationOutcome::Incomplete(value) => *value,
        PreparationOutcome::Ready(_) => {
            panic!("incomplete source gained unsupported calculation admission")
        }
    }
}
#[test]
fn all_five_originals_retain_actual_producers_and_exact_source_bindings() {
    let backend = NativeBackend::new();
    for index in 1..=5 {
        let xml = original(index);
        let source = import(&xml, index);
        let view = resolve_view(
            &source,
            backend.data().snapshot(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        let pending = incomplete(
            backend
                .prepare_view(&source, &view, &EvaluationOptions::default(), &[])
                .unwrap(),
        );
        pending
            .validate_binding(&source, &view, backend.data())
            .unwrap();
        assert!(pending.source().shares_storage_with(&source));
        assert_eq!(pending.source().source_xml(), xml);
        assert_eq!(pending.data_identity(), backend.data().identity());
        assert_eq!(pending.backend_identity(), &backend.identity());
        assert_eq!(pending.request().build.content, xml);
        assert_eq!(pending.report().schema_version, 6);
        assert!(!pending.report().issues.is_empty());
        assert_eq!(
            serde_json::to_value(pending.selected_view()).unwrap(),
            serde_json::to_value(view.report()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(pending.authored_items().report()).unwrap(),
            serde_json::to_value(pending.report().authored_items.as_ref().unwrap()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(pending.authored_skills().report()).unwrap(),
            serde_json::to_value(pending.report().authored_skills.as_ref().unwrap()).unwrap()
        );
        assert_eq!(
            serde_json::to_value(pending.authored_configuration().report()).unwrap(),
            serde_json::to_value(pending.report().authored_configuration.as_ref().unwrap())
                .unwrap()
        );
        // Exercise each live reader through the retained runtime envelope.
        let skills = pending.authored_skills();
        let mut skill_sets = skills
            .skill_set_view(SkillSetReadLimits::default())
            .unwrap();
        assert!(matches!(
            skill_sets.activation().unwrap(),
            SkillSetActivation::Selected(_)
        ));
        for position in 1..=skill_sets.dense_order_len().unwrap() {
            let key = skill_sets.ordered_key(position).unwrap().unwrap();
            let row = skill_sets.winner(key).unwrap().unwrap();
            if let SetOrigin::Authored {
                instance,
                source: occurrence,
            } = skill_sets.origin(&row).unwrap()
            {
                assert_eq!(source.binding(instance).unwrap().source(), occurrence);
                let title = source.attribute(occurrence, "title").unwrap();
                assert_eq!(
                    skill_sets.title(&row).unwrap(),
                    title.as_ref().map(|text| text.decoded())
                );
            }
            for group_index in 1..=skill_sets.group_count(&row).unwrap() {
                let group = skill_sets.group(&row, group_index).unwrap().unwrap();
                assert!(group.attached);
                assert_eq!(
                    source
                        .binding(AuthoredInstanceId::SkillGroup(group.instance))
                        .unwrap()
                        .source(),
                    group.source
                );
            }
        }
        let configuration = pending.authored_configuration();
        let mut config_sets = configuration.read_view(ConfigurationReadLimits::default());
        let ConfigurationActivation::Selected { key, row } = config_sets.active().unwrap() else {
            panic!("original configuration did not reach its active assignment");
        };
        assert!(row.same_identity(&config_sets.winner(key).unwrap().unwrap()));
        if let SetOrigin::Authored {
            instance,
            source: occurrence,
        } = config_sets.origin(&row).unwrap()
        {
            assert_eq!(source.binding(instance).unwrap().source(), occurrence);
            let title = source.attribute(occurrence, "title").unwrap();
            assert_eq!(
                config_sets.title(&row).unwrap(),
                Some(
                    title.as_ref().map(|text| text.decoded()).unwrap_or(
                        &backend
                            .data()
                            .snapshot()
                            .configuration()
                            .data()
                            .authored_load
                            .default_set_title
                    )
                )
            );
        }
        // These rows come from the retained producer, not report reconstruction.
        let items = pending.authored_items();
        let state = items.item_sets().unwrap();
        let mut read = state.read_view(ItemSetReadLimits::default());
        for position in 1..=read.dense_order_len().unwrap() {
            let key = read.ordered_key(position).unwrap().unwrap();
            if let Some(row) = read.winner(key).unwrap()
                && let SetOrigin::Authored {
                    instance,
                    source: occurrence,
                } = items.item_set_origin(&row).unwrap()
            {
                assert_eq!(source.binding(instance).unwrap().source(), occurrence);
            }
        }
    }
}
#[test]
fn retained_owners_outlive_callers_and_can_be_read_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<IncompletePreparation>();
    let data = Arc::new(
        CompiledGameData::compile(Arc::new(
            CompiledGameData::bundled().unwrap().snapshot().clone(),
        ))
        .unwrap(),
    );
    let weak = Arc::downgrade(&data);
    let expected = original(2);
    let pending = {
        let backend = NativeBackend::with_data(Arc::clone(&data), HostClock).unwrap();
        let mut input = request(&expected);
        let pending = incomplete(
            backend
                .prepare_request_with_lineage(&input, BuildLineage::from_bytes([76; 16]))
                .unwrap(),
        );
        input.build.content.clear();
        assert_eq!(pending.request().build.content, expected);
        pending
    };
    drop(data);
    assert!(weak.upgrade().is_some());
    std::thread::scope(|scope| {
        for _ in 0..4 {
            let pending = &pending;
            let expected = &expected;
            scope.spawn(move || {
                assert_eq!(pending.source().source_xml(), expected);
                let items = pending.authored_items();
                let row = items
                    .item_sets()
                    .unwrap()
                    .read_view(ItemSetReadLimits::default())
                    .active()
                    .unwrap()
                    .unwrap();
                items.item_set_origin(&row).unwrap();
            });
        }
    });
    let serialized = serde_json::to_value(pending.report()).unwrap();
    let report = pending.into_report();
    assert_eq!(serde_json::to_value(report).unwrap(), serialized);
    assert!(
        weak.upgrade().is_none(),
        "report-only conversion must release runtime definition owners"
    );
}
#[test]
fn equal_public_identity_does_not_authorize_foreign_build_data_or_selection() {
    let backend = NativeBackend::new();
    let xml = original(2);
    let source = import(&xml, 77);
    let view = resolve_view(
        &source,
        backend.data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let pending = incomplete(
        backend
            .prepare_view(&source, &view, &EvaluationOptions::default(), &[])
            .unwrap(),
    );
    let foreign = import(&xml, 77);
    let foreign_view = resolve_view(
        &foreign,
        backend.data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert_eq!(
        pending
            .validate_binding(&foreign, &foreign_view, backend.data())
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
    let different_data =
        Arc::new(CompiledGameData::compile(Arc::new(backend.data().snapshot().clone())).unwrap());
    assert_eq!(different_data.identity(), backend.data().identity());
    let data_view = resolve_view(
        &source,
        different_data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert_eq!(
        pending
            .validate_binding(&source, &data_view, &different_data)
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
    let selection = ViewRequest {
        weapon_state: if view.report().weapon_state.use_second_weapon_set == Some(true) {
            WeaponStateRequest::Primary
        } else {
            WeaponStateRequest::Secondary
        },
        ..Default::default()
    };
    let changed_view = resolve_view(
        &source,
        backend.data().snapshot(),
        &selection,
        ResolveLimits::default(),
    )
    .unwrap();
    assert_ne!(
        serde_json::to_value(view.report()).unwrap(),
        serde_json::to_value(changed_view.report()).unwrap()
    );
    assert_eq!(
        pending
            .validate_binding(&source, &changed_view, backend.data())
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
    pending
        .validate_binding(&source, &view, backend.data())
        .unwrap();
}
#[test]
fn reached_source_failure_keeps_live_row_and_preserves_compatibility_error() {
    let xml = "<PathOfBuilding2><Items><ItemSet id='7'><SocketIdURL/></ItemSet></Items></PathOfBuilding2>";
    let backend = NativeBackend::new();
    let pending = incomplete(
        backend
            .prepare_request_with_lineage(&request(xml), BuildLineage::from_bytes([78; 16]))
            .unwrap(),
    );
    let items = pending.authored_items();
    assert!(items.report().failure.as_ref().unwrap().source_error);
    let row = items
        .item_sets()
        .unwrap()
        .read_view(ItemSetReadLimits::default())
        .winner(NumericValue::new(7.0))
        .unwrap()
        .unwrap();
    assert!(matches!(
        items.item_set_origin(&row).unwrap(),
        SetOrigin::Authored { .. }
    ));
    let expected = pending.report().legacy_adapter_error.clone().unwrap();
    let error = PreparationOutcome::Incomplete(Box::new(pending))
        .into_ready()
        .err()
        .unwrap();
    assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
    assert_eq!(error.message, expected);
}
