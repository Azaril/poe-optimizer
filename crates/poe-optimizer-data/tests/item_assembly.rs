use poe_optimizer_data::{
    game_data::{GameDataLoader, GameDataSnapshot, LoadLimits, TrustPolicy, bundled_snapshot},
    item_assembly::*,
};
use std::{collections::BTreeMap, sync::OnceLock};
fn snapshot() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn data() -> ItemAssemblyData {
    snapshot().item_assembly().data().clone()
}
#[test]
fn policy_only_catalog_binds_existing_definitions_without_copying() {
    let s = snapshot();
    assert_eq!(s.identity().schema_version, 26);
    let c = s.item_assembly();
    assert_eq!(c.data().capability, ItemAssemblyCapability::PolicyOnly);
    assert_eq!(
        c.source().construction_spans.len(),
        ITEM_ASSEMBLY_SOURCE_ROLES.len()
    );
    let d = c
        .bind(
            s.item_loading(),
            s.item_scalability(),
            &s.package().actor,
            s.modifier_parser(),
        )
        .unwrap();
    assert!(std::ptr::eq(d.items(), s.item_loading()));
    assert!(std::ptr::eq(d.scalability(), s.item_scalability()));
    assert!(std::ptr::eq(
        d.high_precision_mods(),
        &s.package().actor.high_precision_mods
    ));
    assert!(std::ptr::eq(
        d.rune_policy(),
        &s.item_loading().policy().rune_loading
    ));
    for role in [
        ItemAssemblyRuneEffect::Global,
        ItemAssemblyRuneEffect::Rune,
        ItemAssemblyRuneEffect::SoulCore,
    ] {
        let q = d.rune_effect_query(role);
        let bytes = q.name.bytes().collect::<Vec<_>>();
        assert!(q.name.equals(&bytes));
        assert!(!q.name.equals(&[255]));
    }
}
#[test]
fn caller_patterns_payloads_and_divisors_remain_data_not_eager_runtime_checks() {
    let mut p = data();
    p.policy.collection.class_find_pattern = "[\0unfinished".into();
    p.policy.collection.class_capture_pattern.clear();
    p.policy.range.newline_rewrite.replacement = "%0\0é".into();
    p.policy.slots.tag_replacements[0].pattern = "()%b".into();
    p.policy.local.more_divisor = 0.0;
    p.policy.nil_queries.flags = 9_007_199_254_740_991.0;
    p.policy.slots.quality_source = "Caller\0source".into();
    p.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<ItemAssemblyData>(&serde_json::to_vec(&p).unwrap()).unwrap(),
        p
    );
}
#[test]
fn source_identity_rules_are_distinct_from_lua_pattern_bytes() {
    for bad in [
        "/absolute",
        "../escape",
        "C:/absolute",
        "src\\bad",
        "src/\0bad",
    ] {
        let mut p = data();
        p.source.files.insert(bad.into(), "a".repeat(64));
        assert!(p.validate().is_err(), "{bad:?}");
    }
    let mut p = data();
    p.source.module_order.push("missing.lua".into());
    assert!(p.validate().is_err());
}
#[test]
fn complete_source_roles_and_closed_execution_status_are_required() {
    let mut p = data();
    p.source
        .construction_spans
        .remove(ITEM_ASSEMBLY_SOURCE_ROLES[0]);
    assert!(p.validate().is_err());
    let mut p = data();
    let span = p.source.construction_spans.values().next().unwrap().clone();
    p.source.construction_spans.insert("unknown".into(), span);
    assert!(p.validate().is_err());
    let mut value = serde_json::to_value(data()).unwrap();
    value["capability"] = "complete".into();
    assert!(serde_json::from_value::<ItemAssemblyData>(value).is_err());
}
#[test]
fn closed_orders_reject_missing_roles_but_preserve_valid_operand_order() {
    let mut p = data();
    p.policy.collection.rows[1] = p.policy.collection.rows[0];
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.requirements.conversion_read_order[1] = p.policy.requirements.conversion_read_order[0];
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.requirements.attributes[0].source_addends[0] =
        p.policy.requirements.attributes[0].attribute;
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.requirements.attributes[0]
        .source_addends
        .swap(0, 1);
    p.validate().unwrap();
    let mut p = data();
    p.policy.slots.tag_replacements[1].role = p.policy.slots.tag_replacements[0].role;
    assert!(p.validate().is_err());
}
#[test]
fn arbitrary_names_and_overlapping_ordered_rules_are_allowed() {
    let mut p = data();
    let mut rule = p.policy.named_compatibility[0].clone();
    rule.names = vec!["A caller supplied item absent from bases".into()];
    rule.modifier.key = "a new key".into();
    rule.modifier.value = 37.5;
    rule.requirement_override = None;
    p.policy.named_compatibility = vec![rule.clone(), rule];
    p.policy.slots.primary[0].any = vec![ItemAssemblySlotPredicate::BaseTypeEquals {
        value: "caller type".into(),
    }];
    p.validate().unwrap();
}
#[test]
fn nullable_rule_field_is_required_and_unknown_policy_fields_rejected() {
    let mut v = serde_json::to_value(data()).unwrap();
    v["policy"]["named_compatibility"][0]
        .as_object_mut()
        .unwrap()
        .remove("requirement_override");
    assert!(serde_json::from_value::<ItemAssemblyData>(v).is_err());
    let mut v = serde_json::to_value(data()).unwrap();
    v["policy"]["invented"] = "value".into();
    assert!(serde_json::from_value::<ItemAssemblyData>(v).is_err());
}
#[test]
fn source_count_slot_predicate_and_precision_limits_are_bounded() {
    let mut p = data();
    p.policy.collection.duplicate_alternate_count = 257;
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.slots.multislot_any.clear();
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.slots.multislot_any = vec![ItemAssemblySlotPredicate::BaseWeaponTruthy; 65];
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.scale.keyed_value_decimal_places = 16;
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.named_compatibility = vec![p.policy.named_compatibility[0].clone(); 257];
    assert!(p.validate().is_err());
}
#[test]
fn scalar_and_aggregate_text_bounds_precede_catalog_publication() {
    let mut p = data();
    p.policy.collection.source_prefix = "x".repeat(4097);
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.named_compatibility[0].names = vec!["x".repeat(4096); 65];
    assert!(p.validate().is_err());
    let mut p = data();
    p.policy.local.more_offset = f64::NAN;
    assert!(p.validate().is_err());
}
#[test]
fn standalone_binding_reuses_precision_validation_without_actor_quest_requirements() {
    let s = snapshot();
    let c = s.item_assembly();
    let bind = |actor: &poe_optimizer_data::game_data::ActorData| {
        c.bind(
            s.item_loading(),
            s.item_scalability(),
            actor,
            s.modifier_parser(),
        )
        .map(|_| ())
    };
    let mut a = s.package().actor.clone();
    a.spirit_quests.clear();
    bind(&a).unwrap();
    let op = *a
        .high_precision_mods
        .values()
        .next()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    a.high_precision_mods = BTreeMap::from([("Caller".into(), BTreeMap::from([(op, 255)]))]);
    assert!(bind(&a).is_err());
    a.high_precision_mods = BTreeMap::from([("Caller".into(), BTreeMap::new())]);
    assert!(bind(&a).is_err());
    a.high_precision_mods = (0..257)
        .map(|i| (format!("Caller{i}"), BTreeMap::from([(op, 1)])))
        .collect();
    assert!(bind(&a).is_err());
}
#[test]
fn dependency_source_pin_and_shared_file_hash_disagreement_are_rejected() {
    let s = snapshot();
    let mut p = data();
    p.source.upstream_revision = "a".repeat(40);
    assert!(
        p.validate_dependencies(
            s.item_loading().data(),
            s.item_scalability().data(),
            s.modifier_parser().data()
        )
        .is_err()
    );
    let mut p = data();
    let key = p
        .source
        .files
        .keys()
        .find(|k| s.item_loading().data().source.files.contains_key(*k))
        .unwrap()
        .clone();
    p.source.files.insert(key, "a".repeat(64));
    assert!(
        p.validate_dependencies(
            s.item_loading().data(),
            s.item_scalability().data(),
            s.modifier_parser().data()
        )
        .is_err()
    );
}
#[test]
fn injected_common_policy_changes_identity_without_altering_other_sections() {
    let s = snapshot();
    let mut p = s.package().clone();
    p.item_assembly.policy.grants.unknown_name = "Caller unknown".into();
    p.refresh_section_digests().unwrap();
    let n = GameDataLoader::from_bytes(
        &p.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    assert_ne!(n.identity(), s.identity());
    assert_eq!(n.package().modifier_parser, s.package().modifier_parser);
    assert_eq!(
        n.package().unique_requirements,
        s.package().unique_requirements
    );
    assert_eq!(
        n.item_assembly().policy().grants.unknown_name,
        "Caller unknown"
    );
}
#[test]
fn immutable_catalog_is_send_sync_and_parallel_datasets_do_not_share_policy() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ItemAssemblyCatalog>();
    let handles = (0..8)
        .map(|i| {
            let mut p = data();
            p.policy.grants.unknown_name = format!("dataset{i}");
            let c = ItemAssemblyCatalog::new(p).unwrap();
            std::thread::spawn(move || {
                let clone = c.clone();
                assert_eq!(clone.policy().grants.unknown_name, format!("dataset{i}"));
            })
        })
        .collect::<Vec<_>>();
    for h in handles {
        h.join().unwrap();
    }
}
