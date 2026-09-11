//! Candidate support axes must share authored-loader admission with documents.
use poe_optimizer_core::{
    build_identity::BuildLineage, candidate::*, evaluation::*, options::EvaluationOptions,
};
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot};
use poe_optimizer_import::{
    controlled_build::*,
    controlled_mace::{ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative},
};
use poe_optimizer_native::{ActorScratch, CompiledGameData, HostClock, NativeBackend};
use std::sync::{Arc, OnceLock};
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
const TEMPLATE: &str = include_str!("../../../tests/fixtures/calibration/mace-smithing.xml");
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
fn check_controlled_mace(mutation: &str) {
    let backend = backend(mutation);
    let snapshot = backend.data().snapshot().clone();
    let weapons = snapshot
        .package()
        .weapons
        .iter()
        .map(|weapon| NormalMaceAlternative {
            id: weapon.id.clone(),
            item_text: format!(
                "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: 0\nImplicits: 0",
                weapon.name
            ),
        })
        .collect();
    let registry = ControlledMaceCatalog::with_data(
        Arc::new(snapshot),
        TEMPLATE.into(),
        weapons,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
    )
    .unwrap();
    let baseline = backend
        .calculate(&request(registry.template_build()), BUDGET)
        .unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = registry
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let prepared = backend
        .prepare_controlled_mace_with_lineage(&components, &[], BuildLineage::from_bytes([96; 16]))
        .expect("one invalid support axis must preserve the valid empty loadout");
    assert_eq!(prepared.footprint().deferred_support_errors, 1);
    let mut valid = 0;
    let mut rejected = 0;
    for alternative in registry.alternatives() {
        let handle = registry
            .validated_native_candidate(&alternative.candidate, &components)
            .unwrap();
        let request = request(registry.materialize(&alternative.candidate).unwrap());
        if alternative.support.keys().is_empty() {
            let full = backend.calculate(&request, BUDGET).unwrap();
            let typed = prepared.measure(&handle).unwrap();
            assert_eq!(
                serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
                serde_json::to_value(&full.measurements).unwrap()
            );
            valid += 1;
        } else {
            let full = backend
                .prepare_with_lineage(&request, BuildLineage::from_bytes([97; 16]))
                .err()
                .expect("full document rejects hidden support");
            let typed = prepared
                .measure(&handle)
                .expect_err("typed controlled-Mace support axis must use authored admission");
            assert_eq!(typed.kind, full.kind);
            assert_eq!(typed.message, full.message);
            rejected += 1;
        }
    }
    assert_eq!((valid, rejected), (2, 2));
}

#[test]
fn support_added_after_template_preparation_cannot_bypass_authored_loading() {
    for mutation in ["hidden", "level"] {
        check_controlled_build(mutation);
    }
}
#[test]
fn controlled_mace_loadout_axes_reject_changed_support_without_rejecting_empty_loadout() {
    for mutation in ["hidden", "level"] {
        check_controlled_mace(mutation);
    }
}
