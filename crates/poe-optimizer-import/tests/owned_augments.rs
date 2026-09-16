//! Offline acquisition tests: bounded provenance never claims evaluated effects.
use poe_optimizer_import::{owned_augments::*, owned_mapping::*};
use sha2::{Digest, Sha256};

fn catalog() -> AugmentCatalog {
    AugmentCatalog {
        schema_version: 1,
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "test-source".into(),
            files: vec![SourceFilePin {
                path: "src/Data/Finite.lua".into(),
                sha256: "0".repeat(64),
            }],
        },
        augments: vec![AugmentDefinition {
            source_name: "Example".into(),
            selectors: vec![AugmentSelector {
                source_selector: "weapon".into(),
                kind: AugmentKind::Rune,
                normal: vec![
                    AugmentLine {
                        text: "2 damage".into(),
                        stat_order: Some(4.1),
                    },
                    AugmentLine {
                        text: "second".into(),
                        stat_order: Some(4.1),
                    },
                ],
                bonded: Some(vec![AugmentLine {
                    text: "Bonded effect".into(),
                    stat_order: None,
                }]),
                metadata: AugmentMetadata {
                    local_mod: Some(false),
                    limit: Some(1),
                    trade_hashes: Some(vec![AugmentTradeHash {
                        hash: 42,
                        lines: vec!["".into()],
                    }]),
                    ..Default::default()
                },
                semantics: AugmentSemantics::Unconverted,
            }],
        }],
    }
}
#[test]
fn acquisition_retains_exact_bytes_without_authenticating_carried_source_claims() {
    let input = catalog();
    let canonical = encode_owned_augments(&input, Default::default()).unwrap();
    let acquired = decode_owned_augments(&canonical, Default::default()).unwrap();
    assert_eq!(acquired.catalog(), &input);
    assert_eq!(acquired.bytes(), canonical);
    assert_eq!(
        acquired.sha256(),
        format!("{:x}", Sha256::digest(&canonical))
    );
    assert_eq!(
        acquired.counts(),
        AugmentCatalogCounts {
            augments: 1,
            selectors: 1,
            normal_lines: 2,
            bonded_lines: 1,
            trade_hashes: 1,
            trade_lines: 1
        }
    );
    // The synthetic source hash is accepted as provenance, never authority.
    let compact = serde_json::to_vec(&input).unwrap();
    let different = decode_owned_augments(&compact, Default::default()).unwrap();
    assert_eq!(different.catalog(), acquired.catalog());
    assert_ne!(different.sha256(), acquired.sha256());
    assert_eq!(different.bytes(), compact);
}
#[test]
fn canonical_lookup_order_preserves_meaningful_line_order_and_absence() {
    let mut input = catalog();
    let mut other = input.augments[0].clone();
    other.source_name = "Earlier".into();
    input.augments.push(other);
    let mut row = input.augments[0].selectors[0].clone();
    row.source_selector = "armour".into();
    row.normal.clear();
    row.bonded = Some(vec![]);
    input.augments[0].selectors.push(row);
    let first = encode_owned_augments(&input, Default::default()).unwrap();
    input.augments.reverse();
    for a in &mut input.augments {
        a.selectors.reverse();
    }
    assert_eq!(
        encode_owned_augments(&input, Default::default()).unwrap(),
        first
    );
    let acquired = decode_owned_augments(&first, Default::default()).unwrap();
    let weapon = acquired
        .catalog()
        .augments
        .iter()
        .find(|a| a.source_name == "Example")
        .unwrap()
        .selectors
        .iter()
        .find(|a| a.source_selector == "weapon")
        .unwrap();
    assert_eq!(weapon.normal[0].text, "2 damage");
    assert_eq!(weapon.normal[1].text, "second");
    assert_eq!(weapon.metadata.local_mod, Some(false));
    assert_eq!(weapon.metadata.socket_bound, None);
    let mut changed = acquired.catalog().clone();
    changed.augments[0].selectors[0].normal.reverse();
    assert_ne!(
        encode_owned_augments(&changed, Default::default()).unwrap(),
        first
    );
}
#[test]
fn duplicate_lookup_identities_and_invalid_source_pins_reject() {
    for case in 0..8 {
        let mut v = catalog();
        match case {
            0 => v.augments.push(v.augments[0].clone()),
            1 => {
                let r = v.augments[0].selectors[0].clone();
                v.augments[0].selectors.push(r);
            }
            2 => {
                let h = v.augments[0].selectors[0]
                    .metadata
                    .trade_hashes
                    .as_mut()
                    .unwrap();
                h.push(h[0].clone());
            }
            3 => v.source.files.push(v.source.files[0].clone()),
            4 => v.source.files[0].path = "../outside.lua".into(),
            5 => v.source.files[0].sha256 = "not-a-hash".into(),
            6 => v.augments[0].selectors.clear(),
            _ => v.augments[0].source_name = "bad\0name".into(),
        }
        assert!(
            validate_owned_augments(&v, Default::default()).is_err(),
            "case {case}"
        );
    }
}
#[test]
fn strict_wire_rejects_unknown_duplicate_and_missing_fields() {
    let original = serde_json::to_value(catalog()).unwrap();
    for case in 0..7 {
        let mut v = original.clone();
        match case {
            0 => v["schema_version"] = 2.into(),
            1 => v["extra"] = true.into(),
            2 => v["augments"][0]["selectors"][0]["metadata"]["extra"] = true.into(),
            3 => v["augments"][0]["selectors"][0]["kind"] = "future".into(),
            4 => v["augments"][0]["selectors"][0]["semantics"] = "complete".into(),
            5 => {
                v["augments"][0]["selectors"][0]["metadata"]
                    .as_object_mut()
                    .unwrap()
                    .remove("socket_bound");
            }
            _ => {
                v["augments"][0]["selectors"][0]
                    .as_object_mut()
                    .unwrap()
                    .remove("bonded");
            }
        }
        assert!(
            decode_owned_augments(&serde_json::to_vec(&v).unwrap(), Default::default()).is_err(),
            "case {case}"
        );
    }
    let wire = serde_json::to_string(&catalog()).unwrap().replacen(
        "\"schema_version\":1",
        "\"schema_version\":1,\"schema_version\":1",
        1,
    );
    assert!(decode_owned_augments(wire.as_bytes(), Default::default()).is_err());
}
#[test]
fn finite_fractional_orders_and_signed_zero_are_preserved() {
    let mut c = catalog();
    let encoded = encode_owned_augments(&c, Default::default()).unwrap();
    assert_eq!(
        decode_owned_augments(&encoded, Default::default())
            .unwrap()
            .catalog()
            .augments[0]
            .selectors[0]
            .normal[0]
            .stat_order,
        Some(4.1)
    );
    c.augments[0].selectors[0].normal[0].stat_order = Some(-0.0);
    let negative = encode_owned_augments(&c, Default::default()).unwrap();
    c.augments[0].selectors[0].normal[0].stat_order = Some(0.0);
    assert_ne!(
        negative,
        encode_owned_augments(&c, Default::default()).unwrap()
    );
    for n in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        c.augments[0].selectors[0].normal[0].stat_order = Some(n);
        assert!(encode_owned_augments(&c, Default::default()).is_err());
    }
}
#[test]
fn acquisition_serialization_and_aggregate_projection_are_bounded() {
    let mut c = catalog();
    let bytes = encode_owned_augments(&c, Default::default()).unwrap();
    assert!(
        decode_owned_augments(
            &bytes,
            AugmentCatalogLimits {
                max_catalog_bytes: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    for limits in [
        AugmentCatalogLimits {
            max_catalog_bytes: 1,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_lines: 1,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_total_text_bytes: 1,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_work: 1,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_work: 0,
            ..Default::default()
        },
        AugmentCatalogLimits {
            max_work: AugmentCatalogLimits::default().max_work + 1,
            ..Default::default()
        },
    ] {
        assert!(encode_owned_augments(&c, limits).is_err());
    }
    c.augments[0].selectors[0]
        .metadata
        .trade_hashes
        .as_mut()
        .unwrap()[0]
        .hash = 9_007_199_254_740_992;
    assert!(validate_owned_augments(&c, Default::default()).is_err());
}
