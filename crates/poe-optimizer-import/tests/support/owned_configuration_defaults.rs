//! Constructor defaults are injected typed facts, never callback calculations.
use super::*;

const NAMES: [&str; 4] = [
    "integer-control",
    "quantity-control",
    "boolean-control",
    "choice-control",
];

struct DefaultFixture {
    f: Fixture,
    inputs: Vec<ExternalInputDefId>,
    options: Vec<OptionDefId>,
    extra_option_input: ExternalInputDefId,
}

fn default_fixture() -> DefaultFixture {
    let mut f = fixture();
    let mut schema = f.a.schema.input().clone();
    let mut options = Vec::new();
    for _ in 0..2 {
        let id =
            f.a.registry
                .allocate_definition::<OptionDefinition>()
                .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::Option(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(OptionSchema {}),
            }));
        options.push(id);
    }
    let values = [
        ValueSchema::Integer(IntegerRange {
            minimum: BoundedInteger::new(-10).unwrap(),
            maximum: BoundedInteger::new(10).unwrap(),
        }),
        ValueSchema::Quantity(QuantityRange {
            minimum: FiniteQuantity::new(-10., f.unit.clone()).unwrap(),
            maximum: FiniteQuantity::new(10., f.unit.clone()).unwrap(),
        }),
        ValueSchema::Boolean,
        ValueSchema::Option {
            allowed: DeclaredSet::complete(options.clone()),
        },
    ];
    let targets = [
        AssumptionTargetKind::Enemy,
        AssumptionTargetKind::Environment,
        AssumptionTargetKind::Actor,
        AssumptionTargetKind::Enemy,
    ];
    let mut inputs = Vec::new();
    for (value, target) in values.into_iter().zip(targets) {
        let id =
            f.a.registry
                .allocate_definition::<ExternalInputDefinition>()
                .unwrap();
        schema
            .definitions
            .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(ExternalInputSchema {
                    value,
                    targets: vec![target],
                }),
            }));
        inputs.push(id);
    }
    let extra_option_input =
        f.a.registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: extra_option_input.clone(),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(options.clone()),
                },
                targets: vec![AssumptionTargetKind::Enemy],
            }),
        }));
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::Encounter(entry) = descriptor
            && entry.id == f.encounter
        {
            let SchemaState::Known(owner) = &mut entry.schema else {
                panic!()
            };
            owner.external_inputs.members.extend(inputs.clone());
            owner
                .external_inputs
                .members
                .push(extra_option_input.clone());
        }
    }
    rebind(&mut f.a, schema);
    DefaultFixture {
        f,
        inputs,
        options,
        extra_option_input,
    }
}

fn defaults(f: &DefaultFixture) -> Vec<ParameterValue> {
    vec![
        ParameterValue::Integer(BoundedInteger::new(7).unwrap()),
        ParameterValue::Quantity(FiniteQuantity::new(3.5, f.f.unit.clone()).unwrap()),
        ParameterValue::Boolean(true),
        ParameterValue::Option(f.options[1].clone()),
    ]
}

fn default_policy(f: &DefaultFixture) -> NormalizationPolicy {
    let mut p = reviewed(&f.f);
    let targets = [
        ConfigurationInputTarget::Enemy,
        ConfigurationInputTarget::Environment,
        ConfigurationInputTarget::Player,
        ConfigurationInputTarget::Enemy,
    ];
    let mut rows = Vec::new();
    for (index, (target, constructor_default)) in targets.into_iter().zip(defaults(f)).enumerate() {
        let mut recipe = value_recipe(&format!("constructor-{index}"), NAMES[index], index == 2);
        recipe.tiers[0].selectors[0].lane = match index {
            0 | 1 => ValueLane::InputNumber,
            2 => ValueLane::InputBoolean,
            3 => ValueLane::InputString,
            _ => unreachable!(),
        };
        if index == 1 {
            recipe.codec.codec = ValueCodecKind::Quantity {
                syntax: DecimalSyntax::Scientific,
                unit: f.f.unit.clone(),
                scale: RationalScale {
                    numerator: BoundedInteger::new(1).unwrap(),
                    denominator: BoundedInteger::new(1).unwrap(),
                },
            };
        } else if index == 3 {
            recipe.codec.codec = ValueCodecKind::Option {
                tokens: ["Alpha", "Beta"]
                    .into_iter()
                    .zip(&f.options)
                    .map(|(token, value)| OptionToken {
                        token: token.into(),
                        value: value.clone(),
                    })
                    .collect(),
            };
        }
        rows.push(ConfigurationDefaultInput {
            source_name: NAMES[index].into(),
            value_input: f.inputs[index].clone(),
            target,
            recipe,
            constructor_default,
            ignore_numeric_placeholder: false,
        });
    }
    p.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source: *f.f.a.mapping.source_identity(),
        encounter: f.f.encounter.clone(),
        inputs: vec![],
        placeholder_fallback_inputs: vec![],
        option_inputs: vec![],
        default_inputs: rows,
    });
    p
}

fn rows(p: &mut NormalizationPolicy) -> &mut Vec<ConfigurationDefaultInput> {
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut p.configuration_inputs
    else {
        panic!()
    };
    default_inputs
}

fn assumptions(
    result: &NormalizedImport,
    index: usize,
) -> Vec<(ExternalInputDefId, DraftAssumptionTarget, ParameterValue)> {
    result.draft().input().scenario_presets.members[index]
        .scenario
        .assumptions
        .members
        .iter()
        .map(|row| {
            (
                row.input.to_resolved().unwrap(),
                row.target.clone(),
                row.value.to_resolved().unwrap(),
            )
        })
        .collect()
}

fn expected(
    f: &DefaultFixture,
    values: Vec<ParameterValue>,
) -> Vec<(ExternalInputDefId, DraftAssumptionTarget, ParameterValue)> {
    let targets = [
        DraftAssumptionTarget::Enemy,
        DraftAssumptionTarget::Environment,
        DraftAssumptionTarget::Actor(DraftActorKey::Player),
        DraftAssumptionTarget::Enemy,
    ];
    f.inputs
        .iter()
        .cloned()
        .zip(targets)
        .zip(values)
        .map(|((input, target), value)| (input, target, value))
        .collect()
}

#[test]
fn defaults_and_explicit_zero_false_values_preserve_types_targets_and_all_old_obligations() {
    let f = default_fixture();
    for (body, values) in [
        ("", defaults(&f)),
        (
            r#"<Input name="integer-control" number="0"/><Input name="quantity-control" number="0"/><Input name="boolean-control" boolean="false"/><Input name="choice-control" string="Alpha"/>"#,
            vec![
                ParameterValue::Integer(BoundedInteger::new(0).unwrap()),
                ParameterValue::Quantity(FiniteQuantity::new(0., f.f.unit.clone()).unwrap()),
                ParameterValue::Boolean(false),
                ParameterValue::Option(f.options[0].clone()),
            ],
        ),
        (
            r#"<Input name="integer-control" number="-4"/><Input name="quantity-control" number="1e1"/>"#,
            vec![
                ParameterValue::Integer(BoundedInteger::new(-4).unwrap()),
                ParameterValue::Quantity(FiniteQuantity::new(10., f.f.unit.clone()).unwrap()),
                ParameterValue::Boolean(true),
                ParameterValue::Option(f.options[1].clone()),
            ],
        ),
    ] {
        let imported = source(&xml(body), 0xb1);
        let policy = default_policy(&f);
        let mut baseline = policy.clone();
        baseline.configuration_inputs = None;
        let before = normalize(&imported, &f.f, &baseline, Default::default()).unwrap();
        let after = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        assert_eq!(assumptions(&after, 0), expected(&f, values), "{body}");
        let mut inverse = after.draft().input().clone();
        inverse.scenario_presets.members[0]
            .scenario
            .assumptions
            .members
            .clear();
        assert_eq!(inverse, *before.draft().input());
        assert_eq!(after.allocator_after(), before.allocator_after());
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let scenario = after.draft().input().scenario_presets.members[0].id;
        let mut origins = before.sidecar().origins.clone();
        for occurrence in imported
            .occurrences()
            .iter()
            .filter(|row| row.name() == "Input" || row.name() == "ConfigSet")
        {
            let origin = origins
                .iter_mut()
                .find(|row| row.source == occurrence.id())
                .unwrap();
            let target = OwnedOriginTarget::ScenarioPreset(scenario);
            if !origin.links.contains(&target) {
                origin.links.push(target);
            }
        }
        assert_eq!(
            after.sidecar().origins,
            origins,
            "only exact input or ConfigSet provenance is added"
        );
        let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
        let old = serde_json::to_value(before.sidecar()).unwrap();
        for field in ["policy", "draft", "origins"] {
            sidecar[field] = old[field].clone();
        }
        assert_eq!(sidecar, old);
    }
}

#[test]
fn malformed_typed_values_and_saved_placeholders_block_only_the_matching_default() {
    let f = default_fixture();
    for (index, body) in [
        (0, r#"<Input name="integer-control" string="0"/>"#),
        (0, r#"<Input name="integer-control" number="1.5"/>"#),
        (0, r#"<Input name="integer-control" number="11"/>"#),
        (0, r#"<Input name="integer-control" number=" 1"/>"#),
        (1, r#"<Input name="quantity-control" number="NaN"/>"#),
        (1, r#"<Input name="quantity-control" number="10.1"/>"#),
        (2, r#"<Input name="boolean-control" number="0"/>"#),
        (3, r#"<Input name="choice-control" string="alpha"/>"#),
        (0, r#"<Placeholder name="integer-control" number="2"/>"#),
        (1, r#"<Placeholder name="quantity-control" number="2"/>"#),
        (2, r#"<Placeholder name="boolean-control" string="false"/>"#),
        (3, r#"<Placeholder name="choice-control" string="Alpha"/>"#),
        (
            0,
            r#"<Input name="integer-control" number="2"/><Placeholder name="integer-control" number="7"/>"#,
        ),
    ] {
        let imported = source(&xml(body), 0xb2);
        let result = normalize(&imported, &f.f, &default_policy(&f), Default::default()).unwrap();
        let mut wanted = expected(&f, defaults(&f));
        wanted.remove(index);
        assert_eq!(assumptions(&result, 0), wanted, "{body}");
    }
}

#[test]
fn defaults_need_an_unambiguous_fresh_frame_and_never_leak_between_configurations() {
    let f = default_fixture();
    for text in [
        xml(r#"<Input name="integer-control" number="2"/><Input name="integer-control" number="3"/>"#),
        xml(r#"<Input name="integer-control" number="2" boolean="false"/>"#),
        xml(r#"<Input name="integer-control" number="2" future="meaning"/>"#),
        xml(r#"<Input name="integer-control" number="2"><Future/></Input>"#),
        xml(r#"<Input xmlns:q="future" name="integer-control" number="2"/>"#),
        xml(r#"<Input name="boolean-control" boolean="False"/>"#),
        xml(r#"<Placeholder name="boolean-control" boolean="false"/>"#),
        xml("<Future/>"),
        r#"<PathOfBuilding2><Config><Input name="integer-control" number="2"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="01"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/></Config><Config activeConfigSet="1"><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
    ] {
        let imported = source(&text, 0xb3);
        let result = normalize(&imported, &f.f, &default_policy(&f), Default::default()).unwrap();
        assert!(result.draft().input().scenario_presets.members.iter().all(|s| s.scenario.assumptions.members.is_empty()), "{text}");
    }
    let imported = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="boolean-control" boolean="false"/></ConfigSet><ConfigSet id="2"/><ConfigSet id="3"><Input name="encounter-selector" string="other"/></ConfigSet></Config></PathOfBuilding2>"#,
        0xb4,
    );
    let result = normalize(&imported, &f.f, &default_policy(&f), Default::default()).unwrap();
    let mut first = defaults(&f);
    first[2] = ParameterValue::Boolean(false);
    assert_eq!(assumptions(&result, 0), expected(&f, first));
    assert_eq!(assumptions(&result, 1), expected(&f, defaults(&f)));
    assert!(
        assumptions(&result, 2).is_empty(),
        "all targets require their reviewed encounter"
    );
}

#[test]
fn default_policy_rejects_inconsistent_types_recipes_authority_and_duplicate_destinations() {
    let f = default_fixture();
    let imported = source(&xml(""), 0xb5);
    for mutation in 0..18 {
        let mut policy = default_policy(&f);
        let inputs = rows(&mut policy);
        match mutation {
            0 => inputs[1].source_name = inputs[0].source_name.clone(),
            1 => {
                let mut duplicate = inputs[0].clone();
                duplicate.source_name = "duplicate-destination".into();
                duplicate.recipe.tiers[0].selectors[0].name = duplicate.source_name.clone();
                duplicate.recipe.id = key("duplicate-destination");
                inputs.push(duplicate);
            }
            2 => inputs[1].recipe.id = inputs[0].recipe.id.clone(),
            3 => inputs[0].target = ConfigurationInputTarget::Environment,
            4 => inputs[0].constructor_default = ParameterValue::Boolean(false),
            5 => {
                inputs[0].constructor_default =
                    ParameterValue::Integer(BoundedInteger::new(11).unwrap())
            }
            6 => {
                inputs[1].constructor_default =
                    ParameterValue::Quantity(FiniteQuantity::new(11., f.f.unit.clone()).unwrap())
            }
            7 => {
                inputs[1].constructor_default = ParameterValue::Quantity(
                    FiniteQuantity::new(1., UnitDefId::new(ns(), key("missing"))).unwrap(),
                )
            }
            8 => {
                inputs[3].constructor_default =
                    ParameterValue::Option(OptionDefId::new(ns(), key("missing")))
            }
            9 => inputs[0].recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber,
            10 => inputs[2].recipe.tiers[0].selectors[0].lane = ValueLane::InputString,
            11 => inputs[0].recipe.tiers[0].selectors[0].name = "different".into(),
            12 => inputs[0].recipe.codec.whitespace = WhitespacePolicy::TrimAscii,
            13 => inputs[0].recipe.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
            14 => {
                inputs[0].recipe.missing = MissingValuePolicy::Explicit {
                    value: inputs[0].constructor_default.clone(),
                }
            }
            15 => inputs[0].recipe.numeric_aliases.push(NumericTokenAlias {
                token: "alias".into(),
                replacement: "1".into(),
            }),
            16 => {
                let tier = inputs[0].recipe.tiers[0].clone();
                inputs[0].recipe.tiers.push(tier);
            }
            17 => inputs[0].source_name.push(' '),
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f.f, &policy, Default::default()).is_err(),
            "policy mutation {mutation}"
        );
    }
}

#[test]
fn authored_conversion_and_target_identity_are_explicit_while_defaults_are_already_typed() {
    let mut f = default_fixture();
    let mut schema = f.f.a.schema.input().clone();
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::ExternalInput(entry) = descriptor
            && entry.id == f.inputs[0]
        {
            let SchemaState::Known(input) = &mut entry.schema else {
                panic!()
            };
            input.targets.push(AssumptionTargetKind::Environment);
        }
    }
    rebind(&mut f.f.a, schema);
    let mut policy = default_policy(&f);
    let input = &mut rows(&mut policy)[1];
    let ValueCodecKind::Quantity { scale, .. } = &mut input.recipe.codec.codec else {
        panic!()
    };
    scale.numerator = BoundedInteger::new(2).unwrap();
    let mut additional = rows(&mut policy)[0].clone();
    additional.source_name = "environment-integer".into();
    additional.recipe.tiers[0].selectors[0].name = additional.source_name.clone();
    additional.recipe.id = key("environment-integer");
    additional.target = ConfigurationInputTarget::Environment;
    additional.constructor_default = ParameterValue::Integer(BoundedInteger::new(-7).unwrap());
    rows(&mut policy).push(additional);
    for (body, quantity) in [
        ("", 3.5),
        (r#"<Input name="quantity-control" number="2"/>"#, 4.),
    ] {
        let imported = source(&xml(body), 0xb9);
        let result = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        let mut values = defaults(&f);
        values[1] =
            ParameterValue::Quantity(FiniteQuantity::new(quantity, f.f.unit.clone()).unwrap());
        let mut wanted = expected(&f, values);
        wanted.push((
            f.inputs[0].clone(),
            DraftAssumptionTarget::Environment,
            ParameterValue::Integer(BoundedInteger::new(-7).unwrap()),
        ));
        assert_eq!(assumptions(&result, 0), wanted);
    }
    let imported = source(&xml(r#"<Input name="quantity-control" number="6"/>"#), 0xba);
    let result = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
    assert!(
        assumptions(&result, 0)
            .iter()
            .all(|(input, _, _)| input != &f.inputs[1]),
        "decoded values are checked against the final typed domain after conversion"
    );
}

#[test]
fn existing_numeric_and_option_lanes_share_identity_authority_with_defaults() {
    let f = default_fixture();
    let imported = source(&xml(""), 0xb6);
    for mutation in 0..6 {
        let mut policy = default_policy(&f);
        let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
            inputs: old, ..
        }) = reviewed(&f.f).configuration_inputs
        else {
            panic!()
        };
        let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
            inputs,
            option_inputs,
            default_inputs,
            ..
        }) = &mut policy.configuration_inputs
        else {
            panic!()
        };
        inputs.extend(old);
        if mutation < 3 {
            match mutation {
                0 => default_inputs[0].source_name = inputs[0].source_name.clone(),
                1 => default_inputs[0].value_input = inputs[0].value_input.clone(),
                2 => default_inputs[0].recipe.id = inputs[0].recipe.id.clone(),
                _ => unreachable!(),
            }
        } else {
            let input = &default_inputs[3];
            option_inputs.push(ConfigurationOptionInput {
                source_name: input.source_name.clone(),
                value_input: input.value_input.clone(),
                recipe: input.recipe.clone(),
                constructor_default: f.options[1].clone(),
            });
            // Duplicate source, destination and recipe are independently rejected.
            if mutation != 3 {
                option_inputs[0].source_name = "separate-choice".into();
                option_inputs[0].recipe.tiers[0].selectors[0].name = "separate-choice".into();
            }
            if mutation != 4 {
                option_inputs[0].value_input = f.extra_option_input.clone();
            }
            if mutation != 5 {
                option_inputs[0].recipe.id = key("separate-choice");
            }
        }
        assert!(
            normalize(&imported, &f.f, &policy, Default::default()).is_err(),
            "cross-lane mutation {mutation}"
        );
    }
}

#[test]
fn every_target_requires_known_schema_encounter_membership_and_matching_value_domain() {
    for mutation in 0..7 {
        let mut f = default_fixture();
        let mut schema = f.f.a.schema.input().clone();
        for descriptor in &mut schema.definitions {
            match descriptor {
                DefinitionDescriptor::ExternalInput(entry)
                    if mutation < 3 && entry.id == f.inputs[mutation] =>
                {
                    let SchemaState::Known(input) = &mut entry.schema else {
                        panic!()
                    };
                    input.targets = vec![AssumptionTargetKind::Skill];
                }
                DefinitionDescriptor::Encounter(entry)
                    if mutation == 3 && entry.id == f.f.encounter =>
                {
                    let SchemaState::Known(owner) = &mut entry.schema else {
                        panic!()
                    };
                    owner
                        .external_inputs
                        .members
                        .retain(|id| id != &f.inputs[1]);
                }
                DefinitionDescriptor::ExternalInput(entry)
                    if mutation == 4 && entry.id == f.inputs[2] =>
                {
                    entry.schema = unknown(entry.id.clone()).schema
                }
                DefinitionDescriptor::ExternalInput(entry)
                    if mutation == 5 && entry.id == f.inputs[0] =>
                {
                    let SchemaState::Known(input) = &mut entry.schema else {
                        panic!()
                    };
                    input.value = ValueSchema::Boolean;
                }
                DefinitionDescriptor::Option(entry)
                    if mutation == 6 && entry.id == f.options[1] =>
                {
                    entry.schema = unknown(entry.id.clone()).schema
                }
                _ => {}
            }
        }
        rebind(&mut f.f.a, schema);
        let imported = source(&xml(""), 0xb7);
        assert!(
            normalize(&imported, &f.f, &default_policy(&f), Default::default()).is_err(),
            "schema mutation {mutation}"
        );
    }
}

#[test]
fn constructor_defaults_and_authored_values_obey_work_selector_and_output_limits() {
    let f = default_fixture();
    let imported = source(
        &xml(r#"<Input name="quantity-control" number="1e1"/>"#),
        0xb8,
    );
    for mutation in 0..6 {
        let mut limits = NormalizationLimits::default();
        match mutation {
            0 => limits.max_work = 1,
            1 => limits.value.max_selectors = 3,
            2 => limits.value.max_total_selector_bytes = 10,
            3 => limits.value.value.max_source_bytes = 2,
            4 => limits.draft.input.max_collection_entries = 3,
            5 => limits.value.max_selector_bytes = 5,
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f.f, &default_policy(&f), limits).is_err(),
            "limit {mutation}"
        );
    }
    let mut policy = default_policy(&f);
    let repeated = rows(&mut policy)[0].clone();
    *rows(&mut policy) = vec![repeated; 65];
    assert!(matches!(
        normalize(&imported, &f.f, &policy, Default::default()),
        Err(NormalizationError::Limit("configuration input rows"))
    ));
}

#[test]
fn reviewed_numeric_placeholders_do_not_override_authored_values_or_typed_defaults() {
    let f = default_fixture();
    for (body, quantity) in [
        (
            r#"<Placeholder name="quantity-control" number="999"/>"#,
            3.5,
        ),
        (
            r#"<Input name="quantity-control" number="0"/><Placeholder name="quantity-control" number="999"/>"#,
            0.,
        ),
        (
            r#"<Placeholder name="quantity-control" number="999"/><Input name="quantity-control" number="0"/>"#,
            0.,
        ),
    ] {
        let imported = source(&xml(body), 0xbb);
        let mut policy = default_policy(&f);
        let before = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        assert!(
            assumptions(&before, 0)
                .iter()
                .all(|(id, _, _)| id != &f.inputs[1]),
            "the default-false policy still refuses a matching Placeholder"
        );
        rows(&mut policy)[1].ignore_numeric_placeholder = true;
        let after = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        let mut values = defaults(&f);
        values[1] =
            ParameterValue::Quantity(FiniteQuantity::new(quantity, f.f.unit.clone()).unwrap());
        assert_eq!(assumptions(&after, 0), expected(&f, values));
        let placeholder = imported
            .occurrences()
            .iter()
            .find(|row| row.name() == "Placeholder")
            .unwrap()
            .id();
        assert_eq!(
            after
                .sidecar()
                .origins
                .iter()
                .find(|row| row.source == placeholder)
                .unwrap(),
            before
                .sidecar()
                .origins
                .iter()
                .find(|row| row.source == placeholder)
                .unwrap(),
            "ignored display values keep every prior source obligation",
        );
        assert_eq!(before.allocator_after(), after.allocator_after());
        let mut inverse = after.draft().input().clone();
        inverse.scenario_presets.members[0]
            .scenario
            .assumptions
            .members
            .retain(|row| row.input.to_resolved().as_ref() != Some(&f.inputs[1]));
        assert_eq!(inverse, *before.draft().input());
    }
    let mut policy = default_policy(&f);
    rows(&mut policy)[1].ignore_numeric_placeholder = true;
    for body in [
        r#"<Placeholder name="quantity-control" number="bad"/>"#,
        r#"<Placeholder name="quantity-control" number="NaN"/>"#,
        r#"<Placeholder name="quantity-control" string="999"/>"#,
        r#"<Placeholder name="quantity-control" boolean="false"/>"#,
        r#"<Input name="quantity-control" number="0"/><Placeholder name="quantity-control" number="bad"/>"#,
    ] {
        let imported = source(&xml(body), 0xbc);
        let result = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        assert!(
            assumptions(&result, 0)
                .iter()
                .all(|(id, _, _)| id != &f.inputs[1]),
            "{body}"
        );
    }
}

#[test]
fn numeric_placeholder_permission_is_explicit_typed_and_bounded() {
    let f = default_fixture();
    let baseline = default_policy(&f);
    let mut integer = baseline.clone();
    rows(&mut integer)[0].ignore_numeric_placeholder = true;
    for body in [
        r#"<Placeholder name="integer-control" number="999"/>"#,
        r#"<Placeholder name="integer-control" number="999"/><Input name="integer-control" number="0"/>"#,
    ] {
        let imported = source(&xml(body), 0xbf);
        let actual = normalize(&imported, &f.f, &integer, Default::default()).unwrap();
        let mut values = defaults(&f);
        if body.contains("<Input ") {
            values[0] = ParameterValue::Integer(BoundedInteger::new(0).unwrap());
        }
        assert_eq!(assumptions(&actual, 0), expected(&f, values));
    }
    let imported = source(
        &xml(r#"<Placeholder name="integer-control" number="1.5"/>"#),
        0xbf,
    );
    let actual = normalize(&imported, &f.f, &integer, Default::default()).unwrap();
    assert!(
        assumptions(&actual, 0)
            .iter()
            .all(|(id, _, _)| id != &f.inputs[0])
    );
    let wire = serde_json::to_value(&baseline).unwrap();
    for row in wire["configuration_inputs"]["default_inputs"]
        .as_array()
        .unwrap()
    {
        assert!(row.get("ignore_numeric_placeholder").is_none());
    }
    let restored: NormalizationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), wire);
    let imported = source(&xml(""), 0xbd);
    for index in [2, 3] {
        let mut policy = baseline.clone();
        rows(&mut policy)[index].ignore_numeric_placeholder = true;
        assert!(
            normalize(&imported, &f.f, &policy, Default::default()).is_err(),
            "Boolean and Option controls cannot borrow numeric-placeholder authority"
        );
    }
    let mut policy = baseline;
    rows(&mut policy)[1].ignore_numeric_placeholder = true;
    let wire = serde_json::to_value(&policy).unwrap();
    assert_eq!(
        wire["configuration_inputs"]["default_inputs"][1]["ignore_numeric_placeholder"],
        true
    );
    let restored: NormalizationPolicy = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(serde_json::to_value(restored).unwrap(), wire);
    let imported = source(
        &xml(r#"<Placeholder name="quantity-control" number="999"/>"#),
        0xbe,
    );
    for limits in [
        NormalizationLimits {
            max_work: 1,
            ..Default::default()
        },
        {
            let mut limits = NormalizationLimits::default();
            limits.value.value.max_source_bytes = 2;
            limits
        },
    ] {
        assert!(
            matches!(
                normalize(&imported, &f.f, &policy, limits),
                Err(NormalizationError::Limit(_))
            ),
            "even ignored numeric text must pass bounded decoding"
        );
    }
}
