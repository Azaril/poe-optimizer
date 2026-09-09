use poe_optimizer_engine::item_runes::*;
use poe_optimizer_engine::lua_pattern::{CompileLimits, MatchLimits};
fn text() -> RuneText {
    RuneText::compile(b"(%d%.?%d*)", CompileLimits::default()).unwrap()
}
fn policy() -> VectorPolicy {
    VectorPolicy {
        missing_value: 0.0,
        epsilon: 1e-9,
    }
}
#[test]
fn exact_grammar_and_injected_defaults_are_not_general_decimal_parsing() {
    let mut b = RuneBudget::default();
    let p = text()
        .line_parts(b"-12.34 and -.5 or 1e-9", b"@", 7.0, &mut b)
        .unwrap();
    assert_eq!(p.stripped, b"-@.@ and -.@ or @e-@");
    assert_eq!(p.values, [12.0, 34.0, 5.0, 1.0, 9.0]);
    assert_eq!(
        text()
            .line_parts(b"none", b"#", 7.0, &mut b)
            .unwrap()
            .values,
        [7.0]
    );
    let no_numbers = RuneText::compile(b"(%a+)", CompileLimits::default()).unwrap();
    assert_eq!(
        no_numbers.line_parts(b"abc", b"", 9.0, &mut b).unwrap(),
        RuneLineParts {
            stripped: vec![],
            values: vec![9.0]
        }
    );
}
#[test]
fn callback_error_does_not_return_partial_text_and_success_round_trips() {
    let mut b = RuneBudget::default();
    let stored = b"12.34 damage".to_vec();
    assert!(matches!(
        text().combine(&stored, b"5.67 damage", &mut b),
        Err(RuneError::Source(_))
    ));
    assert_eq!(stored, b"12.34 damage");
    assert_eq!(
        text()
            .combine(b"99999999999999 damage", b"1 damage", &mut b)
            .unwrap(),
        b"1e+14 damage"
    );
    assert_eq!(
        text()
            .combine(b"1e+14 damage", b"2 3 damage", &mut b)
            .unwrap(),
        b"3e+17 damage"
    );
    assert_eq!(
        text().combine(b"no values", b"1 2", &mut b).unwrap(),
        b"no values"
    );
}
#[test]
fn order_text_has_source_collisions_and_signed_zero_distinctions() {
    let mut b = RuneBudget::default();
    assert_eq!(
        number_order_key(b"Injected:", 1.000000000000001, &mut b).unwrap(),
        number_order_key(b"Injected:", 1.000000000000002, &mut b).unwrap()
    );
    assert_ne!(
        number_order_key(b"", -0.0, &mut b).unwrap(),
        number_order_key(b"", 0.0, &mut b).unwrap()
    );
}
#[test]
fn ieee_vector_predicates_and_missing_component_policy_are_explicit() {
    let mut b = RuneBudget::default();
    assert!(equal_vectors(&[f64::NAN], &[1.0], policy(), &mut b).unwrap());
    assert!(equal_vectors(&[f64::INFINITY], &[f64::INFINITY], policy(), &mut b).unwrap());
    assert!(!compare_vectors(&[f64::NAN, 10.0], &[1.0, 0.0], policy(), &mut b).unwrap());
    let injected = VectorPolicy {
        missing_value: 4.0,
        epsilon: 0.5,
    };
    assert_eq!(add_vectors(&[], &[3.0], injected, &mut b).unwrap(), [7.0]);
    assert!(equal_vectors(&[], &[4.4], injected, &mut b).unwrap());
}
#[test]
fn source_search_checks_equality_before_negative_or_fractional_caps() {
    let mut b = RuneBudget::default();
    let zero = find_combination(&[&[2.0]], &[0.0], -1.0, None, policy(), &mut b)
        .unwrap()
        .unwrap();
    assert_eq!(zero.count, 0);
    assert!(zero.counts.is_empty());
    let fractional = find_combination(&[&[2.0]], &[4.0], 1.5, None, policy(), &mut b)
        .unwrap()
        .unwrap();
    assert_eq!(fractional.count, 2);
    assert_eq!(
        find_combination(&[&[2.0]], &[4.0], 3.0, Some(&[1.5]), policy(), &mut b)
            .unwrap()
            .unwrap()
            .count,
        2
    );
    assert!(
        find_combination(&[&[2.0]], &[2.0], 3.0, Some(&[f64::NAN]), policy(), &mut b)
            .unwrap()
            .is_none()
    );
}
#[test]
fn ambiguity_proof_retains_source_first_minimum_and_zero_touched_keys() {
    let mut b = RuneBudget::default();
    let tie = find_combination(&[&[2.0], &[2.0]], &[2.0], 2.0, None, policy(), &mut b)
        .unwrap()
        .unwrap();
    assert!(tie.ambiguous_minimum);
    assert_eq!(tie.counts.into_iter().collect::<Vec<_>>(), [(1, 1)]);
    let smaller = find_combination(&[&[1.0], &[2.0]], &[2.0], 3.0, None, policy(), &mut b)
        .unwrap()
        .unwrap();
    assert!(!smaller.ambiguous_minimum);
    assert_eq!(smaller.count, 1);
    assert_eq!(
        smaller.counts.into_iter().collect::<Vec<_>>(),
        [(1, 0), (2, 1)]
    );
}
#[test]
fn output_callbacks_and_matching_have_independent_cumulative_limits() {
    let mut output = RuneBudget::new(RuneLimits {
        max_output_bytes: 1,
        ..RuneLimits::default()
    });
    assert!(matches!(
        text().line_parts(b"1 2", b"###", 1.0, &mut output),
        Err(RuneError::Resource("output bytes"))
    ));
    let mut callbacks = RuneBudget::new(RuneLimits {
        max_callbacks: 1,
        ..RuneLimits::default()
    });
    text().line_parts(b"1", b"#", 1.0, &mut callbacks).unwrap();
    assert!(matches!(
        text().line_parts(b"2", b"#", 1.0, &mut callbacks),
        Err(RuneError::Resource("callbacks"))
    ));
    let mut matching = RuneBudget::new(RuneLimits {
        matching: MatchLimits {
            max_steps: 0,
            ..MatchLimits::default()
        },
        ..RuneLimits::default()
    });
    assert!(matches!(
        text().line_parts(b"1", b"#", 1.0, &mut matching),
        Err(RuneError::Pattern(_))
    ));
}
#[test]
fn divergent_or_large_search_is_resource_failure_never_no_solution() {
    let mut depth = RuneBudget::new(RuneLimits {
        max_search_depth: 3,
        ..RuneLimits::default()
    });
    assert!(matches!(
        find_combination(&[&[0.0]], &[1.0], f64::NAN, None, policy(), &mut depth),
        Err(RuneError::Resource("search depth"))
    ));
    let mut work = RuneBudget::new(RuneLimits {
        max_search_steps: 1,
        ..RuneLimits::default()
    });
    assert!(matches!(
        find_combination(&[&[1.0]], &[1.0], 1.0, None, policy(), &mut work),
        Err(RuneError::Resource("search steps"))
    ));
    let mut dimensions = RuneBudget::new(RuneLimits {
        max_vector_components: 1,
        ..RuneLimits::default()
    });
    assert!(matches!(
        add_vectors(&[1.0, 2.0], &[], policy(), &mut dimensions),
        Err(RuneError::Resource("vector components"))
    ));
}
#[test]
fn ambiguity_is_proven_against_independent_bounded_integer_count_products() {
    // Enumerate a Cartesian product of counts and calculate its objective
    // directly, independently of the source DFS traversal and pruning.
    for values in (0..4).flat_map(|a| (0..4).flat_map(move |b| (0..4).map(move |c| [a, b, c]))) {
        for target in 0..7 {
            for cap in 0..5 {
                let mut minimum = None;
                let mut solutions = 0;
                for a in 0..=cap {
                    for b in 0..=cap {
                        for c in 0..=cap {
                            let count = a + b + c;
                            if count > cap
                                || a * values[0] + b * values[1] + c * values[2] != target
                            {
                                continue;
                            }
                            if minimum.is_none_or(|old| count < old) {
                                minimum = Some(count);
                                solutions = 1;
                            } else if minimum == Some(count) {
                                solutions += 1;
                            }
                        }
                    }
                }
                let vectors = values.map(|n| [n as f64]);
                let actual = find_combination(
                    &vectors.iter().map(|v| v.as_slice()).collect::<Vec<_>>(),
                    &[target as f64],
                    cap as f64,
                    None,
                    policy(),
                    &mut RuneBudget::default(),
                )
                .unwrap();
                assert_eq!(
                    actual.as_ref().map(|r| r.count),
                    minimum,
                    "{values:?}/{target}/{cap}"
                );
                assert_eq!(
                    actual.is_some_and(|r| r.ambiguous_minimum),
                    solutions > 1,
                    "{values:?}/{target}/{cap}"
                );
            }
        }
    }
}
