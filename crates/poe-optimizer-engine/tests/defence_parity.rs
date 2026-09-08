//! Differential checks evaluate the pinned upstream functions, not a rewritten Lua formula.
#![cfg(not(target_arch = "wasm32"))]

use mlua::{Function, Lua, Table};
use poe_optimizer_engine::defence::{
    DefenceConstants, armour_reduction_percent, armour_reduction_rounded_percent, deflect_chance,
    hit_chance, monster_hit_chance, pinned_constants, round_to_integer,
};
use sha2::{Digest, Sha256};

const DEFENCE_SOURCE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/CalcDefence.lua");
const COMMON_SOURCE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/Common.lua");
const DATA_SOURCE: &str =
    include_str!("../../../vendor/path-of-building-poe2/src/Modules/Data.lua");

struct Oracle {
    _lua: Lua,
    hit: Function,
    monster_hit: Function,
    deflect: Function,
    armour: Function,
    rounded_armour: Function,
    round: Function,
}

impl Oracle {
    fn new(constants: DefenceConstants, warm: bool) -> Self {
        verify_source();
        let lua = Lua::new();
        let misc = lua.create_table().unwrap();
        misc.set("ArmourRatio", constants.armour_ratio).unwrap();
        misc.set("DeflectionChanceCap", constants.deflection_chance_cap)
            .unwrap();
        let data = lua.create_table().unwrap();
        data.set("misc", misc).unwrap();
        lua.globals().set("data", data).unwrap();
        lua.load("package.loaded['Modules.CalcBase'] = {}")
            .exec()
            .unwrap();
        // Load the actual upstream round definition with its sole captured local.
        let common = COMMON_SOURCE.replace("\r\n", "\n");
        let start = common.find("function round(val, dec)\n").unwrap();
        let end = common[start..].find("\n--- Rounds down a number").unwrap() + start;
        lua.load(format!(
            "local m_floor = math.floor\n{}",
            &common[start..end]
        ))
        .set_name("pinned-Common-round")
        .exec()
        .unwrap();
        let calcs: Table = lua
            .load(DEFENCE_SOURCE)
            .set_name("pinned-CalcDefence")
            .eval()
            .unwrap();
        lua.load(if warm { "jit.on()" } else { "jit.off()" })
            .exec()
            .unwrap();
        // Calling a wrapper loop makes the warm mode exercise JIT traces, while
        // every case still obtains its answer directly from the upstream function.
        let wrapper: Function = lua
            .load(
                "return function(f) return function(a, b, c) \
                 local result; for i = 1, 200 do result = f(a, b, c) end; \
                 return result end end",
            )
            .eval()
            .unwrap();
        let get = |name: &str| -> Function {
            let function: Function = calcs.get(name).unwrap();
            if warm {
                wrapper.call(function).unwrap()
            } else {
                function
            }
        };
        let hit = get("hitChance");
        let monster_hit = get("monsterHitChance");
        let deflect = get("deflectChance");
        let armour = get("armourReductionF");
        let rounded_armour = get("armourReduction");
        let round = lua.globals().get("round").unwrap();
        Self {
            _lua: lua,
            hit,
            monster_hit,
            deflect,
            armour,
            rounded_armour,
            round,
        }
    }
}

fn verify_source() {
    // CRLF-only normalization matches the evaluator's pinned source manifest.
    for (name, source, expected) in [
        (
            "CalcDefence.lua",
            DEFENCE_SOURCE,
            "b0f498e93dd69ac09deea875dfc186345e26a977419a6b5cec1cf9b04e1f46d2",
        ),
        (
            "Common.lua",
            COMMON_SOURCE,
            "bae6d0704a92fb04ed56a6033f9229eb2683c53785c14b9c5571b4c0591b0fe8",
        ),
        (
            "Data.lua",
            DATA_SOURCE,
            "2c7d37cfdeda234a8741847e6dd5019dcf8ca9752aaed596f333f80489b430a4",
        ),
    ] {
        assert_eq!(
            format!("{:x}", Sha256::digest(source.replace("\r\n", "\n"))),
            expected,
            "Review native translation and provenance after changing {name}"
        );
    }
}

fn assert_same(name: &str, inputs: (f64, f64), native: f64, upstream: f64) {
    if upstream.is_nan() {
        assert!(native.is_nan(), "{name}{inputs:?}: {native:?} != NaN");
    } else if upstream.is_infinite() || upstream == 0.0 {
        // Preserve infinity signs and signed zero; NaN payloads are not portable.
        assert_eq!(
            native.to_bits(),
            upstream.to_bits(),
            "{name}{inputs:?}: {native:?} != {upstream:?}"
        );
    } else {
        let tolerance = 1e-11 * upstream.abs().max(1.0);
        assert!(
            native.is_finite() && (native - upstream).abs() <= tolerance,
            "{name}{inputs:?}: {native:?} != {upstream:?} (tolerance {tolerance})"
        );
    }
}

fn check_pair(oracle: &Oracle, constants: DefenceConstants, a: f64, b: f64) {
    for uncapped in [false, true] {
        assert_same(
            if uncapped { "uncapped_hit" } else { "hit" },
            (a, b),
            hit_chance(a, b, uncapped),
            oracle.hit.call((a, b, uncapped)).unwrap(),
        );
    }
    assert_same(
        "monster_hit",
        (a, b),
        monster_hit_chance(a, b),
        oracle.monster_hit.call((a, b)).unwrap(),
    );
    assert_same(
        "deflect",
        (a, b),
        deflect_chance(a, b, constants),
        oracle.deflect.call((a, b)).unwrap(),
    );
    assert_same(
        "armour",
        (a, b),
        armour_reduction_percent(a, b, constants),
        oracle.armour.call((a, b)).unwrap(),
    );
    assert_same(
        "rounded_armour",
        (a, b),
        armour_reduction_rounded_percent(a, b, constants),
        oracle.rounded_armour.call((a, b)).unwrap(),
    );
}

#[test]
fn defaults_match_pinned_upstream_data() {
    verify_source();
    let constant = |name: &str| -> f64 {
        let prefix = format!("{name} = ");
        DATA_SOURCE
            .lines()
            .find_map(|line| line.trim_start().strip_prefix(&prefix))
            .unwrap()
            .split(',')
            .next()
            .unwrap()
            .parse()
            .unwrap()
    };
    assert_eq!(pinned_constants().armour_ratio, constant("ArmourRatio"));
    assert_eq!(
        pinned_constants().deflection_chance_cap,
        constant("DeflectionChanceCap")
    );
}

#[test]
fn rounding_matches_lua_at_half_boundaries_and_special_values() {
    let oracle = Oracle::new(pinned_constants(), false);
    for integer in -100..=100 {
        let half = f64::from(integer) + 0.5;
        for value in [half.next_down(), half, half.next_up()] {
            assert_same(
                "round",
                (value, 0.0),
                round_to_integer(value),
                oracle.round.call(value).unwrap(),
            );
        }
    }
    for value in [
        -0.0,
        0.0,
        f64::MAX,
        f64::MIN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ] {
        assert_same(
            "round",
            (value, 0.0),
            round_to_integer(value),
            oracle.round.call(value).unwrap(),
        );
    }
    assert_eq!(round_to_integer(-2.5), -2.0);
    assert_eq!(round_to_integer(2.5), 3.0);
}

#[test]
fn boundary_grid_matches_interpreted_and_warm_upstream() {
    let values = [
        -100_000.0,
        -100.0,
        -1.0,
        -0.5,
        -f64::MIN_POSITIVE,
        -0.0,
        0.0,
        f64::MIN_POSITIVE,
        0.5,
        1.0_f64.next_down(),
        1.0,
        1.0_f64.next_up(),
        5.0,
        100.0,
        1_000.0,
        100_000.0,
        1e100,
        f64::MAX,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
    ];
    for warm in [false, true] {
        for constants in [
            pinned_constants(),
            DefenceConstants {
                armour_ratio: 5.0,
                deflection_chance_cap: 75.0,
            },
            DefenceConstants {
                armour_ratio: 20.0,
                deflection_chance_cap: 0.0,
            },
        ] {
            let oracle = Oracle::new(constants, warm);
            for a in values {
                for b in values {
                    check_pair(&oracle, constants, a, b);
                }
            }
        }
    }
}

#[test]
fn threshold_neighbours_and_singular_inputs_match_upstream() {
    let oracle = Oracle::new(pinned_constants(), false);
    // Resolve ratings around every integer-percentage rounding transition.
    for accuracy in [1.0, 100.0, 10_000.0] {
        for percentage in 5..100 {
            let raw_chance = f64::from(percentage) + 0.5;
            let evasion = (125.0 * accuracy / raw_chance - accuracy) / 0.3;
            for rating in [evasion.next_down(), evasion, evasion.next_up()] {
                check_pair(&oracle, pinned_constants(), rating, accuracy);
            }
        }
    }
    for (armour, raw) in [(10.0, -1.0), (-10.0, -1.0), (0.0, 0.0)] {
        check_pair(&oracle, pinned_constants(), armour, raw);
    }
    // Invalid constants are not silently validated or replaced by defaults.
    for constants in [
        DefenceConstants {
            armour_ratio: 0.0,
            deflection_chance_cap: -1.0,
        },
        DefenceConstants {
            armour_ratio: f64::NAN,
            deflection_chance_cap: f64::NAN,
        },
    ] {
        let oracle = Oracle::new(constants, false);
        for pair in [(0.0, 0.0), (1.0, 100.0), (-1.0, 100.0)] {
            check_pair(&oracle, constants, pair.0, pair.1);
        }
    }
}

#[test]
fn positive_domain_invariants_hold_across_rating_ranges() {
    for accuracy in [1.0, 10.0, 100.0, 1_000.0, 100_000.0] {
        let mut last_hit = 100.0;
        let mut last_monster_hit = 100.0;
        let mut last_deflect = 0.0;
        let mut last_armour = 0.0;
        for rating in 0..=2_000 {
            let rating = f64::from(rating) * 50.0;
            let hit = hit_chance(rating, accuracy, false);
            let monster_hit = monster_hit_chance(rating, accuracy);
            let deflect = deflect_chance(rating, accuracy, pinned_constants());
            let armour = armour_reduction_percent(rating, accuracy, pinned_constants());
            assert!((5.0..=100.0).contains(&hit) && hit <= last_hit);
            assert!((5.0..=100.0).contains(&monster_hit) && monster_hit <= last_monster_hit);
            assert!((0.0..=95.0).contains(&deflect) && deflect >= last_deflect);
            assert!((0.0..100.0).contains(&armour) && armour >= last_armour);
            if rating > 0.0 {
                assert_eq!(
                    armour_reduction_percent(-rating, accuracy, pinned_constants()),
                    -armour
                );
            }
            last_hit = hit;
            last_monster_hit = monster_hit;
            last_deflect = deflect;
            last_armour = armour;
        }
    }
    assert_eq!(hit_chance(0.0, 100.0, true), 125.0);
    assert_eq!(armour_reduction_percent(0.0, 0.0, pinned_constants()), 0.0);
}
