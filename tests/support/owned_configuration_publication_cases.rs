//! Explicit import expectations, separate from the publication machinery.
use super::{Control, Expectations, LeafRole, Output};
use LeafRole::{Contributes as C, IgnoredPlaceholder as I, UnselectedProjection as U};
use serde_json::json;

type Row = (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    Option<Vec<f64>>,
    [Option<LeafRole>; 2],
);
fn controls(rows: Vec<Row>) -> Vec<Control> {
    rows.into_iter()
        .map(|(name, input, placeholder, values, roles)| Control {
            name,
            input,
            placeholder,
            values,
            roles,
        })
        .collect()
}
pub fn ignored_enemy(default: f64) -> Expectations {
    Expectations {
        outputs: vec![Output {
            policy_target: "enemy",
            draft_target: json!({"kind":"enemy"}),
            selection: None,
            ignore_placeholder: true,
        }],
        default,
        original_placeholder: I,
        controls: controls(vec![
            (
                "changed-placeholder",
                None,
                Some("999"),
                Some(vec![default]),
                [None, Some(I)],
            ),
            (
                "zero",
                Some("0"),
                Some("999"),
                Some(vec![0.]),
                [Some(C), Some(I)],
            ),
            (
                "fraction",
                Some("-12.5"),
                Some("999"),
                Some(vec![-12.5]),
                [Some(C), Some(I)],
            ),
            (
                "above-probability",
                Some("150"),
                Some("999"),
                Some(vec![150.]),
                [Some(C), Some(I)],
            ),
            (
                "zero-placeholder",
                None,
                Some("0"),
                Some(vec![default]),
                [None, Some(I)],
            ),
            ("absent", None, None, Some(vec![default]), [None, None]),
            (
                "bad-placeholder",
                Some("0"),
                Some("NaN"),
                None,
                [None, None],
            ),
        ]),
    }
}
pub fn distance() -> Expectations {
    Expectations {
        outputs: ["input_then_placeholder", "nonzero_input_then_placeholder"]
            .into_iter()
            .map(|selection| Output {
                policy_target: "enemy",
                draft_target: json!({"kind":"enemy"}),
                selection: Some(selection),
                ignore_placeholder: false,
            })
            .collect(),
        default: 20.,
        original_placeholder: C,
        controls: controls(vec![
            (
                "changed-placeholder",
                None,
                Some("80"),
                Some(vec![80., 80.]),
                [None, Some(C)],
            ),
            (
                "zero",
                Some("0"),
                Some("80"),
                Some(vec![0., 80.]),
                [Some(C), Some(C)],
            ),
            (
                "negative-zero",
                Some("-0"),
                Some("80"),
                Some(vec![0., 80.]),
                [Some(C), Some(C)],
            ),
            (
                "fraction",
                Some("-12.5"),
                Some("80"),
                Some(vec![-12.5, -12.5]),
                [Some(C), Some(U)],
            ),
            (
                "nonzero",
                Some("50"),
                Some("80"),
                Some(vec![50., 50.]),
                [Some(C), Some(U)],
            ),
            (
                "zero-placeholder",
                None,
                Some("0"),
                Some(vec![0., 0.]),
                [None, Some(C)],
            ),
            (
                "both-zero",
                Some("0"),
                Some("0"),
                Some(vec![0., 0.]),
                [Some(C), Some(C)],
            ),
            (
                "zero-constructor",
                Some("0"),
                None,
                Some(vec![0., 20.]),
                [Some(C), None],
            ),
            ("absent", None, None, Some(vec![20., 20.]), [None, None]),
            (
                "boundary",
                Some("1000000"),
                Some("-1000000"),
                Some(vec![1000000., 1000000.]),
                [Some(C), Some(U)],
            ),
            (
                "bad-placeholder",
                Some("0"),
                Some("NaN"),
                None,
                [None, None],
            ),
            (
                "losing-outside-range",
                Some("50"),
                Some("1000001"),
                None,
                [None, None],
            ),
            (
                "input-outside-range",
                Some("-1000001"),
                Some("20"),
                None,
                [None, None],
            ),
        ]),
    }
}
pub fn player_withered() -> Expectations {
    Expectations {
        outputs: vec![Output {
            policy_target: "player",
            draft_target: json!({"kind":"actor","value":{"kind":"player"}}),
            selection: Some("input_then_placeholder"),
            ignore_placeholder: false,
        }],
        default: 15.,
        original_placeholder: C,
        controls: controls(vec![
            (
                "changed-placeholder",
                None,
                Some("25"),
                Some(vec![25.]),
                [None, Some(C)],
            ),
            (
                "zero",
                Some("0"),
                Some("15"),
                Some(vec![0.]),
                [Some(C), Some(U)],
            ),
            (
                "negative-zero",
                Some("-0"),
                Some("15"),
                Some(vec![0.]),
                [Some(C), Some(U)],
            ),
            (
                "fraction",
                Some("-12.5"),
                Some("15"),
                Some(vec![-12.5]),
                [Some(C), Some(U)],
            ),
            (
                "above-effective-cap",
                Some("25"),
                Some("15"),
                Some(vec![25.]),
                [Some(C), Some(U)],
            ),
            (
                "fractional-placeholder",
                None,
                Some("0.5"),
                Some(vec![0.5]),
                [None, Some(C)],
            ),
            (
                "zero-placeholder",
                None,
                Some("0"),
                Some(vec![0.]),
                [None, Some(C)],
            ),
            (
                "both-zero",
                Some("0"),
                Some("0"),
                Some(vec![0.]),
                [Some(C), Some(U)],
            ),
            (
                "zero-constructor",
                Some("0"),
                None,
                Some(vec![0.]),
                [Some(C), None],
            ),
            ("absent", None, None, Some(vec![15.]), [None, None]),
            (
                "boundary",
                Some("1000000"),
                Some("-1000000"),
                Some(vec![1000000.]),
                [Some(C), Some(U)],
            ),
            (
                "negative-boundary",
                None,
                Some("-1000000"),
                Some(vec![-1000000.]),
                [None, Some(C)],
            ),
            (
                "bad-placeholder",
                Some("0"),
                Some("NaN"),
                None,
                [None, None],
            ),
            ("bad-input", Some("NaN"), Some("15"), None, [None, None]),
            (
                "losing-outside-range",
                Some("50"),
                Some("1000001"),
                None,
                [None, None],
            ),
            (
                "input-outside-range",
                Some("-1000001"),
                Some("15"),
                None,
                [None, None],
            ),
        ]),
    }
}
