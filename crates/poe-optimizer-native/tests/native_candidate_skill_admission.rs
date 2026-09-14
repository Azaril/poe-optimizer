//! Candidate support axes must share authored-loader admission with documents.
use poe_optimizer_core::{
    build_identity::BuildLineage, candidate::*, evaluation::*, options::EvaluationOptions,
};
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, CompiledGameData, HostClock, NativeBackend};
use std::sync::{Arc, OnceLock};
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 30_000 };
fn backend(mutation: &str) -> NativeBackend {
    static HIDDEN: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    static LEVEL: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    let cache = match mutation {
        "hidden" => &HIDDEN,
        "level" => &LEVEL,
        _ => unreachable!(),
    };
    let data = cache.get_or_init(|| {
        let mut package = bundled_snapshot().unwrap().package().clone();
        let effect = package
            .skill_preparation
            .effects
            .iter_mut()
            .find(|effect| effect.id == "SupportBrutalityPlayer")
            .unwrap();
        match mutation {
            "hidden" => effect.hide_from_sidebar = Some(true),
            "level" => {
                let mut row = effect.levels[0].clone();
                row.key = 2.0;
                effect.levels = vec![row];
                effect.levels_length = 0;
                effect.next_level_key = Some(2.0);
            }
            _ => unreachable!(),
        }
        package.refresh_section_digests().unwrap();
        let snapshot = GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap();
        Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap())
    });
    NativeBackend::with_data(Arc::clone(data), HostClock).unwrap()
}
fn request(build: BuildDocument) -> EvaluationRequest {
    EvaluationRequest {
        build,
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn check_controlled_build(mutation: &str) {
    let backend = backend(mutation);
    let catalog =
        Arc::new(ControlledBuildCatalog::new(backend.data().clone(), MACE.into(), vec![]).unwrap());
    let domain = ControlledBuildDomain::new(
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
    .unwrap();
    let prepared = backend
        .prepare_controlled_build_with_lineage(
            domain.catalog(),
            &[],
            BuildLineage::from_bytes([95; 16]),
        )
        .expect("an invalid support axis must not abort unaffected candidates");
    assert_eq!(prepared.footprint().deferred_support_errors, 1);
    let empty = domain
        .admit(
            domain.catalog().source_selection(),
            &mut ActorScratch::default(),
        )
        .unwrap();
    let full = backend
        .calculate(&request(domain.materialize(&empty).unwrap()), BUDGET)
        .unwrap();
    let typed = prepared.measure(&empty).unwrap();
    assert_eq!(
        serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
        serde_json::to_value(&full.measurements).unwrap()
    );
    let mut selection = domain.catalog().source_selection();
    let support = domain
        .catalog()
        .support_instance("brutality_i")
        .unwrap()
        .to_owned();
    selection
        .candidate
        .skills
        .values_mut()
        .next()
        .unwrap()
        .support_instance_ids = [support].into_iter().collect();
    let handle = domain
        .admit(selection, &mut ActorScratch::default())
        .unwrap();
    let full = backend
        .prepare_with_lineage(
            &request(domain.materialize(&handle).unwrap()),
            BuildLineage::from_bytes([94; 16]),
        )
        .err()
        .expect("fresh document rejects the source-hidden support");
    let typed = prepared.measure(&handle).expect_err(
        "typed candidate must reject a hidden support which shared authored loading rejects",
    );
    assert_eq!(typed.kind, full.kind);
    assert_eq!(typed.message, full.message);
    assert!(
        prepared.measure(&empty).is_ok(),
        "failed support axis must not poison later valid candidates"
    );
}
#[test]
fn support_added_after_template_preparation_cannot_bypass_authored_loading() {
    for mutation in ["hidden", "level"] {
        check_controlled_build(mutation);
    }
}
