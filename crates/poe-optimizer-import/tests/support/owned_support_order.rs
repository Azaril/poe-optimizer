//! Source encounter order survives normalization independently of record identity.
use super::*;

fn reviewed_policy() -> NormalizationPolicy {
    let mut reviewed = policy();
    reviewed.support_origin_order = Some(SupportOriginOrderPolicy::SavedManualGroupOrder {});
    reviewed
}

fn normalize(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&artifacts.schema),
            item_source: &empty_item_source(&artifacts.schema),
            mappings: &artifacts.mapping,
            registry: &artifacts.registry,
            definitions: &artifacts.schema,
            roles: &artifacts.roles,
            rewards: &artifacts.rewards,
        },
        policy,
        &queries(),
        limits,
    )
}

fn sequence(preset: &SkillPresetDraft) -> &[AuthoredSupportOrderDraft] {
    let order = preset
        .authored_support_order
        .as_ref()
        .expect("reviewed order");
    let DraftListCompletion::Pending { code, .. } = &order.completion else {
        panic!("local order cannot close merged origin discovery")
    };
    assert_eq!(code.as_str(), "support-origin-discovery-not-converted");
    assert!(order.to_resolved().is_none());
    &order.members
}

fn members(sequence: &AuthoredSupportOrderDraft) -> &[SupportAssignmentId] {
    let DraftField::Known { value } = &sequence.assignments else {
        panic!("exact local assignment order should be preserved")
    };
    value
}

#[test]
fn source_order_preserves_disabled_and_duplicate_definitions_without_selecting_winners() {
    let xml = group(&format!(
        "{ACTIVE}{}{SUPPORT}",
        SUPPORT
            .replace("level=\"1\"", "level=\"9\"")
            .replace("enabled=\"true\"", "enabled=\"false\"")
    ));
    let source = source(&xml, 0x65);
    let result = normalize(
        &source,
        &artifacts(true),
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    let input = result.draft().input();
    let preset = &input.skill_presets.members[0];
    let rows = sequence(preset);
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0].target.to_resolved(),
        Some(SkillTarget::Authored(input.skills.members[0].id))
    );
    assert_eq!(
        members(&rows[0]),
        input
            .supports
            .members
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(input.supports.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(input.supports.members[1].enabled.to_resolved(), Some(true));
    assert_eq!(input.gems.members[1].level.to_resolved(), Some(9));
    assert_eq!(input.gems.members[2].level.to_resolved(), Some(1));
    assert_eq!(
        input.gems.members[1].definition,
        input.gems.members[2].definition
    );
    assert_eq!(
        input.query_presets.members[0]
            .queries
            .requests
            .members
            .len(),
        2
    );
    origin_integrity(&source, &result);

    // Relabel occurrence identities consistently, making the first encountered
    // origin's ID greater than the second. Canonical record sorting cannot be
    // allowed to change the independently preserved sequence.
    let mut relabeled = input.clone();
    let first = relabeled.supports.members[0].id;
    let second = relabeled.supports.members[1].id;
    relabeled.supports.members[0].id = second;
    relabeled.supports.members[1].id = first;
    let origins = &mut relabeled.skill_presets.members[0]
        .authored_support_order
        .as_mut()
        .unwrap()
        .members[0]
        .assignments;
    *origins = vec![second, first].into();
    relabeled.supports.members.sort_by_key(|row| row.id);
    validate_draft(&relabeled, DraftLimits::default()).unwrap();
    assert_eq!(
        members(&sequence(&relabeled.skill_presets.members[0])[0]),
        &[second, first]
    );
    assert_eq!(
        relabeled.supports.members[0].enabled.to_resolved(),
        Some(true)
    );
}

#[test]
fn saved_presets_and_exact_targets_do_not_merge_local_sequences() {
    let xml = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="9"><Skill enabled="false">{ACTIVE}{SUPPORT}{SUPPORT}</Skill><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet><SkillSet id="1"><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let source = source(&xml, 0x66);
    let result = normalize(
        &source,
        &artifacts(true),
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    let input = result.draft().input();
    let first = sequence(&input.skill_presets.members[0]);
    let second = sequence(&input.skill_presets.members[1]);
    assert_eq!((first.len(), second.len()), (2, 1));
    assert_eq!(
        (
            members(&first[0]).len(),
            members(&first[1]).len(),
            members(&second[0]).len()
        ),
        (2, 1, 1)
    );
    assert_ne!(first[0].target.to_resolved(), first[1].target.to_resolved());
    assert_ne!(
        first[1].target.to_resolved(),
        second[0].target.to_resolved()
    );
    assert_eq!(input.supports.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(input.supports.members[1].enabled.to_resolved(), Some(false));
    origin_integrity(&source, &result);
}

#[test]
fn ambiguous_generated_and_unreviewed_targets_never_gain_ordered_authored_targets() {
    for xml in [
        group(&format!("{ACTIVE}{SUPPORT}{ACTIVE}")),
        group(&format!("{ACTIVE}{SUPPORT}<Unknown/>")),
        group(&format!("{ACTIVE}{SUPPORT}<Gem gemId=\"unknown\"/>")),
        group(&format!("{ACTIVE}{SUPPORT}")).replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" source=\"Item:1\">",
        ),
    ] {
        let source = source(&xml, 0x67);
        let result = normalize(
            &source,
            &artifacts(true),
            &reviewed_policy(),
            NormalizationLimits::default(),
        )
        .unwrap();
        assert!(sequence(&result.draft().input().skill_presets.members[0]).is_empty());
        assert!(matches!(
            result.draft().input().supports.members[0].target,
            DraftSkillTarget::Pending(_)
        ));
        origin_integrity(&source, &result);
    }
    let mut no_target_proof = reviewed_policy();
    no_target_proof.single_active_support_target = false;
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x68);
    let result = normalize(
        &source,
        &artifacts(true),
        &no_target_proof,
        NormalizationLimits::default(),
    )
    .unwrap();
    assert!(sequence(&result.draft().input().skill_presets.members[0]).is_empty());
}

#[test]
fn empty_local_membership_remains_pending_and_unknown_groups_do_not_erase_known_siblings() {
    let xml = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="1"><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill><Skill source="Item:1" enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{ACTIVE}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let source = source(&xml, 0x69);
    let result = normalize(
        &source,
        &artifacts(true),
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    let input = result.draft().input();
    assert_eq!(sequence(&input.skill_presets.members[0]).len(), 1);
    assert_eq!(
        members(&sequence(&input.skill_presets.members[0])[0]).len(),
        1
    );
    assert_eq!(input.skill_presets.members[0].supports.members.len(), 2);
    assert!(sequence(&input.skill_presets.members[1]).is_empty());
    origin_integrity(&source, &result);
}

#[test]
fn omitted_policy_preserves_wire_digest_and_allocator_behavior() {
    let legacy = policy();
    let wire = serde_json::to_vec(&legacy).unwrap();
    let value = serde_json::to_value(&legacy).unwrap();
    assert!(value.get("support_origin_order").is_none());
    let decoded: NormalizationPolicy = serde_json::from_slice(&wire).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), wire);
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x6a);
    let artifacts = artifacts(true);
    let original = normalize(&source, &artifacts, &legacy, NormalizationLimits::default()).unwrap();
    let roundtrip = normalize(
        &source,
        &artifacts,
        &decoded,
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_eq!(original.sidecar().policy, roundtrip.sidecar().policy);
    assert_eq!(original.sidecar().draft, roundtrip.sidecar().draft);
    assert_eq!(
        original.sidecar().allocator_after,
        roundtrip.sidecar().allocator_after
    );
    assert_eq!(
        serde_json::to_vec(original.draft().input()).unwrap(),
        serde_json::to_vec(roundtrip.draft().input()).unwrap()
    );
    assert!(
        original.draft().input().skill_presets.members[0]
            .authored_support_order
            .is_none()
    );
    let reviewed = normalize(
        &source,
        &artifacts,
        &reviewed_policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    assert_ne!(original.sidecar().policy, reviewed.sidecar().policy);
    assert_ne!(original.sidecar().draft, reviewed.sidecar().draft);
    assert_eq!(
        reviewed.sidecar().allocator_after.last_issued(),
        original.sidecar().allocator_after.last_issued() + 1
    );
    for invalid in [
        serde_json::json!({"kind":"sorted_assignment_ids"}),
        serde_json::json!({"kind":"saved_manual_group_order", "complete":true}),
    ] {
        assert!(serde_json::from_value::<SupportOriginOrderPolicy>(invalid).is_err());
    }
}

#[test]
fn order_expansion_checks_collection_bounds_before_growing_sequences() {
    let artifacts = artifacts(true);
    let mut limits = NormalizationLimits::default();
    limits.draft.input.max_collection_entries = 2;
    let source = source(
        &group(&format!("{ACTIVE}{SUPPORT}{SUPPORT}{SUPPORT}")),
        0x6b,
    );
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed_policy(), limits),
        Err(NormalizationError::Limit("support origin sequence members"))
    ));
    let xml = format!(
        r#"<PathOfBuilding2><Skills><SkillSet id="1"><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill><Skill enabled="true">{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let source = super::source(&xml, 0x6c);
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed_policy(), limits),
        Err(NormalizationError::Limit("support origin sequences"))
    ));
}

#[test]
fn reviewed_order_accounts_for_work_issue_and_origin_link_budgets() {
    let source = source(&group(&format!("{ACTIVE}{SUPPORT}")), 0x6d);
    let artifacts = artifacts(true);
    let baseline = normalize(
        &source,
        &artifacts,
        &policy(),
        NormalizationLimits::default(),
    )
    .unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: middle,
            ..NormalizationLimits::default()
        };
        match normalize(&source, &artifacts, &policy(), limits) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit("work")) => low = middle + 1,
            Err(error) => panic!("unexpected work-bound failure: {error}"),
        }
    }
    let limits = NormalizationLimits {
        max_work: low,
        ..NormalizationLimits::default()
    };
    assert!(normalize(&source, &artifacts, &policy(), limits).is_ok());
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed_policy(), limits),
        Err(NormalizationError::Limit("work"))
    ));
    let mut limits = NormalizationLimits::default();
    limits.draft.max_issues = baseline
        .draft()
        .validate_limits(limits.draft)
        .unwrap()
        .issues
        .len();
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed_policy(), limits),
        Err(NormalizationError::Limit("issues"))
    ));
    let limits = NormalizationLimits {
        max_origin_links: baseline
            .sidecar()
            .origins
            .iter()
            .map(|origin| origin.links.len())
            .sum(),
        ..NormalizationLimits::default()
    };
    assert!(matches!(
        normalize(&source, &artifacts, &reviewed_policy(), limits),
        Err(NormalizationError::Limit("origin links"))
    ));
}
