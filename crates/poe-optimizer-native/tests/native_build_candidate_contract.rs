use poe_optimizer_core::{candidate::*, evaluation::*, options::EvaluationOptions};
use poe_optimizer_data::class_tree::AttributeOption;
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, NativeBackend};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    sync::Arc,
};
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/builds/spark-passive-equipment.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 30_000 };
// Thread-local counters isolate tested calls from libtest's parallel runner.
// The production crates forbid unsafe code; this test allocator delegates every
// operation unchanged to System and records only allocations on the measured thread.
thread_local! {
    static COUNTING: Cell<bool> = const { Cell::new(false) };
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}
struct CountingAllocator;
fn record_allocation() {
    if COUNTING.try_with(Cell::get).unwrap_or(false) {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));
    }
}
unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: System receives the exact layout supplied by the allocator caller.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        record_allocation();
        // SAFETY: System receives the exact layout supplied by the allocator caller.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        record_allocation();
        // SAFETY: All pointer/layout arguments are passed unchanged to System.
        unsafe { System.realloc(ptr, layout, new_size) }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        // SAFETY: The pointer was allocated by System with the supplied layout.
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;
fn allocation_count<T>(run: impl FnOnce() -> T) -> (T, usize) {
    struct Reset;
    impl Drop for Reset {
        fn drop(&mut self) {
            COUNTING.with(|flag| flag.set(false));
        }
    }
    ALLOCATIONS.with(|count| count.set(0));
    COUNTING.with(|flag| flag.set(true));
    let reset = Reset;
    let result = run();
    drop(reset);
    (result, ALLOCATIONS.with(Cell::get))
}

fn request(content: String) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content,
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn make_domain(backend: &NativeBackend, source: &str) -> ControlledBuildDomain {
    let catalog = Arc::new(
        ControlledBuildCatalog::new(backend.data().clone(), source.into(), vec![]).unwrap(),
    );
    ControlledBuildDomain::new(
        catalog,
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 20,
                ascendancy_passive_points: 1,
                active_skill_count: 1,
                supports_per_skill: 2,
                ..Default::default()
            },
            ..Default::default()
        },
        AttributeOptionLocks::default(),
    )
    .unwrap()
}
#[test]
fn changing_connected_tree_attribute_and_equipment_candidates_match_document_path_without_allocations()
 {
    for source in [MACE, SPARK] {
        let backend = NativeBackend::new();
        let domain = make_domain(&backend, source);
        let catalog = domain.catalog();
        let prepared = backend.prepare_controlled_build(catalog, &[]).unwrap();
        let base = catalog.source_selection();
        let node = *base.attribute_options.keys().next().unwrap();
        let mut handles = Vec::new();
        let mut scratch = ActorScratch::default();
        for option in AttributeOption::ALL {
            for gear in [true, false] {
                let mut selection = base.clone();
                selection.attribute_options.insert(node, option);
                if !gear {
                    selection.candidate.equipment.remove("Amulet");
                }
                let handle = domain.admit(selection, &mut scratch).unwrap();
                let document = domain.materialize(&handle).unwrap();
                let actual = backend
                    .calculate(&request(document.content.clone()), BUDGET)
                    .unwrap();
                catalog
                    .validate_native_realization(&handle, &actual, &backend.identity())
                    .unwrap();
                let snapshot = backend
                    .evaluate_controlled_build(&prepared, &handle, BUDGET)
                    .unwrap();
                assert_eq!(
                    serde_json::to_value(prepared.snapshot_measurements(&snapshot)).unwrap(),
                    serde_json::to_value(&actual.measurements).unwrap()
                );
                assert_eq!(actual.exports[0].content, document.content);
                handles.push(handle);
            }
        }
        let (_, allocations) = allocation_count(|| {
            for i in 0..5000 {
                black_box(
                    prepared
                        .measure(black_box(&handles[i % handles.len()]))
                        .unwrap(),
                );
            }
        });
        assert_eq!(allocations, 0);
        assert_eq!(prepared.footprint().retained_xml_bytes, 0);
        assert_eq!(prepared.footprint().cached_candidate_results, 0);
        assert_eq!(catalog.footprint().materialized_candidates, 0);
    }
}
#[test]
fn full_finalist_evidence_rejects_tampered_tree_equipment_actor_context_and_export() {
    let backend = NativeBackend::new();
    let domain = make_domain(&backend, MACE);
    let catalog = domain.catalog();
    let handle = domain
        .admit(catalog.source_selection(), &mut ActorScratch::default())
        .unwrap();
    let fresh = backend
        .calculate(
            &request(domain.materialize(&handle).unwrap().content),
            BUDGET,
        )
        .unwrap();
    catalog
        .validate_native_realization(&handle, &fresh, &backend.identity())
        .unwrap();
    for (media, pointer) in [
        ("native-tree+", "/attribute_options/13397"),
        ("native-tree+", "/paid_nodes/1/effective_node_id"),
        ("native-profile+", "/equipment/Amulet/source_sha256"),
        ("native-profile+", "/actor_resources/life"),
    ] {
        let mut result: EvaluationResult =
            serde_json::from_value(serde_json::to_value(&fresh).unwrap()).unwrap();
        let attachment = result
            .attachments
            .iter_mut()
            .find(|a| a.media_type.contains(media))
            .unwrap();
        let mut evidence: serde_json::Value = serde_json::from_str(&attachment.content).unwrap();
        *evidence.pointer_mut(pointer).unwrap() = serde_json::json!("tampered");
        attachment.content = evidence.to_string();
        assert!(
            catalog
                .validate_native_realization(&handle, &result, &backend.identity())
                .is_err(),
            "accepted {pointer}"
        );
    }
    let mut result: EvaluationResult =
        serde_json::from_value(serde_json::to_value(&fresh).unwrap()).unwrap();
    result.exports[0].content.push(' ');
    assert!(
        catalog
            .validate_native_realization(&handle, &result, &backend.identity())
            .is_err()
    );
    let mut result: EvaluationResult =
        serde_json::from_value(serde_json::to_value(&fresh).unwrap()).unwrap();
    result.context.enemy_level += 1;
    assert!(
        catalog
            .validate_native_realization(&handle, &result, &backend.identity())
            .is_err()
    );
    let other = make_domain(&backend, MACE);
    let foreign = other
        .admit(
            other.catalog().source_selection(),
            &mut ActorScratch::default(),
        )
        .unwrap();
    let prepared = backend.prepare_controlled_build(catalog, &[]).unwrap();
    assert_eq!(
        prepared.calculate(&foreign).unwrap_err().kind,
        EvaluationErrorKind::BackendContract
    );
    assert!(
        backend
            .evaluate_controlled_build(&prepared, &handle, EvaluationBudget { timeout_ms: 0 })
            .is_err()
    );
}

#[test]
fn failing_imported_scalar_composition_is_deferred_while_legal_roots_evaluate() {
    use poe_optimizer_data::game_data::{
        self, GameDataLoader, LoadLimits, PassiveEffect, PassiveStat, TrustPolicy,
    };
    use poe_optimizer_native::{CompiledGameData, HostClock};
    for (source, nodes) in [(MACE, [3936, 13397]), (SPARK, [4739, 22419])] {
        let mut package = game_data::bundled_snapshot().unwrap().package().clone();
        for view in &mut package.passive_effects {
            if nodes.contains(&view.key.physical_node_id) {
                view.effects = vec![PassiveEffect {
                    stat: PassiveStat::ArmourFlat,
                    value: 750_000.0,
                }];
            }
        }
        package.refresh_section_digests().unwrap();
        let snapshot = GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap();
        let backend = NativeBackend::with_data(
            Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
            HostClock,
        )
        .unwrap();
        let domain = make_domain(&backend, source);
        let catalog = domain.catalog();
        // Source admission is structural. Numeric failure belongs to this selected
        // combination, never to all alternatives that share its fixed scenario.
        let source_handle = domain
            .admit(catalog.source_selection(), &mut ActorScratch::default())
            .unwrap();
        let source_error = source_handle.character().unwrap_err().to_owned();
        let full_error = backend
            .calculate(&request(source.into()), BUDGET)
            .unwrap_err();
        assert_eq!(full_error.message, source_error);
        let prepared = backend.prepare_controlled_build(catalog, &[]).unwrap();
        let typed_error = prepared.calculate(&source_handle).err().unwrap();
        assert_eq!(typed_error.kind, full_error.kind);
        assert_eq!(typed_error.message, source_error);
        let mut root = catalog.source_selection();
        root.candidate.passives.clear();
        root.attribute_options.clear();
        let root_handle = domain.admit(root, &mut ActorScratch::default()).unwrap();
        assert!(root_handle.character().is_ok());
        let xml = domain.materialize(&root_handle).unwrap();
        let full = backend.calculate(&request(xml.content), BUDGET).unwrap();
        let typed = prepared.measure(&root_handle).unwrap();
        assert_eq!(
            serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
            serde_json::to_value(&full.measurements).unwrap()
        );
        catalog
            .validate_native_realization(&root_handle, &full, &backend.identity())
            .unwrap();
    }
}

#[test]
fn fixed_scenario_projection_retains_metric_guards() {
    use poe_optimizer_core::metrics::{ActorScope, MetricQuery};
    let backend = NativeBackend::new();
    let domain = make_domain(&backend, MACE);
    let query = MetricQuery {
        actor: ActorScope::Player,
        id: "life".into(),
    };
    for (bad, kind) in [
        (
            vec![query.clone(), query],
            EvaluationErrorKind::InvalidRequest,
        ),
        (
            vec![MetricQuery {
                actor: ActorScope::SelectedMinion,
                id: "life".into(),
            }],
            EvaluationErrorKind::UnsupportedCapability,
        ),
        (
            vec![MetricQuery {
                actor: ActorScope::Player,
                id: "full_dps".into(),
            }],
            EvaluationErrorKind::UnsupportedCapability,
        ),
    ] {
        let error = backend
            .prepare_controlled_build(domain.catalog(), &bad)
            .err()
            .unwrap();
        assert_eq!(error.kind, kind);
    }
}
