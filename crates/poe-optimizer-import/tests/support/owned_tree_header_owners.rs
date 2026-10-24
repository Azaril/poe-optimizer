//! Saved Tree selection retains exact existing owners, without selecting a build.
use super::*;

fn document(attributes: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="81"/><Tree{attributes}><Spec title="first" treeVersion="fixture" classInternalId="alpha" ascendancyInternalId="one" nodes="11"/><Spec title="second" treeVersion="fixture" classInternalId="beta" ascendancyInternalId="two" nodes="22"/></Tree><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#
    )
}

fn normalize(
    xml: &str,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let imported = source(xml, 0x73);
    let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
    let a = artifacts(false);
    normalize_fresh(
        &evidence,
        *imported.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&a.schema),
            item_source: &empty_item_source(&a.schema),
            mappings: &a.mapping,
            registry: &a.registry,
            definitions: &a.schema,
            roles: &a.roles,
            rewards: &a.rewards,
        },
        &policy(),
        &[],
        limits,
    )
}

fn selection(result: &NormalizedImport) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } =
        &result.draft().input().saved_variants.completion
    else {
        panic!("selection must remain unresolved");
    };
    assert_eq!(code.as_str(), "saved-selection-not-converted");
    assert!(result.draft().input().saved_variants.members.is_empty());
    *id
}

fn configuration(result: &NormalizedImport) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } = &result.draft().input().choice_presets.members
        [0]
    .choices
    .completion
    else {
        panic!("whole configuration responsibility remains unresolved");
    };
    assert_eq!(code.as_str(), "configuration-roles-not-converted");
    *id
}

fn expected_links(result: &NormalizedImport, index: usize) -> [OwnedOriginTarget; 3] {
    let draft = result.draft().input();
    [
        OwnedOriginTarget::CharacterPreset(draft.character_presets.members[index].id),
        OwnedOriginTarget::AllocationPreset(draft.allocation_presets.members[index].id),
        OwnedOriginTarget::Issue(selection(result)),
    ]
}

#[test]
fn exact_tree_selection_accounts_only_the_header_and_keeps_all_real_pending_owners() {
    let baseline_xml = document(r#" activeSpec="0""#);
    let baseline = normalize(&baseline_xml, Default::default()).unwrap();
    assert!(
        baseline.sidecar().origins[2]
            .links
            .contains(&OwnedOriginTarget::Issue(configuration(&baseline)))
    );
    for selected in 1..=2 {
        let xml = document(&format!(r#" activeSpec="{selected}""#));
        assert_eq!(
            xml,
            baseline_xml.replace(r#"activeSpec="0""#, &format!(r#"activeSpec="{selected}""#))
        );
        let actual = normalize(&xml, Default::default()).unwrap();
        assert_eq!(actual.draft(), baseline.draft());
        assert_eq!(actual.allocator_after(), baseline.allocator_after());
        let tree = &actual.sidecar().origins[2];
        assert_eq!(tree.links.len(), 3);
        for target in expected_links(&actual, selected - 1) {
            assert!(tree.links.contains(&target));
        }
        assert_eq!(tree.disposition, SourceDisposition::Contributes);
        let before = source(&baseline_xml, 0x73);
        let after = source(&xml, 0x73);
        let before = SourceProjectEvidence::collect(&before, Default::default()).unwrap();
        let after = SourceProjectEvidence::collect(&after, Default::default()).unwrap();
        assert_eq!(before.rows().len(), after.rows().len());
        let mut expected = baseline.sidecar().origins.clone();
        assert_eq!(expected.len(), after.rows().len());
        for (ordinal, (old, new)) in before.rows().iter().zip(after.rows()).enumerate() {
            assert_eq!(old.occurrence().name(), new.occurrence().name());
            assert_eq!(old.occurrence().id(), expected[ordinal].source);
            assert_eq!(
                new.occurrence().id(),
                actual.sidecar().origins[ordinal].source
            );
            assert_eq!(
                old.occurrence().id().ordinal(),
                new.occurrence().id().ordinal()
            );
            expected[ordinal].source = new.occurrence().id();
        }
        expected[2].links = tree.links.clone();
        assert_eq!(expected, actual.sidecar().origins);
        for character in &actual.draft().input().character_presets.members {
            assert!(matches!(character.class, DraftField::Pending(_)));
            assert!(matches!(
                character.rewards.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        assert_eq!(configuration(&actual), configuration(&baseline));
        let replay = normalize(&xml, Default::default()).unwrap();
        assert_eq!(actual.draft(), replay.draft());
        assert_eq!(actual.allocator_after(), replay.allocator_after());
        assert_eq!(
            serde_json::to_value(actual.sidecar()).unwrap(),
            serde_json::to_value(replay.sidecar()).unwrap()
        );
    }
}

#[test]
fn missing_malformed_ambiguous_or_foreign_tree_frames_keep_configuration_responsibility() {
    let valid = document(r#" activeSpec="1""#);
    let mut refused: Vec<_> = [
        "",
        r#" activeSpec="0""#,
        r#" activeSpec="3""#,
        r#" activeSpec="01""#,
        r#" activeSpec="-1""#,
        r#" activeSpec="1.0""#,
        r#" activeSpec="nil""#,
        r#" activeSpec=" 1""#,
        r#" activeSpec="&#49;""#,
        r#" activeSpec="9007199254740992""#,
        r#" activeSpec="1" unknown="true""#,
        r#" activeSpec="1" xmlns="urn:foreign""#,
    ]
    .into_iter()
    .map(document)
    .collect();
    refused.extend([
        valid.replace("</Tree>", "<Unknown/></Tree>"),
        valid.replace("</Tree>", "non-whitespace</Tree>"),
        valid.replace("</Tree>", "</Tree><Tree activeSpec=\"1\"><Spec/></Tree>"),
        valid.replace("</Tree>", "</Tree><Spec/>"),
        valid.replace(
            "</Tree>",
            "</Tree><Tree xmlns=\"urn:foreign\" activeSpec=\"1\"><Spec/></Tree>",
        ),
        valid.replace(
            "<Spec title=\"second\"",
            "<Spec xmlns=\"urn:foreign\" title=\"second\"",
        ),
        valid.replace(
            "<PathOfBuilding2>",
            "<PathOfBuilding2 extra=\"unreviewed\">",
        ),
    ]);
    for xml in refused {
        let actual = normalize(&xml, Default::default()).unwrap();
        let imported = source(&xml, 0x73);
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        let tree = evidence
            .rows()
            .iter()
            .find(|r| r.occurrence().name() == "Tree")
            .unwrap();
        let links = &actual.sidecar().origins[tree.occurrence().id().ordinal() as usize].links;
        assert!(
            links.contains(&OwnedOriginTarget::Issue(configuration(&actual))),
            "{xml}"
        );
        assert!(
            !links.contains(&OwnedOriginTarget::Issue(selection(&actual))),
            "{xml}"
        );
    }
}

#[test]
fn tree_header_work_boundary_is_exact_and_failure_does_not_change_replay() {
    let xml = document(r#" activeSpec="2""#);
    let expected = normalize(&xml, Default::default()).unwrap();
    assert_eq!(expected.sidecar().origins[2].links.len(), 3);
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        match normalize(
            &xml,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(other) => panic!("unexpected boundary failure: {other:?}"),
        }
    }
    let exact = normalize(
        &xml,
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact.draft(), expected.draft());
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(exact.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
    assert!(matches!(
        normalize(
            &xml,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let replay = normalize(&xml, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_value(replay.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
