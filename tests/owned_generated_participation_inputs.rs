//! Saved participation transport only; generated preparation remains incomplete.
#[path = "support/owned_generated_participation_inputs.rs"]
mod family;
#[path = "support/owned_identity_correspondence.rs"]
mod identity;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[allow(dead_code)]
#[path = "support/owned_selected_request.rs"]
mod selected;
#[path = "support/owned_generated_participation_source.rs"]
mod source;
#[path = "support/owned_generated_participation_validation.rs"]
mod validation;

use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

#[test]
fn authored_generated_participation_reuses_existing_policy_without_readiness() {
    family::check_authored();
    source::verify_source(false);
}

#[test]
#[ignore = "requires retained JIT source reports and baseline imports"]
fn retained_generated_participation_source_is_exact() {
    source::verify_source(true);
}

fn publish(input: &Path, output: &Path) -> Value {
    let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap();
    assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
    serde_json::from_slice(&r.stdout).unwrap()
}

#[test]
#[ignore = "requires GENERATED_PARTICIPATION_PRIOR and fresh GENERATED_PARTICIPATION_OUTPUT"]
fn publish_generated_participation_preserving_all_five_builds() {
    let prior_path = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_PARTICIPATION_PRIOR")
            .expect("checked predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_GENERATED_PARTICIPATION_OUTPUT")
            .expect("fresh output"),
    );
    assert!(!out.exists(), "immutable evidence destination");
    source::verify_source(true);
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    let packet = validation::packet();
    let next = family::stage(&prior, &packet);
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
    assert_eq!(prior.query_sets(), next.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    for case in 1..=5 {
        let xml = validation::root().join(format!(
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
        .map(|case| validation::original(case, &prior, &next, &prior_path, &package, &out))
        .collect();
    let controls = validation::controls(&next, &package, &out);
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec_pretty(&json!({
            "originals":originals,"controls":controls,"queries":110,
            "complete_native_builds":0,"generated_readiness_published":false,
            "calculation":"not_run","saved_participation_only":true
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(prior_files, release::inventory(&prior_path));
}
