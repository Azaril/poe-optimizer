//! The existing generated Tree/Item fixture, with checked source construction.
//! Synthetic names are deliberately unrelated to any game skill or build.
use super::*;
use poe_optimizer_import::owned_skill_roles::{
    ObservedGemSourceConstruction, OwnedGemSourceConstruction, OwnedIndexedSourceEffectReference,
    OwnedMissingSourceEffect, OwnedMissingSourceEffectKind, OwnedReversePrimaryCandidate,
    OwnedReversePrimaryCensus, OwnedReversePrimaryRoute, OwnedSkillRolePackageInput,
    OwnedSourceEffectReference, OwnedSourceEffectResolution, OwnedSourceStatSetReference,
};

fn fixture(change: impl FnOnce(&mut OwnedSkillRolePackageInput)) -> Fixture {
    let mut f = inventory_fixture();
    let mut mapping = f.artifacts.mapping.input().clone();
    let mut roles = f.artifacts.roles.input().clone();
    let mut routes = Vec::new();
    for (index, role) in roles.roles.iter_mut().enumerate() {
        let OwnedPrimarySkill::Known(skill) = &role.primary else {
            panic!("fixture has explicit primary identities")
        };
        let (game_id, variant_id) = mapping
            .entries
            .iter()
            .find_map(|entry| match (&entry.source, &entry.outcome) {
                (
                    ExternalSelector::Definition(ExternalOwnerSelector::Gem {
                        game_id: SourceComponent::Text(game),
                        variant_id: SourceComponent::Text(variant),
                    }),
                    MappingOutcome::Mapped {
                        target: SchemaSubject::Definition(DefinitionAddress::Gem(gem)),
                        ..
                    },
                ) if gem == &role.gem => Some((game.clone(), variant.clone())),
                _ => None,
            })
            .unwrap();
        let effect_id = mapping
            .entries
            .iter()
            .find_map(|entry| match (&entry.source, &entry.outcome) {
                (
                    ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                        effect_id: SourceComponent::Text(effect),
                    }),
                    MappingOutcome::Mapped {
                        target: SchemaSubject::Definition(DefinitionAddress::Skill(mapped)),
                        ..
                    },
                ) if mapped == skill => Some(effect.clone()),
                _ => None,
            })
            .unwrap_or_else(|| {
                let effect = format!("fixture-effect-{index}");
                mapping.entries.push(MappingEntry {
                    source: ExternalSelector::Definition(ExternalOwnerSelector::Skill {
                        effect_id: SourceComponent::Text(effect.clone()),
                    }),
                    outcome: MappingOutcome::Mapped {
                        target: subject(skill),
                        basis: MappingBasis::Exact,
                    },
                });
                effect
            });
        let primary = OwnedSourceEffectReference {
            effect_id,
            resolution: OwnedSourceEffectResolution::Mapped(skill.clone()),
        };
        role.construction =
            OwnedGemSourceConstruction::Observed(Box::new(ObservedGemSourceConstruction {
                catalog_gem_key: game_id.clone(),
                winning_declaration: u32::try_from(index + 1).unwrap(),
                game_id: game_id.clone(),
                variant_id,
                primary: primary.clone(),
                declared_additional_effects: vec![],
                constructed_additional_effects: vec![],
                resolved_additional_effects: vec![],
                resolved_effects: vec![primary.clone()],
                display_order: Some(vec![1]),
                // Multiple stat sets do not manufacture a second effect.
                additional_stat_sets: vec![OwnedSourceStatSetReference {
                    index: 1,
                    id: "another-stat-set".into(),
                }],
                missing_references: vec![],
            }));
        routes.push(OwnedReversePrimaryRoute {
            skill: primary,
            candidates: vec![OwnedReversePrimaryCandidate {
                gem: role.gem.clone(),
                catalog_gem_key: game_id,
            }],
        });
    }
    roles.compilation.skill_count = routes.len();
    roles.reverse_primary = OwnedReversePrimaryCensus::Observed { routes };
    f.artifacts.mapping = OwnedMappingIndex::new(
        mapping,
        &f.artifacts.registry,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    roles.mapping = *f.artifacts.mapping.identity();
    change(&mut roles);
    f.artifacts.roles = OwnedSkillRoleIndex::new(
        roles,
        &f.artifacts.mapping,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    f.artifacts.rewards = empty_rewards(&f.artifacts.mapping, &f.artifacts.schema);
    let identity = *f.artifacts.roles.identity();
    let Some(GeneratedSkillInputPolicy::PobSavedGeneratedInputsV1 { roles, .. }) =
        &mut f.policy.generated_skill_inputs
    else {
        unreachable!()
    };
    *roles = identity;
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 {
        roles, inventory, ..
    }) = &mut f.policy.usage_inputs
    else {
        unreachable!()
    };
    *roles = identity;
    for row in &mut inventory.as_mut().unwrap().rows {
        row.fields.extend(
            [
                UsageInventoryFieldSelector::GroupAttribute {
                    name: "mainActiveSkill".into(),
                },
                UsageInventoryFieldSelector::GroupAttribute {
                    name: "mainActiveSkillCalcs".into(),
                },
                UsageInventoryFieldSelector::OccurrenceAttribute {
                    name: "enableGlobal2".into(),
                },
            ]
            .map(|field| UsageInventoryField {
                field,
                destinations: vec![UsageInventoryDestination::InputDisposition],
            }),
        );
    }
    f
}

fn text(main: &str, calcs: &str, secondary: &str) -> String {
    source()
        .replace(
            "<Skill source=",
            &format!(r#"<Skill mainActiveSkill="{main}" mainActiveSkillCalcs="{calcs}" source="#,),
        )
        .replace(
            "<Gem gemId=",
            &format!(r#"<Gem enableGlobal2="{secondary}" gemId="#),
        )
}

fn ambiguous_fixture() -> Fixture {
    let mut f = fixture(|_| {});
    let alias = f
        .artifacts
        .registry
        .allocate_definition::<GemDefinition>()
        .unwrap();
    let mut schema = f.artifacts.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Gem(unknown(alias.clone())));
    f.artifacts.schema =
        OwnedDefinitionSchemaPackage::new(schema.clone(), Default::default()).unwrap();
    let mut mapping = f.artifacts.mapping.input().clone();
    mapping.registry = f.artifacts.registry.identity().unwrap();
    mapping.definitions = f.artifacts.schema.identity().clone();
    mapping.entries.push(MappingEntry {
        source: ExternalSelector::Definition(ExternalOwnerSelector::Gem {
            game_id: SourceComponent::Text("another-container".into()),
            variant_id: SourceComponent::Text("v".into()),
        }),
        outcome: MappingOutcome::Mapped {
            target: subject(&alias),
            basis: MappingBasis::Exact,
        },
    });
    f.artifacts.mapping = OwnedMappingIndex::new(
        mapping,
        &f.artifacts.registry,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.artifacts.roles.input().clone();
    roles.mapping = *f.artifacts.mapping.identity();
    roles.definitions = f.artifacts.schema.identity().clone();
    roles.compilation.staged_registry = f.artifacts.registry.identity().unwrap();
    let mut alternate = roles.roles[0].clone();
    alternate.gem = alias.clone();
    let OwnedGemSourceConstruction::Observed(facts) = &mut alternate.construction else {
        unreachable!()
    };
    facts.catalog_gem_key = "another-container".into();
    facts.game_id = "another-container".into();
    facts.winning_declaration = 2;
    let primary = facts.primary.clone();
    roles.roles.push(alternate);
    roles.compilation.gem_count = roles.roles.len();
    let OwnedReversePrimaryCensus::Observed { routes } = &mut roles.reverse_primary else {
        unreachable!()
    };
    routes
        .iter_mut()
        .find(|route| route.skill == primary)
        .unwrap()
        .candidates
        .push(OwnedReversePrimaryCandidate {
            gem: alias,
            catalog_gem_key: "another-container".into(),
        });
    f.artifacts.roles = OwnedSkillRoleIndex::new(
        roles,
        &f.artifacts.mapping,
        &f.artifacts.schema,
        Default::default(),
    )
    .unwrap();
    rebind_generated_field_schema(&mut f, schema);
    let rows = new_rows(&mut f.policy).clone();
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    let inventory = inventory.take();
    install(&f.artifacts, &mut f.policy, rows);
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        inventory: target, ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *target = inventory;
    f
}

fn without_inventory(f: &Fixture) -> NormalizationPolicy {
    let mut policy = f.policy.clone();
    let Some(UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. }) = &mut policy.usage_inputs
    else {
        unreachable!()
    };
    *inventory = None;
    policy
}

fn assert_pending_unchanged(text: &str, f: &Fixture) {
    let actual = normalize(text, f, &f.policy, Default::default()).unwrap();
    let baseline = normalize(text, f, &without_inventory(f), Default::default()).unwrap();
    assert!(
        matches!(
            generated_usage_completion(&actual),
            DraftListCompletion::Pending { .. }
        ),
        "the reviewed field must remain pending for this synthetic source: {text}"
    );
    assert_eq!(actual.draft().input(), baseline.draft().input());
    assert_eq!(actual.sidecar().origins, baseline.sidecar().origins);
    assert_eq!(actual.allocator_after(), baseline.allocator_after());
}

#[test]
fn exact_generated_singletons_account_both_selectors_and_secondary_without_changing_inputs() {
    let f = fixture(|_| {});
    for main in ["nil", "1"] {
        for calcs in ["nil", "1"] {
            for secondary in ["nil", "false", "true"] {
                let text = text(main, calcs, secondary);
                let actual = normalize_queries_with_retired(
                    &text,
                    &f,
                    &f.policy,
                    &[],
                    Default::default(),
                    1,
                )
                .unwrap();
                let baseline =
                    normalize(&text, &f, &without_inventory(&f), Default::default()).unwrap();
                let mut expected = baseline.draft().input().clone();
                let usage = &mut expected.skill_presets.members[1]
                    .intent
                    .as_mut()
                    .unwrap()
                    .usage;
                let DraftListCompletion::Pending { id, .. } = usage.completion else {
                    panic!()
                };
                usage.completion = DraftListCompletion::Complete;
                assert_eq!(actual.draft().input(), &expected);
                let mut origins = baseline.sidecar().origins.clone();
                for origin in &mut origins {
                    origin
                        .links
                        .retain(|link| *link != OwnedOriginTarget::Issue(id));
                }
                assert_eq!(actual.sidecar().origins, origins);
                assert_eq!(actual.allocator_after(), baseline.allocator_after());
                assert_eq!(numbers(&actual, 1), [Some(1); 4]);
                assert_eq!(bindings(&actual, 1).members.len(), 2);
                assert!(matches!(
                    expected.skill_presets.members[0]
                        .intent
                        .as_ref()
                        .unwrap()
                        .usage
                        .completion,
                    DraftListCompletion::Pending { .. }
                ));
            }
        }
    }
}

#[test]
fn generated_singleton_fields_require_complete_construction_and_unique_reverse_authority() {
    let text = text("1", "nil", "nil");
    let baseline = fixture(|_| {});
    assert_eq!(
        generated_usage_completion(
            &normalize_queries_with_retired(
                &text,
                &baseline,
                &baseline.policy,
                &[],
                Default::default(),
                1,
            )
            .unwrap()
        ),
        &DraftListCompletion::Complete
    );
    for mode in 0..4 {
        let f = fixture(|roles| {
            if mode == 0 {
                roles.reverse_primary = OwnedReversePrimaryCensus::Unavailable {};
                return;
            }
            let row = roles
                .roles
                .iter_mut()
                .find(|role| {
                    matches!(&role.construction, OwnedGemSourceConstruction::Observed(facts)
                    if facts.game_id == "sibling")
                })
                .unwrap();
            if mode == 1 {
                row.construction = OwnedGemSourceConstruction::Unavailable;
                roles.reverse_primary = OwnedReversePrimaryCensus::Unavailable {};
                return;
            }
            let OwnedGemSourceConstruction::Observed(facts) = &mut row.construction else {
                unreachable!()
            };
            let effect = if mode == 2 {
                facts.primary.clone()
            } else {
                OwnedSourceEffectReference {
                    effect_id: "missing-command-control".into(),
                    resolution: OwnedSourceEffectResolution::Missing,
                }
            };
            let indexed = OwnedIndexedSourceEffectReference {
                index: 1,
                effect: effect.clone(),
            };
            facts.declared_additional_effects.push(indexed.clone());
            facts.constructed_additional_effects.push(indexed);
            if mode == 2 {
                facts.resolved_additional_effects.push(effect.clone());
                facts.resolved_effects.push(effect);
            } else {
                for kind in [
                    OwnedMissingSourceEffectKind::DeclaredAdditionalEffect,
                    OwnedMissingSourceEffectKind::ConstructedAdditionalEffect,
                ] {
                    facts.missing_references.push(OwnedMissingSourceEffect {
                        kind,
                        index: Some(1),
                        effect_id: effect.effect_id.clone(),
                    });
                }
            }
        });
        assert_pending_unchanged(&text, &f);
    }
    // Both candidates are individually valid, mapped, complete singletons;
    // the generated reverse lookup still cannot select one by saved Gem ID.
    assert_pending_unchanged(&text, &ambiguous_fixture());
}

#[test]
fn generated_singleton_receipts_do_not_default_fields_or_borrow_other_consumers() {
    let f = fixture(|_| {});
    let original = text("nil", "1", "false");
    let accepted =
        normalize_queries_with_retired(&original, &f, &f.policy, &[], Default::default(), 1)
            .unwrap();
    assert_eq!(
        generated_usage_completion(&accepted),
        &DraftListCompletion::Complete
    );
    for changed in [
        original.replace(r#" mainActiveSkill="nil""#, ""),
        original.replace(r#" mainActiveSkillCalcs="1""#, ""),
        original.replace(r#" enableGlobal2="false""#, ""),
        original.replace(r#"mainActiveSkill="nil""#, r#"mainActiveSkill="0""#),
        original.replace(r#"mainActiveSkillCalcs="1""#, r#"mainActiveSkillCalcs="2""#),
        original.replace(r#"enableGlobal2="false""#, r#"enableGlobal2="FALSE""#),
        original.replace("Tree:7", "Tree:8"),
        original.replace("Item:1:Test Title", "Item:2:Test Title"),
        original.replace(r#"quality="20.25""#, r#"quality="bad""#),
        original.replace(r#"count="nil""#, r#"count="bad""#),
        // Keep the other selected occurrence's valid typed consumers so this
        // tests whole-preset refusal after one exact count fails projection.
        original.replace(
            r#"count="nil" quality="12.5""#,
            r#"count="bad" quality="12.5""#,
        ),
    ] {
        assert_ne!(changed, original);
        assert_pending_unchanged(&changed, &f);
    }
    // Undecodable attributes are refused by plain_row before source/provider
    // admission. Preserve that earlier responsibility, rather than asserting a
    // usage obligation was created and could subsequently be retired.
    let escaped = original.replace(r#"enableGlobal2="false""#, r#"enableGlobal2="f&#97;lse""#);
    let actual = normalize(&escaped, &f, &f.policy, Default::default()).unwrap();
    let baseline = normalize(&escaped, &f, &without_inventory(&f), Default::default()).unwrap();
    for result in [&actual, &baseline] {
        assert!(usage(result, 1).is_empty());
        let inputs = bindings(result, 1);
        assert!(inputs.members.is_empty());
        assert!(
            matches!(&inputs.completion, DraftListCompletion::Pending { code, .. }
            if code.as_str() == "generated-skill-inputs-not-converted")
        );
        assert_retained(result, &escaped);
    }
    assert_eq!(actual.draft().input(), baseline.draft().input());
    assert_eq!(actual.sidecar().origins, baseline.sidecar().origins);
    assert_eq!(actual.allocator_after(), baseline.allocator_after());
    for mode in 0..2 {
        let mut changed = fixture(|_| {});
        let Some(UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. }) =
            &mut changed.policy.usage_inputs
        else {
            unreachable!()
        };
        for row in &mut inventory.as_mut().unwrap().rows {
            let count = row
                .fields
                .iter_mut()
                .find(|field| {
                    field.field
                        == UsageInventoryFieldSelector::OccurrenceAttribute {
                            name: "count".into(),
                        }
                })
                .unwrap();
            if mode == 0 {
                count.destinations = vec![UsageInventoryDestination::InputDisposition];
            } else {
                count
                    .destinations
                    .push(UsageInventoryDestination::Deferred {
                        reason: key("other-count-consumer"),
                    });
            }
        }
        assert_pending_unchanged(&original, &changed);
    }
}

#[test]
fn generated_singleton_exact_work_limit_is_atomic_and_replayable() {
    let f = fixture(|_| {});
    let text = text("1", "nil", "nil");
    let expected =
        normalize_queries_with_retired(&text, &f, &f.policy, &[], Default::default(), 1).unwrap();
    let policy = serde_json::to_vec(&f.policy).unwrap();
    let roles = f.artifacts.roles.input().clone();
    let (mut low, mut high) = (1, NormalizationLimits::default().max_work);
    while low < high {
        let middle = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: middle,
            ..Default::default()
        };
        match normalize_queries_with_retired(&text, &f, &f.policy, &[], limits, 1) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected work refusal: {error}"),
        }
    }
    assert!(low > 1);
    let exact = normalize_queries_with_retired(
        &text,
        &f,
        &f.policy,
        &[],
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
        1,
    )
    .unwrap();
    assert!(matches!(
        normalize_queries_with_retired(
            &text,
            &f,
            &f.policy,
            &[],
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            },
            1
        ),
        Err(NormalizationError::Limit(_))
    ));
    assert_eq!(exact.draft().input(), expected.draft().input());
    assert_eq!(exact.sidecar().origins, expected.sidecar().origins);
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert_eq!(serde_json::to_vec(&f.policy).unwrap(), policy);
    assert_eq!(f.artifacts.roles.input(), &roles);
    let replay = normalize_queries_with_retired(
        &text,
        &f,
        &f.policy,
        &[],
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
        1,
    )
    .unwrap();
    assert_eq!(replay.draft().input(), exact.draft().input());
    assert_eq!(replay.sidecar().origins, exact.sidecar().origins);
    assert_eq!(replay.allocator_after(), exact.allocator_after());
}
