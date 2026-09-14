use super::*;
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_import::{
    build_instance::InstanceImportLimits,
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use std::sync::OnceLock;

fn data() -> &'static Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| CompiledGameData::bundled().unwrap())
}
fn import(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([116; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn prepare(body: &str) -> PreparedConfiguration {
    prepare_xml(&format!("<PathOfBuilding2>{body}</PathOfBuilding2>"))
}
fn prepare_xml(xml: &str) -> PreparedConfiguration {
    let build = import(xml);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_configuration(
        &build,
        &view,
        data(),
        ConfigurationPreparationLimits::default(),
    )
    .unwrap()
}
fn selected<'a>(view: &mut ConfigurationReadView<'a>) -> (NumericValue, ConfigurationSetRow<'a>) {
    match view.active().unwrap() {
        ConfigurationActivation::Selected { key, row } => (key, row),
        ConfigurationActivation::Unavailable => panic!("expected reached active-row assignment"),
    }
}

#[test]
fn retained_reader_preserves_duplicate_rows_winner_order_and_authored_origins() {
    let state = prepare(
        "<Config activeConfigSet='2.5'><ConfigSet id='2.5' title='old'/><ConfigSet id='2.5' title='new'/></Config>",
    );
    let mut view = state.read_view(ConfigurationReadLimits::default());
    assert_eq!(view.produced_len().unwrap(), 2);
    let old = view.produced_row(1).unwrap().unwrap();
    let new = view.produced_row(2).unwrap().unwrap();
    assert!(!old.same_identity(&new));
    assert_eq!(view.title(&old).unwrap(), Some("old"));
    assert_eq!(view.title(&new).unwrap(), Some("new"));
    let winner = view.winner(NumericValue::new(2.5)).unwrap().unwrap();
    assert!(winner.same_identity(&new));
    let (key, active) = selected(&mut view);
    assert_eq!(key.value(), 2.5);
    assert!(active.same_identity(&new));
    assert_eq!(view.dense_order_len().unwrap(), 2);
    assert!(!view.is_singleton().unwrap());
    assert_eq!(view.ordered_key(1).unwrap().unwrap().value(), 2.5);
    assert_eq!(view.ordered_key(2).unwrap().unwrap().value(), 2.5);
    let SetOrigin::Authored {
        instance: first,
        source: first_source,
    } = view.origin(&old).unwrap()
    else {
        panic!("authored row")
    };
    let SetOrigin::Authored {
        instance: second,
        source: second_source,
    } = view.origin(&new).unwrap()
    else {
        panic!("authored row")
    };
    assert_ne!(first, second);
    assert_ne!(first_source, second_source);
    assert!(
        state
            .machine
            .build
            .instances()
            .iter()
            .any(|binding| binding.instance() == first && binding.source() == first_source)
    );
    assert!(
        state
            .machine
            .build
            .instances()
            .iter()
            .any(|binding| binding.instance() == second && binding.source() == second_source)
    );
}

#[test]
fn retained_reader_does_not_turn_sparse_order_into_a_length_claim() {
    let state = prepare(
        "<Config activeConfigSet='2'><ConfigSet id='7'/><Input name='legacy' number='3'/><ConfigSet id='2'/></Config>",
    );
    let mut view = state.read_view(ConfigurationReadLimits::default());
    assert_eq!(view.ordered_key(1).unwrap().unwrap().value(), 7.0);
    assert!(view.ordered_key(2).unwrap().is_none());
    assert_eq!(view.ordered_key(3).unwrap().unwrap().value(), 2.0);
    assert_eq!(
        view.dense_order_len().err().unwrap().kind,
        EvaluationErrorKind::UnsupportedCapability
    );
    assert_eq!(selected(&mut view).0.value(), 2.0);
}

#[test]
fn retained_reader_keeps_registered_failure_prefix_without_inventing_prior_aliases() {
    let state = prepare(
        "<Config><ConfigSet id='2' title='before'/><ConfigSet title='published'/><ConfigSet id='9' title='unreached'/></Config>",
    );
    assert_eq!(
        state.report().status,
        ConfigurationPrefixStatus::SourceFailure
    );
    assert_eq!(
        state.report().failure.as_ref().unwrap().stage,
        "load_config_set"
    );
    assert!(state.report().continuation.is_none());
    let mut view = state.read_view(ConfigurationReadLimits::default());
    assert_eq!(view.produced_len().unwrap(), 2);
    assert!(matches!(
        view.active().unwrap(),
        ConfigurationActivation::Unavailable
    ));
    let published = view.produced_row(2).unwrap().unwrap();
    assert_eq!(view.title(&published).unwrap(), Some("published"));
    let published_key = view.key(&published).unwrap();
    assert!(
        view.winner(published_key)
            .unwrap()
            .unwrap()
            .same_identity(&published)
    );
    assert!(view.winner(NumericValue::new(9.0)).unwrap().is_none());
    assert_eq!(view.dense_order_len().unwrap(), 1);
    let nan = prepare("<Config><ConfigSet id='nan'/></Config>");
    let mut nan_view = nan.read_view(ConfigurationReadLimits::default());
    assert_eq!(nan_view.produced_len().unwrap(), 0);
    assert!(matches!(
        nan_view.active().unwrap(),
        ConfigurationActivation::Unavailable
    ));
    assert!(
        nan_view
            .winner(NumericValue::new(f64::NAN))
            .unwrap()
            .is_none()
    );
}

#[test]
fn retained_reader_keeps_selected_zero_bits_and_noninteger_keys() {
    let state = prepare(
        "<Config activeConfigSet='-0'><ConfigSet id='0' title='zero'/><ConfigSet id='2.5'/><ConfigSet id='inf'/></Config>",
    );
    let mut view = state.read_view(ConfigurationReadLimits::default());
    let (key, active) = selected(&mut view);
    assert_eq!(key.value().to_bits(), (-0.0_f64).to_bits());
    assert_eq!(
        view.key(&active).unwrap().value().to_bits(),
        0.0_f64.to_bits()
    );
    assert!(
        view.winner(NumericValue::new(-0.0))
            .unwrap()
            .unwrap()
            .same_identity(&active)
    );
    assert!(view.winner(NumericValue::new(2.5)).unwrap().is_some());
    assert!(
        view.winner(NumericValue::new(f64::INFINITY))
            .unwrap()
            .is_some()
    );
    let fallback = prepare("<Config activeConfigSet='99'><ConfigSet id='-0'/></Config>");
    let mut fallback_view = fallback.read_view(ConfigurationReadLimits::default());
    assert_eq!(
        selected(&mut fallback_view).0.value().to_bits(),
        (-0.0_f64).to_bits()
    );
}

#[test]
fn retained_reader_separates_constructor_empty_title_and_later_view_selection() {
    let constructor = prepare("");
    let mut view = constructor.read_view(ConfigurationReadLimits::default());
    let (_, active) = selected(&mut view);
    assert_eq!(view.title(&active).unwrap(), None);
    assert!(matches!(
        view.origin(&active).unwrap(),
        SetOrigin::Default {
            domain: SelectionDomain::Configuration,
            container: None
        }
    ));
    assert_eq!(
        constructor.report().continuation.as_ref().unwrap().stage,
        ConfigurationContinuationStage::InitialBuildModList
    );
    let state = prepare(
        "<Config><ConfigSet id='1' title=''/></Config><Config><ConfigSet id='2' title='later'/></Config>",
    );
    let mut view = state.read_view(ConfigurationReadLimits::default());
    let (_, active) = selected(&mut view);
    assert_eq!(view.title(&active).unwrap(), Some(""));
    assert_eq!(view.produced_len().unwrap(), 1);
    assert!(view.winner(NumericValue::new(2.0)).unwrap().is_none());
    assert_eq!(
        state
            .report()
            .continuation
            .as_ref()
            .unwrap()
            .unexecuted_containers
            .len(),
        1
    );
    assert_ne!(
        serde_json::to_value(view.origin(&active).unwrap()).unwrap(),
        serde_json::to_value(state.report().view_selected_set).unwrap()
    );
}

#[test]
fn retained_reader_is_owner_bound_and_read_budgets_leave_the_producer_unchanged() {
    let body = "<Config><ConfigSet id='1' title='alpha'><Input name='kept' string='original'/></ConfigSet></Config>";
    let state = prepare(body);
    let other = prepare(body);
    let before = serde_json::to_vec(state.report()).unwrap();
    let before_text = state.machine.limits.max_text_bytes;
    let before_fields = state.machine.limits.max_fields;
    let before_steps = state.machine.pattern.steps_used();
    let mut foreign_view = other.read_view(ConfigurationReadLimits::default());
    let (_, foreign_row) = selected(&mut foreign_view);
    let mut view = state.read_view(ConfigurationReadLimits::default());
    assert_eq!(
        view.title(&foreign_row).err().unwrap().kind,
        EvaluationErrorKind::BackendContract
    );
    let (_, own_row) = selected(&mut view);
    assert!(own_row.belongs_to(&state));
    assert!(!own_row.same_identity(&foreign_row));
    let mut no_steps = state.read_view(ConfigurationReadLimits {
        max_steps: 0,
        ..Default::default()
    });
    assert_eq!(
        no_steps.produced_len().err().unwrap().kind,
        EvaluationErrorKind::InvalidRequest
    );
    assert_eq!(no_steps.usage(), ConfigurationReadUsage::default());
    let mut text_limit = state.read_view(ConfigurationReadLimits {
        max_text_bytes: 4,
        ..Default::default()
    });
    assert_eq!(
        text_limit.title(&own_row).err().unwrap().kind,
        EvaluationErrorKind::InvalidRequest
    );
    assert_eq!(
        text_limit.usage(),
        ConfigurationReadUsage {
            steps: 1,
            text_bytes: 0
        }
    );
    let mut exact = state.read_view(ConfigurationReadLimits {
        max_steps: 6,
        max_text_bytes: 5,
    });
    assert_eq!(exact.title(&own_row).unwrap(), Some("alpha"));
    assert_eq!(
        exact.usage(),
        ConfigurationReadUsage {
            steps: 6,
            text_bytes: 5
        }
    );
    assert_eq!(state.machine.limits.max_text_bytes, before_text);
    assert_eq!(state.machine.limits.max_fields, before_fields);
    assert_eq!(state.machine.pattern.steps_used(), before_steps);
    assert!(!state.machine.input_origins.is_empty());
    assert!(!state.machine.sources.is_empty());
    assert!(!state.machine.instances.is_empty());
    assert_eq!(serde_json::to_vec(state.report()).unwrap(), before);
}

#[test]
fn retained_reader_all_five_originals_join_the_actual_produced_active_row() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    for n in 1..=5 {
        let xml = std::fs::read_to_string(root.join(format!("build-{n:02}.xml"))).unwrap();
        let state = prepare_xml(&xml);
        assert_eq!(state.report().status, ConfigurationPrefixStatus::Prepared);
        assert_eq!(
            state.report().continuation.as_ref().unwrap().stage,
            ConfigurationContinuationStage::UpdateControls
        );
        let mut view = state.read_view(ConfigurationReadLimits::default());
        assert_eq!(view.produced_len().unwrap(), 1);
        assert!(view.is_singleton().unwrap());
        let key = view.ordered_key(1).unwrap().unwrap();
        let winner = view.winner(key).unwrap().unwrap();
        let (_, active) = selected(&mut view);
        assert!(winner.same_identity(&active));
        assert_eq!(
            serde_json::to_value(view.origin(&active).unwrap()).unwrap(),
            serde_json::to_value(state.report().active_set.unwrap()).unwrap()
        );
    }
}
