//! A shared Config leaf retains every declared semantic responsibility.
use super::*;

fn shared_scalar() -> (Artifacts, NormalizationPolicy, ExternalInputDefId) {
    let (mut a, mut p) = fixture(false, RewardNumericSelection::RecipePrecedence);
    let mut schema = a.schema.input().clone();
    let input = a
        .registry
        .allocate_definition::<ExternalInputDefinition>()
        .unwrap();
    let encounter = a
        .registry
        .allocate_definition::<EncounterDefinition>()
        .unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: input.clone(),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(10).unwrap(),
                }),
                targets: vec![AssumptionTargetKind::Environment],
            }),
        }));
    schema
        .definitions
        .push(DefinitionDescriptor::Encounter(DefinitionEntry {
            id: encounter.clone(),
            schema: SchemaState::Known(EncounterSchema {
                enemy_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(100).unwrap(),
                },
                external_inputs: DeclaredSet::complete(vec![input.clone()]),
            }),
        }));
    rebind_quality_schema(&mut a, &mut p, schema);
    let selector = ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Encounter,
        key: SourceComponent::Text("shared-scalar-context".into()),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    };
    let mut mapping = a.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: selector.clone(),
        outcome: MappingOutcome::Mapped {
            target: subject(&encounter),
            basis: MappingBasis::Exact,
        },
    });
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let schema = a.schema.input().clone();
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
    p.encounter = Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        mapping_source: *a.mapping.source_identity(),
        selector,
        target: encounter.clone(),
        absent_input_names: vec!["explicit-encounter".into()],
    });
    let mut recipe = row(&mut p).parameters[0].recipe.clone();
    recipe.id = key("shared-scalar-output");
    recipe.tiers.truncate(1);
    p.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source: *a.mapping.source_identity(),
        encounter,
        inputs: vec![],
        placeholder_fallback_inputs: vec![],
        option_inputs: vec![],
        default_inputs: vec![ConfigurationDefaultInput {
            source_name: NAME.into(),
            value_input: input.clone(),
            target: ConfigurationInputTarget::Environment,
            recipe,
            constructor_default: ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
            selection: ConfigurationValueSelection::InputOrConstructor,
            ignore_numeric_placeholder: false,
        }],
    });
    accounting(&a, &mut p);
    (a, p, input)
}

#[test]
fn an_intent_cannot_retire_an_unresolved_scalar_responsibility() {
    let (a, mut p, scalar) = shared_scalar();
    assert_eq!(
        a.rewards.rules().len(),
        0,
        "no separate Reward issue can mask this case"
    );
    let zero = xml(&field("Input", "0"));
    assert!(matches!(
        usage_input_tests::run_limits(&zero, &a, &p, Default::default()),
        Err(NormalizationError::Policy(
            "configuration projection responsibilities"
        ))
    ));
    for disposition in &mut p.configuration_dispositions.as_mut().unwrap().rules {
        if disposition.selector.lane == ValueLane::InputNumber {
            disposition
                .destinations
                .push(ConfigurationDestination::Adapter {
                    control: ConfigurationAdapterControl::ScalarInput {
                        input: scalar.clone(),
                    },
                });
        }
    }
    for (value, expected_scalar) in [("0", None), ("3", Some(3))] {
        let text = xml(&field("Input", value));
        let result = run(&text, &a, &p);
        assert_eq!(number(&result, 0), value.parse::<f64>().ok());
        let preset = &result.draft().input().scenario_presets.members[0];
        let actual = preset
            .scenario
            .assumptions
            .members
            .iter()
            .find(|row| row.input.to_resolved().as_ref() == Some(&scalar))
            .map(|row| row.value.to_resolved().unwrap());
        assert_eq!(
            actual,
            expected_scalar.map(|n| { ParameterValue::Integer(BoundedInteger::new(n).unwrap()) })
        );
        let imported = source(&text, 93);
        let origin = imported
            .occurrences()
            .iter()
            .find(|o| o.name() == "Input")
            .unwrap();
        let links = &result.sidecar().origins[origin.id().ordinal() as usize].links;
        assert_eq!(
            links
                .iter()
                .any(|target| matches!(target, OwnedOriginTarget::Issue(_))),
            expected_scalar.is_none(),
            "the leaf remains unresolved until both projections have receipts",
        );
        assert!(matches!(
            preset.scenario.assumptions.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            preset.scenario.usage_intents.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

fn shared_raw_placeholder() -> (Artifacts, NormalizationPolicy) {
    let (mut a, mut p, raw) = shared_scalar();
    p.configuration_dispositions = None;
    row(&mut p).parameters[0].selection =
        RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder;
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        encounter,
        default_inputs,
        ..
    }) = p.configuration_inputs.take()
    else {
        panic!()
    };
    let mut recipe = default_inputs[0].recipe.clone();
    let presence = a
        .registry
        .allocate_definition::<ExternalInputDefinition>()
        .unwrap();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let mut schema = a.schema.input().clone();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }));
    schema
        .definitions
        .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: presence.clone(),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Boolean,
                targets: vec![AssumptionTargetKind::ScenarioEnemyActor],
            }),
        }));
    for definition in &mut schema.definitions {
        match definition {
            DefinitionDescriptor::ExternalInput(row) if row.id == raw => {
                row.schema = SchemaState::Known(ExternalInputSchema {
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(1., unit.clone()).unwrap(),
                        maximum: FiniteQuantity::new(10., unit.clone()).unwrap(),
                    }),
                    targets: vec![AssumptionTargetKind::ScenarioEnemyActor],
                });
            }
            DefinitionDescriptor::Encounter(row) if row.id == encounter => {
                let SchemaState::Known(owner) = &mut row.schema else {
                    panic!()
                };
                owner.external_inputs.members.push(presence.clone());
            }
            _ => {}
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
    recipe.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Scientific,
        unit,
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    p.configuration_inputs = Some(
        ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            mapping_source: *a.mapping.source_identity(),
            encounter,
            inputs: vec![ConfigurationNumericInput {
                source_name: NAME.into(),
                presence_input: presence,
                value_input: raw,
                recipe,
            }],
        },
    );
    (a, p)
}

#[test]
fn automatic_raw_placeholder_accounting_cannot_hide_a_declared_missing_intent() {
    let (a, p) = shared_raw_placeholder();
    assert_eq!(a.rewards.rules().len(), 0);
    let text = xml(&field("Placeholder", "0"));
    for case in 0..3 {
        let mut policy = p.clone();
        match case {
            0 => {}
            1 => {
                let UsageInputPolicy::PobOccurrenceUsageV3 { scenario, .. } =
                    policy.usage_inputs.as_mut().unwrap();
                scenario.clear();
            }
            2 => {
                for tier in &mut row(&mut policy).parameters[0].recipe.tiers {
                    tier.selectors[0].name = "unrelated-stages".into();
                }
            }
            _ => unreachable!(),
        }
        let result = run(&text, &a, &policy);
        assert_eq!(number(&result, 0), None);
        let preset = &result.draft().input().scenario_presets.members[0];
        assert!(
            preset
                .scenario
                .assumptions
                .members
                .iter()
                .any(|row| { row.value.to_resolved() == Some(ParameterValue::Boolean(false)) }),
            "the raw-only presence projection is available in case {case}"
        );
        let imported = source(&text, 93);
        let source = imported
            .occurrences()
            .iter()
            .find(|o| o.name() == "Placeholder")
            .unwrap();
        let origin = &result.sidecar().origins[source.id().ordinal() as usize];
        assert_eq!(
            origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::Issue(_))),
            case == 0,
        );
        if case == 0 {
            assert_eq!(origin.disposition, SourceDisposition::Contributes);
        } else {
            assert_eq!(
                origin.disposition,
                SourceDisposition::SourceOnly(key("overwritten-config-placeholder"))
            );
        }
    }
}
