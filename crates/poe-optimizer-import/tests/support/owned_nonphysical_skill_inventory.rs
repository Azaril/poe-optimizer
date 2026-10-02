//! Exact saved effect IDs establish zero physical assignments, never effect closure.
use super::*;

const EFFECT: &str =
    r#"<Gem skillId="InjectedEffect" nameSpec="misleading support name" level="1" quality="0"/>"#;

fn v3(artifacts: &Artifacts, ids: &[&str]) -> NormalizationPolicy {
    let mut policy = reviewed(artifacts);
    policy.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
            mapping_source: *artifacts.mapping.source_identity(),
            roles: *artifacts.roles.identity(),
            nonphysical_skill_ids: ids.iter().map(|id| (*id).into()).collect(),
        },
    );
    policy
}

fn assert_only_inventory_retired(before: NormalizedImport, after: &NormalizedImport) {
    let mut expected = before.draft().input().clone();
    let DraftListCompletion::Pending { id: retired, .. } =
        expected.skill_presets.members[0].supports.completion
    else {
        panic!("prior inventory obligation")
    };
    expected.skill_presets.members[0].supports.completion = DraftListCompletion::Complete;
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
fn injected_nonphysical_skill_row_retires_only_physical_inventory() {
    let artifacts = artifacts(true);
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill>{ACTIVE}{SUPPORT}</Skill><Skill source="unreviewed-provider">{EFFECT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let imported = source(&xml, 0xc1);
    let before = normalize(
        &imported,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    pending(&before);
    let after = normalize(
        &imported,
        &artifacts,
        &v3(&artifacts, &["InjectedEffect"]),
        Default::default(),
    )
    .unwrap();
    assert_eq!(after.draft().input().gems.members.len(), 2);
    assert_eq!(after.draft().input().skills.members.len(), 1);
    assert_eq!(after.draft().input().supports.members.len(), 1);
    let preset = &after.draft().input().skill_presets.members[0];
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.payload_links.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.support_origins.as_ref().unwrap().completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_only_inventory_retired(before, &after);
    origin_integrity_with_retired(&imported, &after, 1);
}

#[test]
fn nonphysical_skill_ids_do_not_override_present_gem_ids_or_guess_names() {
    let artifacts = artifacts(true);
    let policy = v3(&artifacts, &["InjectedEffect"]);
    for row in [
        EFFECT.replace("skillId=", "gemId=\"\" skillId="),
        EFFECT.replace("skillId=", "gemId=\"unknown\" skillId="),
        EFFECT.replace("skillId=", "gemId=\"support\" skillId="),
        EFFECT.replace("InjectedEffect", "UnknownEffect"),
        EFFECT.replace("InjectedEffect", " InjectedEffect"),
        EFFECT.replace("InjectedEffect", ""),
        EFFECT.replace("skillId=\"InjectedEffect\"", ""),
    ] {
        let result = normalize(
            &source(&xml(&row), 0xc2),
            &artifacts,
            &policy,
            Default::default(),
        )
        .unwrap();
        pending(&result);
        assert!(result.draft().input().supports.members.is_empty());
    }
    // A valid physical selector wins even with a declared nonphysical skillId.
    let row = SUPPORT.replace("<Gem ", "<Gem skillId=\"InjectedEffect\" ");
    let imported = source(&xml(&row), 0xc3);
    let old = normalize(
        &imported,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    let new = normalize(&imported, &artifacts, &policy, Default::default()).unwrap();
    assert_eq!(old.draft(), new.draft());
    assert_eq!(new.draft().input().gems.members.len(), 1);
    assert_eq!(new.draft().input().supports.members.len(), 1);
}

#[test]
fn appended_physical_supports_require_their_own_origin_and_exact_materialization() {
    let artifacts = artifacts(true);
    let policy = v3(&artifacts, &["InjectedEffect"]);
    for (origin, accepted) in [("", true), (" source=\"unreviewed-provider\"", false)] {
        let xml = format!(
            r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill{origin}>{EFFECT}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let result =
            normalize(&source(&xml, 0xc4), &artifacts, &policy, Default::default()).unwrap();
        if accepted {
            assert_eq!(result.draft().input().supports.members.len(), 1);
            assert!(matches!(
                result.draft().input().supports.members[0].target,
                DraftSkillTarget::Pending(_)
            ));
            assert_eq!(
                result.draft().input().skill_presets.members[0]
                    .supports
                    .completion,
                DraftListCompletion::Complete
            );
        } else {
            pending(&result);
            assert!(result.draft().input().supports.members.is_empty());
        }
    }
}

#[test]
fn nonphysical_census_keeps_unknown_rows_local_but_rejects_ambiguous_frames() {
    let artifacts = artifacts(true);
    let policy = v3(&artifacts, &["InjectedEffect"]);
    let two_sets = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill>{EFFECT}{EFFECT}</Skill></SkillSet><SkillSet id="2"><Skill><Gem skillId="UnknownEffect"/></Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = normalize(
        &source(&two_sets, 0xc5),
        &artifacts,
        &policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        result.draft().input().skill_presets.members[0]
            .supports
            .completion,
        DraftListCompletion::Complete
    );
    assert!(matches!(
        result.draft().input().skill_presets.members[1]
            .supports
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(result.draft().input().gems.members.is_empty());
    assert!(result.draft().input().skills.members.is_empty());
    for invalid in [
        xml(EFFECT).replace("<Gem ", "<Gem future=\"x\" "),
        xml(EFFECT).replace(EFFECT, &EFFECT.replace("/>", "><Future/></Gem>")),
        xml(EFFECT).replace("<Gem ", "<Gem xmlns:q=\"future\" "),
        xml(EFFECT).replace("</Skill>", "<Future/></Skill>"),
        xml(EFFECT).replace("activeSkillSet=\"7\"", "activeSkillSet=\"8\""),
        two_sets.replace("id=\"2\"", "id=\"1\""),
    ] {
        pending(
            &normalize(
                &source(&invalid, 0xc6),
                &artifacts,
                &policy,
                Default::default(),
            )
            .unwrap(),
        );
    }
}

#[test]
fn nonphysical_policy_checks_bindings_even_empty_and_rejects_ambiguous_ids() {
    let artifacts = artifacts(true);
    let imported = source(&xml(EFFECT), 0xc7);
    for ids in [vec![], vec!["InjectedEffect"]] {
        for role_binding in [false, true] {
            let mut policy = v3(&artifacts, &ids);
            let Some(
                SupportOriginOrderPolicy::SavedManualGroupOrderWithNonphysicalSkillInventoryV3 {
                    mapping_source,
                    roles,
                    ..
                },
            ) = &mut policy.support_origin_order
            else {
                panic!()
            };
            let invalid = *artifacts.mapping.identity();
            if role_binding {
                *roles = invalid;
            } else {
                *mapping_source = invalid;
            }
            assert!(matches!(
                normalize(&imported, &artifacts, &policy, Default::default()),
                Err(NormalizationError::Binding)
            ));
        }
    }
    for ids in [
        vec![""],
        vec!["InjectedEffect", "InjectedEffect"],
        vec![" InjectedEffect"],
        vec!["InjectedEffect\t"],
        vec!["Injected\nEffect"],
        vec!["\u{a0}InjectedEffect"],
    ] {
        assert!(matches!(
            normalize(
                &imported,
                &artifacts,
                &v3(&artifacts, &ids),
                Default::default()
            ),
            Err(NormalizationError::Policy(_))
        ));
    }
    let long = "x".repeat(129);
    assert!(matches!(
        normalize(
            &imported,
            &artifacts,
            &v3(&artifacts, &[&long]),
            Default::default()
        ),
        Err(NormalizationError::Limit(_))
    ));
    let ids = (0..65).map(|i| format!("Effect{i}")).collect::<Vec<_>>();
    let refs = ids.iter().map(String::as_str).collect::<Vec<_>>();
    assert!(matches!(
        normalize(
            &imported,
            &artifacts,
            &v3(&artifacts, &refs),
            Default::default()
        ),
        Err(NormalizationError::Limit(_))
    ));
    for update in 0..3 {
        let mut limits = NormalizationLimits::default();
        match update {
            0 => limits.value.max_selector_bytes = 3,
            1 => limits.value.max_total_selector_bytes = 3,
            2 => limits.max_work = 1,
            _ => unreachable!(),
        }
        assert!(
            normalize(
                &imported,
                &artifacts,
                &v3(&artifacts, &["InjectedEffect"]),
                limits
            )
            .is_err()
        );
    }
}

#[test]
fn empty_nonphysical_domain_keeps_v2_facts_and_prior_wire_behavior() {
    let artifacts = artifacts(true);
    let v2 = reviewed(&artifacts);
    let wire = serde_json::to_value(&v2).unwrap();
    assert_eq!(
        wire["support_origin_order"]["kind"],
        "saved_manual_group_order_with_physical_inventory_v2"
    );
    assert!(
        wire["support_origin_order"]
            .get("nonphysical_skill_ids")
            .is_none()
    );
    for gems in [format!("{ACTIVE}{SUPPORT}"), EFFECT.into()] {
        let imported = source(&xml(&gems), 0xc8);
        let before = normalize(&imported, &artifacts, &v2, Default::default()).unwrap();
        let after = normalize(
            &imported,
            &artifacts,
            &v3(&artifacts, &[]),
            Default::default(),
        )
        .unwrap();
        assert_eq!(before.draft(), after.draft());
        assert_eq!(before.allocator_after(), after.allocator_after());
        let mut actual = serde_json::to_value(after.sidecar()).unwrap();
        let expected = serde_json::to_value(before.sidecar()).unwrap();
        actual["policy"] = expected["policy"].clone();
        assert_eq!(actual, expected);
    }
    assert_eq!(serde_json::to_value(&v2).unwrap(), wire);
}

#[test]
fn nonphysical_census_reuses_physical_selector_work_under_a_tight_budget() {
    let artifacts = artifacts(true);
    let imported = source(&xml(&format!("{ACTIVE}{}", SUPPORT.repeat(128))), 0xc9);
    let v2 = reviewed(&artifacts);
    let baseline = normalize(&imported, &artifacts, &v2, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: mid,
            ..Default::default()
        };
        match normalize(&imported, &artifacts, &v2, limits) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(error) => panic!("unexpected budget rejection: {error}"),
        }
    }
    // One short, unused declaration needs bounded cold-policy work, not another
    // census of all 129 already reviewed physical rows and their attributes.
    let limits = NormalizationLimits {
        max_work: low + 64,
        ..Default::default()
    };
    let after = normalize(
        &imported,
        &artifacts,
        &v3(&artifacts, &["InjectedEffect"]),
        limits,
    )
    .unwrap();
    assert_eq!(baseline.draft(), after.draft());
    assert_eq!(baseline.allocator_after(), after.allocator_after());
}
