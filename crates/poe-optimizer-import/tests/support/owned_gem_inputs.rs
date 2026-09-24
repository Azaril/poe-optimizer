//! Generic source admission and declared intrinsic parameters, without game labels.
use super::*;

fn only_gem(result: &NormalizedImport) -> &GemDraft {
    assert_eq!(result.draft().input().gems.members.len(), 1);
    &result.draft().input().gems.members[0]
}
fn is_complete(result: &NormalizedImport) -> bool {
    matches!(
        only_gem(result).parameters.completion,
        DraftListCompletion::Complete
    )
}
fn empty_fixture() -> (Artifacts, NormalizationPolicy) {
    let mut a = artifacts(true);
    replace_gem_input_schema(
        &mut a,
        AuthoredGemRole::SkillUse,
        GemParameterShape::Empty,
        false,
    );
    let policy = explicit_empty_gem_policy(&a);
    (a, policy)
}
fn xml(attributes: &str) -> String {
    group(&format!(r#"<Gem gemId="active" variantId="v" {attributes}/ >"#).replace("/ >", "/>"))
}

#[test]
fn omitted_policy_keeps_legacy_wire_identity_but_requires_explicit_source_proof() {
    let (a, reviewed) = empty_fixture();
    let legacy = policy();
    let serialized = serde_json::to_value(&legacy).unwrap();
    assert!(serialized.get("gem_inputs").is_none());
    let decoded: NormalizationPolicy = serde_json::from_value(serialized.clone()).unwrap();
    assert_eq!(decoded, legacy);
    let canonical = serde_json::to_vec(&legacy).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), canonical);
    use poe_optimizer_core::owned_content::digest_owned;
    assert_eq!(
        digest_owned(
            "owned-normalization-policy-v3",
            &(&legacy, &queries()),
            1 << 20
        )
        .unwrap(),
        digest_owned(
            "owned-normalization-policy-v3",
            &(&decoded, &queries()),
            1 << 20
        )
        .unwrap()
    );
    for attributes in [
        "",
        r#"neutral-input="false""#,
        r#"corrupted="true" corruptLevel="1""#,
    ] {
        let result = normalize_with_loadouts(&xml(attributes), &a, &legacy).unwrap();
        assert!(!is_complete(&result));
        assert_eq!(result.sidecar().schema_version, 12);
    }
    assert!(is_complete(
        &normalize_with_loadouts(&xml(""), &a, &reviewed).unwrap()
    ));
    let mut unlisted = reviewed.clone();
    unlisted.gem_inputs.as_mut().unwrap().gems.clear();
    assert!(!is_complete(
        &normalize_with_loadouts(&xml(""), &a, &unlisted).unwrap()
    ));
}

#[test]
fn neutral_guards_distinguish_missing_empty_known_unknown_and_unavailable_values() {
    let (a, mut policy) = empty_fixture();
    policy.gem_inputs.as_mut().unwrap().gems[0].guards = vec![
        GemInputGuard {
            attribute: "flag".into(),
            allowed: vec![
                SourceComponent::Text("false".into()),
                SourceComponent::Text("nil".into()),
            ],
        },
        GemInputGuard {
            attribute: "delta".into(),
            allowed: vec![SourceComponent::Text("0".into())],
        },
    ];
    for (attrs, expected) in [
        (r#"flag="false" delta="0""#, true),
        (r#"flag="nil" delta="0""#, true),
        (r#"flag="true" delta="0""#, false),
        (r#"flag="false" delta="1""#, false),
        (r#"flag="false" delta="+0""#, false),
        (r#"flag="false" delta="nil""#, false),
        (r#"delta="0""#, false),
        (r#"flag="" delta="0""#, false),
        (r#"flag="false" delta="&#48;""#, false),
    ] {
        let result = normalize_with_loadouts(&xml(attrs), &a, &policy).unwrap();
        assert_eq!(is_complete(&result), expected, "{attrs}");
    }
    for attrs in [
        r#"xmlns:p="urn:other" p:flag="false" delta="0""#,
        r#"xmlns="urn:other" flag="false" delta="0""#,
    ] {
        let source = source(&xml(attrs), 93);
        let result = run_with_policy(&source, &a, &[], &policy);
        assert!(
            result.draft().input().gems.members.is_empty(),
            "namespaced rows are not physical gems: {attrs}"
        );
        let evidence =
            SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
        let rows: Vec<_> = evidence
            .rows()
            .iter()
            .filter(|row| row.occurrence().name() == "Gem")
            .collect();
        assert_eq!(rows.len(), 1);
        assert!(rows[0].occurrence().has_namespace_context());
        assert!(
            result
                .draft()
                .input()
                .skill_presets
                .members
                .iter()
                .all(|preset| matches!(
                    preset.skills.completion,
                    DraftListCompletion::Pending { .. }
                ))
        );
    }
    let rule = &mut policy.gem_inputs.as_mut().unwrap().gems[0];
    rule.guards[0].allowed = vec![SourceComponent::Missing];
    assert!(is_complete(
        &normalize_with_loadouts(&xml(r#"delta="0""#), &a, &policy).unwrap()
    ));
    assert!(!is_complete(
        &normalize_with_loadouts(&xml(r#"flag="" delta="0""#), &a, &policy).unwrap()
    ));
    policy.gem_inputs.as_mut().unwrap().gems[0].guards[0].allowed =
        vec![SourceComponent::Text(String::new())];
    assert!(!is_complete(
        &normalize_with_loadouts(&xml(r#"delta="0""#), &a, &policy).unwrap()
    ));
    assert!(is_complete(
        &normalize_with_loadouts(&xml(r#"flag="" delta="0""#), &a, &policy).unwrap()
    ));
}

fn projection_fixture() -> (Artifacts, NormalizationPolicy, Vec<ParameterValue>) {
    let (mut a, mut policy) = empty_fixture();
    let gem = policy.gem_inputs.as_ref().unwrap().gems[0].gem.clone();
    let owner = SlotOwnerDefId::Gem(gem.clone());
    let mut schema = a.schema.input().clone();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let option = a
        .registry
        .allocate_definition::<OptionDefinition>()
        .unwrap();
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
        .push(DefinitionDescriptor::Option(DefinitionEntry {
            id: option.clone(),
            schema: SchemaState::Known(OptionSchema {}),
        }));
    let quantity = |value| FiniteQuantity::new(value, unit.clone()).unwrap();
    let mut specs = vec![
        (
            "flag",
            ValueSchema::Boolean,
            value_recipe("flag", "flag", true),
        ),
        (
            "delta",
            ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(-2).unwrap(),
                maximum: BoundedInteger::new(2).unwrap(),
            }),
            value_recipe("delta", "delta", false),
        ),
        (
            "amount",
            ValueSchema::Quantity(QuantityRange {
                minimum: quantity(-10.),
                maximum: quantity(10.),
            }),
            value_recipe("amount", "amount", false),
        ),
        (
            "mode",
            ValueSchema::Option {
                allowed: DeclaredSet::complete(vec![option.clone()]),
            },
            value_recipe("mode", "mode", false),
        ),
        (
            "optional",
            ValueSchema::Boolean,
            value_recipe("optional", "optional", true),
        ),
    ];
    specs[2].2.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Decimal,
        unit: unit.clone(),
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    specs[3].2.codec.codec = ValueCodecKind::Option {
        tokens: vec![OptionToken {
            token: "a".into(),
            value: option.clone(),
        }],
    };
    specs[4].2.missing = MissingValuePolicy::Absent;
    let mut inputs = vec![];
    for (name, value, recipe) in specs {
        let slot: DeclaredSlot<ParameterSlotDefId> =
            a.registry.allocate_slot(owner.clone()).unwrap();
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value,
                    presence: if name == "optional" {
                        SlotPresence::OptionalOnce
                    } else {
                        SlotPresence::RequiredOnce
                    },
                    sites: vec![ParameterSite::GemParameter],
                }),
            }));
        inputs.push(GemParameterInput {
            slot,
            value: recipe,
        });
    }
    let row = schema
        .definitions
        .iter_mut()
        .find_map(|row| match row {
            DefinitionDescriptor::Gem(row) if row.id == gem => Some(row),
            _ => None,
        })
        .unwrap();
    let SchemaState::Known(gem_schema) = &mut row.schema else {
        unreachable!()
    };
    gem_schema.declarations.parameters =
        DeclaredSet::complete(inputs.iter().map(|input| input.slot.clone()).collect());
    let rule = &mut policy.gem_inputs.as_mut().unwrap().gems[0];
    rule.parameters = inputs;
    rebind_quality_schema(&mut a, &mut policy, schema);
    (
        a,
        policy,
        vec![
            ParameterValue::Boolean(true),
            ParameterValue::Integer(BoundedInteger::new(1).unwrap()),
            ParameterValue::Quantity(quantity(-0.5)),
            ParameterValue::Option(option),
        ],
    )
}
const PROJECTED: &str = r#"flag="true" delta="+1" amount="-0.5" mode="a""#;

#[test]
fn declared_gem_projection_preserves_all_scalar_types_and_physical_occurrences() {
    let (a, policy, expected) = projection_fixture();
    let gem = format!(r#"<Gem gemId="active" variantId="v" {PROJECTED}/ >"#).replace("/ >", "/>");
    let source = source(&group(&format!("{gem}{gem}")), 0x91);
    let result = run_with_policy(&source, &a, &[], &policy);
    let gems = &result.draft().input().gems.members;
    assert_eq!(gems.len(), 2);
    assert_ne!(gems[0].id, gems[1].id);
    for gem in gems {
        assert!(matches!(
            gem.parameters.completion,
            DraftListCompletion::Complete
        ));
        assert_eq!(
            gem.parameters
                .members
                .iter()
                .map(|member| member.value.to_resolved().unwrap())
                .collect::<Vec<_>>(),
            expected
        );
        assert_eq!(
            gem.parameters
                .members
                .iter()
                .map(|member| member.slot.to_resolved().unwrap())
                .collect::<Vec<_>>(),
            policy.gem_inputs.as_ref().unwrap().gems[0].parameters[..4]
                .iter()
                .map(|input| input.slot.clone())
                .collect::<Vec<_>>()
        );
        assert!(matches!(gem.quality, DraftQuality::Pending(_)));
    }
    origin_integrity(&source, &result);
    assert!(
        result
            .draft()
            .input()
            .skills
            .members
            .iter()
            .all(|skill| matches!(skill.scope, DraftField::Pending(_)))
    );
}

#[test]
fn missing_malformed_or_outside_schema_values_keep_valid_members_without_inventing_defaults() {
    let (a, policy, _) = projection_fixture();
    for bad in ["", "nil", "3", "-3", "1.5", "99999999999999999999", "&#49;"] {
        let attrs = PROJECTED.replace("+1", bad);
        let result = normalize_with_loadouts(&xml(&attrs), &a, &policy).unwrap();
        assert!(!is_complete(&result), "{bad}");
        assert_eq!(only_gem(&result).parameters.members.len(), 3, "{bad}");
    }
    for attrs in [
        PROJECTED.replace("-0.5", "10.001"),
        PROJECTED.replace("mode=\"a\"", "mode=\"b\""),
        PROJECTED.replace(" delta=\"+1\"", ""),
    ] {
        let result = normalize_with_loadouts(&xml(&attrs), &a, &policy).unwrap();
        assert!(!is_complete(&result));
        assert_eq!(only_gem(&result).parameters.members.len(), 3);
    }
    let mut explicit = policy.clone();
    explicit.gem_inputs.as_mut().unwrap().gems[0].parameters[1]
        .value
        .missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
    };
    let missing = PROJECTED.replace(" delta=\"+1\"", "");
    let result = normalize_with_loadouts(&xml(&missing), &a, &explicit).unwrap();
    assert!(is_complete(&result));
    assert_eq!(
        only_gem(&result).parameters.members[1].value.to_resolved(),
        Some(ParameterValue::Integer(BoundedInteger::new(0).unwrap()))
    );
    let malformed =
        normalize_with_loadouts(&xml(&PROJECTED.replace("+1", "nil")), &a, &explicit).unwrap();
    assert!(
        !is_complete(&malformed),
        "an explicit missing default cannot consume malformed text"
    );
    explicit.gem_inputs.as_mut().unwrap().gems[0].parameters[1]
        .value
        .missing = MissingValuePolicy::Absent;
    assert!(
        !is_complete(&normalize_with_loadouts(&xml(&missing), &a, &explicit).unwrap()),
        "required absence cannot close a collection"
    );
}

#[test]
fn partial_schema_and_partial_recipe_coverage_never_close_known_members() {
    let (mut a, mut policy, _) = projection_fixture();
    let mut partial_policy = policy.clone();
    partial_policy.gem_inputs.as_mut().unwrap().gems[0]
        .parameters
        .pop();
    let result = normalize_with_loadouts(&xml(PROJECTED), &a, &partial_policy).unwrap();
    assert!(!is_complete(&result));
    assert_eq!(only_gem(&result).parameters.members.len(), 4);
    let gem = policy.gem_inputs.as_ref().unwrap().gems[0].gem.clone();
    let mut schema = a.schema.input().clone();
    for row in &mut schema.definitions {
        if let DefinitionDescriptor::Gem(row) = row
            && row.id == gem
        {
            let SchemaState::Known(value) = &mut row.schema else {
                unreachable!()
            };
            value.declarations.parameters = DeclaredSet::partial(
                value.declarations.parameters.members.clone(),
                vec![SchemaGap {
                    subject: subject(&gem),
                    facet: SchemaFacet::InputSchema,
                    code: key("more-inputs"),
                }],
            );
        }
    }
    rebind_quality_schema(&mut a, &mut policy, schema);
    let result = normalize_with_loadouts(&xml(PROJECTED), &a, &policy).unwrap();
    assert!(!is_complete(&result));
    assert_eq!(only_gem(&result).parameters.members.len(), 4);
}

#[test]
fn schema_bindings_slots_codecs_defaults_and_guards_reject_invalid_authoring() {
    let (a, policy, _) = projection_fixture();
    for case in 0..12 {
        let mut bad = policy.clone();
        let input = bad.gem_inputs.as_mut().unwrap();
        match case {
            0 => input.gems.push(input.gems[0].clone()),
            1 => {
                let row = input.gems[0].parameters[0].clone();
                input.gems[0].parameters.push(row);
            }
            2 => {
                let row = input.gems[0].guards[0].clone();
                input.gems[0].guards.push(row);
            }
            3 => input.gems[0].guards[0]
                .allowed
                .push(SourceComponent::Missing),
            4 => input.gems[0].guards[0].attribute.clear(),
            5 => input.gems[0].guards[0].attribute = "a".repeat(129),
            6 => input.gems[0].parameters[0].value = value_recipe("wrong-type", "flag", false),
            7 => {
                input.gems[0].parameters[0].value.tiers[0].selectors[0].lane =
                    ValueLane::ParentAttribute
            }
            8 => {
                input.gems[0].parameters[1].value.missing = MissingValuePolicy::Explicit {
                    value: ParameterValue::Integer(BoundedInteger::new(3).unwrap()),
                }
            }
            9 => {
                input.gems[0].parameters[0].slot.declaration = SlotOwnerDefId::Gem(
                    a.roles
                        .input()
                        .roles
                        .iter()
                        .find(|row| row.gem != input.gems[0].gem)
                        .unwrap()
                        .gem
                        .clone(),
                )
            }
            10 => {
                input.gems[0].guards[0].allowed = (0..65)
                    .map(|n| SourceComponent::Text(n.to_string()))
                    .collect()
            }
            11 => {
                input.gems[0].guards[0].allowed = vec![SourceComponent::Text(
                    "x".repeat(OwnedMappingLimits::default().max_string_bytes + 1),
                )]
            }
            _ => unreachable!(),
        }
        assert!(
            normalize_with_loadouts(&xml(PROJECTED), &a, &bad).is_err(),
            "case {case}"
        );
    }
    let (other, _) = empty_fixture();
    let mut stale = policy.clone();
    stale.gem_inputs.as_mut().unwrap().definitions = other.schema.identity().clone();
    assert!(matches!(
        normalize_with_loadouts(&xml(PROJECTED), &a, &stale),
        Err(NormalizationError::Binding)
    ));
    let mut wire = serde_json::to_value(&policy).unwrap();
    wire["gem_inputs"]["gems"][0]["unreviewed"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(wire).is_err());
}

#[test]
fn slot_site_and_quantity_unit_must_match_the_direct_gem_declaration() {
    for wrong_site in [true, false] {
        let (mut a, mut policy, _) = projection_fixture();
        let mut schema = a.schema.input().clone();
        if wrong_site {
            let slot = &policy.gem_inputs.as_ref().unwrap().gems[0].parameters[0].slot;
            for row in &mut schema.slots {
                if let SlotDescriptor::Parameter(row) = row
                    && &row.id == slot
                {
                    let SchemaState::Known(value) = &mut row.schema else {
                        unreachable!()
                    };
                    value.sites = vec![ParameterSite::ItemParameter];
                }
            }
        } else {
            let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
            schema
                .definitions
                .push(DefinitionDescriptor::Unit(DefinitionEntry {
                    id: unit.clone(),
                    schema: SchemaState::Known(UnitSchema {
                        dimension: UnitDimension::Count,
                    }),
                }));
            let ValueCodecKind::Quantity {
                unit: codec_unit, ..
            } = &mut policy.gem_inputs.as_mut().unwrap().gems[0].parameters[2]
                .value
                .codec
                .codec
            else {
                unreachable!()
            };
            *codec_unit = unit;
        }
        if wrong_site {
            assert!(
                matches!(
                    OwnedDefinitionSchemaPackage::new(schema, OwnedSchemaLimits::default()),
                    Err(SchemaPackageError::Invalid {
                        kind: SchemaPackageErrorKind::WrongParameterSite,
                        ..
                    })
                ),
                "the owned schema rejects a foreign parameter site before normalization"
            );
            continue;
        }
        rebind_quality_schema(&mut a, &mut policy, schema);
        assert!(normalize_with_loadouts(&xml(PROJECTED), &a, &policy).is_err());
    }
}

#[test]
fn oversized_source_guard_and_tightened_aggregate_work_fail_boundedly() {
    let (a, mut policy) = empty_fixture();
    policy.gem_inputs.as_mut().unwrap().gems[0].guards[0].allowed =
        vec![SourceComponent::Text("small".into())];
    let attrs = format!(
        r#"neutral-input="{}""#,
        "x".repeat(OwnedMappingLimits::default().max_string_bytes + 1)
    );
    assert!(matches!(
        normalize_with_loadouts(&xml(&attrs), &a, &policy),
        Err(NormalizationError::Limit("gem input guard bytes"))
    ));
    let source = source(&xml(""), 0x92);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let result = normalize_fresh(
        &evidence,
        *source.allocator_state(),
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
        &policy,
        &[],
        NormalizationLimits {
            max_work: 1,
            ..NormalizationLimits::default()
        },
    );
    assert!(matches!(result, Err(NormalizationError::Limit(_))));
}
