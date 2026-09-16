//! Independent finite-source snapshots; no modifier parser or build/UI startup.
use poe_optimizer_import::owned_augments::*;
use poe_optimizer_pob::{owned_augments::*, source};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    sync::OnceLock,
};
fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/path-of-building-poe2")
}
fn export() -> &'static AuthenticatedAugmentExport {
    static EXPORT: OnceLock<AuthenticatedAugmentExport> = OnceLock::new();
    EXPORT.get_or_init(|| export_owned_augments(&root(), Default::default()).unwrap())
}
fn row<'a>(catalog: &'a AugmentCatalog, name: &str, selector: &str) -> &'a AugmentSelector {
    catalog
        .augments
        .iter()
        .find(|a| a.source_name == name)
        .unwrap()
        .selectors
        .iter()
        .find(|r| r.source_selector == selector)
        .unwrap()
}
#[test]
fn full_catalog_has_all_five_families_and_explicit_unconverted_coverage() {
    let e = export();
    let c = e.evidence().counts;
    assert_eq!(
        (c.augments, c.selectors, c.normal_lines, c.bonded_lines),
        (287, 594, 629, 559)
    );
    let mut kinds = BTreeMap::new();
    for r in e.catalog().augments.iter().flat_map(|a| &a.selectors) {
        *kinds.entry(r.kind).or_insert(0) += 1;
        assert_eq!(r.semantics, AugmentSemantics::Unconverted);
    }
    assert_eq!(
        kinds,
        BTreeMap::from([
            (AugmentKind::Rune, 435),
            (AugmentKind::SoulCore, 69),
            (AugmentKind::Idol, 77),
            (AugmentKind::AbyssalEye, 12),
            (AugmentKind::CongealedMist, 1)
        ])
    );
    assert_eq!(e.catalog().source.files.len(), 1);
    assert_eq!(e.catalog().source.files[0].path, AUGMENT_SOURCE_PATH);
    assert_eq!(
        e.catalog().source.files[0].sha256,
        source::expected_file_sha256(AUGMENT_SOURCE_PATH).unwrap()
    );
    assert_eq!(
        e.evidence().source_manifest_sha256,
        source::manifest_sha256()
    );
    assert_eq!(
        e.source_bytes(),
        source::read_verified_text(&root(), AUGMENT_SOURCE_PATH)
            .unwrap()
            .as_bytes()
    );
    assert_eq!(
        e.evidence().acquisition_source_sha256,
        e.evidence().normalized_source_sha256
    );
}
#[test]
fn every_line_order_and_metadata_value_matches_independent_finite_snapshot() {
    // Previously committed source extraction is independent evidence. No code
    // from its frozen importer/evaluator is invoked by this comparison.
    let snapshot: Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../poe-optimizer-data/data/game-data.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let originals = snapshot["item_loading"]["modifier_tables"]["Runes"]["fields"]
        .as_object()
        .unwrap();
    let catalog = export().catalog();
    assert_eq!(catalog.augments.len(), originals.len());
    for augment in &catalog.augments {
        let selectors = originals[&augment.source_name]["fields"]
            .as_object()
            .unwrap();
        assert_eq!(augment.selectors.len(), selectors.len());
        for actual in &augment.selectors {
            let original = &selectors[&actual.source_selector];
            let fields = &original["fields"];
            let source_kind = match actual.kind {
                AugmentKind::Rune => "Rune",
                AugmentKind::SoulCore => "SoulCore",
                AugmentKind::Idol => "Idol",
                AugmentKind::AbyssalEye => "AbyssalEye",
                AugmentKind::CongealedMist => "CongealedMist",
            };
            assert_eq!(fields["type"].as_str(), Some(source_kind));
            for (lines, body) in [
                (&actual.normal[..], Some(original)),
                (
                    actual.bonded.as_deref().unwrap_or(&[]),
                    fields.get("bonded"),
                ),
            ] {
                let Some(body) = body else {
                    assert!(lines.is_empty());
                    continue;
                };
                assert_eq!(
                    lines.len(),
                    body["indexed"].as_object().map_or(0, |o| o.len())
                );
                for (i, line) in lines.iter().enumerate() {
                    assert_eq!(
                        Some(line.text.as_str()),
                        body["indexed"][(i + 1).to_string()].as_str()
                    );
                    assert_eq!(line.stat_order, body["fields"]["statOrder"][i].as_f64());
                }
            }
            assert_eq!(actual.bonded.is_some(), fields.get("bonded").is_some());
            for (value, key) in [
                (actual.metadata.local_mod, "localMod"),
                (actual.metadata.socket_bound, "isSocketBound"),
                (
                    actual.metadata.can_socket_in_chakra_slots,
                    "canSocketInChakraSlots",
                ),
                (
                    actual.metadata.can_socket_in_unique_items,
                    "canSocketInUniqueItems",
                ),
                (
                    actual.metadata.can_socket_in_jewellery,
                    "canSocketInJewellery",
                ),
                (
                    actual.metadata.can_socket_in_corrupted_sanctified,
                    "canSocketInCorruptedSanctified",
                ),
            ] {
                assert_eq!(value, fields[key].as_bool(), "{key}");
            }
            assert_eq!(
                actual.metadata.level_requirement.map(f64::from),
                fields["levelReq"].as_f64()
            );
            assert_eq!(
                actual.metadata.limit.map(f64::from),
                fields["limit"].as_f64()
            );
            assert_eq!(
                actual.metadata.limit_id.as_deref(),
                fields["limitId"].as_str()
            );
            assert_eq!(
                actual.metadata.trade_hashes.is_some(),
                fields.get("tradeHashes").is_some()
            );
            if let Some(trades) = &actual.metadata.trade_hashes {
                let expected = fields["tradeHashes"]["indexed"].as_object().unwrap();
                assert_eq!(trades.len(), expected.len());
                for group in trades {
                    assert_eq!(
                        serde_json::to_value(&group.lines).unwrap(),
                        expected[&group.hash.to_string()]
                    );
                }
            }
        }
    }
}
#[test]
fn real_weapon_rows_bonded_only_fractional_and_numberless_cases_are_retained() {
    let c = export().catalog();
    let iron = row(c, "Greater Iron Rune", "weapon");
    assert_eq!(
        iron.normal,
        vec![AugmentLine {
            text: "18% increased Physical Damage".into(),
            stat_order: Some(830.0)
        }]
    );
    assert_eq!(
        iron.bonded,
        Some(vec![AugmentLine {
            text: "20% increased effect of Fully Broken Armour".into(),
            stat_order: Some(5232.0)
        }])
    );
    assert_eq!(iron.metadata.level_requirement, Some(30));
    assert_eq!(iron.metadata.limit, None);
    assert_eq!(
        row(c, "Greater Iron Rune", "staff").normal[0].text,
        "30% increased Spell Damage"
    );
    let silk = row(c, "Idol of Silk", "buckler");
    assert!(silk.normal.is_empty());
    assert_eq!(
        silk.bonded.as_ref().unwrap()[0].text,
        "30% increased Parry Range"
    );
    assert_eq!(
        row(c, "Amanamu's Gaze", "boots").normal[1].stat_order,
        Some(9147.1)
    );
    assert_eq!(
        row(c, "Amanamu's Gaze", "boots")
            .metadata
            .limit_id
            .as_deref(),
        Some("AncientAugment")
    );
    let shard = row(c, "Raven-Touched Shard", "helmet");
    assert_eq!(shard.normal[0].text, "Raven-Touched");
    assert_eq!(shard.bonded, None);
}
#[test]
fn exact_acquisition_repeats_and_candidate_mutations_cannot_impersonate_it() {
    let original = export();
    let repeated = export_owned_augments(&root(), Default::default()).unwrap();
    assert_eq!(repeated.catalog_bytes(), original.catalog_bytes());
    assert_eq!(repeated.evidence(), original.evidence());
    original.validate_catalog(original.catalog()).unwrap();
    for case in 0..5 {
        let mut changed = original.catalog().clone();
        match case {
            0 => changed.augments[0].source_name.push_str(" changed"),
            1 => changed.augments[0].selectors[0].normal[0]
                .text
                .push_str(" changed"),
            2 => changed.augments[0].selectors[0].metadata.limit = Some(999),
            3 => changed.augments[0].selectors[0].bonded = Some(vec![]),
            _ => changed.source.files[0].sha256 = "0".repeat(64),
        }
        assert!(original.validate_catalog(&changed).is_err(), "case {case}");
    }
}
#[test]
fn untrusted_roots_and_tight_limits_cannot_produce_authenticated_catalogs() {
    let temp = tempfile::tempdir().unwrap();
    assert!(export_owned_augments(temp.path(), Default::default()).is_err());
    let default = AugmentExportLimits::default();
    for limit in [
        AugmentExportLimits {
            max_source_bytes: 1,
            ..default
        },
        AugmentExportLimits {
            max_source_entries: 1,
            ..default
        },
        AugmentExportLimits {
            catalog: AugmentCatalogLimits {
                max_augments: 1,
                ..Default::default()
            },
            ..default
        },
        AugmentExportLimits {
            catalog: AugmentCatalogLimits {
                max_lines: 1,
                ..Default::default()
            },
            ..default
        },
        AugmentExportLimits {
            max_lua_bytes: 1,
            ..default
        },
        AugmentExportLimits {
            max_instructions: 100,
            ..default
        },
        AugmentExportLimits {
            max_source_bytes: 0,
            ..default
        },
        AugmentExportLimits {
            max_source_bytes: default.max_source_bytes + 1,
            ..default
        },
    ] {
        assert!(export_owned_augments(&root(), limit).is_err());
    }
}
