//! Global Energy Shield inputs; complete source inventory and coverage stay explicit.
#[path = "support/owned_global_energy_shield_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[path = "support/owned_global_energy_shield_native.rs"]
mod native;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_global_energy_shield_probes.rs"]
mod probes;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_global_energy_shield_validation.rs"]
mod validation;

use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn authored_global_energy_shield_inputs_preserve_open_coverage() {
    family::check_authored();
}

fn publish(input: &Path, output: &Path) -> Value {
    let result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}

#[test]
#[ignore = "requires GLOBAL_ENERGY_SHIELD_INPUTS_PRIOR, fresh GLOBAL_ENERGY_SHIELD_INPUTS_OUTPUT and retained source reports"]
fn publish_global_energy_shield_inputs_preserving_originals() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GLOBAL_ENERGY_SHIELD_INPUTS_PRIOR")
            .expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GLOBAL_ENERGY_SHIELD_INPUTS_OUTPUT")
            .expect("fresh output"),
    );
    assert!(!out.exists(), "immutable evidence destination");
    let prior_inventory = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let next = family::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    fs::write(
        out.join("endpoint.json"),
        serde_json::to_vec(next.input()).unwrap(),
    )
    .unwrap();
    fs::write(
        out.join("receipt.json"),
        serde_json::to_vec(next.receipt()).unwrap(),
    )
    .unwrap();
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    assert_eq!(
        release::inventory(&package).keys().collect::<Vec<_>>(),
        prior_inventory.keys().collect::<Vec<_>>()
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    // Produce every immutable import before comparison, so failures retain the
    // complete diagnostic corpus rather than only the first changed build.
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        release::normalize(
            &prior_path,
            &xml,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &xml,
            case,
            &out.join(format!("original-{case:02}")),
        );
    }
    let originals: Vec<_> = (1..=5)
        .map(|case| validation::compare_original(case, &prior, &next, &prior_path, &package, &out))
        .collect();
    let probe_count = probes::run(&package, &out);
    let native = native::run(&package);
    assert_eq!(release::inventory(&prior_path), prior_inventory);
    fs::write(out.join("validation.json"), serde_json::to_vec_pretty(&json!({"originals":originals,"query_rows":110,"import_probes":probe_count,"native_components":native,"calculation":"not_run","full_build_parity":false})).unwrap()).unwrap();
}
