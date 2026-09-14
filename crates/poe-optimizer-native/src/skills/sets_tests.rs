//! Real native Load histories; no reference state is imported and no constructor
//! or full Load/Sync parity is claimed.
use super::*;
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_import::{
    build_instance::InstanceImportLimits,
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};

fn prepare(xml: &str) -> PreparedSkills {
    prepare_with_limits(xml, SkillPreparationLimits::default()).unwrap()
}
fn prepare_with_limits(
    xml: &str,
    limits: SkillPreparationLimits,
) -> Result<PreparedSkills, EvaluationError> {
    let data = CompiledGameData::bundled().unwrap();
    let build = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([157; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let view = resolve_view(
        &build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_skills(&build, &view, &data, limits)
}
fn n(value: f64) -> NumericValue {
    NumericValue::new(value)
}
fn active<'a>(view: &mut SkillSetReadView<'a>) -> PreparedSkillSetRow<'a> {
    match view.activation().unwrap() {
        SkillSetActivation::Selected(row) => row,
        SkillSetActivation::Unavailable => panic!("expected a produced active alias"),
    }
}
fn source(body: &str) -> String {
    format!("<PathOfBuilding2><Skills>{body}</Skills></PathOfBuilding2>")
}

#[test]
fn unavailable_startup_is_distinct_from_a_loaded_default_with_nil_title() {
    let absent = prepare("<PathOfBuilding2/>");
    assert!(
        absent
            .skill_set_view(SkillSetReadLimits::default())
            .is_none()
    );
    let loaded = prepare(&source(""));
    let mut view = loaded
        .skill_set_view(SkillSetReadLimits::default())
        .unwrap();
    assert!(view.is_singleton().unwrap());
    assert_eq!(view.ordered_key(1).unwrap().unwrap().value(), 1.0);
    let row = active(&mut view);
    assert_eq!(view.title(&row).unwrap(), None);
    assert_eq!(view.group_count(&row).unwrap(), 0);
    let container = view.source().unwrap();
    assert_eq!(
        view.origin(&row).unwrap(),
        SetOrigin::Default {
            domain: SelectionDomain::Skills,
            container: Some(container),
        }
    );
}

#[test]
fn duplicate_winner_retains_distinct_created_rows_and_group_membership() {
    let stage = prepare(&source(
        "<SkillSet id='1' title='Earlier'><Skill label='earlier'/></SkillSet><SkillSet id='1.0' title='Winner'><Skill label='one'/><Skill label='two'/></SkillSet><SkillSet title='Generated'/>",
    ));
    assert_eq!(stage.report().status, SkillPreparationStatus::Complete);
    let sets = stage.sets.as_ref().unwrap();
    assert_eq!(sets.rows.len(), 3);
    assert_eq!(sets.rows[0].groups.len(), 1);
    assert_eq!(sets.rows[1].groups.len(), 2);
    assert_ne!(sets.rows[0].origin, sets.rows[1].origin);
    let bindings: Vec<_> = stage
        .owner
        .instances()
        .iter()
        .filter(|binding| matches!(binding.instance(), AuthoredInstanceId::SkillSet(_)))
        .collect();
    for (row, binding) in sets.rows.iter().zip(bindings) {
        assert_eq!(
            row.origin,
            SetOrigin::Authored {
                instance: binding.instance(),
                source: binding.source()
            }
        );
    }
    let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
    assert_eq!(view.dense_order_len().unwrap(), 3);
    assert!(!view.is_singleton().unwrap());
    assert_eq!(
        (1..=3)
            .map(|i| view.ordered_key(i).unwrap().unwrap().value())
            .collect::<Vec<_>>(),
        vec![1.0, 1.0, 2.0]
    );
    let winner = view.winner(n(1.0)).unwrap().unwrap();
    assert!(winner.same_identity(&active(&mut view)));
    assert_eq!(view.title(&winner).unwrap(), Some("Winner"));
    assert_eq!(view.group_count(&winner).unwrap(), 2);
    assert!(std::ptr::eq(
        view.group(&winner, 1).unwrap().unwrap(),
        &stage.report().groups[1]
    ));
    assert!(view.group(&winner, 3).unwrap().is_none());
    let generated = view.winner(n(2.0)).unwrap().unwrap();
    assert_eq!(view.title(&generated).unwrap(), Some("Generated"));
}

#[test]
fn generated_sparse_ids_and_signed_zero_fractional_infinite_keys_keep_source_values() {
    let stage = prepare(
        "<PathOfBuilding2><Skills activeSkillSet='-0'><SkillSet id='2'/><SkillSet id='8'/><SkillSet/><SkillSet id='-0' title='negative'/><SkillSet id='0' title='positive'/><SkillSet id='0.5' title=''/><SkillSet id='inf'/></Skills></PathOfBuilding2>",
    );
    let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
    let expected = [2.0_f64, 8.0, 3.0, -0.0, 0.0, 0.5, f64::INFINITY];
    for (i, number) in expected.into_iter().enumerate() {
        assert_eq!(
            view.ordered_key(i + 1).unwrap().unwrap().value().to_bits(),
            number.to_bits()
        );
    }
    let zero = view.winner(n(-0.0)).unwrap().unwrap();
    assert!(zero.same_identity(&view.winner(n(0.0)).unwrap().unwrap()));
    assert_eq!(view.title(&zero).unwrap(), Some("positive"));
    assert_eq!(
        view.active_key().unwrap().value().to_bits(),
        (-0.0_f64).to_bits()
    );
    assert_eq!(
        view.row_key(&zero).unwrap().value().to_bits(),
        0.0_f64.to_bits()
    );
    let fraction = view.winner(n(0.5)).unwrap().unwrap();
    assert_eq!(view.title(&fraction).unwrap(), Some(""));
    assert!(view.winner(n(f64::INFINITY)).unwrap().is_some());
    assert!(view.winner(n(f64::NAN)).unwrap().is_none());
}

#[test]
fn source_failure_retains_published_title_and_only_successfully_attached_groups() {
    let stage = prepare(&source(
        "<SkillSet id='7' title='Prefix'><Skill><Gem skillId='SparkPlayer' level='1'/></Skill><Skill><Gem skillId='SparkPlayer'/></Skill></SkillSet><SkillSet id='8'/>",
    ));
    assert_eq!(stage.report().status, SkillPreparationStatus::SourceFailure);
    let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
    let row = view.winner(n(7.0)).unwrap().unwrap();
    assert_eq!(view.title(&row).unwrap(), Some("Prefix"));
    assert_eq!(view.group_count(&row).unwrap(), 1);
    assert!(view.group(&row, 1).unwrap().unwrap().attached);
    assert!(!stage.report().groups[1].attached);
    assert_eq!(view.active_key().unwrap().value(), 0.0);
    assert!(matches!(
        view.activation().unwrap(),
        SkillSetActivation::Unavailable
    ));
    assert!(view.winner(n(8.0)).unwrap().is_none());
}

#[test]
fn later_load_reset_retains_old_group_alias_separately_from_current_map_and_id() {
    let stage = prepare(
        "<PathOfBuilding2><Skills><SkillSet id='1' title='Old'><Skill/></SkillSet></Skills><Skills><SkillSet id='2' title='New'/><SkillSet id='nan'/></Skills></PathOfBuilding2>",
    );
    assert_eq!(stage.report().status, SkillPreparationStatus::SourceFailure);
    let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
    assert_eq!(view.active_key().unwrap().value(), 0.0);
    assert!(view.winner(n(1.0)).unwrap().is_none());
    let active = active(&mut view);
    assert_eq!(view.title(&active).unwrap(), Some("Old"));
    assert_eq!(view.group_count(&active).unwrap(), 1);
    let current = view.winner(n(2.0)).unwrap().unwrap();
    assert_eq!(view.title(&current).unwrap(), Some("New"));
    assert!(!current.same_identity(&active));
    assert_eq!(view.dense_order_len().unwrap(), 1);
    assert_eq!(stage.sets.as_ref().unwrap().rows.len(), 2);
}

#[test]
fn legacy_append_error_keeps_the_actual_missing_winner_and_unattached_group() {
    let stage = prepare(&source("<SkillSet id='2'/><Skill/>"));
    assert_eq!(stage.report().status, SkillPreparationStatus::SourceFailure);
    let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
    assert!(view.winner(n(1.0)).unwrap().is_none());
    let row = view.winner(n(2.0)).unwrap().unwrap();
    assert_eq!(view.group_count(&row).unwrap(), 0);
    assert!(!stage.report().groups[0].attached);
}

#[test]
fn borrowed_reads_reject_foreign_owners_charge_limits_and_preserve_shared_state() {
    let xml = source("<SkillSet id='1' title='\u{03bb}'/>");
    let stage = Arc::new(prepare(&xml));
    let other = prepare(&xml);
    let before = serde_json::to_vec(stage.report()).unwrap();
    let mut view = stage
        .skill_set_view(SkillSetReadLimits {
            max_steps: 100,
            max_text_bytes: 2,
        })
        .unwrap();
    let row = active(&mut view);
    assert!(row.belongs_to(&stage));
    assert_eq!(view.title(&row).unwrap(), Some("\u{03bb}"));
    assert_eq!(
        view.title(&row).unwrap_err().kind,
        EvaluationErrorKind::InvalidRequest
    );
    assert_eq!(view.usage().text_bytes, 2);
    let mut foreign = other.skill_set_view(SkillSetReadLimits::default()).unwrap();
    assert_eq!(
        foreign.title(&row).unwrap_err().kind,
        EvaluationErrorKind::BackendContract
    );
    let mut zero = stage
        .skill_set_view(SkillSetReadLimits {
            max_steps: 0,
            max_text_bytes: 0,
        })
        .unwrap();
    assert_eq!(
        zero.dense_order_len().unwrap_err().kind,
        EvaluationErrorKind::InvalidRequest
    );
    assert_eq!(zero.usage(), SkillSetReadUsage::default());
    let handles: Vec<_> = (0..3)
        .map(|_| {
            let stage = Arc::clone(&stage);
            std::thread::spawn(move || {
                let mut view = stage.skill_set_view(SkillSetReadLimits::default()).unwrap();
                let row = view.winner(n(1.0)).unwrap().unwrap();
                assert_eq!(view.title(&row).unwrap(), Some("\u{03bb}"));
                view.origin(&row).unwrap()
            })
        })
        .collect();
    let expected = view.origin(&row).unwrap();
    for thread in handles {
        assert_eq!(thread.join().unwrap(), expected);
    }
    assert_eq!(serde_json::to_vec(stage.report()).unwrap(), before);
    assert_eq!(
        prepare_with_limits(
            &xml,
            SkillPreparationLimits {
                max_fields: 0,
                ..Default::default()
            }
        )
        .err()
        .unwrap()
        .kind,
        EvaluationErrorKind::InvalidRequest
    );
}
