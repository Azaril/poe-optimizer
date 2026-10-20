//! Injected finite selection and input-schema contracts; no game/source runtime.
use poe_optimizer_core::{
    build_identity::BuildLineage, data::DataIdentity, owned_build::*, owned_definitions::*,
    owned_schema::*,
};
use poe_optimizer_data::owned_schema::*;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_mapping::*,
    owned_reward_policy::*,
    owned_source::SourceAttributeRef,
    owned_value::*,
    owned_value_policy::*,
    source_xml::SourceXmlError,
};

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("reward-test", "v1").unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn selector(name: &str) -> ExternalSelector {
    ExternalSelector::Definition(ExternalOwnerSelector::Reward {
        key: SourceComponent::Text(name.into()),
    })
}
fn declarations(parameters: Vec<DeclaredSlot<ParameterSlotDefId>>) -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(parameters),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn known<I, T>(id: I, schema: T) -> DefinitionEntry<I, T> {
    DefinitionEntry {
        id,
        schema: SchemaState::Known(schema),
    }
}
fn gap(subject: SchemaSubject) -> SchemaGap {
    SchemaGap {
        subject,
        facet: SchemaFacet::InputSchema,
        code: key("not-converted"),
    }
}
struct Fixture {
    registry: OwnedIdRegistry,
    schema: OwnedDefinitionSchemaPackage,
    mapping: OwnedMappingIndex,
    reward: RewardDefId,
    other: RewardDefId,
    parameter: DeclaredSlot<ParameterSlotDefId>,
    options: [OptionDefId; 2],
    units: [UnitDefId; 2],
}
impl Fixture {
    fn new() -> Self {
        let mut registry = OwnedIdRegistry::empty(ns(), OwnedMappingLimits::default()).unwrap();
        let reward: RewardDefId = registry.allocate_definition().unwrap();
        let other: RewardDefId = registry.allocate_definition().unwrap();
        let parameter = registry
            .allocate_slot(SlotOwnerDefId::Reward(reward.clone()))
            .unwrap();
        let options: [OptionDefId; 2] = [
            registry.allocate_definition().unwrap(),
            registry.allocate_definition().unwrap(),
        ];
        let units: [UnitDefId; 2] = [
            registry.allocate_definition().unwrap(),
            registry.allocate_definition().unwrap(),
        ];
        let mut definitions = vec![
            DefinitionDescriptor::Reward(known(
                reward.clone(),
                RewardSchema {
                    declarations: declarations(vec![parameter.clone()]),
                },
            )),
            DefinitionDescriptor::Reward(known(
                other.clone(),
                RewardSchema {
                    declarations: declarations(vec![]),
                },
            )),
        ];
        definitions.extend(
            options
                .iter()
                .cloned()
                .map(|id| DefinitionDescriptor::Option(known(id, OptionSchema {}))),
        );
        definitions.extend(units.iter().cloned().map(|id| {
            DefinitionDescriptor::Unit(known(
                id,
                UnitSchema {
                    dimension: UnitDimension::Count,
                },
            ))
        }));
        let schema = OwnedDefinitionSchemaPackage::new(
            SchemaPackageInput {
                schema_version: OWNED_SCHEMA_PACKAGE_VERSION,
                namespace: ns(),
                release: key("release"),
                semantics_version: key("input-schema-only"),
                definitions,
                slots: vec![SlotDescriptor::Parameter(known(
                    parameter.clone(),
                    ParameterSlotSchema {
                        skill_input: None,
                        value: ValueSchema::Integer(IntegerRange {
                            minimum: BoundedInteger::new(0).unwrap(),
                            maximum: BoundedInteger::new(9).unwrap(),
                        }),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![ParameterSite::RewardParameter],
                    },
                ))],
            },
            OwnedSchemaLimits::default(),
        )
        .unwrap();
        let mapping = OwnedMappingIndex::new(
            MappingPackageInput {
                schema_version: OWNED_MAPPING_PACKAGE_VERSION,
                namespace: ns(),
                registry: registry.identity().unwrap(),
                definitions: schema.identity().clone(),
                source: SourcePin {
                    system: ExternalSourceSystem::PathOfBuilding2,
                    revision: "reviewed-test-source".into(),
                    files: vec![SourceFilePin {
                        path: "injected/identities.json".into(),
                        sha256: "a".repeat(64),
                    }],
                },
                policy_version: key("external-map"),
                entries: [(&reward, "chosen"), (&other, "other")]
                    .into_iter()
                    .map(|(id, name)| MappingEntry {
                        source: selector(name),
                        outcome: MappingOutcome::Mapped {
                            target: SchemaSubject::Definition(id.address()),
                            basis: MappingBasis::Exact,
                        },
                    })
                    .collect(),
            },
            &registry,
            &schema,
            OwnedMappingLimits::default(),
        )
        .unwrap();
        Self {
            registry,
            schema,
            mapping,
            reward,
            other,
            parameter,
            options,
            units,
        }
    }
    fn input(&self) -> RewardPolicyInput {
        RewardPolicyInput {
            schema_version: OWNED_REWARD_POLICY_VERSION,
            namespace: ns(),
            version: key("injected-rewards"),
            definitions: self.schema.identity().clone(),
            mapping: *self.mapping.identity(),
            rules: vec![RewardRuleInput {
                recipe: ValueRecipeInput {
                    numeric_aliases: vec![],
                    id: key("rule"),
                    codec: ValueCodecInput {
                        namespace: ns(),
                        whitespace: WhitespacePolicy::Exact,
                        codec: ValueCodecKind::Boolean {
                            tokens: vec![
                                BooleanToken {
                                    token: "on".into(),
                                    value: true,
                                },
                                BooleanToken {
                                    token: "off".into(),
                                    value: false,
                                },
                            ],
                        },
                    },
                    tiers: vec![ValueTier {
                        selectors: vec![ValueSelector {
                            lane: ValueLane::InputBoolean,
                            name: "caller-choice".into(),
                        }],
                        duplicates: DuplicatePolicy::Reject,
                    }],
                    missing: MissingValuePolicy::Explicit {
                        value: ParameterValue::Boolean(false),
                    },
                },
                output: RewardRuleOutput::Cases(vec![
                    RewardOutcomeCase {
                        when: RewardValue::Boolean(false),
                        outcome: RewardTemplate::None,
                    },
                    RewardOutcomeCase {
                        when: RewardValue::Boolean(true),
                        outcome: RewardTemplate::Reward {
                            selector: selector("chosen"),
                            parameters: vec![ParameterAssignment {
                                slot: self.parameter.clone(),
                                value: integer(3),
                            }],
                        },
                    },
                ]),
            }],
        }
    }
    fn build(&self, input: RewardPolicyInput) -> Result<OwnedRewardPolicy, RewardPolicyError> {
        OwnedRewardPolicy::new(
            input,
            &self.mapping,
            &self.schema,
            RewardPolicyLimits::default(),
        )
    }
    fn rebuild(&mut self, schema: SchemaPackageInput) {
        self.schema =
            OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()).unwrap();
        let mut input = self.mapping.input().clone();
        input.definitions = self.schema.identity().clone();
        input.registry = self.registry.identity().unwrap();
        self.mapping = OwnedMappingIndex::new(
            input,
            &self.registry,
            &self.schema,
            OwnedMappingLimits::default(),
        )
        .unwrap();
    }
    fn slot_schema(&mut self, value: ValueSchema) {
        let mut schema = self.schema.input().clone();
        let SlotDescriptor::Parameter(slot) = &mut schema.slots[0] else {
            unreachable!()
        };
        let SchemaState::Known(slot) = &mut slot.schema else {
            unreachable!()
        };
        slot.value = value;
        self.rebuild(schema);
    }
    fn additional_required_parameter(&mut self) -> DeclaredSlot<ParameterSlotDefId> {
        let parameter = self
            .registry
            .allocate_slot(SlotOwnerDefId::Reward(self.reward.clone()))
            .unwrap();
        let mut input = self.schema.input().clone();
        for descriptor in &mut input.definitions {
            if let DefinitionDescriptor::Reward(row) = descriptor
                && row.id == self.reward
            {
                let SchemaState::Known(schema) = &mut row.schema else {
                    unreachable!()
                };
                schema
                    .declarations
                    .parameters
                    .members
                    .push(parameter.clone());
            }
        }
        input.slots.push(SlotDescriptor::Parameter(known(
            parameter.clone(),
            ParameterSlotSchema {
                skill_input: None,
                value: ValueSchema::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(9).unwrap(),
                }),
                presence: SlotPresence::RequiredOnce,
                sites: vec![ParameterSite::RewardParameter],
            },
        )));
        self.rebuild(input);
        parameter
    }
}
fn parameters(input: &mut RewardPolicyInput) -> &mut Vec<ParameterAssignment> {
    let RewardTemplate::Reward { parameters, .. } = &mut cases(input)[1].outcome else {
        unreachable!()
    };
    parameters
}
fn cases(input: &mut RewardPolicyInput) -> &mut Vec<RewardOutcomeCase> {
    let RewardRuleOutput::Cases(cases) = &mut input.rules[0].output else {
        unreachable!()
    };
    cases
}
fn numeric_input(fixture: &Fixture, selection: RewardNumericSelection) -> RewardPolicyInput {
    let mut input = fixture.input();
    let rule = &mut input.rules[0];
    rule.recipe.codec.codec = ValueCodecKind::Integer {
        syntax: DecimalSyntax::Scientific,
    };
    rule.recipe.missing = MissingValuePolicy::Pending;
    rule.recipe.tiers = [ValueLane::InputNumber, ValueLane::PlaceholderNumber]
        .into_iter()
        .map(|lane| ValueTier {
            selectors: vec![ValueSelector {
                lane,
                name: "caller-count".into(),
            }],
            duplicates: DuplicatePolicy::Reject,
        })
        .collect();
    rule.output = RewardRuleOutput::NumericParameter(Box::new(RewardNumericParameter {
        selector: selector("chosen"),
        parameter: fixture.parameter.clone(),
        fixed_parameters: vec![],
        selection,
    }));
    input
}
fn numeric_decision<'a>(
    policy: &'a OwnedRewardPolicy,
    input: Option<&'a str>,
    placeholder: Option<&'a str>,
) -> RewardDecision<'a> {
    let ids = origins();
    let selectors: Vec<_> = policy.input().rules[0]
        .recipe
        .tiers
        .iter()
        .flat_map(|t| &t.selectors)
        .collect();
    let candidates: Vec<_> = [input, placeholder]
        .into_iter()
        .enumerate()
        .filter_map(|(index, value)| {
            value.map(|value| ValueCandidate {
                selector: selectors[index],
                origin: ids[index],
                value: CandidateValue::Decoded(value),
            })
        })
        .collect();
    policy.decide(&key("rule"), &candidates).unwrap()
}
fn numeric_value(outcome: &RewardOutcome, fixture: &Fixture) -> ParameterValue {
    let RewardOutcome::Reward {
        definition,
        parameters,
    } = outcome
    else {
        panic!("{outcome:?}")
    };
    assert_eq!(definition, &fixture.reward);
    assert_eq!(parameters.len(), 1);
    assert_eq!(parameters[0].slot, fixture.parameter);
    parameters[0].value.clone()
}
fn origins() -> Vec<SourceAttributeRef> {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(b"<PathOfBuilding2><Input a=\"one\"/><Input a=\"two\"/></PathOfBuilding2>")
            .unwrap(),
        BuildLineage::from_bytes([7; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    source
        .occurrences()
        .iter()
        .filter(|row| row.name() == "Input")
        .map(|row| SourceAttributeRef {
            occurrence: row.id(),
            index: 0,
        })
        .collect()
}
fn selected(policy: &OwnedRewardPolicy, text: &str) -> RewardOutcome {
    let input = &policy.input().rules[0].recipe.tiers[0].selectors[0];
    policy
        .decide(
            &key("rule"),
            &[ValueCandidate {
                selector: input,
                origin: origins()[0],
                value: CandidateValue::Decoded(text),
            }],
        )
        .unwrap()
        .outcome
}
fn violation(
    result: Result<OwnedRewardPolicy, RewardPolicyError>,
    expected: RewardPolicyViolation,
) {
    match result.unwrap_err() {
        RewardPolicyError::Invalid { violation, .. } => assert_eq!(violation, expected),
        other => panic!("expected {expected:?}, got {other:?}"),
    }
}

#[test]
fn injected_boolean_and_exact_option_outcomes_bind_without_allocating_occurrences() {
    let fixture = Fixture::new();
    let before = fixture.registry.identity().unwrap();
    let policy = fixture.build(fixture.input()).unwrap();
    assert_eq!(selected(&policy, "off"), RewardOutcome::None);
    assert_eq!(
        selected(&policy, "on"),
        RewardOutcome::Reward {
            definition: fixture.reward.clone(),
            parameters: parameters(&mut fixture.input()).clone()
        }
    );
    let missing = policy.decide(&key("rule"), &[]).unwrap();
    assert!(matches!(
        missing.value.outcome,
        ValueOutcome::Defaulted { .. }
    ));
    assert_eq!(missing.outcome, RewardOutcome::None);
    let mut input = fixture.input();
    input.rules[0].recipe.codec.codec = ValueCodecKind::Option {
        tokens: vec![
            OptionToken {
                token: "Exact option".into(),
                value: fixture.options[0].clone(),
            },
            OptionToken {
                token: "Other option".into(),
                value: fixture.options[1].clone(),
            },
        ],
    };
    input.rules[0].recipe.tiers[0].selectors[0].lane = ValueLane::InputString;
    input.rules[0].recipe.missing = MissingValuePolicy::Pending;
    cases(&mut input)[0].when = RewardValue::Option(fixture.options[1].clone());
    cases(&mut input)[1].when = RewardValue::Option(fixture.options[0].clone());
    let policy = fixture.build(input).unwrap();
    assert!(matches!(
        selected(&policy, "Exact option"),
        RewardOutcome::Reward { .. }
    ));
    assert_eq!(selected(&policy, "Other option"), RewardOutcome::None);
    assert!(matches!(
        selected(&policy, "exact option"),
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::ValuePending
        }
    ));
    assert_eq!(fixture.registry.identity().unwrap(), before);
}

#[test]
fn malformed_winner_and_unavailable_evidence_never_select_a_default_or_lower_tier() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    input.rules[0].recipe.tiers.push(ValueTier {
        selectors: vec![ValueSelector {
            lane: ValueLane::InputString,
            name: "lower".into(),
        }],
        duplicates: DuplicatePolicy::Reject,
    });
    let policy = fixture.build(input).unwrap();
    let origins = origins();
    let selectors = &policy.input().rules[0].recipe.tiers;
    let error = SourceXmlError {
        byte_offset: 3,
        reason: "unavailable lexical field".into(),
    };
    for value in [
        CandidateValue::Decoded("malformed"),
        CandidateValue::Unavailable(&error),
    ] {
        let decision = policy
            .decide(
                &key("rule"),
                &[
                    ValueCandidate {
                        selector: &selectors[0].selectors[0],
                        origin: origins[0],
                        value,
                    },
                    ValueCandidate {
                        selector: &selectors[1].selectors[0],
                        origin: origins[1],
                        value: CandidateValue::Decoded("on"),
                    },
                ],
            )
            .unwrap();
        assert!(matches!(
            decision.outcome,
            RewardOutcome::Unmapped {
                reason: RewardUnmappedReason::ValuePending
            }
        ));
        assert_eq!(decision.value.chosen_tier, Some(0));
        assert_eq!(decision.value.matched.len(), 2);
        assert_eq!(
            decision.value.matched[1].disposition,
            ValueMatchDisposition::ShadowedLowerTier
        );
    }
    let duplicate = ValueCandidate {
        selector: &selectors[0].selectors[0],
        origin: origins[0],
        value: CandidateValue::Decoded("on"),
    };
    assert!(matches!(
        policy.decide(&key("rule"), &[duplicate, duplicate]),
        Err(RewardPolicyError::Value(
            ValuePolicyError::DuplicateOrigin { .. }
        ))
    ));
}

#[test]
fn absent_unmatched_and_explicit_unmapped_are_distinct_from_no_reward() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    input.rules[0].recipe.missing = MissingValuePolicy::Absent;
    cases(&mut input)[1].outcome = RewardTemplate::Unmapped {
        issue: key("caller-gap"),
    };
    let policy = fixture.build(input.clone()).unwrap();
    assert_eq!(
        policy.decide(&key("rule"), &[]).unwrap().outcome,
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::ValueAbsent
        }
    );
    assert_eq!(
        selected(&policy, "on"),
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::Explicit {
                issue: key("caller-gap")
            }
        }
    );
    cases(&mut input).clear();
    assert_eq!(
        selected(&fixture.build(input.clone()).unwrap(), "on"),
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::NoMatchingOutcome
        }
    );
    input.rules.clear();
    let mut invalid_limits = RewardPolicyLimits::default();
    invalid_limits.value.value.max_source_bytes = 0;
    assert!(matches!(
        OwnedRewardPolicy::new(
            input.clone(),
            &fixture.mapping,
            &fixture.schema,
            invalid_limits
        ),
        Err(RewardPolicyError::Value(_))
    ));
    let empty = fixture.build(input).unwrap();
    assert_eq!(empty.rules().len(), 0);
    assert!(matches!(
        empty.decide(&key("rule"), &[]),
        Err(RewardPolicyError::UnknownRule(_))
    ));
}

#[test]
fn mapping_missing_ambiguous_and_unmapped_preserve_uncertainty() {
    let fixture = Fixture::new();
    for outcome in [
        None,
        Some(MappingOutcome::Unmapped {
            issue: key("mapping-gap"),
        }),
        Some(MappingOutcome::Ambiguous {
            candidates: vec![
                SchemaSubject::Definition(fixture.reward.address()),
                SchemaSubject::Definition(fixture.other.address()),
            ],
            issue: key("mapping-choice"),
        }),
    ] {
        let mut raw = fixture.mapping.input().clone();
        raw.entries.retain(|row| row.source != selector("chosen"));
        if let Some(outcome) = outcome {
            raw.entries.push(MappingEntry {
                source: selector("chosen"),
                outcome,
            });
        }
        let mapping = OwnedMappingIndex::new(
            raw,
            &fixture.registry,
            &fixture.schema,
            OwnedMappingLimits::default(),
        )
        .unwrap();
        let mut input = fixture.input();
        input.mapping = *mapping.identity();
        let policy = OwnedRewardPolicy::new(
            input,
            &mapping,
            &fixture.schema,
            RewardPolicyLimits::default(),
        )
        .unwrap();
        assert!(matches!(
            selected(&policy, "on"),
            RewardOutcome::Unmapped { .. }
        ));
    }
}

#[test]
fn known_parameter_contradictions_reject_even_when_another_mapping_is_unknown() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    parameters(&mut input).clear();
    violation(
        fixture.build(input),
        RewardPolicyViolation::RequiredParameterMissing,
    );
    let mut input = fixture.input();
    parameters(&mut input)[0].value = integer(10);
    violation(fixture.build(input), RewardPolicyViolation::OutOfRange);
    let mut input = fixture.input();
    parameters(&mut input)[0].value = ParameterValue::Boolean(true);
    violation(fixture.build(input), RewardPolicyViolation::WrongValueKind);
    let mut input = fixture.input();
    parameters(&mut input)[0].slot.declaration = SlotOwnerDefId::Reward(fixture.other.clone());
    violation(
        fixture.build(input),
        RewardPolicyViolation::WrongParameterOwner,
    );
    let mut input = fixture.input();
    let extra = parameters(&mut input)[0].clone();
    parameters(&mut input).push(extra);
    violation(
        fixture.build(input),
        RewardPolicyViolation::DuplicateParameter,
    );
    let mut input = fixture.input();
    let RewardTemplate::Reward {
        selector: target,
        parameters,
    } = &mut cases(&mut input)[1].outcome
    else {
        unreachable!()
    };
    *target = selector("unknown");
    parameters[0].value = integer(10);
    violation(fixture.build(input), RewardPolicyViolation::OutOfRange);
}

#[test]
fn exact_quantity_units_ranges_and_option_membership_are_checked() {
    let mut fixture = Fixture::new();
    fixture.slot_schema(ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(1.0, fixture.units[0].clone()).unwrap(),
        maximum: FiniteQuantity::new(5.0, fixture.units[0].clone()).unwrap(),
    }));
    let mut input = fixture.input();
    parameters(&mut input)[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(2.0, fixture.units[0].clone()).unwrap());
    assert!(matches!(
        selected(&fixture.build(input.clone()).unwrap(), "on"),
        RewardOutcome::Reward { .. }
    ));
    parameters(&mut input)[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(2.0, fixture.units[1].clone()).unwrap());
    violation(
        fixture.build(input.clone()),
        RewardPolicyViolation::WrongUnit,
    );
    parameters(&mut input)[0].value =
        ParameterValue::Quantity(FiniteQuantity::new(6.0, fixture.units[0].clone()).unwrap());
    violation(fixture.build(input), RewardPolicyViolation::OutOfRange);
    fixture.slot_schema(ValueSchema::Option {
        allowed: DeclaredSet::complete(vec![fixture.options[0].clone()]),
    });
    let mut input = fixture.input();
    parameters(&mut input)[0].value = ParameterValue::Option(fixture.options[1].clone());
    violation(
        fixture.build(input),
        RewardPolicyViolation::OptionNotAllowed,
    );
    fixture.slot_schema(ValueSchema::Option {
        allowed: DeclaredSet::partial(
            vec![fixture.options[0].clone()],
            vec![gap(SchemaSubject::Definition(fixture.reward.address()))],
        ),
    });
    let mut input = fixture.input();
    parameters(&mut input)[0].value = ParameterValue::Option(fixture.options[1].clone());
    assert!(matches!(
        selected(&fixture.build(input).unwrap(), "on"),
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::SchemaPartial { .. }
        }
    ));
}

#[test]
fn unmapped_or_partial_reward_schema_does_not_fabricate_known_coverage() {
    for partial in [false, true] {
        let mut fixture = Fixture::new();
        let mut raw = fixture.schema.input().clone();
        for descriptor in &mut raw.definitions {
            if let DefinitionDescriptor::Reward(row) = descriptor
                && row.id == fixture.reward
            {
                let gaps = vec![gap(SchemaSubject::Definition(fixture.reward.address()))];
                if partial {
                    let SchemaState::Known(schema) = &mut row.schema else {
                        unreachable!()
                    };
                    schema.declarations.parameters.closure = SchemaClosure::Partial { gaps };
                } else {
                    row.schema = SchemaState::Unmapped { gaps };
                }
            }
        }
        fixture.rebuild(raw);
        let policy = fixture.build(fixture.input()).unwrap();
        assert!(matches!(
            selected(&policy, "on"),
            RewardOutcome::Unmapped { .. }
        ));
        assert_eq!(selected(&policy, "off"), RewardOutcome::None);
    }
}

#[test]
fn unsupported_recipes_duplicate_rows_and_foreign_values_reject() {
    let fixture = Fixture::new();
    let mut input = fixture.input();
    input.rules[0].recipe.codec.codec = ValueCodecKind::Integer {
        syntax: DecimalSyntax::Integer,
    };
    violation(
        fixture.build(input),
        RewardPolicyViolation::UnsupportedRecipe,
    );
    let mut input = fixture.input();
    input.rules[0].recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderBoolean;
    violation(fixture.build(input), RewardPolicyViolation::UnsupportedLane);
    let mut input = fixture.input();
    input.rules.push(input.rules[0].clone());
    violation(fixture.build(input), RewardPolicyViolation::DuplicateRule);
    let mut input = fixture.input();
    let duplicate = cases(&mut input)[0].clone();
    cases(&mut input).push(duplicate);
    violation(
        fixture.build(input),
        RewardPolicyViolation::DuplicateOutcome,
    );
    let mut input = fixture.input();
    cases(&mut input)[0].when = RewardValue::Option(fixture.options[0].clone());
    violation(fixture.build(input), RewardPolicyViolation::WrongWhenKind);
    let mut input = fixture.input();
    parameters(&mut input)[0].value = ParameterValue::Option(
        OptionDefId::parse(
            GameVersionNamespace::new("foreign", "v1").unwrap(),
            "option",
        )
        .unwrap(),
    );
    violation(
        fixture.build(input),
        RewardPolicyViolation::ForeignNamespace,
    );
}

#[test]
fn numeric_projection_preserves_explicit_zero_and_skipped_zero_source_authority() {
    let fixture = Fixture::new();
    let ordinary = fixture
        .build(numeric_input(
            &fixture,
            RewardNumericSelection::RecipePrecedence,
        ))
        .unwrap();
    let zero = numeric_decision(&ordinary, Some("0"), Some("7"));
    assert_eq!(numeric_value(&zero.outcome, &fixture), integer(0));
    assert_eq!(zero.numeric.unwrap().selected, Some(origins()[0]));
    let nonzero = fixture
        .build(numeric_input(
            &fixture,
            RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
        ))
        .unwrap();
    for (input, placeholder, expected, selected_index) in [
        (Some("4"), Some("7"), 4, 0),
        (Some("0"), Some("7"), 7, 1),
        (Some("4"), Some("0"), 4, 0),
        (None, Some("7.0"), 7, 1),
    ] {
        let decision = numeric_decision(&nonzero, input, placeholder);
        assert_eq!(
            numeric_value(&decision.outcome, &fixture),
            integer(expected)
        );
        assert_eq!(
            decision.numeric.unwrap().selected,
            Some(origins()[selected_index])
        );
    }
    let fallback = numeric_decision(&nonzero, Some("0"), Some("7"));
    assert!(
        matches!(fallback.value.outcome, ValueOutcome::Selected { origin, .. } if origin == origins()[0])
    );
    assert_eq!(
        fallback.numeric.unwrap().sources,
        vec![
            RewardNumericSource {
                origin: origins()[0],
                disposition: RewardNumericDisposition::ZeroSkipped
            },
            RewardNumericSource {
                origin: origins()[1],
                disposition: RewardNumericDisposition::Selected
            },
        ]
    );
    for (input, placeholder) in [(Some("0"), None), (None, Some("0")), (Some("0"), Some("0"))] {
        let decision = numeric_decision(&nonzero, input, placeholder);
        assert_eq!(
            decision.outcome,
            RewardOutcome::Unmapped {
                reason: RewardUnmappedReason::NoNumericSelection
            }
        );
        let trace = decision.numeric.unwrap();
        assert_eq!(trace.selected, None);
        assert!(
            trace
                .sources
                .iter()
                .all(|s| s.disposition == RewardNumericDisposition::ZeroSkipped)
        );
    }
    let missing = numeric_decision(&nonzero, None, None);
    assert_eq!(
        missing.outcome,
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::ValuePending
        }
    );
    assert!(missing.numeric.unwrap().sources.is_empty());
}

#[test]
fn destination_neutral_numeric_selection_preserves_reward_decisions_and_trace_wire() {
    use poe_optimizer_import::owned_configuration_numeric::{
        ConfigurationNumericOutcome, ConfigurationNumericRecipe,
    };
    let fixture = Fixture::new();
    let domain = ValueSchema::Integer(IntegerRange {
        minimum: BoundedInteger::new(0).unwrap(),
        maximum: BoundedInteger::new(9).unwrap(),
    });
    for selection in [
        RewardNumericSelection::RecipePrecedence,
        RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
    ] {
        let input = numeric_input(&fixture, selection);
        let shared = ConfigurationNumericRecipe::new(
            input.rules[0].recipe.clone(),
            selection,
            Default::default(),
        )
        .unwrap();
        let policy = fixture.build(input).unwrap();
        let ids = origins();
        for values in [
            [Some("0"), Some("7")],
            [Some("4"), Some("0")],
            [Some("0"), Some("0")],
            [Some("4"), Some("NaN")],
            [Some("4"), Some("10")],
            [None, Some("7.0")],
            [None, None],
        ] {
            let candidates: Vec<_> = values
                .iter()
                .enumerate()
                .filter_map(|(i, text)| {
                    text.map(|text| ValueCandidate {
                        selector: &shared.input().tiers[i].selectors[0],
                        origin: ids[i],
                        value: CandidateValue::Decoded(text),
                    })
                })
                .collect();
            let numeric = shared.decide(&candidates, Some(&domain)).unwrap();
            let reward = numeric_decision(&policy, values[0], values[1]);
            assert_eq!(numeric.value, reward.value);
            assert_eq!(
                serde_json::to_vec(&numeric.numeric).unwrap(),
                serde_json::to_vec(reward.numeric.as_ref().unwrap()).unwrap(),
                "the Reward public source trace retains its exact wire"
            );
            match numeric.outcome {
                ConfigurationNumericOutcome::Selected { value } => {
                    assert_eq!(numeric_value(&reward.outcome, &fixture), value);
                }
                ConfigurationNumericOutcome::Unavailable { reason } => {
                    assert_eq!(numeric.numeric.selected, None);
                    let RewardOutcome::Unmapped {
                        reason: reward_reason,
                    } = reward.outcome
                    else {
                        panic!("unavailable selection cannot emit a Reward")
                    };
                    assert_eq!(
                        serde_json::to_value(reason).unwrap(),
                        serde_json::to_value(reward_reason).unwrap()["kind"],
                        "destination mapping preserves the exact failure reason"
                    );
                }
            }
        }
    }
}

#[test]
fn numeric_projection_checks_losing_values_and_refuses_defaults_and_ambiguous_selection() {
    let fixture = Fixture::new();
    for selection in [
        RewardNumericSelection::RecipePrecedence,
        RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
    ] {
        let policy = fixture.build(numeric_input(&fixture, selection)).unwrap();
        for malformed in ["NaN", "", "1.5", "bad"] {
            for (input, placeholder) in [(Some("4"), Some(malformed)), (Some(malformed), Some("7"))]
            {
                let decision = numeric_decision(&policy, input, placeholder);
                assert_eq!(
                    decision.outcome,
                    RewardOutcome::Unmapped {
                        reason: RewardUnmappedReason::InvalidNumericCandidate
                    }
                );
                assert_eq!(decision.numeric.unwrap().selected, None);
            }
        }
        for outside in ["-1", "10"] {
            let decision = numeric_decision(&policy, Some("4"), Some(outside));
            assert_eq!(
                decision.outcome,
                RewardOutcome::Unmapped {
                    reason: RewardUnmappedReason::ProjectedValueOutOfRange
                }
            );
            assert_eq!(decision.numeric.unwrap().selected, None);
        }
    }
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    input.rules[0].recipe.missing = MissingValuePolicy::Explicit { value: integer(0) };
    violation(fixture.build(input), RewardPolicyViolation::NumericDefault);
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    input.rules[0].recipe.tiers[1].duplicates = DuplicatePolicy::LastInSourceOrder;
    violation(
        fixture.build(input),
        RewardPolicyViolation::UnsupportedSelection,
    );
    for lane in [
        ValueLane::Attribute,
        ValueLane::InputString,
        ValueLane::PlaceholderBoolean,
    ] {
        let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
        input.rules[0].recipe.tiers[0].selectors[0].lane = lane;
        violation(fixture.build(input), RewardPolicyViolation::UnsupportedLane);
    }
    let mut input = numeric_input(
        &fixture,
        RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
    );
    input.rules[0].recipe.tiers[1].selectors[0].name = "other-count".into();
    violation(
        fixture.build(input),
        RewardPolicyViolation::UnsupportedSelection,
    );
}

#[test]
fn numeric_projection_rejects_duplicate_losing_lanes_and_unavailable_evidence() {
    let fixture = Fixture::new();
    let policy = fixture
        .build(numeric_input(
            &fixture,
            RewardNumericSelection::NonzeroInputThenNonzeroPlaceholder,
        ))
        .unwrap();
    let selectors: Vec<_> = policy.input().rules[0]
        .recipe
        .tiers
        .iter()
        .flat_map(|t| &t.selectors)
        .collect();
    let ids = origins();
    let extra = SourceAttributeRef {
        occurrence: ids[1].occurrence,
        index: 1,
    };
    let candidates = [
        ValueCandidate {
            selector: selectors[0],
            origin: ids[0],
            value: CandidateValue::Decoded("4"),
        },
        ValueCandidate {
            selector: selectors[1],
            origin: ids[1],
            value: CandidateValue::Decoded("7"),
        },
        ValueCandidate {
            selector: selectors[1],
            origin: extra,
            value: CandidateValue::Decoded("0"),
        },
    ];
    assert!(matches!(
        policy.decide(&key("rule"), &candidates),
        Err(RewardPolicyError::Value(ValuePolicyError::MultipleValues {
            tier: 1,
            count: 2
        }))
    ));
    let unavailable = SourceXmlError {
        byte_offset: 0,
        reason: "unavailable".into(),
    };
    let candidates = [
        ValueCandidate {
            selector: selectors[0],
            origin: ids[0],
            value: CandidateValue::Decoded("4"),
        },
        ValueCandidate {
            selector: selectors[1],
            origin: ids[1],
            value: CandidateValue::Unavailable(&unavailable),
        },
    ];
    assert_eq!(
        policy.decide(&key("rule"), &candidates).unwrap().outcome,
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::InvalidNumericCandidate
        }
    );
}

#[test]
fn numeric_projection_validates_owner_site_required_slots_and_units_before_admission() {
    let mut fixture = Fixture::new();
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    let RewardRuleOutput::NumericParameter(numeric) = &mut input.rules[0].output else {
        unreachable!()
    };
    numeric.parameter.declaration = SlotOwnerDefId::Reward(fixture.other.clone());
    violation(
        fixture.build(input),
        RewardPolicyViolation::WrongParameterOwner,
    );
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    let RewardRuleOutput::NumericParameter(numeric) = &mut input.rules[0].output else {
        unreachable!()
    };
    numeric.fixed_parameters.push(ParameterAssignment {
        slot: fixture.parameter.clone(),
        value: integer(3),
    });
    violation(
        fixture.build(input),
        RewardPolicyViolation::DuplicateParameter,
    );
    fixture.slot_schema(ValueSchema::Boolean);
    violation(
        fixture.build(numeric_input(
            &fixture,
            RewardNumericSelection::RecipePrecedence,
        )),
        RewardPolicyViolation::WrongValueKind,
    );
    fixture.slot_schema(ValueSchema::Quantity(QuantityRange {
        minimum: FiniteQuantity::new(0., fixture.units[0].clone()).unwrap(),
        maximum: FiniteQuantity::new(9., fixture.units[0].clone()).unwrap(),
    }));
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    input.rules[0].recipe.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Decimal,
        unit: fixture.units[0].clone(),
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(2).unwrap(),
        },
    };
    let policy = fixture.build(input.clone()).unwrap();
    assert_eq!(
        numeric_value(
            &numeric_decision(&policy, Some("7"), None).outcome,
            &fixture
        ),
        ParameterValue::Quantity(FiniteQuantity::new(3.5, fixture.units[0].clone()).unwrap())
    );
    let ValueCodecKind::Quantity { unit, .. } = &mut input.rules[0].recipe.codec.codec else {
        unreachable!()
    };
    *unit = fixture.units[1].clone();
    violation(fixture.build(input), RewardPolicyViolation::WrongUnit);
}

#[test]
fn numeric_projection_keeps_required_fixed_parameters_and_unknown_schema_blocking() {
    let mut fixture = Fixture::new();
    let extra = fixture.additional_required_parameter();
    violation(
        fixture.build(numeric_input(
            &fixture,
            RewardNumericSelection::RecipePrecedence,
        )),
        RewardPolicyViolation::RequiredParameterMissing,
    );
    let mut input = numeric_input(&fixture, RewardNumericSelection::RecipePrecedence);
    let RewardRuleOutput::NumericParameter(numeric) = &mut input.rules[0].output else {
        unreachable!()
    };
    numeric.fixed_parameters.push(ParameterAssignment {
        slot: extra.clone(),
        value: integer(2),
    });
    let policy = fixture.build(input.clone()).unwrap();
    assert_eq!(
        numeric_decision(&policy, Some("9"), Some("0")).outcome,
        RewardOutcome::Reward {
            definition: fixture.reward.clone(),
            parameters: vec![
                ParameterAssignment {
                    slot: extra,
                    value: integer(2)
                },
                ParameterAssignment {
                    slot: fixture.parameter.clone(),
                    value: integer(9)
                }
            ],
        }
    );
    let tight = RewardPolicyLimits {
        max_parameters: 1,
        ..Default::default()
    };
    assert!(matches!(
        OwnedRewardPolicy::new(input.clone(), &fixture.mapping, &fixture.schema, tight),
        Err(RewardPolicyError::Limit("parameters"))
    ));
    assert!(matches!(
        encode_reward_policy(&policy, tight),
        Err(RewardPolicyError::Limit("parameters"))
    ));

    let mut raw = fixture.schema.input().clone();
    let SlotDescriptor::Parameter(row) = &mut raw.slots[0] else {
        unreachable!()
    };
    let SchemaState::Known(slot) = &mut row.schema else {
        unreachable!()
    };
    // A foreign site is rejected by the schema boundary itself. A schema with
    // no admitted sites is structurally valid, but cannot authorize projection.
    slot.sites.clear();
    fixture.rebuild(raw);
    input.definitions = fixture.schema.identity().clone();
    input.mapping = *fixture.mapping.identity();
    violation(
        fixture.build(input),
        RewardPolicyViolation::WrongParameterSite,
    );

    for partial in [false, true] {
        let mut fixture = Fixture::new();
        let mut raw = fixture.schema.input().clone();
        if partial {
            for descriptor in &mut raw.definitions {
                if let DefinitionDescriptor::Reward(row) = descriptor
                    && row.id == fixture.reward
                {
                    let SchemaState::Known(schema) = &mut row.schema else {
                        unreachable!()
                    };
                    schema.declarations.parameters = DeclaredSet::partial(
                        vec![fixture.parameter.clone()],
                        vec![gap(SchemaSubject::Definition(fixture.reward.address()))],
                    );
                }
            }
        } else {
            let SlotDescriptor::Parameter(row) = &mut raw.slots[0] else {
                unreachable!()
            };
            row.schema = SchemaState::Unmapped {
                gaps: vec![gap(SchemaSubject::Slot(ParameterSlotDefId::address(
                    &fixture.parameter,
                )))],
            };
        }
        fixture.rebuild(raw);
        let policy = fixture
            .build(numeric_input(
                &fixture,
                RewardNumericSelection::RecipePrecedence,
            ))
            .unwrap();
        assert!(matches!(
            numeric_decision(&policy, Some("4"), None).outcome,
            RewardOutcome::Unmapped { .. }
        ));
    }
}

#[test]
fn finite_wire_identity_and_strict_numeric_union_are_independent() {
    #[derive(serde::Serialize)]
    struct FiniteRule<'a> {
        recipe: &'a ValueRecipeInput,
        outcomes: &'a [RewardOutcomeCase],
    }
    #[derive(serde::Serialize)]
    struct Policy<'a, R> {
        schema_version: u32,
        namespace: &'a GameVersionNamespace,
        version: &'a OwnedDefinitionKey,
        definitions: &'a DataIdentity,
        mapping: &'a poe_optimizer_core::owned_content::OwnedContentDigest,
        rules: Vec<R>,
    }
    let fixture = Fixture::new();
    let input = fixture.input();
    let finite = Policy {
        schema_version: input.schema_version,
        namespace: &input.namespace,
        version: &input.version,
        definitions: &input.definitions,
        mapping: &input.mapping,
        rules: input
            .rules
            .iter()
            .map(|r| {
                let RewardRuleOutput::Cases(outcomes) = &r.output else {
                    unreachable!()
                };
                FiniteRule {
                    recipe: &r.recipe,
                    outcomes,
                }
            })
            .collect(),
    };
    let policy = fixture.build(input.clone()).unwrap();
    let expected = serde_json::to_vec(&finite).unwrap();
    assert_eq!(
        encode_reward_policy(&policy, Default::default()).unwrap(),
        expected
    );
    assert_eq!(
        policy.identity(),
        &poe_optimizer_core::owned_content::digest_owned(
            "owned-reward-policy-v1",
            &finite,
            RewardPolicyLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    let numeric = fixture
        .build(numeric_input(
            &fixture,
            RewardNumericSelection::RecipePrecedence,
        ))
        .unwrap();
    let encoded = encode_reward_policy(&numeric, Default::default()).unwrap();
    // Independent declaration of the existing numeric object proves boxing has
    // changed neither field order/shape nor the complete policy commitment.
    #[derive(serde::Serialize)]
    struct NumericOutput<'a> {
        selector: &'a ExternalSelector,
        parameter: &'a DeclaredSlot<ParameterSlotDefId>,
        fixed_parameters: &'a [ParameterAssignment],
        selection: RewardNumericSelection,
    }
    #[derive(serde::Serialize)]
    struct NumericRule<'a> {
        recipe: &'a ValueRecipeInput,
        outcomes: NumericOutput<'a>,
    }
    let input = numeric.input();
    let numeric_wire = Policy {
        schema_version: input.schema_version,
        namespace: &input.namespace,
        version: &input.version,
        definitions: &input.definitions,
        mapping: &input.mapping,
        rules: input
            .rules
            .iter()
            .map(|r| {
                let RewardRuleOutput::NumericParameter(n) = &r.output else {
                    unreachable!()
                };
                NumericRule {
                    recipe: &r.recipe,
                    outcomes: NumericOutput {
                        selector: &n.selector,
                        parameter: &n.parameter,
                        fixed_parameters: &n.fixed_parameters,
                        selection: n.selection,
                    },
                }
            })
            .collect(),
    };
    assert_eq!(encoded, serde_json::to_vec(&numeric_wire).unwrap());
    assert_eq!(
        numeric.identity(),
        &poe_optimizer_core::owned_content::digest_owned(
            "owned-reward-policy-v1",
            &numeric_wire,
            RewardPolicyLimits::default().max_wire_bytes
        )
        .unwrap()
    );
    let decoded = decode_reward_policy(
        &encoded,
        &fixture.mapping,
        &fixture.schema,
        Default::default(),
    )
    .unwrap();
    assert_eq!(numeric.identity(), decoded.identity());
    let base: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
    assert!(base["rules"][0]["outcomes"].is_object());
    for field in ["unknown", "cases", "outcomes"] {
        let mut malformed = base.clone();
        malformed["rules"][0]["outcomes"][field] = serde_json::json!([]);
        assert!(
            decode_reward_policy(
                &serde_json::to_vec(&malformed).unwrap(),
                &fixture.mapping,
                &fixture.schema,
                Default::default()
            )
            .is_err()
        );
    }
    let mut missing = base.clone();
    missing["rules"][0]["outcomes"]
        .as_object_mut()
        .unwrap()
        .remove("selection");
    assert!(
        decode_reward_policy(
            &serde_json::to_vec(&missing).unwrap(),
            &fixture.mapping,
            &fixture.schema,
            Default::default()
        )
        .is_err()
    );
    let mut mixed = base;
    mixed["rules"][0]["outcomes"] = serde_json::json!([mixed["rules"][0]["outcomes"].clone()]);
    assert!(
        decode_reward_policy(
            &serde_json::to_vec(&mixed).unwrap(),
            &fixture.mapping,
            &fixture.schema,
            Default::default()
        )
        .is_err()
    );
}

#[test]
fn strict_codec_exact_bindings_and_resource_ceilings() {
    let fixture = Fixture::new();
    let policy = fixture.build(fixture.input()).unwrap();
    let limits = RewardPolicyLimits::default();
    let bytes = encode_reward_policy(&policy, limits).unwrap();
    let restored = decode_reward_policy(&bytes, &fixture.mapping, &fixture.schema, limits).unwrap();
    assert_eq!(policy.input(), restored.input());
    assert_eq!(policy.identity(), restored.identity());
    policy
        .verify_bindings(&fixture.mapping, &fixture.schema)
        .unwrap();
    for invalid in [
        format!(
            "{{\"unexpected\":true,{}",
            std::str::from_utf8(&bytes).unwrap().trim_start_matches('{')
        ),
        format!(
            "{{\"schema_version\":1,{}",
            std::str::from_utf8(&bytes).unwrap().trim_start_matches('{')
        ),
    ] {
        assert!(
            decode_reward_policy(
                invalid.as_bytes(),
                &fixture.mapping,
                &fixture.schema,
                limits
            )
            .is_err()
        );
    }
    let mut input = fixture.input();
    input.mapping = "0".repeat(64).parse().unwrap();
    assert!(matches!(
        fixture.build(input),
        Err(RewardPolicyError::Binding)
    ));
    let setters: [fn(&mut RewardPolicyLimits, usize); 5] = [
        |v, n| v.max_rules = n,
        |v, n| v.max_outcomes = n,
        |v, n| v.max_parameters = n,
        |v, n| v.max_schema_work = n,
        |v, n| v.max_wire_bytes = n,
    ];
    for set in setters {
        for n in [0, usize::MAX] {
            let mut tight = limits;
            set(&mut tight, n);
            assert!(matches!(
                OwnedRewardPolicy::new(fixture.input(), &fixture.mapping, &fixture.schema, tight),
                Err(RewardPolicyError::InvalidLimit(_))
            ));
        }
    }
    let mut tight = limits;
    tight.max_outcomes = 1;
    assert!(matches!(
        OwnedRewardPolicy::new(fixture.input(), &fixture.mapping, &fixture.schema, tight),
        Err(RewardPolicyError::Limit("outcomes"))
    ));
    let mut tight = limits;
    tight.max_schema_work = 1;
    assert!(matches!(
        OwnedRewardPolicy::new(fixture.input(), &fixture.mapping, &fixture.schema, tight),
        Err(RewardPolicyError::Limit("schema work"))
    ));
    let mut tight = limits;
    tight.max_wire_bytes = bytes.len() - 1;
    assert!(decode_reward_policy(&bytes, &fixture.mapping, &fixture.schema, tight).is_err());
    let mut input = fixture.input();
    let mut second = input.rules[0].clone();
    second.recipe.id = key("second");
    input.rules.push(second);
    let mut tight = limits;
    tight.max_rules = 1;
    assert!(matches!(
        OwnedRewardPolicy::new(input.clone(), &fixture.mapping, &fixture.schema, tight),
        Err(RewardPolicyError::Limit("rules"))
    ));
    let mut tight = limits;
    tight.max_parameters = 1;
    assert!(matches!(
        OwnedRewardPolicy::new(input, &fixture.mapping, &fixture.schema, tight),
        Err(RewardPolicyError::Limit("parameters"))
    ));
}

struct Faulty<'a> {
    schema: &'a OwnedDefinitionSchemaPackage,
    hidden: DefinitionAddress,
    inconsistent: bool,
}
impl DefinitionSchemaIndex for Faulty<'_> {
    fn identity(&self) -> &DataIdentity {
        self.schema.identity()
    }
    fn namespace(&self) -> &GameVersionNamespace {
        self.schema.namespace()
    }
    fn lookup_definition(&self, address: &DefinitionAddress) -> Option<&DefinitionDescriptor> {
        if address == &self.hidden {
            if self.inconsistent {
                self.schema
                    .input()
                    .definitions
                    .iter()
                    .find(|row| row.address() != *address)
            } else {
                None
            }
        } else {
            self.schema.lookup_definition(address)
        }
    }
    fn lookup_slot(&self, address: &SlotAddress) -> Option<&SlotDescriptor> {
        self.schema.lookup_slot(address)
    }
}
#[test]
fn missing_schema_and_inconsistent_index_are_distinct() {
    let fixture = Fixture::new();
    let mut schema = Faulty {
        schema: &fixture.schema,
        hidden: fixture.reward.address(),
        inconsistent: false,
    };
    let policy = OwnedRewardPolicy::new(
        fixture.input(),
        &fixture.mapping,
        &schema,
        RewardPolicyLimits::default(),
    )
    .unwrap();
    assert!(matches!(
        selected(&policy, "on"),
        RewardOutcome::Unmapped {
            reason: RewardUnmappedReason::DefinitionMissing { .. }
        }
    ));
    schema.inconsistent = true;
    assert!(matches!(
        OwnedRewardPolicy::new(
            fixture.input(),
            &fixture.mapping,
            &schema,
            RewardPolicyLimits::default()
        ),
        Err(RewardPolicyError::IndexFault { .. })
    ));
}
