//! Authenticated base-generated prefix evidence, independent of item kind.
use std::{collections::BTreeSet, fs, path::PathBuf, sync::OnceLock};

use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::owned_item_layouts::{ItemBaseGeneratedPrefix, ItemBaseLayoutCatalog};
use poe_optimizer_pob::{
    owned_item_bases::export_owned_item_bases,
    owned_item_layouts::{
        AuthenticatedItemLayoutExport, ItemLayoutExportLimits, export_owned_item_layouts,
    },
    source,
};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedItemLayoutExport {
    static EXPORT: OnceLock<AuthenticatedItemLayoutExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_item_layouts(&root(), Default::default()).unwrap())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn every_final_base_uses_actual_constructed_buff_membership_including_empty_strings() {
    let export = authenticated();
    let snapshot = bundled_snapshot().unwrap();
    let constructed = snapshot.item_loading();
    assert_eq!(export.catalog().bases.len(), 1756);
    assert_eq!(export.catalog().bases.len(), constructed.bases().len());
    let mut generated = 0;
    for row in &export.catalog().bases {
        let original = constructed.base(&row.source_base).unwrap();
        let mut count = 0;
        for field in ["flask", "charm"] {
            let Some(parent) = original.field(field) else {
                continue;
            };
            let parent = parent
                .as_table()
                .expect("reviewed constructed parent table");
            assert!(parent.indexed.is_empty());
            if let Some(buff) = parent.fields.get("buff") {
                let buff = buff.as_array().expect("reviewed constructed buff sequence");
                assert!(buff.iter().all(|v| matches!(v, ItemMetadataValue::Text(_))));
                count += buff.len();
            }
        }
        assert_eq!(
            row.prefix,
            if count == 0 {
                ItemBaseGeneratedPrefix::Absent
            } else {
                ItemBaseGeneratedPrefix::Present
            },
            "{}",
            row.source_base
        );
        generated += count;
    }
    assert_eq!(generated, 13);
    assert_eq!(export.evidence().bases, 1756);
    assert_eq!(export.evidence().absent_prefixes, 1743);
    assert_eq!(export.evidence().present_prefixes, 13);
    assert_eq!(export.evidence().unsupported_prefixes, 0);
    assert_eq!(export.evidence().buff_lines, 13);
    for (base, expected) in [
        ("Cleansing Charm", ItemBaseGeneratedPrefix::Present),
        ("Sapphire Charm", ItemBaseGeneratedPrefix::Present),
        ("Lesser Life Flask", ItemBaseGeneratedPrefix::Absent),
        ("Sinister Quarterstaff", ItemBaseGeneratedPrefix::Absent),
        ("Ashen Staff", ItemBaseGeneratedPrefix::Absent),
        ("Sacred Focus", ItemBaseGeneratedPrefix::Absent),
    ] {
        assert_eq!(
            export
                .catalog()
                .bases
                .iter()
                .find(|r| r.source_base == base)
                .unwrap()
                .prefix,
            expected,
            "{base}"
        );
    }
}

#[test]
fn exact_finite_catalog_hash_has_provenance_and_preserves_base_v1_bytes() {
    let export = authenticated();
    let base = export_owned_item_bases(&root(), Default::default()).unwrap();
    assert_eq!(export.base_export().catalog_bytes(), base.catalog_bytes());
    assert_eq!(export.base_export().evidence(), base.evidence());
    let mut bytes = serde_json::to_vec_pretty(export.catalog()).unwrap();
    bytes.push(b'\n');
    assert_eq!(export.catalog_bytes(), bytes);
    let evidence = export.evidence();
    assert_eq!(evidence.catalog_sha256, sha(&bytes));
    assert_eq!(evidence.base_catalog_sha256, sha(base.catalog_bytes()));
    assert_ne!(evidence.catalog_sha256, evidence.base_catalog_sha256);
    assert_ne!(evidence.extractor_sha256, base.evidence().extractor_sha256);
    assert_eq!(evidence.source_manifest_sha256, source::manifest_sha256());
    assert_eq!(
        evidence.source_catalog_sha256,
        base.evidence().source_catalog_sha256
    );
    assert_eq!(
        evidence.extraction_source_files,
        base.evidence().extraction_source_files
    );
    assert_eq!(export.catalog().source, base.catalog().source);
    assert_eq!(export.catalog().source.files.len(), 29);
    assert_eq!(evidence.extraction_source_files.len(), 89);
    assert!(
        export
            .catalog()
            .source
            .files
            .iter()
            .any(|p| p.path == "src/Classes/Item.lua")
    );
    for pin in &export.catalog().source.files {
        assert!(evidence.extraction_source_files.contains(pin));
        assert_eq!(pin.sha256, source::expected_file_sha256(&pin.path).unwrap());
    }
    assert!(
        export
            .catalog()
            .bases
            .windows(2)
            .all(|p| p[0].source_base < p[1].source_base)
    );
    let decoded: ItemBaseLayoutCatalog = serde_json::from_slice(&bytes).unwrap();
    export.validate_catalog(&decoded).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["schema_version", "source", "bases"])
    );
    for row in value["bases"].as_array().unwrap() {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["source_base", "prefix"])
        );
    }
    let repeated = export_owned_item_layouts(&root(), Default::default()).unwrap();
    assert_eq!(export.catalog_bytes(), repeated.catalog_bytes());
    assert_eq!(export.evidence(), repeated.evidence());
}

#[test]
fn copied_pins_cannot_authenticate_absence_membership_or_source_changes() {
    let export = authenticated();
    for change in 0..6 {
        let mut candidate = export.catalog().clone();
        match change {
            0 => {
                candidate
                    .bases
                    .iter_mut()
                    .find(|r| r.prefix == ItemBaseGeneratedPrefix::Present)
                    .unwrap()
                    .prefix = ItemBaseGeneratedPrefix::Absent
            }
            1 => {
                candidate.bases.pop();
            }
            2 => candidate.bases.swap(0, 1),
            3 => candidate.bases[0].source_base = "Invented Base".into(),
            4 => candidate.schema_version += 1,
            _ => candidate.bases[0].prefix = ItemBaseGeneratedPrefix::Unsupported,
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
}

#[test]
fn source_authority_cannot_be_replaced_by_copied_finite_catalogs() {
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
    assert!(export_owned_item_layouts(directory.path(), Default::default()).is_err());
    let manifest = source::read_verified_text(&root(), "manifest.xml").unwrap();
    fs::write(
        directory.path().join("manifest.xml"),
        format!("{manifest}\n<!-- changed -->\n"),
    )
    .unwrap();
    assert!(
        export_owned_item_layouts(directory.path(), Default::default())
            .unwrap_err()
            .to_string()
            .contains("manifest.xml")
    );
}

#[test]
fn limits_bound_layouts_buff_work_source_and_exact_output() {
    for limits in [
        ItemLayoutExportLimits {
            max_layouts: 0,
            ..Default::default()
        },
        ItemLayoutExportLimits {
            max_buff_lines_per_base: usize::MAX,
            ..Default::default()
        },
        ItemLayoutExportLimits {
            max_total_buff_lines: 0,
            ..Default::default()
        },
        ItemLayoutExportLimits {
            max_catalog_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(
            export_owned_item_layouts(&root(), limits)
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
    }
    for (limits, message) in [
        (
            ItemLayoutExportLimits {
                max_layouts: 1,
                ..Default::default()
            },
            "layout count limit",
        ),
        (
            ItemLayoutExportLimits {
                max_total_buff_lines: 1,
                ..Default::default()
            },
            "total buff lines limit",
        ),
        (
            ItemLayoutExportLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            },
            "catalog byte limit",
        ),
        (
            ItemLayoutExportLimits {
                bases: poe_optimizer_pob::owned_item_bases::ItemBaseExportLimits {
                    max_source_files: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            "source file limit",
        ),
    ] {
        let error = export_owned_item_layouts(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains(message), "{error}");
    }
}
