//! One checked control, independently selected typed outputs and leaf receipts.
use super::*;

const NAME: &str = "portable-distance";

fn projection_policy(f: &Fixture) -> NormalizationPolicy {
    let mut p = reviewed(f);
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 { inputs, .. }) =
        p.configuration_inputs.take()
    else {
        panic!()
    };
    let default_inputs = inputs
        .into_iter()
        .enumerate()
        .map(|(i, input)| {
            let mut recipe = input.recipe;
            recipe.tiers[0].selectors[0].name = NAME.into();
            ConfigurationDefaultInput {
                source_name: NAME.into(),
                value_input: input.value_input,
                target: ConfigurationInputTarget::Enemy,
                recipe,
                constructor_default: ParameterValue::Quantity(
                    FiniteQuantity::new(20., f.unit.clone()).unwrap(),
                ),
                selection: if i == 0 {
                    ConfigurationValueSelection::InputThenPlaceholder
                } else {
                    ConfigurationValueSelection::NonzeroInputThenPlaceholder
                },
                ignore_numeric_placeholder: false,
            }
        })
        .collect();
    p.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source: *f.a.mapping.source_identity(),
        encounter: f.encounter.clone(),
        inputs: vec![],
        placeholder_fallback_inputs: vec![],
        option_inputs: vec![],
        default_inputs,
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

fn accounted(f: &Fixture, p: NormalizationPolicy) -> NormalizationPolicy {
    let destinations: Vec<_> = f
        .inputs
        .iter()
        .map(|(_, id)| ConfigurationDestination::Adapter {
            control: ConfigurationAdapterControl::ScalarInput { input: id.clone() },
        })
        .collect();
    let rules = [ValueLane::InputNumber, ValueLane::PlaceholderNumber]
        .into_iter()
        .enumerate()
        .map(|(i, lane)| ConfigurationDispositionRule {
            id: key(&format!("projection-roles-{i}")),
            selector: ValueSelector {
                lane,
                name: NAME.into(),
            },
            destinations: destinations.clone(),
        })
        .collect();
    accounting_tests::bind(f, p, rules)
}

fn body(input: Option<&str>, placeholder: Option<&str>) -> String {
    [("Input", input), ("Placeholder", placeholder)]
        .into_iter()
        .filter_map(|(kind, value)| {
            value.map(|value| format!(r#"<{kind} name="{NAME}" number="{value}"/>"#))
        })
        .collect()
}

fn expected(f: &Fixture, a: f64, b: f64) -> Vec<(ExternalInputDefId, ParameterValue)> {
    f.inputs
        .iter()
        .zip([a, b])
        .map(|((_, id), value)| {
            (
                id.clone(),
                ParameterValue::Quantity(FiniteQuantity::new(value, f.unit.clone()).unwrap()),
            )
        })
        .collect()
}

#[test]
fn projections_preserve_zero_and_record_every_selected_and_unselected_leaf_role() {
    let f = fixture();
    let p = accounted(&f, projection_policy(&f));
    for (input, placeholder, a, b) in [
        (None, None, 20., 20.),
        (None, Some("20"), 20., 20.),
        (Some("0"), Some("80"), 0., 80.),
        (Some("-0"), Some("80"), 0., 80.),
        (Some("0"), None, 0., 20.),
        (Some("7"), Some("80"), 7., 7.),
        (Some("-17.25"), Some("80"), -17.25, -17.25),
        (None, Some("0"), 0., 0.),
        (Some("0"), Some("0"), 0., 0.),
        (Some("100"), Some("-100"), 100., 100.),
    ] {
        let text = body(input, placeholder);
        let source = source(&xml(&text), 0x96);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(values(&result, 0), expected(&f, a, b), "{text}");
        let scenario = &result.draft().input().scenario_presets.members[0];
        for (kind, present) in [("Input", input), ("Placeholder", placeholder)] {
            if present.is_none() {
                continue;
            }
            let row = accounting_tests::leaf(&source, &result, NAME, kind);
            assert_eq!(
                row.links,
                vec![OwnedOriginTarget::ScenarioPreset(scenario.id)],
                "{text}"
            );
            let skipped_by_both =
                kind == "Placeholder" && input.is_some_and(|s| s != "0" && s != "-0");
            if skipped_by_both {
                assert_eq!(
                    row.disposition,
                    SourceDisposition::SourceOnly(key("unselected-config-projection"))
                );
            } else {
                assert_eq!(row.disposition, SourceDisposition::Contributes);
            }
        }
        // Importing values never grants whole-scenario completeness.
        assert!(matches!(
            scenario.scenario.assumptions.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert_eq!(
            result.draft(),
            normalize(&source, &f, &p, Default::default())
                .unwrap()
                .draft()
        );
    }
}

#[test]
fn every_projection_reuses_failure_of_the_shared_control_including_the_losing_lane() {
    let f = fixture();
    let p = accounted(&f, projection_policy(&f));
    for text in [
        body(Some("7"), Some("NaN")),
        body(Some("7"), Some("101")),
        body(Some("NaN"), Some("80")),
        body(Some("101"), Some("80")),
        format!(r#"<Input name="{NAME}" string="0"/><Placeholder name="{NAME}" number="80"/>"#),
        format!(r#"<Input name="{NAME}" number="7"/><Placeholder name="{NAME}" string="80"/>"#),
        format!("{}{}", body(Some("0"), Some("80")), body(Some("0"), None)),
        format!("{}{}", body(Some("0"), Some("80")), body(None, Some("80"))),
    ] {
        let source = source(&xml(&text), 0x97);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        assert!(values(&result, 0).is_empty(), "{text}");
        for kind in ["Input", "Placeholder"] {
            accounting_tests::pending(accounting_tests::leaf(&source, &result, NAME, kind));
        }
    }
}

#[test]
fn no_accounting_policy_or_missing_projection_never_retires_a_leaf() {
    let f = fixture();
    let source = source(&xml(&body(Some("0"), Some("80"))), 0x98);
    let plain = projection_policy(&f);
    let result = normalize(&source, &f, &plain, Default::default()).unwrap();
    assert_eq!(values(&result, 0), expected(&f, 0., 80.));
    for kind in ["Input", "Placeholder"] {
        accounting_tests::pending(accounting_tests::leaf(&source, &result, NAME, kind));
    }
    // An output removed from policy cannot satisfy the still-declared role.
    let mut missing = projection_policy(&f);
    rows(&mut missing).pop();
    let missing = accounted(&f, missing);
    let result = normalize(&source, &f, &missing, Default::default()).unwrap();
    assert_eq!(values(&result, 0).len(), 1);
    for kind in ["Input", "Placeholder"] {
        accounting_tests::pending(accounting_tests::leaf(&source, &result, NAME, kind));
    }
    // Omitting a required destination is itself invalid authoring, even if the
    // remaining output's value and exact source link are perfectly valid.
    let mut missing_role = accounted(&f, projection_policy(&f));
    missing_role
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .rules[0]
        .destinations
        .pop();
    assert!(normalize(&source, &f, &missing_role, Default::default()).is_err());
    let mut duplicate_role = accounted(&f, projection_policy(&f));
    let rule = &mut duplicate_role
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .rules[0];
    rule.destinations.push(rule.destinations[0].clone());
    assert!(normalize(&source, &f, &duplicate_role, Default::default()).is_err());
}

#[test]
fn projections_require_one_numeric_source_contract_and_distinct_output_identity() {
    let f = fixture();
    let source = source(&xml(&body(Some("0"), Some("80"))), 0x99);
    for mutation in 0..10 {
        let mut p = projection_policy(&f);
        let r = rows(&mut p);
        match mutation {
            0 => {
                r[1].constructor_default =
                    ParameterValue::Quantity(FiniteQuantity::new(21., f.unit.clone()).unwrap())
            }
            1 => r[1].selection = ConfigurationValueSelection::InputOrConstructor,
            2 => r[1].ignore_numeric_placeholder = true,
            3 => r[1].value_input = r[0].value_input.clone(),
            4 => r[1].recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber,
            5 => {
                let tier = r[1].recipe.tiers[0].clone();
                r[1].recipe.tiers.push(tier);
            }
            6 => r[1].constructor_default = ParameterValue::Boolean(false),
            7 => r[1].recipe.codec.whitespace = WhitespacePolicy::TrimAscii,
            8 => r[1].recipe.tiers[0].selectors[0].name = "another-control".into(),
            9 => {
                let ValueCodecKind::Quantity { scale, .. } = &mut r[1].recipe.codec.codec else {
                    panic!()
                };
                scale.numerator = BoundedInteger::new(2).unwrap();
            }
            _ => unreachable!(),
        }
        assert!(
            normalize(&source, &f, &p, Default::default()).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn projections_recompute_each_scope_and_never_share_authored_values_across_presets() {
    let f = fixture();
    let p = accounted(&f, projection_policy(&f));
    for active in [1, 2] {
        let xml = format!(
            r#"<PathOfBuilding2><Config activeConfigSet="{active}"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2">{}</ConfigSet></Config></PathOfBuilding2>"#,
            body(Some("0"), Some("80")),
            body(Some("7"), Some("80"))
        );
        let source = source(&xml, 0x9a);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(values(&result, 0), expected(&f, 0., 80.));
        assert_eq!(values(&result, 1), expected(&f, 7., 7.));
        for row in result.sidecar().origins.iter().filter(|o| {
            o.links
                .iter()
                .any(|l| matches!(l, OwnedOriginTarget::ScenarioPreset(_)))
        }) {
            let targets: Vec<_> = row
                .links
                .iter()
                .filter(|l| matches!(l, OwnedOriginTarget::ScenarioPreset(_)))
                .collect();
            assert_eq!(targets.len(), 1, "no per-projection duplicate source link");
        }
    }
}

#[test]
fn projection_targets_and_ranges_cannot_alias_receipts_or_split_the_source_census() {
    for changed_range in [false, true] {
        let mut f = fixture();
        let mut schema = f.a.schema.input().clone();
        for definition in &mut schema.definitions {
            if let DefinitionDescriptor::ExternalInput(entry) = definition {
                let SchemaState::Known(value) = &mut entry.schema else {
                    panic!()
                };
                if entry.id == f.inputs[0].1 {
                    value.targets.push(AssumptionTargetKind::Environment);
                }
                if entry.id == f.inputs[1].1 {
                    value.targets.push(AssumptionTargetKind::Environment);
                    if changed_range {
                        let ValueSchema::Quantity(range) = &mut value.value else {
                            panic!()
                        };
                        range.maximum = FiniteQuantity::new(99., f.unit.clone()).unwrap();
                    }
                }
            }
        }
        rebind(&mut f.a, schema);
        let source = source(&xml(&body(Some("0"), Some("80"))), 0x9b);
        let mut p = projection_policy(&f);
        rows(&mut p)[1].target = ConfigurationInputTarget::Environment;
        if changed_range {
            assert!(normalize(&source, &f, &p, Default::default()).is_err());
            continue;
        }
        let p = accounted(&f, p);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        let assumptions = &result.draft().input().scenario_presets.members[0]
            .scenario
            .assumptions
            .members;
        assert_eq!(
            assumptions[0].target,
            DraftAssumptionTarget::Actor(DraftActorKey::ScenarioEnemy)
        );
        assert_eq!(assumptions[1].target, DraftAssumptionTarget::Environment);
        for kind in ["Input", "Placeholder"] {
            let leaf = accounting_tests::leaf(&source, &result, NAME, kind);
            assert_eq!(leaf.links.len(), 1);
        }
        let mut alias = projection_policy(&f);
        rows(&mut alias)[1].value_input = f.inputs[0].1.clone();
        rows(&mut alias)[1].target = ConfigurationInputTarget::Environment;
        assert!(normalize(&source, &f, &alias, Default::default()).is_err());
    }
}

#[test]
fn a_non_numeric_projection_or_changed_source_codec_never_acquires_numeric_authority() {
    let f = fixture();
    let source = source(&xml(&body(Some("0"), Some("80"))), 0x9c);
    let mut p = projection_policy(&f);
    let first = &mut rows(&mut p)[0];
    first.value_input = f.inputs[0].0.clone();
    first.recipe = value_recipe("boolean-projection", NAME, true);
    first.recipe.tiers[0].selectors[0].lane = ValueLane::InputBoolean;
    first.constructor_default = ParameterValue::Boolean(false);
    assert!(normalize(&source, &f, &p, Default::default()).is_err());

    let mut p = projection_policy(&f);
    let ValueCodecKind::Quantity { syntax, .. } = &mut rows(&mut p)[1].recipe.codec.codec else {
        panic!()
    };
    *syntax = DecimalSyntax::Decimal;
    assert!(normalize(&source, &f, &p, Default::default()).is_err());
}

#[test]
fn a_zero_constructor_is_a_known_value_and_does_not_erase_skipped_input_evidence() {
    let f = fixture();
    let mut p = projection_policy(&f);
    for row in rows(&mut p) {
        row.constructor_default =
            ParameterValue::Quantity(FiniteQuantity::new(0., f.unit.clone()).unwrap());
    }
    let p = accounted(&f, p);
    for input in [None, Some("0")] {
        let source = source(&xml(&body(input, None)), 0x9d);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(values(&result, 0), expected(&f, 0., 0.));
        if input.is_some() {
            let row = accounting_tests::leaf(&source, &result, NAME, "Input");
            assert_eq!(row.disposition, SourceDisposition::Contributes);
            assert_eq!(
                row.links,
                vec![OwnedOriginTarget::ScenarioPreset(
                    result.draft().input().scenario_presets.members[0].id
                )]
            );
        }
    }
}

#[test]
fn projection_and_source_order_do_not_choose_the_control_semantics() {
    let f = fixture();
    for reversed_policy in [false, true] {
        let mut p = projection_policy(&f);
        if reversed_policy {
            rows(&mut p).reverse();
        }
        let p = accounted(&f, p);
        for text in [
            body(Some("0"), Some("80")),
            format!("{}{}", body(None, Some("80")), body(Some("0"), None)),
        ] {
            let source = source(&xml(&text), 0x9e);
            let result = normalize(&source, &f, &p, Default::default()).unwrap();
            let mut actual = values(&result, 0);
            actual.sort_by(|a, b| a.0.cmp(&b.0));
            assert_eq!(actual, expected(&f, 0., 80.));
            for kind in ["Input", "Placeholder"] {
                let row = accounting_tests::leaf(&source, &result, NAME, kind);
                assert_eq!(row.links.len(), 1);
                assert_eq!(row.disposition, SourceDisposition::Contributes);
            }
        }
    }
}

#[test]
fn unsupported_frames_and_tight_resource_limits_never_authorize_projection_defaults() {
    let f = fixture();
    let p = accounted(&f, projection_policy(&f));
    for document in [
        xml(&body(Some("0"), Some("80"))).replace("activeConfigSet=\"1\"", "activeConfigSet=\"3\""),
        format!(
            r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2"><Input name="other" number="4"/><Input name="other" number="5"/></ConfigSet></Config></PathOfBuilding2>"#,
            body(Some("0"), Some("80"))
        ),
    ] {
        let source = source(&document, 0x9f);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        assert!(
            result
                .draft()
                .input()
                .scenario_presets
                .members
                .iter()
                .all(|s| s.scenario.assumptions.members.is_empty())
        );
        for kind in ["Input", "Placeholder"] {
            accounting_tests::pending(accounting_tests::leaf(&source, &result, NAME, kind));
        }
    }
    let source = source(&xml(&body(Some("0"), Some("80"))), 0x9f);
    for kind in 0..3 {
        let mut limits = NormalizationLimits::default();
        match kind {
            0 => limits.value.max_selectors = 3,
            1 => limits.value.max_total_selector_bytes = NAME.len() * 3,
            2 => limits.max_work = 1,
            _ => unreachable!(),
        }
        assert!(normalize(&source, &f, &p, limits).is_err());
    }
}

#[test]
fn integral_projection_uses_the_same_selection_without_coercing_fractional_source_values() {
    let mut f = fixture();
    let mut schema = f.a.schema.input().clone();
    for definition in &mut schema.definitions {
        if let DefinitionDescriptor::ExternalInput(entry) = definition
            && f.inputs.iter().any(|(_, id)| *id == entry.id)
        {
            let SchemaState::Known(value) = &mut entry.schema else {
                panic!()
            };
            value.value = ValueSchema::Integer(IntegerRange {
                minimum: BoundedInteger::new(-100).unwrap(),
                maximum: BoundedInteger::new(100).unwrap(),
            });
        }
    }
    rebind(&mut f.a, schema);
    let mut p = projection_policy(&f);
    for row in rows(&mut p) {
        row.recipe.codec.codec = ValueCodecKind::Integer {
            syntax: DecimalSyntax::Scientific,
        };
        row.constructor_default = ParameterValue::Integer(BoundedInteger::new(20).unwrap());
    }
    let p = accounted(&f, p);
    let source = source(&xml(&body(Some("0"), Some("8e1"))), 0xa0);
    let result = normalize(&source, &f, &p, Default::default()).unwrap();
    assert_eq!(
        values(&result, 0),
        f.inputs
            .iter()
            .zip([0, 80])
            .map(|((_, id), value)| (
                id.clone(),
                ParameterValue::Integer(BoundedInteger::new(value).unwrap())
            ))
            .collect::<Vec<_>>()
    );
    let fractional = super::source(&xml(&body(Some("7"), Some("8.5"))), 0xa0);
    let result = normalize(&fractional, &f, &p, Default::default()).unwrap();
    assert!(values(&result, 0).is_empty());
    accounting_tests::pending(accounting_tests::leaf(&fractional, &result, NAME, "Input"));
}
