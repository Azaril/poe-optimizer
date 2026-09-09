use poe_optimizer_engine::lua_bits::or53;

#[test]
fn source_recombines_a_signed_low_word() {
    let two32 = 4_294_967_296.0;
    let two53 = 9_007_199_254_740_992.0;
    for (a, b, expected) in [
        (0.0, 0.0, 0.0),
        (1.0, 2.0, 3.0),
        (3.0, 2.0, 3.0),
        (two32, 1.0, two32 + 1.0),
        (two32 / 2.0, 0.0, -two32 / 2.0),
        (two32 + two32 / 2.0, 0.0, two32 / 2.0),
        (-1.0, 0.0, two53 - two32 - 1.0),
        (two53 - 1.0, 0.0, two53 - two32 - 1.0),
        (two53, 0.0, 0.0),
    ] {
        assert_eq!(or53(a, b).to_bits(), expected.to_bits(), "{a} | {b}");
        assert_eq!(or53(b, a).to_bits(), expected.to_bits(), "{b} | {a}");
    }
}

#[test]
fn low_word_fractions_round_to_even_after_source_modulo() {
    let high_mask: f64 = 9_007_199_254_740_992.0 - 4_294_967_296.0;
    for (a, expected) in [
        (0.5, 0.0),
        (1.5, 2.0),
        (2.5, 2.0),
        (3.5, 4.0),
        (-0.5, high_mask),
        (-1.5, high_mask - 2.0),
        (-2.5, high_mask - 2.0),
    ] {
        assert_eq!(or53(a, 0.0).to_bits(), expected.to_bits(), "{a}");
    }
}
