//! Contract tests use interchangeable pure-Rust backends; no PoB process or Lua is involved.
use poe_optimizer_core::{
    BuildSummary,
    coverage::{BuildCoverage, FullDpsCoverage},
    evaluation::*,
    metrics::*,
    options::*,
};
use std::{cell::Cell, collections::BTreeMap, rc::Rc};

const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 250 };

fn query(actor: ActorScope, id: &str) -> MetricQuery {
    MetricQuery {
        actor,
        id: id.into(),
    }
}

fn request() -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: "<PathOfBuilding2/>".into(),
        },
        options: EvaluationOptions {
            selection: Some(SkillSelection {
                socket_group: 2,
                active_skill: Some(1),
                minion_skill: Some(2),
            }),
            encounter: Some(EncounterOverrides {
                name: "controlled encounter".into(),
                enemy_level: Some(80),
                boss: Some(BossKind::Pinnacle),
                incoming_hit: Some(DamageAmounts {
                    physical: 1_000.0,
                    ..DamageAmounts::default()
                }),
            }),
        },
        metrics: vec![],
    }
}

fn catalog() -> Vec<MetricDefinition> {
    vec![
        MetricDefinition {
            id: "custom_pool".into(),
            unit: MetricUnit::PoolPoints,
            actors: vec![ActorScope::Player],
            description: "A backend-independent test pool".into(),
            schema_version: 1,
        },
        MetricDefinition {
            id: "custom_hit".into(),
            unit: MetricUnit::Damage,
            actors: vec![ActorScope::Player, ActorScope::SelectedMinion],
            description: "A backend-independent test hit".into(),
            schema_version: 2,
        },
    ]
}

fn measurements() -> Vec<MetricMeasurement> {
    catalog()
        .into_iter()
        .flat_map(|definition| {
            definition
                .actors
                .into_iter()
                .map(move |actor| MetricMeasurement {
                    query: query(actor, &definition.id),
                    unit: definition.unit,
                    value: if actor == ActorScope::SelectedMinion {
                        MeasurementValue::Unavailable {
                            reason: "No test minion exists".into(),
                        }
                    } else {
                        MeasurementValue::Finite { value: 100.0 }
                    },
                    schema_version: definition.schema_version,
                })
        })
        .collect()
}

fn result(id: &str, options: EvaluationOptions) -> EvaluationResult {
    EvaluationResult {
        backend: BackendIdentity {
            id: id.into(),
            implementation_version: "test-1".into(),
            rules_revision: "test-rules".into(),
            source_fingerprint: "test-source".into(),
            adapter_fingerprint: "test-adapter".into(),
        },
        build: BuildSummary {
            level: 80,
            class_name: "Test class".into(),
            ascendancy_name: "None".into(),
            tree_version: "test-tree".into(),
            main_socket_group: 2,
            allocated_nodes: vec![],
            skill_groups: 0,
        },
        context: EvaluationContext {
            requested: options,
            calculation_mode: "test".into(),
            enemy_level: 80,
            config_inputs: BTreeMap::from([("external_input".into(), Scalar::Number(1.0))]),
            config_placeholders: BTreeMap::new(),
            player_conditions: BTreeMap::new(),
            enemy_conditions: BTreeMap::new(),
        },
        coverage: BuildCoverage {
            passives: None,
            schema_version: 1,
            active_skill_set_id: None,
            groups: vec![],
            selected_player: None,
            selected_minion: None,
            full_dps: FullDpsCoverage {
                included_group_count: 0,
                selected_group_included: false,
                active_skills: vec![],
                reported_contributions: vec![],
            },
            unresolved_entry_count: 0,
            tree_connections: vec![],
        },
        measurements: measurements(),
        exports: vec![],
        warnings: vec![],
        elapsed_ms: 0.25,
        diagnostic_only: true,
        attachments: vec![],
    }
}

struct FakeBackend {
    capabilities: BackendCapabilities,
    calls: Rc<Cell<usize>>,
    alter: Box<dyn Fn(&mut EvaluationResult)>,
    failure: Option<EvaluationErrorKind>,
}
impl FakeBackend {
    fn new(id: &str) -> Self {
        Self {
            capabilities: BackendCapabilities {
                id: id.into(),
                build_formats: vec![BuildFormat::PathOfBuilding2Xml],
                metrics: catalog(),
                full_build_evaluation: true,
                skill_selection: true,
                encounter_overrides: true,
            },
            calls: Rc::new(Cell::new(0)),
            alter: Box::new(|_| {}),
            failure: None,
        }
    }
    fn altering(alter: impl Fn(&mut EvaluationResult) + 'static) -> Self {
        Self {
            alter: Box::new(alter),
            ..Self::new("test")
        }
    }
}
impl CalculationBackend for FakeBackend {
    fn capabilities(&self) -> BackendCapabilities {
        self.capabilities.clone()
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        budget: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        self.calls.set(self.calls.get() + 1);
        assert_eq!(budget.timeout_ms, BUDGET.timeout_ms);
        assert_eq!(request.build.content, "<PathOfBuilding2/>");
        if let Some(kind) = self.failure {
            return Err(EvaluationError::new(kind, "typed backend failure"));
        }
        let mut result = result(&self.capabilities.id, request.options.clone());
        (self.alter)(&mut result);
        Ok(result)
    }
}

fn assert_contract(alter: impl Fn(&mut EvaluationResult) + 'static) {
    let error = Engine::new(FakeBackend::altering(alter))
        .evaluate(&request(), BUDGET)
        .unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
}

#[test]
fn boxed_backends_share_the_same_engine_contract_and_custom_catalog() {
    for id in ["native-test", "browser-test"] {
        let backend: Box<dyn CalculationBackend> = Box::new(FakeBackend::new(id));
        let engine = Engine::new(backend);
        let result = engine.evaluate(&request(), BUDGET).unwrap();
        assert_eq!(result.backend.id, id);
        assert_eq!(result.context.requested, request().options);
        assert_eq!(result.measurements.len(), 3);
        assert!(matches!(
            result.measurements[2].value,
            MeasurementValue::Unavailable { .. }
        ));
        let json = serde_json::to_string(&result).unwrap();
        let decoded: EvaluationResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.measurements.len(), 3);
    }
}

#[test]
fn unfiltered_request_requires_each_catalog_actor_and_metric() {
    assert_contract(|result| result.measurements.clear());
    assert_contract(|result| {
        result.measurements.pop();
    });
}

#[test]
fn filtered_request_requires_its_measurements_and_removes_declared_extras() {
    let mut request = request();
    request.metrics = vec![query(ActorScope::Player, "custom_pool")];
    let result = Engine::new(FakeBackend::new("test"))
        .evaluate(&request, BUDGET)
        .unwrap();
    assert_eq!(result.measurements.len(), 1);
    assert_eq!(result.measurements[0].query, request.metrics[0]);
    let error = Engine::new(FakeBackend::altering(|result| {
        result.measurements.remove(0);
    }))
    .evaluate(&request, BUDGET)
    .unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
}

#[test]
fn explicit_unavailability_and_nonfinite_classifications_are_complete_results() {
    let engine = Engine::new(FakeBackend::altering(|result| {
        result.measurements[0].value = MeasurementValue::NonFinite {
            kind: NonFiniteKind::PositiveInfinity,
        };
        result.measurements[1].value = MeasurementValue::Unavailable {
            reason: "Not applicable to this candidate".into(),
        };
    }));
    let result = engine.evaluate(&request(), BUDGET).unwrap();
    assert_eq!(result.measurements.len(), 3);
    assert!(
        result
            .measurements
            .iter()
            .all(|metric| metric.value.finite().is_none())
    );
}

#[test]
fn stale_option_echo_or_different_backend_identity_is_rejected() {
    assert_contract(|result| result.context.requested = EvaluationOptions::default());
    assert_contract(|result| {
        result
            .context
            .requested
            .selection
            .as_mut()
            .unwrap()
            .minion_skill = Some(1);
    });
    assert_contract(|result| {
        result.context.requested.encounter.as_mut().unwrap().boss = Some(BossKind::Normal);
    });
    assert_contract(|result| result.backend.id = "another-backend".into());
}

#[test]
fn elapsed_time_and_context_numbers_must_remain_json_roundtrippable() {
    for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_contract(move |result| result.elapsed_ms = value);
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_contract(move |result| {
            result
                .context
                .config_inputs
                .insert("bad".into(), Scalar::Number(value));
        });
        assert_contract(move |result| {
            result
                .context
                .config_placeholders
                .insert("bad".into(), Scalar::Number(value));
        });
    }
    let result = Engine::new(FakeBackend::altering(|result| result.elapsed_ms = 0.0))
        .evaluate(&request(), BUDGET)
        .unwrap();
    assert_eq!(result.elapsed_ms, 0.0);
}

#[test]
fn duplicate_undeclared_mistyped_or_invalid_measurements_are_rejected() {
    assert_contract(|result| result.measurements.push(result.measurements[0].clone()));
    assert_contract(|result| result.measurements[0].query.id = "unadvertised".into());
    assert_contract(|result| result.measurements[0].query.actor = ActorScope::SelectedMinion);
    assert_contract(|result| result.measurements[0].unit = MetricUnit::DamagePerSecond);
    assert_contract(|result| result.measurements[0].schema_version += 1);
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_contract(move |result| {
            result.measurements[0].value = MeasurementValue::Finite { value };
        });
    }
}

#[test]
fn bad_queries_options_and_budgets_fail_before_backend_execution() {
    let backend = FakeBackend::new("test");
    let calls = backend.calls.clone();
    let engine = Engine::new(backend);
    let mut request = request();
    request.metrics = vec![query(ActorScope::Player, "unknown")];
    assert_eq!(
        engine.evaluate(&request, BUDGET).unwrap_err().kind,
        EvaluationErrorKind::UnsupportedCapability
    );
    request.metrics = vec![query(ActorScope::SelectedMinion, "custom_pool")];
    assert_eq!(
        engine.evaluate(&request, BUDGET).unwrap_err().kind,
        EvaluationErrorKind::UnsupportedCapability
    );
    request.metrics = vec![query(ActorScope::Player, "custom_pool"); 2];
    assert_eq!(
        engine.evaluate(&request, BUDGET).unwrap_err().kind,
        EvaluationErrorKind::InvalidRequest
    );
    request.metrics.clear();
    assert_eq!(
        engine
            .evaluate(&request, EvaluationBudget { timeout_ms: 0 })
            .unwrap_err()
            .kind,
        EvaluationErrorKind::InvalidRequest
    );
    request.options.selection.as_mut().unwrap().socket_group = 0;
    assert_eq!(
        engine.evaluate(&request, BUDGET).unwrap_err().kind,
        EvaluationErrorKind::InvalidRequest
    );
    assert_eq!(calls.get(), 0);
}

#[test]
fn unsupported_backend_capabilities_fail_before_execution() {
    for unavailable in 0..4 {
        let mut backend = FakeBackend::new("test");
        match unavailable {
            0 => backend.capabilities.full_build_evaluation = false,
            1 => backend.capabilities.build_formats.clear(),
            2 => backend.capabilities.skill_selection = false,
            _ => backend.capabilities.encounter_overrides = false,
        }
        let calls = backend.calls.clone();
        let error = Engine::new(backend)
            .evaluate(&request(), BUDGET)
            .unwrap_err();
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn backend_failures_keep_their_classification_including_timeout() {
    for kind in [
        EvaluationErrorKind::Timeout,
        EvaluationErrorKind::InvalidRequest,
        EvaluationErrorKind::UnsupportedCapability,
        EvaluationErrorKind::CalculationFailed,
        EvaluationErrorKind::BackendContract,
    ] {
        let backend = FakeBackend {
            failure: Some(kind),
            ..FakeBackend::new("test")
        };
        let error = Engine::new(backend)
            .evaluate(&request(), BUDGET)
            .unwrap_err();
        assert_eq!(error.kind, kind);
        assert_eq!(error.message, "typed backend failure");
        let json = serde_json::to_string(&error).unwrap();
        let decoded: EvaluationError = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.kind, kind);
    }
}

#[test]
fn unknown_request_fields_are_rejected_instead_of_ignored() {
    let original = serde_json::to_value(request()).unwrap();
    for section in [
        "request",
        "build",
        "options",
        "selection",
        "encounter",
        "incoming_hit",
    ] {
        let mut value = original.clone();
        let target = match section {
            "request" => &mut value,
            "build" => &mut value["build"],
            "options" => &mut value["options"],
            "selection" => &mut value["options"]["selection"],
            "encounter" => &mut value["options"]["encounter"],
            _ => &mut value["options"]["encounter"]["incoming_hit"],
        };
        target["misspelled_option"] = serde_json::json!(true);
        assert!(
            serde_json::from_value::<EvaluationRequest>(value).is_err(),
            "{section}"
        );
    }
    let mut value = original;
    value["metrics"] = serde_json::json!([{
        "actor": "player", "id": "custom_pool", "misspelled_option": true
    }]);
    assert!(serde_json::from_value::<EvaluationRequest>(value).is_err());
}
