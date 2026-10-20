//! Numeric progression stays a Choice reward, with exact per-lane accounting.
use super::*;

const NAME: &str = "caller-progression";

fn fixture(selection: RewardNumericSelection) -> Artifacts {
    let mut a = with_reward_policy();
    let mut schema = a.schema.input().clone();
    let reward = schema
        .definitions
        .iter_mut()
        .find_map(|d| match d {
            DefinitionDescriptor::Reward(r) => Some(r),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(shape) = &mut reward.schema else {
        panic!()
    };
    let owner = SlotOwnerDefId::Reward(reward.id.clone());
    let parameter: DeclaredSlot<ParameterSlotDefId> =
        a.registry.allocate_slot(owner.clone()).unwrap();
    let fixed: DeclaredSlot<ParameterSlotDefId> = a.registry.allocate_slot(owner).unwrap();
    shape.declarations.parameters = DeclaredSet::complete(vec![parameter.clone(), fixed.clone()]);
    for (slot, value) in [
        (
            parameter.clone(),
            ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(0).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            }),
        ),
        (fixed.clone(), ValueSchema::Boolean),
    ] {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot,
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::RewardParameter],
                    skill_input: None,
                }),
            }));
    }
    a.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = a.mapping.input().clone();
    mapping.registry = a.registry.identity().unwrap();
    mapping.definitions = a.schema.identity().clone();
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    roles.definitions = a.schema.identity().clone();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, Default::default()).unwrap();
    let mut input = a.rewards.input().clone();
    input.mapping = *a.mapping.identity();
    input.definitions = a.schema.identity().clone();
    input.rules = vec![RewardRuleInput {
        recipe: ValueRecipeInput {
            id: key("numeric-progression"),
            codec: ValueCodecInput {
                namespace: ns(),
                whitespace: WhitespacePolicy::Exact,
                codec: ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                },
            },
            tiers: [ValueLane::InputNumber, ValueLane::PlaceholderNumber]
                .into_iter()
                .map(|lane| ValueTier {
                    selectors: vec![ValueSelector {
                        lane,
                        name: NAME.into(),
                    }],
                    duplicates: DuplicatePolicy::Reject,
                })
                .collect(),
            missing: MissingValuePolicy::Pending,
            numeric_aliases: vec![],
        },
        output: RewardRuleOutput::NumericParameter(Box::new(RewardNumericParameter {
            selector: ExternalSelector::Definition(ExternalOwnerSelector::Reward {
                key: SourceComponent::Text("caller-award".into()),
            }),
            parameter,
            fixed_parameters: vec![ParameterAssignment {
                slot: fixed,
                value: ParameterValue::Boolean(true),
            }],
            selection,
        })),
    }];
    a.rewards = OwnedRewardPolicy::new(input, &a.mapping, &a.schema, Default::default()).unwrap();
    a
}

fn reviewed(a: &Artifacts, accounting: bool) -> NormalizationPolicy {
    let mut p = policy();
    let controls: Vec<_> = a
        .rewards
        .rules()
        .flat_map(|r| {
            r.recipe
                .tiers
                .iter()
                .flat_map(|t| t.selectors.iter())
                .map(|selector| ConfigurationRewardControl {
                    recipe: r.recipe.id.clone(),
                    selector: selector.clone(),
                })
        })
        .collect();
    p.configuration_reward_inventory = Some(
        ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            mapping_source: *a.mapping.source_identity(),
            reward_policy: *a.rewards.identity(),
            controls: controls.clone(),
        },
    );
    if accounting {
        p.configuration_dispositions = Some(ConfigurationDispositionPolicy {
            mapping_source: *a.mapping.source_identity(),
            catalogue_evidence: *a.mapping.identity(),
            constructor_evidence: *a.rewards.identity(),
            dependencies: configuration_disposition_dependencies(
                &p,
                &a.rewards,
                Default::default(),
            )
            .unwrap(),
            rules: controls
                .into_iter()
                .enumerate()
                .map(|(n, c)| ConfigurationDispositionRule {
                    id: key(&format!("reward-control-{n}")),
                    selector: c.selector,
                    destinations: vec![ConfigurationDestination::Adapter {
                        control: ConfigurationAdapterControl::Reward { recipe: c.recipe },
                    }],
                })
                .collect(),
        });
    }
    p
}
fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
}
fn field(element: &str, value: &str) -> String {
    format!(r#"<{element} name="{NAME}" number="{value}"/>"#)
}
fn normalize(
    imported: &ImportedBuildInstance,
    a: &Artifacts,
    p: &NormalizationPolicy,
) -> NormalizedImport {
    let evidence = SourceProjectEvidence::collect(imported, Default::default()).unwrap();
    normalize_fresh(
        &evidence,
        *imported.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&a.schema),
            item_source: &empty_item_source(&a.schema),
            mappings: &a.mapping,
            registry: &a.registry,
            definitions: &a.schema,
            roles: &a.roles,
            rewards: &a.rewards,
        },
        p,
        &queries(),
        Default::default(),
    )
    .unwrap()
}
fn projected(result: &NormalizedImport) -> Vec<i64> {
    result
        .draft()
        .input()
        .rewards
        .members
        .iter()
        .map(|reward| {
            assert_eq!(reward.parameters.completion, DraftListCompletion::Complete);
            let values: Vec<_> = reward
                .parameters
                .members
                .iter()
                .map(|v| v.to_resolved().unwrap().value)
                .collect();
            assert!(values.contains(&ParameterValue::Boolean(true)));
            assert_eq!(values.len(), 2);
            values
                .into_iter()
                .find_map(|v| match v {
                    ParameterValue::Integer(n) => Some(n.get()),
                    _ => None,
                })
                .unwrap()
        })
        .collect()
}
fn origins<'a>(
    imported: &ImportedBuildInstance,
    result: &'a NormalizedImport,
) -> Vec<&'a SourceOwnedOrigin> {
    imported
        .occurrences()
        .iter()
        .filter(|row| ["Input", "Placeholder"].contains(&row.name()))
        .map(|row| &result.sidecar().origins[row.id().ordinal() as usize])
        .collect()
}

#[test]
fn numeric_reward_uses_selected_lane_and_accounts_valid_losing_lanes() {
    let a = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    for (body, expected, selected_at) in [
        (field("Placeholder", "56"), 56, 0),
        (field("Input", "0") + &field("Placeholder", "56"), 56, 1),
        (field("Input", "7") + &field("Placeholder", "56"), 7, 0),
        (field("Input", "7") + &field("Placeholder", "0"), 7, 0),
        (field("Input", "100"), 100, 0),
    ] {
        let imported = source(&xml(&body), 0xa2);
        let result = normalize(&imported, &a, &reviewed(&a, true));
        assert_eq!(projected(&result), vec![expected]);
        let draft = result.draft().input();
        let choice = &draft.choice_presets.members[0];
        assert_eq!(choice.rewards.completion, DraftListCompletion::Complete);
        assert!(matches!(
            choice.choices.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            draft.rewards.completion,
            DraftListCompletion::Pending { .. }
        ));
        for (n, origin) in origins(&imported, &result).into_iter().enumerate() {
            assert!(
                origin
                    .links
                    .contains(&OwnedOriginTarget::ChoicePreset(choice.id))
            );
            assert!(
                !origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::Issue(_)))
            );
            if n == selected_at {
                assert_eq!(origin.disposition, SourceDisposition::Contributes);
                assert!(
                    origin
                        .links
                        .contains(&OwnedOriginTarget::Reward(draft.rewards.members[0].id))
                );
                assert_eq!(origin.links.len(), 2);
            } else {
                assert_eq!(
                    origin.disposition,
                    SourceDisposition::SourceOnly(key("unselected-config-projection"))
                );
                assert_eq!(origin.links.len(), 1);
            }
        }
        let repeated = normalize(&imported, &a, &reviewed(&a, true));
        assert_eq!(result.draft(), repeated.draft());
        assert_eq!(result.sidecar().origins, repeated.sidecar().origins);
        origin_integrity_with_retired(&imported, &result, 1);
    }
}

#[test]
fn absent_zero_malformed_losing_lanes_and_duplicates_cannot_invent_progression() {
    let a = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    let mut bodies = vec![
        String::new(),
        field("Input", "0"),
        field("Placeholder", "0"),
        field("Input", "0") + &field("Placeholder", "0"),
    ];
    for invalid in ["-1", "101", "1.5", "NaN", "bad", " 7"] {
        bodies.push(field("Input", invalid));
        bodies.push(field("Input", invalid) + &field("Placeholder", "56"));
        bodies.push(field("Input", "7") + &field("Placeholder", invalid));
    }
    bodies.extend([
        field("Input", "7") + &field("Input", "7"),
        field("Input", "7") + &field("Placeholder", "56") + &field("Placeholder", "56"),
        format!(r#"<Input name="{NAME}" string="7"/>"#),
        format!(r#"<Input name="{NAME}" number="7" boolean="true"/>"#),
        format!(r#"<Input name="{NAME}" number="7"><Future/></Input>"#),
    ]);
    for body in bodies {
        let imported = source(&xml(&body), 0xa3);
        let result = normalize(&imported, &a, &reviewed(&a, true));
        assert!(projected(&result).is_empty(), "{body}");
        assert!(matches!(
            result.draft().input().choice_presets.members[0]
                .rewards
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(origins(&imported, &result).iter().all(|origin| {
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Issue(_)))
        }));
        origin_integrity(&imported, &result);
    }
    let a = fixture(RewardNumericSelection::RecipePrecedence);
    let imported = source(&xml(&field("Input", "0")), 0xa3);
    assert_eq!(
        projected(&normalize(&imported, &a, &reviewed(&a, true))),
        vec![0]
    );
    let absent = source(&xml(""), 0xa3);
    assert!(projected(&normalize(&absent, &a, &reviewed(&a, true))).is_empty());
}

#[test]
fn numeric_output_does_not_retire_unaccounted_configuration_responsibilities() {
    let a = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    let imported = source(
        &xml(&(field("Input", "0") + &field("Placeholder", "56"))),
        0xa4,
    );
    for deferred in [false, true] {
        let mut p = reviewed(&a, deferred);
        if deferred {
            for row in &mut p.configuration_dispositions.as_mut().unwrap().rules {
                row.destinations.push(ConfigurationDestination::Deferred {
                    responsibility: ConfigurationResponsibility::ScenarioUsage,
                });
            }
        }
        let result = normalize(&imported, &a, &p);
        assert_eq!(projected(&result), vec![56]);
        for origin in origins(&imported, &result) {
            assert_eq!(origin.disposition, SourceDisposition::Contributes);
            assert!(
                origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::Issue(_)))
            );
        }
        origin_integrity_with_retired(&imported, &result, 1);
    }
}

#[test]
fn new_reward_occurrence_shifts_later_ids_without_copying_values_between_configsets() {
    let a = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    let mut empty = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    empty.rewards = empty_rewards(&empty.mapping, &empty.schema);
    let imported = source(
        &format!(
            r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2">{}</ConfigSet><ConfigSet id="3"/></Config></PathOfBuilding2>"#,
            field("Input", "7"),
            field("Placeholder", "56")
        ),
        0xa5,
    );
    let prior = normalize(&imported, &empty, &policy());
    let after = normalize(&imported, &a, &reviewed(&a, true));
    assert_eq!(projected(&after), vec![7, 56]);
    let before = prior.draft().input();
    let next = after.draft().input();
    assert_eq!(
        before.choice_presets.members[0].id,
        next.choice_presets.members[0].id
    );
    assert_eq!(
        next.choice_presets
            .members
            .iter()
            .map(|p| p.rewards.members.len())
            .collect::<Vec<_>>(),
        vec![1, 1, 0]
    );
    assert_eq!(
        next.scenario_presets.members[0].id.instance_id().local(),
        before.scenario_presets.members[0].id.instance_id().local() + 1
    );
    assert_eq!(
        next.scenario_presets.members[1].id.instance_id().local(),
        before.scenario_presets.members[1].id.instance_id().local() + 2
    );
    assert!(matches!(
        next.choice_presets.members[2].rewards.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(
        next.scenario_presets
            .members
            .iter()
            .all(|p| p.scenario.assumptions.members.is_empty()
                && p.scenario.usage.members.is_empty())
    );
}

#[test]
fn exact_numeric_control_census_rejects_missing_duplicate_or_competing_authority() {
    let a = fixture(RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder);
    let imported = source(&xml(&field("Placeholder", "56")), 0xa6);
    for mutation in 0..4 {
        let mut p = reviewed(&a, false);
        let Some(ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            controls, ..
        }) = &mut p.configuration_reward_inventory
        else {
            panic!()
        };
        match mutation {
            0 => {
                controls.pop();
            }
            1 => controls[1] = controls[0].clone(),
            2 => controls[1].recipe = key("another-authority"),
            _ => controls[1].selector.name = "another-name".into(),
        }
        let evidence = SourceProjectEvidence::collect(&imported, Default::default()).unwrap();
        assert!(
            normalize_fresh(
                &evidence,
                *imported.allocator_state(),
                NormalizationArtifacts {
                    tree: None,
                    items: &empty_items(&a.schema),
                    item_source: &empty_item_source(&a.schema),
                    mappings: &a.mapping,
                    registry: &a.registry,
                    definitions: &a.schema,
                    roles: &a.roles,
                    rewards: &a.rewards,
                },
                &p,
                &queries(),
                Default::default()
            )
            .is_err()
        );
    }
}
