//! Actual full-reference producer controls. Missing loader headers/profile
//! selectors are facts about source interpretation, never native input defaults.
use super::*;
#[allow(dead_code)]
#[path = "minion_actions_fixture.rs"]
mod minion;

fn add_field(f: &mut fixture::Fixture, name: &str, group: bool) {
    let field = if group {
        UsageInventoryFieldSelector::GroupAttribute { name: name.into() }
    } else {
        UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() }
    };
    inventory::inventory(f).rows[0]
        .fields
        .push(UsageInventoryField {
            field,
            destinations: vec![UsageInventoryDestination::InputDisposition],
        });
}

fn reference_fixture(minion: bool) -> fixture::Fixture {
    let mut f = if minion {
        let source = minion::Fixture::new();
        let mut f = fixture::Fixture::from_base(source.base);
        f.row().reference_action = source.input;
        inventory::from_fixture(f)
    } else {
        inventory::fixture()
    };
    let names = [
        "statSetIndex",
        "statSetIndexCalcs",
        "skillMinion",
        "skillMinionCalcs",
        "skillMinionSkill",
        "skillMinionSkillCalcs",
    ];
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    physical[0]
        .attributes
        .extend(names.iter().map(|name| (*name).into()));
    for name in if minion { &names[..] } else { &names[..2] } {
        add_field(&mut f, name, false);
    }
    add_field(&mut f, "label", true);
    inventory::rebind(&mut f);
    f
}

fn prove_only_inventory_retirement(f: &mut fixture::Fixture, text: &str) {
    let source_roles = f.base.roles.input().clone();
    let definitions = f.base.schema.input().clone();
    let after = f.run(text, Default::default()).unwrap();
    assert_eq!(completed(&after), [true]);
    let repeat = f.run(text, Default::default()).unwrap();
    assert_eq!(repeat.draft().input(), after.draft().input());
    assert_eq!(repeat.allocator_after(), after.allocator_after());
    assert_eq!(
        serde_json::to_value(repeat.sidecar()).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    inventory::rebind(f);
    let before = f.run(text, Default::default()).unwrap();
    assert_eq!(completed(&before), [false]);
    let mut expected = before.draft().input().clone();
    let usage = expected.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap();
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        unreachable!()
    };
    assert_eq!(usage.members.len(), 1);
    assert_eq!(usage.members[0].parameters.members.len(), 3);
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(
        &expected,
        after.draft().input(),
        "all scalar/usage values, native records and IDs retained"
    );
    let mut origins = before.sidecar().origins.clone();
    let mut removed = 0;
    for origin in &mut origins {
        removed += origin
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(id))
            .count();
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert!(removed >= 2);
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(f.base.roles.input(), &source_roles);
    assert_eq!(f.base.schema.input(), &definitions);
    let decoded = poe_optimizer_import::decode_build(text.as_bytes()).unwrap();
    assert_eq!(decoded.xml, text);
    assert_eq!(decoded.sha256, after.sidecar().source_sha256);
    assert!(matches!(
        after.draft().input().choice_presets.members[0]
            .choices
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn absent_legacy_headers_and_label_are_checked_without_native_defaults() {
    for legacy in ["", r#" statSetIndex="nil" statSetIndexCalcs="nil""#] {
        let mut f = reference_fixture(false);
        let gem = GEM.replace("/>", &format!("{legacy}/>"));
        prove_only_inventory_retirement(&mut f, &xml(&gem));
    }
}

#[test]
fn missing_minion_profiles_and_actions_require_the_successful_exact_reference() {
    // These are independent source defaults with explicit correspondence. The
    // absence of the keyed stat-set map is NOT the overwritten root-header fact.
    for saved in [
        "",
        r#" skillMinion="fixture-minion" skillMinionSkill="1" skillMinionSkillCalcs="1""#,
        r#" skillMinion="fixture-minion" skillMinionCalcs="fixture-minion" skillMinionSkill="1" skillMinionSkillCalcs="1""#,
    ] {
        let mut f = reference_fixture(true);
        let gem = GEM.replace("/>", &format!("{saved}/>"));
        prove_only_inventory_retirement(&mut f, &xml(&gem));
    }
}

#[test]
fn missing_reference_facts_never_replace_invalid_or_unproven_selection() {
    for mode in 0..7 {
        let mut f = reference_fixture(true);
        let mut gem = GEM.to_owned();
        match mode {
            0 => {
                let poe_optimizer_import::owned_source_actions::SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { minion, .. } = &mut f.row().reference_action else { unreachable!() };
                minion.allow_absent = false;
            }
            1 => {
                let poe_optimizer_import::owned_source_actions::SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { absent_action, .. } = &mut f.row().reference_action else { unreachable!() };
                *absent_action = None;
            }
            2 => {
                let poe_optimizer_import::owned_source_actions::SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { actions, .. } = &mut f.row().reference_action else { unreachable!() };
                actions[0].absent_stat_set = None;
            }
            3 => gem = gem.replace("/>", r#" skillMinionCalcs="unknown-profile"/>"#),
            4 => gem = gem.replace("/>", r#" skillMinionSkillCalcs="0"/>"#),
            5 => gem = gem.replace("/>", "><Unhandled/></Gem>"),
            _ => {
                let GemInventoryPolicy::PobFreshPhysicalV3 {
                    primary_dispositions,
                    ..
                } = f.policy.gem_inventory.as_mut().unwrap();
                primary_dispositions.clear();
                // Rebind only the existing policy commitment, with no invented
                // replacement disposition/reference for this negative control.
                let digest = usage_inputs_identity(&f.policy, Default::default()).unwrap();
                let GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. } =
                    f.policy.gem_inventory.as_mut().unwrap();
                *usage_inputs = digest;
            }
        }
        if mode != 6 {
            inventory::rebind(&mut f);
        }
        let text = xml(&gem);
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false], "mode {mode}");
        if mode == 5 {
            // The independent usage projections survive an unsupported child;
            // failed reference accounting must retain their Pending inventory.
            assert!(matches!(
                result.draft().input().gems.members[0].parameters.completion,
                DraftListCompletion::Pending { .. }
            ));
        }
        assert!(matches!(
            result.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            poe_optimizer_import::decode_build(text.as_bytes())
                .unwrap()
                .xml,
            text
        );
    }
}

#[test]
fn flat_reference_cannot_certify_absent_minion_or_group_selection_fields() {
    for name in [
        "skillMinionCalcs",
        "skillMinionSkillCalcs",
        "mainActiveSkill",
    ] {
        let mut f = reference_fixture(false);
        add_field(&mut f, name, name == "mainActiveSkill");
        inventory::rebind(&mut f);
        let result = f.run(&xml(GEM), Default::default()).unwrap();
        assert_eq!(completed(&result), [false], "{name}");
    }
}
