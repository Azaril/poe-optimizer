//! Recorded results are validated without a PoB catalog, source tree, or worker process.
use poe_optimizer_core::{
    evaluation::*,
    metrics::*,
    options::{EvaluationOptions, Scalar},
};
use serde_json::json;

fn recorded() -> EvaluationResult {
    serde_json::from_value(json!({
        "backend": {
            "id": "native-test", "implementation_version": "test",
            "rules_revision": "custom-rules", "source_fingerprint": "recorded-source",
            "adapter_fingerprint": "recorded-adapter"
        },
        "build": {
            "level": 80, "class_name": "Test", "ascendancy_name": "None",
            "tree_version": "custom-tree", "main_socket_group": 1,
            "allocated_nodes": [], "skill_groups": 1
        },
        "context": {
            "requested": {
                "selection": { "socket_group": 1, "active_skill": 1 },
                "encounter": {
                    "name": "test", "enemy_level": 80,
                    "incoming_hit": {
                        "physical": 1000.0, "fire": 0.0, "cold": 0.0,
                        "lightning": 0.0, "chaos": 0.0
                    }
                }
            },
            "calculation_mode": "custom", "enemy_level": 80,
            "config_inputs": { "number": 1.0, "flag": true, "text": "observed" },
            "config_placeholders": {}, "player_conditions": {}, "enemy_conditions": {}
        },
        "coverage": {
            "schema_version": 1, "active_skill_set_id": 1,
            "groups": [{
                "index": 1, "label": "test", "enabled": true, "slot": null,
                "provenance": { "kind": "manual" },
                "include_in_full_dps": true, "group_count": 1.0,
                "main_active_skill": 1,
                "gems": [{
                    "index": 1, "name": "Test skill", "enabled": true,
                    "count": 2.0, "level": 1.0, "quality": 20.0,
                    "resolution": "resolved_gem", "hint": "none",
                    "related_candidates": []
                }]
            }],
            "selected_player": null, "selected_minion": null,
            "full_dps": {
                "included_group_count": 1, "selected_group_included": true,
                "active_skills": [{ "included": true, "count": 2.0, "count_enabled": true }],
                "reported_contributions": [{ "dps": 100.0, "count": 2.0 }]
            },
            "unresolved_entry_count": 0, "tree_connections": []
        },
        "measurements": [{
            "query": { "actor": "player", "id": "arbitrary_metric" },
            "unit": "damage", "value": { "status": "finite", "value": 100.0 },
            "schema_version": 83
        }, {
            "query": { "actor": "selected_minion", "id": "arbitrary_metric" },
            "unit": "damage", "value": { "status": "unavailable", "reason": "No minion" },
            "schema_version": 83
        }],
        "exports": [], "warnings": [], "elapsed_ms": 1.5,
        "diagnostic_only": true, "attachments": []
    }))
    .unwrap()
}

fn assert_invalid(result: &EvaluationResult, expected_message: &str) {
    let error = result.validate_recorded().unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
    assert!(error.message.contains(expected_message), "{error}");
}

#[test]
fn custom_catalog_versions_and_classified_availability_survive_json_round_trip() {
    for value in [
        MeasurementValue::Finite { value: -f64::MAX },
        MeasurementValue::Finite { value: 0.0 },
        MeasurementValue::Finite { value: f64::MAX },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::PositiveInfinity,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NegativeInfinity,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NotANumber,
        },
        MeasurementValue::Unavailable {
            reason: "Unmodeled action".into(),
        },
    ] {
        let mut result = recorded();
        result.measurements[0].value = value.clone();
        result.validate_recorded().unwrap();
        let decoded: EvaluationResult =
            serde_json::from_str(&serde_json::to_string(&result).unwrap()).unwrap();
        decoded.validate_recorded().unwrap();
        assert_eq!(decoded.measurements[0].schema_version, 83);
        assert_eq!(decoded.measurements[0].value, value);
    }
}

#[test]
fn every_query_is_unique_even_when_it_would_be_unused_by_an_objective() {
    let mut result = recorded();
    // The same metric ID for different actors is a distinct query.
    result.validate_recorded().unwrap();
    result.measurements.push(result.measurements[1].clone());
    assert_invalid(&result, "Duplicate recorded measurement");
}

#[test]
fn all_metric_ids_and_schema_versions_are_structurally_valid() {
    for id in ["", " \t\n"] {
        let mut result = recorded();
        result.measurements[1].query.id = id.into();
        assert_invalid(&result, "nonblank");
    }
    let mut result = recorded();
    result.measurements[1].schema_version = 0;
    assert_invalid(&result, "schema versions must be positive");
}

#[test]
fn finite_tags_cannot_hide_nonfinite_numbers() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for index in 0..2 {
            let mut result = recorded();
            result.measurements[index].value = MeasurementValue::Finite { value };
            assert_invalid(&result, "finite measurement");
        }
    }
}

#[test]
fn elapsed_time_and_both_context_scalar_maps_are_checked() {
    for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut result = recorded();
        result.elapsed_ms = value;
        assert_invalid(&result, "elapsed time");
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for placeholders in [false, true] {
            let mut result = recorded();
            let map = if placeholders {
                &mut result.context.config_placeholders
            } else {
                &mut result.context.config_inputs
            };
            map.insert("bad".into(), Scalar::Number(value));
            assert_invalid(&result, "nonfinite number");
        }
    }
    let mut result = recorded();
    result.elapsed_ms = 0.0;
    result.validate_recorded().unwrap();
}

#[test]
fn recorded_options_are_validated_without_executing_them() {
    let mut result = recorded();
    result
        .context
        .requested
        .selection
        .as_mut()
        .unwrap()
        .socket_group = 0;
    assert_invalid(&result, "Recorded evaluation options");
    let mut result = recorded();
    result
        .context
        .requested
        .encounter
        .as_mut()
        .unwrap()
        .enemy_level = Some(0);
    assert_invalid(&result, "Recorded evaluation options");
    for value in [-1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let mut result = recorded();
        result
            .context
            .requested
            .encounter
            .as_mut()
            .unwrap()
            .incoming_hit
            .as_mut()
            .unwrap()
            .physical = value;
        assert_invalid(&result, "Recorded evaluation options");
    }
}

#[test]
fn every_optional_coverage_number_must_be_finite_when_present() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        for field in 0..7 {
            let mut result = recorded();
            let number = match field {
                0 => &mut result.coverage.groups[0].group_count,
                1 => &mut result.coverage.groups[0].gems[0].count,
                2 => &mut result.coverage.groups[0].gems[0].level,
                3 => &mut result.coverage.groups[0].gems[0].quality,
                4 => &mut result.coverage.full_dps.active_skills[0].count,
                5 => &mut result.coverage.full_dps.reported_contributions[0].count,
                _ => &mut result.coverage.full_dps.reported_contributions[0].dps,
            };
            *number = Some(value);
            assert_invalid(&result, "coverage.");
            // Absent evidence remains representable, rather than becoming a numeric zero.
            match field {
                0 => result.coverage.groups[0].group_count = None,
                1 => result.coverage.groups[0].gems[0].count = None,
                2 => result.coverage.groups[0].gems[0].level = None,
                3 => result.coverage.groups[0].gems[0].quality = None,
                4 => result.coverage.full_dps.active_skills[0].count = None,
                5 => result.coverage.full_dps.reported_contributions[0].count = None,
                _ => result.coverage.full_dps.reported_contributions[0].dps = None,
            }
            result.validate_recorded().unwrap();
        }
    }
}

#[test]
fn unsupported_coverage_versions_require_an_explicit_reader() {
    for version in [0, 2, u32::MAX] {
        let mut result = recorded();
        result.coverage.schema_version = version;
        assert_invalid(&result, "coverage schema version");
    }
}

struct CorruptCoverageBackend;
impl CalculationBackend for CorruptCoverageBackend {
    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            id: "native-test".into(),
            build_formats: vec![BuildFormat::PathOfBuilding2Xml],
            metrics: vec![MetricDefinition {
                id: "arbitrary_metric".into(),
                unit: MetricUnit::Damage,
                actors: vec![ActorScope::Player],
                description: "Test only".into(),
                schema_version: 83,
            }],
            full_build_evaluation: true,
            skill_selection: true,
            encounter_overrides: true,
        }
    }
    fn calculate(
        &self,
        request: &EvaluationRequest,
        _: EvaluationBudget,
    ) -> Result<EvaluationResult, EvaluationError> {
        let mut result = recorded();
        result.context.requested = request.options.clone();
        result.measurements.truncate(1);
        result.coverage.full_dps.reported_contributions[0].dps = Some(f64::NAN);
        Ok(result)
    }
}

#[test]
fn live_engine_also_rejects_corrupt_coverage_before_returning_measurements() {
    let request = EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: "<PathOfBuilding2/>".into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    };
    let error = Engine::new(CorruptCoverageBackend)
        .evaluate(&request, EvaluationBudget { timeout_ms: 100 })
        .unwrap_err();
    assert_eq!(error.kind, EvaluationErrorKind::BackendContract);
    assert!(error.message.contains("reported_contributions.dps"));
}
