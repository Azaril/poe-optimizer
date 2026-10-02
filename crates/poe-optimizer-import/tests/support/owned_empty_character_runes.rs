//! Synthetic controls exercise injected empty tokens, source ambiguity and spent IDs.
use super::*;
use poe_optimizer_core::owned_content::digest_owned;

fn reviewed() -> Fixture {
    let mut fixture = fixture();
    let Some(EquipmentMembershipPolicy::PobOrdinaryItemSetsV1 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
    }) = fixture.policy.equipment_membership.take()
    else {
        panic!()
    };
    fixture.policy.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines: *fixture.items.identity(),
            item_source: *fixture.item_source.identity(),
            imported_profiles: vec![],
            empty_character_runes: EmptyCharacterRuneSelections {
                mapping_source: *fixture.artifacts.mapping.source_identity(),
                slot_names: vec!["alpha-rune".into(), "beta-rune".into()],
                explicit_empty_selection: "Vacant".into(),
            },
        },
    );
    fixture
}
fn empty_policy(policy: &mut NormalizationPolicy) -> &mut EmptyCharacterRuneSelections {
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        empty_character_runes,
        ..
    }) = &mut policy.equipment_membership
    else {
        panic!()
    };
    empty_character_runes
}
fn prior_policy(fixture: &Fixture) -> NormalizationPolicy {
    let mut policy = fixture.policy.clone();
    let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
        definitions,
        templates,
        source_base_names,
        loader_jewel_fallback_titles,
        item_lines,
        item_source,
        imported_profiles,
        ..
    }) = policy.equipment_membership.take()
    else {
        panic!()
    };
    policy.equipment_membership = Some(
        EquipmentMembershipPolicy::PobOrdinaryAndImportedItemSetsV2 {
            definitions,
            templates,
            source_base_names,
            loader_jewel_fallback_titles,
            item_lines,
            item_source,
            imported_profiles,
        },
    );
    policy
}
fn evaluate(
    xml: &str,
    fixture: &Fixture,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> std::result::Result<NormalizedImport, NormalizationError> {
    let source = source(xml, 93);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &fixture.items,
            item_source: &fixture.item_source,
            mappings: &fixture.artifacts.mapping,
            registry: &fixture.artifacts.registry,
            definitions: &fixture.artifacts.schema,
            roles: &fixture.artifacts.roles,
            rewards: &fixture.artifacts.rewards,
        },
        policy,
        &queries(),
        limits,
    )?;
    let retired = result
        .draft()
        .input()
        .equipment_presets
        .members
        .iter()
        .filter(|preset| matches!(preset.equipment.completion, DraftListCompletion::Complete))
        .count()
        + absent(&result).len() * 4;
    origin_integrity_with_retired(&source, &result, retired);
    Ok(result)
}
fn absent(result: &NormalizedImport) -> Vec<&SourceOwnedOrigin> {
    result.sidecar().origins.iter().filter(|origin| matches!(&origin.disposition, SourceDisposition::SourceOnly(code) if code.as_str() == "explicit-empty-character-rune-selection")).collect()
}
const EMPTY: &str = r#"<RuneSlot slotName="alpha-rune" runeName="Vacant"/>"#;

#[test]
fn empty_character_runes_retire_exact_records_and_preserve_all_other_facts_and_ids() {
    let fixture = reviewed();
    let input = xml(
        RAW,
        &format!(
            r#"{SLOT}{EMPTY}<RuneSlot slotName="beta-rune" runeName="Vacant"/><SocketIdURL nodeId="123" itemPbURL=""/>"#
        ),
    );
    let old_policy = prior_policy(&fixture);
    let old = evaluate(&input, &fixture, &old_policy, Default::default()).unwrap();
    let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
    assert_eq!(absent(&result).len(), 2);
    assert_eq!(result.allocator_after(), old.allocator_after());
    assert_eq!(
        result.draft().input().equipment.members.len() + 2,
        old.draft().input().equipment.members.len()
    );
    let mut expected = old.draft().input().clone();
    let mut origins = old.sidecar().origins.clone();
    for removed in absent(&result) {
        let prior = &mut origins[removed.source.ordinal() as usize];
        let [
            OwnedOriginTarget::Equipment(id),
            OwnedOriginTarget::Issue(_),
            OwnedOriginTarget::Issue(_),
            OwnedOriginTarget::Issue(_),
        ] = prior.links.as_slice()
        else {
            panic!("exact fabricated use")
        };
        let id = *id;
        expected.equipment.members.retain(|record| record.id != id);
        expected.equipment_presets.members[0]
            .equipment
            .members
            .retain(|use_id| *use_id != id);
        *prior = (*removed).clone();
    }
    assert_eq!(
        serde_json::to_value(expected).unwrap(),
        serde_json::to_value(result.draft().input()).unwrap()
    );
    assert_eq!(origins, result.sidecar().origins);
    assert!(
        !complete(&result, 0),
        "empty runes do not close the ItemSet inventory"
    );
    assert_eq!(
        old.sidecar().schema_version,
        result.sidecar().schema_version
    );
    assert_eq!(
        serde_json::to_value(&old.sidecar().item_texts).unwrap(),
        serde_json::to_value(&result.sidecar().item_texts).unwrap()
    );
}

#[test]
fn empty_character_runes_keep_occupied_siblings_and_independent_archived_sets() {
    let fixture = reviewed();
    let input = format!(
        r#"<PathOfBuilding2><Items activeItemSet="1"><Item id="1">{RAW}</Item><ItemSet id="1">{EMPTY}<RuneSlot slotName="beta-rune" runeName="Occupied"/></ItemSet><ItemSet id="2"><RuneSlot slotName="alpha-rune" runeName="Vacant"/><RuneSlot slotName="alpha-rune" runeName="Occupied"/></ItemSet></Items></PathOfBuilding2>"#
    );
    let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
    assert_eq!(absent(&result).len(), 1);
    assert_eq!(
        result.draft().input().equipment_presets.members[0]
            .equipment
            .members
            .len(),
        1
    );
    assert_eq!(
        result.draft().input().equipment_presets.members[1]
            .equipment
            .members
            .len(),
        2
    );
    assert_eq!(result.draft().input().equipment.members.len(), 3);
    for use_record in &result.draft().input().equipment.members {
        assert!(matches!(use_record.item, DraftField::Pending(_)));
        assert!(matches!(
            use_record.destination,
            DraftEquipmentDestination::Pending(_)
        ));
        assert!(matches!(use_record.scope, DraftField::Pending(_)));
    }
}

#[test]
fn empty_character_runes_reject_ambiguous_siblings_in_both_orders() {
    let fixture = reviewed();
    for sibling in [
        r#"<RuneSlot slotName="alpha-rune" runeName="Vacant"/>"#,
        r#"<RuneSlot slotName="alpha-rune" runeName="Occupied"/>"#,
        r#"<RuneSlot slotName="unknown-rune" runeName="Vacant"/>"#,
        r#"<RuneSlot slotName="beta-rune"/>"#,
        r#"<RuneSlot slotName="beta-rune" runeName=""/>"#,
        r#"<RuneSlot slotName="beta-rune" runeName=" Vacant"/>"#,
        r#"<RuneSlot slotName="beta-rune" runeName="Vacant" extra="1"/>"#,
        r#"<RuneSlot slotName="beta-rune" runeName="Vacant">text</RuneSlot>"#,
        r#"<RuneSlot slotName="beta-rune" runeName="Vacant"><Child/></RuneSlot>"#,
        r#"<RuneSlot xmlns="urn:other" slotName="beta-rune" runeName="Vacant"/>"#,
        r#"<Slot name="alpha-rune" itemId="0"/>"#,
        r#"<Slot name="beta-rune" itemId="1"/>"#,
        r#"<Slot name="coat" itemId="01"/>"#,
        r#"<Slot name="id" itemId="1"/>"#,
        r#"<Slot name="title" itemId="1"/>"#,
        r#"<Slot name="useSecondWeaponSet" itemId="1"/>"#,
        r#"<Slot name="unknown" itemId="1"/>"#,
        r#"<Unknown/>"#,
    ] {
        for slots in [format!("{EMPTY}{sibling}"), format!("{sibling}{EMPTY}")] {
            let input = xml(RAW, &slots);
            let old = evaluate(
                &input,
                &fixture,
                &prior_policy(&fixture),
                Default::default(),
            )
            .unwrap();
            let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
            assert!(absent(&result).is_empty(), "{slots}");
            assert_eq!(old.draft(), result.draft(), "{slots}");
            assert_eq!(old.sidecar().origins, result.sidecar().origins, "{slots}");
        }
    }
}

#[test]
fn empty_character_runes_require_explicit_injected_names_token_and_fresh_frame() {
    let fixture = reviewed();
    for input in [
        xml(RAW, r#"<RuneSlot slotName="alpha-rune" runeName="None"/>"#),
        xml(RAW, r#"<RuneSlot slotName="alpha-rune"/>"#),
        xml(RAW, r#"<RuneSlot runeName="Vacant"/>"#),
        xml(
            RAW,
            r#"<RuneSlot slotName=" alpha-rune" runeName="Vacant"/>"#,
        ),
        xml(RAW, EMPTY).replace("<ItemSet id=\"1\"", "<ItemSet id=\"01\""),
        xml(RAW, EMPTY).replace("<ItemSet id=\"1\"", "<ItemSet extra=\"x\" id=\"1\""),
        xml(RAW, EMPTY).replace("</Items>", "<ItemSet id=\"1\"/></Items>"),
        xml(RAW, EMPTY).replace("</PathOfBuilding2>", "<Items/></PathOfBuilding2>"),
    ] {
        let old = evaluate(
            &input,
            &fixture,
            &prior_policy(&fixture),
            Default::default(),
        )
        .unwrap();
        let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
        assert!(absent(&result).is_empty(), "{input}");
        assert_eq!(old.draft(), result.draft(), "{input}");
    }
    let foreign_root =
        xml(RAW, EMPTY).replace("<PathOfBuilding2>", "<PathOfBuilding2 xmlns=\"urn:other\">");
    assert!(
        matches!(
            decode_build(foreign_root.as_bytes()),
            Err(poe_optimizer_import::ImportError::WrongRoot { .. })
        ),
        "foreign root is rejected before normalization"
    );
    let mut changed = fixture.policy.clone();
    empty_policy(&mut changed).explicit_empty_selection = "Different".into();
    let result = evaluate(
        &xml(
            RAW,
            r#"<RuneSlot slotName="alpha-rune" runeName="Different"/>"#,
        ),
        &fixture,
        &changed,
        Default::default(),
    )
    .unwrap();
    assert_eq!(absent(&result).len(), 1);
}

#[test]
fn empty_character_runes_check_all_commitments_and_bounds_even_with_empty_domain() {
    let fixture = reviewed();
    let input = xml(RAW, EMPTY);
    for index in 0..4 {
        let mut policy = fixture.policy.clone();
        let Some(EquipmentMembershipPolicy::PobOrdinaryImportedAndEmptyCharacterRunesV3 {
            definitions,
            item_lines,
            item_source,
            empty_character_runes,
            ..
        }) = &mut policy.equipment_membership
        else {
            panic!()
        };
        let stale = digest_owned("stale-empty-character-runes", &index, 128).unwrap();
        empty_character_runes.slot_names.clear();
        match index {
            0 => empty_character_runes.mapping_source = stale,
            1 => *item_lines = stale,
            2 => *item_source = stale,
            _ => {
                definitions.release = "stale".into();
            }
        }
        assert!(
            matches!(
                evaluate(&input, &fixture, &policy, Default::default()),
                Err(NormalizationError::Binding)
            ),
            "commitment {index}"
        );
    }
    for names in [
        vec!["alpha-rune".into(); 2],
        vec![String::new()],
        vec![" space".into()],
        vec!["control\n".into()],
        (0..257).map(|n| format!("slot{n}")).collect(),
    ] {
        let mut policy = fixture.policy.clone();
        empty_policy(&mut policy).slot_names = names;
        assert!(matches!(
            evaluate(&input, &fixture, &policy, Default::default()),
            Err(NormalizationError::Policy(_))
        ));
    }
    for token in ["", " empty", "empty\n", "empty\u{2003}"] {
        let mut policy = fixture.policy.clone();
        empty_policy(&mut policy).explicit_empty_selection = token.into();
        assert!(matches!(
            evaluate(&input, &fixture, &policy, Default::default()),
            Err(NormalizationError::Policy(_))
        ));
    }
    let mut policy = fixture.policy.clone();
    empty_policy(&mut policy).slot_names =
        vec!["x".repeat(NormalizationLimits::default().mapping.max_string_bytes + 1)];
    assert!(matches!(
        evaluate(&input, &fixture, &policy, Default::default()),
        Err(NormalizationError::Limit(_))
    ));
    let mut wire = serde_json::to_value(&fixture.policy).unwrap();
    wire["equipment_membership"]["empty_character_runes"]["unknown"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
    let prior = prior_policy(&fixture);
    for policy in [&fixture.policy, &prior, &super::fixture().policy] {
        let bytes = serde_json::to_vec(policy).unwrap();
        let restored: NormalizationPolicy = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(bytes, serde_json::to_vec(&restored).unwrap());
    }
}

#[test]
fn empty_character_runes_prove_loader_write_safety_across_all_saved_sets() {
    let fixture = reviewed();
    for sibling in [
        r#"<Slot name="id" itemId="1"/>"#,
        r#"<Slot name="title" itemId="1"/>"#,
        r#"<Slot name="useSecondWeaponSet" itemId="1"/>"#,
        r#"<Slot name="unknown" itemId="1"/>"#,
        r#"<SocketIdURL nodeId="not-a-number" itemPbURL=""/>"#,
        r#"<RuneSlot slotName="id" runeName="Vacant"/>"#,
    ] {
        let good = format!(r#"<ItemSet id="1">{EMPTY}</ItemSet>"#);
        let bad = format!(r#"<ItemSet id="2" useSecondWeaponSet="true">{sibling}</ItemSet>"#);
        for sets in [format!("{good}{bad}"), format!("{bad}{good}")] {
            let input = format!(
                r#"<PathOfBuilding2><Items activeItemSet="1"><Item id="1">{RAW}</Item>{sets}</Items></PathOfBuilding2>"#
            );
            let old = evaluate(
                &input,
                &fixture,
                &prior_policy(&fixture),
                Default::default(),
            )
            .unwrap();
            let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
            assert!(absent(&result).is_empty(), "{sets}");
            assert_eq!(old.draft(), result.draft(), "{sets}");
            assert_eq!(old.sidecar().origins, result.sidecar().origins, "{sets}");
        }
    }
    // Loader-safe missing and unknown selections stay local; they do not erase
    // the independently proved explicit empty selection in a different set.
    for sibling in [
        r#"<RuneSlot slotName="beta-rune"/>"#,
        r#"<RuneSlot slotName="beta-rune" runeName="Unknown selection"/>"#,
    ] {
        let input = format!(
            r#"<PathOfBuilding2><Items activeItemSet="1"><Item id="1">{RAW}</Item><ItemSet id="1">{EMPTY}</ItemSet><ItemSet id="2">{sibling}</ItemSet></Items></PathOfBuilding2>"#
        );
        let result = evaluate(&input, &fixture, &fixture.policy, Default::default()).unwrap();
        assert_eq!(absent(&result).len(), 1, "{sibling}");
        assert_eq!(
            result.draft().input().equipment_presets.members[1]
                .equipment
                .members
                .len(),
            1
        );
    }
}
