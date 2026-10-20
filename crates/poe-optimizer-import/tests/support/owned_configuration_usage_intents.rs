//! Configuration transport preserves Scenario ownership and independent presets.
use super::*;

const NAME: &str = "caller-stages";

fn fixture(quantity: bool, selection: RewardNumericSelection) -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p) = numeric_usage_input_tests::fixture(quantity);
    let old = usage_input_tests::row(&mut p).clone();
    let mut schema = a.schema.input().clone();
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::UsagePolicy(row) = definition
            && row.id == old.policies[0].policy
        {
            let SchemaState::Known(row) = &mut row.schema else {
                panic!()
            };
            row.scenario_skill_applicability = Some(DeclaredSet::complete(vec![
                UsageSkillApplicability::Supplied(Box::new(UsageSkillSupply {
                    skill: old.primary.clone(),
                    grant: old.grant.clone(),
                    supply: old.supply.clone(),
                })),
            ]));
        }
    }
    let UsageValueSource::ContainingGroupOverride { occurrence, .. } =
        old.policies[0].parameters[0].source.clone()
    else {
        panic!()
    };
    let recipe = ValueRecipeInput {
        id: key("caller-stage-projection"),
        codec: occurrence.codec,
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
    };
    rebind_quality_schema(&mut a, &mut p, schema);
    let scalar = gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        roles,
        scalar_inputs,
        physical,
        occurrences,
        scenario,
        ..
    } = p.usage_inputs.as_mut().unwrap();
    *definitions = a.schema.identity().clone();
    *roles = *a.roles.identity();
    *scalar_inputs = scalar;
    physical.clear();
    occurrences.clear();
    *scenario = vec![ConfigurationUsageIntentInput {
        policy: old.policies[0].policy.clone(),
        parameters: vec![ConfigurationUsageParameterInput {
            slot: old.policies[0].parameters[0].slot.clone(),
            recipe,
            selection,
        }],
    }];
    (a, p)
}

fn row(p: &mut NormalizationPolicy) -> &mut ConfigurationUsageIntentInput {
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } = p.usage_inputs.as_mut().unwrap();
    &mut scenario[0]
}

fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
}

fn field(element: &str, number: &str) -> String {
    format!(r#"<{element} name="{NAME}" number="{number}"/>"#)
}

fn run(text: &str, a: &Artifacts, p: &NormalizationPolicy) -> NormalizedImport {
    usage_input_tests::run_limits(text, a, p, Default::default()).unwrap()
}

fn number(result: &NormalizedImport, index: usize) -> Option<f64> {
    let rows = &result.draft().input().scenario_presets.members[index]
        .scenario
        .usage_intents
        .members;
    if rows.is_empty() {
        return None;
    }
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].parameters.members.len(), 1);
    match rows[0].parameters.members[0].value.to_resolved().unwrap() {
        ParameterValue::Integer(value) => Some(value.get() as f64),
        ParameterValue::Quantity(value) => Some(value.value()),
        _ => panic!(),
    }
}

fn accounting(a: &Artifacts, p: &mut NormalizationPolicy) {
    let input = row(p).clone();
    let parameters = &input.parameters;
    p.configuration_dispositions = Some(ConfigurationDispositionPolicy {
        mapping_source: *a.mapping.source_identity(),
        catalogue_evidence: *a.mapping.identity(),
        constructor_evidence: *a.rewards.identity(),
        dependencies: configuration_disposition_dependencies(p, &a.rewards, Default::default())
            .unwrap(),
        rules: parameters[0]
            .recipe
            .tiers
            .iter()
            .flat_map(|tier| &tier.selectors)
            .enumerate()
            .map(|(n, selector)| ConfigurationDispositionRule {
                id: key(&format!("stage-lane-{n}")),
                selector: selector.clone(),
                destinations: vec![ConfigurationDestination::Adapter {
                    control: ConfigurationAdapterControl::UsageIntent {
                        policy: input.policy.clone(),
                        parameter: parameters[0].slot.clone(),
                    },
                }],
            })
            .collect(),
    });
}

#[test]
fn independent_configurations_retain_values_without_copying_to_skill_presets() {
    let (a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    let text = format!(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2">{}</ConfigSet></Config><Skills activeSkillSet="8"><SkillSet id="7"/><SkillSet id="8"/></Skills></PathOfBuilding2>"#,
        field("Input", "0"),
        field("Placeholder", "4")
    );
    let projected = run(&text, &a, &p);
    let repeated = run(&text, &a, &p);
    assert_eq!(projected.sidecar().schema_version, 23);
    assert_eq!(projected.draft(), repeated.draft());
    assert_eq!(projected.sidecar().origins, repeated.sidecar().origins);
    assert_eq!(number(&projected, 0), Some(0.));
    assert_eq!(number(&projected, 1), Some(4.));
    let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } = p.usage_inputs.as_mut().unwrap();
    scenario.clear();
    let baseline = run(&text, &a, &p);
    assert_eq!(
        projected.draft().input().skill_presets,
        baseline.draft().input().skill_presets
    );
    assert_eq!(
        projected.draft().input().allocator,
        baseline.draft().input().allocator
    );
    for preset in &projected.draft().input().scenario_presets.members {
        assert!(matches!(
            preset.scenario.usage_intents.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(preset.scenario.usage.members.len(), 0);
    }
}

#[test]
fn both_numeric_modes_validate_losing_lanes_and_preserve_explicit_zero_semantics() {
    for quantity in [false, true] {
        for (selection, cases) in [
            (
                RewardNumericSelection::RecipePrecedence,
                [
                    ("0", "4", Some(0.)),
                    ("3", "0", Some(3.)),
                    ("0", "0", Some(0.)),
                ],
            ),
            (
                RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
                [("0", "4", Some(4.)), ("3", "0", Some(3.)), ("0", "0", None)],
            ),
        ] {
            let (a, p) = fixture(quantity, selection);
            for (input, placeholder, expected) in cases {
                let result = run(
                    &xml(&(field("Input", input) + &field("Placeholder", placeholder))),
                    &a,
                    &p,
                );
                assert_eq!(number(&result, 0), expected);
            }
            for body in [
                String::new(),
                field("Input", "bad"),
                field("Input", "3") + &field("Placeholder", "bad"),
                field("Input", "3") + &field("Placeholder", "21"),
                field("Input", "3") + &field("Placeholder", "-1"),
                field("Input", "21") + &field("Placeholder", "4"),
                field("Input", "3") + &field("Input", "4"),
                field("Input", "3") + &field("Placeholder", "4") + &field("Placeholder", "5"),
                format!(r#"<Input name="{NAME}" string="3"/>"#),
                format!(r#"<Input name="{NAME}" number="3" future="x"/>"#),
                format!(r#"<Input name="{NAME}" number="3"><Future/></Input>"#),
            ] {
                let result = run(&xml(&body), &a, &p);
                assert_eq!(number(&result, 0), None, "{selection:?}: {body}");
                assert!(matches!(
                    result.draft().input().scenario_presets.members[0]
                        .scenario
                        .usage_intents
                        .completion,
                    DraftListCompletion::Pending { .. }
                ));
            }
        }
    }
}

#[test]
fn configuration_projections_require_the_whole_policy_and_current_dependencies() {
    let (a, p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    for mutation in 0..8 {
        let mut bad = p.clone();
        match mutation {
            0 => row(&mut bad).parameters.clear(),
            1 => {
                let duplicate = row(&mut bad).parameters[0].clone();
                row(&mut bad).parameters.push(duplicate);
            }
            2 => {
                let duplicate = row(&mut bad).clone();
                let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
                    bad.usage_inputs.as_mut().unwrap();
                scenario.push(duplicate);
            }
            3 => {
                row(&mut bad).parameters[0].recipe.codec.codec =
                    value_recipe("wrong-type", NAME, true).codec.codec
            }
            4 => {
                row(&mut bad).parameters[0].recipe.tiers[0].selectors[0].lane = ValueLane::Attribute
            }
            5 => {
                accounting(&a, &mut bad);
                row(&mut bad).parameters[0].selection =
                    RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder;
            }
            6 => {
                let UsageInputPolicy::PobOccurrenceUsageV3 { scalar_inputs, .. } =
                    bad.usage_inputs.as_mut().unwrap();
                *scalar_inputs = *a.mapping.identity();
            }
            7 => {
                let foreign = bad.gem_inputs.as_ref().unwrap().gems[0].gem.clone();
                row(&mut bad).parameters[0].slot.declaration = SlotOwnerDefId::Gem(foreign);
            }
            _ => unreachable!(),
        }
        assert!(
            usage_input_tests::run_limits(&xml(&field("Input", "3")), &a, &bad, Default::default())
                .is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn configuration_accounting_preserves_inventory_obligations_and_exact_policy_links() {
    let (a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    accounting(&a, &mut p);
    let policy_id = row(&mut p).policy.clone();
    let text = xml(&(field("Input", "3") + &field("Placeholder", "4")));
    let result = run(&text, &a, &p);
    assert_eq!(number(&result, 0), Some(3.));
    let imported = source(&text, 93);
    let preset = &result.draft().input().scenario_presets.members[0];
    let target = OwnedOriginTarget::ScenarioUsageIntent {
        scenario: preset.id,
        policy: policy_id,
    };
    for occurrence in imported
        .occurrences()
        .iter()
        .filter(|o| matches!(o.name(), "Input" | "Placeholder"))
    {
        let origin = &result.sidecar().origins[occurrence.id().ordinal() as usize];
        assert!(origin.links.contains(&target));
        assert!(
            origin
                .links
                .contains(&OwnedOriginTarget::ScenarioPreset(preset.id))
        );
        assert!(
            !origin
                .links
                .iter()
                .any(|l| matches!(l, OwnedOriginTarget::Issue(_)))
        );
    }
    assert!(matches!(
        preset.scenario.usage_intents.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.scenario.usage.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        result.draft().input().choice_presets.members[0]
            .choices
            .completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn partial_supplied_applicability_preserves_typed_intent_with_unmapped_dependencies() {
    for unmapped in [false, true] {
        let (mut a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
        let policy = row(&mut p).policy.clone();
        let mut schema = a.schema.input().clone();
        let supplied = schema
            .definitions
            .iter_mut()
            .find_map(|definition| {
                let DefinitionDescriptor::UsagePolicy(entry) = definition else {
                    return None;
                };
                if entry.id != policy {
                    return None;
                }
                let SchemaState::Known(shape) = &mut entry.schema else {
                    panic!()
                };
                let applicability = shape.scenario_skill_applicability.as_mut().unwrap();
                let [UsageSkillApplicability::Supplied(supplied)] =
                    applicability.members.as_slice()
                else {
                    panic!("one nonempty supplied relation")
                };
                let supplied = supplied.as_ref().clone();
                applicability.closure = SchemaClosure::Partial {
                    gaps: vec![SchemaGap {
                        subject: subject(&policy),
                        facet: SchemaFacet::StaticLinks,
                        code: key("additional-eligible-sources-unreviewed"),
                    }],
                };
                Some(supplied)
            })
            .unwrap();
        if unmapped {
            for definition in &mut schema.definitions {
                if let DefinitionDescriptor::Skill(entry) = definition
                    && entry.id == supplied.skill
                {
                    *entry = unknown(entry.id.clone());
                }
            }
            for slot in &mut schema.slots {
                match slot {
                    SlotDescriptor::Grant(entry) if entry.id == supplied.grant => {
                        entry.schema = SchemaState::Unmapped {
                            gaps: vec![SchemaGap {
                                subject: SchemaSubject::Slot(GrantSlotDefId::address(&entry.id)),
                                facet: SchemaFacet::StaticLinks,
                                code: key("supplying-grant-unreviewed"),
                            }],
                        };
                    }
                    SlotDescriptor::SkillGrant(entry) if entry.id == supplied.supply => {
                        entry.schema = SchemaState::Unmapped {
                            gaps: vec![SchemaGap {
                                subject: SchemaSubject::Slot(SkillGrantSlotDefId::address(
                                    &entry.id,
                                )),
                                facet: SchemaFacet::StaticLinks,
                                code: key("skill-supply-unreviewed"),
                            }],
                        };
                    }
                    _ => {}
                }
            }
        }
        rebind_quality_schema(&mut a, &mut p, schema);
        let scalar = gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap();
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            definitions,
            roles,
            scalar_inputs,
            ..
        } = p.usage_inputs.as_mut().unwrap();
        *definitions = a.schema.identity().clone();
        *roles = *a.roles.identity();
        *scalar_inputs = scalar;
        let schema_before = a.schema.input().clone();
        let text = xml(&(field("Input", "3") + &field("Placeholder", "4")));
        let projected = run(&text, &a, &p);
        accounting(&a, &mut p);
        let accounted = run(&text, &a, &p);
        assert_eq!(number(&accounted, 0), Some(3.));
        assert_eq!(accounted.draft(), projected.draft());
        assert_eq!(accounted.allocator_after(), projected.allocator_after());
        assert_eq!(a.schema.input(), &schema_before);

        let scenario = &accounted.draft().input().scenario_presets.members[0];
        assert!(scenario.scenario.usage.members.is_empty());
        for completion in [
            &scenario.scenario.assumptions.completion,
            &scenario.scenario.usage.completion,
            &scenario.scenario.usage_intents.completion,
            &accounted.draft().input().choice_presets.members[0]
                .choices
                .completion,
        ] {
            assert!(matches!(completion, DraftListCompletion::Pending { .. }));
        }
        let SchemaLookup::Known(shape) = a.schema.definition(&policy) else {
            panic!()
        };
        let applicability = shape.scenario_skill_applicability.as_ref().unwrap();
        assert!(!applicability.is_complete());
        assert_eq!(
            applicability.members,
            [UsageSkillApplicability::Supplied(Box::new(
                supplied.clone()
            ))]
        );
        if unmapped {
            assert!(matches!(
                a.schema.definition(&supplied.skill),
                SchemaLookup::Unmapped(_)
            ));
            assert!(matches!(
                a.schema.slot(&supplied.grant),
                SchemaLookup::Unmapped(_)
            ));
            assert!(matches!(
                a.schema.slot(&supplied.supply),
                SchemaLookup::Unmapped(_)
            ));
        }

        let DraftListCompletion::Pending { id: issue, .. } =
            accounted.draft().input().choice_presets.members[0]
                .choices
                .completion
        else {
            panic!()
        };
        let imported = source(&text, 93);
        let mut origins = projected.sidecar().origins.clone();
        for occurrence in imported
            .occurrences()
            .iter()
            .filter(|occurrence| matches!(occurrence.name(), "Input" | "Placeholder"))
        {
            let origin = &mut origins[occurrence.id().ordinal() as usize];
            assert_eq!(origin.disposition, SourceDisposition::Contributes);
            assert_eq!(
                origin
                    .links
                    .iter()
                    .filter(|link| **link == OwnedOriginTarget::Issue(issue))
                    .count(),
                1
            );
            assert!(
                origin
                    .links
                    .contains(&OwnedOriginTarget::ScenarioUsageIntent {
                        scenario: scenario.id,
                        policy: policy.clone(),
                    })
            );
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(issue));
            if occurrence.name() == "Placeholder" {
                origin.disposition =
                    SourceDisposition::SourceOnly(key("unselected-config-projection"));
            }
        }
        // Only these exact leaves lose their fallback issue; the losing numeric
        // lane is explicitly source-only and retains its exact intent links.
        // No schema, recipient discovery, whole inventory or execution is closed.
        assert_eq!(accounted.sidecar().origins, origins);
    }
}

#[test]
fn current_policy_requires_explicit_scenario_rows_and_bounded_validation() {
    let (a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    let mut wire = serde_json::to_value(p.usage_inputs.as_ref().unwrap()).unwrap();
    wire.as_object_mut().unwrap().remove("scenario");
    assert!(serde_json::from_value::<UsageInputPolicy>(wire).is_err());
    accounting(&a, &mut p);
    let mut dependencies =
        serde_json::to_value(&p.configuration_dispositions.as_ref().unwrap().dependencies).unwrap();
    dependencies.as_object_mut().unwrap().remove("usage_inputs");
    assert!(serde_json::from_value::<ConfigurationDispositionDependencies>(dependencies).is_err());
    let text = xml(&field("Input", "3"));
    let before = run(&text, &a, &p);
    let limits = NormalizationLimits {
        max_work: 1,
        ..Default::default()
    };
    assert!(usage_input_tests::run_limits(&text, &a, &p, limits).is_err());
    let after = run(&text, &a, &p);
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.sidecar().origins, after.sidecar().origins);
}

#[test]
fn declared_shared_projections_cannot_be_accounted_by_only_one_destination() {
    let (a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    let original = row(&mut p).clone();
    // A second numeric destination on the same policy is invalid until its
    // declaration exists; use a second complete copy of the policy definition.
    let mut schema = a.schema.input().clone();
    let mut a = a;
    let new_policy = a
        .registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    let new_parameter: DeclaredSlot<ParameterSlotDefId> = a
        .registry
        .allocate_slot(SlotOwnerDefId::UsagePolicy(new_policy.clone()))
        .unwrap();
    let mut descriptor = schema
        .definitions
        .iter()
        .find_map(|d| match d {
            DefinitionDescriptor::UsagePolicy(r) if r.id == original.policy => Some(r.clone()),
            _ => None,
        })
        .unwrap();
    descriptor.id = new_policy.clone();
    let SchemaState::Known(shape) = &mut descriptor.schema else {
        panic!()
    };
    shape.declarations.parameters.members = vec![new_parameter.clone()];
    schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(descriptor));
    let mut parameter = schema
        .slots
        .iter()
        .find_map(|s| match s {
            SlotDescriptor::Parameter(r) if r.id == original.parameters[0].slot => Some(r.clone()),
            _ => None,
        })
        .unwrap();
    parameter.id = new_parameter.clone();
    schema.slots.push(SlotDescriptor::Parameter(parameter));
    rebind_quality_schema(&mut a, &mut p, schema);
    let scalar = gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        roles,
        scalar_inputs,
        scenario,
        ..
    } = p.usage_inputs.as_mut().unwrap();
    *definitions = a.schema.identity().clone();
    *roles = *a.roles.identity();
    *scalar_inputs = scalar;
    let mut second = original;
    second.policy = new_policy.clone();
    second.parameters[0].slot = new_parameter.clone();
    scenario.push(second);
    accounting(&a, &mut p);
    let text = xml(&field("Input", "3"));
    assert!(usage_input_tests::run_limits(&text, &a, &p, Default::default()).is_err());
    for disposition in &mut p.configuration_dispositions.as_mut().unwrap().rules {
        disposition
            .destinations
            .push(ConfigurationDestination::Adapter {
                control: ConfigurationAdapterControl::UsageIntent {
                    policy: new_policy.clone(),
                    parameter: new_parameter.clone(),
                },
            });
    }
    let result = run(&text, &a, &p);
    assert_eq!(
        result.draft().input().scenario_presets.members[0]
            .scenario
            .usage_intents
            .members
            .len(),
        2
    );
}

#[path = "owned_configuration_usage_scalar.rs"]
mod scalar_tests;
