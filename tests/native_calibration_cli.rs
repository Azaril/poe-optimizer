//! Keep the independent attack vectors after retiring finite-catalog search.
use serde_json::Value;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn dps(report: &Value) -> f64 {
    let measurements = report["evaluation"]["measurements"].as_array().unwrap();
    assert_eq!(measurements.len(), 1);
    assert_eq!(measurements[0]["query"]["id"], "selected_hit_dps");
    assert_eq!(measurements[0]["unit"], "damage_per_second");
    measurements[0]["value"]["value"].as_f64().unwrap()
}
#[test]
fn four_attack_calibrations_and_fresh_exports_match_independent_reference_numbers() {
    let temp = tempfile::tempdir().unwrap();
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/calibration");
    let mut scores = Vec::new();
    for name in [
        "mace-wooden",
        "mace-wooden-brutality",
        "mace-smithing",
        "mace-smithing-brutality",
    ] {
        let reference: Value = serde_json::from_slice(
            &fs::read(fixtures.join(format!("{name}.reference.json"))).unwrap(),
        )
        .unwrap();
        let expected = reference["metrics"]["TotalDPS"].as_f64().unwrap();
        let export = temp.path().join(format!("{name}.xml"));
        let result = success(
            Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .current_dir(temp.path())
                .arg("evaluate")
                .arg(fixtures.join(format!("{name}.xml")))
                .args([
                    "--backend",
                    "native",
                    "--metric",
                    "player.selected_hit_dps",
                    "--export",
                ])
                .arg(&export)
                .output()
                .unwrap(),
        );
        let actual = dps(&result);
        assert!(
            (actual - expected).abs() < 1e-8,
            "{name}: {actual} != {expected}"
        );
        let fresh = success(
            Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .current_dir(temp.path())
                .arg("evaluate")
                .arg(&export)
                .args(["--backend", "native", "--metric", "player.selected_hit_dps"])
                .output()
                .unwrap(),
        );
        assert_eq!(
            fresh["evaluation"]["backend"],
            result["evaluation"]["backend"]
        );
        assert_eq!(dps(&fresh), actual);
        scores.push(actual);
    }
    // Independent fixtures establish an interaction: the preferred weapon
    // changes with Brutality, so additive weapon ranking is invalid.
    assert!(scores[2] > scores[0]);
    assert!(scores[1] > scores[3]);
}
