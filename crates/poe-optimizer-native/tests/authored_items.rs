#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_core::{
    build_identity::BuildLineage, build_view::ViewRequest, evaluation::EvaluationErrorKind,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    item_loading::assembly::AssemblyValue,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{CompiledGameData, items::*};
use std::sync::{Arc, OnceLock};
fn data() -> &'static Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| CompiledGameData::bundled().unwrap())
}
fn import(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([119; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn prepare(xml: &str) -> PreparedItems {
    let build = import(xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_items(&build, &view, data(), ItemPreparationLimits::default()).unwrap()
}
fn doc(items: &str) -> String {
    format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>")
}
fn ring(id: &str) -> String {
    format!("<Item id='{id}'>Rarity: NORMAL\nGold Ring</Item>")
}
#[test]
fn production_provider_registers_owned_ring_with_all_three_slot_lists() {
    let stage = prepare(&doc(&ring("9")));
    let report = stage.report();
    assert!(report.failure.is_none(), "{:#?}", report.failure);
    assert_eq!(report.records[0].status, ItemRecordStatus::Registered);
    let id = stage.registered_id(9.0).unwrap();
    let item = stage.item(id).unwrap();
    assert!(item.is_complete());
    let slots = item
        .field(item.root(), "slotModList")
        .and_then(AssemblyValue::as_table)
        .unwrap();
    assert_eq!(item.table(slots).unwrap().indexed.len(), 3);
    for slot in 1..=3 {
        assert!(matches!(
            item.index(slots, slot),
            Some(AssemblyValue::Table(_))
        ));
    }
    assert!(report.frontiers.contains(&"equipment_participation"));
}
#[test]
fn duplicate_numeric_ids_preserve_order_and_replace_only_lookup_winner() {
    let stage = prepare(&doc(&(ring("1") + &ring("0x1"))));
    let r = stage.report();
    assert!(r.failure.is_none(), "{:#?}", r.failure);
    assert_eq!(r.registration_order.len(), 2);
    assert_ne!(r.registration_order[0], r.registration_order[1]);
    assert_eq!(stage.registered_id(1.0), Some(r.registration_order[1]));
    for id in &r.registration_order {
        assert!(stage.item(*id).unwrap().is_complete());
    }
}
#[test]
fn absent_base_is_skipped_and_invalid_registration_retains_prior_prefix() {
    let stage = prepare(&doc(&("<Item id='0'>No matching item base</Item>"
        .to_owned()
        + &ring("1")
        + &ring("invalid")
        + &ring("3"))));
    let r = stage.report();
    assert_eq!(
        r.records.iter().map(|r| r.status).collect::<Vec<_>>(),
        [
            ItemRecordStatus::NoBase,
            ItemRecordStatus::Registered,
            ItemRecordStatus::SourceFailure,
            ItemRecordStatus::NotProcessed
        ]
    );
    assert_eq!(r.registration_order.len(), 1);
    assert!(r.failure.as_ref().unwrap().source_error);
    assert!(stage.registered_id(3.0).is_none());
}
#[test]
fn nonitem_source_continuation_stops_before_later_items() {
    let stage = prepare(&doc(&(ring("1") + "<ItemSet id='1'/>" + &ring("2"))));
    let r = stage.report();
    assert_eq!(r.registration_order.len(), 1);
    assert_eq!(r.records[1].status, ItemRecordStatus::NotProcessed);
    assert_eq!(
        r.failure.as_ref().unwrap().stage,
        "item_container_continuation"
    );
    let ns = prepare(
        "<PathOfBuilding2><Items xmlns='urn:unhandled'><Item id='1'>Rarity: NORMAL\nGold Ring</Item></Items></PathOfBuilding2>",
    );
    assert_eq!(
        ns.report().failure.as_ref().unwrap().stage,
        "item_namespace_context"
    );
    assert!(ns.report().registration_order.is_empty());
}
#[test]
fn owner_binding_rejects_equal_source_in_another_import_or_data_owner() {
    let xml = doc(&ring("1"));
    let build = import(&xml);
    let foreign = import(&xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let stage =
        prepare_authored_items(&build, &view, data(), ItemPreparationLimits::default()).unwrap();
    stage.validate_binding(&build, &view, data()).unwrap();
    let foreign_view = resolve_view(
        &foreign,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert_eq!(
        stage
            .validate_binding(&foreign, &foreign_view, data())
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
    let other_data =
        Arc::new(CompiledGameData::compile(Arc::new(data().snapshot().clone())).unwrap());
    assert_eq!(
        stage
            .validate_binding(&build, &view, &other_data)
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
}
#[test]
fn limits_do_not_return_truncated_success_reports() {
    let xml = doc(&ring("1"));
    let build = import(&xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    for limits in [
        ItemPreparationLimits {
            max_items: 0,
            ..Default::default()
        },
        ItemPreparationLimits {
            max_instructions: 0,
            ..Default::default()
        },
        ItemPreparationLimits {
            max_state_bytes: 1,
            ..Default::default()
        },
    ] {
        let result = prepare_authored_items(&build, &view, data(), limits);
        assert!(matches!(result,Err(e) if e.kind==EvaluationErrorKind::InvalidRequest));
    }
}
#[test]
fn shared_data_preparation_is_thread_safe_and_item_artifacts_remain_independent() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparedItems>();
    let xml = doc(&ring("1"));
    let sequential = prepare(&xml);
    let expected = serde_json::to_value(sequential.report()).unwrap();
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..2).map(|_| scope.spawn(|| prepare(&xml))).collect();
        for h in handles {
            let other = h.join().unwrap();
            assert_eq!(serde_json::to_value(other.report()).unwrap(), expected);
            let a = sequential
                .item(sequential.registered_id(1.0).unwrap())
                .unwrap();
            let b = other.item(other.registered_id(1.0).unwrap()).unwrap();
            assert!(!a.shares_storage_with(b));
        }
    });
}
#[test]
fn all_original_items_stay_in_report_even_when_the_first_dependency_stops() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let mut total = 0;
    for n in 1..=5 {
        let xml = std::fs::read_to_string(root.join(format!("build-{n:02}.xml"))).unwrap();
        let build = import(&xml);
        let expected = build
            .project_items()
            .unwrap()
            .containers()
            .iter()
            .flat_map(|c| c.children())
            .filter(|n| n.kind() == poe_optimizer_import::item_source::ItemSourceKind::Item)
            .count();
        let stage = prepare(&xml);
        let report = stage.report();
        assert_eq!(report.records.len(), expected);
        total += expected;
        assert!(report.failure.is_some());
        assert!(
            report
                .records
                .iter()
                .any(|r| r.status == ItemRecordStatus::NotProcessed)
        );
        assert!(report.frontiers.contains(&"actor_item_effects"));
    }
    assert_eq!(total, 116);
}

#[test]
fn public_preparation_retains_inventory_stage_for_ready_and_incomplete_results() {
    use poe_optimizer_core::{
        evaluation::{BuildDocument, BuildFormat, EvaluationRequest},
        options::EvaluationOptions,
    };
    use poe_optimizer_native::{HostClock, NativeBackend, PreparationOutcome};
    let backend = NativeBackend::with_data(Arc::clone(data()), HostClock).unwrap();
    let spark = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
    for (xml, ready) in [(spark.to_owned(), true), (doc(&ring("1")), false)] {
        let request = EvaluationRequest {
            build: BuildDocument {
                format: BuildFormat::PathOfBuilding2Xml,
                content: xml,
            },
            options: EvaluationOptions::default(),
            metrics: vec![],
        };
        match backend
            .prepare_request_with_lineage(&request, BuildLineage::from_bytes([118; 16]))
            .unwrap()
        {
            PreparationOutcome::Ready(prepared) => {
                assert!(ready);
                assert_eq!(
                    prepared.authored_items().report().source_sha256,
                    prepared.source().source_sha256()
                );
            }
            PreparationOutcome::Incomplete(report) => {
                assert!(!ready);
                assert_eq!(report.schema_version, 5);
                let items = report.authored_items.as_ref().unwrap();
                assert_eq!(items.records[0].status, ItemRecordStatus::Registered);
                assert_eq!(items.source_sha256, report.view.source_sha256);
                assert!(items.frontiers.contains(&"actor_item_effects"));
            }
        }
    }
}

#[test]
fn production_inventory_registers_local_family_items_before_item_set_activation() {
    let source = doc(
        "<Item id='1'>Rarity: NORMAL\nRusted Cuirass</Item><Item id='2'>Rarity: NORMAL\nUltimate Life Flask</Item><Item id='3'>Rarity: NORMAL\nRuby Charm</Item><ItemSet id='1'/>",
    );
    let stage = prepare(&source);
    let report = stage.report();
    assert_eq!(report.schema_version, 2);
    assert_eq!(report.registration_order.len(), 3, "{:#?}", report.failure);
    assert_eq!(
        report.failure.as_ref().unwrap().stage,
        "item_container_continuation"
    );
    for (id, field) in [(1.0, "armourData"), (2.0, "flaskData"), (3.0, "charmData")] {
        let item = stage.item(stage.registered_id(id).unwrap()).unwrap();
        let local = item
            .field(item.root(), field)
            .and_then(AssemblyValue::as_table)
            .unwrap();
        assert!(!item.table(local).unwrap().fields.is_empty());
        assert!(item.is_complete());
    }
    assert!(
        report
            .records
            .iter()
            .all(|r| r.status == ItemRecordStatus::Registered)
    );
    assert!(report.frontiers.contains(&"actor_item_effects"));
}
