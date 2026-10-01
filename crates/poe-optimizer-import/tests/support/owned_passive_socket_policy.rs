//! An empty admission domain still commits all source and schema dependencies.
use poe_optimizer_core::{
    data::DataIdentity, owned_content::digest_owned, owned_schema::DefinitionSchemaIndex,
};
use poe_optimizer_import::{
    owned_item_lines::OwnedItemLinePolicy, owned_item_source::ItemSourceLayoutPolicy,
    owned_mapping::OwnedMappingIndex, owned_normalize::*, owned_tree_policy::*,
};

pub fn attach<I: DefinitionSchemaIndex>(
    policy: &mut NormalizationPolicy,
    schema: &I,
    mapping: &OwnedMappingIndex,
    items: &OwnedItemLinePolicy,
    source: &ItemSourceLayoutPolicy,
    tree: &TreeNormalizationContent,
) {
    policy.item_modifier_membership = None;
    policy.item_parameter_inputs = None;
    policy.equipment_membership = Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions: schema.identity().clone(),
        templates: vec![],
        source_base_names: vec!["Reviewed source base".into()],
        loader_jewel_fallback_titles: vec!["Reviewed source title".into()],
    });
    policy.passive_socket_membership = Some(
        PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
            definitions: schema.identity().clone(),
            mapping: *mapping.identity(),
            mapping_source: *mapping.source_identity(),
            item_lines: *items.identity(),
            item_source: *source.identity(),
            equipment: equipment_membership_identity(
                policy.equipment_membership.as_ref().unwrap(),
                Default::default(),
            )
            .unwrap(),
            tree_content: tree_content_identity(tree, Default::default()).unwrap(),
            source_bases: vec![],
            bindings: vec![],
        },
    );
}

pub fn corrupt(policy: &mut NormalizationPolicy, field: usize, other: &DataIdentity) {
    let Some(PassiveSocketMembershipPolicy::PobOrdinarySharedSpecSocketsV2 {
        definitions,
        mapping,
        mapping_source,
        item_lines,
        item_source,
        equipment,
        tree_content,
        ..
    }) = &mut policy.passive_socket_membership
    else {
        panic!()
    };
    if field == 0 {
        *definitions = other.clone();
    } else {
        *[
            mapping,
            mapping_source,
            item_lines,
            item_source,
            equipment,
            tree_content,
        ][field - 1] = digest_owned("stale-passive-placement-dependency", &field, 128).unwrap();
    }
}
