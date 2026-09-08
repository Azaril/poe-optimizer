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
    assert_eq!(embedded.package().entrance_effects.len(), 16);
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
    value["entrance_effects"][0]["effects"][0]["stat"] = "run_arbitrary_script".into();
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
        |p| p.entrance_effects.push(p.entrance_effects[0].clone()),
        |p| {
            p.entrance_effects.pop();
        },
        |p| p.entrance_effects[0].effective_node_id = 1,
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
