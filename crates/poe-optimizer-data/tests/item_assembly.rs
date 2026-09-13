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
    assert_eq!(s.identity().schema_version, 35);
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
    p.policy.range.newline_rewrite.replacement = "%0\0Ã©".into();
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

#[test]
fn local_families_retain_complete_orders_and_query_targets() {
    let d = data();
    assert_eq!(d.schema_version, 6);
    let a = &d.policy.armour;
    assert_eq!(a.queries.len(), 18);
    assert_eq!(a.queries[0].role, ItemAssemblyArmourRole::ArmourBase);
    assert_eq!(
        a.queries[17].role,
        ItemAssemblyArmourRole::DefencesIncreased
    );
    assert_eq!(
        a.defences[2].increased_roles,
        [
            ItemAssemblyArmourRole::EnergyShieldIncreased,
            ItemAssemblyArmourRole::ArmourEnergyShieldIncreased,
            ItemAssemblyArmourRole::EvasionEnergyShieldIncreased,
            ItemAssemblyArmourRole::DefencesIncreased,
        ]
    );
    assert_eq!(
        a.per_level[1].increased_roles,
        a.defences[2].increased_roles
    );
    assert_eq!(a.movement.condition, "IgnoreMovementPenalties");
    assert!(a.movement.negated);
    let f = &d.policy.flask;
    assert_eq!(f.recovery.channels[0].role, ItemAssemblyRecoveryRole::Life);
    assert_eq!(f.recovery.channels[1].role, ItemAssemblyRecoveryRole::Mana);
    assert!(f.recovery.channels[0].additional.is_some());
    assert!(f.recovery.channels[1].additional.is_none());
    for channel in &f.recovery.channels {
        assert_eq!(channel.effect_not_removed.list, ItemAssemblyQueryList::Base);
    }
    assert_eq!(f.charges.maximum_output, "chargesMax");
    assert_eq!(f.charges.used_output, "chargesUsed");
    assert_eq!(d.policy.charm.charges.effect_queries[0].name, "CharmEffect");
}

#[test]
fn local_family_parameters_are_injected_without_eager_arithmetic_admission() {
    let mut d = data();
    d.policy.armour.queries.swap(0, 1);
    d.policy.armour.queries[1].query.name = "Custom local base".into();
    d.policy.armour.queries[1].base.as_mut().unwrap().field = "customBase".into();
    d.policy.armour.defences[0].output = "customDefence".into();
    d.policy.armour.defences[0].increased_roles.swap(0, 1);
    d.policy.armour.percent_divisor = 0.0;
    d.policy.armour.movement.condition = "CustomCondition".into();
    d.policy.flask.recovery.channels.swap(0, 1);
    d.policy.flask.recovery.channels[0].effect_not_removed.list = ItemAssemblyQueryList::Slot;
    d.policy.flask.fraction_base = -3.0;
    d.policy.flask.percent_divisor = 0.0;
    d.policy.charm.overrides.key_field = "customKey".into();
    d.policy.charm.charges.effect_queries.swap(0, 1);
    ItemAssemblyCatalog::new(d).unwrap();
}

#[test]
fn local_family_closed_roles_and_references_are_checked() {
    let mut d = data();
    d.policy.armour.queries[1].role = d.policy.armour.queries[0].role;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.queries[0].base = None;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.defences[1].role = d.policy.armour.defences[0].role;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.defences[0].base_roles = vec![ItemAssemblyArmourRole::ArmourIncreased];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.per_level[0].base_role = ItemAssemblyArmourRole::ArmourBase;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.flask.recovery.channels[1].role = d.policy.flask.recovery.channels[0].role;
    assert!(d.validate().is_err());
}

#[test]
fn local_family_bounds_and_unknown_or_omitted_fields_are_checked() {
    let mut d = data();
    d.policy.armour.defences[0].increased_roles = vec![];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.defences[0].base_roles = vec![ItemAssemblyArmourRole::ArmourBase; 19];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.flask.duration.round_places = 16;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.charm.percent_divisor = f64::INFINITY;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.armour.overrides.query_name = "x".repeat(4097);
    assert!(d.validate().is_err());
    let original = serde_json::to_value(data()).unwrap();
    let mut omitted = original.clone();
    omitted["policy"]["armour"]["queries"][0]
        .as_object_mut()
        .unwrap()
        .remove("base");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut omitted = original.clone();
    omitted["policy"]["flask"]["recovery"]["channels"][1]
        .as_object_mut()
        .unwrap()
        .remove("additional");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut unknown = original;
    unknown["policy"]["charm"]["execute"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ItemAssemblyData>(unknown).is_err());
}

#[test]
fn weapon_policy_retains_captured_order_and_distinct_residual_branches() {
    let d = data();
    let p = &d.policy.weapon;
    assert_eq!(p.base_field, "weapon");
    assert_eq!(p.output_field, "weaponData");
    assert_eq!(
        p.damage
            .channels
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>(),
        ["Physical", "Lightning", "Cold", "Fire", "Chaos"]
    );
    assert_eq!(
        p.damage.channels[0].kind,
        ItemAssemblyWeaponDamageKind::Physical
    );
    assert_eq!(
        p.damage.channels[4].kind,
        ItemAssemblyWeaponDamageKind::Unscaled
    );
    for c in &p.damage.channels[1..4] {
        assert_eq!(c.kind, ItemAssemblyWeaponDamageKind::Elemental);
        assert_eq!(
            c.increased.as_ref().unwrap().name,
            format!("Local{}Damage", c.name)
        );
    }
    assert_eq!(p.attack_speed.quality_divisor, 8.0);
    assert_eq!(p.range.metre_multiplier, 10.0);
    assert_eq!(p.critical.quality_divisor, 4.0);
    assert_eq!(p.overrides.query_name, "WeaponData");
    assert_eq!(p.total_output, "TotalDPS");
    assert_eq!(p.residual.untagged.len(), 5);
    assert_eq!(p.residual.critical.names, ["PoisonChance", "BleedChance"]);
    assert_eq!(
        p.residual.critical.flags.operation,
        ItemAssemblyWeaponFlagComparison::NotEqual
    );
    assert_eq!(p.residual.critical_condition, "CriticalStrike");
    assert_eq!(p.residual.primary_condition, "MainHandAttack");
    assert_eq!(p.residual.other_condition, "OffHandAttack");
}

#[test]
fn weapon_parameters_are_injected_and_do_not_eagerly_validate_runtime_arithmetic() {
    let mut d = data();
    let p = &mut d.policy.weapon;
    p.damage.channels.swap(0, 4);
    p.damage.channels[0].name = "Custom unscaled channel".into();
    p.damage.channels[0].minimum.base_field = "callerMinimum".into();
    p.damage.channels[0].minimum.output = "callerOutput".into();
    p.damage.channels[0].minimum.query.name = "Caller query".into();
    p.damage.channels[0].minimum.query.mod_type = "FLAG".into();
    p.attack_speed.query.flags = 9_007_199_254_740_991.0;
    p.attack_speed.quality_divisor = 0.0;
    p.percent_divisor = 0.0;
    p.damage.average_divisor = -2.0;
    p.residual.untagged[0].names = vec!["Caller residual".into()];
    p.residual.untagged[0].flags.value = 0.25;
    p.residual.primary_condition = "CallerCondition".into();
    p.overrides.value_field = "callerValue".into();
    p.total_output = "callerTotal".into();
    d.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<ItemAssemblyData>(&serde_json::to_vec(&d).unwrap()).unwrap(),
        d
    );
    ItemAssemblyCatalog::new(d).unwrap();
}

#[test]
fn weapon_shape_limits_and_channel_classification_are_checked() {
    let mut d = data();
    d.policy.weapon.damage.channels.clear();
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.damage.channels[1].name = d.policy.weapon.damage.channels[0].name.clone();
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.damage.channels[1].increased = None;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.damage.channels[0].increased =
        Some(d.policy.weapon.damage.physical_increased.clone());
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.damage.channels = vec![d.policy.weapon.damage.channels[0].clone(); 33];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.residual.untagged = vec![d.policy.weapon.residual.untagged[0].clone(); 65];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.residual.critical.names.clear();
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.attack_rate.round_places = 16;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.residual.keyword_flags[1] = f64::NAN;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.weapon.damage.channels[0].dps_output = "x".repeat(4097);
    assert!(d.validate().is_err());
}

#[test]
fn weapon_required_nullable_field_and_closed_schema_are_checked() {
    let original = serde_json::to_value(data()).unwrap();
    let mut omitted = original.clone();
    omitted["policy"]["weapon"]["damage"]["channels"][0]
        .as_object_mut()
        .unwrap()
        .remove("increased");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut omitted = original.clone();
    omitted["policy"].as_object_mut().unwrap().remove("weapon");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut unknown = original;
    unknown["policy"]["weapon"]["residual"]["execute"] = true.into();
    assert!(serde_json::from_value::<ItemAssemblyData>(unknown).is_err());
}

#[test]
fn jewel_policy_preserves_queries_alias_fields_and_cluster_operands() {
    let d = data();
    let p = &d.policy.jewel;
    assert_eq!(p.output_field, "jewelData");
    assert_eq!(p.grand_spectrum.name_item_field, "name");
    assert_eq!(p.grand_spectrum.name_pattern, "Grand Spectrum");
    assert_eq!(p.grand_spectrum.modifier_name, "Multiplier:GrandSpectrum");
    assert_eq!(p.grand_spectrum.modifier_type, "BASE");
    assert_eq!(p.grand_spectrum.modifier_value, 1.0);
    assert_eq!(p.grand_spectrum.minion_name, "MinionModifier");
    assert_eq!(p.grand_spectrum.minion_type, "LIST");
    assert_eq!(p.grand_spectrum.nested_mod_field, "mod");
    assert_eq!(p.functions.query_name, "JewelFunc");
    assert_eq!(p.functions.output_field, "funcList");
    assert_eq!(p.overrides.query_name, "JewelData");
    assert_eq!(p.alternate_class_start.query_name, "AlternateClassStart");
    assert_eq!(p.from_nothing.guard_query_name, "FromNothingKeystones");
    assert_eq!(
        p.from_nothing.entries.query_name,
        p.from_nothing.guard_query_name
    );
    assert_eq!(p.from_nothing.output_field, "fromNothingKeystones");
    let c = &p.cluster;
    assert_eq!(c.item_field, "clusterJewel");
    assert_eq!(c.notables.query_name, "ClusterJewelNotable");
    assert_eq!(c.added_mods.query_name, "AddToClusterJewelNode");
    assert_eq!(c.correction.matching_skill, "affliction_curse_effect");
    assert_eq!(
        c.correction.replacement_skill,
        "affliction_curse_effect_small"
    );
    assert_eq!(c.correction.node_count_below, 4.0);
    assert_eq!(c.min_nodes_field, "minNodes");
    assert_eq!(c.max_nodes_field, "maxNodes");
    assert_eq!(c.validity.output_field, "clusterJewelValid");
    assert_eq!(
        c.validity.nothingness_count_field,
        "clusterJewelNothingnessCount"
    );
}

#[test]
fn jewel_custom_operands_are_not_eagerly_evaluated_or_source_authenticated() {
    let mut d = data();
    let p = &mut d.policy.jewel;
    p.output_field = "callerJewel".into();
    p.grand_spectrum.name_pattern = "[%unfinished\0".into();
    p.grand_spectrum.modifier_name = "Caller multiplier".into();
    p.grand_spectrum.modifier_value = -0.25;
    p.grand_spectrum.minion_type = "FLAG".into();
    p.functions.query_name = "Caller opaque values".into();
    p.from_nothing.guard_query_name = "Guard query".into();
    p.from_nothing.entries.query_name = "Separate entries query".into();
    p.from_nothing.entries.key_field = "callerKey".into();
    p.cluster.node_count_field = "callerCount".into();
    p.cluster.min_nodes_field = "callerMinimum".into();
    p.cluster.correction.node_count_below = -1.5;
    p.cluster.validity.output_field = "callerValidity".into();
    p.cluster.validity.keystone_field = "callerValue".into();
    d.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<ItemAssemblyData>(&serde_json::to_vec(&d).unwrap()).unwrap(),
        d
    );
    ItemAssemblyCatalog::new(d).unwrap();
}

#[test]
fn jewel_bounds_and_required_closed_fields_are_checked() {
    let mut d = data();
    d.policy.jewel.grand_spectrum.name_pattern = "x".repeat(4097);
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.jewel.cluster.correction.node_count_below = f64::INFINITY;
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.jewel.grand_spectrum.modifier_value = f64::NAN;
    assert!(d.validate().is_err());
    let original = serde_json::to_value(data()).unwrap();
    let mut omitted = original.clone();
    omitted["policy"]["jewel"]["from_nothing"]
        .as_object_mut()
        .unwrap()
        .remove("guard_query_name");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut omitted = original.clone();
    omitted["policy"]["jewel"]["cluster"]["validity"]
        .as_object_mut()
        .unwrap()
        .remove("keystone_field");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut unknown = original;
    unknown["policy"]["jewel"]["execute"] = true.into();
    assert!(serde_json::from_value::<ItemAssemblyData>(unknown).is_err());
}

#[test]
fn slot_validity_catalog_retains_complete_source_policy_and_span() {
    let d = data();
    let p = &d.policy.slot_validity;
    let span = &d.source.construction_spans["slot_validity"];
    assert_eq!(span.path, "src/Classes/ItemsTab.lua");
    assert_eq!((span.line, span.end_line), (2603, 2687));
    assert_eq!(p.jewel.slot_type, "Jewel");
    assert_eq!(p.jewel.unique_rarities, ["UNIQUE", "RELIC"]);
    assert_eq!(p.jewel.contained_socket_field, "containJewelSocket");
    assert_eq!(p.jewel.expansion_size_field, "size");
    assert_eq!(p.jewel.cluster_size_field, "sizeIndex");
    assert_eq!(p.jewel.outer_size, 2.0);
    assert_eq!(p.flask.routes[0].base_name_pattern, "Life Flask");
    assert_eq!(p.flask.routes[1].slot_name_pattern, "Flask 2");
    assert_eq!(p.subtypes[0].base_subtype, "Transcendent Arm");
    assert_eq!(p.embedded.restriction_field, "canSocketJewelBase");
    assert_eq!(
        p.weapon.primary_slots,
        ["Weapon 1", "Weapon 1 Swap", "Weapon"]
    );
    assert_eq!(p.weapon.offhand_slots[1].primary, "Weapon 1 Swap");
    assert_eq!(p.weapon.empty_selection, 0.0);
    assert_eq!(p.weapon.giants_blood.query_name, "GiantsBlood");
    assert!(p.weapon.instruments_of_power.default);
    assert_eq!(
        p.weapon.ordinary_offhand_types,
        ["Shield", "Focus", "Sceptre"]
    );
    assert_eq!(p.weapon.giant_tags, ["axe", "mace", "sword"]);
}

#[test]
fn slot_validity_custom_game_operands_are_bounded_data_not_pinned_source() {
    let mut d = data();
    let p = &mut d.policy.slot_validity;
    p.slot_pattern = "[unfinished\0".into();
    p.jewel.slot_type = "Caller socket".into();
    p.jewel.outer_size = -0.25;
    p.jewel.expansion_field = "callerExpansion".into();
    p.flask.routes.swap(0, 1);
    p.subtypes[0].slot_type = "Caller arm".into();
    p.embedded.parent_rewrite.replacement = "%0\0".into();
    p.weapon.primary_slots = vec!["Caller primary".into()];
    p.weapon.offhand_slots[0].offhand = "Caller offhand".into();
    p.weapon.empty_selection = -0.0;
    p.weapon.giants_blood.state_field = "callerFlag".into();
    p.weapon.giants_blood.query_name = "CallerFlag".into();
    p.weapon.giants_blood.default = false;
    p.weapon.giant_tags.reverse();
    d.validate().unwrap();
    assert_eq!(
        serde_json::from_slice::<ItemAssemblyData>(&serde_json::to_vec(&d).unwrap()).unwrap(),
        d
    );
    ItemAssemblyCatalog::new(d).unwrap();
}

#[test]
fn slot_validity_shape_and_storage_bounds_are_enforced() {
    for nonfinite in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut d = data();
        d.policy.slot_validity.jewel.outer_size = nonfinite;
        assert!(d.validate().is_err());
        let mut d = data();
        d.policy.slot_validity.weapon.empty_selection = nonfinite;
        assert!(d.validate().is_err());
    }
    let mut d = data();
    d.policy.slot_validity.weapon.primary_slots.clear();
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.slot_validity.weapon.giant_tags = vec!["tag".into(); 65];
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.slot_validity.weapon.offhand_slots[1].offhand =
        d.policy.slot_validity.weapon.offhand_slots[0]
            .offhand
            .clone();
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.slot_validity.embedded.restriction_field = "x".repeat(4097);
    assert!(d.validate().is_err());
    let mut d = data();
    d.policy.slot_validity.weapon.primary_slots = vec!["x".repeat(4096); 64];
    assert!(d.validate().is_err()); // The rest of the policy also consumes aggregate bytes.
}

#[test]
fn slot_validity_required_closed_schema_and_source_role_are_checked() {
    let original = serde_json::to_value(data()).unwrap();
    let mut omitted = original.clone();
    omitted["policy"]
        .as_object_mut()
        .unwrap()
        .remove("slot_validity");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut omitted = original.clone();
    omitted["policy"]["slot_validity"]["weapon"]["giants_blood"]
        .as_object_mut()
        .unwrap()
        .remove("default");
    assert!(serde_json::from_value::<ItemAssemblyData>(omitted).is_err());
    let mut unknown = original.clone();
    unknown["policy"]["slot_validity"]["jewel"]["execute"] = true.into();
    assert!(serde_json::from_value::<ItemAssemblyData>(unknown).is_err());
    let mut wrong_arity = original;
    wrong_arity["policy"]["slot_validity"]["flask"]["routes"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert!(serde_json::from_value::<ItemAssemblyData>(wrong_arity).is_err());
    let mut d = data();
    d.source.construction_spans.remove("slot_validity");
    assert!(d.validate().is_err());
}

#[test]
fn directly_constructed_slot_policy_uses_the_catalogs_shape_and_storage_guards() {
    let mut p = data().policy.slot_validity;
    p.jewel.outer_size = -2.5;
    p.slot_pattern = "[unfinished".into();
    p.validate().unwrap();
    p.weapon.giant_tags = vec!["tag".into(); 65];
    assert!(p.validate().is_err());
    p.weapon.giant_tags = vec!["tag".into()];
    p.weapon.giants_blood.query_name = "x".repeat(4097);
    assert!(p.validate().is_err());
    p.weapon.giants_blood.query_name = "Caller".into();
    p.weapon.primary_slots = vec!["x".repeat(4096); 64];
    assert!(p.validate().is_err());
}
