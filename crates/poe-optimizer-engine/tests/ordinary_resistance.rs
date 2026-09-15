//! Source-free numerical laws. Existing modifier_parity resistance/receiving
//! oracles exercise this same implementation through both migrated callers.
use poe_optimizer_engine::resistance::ordinary::{
    self, OrdinaryResistanceInput, OrdinaryResistanceOutput, OrdinaryResistanceParameters,
    ResistanceChannel, ResistanceFinishInput, ResistanceNumber,
};
fn parameters() -> OrdinaryResistanceParameters {
    OrdinaryResistanceParameters {
        maximum_cap: 90.5,
        resistance_cap: 75.99,
        floor: -199.99,
    }
}
fn input(base: f64) -> OrdinaryResistanceInput {
    OrdinaryResistanceInput {
        base,
        increased_percent: 0.0,
        more_multiplier: 1.0,
    }
}
fn bits(actual: f64, expected: f64) {
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{actual:?} != {expected:?}"
    );
}
fn finite(output: &OrdinaryResistanceOutput, channel: ResistanceChannel, expected: f64) {
    let ResistanceNumber::Finite { value } = output.channel(channel) else {
        panic!("expected finite {channel:?}: {output:?}");
    };
    bits(value, expected);
}
#[test]
fn signed_fractional_totals_and_limits_are_truncated_separately_before_clamping() {
    for (base, total, resistance) in [
        (-1000.5, -1000.0, -199.0),
        (-199.99, -199.0, -199.0),
        (-198.99, -198.0, -198.0),
        (-1.99, -1.0, -1.0),
        (-0.99, -0.0, -0.0),
        (-0.0, -0.0, -0.0),
        (0.0, 0.0, 0.0),
        (0.99, 0.0, 0.0),
        (1.99, 1.0, 1.0),
        (74.99, 74.0, 74.0),
        (75.99, 75.0, 75.0),
        (1000.5, 1000.0, 75.0),
    ] {
        let output = ordinary::calculate(parameters(), input(base));
        finite(&output, ResistanceChannel::PreTruncationTotal, base);
        finite(&output, ResistanceChannel::Total, total);
        finite(&output, ResistanceChannel::Resistance, resistance);
        finite(&output, ResistanceChannel::Cap, 75.0);
        finite(&output, ResistanceChannel::Floor, -199.0);
    }
    let lower_source_cap = ordinary::calculate(
        OrdinaryResistanceParameters {
            maximum_cap: 50.99,
            ..parameters()
        },
        input(70.5),
    );
    finite(&lower_source_cap, ResistanceChannel::Total, 70.0);
    finite(&lower_source_cap, ResistanceChannel::Cap, 50.0);
    finite(&lower_source_cap, ResistanceChannel::Resistance, 50.0);
}
#[test]
fn increased_and_more_combine_before_the_nonnegative_multiplier_clamp() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryResistanceInput {
            base: 10.25,
            increased_percent: 50.0,
            more_multiplier: 2.0,
        },
    );
    finite(&output, ResistanceChannel::Multiplier, 3.0);
    finite(&output, ResistanceChannel::PreTruncationTotal, 30.75);
    finite(&output, ResistanceChannel::Total, 30.0);
    // Raw numeric compatibility vectors, not source modifier-family admission.
    // Clamping the increased factor before MORE would incorrectly yield zero.
    let negative_factors = ordinary::calculate(
        parameters(),
        OrdinaryResistanceInput {
            base: -10.25,
            increased_percent: -150.0,
            more_multiplier: -2.0,
        },
    );
    finite(&negative_factors, ResistanceChannel::Multiplier, 1.0);
    finite(&negative_factors, ResistanceChannel::Total, -10.0);
    for increased_percent in [-100.0, -150.0] {
        let clipped = ordinary::calculate(
            parameters(),
            OrdinaryResistanceInput {
                base: -10.25,
                increased_percent,
                more_multiplier: 1.0,
            },
        );
        finite(&clipped, ResistanceChannel::Multiplier, 0.0);
        finite(&clipped, ResistanceChannel::PreTruncationTotal, -0.0);
        finite(&clipped, ResistanceChannel::Resistance, -0.0);
    }
}
#[test]
fn lua_equal_operand_selection_preserves_the_cap_and_floor_zero_signs() {
    for maximum in [-0.0, 0.0] {
        for resistance_cap in [-0.0, 0.0] {
            let output = ordinary::calculate(
                OrdinaryResistanceParameters {
                    maximum_cap: maximum,
                    resistance_cap,
                    floor: -1.0,
                },
                input(0.0),
            );
            // The ordinary cap's second operand wins equality.
            finite(&output, ResistanceChannel::Cap, resistance_cap);
            finite(&output, ResistanceChannel::Resistance, resistance_cap);
        }
    }
    for total in [-0.0, 0.0] {
        for maximum in [-0.0, 0.0] {
            let output = ordinary::finish(ResistanceFinishInput {
                total,
                maximum,
                floor: -1.0,
            });
            bits(output.total, total);
            bits(output.resistance, maximum);
        }
        for floor in [-0.0, 0.0] {
            let output = ordinary::finish(ResistanceFinishInput {
                total,
                maximum: 1.0,
                floor,
            });
            bits(output.resistance, floor);
        }
    }
    let tiny = ordinary::calculate(
        parameters(),
        OrdinaryResistanceInput {
            base: -f64::from_bits(1),
            increased_percent: -50.0,
            more_multiplier: 1.0,
        },
    );
    finite(&tiny, ResistanceChannel::PreTruncationTotal, -0.0);
    finite(&tiny, ResistanceChannel::Total, -0.0);
}
#[test]
fn resolved_zero_overrides_and_maximum_bypass_need_no_truthiness_or_source_defaults() {
    let ordinary = ordinary::calculate(
        OrdinaryResistanceParameters {
            maximum_cap: 50.0,
            resistance_cap: 100.0,
            floor: -200.0,
        },
        input(90.0),
    );
    finite(&ordinary, ResistanceChannel::Resistance, 50.0);
    // The caller has selected a maximum override, so finish does not reapply 50.
    let selected = ordinary::finish(ResistanceFinishInput {
        total: 90.75,
        maximum: 100.99,
        floor: -200.5,
    });
    bits(selected.total, 90.0);
    bits(selected.cap, 100.0);
    bits(selected.resistance, 90.0);
    let zero_total = ordinary::finish(ResistanceFinishInput {
        total: 0.0,
        maximum: 100.0,
        floor: -200.0,
    });
    bits(zero_total.resistance, 0.0);
    let zero_maximum = ordinary::finish(ResistanceFinishInput {
        total: 90.0,
        maximum: 0.0,
        floor: -200.0,
    });
    bits(zero_maximum.resistance, 0.0);
    // Numeric computability is independent of whether an injected bound pair is legal.
    let inverted = ordinary::finish(ResistanceFinishInput {
        total: -10.0,
        maximum: 5.75,
        floor: 15.99,
    });
    bits(inverted.cap, 5.0);
    bits(inverted.floor, 15.0);
    bits(inverted.resistance, 15.0);
}
#[test]
fn overflow_and_nonfinite_totals_remain_visible_when_the_final_resistance_is_finite() {
    for (base, expected, resistance) in [
        (f64::MAX, ResistanceNumber::PositiveInfinity, 75.0),
        (-f64::MAX, ResistanceNumber::NegativeInfinity, -199.0),
    ] {
        let output = ordinary::calculate(
            parameters(),
            OrdinaryResistanceInput {
                increased_percent: 100.0,
                ..input(base)
            },
        );
        assert_eq!(
            output.channel(ResistanceChannel::PreTruncationTotal),
            expected
        );
        assert_eq!(output.channel(ResistanceChannel::Total), expected);
        finite(&output, ResistanceChannel::Resistance, resistance);
    }
    let unordered = ordinary::calculate(parameters(), input(f64::NAN));
    assert_eq!(
        unordered.channel(ResistanceChannel::PreTruncationTotal),
        ResistanceNumber::NotANumber
    );
    assert_eq!(
        unordered.channel(ResistanceChannel::Total),
        ResistanceNumber::NotANumber
    );
    finite(&unordered, ResistanceChannel::Resistance, 75.0);
    // Explicit raw NaN/Infinity vectors classify operation behavior only. Existing
    // adapters still reject invalid inputs and nonfinite pre-truncation totals.
    let zero_times_infinity = ordinary::calculate(
        parameters(),
        OrdinaryResistanceInput {
            increased_percent: -100.0,
            ..input(f64::INFINITY)
        },
    );
    finite(&zero_times_infinity, ResistanceChannel::Multiplier, 0.0);
    assert_eq!(
        zero_times_infinity.channel(ResistanceChannel::Total),
        ResistanceNumber::NotANumber
    );
    finite(&zero_times_infinity, ResistanceChannel::Resistance, 75.0);
}
#[test]
fn nonfinite_limits_are_classified_independently_without_erasing_other_channels() {
    let output = ordinary::calculate(
        OrdinaryResistanceParameters {
            maximum_cap: f64::INFINITY,
            resistance_cap: f64::INFINITY,
            floor: f64::NEG_INFINITY,
        },
        input(12.5),
    );
    assert_eq!(
        output.channel(ResistanceChannel::Cap),
        ResistanceNumber::PositiveInfinity
    );
    assert_eq!(
        output.channel(ResistanceChannel::Floor),
        ResistanceNumber::NegativeInfinity
    );
    finite(&output, ResistanceChannel::Total, 12.0);
    finite(&output, ResistanceChannel::Resistance, 12.0);
    let unordered = ordinary::finish(ResistanceFinishInput {
        total: 12.0,
        maximum: 75.0,
        floor: f64::NAN,
    });
    assert_eq!(
        ResistanceNumber::classify(unordered.resistance),
        ResistanceNumber::NotANumber
    );
    bits(unordered.total, 12.0);
    let finite_zero = ResistanceNumber::classify(-0.0);
    let ResistanceNumber::Finite { value } = finite_zero else {
        unreachable!()
    };
    bits(value, -0.0);
}
#[test]
fn finite_clamping_is_monotone_bounded_and_idempotent_for_explicit_legal_limits() {
    for (maximum, floor) in [(9.99, -2.99), (75.1, -200.9), (0.0, -10.0), (5.0, 5.0)] {
        let mut previous = f64::NEG_INFINITY;
        for total in [
            -1000.0, -200.9, -10.9, -2.99, -0.25, 0.0, 0.25, 5.0, 9.99, 75.9, 1000.0,
        ] {
            let output = ordinary::finish(ResistanceFinishInput {
                total,
                maximum,
                floor,
            });
            assert!(output.resistance >= previous);
            assert!(output.resistance >= output.floor && output.resistance <= output.cap);
            bits(output.resistance.fract(), 0.0);
            let again = ordinary::finish(ResistanceFinishInput {
                total: output.resistance,
                maximum,
                floor,
            });
            bits(again.resistance, output.resistance);
            previous = output.resistance;
        }
    }
}
