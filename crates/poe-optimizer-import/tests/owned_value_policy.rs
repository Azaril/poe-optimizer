use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_definitions::*,
};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_source::SourceAttributeRef,
    owned_value::*,
    owned_value_policy::*,
    source_xml::SourceXmlError,
};
use serde_json::json;

fn ns() -> GameVersionNamespace {
    GameVersionNamespace::new("recipe-game", "v1").unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn unit(name: &str) -> UnitDefId {
    UnitDefId::parse(ns(), name).unwrap()
}
fn option(name: &str) -> OptionDefId {
    OptionDefId::parse(ns(), name).unwrap()
}
fn selector(lane: ValueLane, name: &str) -> ValueSelector {
    ValueSelector {
        lane,
        name: name.into(),
    }
}
fn input_selector() -> ValueSelector {
    selector(ValueLane::InputNumber, "level")
}
fn placeholder_selector() -> ValueSelector {
    selector(ValueLane::PlaceholderNumber, "level")
}
fn tier(selectors: Vec<ValueSelector>, duplicates: DuplicatePolicy) -> ValueTier {
    ValueTier {
        selectors,
        duplicates,
    }
}
fn codec(kind: ValueCodecKind) -> ValueCodecInput {
    ValueCodecInput {
        namespace: ns(),
        whitespace: WhitespacePolicy::Exact,
        codec: kind,
    }
}
fn recipe_input() -> ValueRecipeInput {
    ValueRecipeInput {
        id: OwnedDefinitionKey::new("level-policy").unwrap(),
        codec: codec(ValueCodecKind::Integer {
            syntax: DecimalSyntax::Integer,
        }),
        tiers: vec![
            tier(vec![input_selector()], DuplicatePolicy::LastInSourceOrder),
            tier(vec![placeholder_selector()], DuplicatePolicy::Reject),
        ],
        missing: MissingValuePolicy::Explicit { value: integer(99) },
    }
}
fn recipe() -> ValueRecipe {
    ValueRecipe::new(recipe_input(), ValuePolicyLimits::default()).unwrap()
}
fn source(extra: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(decode_build(format!("<PathOfBuilding2><Input number=\"1\"/><Input number=\"2\"/><Input number=\"3\"/>{extra}</PathOfBuilding2>").as_bytes()).unwrap(),
        BuildLineage::from_bytes([0x71;16]),InstanceImportLimits::default()).unwrap()
}
fn origins() -> Vec<SourceAttributeRef> {
    source("")
        .occurrences()
        .iter()
        .filter(|row| row.name() == "Input")
        .map(|row| SourceAttributeRef {
            occurrence: row.id(),
            index: 0,
        })
        .collect()
}
fn fact<'a>(
    selector: &'a ValueSelector,
    origin: SourceAttributeRef,
    text: &'a str,
) -> ValueCandidate<'a> {
    ValueCandidate {
        selector,
        origin,
        value: CandidateValue::Decoded(text),
    }
}
fn selected(decision: ValueDecision<'_>) -> ParameterValue {
    match decision.outcome {
        ValueOutcome::Selected { value, .. } => value,
        other => panic!("expected selected, got {other:?}"),
    }
}

#[test]
fn first_present_tier_wins_over_later_source_order_and_trace_keeps_shadowed_values() {
    let origins = origins();
    let input = input_selector();
    let placeholder = placeholder_selector();
    let decision = recipe()
        .decide(&[
            fact(&placeholder, origins[2], "82"),
            fact(&input, origins[0], "10"),
        ])
        .unwrap();
    assert_eq!(decision.chosen_tier, Some(0));
    assert_eq!(
        decision.recipe,
        OwnedDefinitionKey::new("level-policy").unwrap()
    );
    assert_eq!(decision.matched.len(), 2);
    assert_eq!(decision.matched[0].origin, origins[0]);
    assert_eq!(
        decision.matched[0].disposition,
        ValueMatchDisposition::Selected
    );
    assert_eq!(
        decision.matched[1].disposition,
        ValueMatchDisposition::ShadowedLowerTier
    );
    assert_eq!(selected(decision), integer(10));
}

#[test]
fn last_duplicate_is_source_order_not_candidate_or_selector_order() {
    let origins = origins();
    let input = input_selector();
    let parent = selector(ValueLane::ParentAttribute, "parent-level");
    let mut raw = recipe_input();
    raw.tiers[0].selectors.push(parent.clone());
    let recipe = ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap();
    for facts in [
        vec![
            fact(&input, origins[2], "30"),
            fact(&parent, origins[0], "10"),
        ],
        vec![
            fact(&parent, origins[0], "10"),
            fact(&input, origins[2], "30"),
        ],
    ] {
        let decision = recipe.decide(&facts).unwrap();
        assert_eq!(
            decision.matched[0].disposition,
            ValueMatchDisposition::ShadowedSameTier
        );
        assert_eq!(selected(decision), integer(30));
    }
    let mut origin = origins[0];
    origin.index = 1;
    // Context membership remains the normalizer's responsibility; the utility sorts exact refs.
    assert_eq!(
        selected(
            recipe
                .decide(&[fact(&input, origin, "20"), fact(&input, origins[0], "10")])
                .unwrap()
        ),
        integer(20)
    );
}

#[test]
fn selected_false_zero_and_empty_text_are_present_not_missing() {
    let origins = origins();
    let input = input_selector();
    let placeholder = placeholder_selector();
    assert_eq!(
        selected(
            recipe()
                .decide(&[
                    fact(&input, origins[0], "0"),
                    fact(&placeholder, origins[1], "82")
                ])
                .unwrap()
        ),
        integer(0)
    );
    let mut raw = recipe_input();
    raw.codec = codec(ValueCodecKind::Boolean {
        tokens: vec![
            BooleanToken {
                token: "false".into(),
                value: false,
            },
            BooleanToken {
                token: String::new(),
                value: false,
            },
        ],
    });
    raw.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Boolean(true),
    };
    let recipe = ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap();
    for text in ["false", ""] {
        assert_eq!(
            selected(recipe.decide(&[fact(&input, origins[0], text)]).unwrap()),
            ParameterValue::Boolean(false)
        );
    }
}

#[test]
fn selected_decode_failure_and_unavailable_fact_never_fall_back_or_use_default() {
    let origins = origins();
    let input = input_selector();
    let placeholder = placeholder_selector();
    let recipe = recipe();
    for text in ["", " ", "not-an-integer"] {
        let decision = recipe
            .decide(&[
                fact(&input, origins[0], text),
                fact(&placeholder, origins[1], "82"),
            ])
            .unwrap();
        assert_eq!(decision.chosen_tier, Some(0));
        assert!(matches!(
            decision.outcome,
            ValueOutcome::Pending {
                reason: ValuePendingReason::Decode(ValueDecodeError::MalformedDecimal)
            }
        ));
        let json = serde_json::to_value(&decision).unwrap();
        assert_eq!(
            json["outcome"]["reason"]["diagnostic"]["code"],
            "malformed_decimal"
        );
    }
    let error = SourceXmlError {
        byte_offset: 7,
        reason: "unavailable lexical field".into(),
    };
    let decision = recipe
        .decide(&[
            ValueCandidate {
                selector: &input,
                origin: origins[0],
                value: CandidateValue::Unavailable(&error),
            },
            fact(&placeholder, origins[1], "82"),
        ])
        .unwrap();
    let ValueOutcome::Pending {
        reason: ValuePendingReason::Unavailable(found),
    } = decision.outcome
    else {
        panic!("lost lexical failure")
    };
    assert!(std::ptr::eq(found, &error));
    assert_eq!(
        decision.matched[0].disposition,
        ValueMatchDisposition::Selected
    );
}

#[test]
fn last_policy_selects_before_decode_and_does_not_scan_for_a_successful_value() {
    let origins = origins();
    let input = input_selector();
    let recipe = recipe();
    assert_eq!(
        selected(
            recipe
                .decide(&[
                    fact(&input, origins[0], "bad"),
                    fact(&input, origins[1], "2")
                ])
                .unwrap()
        ),
        integer(2)
    );
    let decision = recipe
        .decide(&[
            fact(&input, origins[0], "2"),
            fact(&input, origins[1], "bad"),
        ])
        .unwrap();
    assert!(matches!(
        decision.outcome,
        ValueOutcome::Pending {
            reason: ValuePendingReason::Decode(_)
        }
    ));
}

#[test]
fn reject_policy_reports_ambiguity_in_first_present_tier_even_when_values_equal() {
    let origins = origins();
    let input = input_selector();
    let placeholder = placeholder_selector();
    let mut raw = recipe_input();
    raw.tiers[0].duplicates = DuplicatePolicy::Reject;
    let recipe = ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap();
    assert_eq!(
        recipe
            .decide(&[fact(&input, origins[1], "2"), fact(&input, origins[0], "2")])
            .unwrap_err(),
        ValuePolicyError::MultipleValues { tier: 0, count: 2 }
    );
    // Duplicate lower-priority values are shadowed, not selected ambiguity.
    assert_eq!(
        selected(
            recipe
                .decide(&[
                    fact(&input, origins[0], "1"),
                    fact(&placeholder, origins[1], "2"),
                    fact(&placeholder, origins[2], "3")
                ])
                .unwrap()
        ),
        integer(1)
    );
}

#[test]
fn duplicate_origins_and_mixed_snapshots_reject_even_for_unmatched_context_facts() {
    let origins = origins();
    let input = input_selector();
    let unknown = selector(ValueLane::Attribute, "unknown");
    let recipe = recipe();
    assert_eq!(
        recipe
            .decide(&[
                fact(&input, origins[0], "1"),
                fact(&unknown, origins[0], "1")
            ])
            .unwrap_err(),
        ValuePolicyError::DuplicateOrigin { origin: origins[0] }
    );
    let other = source("<Other/>");
    let other_origin = SourceAttributeRef {
        occurrence: other
            .occurrences()
            .iter()
            .find(|row| row.name() == "Input")
            .unwrap()
            .id(),
        index: 0,
    };
    assert_eq!(
        recipe
            .decide(&[
                fact(&input, origins[0], "1"),
                fact(&unknown, other_origin, "1")
            ])
            .unwrap_err(),
        ValuePolicyError::MixedSourceSnapshots
    );
}

#[test]
fn no_match_distinguishes_explicit_default_pending_and_absence() {
    let origins = origins();
    let unknown = selector(ValueLane::Attribute, "other");
    let decision = recipe()
        .decide(&[fact(&unknown, origins[0], "17")])
        .unwrap();
    assert_eq!(decision.chosen_tier, None);
    assert!(decision.matched.is_empty());
    assert_eq!(
        decision.outcome,
        ValueOutcome::Defaulted { value: integer(99) }
    );
    for policy in [MissingValuePolicy::Pending, MissingValuePolicy::Absent] {
        let mut raw = recipe_input();
        raw.tiers.clear();
        raw.missing = policy.clone();
        let recipe = ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap();
        let decision = recipe.decide(&[]).unwrap();
        assert_eq!(decision.chosen_tier, None);
        match policy {
            MissingValuePolicy::Pending => assert!(matches!(
                decision.outcome,
                ValueOutcome::Pending {
                    reason: ValuePendingReason::Missing
                }
            )),
            MissingValuePolicy::Absent => assert!(matches!(decision.outcome, ValueOutcome::Absent)),
            _ => unreachable!(),
        }
    }
}

#[test]
fn default_types_namespaces_and_exact_quantity_units_are_validated_before_deciding() {
    let mut raw = recipe_input();
    raw.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Boolean(false),
    };
    assert_eq!(
        ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::DefaultKindMismatch
    );
    let quantity_codec = codec(ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Decimal,
        unit: unit("seconds"),
        scale: RationalScale {
            numerator: BoundedInteger::new(2).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    });
    let mut raw = recipe_input();
    raw.codec = quantity_codec.clone();
    raw.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Quantity(FiniteQuantity::new(3.0, unit("minutes")).unwrap()),
    };
    assert_eq!(
        ValueRecipe::new(raw.clone(), ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::DefaultUnitMismatch
    );
    raw.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Quantity(
            FiniteQuantity::new(
                3.0,
                UnitDefId::parse(GameVersionNamespace::new("other", "v1").unwrap(), "seconds")
                    .unwrap(),
            )
            .unwrap(),
        ),
    };
    assert_eq!(
        ValueRecipe::new(raw.clone(), ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::ForeignDefaultNamespace
    );
    let value = ParameterValue::Quantity(FiniteQuantity::new(3.0, unit("seconds")).unwrap());
    raw.missing = MissingValuePolicy::Explicit {
        value: value.clone(),
    };
    let recipe = ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap();
    assert_eq!(
        recipe.decide(&[]).unwrap().outcome,
        ValueOutcome::Defaulted { value }
    );
    let origins = origins();
    let input = input_selector();
    let ParameterValue::Quantity(value) =
        selected(recipe.decide(&[fact(&input, origins[0], "3")]).unwrap())
    else {
        panic!("not quantity")
    };
    assert_eq!(value.value(), 6.0);
}

#[test]
fn explicit_option_default_need_not_have_a_lexical_token_but_must_share_namespace() {
    let mut raw = recipe_input();
    raw.codec = codec(ValueCodecKind::Option {
        tokens: vec![OptionToken {
            token: "source".into(),
            value: option("source-option"),
        }],
    });
    let value = ParameterValue::Option(option("default-only"));
    raw.missing = MissingValuePolicy::Explicit {
        value: value.clone(),
    };
    assert_eq!(
        ValueRecipe::new(raw.clone(), ValuePolicyLimits::default())
            .unwrap()
            .decide(&[])
            .unwrap()
            .outcome,
        ValueOutcome::Defaulted { value }
    );
    raw.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Option(
            OptionDefId::parse(
                GameVersionNamespace::new("other", "v1").unwrap(),
                "default-only",
            )
            .unwrap(),
        ),
    };
    assert_eq!(
        ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::ForeignDefaultNamespace
    );
}

#[test]
fn malformed_recipes_reject_duplicate_selectors_empty_tiers_and_missing_explicit_fields() {
    let mut raw = recipe_input();
    raw.tiers[1].selectors.push(input_selector());
    assert_eq!(
        ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::DuplicateSelector {
            tier: 1,
            first_tier: 0
        }
    );
    let mut raw = recipe_input();
    raw.tiers[0].selectors.clear();
    assert_eq!(
        ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::EmptyTier { tier: 0 }
    );
    let mut raw = recipe_input();
    raw.tiers[0].selectors[0].name.clear();
    assert_eq!(
        ValueRecipe::new(raw, ValuePolicyLimits::default()).unwrap_err(),
        ValuePolicyError::EmptySelector { tier: 0 }
    );
    let value = serde_json::to_value(recipe_input()).unwrap();
    assert_eq!(
        serde_json::from_value::<ValueRecipeInput>(value.clone()).unwrap(),
        recipe_input()
    );
    let mut unknown = value.clone();
    unknown["unexpected"] = json!(true);
    assert!(serde_json::from_value::<ValueRecipeInput>(unknown).is_err());
    let mut missing = value.clone();
    missing["missing"] = json!({"kind":"explicit"});
    assert!(serde_json::from_value::<ValueRecipeInput>(missing).is_err());
    let mut missing = value;
    missing.as_object_mut().unwrap().remove("missing");
    assert!(serde_json::from_value::<ValueRecipeInput>(missing).is_err());
}

#[test]
fn recipe_and_context_limits_bound_unmatched_inputs_and_complete_trace() {
    assert!(matches!(
        ValueRecipe::new(
            recipe_input(),
            ValuePolicyLimits {
                max_selectors: 1,
                ..ValuePolicyLimits::default()
            }
        ),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::Selectors,
            ..
        })
    ));
    assert!(matches!(
        ValueRecipe::new(
            recipe_input(),
            ValuePolicyLimits {
                max_total_selector_bytes: 9,
                ..ValuePolicyLimits::default()
            }
        ),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::TotalSelectorBytes,
            ..
        })
    ));
    assert!(matches!(
        ValueRecipe::new(
            recipe_input(),
            ValuePolicyLimits {
                max_candidates: 0,
                ..ValuePolicyLimits::default()
            }
        ),
        Err(ValuePolicyError::InvalidLimit { .. })
    ));
    let origins = origins();
    let input = input_selector();
    let unknown = selector(ValueLane::Attribute, "other");
    let recipe = ValueRecipe::new(
        recipe_input(),
        ValuePolicyLimits {
            max_candidates: 1,
            ..ValuePolicyLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        recipe.decide(&[
            fact(&input, origins[0], "1"),
            fact(&unknown, origins[1], "2")
        ]),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::Candidates,
            ..
        })
    ));
    let recipe = ValueRecipe::new(
        recipe_input(),
        ValuePolicyLimits {
            max_trace_entries: 1,
            ..ValuePolicyLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        recipe.decide(&[fact(&input, origins[0], "1"), fact(&input, origins[1], "2")]),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::TraceEntries,
            ..
        })
    ));
    let recipe = ValueRecipe::new(
        recipe_input(),
        ValuePolicyLimits {
            max_total_candidate_bytes: 5,
            ..ValuePolicyLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        recipe.decide(&[fact(&unknown, origins[0], "1")]),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::TotalCandidateBytes,
            ..
        })
    ));
    let recipe = ValueRecipe::new(
        recipe_input(),
        ValuePolicyLimits {
            value: OwnedValueLimits {
                max_source_bytes: 1,
                ..OwnedValueLimits::default()
            },
            ..ValuePolicyLimits::default()
        },
    )
    .unwrap();
    assert!(matches!(
        recipe.decide(&[fact(&unknown, origins[0], "22")]),
        Err(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::CandidateBytes,
            ..
        })
    ));
}
