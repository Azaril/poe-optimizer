//! Selector-only integration controls over real normalization. These synthetic
//! definitions retain missing references; they do not model a missing mechanic
//! as absent or claim a complete native build.
use super::*;

fn missing_command(facts: &mut ObservedGemSourceConstruction) {
    let reference = OwnedIndexedSourceEffectReference {
        index: 1,
        effect: OwnedSourceEffectReference {
            effect_id: "unresolved-command".into(),
            resolution: OwnedSourceEffectResolution::Missing,
        },
    };
    facts.declared_additional_effects.push(reference.clone());
    facts.constructed_additional_effects.push(reference);
    for kind in [
        OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
        OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
    ] {
        facts.missing_references.push(OwnedMissingSourceEffect {
            kind,
            index: Some(1),
            effect_id: "unresolved-command".into(),
        });
    }
}

fn selector_fixture(change: impl FnOnce(&mut ObservedGemSourceConstruction)) -> fixture::Fixture {
    let mut f = singleton_secondary_fixture(|facts| {
        missing_command(facts);
        change(facts);
    });
    // The existing fixture has a real typed secondary-switch consumer. Keep
    // that consumer, but do not additionally demand the independent unread
    // secondary proof in this selector-only policy. A counterfactual below
    // reinstates it and must remain Pending for an exact literal false.
    let field = inventory::inventory(&mut f).rows[0]
        .fields
        .iter_mut()
        .find(|field| {
            matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name }
            if name == "enableGlobal2")
        })
        .unwrap();
    assert_eq!(field.destinations.len(), 2);
    field
        .destinations
        .retain(|destination| !matches!(destination, UsageInventoryDestination::InputDisposition));
    assert!(matches!(
        field.destinations.as_slice(),
        [UsageInventoryDestination::UsageParameter { .. }]
    ));
    for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
        inventory::inventory(&mut f).rows[0]
            .fields
            .push(UsageInventoryField {
                field: UsageInventoryFieldSelector::GroupAttribute { name: name.into() },
                destinations: vec![UsageInventoryDestination::InputDisposition],
            });
    }
    inventory::rebind(&mut f);
    f
}

fn assert_only_usage_retired(before: &NormalizedImport, after: &NormalizedImport) {
    assert_eq!(completed(before), [false]);
    assert_eq!(completed(after), [true]);
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
        "all native records, values and allocator retained"
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
    assert_eq!(
        before.sidecar().source_sha256,
        after.sidecar().source_sha256
    );
}

#[test]
fn constructed_primary_selectors_account_only_the_exact_source_reference() {
    for (main, calcs) in [("nil", "1"), ("1", "nil")] {
        let mut f = selector_fixture(|_| {});
        let role_input = f.base.roles.input().clone();
        let schema_input = f.base.schema.input().clone();
        let text = selected_group(GEM, main, calcs);
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [true]);
        // The whole-draft inverse below preserves every unrelated obligation.
        // Complete selection admission is exercised by the parent finalizer test.
        let repeat = f.run(&text, Default::default()).unwrap();
        assert_eq!(repeat.draft().input(), after.draft().input());
        assert_eq!(
            serde_json::to_value(repeat.sidecar()).unwrap(),
            serde_json::to_value(after.sidecar()).unwrap()
        );
        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            f.policy.usage_inputs.as_mut().unwrap();
        *inventory = None;
        inventory::rebind(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_only_usage_retired(&before, &after);
        assert_eq!(
            f.base.roles.input(),
            &role_input,
            "full unresolved Command census is retained"
        );
        assert_eq!(
            f.base.schema.input(),
            &schema_input,
            "native declaration coverage does not change"
        );
        let decoded = poe_optimizer_import::decode_build(text.as_bytes()).unwrap();
        assert_eq!(decoded.xml, text);
        assert_eq!(after.sidecar().source_sha256, decoded.sha256);
    }
}

#[test]
fn omitted_mapped_effect_cannot_borrow_the_constructed_selector_receipt() {
    // The index admits the mapped reference and every individual identity, but
    // a final list omitting it cannot prove there was only one source candidate.
    for mode in 0..3 {
        let mut f = selector_fixture(|facts| {
            let mut effect = facts.primary.clone();
            if mode != 0 {
                effect.effect_id = "effect-alias".into();
            }
            if mode == 2 {
                facts.resolved_effects[0] = effect;
            } else {
                facts
                    .constructed_additional_effects
                    .push(OwnedIndexedSourceEffectReference { index: 2, effect });
            }
        });
        let text = selected_group(GEM, "nil", "1");
        let with_inventory = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&with_inventory), [false], "mode {mode}");
        let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
            f.policy.usage_inputs.as_mut().unwrap();
        *inventory = None;
        inventory::rebind(&mut f);
        let without_inventory = f.run(&text, Default::default()).unwrap();
        assert_eq!(
            with_inventory.draft().input(),
            without_inventory.draft().input()
        );
        assert_eq!(
            with_inventory.sidecar().origins,
            without_inventory.sidecar().origins
        );
    }
}

#[test]
fn constructed_selector_does_not_discharge_independent_or_unavailable_consumers() {
    for mode in 0..4 {
        let mut f = selector_fixture(|_| {});
        match mode {
            0 => {
                let field = inventory::inventory(&mut f).rows[0].fields.iter_mut().find(|field| {
                    matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "enableGlobal2")
                }).unwrap();
                field
                    .destinations
                    .push(UsageInventoryDestination::InputDisposition);
            }
            1 => {
                let field = inventory::inventory(&mut f).rows[0].fields.iter_mut().find(|field| {
                    matches!(&field.field, UsageInventoryFieldSelector::GroupAttribute { name } if name == "mainActiveSkill")
                }).unwrap();
                field
                    .destinations
                    .push(UsageInventoryDestination::Deferred {
                        reason: actions::key("other-selector-consumer"),
                    });
            }
            2 => {
                let mut roles = f.base.roles.input().clone();
                roles.roles[0].construction = OwnedGemSourceConstruction::Unavailable;
                f.base.roles = OwnedSkillRoleIndex::new(
                    roles,
                    &f.base.mapping,
                    &f.base.schema,
                    Default::default(),
                )
                .unwrap();
                rebind_selector_fixture(&mut f);
            }
            _ => {}
        }
        inventory::rebind(&mut f);
        let mut gem = GEM.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="false""#);
        if mode == 3 {
            gem = gem.replace(r#"count="1""#, r#"count="bad""#);
        }
        let text = selected_group(&gem, "nil", "1");
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false], "mode {mode}");
        assert_eq!(
            poe_optimizer_import::decode_build(text.as_bytes())
                .unwrap()
                .xml,
            text
        );
    }
}
