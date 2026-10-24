//! A reviewed inapplicable branch completes only its exact source consumer.
//! These injected policies establish no production configuration census.
use super::*;

const NAME: &str = "observed-unused-value";
const RULE: &str = "reviewed-unused-observation";
const CONTROL: &str = "unused-observation-source";
const REASON: &str = "branch-does-not-consume-observation";

fn add_observation(f: &Fixture, mut p: NormalizationPolicy, name: &str) -> NormalizationPolicy {
    let mut recipe = value_recipe("reviewed-unused-number", name, false);
    recipe.tiers[0].selectors[0].lane = ValueLane::PlaceholderNumber;
    recipe.codec.codec = ValueCodecKind::Quantity {
        syntax: DecimalSyntax::Scientific,
        unit: f.unit.clone(),
        scale: RationalScale {
            numerator: BoundedInteger::new(1).unwrap(),
            denominator: BoundedInteger::new(1).unwrap(),
        },
    };
    let selector = recipe.tiers[0].selectors[0].clone();
    let owner = p.configuration_dispositions.as_mut().unwrap();
    owner.rules.push(ConfigurationDispositionRule {
        id: key(RULE),
        selector: selector.clone(),
        destinations: vec![ConfigurationDestination::InapplicableObservation {
            recipe: Box::new(recipe),
            absent_input_names: vec!["encounter-selector".into()],
            reason: key(REASON),
        }],
    });
    let inventory = owner.inventory.as_mut().unwrap();
    inventory
        .source_census
        .controls
        .push(ConfigurationInventoryControl {
            id: key(CONTROL),
            selector: Some(selector),
            source_states: vec![ConfigurationSourceState::Placeholder],
        });
    inventory
        .dispositions
        .push(ConfigurationInventoryDisposition {
            source_control: key(CONTROL),
            target: ConfigurationInventoryTarget::Inapplicable { rule: key(RULE) },
        });
    reseal(&mut p);
    p
}

fn body(name: &str, value: &str) -> String {
    format!(r#"<Input name="raw-0" number="0"/><Placeholder name="{name}" number="{value}"/>"#)
}

fn assert_observation(source: &ImportedBuildInstance, actual: &NormalizedImport, name: &str) {
    let leaf = accounting_tests::leaf(source, actual, name, "Placeholder");
    assert_eq!(leaf.disposition, SourceDisposition::SourceOnly(key(REASON)));
    assert_eq!(
        leaf.links,
        [OwnedOriginTarget::ScenarioPreset(
            actual.draft().input().scenario_presets.members[0].id
        )]
    );
}

#[test]
fn exact_observation_and_adapter_receipts_complete_only_the_choice_inventory() {
    let f = fixture();
    for (name, value) in [
        (NAME, "0"),
        ("renamed-observation", "-19.75"),
        ("another-spelling", "1e99"),
    ] {
        let p = add_observation(&f, bounded(&f), name);
        let imported = source(&xml(&body(name, value)), 0xb1);
        let before = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&before), [false]);
        assert_eq!(completed(&actual), [true], "{name}={value}");
        assert_observation(&imported, &actual, name);
        let mut expected = before.draft().input().clone();
        expected.choice_presets.members[0].choices.completion = DraftListCompletion::Complete;
        assert_eq!(
            actual.draft().input(),
            &expected,
            "inapplicability supplies no value or other completion"
        );
        assert_eq!(actual.allocator_after(), before.allocator_after());
        assert_eq!(values(&actual, 0), values(&before, 0));
        let scenario = &actual.draft().input().scenario_presets.members[0].scenario;
        assert!(matches!(
            scenario.assumptions.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            scenario.usage.completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            scenario.usage_intents.completion,
            DraftListCompletion::Pending { .. }
        ));
        let repeated = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(
            serde_json::to_value(actual.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
    }
}

#[test]
fn absent_malformed_duplicate_wrong_lane_and_foreign_branch_sources_do_not_supply_a_proof() {
    let f = fixture();
    let p = add_observation(&f, bounded(&f), NAME);
    let baseline = normalize(
        &source(&xml(&body(NAME, "0")), 0xb2),
        &f,
        &p,
        Default::default(),
    )
    .unwrap();
    assert_eq!(completed(&baseline), [true]);
    for (label, observed) in [
        ("absent", String::new()),
        (
            "nonfinite",
            format!(r#"<Placeholder name="{NAME}" number="NaN"/>"#),
        ),
        (
            "invalid spelling",
            format!(r#"<Placeholder name="{NAME}" number=" 3"/>"#),
        ),
        (
            "wrong value lane",
            format!(r#"<Placeholder name="{NAME}" string="3"/>"#),
        ),
        (
            "authored lane",
            format!(r#"<Input name="{NAME}" number="3"/>"#),
        ),
        (
            "duplicate",
            format!(
                r#"<Placeholder name="{NAME}" number="3"/><Placeholder name="{NAME}" number="4"/>"#
            ),
        ),
        (
            "authored competing source",
            format!(r#"<Placeholder name="{NAME}" number="3"/><Input name="{NAME}" number="0"/>"#),
        ),
        (
            "future attribute",
            format!(r#"<Placeholder name="{NAME}" number="3" future="x"/>"#),
        ),
        (
            "foreign encounter",
            format!(
                r#"<Placeholder name="{NAME}" number="3"/><Input name="encounter-selector" string="other"/>"#
            ),
        ),
        (
            "branch placeholder",
            format!(
                r#"<Placeholder name="{NAME}" number="3"/><Placeholder name="encounter-selector" string="None"/>"#
            ),
        ),
    ] {
        let imported = source(
            &xml(&format!(r#"<Input name="raw-0" number="0"/>{observed}"#)),
            0xb2,
        );
        let before = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&actual), [false], "{label}");
        assert_unchanged(&before, &actual);
        for occurrence in imported.occurrences().iter().filter(|row| {
            imported
                .attribute(row.id(), "name")
                .unwrap()
                .is_some_and(|a| a.decoded() == NAME)
        }) {
            accounting_tests::pending(
                &actual.sidecar().origins[occurrence.id().ordinal() as usize],
            );
        }
    }
    // A valid local proof cannot account for a different, unreviewed sibling.
    let imported = source(
        &xml(&format!(
            r#"{}<Placeholder name="unknown" number="3"/>"#,
            body(NAME, "0")
        )),
        0xb2,
    );
    let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&actual), [false]);
    assert_observation(&imported, &actual, NAME);
    accounting_tests::pending(accounting_tests::leaf(
        &imported,
        &actual,
        "unknown",
        "Placeholder",
    ));
}

#[test]
fn inventory_requires_the_exact_checked_rule_selector_state_and_unique_consumer() {
    let f = fixture();
    let base = add_observation(&f, bounded(&f), NAME);
    let imported = source(&xml(&body(NAME, "0")), 0xb3);
    assert_eq!(
        completed(&normalize(&imported, &f, &base, Default::default()).unwrap()),
        [true]
    );
    for mode in 0..10 {
        let mut p = base.clone();
        let owner = p.configuration_dispositions.as_mut().unwrap();
        let inventory = owner.inventory.as_mut().unwrap();
        match mode {
            0 => {
                inventory.dispositions.last_mut().unwrap().target =
                    ConfigurationInventoryTarget::Inapplicable {
                        rule: key("missing-rule"),
                    }
            }
            1 => {
                inventory.dispositions.last_mut().unwrap().target =
                    ConfigurationInventoryTarget::Inapplicable {
                        rule: key("saved-0"),
                    }
            }
            2 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .selector
                    .as_mut()
                    .unwrap()
                    .name = "foreign-observation".into()
            }
            3 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .selector
                    .as_mut()
                    .unwrap()
                    .lane = ValueLane::InputNumber
            }
            4 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .source_states = vec![ConfigurationSourceState::Absent]
            }
            5 => inventory
                .source_census
                .controls
                .last_mut()
                .unwrap()
                .source_states
                .push(ConfigurationSourceState::Absent),
            6 => {
                let mut control = inventory.source_census.controls.last().unwrap().clone();
                control.id = key("second-consumer");
                inventory.source_census.controls.push(control);
                inventory
                    .dispositions
                    .push(ConfigurationInventoryDisposition {
                        source_control: key("second-consumer"),
                        target: ConfigurationInventoryTarget::Inapplicable { rule: key(RULE) },
                    });
            }
            7 => {
                inventory.dispositions.pop();
            }
            8 => {
                owner.rules.last_mut().unwrap().destinations =
                    vec![ConfigurationDestination::Deferred {
                        responsibility: ConfigurationResponsibility::ScenarioUsage,
                    }]
            }
            9 => owner.rules.last_mut().unwrap().destinations.push(
                ConfigurationDestination::Deferred {
                    responsibility: ConfigurationResponsibility::ScenarioUsage,
                },
            ),
            _ => unreachable!(),
        }
        reseal(&mut p);
        let before = serde_json::to_value(&p).unwrap();
        assert!(
            matches!(
                normalize(&imported, &f, &p, Default::default()),
                Err(NormalizationError::Policy(_))
            ),
            "invalid rule binding {mode}"
        );
        assert_eq!(serde_json::to_value(&p).unwrap(), before);
    }
}

#[test]
fn partial_or_missing_census_authority_cannot_promote_a_valid_observation() {
    let f = fixture();
    let base = add_observation(&f, bounded(&f), NAME);
    let imported = source(&xml(&body(NAME, "0")), 0xb4);
    assert_eq!(
        completed(&normalize(&imported, &f, &base, Default::default()).unwrap()),
        [true]
    );
    for mode in 0..3 {
        let mut p = base.clone();
        let owner = p.configuration_dispositions.as_mut().unwrap();
        let inventory = owner.inventory.as_mut().unwrap();
        match mode {
            0 => {
                inventory.source_census.closure = ConfigurationCensusClosure::Partial {
                    gaps: vec![key("unreviewed-configuration-family")],
                }
            }
            1 => {
                // The rule still successfully accounts for its leaf, but its
                // proof is surplus to this alleged complete source inventory.
                inventory.source_census.controls.pop();
                inventory.dispositions.pop();
            }
            2 => {
                let mut extra = owner.rules.last().unwrap().clone();
                extra.id = key("second-observation-rule");
                extra.selector.name = "other-required-observation".into();
                let ConfigurationDestination::InapplicableObservation { recipe, .. } =
                    &mut extra.destinations[0]
                else {
                    panic!()
                };
                recipe.id = key("second-observation-value");
                recipe.tiers[0].selectors[0] = extra.selector.clone();
                inventory
                    .source_census
                    .controls
                    .push(ConfigurationInventoryControl {
                        id: key("other-required-source"),
                        selector: Some(extra.selector.clone()),
                        source_states: vec![ConfigurationSourceState::Placeholder],
                    });
                inventory
                    .dispositions
                    .push(ConfigurationInventoryDisposition {
                        source_control: key("other-required-source"),
                        target: ConfigurationInventoryTarget::Inapplicable {
                            rule: extra.id.clone(),
                        },
                    });
                owner.rules.push(extra);
            }
            _ => unreachable!(),
        }
        reseal(&mut p);
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&actual), [false], "incomplete authority {mode}");
        assert_observation(&imported, &actual, NAME);
        let before = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
        assert_unchanged(&before, &actual);
    }
}

#[test]
fn same_named_observations_remain_bound_to_each_independent_configuration() {
    let (f, p) = custom_modifiers::scoped_fixture();
    let p = add_observation(&f, p, NAME);
    for selected in ["1", "2"] {
        for swapped in [false, true] {
            for (label, second, second_complete) in [
                ("independent value", body(NAME, "-9.5"), true),
                ("missing", String::new(), false),
                ("malformed value", body(NAME, "NaN"), false),
                (
                    "foreign encounter",
                    format!(
                        r#"{}<Input name="encounter-selector" string="other"/>"#,
                        body(NAME, "3")
                    ),
                    false,
                ),
            ] {
                let first = format!(r#"<ConfigSet id="1">{}</ConfigSet>"#, body(NAME, "0"));
                let second = format!(r#"<ConfigSet id="2">{second}</ConfigSet>"#);
                let sets = if swapped {
                    format!("{second}{first}")
                } else {
                    format!("{first}{second}")
                };
                let imported = source(
                    &format!(
                        r#"<PathOfBuilding2><Config activeConfigSet="{selected}">{sets}</Config></PathOfBuilding2>"#
                    ),
                    0xb5,
                );
                let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
                assert_eq!(
                    completed(&actual),
                    if swapped {
                        [second_complete, true]
                    } else {
                        [true, second_complete]
                    },
                    "{label}, selected={selected}, swapped={swapped}"
                );
                assert!(actual.draft().input().rewards.members.is_empty());
                for occurrence in imported.occurrences().iter().filter(|row| {
                    row.name() == "Placeholder"
                        && imported
                            .attribute(row.id(), "name")
                            .unwrap()
                            .is_some_and(|a| a.decoded() == NAME)
                }) {
                    let first_scope = imported
                        .attribute(occurrence.parent().unwrap(), "id")
                        .unwrap()
                        .unwrap()
                        .decoded()
                        == "1";
                    let origin = &actual.sidecar().origins[occurrence.id().ordinal() as usize];
                    if first_scope || second_complete {
                        let index = usize::from(if first_scope { swapped } else { !swapped });
                        assert_eq!(
                            origin.disposition,
                            SourceDisposition::SourceOnly(key(REASON))
                        );
                        assert_eq!(
                            origin.links,
                            [OwnedOriginTarget::ScenarioPreset(
                                actual.draft().input().scenario_presets.members[index].id
                            )]
                        );
                    } else {
                        accounting_tests::pending(origin);
                    }
                }
            }
        }
    }
}

#[test]
fn late_observation_scope_work_failure_is_atomic_and_exact_budget_replays() {
    let (f, p) = custom_modifiers::scoped_fixture();
    let p = add_observation(&f, p, NAME);
    let imported = source(
        &format!(
            r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1">{}</ConfigSet><ConfigSet id="2">{}</ConfigSet></Config></PathOfBuilding2>"#,
            body(NAME, "0"),
            body(NAME, "1e99")
        ),
        0xb6,
    );
    let expected = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&expected), [true, true]);
    let before_policy = serde_json::to_value(&p).unwrap();
    let before_allocator = *imported.allocator_state();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let middle = low + (high - low) / 2;
        match normalize(
            &imported,
            &f,
            &p,
            NormalizationLimits {
                max_work: middle,
                ..Default::default()
            },
        ) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(other) => panic!("unexpected budget-search error: {other:?}"),
        }
    }
    let exact = normalize(
        &imported,
        &f,
        &p,
        NormalizationLimits {
            max_work: low,
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(exact.draft(), expected.draft());
    assert_eq!(
        serde_json::to_value(exact.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
    assert!(matches!(
        normalize(
            &imported,
            &f,
            &p,
            NormalizationLimits {
                max_work: low - 1,
                ..Default::default()
            }
        ),
        Err(NormalizationError::Limit(_))
    ));
    assert_eq!(*imported.allocator_state(), before_allocator);
    assert_eq!(serde_json::to_value(&p).unwrap(), before_policy);
    let missing = source(&xml(""), 0xb6);
    assert_eq!(
        completed(&normalize(&missing, &f, &p, Default::default()).unwrap()),
        [false]
    );
    let repeated = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(repeated.draft(), expected.draft());
    assert_eq!(
        serde_json::to_value(repeated.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
