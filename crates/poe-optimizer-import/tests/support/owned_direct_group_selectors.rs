//! Exact manual source-reference accounting through the public normalizer.
//! Synthetic constructor facts model primary plus Command; no native Command
//! implementation, support eligibility or complete-build claim is made here.
use super::*;
use poe_optimizer_import::owned_skill_catalog::*;

fn refresh(f: &mut Fixture, roles: OwnedSkillRolePackageInput) {
    let a = &mut f.base.base;
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    let mut rewards = f.base.rewards.input().clone();
    rewards.mapping = *a.mapping.identity();
    f.base.rewards =
        OwnedRewardPolicy::new(rewards, &a.mapping, &a.schema, Default::default()).unwrap();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        roles,
        dispositions,
        ..
    } = f.base.policy.direct_skill_inputs.as_mut().unwrap();
    *roles = *a.roles.identity();
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { roles, .. } =
        &mut dispositions[0].reference_action
    else {
        panic!()
    };
    *roles = *a.roles.identity();
    f.reference = dispositions[0].reference_action.clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { roles, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    *roles = *a.roles.identity();
    rebind_usage(f);
}

fn fixture() -> Fixture {
    let mut f = configured_reference_absence();
    let a = &mut f.base.base;
    let mut roles = a.roles.input().clone();
    let row = roles.roles.iter_mut().find(|row| row.gem == a.gem).unwrap();
    let OwnedPrimarySkill::Known(primary) = &row.primary else {
        panic!()
    };
    // Deliberately share the native Skill identity through an explicit reviewed
    // alias: a changed first *source effect* must still refuse selection.
    let mut mapping = a.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text("command-effect".into()),
        }),
        outcome: MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(primary.clone())),
            basis: MappingBasis::ReviewedAlias {
                reason: actions::key("synthetic-command-alias"),
            },
        },
    });
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let effect = OwnedSourceEffectReference {
        effect_id: "effect".into(),
        resolution: OwnedSourceEffectResolution::Mapped(primary.clone()),
    };
    let command = OwnedSourceEffectReference {
        effect_id: "command-effect".into(),
        resolution: OwnedSourceEffectResolution::Mapped(primary.clone()),
    };
    let extra = OwnedIndexedSourceEffectReference {
        index: 1,
        effect: command.clone(),
    };
    row.construction =
        OwnedGemSourceConstruction::Observed(Box::new(ObservedGemSourceConstruction {
            catalog_gem_key: "manual-direct-catalog".into(),
            winning_declaration: 1,
            game_id: "physical".into(),
            variant_id: "variant".into(),
            primary: effect.clone(),
            declared_additional_effects: vec![extra.clone()],
            constructed_additional_effects: vec![extra],
            resolved_additional_effects: vec![command.clone()],
            resolved_effects: vec![effect, command],
            display_order: None,
            additional_stat_sets: vec![],
            missing_references: vec![],
        }));
    roles.mapping = *a.mapping.identity();
    refresh(&mut f, roles);
    let UsageInputPolicy::PobOccurrenceUsageV3 { occurrences, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    occurrences[0]
        .group_attributes
        .extend(["mainActiveSkill".into(), "mainActiveSkillCalcs".into()]);
    for name in ["source", "mainActiveSkill", "mainActiveSkillCalcs"] {
        inventory_mut(&mut f).rows[0]
            .fields
            .push(UsageInventoryField {
                field: UsageInventoryFieldSelector::GroupAttribute { name: name.into() },
                destinations: vec![UsageInventoryDestination::InputDisposition],
            });
    }
    f
}

fn text(gems: &str, selectors: &str) -> String {
    inventory::xml(gems).replace(
        r#"<Skill enabled="true">"#,
        &format!(r#"<Skill enabled="true"{selectors}>"#),
    )
}
const SELECTORS: &str = r#" mainActiveSkill="nil" mainActiveSkillCalcs="1""#;

fn without_inventory(f: &mut Fixture) {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.base.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
}

fn check_only_usage_retired(before: &NormalizedImport, after: &NormalizedImport) {
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut expected = before.draft().input().clone();
    let mut retired = Vec::new();
    for preset in &mut expected.skill_presets.members {
        let usage = preset.usage_preferences.as_mut().unwrap();
        let DraftListCompletion::Pending { id, ref code } = usage.completion else {
            panic!()
        };
        assert_eq!(code.as_str(), "usage-preferences-not-converted");
        assert_eq!(usage.members.len(), 1, "independent count binding remains");
        assert_eq!(usage.members[0].parameters.members.len(), 1);
        retired.push(id);
        usage.completion = DraftListCompletion::Complete;
    }
    assert!(!retired.is_empty());
    assert_eq!(&expected, after.draft().input());
    let mut sidecar = serde_json::to_value(before.sidecar()).unwrap();
    let after_sidecar = serde_json::to_value(after.sidecar()).unwrap();
    let mut origins = before.sidecar().origins.clone();
    for issue in retired {
        let mut removed = 0;
        for origin in &mut origins {
            removed += origin
                .links
                .iter()
                .filter(|link| **link == OwnedOriginTarget::Issue(issue))
                .count();
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(issue));
        }
        assert!(removed >= 2);
    }
    assert_eq!(origins, after.sidecar().origins);
    sidecar["origins"] = serde_json::to_value(origins).unwrap();
    // Source, links and all other sidecar metadata are compared in full. These
    // two commitments alone change with the explicit policy and resulting draft.
    assert_ne!(sidecar["policy"], after_sidecar["policy"]);
    assert_ne!(sidecar["draft"], after_sidecar["draft"]);
    sidecar["policy"] = after_sidecar["policy"].clone();
    sidecar["draft"] = after_sidecar["draft"].clone();
    assert_eq!(sidecar, after_sidecar);
}

fn assert_pending_unchanged(f: &mut Fixture, input: &str) {
    let result = run(f, input);
    assert!(!usage_complete(&result));
    assert_eq!(
        decode_build(input.as_bytes()).unwrap().sha256,
        result.sidecar().source_sha256
    );
    without_inventory(f);
    let old = run(f, input);
    assert_eq!(old.draft().input(), result.draft().input());
    assert_eq!(old.sidecar().origins, result.sidecar().origins);
    assert_eq!(old.allocator_after(), result.allocator_after());
    let mut old_sidecar = serde_json::to_value(old.sidecar()).unwrap();
    let new_sidecar = serde_json::to_value(result.sidecar()).unwrap();
    old_sidecar["policy"] = new_sidecar["policy"].clone();
    assert_eq!(old_sidecar, new_sidecar);
}

#[test]
fn direct_constructed_prefix_accounts_both_group_fields_and_genuine_manual_absence() {
    for main in ["nil", "1"] {
        for calcs in ["nil", "1"] {
            let mut f = fixture();
            let roles = f.base.base.roles.input().clone();
            let schema = f.base.base.schema.input().clone();
            let input = text(
                inventory::GEM,
                &format!(r#" mainActiveSkill="{main}" mainActiveSkillCalcs="{calcs}""#),
            );
            let after = run(&f, &input);
            assert!(usage_complete(&after));
            assert_eq!(complete(&after), [true]);
            assert!(after.draft().input().gems.members.is_empty());
            assert_eq!(
                values(&after.draft().input().skills.members[0]),
                [Some(17.0), Some(0.0)]
            );
            assert_eq!(&roles, f.base.base.roles.input());
            assert_eq!(&schema, f.base.base.schema.input());
            let repeat = run(&f, &input);
            assert_eq!(repeat.draft().input(), after.draft().input());
            assert_eq!(repeat.allocator_after(), after.allocator_after());
            assert_eq!(
                serde_json::to_value(repeat.sidecar()).unwrap(),
                serde_json::to_value(after.sidecar()).unwrap()
            );
            without_inventory(&mut f);
            check_only_usage_retired(&run(&f, &input), &after);
            assert!(matches!(
                after.draft().input().choice_presets.members[0]
                    .choices
                    .completion,
                DraftListCompletion::Pending { .. }
            ));
        }
    }
}

#[test]
fn direct_group_selection_refuses_other_source_routes_and_nonliteral_or_missing_selectors() {
    for source in ["", "Tree:1", "Item:1"] {
        let mut f = fixture();
        let input = text(inventory::GEM, &format!(r#"{SELECTORS} source="{source}""#));
        assert_pending_unchanged(&mut f, &input);
    }
    for name in ["mainActiveSkill", "mainActiveSkillCalcs"] {
        for token in [
            None,
            Some("0"),
            Some("2"),
            Some(""),
            Some("1.0"),
            Some("01"),
            Some(" 1"),
            Some("&#49;"),
        ] {
            let mut f = fixture();
            let prior = if name == "mainActiveSkill" {
                "nil"
            } else {
                "1"
            };
            let original = format!(r#" {name}="{prior}""#);
            let replacement = token.map_or(String::new(), |v| format!(r#" {name}="{v}""#));
            let input = text(inventory::GEM, &SELECTORS.replace(&original, &replacement));
            assert_pending_unchanged(&mut f, &input);
        }
    }
    let duplicate = text(
        inventory::GEM,
        &format!(r#"{SELECTORS} mainActiveSkill="nil""#),
    );
    assert!(
        decode_build(duplicate.as_bytes()).is_err(),
        "duplicate raw lanes fail before receipt construction"
    );
}

#[test]
fn direct_prefix_cannot_borrow_unavailable_reordered_or_unproved_reference_authority() {
    for mode in 0..12 {
        let mut f = fixture();
        let mut gem = inventory::GEM.to_owned();
        match mode {
            0..=2 => {
                let mut roles = f.base.base.roles.input().clone();
                let role = roles
                    .roles
                    .iter_mut()
                    .find(|r| r.gem == f.base.base.gem)
                    .unwrap();
                if mode == 0 {
                    role.construction = OwnedGemSourceConstruction::Unavailable;
                } else {
                    let OwnedGemSourceConstruction::Observed(facts) = &mut role.construction else {
                        panic!()
                    };
                    facts.resolved_effects.swap(0, 1);
                    if mode == 1 {
                        facts.display_order = Some(vec![2, 1]);
                    }
                    // Mode2 changes the observed first source effect even though
                    // both source IDs map to exactly the same native Skill.
                }
                refresh(&mut f, roles);
            }
            3 => gem = gem.replace(r#"enabled="true""#, r#"enabled="false""#),
            4 => gem = gem.replace(r#"enableGlobal1="true""#, r#"enableGlobal1="false""#),
            5 => gem = gem.replace(r#"variantId="variant""#, r#"variantId="wrong""#),
            6 => gem = gem.replace(r#"quality="0""#, r#"quality="bad""#),
            7 => {
                let DirectSkillInputPolicy::PobManualDirectSkillV2 { dispositions, .. } =
                    f.base.policy.direct_skill_inputs.as_mut().unwrap();
                dispositions.clear();
                rebind_usage(&mut f);
            }
            8 => gem = inventory::with_children("<Unhandled/>"),
            9 => gem = gem.replace("/>", r#" skillMinionCalcs="unknown-profile"/>"#),
            10 => gem = gem.replace(r#"count="1""#, r#"count="bad""#),
            _ => gem = gem.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="false""#),
        }
        assert_pending_unchanged(&mut f, &text(&gem, SELECTORS));
    }
    for name in ["source", "mainActiveSkill", "mainActiveSkillCalcs"] {
        let mut f = fixture();
        inventory_mut(&mut f).rows[0].fields.iter_mut().find(|field|
            matches!(&field.field, UsageInventoryFieldSelector::GroupAttribute { name: n } if n == name)
        ).unwrap().destinations.push(UsageInventoryDestination::Deferred { reason: actions::key("independent-group-consumer") });
        assert_pending_unchanged(&mut f, &text(inventory::GEM, SELECTORS));
    }
}

#[test]
fn direct_repeats_need_independent_groups_and_every_source_consumer() {
    // Two saved primary rows in one group cannot reuse a first-prefix receipt.
    let mut f = fixture();
    assert_pending_unchanged(&mut f, &text(&inventory::GEM.repeat(2), SELECTORS));
    // An unknown support/active sibling cannot be skipped by source order.
    let mut f = fixture();
    assert_pending_unchanged(
        &mut f,
        &text(
            &format!("{}<Gem gemId=\"unknown\"/>", inventory::GEM),
            SELECTORS,
        ),
    );
    // Independent presets retain distinct IDs, count bindings and retirements.
    let mut f = fixture();
    let one = text(inventory::GEM, SELECTORS);
    let start = one.find("<SkillSet ").unwrap();
    let end = one.find("</SkillSet>").unwrap() + "</SkillSet>".len();
    let second = one[start..end].replacen("id=\"1\"", "id=\"2\"", 1);
    let input = one.replacen("</Skills>", &format!("{second}</Skills>"), 1);
    let after = run(&f, &input);
    assert_eq!(after.draft().input().skills.members.len(), 2);
    assert_ne!(
        after.draft().input().skills.members[0].id,
        after.draft().input().skills.members[1].id
    );
    assert!(
        after
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .all(|p| matches!(
                p.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Complete
            ))
    );
    let repeat = run(&f, &input);
    assert_eq!(repeat.draft().input(), after.draft().input());
    assert_eq!(
        serde_json::to_value(repeat.sidecar()).unwrap(),
        serde_json::to_value(after.sidecar()).unwrap()
    );
    without_inventory(&mut f);
    check_only_usage_retired(&run(&f, &input), &after);
}

#[test]
fn direct_selector_proof_respects_default_work_budget_and_atomic_failure() {
    let f = fixture();
    let input = text(inventory::GEM, SELECTORS);
    let expected = run(&f, &input);
    assert!(usage_complete(&expected));
    let (mut low, mut high) = (1, NormalizationLimits::default().max_work);
    while low < high {
        let mid = low + (high - low) / 2;
        match f.base.run(
            &input,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        ) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_))
            | Err(NormalizationError::SourceAction(SourceActionError::Limit(_))) => low = mid + 1,
            Err(error) => panic!("{error:?}"),
        }
    }
    assert!(matches!(
        f.base.run(
            &input,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    let exact = f
        .base
        .run(
            &input,
            NormalizationLimits {
                max_work: low,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(exact.draft().input(), expected.draft().input());
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert_eq!(
        serde_json::to_value(exact.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
