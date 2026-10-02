//! V3 empty domains still bind every reviewed source and inherited item artifact.
use poe_optimizer_core::owned_content::digest_owned;
use poe_optimizer_import::{
    owned_item_lines::OwnedItemLinePolicy, owned_item_source::ItemSourceLayoutPolicy,
    owned_mapping::OwnedMappingIndex, owned_normalize::*,
};

pub fn upgrade(
    policy: &mut NormalizationPolicy,
    mapping: &OwnedMappingIndex,
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
) {
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
    }) = policy.equipment_membership.take()
    else {
        panic!()
    };
    policy.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines: *items.identity(),
            item_source: *source.identity(),
            imported_profiles: vec![],
            empty_character_runes: EmptyCharacterRuneSelections {
                mapping_source: *mapping.source_identity(),
                slot_names: vec![],
                explicit_empty_selection: "Vacant".into(),
            },
        },
    );
    rebind_equipment(policy);
}
fn rebind_equipment(policy: &mut NormalizationPolicy) {
    let digest = equipment_membership_identity(
        policy.equipment_membership.as_ref().unwrap(),
        Default::default(),
    )
    .unwrap();
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 { equipment, .. }) =
        &mut policy.passive_socket_membership
    else {
        panic!()
    };
    *equipment = digest;
}
pub fn corrupt(policy: &mut NormalizationPolicy, field: usize) {
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        definitions,
        item_lines,
        item_source,
        empty_character_runes,
        ..
    }) = &mut policy.equipment_membership
    else {
        panic!()
    };
    let stale = digest_owned("stale-empty-character-rune-dependency", &field, 128).unwrap();
    match field {
        0 => definitions.release = "stale-rune-release".into(),
        1 => *item_lines = stale,
        2 => *item_source = stale,
        3 => empty_character_runes.mapping_source = stale,
        _ => unreachable!(),
    }
    // Isolate the V3 commitment: its dependent passive digest is kept coherent.
    rebind_equipment(policy);
}
