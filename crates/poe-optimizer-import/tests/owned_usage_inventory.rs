//! Exhaustive saved-field accounting is independent of input/value projection.
#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod fixture;
#[path = "support/owned_usage_inventory.rs"]
mod inventory;
#[path = "support/owned_primary_usage_completion.rs"]
mod primary_completion;
#[path = "support/owned_secondary_true_usage.rs"]
mod secondary_true;
use fixture::{GEM, xml};
use poe_optimizer_core::owned_draft::*;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_normalize::*;
use poe_optimizer_import::{
    owned_mapping::*, owned_reward_policy::OwnedRewardPolicy, owned_skill_catalog::*,
};

// Physical group selectors remain independent of other usage obligations.
fn group_selector_fixture() -> fixture::Fixture {
    let mut f = singleton_secondary_fixture(|_| {});
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

fn selected_group(gems: &str, main: &str, calcs: &str) -> String {
    xml(gems).replace(
        r#"<Skill enabled="true">"#,
        &format!(
            r#"<Skill enabled="true" mainActiveSkill="{main}" mainActiveSkillCalcs="{calcs}">"#
        ),
    )
}

fn rebind_selector_fixture(f: &mut fixture::Fixture) {
    let definitions = f.base.schema.identity().clone();
    let GemQualityPolicy::Attributes(quality) = &mut f.policy.gem_quality else {
        unreachable!()
    };
    quality.definitions = definitions.clone();
    f.policy.gem_inputs.as_mut().unwrap().definitions = definitions.clone();
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = definitions.clone();
    rewards.mapping = *f.base.mapping.identity();
    f.rewards =
        OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let scalar = gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: bound,
        roles,
        scalar_inputs,
        ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *bound = definitions;
    *roles = *f.base.roles.identity();
    *scalar_inputs = scalar;
    inventory::rebind(f);
}

/// A synthetic physical support has a distinct Gem and owned intrinsic slots.
/// Sharing this test's Skill definition via an explicit reviewed source alias
/// is intentionally insufficient without its separate SupportAssignment role,
/// exact source construction, live target and complete input receipt.
fn add_selector_support(f: &mut fixture::Fixture) -> String {
    add_selector_support_with_parameter_closure(f, true)
}

fn add_selector_support_with_parameter_closure(
    f: &mut fixture::Fixture,
    complete_parameters: bool,
) -> String {
    let gem = f
        .base
        .registry
        .allocate_definition::<GemDefinition>()
        .unwrap();
    let mut schema = f.base.schema.input().clone();
    let SchemaLookup::Known(original) = f.base.schema.definition(&f.base.gem) else {
        unreachable!()
    };
    let mut support_schema = original.clone();
    let old_slots = support_schema.declarations.parameters.members.clone();
    let slots: Vec<_> = old_slots
        .iter()
        .map(|_| {
            f.base
                .registry
                .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Gem(gem.clone()))
                .unwrap()
        })
        .collect();
    for (old, slot) in old_slots.iter().zip(&slots) {
        let entry = schema
            .slots
            .iter()
            .find_map(|descriptor| match descriptor {
                SlotDescriptor::Parameter(entry) if &entry.id == old => Some(entry.clone()),
                _ => None,
            })
            .unwrap();
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: entry.schema,
            }));
    }
    support_schema.roles = vec![AuthoredGemRole::SupportAssignment];
    support_schema.skills = DeclaredSet::complete(vec![]);
    support_schema.declarations.parameters = if complete_parameters {
        DeclaredSet::complete(slots.clone())
    } else {
        DeclaredSet::partial(
            slots.clone(),
            vec![SchemaGap {
                subject: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                facet: SchemaFacet::InputSchema,
                code: actions::key("fixture-support-inputs"),
            }],
        )
    };
    support_schema.declarations.grants = DeclaredSet::complete(vec![]);
    support_schema.declarations.skill_grants = DeclaredSet::complete(vec![]);
    schema
        .definitions
        .push(DefinitionDescriptor::Gem(DefinitionEntry {
            id: gem.clone(),
            schema: SchemaState::Known(support_schema),
        }));
    f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut roles = f.base.roles.input().clone();
    let mut support_role = roles
        .roles
        .iter()
        .find(|row| row.gem == f.base.gem)
        .unwrap()
        .clone();
    let OwnedPrimarySkill::Known(primary) = &support_role.primary else {
        unreachable!()
    };
    let mut mapping = f.base.mapping.input().clone();
    mapping.registry = f.base.registry.identity().unwrap();
    mapping.definitions = f.base.schema.identity().clone();
    mapping.entries.extend([
        MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text("selector-support".into()),
                variant_id: SourceComponent::Text("variant".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone())),
                basis: MappingBasis::Exact,
            },
        },
        MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                effect_id: SourceComponent::Text("selector-support-effect".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: SchemaSubject::Definition(DefinitionAddress::Skill(primary.clone())),
                basis: MappingBasis::ReviewedAlias {
                    reason: actions::key("synthetic-selector-support-effect"),
                },
            },
        },
    ]);
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    support_role.gem = gem.clone();
    support_role.role = OwnedGemRole::Known(AuthoredGemRole::SupportAssignment);
    let OwnedGemSourceConstruction::Observed(facts) = &mut support_role.construction else {
        unreachable!()
    };
    facts.catalog_gem_key = "selector-support-catalog".into();
    facts.winning_declaration = 2;
    facts.game_id = "selector-support".into();
    facts.primary.effect_id = "selector-support-effect".into();
    facts.resolved_effects = vec![facts.primary.clone()];
    facts.additional_stat_sets.clear();
    roles.roles.push(support_role);
    roles.compilation.gem_count += 1;
    roles.definitions = f.base.schema.identity().clone();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let inputs = f.policy.gem_inputs.as_mut().unwrap();
    let mut parameters = inputs
        .gems
        .iter()
        .find(|row| row.gem == f.base.gem)
        .unwrap()
        .clone();
    parameters.gem = gem.clone();
    for parameter in &mut parameters.parameters {
        let index = old_slots
            .iter()
            .position(|slot| *slot == parameter.slot)
            .unwrap();
        parameter.slot = slots[index].clone();
    }
    inputs.gems.push(parameters);
    let mut physical = f.row().physical.clone();
    physical.gem = gem.clone();
    physical.game_id = "selector-support".into();
    physical.skill_id = "selector-support-effect".into();
    physical.name_spec = "Fixture support".into();
    physical.corrupted = slots[old_slots
        .iter()
        .position(|slot| *slot == physical.corrupted)
        .unwrap()]
    .clone();
    physical.corruption_level = slots[old_slots
        .iter()
        .position(|slot| *slot == physical.corruption_level)
        .unwrap()]
    .clone();
    let GemInventoryPolicy::PobFreshPhysicalV3 { supports, .. } =
        f.policy.gem_inventory.as_mut().unwrap();
    supports.push(physical);
    let mut fields: Vec<_> = [
        "gemId",
        "variantId",
        "skillId",
        "nameSpec",
        "level",
        "quality",
        "corrupted",
        "corruptLevel",
        "enabled",
        "count",
        "enableGlobal1",
        "enableGlobal2",
    ]
    .into_iter()
    .map(|name| UsageInventoryField {
        field: UsageInventoryFieldSelector::OccurrenceAttribute { name: name.into() },
        destinations: vec![UsageInventoryDestination::InputDisposition],
    })
    .collect();
    fields.push(UsageInventoryField {
        field: UsageInventoryFieldSelector::ReferenceDescendants,
        destinations: vec![UsageInventoryDestination::InputDisposition],
    });
    inventory::inventory(f).rows.push(UsageInventoryRow {
        source: UsageInventorySource::Physical { gem },
        fields,
    });
    rebind_selector_fixture(f);
    GEM.replace(r#"gemId="physical""#, r#"gemId="selector-support""#)
        .replace(
            r#"skillId="effect""#,
            r#"skillId="selector-support-effect""#,
        )
        .replace("Fixture skill", "Fixture support")
}

#[test]
fn exact_singleton_group_selectors_account_only_the_existing_group_fields() {
    let mut f = group_selector_fixture();
    for main in ["nil", "1"] {
        for calcs in ["nil", "1"] {
            let text = selected_group(GEM, main, calcs);
            let result = f.run(&text, Default::default()).unwrap();
            assert_eq!(completed(&result), [true], "{main}/{calcs}");
            let again = f.run(&text, Default::default()).unwrap();
            assert_eq!(result.draft().input(), again.draft().input());
            assert_eq!(
                serde_json::to_value(result.sidecar()).unwrap(),
                serde_json::to_value(again.sidecar()).unwrap()
            );
        }
    }
    let text = selected_group(GEM, "nil", "1");
    let after = f.run(&text, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: rows, ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *rows = None;
    inventory::rebind(&mut f);
    let before = f.run(&text, Default::default()).unwrap();
    let mut expected = before.draft().input().clone();
    let usage = expected.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap();
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        unreachable!()
    };
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    let mut origins = before.sidecar().origins.clone();
    for origin in &mut origins {
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert_eq!(origins, after.sidecar().origins);
}

#[test]
fn checked_singleton_support_companions_account_only_their_neutral_usage_fields() {
    let mut f = group_selector_fixture();
    let support = add_selector_support(&mut f);
    for gems in [
        format!("{support}{GEM}{support}"),
        format!("{GEM}{support}{support}"),
    ] {
        let result = f
            .run(&selected_group(&gems, "nil", "1"), Default::default())
            .unwrap();
        // Every support field now has its separate checked singleton proof;
        // the primary's group selector proof supplies no support permission.
        assert_eq!(completed(&result), [true]);
        assert_eq!(result.draft().input().skills.members.len(), 1);
        assert_eq!(result.draft().input().supports.members.len(), 2);
        assert!(
            result
                .draft()
                .input()
                .gems
                .members
                .iter()
                .all(|gem| { matches!(gem.parameters.completion, DraftListCompletion::Complete) })
        );
    }
    for changed in [
        support.replace(r#"count="1""#, r#"count="2""#),
        support.replace(r#"level="17""#, r#"level="broken""#),
        support.replace(r#"variantId="variant""#, r#"variantId="other""#),
        support.replace(
            r#"skillId="selector-support-effect""#,
            r#"skillId="effect""#,
        ),
        support.replace("<Gem ", r#"<Gem xmlns="foreign" "#),
    ] {
        let result = f
            .run(
                &selected_group(&format!("{GEM}{changed}"), "1", "1"),
                Default::default(),
            )
            .unwrap();
        assert_eq!(completed(&result), [false], "{changed}");
    }
    let mut roles = f.base.roles.input().clone();
    roles
        .roles
        .iter_mut()
        .find(|row| row.gem != f.base.gem)
        .unwrap()
        .construction = OwnedGemSourceConstruction::Unavailable;
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    rebind_selector_fixture(&mut f);
    assert_eq!(
        completed(
            &f.run(
                &selected_group(&format!("{GEM}{support}"), "1", "1"),
                Default::default()
            )
            .unwrap()
        ),
        [false]
    );
}

#[test]
fn neutral_group_fact_refuses_missing_escaped_generated_duplicate_and_unproved_frames() {
    let f = group_selector_fixture();
    for (main, calcs) in [
        ("0", "1"),
        ("1", "2"),
        ("", "nil"),
        ("n&#105;l", "1"),
        ("1", "&#49;"),
    ] {
        assert_eq!(
            completed(
                &f.run(&selected_group(GEM, main, calcs), Default::default())
                    .unwrap()
            ),
            [false]
        );
    }
    let ordinary = selected_group(GEM, "nil", "1");
    for changed in [
        ordinary.replace(r#" mainActiveSkill="nil""#, ""),
        ordinary.replace(r#" mainActiveSkillCalcs="1""#, ""),
        ordinary.replace(
            r#"<Skill enabled="true""#,
            r#"<Skill source="" enabled="true""#,
        ),
        selected_group(&format!("{GEM}{GEM}"), "1", "1"),
        selected_group(
            &format!(
                "{GEM}{}",
                GEM.replace(r#"gemId="physical""#, r#"gemId="unknown""#)
            ),
            "1",
            "1",
        ),
        selected_group(
            &GEM.replace(r#"quality="0""#, r#"quality="broken""#),
            "1",
            "1",
        ),
        selected_group(&fixture::with_children("<Unknown/>"), "1", "1"),
    ] {
        assert_eq!(
            completed(&f.run(&changed, Default::default()).unwrap()),
            [false],
            "{changed}"
        );
    }
    // Provider-generated source syntax is rejected before physical
    // materialization, so it has no usage_preferences list to inspect.
    let text = ordinary.replace(
        r#"<Skill enabled="true""#,
        r#"<Skill source="Tree:1" enabled="true""#,
    );
    let mut generated = group_selector_fixture();
    let after = generated.run(&text, Default::default()).unwrap();
    assert!(after.draft().input().gems.members.is_empty());
    assert!(after.draft().input().skills.members.is_empty());
    let preset = &after.draft().input().skill_presets.members[0];
    assert!(preset.usage_preferences.is_none());
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    let decoded = poe_optimizer_import::decode_build(text.as_bytes()).unwrap();
    assert_eq!(decoded.xml, text);
    assert_eq!(after.sidecar().source_sha256, decoded.sha256);
    assert_eq!(after.sidecar().source_bytes, text.len());
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: rows, ..
    } = generated.policy.usage_inputs.as_mut().unwrap();
    *rows = None;
    inventory::rebind(&mut generated);
    let before = generated.run(&text, Default::default()).unwrap();
    assert_eq!(after.draft().input(), before.draft().input());
    assert_eq!(after.sidecar().origins, before.sidecar().origins);
    assert_eq!(after.allocator_after(), before.allocator_after());
}

#[test]
fn group_selector_destination_remains_conjunctive_and_requires_current_construction() {
    let mut f = group_selector_fixture();
    let field = inventory::inventory(&mut f).rows[0].fields.iter_mut().find(|field| {
        matches!(&field.field, UsageInventoryFieldSelector::GroupAttribute { name } if name == "mainActiveSkill")
    }).unwrap();
    field
        .destinations
        .push(UsageInventoryDestination::Deferred {
            reason: actions::key("second-selector-consumer"),
        });
    inventory::rebind(&mut f);
    assert_eq!(
        completed(
            &f.run(&selected_group(GEM, "1", "1"), Default::default())
                .unwrap()
        ),
        [false]
    );
    let mut f = group_selector_fixture();
    let mut roles = f.base.roles.input().clone();
    let OwnedGemSourceConstruction::Observed(facts) = &mut roles.roles[0].construction else {
        unreachable!()
    };
    facts
        .constructed_additional_effects
        .push(OwnedIndexedSourceEffectReference {
            index: 3,
            effect: facts.primary.clone(),
        });
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    rebind_selector_fixture(&mut f);
    assert_eq!(
        completed(
            &f.run(&selected_group(GEM, "1", "1"), Default::default())
                .unwrap()
        ),
        [false]
    );
}

#[test]
fn group_selector_catalog_lookup_and_final_check_are_bounded() {
    let mut f = group_selector_fixture();
    let text = selected_group(GEM, "nil", "1");
    let needed = |f: &fixture::Fixture| {
        let (mut low, mut high) = (1usize, NormalizationLimits::default().max_work);
        assert!(
            f.run(
                &text,
                NormalizationLimits {
                    max_work: high,
                    ..Default::default()
                }
            )
            .is_ok()
        );
        while low < high {
            let mid = low + (high - low) / 2;
            match f.run(
                &text,
                NormalizationLimits {
                    max_work: mid,
                    ..Default::default()
                },
            ) {
                Ok(_) => high = mid,
                Err(NormalizationError::Limit(_)) => low = mid + 1,
                Err(error) => panic!("{error:?}"),
            }
        }
        low
    };
    let minimum = needed(&f);
    assert_eq!(
        completed(
            &f.run(
                &text,
                NormalizationLimits {
                    max_work: minimum,
                    ..Default::default()
                }
            )
            .unwrap()
        ),
        [true]
    );
    assert!(matches!(
        f.run(
            &text,
            NormalizationLimits {
                max_work: minimum - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    add_unrelated_role_rows(&mut f, 1024);
    assert_eq!(
        completed(
            &f.run(
                &text,
                NormalizationLimits {
                    max_work: minimum + 4096,
                    ..Default::default()
                }
            )
            .unwrap()
        ),
        [true]
    );
}

#[test]
fn support_source_disposition_is_independent_of_scalar_inventory_completeness() {
    let mut reference = None;
    for complete_parameters in [true, false] {
        let mut f = group_selector_fixture();
        let support = add_selector_support_with_parameter_closure(&mut f, complete_parameters);
        let text = selected_group(&format!("{support}{GEM}{support}"), "nil", "1");
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [true]);
        assert_eq!(after.draft().input().supports.members.len(), 2);
        assert!(
            after
                .draft()
                .input()
                .gems
                .members
                .iter()
                .all(|gem| gem.to_resolved().is_some())
        );
        if let Some(expected) = &reference {
            assert_eq!(after.draft().input(), expected);
        } else {
            reference = Some(after.draft().input().clone());
        }
        let nonneutral = support.replace(r#"count="1""#, r#"count="2""#);
        let rejected = f
            .run(
                &selected_group(&format!("{GEM}{nonneutral}"), "1", "1"),
                Default::default(),
            )
            .unwrap();
        assert_eq!(completed(&rejected), [false]);
        let support = &rejected.draft().input().supports.members[0];
        let source = support.support.to_resolved().unwrap();
        let gem = rejected
            .draft()
            .input()
            .gems
            .members
            .iter()
            .find(|gem| gem.id == source)
            .unwrap();
        assert_eq!(
            matches!(gem.parameters.completion, DraftListCompletion::Complete),
            complete_parameters,
            "refused source proof cannot downgrade independently complete scalar inputs"
        );
    }
}

#[test]
fn support_neutral_usage_receipts_require_every_exact_literal_and_consumer() {
    let mut f = group_selector_fixture();
    let support = add_selector_support(&mut f);
    let text = selected_group(&format!("{support}{GEM}{support}"), "nil", "1");
    let after = f.run(&text, Default::default()).unwrap();
    for gem in &after.draft().input().gems.members {
        assert!(
            gem.to_resolved().is_some(),
            "unresolved positive Gem: {gem:?}"
        );
    }
    for support in &after.draft().input().supports.members {
        assert!(
            support.to_resolved().is_some(),
            "unresolved positive Support: {support:?}"
        );
    }
    assert_eq!(completed(&after), [true]);
    assert_eq!(after.draft().input().supports.members.len(), 2);
    let again = f.run(&text, Default::default()).unwrap();
    assert_eq!(after.draft().input(), again.draft().input());
    assert_eq!(
        serde_json::to_value(after.sidecar()).unwrap(),
        serde_json::to_value(again.sidecar()).unwrap()
    );
    for (field, tokens) in [
        ("count", vec!["0", "2", "01", "1.0", "nil", "", "&#49;"]),
        (
            "enableGlobal1",
            vec!["false", "nil", "", "TRUE", "1", "tr&#117;e"],
        ),
        (
            "enableGlobal2",
            vec!["false", "nil", "", "TRUE", "1", "tr&#117;e"],
        ),
    ] {
        let expected = if field == "count" { "1" } else { "true" };
        let original = format!(r#" {field}="{expected}""#);
        for token in tokens {
            let changed = support.replace(&original, &format!(r#" {field}="{token}""#));
            let result = f
                .run(
                    &selected_group(&format!("{GEM}{changed}"), "1", "1"),
                    Default::default(),
                )
                .unwrap();
            assert_eq!(completed(&result), [false], "{field}={token}");
        }
        let changed = support.replace(&original, "");
        assert_eq!(
            completed(
                &f.run(
                    &selected_group(&format!("{GEM}{changed}"), "1", "1"),
                    Default::default()
                )
                .unwrap()
            ),
            [false],
            "missing {field}"
        );
    }
    let original = f.policy.usage_inputs.clone();
    for name in ["count", "enableGlobal1", "enableGlobal2"] {
        f.policy.usage_inputs = original.clone();
        let main = f.base.gem.clone();
        let row = inventory::inventory(&mut f)
            .rows
            .iter_mut()
            .find(
                |row| matches!(&row.source, UsageInventorySource::Physical { gem } if *gem != main),
            )
            .unwrap();
        row.fields.iter_mut().find(|field| {
            matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name: field } if field == name)
        }).unwrap().destinations.push(UsageInventoryDestination::Deferred {
            reason: actions::key("unaccounted-support-consumer"),
        });
        inventory::rebind(&mut f);
        assert_eq!(
            completed(&f.run(&text, Default::default()).unwrap()),
            [false],
            "{name}"
        );
    }
}

#[test]
fn support_usage_fields_do_not_borrow_singleton_authority_for_unresolved_children() {
    for mode in 0..5 {
        let mut f = group_selector_fixture();
        let support = add_selector_support(&mut f);
        let mut roles = f.base.roles.input().clone();
        let role = roles
            .roles
            .iter_mut()
            .find(|role| role.gem != f.base.gem)
            .unwrap();
        let OwnedGemSourceConstruction::Observed(facts) = &mut role.construction else {
            unreachable!()
        };
        match mode {
            0 => {
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
            }
            1 => facts.display_order = Some(vec![1, 1]),
            2 => {
                let extra = OwnedIndexedSourceEffectReference {
                    index: 2,
                    effect: OwnedSourceEffectReference {
                        effect_id: "unresolved-support-child".into(),
                        resolution: OwnedSourceEffectResolution::Missing,
                    },
                };
                facts.declared_additional_effects.push(extra.clone());
                facts.constructed_additional_effects.push(extra);
                facts.missing_references = [
                    OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
                    OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
                ]
                .into_iter()
                .map(|kind| OwnedMissingSourceEffect {
                    kind,
                    index: Some(2),
                    effect_id: "unresolved-support-child".into(),
                })
                .collect();
                assert_eq!(
                    facts.resolved_effects.as_slice(),
                    std::slice::from_ref(&facts.primary)
                );
            }
            3 => {
                // The public fixture intentionally has an exact reviewed
                // mapping to the same native Skill for this distinct effect.
                facts.resolved_effects[0].effect_id = "effect".into();
            }
            _ => role.construction = OwnedGemSourceConstruction::Unavailable,
        }
        f.base.roles =
            OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
                .unwrap();
        rebind_selector_fixture(&mut f);
        let result = f
            .run(
                &selected_group(&format!("{GEM}{support}"), "1", "1"),
                Default::default(),
            )
            .unwrap();
        assert_eq!(completed(&result), [false], "mode {mode}");
        assert_eq!(result.draft().input().supports.members.len(), 1);
        assert!(
            result
                .draft()
                .input()
                .gems
                .members
                .iter()
                .all(|gem| gem.to_resolved().is_some()),
            "complete scalar inputs do not substitute for source construction"
        );
    }
}

#[test]
fn support_usage_completion_preserves_other_outputs_and_requires_whole_group_accounting() {
    let mut f = group_selector_fixture();
    let support = add_selector_support(&mut f);
    let text = selected_group(&format!("{GEM}{support}"), "nil", "1");
    let after = f.run(&text, Default::default()).unwrap();
    assert_eq!(completed(&after), [true]);
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: rows, ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *rows = None;
    inventory::rebind(&mut f);
    let before = f.run(&text, Default::default()).unwrap();
    let mut expected = before.draft().input().clone();
    let usage = expected.skill_presets.members[0]
        .usage_preferences
        .as_mut()
        .unwrap();
    let DraftListCompletion::Pending { id, .. } = usage.completion else {
        unreachable!()
    };
    usage.completion = DraftListCompletion::Complete;
    assert_eq!(&expected, after.draft().input());
    let mut origins = before.sidecar().origins.clone();
    for origin in &mut origins {
        origin
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert_eq!(origins, after.sidecar().origins);
    assert_eq!(after.allocator_after(), before.allocator_after());

    let mut f = group_selector_fixture();
    let support = add_selector_support(&mut f);
    for changed in [
        GEM.replace(r#"count="1""#, r#"count="broken""#),
        GEM.replace(r#"quality="0""#, r#"quality="broken""#),
    ] {
        assert_eq!(
            completed(
                &f.run(
                    &selected_group(&format!("{changed}{support}"), "1", "1"),
                    Default::default()
                )
                .unwrap()
            ),
            [false]
        );
    }
}

#[test]
fn support_usage_proof_work_is_bounded_without_scanning_unrelated_catalog_rows() {
    let mut f = group_selector_fixture();
    let support = add_selector_support(&mut f);
    let text = selected_group(&format!("{support}{GEM}{support}"), "nil", "1");
    let (mut low, mut high) = (1usize, NormalizationLimits::default().max_work);
    assert_eq!(
        completed(&f.run(&text, Default::default()).unwrap()),
        [true]
    );
    while low < high {
        let mid = low + (high - low) / 2;
        match f.run(
            &text,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        ) {
            Ok(_) => high = mid,
            Err(NormalizationError::Limit(_)) => low = mid + 1,
            Err(error) => panic!("{error:?}"),
        }
    }
    assert_eq!(
        completed(
            &f.run(
                &text,
                NormalizationLimits {
                    max_work: low,
                    ..Default::default()
                }
            )
            .unwrap()
        ),
        [true]
    );
    assert!(matches!(
        f.run(
            &text,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    add_unrelated_role_rows(&mut f, 1024);
    assert_eq!(
        completed(
            &f.run(
                &text,
                NormalizationLimits {
                    max_work: low + 8192,
                    ..Default::default()
                }
            )
            .unwrap()
        ),
        [true]
    );
}

fn completed(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .skill_presets
        .members
        .iter()
        .map(|preset| {
            matches!(
                preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Complete
            )
        })
        .collect()
}

#[test]
fn actual_input_and_usage_receipts_retire_only_the_whole_usage_issue() {
    let mut f = inventory::fixture();
    let text = xml(GEM);
    let after = f.run(&text, Default::default()).unwrap();
    assert_eq!(completed(&after), [true]);
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    inventory::rebind(&mut f);
    let before = f.run(&text, Default::default()).unwrap();
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
        "values, identities and allocator are unchanged"
    );
    let mut expected_origins = before.sidecar().origins.clone();
    let mut count = 0;
    for row in &mut expected_origins {
        count += row
            .links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::Issue(id))
            .count();
        row.links
            .retain(|link| *link != OwnedOriginTarget::Issue(id));
    }
    assert!(count >= 2);
    assert_eq!(expected_origins, after.sidecar().origins);
    assert!(
        after
            .draft()
            .input()
            .gems
            .members
            .iter()
            .all(|gem| matches!(gem.parameters.completion, DraftListCompletion::Complete))
    );
}

#[test]
fn primary_true_input_fact_requires_the_actual_projection_and_all_declared_consumers() {
    let mut f = inventory::fixture();
    inventory::inventory(&mut f).rows[0]
        .fields
        .iter_mut()
        .find(|field| {
            matches!(
                &field.field,
                UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "enableGlobal1"
            )
        })
        .unwrap()
        .destinations
        .push(UsageInventoryDestination::InputDisposition);
    inventory::rebind(&mut f);
    let source = xml(GEM);
    let result = f.run(&source, Default::default()).unwrap();
    assert_eq!(completed(&result), [true]);
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
            r#"enableGlobal1="true""#,
            &format!(r#"enableGlobal1="{token}""#),
        ));
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false], "{token}");
    }
    let result = f
        .run(
            &xml(&GEM.replace(r#" enableGlobal1="true""#, "")),
            Default::default(),
        )
        .unwrap();
    assert_eq!(completed(&result), [false], "missing is not literal true");

    // A true field cannot bypass either the intrinsic proof or another live
    // usage responsibility; successful scalar transport is kept independently.
    for text in [
        xml(&GEM.replace(r#"quality="0""#, r#"quality="bad""#)),
        xml(&GEM.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="bad""#)),
    ] {
        assert_eq!(
            completed(&f.run(&text, Default::default()).unwrap()),
            [false]
        );
    }
    inventory::defer(&mut f, "enableGlobal1");
    assert_eq!(
        completed(&f.run(&source, Default::default()).unwrap()),
        [false]
    );
}

fn singleton_secondary_fixture(
    mutation: impl FnOnce(&mut ObservedGemSourceConstruction),
) -> fixture::Fixture {
    let mut f = inventory::fixture();
    let mut roles = f.base.roles.input().clone();
    let row = roles
        .roles
        .iter_mut()
        .find(|row| row.gem == f.base.gem)
        .unwrap();
    let OwnedPrimarySkill::Known(primary) = &row.primary else {
        unreachable!()
    };
    let primary_id = primary.clone();
    let effect = OwnedSourceEffectReference {
        effect_id: "effect".into(),
        resolution: OwnedSourceEffectResolution::Mapped(primary.clone()),
    };
    let mut construction = ObservedGemSourceConstruction {
        catalog_gem_key: "fixture-physical".into(),
        winning_declaration: 1,
        game_id: "physical".into(),
        variant_id: "variant".into(),
        primary: effect.clone(),
        declared_additional_effects: vec![],
        constructed_additional_effects: vec![],
        resolved_additional_effects: vec![],
        resolved_effects: vec![effect],
        display_order: None,
        additional_stat_sets: vec![OwnedSourceStatSetReference {
            index: 1,
            id: "separate-stat-set".into(),
        }],
        missing_references: vec![],
    };
    mutation(&mut construction);
    row.construction = OwnedGemSourceConstruction::Observed(Box::new(construction));
    // A real alias mapping makes the same-Skill/different-source-effect control
    // pass ordinary role binding; the receipt must still distinguish identities.
    let mut mapping = f.base.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
            effect_id: SourceComponent::Text("effect-alias".into()),
        }),
        outcome: MappingOutcome::Mapped {
            target: SchemaSubject::Definition(DefinitionAddress::Skill(primary_id)),
            basis: MappingBasis::ReviewedAlias {
                reason: actions::key("fixture-same-skill-different-source-effect"),
            },
        },
    });
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let mut rewards = f.rewards.input().clone();
    rewards.mapping = *f.base.mapping.identity();
    f.rewards =
        OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 { roles, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *roles = *f.base.roles.identity();
    inventory::inventory(&mut f).rows[0]
        .fields
        .iter_mut()
        .find(|field| {
            matches!(&field.field,
            UsageInventoryFieldSelector::OccurrenceAttribute { name }
            if name == "enableGlobal2")
        })
        .unwrap()
        .destinations
        .push(UsageInventoryDestination::InputDisposition);
    inventory::rebind(&mut f);
    f
}

#[test]
fn exact_physical_singleton_secondary_fact_preserves_all_other_consumers() {
    let mut f = singleton_secondary_fixture(|_| {});
    for token in ["true", "false"] {
        let text = xml(&GEM.replace(
            r#"enableGlobal2="true""#,
            &format!(r#"enableGlobal2="{token}""#),
        ));
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [true], "{token}");
        assert_eq!(
            result.draft().input().skill_presets.members[0]
                .usage_preferences
                .as_ref()
                .unwrap()
                .members[0]
                .parameters
                .members
                .len(),
            3
        );
    }
    for token in ["nil", "", " true", "true ", "TRUE", "1", "tr&#117;e"] {
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
        xml(&GEM.replace(r#"count="1""#, r#"count="bad""#)),
        xml(&GEM.replace(r#"quality="0""#, r#"quality="bad""#)),
        xml(&GEM.replace(r#"skillId="effect""#, r#"skillId="another-effect""#)),
    ] {
        assert_eq!(
            completed(&f.run(&text, Default::default()).unwrap()),
            [false]
        );
    }
    // An unknown exact Gem selector is refused before physical materialization
    // and before a usage list exists. Absence must not authorize an empty
    // completion or discard the unresolved source's membership obligations.
    let text = xml(&GEM.replace(r#"variantId="variant""#, r#"variantId="other-variant""#));
    let mut unknown = singleton_secondary_fixture(|_| {});
    let after = unknown.run(&text, Default::default()).unwrap();
    assert!(after.draft().input().gems.members.is_empty());
    assert!(after.draft().input().skills.members.is_empty());
    let preset = &after.draft().input().skill_presets.members[0];
    assert!(preset.usage_preferences.is_none());
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    let decoded = poe_optimizer_import::decode_build(text.as_bytes()).unwrap();
    assert_eq!(decoded.xml, text, "the unknown source is retained verbatim");
    assert_eq!(after.sidecar().source_sha256, decoded.sha256);
    assert_eq!(after.sidecar().source_bytes, text.len());
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        unknown.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    inventory::rebind(&mut unknown);
    let before = unknown.run(&text, Default::default()).unwrap();
    assert_eq!(after.draft().input(), before.draft().input());
    assert_eq!(after.sidecar().origins, before.sidecar().origins);
    assert_eq!(after.allocator_after(), before.allocator_after());

    inventory::defer(&mut f, "enableGlobal2");
    assert_eq!(
        completed(&f.run(&xml(GEM), Default::default()).unwrap()),
        [false]
    );
}

#[test]
fn unavailable_construction_cannot_mint_a_secondary_field_fact() {
    let mut f = inventory::fixture();
    assert!(matches!(
        &f.base.roles.role(&f.base.gem).unwrap().construction,
        OwnedGemSourceConstruction::Unavailable
    ));
    inventory::inventory(&mut f).rows[0]
        .fields
        .iter_mut()
        .find(|field| {
            matches!(&field.field,
            UsageInventoryFieldSelector::OccurrenceAttribute { name }
            if name == "enableGlobal2")
        })
        .unwrap()
        .destinations
        .push(UsageInventoryDestination::InputDisposition);
    inventory::rebind(&mut f);
    assert_eq!(
        completed(
            &f.run(
                &xml(&GEM.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="false""#)),
                Default::default()
            )
            .unwrap()
        ),
        [false]
    );
}

#[test]
fn constructed_or_reordered_effects_cannot_borrow_singleton_authority() {
    for mode in 0..6 {
        let f = singleton_secondary_fixture(|facts| {
            match mode {
                0 => {
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
                }
                1 => {
                    facts
                        .constructed_additional_effects
                        .push(OwnedIndexedSourceEffectReference {
                            index: 3,
                            effect: facts.primary.clone(),
                        });
                }
                2 => facts
                    .resolved_additional_effects
                    .push(facts.primary.clone()),
                3 => facts.display_order = Some(vec![1, 1]),
                4 => {
                    // Valid typed Missing: no source mapping exists, yet the
                    // resolved list still contains exactly the primary.
                    let missing = OwnedIndexedSourceEffectReference {
                        index: 1,
                        effect: OwnedSourceEffectReference {
                            effect_id: "missing-command".into(),
                            resolution: OwnedSourceEffectResolution::Missing,
                        },
                    };
                    facts.declared_additional_effects.push(missing.clone());
                    facts.constructed_additional_effects.push(missing);
                    facts.missing_references = [
                        OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
                        OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
                    ]
                    .into_iter()
                    .map(|kind| OwnedMissingSourceEffect {
                        kind,
                        index: Some(1),
                        effect_id: "missing-command".into(),
                    })
                    .collect();
                    assert_eq!(
                        facts.resolved_effects.as_slice(),
                        std::slice::from_ref(&facts.primary)
                    );
                }
                _ => {
                    facts.resolved_effects[0].effect_id = "effect-alias".into();
                    assert_eq!(
                        facts.resolved_effects[0].resolution,
                        facts.primary.resolution
                    );
                    assert_ne!(facts.resolved_effects[0].effect_id, facts.primary.effect_id);
                }
            }
        });
        assert_eq!(
            completed(
                &f.run(
                    &xml(&GEM.replace(r#"enableGlobal2="true""#, r#"enableGlobal2="false""#)),
                    Default::default()
                )
                .unwrap()
            ),
            [false],
            "mode {mode}"
        );
    }
}

fn add_unrelated_role_rows(f: &mut fixture::Fixture, count: usize) {
    let mut schema = f.base.schema.input().clone();
    let mut mapping = f.base.mapping.input().clone();
    let mut roles = f.base.roles.input().clone();
    for index in 0..count {
        let gem = f
            .base
            .registry
            .allocate_definition::<GemDefinition>()
            .unwrap();
        let subject = SchemaSubject::Definition(DefinitionAddress::Gem(gem.clone()));
        schema
            .definitions
            .push(DefinitionDescriptor::Gem(DefinitionEntry {
                id: gem.clone(),
                schema: SchemaState::Unmapped {
                    gaps: vec![SchemaGap {
                        subject: subject.clone(),
                        facet: SchemaFacet::InputSchema,
                        code: actions::key("unrelated-source-unreviewed"),
                    }],
                },
            }));
        mapping.entries.push(MappingEntry {
            source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                game_id: SourceComponent::Text(format!("unrelated-{index}")),
                variant_id: SourceComponent::Text("variant".into()),
            }),
            outcome: MappingOutcome::Mapped {
                target: subject,
                basis: MappingBasis::Exact,
            },
        });
        roles.roles.push(OwnedGemRoleRow {
            gem,
            primary: OwnedPrimarySkill::Unmapped {
                issue: actions::key("unknown-primary"),
            },
            role: OwnedGemRole::Unmapped {
                issue: actions::key("unknown-role"),
            },
            materialization: OwnedGemMaterialization::Unmapped {
                issue: actions::key("unknown-materialization"),
            },
            construction: OwnedGemSourceConstruction::Unavailable,
        });
    }
    roles.compilation.gem_count += count;
    f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let definitions = f.base.schema.identity().clone();
    mapping.definitions = definitions.clone();
    mapping.registry = f.base.registry.identity().unwrap();
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    roles.definitions = definitions.clone();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let GemQualityPolicy::Attributes(quality) = &mut f.policy.gem_quality else {
        unreachable!()
    };
    quality.definitions = definitions.clone();
    f.policy.gem_inputs.as_mut().unwrap().definitions = definitions.clone();
    let scalar = gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions: bound,
        roles,
        scalar_inputs,
        ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *bound = definitions.clone();
    *roles = *f.base.roles.identity();
    *scalar_inputs = scalar;
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = definitions;
    rewards.mapping = *f.base.mapping.identity();
    f.rewards =
        OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    inventory::rebind(f);
}

#[test]
fn unrelated_catalog_rows_do_not_consume_linear_receipt_work() {
    let small = singleton_secondary_fixture(|_| {});
    let text = xml(GEM);
    // Find the public normalizer's actual successful budget boundary; the test
    // does not duplicate its lookup algorithm or depend on its total base cost.
    let (mut failing, mut passing) = (0, NormalizationLimits::default().max_work);
    assert!(small.run(&text, Default::default()).is_ok());
    while passing - failing > 1 {
        let middle = failing + (passing - failing) / 2;
        match small.run(
            &text,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(result) => {
                assert_eq!(completed(&result), [true]);
                passing = middle;
            }
            Err(NormalizationError::Limit(kind)) if kind == "work" || kind.ends_with(" work") => {
                failing = middle
            }
            Err(error) => panic!("unexpected budget failure: {error:?}"),
        }
    }
    let baseline = small
        .run(
            &text,
            NormalizationLimits {
                max_work: passing,
                ..Default::default()
            },
        )
        .unwrap();
    let mut large = singleton_secondary_fixture(|_| {});
    add_unrelated_role_rows(&mut large, 1024);
    // A fixed small allowance admits the extra indexed comparisons; a scan
    // charge of catalog size times identity bytes exceeds this allowance.
    let after = large
        .run(
            &text,
            NormalizationLimits {
                max_work: passing + 2048,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(completed(&after), [true]);
    assert_eq!(baseline.draft().input(), after.draft().input());
}

#[test]
fn a_second_consumer_or_missing_field_keeps_the_shared_inventory_pending() {
    for mode in 0..3 {
        let mut f = inventory::fixture();
        match mode {
            0 => inventory::defer(&mut f, "enableGlobal2"),
            1 => {
                inventory::inventory(&mut f).rows[0].fields.retain(|row| !matches!(
                    &row.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "quality"
                ));
                inventory::rebind(&mut f);
            }
            _ => {
                inventory::inventory(&mut f).rows[0]
                    .fields
                    .push(UsageInventoryField {
                        field: UsageInventoryFieldSelector::GroupAttribute {
                            name: "mainActiveSkill".into(),
                        },
                        destinations: vec![UsageInventoryDestination::InputDisposition],
                    });
                inventory::rebind(&mut f);
            }
        }
        let result = f.run(&xml(GEM), Default::default()).unwrap();
        assert_eq!(completed(&result), [false], "mode {mode}");
        assert_eq!(
            result.draft().input().skill_presets.members[0]
                .usage_preferences
                .as_ref()
                .unwrap()
                .members
                .len(),
            1
        );
    }
}

#[test]
fn a_successful_parameter_cannot_claim_an_unrelated_source_field() {
    let mut f = inventory::fixture();
    let fields = &mut inventory::inventory(&mut f).rows[0].fields;
    let count = fields
        .iter()
        .find(|row| {
            matches!(&row.field,
        UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "count")
        })
        .unwrap()
        .destinations
        .clone();
    fields
        .iter_mut()
        .find(|row| {
            matches!(&row.field,
        UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "level")
        })
        .unwrap()
        .destinations = count;
    inventory::rebind(&mut f);
    assert!(matches!(
        f.run(&xml(GEM), Default::default()),
        Err(NormalizationError::Policy(
            "usage inventory parameter authority"
        ))
    ));
}

#[test]
fn complete_census_includes_late_unknown_rows_and_repeated_occurrences() {
    let f = inventory::fixture();
    let repeated = f
        .run(&xml(&format!("{GEM}{GEM}")), Default::default())
        .unwrap();
    assert_eq!(completed(&repeated), [true]);
    let usage = repeated.draft().input().skill_presets.members[0]
        .usage_preferences
        .as_ref()
        .unwrap();
    assert_eq!(usage.members.len(), 2);
    assert_ne!(usage.members[0].target, usage.members[1].target);
    for changed in [
        GEM.replace("gemId=\"physical\"", "gemId=\"unknown\""),
        GEM.replace("level=\"17\"", "level=\"broken\""),
        GEM.replace("/>", r#" unexpected="true"/>"#),
    ] {
        let result = f
            .run(&xml(&format!("{GEM}{changed}")), Default::default())
            .unwrap();
        assert_eq!(completed(&result), [false]);
    }
}

#[test]
fn group_selection_and_unreviewed_reference_children_remain_pending() {
    let f = inventory::fixture();
    for text in [
        xml(GEM).replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" mainActiveSkill=\"1\">",
        ),
        xml(&fixture::with_children("<Unknown/>")),
        xml(&fixture::with_children(&fixture::maps("1", "2"))),
        xml(&fixture::with_children(&fixture::maps("unknown", "1"))),
    ] {
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false]);
    }
}

#[test]
fn early_refusal_keeps_independent_presets_and_all_other_outputs_exact() {
    for refused_first in [true, false] {
        let mut f = inventory::fixture();
        let refused = format!(
            "{}{}",
            GEM.replace(r#"count="1""#, r#"count="broken""#),
            GEM.repeat(12)
        );
        let (first, second) = if refused_first {
            (refused.as_str(), GEM)
        } else {
            (GEM, refused.as_str())
        };
        let text = format!(
            r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true">{first}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{second}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [!refused_first, refused_first]);
        assert_eq!(after.draft().input().gems.members.len(), 14);
        assert_eq!(after.draft().input().skills.members.len(), 14);
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            inventory: rows, ..
        } = f.policy.usage_inputs.as_mut().unwrap();
        *rows = None;
        inventory::rebind(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        let mut expected = before.draft().input().clone();
        let index = usize::from(refused_first);
        let usage = expected.skill_presets.members[index]
            .usage_preferences
            .as_mut()
            .unwrap();
        let DraftListCompletion::Pending { id: issue, .. } = usage.completion else {
            unreachable!()
        };
        usage.completion = DraftListCompletion::Complete;
        assert_eq!(&expected, after.draft().input());
        let mut origins = before.sidecar().origins.clone();
        for origin in &mut origins {
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(issue));
        }
        assert_eq!(origins, after.sidecar().origins);
        assert_eq!(
            before.sidecar().source_sha256,
            after.sidecar().source_sha256
        );
        assert_eq!(before.sidecar().source_bytes, after.sidecar().source_bytes);
    }
}

#[test]
fn early_refusal_does_not_skip_whole_source_or_policy_validation() {
    let f = inventory::fixture();
    let rejected = GEM.replace(r#"count="1""#, r#"count="broken""#);
    for tail in ["", "<Unknown/>"] {
        let text = format!(
            r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true">{rejected}{tail}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false, tail.is_empty()]);
    }
    // Policy compilation checks the entire injected policy, even when source
    // completion will refuse immediately for a missing count.
    let mut invalid = inventory::fixture();
    inventory::inventory(&mut invalid).rows[0]
        .fields
        .push(UsageInventoryField {
            field: UsageInventoryFieldSelector::OccurrenceAttribute {
                name: "unrecognized-field".into(),
            },
            destinations: vec![UsageInventoryDestination::InputDisposition],
        });
    inventory::rebind(&mut invalid);
    assert!(matches!(
        invalid.run(&xml(&rejected), Default::default()),
        Err(NormalizationError::Policy(_))
    ));
}

#[test]
fn presets_complete_independently_and_a_late_unknown_row_is_not_dropped() {
    let f = inventory::fixture();
    let unknown = GEM.replace("gemId=\"physical\"", "gemId=\"unknown\"");
    for unknown_first in [false, true] {
        let first = if unknown_first {
            format!("{GEM}{unknown}")
        } else {
            GEM.into()
        };
        let second = if unknown_first {
            GEM.into()
        } else {
            format!("{GEM}{unknown}")
        };
        let text = format!(
            r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true">{first}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{second}</Skill></SkillSet></Skills></PathOfBuilding2>"#
        );
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [!unknown_first, unknown_first]);
        assert_eq!(result.draft().input().skill_presets.members.len(), 2);
        assert_eq!(result.draft().input().gems.members.len(), 2);
    }
}

#[test]
fn overridden_malformed_lane_does_not_gain_completion_from_the_winner() {
    let f = inventory::fixture();
    for losing in ["broken", "1001"] {
        let text = xml(&GEM.replace("count=\"1\"", &format!("count=\"{losing}\""))).replace(
            "<Skill enabled=\"true\">",
            "<Skill enabled=\"true\" groupCount=\"2\">",
        );
        let result = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&result), [false]);
        let usage = result.draft().input().skill_presets.members[0]
            .usage_preferences
            .as_ref()
            .unwrap();
        assert!(
            matches!(
                usage.members[0].parameters.completion,
                DraftListCompletion::Complete
            ),
            "the successful existing override transport is unchanged"
        );
        if losing == "1001" {
            assert!(
                matches!(
                    result.draft().input().gems.members[0].parameters.completion,
                    DraftListCompletion::Complete
                ),
                "syntax-only intrinsic disposition succeeded"
            );
        }
    }
}

#[test]
fn every_override_lane_needs_its_own_declared_consumer_even_when_absent() {
    let mut f = inventory::fixture();
    inventory::inventory(&mut f).rows[0].fields.retain(|row| {
        !matches!(
            &row.field, UsageInventoryFieldSelector::GroupAttribute { name } if name == "groupCount"
        )
    });
    inventory::rebind(&mut f);
    let result = f.run(&xml(GEM), Default::default()).unwrap();
    assert_eq!(
        completed(&result),
        [false],
        "occurrence count does not cover the omitted group-override consumer"
    );
    assert!(matches!(
        result.draft().input().gems.members[0].parameters.completion,
        DraftListCompletion::Complete
    ));
    assert!(matches!(
        result.draft().input().skill_presets.members[0]
            .usage_preferences
            .as_ref()
            .unwrap()
            .members[0]
            .parameters
            .completion,
        DraftListCompletion::Complete
    ));
}

#[test]
fn absent_capability_preserves_current_wire_and_low_work_is_an_error() {
    let mut f = inventory::fixture();
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    let wire = serde_json::to_value(&f.policy.usage_inputs).unwrap();
    assert!(wire.get("inventory").is_none());
    inventory::rebind(&mut f);
    assert_eq!(
        completed(&f.run(&xml(GEM), Default::default()).unwrap()),
        [false]
    );
    let f = inventory::fixture();
    let error = f.run(
        &xml(GEM),
        NormalizationLimits {
            max_work: 1,
            ..Default::default()
        },
    );
    assert!(matches!(error, Err(NormalizationError::Limit(_))));
}

#[test]
fn checked_finalizer_preserves_other_obligations_after_usage_completion() {
    use poe_optimizer_core::{build_identity::*, owned_project::VariantSelection};
    let f = inventory::fixture();
    let text = xml(GEM).replace(
        "</PathOfBuilding2>",
        r#"<Tree><Spec id="1"/></Tree><Items activeItemSet="1"><ItemSet id="1"/></Items></PathOfBuilding2>"#,
    );
    let normalized = f.run(&text, Default::default()).unwrap();
    assert_eq!(completed(&normalized), [true]);
    let mut input = normalized.draft().input().clone();
    // An explicit caller-authored loadout permits selecting these incomplete
    // presets; it supplies no imported scope/default or numerical coverage.
    let mut allocator = InstanceAllocator::from_state(input.allocator);
    let loadout = allocator.allocate::<WeaponLoadoutId>().unwrap();
    input.weapon_loadouts.members.push(loadout);
    input.allocator = allocator.state();
    let selection = EvaluationSelection {
        build: VariantSelection {
            character: input.character_presets.members[0].id,
            equipment: input.equipment_presets.members[0].id,
            allocations: input.allocation_presets.members[0].id,
            skills: input.skill_presets.members[0].id,
            choices: input.choice_presets.members[0].id,
            active_weapon_loadout: loadout,
        },
        scenario: input.scenario_presets.members[0].id,
        queries: input.query_presets.members[0].id,
    };
    let draft = DraftSession::new(input, Default::default()).unwrap();
    let proof = draft
        .prove_intent(&f.base.schema, Default::default(), Default::default())
        .unwrap();
    let result = draft
        .finalize_selection_checked(
            &f.base.schema,
            selection,
            &proof,
            Default::default(),
            Default::default(),
        )
        .unwrap();
    let DraftFinalization::Pending { issues, .. } = result.finalization() else {
        panic!("usage completion is not complete-build admission")
    };
    assert!(!issues.is_empty());
    assert!(
        !issues
            .iter()
            .any(|issue| issue.code.as_str() == "usage-preferences-not-converted")
    );
}

#[path = "support/owned_constructed_source_selectors.rs"]
mod constructed_source_selectors;

#[path = "support/owned_reference_absence_usage.rs"]
mod reference_absence_usage;
