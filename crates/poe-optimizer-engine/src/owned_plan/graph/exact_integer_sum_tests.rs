//! Unordered integer reduction laws and failure propagation over the worker kernel.
use super::*;

fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
fn integer(value: i64) -> ParameterValue {
    ParameterValue::Integer(BoundedInteger::new(value).unwrap())
}
fn binding(effects: Vec<usize>, complete: bool) -> ReadBinding {
    ReadBinding::Reduction {
        effects,
        reduction: ContributionReduction::ExactIntegerSum,
        empty: integer(0),
        complete,
    }
}
fn evaluate(values: &[Option<EffectValue>], order: Vec<usize>) -> EffectValue {
    read(&binding(order, true), values, &key("sum"), &mut 100).unwrap()
}
fn numbers(values: &[i64]) -> Vec<Option<EffectValue>> {
    values
        .iter()
        .map(|value| Some(known(integer(*value))))
        .collect()
}
fn overflow() -> EffectValue {
    EffectValue::NumericalError {
        node: key("sum"),
        reason: NumericalFailure::IntegerOverflow,
    }
}

#[test]
fn exact_integer_sum_checks_complete_membership_integer_values_and_zero_identity() {
    assert_eq!(evaluate(&[], vec![]), known(integer(0)));
    assert_eq!(
        evaluate(&[Some(EffectValue::Inactive)], vec![0]),
        known(integer(0))
    );
    assert_eq!(
        read(&binding(vec![], false), &[], &key("sum"), &mut 1).unwrap(),
        EffectValue::unresolved(PlanGapReason::IncompleteContributors),
    );
    for empty in [integer(1), ParameterValue::Boolean(false)] {
        let bad = ReadBinding::Reduction {
            effects: vec![],
            reduction: ContributionReduction::ExactIntegerSum,
            empty,
            complete: true,
        };
        assert!(read(&bad, &[], &key("sum"), &mut 1).is_err());
    }
    let unit = UnitDefId::parse(GameVersionNamespace::new("sum", "v1").unwrap(), "unit").unwrap();
    for value in [
        ParameterValue::Boolean(false),
        ParameterValue::Quantity(FiniteQuantity::new(0.0, unit).unwrap()),
    ] {
        assert!(
            read(
                &binding(vec![0], true),
                &[Some(known(value))],
                &key("sum"),
                &mut 2
            )
            .is_err()
        );
    }
    assert!(read(&binding(vec![0], true), &[None], &key("sum"), &mut 2).is_err());
}

#[test]
fn exact_integer_sum_allows_cancellation_before_the_final_public_range_check() {
    let max = BoundedInteger::MAX;
    let min = BoundedInteger::MIN;
    for (input, expected) in [
        ([max, max, -max], max),
        ([min, min, -min], min),
        ([max, 1, -1], max),
        ([min, -1, 1], min),
        ([1, 1, 1], 3),
    ] {
        let values = numbers(&input);
        for order in [vec![0, 1, 2], vec![2, 1, 0], vec![1, 0, 2], vec![0, 2, 1]] {
            assert_eq!(evaluate(&values, order), known(integer(expected)));
        }
    }
    for input in [[max, 1], [min, -1], [max, max], [min, min]] {
        assert_eq!(evaluate(&numbers(&input), vec![0, 1]), overflow());
        assert_eq!(evaluate(&numbers(&input), vec![1, 0]), overflow());
    }
    // The ordinary ordered reducer deliberately preserves intermediate overflow.
    let old = ReadBinding::Reduction {
        effects: vec![0, 1, 2],
        reduction: ContributionReduction::Sum,
        empty: integer(0),
        complete: true,
    };
    assert_eq!(
        read(&old, &numbers(&[max, max, -max]), &key("sum"), &mut 4).unwrap(),
        overflow()
    );
}

#[test]
fn exact_integer_sum_handles_the_entire_hard_effect_domain_without_i64_prefix_overflow() {
    let limits = PlanLimits::default();
    assert_eq!(limits.max_effects, 65_536);
    let mut too_large = limits;
    too_large.max_effects += 1;
    assert!(too_large.validate().is_err());
    let mut values = vec![Some(known(integer(BoundedInteger::MAX))); limits.max_effects / 2];
    values.extend(vec![
        Some(known(integer(BoundedInteger::MIN)));
        limits.max_effects / 2
    ]);
    for order in [
        (0..values.len()).collect(),
        (0..values.len()).rev().collect(),
    ] {
        let mut work = values.len() + 1;
        assert_eq!(
            read(&binding(order, true), &values, &key("sum"), &mut work).unwrap(),
            known(integer(0))
        );
        assert_eq!(work, 0);
    }
}

#[test]
fn exact_integer_sum_preserves_canonical_failures_and_never_short_circuits_members() {
    let failures = [
        EffectValue::Unresolved {
            reason: PlanGapReason::MissingInput,
            read: Some(key("source")),
        },
        EffectValue::unresolved(PlanGapReason::MissingProducer),
        EffectValue::UnsupportedValue { value: integer(2) },
        EffectValue::UnsupportedDomain {
            node: key("range"),
            domain: UnsupportedDomainCause::NumericRange {
                value: integer(2),
                domain: Box::new(NumericDomain::Integer(IntegerRange {
                    minimum: BoundedInteger::new(0).unwrap(),
                    maximum: BoundedInteger::new(1).unwrap(),
                })),
            },
        },
        EffectValue::NumericalError {
            node: key("divide"),
            reason: NumericalFailure::DivisionByZero,
        },
    ];
    for a in &failures {
        for b in &failures {
            let values = vec![
                Some(known(integer(BoundedInteger::MAX))),
                Some(known(integer(1))),
                Some(a.clone()),
                Some(b.clone()),
                Some(EffectValue::Inactive),
            ];
            let expected = if diagnostic_order::compare(a, b).is_le() {
                a
            } else {
                b
            };
            for order in [vec![0, 1, 2, 3, 4], vec![4, 3, 2, 1, 0]] {
                let mut work = 6;
                let result = read(
                    &binding(order.clone(), true),
                    &values,
                    &key("sum"),
                    &mut work,
                )
                .unwrap();
                assert_eq!(&result, expected);
                assert_eq!(work, 0);
                assert!(matches!(
                    read(&binding(order, true), &values, &key("sum"), &mut 5),
                    Err(PlanError::Limit("work"))
                ));
            }
        }
    }
}
