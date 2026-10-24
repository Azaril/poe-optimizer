//! Root preferences cannot override independent item-set state or select a build.
use super::*;

fn document(header: &str, first_swap: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="81"/><Items{header}><Item id="1">Unknown item</Item><ItemSet id="9" title="same" useSecondWeaponSet="{first_swap}"><Slot name="Ring 1" itemId="1"/></ItemSet><ItemSet id="2" title="same" useSecondWeaponSet="true"><Slot name="Ring 2" itemId="1"/></ItemSet></Items><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#
    )
}

fn pending(list: &DraftListCompletion, expected: &str) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } = list else {
        panic!("{expected} must remain unresolved")
    };
    assert_eq!(code.as_str(), expected);
    *id
}

fn assert_header_only(
    before_xml: &str,
    after_xml: &str,
    before: &NormalizedImport,
    after: &NormalizedImport,
    selected: usize,
) {
    assert_eq!(after.draft(), before.draft());
    assert_eq!(after.allocator_after(), before.allocator_after());
    let draft = after.draft().input();
    let selection = pending(
        &draft.saved_variants.completion,
        "saved-selection-not-converted",
    );
    assert!(draft.saved_variants.members.is_empty());
    pending(
        &draft.weapon_loadouts.completion,
        "loadout-policy-not-converted",
    );
    pending(
        &draft.choice_presets.members[0].choices.completion,
        "configuration-roles-not-converted",
    );
    let owners = [
        OwnedOriginTarget::EquipmentPreset(draft.equipment_presets.members[selected].id),
        OwnedOriginTarget::Issue(selection),
    ];
    assert_eq!(after.sidecar().origins[2].links, owners);
    assert_eq!(
        after.sidecar().origins[2].disposition,
        SourceDisposition::Contributes
    );
    let before_source = source(before_xml, 0x74);
    let after_source = source(after_xml, 0x74);
    let before_source = SourceProjectEvidence::collect(&before_source, Default::default()).unwrap();
    let after_source = SourceProjectEvidence::collect(&after_source, Default::default()).unwrap();
    assert_eq!(before_source.rows().len(), after_source.rows().len());
    let mut expected = before.sidecar().origins.clone();
    // The caller proves the exact header-only XML substitution. Authenticate the
    // complete occurrence correspondence before rebinding content-addressed IDs.
    for (ordinal, (old, new)) in before_source
        .rows()
        .iter()
        .zip(after_source.rows())
        .enumerate()
    {
        assert_eq!(old.occurrence().name(), new.occurrence().name());
        assert_eq!(old.occurrence().id(), expected[ordinal].source);
        assert_eq!(
            new.occurrence().id(),
            after.sidecar().origins[ordinal].source
        );
        assert_eq!(
            old.occurrence().id().ordinal(),
            new.occurrence().id().ordinal()
        );
        expected[ordinal].source = new.occurrence().id();
    }
    expected[2].links = owners.to_vec();
    assert_eq!(expected, after.sidecar().origins);
}

#[test]
fn exact_keys_and_root_preferences_preserve_every_independent_set_and_pending_owner() {
    let a = artifacts(false);
    for set_swap in ["true", "false", "nil"] {
        let baseline_header = r#" activeItemSet="0""#;
        let baseline_xml = document(baseline_header, set_swap);
        let baseline = run(&source(&baseline_xml, 0x74), &a, &queries());
        let config = pending(
            &baseline.draft().input().choice_presets.members[0]
                .choices
                .completion,
            "configuration-roles-not-converted",
        );
        assert_eq!(
            baseline.sidecar().origins[2].links,
            [OwnedOriginTarget::Issue(config)]
        );
        assert_ne!(
            baseline.draft().input().equipment_presets.members[0].id,
            baseline.draft().input().equipment_presets.members[1].id,
        );
        for (key, selected) in [("9", 0), ("2", 1)] {
            for root_swap in [None, Some("true"), Some("false"), Some("nil")] {
                for differences in [None, Some("true"), Some("false"), Some("nil")] {
                    let mut header = format!(r#" activeItemSet="{key}""#);
                    for (name, value) in [
                        ("useSecondWeaponSet", root_swap),
                        ("showStatDifferences", differences),
                    ] {
                        if let Some(value) = value {
                            header.push_str(&format!(r#" {name}="{value}""#));
                        }
                    }
                    let xml = document(&header, set_swap);
                    assert_eq!(
                        xml,
                        baseline_xml.replacen(
                            &format!("<Items{baseline_header}>"),
                            &format!("<Items{header}>"),
                            1
                        )
                    );
                    let result = run(&source(&xml, 0x74), &a, &queries());
                    assert_header_only(&baseline_xml, &xml, &baseline, &result, selected);
                    let replay = run(&source(&xml, 0x74), &a, &queries());
                    assert_eq!(result.draft(), replay.draft());
                    assert_eq!(result.allocator_after(), replay.allocator_after());
                    assert_eq!(
                        serde_json::to_value(result.sidecar()).unwrap(),
                        serde_json::to_value(replay.sidecar()).unwrap()
                    );
                }
            }
        }
    }
}

#[test]
fn legacy_ambiguous_malformed_and_foreign_items_frames_keep_configuration_responsibility() {
    let good = document(r#" activeItemSet="9""#, "false");
    let mut refused: Vec<_> = [
        "",
        r#" activeItemSet="0""#,
        r#" activeItemSet="1""#,
        r#" activeItemSet="09""#,
        r#" activeItemSet="-9""#,
        r#" activeItemSet="9.0""#,
        r#" activeItemSet="nil""#,
        r#" activeItemSet=" 9""#,
        r#" activeItemSet="&#57;""#,
        r#" activeItemSet="4294967296""#,
        r#" activeItemSet="9" future="true""#,
        r#" activeItemSet="9" xmlns="urn:foreign""#,
        r#" activeItemSet="9" useSecondWeaponSet="0""#,
        r#" activeItemSet="9" useSecondWeaponSet="&#116;rue""#,
        r#" activeItemSet="9" showStatDifferences="TRUE""#,
        r#" activeItemSet="9" showStatDifferences="&#102;alse""#,
    ]
    .into_iter()
    .map(|header| document(header, "false"))
    .collect();
    refused.extend([
        good.replace("</Items>", "<Unknown/></Items>"),
        good.replace("</Items>", "<Slot name=\"Ring 1\" itemId=\"1\"/></Items>"),
        good.replace("</Items>", "non-whitespace</Items>"),
        good.replace("</Items>", "</Items><Items/>"),
        good.replace("</Items>", "</Items><Items xmlns=\"urn:foreign\"/>"),
        good.replace("<ItemSet id=\"2\"", "<ItemSet id=\"9\""),
        good.replace("<ItemSet id=\"2\"", "<ItemSet id=\"02\""),
        good.replace("<ItemSet id=\"2\"", "<ItemSet xmlns=\"urn:foreign\" id=\"2\""),
        good.replace("<PathOfBuilding2>", "<PathOfBuilding2 future=\"unreviewed\">"),
        good.replace("<PathOfBuilding2>", "<PathOfBuilding2>non-whitespace"),
        r#"<PathOfBuilding2><Items activeItemSet="9" useSecondWeaponSet="true"><Item id="1">Unknown item</Item></Items><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Items activeItemSet="9" useSecondWeaponSet="true"/><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
    ]);
    let a = artifacts(false);
    for xml in refused {
        let imported = source(&xml, 0x74);
        let result = run(&imported, &a, &queries());
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let items = evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "Items")
            .unwrap()
            .occurrence()
            .id();
        let config = pending(
            &result.draft().input().choice_presets.members[0]
                .choices
                .completion,
            "configuration-roles-not-converted",
        );
        let selection = pending(
            &result.draft().input().saved_variants.completion,
            "saved-selection-not-converted",
        );
        let links = &result.sidecar().origins[items.ordinal() as usize].links;
        assert!(links.contains(&OwnedOriginTarget::Issue(config)), "{xml}");
        assert!(
            !links.contains(&OwnedOriginTarget::Issue(selection)),
            "{xml}"
        );
    }
}
