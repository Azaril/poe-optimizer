//! Authored group/occurrence preferences and the real Sniper preparation path.
#[path = "support/owned_skill_participation.rs"]
mod family;
#[allow(dead_code)]
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[path = "support/owned_skill_participation_native.rs"]
mod native;
#[path = "support/owned_skill_participation_preservation.rs"]
mod participation_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
#[allow(dead_code)]
#[path = "support/owned_sniper_final_inputs.rs"]
mod sniper_family;
use serde_json::{Value, json};
use std::{fs, path::PathBuf, process::Command};

#[test]
fn participation_uses_two_required_inputs_and_one_early_boolean_without_supply_changes() {
    family::check_authored();
}

#[test]
#[ignore = "requires retained authenticated participation source reports"]
fn participation_source_projection_preserves_preview_and_exact_source_identity() {
    family::source(true);
}

#[test]
#[ignore = "requires current SKILL_PARTICIPATION_PRIOR and fresh OUTPUT"]
fn publish_participation_policy_against_current_usage_transport() {
    let prior = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKILL_PARTICIPATION_PRIOR")
            .expect("current predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_SKILL_PARTICIPATION_OUTPUT").expect("fresh output"),
    );
    assert!(!out.exists());
    let prior_files = release::inventory(&prior);
    let previous = release::load(&prior);
    let next = family::stage(&previous);
    fs::create_dir_all(&out).unwrap();
    let input = out.join("endpoint.json");
    fs::write(&input, serde_json::to_vec(next.input()).unwrap()).unwrap();
    fs::write(
        out.join("receipt.json"),
        serde_json::to_vec(next.receipt()).unwrap(),
    )
    .unwrap();
    let publish = |input: &std::path::Path, target: &std::path::Path| {
        let r = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .arg("assemble-owned-release")
            .arg(input)
            .arg("--output")
            .arg(target)
            .output()
            .unwrap();
        assert!(r.status.success(), "{}", String::from_utf8_lossy(&r.stderr));
        let v: Value = serde_json::from_slice(&r.stdout).unwrap();
        assert_eq!(v, json!(next.receipt()));
    };
    let package = out.join("package");
    publish(&input, &package);
    publish(&package, &out.join("rebuilt"));
    assert_eq!(
        release::inventory(&package),
        release::inventory(&out.join("rebuilt"))
    );
    family::assert_endpoint(&release::load(&package));
    let cases = participation_preservation::originals(&previous, &next, &prior, &package, &out);
    let controls = participation_preservation::controls(&next, &package, &out);
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec_pretty(&json!({
            "before":previous.receipt().input,"after":next.receipt().input,"queries":110,
            "cases":cases,"controls":controls,"complete_native_builds":0,
            "physical_availability_changed":false,"source_preview_imported":false
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(prior_files, release::inventory(&prior));
}
