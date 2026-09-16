//! Exhaustive finite profile projection checks against the separately retained
//! game-data snapshot, with source and content authentication tested independently.
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::{
    owned_item_bases::ItemBaseWeaponField, owned_weapon_profiles::WeaponProfileCatalog,
};
use poe_optimizer_pob::{
    owned_item_bases::export_owned_item_bases,
    owned_weapon_profiles::{
        AuthenticatedWeaponProfileExport, WeaponProfileExportLimits, export_owned_weapon_profiles,
    },
    source,
};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedWeaponProfileExport {
    static EXPORT: OnceLock<AuthenticatedWeaponProfileExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_weapon_profiles(&root(), Default::default()).unwrap())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn every_final_profile_and_numeric_field_matches_the_independent_snapshot_exactly() {
    let export = authenticated();
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.item_loading();
    let mut all_fields = BTreeSet::new();
    let mut fields = 0;
    let mut nonphysical = 0;
    assert_eq!(export.catalog().profiles.len(), 337);
    let expected_names: BTreeSet<_> = original
        .bases()
        .iter()
        .filter(|base| {
            matches!(
                base.field("weapon"),
                Some(ItemMetadataValue::Table(_) | ItemMetadataValue::Array(_))
            )
        })
        .map(|base| base.name.as_str())
        .collect();
    assert_eq!(
        expected_names,
        export
            .catalog()
            .profiles
            .iter()
            .map(|row| row.base.as_str())
            .collect()
    );
    for row in &export.catalog().profiles {
        let Some(ItemMetadataValue::Table(table)) =
            original.base(&row.base).unwrap().field("weapon")
        else {
            panic!(
                "reviewed profile must have a named numeric table: {}",
                row.base
            );
        };
        assert!(table.indexed.is_empty());
        assert_eq!(row.fields.len(), table.fields.len(), "{}", row.base);
        for (key, value) in &table.fields {
            let ItemMetadataValue::Number(expected) = value else {
                panic!("non numeric retained field {key}");
            };
            assert_eq!(
                row.fields[key].to_bits(),
                expected.to_bits(),
                "{} {key}",
                row.base
            );
            all_fields.insert(key.clone());
            fields += 1;
        }
        if !row.fields.contains_key("PhysicalMin") {
            nonphysical += 1;
        }
    }
    assert_eq!(nonphysical, 6);
    assert_eq!(all_fields.len(), 14);
    assert_eq!(export.evidence().fields, fields);
    assert_eq!(
        export
            .evidence()
            .field_names
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>(),
        all_fields
    );
    assert_eq!(export.evidence().profiles, 337);
    assert!(
        export
            .catalog()
            .profiles
            .windows(2)
            .all(|rows| rows[0].base < rows[1].base)
    );
}

#[test]
fn authored_absence_is_not_a_zero_or_a_missing_whole_profile() {
    let export = authenticated();
    let find = |name: &str| {
        export
            .catalog()
            .profiles
            .iter()
            .find(|row| row.base == name)
    };
    let spear = find("Grand Spear").unwrap();
    assert!(!spear.fields.contains_key("ReloadTimeBase"));
    assert!(spear.fields.contains_key("PhysicalMin"));
    for name in ["Ashen Staff", "Rattling Sceptre", "Sacred Focus"] {
        assert!(find(name).is_none(), "{name}");
        assert_eq!(
            export
                .base_export()
                .catalog()
                .bases
                .iter()
                .find(|row| row.name == name)
                .unwrap()
                .weapon_field,
            ItemBaseWeaponField::Absent
        );
    }
    let elemental = export
        .catalog()
        .profiles
        .iter()
        .find(|row| !row.fields.contains_key("PhysicalMin"))
        .unwrap();
    assert!(!elemental.fields.contains_key("PhysicalMax"));
    assert!(elemental.fields.keys().any(|key| key.ends_with("Min")));
    let crossbow = export
        .catalog()
        .profiles
        .iter()
        .find(|row| row.fields.contains_key("ReloadTimeBase"))
        .unwrap();
    assert!(crossbow.fields["ReloadTimeBase"] > 0.0);
    let mut forged = export.catalog().clone();
    forged
        .profiles
        .iter_mut()
        .find(|row| row.base == spear.base)
        .unwrap()
        .fields
        .insert("ReloadTimeBase".into(), 0.0);
    assert!(export.validate_catalog(&forged).is_err());
}

#[test]
fn exact_profile_hash_is_distinct_and_base_v1_bytes_remain_unchanged() {
    let export = authenticated();
    let separate_base = export_owned_item_bases(&root(), Default::default()).unwrap();
    assert_eq!(
        export.base_export().catalog_bytes(),
        separate_base.catalog_bytes()
    );
    assert_eq!(export.base_export().evidence(), separate_base.evidence());
    let mut expected = serde_json::to_vec_pretty(export.catalog()).unwrap();
    expected.push(b'\n');
    assert_eq!(expected, export.catalog_bytes());
    let evidence = export.evidence();
    assert_eq!(evidence.catalog_sha256, sha(export.catalog_bytes()));
    assert_eq!(
        evidence.base_catalog_sha256,
        sha(separate_base.catalog_bytes())
    );
    assert_eq!(
        export.catalog().base_catalog_sha256,
        evidence.base_catalog_sha256
    );
    assert_ne!(evidence.catalog_sha256, evidence.base_catalog_sha256);
    assert_eq!(export.catalog().source, separate_base.catalog().source);
    assert_eq!(export.catalog().source.files.len(), 29);
    assert_eq!(evidence.extraction_source_files.len(), 89);
    assert_eq!(
        evidence.extraction_source_files,
        separate_base.evidence().extraction_source_files
    );
    assert_eq!(
        evidence.source_catalog_sha256,
        separate_base.evidence().source_catalog_sha256
    );
    assert_eq!(evidence.source_manifest_sha256, source::manifest_sha256());
    assert_ne!(
        evidence.extractor_sha256,
        separate_base.evidence().extractor_sha256
    );
    for pin in &export.catalog().source.files {
        assert!(evidence.extraction_source_files.contains(pin));
        assert_eq!(pin.sha256, source::expected_file_sha256(&pin.path).unwrap());
    }
    let decoded: WeaponProfileCatalog = serde_json::from_slice(export.catalog_bytes()).unwrap();
    export.validate_catalog(&decoded).unwrap();
    let json: serde_json::Value = serde_json::from_slice(export.catalog_bytes()).unwrap();
    for row in json["profiles"].as_array().unwrap() {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["base", "fields"])
        );
    }
}

#[test]
fn copied_source_metadata_cannot_authenticate_replacement_values_or_membership() {
    let export = authenticated();
    for change in 0..8 {
        let mut candidate = export.catalog().clone();
        match change {
            0 => *candidate.profiles[0].fields.values_mut().next().unwrap() += 1.0,
            1 => {
                candidate.profiles[0].fields.pop_first();
            }
            2 => {
                candidate.profiles[0].fields.insert("Invented".into(), 0.0);
            }
            3 => {
                candidate.profiles.pop();
            }
            4 => candidate.profiles.swap(0, 1),
            5 => candidate.profiles[0].base = "Ashen Staff".into(),
            6 => candidate.schema_version += 1,
            _ => candidate.base_catalog_sha256 = "0".repeat(64),
        }
        assert_eq!(candidate.source, export.catalog().source);
        assert!(
            export.validate_catalog(&candidate).is_err(),
            "mutation {change}"
        );
    }
    let mut candidate = export.catalog().clone();
    candidate.source.files[0].sha256 = "0".repeat(64);
    assert!(export.validate_catalog(&candidate).is_err());
    let mut candidate = export.catalog().clone();
    *candidate.profiles[0].fields.values_mut().next().unwrap() = f64::NAN;
    assert!(export.validate_catalog(&candidate).is_err());
}

#[test]
fn source_authority_is_required_even_with_copied_owned_exports() {
    let directory = tempfile::tempdir().unwrap();
    fs::write(
        directory.path().join("catalog.json"),
        authenticated().catalog_bytes(),
    )
    .unwrap();
    fs::write(
        directory.path().join("base-catalog.json"),
        authenticated().base_export().catalog_bytes(),
    )
    .unwrap();
    assert!(export_owned_weapon_profiles(directory.path(), Default::default()).is_err());
    let manifest = source::read_verified_text(&root(), "manifest.xml").unwrap();
    fs::write(
        directory.path().join("manifest.xml"),
        format!("{manifest}\n<!-- changed -->\n"),
    )
    .unwrap();
    assert!(
        export_owned_weapon_profiles(directory.path(), Default::default())
            .unwrap_err()
            .to_string()
            .contains("manifest.xml")
    );
}

#[test]
fn limits_bound_profile_count_fields_source_and_serialized_output() {
    for limits in [
        WeaponProfileExportLimits {
            max_profiles: 0,
            ..Default::default()
        },
        WeaponProfileExportLimits {
            max_fields_per_profile: usize::MAX,
            ..Default::default()
        },
        WeaponProfileExportLimits {
            max_total_fields: 0,
            ..Default::default()
        },
        WeaponProfileExportLimits {
            max_catalog_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(
            export_owned_weapon_profiles(&root(), limits)
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
    }
    for (limits, expected) in [
        (
            WeaponProfileExportLimits {
                max_profiles: 1,
                ..Default::default()
            },
            "profile count limit",
        ),
        (
            WeaponProfileExportLimits {
                max_fields_per_profile: 1,
                ..Default::default()
            },
            "fields per profile limit",
        ),
        (
            WeaponProfileExportLimits {
                max_total_fields: 1,
                ..Default::default()
            },
            "total field count limit",
        ),
        (
            WeaponProfileExportLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            },
            "catalog byte limit",
        ),
        (
            WeaponProfileExportLimits {
                bases: poe_optimizer_pob::owned_item_bases::ItemBaseExportLimits {
                    max_source_files: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            "source file limit",
        ),
    ] {
        let error = export_owned_weapon_profiles(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn repeated_authenticated_projection_is_deterministic() {
    let first = authenticated();
    let second = export_owned_weapon_profiles(&root(), Default::default()).unwrap();
    assert_eq!(first.catalog_bytes(), second.catalog_bytes());
    assert_eq!(first.evidence(), second.evidence());
    first.validate_catalog(second.catalog()).unwrap();
}
