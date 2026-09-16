//! Offline source-to-catalog parity. Expected values are authored independently
//! of the exporter and compare every source row/field, including excluded class 0.
use poe_optimizer_import::{
    owned_intrinsic_attack::{IntrinsicAttackClass, IntrinsicSourceValue},
    owned_mapping::{ExternalSourceSystem, SourceFilePin, SourcePin},
};
use poe_optimizer_pob::{
    owned_intrinsic_attack::{
        DATA_PATH, IntrinsicAttackExportLimits, MISC_PATH, export_owned_intrinsic_attack,
    },
    source,
};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn inputs() -> (Vec<u8>, Vec<u8>, SourcePin) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2");
    let source = SourcePin {
        system: ExternalSourceSystem::PathOfBuilding2,
        revision: source::UPSTREAM_REVISION.into(),
        files: [MISC_PATH, DATA_PATH]
            .into_iter()
            .map(|path| SourceFilePin {
                path: path.into(),
                sha256: source::expected_file_sha256(path).unwrap(),
            })
            .collect(),
    };
    (
        fs::read(root.join(MISC_PATH)).unwrap(),
        fs::read(root.join(DATA_PATH)).unwrap(),
        source,
    )
}
fn expected_classes() -> Vec<IntrinsicAttackClass> {
    [
        ("0", 6.0),
        ("1", 5.0),
        ("10", 5.0),
        ("11", 6.0),
        ("2", 5.0),
        ("6", 8.0),
        ("7", 5.0),
        ("8", 5.0),
        ("9", 6.0),
    ]
    .into_iter()
    .map(|(class_key, physical_max)| IntrinsicAttackClass {
        class_key: class_key.into(),
        fields: BTreeMap::from([
            ("AttackRate".into(), IntrinsicSourceValue::Number(1.65)),
            ("CritChance".into(), IntrinsicSourceValue::Number(5.0)),
            ("PhysicalMin".into(), IntrinsicSourceValue::Number(2.0)),
            (
                "PhysicalMax".into(),
                IntrinsicSourceValue::Number(physical_max),
            ),
            ("type".into(), IntrinsicSourceValue::Text("None".into())),
        ]),
    })
    .collect()
}

#[test]
fn pinned_source_table_matches_independent_complete_snapshot() {
    let (misc, data, source) = inputs();
    let catalog = export_owned_intrinsic_attack(&misc, &data, &source, Default::default()).unwrap();
    assert_eq!(catalog.schema_version, 1);
    assert_eq!(catalog.source, source);
    assert_eq!(catalog.classes, expected_classes());
    let encoded = serde_json::to_vec(&catalog).unwrap();
    let decoded = serde_json::from_slice::<
        poe_optimizer_import::owned_intrinsic_attack::IntrinsicAttackCatalog,
    >(&encoded)
    .unwrap();
    assert_eq!(decoded, catalog);
}

#[test]
fn crlf_and_source_pin_order_produce_identical_catalogs() {
    let (misc, data, source) = inputs();
    let expected =
        export_owned_intrinsic_attack(&misc, &data, &source, Default::default()).unwrap();
    let misc = String::from_utf8(misc).unwrap().replace("\r\n", "\n");
    let data = String::from_utf8(data).unwrap().replace("\r\n", "\n");
    let mut reversed = source.clone();
    reversed.files.reverse();
    for line_ending in ["\n", "\r\n"] {
        let actual = export_owned_intrinsic_attack(
            misc.replace('\n', line_ending).as_bytes(),
            data.replace('\n', line_ending).as_bytes(),
            &reversed,
            Default::default(),
        )
        .unwrap();
        assert_eq!(actual, expected);
    }
}

#[test]
fn changed_source_cannot_be_accepted_even_with_a_matching_caller_hash() {
    let (misc, data, source) = inputs();
    for change_misc in [true, false] {
        let mut changed_misc = misc.clone();
        let mut changed_data = data.clone();
        let (changed, path) = if change_misc {
            (&mut changed_misc, MISC_PATH)
        } else {
            (&mut changed_data, DATA_PATH)
        };
        changed.extend_from_slice(b"\nio.open('should-not-exist', 'w')\n");
        let mut repinned = source.clone();
        let normalized = String::from_utf8(changed.clone())
            .unwrap()
            .replace("\r\n", "\n");
        repinned
            .files
            .iter_mut()
            .find(|pin| pin.path == path)
            .unwrap()
            .sha256 = format!("{:x}", Sha256::digest(normalized.as_bytes()));
        assert!(
            export_owned_intrinsic_attack(
                &changed_misc,
                &changed_data,
                &source,
                Default::default()
            )
            .unwrap_err()
            .to_string()
            .contains("source bytes differ")
        );
        assert!(
            export_owned_intrinsic_attack(
                &changed_misc,
                &changed_data,
                &repinned,
                Default::default()
            )
            .unwrap_err()
            .to_string()
            .contains("source pin differs")
        );
    }
}

#[test]
fn malformed_missing_and_unreviewed_source_identities_fail_closed() {
    let (misc, data, source) = inputs();
    for bytes in [
        vec![0xff],
        vec![],
        b"return {".to_vec(),
        b"\x1bLua".to_vec(),
    ] {
        assert!(export_owned_intrinsic_attack(&bytes, &data, &source, Default::default()).is_err());
        assert!(export_owned_intrinsic_attack(&misc, &bytes, &source, Default::default()).is_err());
    }
    for changed in 0..6 {
        let mut pin = source.clone();
        match changed {
            0 => pin.revision = "unreviewed".into(),
            1 => pin.system = ExternalSourceSystem::PathOfBuilding1,
            2 => {
                pin.files.pop();
            }
            3 => pin.files.push(pin.files[0].clone()),
            4 => pin.files[1] = pin.files[0].clone(),
            5 => pin.files[0].sha256 = "0".repeat(64),
            _ => unreachable!(),
        }
        assert!(
            export_owned_intrinsic_attack(&misc, &data, &pin, Default::default()).is_err(),
            "accepted identity variant {changed}"
        );
    }
}

#[test]
fn resource_limits_are_enforced_before_or_during_complete_export() {
    let (misc, data, source) = inputs();
    let default = IntrinsicAttackExportLimits::default();
    for limits in [
        IntrinsicAttackExportLimits {
            max_source_bytes: 1,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_block_bytes: 1,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_lua_bytes: 1,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_instructions: 1,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_classes: 8,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_fields: 4,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_text_bytes: 1,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_source_bytes: 0,
            ..default
        },
        IntrinsicAttackExportLimits {
            max_source_bytes: usize::MAX,
            ..default
        },
    ] {
        assert!(
            export_owned_intrinsic_attack(&misc, &data, &source, limits).is_err(),
            "accepted limits {limits:?}"
        );
    }
}
