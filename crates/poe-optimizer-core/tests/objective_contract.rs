use poe_optimizer_core::{metrics::*, objective::*};
fn query(id: &str) -> MetricQuery {
    MetricQuery {
        actor: ActorScope::Player,
        id: id.into(),
    }
}
fn catalog() -> Vec<MetricDefinition> {
    ["custom.cost", "custom.bound"]
        .into_iter()
        .map(|id| MetricDefinition {
            id: id.into(),
            unit: MetricUnit::Percent,
            actors: vec![ActorScope::Player],
            description: "Synthetic metric with no PoB dependency".into(),
            schema_version: 7,
        })
        .collect()
}
fn spec() -> ObjectiveSpec {
    ObjectiveSpec {
        schema_version: 1,
        objective: ObjectivePolicy::Scalar {
            metric: query("custom.cost"),
            unit: MetricUnit::Percent,
            direction: Direction::Minimize,
        },
        constraints: vec![],
    }
}
fn constraint(id: &str, operator: Comparison, threshold: f64) -> MetricConstraint {
    MetricConstraint {
        id: id.into(),
        metric: query("custom.bound"),
        unit: MetricUnit::Percent,
        operator,
        threshold,
        violation_scale: 10.0,
    }
}
fn measurement(id: &str, value: MeasurementValue) -> MetricMeasurement {
    MetricMeasurement {
        query: query(id),
        unit: MetricUnit::Percent,
        value,
        schema_version: 7,
    }
}
fn values(bound: f64) -> Vec<MetricMeasurement> {
    vec![
        measurement("custom.cost", MeasurementValue::from_number(42.0)),
        measurement("custom.bound", MeasurementValue::from_number(bound)),
    ]
}

#[test]
fn custom_catalog_minimization_and_no_implicit_constraints() {
    let compiled = spec().compile(&catalog()).unwrap();
    let policy: &dyn ScoringPolicy = &compiled;
    let result = policy.assess(&values(0.0)).unwrap();
    assert_eq!(result.status, AssessmentStatus::ConstraintsSatisfied);
    assert_eq!(result.objective_value.finite(), Some(42.0));
    assert_eq!(result.objective_score.finite(), Some(-42.0));
    assert!(result.constraints.is_empty());
    assert_eq!(result.total_normalized_violation.finite(), Some(0.0));
    assert_eq!(policy.required_metrics(), vec![query("custom.cost")]);
}

#[test]
fn exact_strict_and_inclusive_bounds_do_not_use_parity_tolerance() {
    for operator in [
        Comparison::AtLeast,
        Comparison::GreaterThan,
        Comparison::AtMost,
        Comparison::LessThan,
    ] {
        let mut spec = spec();
        spec.constraints
            .push(constraint("boundary", operator, 75.0));
        let policy = spec.compile(&catalog()).unwrap();
        for value in [75_f64.next_down(), 75.0, 75_f64.next_up()] {
            let pass = match operator {
                Comparison::AtLeast => value >= 75.0,
                Comparison::GreaterThan => value > 75.0,
                Comparison::AtMost => value <= 75.0,
                Comparison::LessThan => value < 75.0,
            };
            let result = policy.assess(&values(value)).unwrap();
            assert_eq!(
                result.status == AssessmentStatus::ConstraintsSatisfied,
                pass,
                "{operator:?} {value:?}"
            );
            assert_eq!(
                result.constraints[0].strict_boundary,
                !pass && value == 75.0
            );
            if value == 75.0 {
                assert_eq!(
                    result.constraints[0].shortfall.as_ref().unwrap().finite(),
                    Some(0.0)
                );
            }
        }
    }
}

#[test]
fn contradictory_bounds_are_checked_by_metric_not_label() {
    for lower in [Comparison::AtLeast, Comparison::GreaterThan] {
        for upper in [Comparison::AtMost, Comparison::LessThan] {
            for (lo, hi) in [(75.0, 74.0), (75.0, 75.0), (75.0, 76.0)] {
                let mut spec = spec();
                spec.constraints = vec![
                    constraint("lower", lower, lo),
                    constraint("upper", upper, hi),
                ];
                let contradictory = lo > hi
                    || (lo == hi
                        && (lower == Comparison::GreaterThan || upper == Comparison::LessThan));
                assert_eq!(spec.compile(&catalog()).is_err(), contradictory);
            }
        }
    }
}

#[test]
fn invalid_specifications_and_ambiguous_catalogs_fail_before_measurements() {
    let check = |spec: ObjectiveSpec| assert!(spec.compile(&catalog()).is_err());
    let mut bad = spec();
    bad.schema_version = 2;
    check(bad);
    let mut bad = spec();
    bad.constraints
        .push(constraint("x", Comparison::AtLeast, 0.0));
    bad.constraints[0].metric = query("not-registered");
    check(bad);
    let mut bad = spec();
    bad.constraints
        .push(constraint("x", Comparison::AtLeast, 0.0));
    bad.constraints[0].unit = MetricUnit::Damage;
    check(bad);
    for threshold in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let mut bad = spec();
        bad.constraints
            .push(constraint("x", Comparison::AtLeast, threshold));
        check(bad);
    }
    for scale in [0.0, -1.0, f64::INFINITY, f64::NAN] {
        let mut bad = spec();
        bad.constraints
            .push(constraint("x", Comparison::AtLeast, 0.0));
        bad.constraints[0].violation_scale = scale;
        check(bad);
    }
    for id in ["", " "] {
        let mut bad = spec();
        bad.constraints
            .push(constraint(id, Comparison::AtLeast, 0.0));
        check(bad);
    }
    let mut bad = spec();
    bad.constraints = vec![constraint("same", Comparison::AtLeast, 0.0); 2];
    check(bad);
    let mut bad = spec();
    bad.constraints = vec![constraint("same", Comparison::AtLeast, 0.0); MAX_CONSTRAINTS + 1];
    check(bad);
    let mut ambiguous = catalog();
    ambiguous.push(ambiguous[0].clone());
    assert!(spec().compile(&ambiguous).is_err());
    let mut version = catalog();
    version[0].schema_version = 0;
    assert!(spec().compile(&version).is_err());
}

#[test]
fn missing_and_nonfinite_values_are_unavailable_not_zero_or_automatic_pass() {
    let mut spec = spec();
    spec.constraints
        .push(constraint("x", Comparison::AtLeast, 75.0));
    let policy = spec.compile(&catalog()).unwrap();
    for value in [
        MeasurementValue::Unavailable {
            reason: "unsupported mechanic".into(),
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::PositiveInfinity,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NegativeInfinity,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NotANumber,
        },
    ] {
        let mut measurements = values(75.0);
        measurements[1].value = value.clone();
        let result = policy.assess(&measurements).unwrap();
        assert_eq!(result.status, AssessmentStatus::Unavailable);
        assert_eq!(result.constraints[0].observed, value);
        assert_eq!(result.constraints[0].status, ConstraintStatus::Unavailable);
        assert!(result.constraints[0].shortfall.is_none());
        assert!(result.total_normalized_violation.finite().is_none());
        measurements[0].value = value.clone();
        let result = policy.assess(&measurements).unwrap();
        assert_eq!(result.objective_value, value);
        assert!(result.objective_score.finite().is_none());
    }
    let result = policy.assess(&values(75.0)[..1]).unwrap();
    assert_eq!(result.status, AssessmentStatus::Unavailable);
    assert!(matches!(
        result.constraints[0].observed,
        MeasurementValue::Unavailable { .. }
    ));
}

#[test]
fn duplicate_mistyped_or_corrupt_finite_measurements_are_contract_errors() {
    let policy = spec().compile(&catalog()).unwrap();
    let mut bad = values(0.0);
    bad.push(bad[0].clone());
    assert!(policy.assess(&bad).is_err());
    let mut bad = values(0.0);
    bad[0].unit = MetricUnit::Damage;
    assert!(policy.assess(&bad).is_err());
    let mut bad = values(0.0);
    bad[0].schema_version = 8;
    assert!(policy.assess(&bad).is_err());
    for number in [f64::INFINITY, f64::NAN] {
        let mut bad = values(0.0);
        bad[0].value = MeasurementValue::Finite { value: number };
        assert!(policy.assess(&bad).is_err());
    }
}

#[test]
fn normalized_shortfalls_use_explicit_scales_and_preserve_overflow() {
    let mut spec = spec();
    spec.constraints
        .push(constraint("x", Comparison::AtLeast, 75.0));
    let result = spec
        .compile(&catalog())
        .unwrap()
        .assess(&values(50.0))
        .unwrap();
    assert_eq!(
        result.constraints[0].shortfall.as_ref().unwrap().finite(),
        Some(25.0)
    );
    assert_eq!(result.total_normalized_violation.finite(), Some(2.5));
    spec.constraints[0].threshold = f64::MAX;
    let result = spec
        .compile(&catalog())
        .unwrap()
        .assess(&values(-f64::MAX))
        .unwrap();
    assert_eq!(result.status, AssessmentStatus::ConstraintsViolated);
    assert_eq!(
        result.total_normalized_violation,
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::PositiveInfinity
        }
    );
    serde_json::to_vec(&result).unwrap();
}

#[test]
fn shared_objective_constraint_queries_are_deduplicated_without_losing_constraints() {
    let mut spec = spec();
    let ObjectivePolicy::Scalar { direction, .. } = &mut spec.objective;
    *direction = Direction::Maximize;
    for (id, operator, threshold) in [
        ("lo", Comparison::AtLeast, 40.0),
        ("hi", Comparison::AtMost, 50.0),
    ] {
        let mut bound = constraint(id, operator, threshold);
        bound.metric = query("custom.cost");
        spec.constraints.push(bound);
    }
    let policy = spec.compile(&catalog()).unwrap();
    assert_eq!(policy.required_metrics().len(), 1);
    let result = policy.assess(&values(0.0)).unwrap();
    assert_eq!(result.constraints.len(), 2);
    assert_eq!(result.objective_score.finite(), Some(42.0));
    assert_eq!(result.status, AssessmentStatus::ConstraintsSatisfied);
}

#[test]
fn json_rejects_unknown_fields_operators_and_policy_modes() {
    let original = serde_json::to_value(spec()).unwrap();
    let mut unknown = original.clone();
    unknown["misspelled"] = true.into();
    assert!(serde_json::from_value::<ObjectiveSpec>(unknown).is_err());
    let mut unknown = original.clone();
    unknown["objective"]["kind"] = "pareto".into();
    assert!(serde_json::from_value::<ObjectiveSpec>(unknown).is_err());
    let mut unknown = original;
    unknown["objective"]["weight"] = 1.into();
    assert!(serde_json::from_value::<ObjectiveSpec>(unknown).is_err());
    assert!(serde_json::from_str::<Comparison>("\"==\"").is_err());
}

#[test]
fn primary_unavailability_keeps_satisfied_constraints_and_versioned_evidence() {
    let mut spec = spec();
    spec.constraints
        .push(constraint("x", Comparison::AtLeast, 75.0));
    let policy = spec.compile(&catalog()).unwrap();
    for primary in [
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NotANumber,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::PositiveInfinity,
        },
        MeasurementValue::NonFinite {
            kind: NonFiniteKind::NegativeInfinity,
        },
    ] {
        let mut measurements = values(75.0);
        measurements[0].value = primary;
        let result = policy.assess(&measurements).unwrap();
        assert_eq!(result.status, AssessmentStatus::Unavailable);
        assert_eq!(result.constraints[0].status, ConstraintStatus::Satisfied);
        assert!(
            result
                .measurements
                .iter()
                .all(|value| value.schema_version == 7)
        );
    }
    let result = policy.assess(&values(75.0)[1..]).unwrap();
    assert_eq!(result.status, AssessmentStatus::Unavailable);
    assert_eq!(result.constraints[0].status, ConstraintStatus::Satisfied);
}
