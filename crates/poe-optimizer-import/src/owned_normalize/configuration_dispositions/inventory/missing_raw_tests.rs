//! Private source/owner corruption controls. Synthetic declared consumers prove
//! the accounting mechanism, not a production configuration catalogue.
use super::*;

const XML: &str = r#"<PathOfBuilding2><Skills activeSkillSet="1"><SkillSet id="1"><Skill><Gem/></Skill></SkillSet><SkillSet id="2"><Skill><Gem/></Skill></SkillSet></Skills><Config activeConfigSet="1"><ConfigSet id="1"><Input name="other" number="0"/></ConfigSet><ConfigSet id="2"><Input name="other" number="0"/></ConfigSet></Config></PathOfBuilding2>"#;

struct Case {
    draft: DraftSessionInput,
    sources: Vec<SourceOccurrenceId>,
}

fn fields() -> Vec<ConfigurationScenarioField> {
    vec![
        ConfigurationScenarioField::Assumptions,
        ConfigurationScenarioField::Usage,
        ConfigurationScenarioField::UsageIntents,
    ]
}

fn controls() -> Vec<(ConfigurationInventoryControl, ConfigurationInventoryTarget)> {
    fields()
        .into_iter()
        .enumerate()
        .map(|(index, field)| {
            (
                ConfigurationInventoryControl {
                    id: key(&format!("independent-{index}")),
                    selector: Some(ValueSelector {
                        lane: ValueLane::InputNumber,
                        name: "missing".into(),
                    }),
                    source_states: vec![ConfigurationSourceState::Absent],
                },
                ConfigurationInventoryTarget::MissingRawRetainedScenario {
                    fields: vec![field],
                },
            )
        })
        .collect()
}

fn authority(case: &Case) -> Scope<'_> {
    let choice = &case.draft.choice_presets.members[0];
    let DraftListCompletion::Pending { id, .. } = choice.choices.completion else {
        panic!()
    };
    Scope {
        source: case.sources[0],
        scenario: &case.draft.scenario_presets.members[0],
        choice: choice.id,
        issue: id,
    }
}

fn with_case(test: impl FnOnce(&mut Builder<'_, '_>, Case)) {
    gem_inventory::with_usage_inventory_destination(XML, |b, skill, preset, _, _| {
        let sources: Vec<_> = b
            .evidence
            .rows()
            .iter()
            .filter(|r| r.occurrence().name() == "ConfigSet")
            .map(|r| r.occurrence().id())
            .collect();
        let root = b.evidence.rows()[0].occurrence().id();
        let namespace = b.roles.input().namespace.clone();
        let mut registry = OwnedIdRegistry::empty(namespace.clone(), Default::default()).unwrap();
        let input = registry
            .allocate_definition::<ExternalInputDefinition>()
            .unwrap();
        let policy = registry
            .allocate_definition::<UsagePolicyDefinition>()
            .unwrap();
        let parameter = registry
            .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(policy.clone()))
            .unwrap();
        let parameters = || {
            complete(vec![ParameterDraft {
                slot: parameter.clone().into(),
                value: ParameterValue::Boolean(true).into(),
            }])
        };
        let use_id = skill.id;
        let mut draft = DraftSessionInput {
            allocator: b.allocator.state(),
            revision: b.evidence.identity().revision,
            game_version: namespace.clone(),
            weapon_loadouts: complete(vec![]),
            items: complete(vec![]),
            gems: complete(vec![]),
            rewards: complete(vec![]),
            equipment: complete(vec![]),
            allocations: complete(vec![]),
            skills: complete(vec![skill]),
            supports: complete(vec![]),
            payload_links: complete(vec![]),
            character_presets: complete(vec![]),
            equipment_presets: complete(vec![]),
            allocation_presets: complete(vec![]),
            skill_presets: complete(vec![preset]),
            choice_presets: complete(vec![]),
            scenario_presets: complete(vec![]),
            query_presets: complete(vec![]),
            saved_variants: b
                .closure(root, "saved-selection-not-converted", vec![])
                .unwrap(),
        };
        for source in &sources {
            let choice = ChoicePresetDraft {
                id: b.id().unwrap(),
                choices: b
                    .closure(*source, "configuration-roles-not-converted", vec![])
                    .unwrap(),
                rewards: b
                    .closure(*source, "configuration-rewards-not-converted", vec![])
                    .unwrap(),
            };
            let scenario = ScenarioPresetDraft {
                id: b.id().unwrap(),
                scenario: ScenarioDraft {
                    game_version: namespace.clone(),
                    enemy: EnemyDraft {
                        encounter: b.pending(*source, "encounter-not-converted").unwrap(),
                        level: b.pending(*source, "enemy-level-not-converted").unwrap(),
                    },
                    assumptions: b
                        .closure(
                            *source,
                            "external-assumptions-not-converted",
                            vec![ExternalAssumptionDraft {
                                input: input.clone().into(),
                                target: DraftAssumptionTarget::Environment,
                                value: ParameterValue::Boolean(false).into(),
                            }],
                        )
                        .unwrap(),
                    usage: b
                        .closure(
                            *source,
                            "usage-not-converted",
                            vec![UsagePolicyDraft {
                                policy: policy.clone().into(),
                                target: DraftUsageTarget::Skill(DraftSkillTarget::Authored(
                                    use_id.into(),
                                )),
                                parameters: parameters(),
                            }],
                        )
                        .unwrap(),
                    usage_intents: b
                        .closure(
                            *source,
                            "scenario-usage-intents-not-converted",
                            vec![ScenarioUsageIntentDraft {
                                policy: policy.clone().into(),
                                parameters: parameters(),
                            }],
                        )
                        .unwrap(),
                },
            };
            b.link(*source, OwnedOriginTarget::ChoicePreset(choice.id))
                .unwrap();
            b.link(*source, OwnedOriginTarget::ScenarioPreset(scenario.id))
                .unwrap();
            draft.choice_presets.members.push(choice);
            draft.scenario_presets.members.push(scenario);
        }
        draft.allocator = b.allocator.state();
        test(b, Case { draft, sources });
    });
}

fn checked(
    b: &mut Builder<'_, '_>,
    case: &Case,
    controls: &[(ConfigurationInventoryControl, ConfigurationInventoryTarget)],
) -> missing_raw::Checked {
    missing_raw::collect(b, &case.draft, &authority(case), controls)
        .unwrap()
        .unwrap()
        .check(b, &case.draft, &authority(case))
        .unwrap()
        .unwrap()
}

fn baseline(b: &mut Builder<'_, '_>, case: &Case) {
    let before = case.draft.clone();
    let origins = b.origins.clone();
    let allocator = b.allocator.state();
    let controls = controls();
    let mut proof = checked(b, case, &controls);
    assert!(!proof.consumed());
    for (control, target) in &controls {
        assert!(
            proof
                .consume(
                    b,
                    case.sources[0],
                    authority(case).scenario.id,
                    control,
                    target
                )
                .unwrap()
        );
    }
    assert!(proof.consumed());
    assert!(
        !proof
            .consume(
                b,
                case.sources[0],
                authority(case).scenario.id,
                &controls[0].0,
                &controls[0].1
            )
            .unwrap()
    );
    assert_eq!(case.draft, before);
    assert_eq!(b.origins, origins);
    assert_eq!(b.allocator.state(), allocator);
}

#[test]
fn missing_raw_preserves_known_members_and_separate_pending_responsibilities() {
    with_case(|b, case| {
        for scenario in &case.draft.scenario_presets.members {
            assert_eq!(scenario.scenario.assumptions.members.len(), 1);
            assert_eq!(scenario.scenario.usage.members.len(), 1);
            assert_eq!(scenario.scenario.usage_intents.members.len(), 1);
        }
        baseline(b, &case);
        // A conjunction is checked in full; sharing a raw name never merges
        // independent source consumers or their still-Pending fields.
        let mut inputs = controls();
        inputs[0].1 = ConfigurationInventoryTarget::MissingRawRetainedScenario { fields: fields() };
        let mut proof = checked(b, &case, &inputs);
        for (control, target) in &inputs {
            assert!(
                proof
                    .consume(
                        b,
                        case.sources[0],
                        authority(&case).scenario.id,
                        control,
                        target
                    )
                    .unwrap()
            );
        }
        assert!(proof.consumed());
    });
}

fn mutation(b: &mut Builder<'_, '_>, case: &mut Case, index: usize) {
    let source = case.sources[0];
    let other = case.sources[1];
    let scenario = case.draft.scenario_presets.members[0].id;
    let choice = case.draft.choice_presets.members[0].id;
    let assumed = pending_issue(
        &case.draft.scenario_presets.members[0]
            .scenario
            .assumptions
            .completion,
    );
    let usage = pending_issue(
        &case.draft.scenario_presets.members[0]
            .scenario
            .usage
            .completion,
    );
    match index {
        0 => {
            case.draft.scenario_presets.members[0]
                .scenario
                .assumptions
                .completion = DraftListCompletion::Complete
        }
        1 => {
            if let DraftListCompletion::Pending { code, .. } =
                &mut case.draft.scenario_presets.members[0]
                    .scenario
                    .usage
                    .completion
            {
                *code = key("wrong-code");
            }
        }
        2 => {
            if let DraftListCompletion::Pending { id, .. } =
                &mut case.draft.scenario_presets.members[0]
                    .scenario
                    .usage
                    .completion
            {
                *id = assumed;
            }
        }
        3 => {
            case.draft.scenario_presets.members[1]
                .scenario
                .assumptions
                .completion = case.draft.scenario_presets.members[0]
                .scenario
                .assumptions
                .completion
                .clone()
        }
        4 => {
            case.draft.choice_presets.members[1].rewards.completion =
                case.draft.scenario_presets.members[0]
                    .scenario
                    .assumptions
                    .completion
                    .clone()
        }
        5 => b.origins[source.ordinal() as usize]
            .links
            .retain(|link| *link != OwnedOriginTarget::Issue(usage)),
        6 => b.origins[source.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::Issue(usage)),
        7 => b.origins[other.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::Issue(usage)),
        8 => b.origins[other.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::ScenarioPreset(scenario)),
        9 => b.origins[other.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::ChoicePreset(choice)),
        10 => b.origins[source.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::ScenarioPreset(scenario)),
        11 => b.origins[source.ordinal() as usize]
            .links
            .push(OwnedOriginTarget::ChoicePreset(choice)),
        12 => case
            .draft
            .scenario_presets
            .members
            .push(case.draft.scenario_presets.members[0].clone()),
        13 => case
            .draft
            .choice_presets
            .members
            .push(case.draft.choice_presets.members[0].clone()),
        14 => {
            b.origins[source.ordinal() as usize].disposition =
                SourceDisposition::SourceOnly(key("foreign"))
        }
        15 => b.origins[source.ordinal() as usize].source = other,
        16 => b.origins[other.ordinal() as usize].source = source,
        17 => {
            if let DraftField::Pending(p) =
                &mut case.draft.scenario_presets.members[0].scenario.enemy.level
            {
                p.id = usage;
            }
        }
        18 => {
            let id = b.id().unwrap();
            if let DraftListCompletion::Pending { id: existing, .. } =
                &mut case.draft.scenario_presets.members[0]
                    .scenario
                    .usage
                    .completion
            {
                *existing = id;
            }
            b.origins[source.ordinal() as usize]
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(usage));
            b.origins[source.ordinal() as usize]
                .links
                .push(OwnedOriginTarget::Issue(id));
        }
        _ => unreachable!(),
    }
}

fn pending_issue(completion: &DraftListCompletion) -> DraftIssueId {
    let DraftListCompletion::Pending { id, .. } = completion else {
        panic!()
    };
    *id
}

#[test]
fn missing_raw_rechecks_stale_owners_fields_and_reverse_unselected_authority() {
    for index in 0..19 {
        with_case(|b, mut case| {
            baseline(b, &case);
            let inputs = controls();
            let proof = missing_raw::collect(b, &case.draft, &authority(&case), &inputs)
                .unwrap()
                .unwrap();
            mutation(b, &mut case, index);
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            assert!(
                proof
                    .check(b, &case.draft, &authority(&case))
                    .unwrap()
                    .is_none(),
                "mutation {index}"
            );
            // A legitimately replaced issue can be proved in a new attempt;
            // the old proof still cannot silently change its responsibility.
            assert_eq!(
                missing_raw::collect(b, &case.draft, &authority(&case), &inputs)
                    .unwrap()
                    .is_some(),
                index == 18,
                "mutation {index}"
            );
            assert_eq!(case.draft, before);
            assert_eq!(b.origins, origins);
            assert_eq!(b.allocator.state(), allocator);
        });
    }
}

#[test]
fn missing_raw_consumer_cannot_substitute_source_selector_target_or_control() {
    for index in 0..8 {
        with_case(|b, case| {
            baseline(b, &case);
            let inputs = controls();
            let mut proof = checked(b, &case, &inputs);
            let (mut control, mut target) = inputs[0].clone();
            let mut source = case.sources[0];
            let mut scenario = authority(&case).scenario.id;
            match index {
                0 => source = case.sources[1],
                1 => scenario = case.draft.scenario_presets.members[1].id,
                2 => control.id = key("unproved-control"),
                3 => control.selector.as_mut().unwrap().name = "other-name".into(),
                4 => control.selector.as_mut().unwrap().lane = ValueLane::InputBoolean,
                5 => control.source_states = vec![ConfigurationSourceState::ConstructorDefault],
                6 => target = ConfigurationInventoryTarget::MissingRawNoAdditionalInput {},
                7 => target = inputs[1].1.clone(),
                _ => unreachable!(),
            }
            let origins = b.origins.clone();
            assert!(
                !proof
                    .consume(b, source, scenario, &control, &target)
                    .unwrap(),
                "mutation {index}"
            );
            assert!(!proof.consumed());
            assert_eq!(b.origins, origins);
        });
    }
}

#[test]
fn missing_raw_no_additional_input_does_not_borrow_or_require_scenario_completion() {
    with_case(|b, mut case| {
        baseline(b, &case);
        let mut inputs = controls();
        inputs.truncate(1);
        inputs[0].1 = ConfigurationInventoryTarget::MissingRawNoAdditionalInput {};
        for scenario in &mut case.draft.scenario_presets.members {
            scenario.scenario.assumptions.completion = DraftListCompletion::Complete;
            scenario.scenario.usage.completion = DraftListCompletion::Complete;
            scenario.scenario.usage_intents.completion = DraftListCompletion::Complete;
        }
        let before = case.draft.clone();
        let mut proof = checked(b, &case, &inputs);
        assert!(
            proof
                .consume(
                    b,
                    case.sources[0],
                    authority(&case).scenario.id,
                    &inputs[0].0,
                    &inputs[0].1
                )
                .unwrap()
        );
        assert!(proof.consumed());
        assert_eq!(case.draft, before);
    });
}

#[test]
fn missing_raw_work_is_bounded_for_cold_and_cached_proofs_without_mutation() {
    for cached in [false, true] {
        with_case(|b, case| {
            let inputs = controls();
            if cached {
                baseline(b, &case);
            }
            let start = b.work;
            let cache = b.fresh_configuration_sets.clone();
            let before = case.draft.clone();
            let origins = b.origins.clone();
            let allocator = b.allocator.state();
            let _ = checked(b, &case, &inputs);
            let required = b.work;
            for sufficient in [true, false] {
                b.work = start;
                b.fresh_configuration_sets = cache.clone();
                b.limits.max_work = required - usize::from(!sufficient);
                let result = missing_raw::collect(b, &case.draft, &authority(&case), &inputs)
                    .and_then(|proof| proof.unwrap().check(b, &case.draft, &authority(&case)));
                if sufficient {
                    assert!(result.unwrap().is_some());
                } else {
                    assert!(matches!(result, Err(NormalizationError::Limit(_))));
                }
                assert_eq!(case.draft, before);
                assert_eq!(b.origins, origins);
                assert_eq!(b.allocator.state(), allocator);
            }
        });
    }
}

#[test]
fn partial_missing_raw_inventory_exits_before_source_or_owner_inspection() {
    with_case(|b, case| {
        let adapters = AdapterReceipts {
            encounters: &BTreeMap::new(),
            levels: &BTreeMap::new(),
            inputs: &BTreeMap::new(),
            usage: &BTreeMap::new(),
            usage_policy: None,
            rewards: &BTreeMap::new(),
        };
        let scope = authority(&case);
        let receipts = receipts::ScalarReceipts::collect(
            b,
            scope.source,
            scope.scenario,
            scope.choice,
            &adapters,
        )
        .unwrap()
        .finish(b, &case.draft, &adapters, false)
        .unwrap();
        let inventory = CompiledInventory {
            complete: false,
            strict_empty_custom_modifiers: false,
            missing_raw: true,
            controls: controls(),
        };
        b.fresh_configuration_sets = None;
        let start = b.work;
        let maximum = b.limits.max_work;
        let before = case.draft.clone();
        let origins = b.origins.clone();
        let allocator = b.allocator.state();
        b.limits.max_work = start;
        assert!(
            prepare(b, &case.draft, authority(&case), &inventory, &receipts)
                .unwrap()
                .is_none()
        );
        assert!(
            !match_receipts(
                b,
                case.sources[0],
                authority(&case).scenario.id,
                &inventory,
                &[],
                &[],
                AuxiliaryReceipts::default()
            )
            .unwrap()
        );
        assert_eq!(b.work, start);
        assert!(b.fresh_configuration_sets.is_none());

        // Without missing targets, prepare must have exactly the previous
        // matcher cost on this unaccounted raw leaf; no new frame/owner scan.
        let ordinary = CompiledInventory {
            complete: true,
            missing_raw: false,
            strict_empty_custom_modifiers: false,
            controls: vec![],
        };
        b.limits.max_work = maximum;
        assert!(
            !match_receipts(
                b,
                scope.source,
                scope.scenario.id,
                &ordinary,
                receipts.iter().as_slice(),
                receipts.inapplicable(),
                AuxiliaryReceipts::default()
            )
            .unwrap()
        );
        let matched_work = b.work;
        b.work = start;
        assert!(
            prepare(b, &case.draft, authority(&case), &ordinary, &receipts)
                .unwrap()
                .is_none()
        );
        assert_eq!(b.work, matched_work);
        assert!(b.fresh_configuration_sets.is_none());
        assert_eq!(case.draft, before);
        assert_eq!(b.origins, origins);
        assert_eq!(b.allocator.state(), allocator);
    });
}
