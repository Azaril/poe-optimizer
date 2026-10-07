//! Exact string controls extend, rather than reinterpret, the numeric lanes.
use super::*;

const TOKENS: [&str; 7] = [
    "Average",
    "Untyped",
    "DamageOverTime",
    "Melee",
    "Projectile",
    "Spell",
    "SpellProjectile",
];
const SOURCE_NAME: &str = "enemyDamageType";

struct OptionFixture {
    f: Fixture,
    value_input: ExternalInputDefId,
    options: Vec<OptionDefId>,
}

fn option_fixture() -> OptionFixture {
    let mut f = fixture();
    let mut schema = f.a.schema.input().clone();
    let options: Vec<_> = TOKENS
        .iter()
        .map(|_| {
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
            id
        })
        .collect();
    let value_input =
        f.a.registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
    schema
        .definitions
        .push(DefinitionDescriptor::ExternalInput(DefinitionEntry {
            id: value_input.clone(),
            schema: SchemaState::Known(ExternalInputSchema {
                value: ValueSchema::Option {
                    allowed: DeclaredSet::complete(options.clone()),
                },
                targets: vec![AssumptionTargetKind::Enemy],
            }),
        }));
    for entry in &mut schema.definitions {
        if let DefinitionDescriptor::Encounter(entry) = entry
            && entry.id == f.encounter
        {
            let SchemaState::Known(encounter) = &mut entry.schema else {
                panic!()
            };
            encounter.external_inputs.members.push(value_input.clone());
        }
    }
    rebind(&mut f.a, schema);
    OptionFixture {
        f,
        value_input,
        options,
    }
}

fn option_policy(f: &OptionFixture) -> NormalizationPolicy {
    let mut policy = fallback_tests::fallback_policy(&f.f);
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
        mapping_source,
        encounter,
        inputs,
        placeholder_fallback_inputs,
    }) = policy.configuration_inputs.take()
    else {
        panic!()
    };
    let mut recipe = value_recipe("incoming-category", SOURCE_NAME, false);
    recipe.tiers[0].selectors[0].lane = ValueLane::InputString;
    recipe.codec.codec = ValueCodecKind::Option {
        tokens: TOKENS
            .iter()
            .zip(&f.options)
            .map(|(token, value)| OptionToken {
                token: (*token).into(),
                value: value.clone(),
            })
            .collect(),
    };
    policy.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source,
        encounter,
        inputs,
        placeholder_fallback_inputs,
        option_inputs: vec![ConfigurationOptionInput {
            source_name: SOURCE_NAME.into(),
            value_input: f.value_input.clone(),
            recipe,
            constructor_default: f.options[0].clone(),
        }],
        default_inputs: vec![],
    });
    policy
}

fn option_row(policy: &mut NormalizationPolicy) -> &mut ConfigurationOptionInput {
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { option_inputs, .. }) =
        &mut policy.configuration_inputs
    else {
        panic!()
    };
    &mut option_inputs[0]
}

#[test]
fn options_are_injected_exact_typed_values_and_do_not_complete_the_scenario() {
    let f = option_fixture();
    for (token, option) in TOKENS.iter().zip(&f.options) {
        let body = format!(
            r#"<Input name="{SOURCE_NAME}" string="{token}"/><Input name="raw-0" number="17"/><Placeholder name="raw-1" number="23"/>"#
        );
        let imported = source(&xml(&body), 0xa1);
        let baseline = normalize(
            &imported,
            &f.f,
            &fallback_tests::fallback_policy(&f.f),
            Default::default(),
        )
        .unwrap();
        let actual = normalize(&imported, &f.f, &option_policy(&f), Default::default()).unwrap();
        let mut expected = values(&baseline, 0);
        expected.push((
            f.value_input.clone(),
            ParameterValue::Option(option.clone()),
        ));
        assert_eq!(values(&actual, 0), expected, "{token}");
        let mut restored = actual.draft().input().clone();
        restored.scenario_presets.members[0]
            .scenario
            .assumptions
            .members
            .pop();
        assert_eq!(restored, *baseline.draft().input());
        assert_eq!(actual.allocator_after(), baseline.allocator_after());
        assert!(matches!(
            actual.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let scenario = actual.draft().input().scenario_presets.members[0].id;
        let origin = imported
            .occurrences()
            .iter()
            .find(|row| row.name() == "Input")
            .unwrap()
            .id();
        let mut expected_origins = baseline.sidecar().origins.clone();
        expected_origins
            .iter_mut()
            .find(|row| row.source == origin)
            .unwrap()
            .links
            .push(OwnedOriginTarget::ScenarioPreset(scenario));
        assert_eq!(actual.sidecar().origins, expected_origins);
    }
}

#[test]
fn only_absence_on_a_fresh_frame_admits_the_injected_constructor_default() {
    let f = option_fixture();
    let imported = source(&xml(""), 0xa2);
    let baseline = normalize(
        &imported,
        &f.f,
        &fallback_tests::fallback_policy(&f.f),
        Default::default(),
    )
    .unwrap();
    // The supplied default need not be the first codec token or a game constant.
    let mut policy = option_policy(&f);
    option_row(&mut policy).constructor_default = f.options[5].clone();
    let actual = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
    assert_eq!(
        values(&actual, 0).last(),
        Some(&(
            f.value_input.clone(),
            ParameterValue::Option(f.options[5].clone())
        ))
    );
    let scope = imported
        .occurrences()
        .iter()
        .find(|row| row.name() == "ConfigSet")
        .unwrap()
        .id();
    let scenario = actual.draft().input().scenario_presets.members[0].id;
    assert_eq!(actual.sidecar().origins, baseline.sidecar().origins);
    let links = &actual
        .sidecar()
        .origins
        .iter()
        .find(|row| row.source == scope)
        .unwrap()
        .links;
    assert_eq!(
        links
            .iter()
            .filter(|link| **link == OwnedOriginTarget::ScenarioPreset(scenario))
            .count(),
        1,
        "constructor provenance reuses the exact existing scope link"
    );

    for body in [
        r#"<Input name="enemyDamageType" string="Future"/>"#,
        r#"<Input name="enemyDamageType" string=" Average"/>"#,
        r#"<Input name="enemyDamageType" string="Average "/>"#,
        r#"<Input name="enemyDamageType" string="average"/>"#,
        r#"<Input name="enemyDamageType" string=""/>"#,
        r#"<Input name="enemyDamageType" number="1"/>"#,
        r#"<Input name="enemyDamageType" boolean="true"/>"#,
        r#"<Placeholder name="enemyDamageType" string="Average"/>"#,
        r#"<Placeholder name="enemyDamageType" number="1"/>"#,
        r#"<Input name="enemyDamageType" string="Melee"/><Placeholder name="enemyDamageType" string="Average"/>"#,
        r#"<Placeholder name="enemyDamageType" string="Average"/><Input name="enemyDamageType" string="Melee"/>"#,
    ] {
        let imported = source(&xml(body), 0xa3);
        let actual = normalize(&imported, &f.f, &policy, Default::default()).unwrap();
        assert!(
            values(&actual, 0)
                .iter()
                .all(|(id, _)| id != &f.value_input),
            "{body}"
        );
    }
}

#[test]
fn ambiguous_aliased_namespaced_and_unrecognized_frames_never_supply_defaults() {
    let f = option_fixture();
    let duplicated = xml(r#"<Input name="enemyDamageType" name="unrelated" string="Average"/>"#);
    assert!(
        decode_build(duplicated.as_bytes()).is_err(),
        "duplicate attributes are rejected at XML intake before normalization"
    );
    for text in [
        xml(r#"<Input name="enemyDamageType" string="Average"/><Input name="enemyDamageType" string="Average"/>"#),
        xml(r#"<Input name="enemyDamageType" string="Average" number="1"/>"#),
        xml(r#"<Input name="enemyDamageType" string="Average"><Future/></Input>"#),
        xml(r#"<Input xmlns:q="future" name="enemyDamageType" string="Average"/>"#),
        xml(r#"<Input name="enemyDamageType" string="Average" future="1"/>"#),
        xml(r#"<Future name="enemyDamageType" string="Average"/>"#),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="01"/><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"/></Config></PathOfBuilding2>"#.into(),
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"/></Config><Config activeConfigSet="2"><ConfigSet id="2"/></Config></PathOfBuilding2>"#.into(),
    ] {
        let imported = source(&text, 0xa4);
        let actual = normalize(&imported, &f.f, &option_policy(&f), Default::default()).unwrap();
        assert!(actual.draft().input().scenario_presets.members.iter().all(|s| s.scenario.assumptions.members.is_empty()), "{text}");
    }
}

#[test]
fn string_controls_are_scope_local_and_require_the_reviewed_encounter() {
    let f = option_fixture();
    let imported = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="enemyDamageType" string="Melee"/></ConfigSet><ConfigSet id="2"/><ConfigSet id="3"><Input name="encounter-selector" string="other"/></ConfigSet><ConfigSet id="4"><Input name="enemyDamageType" string="Future"/></ConfigSet></Config></PathOfBuilding2>"#,
        0xa5,
    );
    let actual = normalize(&imported, &f.f, &option_policy(&f), Default::default()).unwrap();
    assert_eq!(
        values(&actual, 0).last(),
        Some(&(
            f.value_input.clone(),
            ParameterValue::Option(f.options[3].clone())
        ))
    );
    assert_eq!(
        values(&actual, 1).last(),
        Some(&(
            f.value_input.clone(),
            ParameterValue::Option(f.options[0].clone())
        ))
    );
    assert!(values(&actual, 2).is_empty());
    assert!(
        values(&actual, 3)
            .iter()
            .all(|(id, _)| id != &f.value_input)
    );
}

#[test]
fn option_policy_rejects_cross_lane_identities_namespaces_and_unreviewed_recipes() {
    let f = option_fixture();
    let imported = source(&xml(""), 0xa6);
    for mutation in 0..26 {
        let mut policy = option_policy(&f);
        let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs,
            option_inputs,
            ..
        }) = &mut policy.configuration_inputs
        else {
            panic!()
        };
        let input = &mut option_inputs[0];
        match mutation {
            0 => *mapping_source = *f.f.a.mapping.identity(),
            1 => *encounter = EncounterDefId::new(ns(), key("missing")),
            2 => input.source_name = inputs[0].source_name.clone(),
            3 => input.source_name = placeholder_fallback_inputs[0].source_name.clone(),
            4 => input.value_input = inputs[0].presence_input.clone(),
            5 => input.value_input = placeholder_fallback_inputs[0].value_input.clone(),
            6 => input.recipe.id = inputs[0].recipe.id.clone(),
            7 => input.recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderString,
            8 => input.recipe.tiers[0].selectors[0].name = "other".into(),
            9 => input.recipe.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
            10 => input.recipe.codec.whitespace = WhitespacePolicy::TrimAscii,
            11 => {
                input.recipe.missing = MissingValuePolicy::Explicit {
                    value: ParameterValue::Option(f.options[0].clone()),
                }
            }
            12 => input.recipe.numeric_aliases.push(NumericTokenAlias {
                token: "alias".into(),
                replacement: "Average".into(),
            }),
            13 => {
                input.value_input = ExternalInputDefId::new(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    key("input"),
                )
            }
            14 => {
                input.constructor_default = OptionDefId::new(
                    GameVersionNamespace::new("foreign", "v1").unwrap(),
                    key("option"),
                )
            }
            15 => {
                input.recipe.codec.namespace = GameVersionNamespace::new("foreign", "v1").unwrap()
            }
            16 => input.constructor_default = OptionDefId::new(ns(), key("missing")),
            17 => input.recipe.codec.codec = ValueCodecKind::Boolean { tokens: vec![] },
            18 => input.recipe.tiers.push(input.recipe.tiers[0].clone()),
            19 => input.source_name.push(' '),
            20..=23 => {
                let ValueCodecKind::Option { tokens } = &mut input.recipe.codec.codec else {
                    panic!()
                };
                match mutation {
                    20 => tokens[0].value = OptionDefId::new(ns(), key("missing")),
                    21 => {
                        tokens[0].value = OptionDefId::new(
                            GameVersionNamespace::new("foreign", "v1").unwrap(),
                            key("option"),
                        )
                    }
                    22 => tokens.push(tokens[0].clone()),
                    23 => tokens.clear(),
                    _ => unreachable!(),
                }
            }
            24 => option_inputs.push(option_inputs[0].clone()),
            25 => {
                let mut other = option_inputs[0].clone();
                other.source_name = "distinct-option".into();
                other.recipe.id = key("distinct-option");
                other.recipe.tiers[0].selectors[0].name = other.source_name.clone();
                option_inputs.push(other);
            }
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f.f, &policy, Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn option_target_schema_and_every_mapped_or_default_definition_must_be_known() {
    for mutation in 0..7 {
        let mut f = option_fixture();
        let mut schema = f.f.a.schema.input().clone();
        for descriptor in &mut schema.definitions {
            match descriptor {
                DefinitionDescriptor::ExternalInput(entry) if entry.id == f.value_input => {
                    if mutation == 0 {
                        entry.schema = unknown(entry.id.clone()).schema;
                    } else {
                        let SchemaState::Known(raw) = &mut entry.schema else {
                            panic!()
                        };
                        match mutation {
                            1 => raw.targets = vec![AssumptionTargetKind::Actor],
                            2 => raw.value = ValueSchema::Boolean,
                            5 | 6 => {
                                let ValueSchema::Option { allowed } = &mut raw.value else {
                                    panic!()
                                };
                                allowed
                                    .members
                                    .retain(|id| id != &f.options[usize::from(mutation == 6)]);
                            }
                            _ => {}
                        }
                    }
                }
                DefinitionDescriptor::Encounter(entry) if mutation == 3 => {
                    let SchemaState::Known(owner) = &mut entry.schema else {
                        panic!()
                    };
                    owner
                        .external_inputs
                        .members
                        .retain(|id| id != &f.value_input);
                }
                DefinitionDescriptor::Option(entry)
                    if mutation == 4 && entry.id == f.options[1] =>
                {
                    entry.schema = unknown(entry.id.clone()).schema;
                }
                _ => {}
            }
        }
        rebind(&mut f.f.a, schema);
        let imported = source(&xml(""), 0xa7);
        assert!(
            normalize(&imported, &f.f, &option_policy(&f), Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn option_policy_source_work_and_output_budgets_are_bounded() {
    let f = option_fixture();
    let imported = source(
        &xml(r#"<Input name="enemyDamageType" string="SpellProjectile"/>"#),
        0xa8,
    );
    for mutation in 0..9 {
        let mut limits = NormalizationLimits::default();
        match mutation {
            0 => limits.max_work = 1,
            1 => limits.value.max_selectors = 3,
            2 => limits.value.max_total_selector_bytes = 16,
            3 => limits.value.value.max_source_bytes = 3,
            4 => limits.draft.input.max_collection_entries = 4,
            5 => limits.value.value.max_tokens = 6,
            6 => limits.value.value.max_token_bytes = 3,
            7 => limits.value.value.max_total_token_bytes = 20,
            8 => limits.value.max_selector_bytes = 4,
            _ => unreachable!(),
        }
        assert!(
            normalize(&imported, &f.f, &option_policy(&f), limits).is_err(),
            "limit {mutation}"
        );
    }
    let mut policy = option_policy(&f);
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { option_inputs, .. }) =
        &mut policy.configuration_inputs
    else {
        panic!()
    };
    *option_inputs = vec![option_inputs[0].clone(); 63];
    assert!(matches!(
        normalize(&imported, &f.f, &policy, Default::default()),
        Err(NormalizationError::Limit("configuration input rows"))
    ));
}

#[test]
fn empty_option_extension_preserves_v1_v2_drafts_origins_and_historical_wire() {
    let f = option_fixture();
    for old_policy in [reviewed(&f.f), fallback_tests::fallback_policy(&f.f)] {
        let wire = serde_json::to_value(&old_policy).unwrap();
        assert!(wire["configuration_inputs"].get("option_inputs").is_none());
        let mut extended = old_policy.clone();
        let (mapping_source, encounter, inputs, placeholder_fallback_inputs) =
            match extended.configuration_inputs.take().unwrap() {
                ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 {
                    mapping_source,
                    encounter,
                    inputs,
                } => (mapping_source, encounter, inputs, vec![]),
                ConfigurationInputsPolicy::PobFreshNumericConfigFallbacksV2 {
                    mapping_source,
                    encounter,
                    inputs,
                    placeholder_fallback_inputs,
                } => (
                    mapping_source,
                    encounter,
                    inputs,
                    placeholder_fallback_inputs,
                ),
                _ => unreachable!(),
            };
        extended.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
            mapping_source,
            encounter,
            inputs,
            placeholder_fallback_inputs,
            option_inputs: vec![],
            default_inputs: vec![],
        });
        for body in [
            "",
            r#"<Input name="raw-0" number="0"/><Placeholder name="raw-1" number="37"/>"#,
            r#"<Input name="raw-1" number="-7"/><Placeholder name="raw-1" number="37"/>"#,
        ] {
            let imported = source(&xml(body), 0xa9);
            let before = normalize(&imported, &f.f, &old_policy, Default::default()).unwrap();
            let after = normalize(&imported, &f.f, &extended, Default::default()).unwrap();
            assert_eq!(before.draft(), after.draft());
            assert_eq!(before.allocator_after(), after.allocator_after());
            let mut sidecar = serde_json::to_value(after.sidecar()).unwrap();
            let baseline = serde_json::to_value(before.sidecar()).unwrap();
            sidecar["policy"] = baseline["policy"].clone();
            assert_eq!(sidecar, baseline);
        }
        assert_eq!(serde_json::to_value(old_policy).unwrap(), wire);
    }
}

fn minimum_work(
    imported: &ImportedBuildInstance,
    fixture: &Fixture,
    policy: &NormalizationPolicy,
) -> usize {
    let mut low = 1usize;
    let mut high = NormalizationLimits::default().max_work;
    assert!(normalize(imported, fixture, policy, Default::default()).is_ok());
    while low < high {
        let middle = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: middle,
            ..Default::default()
        };
        match normalize(imported, fixture, policy, limits) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected error while measuring work: {error}"),
        }
    }
    low
}

#[test]
fn shared_configuration_census_avoids_revalidating_immutable_source_bytes() {
    let f = option_fixture();
    // These valid, unrelated controls must all be inspected to authenticate
    // the frame. Consumer-specific selection still visits them independently.
    let value = "x".repeat(96);
    let body: String = (0..200)
        .map(|index| format!(r#"<Input name="unrelated-{index}" string="{value}"/>"#))
        .collect();
    let imported = source(&xml(&body), 0xaa);
    let combined = option_policy(&f);
    let mut encounter_only = combined.clone();
    encounter_only.configuration_inputs = None;
    let single_work = minimum_work(&imported, &f.f, &encounter_only);
    let combined_work = minimum_work(&imported, &f.f, &combined);
    assert!(
        combined_work > single_work,
        "independent consumer work is charged"
    );
    assert!(
        combined_work - single_work < body.len() / 2,
        "a second policy consumer must reuse the source-only census: single={single_work}, combined={combined_work}, source_bytes={}",
        body.len(),
    );
    let expected = normalize(&imported, &f.f, &combined, Default::default()).unwrap();
    let exact = normalize(
        &imported,
        &f.f,
        &combined,
        NormalizationLimits {
            max_work: combined_work,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact.draft(), expected.draft());
    assert_eq!(
        serde_json::to_vec(exact.sidecar()).unwrap(),
        serde_json::to_vec(expected.sidecar()).unwrap()
    );
    assert_eq!(exact.allocator_after(), expected.allocator_after());
    assert!(matches!(
        normalize(
            &imported,
            &f.f,
            &combined,
            NormalizationLimits {
                max_work: combined_work - 1,
                ..Default::default()
            },
        ),
        Err(NormalizationError::Limit(_)),
    ));
}

#[test]
fn rejected_and_failed_censuses_never_become_defaults_or_leak_across_imports() {
    let f = option_fixture();
    let policy = option_policy(&f);
    for body in [
        "<Future/>",
        r#"<Input name="enemyDamageType" string="Average"/><Input name="enemyDamageType" string="Melee"/>"#,
        r#"<Input name="enemyDamageType" string="Average" number="1"/>"#,
    ] {
        let rejected = source(&xml(body), 0xab);
        let result = normalize(&rejected, &f.f, &policy, Default::default()).unwrap();
        assert!(
            values(&result, 0).is_empty(),
            "every consumer retains rejected-frame authority"
        );
        assert!(
            result.draft().input().scenario_presets.members[0]
                .scenario
                .enemy
                .encounter
                .to_resolved()
                .is_none()
        );
    }
    let valid = source(&xml(""), 0xac);
    assert!(matches!(
        normalize(
            &valid,
            &f.f,
            &policy,
            NormalizationLimits {
                max_work: 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_)),
    ));
    let actual = normalize(&valid, &f.f, &policy, Default::default()).unwrap();
    assert_eq!(
        values(&actual, 0).last(),
        Some(&(
            f.value_input.clone(),
            ParameterValue::Option(f.options[0].clone())
        ))
    );
    assert_eq!(
        actual.draft().input().scenario_presets.members[0]
            .scenario
            .enemy
            .encounter
            .to_resolved(),
        Some(f.f.encounter)
    );
}
