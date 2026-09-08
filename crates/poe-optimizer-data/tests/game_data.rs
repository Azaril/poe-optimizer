use poe_optimizer_data::game_data::*;

fn custom(mut package: GameDataPackage) -> Result<GameDataSnapshot, GameDataError> {
    package.refresh_section_digests()?;
    GameDataLoader::from_bytes(
        &package.canonical_bytes()?,
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
}
fn reviewed() -> GameDataSnapshot {
    bundled_snapshot().unwrap()
}

#[test]
fn embedded_and_external_bytes_share_one_validated_loader() {
    let embedded = reviewed();
    let external = GameDataLoader::from_bytes(
        bundled_package_bytes(),
        &TrustPolicy::Reviewed {
            expected_sha256: bundled_package_sha256().into(),
        },
        &LoadLimits::default(),
    )
    .unwrap();
    assert_eq!(embedded.identity(), external.identity());
    assert_eq!(embedded.package(), external.package());
    assert_eq!(embedded.trust(), external.trust());
    assert_eq!(
        embedded.tree(),
        poe_optimizer_data::bundled::class_tree().unwrap()
    );
    embedded.identity().validate().unwrap();
    assert_eq!(
        embedded.package().canonical_bytes().unwrap(),
        bundled_package_bytes()
    );
    assert_eq!(embedded.package().passive_effects.len(), 20);
}
#[test]
fn explicit_custom_balance_has_content_identity_without_claiming_review() {
    let original = reviewed();
    let mut package = original.package().clone();
    package.spark.lightning_maximum += 2.0;
    let custom = custom(package).unwrap();
    assert_ne!(original.identity(), custom.identity());
    assert_eq!(original.identity().release, custom.identity().release);
    assert_eq!(custom.trust(), &DataTrust::CustomUnreviewed);
    let wrong_policy = TrustPolicy::Reviewed {
        expected_sha256: bundled_package_sha256().into(),
    };
    assert!(
        GameDataLoader::from_bytes(
            &custom.package().canonical_bytes().unwrap(),
            &wrong_policy,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("host-supplied")
    );
}
#[test]
fn package_self_asserted_hashes_do_not_authenticate_edited_content() {
    let mut package = reviewed().package().clone();
    package.character.life_per_level += 1.0;
    assert!(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("section SHA-256")
    );
    package.refresh_section_digests().unwrap();
    assert!(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::Reviewed {
                expected_sha256: "0".repeat(64)
            },
            &LoadLimits::default()
        )
        .is_err()
    );
}
#[test]
fn recursive_duplicate_keys_and_resource_limits_reject_before_snapshot() {
    let reject = |bytes: &[u8], limits: LoadLimits, message: &str| {
        assert!(
            GameDataLoader::from_bytes(bytes, &TrustPolicy::AllowCustom, &limits)
                .unwrap_err()
                .to_string()
                .contains(message)
        );
    };
    reject(
        br#"{"outer":{"same":1,"same":2}}"#,
        LoadLimits::default(),
        "duplicate JSON key",
    );
    reject(
        br#"{"outer":{"x":[[[1]]]}}"#,
        LoadLimits {
            max_depth: 3,
            ..LoadLimits::default()
        },
        "depth/value limit",
    );
    reject(
        br#"[1,2,3]"#,
        LoadLimits {
            max_values: 2,
            ..LoadLimits::default()
        },
        "depth/value limit",
    );
    reject(
        br#""abcdef""#,
        LoadLimits {
            max_string_bytes: 3,
            ..LoadLimits::default()
        },
        "string limit",
    );
    reject(
        bundled_package_bytes(),
        LoadLimits {
            max_bytes: 100,
            ..LoadLimits::default()
        },
        "byte limit",
    );
    reject(
        br#"{"x":1e1000}"#,
        LoadLimits::default(),
        "number out of range",
    );
    reject(br#"{}{}"#, LoadLimits::default(), "trailing characters");
}
#[test]
fn manifest_versions_required_fields_and_unknown_operations_are_closed() {
    for mutate in [
        |p: &mut GameDataPackage| p.manifest.schema_version += 1,
        |p: &mut GameDataPackage| p.manifest.game = "poe1".into(),
        |p: &mut GameDataPackage| p.manifest.semantics_version = "unimplemented".into(),
        |p: &mut GameDataPackage| p.manifest.coverage.clear(),
    ] {
        let mut package = reviewed().package().clone();
        mutate(&mut package);
        assert!(custom(package).is_err());
    }
    let mut value = serde_json::to_value(reviewed().package()).unwrap();
    value.as_object_mut().unwrap().remove("monsters");
    assert!(
        GameDataLoader::from_bytes(
            &serde_json::to_vec(&value).unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .is_err()
    );
    let mut value = serde_json::to_value(reviewed().package()).unwrap();
    value["passive_effects"][0]["effects"][0]["stat"] = "run_arbitrary_script".into();
    assert!(
        GameDataLoader::from_bytes(
            &serde_json::to_vec(&value).unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .unwrap_err()
        .to_string()
        .contains("unknown variant")
    );
}
#[test]
fn record_references_duplicate_selectors_and_ambiguous_fields_reject() {
    let mutations: Vec<fn(&mut GameDataPackage)> = vec![
        |p| p.weapons.push(p.weapons[0].clone()),
        |p| p.passive_effects.push(p.passive_effects[0].clone()),
        |p| {
            p.passive_effects.pop();
        },
        |p| p.passive_effects[0].effective_node_id = 1,
        |p| p.spark.default_class_id = u32::MAX,
        |p| p.quests.config_keys[0] = "enemyIsBoss".into(),
        |p| p.quests.config_keys[0] = p.quests.config_keys[1].clone(),
        |p| p.spark.skill_id = p.mace.skill_id.clone(),
        |p| {
            p.monsters.armour.pop();
        },
        |p| p.weapons[0].attack_rate = 0.0,
        |p| p.spark.lightning_minimum = p.spark.lightning_maximum + 1.0,
        |p| p.defence.hit_chance_floor = p.defence.hit_chance_cap + 1.0,
        |p| p.character.base_evasion = -1.0,
    ];
    for mutate in mutations {
        let mut package = reviewed().package().clone();
        mutate(&mut package);
        assert!(custom(package).is_err());
    }
}
#[test]
fn custom_numeric_data_cannot_relax_structural_tree_source_pin() {
    let mut package = reviewed().package().clone();
    package.tree.classes.get_mut(&1).unwrap().base_strength += 1;
    assert!(
        custom(package)
            .unwrap_err()
            .to_string()
            .contains("compiled trusted digest")
    );
    let mut package = reviewed().package().clone();
    package.tree.source.upstream_revision = "0".repeat(40);
    assert!(custom(package).is_err());
}
#[test]
fn snapshot_owned_data_is_send_sync_and_remains_unchanged_by_package_edits() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<GameDataSnapshot>();
    let snapshot = reviewed();
    let old = snapshot.identity().clone();
    let mut edited = snapshot.package().clone();
    edited.character.base_evasion += 3.0;
    let changed = custom(edited).unwrap();
    assert_eq!(snapshot.identity(), &old);
    assert_ne!(snapshot.identity(), changed.identity());
}

#[test]
fn unknown_nested_source_enum_fields_and_integer_key_aliases_do_not_disappear() {
    let mut value = serde_json::to_value(reviewed().package()).unwrap();
    let fields = value["tree"]["classes"]["1"]["source"]["named"]
        .as_object_mut()
        .unwrap();
    fields
        .values_mut()
        .next()
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("unmodeled_mechanic".into(), true.into());
    let bytes = serde_json::to_vec(&value).unwrap();
    let error =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap_err();
    assert!(error.to_string().contains("unknown or ambiguous field"));
    assert!(GameDataPackage::decode_for_authoring(&bytes, &LoadLimits::default()).is_err());
    let mut value = serde_json::to_value(reviewed().package()).unwrap();
    let classes = value["tree"]["classes"].as_object_mut().unwrap();
    let original = classes["1"].clone();
    classes.insert("01".into(), original);
    assert!(
        GameDataLoader::from_bytes(
            &serde_json::to_vec(&value).unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default()
        )
        .is_err()
    );
}

#[test]
fn requirement_schema_is_explicit_bounded_and_content_bound() {
    let original = reviewed();
    assert_eq!(original.identity().schema_version, 3);
    assert_eq!(
        original.identity().semantics_version,
        "poe2-native-profiles-v3"
    );
    let mut package = original.package().clone();
    package.weapons[0].requirements = RequirementData {
        level: 100,
        attributes: AttributeRequirements {
            strength: 1_000_000,
            dexterity: 17,
            intelligence: 21,
        },
    };
    package.mace.requirements.attributes.strength = 15;
    package.mace.brutality.color = SupportColor::Blue;
    package.mace.support_attribute_costs.intelligence = 7;
    let changed = custom(package).unwrap();
    assert_ne!(changed.identity(), original.identity());
    assert_eq!(changed.trust(), &DataTrust::CustomUnreviewed);
    assert_eq!(changed.package().mace.brutality.color, SupportColor::Blue);
    for mutate in [
        |p: &mut GameDataPackage| p.weapons[0].requirements.level = 101,
        |p: &mut GameDataPackage| p.spark.requirements.level = 101,
        |p: &mut GameDataPackage| p.mace.requirements.level = 101,
        |p: &mut GameDataPackage| p.mace.brutality.requirements.level = 101,
        |p: &mut GameDataPackage| p.mace.brutality.requirements.attributes.strength = 1,
        |p: &mut GameDataPackage| p.mace.support_attribute_costs.strength = 1_000_001,
        |p: &mut GameDataPackage| p.weapons[0].requirements.attributes.strength = 1_000_001,
        |p: &mut GameDataPackage| p.manifest.schema_version = 1,
        |p: &mut GameDataPackage| p.manifest.semantics_version = "poe2-native-profiles-v1".into(),
    ] {
        let mut package = original.package().clone();
        mutate(&mut package);
        assert!(custom(package).is_err());
    }
    for mutate in [
        |v: &mut serde_json::Value| {
            v["weapons"][0]
                .as_object_mut()
                .unwrap()
                .remove("requirements");
        },
        |v: &mut serde_json::Value| {
            v["weapons"][0]["requirements"]["attributes"]["strength"] = (-1).into()
        },
        |v: &mut serde_json::Value| {
            v["weapons"][0]["requirements"]["attributes"]["strength"] = 1.5.into()
        },
        |v: &mut serde_json::Value| v["weapons"][0]["requirements"]["item_level"] = 1.into(),
        |v: &mut serde_json::Value| v["mace"]["brutality"]["color"] = "white".into(),
    ] {
        let mut value = serde_json::to_value(original.package()).unwrap();
        mutate(&mut value);
        assert!(
            GameDataPackage::decode_for_authoring(
                &serde_json::to_vec(&value).unwrap(),
                &LoadLimits::default()
            )
            .is_err()
        );
    }
}

#[test]
fn signed_values_are_allowed_only_for_closed_resistance_operations() {
    let original = reviewed();
    for stat in [
        PassiveStat::FireResistanceFlat,
        PassiveStat::ColdResistanceFlat,
        PassiveStat::LightningResistanceFlat,
        PassiveStat::ChaosResistanceFlat,
        PassiveStat::ElementalResistanceFlat,
    ] {
        for value in [-1_000_000.0, -0.75, 0.0, 1_000_000.0] {
            let mut package = original.package().clone();
            let effect = &mut package
                .passive_effects
                .iter_mut()
                .find(|record| record.ascendancy_id.is_some())
                .unwrap()
                .effects[0];
            *effect = PassiveEffect { stat, value };
            let edited = custom(package).unwrap();
            assert_ne!(edited.identity(), original.identity());
        }
        for value in [-1_000_001.0, 1_000_001.0, f64::INFINITY, f64::NAN] {
            let mut package = original.package().clone();
            package.passive_effects[0].effects[0] = PassiveEffect { stat, value };
            assert!(custom(package).is_err());
        }
    }
    for stat in [
        PassiveStat::ArmourFlat,
        PassiveStat::EvasionFlat,
        PassiveStat::EnergyShieldFlat,
        PassiveStat::SkillSpeedIncreased,
        PassiveStat::SpellDamageIncreased,
        PassiveStat::AttackDamageIncreased,
        PassiveStat::MeleeDamageIncreased,
        PassiveStat::ProjectileDamageIncreased,
        PassiveStat::MinionDamageIncreased,
    ] {
        let mut package = original.package().clone();
        package.passive_effects[0].effects[0] = PassiveEffect { stat, value: -0.5 };
        assert!(custom(package).is_err());
    }
    let mut package = original.package().clone();
    package.character.life_per_level = -0.5;
    assert!(custom(package).is_err());
}

#[test]
fn passive_effect_records_require_complete_exact_class_and_ascendancy_ownership() {
    let snapshot = reviewed();
    let warrior = snapshot
        .passive_effects(6, Some("Warrior3"), 14960)
        .unwrap();
    assert_eq!(
        warrior.effects,
        [PassiveEffect {
            stat: PassiveStat::FireResistanceFlat,
            value: 8.0
        }]
    );
    assert!(snapshot.passive_effects(6, None, 14960).is_none());
    assert!(
        snapshot
            .passive_effects(11, Some("Warrior3"), 14960)
            .is_none()
    );
    assert!(
        snapshot
            .passive_effects(6, Some("Warrior1"), 14960)
            .is_none()
    );
    for edit in 0..8 {
        let mut package = snapshot.package().clone();
        let index = package
            .passive_effects
            .iter()
            .position(|record| record.ascendancy_id.as_deref() == Some("Warrior3"))
            .unwrap();
        match edit {
            0 => {
                package.passive_effects.remove(index);
            }
            1 => package
                .passive_effects
                .push(package.passive_effects[index].clone()),
            2 => package.passive_effects[index].class_id = 11,
            3 => package.passive_effects[index].ascendancy_id = None,
            4 => package.passive_effects[index].ascendancy_id = Some("Warrior1".into()),
            5 => package.passive_effects[index].physical_node_id = 3936,
            6 => package.passive_effects[index].effective_node_id = 3936,
            _ => package.passive_effects[index].effects.clear(),
        }
        assert!(custom(package).is_err(), "edit {edit}");
    }
}

#[test]
fn resistance_schema_requires_explicit_regeneration_of_old_packages() {
    let snapshot = reviewed();
    for edit in 0..3 {
        let mut package = snapshot.package().clone();
        match edit {
            0 => package.manifest.schema_version = 2,
            1 => package.manifest.semantics_version = "poe2-native-profiles-v2".into(),
            _ => package.tree.schema_version = 1,
        }
        assert!(custom(package).is_err());
    }
    let mut old = serde_json::to_value(snapshot.package()).unwrap();
    let effects = old
        .as_object_mut()
        .unwrap()
        .remove("passive_effects")
        .unwrap();
    old["entrance_effects"] = effects;
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&old).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}

#[test]
fn player_global_resistance_cap_is_injected_required_and_bounded() {
    let snapshot = reviewed();
    assert_eq!(snapshot.package().defence.resistance_maximum_cap, 90.0);
    let mut package = snapshot.package().clone();
    package.defence.resistance_maximum_cap = 70.0;
    assert_eq!(
        custom(package)
            .unwrap()
            .package()
            .defence
            .resistance_maximum_cap,
        70.0
    );
    for cap in [-1.0, 101.0, f64::NAN] {
        let mut package = snapshot.package().clone();
        package.defence.resistance_maximum_cap = cap;
        assert!(custom(package).is_err());
    }
    let mut value = serde_json::to_value(snapshot.package()).unwrap();
    value["defence"]
        .as_object_mut()
        .unwrap()
        .remove("resistance_maximum_cap");
    assert!(
        GameDataPackage::decode_for_authoring(
            &serde_json::to_vec(&value).unwrap(),
            &LoadLimits::default()
        )
        .is_err()
    );
}
