//! Original helper versus production native scalar operations. This finite grid
//! is separate from full-build consumer acquisition and proves no game domain.
use super::native_rounding::NativeLifeRounding;
use mlua::{Function, Lua, Table};
use poe_optimizer_pob::source;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, path::Path};

const PATH: &str = "src/Modules/Common.lua";
const TWO52: f64 = 4_503_599_627_370_496.0;
const TWO53: f64 = 9_007_199_254_740_992.0;
const ANCHORS: [f64; 18] = [
    0.0,
    1.0,
    2.0,
    3.0,
    15.0,
    255.0,
    1023.0,
    1250.0,
    1312.0,
    1320.0,
    4095.0,
    65535.0,
    1_000_000.0,
    2_147_483_648.0,
    1_099_511_627_776.0,
    2_251_799_813_685_248.0,
    TWO52,
    TWO53,
];

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bits(value: f64) -> String {
    format!("{:016x}", value.to_bits())
}

fn grid() -> Vec<f64> {
    let mut values = BTreeSet::new();
    for anchor in ANCHORS {
        for offset in [0.0, 0.5] {
            for center in [anchor + offset, -(anchor + offset)] {
                for value in [center.next_down(), center, center.next_up()] {
                    assert!(value.is_finite());
                    values.insert(value.to_bits());
                }
            }
        }
    }
    // Deduplicate by bits so +0/-0 and adjacent representable inputs survive.
    values.into_iter().map(f64::from_bits).collect()
}

fn original_round(text: &str) -> &str {
    let first = "function round(val, dec)\n";
    assert_eq!(text.matches(first).count(), 1);
    let start = text.find(first).unwrap();
    assert_eq!(
        text[..start].bytes().filter(|b| *b == b'\n').count() + 1,
        722
    );
    let length = text[start..].find("\nend\n").unwrap() + "\nend\n".len();
    let body = &text[start..start + length];
    assert_eq!(body.lines().count(), 7);
    assert!(body.ends_with("\tend\nend\n"));
    body
}

fn expected_kind(input: f64) -> &'static str {
    if input.to_bits() == 0.5_f64.next_down().to_bits() {
        "below-half-addition-masked-by-minimum"
    } else if input.abs() >= TWO52 && input.abs() < TWO53 && input.abs() as u64 % 2 == 1 {
        if input.is_sign_positive() {
            "large-integer-divergence"
        } else {
            "negative-large-integer-masked-by-minimum"
        }
    } else {
        "equal-numeric-results"
    }
}

fn run(extracted: &str, inputs: &[f64], jit: bool) -> Vec<Value> {
    let lua = Lua::new();
    lua.load(if jit {
        "jit.on();jit.flush()"
    } else {
        "jit.off();jit.flush()"
    })
    .exec()
    .unwrap();
    let enabled: bool = lua.load("return jit.status()").eval().unwrap();
    assert_eq!(enabled, jit);
    let math: Table = lua.globals().get("math").unwrap();
    let floor: Function = math.get("floor").unwrap();
    let maximum: Function = math.get("max").unwrap();
    // The original body is loaded verbatim. Its only supplied global dependency
    // is math.floor under the exact original upvalue name. There is no source
    // loader, replacement business method or alternate rounding implementation.
    let env = lua.create_table().unwrap();
    env.set("m_floor", floor.clone()).unwrap();
    let original: Function = lua
        .load(format!("{extracted}\nreturn round"))
        .set_name("@verified-Common.lua-round-extract")
        .set_environment(env.clone())
        .eval()
        .unwrap();
    assert_eq!(env.get::<Function>("m_floor").unwrap(), floor);
    assert_eq!(env.clone().pairs::<ValueKey, mlua::Value>().count(), 2);
    let mut native = NativeLifeRounding::new();
    let mut rows = Vec::new();
    for &input in inputs {
        let rounded: f64 = original.call(input).unwrap();
        let clamped: f64 = maximum.call((rounded, 1.0)).unwrap();
        let (native_rounded, native_clamped) = native.evaluate(input);
        let kind = expected_kind(input);
        match kind {
            "below-half-addition-masked-by-minimum" => {
                assert_eq!((rounded, native_rounded), (1.0, 0.0));
                assert_eq!((clamped, native_clamped), (1.0, 1.0));
            }
            "large-integer-divergence" | "negative-large-integer-masked-by-minimum" => {
                assert_eq!(rounded, input + 1.0);
                assert_eq!(native_rounded, input);
                if input > 0.0 {
                    assert_ne!(clamped, native_clamped);
                } else {
                    assert_eq!((clamped, native_clamped), (1.0, 1.0));
                }
            }
            "equal-numeric-results" => {
                assert_eq!(
                    (rounded, clamped),
                    (native_rounded, native_clamped),
                    "{input:?}"
                );
            }
            _ => unreachable!(),
        }
        rows.push(json!({"input":input,"input_bits":bits(input),"classification":kind,
            "source":{"rounded":rounded,"rounded_bits":bits(rounded),"minimum_one":clamped,"minimum_one_bits":bits(clamped)},
            "native":{"rounded":native_rounded,"rounded_bits":bits(native_rounded),"minimum_one":native_clamped,"minimum_one_bits":bits(native_clamped)},
            "rounded_numeric_equal":rounded==native_rounded,"minimum_one_equal":clamped==native_clamped}));
    }
    // The same exact inputs in reverse order must retain each operation's result;
    // this exercises reused native scratch without asserting game contribution
    // order or acquiring another source model.
    for (&input, expected) in inputs.iter().zip(&rows).rev() {
        let rounded: f64 = original.call(input).unwrap();
        let clamped: f64 = maximum.call((rounded, 1.0)).unwrap();
        let (nr, nc) = native.evaluate(input);
        assert_eq!(bits(rounded), expected["source"]["rounded_bits"]);
        assert_eq!(bits(clamped), expected["source"]["minimum_one_bits"]);
        assert_eq!(bits(nr), expected["native"]["rounded_bits"]);
        assert_eq!(bits(nc), expected["native"]["minimum_one_bits"]);
    }
    assert_eq!(lua.load("return jit.status()").eval::<bool>().unwrap(), jit);
    rows
}

// Only the two named globals may enter the isolated extraction environment.
type ValueKey = mlua::Value;

pub fn probe(root: &Path) -> Value {
    let source = source::read_verified_text(root, PATH).unwrap();
    let source_hash = hash(source.as_bytes());
    assert_eq!(source_hash, source::expected_file_sha256(PATH).unwrap());
    let extracted = original_round(&source);
    let inputs = grid();
    assert!(inputs.len() < 512);
    for required in [
        0.0,
        -0.0,
        f64::from_bits(1),
        -f64::from_bits(1),
        0.5_f64.next_down(),
        0.5,
        0.5_f64.next_up(),
        1312.5_f64.next_down(),
        1312.5,
        1312.5_f64.next_up(),
        TWO52 + 1.0,
        TWO53 - 1.0,
    ] {
        assert!(inputs.iter().any(|v| v.to_bits() == required.to_bits()));
    }
    let off = run(extracted, &inputs, false);
    let on = run(extracted, &inputs, true);
    assert_eq!(
        off, on,
        "exact configured-JIT scalar helper and native results"
    );
    let masked = off
        .iter()
        .filter(|r| r["rounded_numeric_equal"] == false && r["minimum_one_equal"] == true)
        .count();
    let divergent = off
        .iter()
        .filter(|r| r["minimum_one_equal"] == false)
        .count();
    assert!(masked > 0 && divergent > 0);
    json!({"schema_version":1,"status":"passed","protocol":"verified-original-helper-and-native-scalar-grid",
        "source":{"path":PATH,"normalized_sha256":source_hash,"first_line":722,"last_line":728,
            "extracted_sha256":hash(extracted.as_bytes()),"extracted_bytes":extracted.len(),"dependency":"m_floor = math.floor"},
        "scope":{"player_consumer":"CalcDefence.lua:96","decimal_argument_supplied":false,
            "source_body_verbatim":true,"whole_source_file_executed":false,"full_builds_loaded":0,
            "native":"production Round(NearestTiesPositive, quantum=1) then Maximum(1)",
            "finite_regression_only":true,"whole_float_equivalence":false,"game_input_domain_proved":false,
            "final_life_reducer":false,"game_contributor_coverage":false,"source_compatibility_semantics_adopted":false,
            "signed_zero_inputs_retained_by_bits":true,"output_bits_retained":true,
            "nonfinite_inputs_excluded":true,"configured_jit_is_not_trace_coverage":true},
        "grid":{"anchors":ANCHORS,"offsets":[0.0,0.5],"signs":[1,-1],"neighbors":"next_down, identity, next_up",
            "deduplication":"input f64 bits","max_cases":512,"minimum_input":inputs.iter().copied().reduce(f64::min).unwrap(),
            "maximum_input":inputs.iter().copied().reduce(f64::max).unwrap()},
        "fresh_lua_environments":2,"jit_modes":[false,true],"exact_cross_jit_agreement":true,
        "reverse_reuse_exact":true,"cases_per_mode":off.len(),"masked_rounding_differences":masked,
        "retained_final_differences":divergent,"cases":off})
}
