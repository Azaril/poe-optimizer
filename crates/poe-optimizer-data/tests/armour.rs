use poe_optimizer_data::game_data::*;
fn custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
#[test]
fn armour_base_catalog_is_complete_injected_and_content_bound() {
    let original = bundled_snapshot().unwrap();
    let package = original.package();
    assert_eq!(package.armour_bases.len(), 288);
    for (slot, count) in [
        (EquipmentSlot::Helmet, 112),
        (EquipmentSlot::Gloves, 88),
        (EquipmentSlot::Boots, 88),
    ] {
        assert_eq!(
            package
                .armour_bases
                .iter()
                .filter(|base| base.slot == slot)
                .count(),
            count
        );
    }
    for (name, armour, evasion, es) in [
        ("Rusted Greathelm", 29.0, 0.0, 0.0),
        ("Suede Bracers", 0.0, 10.0, 0.0),
        ("Straw Sandals", 0.0, 0.0, 14.0),
    ] {
        let base = package.armour_base_by_name(name).unwrap();
        assert_eq!(
            (base.armour, base.evasion, base.energy_shield),
            (armour, evasion, es)
        );
        assert_eq!(package.armour_base(&base.id), Some(base));
    }
    assert!(package.armour_base("unknown").is_none());
    let mut changed = package.clone();
    let base = changed
        .armour_bases
        .iter_mut()
        .find(|base| base.id == "rusted_greathelm")
        .unwrap();
    base.name = "Authored Iron Helmet".into();
    base.armour = 31.25;
    base.evasion = 17.5;
    base.energy_shield = 4.25;
    base.quality = 10;
    base.requirements.level = 33;
    base.requirements.attributes.strength = 7;
    let changed = custom(changed).unwrap();
    assert_ne!(changed.identity(), original.identity());
    assert_eq!(changed.trust(), &DataTrust::CustomUnreviewed);
    assert!(
        changed
            .package()
            .armour_base_by_name("Rusted Greathelm")
            .is_none()
    );
    assert!(
        changed
            .package()
            .armour_base_by_name("Authored Iron Helmet")
            .is_some()
    );
}
#[test]
fn armour_catalog_requires_closed_slots_bounded_values_and_complete_schema() {
    let original = bundled_snapshot().unwrap();
    let edits: &[fn(&mut GameDataPackage)] = &[
        |p| p.armour_bases.clear(),
        |p| p.armour_bases.push(p.armour_bases[0].clone()),
        |p| p.armour_bases[0].slot = EquipmentSlot::Amulet,
        |p| p.armour_bases[0].id = "Bad Name".into(),
        |p| p.armour_bases[0].name = " ".into(),
        |p| p.armour_bases[0].name = p.armour_bases[1].name.clone(),
        |p| p.armour_bases[0].armour = -0.1,
        |p| p.armour_bases[0].evasion = f64::NAN,
        |p| p.armour_bases[0].energy_shield = 1_000_001.0,
        |p| p.armour_bases[0].quality = 21,
        |p| p.armour_bases[0].requirements.level = 101,
        |p| p.armour_bases[0].requirements.attributes.intelligence = 1_000_001,
        |p| p.manifest.schema_version = 8,
    ];
    for (i, edit) in edits.iter().enumerate() {
        let mut p = original.package().clone();
        edit(&mut p);
        assert!(custom(p).is_err(), "edit{i}");
    }
    for field in ["armour_bases", "armour", "quality", "requirements"] {
        let mut raw = serde_json::to_value(original.package()).unwrap();
        if field == "armour_bases" {
            raw.as_object_mut().unwrap().remove(field);
        } else {
            raw["armour_bases"][0]
                .as_object_mut()
                .unwrap()
                .remove(field);
        }
        assert!(
            GameDataPackage::decode_for_authoring(
                &serde_json::to_vec(&raw).unwrap(),
                &LoadLimits::default()
            )
            .is_err(),
            "{field}"
        );
    }
    let mut raw = serde_json::to_value(original.package()).unwrap();
    raw["armour_bases"][0]["ward"] = 1.into();
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&raw).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn paired_armour_aliases_remain_local_only_with_strict_operation_and_passive_guards() {
    let original = bundled_snapshot().unwrap();
    for stat in [
        ActorStat::ArmourAndEnergyShield,
        ActorStat::EvasionAndEnergyShield,
    ] {
        assert!(stat.is_local_armour_only());
        assert!(!stat.is_receiving_defence());
        let mut record = ActorModifierRecord {
            stat,
            effect: ActorModifierEffect::Numeric {
                operation: ActorNumericOperation::Base,
                value: 17.0,
            },
            source: None,
            flags: 0,
            keyword_flags: 0,
            tags: vec![],
        };
        record.validate().unwrap();
        record.tags.push(ActorModifierTag::Global);
        assert!(record.validate().is_err());
        record.tags.clear();
        for op in [ActorNumericOperation::More, ActorNumericOperation::Override] {
            record.effect = ActorModifierEffect::Numeric {
                operation: op,
                value: 1.0,
            };
            assert!(record.validate().is_err());
        }
        record.effect = ActorModifierEffect::Numeric {
            operation: ActorNumericOperation::Increased,
            value: 1.0,
        };
        record.validate().unwrap();
        let mut p = original.package().clone();
        let passive = p
            .passive_effects
            .iter_mut()
            .find(|r| !r.actor_modifiers.is_empty())
            .unwrap();
        record.source = passive.actor_modifiers[0].source.clone();
        passive.actor_modifiers.push(record);
        assert!(custom(p).is_err());
    }
    assert_eq!(
        original
            .package()
            .actor
            .modifier_rule("armour_and_energy_shield_base")
            .unwrap()
            .modifiers[0]
            .stat,
        ActorStat::ArmourAndEnergyShield
    );
    assert_eq!(
        original
            .package()
            .actor
            .modifier_rule("evasion_rating_and_energy_shield_increased")
            .unwrap()
            .modifiers[0]
            .stat,
        ActorStat::EvasionAndEnergyShield
    );
}
