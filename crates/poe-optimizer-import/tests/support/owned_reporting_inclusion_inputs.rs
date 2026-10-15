//! The actual injected Boolean grammar in the existing synthetic source fixture.
use super::*;
fn reporting_fixture() -> (Artifacts, NormalizationPolicy, UsagePolicyDefId) {
    let (mut a, mut p) = fixture();
    let mut input: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../data/owned/poe2/3887ae68/reporting-inclusion/usage.json"
    ))
    .unwrap();
    input["parameters"][0]["source"]["value"]["codec"]["namespace"] =
        serde_json::json!(a.schema.input().namespace);
    let source: UsageValueSource =
        serde_json::from_value(input["parameters"][0]["source"].clone()).unwrap();
    row(&mut p).group_attributes = vec!["enabled".into(), "includeInFullDPS".into()];
    let policy = add_policy(&mut a, &mut p, vec![(ValueSchema::Boolean, source)]);
    (a, p, policy)
}
fn xml(value: Option<&str>) -> String {
    let attribute = value.map_or(String::new(), |v| format!(" includeInFullDPS=\"{v}\""));
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="7"><SkillSet id="7"><Skill enabled="true"{attribute}>{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
#[test]
fn reporting_literal_true_false_nil_are_preserved_without_completing_usage() {
    let (a, p, policy) = reporting_fixture();
    for (text, expected) in [("true", true), ("false", false), ("nil", false)] {
        let result = normalize_with_loadouts(&xml(Some(text)), &a, &p).unwrap();
        let all = preferences(&result);
        assert_eq!(all.members.len(), 2);
        let record = all
            .members
            .iter()
            .find(|r| r.policy.to_resolved() == Some(policy.clone()))
            .unwrap();
        assert_eq!(record.target, all.members[0].target);
        assert_eq!(record.parameters.members.len(), 1);
        assert_eq!(
            record.parameters.members[0].value.to_resolved(),
            Some(ParameterValue::Boolean(expected))
        );
        assert!(matches!(
            record.parameters.completion,
            DraftListCompletion::Complete
        ));
        assert!(matches!(
            all.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            result.draft().input().skills.members[0]
                .enabled
                .to_resolved(),
            Some(true)
        );
        assert_eq!(
            all.members[0].parameters.members[0].value.to_resolved(),
            Some(ParameterValue::Boolean(true))
        );
    }
}
#[test]
fn absent_or_malformed_reporting_value_does_not_become_false_or_borrow_occurrence() {
    let (a, p, policy) = reporting_fixture();
    for raw in [
        None,
        Some(""),
        Some("0"),
        Some("TRUE"),
        Some(" true"),
        Some("unknown"),
    ] {
        let result = normalize_with_loadouts(&xml(raw), &a, &p).unwrap();
        let all = preferences(&result);
        let record = all
            .members
            .iter()
            .find(|r| r.policy.to_resolved() == Some(policy.clone()))
            .unwrap();
        assert!(record.parameters.members.is_empty());
        assert!(matches!(
            record.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            all.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            all.members[0].parameters.members.len(),
            2,
            "other authored preferences survive"
        );
    }
    let duplicate = xml(Some("true")).replace(
        "includeInFullDPS=\"true\"",
        "includeInFullDPS=\"true\" includeInFullDPS=\"false\"",
    );
    assert!(decode_build(duplicate.as_bytes()).is_err());
}
#[test]
fn reporting_group_projection_keeps_each_eligible_occurrence_and_archived_preset_separate() {
    let (a, p, policy) = reporting_fixture();
    let unknown = GEM.replace("reviewed=\"yes\"", "reviewed=\"no\"");
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7"><Skill enabled="false" includeInFullDPS="true">{GEM}{GEM}{unknown}</Skill></SkillSet><SkillSet id="8"><Skill enabled="true" includeInFullDPS="false">{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = normalize_with_loadouts(&text, &a, &p).unwrap();
    let draft = result.draft().input();
    assert_eq!(draft.skill_presets.members.len(), 2);
    for (preset, expected, count) in [
        (&draft.skill_presets.members[0], true, 2),
        (&draft.skill_presets.members[1], false, 1),
    ] {
        let records: Vec<_> = preset
            .usage_preferences
            .as_ref()
            .unwrap()
            .members
            .iter()
            .filter(|r| r.policy.to_resolved() == Some(policy.clone()))
            .collect();
        assert_eq!(records.len(), count);
        for record in &records {
            assert_eq!(
                record.parameters.members[0].value.to_resolved(),
                Some(ParameterValue::Boolean(expected))
            );
        }
        if count == 2 {
            assert_ne!(records[0].target, records[1].target);
        }
        assert!(matches!(
            preset.usage_preferences.as_ref().unwrap().completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    assert_eq!(
        draft.skills.members[0].enabled.to_resolved(),
        Some(false),
        "report inclusion cannot activate a disabled group"
    );
}
