use poe_optimizer_import::{
    owned_item_observations::*,
    owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin},
};

fn catalog() -> ItemObservationCatalog {
    ItemObservationCatalog {
        schema_version: OWNED_ITEM_OBSERVATION_VERSION,
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "fixture".into(),
            files: vec![SourceFilePin {
                path: "fixture.lua".into(),
                sha256: "a".repeat(64),
            }],
        },
        bases: vec![ItemObservationBaseRow {
            source_base: "Fixture Base".into(),
            weapon_branch: ItemObservationWeaponBranch::Absent,
            armour: ItemObservationArmourField::Table {
                armour: ItemObservationNumericField::Finite { value: 10.0 },
                evasion: ItemObservationNumericField::Absent,
                energy_shield: ItemObservationNumericField::Absent,
                ward: ItemObservationNumericField::Absent,
                block_chance: ItemObservationNumericField::Absent,
                movement_penalty: ItemObservationNumericField::Absent,
            },
            spirit: ItemObservationNumericField::Finite { value: 0.0 },
            charm_slots: ItemObservationNumericField::Finite { value: 0.0 },
            defence_base_retarget: ItemObservationRetarget::None,
        }],
    }
}

#[test]
fn recomputation_eligibility_requires_exact_branch_and_finite_fields() {
    let mut row = catalog().bases.remove(0);
    assert!(row.can_observe_defences());
    assert!(row.can_observe_spirit());
    assert!(row.can_observe_charm_slots());
    row.weapon_branch = ItemObservationWeaponBranch::Truthy;
    assert!(!row.can_observe_defences());
    row.weapon_branch = ItemObservationWeaponBranch::False;
    assert!(row.can_observe_defences());
    row.defence_base_retarget = ItemObservationRetarget::Possible;
    assert!(!row.can_observe_defences());
    row.defence_base_retarget = ItemObservationRetarget::None;
    if let ItemObservationArmourField::Table { block_chance, .. } = &mut row.armour {
        *block_chance = ItemObservationNumericField::Unsupported;
    }
    assert!(!row.can_observe_defences());
    row.spirit = ItemObservationNumericField::Absent;
    row.charm_slots = ItemObservationNumericField::Unsupported;
    assert!(!row.can_observe_spirit());
    assert!(!row.can_observe_charm_slots());
    row.spirit = ItemObservationNumericField::Finite { value: f64::NAN };
    assert!(!row.can_observe_spirit());
}

#[test]
fn decode_rejects_unknown_fields_duplicates_unsorted_rows_and_nonfinite_values() {
    let original = catalog();
    let bytes = serde_json::to_vec(&original).unwrap();
    assert_eq!(
        decode_item_observation_catalog(&bytes, Default::default()).unwrap(),
        original
    );
    for change in 0..7 {
        let mut value = original.clone();
        match change {
            0 => value.schema_version += 1,
            1 => value.bases.push(value.bases[0].clone()),
            2 => value.source.files.push(value.source.files[0].clone()),
            3 => value.bases[0].source_base.push('\n'),
            4 => value.source.files[0].sha256 = "x".repeat(64),
            5 => {
                value.bases[0].charm_slots = ItemObservationNumericField::Finite {
                    value: f64::INFINITY,
                }
            }
            _ => {
                let mut second = value.bases[0].clone();
                second.source_base = "Earlier".into();
                value.bases.push(second);
            }
        }
        assert!(
            validate_item_observation_catalog(&value, Default::default()).is_err(),
            "change {change}"
        );
    }
    let mut value = serde_json::to_value(&original).unwrap();
    value["bases"][0]["callback"] = serde_json::json!("must not escape into owned data");
    assert!(
        decode_item_observation_catalog(&serde_json::to_vec(&value).unwrap(), Default::default())
            .is_err()
    );
}

#[test]
fn limits_bound_decode_and_direct_validation() {
    let original = catalog();
    let bytes = serde_json::to_vec(&original).unwrap();
    for limits in [
        ItemObservationLimits {
            max_catalog_bytes: bytes.len() - 1,
            ..Default::default()
        },
        ItemObservationLimits {
            max_work: 1,
            ..Default::default()
        },
        ItemObservationLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
        ItemObservationLimits {
            max_bases: 0,
            ..Default::default()
        },
        ItemObservationLimits {
            max_work: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(decode_item_observation_catalog(&bytes, limits).is_err());
    }
    let mut many = original;
    many.bases = vec![many.bases[0].clone(); 2];
    assert!(
        validate_item_observation_catalog(
            &many,
            ItemObservationLimits {
                max_bases: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
}
