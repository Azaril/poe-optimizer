//! Finite offline projection and provenance validation, without PoB UI/evaluation.
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::owned_item_bases::{ItemBaseCatalog, ItemBaseWeaponField};
use poe_optimizer_pob::{
    owned_item_bases::{
        AuthenticatedItemBaseExport, ItemBaseExportLimits, export_owned_item_bases,
    },
    source,
};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedItemBaseExport {
    static EXPORT: OnceLock<AuthenticatedItemBaseExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_item_bases(&root(), Default::default()).unwrap())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn all_final_bases_match_independent_reviewed_catalog_including_nonphysical_profiles() {
    let export = authenticated();
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.item_loading();
    assert_eq!(export.catalog().bases.len(), 1756);
    assert_eq!(export.evidence().table_weapon_fields, 337);
    assert_eq!(export.evidence().absent_weapon_fields, 1419);
    assert_eq!(export.evidence().unsupported_weapon_fields, 0);
    assert_eq!(original.bases().len(), export.catalog().bases.len());
    let mut nonphysical_profiles = 0;
    for row in &export.catalog().bases {
        let base = original.base(&row.name).unwrap();
        assert_eq!(row.item_type, base.item_type);
        assert_eq!(row.source_module, base.source_module);
        match base.field("weapon") {
            None => assert_eq!(row.weapon_field, ItemBaseWeaponField::Absent),
            Some(ItemMetadataValue::Table(table)) => {
                assert_eq!(row.weapon_field, ItemBaseWeaponField::Table);
                if !table.fields.contains_key("PhysicalMin") {
                    nonphysical_profiles += 1;
                    assert!(table.fields.keys().any(|name| name.ends_with("Min")));
                }
            }
            other => panic!("unexpected reviewed weapon field: {other:?}"),
        }
    }
    assert_eq!(nonphysical_profiles, 6);
    for (name, expected) in [
        ("Ashen Staff", ItemBaseWeaponField::Absent),
        ("Sinister Quarterstaff", ItemBaseWeaponField::Table),
        ("Rattling Sceptre", ItemBaseWeaponField::Absent),
        ("Sacred Focus", ItemBaseWeaponField::Absent),
        ("Grand Spear", ItemBaseWeaponField::Table),
        ("Energy Blade One Handed", ItemBaseWeaponField::Table),
        ("Energy Blade Two Handed", ItemBaseWeaponField::Table),
    ] {
        let rows: Vec<_> = export
            .catalog()
            .bases
            .iter()
            .filter(|row| row.name == name)
            .collect();
        assert_eq!(
            rows.len(),
            1,
            "final table must collapse source reassignments: {name}"
        );
        assert_eq!(rows[0].weapon_field, expected, "{name}");
    }
    let staff_kinds: BTreeSet<_> = export
        .catalog()
        .bases
        .iter()
        .filter(|row| row.item_type == "Staff")
        .map(|row| format!("{:?}", row.weapon_field))
        .collect();
    assert_eq!(
        staff_kinds,
        BTreeSet::from(["Absent".into(), "Table".into()])
    );
}

#[test]
fn exact_catalog_bytes_have_their_own_hash_and_narrow_pin_retains_broad_evidence() {
    let export = authenticated();
    let evidence = export.evidence();
    let mut expected_bytes = serde_json::to_vec_pretty(export.catalog()).unwrap();
    expected_bytes.push(b'\n');
    assert_eq!(export.catalog_bytes(), expected_bytes);
    assert_eq!(evidence.catalog_sha256, sha(export.catalog_bytes()));
    assert_eq!(evidence.source_manifest_sha256, source::manifest_sha256());
    assert_ne!(evidence.catalog_sha256, evidence.source_manifest_sha256);
    let snapshot = bundled_snapshot().unwrap();
    assert_eq!(
        evidence.source_catalog_sha256,
        sha(&serde_json::to_vec(snapshot.item_loading().data()).unwrap())
    );
    assert_ne!(evidence.catalog_sha256, evidence.source_catalog_sha256);
    assert_eq!(export.catalog().source.files.len(), 29);
    assert_eq!(evidence.extraction_source_files.len(), 89);
    for pin in &export.catalog().source.files {
        assert!(evidence.extraction_source_files.contains(pin));
        assert_eq!(pin.sha256, source::expected_file_sha256(&pin.path).unwrap());
    }
    assert!(
        export
            .catalog()
            .bases
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name)
    );
    let decoded: ItemBaseCatalog = serde_json::from_slice(export.catalog_bytes()).unwrap();
    export.validate_catalog(&decoded).unwrap();
    let value: serde_json::Value = serde_json::from_slice(export.catalog_bytes()).unwrap();
    for row in value["bases"].as_array().unwrap() {
        let keys: BTreeSet<_> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            BTreeSet::from(["name", "item_type", "source_module", "weapon_field"])
        );
    }
}

#[test]
fn copied_source_metadata_does_not_authenticate_changed_catalog_content() {
    let export = authenticated();
    for change in 0..7 {
        let mut candidate: ItemBaseCatalog =
            serde_json::from_slice(export.catalog_bytes()).unwrap();
        match change {
            0 => {
                candidate
                    .bases
                    .iter_mut()
                    .find(|row| row.name == "Ashen Staff")
                    .unwrap()
                    .weapon_field = ItemBaseWeaponField::Table
            }
            1 => candidate.bases[0].item_type = "Forged type".into(),
            2 => candidate.bases[0].name = "Forged name".into(),
            3 => candidate.bases[0].source_module = "src/Data/Bases/staff.lua".into(),
            4 => {
                candidate.bases.pop();
            }
            5 => candidate.bases.swap(0, 1),
            _ => candidate.schema_version += 1,
        }
        assert_eq!(candidate.source, export.catalog().source);
        let error = export.validate_catalog(&candidate).unwrap_err().to_string();
        assert!(error.contains("independently authenticated"), "{error}");
    }
    let mut candidate = export.catalog().clone();
    candidate.source.files[0].sha256 = "0".repeat(64);
    assert!(export.validate_catalog(&candidate).is_err());
}

#[test]
fn source_root_is_independently_verified_before_any_construction() {
    let missing = tempfile::tempdir().unwrap();
    assert!(export_owned_item_bases(missing.path(), Default::default()).is_err());
    // The root manifest is the first pinned file. Matching a caller-authored
    // revision string or catalog beside it cannot override its reviewed bytes.
    let manifest = source::read_verified_text(&root(), "manifest.xml").unwrap();
    fs::write(
        missing.path().join("manifest.xml"),
        format!("{manifest}\n<!-- changed -->\n"),
    )
    .unwrap();
    fs::write(
        missing.path().join("catalog.json"),
        authenticated().catalog_bytes(),
    )
    .unwrap();
    let error = export_owned_item_bases(missing.path(), Default::default())
        .unwrap_err()
        .to_string();
    assert!(error.contains("manifest.xml"), "{error}");
}

#[test]
fn resource_limits_reject_invalid_and_excess_work() {
    for limits in [
        ItemBaseExportLimits {
            max_source_files: 0,
            ..Default::default()
        },
        ItemBaseExportLimits {
            max_source_bytes: usize::MAX,
            ..Default::default()
        },
        ItemBaseExportLimits {
            max_bases: 0,
            ..Default::default()
        },
        ItemBaseExportLimits {
            max_text_bytes: usize::MAX,
            ..Default::default()
        },
        ItemBaseExportLimits {
            max_catalog_bytes: 0,
            ..Default::default()
        },
    ] {
        let error = export_owned_item_bases(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains("invalid"), "{error}");
    }
    for (limits, expected) in [
        (
            ItemBaseExportLimits {
                max_source_files: 1,
                ..Default::default()
            },
            "source file limit",
        ),
        (
            ItemBaseExportLimits {
                max_source_bytes: 1,
                ..Default::default()
            },
            "source byte limit",
        ),
        (
            ItemBaseExportLimits {
                max_bases: 1,
                ..Default::default()
            },
            "base count limit",
        ),
        (
            ItemBaseExportLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            },
            "catalog byte limit",
        ),
    ] {
        let error = export_owned_item_bases(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn repeated_verified_extraction_is_deterministic() {
    let first = authenticated();
    let second = export_owned_item_bases(&root(), Default::default()).unwrap();
    assert_eq!(first.catalog_bytes(), second.catalog_bytes());
    assert_eq!(first.evidence(), second.evidence());
    first.validate_catalog(second.catalog()).unwrap();
}
