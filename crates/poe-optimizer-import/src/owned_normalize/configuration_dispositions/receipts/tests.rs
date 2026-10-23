use super::*;

fn instance(local: u64) -> InstanceId {
    InstanceId::from_parts(BuildLineage::from_bytes([53; 16]), local).unwrap()
}

fn source_ids() -> Vec<SourceOccurrenceId> {
    let imported = crate::build_instance::ImportedBuildInstance::from_decoded(
        crate::decode_build(
            br#"<PathOfBuilding2><Config><ConfigSet id="1"><Input name="shared" number="0"/></ConfigSet><ConfigSet id="2"/></Config></PathOfBuilding2>"#,
        ).unwrap(),
        BuildLineage::from_bytes([53; 16]),
        Default::default(),
    ).unwrap();
    imported.occurrences().iter().map(|row| row.id()).collect()
}

fn selector() -> ValueSelector {
    ValueSelector {
        lane: ValueLane::InputNumber,
        name: "shared".into(),
    }
}

fn control(name: &str) -> ConfigurationAdapterControl {
    ConfigurationAdapterControl::Reward { recipe: key(name) }
}

fn row(
    scope: SourceOccurrenceId,
    origin: Option<SourceOccurrenceId>,
    source_state: ConfigurationSourceState,
) -> CheckedConfigurationReceipt {
    CheckedConfigurationReceipt {
        scope,
        origin,
        selector: Some(selector()),
        control: control("consumer"),
        source_state,
        target: OwnedOriginTarget::ChoicePreset(ChoicePresetId::from_instance_id(instance(1))),
        additional_targets: vec![OwnedOriginTarget::Reward(
            RewardSelectionId::from_instance_id(instance(2)),
        )],
        source_only: None,
    }
}

// These private fixtures test collection/matching only. The ordinary producer
// tests separately prove current values, order, target and source correspondence.
#[test]
fn receipts_preserve_no_origin_states_order_targets_and_duplicate_consumers() {
    let ids = source_ids();
    let scope = ids[2];
    let origin = ids[3];
    let scenario = ScenarioPresetId::from_instance_id(instance(3));
    let states = [
        ConfigurationSourceState::Authored,
        ConfigurationSourceState::AuthoredNone,
        ConfigurationSourceState::Placeholder,
        ConfigurationSourceState::ConstructorDefault,
        ConfigurationSourceState::ConstructorDefaultNone,
        ConfigurationSourceState::Absent,
        ConfigurationSourceState::OverwrittenRawPlaceholder,
        ConfigurationSourceState::IgnoredDefaultPlaceholder,
        ConfigurationSourceState::UnselectedProjection,
    ];
    let receipts = ScopeReceipts {
        scope,
        scenario,
        rows: states
            .iter()
            .enumerate()
            .map(|(i, state)| row(scope, (!matches!(i, 3..=5)).then_some(origin), *state))
            .collect(),
        encounter_absence: EncounterAbsence::MissingProof,
        custom_modifiers: None,
    };
    assert_eq!(receipts.len(), states.len());
    assert_eq!(
        receipts
            .rows
            .iter()
            .map(|r| r.source_state)
            .collect::<Vec<_>>(),
        states
    );
    for receipt in &receipts.rows {
        assert_eq!(
            receipt.target,
            OwnedOriginTarget::ChoicePreset(ChoicePresetId::from_instance_id(instance(1)))
        );
        assert_eq!(
            receipt.additional_targets,
            vec![OwnedOriginTarget::Reward(
                RewardSelectionId::from_instance_id(instance(2))
            )]
        );
        assert_eq!(receipt.source_only, None);
    }
    let selected = selector();
    let consumer = control("consumer");
    let found: Vec<_> = receipts
        .matching_leaf(scope, origin, &selected, &consumer)
        .collect();
    assert_eq!(
        found.len(),
        6,
        "duplicates must remain ambiguous, not become one matching receipt"
    );
    assert!(found.iter().all(|r| r.origin == Some(origin)));
    assert_eq!(found[0].source_state, ConfigurationSourceState::Authored);
    assert_eq!(
        found[5].source_state,
        ConfigurationSourceState::UnselectedProjection
    );
    assert_eq!(
        receipts.rows.iter().filter(|r| r.origin.is_none()).count(),
        3
    );
}

#[test]
fn leaf_lookup_requires_exact_source_scope_lane_name_and_consumer() {
    let ids = source_ids();
    let scope = ids[2];
    let origin = ids[3];
    let selected = selector();
    let consumer = control("consumer");
    for mutation in 0..7 {
        let mut actual = row(scope, Some(origin), ConfigurationSourceState::Authored);
        match mutation {
            0 => {}
            1 => actual.scope = ids[4],
            2 => actual.origin = None,
            3 => actual.origin = Some(ids[4]),
            4 => actual.selector.as_mut().unwrap().lane = ValueLane::PlaceholderNumber,
            5 => actual.selector.as_mut().unwrap().name = "other".into(),
            6 => actual.control = control("another-consumer"),
            _ => unreachable!(),
        }
        let receipts = ScopeReceipts {
            scope,
            scenario: ScenarioPresetId::from_instance_id(instance(3)),
            rows: vec![actual],
            encounter_absence: EncounterAbsence::MissingProof,
            custom_modifiers: None,
        };
        assert_eq!(
            receipts
                .matching_leaf(scope, origin, &selected, &consumer)
                .count(),
            usize::from(mutation == 0),
            "mutation {mutation}"
        );
    }
}

#[test]
fn encounter_absence_is_exact_borrowed_source_authority_not_adapter_presence() {
    let ids = source_ids();
    let scope = ids[2];
    let scenario = ScenarioPresetId::from_instance_id(instance(3));
    let absent: BTreeSet<String> = ["first".into(), "second".into()].into();
    let requested: BTreeSet<String> = ["second".into()].into();
    for state in [
        EncounterAbsence::MissingProof,
        EncounterAbsence::Unavailable,
        EncounterAbsence::Checked(&absent),
    ] {
        let proven = matches!(state, EncounterAbsence::Checked(_));
        let supplied = !matches!(state, EncounterAbsence::MissingProof);
        let receipts = ScopeReceipts {
            scope,
            scenario,
            rows: vec![],
            encounter_absence: state,
            custom_modifiers: None,
        };
        assert_eq!(receipts.has_encounter_proof(), supplied);
        assert_eq!(
            receipts.proves_encounter_branch(scope, scenario, &requested),
            proven
        );
        assert!(!receipts.proves_encounter_branch(ids[4], scenario, &requested));
        assert!(!receipts.proves_encounter_branch(
            scope,
            ScenarioPresetId::from_instance_id(instance(4)),
            &requested
        ));
        assert!(!receipts.proves_encounter_branch(scope, scenario, &["unproved".into()].into()));
    }
    assert_eq!(
        absent,
        ["first".into(), "second".into()].into(),
        "the collector borrows, never edits the producer's guard set"
    );
}
