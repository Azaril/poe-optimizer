//! Physical inventory is independent of target/effect/readiness obligations.
use super::*;
#[path = "owned_payload_inventory.rs"]
mod payload_inventory_tests;

#[path = "owned_nonphysical_skill_inventory.rs"]
mod nonphysical_skill_inventory_tests;

fn reviewed(artifacts: &Artifacts) -> NormalizationPolicy {
    let mut policy = policy();
    policy.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source: *artifacts.mapping.source_identity(),
            roles: *artifacts.roles.identity(),
        },
    );
    policy
}

fn xml(gems: &str) -> String {
    group(gems).replace("<Skills>", "<Skills activeSkillSet=\"7\">")
}

fn normalize(
    source: &ImportedBuildInstance,
    artifacts: &Artifacts,
    policy: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> std::result::Result<NormalizedImport, NormalizationError> {
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

fn pending(result: &NormalizedImport) {
    assert!(
        result
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .all(|preset| {
                matches!(&preset.supports.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "support-membership-not-converted")
            })
    );
}

#[test]
fn exact_physical_inventory_and_authored_order_retain_spent_identities() {
    let artifacts = artifacts(true);
    let source = source(
        &xml(&format!(
            "{ACTIVE}{}{SUPPORT}",
            SUPPORT.replace("enabled=\"true\"", "enabled=\"false\"")
        )),
        0xb1,
    );
    let mut old = reviewed(&artifacts);
    old.support_origin_order = Some(SupportOriginOrderPolicy::SavedManualGroupOrder {});
    let before = normalize(&source, &artifacts, &old, Default::default()).unwrap();
    let after = normalize(
        &source,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    let mut expected = before.draft().input().clone();
    let DraftListCompletion::Pending { id: retired, .. } =
        expected.skill_presets.members[0].supports.completion
    else {
        panic!("prior obligation")
    };
    expected.skill_presets.members[0].supports.completion = DraftListCompletion::Complete;
    let order = expected.skill_presets.members[0]
        .authored_support_order
        .as_mut()
        .unwrap();
    let DraftListCompletion::Pending {
        id: order_issue, ..
    } = order.completion
    else {
        panic!("prior authored-order obligation")
    };
    order.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut sidecar = before.into_parts().1;
    sidecar.policy = after.sidecar().policy;
    sidecar.draft = after.sidecar().draft;
    let mut removed = 0;
    for origin in &mut sidecar.origins {
        origin.links.retain(|link| {
            if matches!(link, OwnedOriginTarget::Issue(id) if *id == retired || *id == order_issue)
            {
                removed += 1;
                false
            } else {
                true
            }
        });
    }
    assert_eq!(removed, 2);
    assert_eq!(
        serde_json::to_value(sidecar).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    let input = after.draft().input();
    let preset = &input.skill_presets.members[0];
    assert_eq!(
        preset.supports.members,
        input
            .supports
            .members
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>()
    );
    assert_eq!(input.supports.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(input.supports.members[1].enabled.to_resolved(), Some(true));
    assert!(matches!(
        preset.authored_support_order.as_ref().unwrap().completion,
        DraftListCompletion::Complete
    ));
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.payload_links.completion,
        DraftListCompletion::Pending { .. }
    ));
    origin_integrity_with_retired(&source, &after, 2);
}

#[test]
fn provider_only_and_multiple_active_roots_do_not_require_resolved_support_targets() {
    for provider_only in [true, false] {
        let mut artifacts = artifacts(true);
        let gems = if provider_only {
            replace_materialization(
                &mut artifacts,
                "active",
                OwnedGemMaterialization::ProviderOnly,
            );
            format!("{ACTIVE}{SUPPORT}{SUPPORT}")
        } else {
            format!("{ACTIVE}{SUPPORT}{ACTIVE}{SUPPORT}")
        };
        let source = source(&xml(&gems), 0xb2);
        let after = normalize(
            &source,
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        let input = after.draft().input();
        assert_eq!(input.supports.members.len(), 2);
        assert_eq!(
            input.skills.members.len(),
            if provider_only { 0 } else { 2 }
        );
        assert!(
            input
                .supports
                .members
                .iter()
                .all(|s| matches!(s.target, DraftSkillTarget::Pending(_)))
        );
        let preset = &input.skill_presets.members[0];
        assert_eq!(preset.supports.completion, DraftListCompletion::Complete);
        assert!(matches!(
            preset.authored_support_order.as_ref().unwrap().completion,
            DraftListCompletion::Pending { .. }
        ));
        origin_integrity_with_retired(&source, &after, 1);
    }
}

#[test]
fn inventories_are_ordered_per_saved_set_and_allow_reviewed_empty_inventory() {
    let artifacts = artifacts(true);
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill source="Tree:123">{ACTIVE}{SUPPORT}{SUPPORT}</Skill></SkillSet><SkillSet id="2"><Skill>{ACTIVE}</Skill></SkillSet><SkillSet id="3"/></Skills></PathOfBuilding2>"#
    );
    let source = source(&xml, 0xb3);
    let result = normalize(
        &source,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    let input = result.draft().input();
    assert_eq!(input.skill_presets.members.len(), 3);
    assert_eq!(input.supports.members.len(), 2);
    assert!(
        input
            .supports
            .members
            .iter()
            .all(|s| matches!(s.target, DraftSkillTarget::Pending(_)))
    );
    for (preset, count) in input.skill_presets.members.iter().zip([2, 0, 0]) {
        assert_eq!(preset.supports.completion, DraftListCompletion::Complete);
        assert_eq!(preset.supports.members.len(), count);
    }
    assert_eq!(
        input.skill_presets.members[0].supports.members,
        input
            .supports
            .members
            .iter()
            .map(|s| s.id)
            .collect::<Vec<_>>()
    );
    assert!(matches!(
        input.skill_presets.members[0]
            .authored_support_order
            .as_ref()
            .unwrap()
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    for preset in &input.skill_presets.members[1..] {
        assert_eq!(
            preset.authored_support_order.as_ref().unwrap().completion,
            DraftListCompletion::Complete
        );
    }
    origin_integrity_with_retired(&source, &result, 5);
}

#[test]
fn authored_order_refuses_unrepresented_slot_relations_even_without_enabled_supports() {
    let artifacts = artifacts(true);
    for slot in ["", "nil", "Weapon 1", "future-slot"] {
        for contents in [format!("{ACTIVE}{SUPPORT}"), ACTIVE.into(), String::new()] {
            let text = xml(&contents).replace(
                "<Skill enabled=\"true\">",
                &format!("<Skill enabled=\"false\" slot=\"{slot}\">"),
            );
            let imported = source(&text, 0xd1);
            let result = normalize(
                &imported,
                &artifacts,
                &reviewed(&artifacts),
                Default::default(),
            )
            .unwrap();
            let preset = &result.draft().input().skill_presets.members[0];
            assert_eq!(preset.supports.completion, DraftListCompletion::Complete);
            assert!(
                matches!(
                    preset.authored_support_order.as_ref().unwrap().completion,
                    DraftListCompletion::Pending { .. }
                ),
                "slot={slot:?}"
            );
            origin_integrity_with_retired(&imported, &result, 1);
        }
    }
}

#[test]
fn authored_relationships_are_proved_per_preset_without_erasing_unresolved_siblings() {
    let artifacts = artifacts(true);
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="false">{ACTIVE}{SUPPORT}{SUPPORT}</Skill></SkillSet><SkillSet id="2"><Skill slot="Weapon 1">{ACTIVE}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let imported = source(&text, 0xd2);
    let result = normalize(
        &imported,
        &artifacts,
        &reviewed(&artifacts),
        Default::default(),
    )
    .unwrap();
    let presets = &result.draft().input().skill_presets.members;
    let first = presets[0]
        .authored_support_order
        .as_ref()
        .unwrap()
        .to_resolved()
        .unwrap();
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].assignments, presets[0].supports.members);
    assert!(matches!(
        presets[1]
            .authored_support_order
            .as_ref()
            .unwrap()
            .completion,
        DraftListCompletion::Pending { .. }
    ));
    origin_integrity_with_retired(&imported, &result, 3);
}

#[test]
fn authored_empty_order_needs_a_complete_source_and_physical_census() {
    let artifacts = artifacts(true);
    for (text, known) in [
        (xml(""), true),
        (xml(ACTIVE), true),
        (xml("<Gem gemId=\"unknown\"/>"), false),
        (xml(ACTIVE).replace("</Skill>", "<Unknown/></Skill>"), false),
        (
            xml(ACTIVE).replace(
                "<Skill enabled=\"true\">",
                "<Skill enabled=\"true\" source=\"\">",
            ),
            false,
        ),
        (
            xml(ACTIVE).replace(
                "<Skill enabled=\"true\">",
                "<Skill enabled=\"true\" source=\"nil\">",
            ),
            false,
        ),
    ] {
        let result = normalize(
            &source(&text, 0xd3),
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        let order = result.draft().input().skill_presets.members[0]
            .authored_support_order
            .as_ref()
            .unwrap();
        assert_eq!(
            matches!(order.completion, DraftListCompletion::Complete),
            known,
            "{text}"
        );
        assert!(order.members.is_empty());
    }
}

#[test]
fn known_generated_active_roots_do_not_require_physical_support_origin_admission() {
    let mut artifacts = artifacts(true);
    add_active_sibling(&mut artifacts, OwnedGemMaterialization::ProviderOnly);
    let mut policy = reviewed(&artifacts);
    policy.generated_support_prefixes.clear();
    let provider = ACTIVE.replace("gemId=\"active\"", "gemId=\"sibling\"");
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill>{ACTIVE}{SUPPORT}</Skill><Skill source="Tree:123">{provider}</Skill><Skill source="Item:7:unknown provider">{ACTIVE}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let imported = source(&xml, 0xbb);
    let result = normalize(&imported, &artifacts, &policy, Default::default()).unwrap();
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 1);
    assert_eq!(draft.gems.members.len(), 2);
    assert_eq!(draft.supports.members.len(), 1);
    let preset = &draft.skill_presets.members[0];
    assert_eq!(preset.supports.completion, DraftListCompletion::Complete);
    assert!(matches!(
        preset.authored_support_order.as_ref().unwrap().completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    origin_integrity_with_retired(&imported, &result, 1);

    // The same non-admitted origin cannot hide a real physical support row.
    let physical_support = xml.replace(&provider, &format!("{provider}{SUPPORT}"));
    let result = normalize(
        &source(&physical_support, 0xbc),
        &artifacts,
        &policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(result.draft().input().supports.members.len(), 1);
    pending(&result);
    let unknown_role = xml.replace(
        &provider,
        &provider.replace("gemId=\"sibling\"", "gemId=\"unknown\""),
    );
    pending(
        &normalize(
            &source(&unknown_role, 0xbd),
            &artifacts,
            &policy,
            Default::default(),
        )
        .unwrap(),
    );
}

#[test]
fn unknown_roles_identities_materialization_or_group_sources_keep_inventory_pending() {
    let base = xml(&format!("{ACTIVE}{SUPPORT}"));
    for invalid in [
        base.replace("gemId=\"active\"", "gemId=\"unknown\""),
        base.replace("gemId=\"support\"", "gemId=\"unknown\""),
        base.replace("variantId=\"v\"", "variantId=\"unknown\""),
        base.replace("variantId=\"v\"", ""),
        base.replace("gemId=\"active\"", "gemId=\"\""),
        base.replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" source=\"unreviewed:123\">",
        ),
    ] {
        let artifacts = artifacts(true);
        let result = normalize(
            &source(&invalid, 0xb4),
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap();
        pending(&result);
    }
    let mut artifacts = artifacts(true);
    replace_materialization(
        &mut artifacts,
        "active",
        OwnedGemMaterialization::Unmapped {
            issue: key("unknown-materialization"),
        },
    );
    pending(
        &normalize(
            &source(&base, 0xb5),
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap(),
    );
    let mut artifacts = super::artifacts(true);
    let mut role_input = artifacts.roles.input().clone();
    role_input
        .roles
        .iter_mut()
        .find(|row| matches!(row.role, OwnedGemRole::Known(AuthoredGemRole::SkillUse)))
        .unwrap()
        .role = OwnedGemRole::Unmapped {
        issue: key("unknown-role"),
    };
    artifacts.roles = OwnedSkillRoleIndex::new(
        role_input,
        &artifacts.mapping,
        &artifacts.schema,
        SkillCatalogLimits::default(),
    )
    .unwrap();
    pending(
        &normalize(
            &source(&base, 0xb6),
            &artifacts,
            &reviewed(&artifacts),
            Default::default(),
        )
        .unwrap(),
    );
}

#[test]
fn ambiguous_or_unreviewed_source_frames_cannot_prove_absence_in_any_set() {
    let base = xml(&format!("{ACTIVE}{SUPPORT}"));
    for invalid in [
        base.replace("activeSkillSet=\"7\"", "activeSkillSet=\"07\""),
        base.replace("activeSkillSet=\"7\"", "activeSkillSet=\"99\""),
        base.replace(" activeSkillSet=\"7\"", ""),
        base.replace("id=\"7\"><Skill", "id=\"07\"><Skill"),
        base.replace("</Skills>", "<SkillSet id=\"7\"/></Skills>"),
        base.replace("</Skills>", "<SkillSet id=\"8\"><Unknown/></SkillSet></Skills>"),
        base.replace("</Skills>", "</Skills><Skills activeSkillSet=\"8\"><SkillSet id=\"8\"/></Skills>"),
        base.replace("</Skill>", "<Unknown/></Skill>"),
        base.replace("<Skill enabled=\"true\">", "<Skill enabled=\"true\" surprise=\"true\">"),
        base.replace("<Skill enabled=\"true\">", "<Skill enabled=\"true\" xmlns:x=\"urn:test\">"),
        base.replace(SUPPORT, "<Gem gemId=\"support\" variantId=\"v\"><StatSetIndex skillId=\"x\" value=\"1\"/></Gem>"),
        base.replace("</Skill>", "text</Skill>"),
    ] {
        let artifacts = artifacts(true);
        let result = normalize(&source(&invalid, 0xb7), &artifacts, &reviewed(&artifacts), Default::default()).unwrap();
        pending(&result);
    }
    let duplicate = base.replace("gemId=\"support\"", "gemId=\"support\" gemId=\"support\"");
    assert!(matches!(decode_build(duplicate.as_bytes()),
        Err(poe_optimizer_import::ImportError::InvalidXml(roxmltree::Error::DuplicatedAttribute(ref name, _))) if name == "gemId"));
}

#[test]
fn stale_bindings_fail_even_when_no_gem_rows_exist_and_wire_remains_opt_in() {
    let artifacts = artifacts(true);
    let source = source(&xml(""), 0xb8);
    let valid = reviewed(&artifacts);
    let wire = serde_json::to_value(&valid.support_origin_order).unwrap();
    assert_eq!(
        wire["kind"],
        "saved_manual_group_order_with_physical_inventory_v2"
    );
    let restored: SupportOriginOrderPolicy = serde_json::from_value(wire).unwrap();
    assert_eq!(Some(restored), valid.support_origin_order);
    assert_eq!(
        normalize(&source, &artifacts, &valid, Default::default())
            .unwrap()
            .draft()
            .input()
            .skill_presets
            .members[0]
            .supports
            .completion,
        DraftListCompletion::Complete
    );
    for index in 0..2 {
        let mut stale = valid.clone();
        let Some(SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source,
            roles,
        }) = &mut stale.support_origin_order
        else {
            unreachable!()
        };
        let digest =
            poe_optimizer_core::owned_content::digest_owned("stale-support-inventory", &0, 100)
                .unwrap();
        if index == 0 {
            *mapping_source = digest;
        } else {
            *roles = digest;
        }
        assert!(matches!(
            normalize(&source, &artifacts, &stale, Default::default()),
            Err(NormalizationError::Binding)
        ));
    }
    for old in [
        None,
        Some(SupportOriginOrderPolicy::SavedManualGroupOrder {}),
    ] {
        let mut policy = valid.clone();
        policy.support_origin_order = old;
        let wire = serde_json::to_vec(&policy).unwrap();
        let restored: NormalizationPolicy = serde_json::from_slice(&wire).unwrap();
        assert_eq!(serde_json::to_vec(&restored).unwrap(), wire);
        pending(&normalize(&source, &artifacts, &restored, Default::default()).unwrap());
    }
}

#[test]
fn inventory_inspection_charges_work_and_honors_collection_bounds() {
    let artifacts = artifacts(true);
    let source = source(&xml(&format!("{ACTIVE}{SUPPORT}")), 0xb9);
    let mut old = reviewed(&artifacts);
    old.support_origin_order = Some(SupportOriginOrderPolicy::SavedManualGroupOrder {});
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        match normalize(
            &source,
            &artifacts,
            &old,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected error: {error}"),
        }
    }
    assert!(matches!(
        normalize(
            &source,
            &artifacts,
            &reviewed(&artifacts),
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let mut limits = NormalizationLimits::default();
    // Leave room for the two independent query templates so this probes the
    // inventory's set bound rather than the earlier query-input bound.
    limits.draft.input.max_collection_entries = 2;
    let two_sets = xml("").replace(
        "</Skills>",
        "<SkillSet id=\"8\"/><SkillSet id=\"9\"/></Skills>",
    );
    assert!(matches!(
        normalize(
            &super::source(&two_sets, 0xba),
            &artifacts,
            &reviewed(&artifacts),
            limits
        ),
        Err(NormalizationError::Limit("support inventory sets"))
    ));
}
