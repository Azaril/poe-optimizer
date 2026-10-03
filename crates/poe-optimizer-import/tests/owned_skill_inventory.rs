//! Direct roots and generated representations share the same authored inventory.
#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "support/direct_minion_actions_fixture.rs"]
mod direct_minions;
#[allow(dead_code)]
#[path = "support/direct_dispositions_fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod inventory;
#[allow(dead_code)]
#[path = "support/minion_actions_fixture.rs"]
mod minions;
use fixture::Fixture;
use poe_optimizer_core::owned_draft::*;
use poe_optimizer_import::{owned_normalize::*, owned_source_actions::*};

#[path = "support/owned_skill_inventory_transitions.rs"]
mod transitions;

fn configure(f: &mut Fixture) {
    f.base.policy.skill_inventory = Some(SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        mapping_source: *f.base.base.mapping.source_identity(),
        roles: *f.base.base.roles.identity(),
        direct_inputs: Some(
            direct_skill_inputs_identity(
                f.base.policy.direct_skill_inputs.as_ref().unwrap(),
                Default::default(),
            )
            .unwrap(),
        ),
        generated_groups: vec![],
    });
}
fn add_generated(f: &mut Fixture) {
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
        catalog_gem,
        game_id,
        variant_id,
        skill_id,
        name_spec,
        ..
    } = &f.reference
    else {
        unreachable!()
    };
    let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
        generated_groups, ..
    } = f.base.policy.skill_inventory.as_mut().unwrap();
    for source in [
        GeneratedSkillGroupSource::TreeAllocation,
        GeneratedSkillGroupSource::ItemGrant,
    ] {
        generated_groups.push(GeneratedSkillGroupInventory {
            source,
            gem: catalog_gem.clone(),
            game_id: game_id.clone(),
            variant_id: variant_id.clone(),
            skill_id: skill_id.clone(),
            name_spec: name_spec.clone(),
        });
    }
}
fn flags(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .skill_presets
        .members
        .iter()
        .map(|p| matches!(p.skills.completion, DraftListCompletion::Complete))
        .collect()
}
fn exact_delta(mut f: Fixture, text: &str, expected: &[bool]) -> NormalizedImport {
    let reviewed = f.base.policy.skill_inventory.take().unwrap();
    let before = f.base.run(text, Default::default()).unwrap();
    f.base.policy.skill_inventory = Some(reviewed);
    let after = f.base.run(text, Default::default()).unwrap();
    assert_eq!(flags(&after), expected);
    let mut draft = before.draft().input().clone();
    let mut retired = std::collections::BTreeSet::new();
    for (preset, complete) in draft.skill_presets.members.iter_mut().zip(expected) {
        if *complete {
            let DraftListCompletion::Pending { id, code } = &preset.skills.completion else {
                panic!("prior inventory");
            };
            assert_eq!(code.as_str(), "skill-membership-not-converted");
            retired.insert(*id);
            preset.skills.completion = DraftListCompletion::Complete;
        }
    }
    assert_eq!(&draft, after.draft().input());
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut sidecar = before.into_parts().1;
    sidecar.policy = after.sidecar().policy;
    sidecar.draft = after.sidecar().draft;
    let mut removed = std::collections::BTreeSet::new();
    for origin in &mut sidecar.origins {
        origin.links.retain(|link| {
            if let OwnedOriginTarget::Issue(id) = link
                && retired.contains(id)
            {
                assert!(removed.insert(*id));
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
    after
}

#[test]
fn repeated_direct_occurrences_keep_raw_values_and_no_physical_container() {
    let mut f = Fixture::new();
    configure(&mut f);
    let text = inventory::xml(&format!(
        "{}{}",
        inventory::GEM,
        inventory::GEM.replace("level=\"17\"", "level=\"18\"")
    ));
    let after = exact_delta(f, &text, &[true]);
    assert!(after.draft().input().gems.members.is_empty());
    assert_eq!(after.draft().input().skills.members.len(), 2);
    let preset = &after.draft().input().skill_presets.members[0];
    assert_ne!(preset.skills.members[0], preset.skills.members[1]);
    assert!(matches!(
        preset.usage_preferences.as_ref().unwrap().completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn nested_maps_need_the_existing_exact_disposition_proof() {
    let maps = minions::maps(&minions::entry_map("1", "1"), &minions::entry_map("2", "2"));
    for (children, expected) in [
        (maps.clone(), true),
        (
            maps.replace("skillIndex=\"1\"", "skillIndex=\"bad\""),
            false,
        ),
        (format!("{maps}<Unreviewed/>"), false),
    ] {
        let mut f = Fixture::new();
        configure(&mut f);
        let text = inventory::xml(&inventory::with_children(&children));
        exact_delta(f, &text, &[expected]);
    }
}

#[test]
fn stale_direct_policy_commitment_rejects_even_when_source_still_matches() {
    let mut f = Fixture::new();
    configure(&mut f);
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        group_attributes, ..
    } = f.base.policy.direct_skill_inputs.as_mut().unwrap()
    else {
        unreachable!()
    };
    group_attributes.push("another-field".into());
    assert!(matches!(
        f.base
            .run(&inventory::xml(inventory::GEM), Default::default()),
        Err(NormalizationError::Binding)
    ));
}

#[test]
fn reviewed_orphan_generated_rows_are_not_extra_authored_roots() {
    for origin in ["Tree:123", "Item:987:Absent provider"] {
        let mut f = Fixture::new();
        configure(&mut f);
        add_generated(&mut f);
        let text = inventory::xml(inventory::GEM).replace(
            "</SkillSet>",
            &format!(
                r#"<Skill source="{origin}">{}</Skill></SkillSet>"#,
                inventory::GEM
            ),
        );
        let after = exact_delta(f, &text, &[true]);
        assert_eq!(after.draft().input().skills.members.len(), 1);
        assert!(after.draft().input().gems.members.is_empty());
        assert!(matches!(
            after.draft().input().skill_presets.members[0]
                .supports
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn unknown_generated_tuple_siblings_and_noncanonical_keys_stay_pending() {
    for (origin, row) in [
        (
            "Tree:123",
            inventory::GEM.replace("skillId=\"effect\"", "skillId=\"unknown\""),
        ),
        ("Tree:123", format!("{}{}", inventory::GEM, inventory::GEM)),
        ("Tree:123", format!("{}<Unexpected/>", inventory::GEM)),
        ("Tree:0123", inventory::GEM.to_owned()),
        ("Tree:0", inventory::GEM.to_owned()),
        ("Tree:9007199254740992", inventory::GEM.to_owned()),
        ("Item:0987:Alias", inventory::GEM.to_owned()),
        ("Item:987", inventory::GEM.to_owned()),
        ("Unreviewed:123", inventory::GEM.to_owned()),
    ] {
        let mut f = Fixture::new();
        configure(&mut f);
        add_generated(&mut f);
        let text = inventory::xml(inventory::GEM).replace(
            "</SkillSet>",
            &format!(r#"<Skill source="{origin}">{row}</Skill></SkillSet>"#),
        );
        exact_delta(f, &text, &[false]);
    }
}

#[test]
fn generated_to_manual_source_edit_creates_an_independent_authored_instance() {
    let mut f = Fixture::new();
    configure(&mut f);
    add_generated(&mut f);
    let text = inventory::xml(inventory::GEM).replace(
        "</SkillSet>",
        &format!("<Skill>{}</Skill></SkillSet>", inventory::GEM),
    );
    let after = exact_delta(f, &text, &[true]);
    assert_eq!(after.draft().input().skills.members.len(), 2);
}

#[test]
fn generated_policy_rejects_duplicate_or_unbound_catalog_identities() {
    for invalid in 0..5 {
        let mut f = Fixture::new();
        configure(&mut f);
        add_generated(&mut f);
        let SkillInventoryPolicy::PobFreshAuthoredRootsV1 {
            generated_groups, ..
        } = f.base.policy.skill_inventory.as_mut().unwrap();
        match invalid {
            0 => generated_groups.push(generated_groups[0].clone()),
            1 => generated_groups[0].skill_id = "unreviewed-effect".into(),
            2 => generated_groups[0].game_id = "unreviewed-game-id".into(),
            3 => generated_groups[0].variant_id = "unreviewed-variant".into(),
            _ => generated_groups[0].name_spec.clear(),
        }
        assert!(
            matches!(
                f.base
                    .run(&inventory::xml(inventory::GEM), Default::default()),
                Err(NormalizationError::Policy(_))
            ),
            "invalid generated identity {invalid}"
        );
    }
}
