//! Injected, independently declared missing-source consumers. These fixtures
//! establish neither a production catalogue nor absence of constructor effects.
use super::*;
use poe_optimizer_import::owned_normalize::ConfigurationScenarioField;

const NAME: &str = "reviewed-missing-consumer";

fn add_missing(
    mut policy: NormalizationPolicy,
    id: &str,
    name: &str,
    lane: ValueLane,
    target: ConfigurationInventoryTarget,
) -> NormalizationPolicy {
    let inventory = policy
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap();
    inventory
        .source_census
        .controls
        .push(ConfigurationInventoryControl {
            id: key(id),
            selector: Some(ValueSelector {
                lane,
                name: name.into(),
            }),
            source_states: vec![ConfigurationSourceState::Absent],
        });
    inventory
        .dispositions
        .push(ConfigurationInventoryDisposition {
            source_control: key(id),
            target,
        });
    reseal(&mut policy);
    policy
}

fn retained(fields: &[ConfigurationScenarioField]) -> ConfigurationInventoryTarget {
    ConfigurationInventoryTarget::MissingRawRetainedScenario {
        fields: fields.to_vec(),
    }
}

fn all_fields() -> Vec<ConfigurationScenarioField> {
    vec![
        ConfigurationScenarioField::Assumptions,
        ConfigurationScenarioField::Usage,
        ConfigurationScenarioField::UsageIntents,
    ]
}

fn assert_choice_only(
    imported: &ImportedBuildInstance,
    before: &NormalizedImport,
    after: &NormalizedImport,
    completed_scopes: &[bool],
) {
    assert_eq!(completed(after), completed_scopes);
    assert!(completed(before).iter().all(|value| !value));
    let mut expected = before.draft().input().clone();
    let mut origins = before.sidecar().origins.clone();
    let mut retired = Vec::new();
    for (preset, complete) in expected
        .choice_presets
        .members
        .iter_mut()
        .zip(completed_scopes)
    {
        if !complete {
            continue;
        }
        let DraftListCompletion::Pending { id, code } = &preset.choices.completion else {
            panic!()
        };
        assert_eq!(code.as_str(), "configuration-roles-not-converted");
        assert!(preset.choices.members.is_empty());
        retired.push(*id);
        preset.choices.completion = DraftListCompletion::Complete;
    }
    if !retired.is_empty() {
        for issue in retired {
            let mut found_scope = false;
            for origin in &mut origins {
                if origin.links.contains(&OwnedOriginTarget::Issue(issue)) {
                    let row = imported.occurrence(origin.source).unwrap();
                    assert!(matches!(row.name(), "Config" | "ConfigSet"));
                    found_scope |= row.name() == "ConfigSet";
                    assert_eq!(
                        origin
                            .links
                            .iter()
                            .filter(|link| **link == OwnedOriginTarget::Issue(issue))
                            .count(),
                        1
                    );
                    origin
                        .links
                        .retain(|link| *link != OwnedOriginTarget::Issue(issue));
                }
            }
            assert!(found_scope);
        }
        let DraftListCompletion::Pending { id, code } = &expected.saved_variants.completion else {
            panic!()
        };
        assert_eq!(code.as_str(), "saved-selection-not-converted");
        let config = imported
            .occurrences()
            .iter()
            .find(|row| row.name() == "Config")
            .unwrap()
            .id();
        if !origins[config.ordinal() as usize]
            .links
            .contains(&OwnedOriginTarget::Issue(*id))
        {
            origins[config.ordinal() as usize]
                .links
                .push(OwnedOriginTarget::Issue(*id));
        }
    }
    assert_eq!(after.draft().input(), &expected);
    assert_eq!(after.sidecar().origins, origins);
    assert_eq!(after.allocator_after(), before.allocator_after());
    assert_eq!(
        after.draft().input().scenario_presets,
        before.draft().input().scenario_presets
    );
    let mut expected_sidecar = serde_json::to_value(before.sidecar()).unwrap();
    expected_sidecar["policy"] = serde_json::to_value(after.sidecar().policy).unwrap();
    expected_sidecar["draft"] = serde_json::to_value(after.sidecar().draft).unwrap();
    expected_sidecar["origins"] = serde_json::to_value(origins).unwrap();
    assert_eq!(
        serde_json::to_value(after.sidecar()).unwrap(),
        expected_sidecar
    );
    for preset in &after.draft().input().scenario_presets.members {
        for (completion, expected_code) in [
            (
                &preset.scenario.assumptions.completion,
                "external-assumptions-not-converted",
            ),
            (&preset.scenario.usage.completion, "usage-not-converted"),
            (
                &preset.scenario.usage_intents.completion,
                "scenario-usage-intents-not-converted",
            ),
        ] {
            assert!(
                matches!(completion, DraftListCompletion::Pending { code, .. } if code.as_str() == expected_code)
            );
        }
    }
}

#[test]
fn missing_branches_preserve_materialized_scenario_members_and_only_close_choice() {
    let f = fixture();
    let targets = [
        ConfigurationInventoryTarget::MissingRawNoAdditionalInput {},
        retained(&[ConfigurationScenarioField::Assumptions]),
        retained(&[ConfigurationScenarioField::Usage]),
        retained(&[ConfigurationScenarioField::UsageIntents]),
        retained(&[
            ConfigurationScenarioField::Assumptions,
            ConfigurationScenarioField::Usage,
        ]),
        retained(&all_fields()),
    ];
    let imported = source(&xml(r#"<Input name="raw-0" number="3"/>"#), 0xc1);
    for target in targets {
        let p = add_missing(
            bounded(&f),
            "missing-branch",
            NAME,
            ValueLane::InputNumber,
            target,
        );
        let before = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert!(
            !values(&before, 0).is_empty(),
            "known assumptions must coexist with unresolved membership"
        );
        assert_choice_only(&imported, &before, &actual, &[true]);
        assert_eq!(values(&actual, 0), values(&before, 0));
        assert_eq!(
            actual.sidecar().origins.len(),
            imported.occurrences().len(),
            "no fabricated leaf for missing source"
        );
    }
}

#[test]
fn any_named_input_or_placeholder_blocks_missing_even_when_an_adapter_consumes_zero() {
    let f = fixture();
    // raw-0 already has a real numeric adapter. Its authored zero is otherwise
    // fully accounted, so the refusal cannot be explained by an unknown leaf.
    let absent = source(&xml(""), 0xc2);
    let zero = source(&xml(r#"<Input name="raw-0" number="0"/>"#), 0xc2);
    assert_eq!(
        completed(&normalize(&zero, &f, &bounded(&f), Default::default()).unwrap()),
        [true]
    );
    for lane in [
        ValueLane::InputNumber,
        ValueLane::InputBoolean,
        ValueLane::InputString,
        ValueLane::PlaceholderNumber,
        ValueLane::PlaceholderString,
    ] {
        for target in [
            ConfigurationInventoryTarget::MissingRawNoAdditionalInput {},
            retained(&all_fields()),
        ] {
            let p = add_missing(
                bounded(&f),
                "raw-zero-missing-branch",
                "raw-0",
                lane,
                target,
            );
            assert_eq!(
                completed(&normalize(&absent, &f, &p, Default::default()).unwrap()),
                [true]
            );
            for child in [
                r#"<Input name="raw-0" number="0"/>"#,
                r#"<Input name="raw-0" boolean="false"/>"#,
                r#"<Input name="raw-0" string=""/>"#,
                r#"<Placeholder name="raw-0" number="0"/>"#,
                r#"<Placeholder name="raw-0" boolean="false"/>"#,
                r#"<Placeholder name="raw-0" string=""/>"#,
                r#"<Input name="raw-0"/>"#,
                r#"<Input name="raw-0" number="NaN"/>"#,
                r#"<Input name="raw-0" number="0"/><Placeholder name="raw-0" string=""/>"#,
            ] {
                let imported = source(&xml(child), 0xc2);
                let before =
                    normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
                let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
                assert_eq!(completed(&actual), [false], "{child}");
                assert_unchanged(&before, &actual);
            }
        }
    }
}

#[test]
fn distinct_missing_consumers_share_a_name_and_field_without_collapsing_the_census() {
    let f = fixture();
    let mut p = bounded(&f);
    for (id, target) in [
        (
            "first-reader",
            retained(&[ConfigurationScenarioField::Usage]),
        ),
        (
            "second-reader",
            retained(&[ConfigurationScenarioField::Usage]),
        ),
        (
            "third-reader",
            ConfigurationInventoryTarget::MissingRawNoAdditionalInput {},
        ),
        ("fourth-reader", retained(&all_fields())),
    ] {
        p = add_missing(p, id, NAME, ValueLane::InputString, target);
    }
    let imported = source(&xml(""), 0xc3);
    let expected = normalize(&imported, &f, &p, Default::default()).unwrap();
    let before = normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
    assert_choice_only(&imported, &before, &expected, &[true]);
    let mut reversed = p.clone();
    let inventory = reversed
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap();
    inventory.source_census.controls.reverse();
    inventory.dispositions.reverse();
    reseal(&mut reversed);
    let actual = normalize(&imported, &f, &reversed, Default::default()).unwrap();
    assert_eq!(actual.draft(), expected.draft());
    assert_eq!(actual.sidecar().origins, expected.sidecar().origins);
    assert_eq!(actual.allocator_after(), expected.allocator_after());
    let mut missing = p.clone();
    missing
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap()
        .dispositions
        .pop();
    assert!(normalize(&imported, &f, &missing, Default::default()).is_err());
}

#[test]
fn partial_census_unknown_leaves_and_outside_responsibilities_still_block_completion() {
    let f = fixture();
    let base = add_missing(
        bounded(&f),
        "retained-scenario",
        NAME,
        ValueLane::InputBoolean,
        retained(&all_fields()),
    );
    assert_eq!(
        completed(&normalize(&source(&xml(""), 0xc4), &f, &base, Default::default()).unwrap()),
        [true]
    );
    let mut partial = base.clone();
    partial
        .configuration_dispositions
        .as_mut()
        .unwrap()
        .inventory
        .as_mut()
        .unwrap()
        .source_census
        .closure = ConfigurationCensusClosure::Partial {
        gaps: vec![key("independent-unreviewed-reader")],
    };
    reseal(&mut partial);
    let imported = source(&xml(""), 0xc4);
    let before = normalize(
        &imported,
        &f,
        &without_inventory(&partial),
        Default::default(),
    )
    .unwrap();
    let actual = normalize(&imported, &f, &partial, Default::default()).unwrap();
    assert_unchanged(&before, &actual);
    for text in [
        xml(r#"<Input name="unreviewed" string=""/>"#),
        xml(r#"<Placeholder name="unreviewed" number="0"/>"#),
        xml(r#"<CustomModifierBlock enabled="false">unreviewed</CustomModifierBlock>"#),
        xml("").replace("<Config ", "<Future/><Config "),
        xml("").replace(
            "<Config ",
            "<Calcs><Input name='skill_number' number='1'/></Calcs><Config ",
        ),
        xml("").replace("<ConfigSet ", "<ConfigSet future='true' "),
        xml("").replace(
            "</Config>",
            "<ConfigSet id='2'><Future/></ConfigSet></Config>",
        ),
    ] {
        let imported = source(&text, 0xc4);
        let before =
            normalize(&imported, &f, &without_inventory(&base), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &base, Default::default()).unwrap();
        assert!(completed(&actual).iter().all(|value| !value), "{text}");
        assert_unchanged(&before, &actual);
    }
}

#[test]
fn missing_control_shapes_and_retained_field_sets_are_checked_before_use() {
    let f = fixture();
    let base = add_missing(
        bounded(&f),
        "retained-scenario",
        NAME,
        ValueLane::InputNumber,
        retained(&all_fields()),
    );
    let imported = source(&xml(""), 0xc5);
    assert_eq!(
        completed(&normalize(&imported, &f, &base, Default::default()).unwrap()),
        [true]
    );
    for mode in 0..12 {
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
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .selector = None
            }
            1 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .source_states = vec![ConfigurationSourceState::Authored]
            }
            2 => inventory
                .source_census
                .controls
                .last_mut()
                .unwrap()
                .source_states
                .push(ConfigurationSourceState::Authored),
            3 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .source_states = vec![ConfigurationSourceState::ConstructorDefault]
            }
            4 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .selector
                    .as_mut()
                    .unwrap()
                    .lane = ValueLane::Attribute
            }
            5 => inventory
                .source_census
                .controls
                .last_mut()
                .unwrap()
                .selector
                .as_mut()
                .unwrap()
                .name
                .clear(),
            6 => inventory.dispositions.last_mut().unwrap().target = retained(&[]),
            7 => {
                inventory.dispositions.last_mut().unwrap().target = retained(&[
                    ConfigurationScenarioField::Assumptions,
                    ConfigurationScenarioField::Assumptions,
                ])
            }
            8 => inventory
                .source_census
                .controls
                .push(inventory.source_census.controls.last().unwrap().clone()),
            9 => inventory
                .dispositions
                .push(inventory.dispositions.last().unwrap().clone()),
            10 => {
                inventory.source_census.controls.last_mut().unwrap().id =
                    key("unsealed-changed-consumer")
            }
            11 => {
                inventory.dispositions.last_mut().unwrap().source_control = key("unknown-consumer")
            }
            _ => unreachable!(),
        }
        if mode != 10 {
            reseal(&mut p);
        }
        assert!(
            normalize(&imported, &f, &p, Default::default()).is_err(),
            "mode {mode}"
        );
    }
    for wire in [
        serde_json::json!({"kind":"missing_raw_retained_scenario","fields":["unknown"]}),
        serde_json::json!({"kind":"missing_raw_retained_scenario","fields":null}),
        serde_json::json!({"kind":"missing_raw_no_additional_input","fields":["usage"]}),
    ] {
        assert!(serde_json::from_value::<ConfigurationInventoryTarget>(wire).is_err());
    }
}

fn two_sets(second: &str, selected: &str, reversed: bool) -> String {
    let first = r#"<ConfigSet id="1"><Input name="raw-1" number="3"/></ConfigSet>"#;
    let second = format!(r#"<ConfigSet id="2">{second}</ConfigSet>"#);
    let sets = if reversed {
        format!("{second}{first}")
    } else {
        format!("{first}{second}")
    };
    format!(
        r#"<PathOfBuilding2><Config activeConfigSet="{selected}">{sets}</Config></PathOfBuilding2>"#
    )
}

#[test]
fn missing_branch_proofs_are_per_configuration_even_for_same_name_and_unselected_sets() {
    let (f, p) = custom_modifiers::scoped_fixture();
    let p = add_missing(
        p,
        "retained-source",
        "raw-0",
        ValueLane::InputString,
        retained(&all_fields()),
    );
    for selected in ["1", "2"] {
        for reversed in [false, true] {
            for (second, complete) in [("", true), (r#"<Input name="raw-0" number="0"/>"#, false)] {
                let imported = source(&two_sets(second, selected, reversed), 0xc6);
                let before =
                    normalize(&imported, &f, &without_inventory(&p), Default::default()).unwrap();
                let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
                assert_choice_only(
                    &imported,
                    &before,
                    &actual,
                    &if reversed {
                        [complete, true]
                    } else {
                        [true, complete]
                    },
                );
            }
        }
    }
}

fn minimum_budget(
    imported: &ImportedBuildInstance,
    f: &Fixture,
    p: &NormalizationPolicy,
    links: bool,
) -> usize {
    let mut low = 1;
    let mut high = if links {
        NormalizationLimits::default().max_origin_links
    } else {
        NormalizationLimits::default().max_work
    };
    while low < high {
        let middle = low + (high - low) / 2;
        let mut limits = NormalizationLimits::default();
        if links {
            limits.max_origin_links = middle;
        } else {
            limits.max_work = middle;
        }
        match normalize(imported, f, p, limits) {
            Ok(_) => high = middle,
            Err(NormalizationError::Limit(_)) => low = middle + 1,
            Err(error) => panic!("unexpected budget error: {error:?}"),
        }
    }
    low
}

#[test]
fn missing_admission_work_and_link_limits_are_atomic_and_replay_preserves_inputs() {
    let (f, base) = custom_modifiers::scoped_fixture();
    let p = add_missing(
        base.clone(),
        "retained-source",
        "raw-0",
        ValueLane::InputNumber,
        retained(&all_fields()),
    );
    let imported = source(&two_sets("", "2", false), 0xc7);
    let expected = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&expected), [true, true]);
    let input_allocator = *imported.allocator_state();
    let input_policy = serde_json::to_value(&p).unwrap();
    for links in [false, true] {
        let exact_budget = minimum_budget(&imported, &f, &p, links);
        for remaining in [exact_budget - 1, exact_budget] {
            let mut limits = NormalizationLimits::default();
            if links {
                limits.max_origin_links = remaining;
            } else {
                limits.max_work = remaining;
            }
            let result = normalize(&imported, &f, &p, limits);
            if remaining < exact_budget {
                assert!(matches!(result, Err(NormalizationError::Limit(_))));
            } else {
                let exact = result.unwrap();
                assert_eq!(exact.draft(), expected.draft());
                assert_eq!(
                    serde_json::to_value(exact.sidecar()).unwrap(),
                    serde_json::to_value(expected.sidecar()).unwrap()
                );
            }
            assert_eq!(*imported.allocator_state(), input_allocator);
            assert_eq!(serde_json::to_value(&p).unwrap(), input_policy);
        }
    }
    // Removing the inventory removes the whole new authority. Private work
    // counters separately test that the disabled path performs no new scan.
    let disabled = without_inventory(&p);
    let old_disabled = without_inventory(&base);
    assert_eq!(disabled, old_disabled);
    assert_eq!(
        completed(&normalize(&imported, &f, &disabled, Default::default()).unwrap()),
        [false, false]
    );
    let refused = source(
        &two_sets(r#"<Input name="raw-0" number="0"/>"#, "2", false),
        0xc7,
    );
    assert_eq!(
        completed(&normalize(&refused, &f, &p, Default::default()).unwrap()),
        [true, false]
    );
    let replay = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(replay.draft(), expected.draft());
    assert_eq!(
        serde_json::to_value(replay.sidecar()).unwrap(),
        serde_json::to_value(expected.sidecar()).unwrap()
    );
}
