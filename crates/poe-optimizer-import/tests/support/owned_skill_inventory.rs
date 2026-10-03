//! Authored root inventory is independent of values, usage and generated effects.
use super::*;

fn reviewed(artifacts: &Artifacts) -> NormalizationPolicy {
    let mut policy = policy();
    policy.skill_inventory = Some(SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        mapping_source: *artifacts.mapping.source_identity(),
        roles: *artifacts.roles.identity(),
        direct_inputs: None,
        generated_groups: vec![],
    });
    policy
}
fn xml(groups: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1">{groups}</SkillSet></Skills></PathOfBuilding2>"#
    )
}
fn normalize(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, Default::default()).unwrap();
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
fn completions(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .skill_presets
        .members
        .iter()
        .map(|p| matches!(p.skills.completion, DraftListCompletion::Complete))
        .collect()
}
fn only_membership_changes(before: NormalizedImport, after: &NormalizedImport, selected: &[usize]) {
    let mut expected = before.draft().input().clone();
    let mut retired = BTreeSet::new();
    for &index in selected {
        let completion = &mut expected.skill_presets.members[index].skills.completion;
        let DraftListCompletion::Pending { id, code } = completion else {
            panic!("original membership must be pending");
        };
        assert_eq!(code.as_str(), "skill-membership-not-converted");
        retired.insert(*id);
        *completion = DraftListCompletion::Complete;
    }
    assert_eq!(&expected, after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut sidecar = before.into_parts().1;
    sidecar.policy = after.sidecar().policy;
    sidecar.draft = after.sidecar().draft;
    let mut removed = BTreeSet::new();
    for row in &mut sidecar.origins {
        row.links.retain(|link| {
            if let OwnedOriginTarget::Issue(id) = link
                && retired.contains(id)
            {
                assert!(
                    removed.insert(*id),
                    "retire only the exact originating issue"
                );
                false
            } else {
                true
            }
        });
    }
    assert_eq!(removed, retired);
    assert_eq!(
        serde_json::to_value(sidecar).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
}

#[test]
fn physical_roots_preserve_order_identity_disabled_state_and_unresolved_values() {
    let artifacts = artifacts(true);
    let text = xml(&format!(
        r#"<Skill enabled="false">{ACTIVE}{SUPPORT}{ACTIVE}</Skill>"#
    ));
    let source = source(&text, 0xe1);
    let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
    let after = normalize(
        &source,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    assert_eq!(completions(&after), [true]);
    let draft = after.draft().input();
    assert_eq!(draft.skills.members.len(), 2);
    assert_eq!(
        draft.skill_presets.members[0].skills.members,
        draft
            .skills
            .members
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>()
    );
    assert_ne!(draft.skills.members[0].id, draft.skills.members[1].id);
    assert!(
        draft
            .skills
            .members
            .iter()
            .all(|s| s.enabled.to_resolved() == Some(false))
    );
    assert!(
        draft
            .gems
            .members
            .iter()
            .all(|g| matches!(g.parameters.completion, DraftListCompletion::Pending { .. }))
    );
    assert!(matches!(
        draft.skill_presets.members[0].supports.completion,
        DraftListCompletion::Pending { .. }
    ));
    only_membership_changes(before, &after, &[0]);
    origin_integrity_with_retired(&source, &after, 1);
}

#[test]
fn empty_archived_and_multiple_root_presets_have_independent_exact_inventories() {
    let artifacts = artifacts(true);
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill>{ACTIVE}{ACTIVE}{SUPPORT}</Skill></SkillSet><SkillSet id="2"/><SkillSet id="3"><Skill>{ACTIVE}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let source = source(&text, 0xe2);
    let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
    let after = normalize(
        &source,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    assert_eq!(completions(&after), [true, true, true]);
    assert_eq!(
        after
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .map(|p| p.skills.members.len())
            .collect::<Vec<_>>(),
        [2, 0, 1]
    );
    only_membership_changes(before, &after, &[0, 1, 2]);
    origin_integrity_with_retired(&source, &after, 3);
}

#[test]
fn unknown_or_unadmitted_manual_sources_cannot_hide_authored_roots() {
    for provider_only in [false, true] {
        let mut artifacts = artifacts(true);
        if provider_only {
            replace_materialization(
                &mut artifacts,
                "active",
                OwnedGemMaterialization::ProviderOnly,
            );
        }
        let row = if provider_only {
            ACTIVE.to_owned()
        } else {
            ACTIVE.replace("gemId=\"active\"", "gemId=\"unreviewed\"")
        };
        let text = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill>{row}{SUPPORT}</Skill></SkillSet><SkillSet id="2"/></Skills></PathOfBuilding2>"#
        );
        let source = source(&text, 0xe3);
        let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
        let after = normalize(
            &source,
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        assert_eq!(completions(&after), [false, true]);
        only_membership_changes(before, &after, &[1]);
    }
}

#[test]
fn present_empty_or_unknown_source_never_claims_manual_membership() {
    let artifacts = artifacts(true);
    for origin in ["", "nil", "unreviewed", "Tree:7", "Item:7:unknown"] {
        let text = xml(&format!(r#"<Skill source="{origin}">{ACTIVE}</Skill>"#));
        let source = source(&text, 0xe4);
        let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
        let after = normalize(
            &source,
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        assert_eq!(completions(&after), [false], "{origin:?}");
        only_membership_changes(before, &after, &[]);
    }
}

#[test]
fn unreviewed_rows_and_ambiguous_container_keys_remain_pending() {
    let artifacts = artifacts(true);
    let valid = xml(&format!("<Skill>{ACTIVE}</Skill>"));
    for text in [
        valid.replace("<Skill>", "<Skill unreviewed=\"true\">"),
        valid.replace("<Gem ", "<Gem unreviewed=\"true\" "),
        valid.replace("</Skill>", "<Unexpected/></Skill>"),
        valid.replace("</SkillSet>", "<Unexpected/></SkillSet>"),
        valid.replace("</Skills>", "<SkillSet id=\"1\"/></Skills>"),
        valid.replace("</Skills>", "<SkillSet id=\"01\"/></Skills>"),
        valid.replace("<SkillSet id=\"1\">", "<SkillSet>"),
        valid.replace("<SkillSet id=\"1\">", "<SkillSet id=\"0\">"),
        valid.replace(
            "<Skills activeSkillSet=\"1\">",
            "<Skills activeSkillSet=\"99\">",
        ),
        valid.replace("<Skill>", "<Skill xmlns=\"unreviewed\">"),
    ] {
        let source = source(&text, 0xe5);
        let before = normalize(&source, &artifacts, &policy(), Default::default()).unwrap();
        let after = normalize(
            &source,
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        assert!(completions(&after).iter().all(|v| !v), "{text}");
        only_membership_changes(before, &after, &[]);
    }
}

#[test]
fn missing_policy_preserves_wire_and_stale_bindings_reject_before_materialization() {
    let artifacts = artifacts(true);
    let text = xml(&format!("<Skill>{ACTIVE}</Skill>"));
    let source = source(&text, 0xe6);
    let bytes = serde_json::to_vec(&policy()).unwrap();
    assert!(!String::from_utf8_lossy(&bytes).contains("skill_inventory"));
    let mut null = serde_json::to_value(policy()).unwrap();
    null["skill_inventory"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<NormalizationPolicy>(null).is_err());
    for field in 0..3 {
        let mut configured = reviewed(&artifacts);
        let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
            mapping_source,
            roles,
            direct_inputs,
            ..
        } = configured.skill_inventory.as_mut().unwrap();
        let wrong = "0".repeat(64).parse().unwrap();
        match field {
            0 => *mapping_source = wrong,
            1 => *roles = wrong,
            _ => *direct_inputs = Some(wrong),
        }
        assert!(matches!(
            normalize(&source, &artifacts, &configured, Default::default()),
            Err(NormalizationError::Binding)
        ));
    }
    assert_eq!(bytes, serde_json::to_vec(&policy()).unwrap());
}

#[test]
fn inventory_work_is_bounded_and_failed_attempt_leaves_no_state() {
    let artifacts = artifacts(true);
    let text = xml(&format!("<Skill>{ACTIVE}{ACTIVE}{SUPPORT}</Skill>"));
    let source = source(&text, 0xe7);
    let configured = reviewed(&artifacts);
    let expected = normalize(&source, &artifacts, &configured, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        let result = normalize(
            &source,
            &artifacts,
            &configured,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        );
        if result.is_ok() {
            high = middle;
        } else {
            assert!(matches!(result, Err(NormalizationError::Limit(_))));
            low = middle + 1;
        }
    }
    let before = *source.allocator_state();
    assert!(matches!(
        normalize(
            &source,
            &artifacts,
            &configured,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let retry = normalize(
        &source,
        &artifacts,
        &configured,
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(source.allocator_state(), &before);
    assert_eq!(retry.draft(), expected.draft());
    assert_eq!(
        serde_json::to_vec(retry.sidecar()).unwrap(),
        serde_json::to_vec(expected.sidecar()).unwrap()
    );
}

#[test]
fn duplicate_attributes_are_rejected_before_inventory_normalization() {
    let text = xml(&format!("<Skill>{ACTIVE}</Skill>")).replace("<Gem ", "<Gem variantId=\"v\" ");
    assert!(decode_build(text.as_bytes()).is_err());
}
