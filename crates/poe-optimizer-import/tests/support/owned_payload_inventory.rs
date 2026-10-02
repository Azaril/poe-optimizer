use super::*;
const EFFECT: &str =
    r#"<Gem skillId="InjectedEffect" nameSpec="misleading support name" level="1" quality="0"/>"#;

fn payload(a: &Artifacts) -> NormalizationPolicy {
    let mut p = policy();
    p.payload_inventory = Some(
        PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
            mapping_source: *a.mapping.source_identity(),
            roles: *a.roles.identity(),
            non_container_gems: a
                .roles
                .input()
                .roles
                .iter()
                .map(|r| r.gem.clone())
                .collect(),
            nonphysical_non_container_skill_ids: vec!["InjectedEffect".into()],
        },
    );
    p
}
fn payload_complete(result: &NormalizedImport) {
    let preset = &result.draft().input().skill_presets.members[0];
    assert!(preset.payload_links.members.is_empty());
    assert_eq!(
        preset.payload_links.completion,
        DraftListCompletion::Complete
    );
}
fn payload_pending(result: &NormalizedImport) {
    assert!(
        result
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .all(|p| {
                matches!(&p.payload_links.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "payload-membership-not-converted")
            })
    );
}
fn only_payload_retired(before: NormalizedImport, after: &NormalizedImport) {
    let mut expected = before.draft().input().clone();
    let DraftListCompletion::Pending { id: retired, .. } =
        expected.skill_presets.members[0].payload_links.completion
    else {
        panic!("prior payload gate");
    };
    expected.skill_presets.members[0].payload_links.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut sidecar = before.into_parts().1;
    sidecar.policy = after.sidecar().policy;
    sidecar.draft = after.sidecar().draft;
    let mut removed = 0;
    for origin in &mut sidecar.origins {
        origin.links.retain(|link| {
            if matches!(link, OwnedOriginTarget::Issue(id) if *id == retired) {
                removed += 1;
                false
            } else {
                true
            }
        });
    }
    assert_eq!(removed, 1);
    assert_eq!(
        serde_json::to_value(sidecar).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
}

#[test]
fn empty_payload_inventory_is_independent_and_preserves_every_other_fact() {
    let a = artifacts(true);
    let imported = source(&xml(&format!("{ACTIVE}{SUPPORT}")), 0xd1);
    for support_enabled in [false, true] {
        let before_policy = if support_enabled {
            reviewed(&a)
        } else {
            policy()
        };
        let mut after_policy = before_policy.clone();
        after_policy.payload_inventory = payload(&a).payload_inventory;
        let before = normalize(&imported, &a, &before_policy, Default::default()).unwrap();
        let after = normalize(&imported, &a, &after_policy, Default::default()).unwrap();
        payload_complete(&after);
        only_payload_retired(before, &after);
        if !support_enabled {
            pending(&after);
            assert!(
                after.draft().input().skill_presets.members[0]
                    .support_origins
                    .is_none()
            );
        }
    }
    let old = policy();
    let encoded = serde_json::to_value(&old).unwrap();
    assert!(encoded.get("payload_inventory").is_none());
    assert_eq!(
        serde_json::from_value::<NormalizationPolicy>(encoded).unwrap(),
        old
    );
}

#[test]
fn noncontainer_proof_does_not_materialize_provider_inputs_or_targets() {
    let mut a = artifacts(true);
    replace_materialization(&mut a, "active", OwnedGemMaterialization::ProviderOnly);
    let p = payload(&a);
    let imported = source(&xml(&format!("{ACTIVE}{SUPPORT}")), 0xd2);
    let before = normalize(&imported, &a, &policy(), Default::default()).unwrap();
    let after = normalize(&imported, &a, &p, Default::default()).unwrap();
    payload_complete(&after);
    assert!(after.draft().input().skills.members.is_empty());
    assert_eq!(after.draft().input().gems.members.len(), 1);
    assert!(matches!(
        after.draft().input().supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    only_payload_retired(before, &after);
}

#[test]
fn every_row_must_be_classified_and_multiple_non_supports_remain_pending() {
    let a = artifacts(true);
    let p = payload(&a);
    for rows in [
        format!("{ACTIVE}{ACTIVE}{SUPPORT}"),
        format!("{ACTIVE}{EFFECT}"),
        format!("{EFFECT}{EFFECT}"),
        ACTIVE.replace("gemId=\"active\"", "gemId=\"unknown\""),
        ACTIVE.replace("variantId=\"v\"", "variantId=\"unknown\""),
        ACTIVE.replace("variantId=\"v\"", ""),
        EFFECT.replace("InjectedEffect", "UnknownEffect"),
    ] {
        payload_pending(
            &normalize(&source(&xml(&rows), 0xd3), &a, &p, Default::default()).unwrap(),
        );
    }
    let mut excludes_container = p.clone();
    let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        non_container_gems,
        ..
    }) = &mut excludes_container.payload_inventory
    else {
        panic!();
    };
    let active = a
        .roles
        .input()
        .roles
        .iter()
        .find(|r| matches!(r.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse)))
        .unwrap();
    non_container_gems.retain(|g| g != &active.gem);
    payload_pending(
        &normalize(
            &source(&xml(ACTIVE), 0xd4),
            &a,
            &excludes_container,
            Default::default(),
        )
        .unwrap(),
    );
}

#[test]
fn skill_id_nonphysical_lane_obeys_exact_selector_precedence() {
    let a = artifacts(true);
    let p = payload(&a);
    payload_complete(&normalize(&source(&xml(EFFECT), 0xd5), &a, &p, Default::default()).unwrap());
    for row in [
        EFFECT.replace("skillId=", "gemId=\"\" skillId="),
        EFFECT.replace("skillId=", "gemId=\"invalid\" skillId="),
        EFFECT.replace("skillId=", "gemId=\"active\" skillId="),
        EFFECT.replace("skillId=\"InjectedEffect\"", ""),
        EFFECT.replace("InjectedEffect", " InjectedEffect"),
    ] {
        payload_pending(&normalize(&source(&xml(&row), 0xd6), &a, &p, Default::default()).unwrap());
    }
    let physical = SUPPORT.replace("<Gem ", "<Gem skillId=\"UnknownEffect\" ");
    let result = normalize(&source(&xml(&physical), 0xd7), &a, &p, Default::default()).unwrap();
    payload_complete(&result);
    assert_eq!(result.draft().input().supports.members.len(), 1);
}

#[test]
fn all_skillset_frames_are_proved_before_any_payload_gate_is_retired() {
    let a = artifacts(true);
    let p = payload(&a);
    let base = xml(&format!("{ACTIVE}{SUPPORT}"));
    for invalid in [
        base.replace("<Gem ", "<Gem future=\"x\" "),
        base.replace("<Gem ", "<Gem xmlns:q=\"future\" "),
        base.replace("</Skill>", "<Future/></Skill>"),
        base.replace("</Skills>", "<SkillSet id=\"7\"/></Skills>"),
        base.replace(
            "</Skills>",
            "<SkillSet id=\"8\"><Skill><Future/></Skill></SkillSet></Skills>",
        ),
        base.replace("activeSkillSet=\"7\"", "activeSkillSet=\"9\""),
    ] {
        payload_pending(&normalize(&source(&invalid, 0xd8), &a, &p, Default::default()).unwrap());
    }
    let separate = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill><Gem skillId="UnknownEffect"/></Skill></SkillSet><SkillSet id="2"><Skill>{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = normalize(&source(&separate, 0xd9), &a, &p, Default::default()).unwrap();
    assert!(matches!(
        result.draft().input().skill_presets.members[0]
            .payload_links
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_eq!(
        result.draft().input().skill_presets.members[1]
            .payload_links
            .completion,
        DraftListCompletion::Complete
    );
}

#[test]
fn payload_policy_validates_bindings_domains_duplicates_and_bounds_even_empty() {
    let a = artifacts(true);
    let imported = source(&xml(ACTIVE), 0xda);
    for empty in [false, true] {
        for stale_roles in [false, true] {
            let mut p = payload(&a);
            let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
                mapping_source,
                roles,
                non_container_gems,
                nonphysical_non_container_skill_ids,
            }) = &mut p.payload_inventory
            else {
                panic!();
            };
            if empty {
                non_container_gems.clear();
                nonphysical_non_container_skill_ids.clear();
            }
            if stale_roles {
                *roles = *a.mapping.identity();
            } else {
                *mapping_source = *a.mapping.identity();
            }
            assert!(matches!(
                normalize(&imported, &a, &p, Default::default()),
                Err(NormalizationError::Binding)
            ));
        }
    }
    for invalid_ids in [
        vec![""],
        vec!["InjectedEffect", "InjectedEffect"],
        vec![" InjectedEffect"],
        vec!["Injected\nEffect"],
    ] {
        let mut p = payload(&a);
        let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
            nonphysical_non_container_skill_ids,
            ..
        }) = &mut p.payload_inventory
        else {
            panic!();
        };
        *nonphysical_non_container_skill_ids = invalid_ids.into_iter().map(str::to_owned).collect();
        assert!(matches!(
            normalize(&imported, &a, &p, Default::default()),
            Err(NormalizationError::Policy(_))
        ));
    }
    for unknown in [false, true] {
        let mut p = payload(&a);
        let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
            non_container_gems,
            ..
        }) = &mut p.payload_inventory
        else {
            panic!();
        };
        if unknown {
            let mut registry = a.registry.clone();
            non_container_gems.push(registry.allocate_definition::<GemDefinition>().unwrap());
        } else {
            non_container_gems.push(non_container_gems[0].clone());
        }
        assert!(matches!(
            normalize(&imported, &a, &p, Default::default()),
            Err(NormalizationError::Policy(_))
        ));
    }
    let mut limits = NormalizationLimits::default();
    limits.draft.input.max_collection_entries = 1;
    assert!(matches!(
        normalize(&imported, &a, &payload(&a), limits),
        Err(NormalizationError::Limit(_))
    ));

    let mut p = payload(&a);
    let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        nonphysical_non_container_skill_ids,
        ..
    }) = &mut p.payload_inventory
    else {
        panic!();
    };
    nonphysical_non_container_skill_ids.push("AnotherEffect".into());
    let mut limits = NormalizationLimits::default();
    limits.value.max_selectors = 1;
    assert!(matches!(
        normalize(&imported, &a, &p, limits),
        Err(NormalizationError::Limit(
            "payload inventory skill selectors"
        ))
    ));

    let mut p = payload(&a);
    let Some(PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
        non_container_gems,
        nonphysical_non_container_skill_ids,
        ..
    }) = &mut p.payload_inventory
    else {
        panic!();
    };
    let active = a
        .roles
        .input()
        .roles
        .iter()
        .find(|r| matches!(r.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse)))
        .unwrap();
    non_container_gems.retain(|gem| gem == &active.gem);
    nonphysical_non_container_skill_ids.clear();
    let mut limits = NormalizationLimits::default();
    limits.draft.input.max_collection_entries = 2;
    let imported = source(&xml(&format!("{ACTIVE}{ACTIVE}{ACTIVE}")), 0xdb);
    assert!(matches!(
        normalize(&imported, &a, &p, limits),
        Err(NormalizationError::Limit("payload inventory source rows"))
    ));
}
