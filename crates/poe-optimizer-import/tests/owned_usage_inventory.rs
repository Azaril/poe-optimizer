//! Exhaustive saved-field accounting is independent of input/value projection.
#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod fixture;
#[path = "support/owned_usage_inventory.rs"]
mod inventory;
use fixture::{GEM, xml};
use poe_optimizer_core::owned_draft::*;
use poe_optimizer_core::{owned_definitions::*, owned_schema::*};
use poe_optimizer_data::owned_schema::OwnedDefinitionSchemaPackage;
use poe_optimizer_import::owned_normalize::*;
use poe_optimizer_import::{
    owned_mapping::*, owned_reward_policy::OwnedRewardPolicy, owned_skill_catalog::*,
};

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
        completed(&f.run(&xml(GEM), Default::default()).unwrap()),
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
                    assert_eq!(facts.resolved_effects, [facts.primary.clone()]);
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
            completed(&f.run(&xml(GEM), Default::default()).unwrap()),
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
