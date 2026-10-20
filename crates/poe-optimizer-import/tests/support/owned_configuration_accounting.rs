//! Common partial accounting; every name and numeric grammar is injected.
use super::*;

pub(super) fn bind(
    f: &Fixture,
    mut p: NormalizationPolicy,
    rules: Vec<ConfigurationDispositionRule>,
) -> NormalizationPolicy {
    let dependencies =
        configuration_disposition_dependencies(&p, &f.a.rewards, Default::default()).unwrap();
    p.configuration_dispositions = Some(ConfigurationDispositionPolicy {
        inventory: None,
        mapping_source: *f.a.mapping.source_identity(),
        catalogue_evidence: *f.a.mapping.identity(),
        constructor_evidence: *f.a.rewards.identity(),
        dependencies,
        rules,
    });
    p
}

fn observation(f: &Fixture, name: &str) -> ConfigurationDispositionRule {
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
    ConfigurationDispositionRule {
        id: key("reviewed-unused-observation"),
        selector: recipe.tiers[0].selectors[0].clone(),
        destinations: vec![ConfigurationDestination::InapplicableObservation {
            recipe: Box::new(recipe),
            absent_input_names: vec!["encounter-selector".into()],
            reason: key("branch-does-not-consume-observation"),
        }],
    }
}

pub(super) fn leaf<'a>(
    source: &ImportedBuildInstance,
    result: &'a NormalizedImport,
    name: &str,
    kind: &str,
) -> &'a SourceOwnedOrigin {
    let origin = source
        .occurrences()
        .iter()
        .find(|r| {
            r.name() == kind
                && source
                    .attribute(r.id(), "name")
                    .unwrap()
                    .is_some_and(|a| a.decoded() == name)
        })
        .unwrap()
        .id();
    &result.sidecar().origins[origin.ordinal() as usize]
}

pub(super) fn pending(origin: &SourceOwnedOrigin) {
    assert_eq!(origin.disposition, SourceDisposition::Contributes);
    assert!(
        origin
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::Issue(_)))
    );
}

#[test]
fn injected_branch_observation_is_value_independent_and_changes_only_its_leaf() {
    let f = fixture();
    for (name, number) in [
        ("unused-a", "0"),
        ("unused-b", "-19.75"),
        ("unrelated-spelling", "1e99"),
    ] {
        let p = bind(&f, reviewed(&f), vec![observation(&f, name)]);
        let source = source(
            &xml(&format!(
                r#"<Placeholder name="{name}" number="{number}"/><Placeholder name="unknown" number="4"/><Input name="authored-unknown" string="x"/><CustomModifierBlock title="unconverted" enabled="true">opaque</CustomModifierBlock>"#
            )),
            0x85,
        );
        let before = normalize(&source, &f, &reviewed(&f), Default::default()).unwrap();
        let after = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(before.draft(), after.draft());
        assert_eq!(before.allocator_after(), after.allocator_after());
        let changed = leaf(&source, &after, name, "Placeholder");
        assert_eq!(
            changed.disposition,
            SourceDisposition::SourceOnly(key("branch-does-not-consume-observation"))
        );
        assert_eq!(
            changed.links,
            vec![OwnedOriginTarget::ScenarioPreset(
                after.draft().input().scenario_presets.members[0].id
            )]
        );
        let differences = before
            .sidecar()
            .origins
            .iter()
            .zip(&after.sidecar().origins)
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(differences, 1);
        pending(leaf(&source, &after, "unknown", "Placeholder"));
        pending(leaf(&source, &after, "authored-unknown", "Input"));
        assert!(matches!(
            after.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
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
        let repeated = normalize(&source, &f, &p, Default::default()).unwrap();
        assert_eq!(
            serde_json::to_vec(after.sidecar()).unwrap(),
            serde_json::to_vec(repeated.sidecar()).unwrap()
        );
    }
}

#[test]
fn observation_refuses_malformed_values_wrong_lanes_and_present_branch_writes() {
    let f = fixture();
    let p = bind(&f, reviewed(&f), vec![observation(&f, "observation")]);
    for body in [
        r#"<Placeholder name="observation" number="NaN"/>"#,
        r#"<Placeholder name="observation" number="inf"/>"#,
        r#"<Placeholder name="observation" number=" 3"/>"#,
        r#"<Placeholder name="observation" string="3"/>"#,
        r#"<Placeholder name="observation" number="3"/><Input name="observation" number="0"/>"#,
        r#"<Placeholder name="observation" number="3"/><Input name="encounter-selector" string="None"/>"#,
        r#"<Placeholder name="observation" number="3"/><Placeholder name="encounter-selector" number="0"/>"#,
        r#"<Placeholder name="observation" number="3"/><Placeholder name="encounter-selector" string="None"/>"#,
        r#"<Placeholder name="observation" number="3"/><Placeholder name="observation" number="4"/>"#,
        r#"<Placeholder name="observation" number="3" future="x"/>"#,
    ] {
        let source = source(&xml(body), 0x86);
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        pending(leaf(&source, &result, "observation", "Placeholder"));
    }
    for second in [
        r#"<ConfigSet id="1"/>"#,
        r#"<ConfigSet id="2"><Future/></ConfigSet>"#,
        r#"<ConfigSet id="2"><Placeholder name="observation" number="3"/><Input name="encounter-selector" string="None"/></ConfigSet>"#,
    ] {
        let source = source(
            &format!(
                r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><Placeholder name="observation" number="3"/></ConfigSet>{second}</Config></PathOfBuilding2>"#
            ),
            0x87,
        );
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        // Historical foreign fallback links are never removed by local proof.
        pending(leaf(&source, &result, "observation", "Placeholder"));
    }
}

#[test]
fn governing_all_destination_rule_cannot_be_bypassed_by_raw_placeholder_receipt() {
    let f = fixture();
    let source = source(&xml(r#"<Placeholder name="raw-0" number="3"/>"#), 0x88);
    for extra in [
        None,
        Some(ConfigurationDestination::Deferred {
            responsibility: ConfigurationResponsibility::ScenarioUsage,
        }),
        Some(ConfigurationDestination::Adapter {
            control: ConfigurationAdapterControl::ScalarInput {
                input: f.inputs[1].1.clone(),
            },
        }),
    ] {
        let mut destinations = vec![ConfigurationDestination::Adapter {
            control: ConfigurationAdapterControl::ScalarInput {
                input: f.inputs[0].1.clone(),
            },
        }];
        let complete = extra.is_none();
        destinations.extend(extra);
        let p = bind(
            &f,
            reviewed(&f),
            vec![ConfigurationDispositionRule {
                id: key("all-roles"),
                selector: ValueSelector {
                    lane: ValueLane::PlaceholderNumber,
                    name: "raw-0".into(),
                },
                destinations,
            }],
        );
        let result = normalize(&source, &f, &p, Default::default()).unwrap();
        let origin = leaf(&source, &result, "raw-0", "Placeholder");
        if complete {
            assert_eq!(
                origin.disposition,
                SourceDisposition::SourceOnly(key("overwritten-config-placeholder"))
            );
        } else {
            pending(origin);
        }
    }
}

#[test]
fn scalar_receipts_account_only_exact_selected_control_and_keep_partial_failures() {
    let f = fixture();
    let source = source(
        &xml(r#"<Input name="raw-0" number="0"/><Input name="raw-1" number="NaN"/>"#),
        0x89,
    );
    let rules = (0..2)
        .map(|i| ConfigurationDispositionRule {
            id: key(&format!("input-{i}")),
            selector: ValueSelector {
                lane: ValueLane::InputNumber,
                name: format!("raw-{i}"),
            },
            destinations: vec![ConfigurationDestination::Adapter {
                control: ConfigurationAdapterControl::ScalarInput {
                    input: f.inputs[i].1.clone(),
                },
            }],
        })
        .collect();
    let p = bind(&f, reviewed(&f), rules);
    let result = normalize(&source, &f, &p, Default::default()).unwrap();
    let exact = leaf(&source, &result, "raw-0", "Input");
    assert_eq!(exact.disposition, SourceDisposition::Contributes);
    assert_eq!(
        exact.links,
        vec![OwnedOriginTarget::ScenarioPreset(
            result.draft().input().scenario_presets.members[0].id
        )]
    );
    pending(leaf(&source, &result, "raw-1", "Input"));
}

#[test]
fn configuration_commitments_reject_changed_or_missing_adapters_and_actual_rewards() {
    let f = fixture();
    let source = source(
        &xml(r#"<Placeholder name="observation" number="3"/>"#),
        0x8a,
    );
    let p = bind(&f, reviewed(&f), vec![observation(&f, "observation")]);
    for mutation in 0..6 {
        let mut changed = p.clone();
        match mutation {
            0 => changed.encounter = None,
            1 => changed.configuration_inputs = None,
            2 => {
                changed
                    .configuration_dispositions
                    .as_mut()
                    .unwrap()
                    .dependencies
                    .rewards = *f.a.mapping.identity()
            }
            3 => {
                changed
                    .configuration_dispositions
                    .as_mut()
                    .unwrap()
                    .mapping_source = *f.a.mapping.identity()
            }
            4 => {
                changed
                    .configuration_dispositions
                    .as_mut()
                    .unwrap()
                    .dependencies
                    .enemy_level = Some(*f.a.mapping.identity())
            }
            5 => {
                changed
                    .configuration_dispositions
                    .as_mut()
                    .unwrap()
                    .dependencies
                    .reward_inventory = Some(*f.a.mapping.identity())
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                normalize(&source, &f, &changed, Default::default()),
                Err(NormalizationError::Binding)
            ),
            "mutation {mutation}"
        );
    }
    for mutation in 0..5 {
        let mut changed = p.clone();
        let rule = &mut changed.configuration_dispositions.as_mut().unwrap().rules[0];
        let ConfigurationDestination::InapplicableObservation {
            recipe,
            absent_input_names,
            ..
        } = &mut rule.destinations[0]
        else {
            panic!()
        };
        match mutation {
            0 => absent_input_names.push("unproved".into()),
            1 => absent_input_names.clear(),
            2 => absent_input_names.push(absent_input_names[0].clone()),
            3 => recipe.tiers[0].selectors[0].name = "other".into(),
            4 => rule.selector.lane = ValueLane::InputNumber,
            _ => unreachable!(),
        }
        assert!(
            normalize(&source, &f, &changed, Default::default()).is_err(),
            "grammar {mutation}"
        );
    }
    let limited = NormalizationLimits {
        max_work: 1,
        ..Default::default()
    };
    assert!(normalize(&source, &f, &p, limited).is_err());
    assert!(normalize(&source, &f, &p, Default::default()).is_ok());
}

#[test]
fn empty_accounting_keeps_raw_only_phase_absence_and_all_origin_content() {
    let f = fixture();
    for body in [
        "",
        r#"<Placeholder name="raw-0" number="3"/>"#,
        r#"<Input name="raw-0" number="0"/><Placeholder name="raw-0" number="3"/><Placeholder name="raw-1" number="37"/>"#,
        r#"<Placeholder name="raw-0" number="3"/><Input name="raw-1" number="NaN"/><Input name="unknown" boolean="false"/>"#,
        r#"<Placeholder name="raw-0" number="3"/><CustomModifierBlock title="unreviewed" enabled="true">opaque</CustomModifierBlock>"#,
    ] {
        let imported = source(&xml(body), 0xa1);
        let ungoverned = fallback_tests::fallback_policy(&f);
        let governed = bind(&f, ungoverned.clone(), vec![]);
        let before = normalize(&imported, &f, &ungoverned, Default::default()).unwrap();
        let after = normalize(&imported, &f, &governed, Default::default()).unwrap();
        let repeated = normalize(&imported, &f, &governed, Default::default()).unwrap();
        assert_eq!(after.draft(), before.draft(), "source {body}");
        assert_eq!(
            after.sidecar().origins,
            before.sidecar().origins,
            "source {body}"
        );
        assert_eq!(after.allocator_after(), before.allocator_after());
        assert_eq!(after.draft(), repeated.draft());
        assert_eq!(
            serde_json::to_value(after.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
        assert!(matches!(
            after.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        assert!(matches!(
            after.draft().input().scenario_presets.members[0]
                .scenario
                .assumptions
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        if body.contains("name=\"raw-0\"") {
            let raw = leaf(&imported, &after, "raw-0", "Placeholder");
            assert_eq!(
                raw.disposition,
                SourceDisposition::SourceOnly(key("overwritten-config-placeholder"))
            );
        }
    }
}

#[test]
fn checked_encounter_guards_do_not_hide_other_leaf_or_inventory_responsibilities() {
    let f = fixture();
    let raw_rule = ConfigurationDispositionRule {
        id: key("actual-scalar-consumer"),
        selector: ValueSelector {
            lane: ValueLane::InputNumber,
            name: "raw-0".into(),
        },
        destinations: vec![ConfigurationDestination::Adapter {
            control: ConfigurationAdapterControl::ScalarInput {
                input: f.inputs[0].1.clone(),
            },
        }],
    };
    let p = bind(
        &f,
        reviewed(&f),
        vec![raw_rule, observation(&f, "observed")],
    );
    for blocked in [false, true] {
        let guard = if blocked {
            r#"<Input name="encounter-selector" string="other"/>"#
        } else {
            ""
        };
        let imported = source(
            &xml(&format!(
                r#"<Input name="raw-0" number="0"/><Placeholder name="observed" number="-3"/><Input name="unknown" boolean="false"/>{guard}"#
            )),
            0xa2,
        );
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        let scalar = leaf(&imported, &actual, "raw-0", "Input");
        let observation = leaf(&imported, &actual, "observed", "Placeholder");
        if blocked {
            pending(scalar);
            pending(observation);
        } else {
            assert_eq!(
                scalar.links,
                vec![OwnedOriginTarget::ScenarioPreset(
                    actual.draft().input().scenario_presets.members[0].id
                )]
            );
            assert_eq!(
                observation.disposition,
                SourceDisposition::SourceOnly(key("branch-does-not-consume-observation"))
            );
        }
        pending(leaf(&imported, &actual, "unknown", "Input"));
        assert!(matches!(
            actual.draft().input().choice_presets.members[0]
                .choices
                .completion,
            DraftListCompletion::Pending { .. }
        ));
        let repeated = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(actual.draft(), repeated.draft());
        assert_eq!(actual.sidecar().origins, repeated.sidecar().origins);
        assert_eq!(actual.allocator_after(), repeated.allocator_after());
        let low = NormalizationLimits {
            max_work: 1,
            ..Default::default()
        };
        assert!(normalize(&imported, &f, &p, low).is_err());
        let recovered = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(actual.draft(), recovered.draft());
        assert_eq!(actual.sidecar().origins, recovered.sidecar().origins);
        assert_eq!(actual.allocator_after(), recovered.allocator_after());
    }
}
