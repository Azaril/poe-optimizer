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
fn valid_sets_allow_later_items_and_namespace_guards_still_stop_loading() {
    let stage = prepare(&doc(&(ring("1") + "<ItemSet id='1'/>" + &ring("2"))));
    let r = stage.report();
    assert_eq!(r.registration_order.len(), 2);
    assert_eq!(r.records[1].status, ItemRecordStatus::Registered);
    assert!(r.failure.is_none(), "{:#?}", r.failure);
    assert_eq!(
        stage.item_sets().unwrap().phase(),
        poe_optimizer_import::item_sets::ItemSetPhase::AwaitingActivation
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
        // If an item itself stops, no later record may have been processed.
        // These supplied sets are valid: completed inventories now reach activation.
        let mut stopped = false;
        for record in &report.records {
            if stopped {
                assert_eq!(record.status, ItemRecordStatus::NotProcessed);
            }
            stopped |= matches!(
                record.status,
                ItemRecordStatus::Pending
                    | ItemRecordStatus::SourceFailure
                    | ItemRecordStatus::NotProcessed
            );
        }
        if stopped {
            assert!(report.failure.is_some());
        } else {
            assert!(report.failure.is_none(), "{:#?}", report.failure);
            assert_eq!(
                stage.item_sets().unwrap().phase(),
                poe_optimizer_import::item_sets::ItemSetPhase::AwaitingActivation
            );
        }
        assert_eq!(
            report
                .records
                .iter()
                .filter(|r| r.status == ItemRecordStatus::Registered)
                .count(),
            report.registration_order.len()
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
                assert_eq!(report.schema_version, 6);
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
    assert_eq!(report.schema_version, 4);
    assert_eq!(report.registration_order.len(), 3, "{:#?}", report.failure);
    assert!(report.failure.is_none(), "{:#?}", report.failure);
    assert_eq!(
        stage.item_sets().unwrap().phase(),
        poe_optimizer_import::item_sets::ItemSetPhase::AwaitingActivation
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

#[test]
fn production_inventory_registers_weapon_slot_graphs_before_activation() {
    let source = doc(
        "<Item id='1'>Rarity: NORMAL\nWooden Club</Item><Item id='2'>Rarity: NORMAL\nCrude Bow</Item><Item id='3'>Rarity: NORMAL\nMakeshift Crossbow</Item><ItemSet id='1'/>",
    );
    let stage = prepare(&source);
    let report = stage.report();
    assert_eq!(report.registration_order.len(), 3, "{:#?}", report.failure);
    assert!(report.failure.is_none(), "{:#?}", report.failure);
    assert_eq!(
        stage.item_sets().unwrap().phase(),
        poe_optimizer_import::item_sets::ItemSetPhase::AwaitingActivation
    );
    for id in [1.0, 2.0, 3.0] {
        let item = stage.item(stage.registered_id(id).unwrap()).unwrap();
        assert!(item.is_complete());
        let weapons = item
            .field(item.root(), "weaponData")
            .unwrap()
            .as_table()
            .unwrap();
        let slots = &item.table(weapons).unwrap().indexed;
        let main = slots[&1].as_table().unwrap();
        let off = slots[&2].as_table().unwrap();
        assert_ne!(main, off, "weapon slot outputs are distinct owned tables");
        for slot in [main, off] {
            assert!(
                matches!(item.field(slot, "AttackRate"), Some(AssemblyValue::Number(n)) if *n > 0.0)
            );
            assert!(
                matches!(item.field(slot, "TotalDPS"), Some(AssemblyValue::Number(n)) if *n > 0.0)
            );
            assert_eq!(item.field(slot, "ReloadTime").is_some(), id == 3.0);
        }
    }
    assert!(report.frontiers.contains(&"actor_item_effects"));
}

#[test]
fn production_jewel_radius_uses_startup_context_before_saved_tree_selection() {
    use poe_optimizer_import::item_loading::JewelRadiusProvenance;
    let items = "<Items><Item id='1'>Rarity: NORMAL\nRuby\nRadius: Small</Item></Items>";
    // Both root orders load Items before the saved Tree in original Build loading.
    for tree in [
        "<Tree activeSpec='1'><Spec treeVersion='0_1'/></Tree>",
        "<Spec/>",
    ] {
        for xml in [
            format!("<PathOfBuilding2>{tree}{items}</PathOfBuilding2>"),
            format!("<PathOfBuilding2>{items}{tree}</PathOfBuilding2>"),
        ] {
            let stage = prepare(&xml);
            let report = stage.report();
            assert!(report.failure.is_none(), "{:?}", report.failure);
            let context = report.jewel_radius_context.as_ref().unwrap();
            assert_eq!(
                context.provenance,
                JewelRadiusProvenance::BuildInitialization
            );
            assert_eq!(
                context.requested_tree_version,
                data()
                    .snapshot()
                    .item_loading()
                    .policy()
                    .jewel_radius
                    .latest_tree_version
            );
            let item = stage.item(stage.registered_id(1.).unwrap()).unwrap();
            assert_eq!(
                item.field(item.root(), "jewelRadiusIndex")
                    .unwrap()
                    .as_number(),
                Some(1.)
            );
            assert_ne!(context.requested_tree_version, "0_1");
            assert!(
                item.field(item.root(), "jewelData")
                    .unwrap()
                    .as_table()
                    .is_some()
            );
        }
    }
}

#[test]
fn malformed_set_preserves_registered_prefix_and_stops_later_items() {
    use poe_optimizer_import::item_sets::ItemSetPhase;
    for child in [
        "<SocketIdURL/>",
        "<Slot name='id' itemId='1'/>",
        "<RuneSlot slotName='title' runeName='x'/>",
    ] {
        let stage = prepare(&doc(&(ring("1")
            + "<ItemSet id='2'>"
            + child
            + "</ItemSet>"
            + &ring("3"))));
        let report = stage.report();
        assert_eq!(report.registration_order.len(), 1);
        assert_eq!(report.records[1].status, ItemRecordStatus::NotProcessed);
        let failure = report.failure.as_ref().unwrap();
        assert!(failure.source_error, "{failure:?}");
        assert!(failure.source.is_some());
        assert!(failure.instance.is_none());
        assert_eq!(failure.stage, "item_set_loading");
        assert_eq!(stage.item_sets().unwrap().phase(), ItemSetPhase::Failed);
    }
}
#[test]
fn item_failure_prevents_later_set_operations_and_ignored_source_is_not_recursive() {
    use poe_optimizer_import::item_sets::ItemSetPhase;
    let stage = prepare(&doc(
        &(ring("bad") + "<ItemSet id='2'><SocketIdURL/></ItemSet>")
    ));
    assert_eq!(
        stage.report().failure.as_ref().unwrap().stage,
        "item_loading"
    );
    let state = stage.item_sets().unwrap();
    assert_eq!(state.phase(), ItemSetPhase::Loading);
    assert!(
        state.failure().is_none(),
        "unreached malformed set must not execute"
    );
    let stage = prepare(&doc(
        &("<Unconsumed><ItemSet><SocketIdURL/></ItemSet></Unconsumed>text".to_owned() + &ring("1")),
    ));
    assert!(stage.report().failure.is_none());
    assert_eq!(stage.report().registration_order.len(), 1);
    assert_eq!(
        stage.item_sets().unwrap().phase(),
        ItemSetPhase::AwaitingActivation
    );
}
#[test]
fn trade_children_use_attributes_independent_of_name_but_text_is_a_source_error() {
    let stat = data()
        .snapshot()
        .item_assembly()
        .policy()
        .inventory
        .power_stats
        .rows
        .iter()
        .find_map(|row| row.stat.as_deref())
        .unwrap();
    let stage = prepare(&doc(&format!(
        "<TradeSearchWeights><AnyName stat='{stat}' weightMult='2'/></TradeSearchWeights>{}",
        ring("1")
    )));
    assert!(stage.report().failure.is_none());
    assert_eq!(stage.report().registration_order.len(), 1);
    for text in ["text", "<![CDATA[text]]>"] {
        let stage = prepare(&doc(&format!(
            "{}<TradeSearchWeights><AnyName stat='{stat}'/>{text}</TradeSearchWeights>{}",
            ring("1"),
            ring("2")
        )));
        let failure = stage.report().failure.as_ref().unwrap();
        assert!(failure.source_error);
        assert_eq!(failure.stage, "item_set_loading");
        assert_eq!(
            stage.report().records[1].status,
            ItemRecordStatus::NotProcessed
        );
    }
}
#[test]
fn items_and_sets_share_one_cumulative_byte_budget() {
    let xml = doc(&(ring("1") + "<ItemSet id='1'><Slot name='Ring 1' itemId='1'/></ItemSet>"));
    let full = prepare(&xml);
    let state_bytes = full.item_sets().unwrap().usage().bytes;
    let item_bytes: usize = full
        .report()
        .records
        .iter()
        .map(|r| {
            serde_json::to_vec(r.loading_state.as_ref().unwrap())
                .unwrap()
                .len()
                + full.item(r.instance).unwrap().usage().bytes
        })
        .sum();
    assert!(state_bytes > 0 && item_bytes > 0);
    let build = import(&xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let total = state_bytes + item_bytes;
    let exact = prepare_authored_items(
        &build,
        &view,
        data(),
        ItemPreparationLimits {
            max_state_bytes: total,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(exact.report().failure.is_none());
    assert!(
        matches!(prepare_authored_items(&build,&view,data(),ItemPreparationLimits{max_state_bytes:total-1,..Default::default()}),Err(e) if e.kind==EvaluationErrorKind::InvalidRequest)
    );
}

#[test]
fn interleaved_production_loading_preserves_saved_choices_without_activating_them() {
    use poe_optimizer_engine::source_program::{
        ProgramTableId as Id, ProgramValue as V, ProgramValueGraph,
    };
    fn table(value: &V) -> Id {
        match value {
            V::Table(id) => *id,
            other => panic!("expected table: {other:?}"),
        }
    }
    fn get(graph: &ProgramValueGraph, id: Id, key: V) -> &V {
        graph.tables[id.0 as usize - 1]
            .entries
            .iter()
            .find_map(|(k, v)| (k == &key).then_some(v))
            .unwrap_or(&V::Nil)
    }
    fn key(value: &str) -> V {
        V::Bytes(value.as_bytes().to_vec())
    }
    let policy = &data().snapshot().item_assembly().policy().inventory;
    let rune = &policy.layout.rune_slots[0].name;
    let socket = policy.layout.passive.nodes.ids[0];
    let xml = format!(
        "<PathOfBuilding2><Items activeItemSet='0x2' showStatDifferences='false'><Slot name='Charm 1' itemId='4' active='true'/>{}<ItemSet id='2' title='First'/>{}<ItemSet id='0x2' title='Last' useSecondWeaponSet='true'><Slot name='Ring 1' itemId='2' note='chosen'/><RuneSlot slotName='{rune}' runeName='caller rune'/><SocketIdURL nodeId='{socket}' itemPbURL='caller url'/></ItemSet></Items></PathOfBuilding2>",
        ring("1"),
        ring("2")
    );
    let prepared = prepare(&xml);
    assert!(prepared.report().failure.is_none());
    assert_eq!(prepared.report().registration_order.len(), 2);
    let state = prepared.item_sets().unwrap();
    let pending = state.continuation().unwrap();
    assert_eq!(pending.requested_set.value(), Some(2.0));
    assert_eq!(pending.show_stat_differences, Some(false));
    let graph = state.snapshot().unwrap();
    let root = table(&graph.values[0]);
    assert_eq!(get(&graph, root, key("activeItemSetId")), &V::Number(0.0));
    assert_eq!(
        get(&graph, root, key("showStatDifferences")),
        &V::Boolean(policy.defaults.show_stat_differences)
    );
    let previous = table(get(&graph, root, key("previousActiveItemSet")));
    assert_eq!(get(&graph, root, key("activeItemSet")), &V::Table(previous));
    let sets = table(get(&graph, root, key("itemSets")));
    let selected = table(get(&graph, sets, V::Number(2.0)));
    assert_ne!(selected, previous);
    assert_eq!(get(&graph, selected, key("title")), &key("Last"));
    assert_eq!(
        get(&graph, selected, key("useSecondWeaponSet")),
        &V::Boolean(true)
    );
    let order = table(get(&graph, root, key("itemSetOrderList")));
    assert_eq!(get(&graph, order, V::Number(1.0)), &V::Number(2.0));
    assert_eq!(get(&graph, order, V::Number(2.0)), &V::Number(2.0));
    let row = table(get(&graph, selected, key("Ring 1")));
    assert_eq!(get(&graph, row, key("selItemId")), &V::Number(2.0));
    assert_eq!(get(&graph, row, key("note")), &key("chosen"));
    let row = table(get(&graph, selected, key(rune)));
    assert_eq!(get(&graph, row, key("runeName")), &key("caller rune"));
    let row = table(get(&graph, selected, V::Number(f64::from(socket))));
    assert_eq!(get(&graph, row, key("pbURL")), &key("caller url"));
    let slots = table(get(&graph, root, key("slots")));
    let ring = table(get(&graph, slots, key("Ring 1")));
    assert_eq!(
        get(&graph, ring, key("selItemId")),
        &V::Number(0.0),
        "saved choices must not leak into live controls before activation"
    );
    let charm = table(get(&graph, slots, key("Charm 1")));
    assert_eq!(get(&graph, charm, key("selItemId")), &V::Number(4.0));
    assert_eq!(get(&graph, charm, key("active")), &V::Boolean(true));
}

#[test]
fn strict_rune_order_advances_original_items_without_rewriting_saved_runes() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    // Actual previously blocked records, not alternate builds in the production path.
    // Their regular inference has two minimum combinations, but exact descending
    // candidate vectors determine the source's first one. Explicit saved runes
    // must survive even when the inferred names differ from those saved names.
    for (build_number, authored_id) in [(1, "3"), (2, "6")] {
        let xml =
            std::fs::read_to_string(root.join(format!("build-{build_number:02}.xml"))).unwrap();
        let stage = prepare(&xml);
        let record = stage
            .report()
            .records
            .iter()
            .find(|row| row.authored_id.as_deref() == Some(authored_id))
            .unwrap();
        assert_eq!(
            record.status,
            ItemRecordStatus::Registered,
            "build {build_number} item {authored_id}: {:?}",
            stage.report().failure
        );
        let state = record.loading_state.as_ref().unwrap();
        assert_eq!(state.runes, ["Greater Iron Rune", "Greater Iron Rune"]);
        let regular = state
            .rune_mod_lines
            .iter()
            .find(|row| row.line == "36% increased Armour, Evasion and Energy Shield")
            .unwrap();
        assert_eq!(regular.rune_count.and_then(|n| n.value()), Some(2.0));
        assert_eq!(regular.augment_type.as_deref(), Some("Rune"));
        assert!(stage.item(record.instance).unwrap().is_complete());
        assert!(
            stage
                .report()
                .frontiers
                .contains(&"equipment_participation")
        );
    }
}

#[test]
fn original_advanced_unique_flask_registers_with_its_single_line_order() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908/build-02.xml");
    // Read the unchanged complete original through the production coordinator.
    // One explicit line still requires the source lookup/order mutation.
    let xml = std::fs::read_to_string(path).unwrap();
    let prepared = prepare(&xml);
    let record = prepared
        .report()
        .records
        .iter()
        .find(|row| row.authored_id.as_deref() == Some("34"))
        .unwrap();
    assert_eq!(
        record.status,
        ItemRecordStatus::Registered,
        "{:?}",
        prepared.report().failure
    );
    let state = record.loading_state.as_ref().unwrap();
    assert_eq!(state.explicit_mod_lines.len(), 1);
    let line = &state.explicit_mod_lines[0];
    assert_eq!(line.line, "(70-80)% reduced Amount Recovered");
    assert_eq!(line.order.and_then(|number| number.value()), Some(930.0));
    assert_eq!(prepared.registered_id(34.0), Some(record.instance));
    let item = prepared.item(record.instance).unwrap();
    assert!(item.is_complete());
    let rows = item
        .field(item.root(), "explicitModLines")
        .and_then(AssemblyValue::as_table)
        .unwrap();
    let row = item
        .index(rows, 1)
        .and_then(AssemblyValue::as_table)
        .unwrap();
    assert_eq!(
        item.field(row, "order").and_then(AssemblyValue::as_number),
        Some(930.0)
    );
    assert!(
        prepared
            .report()
            .frontiers
            .contains(&"equipment_participation")
    );
}
