//! Build-header ownership preserves the separate unresolved selection and children.
use super::*;

#[path = "owned_character_reward_policy.rs"]
mod character_reward_policy;

const BUILD: &str = r#"<Build targetVersion="0_1" viewMode="IMPORT" level="73" className="Caller label" ascendClassName="Other label" mainSocketGroup="1" characterLevelAutoMode="false"><PlayerStat stat="caller-cache" value="99"/><MinionStat stat="caller-cache" value="12"/><Buffs buffList="caller-state"/><BeastCompanion id="caller-beast"/><TimelessData devotionVariant1="1" devotionVariant2="2" searchList="" searchListFallback="" socketFilterDistance="0"/></Build>"#;

fn document() -> String {
    let specs: String = (1..=3)
        .map(|class| {
            format!(
                r#"<Spec title="same" treeVersion="0_5" classId="{class}" classInternalId="{class}" ascendClassId="0" ascendancyInternalId="" nodes="{class}"/>"#
            )
        })
        .collect();
    format!(
        r#"<PathOfBuilding2>{BUILD}<Tree activeSpec="2">{specs}</Tree><Skills activeSkillSet="9"><SkillSet id="9" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" level="17" enabled="true"/></Skill><Skill source="Tree:99"><Gem skillId="caller-generated" enabled="true"/></Skill></SkillSet><SkillSet id="2" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" level="1" enabled="true"/></Skill></SkillSet></Skills><Config activeConfigSet="1"><ConfigSet id="1"><Input name="caller-toggle" boolean="true"/><Input name="unrelated-role" number="7"/></ConfigSet></Config></PathOfBuilding2>"#
    )
}

fn reviewed(a: &Artifacts) -> NormalizationPolicy {
    let mut p = policy();
    p.character_reward_inventory = Some(character_reward_policy::policy(&a.mapping));
    p
}

fn normalize(
    imported: &ImportedBuildInstance,
    a: &Artifacts,
    p: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(imported, Default::default()).unwrap();
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
        p,
        &queries(),
        limits,
    )
}

fn pending(completion: &DraftListCompletion, expected: &str) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } = completion else {
        panic!("{expected} must remain unresolved")
    };
    assert_eq!(code.as_str(), expected);
    *id
}

fn selection(result: &NormalizedImport) -> DraftIssueId {
    assert!(result.draft().input().saved_variants.members.is_empty());
    pending(
        &result.draft().input().saved_variants.completion,
        "saved-selection-not-converted",
    )
}

fn configuration(result: &NormalizedImport) -> DraftIssueId {
    pending(
        &result.draft().input().choice_presets.members[0]
            .choices
            .completion,
        "configuration-roles-not-converted",
    )
}

fn assert_accounted(result: &NormalizedImport, selected: usize, level: u16) {
    let draft = result.draft().input();
    assert_eq!(draft.character_presets.members.len(), 3);
    assert_eq!(draft.skill_presets.members.len(), 2);
    let mut expected = Vec::new();
    let mut distinct = BTreeSet::new();
    for character in &draft.character_presets.members {
        assert_eq!(character.level.to_resolved(), Some(level));
        assert!(distinct.insert(character.id));
        expected.push(OwnedOriginTarget::CharacterPreset(character.id));
    }
    expected.push(OwnedOriginTarget::SkillPreset(
        draft.skill_presets.members[selected].id,
    ));
    expected.push(OwnedOriginTarget::Issue(selection(result)));
    let header = &result.sidecar().origins[1];
    assert_eq!(header.disposition, SourceDisposition::Contributes);
    assert_eq!(header.links.len(), expected.len());
    for link in expected {
        assert_eq!(
            header
                .links
                .iter()
                .filter(|actual| **actual == link)
                .count(),
            1
        );
    }
    assert!(
        !header
            .links
            .contains(&OwnedOriginTarget::Issue(configuration(result)))
    );
    assert!(
        !header
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::QueryPreset(_)))
    );
}

fn assert_refused(imported: &ImportedBuildInstance, result: &NormalizedImport, context: &str) {
    let evidence = SourceProjectEvidence::collect(imported, Default::default()).unwrap();
    let headers: Vec<_> = evidence
        .rows()
        .iter()
        .filter(|row| row.occurrence().name() == "Build")
        .collect();
    assert!(!headers.is_empty());
    for header in headers {
        let links = &result.sidecar().origins[header.occurrence().id().ordinal() as usize].links;
        assert!(
            links.contains(&OwnedOriginTarget::Issue(configuration(result))),
            "{context}"
        );
        assert!(
            !links.contains(&OwnedOriginTarget::Issue(selection(result))),
            "{context}"
        );
        assert!(!links.iter().any(|link| matches!(
            link,
            OwnedOriginTarget::CharacterPreset(_)
                | OwnedOriginTarget::SkillPreset(_)
                | OwnedOriginTarget::QueryPreset(_)
        )));
    }
}

fn only_header_links_changed(
    before_xml: &str,
    after_xml: &str,
    before: &NormalizedImport,
    after: &NormalizedImport,
) {
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(configuration(before), configuration(after));
    assert_eq!(selection(before), selection(after));
    let before_import = source(before_xml, 0x76);
    let after_import = source(after_xml, 0x76);
    let old = SourceProjectEvidence::collect(&before_import, Default::default()).unwrap();
    let new = SourceProjectEvidence::collect(&after_import, Default::default()).unwrap();
    assert_eq!(old.rows().len(), new.rows().len());
    let mut expected = before.sidecar().origins.clone();
    // The caller proves the exact single-header edit. Rebind only snapshot IDs;
    // all descendant provenance, cached outputs and unresolved owners survive.
    for (ordinal, (a, b)) in old.rows().iter().zip(new.rows()).enumerate() {
        assert_eq!(a.occurrence().name(), b.occurrence().name());
        assert_eq!(a.occurrence().id(), expected[ordinal].source);
        assert_eq!(b.occurrence().id(), after.sidecar().origins[ordinal].source);
        assert_eq!(a.occurrence().id().ordinal(), b.occurrence().id().ordinal());
        expected[ordinal].source = b.occurrence().id();
    }
    expected[1].links = after.sidecar().origins[1].links.clone();
    assert_eq!(expected, after.sidecar().origins);
}

#[test]
fn exact_build_level_owns_all_characters_and_saved_skill_key_without_selecting_queries() {
    let a = with_reward_policy();
    let p = reviewed(&a);
    for (key, selected, group) in [("9", 0, "1"), ("9", 0, "2"), ("2", 1, "1")] {
        for (level, view) in [(1, "IMPORT"), (73, "CALCS"), (100, "TREE")] {
            let actual_xml = document()
                .replace(
                    r#"activeSkillSet="9""#,
                    &format!(r#"activeSkillSet="{key}""#),
                )
                .replacen(r#"level="73""#, &format!(r#"level="{level}""#), 1)
                .replace(
                    r#"mainSocketGroup="1""#,
                    &format!(r#"mainSocketGroup="{group}""#),
                )
                .replace(r#"viewMode="IMPORT""#, &format!(r#"viewMode="{view}""#));
            let before_xml =
                actual_xml.replace(&format!(r#"viewMode="{view}""#), r#"viewMode="unreviewed""#);
            assert_eq!(
                actual_xml,
                before_xml.replace(r#"viewMode="unreviewed""#, &format!(r#"viewMode="{view}""#))
            );
            let before_import = source(&before_xml, 0x76);
            let actual_import = source(&actual_xml, 0x76);
            let before = normalize(&before_import, &a, &p, Default::default()).unwrap();
            let actual = normalize(&actual_import, &a, &p, Default::default()).unwrap();
            assert_refused(&before_import, &before, "unreviewed view baseline");
            assert_accounted(&actual, selected, level);
            only_header_links_changed(&before_xml, &actual_xml, &before, &actual);
            // Independent Config rewards and source-generated responsibilities
            // are not treated as Character rewards or MAIN output selection.
            assert_eq!(actual.draft().input().rewards.members.len(), 1);
            assert_eq!(
                actual.draft().input().choice_presets.members[0]
                    .rewards
                    .members
                    .len(),
                1
            );
            // These synthetic Specs omit the independent reward frame's
            // URL/Sockets/Overrides. Header ownership cannot complete it.
            for character in &actual.draft().input().character_presets.members {
                assert!(character.rewards.members.is_empty());
                pending(
                    &character.rewards.completion,
                    "character-rewards-not-converted",
                );
            }
        }
    }
}

#[test]
fn equal_numbers_from_other_attributes_defaults_or_unreviewed_recipes_are_not_level_authority() {
    let a = with_reward_policy();
    let p = reviewed(&a);
    let xml = document()
        .replace("Caller label", "73")
        .replace("Other label", "73");
    let imported = source(&xml, 0x76);
    let baseline = normalize(&imported, &a, &p, Default::default()).unwrap();
    assert_accounted(&baseline, 0, 73);
    let mut cases = Vec::new();
    for attribute in ["className", "ascendClassName"] {
        let mut changed = p.clone();
        changed.character_level.tiers[0].selectors[0].name = attribute.into();
        cases.push((attribute, changed));
    }
    let mut defaulted = p.clone();
    defaulted.character_level.tiers[0].selectors[0].name = "missing-level".into();
    defaulted.character_level.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Integer(BoundedInteger::new(73).unwrap()),
    };
    cases.push(("matching explicit default", defaulted));
    let mut extra_selector = p.clone();
    extra_selector.character_level.tiers[0]
        .selectors
        .push(ValueSelector {
            lane: ValueLane::Attribute,
            name: "absent-other-source".into(),
        });
    cases.push(("unreviewed selector set", extra_selector));
    for (label, changed) in cases {
        let actual = normalize(&imported, &a, &changed, Default::default()).unwrap();
        assert_eq!(actual.draft(), baseline.draft(), "{label}");
        assert_eq!(
            actual.allocator_after(),
            baseline.allocator_after(),
            "{label}"
        );
        assert_refused(&imported, &actual, label);
    }
    let mut foreign_lane = p.clone();
    foreign_lane.character_level.tiers[0].selectors[0].lane = ValueLane::ParentAttribute;
    assert!(matches!(
        normalize(&imported, &a, &foreign_lane, Default::default()),
        Err(NormalizationError::Policy(_))
    ));
    // Missing source-frame authority cannot be supplied by otherwise correct
    // scalar output, and a different injected target is not silently accepted.
    let mut no_authority = p.clone();
    no_authority.character_reward_inventory = None;
    assert_refused(
        &imported,
        &normalize(&imported, &a, &no_authority, Default::default()).unwrap(),
        "missing source authority",
    );
    let mut other_target = p.clone();
    let CharacterRewardInventoryPolicy::PobFreshCharacterOnlyEmptyV1 { target_version, .. } =
        other_target.character_reward_inventory.as_mut().unwrap();
    *target_version = "other".into();
    assert_refused(
        &imported,
        &normalize(&imported, &a, &other_target, Default::default()).unwrap(),
        "different source target version",
    );
}

#[test]
fn malformed_legacy_and_crossed_frames_do_not_retire_build_responsibility() {
    let a = with_reward_policy();
    let p = reviewed(&a);
    let good = document();
    assert_accounted(
        &normalize(&source(&good, 0x76), &a, &p, Default::default()).unwrap(),
        0,
        73,
    );
    let mut refused = Vec::new();
    for (attribute, old, values) in [
        (
            "level",
            "73",
            &["0", "101", "073", "73.0", "nil", "&#55;3"][..],
        ),
        (
            "mainSocketGroup",
            "1",
            &[
                "0",
                "3",
                "01",
                "-1",
                "1.0",
                "nil",
                "&#49;",
                "9007199254740992",
            ][..],
        ),
        (
            "characterLevelAutoMode",
            "false",
            &["true", "nil", "FALSE", "&#102;alse"][..],
        ),
        ("targetVersion", "0_1", &["other", "0&#95;1"][..]),
        (
            "viewMode",
            "IMPORT",
            &["unknown", "import", "IMP&#79;RT"][..],
        ),
    ] {
        let original = format!(r#"{attribute}="{old}""#);
        refused.push((
            format!("missing {attribute}"),
            good.replacen(&original, "", 1),
        ));
        for value in values {
            refused.push((
                format!("{attribute}={value}"),
                good.replacen(&original, &format!(r#"{attribute}="{value}""#), 1),
            ));
        }
    }
    for (label, from, to) in [
        (
            "legacy MAIN precedence",
            "<Build ",
            "<Build mainSkillIndex=\"1\" ",
        ),
        ("unknown header", "<Build ", "<Build future=\"true\" "),
        ("foreign header", "<Build ", "<Build xmlns=\"urn:foreign\" "),
        ("unknown child", "</Build>", "<Unknown/></Build>"),
        ("foreign child", "<Buffs ", "<Buffs xmlns=\"urn:foreign\" "),
        ("header text", "</Build>", "unreviewed</Build>"),
        (
            "missing selected key",
            "activeSkillSet=\"9\"",
            "activeSkillSet=\"1\"",
        ),
        (
            "ordinal instead of key",
            "activeSkillSet=\"9\"",
            "activeSkillSet=\"01\"",
        ),
        (
            "duplicate skill keys",
            "SkillSet id=\"2\"",
            "SkillSet id=\"9\"",
        ),
        (
            "foreign skill set",
            "SkillSet id=\"2\"",
            "SkillSet xmlns=\"urn:foreign\" id=\"2\"",
        ),
        (
            "foreign Spec",
            "Spec title=\"same\"",
            "Spec xmlns=\"urn:foreign\" title=\"same\"",
        ),
        ("orphan Spec", "</Tree>", "</Tree><Spec nodes=\"4\"/>"),
        (
            "duplicate Tree",
            "</Tree>",
            "</Tree><Tree activeSpec=\"1\"><Spec nodes=\"4\"/></Tree>",
        ),
        (
            "unknown root",
            "<PathOfBuilding2>",
            "<PathOfBuilding2 extra=\"true\">",
        ),
    ] {
        refused.push((label.into(), good.replacen(from, to, 1)));
    }
    refused.push((
        "duplicate Build".into(),
        good.replace(BUILD, &format!("{BUILD}{BUILD}")),
    ));
    refused.push((
        "MAIN outside selected set".into(),
        good.replace("activeSkillSet=\"9\"", "activeSkillSet=\"2\"")
            .replace("mainSocketGroup=\"1\"", "mainSocketGroup=\"2\""),
    ));
    for (label, xml) in refused {
        let imported = source(&xml, 0x76);
        let actual = normalize(&imported, &a, &p, Default::default())
            .unwrap_or_else(|error| panic!("{label}: {error:?}"));
        assert_refused(&imported, &actual, &label);
    }
}

#[test]
fn saved_labels_do_not_select_classes_or_change_any_native_query_or_source_child() {
    let a = with_reward_policy();
    let p = reviewed(&a);
    let base = document();
    let before = normalize(&source(&base, 0x76), &a, &p, Default::default()).unwrap();
    assert_accounted(&before, 0, 73);
    for (class, ascendancy) in [("unrelated label", "Other class"), ("", ""), ("73", "73")] {
        let xml = base
            .replace("Caller label", class)
            .replace("Other label", ascendancy);
        let actual = normalize(&source(&xml, 0x76), &a, &p, Default::default()).unwrap();
        assert_accounted(&actual, 0, 73);
        only_header_links_changed(&base, &xml, &before, &actual);
    }
}

#[test]
fn late_work_and_origin_link_limits_are_exact_and_leave_inputs_and_replay_unchanged() {
    let a = with_reward_policy();
    let p = reviewed(&a);
    let policy_before = serde_json::to_value(&p).unwrap();
    let xml = document();
    let imported = source(&xml, 0x76);
    let allocator_before = *imported.allocator_state();
    let expected = normalize(&imported, &a, &p, Default::default()).unwrap();
    assert_accounted(&expected, 0, 73);
    for work in [true, false] {
        let bounded = |maximum| {
            let mut limits = NormalizationLimits::default();
            if work {
                limits.max_work = maximum;
            } else {
                limits.max_origin_links = maximum;
            }
            normalize(&imported, &a, &p, limits)
        };
        let mut low = 1;
        let mut high = if work {
            NormalizationLimits::default().max_work
        } else {
            NormalizationLimits::default().max_origin_links
        };
        while low < high {
            let middle = low + (high - low) / 2;
            match bounded(middle) {
                Ok(_) => high = middle,
                Err(NormalizationError::Limit(_)) => low = middle + 1,
                Err(error) => panic!("unexpected boundary error: {error:?}"),
            }
        }
        let exact = bounded(low).unwrap();
        assert_eq!(exact.draft(), expected.draft());
        assert_eq!(
            serde_json::to_value(exact.sidecar()).unwrap(),
            serde_json::to_value(expected.sidecar()).unwrap()
        );
        assert!(matches!(
            bounded(low - 1),
            Err(NormalizationError::Limit(_))
        ));
        assert_eq!(*imported.allocator_state(), allocator_before);
        assert_eq!(serde_json::to_value(&p).unwrap(), policy_before);
    }
    let other_xml = xml.replace("activeSkillSet=\"9\"", "activeSkillSet=\"2\"");
    let other = normalize(&source(&other_xml, 0x76), &a, &p, Default::default()).unwrap();
    assert_accounted(&other, 1, 73);
    let replay = normalize(&imported, &a, &p, Default::default()).unwrap();
    assert_eq!(replay.draft(), expected.draft());
    assert_eq!(
        serde_json::to_value(replay.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
