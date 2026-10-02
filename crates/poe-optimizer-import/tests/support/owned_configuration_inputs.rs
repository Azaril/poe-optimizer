//! Injected raw overrides: absence is a Boolean fact, never a numeric default.
#[path = "owned_configuration_fallbacks.rs"]
mod fallback_tests;
use super::*;
use serde_json::Value;

struct Fixture {
    a: Artifacts,
    encounter: EncounterDefId,
    unit: UnitDefId,
    inputs: Vec<(ExternalInputDefId, ExternalInputDefId)>,
}

fn selector() -> ExternalSelector {
    ExternalSelector::Catalog {
        kind: ExternalCatalogKind::Encounter,
        key: SourceComponent::Text("finite-default".into()),
        version: SourceComponent::Missing,
        variant: SourceComponent::Missing,
    }
}

fn rebind(a: &mut Artifacts, schema: SchemaPackageInput) {
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
    a.rewards = empty_rewards(&a.mapping, &a.schema);
}

fn fixture() -> Fixture {
    let mut a = artifacts(false);
    let mut schema = a.schema.input().clone();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }));
    let mut inputs = Vec::new();
    for _ in 0..2 {
        let presence = a
            .registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
        let raw = a
            .registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
        for (id, value) in [
            (presence.clone(), ValueSchema::Boolean),
            (
                raw.clone(),
                ValueSchema::Quantity(QuantityRange {
                    minimum: FiniteQuantity::new(-100., unit.clone()).unwrap(),
                    maximum: FiniteQuantity::new(100., unit.clone()).unwrap(),
                }),
            ),
        ] {
            schema
                .definitions
                .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
                    id,
                    schema: SchemaState::Known(ExternalInputSchema {
                        value,
                        targets: vec![AssumptionTargetKind::Enemy],
                    }),
                }));
        }
        inputs.push((presence, raw));
    }
    let encounter = a
        .registry
        .allocate_definition::<EncounterDefinition>()
        .unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::Encounter(DefinitionEntry {
            id: encounter.clone(),
            schema: SchemaState::Known(EncounterSchema {
                enemy_level: IntegerRange {
                    minimum: BoundedInteger::new(1).unwrap(),
                    maximum: BoundedInteger::new(90).unwrap(),
                },
                external_inputs: DeclaredSet::partial(
                    inputs
                        .iter()
                        .flat_map(|(p, v)| [p.clone(), v.clone()])
                        .collect(),
                    vec![SchemaGap {
                        subject: subject(&encounter),
                        facet: SchemaFacet::StaticLinks,
                        code: key("other-inputs-unconverted"),
                    }],
                ),
            }),
        }));
    rebind(&mut a, schema);
    let mut mapping = a.mapping.input().clone();
    mapping.entries.push(MappingEntry {
        source: selector(),
        outcome: MappingOutcome::Mapped {
            target: subject(&encounter),
            basis: MappingBasis::Exact,
        },
    });
    a.mapping =
        OwnedMappingIndex::new(mapping, &a.registry, &a.schema, Default::default()).unwrap();
    let schema = a.schema.input().clone();
    rebind(&mut a, schema);
    Fixture {
        a,
        encounter,
        unit,
        inputs,
    }
}

fn reviewed(f: &Fixture) -> NormalizationPolicy {
    let mut p = policy();
    p.encounter = Some(EncounterPolicy::PobFreshDefaultConfigEncounterV1 {
        mapping_source: *f.a.mapping.source_identity(),
        selector: selector(),
        target: f.encounter.clone(),
        absent_input_names: vec!["encounter-selector".into()],
    });
    let inputs = f
        .inputs
        .iter()
        .enumerate()
        .map(|(i, (presence, value))| {
            let name = format!("raw-{i}");
            let mut recipe = value_recipe(&format!("override-{i}"), &name, false);
            recipe.tiers[0].selectors[0].lane = ValueLane::InputNumber;
            recipe.codec.codec = ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: f.unit.clone(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            };
            ConfigurationNumericInput {
                source_name: name,
                presence_input: presence.clone(),
                value_input: value.clone(),
                recipe,
            }
        })
        .collect();
    p.configuration_inputs = Some(
        ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            mapping_source: *f.a.mapping.source_identity(),
            encounter: f.encounter.clone(),
            inputs,
        },
    );
    p
}

fn xml(body: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{body}</ConfigSet></Config></PathOfBuilding2>"#
    )
}

fn normalize(
    source: &ImportedBuildInstance,
    f: &Fixture,
    p: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let evidence = SourceProjectEvidence::collect(source, Default::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&f.a.schema),
            item_source: &empty_item_source(&f.a.schema),
            mappings: &f.a.mapping,
            registry: &f.a.registry,
            definitions: &f.a.schema,
            roles: &f.a.roles,
            rewards: &f.a.rewards,
        },
        p,
        &queries(),
        limits,
    )
}

fn values(result: &NormalizedImport, index: usize) -> Vec<(ExternalInputDefId, ParameterValue)> {
    result.draft().input().scenario_presets.members[index]
        .scenario
        .assumptions
        .members
        .iter()
        .map(|a| {
            assert_eq!(a.target, DraftAssumptionTarget::Enemy);
            (
                a.input.to_resolved().unwrap(),
                a.value.to_resolved().unwrap(),
            )
        })
        .collect()
}

#[test]
fn omitted_numeric_profile_preserves_serialization_draft_and_sidecar() {
    let f = fixture();
    let mut p = reviewed(&f);
    p.configuration_inputs = None;
    let wire = serde_json::to_value(&p).unwrap();
    assert!(wire.get("configuration_inputs").is_none());
    let mut nullable = wire.clone();
    nullable["configuration_inputs"] = Value::Null;
    let restored: NormalizationPolicy = serde_json::from_value(nullable).unwrap();
    assert_eq!(serde_json::to_value(&restored).unwrap(), wire);
    let source = source(&xml(r#"<Input name="raw-0" number="23"/>"#), 0x71);
    let a = normalize(&source, &f, &p, Default::default()).unwrap();
    let b = normalize(&source, &f, &restored, Default::default()).unwrap();
    assert_eq!(a.draft(), b.draft());
    assert_eq!(
        serde_json::to_vec(a.sidecar()).unwrap(),
        serde_json::to_vec(b.sidecar()).unwrap()
    );
    assert_eq!(a.allocator_after(), b.allocator_after());
    assert!(values(&b, 0).is_empty());
}

#[test]
fn absent_placeholders_and_explicit_raw_numbers_remain_distinct_and_preserve_every_old_fact() {
    let f = fixture();
    for (body, raw) in [
        ("", None),
        (r#"<Placeholder name="raw-0" number="47"/>"#, None),
        (r#"<Input name="raw-0" number="0"/>"#, Some(0.)),
        (r#"<Input name="raw-0" number="-17.25"/>"#, Some(-17.25)),
        (
            r#"<Input name="raw-0" number="1e2"/><Placeholder name="raw-0" number="47"/>"#,
            Some(100.),
        ),
    ] {
        let source = source(&xml(body), 0x72);
        let p = reviewed(&f);
        let mut old_policy = p.clone();
        old_policy.configuration_inputs = None;
        let before = normalize(&source, &f, &old_policy, Default::default()).unwrap();
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        let mut expected = vec![(
            f.inputs[0].0.clone(),
            ParameterValue::Boolean(raw.is_some()),
        )];
        if let Some(raw) = raw {
            expected.push((
                f.inputs[0].1.clone(),
                ParameterValue::Quantity(FiniteQuantity::new(raw, f.unit.clone()).unwrap()),
            ));
        }
        expected.push((f.inputs[1].0.clone(), ParameterValue::Boolean(false)));
        assert_eq!(values(&after, 0), expected, "{body}");
        let mut restored = after.draft().input().clone();
        restored.scenario_presets.members[0]
            .scenario
            .assumptions
            .members
            .clear();
        assert_eq!(restored, *before.draft().input());
        assert_eq!(before.allocator_after(), after.allocator_after());
        let mut origins = after.sidecar().origins.clone();
        let scenario = after.draft().input().scenario_presets.members[0].id;
        for (row, old) in origins.iter_mut().zip(&before.sidecar().origins) {
            row.links.retain(|link| {
                old.links.contains(link) || *link != OwnedOriginTarget::ScenarioPreset(scenario)
            });
        }
        assert_eq!(
            serde_json::to_value(origins).unwrap(),
            serde_json::to_value(&before.sidecar().origins).unwrap()
        );
        if raw.is_some() {
            let row = source
                .occurrences()
                .iter()
                .find(|o| o.name() == "Input")
                .unwrap();
            let next = after
                .sidecar()
                .origins
                .iter()
                .find(|o| o.source == row.id())
                .unwrap();
            assert!(
                next.links
                    .contains(&OwnedOriginTarget::ScenarioPreset(scenario))
            );
        } else {
            assert_eq!(
                serde_json::to_value(&after.sidecar().origins).unwrap(),
                serde_json::to_value(&before.sidecar().origins).unwrap()
            );
        }
        let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
        let old = serde_json::to_value(before.sidecar()).unwrap();
        for field in ["policy", "draft", "origins"] {
            sidecar[field] = old[field].clone();
        }
        assert_eq!(sidecar, old);
        origin_integrity_with_retired(&source, &after, 1);
    }
}

#[test]
fn wrong_typed_alias_malformed_and_out_of_domain_values_never_become_absence() {
    let f = fixture();
    for body in [
        r#"<Input name="raw-0" string="0"/>"#,
        r#"<Input name="raw-0" boolean="false"/>"#,
        r#"<Placeholder name="raw-0" string="0"/>"#,
        r#"<Placeholder name="raw-0" number="bad"/>"#,
        r#"<Input name="raw-0" number="NaN"/>"#,
        r#"<Input name="raw-0" number="inf"/>"#,
        r#"<Input name="raw-0" number=" 1"/>"#,
        r#"<Input name="raw-0" number="100.1"/>"#,
        r#"<Input name="raw-0" number="-100.1"/>"#,
        r#"<Input name="raw-0" number="17"/><Placeholder name="raw-0" string="24"/>"#,
    ] {
        let source = source(&xml(body), 0x73);
        let result = normalize(&source, &f, &reviewed(&f), Default::default()).unwrap();
        assert_eq!(
            values(&result, 0),
            vec![(f.inputs[1].0.clone(), ParameterValue::Boolean(false))],
            "{body}"
        );
        assert!(matches!(
            result.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn ambiguous_scope_and_unknown_shapes_never_establish_missing_controls() {
    let f = fixture();
    for text in [
        xml(r#"<Input name="raw-0" number="1"/><Input name="raw-0" number="2"/>"#),
        xml(r#"<Input name="raw-0" number="1" string="2"/>"#),
        xml(r#"<Input name="raw-0" number="1"><Future/></Input>"#),
        xml(r#"<Input xmlns:q="future" name="raw-0" number="1"/>"#),
        xml("<Future/>"),
        r#"<PathOfBuilding2><Config><Input name="raw-0" number="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="01"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/></Config><Config activeConfigSet="2"><ConfigSet id="2"/></Config></PathOfBuilding2>"#.into(),
    ] {
        let source = source(&text, 0x74);
        let result = normalize(&source, &f, &reviewed(&f), Default::default()).unwrap();
        assert!(result.draft().input().scenario_presets.members.iter().all(|s| s.scenario.assumptions.members.is_empty()), "{text}");
    }
}

#[test]
fn configurations_are_independent_and_wrong_encounter_cannot_supply_any_values() {
    let f = fixture();
    let source = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="raw-0" number="7"/></ConfigSet><ConfigSet id="2"><Input name="raw-1" number="-9"/></ConfigSet><ConfigSet id="3"><Input name="encounter-selector" string="other"/></ConfigSet></Config></PathOfBuilding2>"#,
        0x75,
    );
    let result = normalize(&source, &f, &reviewed(&f), Default::default()).unwrap();
    assert_eq!(
        values(&result, 0),
        vec![
            (f.inputs[0].0.clone(), ParameterValue::Boolean(true)),
            (
                f.inputs[0].1.clone(),
                ParameterValue::Quantity(FiniteQuantity::new(7., f.unit.clone()).unwrap())
            ),
            (f.inputs[1].0.clone(), ParameterValue::Boolean(false))
        ]
    );
    assert_eq!(
        values(&result, 1),
        vec![
            (f.inputs[0].0.clone(), ParameterValue::Boolean(false)),
            (f.inputs[1].0.clone(), ParameterValue::Boolean(true)),
            (
                f.inputs[1].1.clone(),
                ParameterValue::Quantity(FiniteQuantity::new(-9., f.unit.clone()).unwrap())
            )
        ]
    );
    assert!(values(&result, 2).is_empty());
    origin_integrity_with_retired(&source, &result, 2);
}

#[test]
fn policy_source_types_membership_units_and_recipe_authority_are_checked() {
    let f = fixture();
    let source = source(&xml(""), 0x76);
    for mutation in 0..13 {
        let mut p = reviewed(&f);
        let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            mapping_source,
            encounter,
            inputs,
        }) = &mut p.configuration_inputs
        else {
            panic!()
        };
        match mutation {
            0 => *mapping_source = *f.a.mapping.identity(),
            1 => *encounter = EncounterDefId::new(ns(), key("missing")),
            2 => inputs[0].presence_input = inputs[0].value_input.clone(),
            3 => inputs[1].source_name = inputs[0].source_name.clone(),
            4 => inputs[1].value_input = inputs[0].value_input.clone(),
            5 => inputs[1].recipe.id = inputs[0].recipe.id.clone(),
            6 => inputs[0].recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber,
            7 => inputs[0].recipe.tiers[0].selectors[0].name = "other".into(),
            8 => inputs[0].recipe.codec.whitespace = WhitespacePolicy::TrimAscii,
            9 => inputs[0].recipe.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
            10 => {
                inputs[0].presence_input = ExternalInputDefId::new(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    key("x"),
                )
            }
            11 => {
                let ValueCodecKind::Quantity { scale, .. } = &mut inputs[0].recipe.codec.codec
                else {
                    panic!()
                };
                scale.numerator = BoundedInteger::new(2).unwrap();
            }
            12 => {
                let ValueCodecKind::Quantity { unit, .. } = &mut inputs[0].recipe.codec.codec
                else {
                    panic!()
                };
                *unit = UnitDefId::new(ns(), key("wrong-unit"));
            }
            _ => unreachable!(),
        }
        assert!(
            normalize(&source, &f, &p, Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
    for mutation in 0..4 {
        let mut f = fixture();
        let mut schema = f.a.schema.input().clone();
        for descriptor in &mut schema.definitions {
            match descriptor {
                DefinitionDescriptor::ExternalInput(entry)
                    if entry.id == f.inputs[0].0 && mutation == 0 =>
                {
                    entry.schema = unknown(entry.id.clone()).schema
                }
                DefinitionDescriptor::ExternalInput(entry)
                    if entry.id == f.inputs[0].1 && mutation == 1 =>
                {
                    let SchemaState::Known(raw) = &mut entry.schema else {
                        panic!()
                    };
                    raw.targets = vec![AssumptionTargetKind::Actor];
                }
                DefinitionDescriptor::Encounter(entry) if mutation == 2 => {
                    let SchemaState::Known(owner) = &mut entry.schema else {
                        panic!()
                    };
                    owner
                        .external_inputs
                        .members
                        .retain(|id| id != &f.inputs[0].1);
                }
                DefinitionDescriptor::ExternalInput(entry)
                    if entry.id == f.inputs[0].1 && mutation == 3 =>
                {
                    let SchemaState::Known(raw) = &mut entry.schema else {
                        panic!()
                    };
                    raw.value = ValueSchema::Boolean;
                }
                _ => {}
            }
        }
        rebind(&mut f.a, schema);
        assert!(
            normalize(&source, &f, &reviewed(&f), Default::default()).is_err(),
            "schema mutation {mutation}"
        );
    }
}

#[test]
fn policy_source_and_output_budgets_fail_before_unbounded_materialization() {
    let f = fixture();
    let source = source(&xml(r#"<Input name="raw-0" number="17.25"/>"#), 0x77);
    for mutation in 0..5 {
        let mut limits = NormalizationLimits::default();
        match mutation {
            0 => limits.max_work = 1,
            1 => limits.value.max_selectors = 1,
            2 => limits.value.max_total_selector_bytes = 5,
            3 => limits.value.value.max_source_bytes = 4,
            4 => limits.draft.input.max_collection_entries = 3,
            _ => unreachable!(),
        }
        assert!(
            normalize(&source, &f, &reviewed(&f), limits).is_err(),
            "{mutation}"
        );
    }
    let mut p = reviewed(&f);
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 { inputs, .. }) =
        &mut p.configuration_inputs
    else {
        panic!()
    };
    *inputs = vec![inputs[0].clone(); 65];
    assert!(matches!(
        normalize(&source, &f, &p, Default::default()),
        Err(NormalizationError::Limit("configuration input rows"))
    ));
}
