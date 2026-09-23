//! Prepared lines retain socketed identities and never claim evaluated effects.
use poe_optimizer_core::{build_identity::*, owned_definitions::*};
use poe_optimizer_import::{owned_augment_reconstruction::*, owned_augments::*, owned_mapping::*};
use std::{fs, path::Path};
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("poe2", "test-augments").unwrap()
}
fn template(i: usize) -> ItemTemplateDefId {
    ItemTemplateDefId::new(ns(), key(&format!("augment.{i}")))
}
fn instance(i: u64) -> InstanceId {
    InstanceId::from_parts(BuildLineage::from_bytes([1; 16]), i).unwrap()
}
fn line(text: &str, order: f64) -> AugmentLine {
    AugmentLine {
        text: text.into(),
        stat_order: Some(order),
    }
}
fn acquired(c: AugmentCatalog) -> AcquiredAugmentCatalog {
    decode_owned_augments(
        &encode_owned_augments(&c, Default::default()).unwrap(),
        Default::default(),
    )
    .unwrap()
}
fn synthetic(rows: &[(&str, &str, f64)]) -> AcquiredAugmentCatalog {
    acquired(AugmentCatalog {
        schema_version: 1,
        source: SourcePin {
            system: ExternalSourceSystem::PathOfBuilding2,
            revision: "synthetic".into(),
            files: vec![SourceFilePin {
                path: "data.lua".into(),
                sha256: "0".repeat(64),
            }],
        },
        augments: rows
            .iter()
            .map(|(name, text, order)| AugmentDefinition {
                source_name: (*name).into(),
                selectors: vec![AugmentSelector {
                    source_selector: "weapon".into(),
                    kind: AugmentKind::Rune,
                    normal: vec![line(text, *order)],
                    bonded: None,
                    metadata: Default::default(),
                    semantics: AugmentSemantics::Unconverted,
                }],
            })
            .collect(),
    })
}
fn policy(c: &AcquiredAugmentCatalog) -> AugmentReconstructionPolicy {
    AugmentReconstructionPolicy {
        schema_version: 1,
        version: key("reviewed-update-runes"),
        catalog_sha256: c.sha256().into(),
        dialect: AugmentReconstructionDialect::PobUpdateRunesV1,
        missing_stat_order: 0.0,
        bonded_display_prefix: "Bonded: ".into(),
        bindings: c
            .catalog()
            .augments
            .iter()
            .enumerate()
            .map(|(i, a)| AugmentTemplateBinding {
                source_name: a.source_name.clone(),
                template: template(i),
            })
            .collect(),
    }
}
fn request(p: &AugmentReconstructionPolicy, names: &[&str]) -> AugmentReconstructionRequest {
    let host = AugmentHost {
        item: ItemRecordId::from_instance_id(instance(1)),
        equipment_use: ItemSlotUseId::from_instance_id(instance(2)),
        template: template(9999),
    };
    AugmentReconstructionRequest {
        host: host.clone(),
        active_socket_count: AugmentFact::Known(names.len() as u32),
        categories: AugmentFact::Known(AugmentCategories {
            broad: Some("weapon".into()),
            specific: "spear".into(),
            extra_soul_core_selectors: vec![],
        }),
        selections: names
            .iter()
            .enumerate()
            .map(|(i, name)| AugmentSocketSelection::Occupied {
                slot: SocketSlotDefId::new(ns(), key(&format!("socket.{i}"))),
                source_name: (*name).into(),
                occurrence: SocketedAugmentOccurrence {
                    item: ItemRecordId::from_instance_id(instance(3 + i as u64 * 2)),
                    equipment_use: ItemSlotUseId::from_instance_id(instance(4 + i as u64 * 2)),
                    template: p
                        .bindings
                        .iter()
                        .find(|r| r.source_name == *name)
                        .map(|r| r.template.clone())
                        .unwrap_or_else(|| template(9998)),
                    container: host.equipment_use,
                },
            })
            .collect(),
        activation: AugmentActivationContext {
            normal_enabled: AugmentFact::Known(true),
            global_bonded_enabled: AugmentFact::Unresolved {
                code: key("global-bonded-unreviewed"),
            },
            item_idols_bonded_enabled: AugmentFact::Known(false),
        },
        magnitude: AugmentMagnitudeContext {
            global_increase_percent: AugmentFact::Known(0.0),
            rune_increase_percent: AugmentFact::Unresolved {
                code: key("rune-effect-unreviewed"),
            },
            soul_core_increase_percent: AugmentFact::Known(0.0),
            source_magnitude_already_applied: AugmentFact::Known(false),
        },
    }
}
fn reconstruct(
    c: &AcquiredAugmentCatalog,
    p: &AugmentReconstructionPolicy,
    r: &AugmentReconstructionRequest,
) -> PreparedAugmentReport {
    reconstruct_owned_augments(c, p, r, Default::default()).unwrap()
}
fn tracked() -> AcquiredAugmentCatalog {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    decode_owned_augments(
        &fs::read(root.join("data/owned/poe2/3887ae68/augments/catalog.json")).unwrap(),
        Default::default(),
    )
    .unwrap()
}
#[test]
fn original_weapon_selections_reconstruct_without_flattening_socketed_occurrences() {
    let c = tracked();
    let p = policy(&c);
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    for (file, item_id, expected, count) in [
        ("build-02.xml", "26", "18% increased Physical Damage", 1),
        ("build-03.xml", "17", "36% increased Physical Damage", 2),
    ] {
        let xml = fs::read_to_string(
            root.join("tests/fixtures/builds/breadth-20260908")
                .join(file),
        )
        .unwrap();
        let doc = roxmltree::Document::parse(&xml).unwrap();
        let item = doc
            .descendants()
            .find(|n| n.has_tag_name("Item") && n.attribute("id") == Some(item_id))
            .unwrap();
        let text = item.text().unwrap();
        let names: Vec<_> = text
            .lines()
            .filter_map(|line| line.trim().strip_prefix("Rune: "))
            .collect();
        let r = request(&p, &names);
        let result = reconstruct(&c, &p, &r);
        assert!(result.issues.is_empty(), "{file}: {:?}", result.issues);
        assert_eq!(names.len(), count);
        assert_eq!(result.lines[0].text, expected);
        assert_eq!(result.lines[0].members.len(), count);
        assert_eq!(result.selections, r.selections);
        assert_eq!(result.host, r.host);
        for (i, member) in result.lines[0].members.iter().enumerate() {
            assert_eq!(member.selection_index, i);
            assert_eq!(member.occurrence.container, r.host.equipment_use);
            assert_eq!(member.source_text, "18% increased Physical Damage");
        }
        assert_eq!(
            result.lines[1].display_text,
            format!(
                "Bonded: {}% increased effect of Fully Broken Armour",
                20 * count
            )
        );
        assert_eq!(result.activation, r.activation);
        assert_eq!(result.magnitude, r.magnitude);
        assert_eq!(
            result.context_application,
            AugmentContextApplication::Unapplied
        );
        assert_eq!(result.semantics, AugmentSemantics::Unconverted);
    }
}
#[test]
fn every_tracked_selector_prepares_from_explicit_context_without_name_dispatch() {
    let c = tracked();
    let p = policy(&c);
    let mut count = 0;
    for a in &c.catalog().augments {
        for row in &a.selectors {
            let mut r = request(&p, &[&a.source_name]);
            r.categories = AugmentFact::Known(AugmentCategories {
                broad: None,
                specific: row.source_selector.clone(),
                extra_soul_core_selectors: vec![],
            });
            let result = reconstruct(&c, &p, &r);
            assert!(
                result.issues.is_empty(),
                "{} / {}: {:?}",
                a.source_name,
                row.source_selector,
                result.issues
            );
            assert_eq!(
                result.lines.iter().map(|l| l.members.len()).sum::<usize>(),
                row.normal.len() + row.bonded.as_ref().map_or(0, Vec::len)
            );
            assert!(result.lines.iter().all(|l| l.kind == row.kind));
            count += 1;
        }
    }
    assert_eq!(count, 594);
}
#[test]
fn first_text_controls_mixed_skeleton_and_signed_numeric_merges() {
    for (first, second, expected) in [
        (
            "20% increased Damage",
            "5% reduced Damage",
            "25% increased Damage",
        ),
        ("-20% Resistance", "+8% Resistance", "-28% Resistance"),
    ] {
        let c = synthetic(&[("A", first, 1.0), ("B", second, 1.0)]);
        let p = policy(&c);
        let r = request(&p, &["A", "B"]);
        let result = reconstruct(&c, &p, &r);
        assert!(result.issues.is_empty());
        assert_eq!(result.lines[0].text, expected);
        assert_eq!(
            result.lines[0].diagnostics,
            vec![AugmentMergeDiagnostic::DifferentIncomingSkeleton { member_index: 1 }]
        );
        assert_eq!(result.lines[0].members[1].source_text, second);
    }
}
#[test]
fn source_ignores_extra_incoming_numbers_but_missing_matches_are_unavailable() {
    let c = synthetic(&[
        ("A", "2 damage", 1.0),
        ("B", "3 to 4 damage", 1.0),
        ("C", "No numbers", 1.0),
    ]);
    let p = policy(&c);
    let result = reconstruct(&c, &p, &request(&p, &["A", "B"]));
    assert_eq!(result.lines[0].text, "5 damage");
    assert!(result.lines[0].diagnostics.contains(
        &AugmentMergeDiagnostic::IgnoredIncomingNumbers {
            member_index: 1,
            count: 1
        }
    ));
    let missing = reconstruct(&c, &p, &request(&p, &["B", "A"]));
    assert!(missing.lines.is_empty());
    assert!(matches!(
        missing.issues[0],
        AugmentReconstructionIssue::MissingNumericMatch { .. }
    ));
    let numberless = reconstruct(&c, &p, &request(&p, &["C", "B"]));
    assert_eq!(numberless.lines[0].text, "No numbers");
    assert!(numberless.lines[0].diagnostics.contains(
        &AugmentMergeDiagnostic::IgnoredIncomingNumbers {
            member_index: 1,
            count: 2
        }
    ));
}
#[test]
fn each_merge_reparses_source_formatted_text_and_order_keys_use_source_formatting() {
    let c = synthetic(&[
        ("A", "1.2345678901234 value", 1.0),
        ("B", "0.00000000000006 value", 1.0),
    ]);
    let p = policy(&c);
    let result = reconstruct(&c, &p, &request(&p, &["A", "B", "B"]));
    assert_eq!(result.lines[0].text, "1.2345678901236 value");
    let c = synthetic(&[
        ("A", "2 value", 100000000000000.0),
        ("B", "3 value", 100000000000001.0),
    ]);
    let p = policy(&c);
    let result = reconstruct(&c, &p, &request(&p, &["A", "B"]));
    assert_eq!(result.lines.len(), 1);
    assert_eq!(result.lines[0].text, "5 value");
    assert_eq!(result.lines[0].order_key, "1e+14");
    assert_eq!(result.lines[0].first_stat_order, 100000000000000.0);
}
#[test]
fn lanes_families_and_equal_order_encounters_remain_distinct_and_stable() {
    let c = synthetic(&[("A", "2 value", 0.0), ("B", "3 value", -0.0)]);
    let mut input = c.catalog().clone();
    input.augments[1].selectors[0].kind = AugmentKind::Idol;
    input.augments[0].selectors[0].bonded = Some(vec![line("4 value", 0.0)]);
    let c = acquired(input);
    let p = policy(&c);
    let result = reconstruct(&c, &p, &request(&p, &["A", "B"]));
    assert_eq!(
        result
            .lines
            .iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>(),
        vec!["2 value", "4 value", "3 value"]
    );
    assert_eq!(result.lines[1].lane, AugmentLane::Bonded);
    assert_eq!(result.lines[2].kind, AugmentKind::Idol);
}
#[test]
fn broad_specific_and_explicit_soul_core_order_preserve_duplicate_contributions() {
    let c = synthetic(&[("A", "2 value", 1.0)]);
    let mut input = c.catalog().clone();
    let row = &mut input.augments[0].selectors[0];
    row.kind = AugmentKind::SoulCore;
    let mut specific = row.clone();
    specific.source_selector = "spear".into();
    specific.normal[0].text = "3 value".into();
    input.augments[0].selectors.push(specific);
    let c = acquired(input);
    let p = policy(&c);
    let mut r = request(&p, &["A"]);
    let AugmentFact::Known(categories) = &mut r.categories else {
        unreachable!()
    };
    categories.extra_soul_core_selectors = vec!["weapon".into()];
    let result = reconstruct(&c, &p, &r);
    assert_eq!(result.lines[0].text, "7 value");
    assert_eq!(
        result.lines[0]
            .members
            .iter()
            .map(|m| m.source_selector.as_str())
            .collect::<Vec<_>>(),
        vec!["weapon", "spear", "weapon"]
    );
    r.categories = AugmentFact::Unresolved {
        code: key("extra-order-unproved"),
    };
    assert!(reconstruct(&c, &p, &r).lines.is_empty());
}
#[test]
fn unknown_surplus_and_missing_selection_inputs_never_become_ignored_or_empty() {
    let c = synthetic(&[("A", "2 value", 1.0)]);
    let p = policy(&c);
    let mut r = request(&p, &["A", "unknown"]);
    r.active_socket_count = AugmentFact::Known(1);
    let result = reconstruct(&c, &p, &r);
    assert!(result.lines.is_empty());
    assert!(
        result
            .issues
            .iter()
            .any(|i| matches!(i, AugmentReconstructionIssue::SocketMembership { .. }))
    );
    assert!(result.issues.iter().any(|i| matches!(
        i,
        AugmentReconstructionIssue::UnknownAugment { selection_index: 1 }
    )));
    let mut r = request(&p, &["A"]);
    r.active_socket_count = AugmentFact::Known(2);
    assert!(reconstruct(&c, &p, &r).lines.is_empty());
    r.active_socket_count = AugmentFact::Unresolved {
        code: key("count-unproved"),
    };
    assert!(reconstruct(&c, &p, &r).lines.is_empty());
    let mut r = request(&p, &["A"]);
    r.selections[0] = AugmentSocketSelection::Empty {
        slot: SocketSlotDefId::new(ns(), key("socket.0")),
    };
    let empty = reconstruct(&c, &p, &r);
    assert!(empty.issues.is_empty());
    assert!(empty.lines.is_empty());
}
#[test]
fn ancestry_duplicate_occurrences_and_wrong_template_bindings_are_rejected() {
    let c = synthetic(&[("A", "2 value", 1.0)]);
    let p = policy(&c);
    for case in 0..4 {
        let mut r = request(&p, &["A", "A"]);
        let AugmentSocketSelection::Occupied { occurrence, .. } = &mut r.selections[1] else {
            unreachable!()
        };
        match case {
            0 => occurrence.container = ItemSlotUseId::from_instance_id(instance(999)),
            1 => occurrence.item = ItemRecordId::from_instance_id(instance(3)),
            2 => {
                occurrence.equipment_use = ItemSlotUseId::from_instance_id(
                    InstanceId::from_parts(BuildLineage::from_bytes([2; 16]), 8).unwrap(),
                )
            }
            _ => occurrence.equipment_use = ItemSlotUseId::from_instance_id(instance(5)),
        }
        assert!(
            reconstruct_owned_augments(&c, &p, &r, Default::default()).is_err(),
            "case {case}"
        );
    }
    let mut r = request(&p, &["A"]);
    let AugmentSocketSelection::Occupied { occurrence, .. } = &mut r.selections[0] else {
        unreachable!()
    };
    occurrence.template = template(345);
    assert!(matches!(
        reconstruct(&c, &p, &r).issues[0],
        AugmentReconstructionIssue::MissingTemplateBinding { .. }
    ));
}
#[test]
fn activation_and_magnitudes_are_carried_unapplied_and_never_invented() {
    let c = synthetic(&[("A", "2 value", 1.0)]);
    let p = policy(&c);
    let mut r = request(&p, &["A"]);
    let first = reconstruct(&c, &p, &r);
    r.activation.normal_enabled = AugmentFact::Known(false);
    r.activation.global_bonded_enabled = AugmentFact::Known(true);
    r.magnitude.global_increase_percent = AugmentFact::Known(250.0);
    r.magnitude.source_magnitude_already_applied = AugmentFact::Known(true);
    let changed = reconstruct(&c, &p, &r);
    assert_eq!(first.lines, changed.lines);
    assert_ne!(first.request, changed.request);
    assert_eq!(changed.activation, r.activation);
    assert_eq!(changed.magnitude, r.magnitude);
    r.magnitude.global_increase_percent = AugmentFact::Known(f64::NAN);
    assert!(reconstruct_owned_augments(&c, &p, &r, Default::default()).is_err());
}
#[test]
fn policy_binding_missing_order_and_all_output_work_limits_are_explicit() {
    let c = synthetic(&[("A", "2 value", 1.0)]);
    let mut input = c.catalog().clone();
    input.augments[0].selectors[0].normal[0].stat_order = None;
    let c = acquired(input);
    let mut p = policy(&c);
    p.missing_stat_order = 12.5;
    let r = request(&p, &["A"]);
    assert_eq!(reconstruct(&c, &p, &r).lines[0].first_stat_order, 12.5);
    for limit in [
        AugmentReconstructionLimits {
            max_work: 1,
            ..Default::default()
        },
        AugmentReconstructionLimits {
            max_output_bytes: 1,
            ..Default::default()
        },
        AugmentReconstructionLimits {
            max_wire_bytes: 1,
            ..Default::default()
        },
        AugmentReconstructionLimits {
            max_text_bytes: 1,
            ..Default::default()
        },
        AugmentReconstructionLimits {
            max_work: 0,
            ..Default::default()
        },
    ] {
        assert!(reconstruct_owned_augments(&c, &p, &r, limit).is_err());
    }
    let mut bad = p.clone();
    bad.catalog_sha256 = "0".repeat(64);
    assert!(reconstruct_owned_augments(&c, &bad, &r, Default::default()).is_err());
    bad = p.clone();
    bad.bindings.push(bad.bindings[0].clone());
    assert!(reconstruct_owned_augments(&c, &bad, &r, Default::default()).is_err());
    bad = p;
    bad.missing_stat_order = f64::INFINITY;
    assert!(reconstruct_owned_augments(&c, &bad, &r, Default::default()).is_err());
}

#[test]
fn multi_digit_decimals_follow_the_exact_source_digit_optional_dot_digit_star_pattern() {
    for (first, second, count, expected) in [
        ("12.5 value", "12.5 value", 1, "24.10 value"),
        ("12.5 value", "12.5 value", 2, "36.15 value"),
        ("123.45 value", "123.45 value", 2, "369.135 value"),
        ("-12.5 value", "+12.5 value", 2, "-36.15 value"),
        ("1.25 value", "12.5 value", 1, "13.25 value"),
        ("1.25 value", "12.5 value", 2, "25.30 value"),
    ] {
        let c = synthetic(&[("A", first, 1.0), ("B", second, 1.0)]);
        let p = policy(&c);
        let names = if count == 1 {
            vec!["A", "B"]
        } else {
            vec!["A", "B", "B"]
        };
        let result = reconstruct(&c, &p, &request(&p, &names));
        assert!(result.issues.is_empty());
        assert_eq!(result.lines[0].text, expected, "{names:?}");
    }
}

#[test]
fn retained_identity_group_and_diagnostic_expansion_hits_output_bound_before_later_text() {
    // The final overlong source line is a witness: a late serialization-only
    // cap reaches that line and reports text bytes instead of output bytes.
    for distinct_groups in [false, true] {
        let c = synthetic(&[("A", "1 value", 1.0)]);
        let mut raw = c.catalog().clone();
        raw.augments[0].selectors[0].normal = (0..100)
            .map(|i| {
                line(
                    if i % 2 == 0 { "1 value" } else { "1 altered" },
                    if distinct_groups { i as f64 } else { 1.0 },
                )
            })
            .collect();
        let valid = acquired(raw.clone());
        let p = policy(&valid);
        let result = reconstruct(&valid, &p, &request(&p, &["A"]));
        assert_eq!(
            result.lines.iter().map(|l| l.members.len()).sum::<usize>(),
            100
        );
        raw.augments[0].selectors[0]
            .normal
            .push(line(&"x".repeat(65), 999.0));
        let c = acquired(raw);
        let p = policy(&c);
        let limits = AugmentReconstructionLimits {
            max_text_bytes: 64,
            max_output_bytes: 16 * 1024,
            ..Default::default()
        };
        assert!(matches!(
            reconstruct_owned_augments(&c, &p, &request(&p, &["A"]), limits),
            Err(AugmentReconstructionError::Limit("output bytes"))
        ));
    }
}
