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
                outcomes: vec![
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
                ],
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
}
fn parameters(input: &mut RewardPolicyInput) -> &mut Vec<ParameterAssignment> {
    let RewardTemplate::Reward { parameters, .. } = &mut input.rules[0].outcomes[1].outcome else {
        unreachable!()
    };
    parameters
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
    input.rules[0].outcomes[0].when = RewardValue::Option(fixture.options[1].clone());
    input.rules[0].outcomes[1].when = RewardValue::Option(fixture.options[0].clone());
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
    input.rules[0].outcomes[1].outcome = RewardTemplate::Unmapped {
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
    input.rules[0].outcomes.clear();
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
    } = &mut input.rules[0].outcomes[1].outcome
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
    let duplicate = input.rules[0].outcomes[0].clone();
    input.rules[0].outcomes.push(duplicate);
    violation(
        fixture.build(input),
        RewardPolicyViolation::DuplicateOutcome,
    );
    let mut input = fixture.input();
    input.rules[0].outcomes[0].when = RewardValue::Option(fixture.options[0].clone());
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
