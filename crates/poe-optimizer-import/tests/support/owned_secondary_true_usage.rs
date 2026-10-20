//! Exact true is a neutral source gate, not an unread-singleton assertion.
use super::*;

fn require_secondary(f: &mut fixture::Fixture) {
    inventory::inventory(f).rows[0]
        .fields
        .iter_mut()
        .find(|field| {
            matches!(&field.field,
            UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "enableGlobal2")
        })
        .unwrap()
        .destinations
        .push(UsageInventoryDestination::InputDisposition);
    inventory::rebind(f);
}

fn without_inventory(f: &mut fixture::Fixture) {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    inventory::rebind(f);
}

fn assert_retirements(before: &NormalizedImport, after: &NormalizedImport, complete: &[bool]) {
    let mut expected = before.draft().input().clone();
    let mut origins = before.sidecar().origins.clone();
    for (preset, retire) in expected.skill_presets.members.iter_mut().zip(complete) {
        if !retire {
            continue;
        }
        let usage = preset.usage_preferences.as_mut().unwrap();
        let DraftListCompletion::Pending { id, .. } = usage.completion else {
            panic!()
        };
        usage.completion = DraftListCompletion::Complete;
        for origin in &mut origins {
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(id));
        }
    }
    assert_eq!(&expected, after.draft().input());
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());
}

#[test]
fn exact_secondary_true_needs_live_input_proof_but_not_singleton_construction() {
    let mut unavailable = inventory::fixture();
    require_secondary(&mut unavailable);
    assert!(matches!(
        unavailable
            .base
            .roles
            .role(&unavailable.base.gem)
            .unwrap()
            .construction,
        OwnedGemSourceConstruction::Unavailable
    ));
    let mut multiple = singleton_secondary_fixture(|facts| {
        let extra = OwnedIndexedSourceEffectReference {
            index: 1,
            effect: facts.primary.clone(),
        };
        facts.declared_additional_effects.push(extra.clone());
        facts.constructed_additional_effects.push(extra);
        facts
            .resolved_additional_effects
            .push(facts.primary.clone());
        facts.resolved_effects.push(facts.primary.clone());
    });
    for f in [&mut unavailable, &mut multiple] {
        let text = xml(GEM);
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [true]);
        without_inventory(f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_retirements(&before, &after, &[true]);
        // The same source/reference and values remain unchanged. No additional
        // effect or Action was invented to account for the literal field.
        assert_eq!(before.draft().input().skills, after.draft().input().skills);
    }
}

#[test]
fn exact_secondary_true_refuses_other_tokens_failed_proofs_and_second_consumers() {
    let mut f = inventory::fixture();
    require_secondary(&mut f);
    for token in [
        "false",
        "nil",
        "",
        " true",
        "true ",
        "TRUE",
        "1",
        "tr&#117;e",
    ] {
        let text = xml(&GEM.replace(
            r#"enableGlobal2="true""#,
            &format!(r#"enableGlobal2="{token}""#),
        ));
        assert_eq!(
            completed(&f.run(&text, Default::default()).unwrap()),
            [false],
            "{token}"
        );
    }
    for text in [
        xml(&GEM.replace(r#" enableGlobal2="true""#, "")),
        xml(&GEM.replace(r#"quality="0""#, r#"quality="bad""#)),
        xml(&GEM.replace(r#"count="1""#, r#"count="bad""#)),
        xml(&GEM.replace(r#"skillId="effect""#, r#"skillId="other-effect""#)),
        xml(&fixture::with_children("<Unknown/>")),
    ] {
        assert_eq!(
            completed(&f.run(&text, Default::default()).unwrap()),
            [false]
        );
    }
    inventory::defer(&mut f, "enableGlobal2");
    assert_eq!(
        completed(&f.run(&xml(GEM), Default::default()).unwrap()),
        [false]
    );
}

#[test]
fn neutral_secondary_is_exactly_per_occurrence_and_preserves_other_preset_obligations() {
    let mut f = inventory::fixture();
    require_secondary(&mut f);
    let no = GEM.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="false""#);
    for (first, second, expected) in [
        (GEM, no.as_str(), [true, false]),
        (no.as_str(), GEM, [false, true]),
    ] {
        let text = format!(
            r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="true">{first}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{second}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), expected);
        let saved = f.policy.usage_inputs.clone();
        without_inventory(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_retirements(&before, &after, &expected);
        f.policy.usage_inputs = saved;
        inventory::rebind(&mut f);
    }
}
