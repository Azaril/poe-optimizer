//! The public prepared adapter uses owned inventory winners and injected rules.
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_data::{
    game_data::{GameDataLoader, LoadLimits, TrustPolicy},
    item_loading::{ItemMetadataTable, ItemMetadataValue},
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    item_loading::assembly::{AssemblyError, AssemblyErrorKind},
    item_slot_validity::{SlotValidityContext, SlotValidityLimits, SlotValidityResult, Value},
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{CompiledGameData, items::*};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
};
fn data() -> &'static Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| CompiledGameData::bundled().unwrap())
}
fn prepare(xml: &str, data: &Arc<CompiledGameData>, lineage: u8) -> PreparedItems {
    let build = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let view = resolve_view(
        &build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_items(&build, &view, data, ItemPreparationLimits::default()).unwrap()
}
fn base(item_type: &str) -> String {
    data()
        .snapshot()
        .item_loading()
        .data()
        .bases
        .iter()
        .find(|b| b.item_type == item_type && !b.hidden().unwrap_or(false))
        .unwrap()
        .name
        .clone()
}
fn item(id: &str, name: &str) -> String {
    let name = name.replace('&', "&amp;").replace('<', "&lt;");
    format!("<Item id='{id}'>Rarity: NORMAL\n{name}</Item>")
}
fn doc(items: &str) -> String {
    format!("<PathOfBuilding2><Items>{items}</Items></PathOfBuilding2>")
}
fn truth(result: SlotValidityResult<'_>) -> bool {
    match result {
        SlotValidityResult::NoValues => false,
        SlotValidityResult::Value(value) => value.truthy(),
    }
}
fn selection(slot: &str, id: ItemMetadataValue) -> ItemMetadataTable {
    ItemMetadataTable {
        fields: BTreeMap::from([(
            slot.into(),
            ItemMetadataValue::Table(ItemMetadataTable {
                fields: BTreeMap::from([("selItemId".into(), id)]),
                indexed: BTreeMap::new(),
            }),
        )]),
        indexed: BTreeMap::new(),
    }
}
struct MissingContext;
impl<'a> SlotValidityContext<'a> for MissingContext {}
struct NoCalcs;
impl<'a> SlotValidityContext<'a> for NoCalcs {
    fn has_calculation_environment(&mut self) -> Result<bool, AssemblyError> {
        Ok(false)
    }
}
struct EmptyNodes;
impl<'a> SlotValidityContext<'a> for EmptyNodes {
    fn tree_node(&mut self, _: Value<'_>) -> Result<Value<'a>, AssemblyError> {
        Ok(Value::Nil)
    }
    fn effective_node(&mut self, _: Value<'_>) -> Result<Value<'a>, AssemblyError> {
        Ok(Value::Nil)
    }
}
#[test]
fn prepared_queries_reuse_storage_remain_read_only_and_run_in_parallel() {
    let stage = prepare(&doc(&item("1", "Gold Ring")), data(), 210);
    let id = stage.registered_id(1.0).unwrap();
    let before = serde_json::to_value(stage.report()).unwrap();
    let graph = stage.item(id).unwrap().clone();
    let program = stage.slot_evaluator(SlotValidityLimits::default()).unwrap();
    let clone = program.clone();
    assert!(program.shares_program_with(&clone));
    let set = ItemMetadataTable::default();
    std::thread::scope(|scope| {
        let mut handles = Vec::new();
        for evaluator in [&program, &clone] {
            let set = &set;
            handles.push(scope.spawn(move || {
                let mut context = MissingContext;
                for _ in 0..32 {
                    assert!(truth(
                        evaluator
                            .check(
                                PreparedItemSlotRequest {
                                    item: id,
                                    slot_name: "Ring 1",
                                    item_set: Value::table(set),
                                    flag_state: Value::Nil,
                                },
                                &mut context
                            )
                            .unwrap()
                    ));
                }
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
    });
    assert_eq!(serde_json::to_value(stage.report()).unwrap(), before);
    assert!(graph.shares_storage_with(stage.item(id).unwrap()));
    assert!(
        stage
            .report()
            .frontiers
            .contains(&"equipment_participation")
    );
}
#[test]
fn foreign_or_unregistered_records_fail_without_becoming_invalid_equipment() {
    let xml = doc(&item("1", "Gold Ring"));
    let stage = prepare(&xml, data(), 211);
    let foreign = prepare(&xml, data(), 212);
    let program = stage.slot_evaluator(SlotValidityLimits::default()).unwrap();
    let set = ItemMetadataTable::default();
    let error = program
        .check(
            PreparedItemSlotRequest {
                item: foreign.registered_id(1.0).unwrap(),
                slot_name: "Ring 1",
                item_set: Value::table(&set),
                flag_state: Value::Nil,
            },
            &mut MissingContext,
        )
        .unwrap_err();
    assert_eq!(error.kind, AssemblyErrorKind::Unsupported);
}
#[test]
fn registered_inventory_winners_override_external_context_and_do_not_coerce_keys() {
    let bow = base("Bow");
    let quiver = base("Quiver");
    let a = prepare(&doc(&(item("9", &bow) + &item("2", &quiver))), data(), 213);
    let b = prepare(
        &doc(&(item("9", &bow) + &item("9", "Wooden Club") + &item("2", &quiver))),
        data(),
        214,
    );
    let named = selection("Weapon 1", ItemMetadataValue::Number(9.0));
    for (stage, expected) in [(&a, true), (&b, false)] {
        let program = stage.slot_evaluator(SlotValidityLimits::default()).unwrap();
        assert_eq!(
            truth(
                program
                    .check(
                        PreparedItemSlotRequest {
                            item: stage.registered_id(2.0).unwrap(),
                            slot_name: "Weapon 2",
                            item_set: Value::table(&named),
                            flag_state: Value::Nil,
                        },
                        &mut NoCalcs
                    )
                    .unwrap()
            ),
            expected
        );
    }
    let text_key = selection("Weapon 1", ItemMetadataValue::Text("9".into()));
    let program = a.slot_evaluator(SlotValidityLimits::default()).unwrap();
    assert!(!truth(
        program
            .check(
                PreparedItemSlotRequest {
                    item: a.registered_id(2.0).unwrap(),
                    slot_name: "Weapon 2",
                    item_set: Value::table(&text_key),
                    flag_state: Value::Nil,
                },
                &mut NoCalcs
            )
            .unwrap()
    ));
    let error = program
        .check(
            PreparedItemSlotRequest {
                item: a.registered_id(2.0).unwrap(),
                slot_name: "Weapon 2",
                item_set: Value::table(&named),
                flag_state: Value::Nil,
            },
            &mut MissingContext,
        )
        .unwrap_err();
    assert_eq!(error.kind, AssemblyErrorKind::Unsupported);
}
#[test]
fn changed_definitions_drive_the_prepared_adapter_without_global_default_rules() {
    let mut package = data().snapshot().package().clone();
    package.item_assembly.policy.slot_validity.jewel.slot_type = "Ring".into();
    package.item_assembly.policy.slot_validity.jewel.item_type = "Ring".into();
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let custom = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
    let stage = prepare(&doc(&item("1", "Gold Ring")), &custom, 215);
    let program = stage.slot_evaluator(SlotValidityLimits::default()).unwrap();
    let set = ItemMetadataTable::default();
    assert!(!truth(
        program
            .check(
                PreparedItemSlotRequest {
                    item: stage.registered_id(1.0).unwrap(),
                    slot_name: "Ring 1",
                    item_set: Value::table(&set),
                    flag_state: Value::Nil,
                },
                &mut EmptyNodes
            )
            .unwrap()
    ));
    assert_ne!(stage.report().data_identity, *data().identity());
}
