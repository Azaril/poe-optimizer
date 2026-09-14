//! Native publication-lineage mechanics and the actual preparation boundary.
//! No original-source execution or complete Load/Sync claim is made here.
use super::*;
use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_data::{
    game_data::{GameDataSnapshot, bundled_snapshot},
    item_assembly::ItemInventoryPolicy,
};
use poe_optimizer_import::{
    build_instance::InstanceImportLimits,
    decode_build,
    item_sets::{ItemSetLimits, ItemSetReadLimits},
    selected_view::{NumericValue, ResolveLimits, resolve_view},
};
use std::sync::{Arc, OnceLock};

fn policy() -> &'static ItemInventoryPolicy {
    static SNAPSHOT: OnceLock<GameDataSnapshot> = OnceLock::new();
    &SNAPSHOT
        .get_or_init(|| bundled_snapshot().unwrap())
        .item_assembly()
        .policy()
        .inventory
}
fn import(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([143; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
struct Harness {
    build: ImportedBuildInstance,
    state: ItemSetState,
    lineage: ItemSetLineage,
    sources: BTreeMap<usize, SourceOccurrenceId>,
    bytes: usize,
    instructions: usize,
}
impl Harness {
    fn new(children: &str) -> Self {
        let build = import(&format!(
            "<PathOfBuilding2><Items>{children}</Items></PathOfBuilding2>"
        ));
        let sources = build
            .occurrences()
            .iter()
            .map(|r| (r.range().start, r.id()))
            .collect();
        let mut lineage = ItemSetLineage::default();
        let mut bytes = 64 * 1024 * 1024;
        let mut instructions = 131072;
        lineage.reserve(&mut bytes, &mut instructions).unwrap();
        let state = ItemSetState::new(
            policy(),
            ItemSetLimits {
                max_bytes: bytes,
                ..Default::default()
            },
        )
        .unwrap();
        lineage
            .capture(
                &state,
                0,
                SetOrigin::Default {
                    domain: SelectionDomain::Items,
                    container: None,
                },
            )
            .unwrap();
        Self {
            build,
            state,
            lineage,
            sources,
            bytes,
            instructions,
        }
    }
    fn apply(&mut self, index: usize) -> Option<ItemPreparationFailure> {
        let projection = self.build.project_items().unwrap();
        let node = &projection.containers()[0].children()[index];
        apply(
            &mut self.state,
            node,
            &mut ItemSetLoadContext {
                build: &self.build,
                sources: &self.sources,
                lineage: &mut self.lineage,
                instructions_left: &mut self.instructions,
                bytes_left: &mut self.bytes,
            },
        )
        .unwrap()
    }
    fn finish(&mut self) -> Option<ItemPreparationFailure> {
        let projection = self.build.project_items().unwrap();
        finish(
            &mut self.state,
            &projection.containers()[0],
            &mut ItemSetLoadContext {
                build: &self.build,
                sources: &self.sources,
                lineage: &mut self.lineage,
                instructions_left: &mut self.instructions,
                bytes_left: &mut self.bytes,
            },
        )
        .unwrap()
    }
    fn authored(&self, ordinal: usize) -> SetOrigin {
        let binding = self
            .build
            .instances()
            .iter()
            .filter(|binding| matches!(binding.instance(), AuthoredInstanceId::ItemSet(_)))
            .nth(ordinal)
            .unwrap();
        SetOrigin::Authored {
            instance: binding.instance(),
            source: binding.source(),
        }
    }
    fn winner(&self, key: f64) -> ItemSetIdentity {
        self.state
            .read_view(ItemSetReadLimits::default())
            .winner(NumericValue::new(key))
            .unwrap()
            .unwrap()
            .identity()
    }
}

#[test]
fn constructor_reset_and_fallback_keep_distinct_default_origins() {
    let mut h = Harness::new("");
    let constructor = h.winner(1.0);
    assert_eq!(
        h.lineage.origins[&constructor],
        SetOrigin::Default {
            domain: SelectionDomain::Items,
            container: None
        }
    );
    h.state.begin_load().unwrap();
    assert!(
        h.state
            .read_view(ItemSetReadLimits::default())
            .previous()
            .unwrap()
            .unwrap()
            .identity()
            .same_identity(&constructor)
    );
    assert!(h.finish().is_none());
    let fallback = h.winner(1.0);
    assert!(!constructor.same_identity(&fallback));
    let container = h.build.project_items().unwrap().containers()[0]
        .element()
        .source_range()
        .start;
    let expected = SetOrigin::Default {
        domain: SelectionDomain::Items,
        container: Some(h.sources[&container]),
    };
    assert_eq!(h.lineage.origins[&fallback], expected);
    assert_eq!(h.lineage.origins.len(), 2);
    let mut read = h.state.read_view(ItemSetReadLimits::default());
    let active = read.active().unwrap().unwrap();
    assert_eq!(h.lineage.origin(&h.state, &active).unwrap(), expected);
    assert!(active.identity().same_identity(&fallback));
    assert!(read.previous().unwrap().unwrap().same_identity(&active));
    assert_eq!(h.state.phase(), ItemSetPhase::AwaitingActivation);
}

#[test]
fn duplicate_and_generated_keys_keep_each_exact_authored_occurrence() {
    let mut h = Harness::new(
        "<ItemSet id='1' title='First'/><ItemSet id='0x1' title='Last'/><ItemSet title='Generated'/>",
    );
    h.state.begin_load().unwrap();
    assert!(h.apply(0).is_none());
    let first = h.winner(1.0);
    assert!(h.apply(1).is_none());
    let winner = h.winner(1.0);
    assert!(!first.same_identity(&winner));
    assert!(h.apply(2).is_none());
    let generated = h.winner(2.0);
    for (id, ordinal) in [(&first, 0), (&winner, 1), (&generated, 2)] {
        assert_eq!(h.lineage.origins[id], h.authored(ordinal));
    }
    assert_eq!(h.lineage.origins.len(), 4);
    assert_eq!(h.state.creation_count(), 4);
    let mut read = h.state.read_view(ItemSetReadLimits::default());
    assert_eq!(read.dense_order_len().unwrap(), 3);
    assert_eq!(
        (1..=3)
            .map(|n| read.ordered_key(n).unwrap().unwrap().value())
            .collect::<Vec<_>>(),
        vec![1.0, 1.0, 2.0]
    );
}

#[test]
fn actual_source_error_keeps_published_origin_across_allowed_restart() {
    let mut h = Harness::new("<ItemSet id='7'><SocketIdURL/></ItemSet><ItemSet id='8'/>");
    let constructor = h.winner(1.0);
    h.state.begin_load().unwrap();
    let failure = h.apply(0).unwrap();
    assert!(failure.source_error);
    let failed_row = h.winner(7.0);
    assert_eq!(h.lineage.origins[&failed_row], h.authored(0));
    assert_eq!(h.lineage.origins.len(), 2);
    assert!(
        h.state
            .read_view(ItemSetReadLimits::default())
            .winner(NumericValue::new(8.0))
            .unwrap()
            .is_none()
    );
    // Component restart after the reached failure is supported. This does not
    // simulate completing the failed original Load or skipping a pending Sync.
    h.state.begin_load().unwrap();
    assert!(
        h.state
            .read_view(ItemSetReadLimits::default())
            .previous()
            .unwrap()
            .unwrap()
            .identity()
            .same_identity(&constructor)
    );
    assert!(h.apply(1).is_none());
    let second = h.winner(8.0);
    assert_eq!(h.lineage.origins[&second], h.authored(1));
    assert_eq!(h.lineage.origins[&failed_row], h.authored(0));
    assert_eq!(h.lineage.origins.len(), 3);
}

#[test]
fn failed_publication_preserves_source_error_without_fabricating_origin() {
    let mut h = Harness::new("<ItemSet id='nan'/><ItemSet id='8'/>");
    h.state.begin_load().unwrap();
    let failure = h.apply(0).unwrap();
    assert!(failure.source_error);
    assert!(failure.message.contains("NaN"));
    assert_eq!(h.state.creation_count(), 1);
    assert_eq!(h.lineage.origins.len(), 1);
}

#[test]
fn reservation_failure_precedes_creation_and_late_resource_keeps_origin() {
    let mut h = Harness::new("<ItemSet id='7'/>");
    h.state.begin_load().unwrap();
    let before = h.state.creation_count();
    let mut too_small = ItemSetLineage::ENTRY_BYTES - 1;
    let mut steps = 10;
    let failure = h.lineage.reserve(&mut too_small, &mut steps).unwrap_err();
    assert_eq!(failure.kind, EvaluationErrorKind::InvalidRequest);
    assert_eq!(h.state.creation_count(), before);
    assert_eq!(too_small, ItemSetLineage::ENTRY_BYTES - 1);
    assert!(!h.lineage.reserved);

    // Measure only logical counters of the same native operation to select the
    // last value-write frontier; no observed graph/poststate is reimported.
    let mut probe = ItemSetState::new(policy(), ItemSetLimits::default()).unwrap();
    probe.begin_load().unwrap();
    probe
        .begin_item_set(ItemSetInput {
            id: Some("7"),
            ..Default::default()
        })
        .unwrap();
    let max_values = probe.usage().values - 1;
    let mut state = ItemSetState::new(
        policy(),
        ItemSetLimits {
            max_values,
            ..Default::default()
        },
    )
    .unwrap();
    let mut lineage = ItemSetLineage::default();
    let mut bytes = 64 * 1024 * 1024;
    let mut instructions = 131072;
    lineage.reserve(&mut bytes, &mut instructions).unwrap();
    lineage
        .capture(
            &state,
            0,
            SetOrigin::Default {
                domain: SelectionDomain::Items,
                container: None,
            },
        )
        .unwrap();
    state.begin_load().unwrap();
    let origin = h.authored(0);
    let outcome = ItemSetLoadContext {
        build: &h.build,
        sources: &h.sources,
        lineage: &mut lineage,
        instructions_left: &mut instructions,
        bytes_left: &mut bytes,
    }
    .creating(&mut state, origin, |s| {
        s.begin_item_set(ItemSetInput {
            id: Some("7"),
            ..Default::default()
        })
    })
    .unwrap();
    assert_eq!(outcome.unwrap_err().kind, AssemblyErrorKind::Resource);
    assert_eq!(state.creation_count(), 2);
    let row = state
        .read_view(ItemSetReadLimits::default())
        .winner(NumericValue::new(7.0))
        .unwrap()
        .unwrap();
    assert_eq!(lineage.origin(&state, &row).unwrap(), origin);
}

#[test]
fn actual_preparation_resolves_lineage_and_rejects_equal_xml_foreign_owners() {
    static DATA: OnceLock<Arc<crate::CompiledGameData>> = OnceLock::new();
    let data = DATA.get_or_init(|| crate::CompiledGameData::bundled().unwrap());
    let xml = "<PathOfBuilding2><Items><ItemSet id='7'><SocketIdURL/></ItemSet></Items></PathOfBuilding2>";
    let build = import(xml);
    let view = resolve_view(
        &build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let stage = crate::items::prepare_authored_items(
        &build,
        &view,
        data,
        crate::items::ItemPreparationLimits::default(),
    )
    .unwrap();
    assert!(stage.report().failure.as_ref().unwrap().source_error);
    stage.validate_binding(&build, &view, data).unwrap();
    let state = stage.item_sets().unwrap();
    let row = state
        .read_view(ItemSetReadLimits::default())
        .winner(NumericValue::new(7.0))
        .unwrap()
        .unwrap();
    let binding = build
        .instances()
        .iter()
        .find(|b| matches!(b.instance(), AuthoredInstanceId::ItemSet(_)))
        .unwrap();
    assert_eq!(
        stage.item_set_origin(&row).unwrap(),
        SetOrigin::Authored {
            instance: binding.instance(),
            source: binding.source()
        }
    );
    let foreign = Harness::new("<ItemSet id='7'/>");
    let foreign_row = foreign
        .state
        .read_view(ItemSetReadLimits::default())
        .active()
        .unwrap()
        .unwrap();
    assert_eq!(
        stage.item_set_origin(&foreign_row).unwrap_err().kind,
        EvaluationErrorKind::BackendContract
    );
    let other_build = import(xml);
    let other_view = resolve_view(
        &other_build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert_eq!(
        stage
            .validate_binding(&other_build, &other_view, data)
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
    let other_data =
        Arc::new(crate::CompiledGameData::compile(Arc::new(data.snapshot().clone())).unwrap());
    assert_eq!(
        stage
            .validate_binding(&build, &view, &other_data)
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
}
