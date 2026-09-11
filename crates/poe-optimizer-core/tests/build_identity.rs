use poe_optimizer_core::build_identity::{
    BuildIdentityError, BuildLineage, BuildRevision, ConfigSetId, InstanceAllocator,
    InstanceAllocatorState, InstanceClone, InstanceId, ItemRecordId, ItemSetId, ItemSlotUseId,
    PassiveSpecId, SkillEntryId, SkillGroupId, SkillSetId,
};
use serde_json::json;

fn lineage() -> BuildLineage {
    BuildLineage::from_bytes([0xe7; 16])
}

#[test]
fn full_width_values_round_trip_as_exact_strings_not_json_numbers() {
    let lineage = BuildLineage::from_bytes([0xff; 16]);
    assert_eq!(
        serde_json::to_value(lineage).unwrap(),
        json!("ffffffffffffffffffffffffffffffff")
    );
    let instance = InstanceId::from_parts(lineage, u64::MAX).unwrap();
    let value = serde_json::to_value(instance).unwrap();
    assert_eq!(
        value,
        json!({"lineage":"ffffffffffffffffffffffffffffffff","local":"ffffffffffffffff"})
    );
    assert_eq!(
        serde_json::from_value::<InstanceId>(value).unwrap(),
        instance
    );
    for n in [0, 1, (1_u64 << 53) + 1, u64::MAX] {
        let revision = BuildRevision::from_u64(n);
        let value = serde_json::to_value(revision).unwrap();
        assert!(value.is_string());
        assert_eq!(
            serde_json::from_value::<BuildRevision>(value).unwrap(),
            revision
        );
        assert_eq!(
            revision.to_string().parse::<BuildRevision>().unwrap(),
            revision
        );
    }
    assert_eq!(
        lineage.to_string().parse::<BuildLineage>().unwrap(),
        lineage
    );
    assert_eq!(lineage.bytes(), [0xff; 16]);
}

#[test]
fn malformed_noncanonical_missing_and_duplicate_wire_fields_reject() {
    for value in [
        json!(0),
        json!(null),
        json!(""),
        json!("0"),
        json!("0x00000000000000000000000000000000"),
        json!("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF"),
        json!("0000000000000000000000000000000g"),
        json!("00000000000000000000000000000000\0"),
        json!("000000000000000000000000000000é"),
    ] {
        assert!(serde_json::from_value::<BuildLineage>(value).is_err());
    }
    for value in [
        json!(1),
        json!("1"),
        json!("FFFFFFFFFFFFFFFF"),
        json!("000000000000000g"),
        json!("00000000000000000"),
    ] {
        assert!(serde_json::from_value::<BuildRevision>(value).is_err());
    }
    for value in [
        json!({"lineage":lineage(),"local":1}),
        json!({"lineage":lineage(),"local":"0000000000000000"}),
        json!({"lineage":lineage()}),
        json!({"local":"0000000000000001"}),
        json!({"lineage":lineage(),"local":"0000000000000001","authority":true}),
    ] {
        assert!(serde_json::from_value::<InstanceId>(value).is_err());
    }
    let duplicate = format!(
        "{{\"lineage\":{},\"local\":\"0000000000000001\",\"local\":\"0000000000000002\"}}",
        serde_json::to_string(&lineage()).unwrap()
    );
    assert!(serde_json::from_str::<InstanceId>(&duplicate).is_err());
    assert!(
        serde_json::from_value::<InstanceAllocatorState>(
            json!({"lineage":lineage(),"last_issued":1})
        )
        .is_err()
    );
    assert!(
        serde_json::from_value::<InstanceAllocatorState>(
            json!({"lineage":lineage(),"last_issued":"0000000000000001","extra":true})
        )
        .is_err()
    );
    assert_eq!(
        InstanceId::from_parts(lineage(), 0),
        Err(BuildIdentityError::ZeroLocal)
    );
}

#[test]
fn all_typed_domains_share_one_unique_local_namespace() {
    let mut allocator = InstanceAllocator::new(lineage());
    let ids = [
        allocator.allocate::<SkillSetId>().unwrap().instance_id(),
        allocator.allocate::<SkillGroupId>().unwrap().instance_id(),
        allocator.allocate::<SkillEntryId>().unwrap().instance_id(),
        allocator.allocate::<ItemSetId>().unwrap().instance_id(),
        allocator.allocate::<ItemRecordId>().unwrap().instance_id(),
        allocator.allocate::<ItemSlotUseId>().unwrap().instance_id(),
        allocator.allocate::<PassiveSpecId>().unwrap().instance_id(),
        allocator.allocate::<ConfigSetId>().unwrap().instance_id(),
    ];
    for (index, id) in ids.into_iter().enumerate() {
        assert_eq!(id.local(), index as u64 + 1);
        assert_eq!(id.lineage(), lineage());
    }
    assert_eq!(allocator.state().last_issued(), 8);
}

#[test]
fn edits_reordering_and_equal_definitions_preserve_occurrences_clones_do_not() {
    let mut allocator = InstanceAllocator::new(lineage());
    let first: SkillEntryId = allocator.allocate().unwrap();
    let second: SkillEntryId = allocator.allocate().unwrap();
    let mut entries = [(first, "same skill"), (second, "same skill")];
    entries.reverse();
    entries[1].1 = "edited skill";
    assert_eq!(allocator.preserve(entries[1].0).unwrap(), first);
    assert_eq!(allocator.preserve(entries[0].0).unwrap(), second);
    assert_ne!(first, second);
    let cloned = allocator.clone_instance(first).unwrap();
    assert_eq!(cloned.origin, first);
    assert_ne!(cloned.id, first);
    assert_ne!(cloned.id, second);
    assert_eq!(cloned.id.local(), 3);
    assert_eq!(
        serde_json::from_value::<SkillEntryId>(serde_json::to_value(first).unwrap()).unwrap(),
        first
    );
    assert_eq!(
        serde_json::from_value::<InstanceClone<SkillEntryId>>(
            serde_json::to_value(cloned).unwrap()
        )
        .unwrap(),
        cloned
    );
}

#[test]
fn saved_record_and_two_slot_uses_remain_three_distinct_occurrences() {
    let mut allocator = InstanceAllocator::new(lineage());
    let record: ItemRecordId = allocator.allocate().unwrap();
    let left: ItemSlotUseId = allocator.allocate().unwrap();
    let right: ItemSlotUseId = allocator.allocate().unwrap();
    let uses = [(left, record), (right, record)];
    assert_eq!(uses[0].1, uses[1].1);
    assert_ne!(uses[0].0, uses[1].0);
    assert_ne!(left.instance_id(), record.instance_id());
    assert_ne!(right.instance_id(), record.instance_id());
}

#[test]
fn persisted_watermark_survives_deletion_of_the_highest_instance() {
    let mut allocator = InstanceAllocator::new(lineage());
    let surviving: SkillGroupId = allocator.allocate().unwrap();
    let removed: SkillGroupId = allocator.allocate().unwrap();
    let checkpoint = serde_json::to_value(allocator.state()).unwrap();
    assert_eq!(checkpoint["last_issued"], "0000000000000002");
    let restored: InstanceAllocatorState = serde_json::from_value(checkpoint).unwrap();
    let mut resumed = InstanceAllocator::from_state(restored);
    assert_eq!(resumed.preserve(surviving).unwrap(), surviving);
    let inserted: SkillGroupId = resumed.allocate().unwrap();
    assert_ne!(inserted, removed);
    assert_eq!(inserted.local(), 3);
    assert_eq!(resumed.state().lineage(), lineage());
}

#[test]
fn foreign_lineage_and_future_ids_fail_without_advancing_the_allocator() {
    let mut allocator = InstanceAllocator::new(lineage());
    let current: SkillSetId = allocator.allocate().unwrap();
    let foreign = SkillSetId::from_instance_id(
        InstanceId::from_parts(BuildLineage::from_bytes([1; 16]), current.local()).unwrap(),
    );
    let saved = allocator.state();
    assert!(matches!(
        allocator.preserve(foreign),
        Err(BuildIdentityError::ForeignLineage { .. })
    ));
    assert!(matches!(
        allocator.clone_instance(foreign),
        Err(BuildIdentityError::ForeignLineage { .. })
    ));
    assert!(matches!(
        allocator.reserve_existing(foreign),
        Err(BuildIdentityError::ForeignLineage { .. })
    ));
    let future = SkillSetId::from_instance_id(InstanceId::from_parts(lineage(), 2).unwrap());
    assert_eq!(
        allocator.preserve(future),
        Err(BuildIdentityError::BeyondWatermark)
    );
    assert_eq!(
        allocator.clone_instance(future),
        Err(BuildIdentityError::BeyondWatermark)
    );
    assert_eq!(allocator.state(), saved);
    assert!(InstanceAllocator::from_existing(lineage(), [foreign.instance_id()]).is_err());
}

#[test]
fn existing_import_order_does_not_change_the_next_identifier() {
    let low = InstanceId::from_parts(lineage(), 2).unwrap();
    let high = InstanceId::from_parts(lineage(), 99).unwrap();
    let mut first = InstanceAllocator::from_existing(lineage(), [low, high]).unwrap();
    let mut second = InstanceAllocator::from_existing(lineage(), [high, low]).unwrap();
    assert_eq!(
        first.allocate::<InstanceId>().unwrap(),
        second.allocate::<InstanceId>().unwrap()
    );
    assert_eq!(first.state().last_issued(), 100);
    first.reserve_existing(low).unwrap();
    assert_eq!(first.state().last_issued(), 100);
}

#[test]
fn instance_and_revision_exhaustion_never_wrap_or_mutate_prior_values() {
    let state = InstanceAllocatorState::from_parts(lineage(), u64::MAX - 1);
    let mut allocator = InstanceAllocator::from_state(state);
    let last: PassiveSpecId = allocator.allocate().unwrap();
    assert_eq!(last.local(), u64::MAX);
    let exhausted = allocator.state();
    assert_eq!(
        allocator.allocate::<PassiveSpecId>(),
        Err(BuildIdentityError::InstanceExhausted)
    );
    assert_eq!(
        allocator.clone_instance(last),
        Err(BuildIdentityError::InstanceExhausted)
    );
    assert_eq!(allocator.preserve(last).unwrap(), last);
    assert_eq!(allocator.state(), exhausted);
    let restored: InstanceAllocatorState =
        serde_json::from_str(&serde_json::to_string(&exhausted).unwrap()).unwrap();
    assert_eq!(
        InstanceAllocator::from_state(restored).allocate::<InstanceId>(),
        Err(BuildIdentityError::InstanceExhausted)
    );
    assert_eq!(BuildRevision::INITIAL.checked_next().unwrap().get(), 1);
    assert_eq!(
        BuildRevision::from_u64(u64::MAX).checked_next(),
        Err(BuildIdentityError::RevisionExhausted)
    );
}

#[test]
fn host_values_are_not_hidden_membership_or_global_allocation_authority() {
    let zero_lineage = BuildLineage::from_bytes([0; 16]);
    let state = InstanceAllocatorState::from_parts(zero_lineage, 10);
    let mut one = InstanceAllocator::from_state(state);
    let mut two = InstanceAllocator::from_state(state);
    assert_eq!(
        one.allocate::<ConfigSetId>().unwrap(),
        two.allocate::<ConfigSetId>().unwrap()
    );
    let unregistered_but_well_formed = InstanceId::from_parts(zero_lineage, 7).unwrap();
    assert_eq!(
        one.preserve(unregistered_but_well_formed).unwrap(),
        unregistered_but_well_formed
    );
    // The imported build rejects absent or domain-mismatched occurrences separately.
    let slots: Vec<ItemSlotUseId> = Vec::new();
    assert!(!slots.contains(&ItemSlotUseId::from_instance_id(
        unregistered_but_well_formed
    )));
}
