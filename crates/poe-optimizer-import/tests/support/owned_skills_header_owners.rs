//! Saved editing/ranking preferences never become Gem inputs or native queries.
use super::*;

const PREFERENCES: [(&str, &str); 6] = [
    ("defaultGemLevel", "normalMaximum"),
    ("defaultGemQuality", "0"),
    ("sortGemsByDPS", "true"),
    ("showLegacyGems", "false"),
    ("showSupportGemTypes", "ALL"),
    ("sortGemsByDPSField", "CombinedDPS"),
];

fn preferences(rows: &[(&str, &str)]) -> String {
    rows.iter()
        .map(|(name, value)| format!(r#" {name}="{value}""#))
        .collect()
}

fn document(header: &str, first_inputs: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Build level="81"/><Skills{header}><SkillSet id="9" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" enabled="true" {first_inputs}/></Skill></SkillSet><SkillSet id="2" title="same"><Skill enabled="true"><Gem gemId="active" variantId="v" level="1" quality="3" enabled="true"/><Gem gemId="support" variantId="v" level="1" quality="5" enabled="true"/></Skill><Skill source="Tree:99"><Gem gemId="active" variantId="v" level="1" quality="0" enabled="true"/></Skill></SkillSet></Skills><Config><ConfigSet id="1"/></Config></PathOfBuilding2>"#
    )
}

fn pending(list: &DraftListCompletion, expected: &str) -> DraftIssueId {
    let DraftListCompletion::Pending { id, code } = list else {
        panic!("{expected} must remain unresolved")
    };
    assert_eq!(code.as_str(), expected);
    *id
}

fn only_header_changed(
    before_xml: &str,
    after_xml: &str,
    before: &NormalizedImport,
    after: &NormalizedImport,
    selected: usize,
) {
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let draft = after.draft().input();
    let selection = pending(
        &draft.saved_variants.completion,
        "saved-selection-not-converted",
    );
    assert!(draft.saved_variants.members.is_empty());
    pending(
        &draft.choice_presets.members[0].choices.completion,
        "configuration-roles-not-converted",
    );
    let links = vec![
        OwnedOriginTarget::SkillPreset(draft.skill_presets.members[selected].id),
        OwnedOriginTarget::Issue(selection),
    ];
    assert_eq!(after.sidecar().origins[2].links, links);
    assert_eq!(
        after.sidecar().origins[2].disposition,
        SourceDisposition::Contributes
    );
    let old = source(before_xml, 0x75);
    let new = source(after_xml, 0x75);
    let old = SourceProjectEvidence::collect(&old, Default::default()).unwrap();
    let new = SourceProjectEvidence::collect(&new, Default::default()).unwrap();
    assert_eq!(old.rows().len(), new.rows().len());
    let mut expected = before.sidecar().origins.clone();
    // The caller authenticates the exact header-only XML substitution before
    // these content-addressed snapshot IDs are rebound by source occurrence.
    for (ordinal, (a, b)) in old.rows().iter().zip(new.rows()).enumerate() {
        assert_eq!(a.occurrence().name(), b.occurrence().name());
        assert_eq!(a.occurrence().id(), expected[ordinal].source);
        assert_eq!(b.occurrence().id(), after.sidecar().origins[ordinal].source);
        assert_eq!(a.occurrence().id().ordinal(), b.occurrence().id().ordinal());
        expected[ordinal].source = b.occurrence().id();
    }
    expected[2].links = links;
    assert_eq!(expected, after.sidecar().origins);
}

#[test]
fn keyed_preferences_and_omissions_preserve_actual_gem_inputs_queries_and_child_obligations() {
    let (a, policy, _, _) = quality_artifacts();
    let mut variants = vec![preferences(&PREFERENCES), String::new()];
    for omitted in 0..PREFERENCES.len() {
        let rows: Vec<_> = PREFERENCES
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, row)| (index != omitted).then_some(row))
            .collect();
        variants.push(preferences(&rows));
    }
    for (name, values) in [
        (
            "defaultGemLevel",
            &["normalMaximum", "corruptedMaximum", "characterLevel"][..],
        ),
        ("defaultGemQuality", &["0", "1", "20", "23"][..]),
        ("sortGemsByDPS", &["true", "false"][..]),
        ("showLegacyGems", &["true", "false"][..]),
        ("showSupportGemTypes", &["ALL", "LINEAGE", "NORMAL"][..]),
        (
            "sortGemsByDPSField",
            &[
                "FullDPS",
                "CombinedDPS",
                "TotalDPS",
                "AverageDamage",
                "TotalDot",
                "BleedDPS",
                "IgniteDPS",
                "TotalPoisonDPS",
                "TotalEHP",
            ][..],
        ),
    ] {
        for value in values {
            let rows: Vec<_> = PREFERENCES
                .iter()
                .map(|&(key, old)| (key, if key == name { *value } else { old }))
                .collect();
            variants.push(preferences(&rows));
        }
    }
    variants.sort();
    variants.dedup();
    for first_inputs in [
        r#"level="17" quality="19""#,
        "",
        r#"level="bad" quality="bad""#,
    ] {
        let baseline_header = format!(r#" activeSkillSet="0"{}"#, preferences(&PREFERENCES));
        let baseline_xml = document(&baseline_header, first_inputs);
        let baseline = run_with_policy(&source(&baseline_xml, 0x75), &a, &queries(), &policy);
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
        assert_eq!(baseline.draft().input().skill_presets.members.len(), 2);
        let first = &baseline.draft().input().gems.members[0];
        if first_inputs.contains("17") {
            assert_eq!(first.level.to_resolved(), Some(17));
            assert_eq!(
                first.quality.to_resolved().unwrap().unwrap().amount.value(),
                19.0
            );
        } else {
            assert!(matches!(first.level, DraftField::Pending(_)));
            assert!(matches!(first.quality, DraftQuality::Pending(_)));
        }
        for (key, selected) in [("9", 0), ("2", 1)] {
            for prefs in &variants {
                let header = format!(r#" activeSkillSet="{key}"{prefs}"#);
                let xml = document(&header, first_inputs);
                assert_eq!(
                    xml,
                    baseline_xml.replacen(
                        &format!("<Skills{baseline_header}>"),
                        &format!("<Skills{header}>"),
                        1
                    )
                );
                let actual = run_with_policy(&source(&xml, 0x75), &a, &queries(), &policy);
                only_header_changed(&baseline_xml, &xml, &baseline, &actual, selected);
                let replay = run_with_policy(&source(&xml, 0x75), &a, &queries(), &policy);
                assert_eq!(actual.draft(), replay.draft());
                assert_eq!(actual.allocator_after(), replay.allocator_after());
                assert_eq!(
                    serde_json::to_value(actual.sidecar()).unwrap(),
                    serde_json::to_value(replay.sidecar()).unwrap()
                );
            }
        }
    }
}

#[test]
fn unknown_legacy_malformed_or_ambiguous_skill_headers_keep_configuration_responsibility() {
    let good_header = format!(r#" activeSkillSet="9"{}"#, preferences(&PREFERENCES));
    let good = document(&good_header, r#"level="17" quality="19""#);
    let mut refused = Vec::new();
    for key in [
        "0",
        "1",
        "09",
        "-9",
        "9.0",
        "nil",
        " 9",
        "&#57;",
        "9007199254740992",
    ] {
        refused.push(good.replace(
            r#"activeSkillSet="9""#,
            &format!(r#"activeSkillSet="{key}""#),
        ));
    }
    refused.push(good.replace(r#" activeSkillSet="9""#, ""));
    for (name, original, values) in [
        (
            "defaultGemLevel",
            "normalMaximum",
            &["", "20", "awakenedMaximum", "unknown", "normalMaxim&#117;m"][..],
        ),
        (
            "defaultGemQuality",
            "0",
            &["", "24", "-1", "00", "1.5", "0.0", "NaN", "nil", "&#48;"][..],
        ),
        (
            "sortGemsByDPS",
            "true",
            &["nil", "TRUE", "1", "tr&#117;e"][..],
        ),
        ("showLegacyGems", "false", &["nil", "FALSE", "0"][..]),
        (
            "showSupportGemTypes",
            "ALL",
            &["all", "unknown", "&#65;LL"][..],
        ),
        (
            "sortGemsByDPSField",
            "CombinedDPS",
            &["", "UnknownMetric", "CombinedDP&#83;"][..],
        ),
    ] {
        for value in values {
            refused.push(good.replace(
                &format!(r#"{name}="{original}""#),
                &format!(r#"{name}="{value}""#),
            ));
        }
    }
    for extra in [
        r#"matchGemLevelToCharacterLevel="true""#,
        r#"matchGemLevelToCharacterLevel="false""#,
        r#"future="true""#,
        r#"xmlns="urn:foreign""#,
    ] {
        refused.push(good.replace("<Skills ", &format!("<Skills {extra} ")));
    }
    refused.extend([
        good.replace("</Skills>", "<Unknown/></Skills>"),
        good.replace("</Skills>", "<Skill/></Skills>"),
        good.replace("</Skills>", "non-whitespace</Skills>"),
        good.replace("</Skills>", "</Skills><Skills/>"),
        good.replace("</Skills>", "</Skills><Skills xmlns=\"urn:foreign\"/>"),
        good.replace("<SkillSet id=\"2\"", "<SkillSet id=\"9\""),
        good.replace("<SkillSet id=\"2\"", "<SkillSet id=\"02\""),
        good.replace(
            "<SkillSet id=\"2\"",
            "<SkillSet xmlns=\"urn:foreign\" id=\"2\"",
        ),
        good.replace("<PathOfBuilding2>", "<PathOfBuilding2 unknown=\"true\">"),
        good.replace("<PathOfBuilding2>", "<PathOfBuilding2>non-whitespace"),
    ]);
    let a = artifacts(true);
    for xml in refused {
        let imported = source(&xml, 0x75);
        let actual = run(&imported, &a, &queries());
        let config = pending(
            &actual.draft().input().choice_presets.members[0]
                .choices
                .completion,
            "configuration-roles-not-converted",
        );
        let selection = pending(
            &actual.draft().input().saved_variants.completion,
            "saved-selection-not-converted",
        );
        assert!(
            actual.sidecar().origins[2]
                .links
                .contains(&OwnedOriginTarget::Issue(config)),
            "{xml}"
        );
        assert!(
            !actual.sidecar().origins[2]
                .links
                .contains(&OwnedOriginTarget::Issue(selection)),
            "{xml}"
        );
    }
}
