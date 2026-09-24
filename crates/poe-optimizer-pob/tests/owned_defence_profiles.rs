//! Exhaustive raw defence-base projection against the separately retained final
//! constructed snapshot. These are acquisition/presence checks, not local item
//! assembly, equipped activation, actor defences, or complete-build parity.
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::OnceLock,
};

use poe_optimizer_data::{game_data::bundled_snapshot, item_loading::ItemMetadataValue};
use poe_optimizer_import::owned_defence_profiles::{DefenceProfileCatalog, DefenceProfilePresence};
use poe_optimizer_pob::{
    owned_defence_profiles::{
        AuthenticatedDefenceProfileExport, DefenceProfileExportLimits,
        export_owned_defence_profiles,
    },
    owned_item_bases::{ItemBaseExportLimits, export_owned_item_bases},
    source,
};
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn authenticated() -> &'static AuthenticatedDefenceProfileExport {
    static EXPORT: OnceLock<AuthenticatedDefenceProfileExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_defence_profiles(&root(), Default::default()).unwrap())
}
fn sha(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn fields(profile: &DefenceProfilePresence) -> Option<&BTreeMap<String, f64>> {
    match profile {
        DefenceProfilePresence::Absent => None,
        DefenceProfilePresence::Table { fields } => Some(fields),
    }
}
fn fields_mut(profile: &mut DefenceProfilePresence) -> &mut BTreeMap<String, f64> {
    match profile {
        DefenceProfilePresence::Absent => panic!("fixture must have a table"),
        DefenceProfilePresence::Table { fields } => fields,
    }
}

#[test]
fn every_final_base_presence_and_numeric_field_matches_the_independent_snapshot() {
    let export = authenticated();
    let snapshot = bundled_snapshot().unwrap();
    let original = snapshot.item_loading();
    assert_eq!(original.bases().len(), 1756);
    assert_eq!(export.catalog().profiles.len(), 1756);
    assert_eq!(export.evidence().profiles, 1756);
    assert_eq!(
        original
            .bases()
            .iter()
            .map(|base| base.name.as_str())
            .collect::<BTreeSet<_>>(),
        export
            .catalog()
            .profiles
            .iter()
            .map(|row| row.base.as_str())
            .collect::<BTreeSet<_>>()
    );
    let mut counts = BTreeMap::new();
    let (mut tables, mut absent, mut empty) = (0, 0, 0);
    for row in &export.catalog().profiles {
        let expected = original.base(&row.base).unwrap().field("armour");
        match (expected, &row.profile) {
            (None, DefenceProfilePresence::Absent) => absent += 1,
            (Some(ItemMetadataValue::Table(table)), DefenceProfilePresence::Table { fields }) => {
                assert!(table.indexed.is_empty(), "{}", row.base);
                assert_eq!(fields.len(), table.fields.len(), "{}", row.base);
                tables += 1;
                empty += usize::from(fields.is_empty());
                for (key, expected) in &table.fields {
                    let ItemMetadataValue::Number(expected) = expected else {
                        panic!("nonnumeric final defence field: {} {key}", row.base);
                    };
                    assert_eq!(
                        fields[key].to_bits(),
                        expected.to_bits(),
                        "{} {key}",
                        row.base
                    );
                    *counts.entry(key.clone()).or_insert(0usize) += 1;
                }
            }
            (Some(ItemMetadataValue::Array(array)), DefenceProfilePresence::Table { fields })
                if array.is_empty() =>
            {
                assert!(fields.is_empty(), "{}", row.base);
                tables += 1;
                empty += 1;
            }
            _ => panic!("whole-profile presence differs for {}", row.base),
        }
    }
    assert_eq!((tables, absent, empty), (1240, 516, 6));
    assert_eq!(
        counts,
        BTreeMap::from([
            ("Armour".into(), 588),
            ("Evasion".into(), 540),
            ("EnergyShield".into(), 550),
            ("Ward".into(), 675),
            ("BlockChance".into(), 193),
            ("MovementPenalty".into(), 485),
        ])
    );
    assert_eq!(export.evidence().fields, 3031);
    assert_eq!(export.evidence().fields, counts.values().sum::<usize>());
    assert_eq!(export.evidence().table_profiles, tables);
    assert_eq!(export.evidence().absent_profiles, absent);
    assert_eq!(export.evidence().empty_profiles, empty);
    assert_eq!(
        export.evidence().field_names,
        counts.into_keys().collect::<Vec<_>>()
    );
    assert!(
        export
            .catalog()
            .profiles
            .windows(2)
            .all(|rows| rows[0].base < rows[1].base)
    );
}

#[test]
fn whole_profile_absence_empty_tables_and_missing_fields_remain_distinct() {
    let export = authenticated();
    let find = |name: &str| {
        &export
            .catalog()
            .profiles
            .iter()
            .find(|row| row.base == name)
            .unwrap()
            .profile
    };
    for name in ["Iron Ring", "Ashen Staff", "Grand Spear"] {
        assert!(
            matches!(find(name), DefenceProfilePresence::Absent),
            "{name}"
        );
    }
    let empty_names = export
        .catalog()
        .profiles
        .iter()
        .filter_map(|row| {
            fields(&row.profile)
                .filter(|values| values.is_empty())
                .map(|_| row.base.as_str())
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        empty_names,
        BTreeSet::from([
            "Fists of Stone",
            "Garment",
            "Golden Caligae",
            "Golden Visage",
            "Golden Wreath",
            "Runeforged Fists of Stone",
        ])
    );
    let cuirass = fields(find("Rusted Cuirass")).unwrap();
    assert_eq!(cuirass["Armour"], 45.0);
    assert_eq!(cuirass["MovementPenalty"].to_bits(), 0.05f64.to_bits());
    assert!(!cuirass.contains_key("Evasion") && !cuirass.contains_key("Ward"));
    assert!(!cuirass.contains_key("BlockChance"));
    assert!(
        fields(find("Sacred Focus"))
            .unwrap()
            .contains_key("EnergyShield")
    );
    let mut zero_forgery = export.catalog().clone();
    let row = zero_forgery
        .profiles
        .iter_mut()
        .find(|row| row.base == "Rusted Cuirass")
        .unwrap();
    fields_mut(&mut row.profile).insert("Evasion".into(), 0.0);
    assert!(export.validate_catalog(&zero_forgery).is_err());
}

#[test]
fn exact_export_identity_and_existing_base_catalog_bytes_remain_bound() {
    let export = authenticated();
    let separate_base = export_owned_item_bases(&root(), Default::default()).unwrap();
    assert_eq!(
        export.base_export().catalog_bytes(),
        separate_base.catalog_bytes()
    );
    assert_eq!(export.base_export().evidence(), separate_base.evidence());
    let mut bytes = serde_json::to_vec_pretty(export.catalog()).unwrap();
    bytes.push(b'\n');
    assert_eq!(bytes, export.catalog_bytes());
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
    let decoded: DefenceProfileCatalog = serde_json::from_slice(export.catalog_bytes()).unwrap();
    export.validate_catalog(&decoded).unwrap();
    let json: serde_json::Value = serde_json::from_slice(export.catalog_bytes()).unwrap();
    for row in json["profiles"].as_array().unwrap() {
        assert_eq!(
            row.as_object()
                .unwrap()
                .keys()
                .map(String::as_str)
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["base", "profile"])
        );
    }
}

#[test]
fn copied_metadata_cannot_authenticate_changed_values_presence_or_membership() {
    let export = authenticated();
    for change in 0..11 {
        let mut candidate = export.catalog().clone();
        let present = candidate
            .profiles
            .iter()
            .position(|row| fields(&row.profile).is_some_and(|fields| !fields.is_empty()))
            .unwrap();
        match change {
            0 => {
                *fields_mut(&mut candidate.profiles[present].profile)
                    .values_mut()
                    .next()
                    .unwrap() += 1.0
            }
            1 => {
                fields_mut(&mut candidate.profiles[present].profile).pop_first();
            }
            2 => {
                fields_mut(&mut candidate.profiles[present].profile).insert("Invented".into(), 0.0);
            }
            3 => {
                candidate.profiles.pop();
            }
            4 => candidate.profiles.swap(0, 1),
            5 => candidate.profiles[present].base = "Iron Ring".into(),
            6 => candidate.schema_version += 1,
            7 => candidate.base_catalog_sha256 = "0".repeat(64),
            8 => candidate.profiles[present].profile = DefenceProfilePresence::Absent,
            9 => {
                let absent = candidate
                    .profiles
                    .iter_mut()
                    .find(|row| matches!(row.profile, DefenceProfilePresence::Absent))
                    .unwrap();
                absent.profile = DefenceProfilePresence::Table {
                    fields: BTreeMap::new(),
                };
            }
            _ => {
                let empty = candidate
                    .profiles
                    .iter_mut()
                    .find(|row| fields(&row.profile).is_some_and(|fields| fields.is_empty()))
                    .unwrap();
                empty.profile = DefenceProfilePresence::Absent;
            }
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
    let present = candidate
        .profiles
        .iter_mut()
        .find(|row| fields(&row.profile).is_some_and(|fields| !fields.is_empty()))
        .unwrap();
    *fields_mut(&mut present.profile)
        .values_mut()
        .next()
        .unwrap() = f64::NAN;
    assert!(export.validate_catalog(&candidate).is_err());
}

#[test]
fn source_authority_is_required_despite_copied_export_documents() {
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
    assert!(export_owned_defence_profiles(directory.path(), Default::default()).is_err());
    let manifest = source::read_verified_text(&root(), "manifest.xml").unwrap();
    fs::write(
        directory.path().join("manifest.xml"),
        format!("{manifest}\n<!-- changed -->\n"),
    )
    .unwrap();
    assert!(
        export_owned_defence_profiles(directory.path(), Default::default())
            .unwrap_err()
            .to_string()
            .contains("manifest.xml")
    );
}

#[test]
fn limits_bound_all_bases_raw_fields_source_and_serialized_output() {
    for limits in [
        DefenceProfileExportLimits {
            max_profiles: 0,
            ..Default::default()
        },
        DefenceProfileExportLimits {
            max_fields_per_profile: usize::MAX,
            ..Default::default()
        },
        DefenceProfileExportLimits {
            max_total_fields: 0,
            ..Default::default()
        },
        DefenceProfileExportLimits {
            max_catalog_bytes: usize::MAX,
            ..Default::default()
        },
    ] {
        assert!(
            export_owned_defence_profiles(&root(), limits)
                .unwrap_err()
                .to_string()
                .contains("invalid")
        );
    }
    for (limits, expected) in [
        (
            DefenceProfileExportLimits {
                max_profiles: 1,
                ..Default::default()
            },
            "profile count limit",
        ),
        (
            DefenceProfileExportLimits {
                max_fields_per_profile: 1,
                ..Default::default()
            },
            "fields per profile limit",
        ),
        (
            DefenceProfileExportLimits {
                max_total_fields: 1,
                ..Default::default()
            },
            "total field count limit",
        ),
        (
            DefenceProfileExportLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            },
            "catalog byte limit",
        ),
        (
            DefenceProfileExportLimits {
                bases: ItemBaseExportLimits {
                    max_source_files: 1,
                    ..Default::default()
                },
                ..Default::default()
            },
            "source file limit",
        ),
    ] {
        let error = export_owned_defence_profiles(&root(), limits)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
    }
}

#[test]
fn repeated_authenticated_projection_is_deterministic() {
    let first = authenticated();
    let second = export_owned_defence_profiles(&root(), Default::default()).unwrap();
    assert_eq!(first.catalog_bytes(), second.catalog_bytes());
    assert_eq!(first.evidence(), second.evidence());
    first.validate_catalog(second.catalog()).unwrap();
}
