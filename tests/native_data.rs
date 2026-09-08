//! Data selection is immutable state: prepared values and result identity must
//! never bleed between datasets or depend on process-global default selection.
use poe_optimizer_core::{evaluation::*, options::*};
use poe_optimizer_data::game_data::{
    self, GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy,
};
use poe_optimizer_native::{CompiledGameData, EvaluationClock, NativeBackend, NativeCalculation};
use std::{collections::BTreeMap, sync::Arc, time::Duration};

const XML: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 10_000 };
#[derive(Clone, Copy)]
struct FixedClock;
impl EvaluationClock for FixedClock {
    fn now(&self) -> Duration {
        Duration::ZERO
    }
}
fn request() -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: XML.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn custom(mut change: impl FnMut(&mut GameDataPackage)) -> Arc<CompiledGameData> {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    change(&mut package);
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap())
}
fn fresh_reviewed() -> Arc<CompiledGameData> {
    Arc::new(CompiledGameData::compile(Arc::new(game_data::bundled_snapshot().unwrap())).unwrap())
}
fn backend(data: Arc<CompiledGameData>) -> NativeBackend<FixedClock> {
    NativeBackend::with_data(data, FixedClock).unwrap()
}
fn values(result: &EvaluationResult) -> BTreeMap<String, f64> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.clone(), m.value.finite().unwrap()))
        .collect()
}
fn dps(calculation: NativeCalculation) -> f64 {
    match calculation {
        NativeCalculation::Spark(output) => output.hit_dps,
        NativeCalculation::Mace(_) => panic!("Unexpected selected profile"),
    }
}

#[test]
fn independent_data_instances_change_only_the_intended_spark_values_and_keep_a_b_a_isolated() {
    let a = Arc::new(backend(fresh_reviewed()));
    let b = Arc::new(backend(custom(|package| {
        package.spark.lightning_maximum += 7.0
    })));
    let request = request();
    let pa = Arc::new(a.prepare(&request).unwrap());
    let pb = Arc::new(b.prepare(&request).unwrap());
    let before = a.evaluate_prepared(&pa, BUDGET).unwrap();
    let changed = b.evaluate_prepared(&pb, BUDGET).unwrap();
    assert_ne!(a.identity(), b.identity());
    assert_eq!(
        a.identity().adapter_fingerprint,
        b.identity().adapter_fingerprint
    );
    assert_eq!(
        a.identity().source_fingerprint,
        b.identity().source_fingerprint
    );
    assert_ne!(before.backend.data, changed.backend.data);
    assert_eq!(pa.data_identity(), a.data().identity());
    assert_eq!(pb.data_identity(), b.data().identity());
    let av = values(&before);
    let bv = values(&changed);
    assert!(bv["selected_hit_dps"] > av["selected_hit_dps"]);
    assert!(bv["selected_average_hit"] > av["selected_average_hit"]);
    for (key, value) in &av {
        if !["selected_hit_dps", "selected_average_hit"].contains(&key.as_str()) {
            assert_eq!(*value, bv[key], "unrelated value {key}");
        }
    }
    assert_eq!(av, values(&a.evaluate_prepared(&pa, BUDGET).unwrap()));
    assert_eq!(
        before.backend,
        a.evaluate_prepared(&pa, BUDGET).unwrap().backend
    );
    // Both pre-parsed pipelines execute concurrently on shared immutable data,
    // and the original A result stays stable after every B calculation.
    std::thread::scope(|scope| {
        for index in 0..4 {
            let (backend, prepared, expected) = if index % 2 == 0 {
                (Arc::clone(&a), Arc::clone(&pa), av["selected_hit_dps"])
            } else {
                (Arc::clone(&b), Arc::clone(&pb), bv["selected_hit_dps"])
            };
            scope.spawn(move || {
                for _ in 0..128 {
                    assert_eq!(dps(prepared.calculate().unwrap()), expected);
                    let result = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
                    assert_eq!(values(&result)["selected_hit_dps"], expected);
                    assert_eq!(
                        result.backend.data.as_ref().unwrap(),
                        prepared.data_identity()
                    );
                }
            });
        }
    });
    assert_eq!(av, values(&a.evaluate_prepared(&pa, BUDGET).unwrap()));
    // Serialization records content identity; no pointer or hidden singleton is
    // needed to replay the result's declared dataset selection.
    let recorded: EvaluationResult =
        serde_json::from_slice(&serde_json::to_vec(&changed).unwrap()).unwrap();
    assert_eq!(recorded.backend, changed.backend);
    recorded.validate_recorded().unwrap();
}

#[test]
fn prepared_values_accept_equal_content_from_a_separate_instance_and_reject_different_content() {
    let original = backend(fresh_reviewed());
    let equivalent = backend(fresh_reviewed());
    assert!(!Arc::ptr_eq(original.data(), equivalent.data()));
    assert_eq!(original.identity(), equivalent.identity());
    let prepared = original.prepare(&request()).unwrap();
    let expected = original.evaluate_prepared(&prepared, BUDGET).unwrap();
    assert_eq!(
        values(&expected),
        values(&equivalent.evaluate_prepared(&prepared, BUDGET).unwrap())
    );
    let different = backend(custom(|package| package.spark.cast_time *= 2.0));
    let error = different.evaluate_prepared(&prepared, BUDGET).unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
    assert_eq!(
        values(&expected),
        values(&original.evaluate_prepared(&prepared, BUDGET).unwrap())
    );
    // Equal bytes loaded as unreviewed have the same content identity. Trust is
    // separate evidence and must reflect the accepting backend's load policy.
    let custom_same = backend(custom(|_| {}));
    assert_eq!(original.identity(), custom_same.identity());
    let result = custom_same.evaluate_prepared(&prepared, BUDGET).unwrap();
    let evidence = result
        .attachments
        .iter()
        .find(|a| {
            a.media_type
                .starts_with("application/vnd.poe-optimizer.game-data+")
        })
        .unwrap();
    let evidence: serde_json::Value = serde_json::from_str(&evidence.content).unwrap();
    assert_eq!(evidence["trust"]["status"], "custom_unreviewed");
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.to_lowercase().contains("unreviewed"))
    );
}

struct WrongDataset {
    advertised: NativeBackend<FixedClock>,
    actual: NativeBackend<FixedClock>,
}
impl CalculationBackend for WrongDataset {
    fn identity(&self) -> Option<BackendIdentity> {
        Some(self.advertised.identity())
    }
    fn capabilities(&self) -> BackendCapabilities {
        self.advertised.capabilities()
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        self.actual.calculate(request, budget)
    }
}
#[test]
fn shared_engine_rejects_a_backend_that_returns_another_data_identity() {
    let dishonest = WrongDataset {
        advertised: backend(fresh_reviewed()),
        actual: backend(custom(|package| package.spark.lightning_maximum += 1.0)),
    };
    let boxed: Box<dyn CalculationBackend> = Box::new(dishonest);
    let error = Engine::new(boxed).evaluate(&request(), BUDGET).unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
    assert!(error.message.contains("identity"));
}
