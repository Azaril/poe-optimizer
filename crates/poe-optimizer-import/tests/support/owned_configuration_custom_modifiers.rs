//! A complete, strict-empty saved-source family uses the existing accounting
//! owner. These injected fixtures establish no production catalogue authority.
use super::*;
use poe_optimizer_import::build_instance::SourceOccurrenceId;

fn with_family(f: &Fixture) -> NormalizationPolicy {
    let mut p = bounded(f);
    let inventory = p
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
            id: key("custom-modifier-source-family"),
            selector: None,
            source_states: vec![
                ConfigurationSourceState::Absent,
                ConfigurationSourceState::AuthoredNone,
            ],
        });
    inventory
        .dispositions
        .push(ConfigurationInventoryDisposition {
            source_control: key("custom-modifier-source-family"),
            target: ConfigurationInventoryTarget::Adapters {
                controls: vec![ConfigurationAdapterControl::StrictEmptyCustomModifiers],
            },
        });
    reseal(&mut p);
    p
}

fn family_origins(source: &ImportedBuildInstance) -> Vec<SourceOccurrenceId> {
    source
        .occurrences()
        .iter()
        .filter(|row| {
            row.name() == "CustomModifierBlock"
                || source
                    .attribute(row.id(), "name")
                    .unwrap()
                    .is_some_and(|a| a.decoded() == "customMods")
        })
        .map(|row| row.id())
        .collect()
}

fn assert_source_only(
    source: &ImportedBuildInstance,
    actual: &NormalizedImport,
    scenario: ScenarioPresetId,
) {
    for origin in family_origins(source) {
        let row = &actual.sidecar().origins[origin.ordinal() as usize];
        assert_eq!(row.source, origin);
        assert_eq!(
            row.disposition,
            SourceDisposition::SourceOnly(key("literal-empty-custom-modifier-source"))
        );
        assert_eq!(row.links, [OwnedOriginTarget::ScenarioPreset(scenario)]);
    }
}

#[test]
fn strict_empty_family_preserves_each_source_and_only_closes_reviewed_choice_inventory() {
    let f = fixture();
    let p = with_family(&f);
    let absent = normalize(&source(&xml(""), 0xa1), &f, &p, Default::default()).unwrap();
    for body in [
        "",
        r#"<CustomModifierBlock/>"#,
        r#"<CustomModifierBlock title="" enabled="true"></CustomModifierBlock>"#,
        r#"<CustomModifierBlock title="same" enabled="false"/><CustomModifierBlock title="same"/>"#,
        r#"<Input name="customMods" string=""/>"#,
        r#"<CustomModifierBlock enabled="true"/><Input name="customMods" string=""/><CustomModifierBlock enabled="false"/>"#,
    ] {
        let imported = source(&xml(body), 0xa1);
        let before = normalize(&imported, &f, &bounded(&f), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&actual), [true], "{body}");
        assert_eq!(completed(&before), [body.is_empty()]);
        assert_source_only(
            &imported,
            &actual,
            actual.draft().input().scenario_presets.members[0].id,
        );
        let mut expected = before.draft().input().clone();
        expected.choice_presets.members[0].choices.completion = DraftListCompletion::Complete;
        assert_eq!(actual.draft().input(), &expected);
        assert_eq!(actual.allocator_after(), before.allocator_after());
        assert_eq!(
            actual.draft().input().scenario_presets,
            before.draft().input().scenario_presets,
            "empty custom sources supply no native assumptions or usage"
        );
        let repeated = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(
            serde_json::to_value(actual.sidecar()).unwrap(),
            serde_json::to_value(repeated.sidecar()).unwrap()
        );
    }
    let replay = normalize(&source(&xml(""), 0xa1), &f, &p, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_value(absent.sidecar()).unwrap(),
        serde_json::to_value(replay.sidecar()).unwrap(),
        "A/B/A cannot retain another source family's occurrences"
    );
}

#[test]
fn nonempty_disabled_overwritten_whitespace_and_unsupported_sources_remain_pending_as_a_family() {
    let f = fixture();
    let p = with_family(&f);
    for body in [
        r#"<CustomModifierBlock>unparsed</CustomModifierBlock>"#,
        r#"<CustomModifierBlock enabled="true">unparsed</CustomModifierBlock>"#,
        r#"<CustomModifierBlock enabled="false">unparsed</CustomModifierBlock>"#,
        "<CustomModifierBlock> \t\n </CustomModifierBlock>",
        r#"<CustomModifierBlock>&#32;</CustomModifierBlock>"#,
        r#"<CustomModifierBlock><![CDATA[ ]]></CustomModifierBlock>"#,
        r#"<Input name="customMods" string="unparsed"/>"#,
        r#"<Input name="customMods" string=" "/>"#,
        r#"<CustomModifierBlock/><Input name="customMods" string="unparsed"/>"#,
        r#"<CustomModifierBlock enabled="false">unparsed</CustomModifierBlock><Input name="customMods" string=""/>"#,
        r#"<CustomModifierBlock/><CustomModifierBlock enabled="false">unparsed</CustomModifierBlock>"#,
        r#"<Placeholder name="customMods" string=""/>"#,
        r#"<Placeholder name="customMods" string="unparsed"/>"#,
        r#"<Input name="customMods" number="0"/>"#,
        r#"<Input name="customMods" boolean="false"/>"#,
        r#"<Input name="customMods" string=""/><Input name="customMods" string=""/>"#,
        r#"<CustomModifierBlock enabled="False"/>"#,
        r#"<CustomModifierBlock future="unknown"/>"#,
        r#"<CustomModifierBlock xmlns="urn:unknown"/>"#,
        r#"<CustomModifierBlock><Future/></CustomModifierBlock>"#,
        r#"<CustomModifierBlock><!-- source comment --></CustomModifierBlock>"#,
    ] {
        let imported = source(&xml(body), 0xa2);
        let before = normalize(&imported, &f, &bounded(&f), Default::default()).unwrap();
        let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
        assert_eq!(completed(&actual), [false], "{body}");
        assert_unchanged(&before, &actual);
        for origin in family_origins(&imported) {
            accounting_tests::pending(&actual.sidecar().origins[origin.ordinal() as usize]);
        }
    }
}

#[test]
fn source_family_authority_is_explicit_bounded_and_cannot_be_a_scalar_wildcard() {
    let f = fixture();
    let base = with_family(&f);
    let imported = source(&xml(r#"<CustomModifierBlock/>"#), 0xa3);
    for mode in 0..9 {
        let mut p = base.clone();
        let owner = p.configuration_dispositions.as_mut().unwrap();
        let inventory = owner.inventory.as_mut().unwrap();
        match mode {
            0 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .selector = Some(input_selector(0))
            }
            1 => {
                inventory
                    .source_census
                    .controls
                    .last_mut()
                    .unwrap()
                    .source_states = vec![ConfigurationSourceState::Absent]
            }
            2 => inventory
                .source_census
                .controls
                .last_mut()
                .unwrap()
                .source_states
                .push(ConfigurationSourceState::Authored),
            3 => {
                let ConfigurationInventoryTarget::Adapters { controls } =
                    &mut inventory.dispositions.last_mut().unwrap().target
                else {
                    panic!()
                };
                controls.push(ConfigurationAdapterControl::Encounter);
            }
            4 => {
                let mut control = inventory.source_census.controls.last().unwrap().clone();
                control.id = key("duplicate-family");
                let mut disposition = inventory.dispositions.last().unwrap().clone();
                disposition.source_control = control.id.clone();
                inventory.source_census.controls.push(control);
                inventory.dispositions.push(disposition);
            }
            5 => inventory.source_census.catalogue_evidence = *f.a.rewards.identity(),
            6 => inventory.source_census.constructor_evidence = *f.a.mapping.identity(),
            7 => inventory.source_census.controls.last_mut().unwrap().id = key("unsealed-change"),
            8 => owner.rules.push(ConfigurationDispositionRule {
                id: key("not-a-scalar-adapter"),
                selector: ValueSelector {
                    lane: ValueLane::InputString,
                    name: "customMods".into(),
                },
                destinations: vec![ConfigurationDestination::Adapter {
                    control: ConfigurationAdapterControl::StrictEmptyCustomModifiers,
                }],
            }),
            _ => unreachable!(),
        }
        if mode != 7 {
            reseal(&mut p);
        }
        let prior = serde_json::to_value(&p).unwrap();
        assert!(
            normalize(&imported, &f, &p, Default::default()).is_err(),
            "mode {mode}"
        );
        assert_eq!(serde_json::to_value(&p).unwrap(), prior);
    }
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
        gaps: vec![key("unreviewed-configuration-family")],
    };
    reseal(&mut partial);
    let actual = normalize(&imported, &f, &partial, Default::default()).unwrap();
    assert_eq!(completed(&actual), [false]);
    assert_source_only(
        &imported,
        &actual,
        actual.draft().input().scenario_presets.members[0].id,
    );
    let without = normalize(&imported, &f, &without_inventory(&base), Default::default()).unwrap();
    assert_eq!(completed(&without), [false]);
    accounting_tests::pending(
        &without.sidecar().origins[family_origins(&imported)[0].ordinal() as usize],
    );
}

pub(super) fn scoped_fixture() -> (Fixture, NormalizationPolicy) {
    let mut f = fixture();
    // A complete reward census must contain a real rule. Its independently
    // authored default produces no reward; an enabled input stays unreviewed.
    // This exercises scoped accounting before the historical fallback pass.
    let mut recipe = value_recipe("fixture-reward", "fixture-reward-toggle", true);
    recipe.tiers[0].selectors[0].lane = ValueLane::InputBoolean;
    recipe.missing = MissingValuePolicy::Explicit {
        value: ParameterValue::Boolean(false),
    };
    let control = ConfigurationRewardControl {
        recipe: recipe.id.clone(),
        selector: recipe.tiers[0].selectors[0].clone(),
    };
    let mut rewards = f.a.rewards.input().clone();
    rewards.version = key("fixture-scoped-rewards");
    rewards.rules = vec![RewardRuleInput {
        recipe,
        output: RewardRuleOutput::Cases(vec![
            RewardOutcomeCase {
                when: RewardValue::Boolean(false),
                outcome: RewardTemplate::None,
            },
            RewardOutcomeCase {
                when: RewardValue::Boolean(true),
                outcome: RewardTemplate::Unmapped {
                    issue: key("fixture-reward-unreviewed"),
                },
            },
        ]),
    }];
    f.a.rewards =
        OwnedRewardPolicy::new(rewards, &f.a.mapping, &f.a.schema, Default::default()).unwrap();
    let mut p = with_family(&f);
    p.configuration_reward_inventory = Some(
        ConfigurationRewardInventoryPolicy::PobFreshGeneratedControlsV1 {
            mapping_source: *f.a.mapping.source_identity(),
            reward_policy: *f.a.rewards.identity(),
            controls: vec![control.clone()],
        },
    );
    let inventory = p
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
            id: key("fixture-reward-default"),
            selector: Some(control.selector),
            source_states: vec![ConfigurationSourceState::ConstructorDefaultNone],
        });
    inventory
        .dispositions
        .push(ConfigurationInventoryDisposition {
            source_control: key("fixture-reward-default"),
            target: ConfigurationInventoryTarget::Adapters {
                controls: vec![ConfigurationAdapterControl::Reward {
                    recipe: control.recipe,
                }],
            },
        });
    reseal(&mut p);
    p.configuration_dispositions.as_mut().unwrap().dependencies =
        configuration_disposition_dependencies(&p, &f.a.rewards, Default::default()).unwrap();
    (f, p)
}

#[test]
fn every_configuration_keeps_its_own_family_and_nonempty_dormant_sets_remain_pending() {
    let (f, p) = scoped_fixture();
    for selected in ["1", "2"] {
        for swapped in [false, true] {
            let empty = r#"<ConfigSet id="1"><CustomModifierBlock title="same"/><CustomModifierBlock title="same" enabled="false"/></ConfigSet>"#;
            let pending = r#"<ConfigSet id="2"><CustomModifierBlock enabled="false">unparsed</CustomModifierBlock></ConfigSet>"#;
            let sets = if swapped {
                format!("{pending}{empty}")
            } else {
                format!("{empty}{pending}")
            };
            let imported = source(
                &format!(
                    r#"<PathOfBuilding2><Config activeConfigSet="{selected}">{sets}</Config></PathOfBuilding2>"#
                ),
                0xa4,
            );
            let actual = normalize(&imported, &f, &p, Default::default()).unwrap();
            assert!(actual.draft().input().rewards.members.is_empty());
            assert_eq!(
                completed(&actual),
                if swapped {
                    [false, true]
                } else {
                    [true, false]
                }
            );
            let empty_scenario =
                actual.draft().input().scenario_presets.members[usize::from(swapped)].id;
            for origin in family_origins(&imported) {
                let occurrence = &imported.occurrences()[origin.ordinal() as usize];
                let parent = occurrence.parent().unwrap();
                let row = &actual.sidecar().origins[origin.ordinal() as usize];
                if imported.attribute(parent, "id").unwrap().unwrap().decoded() == "1" {
                    assert_eq!(
                        row.disposition,
                        SourceDisposition::SourceOnly(key("literal-empty-custom-modifier-source"))
                    );
                    assert_eq!(
                        row.links,
                        [OwnedOriginTarget::ScenarioPreset(empty_scenario)]
                    );
                } else {
                    accounting_tests::pending(row);
                }
            }
        }
    }
    let malformed = source(
        r#"<PathOfBuilding2><Config activeConfigSet="1"><ConfigSet id="1"><CustomModifierBlock/></ConfigSet><ConfigSet id="2"><CustomModifierBlock enabled="future"/></ConfigSet></Config></PathOfBuilding2>"#,
        0xa5,
    );
    let actual = normalize(&malformed, &f, &p, Default::default()).unwrap();
    assert_eq!(completed(&actual), [false, false]);
    for origin in family_origins(&malformed) {
        accounting_tests::pending(&actual.sidecar().origins[origin.ordinal() as usize]);
    }
}

#[test]
fn exact_work_boundary_and_late_scope_failure_preserve_inputs_and_replay() {
    let (f, p) = scoped_fixture();
    let imported = source(
        r#"<PathOfBuilding2><Config activeConfigSet="2"><ConfigSet id="1"><CustomModifierBlock/></ConfigSet><ConfigSet id="2"><CustomModifierBlock title="second"/><Input name="customMods" string=""/></ConfigSet></Config></PathOfBuilding2>"#,
        0xa6,
    );
    let expected = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert!(expected.draft().input().rewards.members.is_empty());
    assert_eq!(completed(&expected), [true, true]);
    let policy_before = serde_json::to_value(&p).unwrap();
    let allocator_before = *imported.allocator_state();
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while low < high {
        let mid = low + (high - low) / 2;
        let result = normalize(
            &imported,
            &f,
            &p,
            NormalizationLimits {
                max_work: mid,
                ..Default::default()
            },
        );
        if result.is_ok() {
            high = mid;
        } else {
            low = mid + 1;
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
    assert_eq!(exact.sidecar().origins, expected.sidecar().origins);
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
    assert_eq!(*imported.allocator_state(), allocator_before);
    assert_eq!(serde_json::to_value(&p).unwrap(), policy_before);
    let repeated = normalize(&imported, &f, &p, Default::default()).unwrap();
    assert_eq!(
        serde_json::to_value(expected.sidecar()).unwrap(),
        serde_json::to_value(repeated.sidecar()).unwrap()
    );
}
