//! Joins actual authored item properties to the existing finite Player replay.
//! Only test-local inventories are completed; production remains Partial.
use super::{
    equipment, evasion_family, evasion_native, family as item_family, item_native,
    replay::ReplayInput,
};
use item_family::{def, key, subject};
use poe_optimizer_core::{
    build_identity::InstanceAllocator, owned_build::*, owned_definitions::*,
    owned_draft::ItemDraft, owned_rules::*, owned_schema::*, owned_stages::*,
};
use poe_optimizer_engine::owned_plan::*;
use rayon::prelude::*;
use serde_json::{Value, json};

fn schema_mut(i: &mut ReplayInput, template: ItemTemplateDefId) -> &mut ItemTemplateSchema {
    let d = i
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == template.address())
        .unwrap();
    let DefinitionDescriptor::ItemTemplate(d) = d else {
        panic!("item template")
    };
    let SchemaState::Known(s) = &mut d.schema else {
        panic!("known template")
    };
    s
}
pub(crate) fn fixture() -> ReplayInput {
    let mut i = evasion_native::fixture();
    item_native::install(&mut i);
    let d: Value = evasion_family::read("dependencies.json");
    // The earlier replay did not demand Helmet quality. Supply the exact saved
    // property input now, preserving any already recorded matching quality.
    for item in &mut i.build.items {
        if ![
            def::<ItemTemplateDefinition>(0x1f1c),
            def(0x2007),
            def(0x238c),
        ]
        .contains(&item.template)
        {
            continue;
        }
        let saved: Vec<_> = d["selected_snapshot"]["items"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["template"]["value"] == json!(item.template))
            .collect();
        assert_eq!(saved.len(), 1);
        let saved: ItemDraft = serde_json::from_value(saved[0].clone()).unwrap();
        let quality = saved.quality.to_resolved().unwrap();
        if item.quality.is_some() {
            assert_eq!(item.quality, quality);
        }
        item.quality = quality;
    }
    let source = d["selected_snapshot"]["items"]["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["template"]["value"] == json!(def::<ItemTemplateDefinition>(0x1e0e)))
        .unwrap();
    let source: ItemDraft = serde_json::from_value(source.clone()).unwrap();
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    let id = allocator.allocate().unwrap();
    let use_id = allocator.allocate().unwrap();
    let template = source.template.to_resolved().unwrap();
    let declared = schema_mut(&mut i, template.clone())
        .declarations
        .parameters
        .members
        .clone();
    let mut parameters = source.parameters.to_resolved().unwrap();
    parameters.retain(|p| declared.contains(&p.slot));
    // These are the exact imported property inputs and physical slot. Other
    // modifier responsibilities of this saved item remain outside this finite
    // component and are not cleared in production data.
    i.build.items.push(ItemRecord {
        id,
        template: template.clone(),
        parameters,
        item_level: source.item_level.to_resolved().unwrap(),
        quality: source.quality.to_resolved().unwrap(),
        modifiers: vec![],
        modifier_orders: poe_optimizer_core::owned_build::ItemModifierOrders {
            contributions: vec![],
            numeric_transforms: vec![],
        },
    });
    assert!(
        !i.build
            .equipment
            .iter()
            .any(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x69)))
    );
    i.build.equipment.push(EquipmentUse {
        id: use_id,
        item: id,
        destination: EquipmentDestination::CharacterSlot(def(0x69)),
        scope: LoadoutScope::Shared,
    });
    i.build.allocator = allocator.state();
    let property: Value = item_family::read("dependencies.json");
    let modifiers: Vec<ModifierDefId> = property["families"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| serde_json::from_value(f["canonical"].clone()).unwrap())
        .collect();
    for template in [0x1f1c, 0x1e0e, 0x2007, 0x238c] {
        let s = schema_mut(&mut i, def(template));
        for m in &modifiers {
            if !s.modifiers.members.contains(m) {
                s.modifiers.members.push(m.clone());
            }
        }
        s.quality.allowed_kinds = DeclaredSet::complete(vec![def(6)]);
        s.modifiers.closure = SchemaClosure::Complete;
        s.declarations.parameters.closure = SchemaClosure::Complete;
        s.declarations.choices.closure = SchemaClosure::Complete;
        s.declarations.grants.closure = SchemaClosure::Complete;
        s.declarations.actors.closure = SchemaClosure::Complete;
        s.declarations.skill_grants.closure = SchemaClosure::Complete;
        s.declarations.outputs.closure = SchemaClosure::Complete;
        s.declarations.sockets.closure = SchemaClosure::Complete;
        s.socket_destinations = DeclaredSet::complete(vec![]);
        i.rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(def::<ItemTemplateDefinition>(template)))
            .unwrap()
            .programs
            .closure = SchemaClosure::Complete;
    }
    for modifier in modifiers {
        let descriptor = i
            .schema
            .definitions
            .iter_mut()
            .find(|d| d.address() == modifier.address())
            .unwrap();
        let DefinitionDescriptor::Modifier(row) = descriptor else {
            unreachable!()
        };
        let SchemaState::Known(s) = &mut row.schema else {
            unreachable!()
        };
        s.declarations.parameters.closure = SchemaClosure::Complete;
        i.rules
            .owners
            .iter_mut()
            .find(|o| o.owner == subject(modifier.clone()))
            .unwrap()
            .programs
            .closure = SchemaClosure::Complete;
    }
    install(&mut i);
    i.rebind_test_edit().unwrap();
    i
}
pub fn install(i: &mut ReplayInput) {
    for d in equipment::definitions() {
        assert!(
            !i.schema
                .definitions
                .iter()
                .any(|x| x.address() == d.address())
        );
        i.schema.definitions.push(d);
    }
    let c = equipment::consumer();
    i.rules.owners.push(c.subtotal_owner);
    i.stages.stages.push(EvaluationStage {
        id: key(equipment::STAGE),
        predecessors: vec![key("item-evasion-final"), key("deliver")],
    });
    let final_stage = i
        .stages
        .stages
        .iter_mut()
        .find(|s| s.id == key("player-evasion-final"))
        .unwrap();
    final_stage.predecessors.push(key(equipment::STAGE));
    evasion_native::declare(
        i,
        subject(def::<ActorDefinition>(0x332a)),
        c.slot_program,
        equipment::STAGE,
    );
    let old = i
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.final_owner.owner)
        .unwrap();
    assert_eq!(
        old.programs.members,
        evasion_family::consumer().owner.programs.members
    );
    old.programs.members = c.final_owner.programs.members;
    for (scope, stat, stage) in [
        (
            RuleEntityKind::EquipmentUse,
            equipment::PROPERTY,
            "item-evasion-final",
        ),
        (RuleEntityKind::Actor, equipment::SUBTOTAL, equipment::STAGE),
    ] {
        i.stages.frozen_channels.push(FrozenStageChannel {
            channel: StageChannel::Stat {
                scope,
                stat: def(stat),
            },
            stage: key(stage),
        });
    }
}
fn selected_item(i: &ReplayInput, slot: u64) -> poe_optimizer_core::build_identity::ItemRecordId {
    i.build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(slot)))
        .unwrap()
        .item
}
pub(crate) fn positive(i: &mut ReplayInput, slot: u64) {
    let item = selected_item(i, slot);
    for (m, n) in [
        (0x2a63, 13.),
        (0x2a97, 17.),
        (0x2b45, 31.),
        (0x2b77, 37.),
        (0x2ba9, 41.),
    ] {
        item_native::add_modifier_to_item(i, item, m, n);
    }
}
pub(crate) fn subtotal(report: &SupportEffectsReport) -> EffectValue {
    let SupportEffectsOutcome::Evaluated { effects } = &report.outcome else {
        panic!("finite component unavailable")
    };
    effects
        .values
        .iter()
        .find(|v| {
            v.key
                == PlanValueKey::Stat {
                    entity: ConcreteEntity::Actor(ActorKey::Player),
                    stat: def(equipment::SUBTOTAL),
                }
        })
        .unwrap()
        .value
        .clone()
}
fn known_subtotal(report: &SupportEffectsReport, n: f64) {
    assert_eq!(
        subtotal(report),
        EffectValue::Known {
            value: ParameterValue::Quantity(
                FiniteQuantity::new(n, def(evasion_family::UNIT)).unwrap()
            )
        }
    );
}
#[test]
fn actual_properties_feed_saved_player_evasion_and_positive_hybrid_controls() {
    let mut i = fixture();
    let r = evasion_native::evaluate(&i);
    evasion_native::known(&r, 10.);
    known_subtotal(&r, 0.);
    positive(&mut i, 0x66);
    i.rebind_test_edit().unwrap();
    let r = evasion_native::evaluate(&i);
    evasion_native::known(&r, 119.);
    known_subtotal(&r, 108.75);
    positive(&mut i, 0x69);
    i.rebind_test_edit().unwrap();
    let r = evasion_native::evaluate(&i);
    evasion_native::known(&r, 228.);
    known_subtotal(&r, 217.5);
    // Item-local75 is independently retained source evidence. The two-slot
    // joined total228 is a synthetic native composition control, not a new VM.
    let id = selected_item(&i, 0x66);
    i.build
        .items
        .iter_mut()
        .find(|x| x.id == id)
        .unwrap()
        .quality
        .as_mut()
        .unwrap()
        .amount = FiniteQuantity::new(0., def(2)).unwrap();
    i.rebind_test_edit().unwrap();
    // Local Helmet becomes63: gear contributes200.1, then the unchanged
    // intrinsic7 contributes10.15 before final rounding to210.
    let report = evasion_native::evaluate(&i);
    known_subtotal(&report, 200.1);
    evasion_native::known(&report, 210.);
}
#[test]
fn empty_slots_are_lazy_but_occupied_missing_properties_are_not_zero() {
    let mut i = fixture();
    let c = item_family::consumer();
    let final_receiver = c
        .receivers
        .iter()
        .find(|r| r.stat == def(equipment::PROPERTY))
        .unwrap();
    i.rules
        .receivers
        .members
        .retain(|r| r.id != final_receiver.id);
    i.rebind_test_edit().unwrap();
    let r = evasion_native::evaluate(&i);
    assert!(matches!(
        evasion_native::value(&r),
        EffectValue::Unresolved { .. }
    ));
    i.build.equipment.retain(|e|!matches!(&e.destination,EquipmentDestination::CharacterSlot(s)if[def::<EquipmentSlotDefinition>(0x66),def(0x68),def(0x69),def(0x67),def(0x65)].contains(s)));
    i.rebind_test_edit().unwrap();
    let r = evasion_native::evaluate(&i);
    evasion_native::known(&r, 10.);
    known_subtotal(&r, 0.);
}
#[test]
fn present_zero_item_replacement_remains_unavailable_to_the_occupied_slot() {
    let mut i = fixture();
    let owner = subject(def::<ItemTemplateDefinition>(0x1f1c));
    let program = key("synthetic.slot-evasion-replacement");
    let effect = key("replacement");
    item_native::declare(
        &mut i,
        owner.clone(),
        RuleProgram {
            id: program.clone(),
            context: RuleEntityKind::EquipmentUse,
            reads: vec![],
            nodes: vec![RuleNode {
                id: key("zero"),
                expression: RuleExpression::Literal {
                    value: ParameterValue::Quantity(
                        FiniteQuantity::new(0., def(evasion_family::UNIT)).unwrap(),
                    ),
                },
            }],
            effects: vec![RuleEffect {
                id: effect.clone(),
                when: None,
                effect: RuleEffectKind::Contribute {
                    entity: RuleEntity::Current,
                    stat: def(0x33a8),
                    contribution: ContributionKind::Override,
                    value: key("zero"),
                },
            }],
        },
        "item-evasion-delivery",
    );
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id.as_str() == "item-evasion-replacement")
        .unwrap()
        .groups[0]
        .members
        .members
        .push(ContributionMember {
            producer: ContributionProducer::ProgramEffect(ProgramContributionProducer {
                owner,
                program,
                effect,
                origin: ContributionOrigin::EquipmentUse {
                    slots: vec![def(0x66)],
                },
            }),
            order: None,
        });
    i.rebind_test_edit().unwrap();
    let report = evasion_native::evaluate(&i);
    assert!(matches!(
        evasion_native::value(&report),
        EffectValue::Unresolved { .. }
    ));
    assert!(matches!(subtotal(&report), EffectValue::Unresolved { .. }));
}
#[test]
fn ambiguous_active_equipment_is_not_collapsed_by_equal_item_identity() {
    let mut i = fixture();
    let mut duplicate = i
        .build
        .equipment
        .iter()
        .find(|e| e.destination == EquipmentDestination::CharacterSlot(def(0x66)))
        .unwrap()
        .clone();
    let mut allocator = InstanceAllocator::from_state(i.build.allocator);
    duplicate.id = allocator.allocate().unwrap();
    i.build.allocator = allocator.state();
    i.build.equipment.push(duplicate);
    i.rebind_test_edit().unwrap();
    let report = evasion_native::evaluate(&i);
    assert!(matches!(
        evasion_native::value(&report),
        EffectValue::Unresolved { .. }
    ));
}
#[test]
fn published_partial_owner_and_late_property_are_still_blocking() {
    let base = fixture();
    let mut partial = base.clone();
    let c = equipment::consumer();
    partial
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.final_owner.owner)
        .unwrap()
        .programs
        .closure = c.final_owner.programs.closure;
    partial.rebind_test_edit().unwrap();
    let p = partial.compile().unwrap();
    assert!(matches!(
        p.evaluate(&mut p.new_scratch()).unwrap().outcome,
        SupportEffectsOutcome::Unavailable { .. }
    ));
    let mut late = base;
    late.stages.frozen_channels.iter_mut().find(|f|matches!(&f.channel,StageChannel::Stat{scope:RuleEntityKind::EquipmentUse,stat}if *stat==def(equipment::PROPERTY))).unwrap().stage=key("player-evasion-final");
    assert!(late.rebind_test_edit().is_err());
}
#[test]
fn positive_slot_join_preserves_equipment_identity_reorder_and_scratch() {
    let mut i = fixture();
    positive(&mut i, 0x66);
    positive(&mut i, 0x69);
    i.rebind_test_edit().unwrap();
    let p = i.compile().unwrap();
    let mut scratch = p.new_scratch();
    let expected = p.evaluate(&mut scratch).unwrap();
    evasion_native::known(&expected, 228.);
    i.build.equipment.reverse();
    i.build.items.reverse();
    i.rebind_test_edit().unwrap();
    let other = i.compile().unwrap();
    assert_eq!(other.evaluate(&mut scratch).unwrap(), expected);
    assert_eq!(p.evaluate(&mut scratch).unwrap(), expected);
    let parallel: Vec<_> = (0..8)
        .into_par_iter()
        .map_init(|| p.new_scratch(), |s, _| p.evaluate(s).unwrap())
        .collect();
    assert!(parallel.iter().all(|r| r == &expected));
}
