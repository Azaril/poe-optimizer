//! Finite source facts are validated against the separately bundled construction.
use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::owned_item_observations::*;
use poe_optimizer_pob::{owned_item_observations::*, source};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, sync::OnceLock};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedItemObservationExport {
    static EXPORT: OnceLock<AuthenticatedItemObservationExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_item_observations(&root(), Default::default()).unwrap())
}
fn finite(value: Option<&ItemMetadataValue>) -> Option<f64> {
    value
        .and_then(ItemMetadataValue::as_f64)
        .filter(|n| n.is_finite())
}
#[test]
fn all_constructed_bases_retain_exact_fields_and_recomputation_branch() {
    let export = authenticated();
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.item_loading();
    assert_eq!(export.catalog().bases.len(), original.bases().len());
    assert_eq!(export.catalog().bases.len(), 1756);
    for row in &export.catalog().bases {
        let base = original.base(&row.source_base).unwrap();
        assert_eq!(
            row.can_observe_spirit(),
            finite(base.field("spirit")).is_some(),
            "{}",
            row.source_base
        );
        assert_eq!(
            row.can_observe_charm_slots(),
            finite(base.field("charmLimit")).is_some(),
            "{}",
            row.source_base
        );
        for (actual, expected) in [
            (row.spirit, finite(base.field("spirit"))),
            (row.charm_slots, finite(base.field("charmLimit"))),
        ] {
            if let Some(value) = expected {
                assert_eq!(actual, ItemObservationNumericField::Finite { value });
            }
        }
        let weapon_truthy = !matches!(
            base.field("weapon"),
            None | Some(ItemMetadataValue::Boolean(false))
        );
        assert_eq!(
            row.weapon_branch == ItemObservationWeaponBranch::Truthy,
            weapon_truthy
        );
        if let Some(ItemMetadataValue::Table(table)) = base.field("armour") {
            // Every current armour table has only these reviewed numeric channels.
            assert!(table.indexed.is_empty());
            for (name, value) in &table.fields {
                assert!(
                    matches!(
                        name.as_str(),
                        "Armour"
                            | "Evasion"
                            | "EnergyShield"
                            | "Ward"
                            | "BlockChance"
                            | "MovementPenalty"
                    ),
                    "{}: {name}",
                    row.source_base
                );
                assert!(value.as_f64().is_some_and(f64::is_finite));
            }
            let ItemObservationArmourField::Table {
                armour,
                evasion,
                energy_shield,
                ward,
                block_chance,
                movement_penalty,
            } = row.armour
            else {
                panic!("missing armour table: {}", row.source_base)
            };
            for (name, actual) in [
                ("Armour", armour),
                ("Evasion", evasion),
                ("EnergyShield", energy_shield),
                ("Ward", ward),
                ("BlockChance", block_chance),
                ("MovementPenalty", movement_penalty),
            ] {
                let expected = finite(table.fields.get(name))
                    .map_or(ItemObservationNumericField::Absent, |value| {
                        ItemObservationNumericField::Finite { value }
                    });
                assert_eq!(actual, expected, "{}: {name}", row.source_base);
            }
            assert_eq!(
                row.can_observe_defences(),
                !weapon_truthy && row.defence_base_retarget == ItemObservationRetarget::None
            );
        } else {
            assert_eq!(
                row.armour,
                ItemObservationArmourField::Absent,
                "{}",
                row.source_base
            );
            assert!(!row.can_observe_defences());
        }
    }
    let get = |name: &str| {
        export
            .catalog()
            .bases
            .iter()
            .find(|b| b.source_base == name)
            .unwrap()
    };
    assert!(get("Sacred Focus").can_observe_defences());
    assert!(get("Frayed Shoes").can_observe_defences());
    assert!(get("Vile Robe").can_observe_defences());
    assert!(!get("Grand Spear").can_observe_defences());
    assert!(!get("Iron Ring").can_observe_defences());
    assert!(get("Shrine Sceptre").can_observe_spirit());
    assert!(!get("Ashen Staff").can_observe_spirit());
    assert_eq!(
        get("Rawhide Belt").charm_slots,
        ItemObservationNumericField::Finite { value: 0.0 }
    );
    assert!(get("Rawhide Belt").can_observe_charm_slots());
    assert!(export.evidence().defence_observation_bases > 0);
    assert!(export.evidence().spirit_observation_bases > 0);
    assert!(export.evidence().charm_slot_observation_bases > 0);
}
#[test]
fn exact_output_binding_rejects_forged_facts_and_preserves_old_base_catalog() {
    let export = authenticated();
    let bytes = export.catalog_bytes();
    let mut expected = serde_json::to_vec_pretty(export.catalog()).unwrap();
    expected.push(b'\n');
    assert_eq!(bytes, expected);
    assert_eq!(
        export.evidence().catalog_sha256,
        format!("{:x}", Sha256::digest(bytes))
    );
    assert_eq!(
        export.evidence().base_catalog_sha256,
        export.base_export().evidence().catalog_sha256
    );
    assert_eq!(
        export.catalog().source,
        export.base_export().catalog().source
    );
    assert_eq!(
        export.evidence().source_manifest_sha256,
        source::manifest_sha256()
    );
    let prior = export.base_export();
    let mut base_bytes = serde_json::to_vec_pretty(prior.catalog()).unwrap();
    base_bytes.push(b'\n');
    assert_eq!(prior.catalog_bytes(), base_bytes);
    assert_eq!(
        prior.catalog_bytes(),
        include_bytes!("../../../data/owned/poe2/3887ae68/item-bases/catalog.json")
    );
    let decoded = decode_item_observation_catalog(bytes, Default::default()).unwrap();
    export.validate_catalog(&decoded).unwrap();
    for change in 0..6 {
        let mut candidate = decoded.clone();
        match change {
            0 => {
                candidate.bases[0].weapon_branch =
                    if candidate.bases[0].weapon_branch == ItemObservationWeaponBranch::Absent {
                        ItemObservationWeaponBranch::Truthy
                    } else {
                        ItemObservationWeaponBranch::Absent
                    }
            }
            1 => candidate.bases[0].spirit = ItemObservationNumericField::Finite { value: 999.0 },
            2 => candidate.bases[0].defence_base_retarget = ItemObservationRetarget::Possible,
            3 => {
                candidate.bases.pop();
            }
            4 => candidate.source.files[0].sha256 = "0".repeat(64),
            _ => {
                candidate
                    .bases
                    .iter_mut()
                    .find(|b| b.source_base == "Rawhide Belt")
                    .unwrap()
                    .charm_slots = ItemObservationNumericField::Finite { value: -0.0 }
            }
        }
        assert!(
            export.validate_catalog(&candidate).is_err(),
            "forgery {change}"
        );
    }
}
#[test]
fn acquisition_requires_verified_source_and_respects_limits() {
    let missing = tempfile::tempdir().unwrap();
    assert!(export_owned_item_observations(missing.path(), Default::default()).is_err());
    for limits in [
        ItemObservationLimits {
            max_work: 0,
            ..Default::default()
        },
        ItemObservationLimits {
            max_catalog_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        let error = export_owned_item_observations(
            &root(),
            ItemObservationExportLimits {
                catalog: limits,
                ..Default::default()
            },
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("invalid"), "{error}");
    }
    let error = export_owned_item_observations(
        &root(),
        ItemObservationExportLimits {
            catalog: ItemObservationLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap_err()
    .to_string();
    assert!(error.contains("catalog byte limit"), "{error}");
}
