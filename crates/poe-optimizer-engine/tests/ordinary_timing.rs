//! Pure timing numerical laws; the existing action_speed_parity source oracle
//! exercises this same operation through the retained legacy adapter.
use poe_optimizer_data::game_data::{DirectActionTimingData, DirectActionTimingEligibility};
use poe_optimizer_engine::timing::{
    self, DirectActionTimingInput,
    ordinary::{
        self, OrdinaryTimingInput, OrdinaryTimingOutput, OrdinaryTimingParameters, TimingChannel,
        TimingNumber,
    },
};

fn parameters() -> OrdinaryTimingParameters {
    OrdinaryTimingParameters {
        server_tick_rate: 4.0,
        speed_multiplier_rounding_precision: 2,
    }
}
fn input() -> OrdinaryTimingInput {
    OrdinaryTimingInput {
        base_time: 0.5,
        increased_percent: 0.0,
        more_multiplier: 1.0,
        additional_attack_time: 0.0,
        additional_cast_time: 0.0,
        action_speed_multiplier: 1.0,
        repeats: 1.0,
    }
}
fn assert_bits(actual: f64, expected: f64) {
    assert_eq!(
        actual.to_bits(),
        expected.to_bits(),
        "{actual:?} != {expected:?}"
    );
}
fn finite(output: &OrdinaryTimingOutput, channel: TimingChannel, expected: f64) {
    let TimingNumber::Finite { value } = output.channel(channel) else {
        panic!("channel was not finite: {output:?}");
    };
    assert_bits(value, expected);
}

#[test]
fn action_speed_precedes_the_injected_cap_and_repeats_change_its_ceiling() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            action_speed_multiplier: 3.0,
            ..input()
        },
    );
    finite(&output, TimingChannel::PreCapRate, 6.0);
    finite(&output, TimingChannel::ActionRate, 4.0);
    finite(&output, TimingChannel::ActionTime, 0.25);
    finite(&output, TimingChannel::BaseTime, 0.5);
    finite(&output, TimingChannel::SpeedMultiplier, 1.0);
    finite(&output, TimingChannel::ActionSpeedMultiplier, 3.0);
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            action_speed_multiplier: 3.0,
            repeats: 2.0,
            ..input()
        },
    );
    finite(&output, TimingChannel::PreCapRate, 6.0);
    finite(&output, TimingChannel::ActionRate, 6.0);
    finite(&output, TimingChannel::ActionTime, 1.0 / 6.0);
}

#[test]
fn infinite_pre_cap_rate_does_not_erase_finite_capped_rate_or_time() {
    for base_time in [0.0, 1e-310] {
        let output = ordinary::calculate(
            parameters(),
            OrdinaryTimingInput {
                base_time,
                ..input()
            },
        );
        assert_eq!(
            output.channel(TimingChannel::PreCapRate),
            TimingNumber::PositiveInfinity
        );
        finite(&output, TimingChannel::ActionRate, 4.0);
        finite(&output, TimingChannel::ActionTime, 0.25);
    }
}

#[test]
fn negative_infinity_and_nan_have_independent_channel_classifications() {
    let negative = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            base_time: 0.0,
            action_speed_multiplier: -1.0,
            ..input()
        },
    );
    assert_eq!(
        negative.channel(TimingChannel::PreCapRate),
        TimingNumber::NegativeInfinity
    );
    assert_eq!(
        negative.channel(TimingChannel::ActionRate),
        TimingNumber::NegativeInfinity
    );
    finite(&negative, TimingChannel::ActionTime, -0.0);
    // 0/0 is unavailable numerically in the pre-cap channel, while the historical
    // unordered comparison selects the explicit finite ceiling.
    let unordered = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            base_time: 0.0,
            increased_percent: -100.0,
            ..input()
        },
    );
    assert_eq!(
        unordered.channel(TimingChannel::PreCapRate),
        TimingNumber::NotANumber
    );
    finite(&unordered, TimingChannel::ActionRate, 4.0);
    finite(&unordered, TimingChannel::ActionTime, 0.25);
}

#[test]
fn tiny_finite_rate_can_have_infinite_time_without_erasing_the_rate() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            action_speed_multiplier: 1e-310,
            ..input()
        },
    );
    finite(&output, TimingChannel::PreCapRate, 2e-310);
    finite(&output, TimingChannel::ActionRate, 2e-310);
    assert_eq!(
        output.channel(TimingChannel::ActionTime),
        TimingNumber::PositiveInfinity
    );
}

#[test]
fn finite_negative_zero_survives_except_for_the_explicit_zero_time_case() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            action_speed_multiplier: -0.0,
            ..input()
        },
    );
    finite(&output, TimingChannel::PreCapRate, -0.0);
    finite(&output, TimingChannel::ActionRate, -0.0);
    finite(&output, TimingChannel::ActionTime, 0.0);
    finite(&output, TimingChannel::ActionSpeedMultiplier, -0.0);
    // On equality the cap's sign wins. This raw compatibility vector is not a
    // gameplay assertion that a nonpositive server rate is a valid configuration.
    let equal = ordinary::calculate(
        OrdinaryTimingParameters {
            server_tick_rate: -0.0,
            ..parameters()
        },
        OrdinaryTimingInput {
            action_speed_multiplier: 0.0,
            ..input()
        },
    );
    finite(&equal, TimingChannel::PreCapRate, 0.0);
    finite(&equal, TimingChannel::ActionRate, -0.0);
    finite(&equal, TimingChannel::ActionTime, 0.0);
}

#[test]
fn reciprocal_and_separate_addition_order_cannot_be_algebraically_reassociated() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            base_time: 1e16,
            additional_attack_time: -1e16,
            additional_cast_time: 1.0,
            ..input()
        },
    );
    // Left-to-right denominator is (1e16 - 1e16) + 1 = 1. Grouping the two
    // additional times first would lose 1 and produce an infinite pre-cap rate.
    finite(&output, TimingChannel::PreCapRate, 1.0);
    finite(&output, TimingChannel::ActionRate, 1.0);
    finite(&output, TimingChannel::ActionTime, 1.0);
}

#[test]
fn precision_and_historical_half_rounding_remain_explicit_operation_inputs() {
    let input = OrdinaryTimingInput {
        increased_percent: 13.555,
        ..input()
    };
    for (precision, expected) in [(2, 1.14), (3, 1.136), (0, 1.0)] {
        let output = ordinary::calculate(
            OrdinaryTimingParameters {
                speed_multiplier_rounding_precision: precision,
                ..parameters()
            },
            input,
        );
        finite(&output, TimingChannel::SpeedMultiplier, expected);
    }
    let negative_half = ordinary::calculate(
        OrdinaryTimingParameters {
            speed_multiplier_rounding_precision: 0,
            ..parameters()
        },
        OrdinaryTimingInput {
            increased_percent: -250.0,
            ..input
        },
    );
    finite(&negative_half, TimingChannel::SpeedMultiplier, -1.0);
    let integral = ordinary::calculate(
        OrdinaryTimingParameters {
            speed_multiplier_rounding_precision: 0,
            ..parameters()
        },
        OrdinaryTimingInput {
            increased_percent: 0.0,
            more_multiplier: 4_503_599_627_370_497.0,
            ..input
        },
    );
    // The preserved floor(x + .5) contract is intentionally different from the
    // generic owned operation's stable mathematical nearest rounding.
    finite(
        &integral,
        TimingChannel::SpeedMultiplier,
        4_503_599_627_370_498.0,
    );
}

#[test]
fn raw_nonfinite_inputs_do_not_trigger_an_aggregate_short_circuit() {
    let output = ordinary::calculate(
        parameters(),
        OrdinaryTimingInput {
            action_speed_multiplier: f64::NAN,
            ..input()
        },
    );
    assert_eq!(
        output.channel(TimingChannel::ActionSpeedMultiplier),
        TimingNumber::NotANumber
    );
    assert_eq!(
        output.channel(TimingChannel::PreCapRate),
        TimingNumber::NotANumber
    );
    finite(&output, TimingChannel::ActionRate, 4.0);
    finite(&output, TimingChannel::ActionTime, 0.25);
    let output = ordinary::calculate(
        OrdinaryTimingParameters {
            server_tick_rate: f64::NAN,
            ..parameters()
        },
        input(),
    );
    finite(&output, TimingChannel::PreCapRate, 2.0);
    assert_eq!(
        output.channel(TimingChannel::ActionRate),
        TimingNumber::NotANumber
    );
    assert_eq!(
        output.channel(TimingChannel::ActionTime),
        TimingNumber::NotANumber
    );
}

#[test]
fn legacy_field_mapping_retains_all_bits_over_existing_source_parity_vectors() {
    // Same45 cold numerical vectors as the retained original-source timing
    // oracle (which also runs warm for90). No bundled/default dataset is loaded.
    let mut checks = 0;
    for tick in [1.0 / 0.033, 7.125, 1e6] {
        let parameters = OrdinaryTimingParameters {
            server_tick_rate: tick,
            speed_multiplier_rounding_precision: if tick == 7.125 { 3 } else { 2 },
        };
        let data = DirectActionTimingData {
            server_tick_rate: parameters.server_tick_rate,
            speed_multiplier_rounding_precision: parameters.speed_multiplier_rounding_precision,
            default_repeats: 1,
            eligibility: DirectActionTimingEligibility::OrdinarySelfCast,
        };
        for (
            base_time,
            increased_percent,
            more_multiplier,
            action_speed_multiplier,
            additional_attack_time,
            additional_cast_time,
            repeats,
        ) in [
            (0.7, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (0.7, 13.555, 0.87, 1.25, 0.0, 0.0, 1.0),
            (0.7, -100.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (0.7, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0),
            (0.7, 0.0, 1.0, -0.0, 0.0, 0.0, 1.0),
            (0.7, 0.0, 1.0, -1.25, 0.0, 0.0, 1.0),
            (0.7, 0.0, 1.0, 1e-310, 0.0, 0.0, 1.0),
            (1e-310, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (0.0, -100.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (1e6, 1e6, -1.0, 1e6, 0.0, 0.0, 1.0),
            (-0.7, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0),
            (0.7, 1e6, 1e6, 1e6, 0.0, 0.0, 1.0),
            (0.7, 25.0, 1.25, 1.5, 0.125, 0.25, 2.0),
            (0.7, 25.0, 1.25, 1.5, -0.125, -0.25, 0.0),
        ] {
            let pure = ordinary::calculate(
                parameters,
                OrdinaryTimingInput {
                    base_time,
                    increased_percent,
                    more_multiplier,
                    action_speed_multiplier,
                    additional_attack_time,
                    additional_cast_time,
                    repeats,
                },
            );
            let legacy = timing::calculate(
                &data,
                DirectActionTimingInput {
                    base_time,
                    increased: increased_percent,
                    more: more_multiplier,
                    action_speed_mod: action_speed_multiplier,
                    additional_attack_time,
                    additional_cast_time,
                    repeats,
                },
            );
            for (a, b) in [
                (pure.base_time, legacy.base_time),
                (pure.speed_multiplier, legacy.speed_multiplier),
                (pure.pre_cap_rate, legacy.cast_rate),
                (pure.action_rate, legacy.speed),
                (pure.action_time, legacy.time),
                (pure.action_speed_multiplier, legacy.action_speed_mod),
            ] {
                if a.is_nan() {
                    assert!(b.is_nan());
                } else {
                    assert_bits(a, b);
                }
            }
            checks += 1;
        }
    }
    assert_eq!(checks, 45);
}

#[test]
fn pure_values_can_be_shared_across_workers_and_reused_without_state() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<OrdinaryTimingParameters>();
    send_sync::<OrdinaryTimingInput>();
    send_sync::<OrdinaryTimingOutput>();
    let a = input();
    let b = OrdinaryTimingInput {
        action_speed_multiplier: 3.0,
        ..a
    };
    let expected = ordinary::calculate(parameters(), a);
    std::thread::scope(|scope| {
        for _ in 0..8 {
            scope.spawn(move || {
                assert_eq!(ordinary::calculate(parameters(), a), expected);
                assert_eq!(ordinary::calculate(parameters(), b).action_rate, 4.0);
                assert_eq!(ordinary::calculate(parameters(), a), expected);
            });
        }
    });
}
