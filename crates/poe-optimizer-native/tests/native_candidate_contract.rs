//! Differential boundary tests. Independent numerical PoB oracles remain in the
//! source/core suites; these verify the optimized adapter loses no admitted state.
use poe_optimizer_core::{evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_data::{
    class_tree::{self, ClassTreeSelection},
    game_data::{self, GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy},
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportLoadout, NativeMaceCandidate, NormalMaceAlternative,
};
use poe_optimizer_native::{CompiledGameData, EvaluationClock, HostClock, NativeBackend};
use std::{
    alloc::{GlobalAlloc, Layout, System},
    cell::Cell,
    hint::black_box,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};

const TEMPLATE: &str = include_str!("../../../tests/fixtures/calibration/mace-smithing.xml");
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
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn loadouts() -> Vec<MaceSupportLoadout> {
    [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
    .into_iter()
    .map(|keys| MaceSupportLoadout::new(keys.into_iter().map(str::to_owned).collect()).unwrap())
    .collect()
}
fn weapons(data: &GameDataSnapshot) -> Vec<NormalMaceAlternative> {
    data.package()
        .weapons
        .iter()
        .flat_map(|weapon| {
            [0, 20].map(|quality| NormalMaceAlternative {
                id: format!("{}-q{quality}", weapon.id),
                item_text: format!(
                    "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: {quality}\nImplicits: 0",
                    weapon.name
                ),
            })
        })
        .collect()
}
fn catalog(data: Arc<GameDataSnapshot>, xml: &str) -> ControlledMaceCatalog {
    let choices = class_tree::selections(data.tree()).unwrap();
    ControlledMaceCatalog::with_tree_loadouts(
        data.clone(),
        xml.into(),
        weapons(&data),
        loadouts(),
        choices,
    )
    .unwrap()
}
fn custom(edit: impl FnOnce(&mut GameDataPackage)) -> Arc<GameDataSnapshot> {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    Arc::new(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap(),
    )
}
fn custom_snapshot() -> Arc<GameDataSnapshot> {
    custom(|package| {
        package
            .supports
            .iter_mut()
            .find(|gem| gem.id == "rapid_attacks_i")
            .unwrap()
            .modifiers[0]
            .value = 37.0;
        package
            .supports
            .iter_mut()
            .find(|gem| gem.id == "heavy_swing")
            .unwrap()
            .modifiers[0]
            .value = 13.25;
        package.weapons[0].physical_minimum += 0.75;
        package.character.life_per_strength = 2.75;
        package
            .passive_effects
            .iter_mut()
            .find(|effect| effect.key.physical_node_id == 24475)
            .unwrap()
            .effects[0]
            .value = -7.5;
    })
}
fn assert_measurements(actual: &[MetricMeasurement], expected: &[MetricMeasurement]) {
    assert_eq!(actual.len(), expected.len());
    for (actual, expected) in actual.iter().zip(expected) {
        assert_eq!(actual.query, expected.query);
        assert_eq!(actual.unit, expected.unit);
        assert_eq!(actual.schema_version, expected.schema_version);
        assert_eq!(actual.value, expected.value, "{}", actual.query.id);
        if let (
            MeasurementValue::Finite { value: actual },
            MeasurementValue::Finite { value: expected },
        ) = (&actual.value, &expected.value)
        {
            assert_eq!(
                actual.to_bits(),
                expected.to_bits(),
                "finite values including signed zero"
            );
        }
    }
}
fn backend(data: Arc<GameDataSnapshot>) -> NativeBackend {
    NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(data).unwrap()),
        HostClock,
    )
    .unwrap()
}
fn parity_matrix(data: Arc<GameDataSnapshot>, xml: &str) -> (usize, usize) {
    let registry = catalog(data.clone(), xml);
    let backend = backend(data);
    let baseline = backend.calculate(&request(xml), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    assert_eq!(components.axis_counts(), [4, 105, 7]);
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    let footprint = prepared.footprint();
    assert_eq!(
        (
            footprint.weapon_components,
            footprint.tree_components,
            footprint.support_components
        ),
        (4, 105, 7)
    );
    assert_eq!(footprint.retained_xml_bytes, 0);
    assert_eq!(footprint.cached_candidate_results, 0);
    assert!(
        // Includes 105 newly retained, bounded numeric actor components.
        footprint.owned_component_bytes < 64 * 1024,
        "bounded axis buffers: {footprint:?}"
    );
    let mut legal = 0;
    let mut rejected = 0;
    for alternative in registry.alternatives() {
        let handle = registry.validated_native_candidate(&alternative.candidate, &components);
        if !registry
            .requirements(&alternative.candidate)
            .unwrap()
            .is_legal()
        {
            assert!(handle.is_err());
            rejected += 1;
            continue;
        }
        let handle = handle.unwrap();
        let full_request = request(
            &registry
                .materialize(&alternative.candidate)
                .unwrap()
                .content,
        );
        let full = backend.calculate(&full_request, BUDGET).unwrap();
        registry
            .validate_native_realization(&alternative.candidate, &full, &scenario)
            .unwrap();
        let typed = backend
            .evaluate_controlled_mace(&prepared, &handle, BUDGET)
            .unwrap();
        assert!(typed.diagnostic_only());
        assert!(typed.elapsed_ms() >= 0.0);
        assert_measurements(&prepared.snapshot_measurements(&typed), &full.measurements);
        let pure_full = backend.prepare(&full_request).unwrap().calculate().unwrap();
        let poe_optimizer_native::NativeCalculation::Mace(expected) = pure_full else {
            panic!("Mace input")
        };
        assert_eq!(prepared.calculate(&handle).unwrap(), expected);
        legal += 1;
    }
    (legal, rejected)
}
#[test]
fn every_legal_candidate_matches_full_document_outputs_measurements_and_realization() {
    assert_eq!(
        parity_matrix(Arc::new(game_data::bundled_snapshot().unwrap()), TEMPLATE),
        (1884, 1056)
    );
}
#[test]
fn all_custom_data_candidates_match_full_document_without_reviewed_value_fallback() {
    let xml = TEMPLATE
        .replace(
            "name=\"enemyArmour\" number=\"0\"",
            "name=\"enemyArmour\" number=\"1500\"",
        )
        .replace(
            "name=\"enemyFireResist\" number=\"0\"",
            "name=\"enemyFireResist\" number=\"70\"",
        );
    assert_ne!(xml, TEMPLATE);
    assert_eq!(parity_matrix(custom_snapshot(), &xml), (1884, 1056));
}
#[test]
fn private_catalog_and_data_bindings_reject_cross_instance_handles() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let first = catalog(data.clone(), TEMPLATE);
    let second = catalog(data.clone(), TEMPLATE);
    let backend = backend(data);
    let baseline = backend.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let first_scenario = first
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let second_scenario = second
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let first_components = first
        .native_components(&first_scenario, &backend.identity())
        .unwrap();
    let second_components = second
        .native_components(&second_scenario, &backend.identity())
        .unwrap();
    let prepared = backend
        .prepare_controlled_mace(&first_components, &[])
        .unwrap();
    let legal = first
        .alternatives()
        .iter()
        .find(|alt| first.requirements(&alt.candidate).unwrap().is_legal())
        .unwrap();
    let foreign = second
        .validated_native_candidate(&legal.candidate, &second_components)
        .unwrap();
    assert!(
        prepared
            .calculate(&foreign)
            .unwrap_err()
            .message
            .contains("different prepared catalog")
    );
    let own = first
        .validated_native_candidate(&legal.candidate, &first_components)
        .unwrap();
    assert!(prepared.calculate(&own).is_ok());
    let changed_backend = NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(custom_snapshot()).unwrap()),
        HostClock,
    )
    .unwrap();
    assert!(
        changed_backend
            .prepare_controlled_mace(&first_components, &[])
            .is_err()
    );
    assert!(
        changed_backend
            .evaluate_controlled_mace(&prepared, &own, BUDGET)
            .is_err()
    );
    // The calculation holds no source/catalog XML; a valid private handle stays
    // usable after all source-facing objects are dropped.
    drop(first_components);
    drop(second_components);
    drop(first);
    drop(second);
    assert!(prepared.calculate(&own).is_ok());
}
#[test]
fn metric_selection_and_request_errors_preserve_full_backend_contract() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data.clone(), TEMPLATE);
    let backend = backend(data);
    let baseline = backend.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let candidate = &registry
        .alternatives()
        .iter()
        .find(|alt| registry.requirements(&alt.candidate).unwrap().is_legal())
        .unwrap()
        .candidate;
    let handle = registry
        .validated_native_candidate(candidate, &components)
        .unwrap();
    let queries = ["selected_hit_dps", "selected_average_hit", "life"].map(|id| MetricQuery {
        actor: ActorScope::Player,
        id: id.into(),
    });
    let prepared = backend
        .prepare_controlled_mace(&components, &queries)
        .unwrap();
    let values = prepared.snapshot_measurements(&prepared.measure(&handle).unwrap());
    let mut full = request(&registry.materialize(candidate).unwrap().content);
    full.metrics = queries.to_vec();
    assert_measurements(
        &values,
        &backend.calculate(&full, BUDGET).unwrap().measurements,
    );
    assert_eq!(values[0].query.id, "life");
    assert!(matches!(
        values[1].value,
        MeasurementValue::Unavailable { .. }
    ));
    for bad in [
        vec![queries[0].clone(), queries[0].clone()],
        vec![MetricQuery {
            actor: ActorScope::SelectedMinion,
            id: "life".into(),
        }],
        vec![MetricQuery {
            actor: ActorScope::Player,
            id: "full_dps".into(),
        }],
    ] {
        assert!(backend.prepare_controlled_mace(&components, &bad).is_err());
    }
    assert_eq!(
        backend
            .evaluate_controlled_mace(&prepared, &handle, EvaluationBudget { timeout_ms: 0 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::InvalidRequest
    );
}
#[test]
fn invalid_owner_and_requirement_states_never_receive_private_calculation_handles() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let invalid = ClassTreeSelection {
        class_id: 1,
        ascendancy_id: Some("Monk3".into()),
        entrance_node_id: None,
        ascendancy_node_id: Some(24475),
    };
    assert!(
        ControlledMaceCatalog::with_tree_loadouts(
            data.clone(),
            TEMPLATE.into(),
            weapons(&data),
            loadouts(),
            vec![invalid]
        )
        .is_err()
    );
    assert!(MaceSupportLoadout::new(vec!["brutality_i".into(), "brutality_i".into()]).is_err());
    let registry = catalog(data.clone(), TEMPLATE);
    let backend = backend(data);
    let baseline = backend.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let rejected = registry
        .alternatives()
        .iter()
        .filter(|alt| !registry.requirements(&alt.candidate).unwrap().is_legal())
        .count();
    assert_eq!(rejected, 1056);
    for alt in registry
        .alternatives()
        .iter()
        .filter(|alt| !registry.requirements(&alt.candidate).unwrap().is_legal())
    {
        assert!(
            registry
                .validated_native_candidate(&alt.candidate, &components)
                .is_err()
        );
    }
}
#[test]
fn mixed_typed_calculations_allocate_nothing_but_scheduler_adaptation_is_explicitly_owned() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data.clone(), TEMPLATE);
    let backend = backend(data);
    let baseline = backend.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    let handles: Vec<NativeMaceCandidate> = registry
        .alternatives()
        .iter()
        .filter_map(|alt| {
            registry
                .validated_native_candidate(&alt.candidate, &components)
                .ok()
        })
        .collect();
    assert_eq!(handles.len(), 1884);
    let (_, allocations) = allocation_count(|| {
        for candidate in handles.iter().cycle().take(8_000) {
            black_box(prepared.calculate(black_box(candidate)).unwrap());
            black_box(prepared.measure(black_box(candidate)).unwrap());
            black_box(
                backend
                    .evaluate_controlled_mace(&prepared, black_box(candidate), BUDGET)
                    .unwrap(),
            );
        }
    });
    assert_eq!(
        allocations, 0,
        "numeric/stack measurement paths must allocate nothing"
    );
    let snapshot = prepared.measure(&handles[0]).unwrap();
    let (measurements, allocations) =
        allocation_count(|| prepared.snapshot_measurements(&snapshot));
    assert_eq!(measurements.len(), 10);
    assert!(
        allocations > 0,
        "owned scheduler contract remains an explicit cost"
    );
}
struct StepClock {
    now: AtomicU64,
    step: u64,
}
impl EvaluationClock for StepClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.now.fetch_add(self.step, Ordering::Relaxed))
    }
}
struct BackwardClock(AtomicU64);
impl EvaluationClock for BackwardClock {
    fn now(&self) -> Duration {
        Duration::from_millis(self.0.fetch_sub(1, Ordering::Relaxed))
    }
}
#[test]
fn typed_deadlines_use_the_same_host_clock_contract_as_full_evaluation() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data.clone(), TEMPLATE);
    let host = backend(data);
    let baseline = host.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &host.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &host.identity())
        .unwrap();
    let candidate = &registry
        .alternatives()
        .iter()
        .find(|alt| registry.requirements(&alt.candidate).unwrap().is_legal())
        .unwrap()
        .candidate;
    let handle = registry
        .validated_native_candidate(candidate, &components)
        .unwrap();
    let step = NativeBackend::with_data(
        host.data().clone(),
        StepClock {
            now: AtomicU64::new(0),
            step: 5,
        },
    )
    .unwrap();
    let prepared = step.prepare_controlled_mace(&components, &[]).unwrap();
    assert_eq!(
        step.evaluate_controlled_mace(&prepared, &handle, EvaluationBudget { timeout_ms: 10 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::Timeout
    );
    let back =
        NativeBackend::with_data(host.data().clone(), BackwardClock(AtomicU64::new(10))).unwrap();
    assert_eq!(
        back.evaluate_controlled_mace(&prepared, &handle, BUDGET)
            .unwrap_err()
            .kind,
        EvaluationErrorKind::BackendContract
    );
}

#[test]
fn unused_custom_character_composition_failures_do_not_abort_other_candidates() {
    let data = custom(|package| {
        for record in package.passive_effects.iter_mut().filter(|record| {
            record.key.physical_node_id == 3936 || record.key.physical_node_id == 14960
        }) {
            record.effects[0].stat = game_data::PassiveStat::FireResistanceFlat;
            record.effects[0].value = 1_000_000.0;
        }
    });
    let registry = catalog(data.clone(), TEMPLATE);
    let backend = backend(data);
    let baseline = backend.calculate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    assert_eq!(prepared.footprint().deferred_character_errors, 1);
    let mut bad = 0;
    let mut good = 0;
    for alternative in registry.alternatives() {
        let Ok(handle) = registry.validated_native_candidate(&alternative.candidate, &components)
        else {
            continue;
        };
        let typed = prepared.calculate(&handle);
        if alternative.tree.entrance_node_id == Some(3936)
            && alternative.tree.ascendancy_node_id == Some(14960)
        {
            let full = backend.calculate(
                &request(
                    &registry
                        .materialize(&alternative.candidate)
                        .unwrap()
                        .content,
                ),
                BUDGET,
            );
            let typed = typed.unwrap_err();
            let full = full.unwrap_err();
            assert_eq!(typed.kind, full.kind);
            assert_eq!(typed.message, full.message);
            bad += 1;
        } else {
            assert!(typed.is_ok());
            good += 1;
        }
    }
    assert_eq!(bad, 28);
    assert_eq!(good, 1856);
}

fn local_weapon_catalog(data: Arc<GameDataSnapshot>, xml: &str) -> ControlledMaceCatalog {
    let example: serde_json::Value = serde_json::from_str(include_str!(
        "../../../examples/mace-local-weapon-search.json"
    ))
    .unwrap();
    let alternatives = serde_json::from_value(example["weapons"].clone()).unwrap();
    ControlledMaceCatalog::with_tree_loadouts(
        data.clone(),
        xml.into(),
        alternatives,
        loadouts(),
        class_tree::selections(data.tree()).unwrap(),
    )
    .unwrap()
}
fn local_weapon_matrix(data: Arc<GameDataSnapshot>, xml: &str) -> usize {
    let registry = local_weapon_catalog(data.clone(), xml);
    let backend = backend(data);
    let baseline = backend.calculate(&request(xml), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    assert_eq!(components.axis_counts(), [6, 105, 7]);
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    let footprint = prepared.footprint();
    assert_eq!(footprint.retained_xml_bytes, 0);
    assert_eq!(footprint.cached_candidate_results, 0);
    assert_eq!(footprint.deferred_weapon_errors, 0);
    let mut legal = 0;
    let mut level_rejections = 0;
    for alternative in registry.alternatives() {
        let handle = registry.validated_native_candidate(&alternative.candidate, &components);
        let requirements = registry.requirements(&alternative.candidate).unwrap();
        if alternative.weapon_id == "wooden-level-80" {
            assert_eq!(requirements.required.level, 80);
            assert_eq!(requirements.available.level, 60);
            assert!(!requirements.is_legal());
            level_rejections += 1;
        }
        if !requirements.is_legal() {
            assert!(handle.is_err());
            continue;
        }
        let handle = handle.unwrap();
        let req = request(
            &registry
                .materialize(&alternative.candidate)
                .unwrap()
                .content,
        );
        let result = backend.calculate(&req, BUDGET).unwrap();
        registry
            .validate_native_realization(&alternative.candidate, &result, &scenario)
            .unwrap();
        assert_eq!(result.exports[0].content, req.build.content);
        let snapshot = backend
            .evaluate_controlled_mace(&prepared, &handle, BUDGET)
            .unwrap();
        assert_measurements(
            &prepared.snapshot_measurements(&snapshot),
            &result.measurements,
        );
        let poe_optimizer_native::NativeCalculation::Mace(full) =
            backend.prepare(&req).unwrap().calculate().unwrap()
        else {
            panic!("Mace profile")
        };
        assert_eq!(prepared.calculate(&handle).unwrap(), full);
        legal += 1;
    }
    assert_eq!(level_rejections, 105 * 7);
    assert_eq!(registry.alternatives().len(), 4410);
    assert!(legal > 2000, "matrix must cover broad mixed axes: {legal}");
    assert_eq!(footprint.actor_components, 105);
    legal
}
#[test]
fn normal_and_rare_weapon_candidates_match_full_documents_across_all_class_support_axes() {
    local_weapon_matrix(Arc::new(game_data::bundled_snapshot().unwrap()), TEMPLATE);
}
#[test]
fn local_weapons_retain_injected_rounding_caps_damage_presence_and_support_values() {
    let data = custom(|package| {
        package.character.critical_chance_cap = 17.5;
        for weapon in &mut package.weapons {
            weapon.attack_rate = 1.235;
            weapon.physical_minimum = 0.0;
            weapon.physical_maximum = 12.49;
            weapon.fire_minimum = 0.0;
            weapon.fire_maximum = 2.5;
        }
        package
            .supports
            .iter_mut()
            .find(|s| s.id == "rapid_attacks_i")
            .unwrap()
            .modifiers[0]
            .value = 37.0;
    });
    let xml = TEMPLATE
        .replace(
            "enemyArmour\" number=\"0\"",
            "enemyArmour\" number=\"125.25\"",
        )
        .replace(
            "enemyFireResist\" number=\"0\"",
            "enemyFireResist\" number=\"-37.5\"",
        );
    assert_ne!(xml, TEMPLATE);
    local_weapon_matrix(data, &xml);
}
#[test]
fn mixed_local_weapon_and_actor_snapshots_remain_allocation_free() {
    let template = include_str!("../../../tests/fixtures/builds/mace-actor-resources.xml");
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = local_weapon_catalog(data.clone(), template);
    let backend = backend(data);
    let baseline = backend.calculate(&request(template), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    assert!(
        components
            .weapons()
            .iter()
            .any(|w| !w.local_modifiers().is_empty())
    );
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    let handles: Vec<_> = registry
        .alternatives()
        .iter()
        .filter_map(|a| {
            registry
                .validated_native_candidate(&a.candidate, &components)
                .ok()
        })
        .collect();
    let (_, allocations) = allocation_count(|| {
        for handle in handles.iter().cycle().take(8000) {
            black_box(prepared.calculate(black_box(handle)).unwrap());
            black_box(prepared.measure(black_box(handle)).unwrap());
            black_box(
                backend
                    .evaluate_controlled_mace(&prepared, black_box(handle), BUDGET)
                    .unwrap(),
            );
        }
    });
    assert_eq!(allocations, 0);
}

#[test]
fn actor_config_all_joint_axes_match_documents_with_injected_resources() {
    let xml = include_str!("../../../tests/fixtures/builds/mace-actor-resources.xml");
    assert_eq!(
        local_weapon_matrix(Arc::new(game_data::bundled_snapshot().unwrap()), xml),
        3675
    );
    let data = custom(|p| {
        p.character.life_per_strength = 2.75;
        if let poe_optimizer_data::game_data::ActorModifierEffect::Numeric { value, .. } =
            &mut p.actor.spirit_quests[0].modifiers[0].effect
        {
            *value += 7.0;
        } else {
            panic!("numeric Spirit quest");
        }
        p.actor
            .high_precision_mods
            .entry("Life".into())
            .or_default()
            .insert(
                poe_optimizer_data::game_data::ActorNumericOperation::More,
                2,
            );
    });
    let xml = xml.replace("+30 to Spirit", "+30 to Spirit\n1% more maximum Life\n1% more maximum Life\nGain no inherent bonuses from dexterity");
    assert_eq!(local_weapon_matrix(data, &xml), 3675);
}
