//! Actual normalizer controls; the reviewed fixture census is authored from its
//! three configured source consumers, never inferred from emitted output counts.
use super::*;
#[path = "owned_configuration_custom_modifiers.rs"]
mod custom_modifiers;
#[path = "owned_configuration_inventory_real.rs"]
mod real;

fn input_selector(index: usize) -> ValueSelector {
    ValueSelector {
        lane: ValueLane::InputNumber,
        name: format!("raw-{index}"),
    }
}

fn reseal(p: &mut NormalizationPolicy) {
    let inventory = p
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap();
    inventory.source_census_identity =
        configuration_inventory_census_identity(&inventory.source_census, Default::default())
            .unwrap();
}

fn bounded(f: &Fixture) -> NormalizationPolicy {
    let rules = (0..2)
        .map(|index| ConfigurationDispositionRule {
            id: key(&format!("saved-{index}")),
            selector: input_selector(index),
            destinations: vec![ConfigurationDestination::Adapter {
                control: ConfigurationAdapterControl::ScalarInput {
                    input: f.inputs[index].1.clone(),
                },
            }],
        })
        .collect();
    let mut p = accounting_tests::bind(f, reviewed(f), rules);
    let owner = p.configuration_dispositions.as_mut().unwrap();
    let mut controls: Vec<_> = (0..2)
        .map(|index| ConfigurationInventoryControl {
            id: key(&format!("source-{index}")),
            selector: Some(input_selector(index)),
            source_states: vec![
                ConfigurationSourceState::Authored,
                ConfigurationSourceState::Absent,
            ],
        })
        .collect();
    controls.push(ConfigurationInventoryControl {
        id: key("fresh-encounter"),
        selector: None,
        source_states: vec![ConfigurationSourceState::Absent],
    });
    let census = ConfigurationSourceCensus {
        mapping_source: owner.mapping_source,
        catalogue_evidence: owner.catalogue_evidence,
        constructor_evidence: owner.constructor_evidence,
        closure: ConfigurationCensusClosure::Complete,
        controls,
    };
    let mut dispositions: Vec<_> = (0..2)
        .map(|index| ConfigurationInventoryDisposition {
            source_control: key(&format!("source-{index}")),
            adapters: vec![ConfigurationAdapterControl::ScalarInput {
                input: f.inputs[index].1.clone(),
            }],
        })
        .collect();
    dispositions.push(ConfigurationInventoryDisposition {
        source_control: key("fresh-encounter"),
        adapters: vec![ConfigurationAdapterControl::Encounter],
    });
    owner.inventory = Some(ConfigurationInventoryPolicy {
        source_census_identity: configuration_inventory_census_identity(
            &census,
            Default::default(),
        )
        .unwrap(),
        source_census: census,
        dispositions,
    });
    p
}

fn without_inventory(p: &NormalizationPolicy) -> NormalizationPolicy {
    let mut p = p.clone();
    p.configuration_dispositions.as_mut().unwrap().inventory = None;
    p
}

fn completed(n: &NormalizedImport) -> Vec<bool> {
    n.draft()
        .input()
        .choice_presets
        .members
        .iter()
        .map(|p| matches!(p.choices.completion, DraftListCompletion::Complete))
        .collect()
}

fn assert_unchanged(before: &NormalizedImport, after: &NormalizedImport) {
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.sidecar().origins, after.sidecar().origins);
    assert_eq!(before.allocator_after(), after.allocator_after());
    let mut a = serde_json::to_value(before.sidecar()).unwrap();
    let mut b = serde_json::to_value(after.sidecar()).unwrap();
    assert_ne!(a["policy"], b["policy"]);
    a.as_object_mut().unwrap().remove("policy");
    b.as_object_mut().unwrap().remove("policy");
    assert_eq!(a, b);
}

#[test]
fn actual_adapters_close_only_checked_choices_and_preserve_saved_selection_responsibility() {
    let f = fixture();
    let p = bounded(&f);
    for body in [
        "",
        r#"<Input name="raw-0" number="0"/>"#,
        r#"<Input name="raw-1" number="-4.5"/><Input name="raw-0" number="3"/>"#,
    ] {
        let source = source(&xml(body), 0x91);
        let before = normalize(&source, &f, &without_inventory(&p), Default::default()).unwrap();
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&before), [false]);
        assert_eq!(completed(&after), [true]);
        let mut expected = before.draft().input().clone();
        let DraftListCompletion::Pending { id: retired, .. } = expected.choice_presets.members[0]
            .choices
            .completion
            .clone()
        else {
            panic!()
        };
        expected.choice_presets.members[0].choices.completion = DraftListCompletion::Complete;
        assert_eq!(after.draft().input(), &expected);
        let DraftListCompletion::Pending { id: selection, .. } = expected.saved_variants.completion
        else {
            panic!()
        };
        let config = source
            .occurrences()
            .iter()
            .find(|r| r.name() == "Config")
            .unwrap()
            .id();
        let mut origins = before.sidecar().origins.clone();
        assert_eq!(
            origins
                .iter()
                .filter(|o| o.links.contains(&OwnedOriginTarget::Issue(retired)))
                .count(),
            2
        );
        for row in &mut origins {
            row.links
                .retain(|v| *v != OwnedOriginTarget::Issue(retired));
        }
        origins[config.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::Issue(selection));
        assert_eq!(origins, after.sidecar().origins);
        assert_eq!(before.allocator_after(), after.allocator_after());
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .usage
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .usage_intents
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let repeated = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(
            serde_json::to_value(after.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
    }
}

#[test]
fn missing_invalid_unknown_and_foreign_source_authority_remain_pending() {
    let f = fixture();
    let p = bounded(&f);
    for document in [
        xml(r#"<Input name="raw-0" number="NaN"/>"#),
        xml(r#"<Input name="raw-0" string="0"/>"#),
        xml(r#"<Input name="raw-0" number="3"/><Input name="unknown" boolean="true"/>"#),
        xml(r#"<CustomModifierBlock enabled="false">unreviewed</CustomModifierBlock>"#),
        xml(r#"<Placeholder name="raw-0" number="3"/>"#),
        xml(r#"<Input name="raw-0" number="0" future="x"/>"#),
        xml(r#"<Input name="encounter-selector" string="other"/>"#),
        xml("").replace("<Config ", "<Future/><Config "),
        xml("").replace("activeConfigSet=\"1\"", "activeConfigSet=\"2\""),
    ] {
        let source = source(&document, 0x92);
        let before = normalize(&source, &f, &without_inventory(&p), Default::default()).unwrap();
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert!(completed(&after).iter().all(|c| !*c), "{document}");
        assert_unchanged(&before, &after);
    }
}

#[test]
fn independent_census_is_not_reconstructed_from_successful_receipts() {
    let f = fixture();
    let base = bounded(&f);
    let source = source(&xml(""), 0x93);
    for mode in 0..8 {
        let mut p = base.clone();
        let inventory = p
            .configuration_dispositions
            .as_mut()
            .unwrap()
            .inventory
            .as_mut()
            .unwrap();
        match mode {
            0 => {
                inventory.source_census.closure = ConfigurationCensusClosure::Partial {
                    gaps: vec![key("unreviewed-default")],
                }
            }
            1 => {
                inventory.source_census.closure = ConfigurationCensusClosure::Partial {
                    gaps: vec![key("catalogue-not-reviewed")],
                };
                inventory.source_census.controls.clear();
                inventory.dispositions.clear();
            }
            2 => {
                inventory.source_census.controls[0].source_states =
                    vec![ConfigurationSourceState::ConstructorDefault]
            }
            3 => {
                inventory.source_census.controls.pop();
                inventory.dispositions.pop();
            }
            4 => inventory.dispositions.pop().map(|_| ()).unwrap(),
            5 => {
                inventory.source_census.controls.clear();
                inventory.dispositions.clear();
            }
            6 => {
                inventory.source_census.controls[0].id = key("changed-unsealed-census");
            }
            7 => inventory.source_census.mapping_source = *f.a.rewards.identity(),
            _ => unreachable!(),
        }
        if mode != 6 {
            reseal(&mut p);
        }
        let result = normalize(&source, &f, &p, Default::default());
        if mode >= 4 {
            assert!(result.is_err(), "mode {mode}");
        } else {
            let after = result.unwrap();
            assert_eq!(completed(&after), [false]);
            let before =
                normalize(&source, &f, &without_inventory(&p), Default::default()).unwrap();
            assert_unchanged(&before, &after);
        }
    }
}

#[test]
fn consumer_destinations_are_conjunctive_and_duplicate_catalogue_names_are_not_collapsed() {
    let f = fixture();
    let base = bounded(&f);
    let source = source(&xml(r#"<Input name="raw-0" number="3"/>"#), 0x94);
    let mut p = base.clone();
    p.configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap()
        .dispositions[0]
        .adapters
        .push(ConfigurationAdapterControl::ScalarInput {
            input: f.inputs[1].1.clone(),
        });
    let after = normalize(&source, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&after), [false]);
    let before = normalize(&source, &f, &without_inventory(&p), Default::default()).unwrap();
    assert_unchanged(&before, &after);
    let mut p = base.clone();
    let inventory = p
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap();
    let mut duplicate = inventory.source_census.controls[0].clone();
    duplicate.id = key("separate-callback-same-name");
    inventory.source_census.controls.push(duplicate);
    let mut duplicate = inventory.dispositions[0].clone();
    duplicate.source_control = key("separate-callback-same-name");
    inventory.dispositions.push(duplicate);
    reseal(&mut p);
    assert!(normalize(&source, &f, &p, Default::default()).is_err());
}

#[test]
fn constructor_default_requires_its_actual_value_receipt_not_a_true_absence_receipt() {
    let f = fixture();
    let mut p = bounded(&f);
    let Some(ConfigurationInputsPolicy::PobFreshNumericConfigOverridesV1 { inputs, .. }) =
        p.configuration_inputs.take()
    else {
        panic!()
    };
    let defaults = inputs
        .iter()
        .enumerate()
        .map(|(index, input)| ConfigurationDefaultInput {
            source_name: input.source_name.clone(),
            value_input: input.value_input.clone(),
            target: ConfigurationInputTarget::Enemy,
            recipe: input.recipe.clone(),
            constructor_default: ParameterValue::Quantity(
                FiniteQuantity::new(7.0 + index as f64, f.unit.clone()).unwrap(),
            ),
            selection: ConfigurationValueSelection::InputOrConstructor,
            ignore_numeric_placeholder: false,
        })
        .collect();
    p.configuration_inputs = Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 {
        mapping_source: *f.a.mapping.source_identity(),
        encounter: f.encounter.clone(),
        inputs: vec![],
        placeholder_fallback_inputs: vec![],
        option_inputs: vec![],
        default_inputs: defaults,
    });
    p.configuration_dispositions.as_mut().unwrap().dependencies =
        configuration_disposition_dependencies(&p, &f.a.rewards, Default::default()).unwrap();
    let source = source(&xml(""), 0x95);
    assert_eq!(
        completed(&normalize(&source, &f, &p, Default::default()).unwrap()),
        [false]
    );
    for row in &mut p
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap()
        .source_census
        .controls[..2]
    {
        row.source_states = vec![
            ConfigurationSourceState::Authored,
            ConfigurationSourceState::ConstructorDefault,
        ];
    }
    reseal(&mut p);
    let after = normalize(&source, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&after), [true]);
    assert_eq!(
        values(&after, 0)
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<_>>(),
        vec![
            ParameterValue::Quantity(FiniteQuantity::new(7.0, f.unit.clone()).unwrap()),
            ParameterValue::Quantity(FiniteQuantity::new(8.0, f.unit.clone()).unwrap())
        ]
    );
    let mut incomplete = p.clone();
    let Some(ConfigurationInputsPolicy::PobFreshConfigInputsV3 { default_inputs, .. }) =
        &mut incomplete.configuration_inputs
    else {
        panic!()
    };
    default_inputs.pop();
    incomplete
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .dependencies =
        configuration_disposition_dependencies(&incomplete, &f.a.rewards, Default::default())
            .unwrap();
    assert_eq!(
        completed(&normalize(&source, &f, &incomplete, Default::default()).unwrap()),
        [false]
    );
}

#[test]
fn independent_sets_do_not_borrow_each_others_proof_and_late_budget_failure_returns_no_result() {
    let f = fixture();
    let p = bounded(&f);
    let text = r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><Input name="raw-0" number="3"/></ConfigSet><ConfigSet id="2"><Input name="raw-0" number="NaN"/></ConfigSet></Config></PathOfBuilding2>"#;
    let imported = source(text, 0x96);
    // Historical foreign fallback links also remain blocking. A local proof is
    // never permission to erase another ConfigSet's outstanding responsibility.
    let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&actual), [false, false]);
    let baseline = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
    assert_unchanged(&baseline, &actual);
    let single = source(&xml(r#"<Input name="raw-0" number="3"/>"#), 0x97);
    let expected = normalize(&single, &f, &p, Default::default()).unwrap();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        let limits = NormalizationLimits {
            max_work: mid,
            ..Default::default()
        };
        if normalize(&single, &f, &p, limits).is_ok() {
            high = mid;
        } else {
            low = mid + 1;
        }
    }
    let limits = NormalizationLimits {
        max_work: low - 1,
        ..Default::default()
    };
    assert!(matches!(
        normalize(&single, &f, &p, limits),
        Err(NormalizationError::Limit(_))
    ));
    let repeated = normalize(&single, &f, &p, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_value(expected.sidecar()).unwrap(),
        serde_json::to_value(repeated.sidecar()).unwrap()
    );
}
