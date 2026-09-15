//! Production assembler CLI from explicit persisted files; no source/game runtime.
#[path = "../crates/poe-optimizer-import/tests/support/owned_recipe_fixture.rs"]
mod fixture;
use poe_optimizer_data::{
    owned_routing::{RoutingLimits, decode_action_routing},
    owned_rules::{RuleStorageLimits, decode_rule_package},
    owned_schema::{OwnedSchemaLimits, decode_schema_package},
};
use poe_optimizer_import::owned_mapping::{OwnedMappingLimits, decode_registry};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
fn run(dir: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn emitted_artifacts_reload_through_public_consumers_without_source_inputs() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("recipe.json"),
        serde_json::to_vec(&fixture::recipe(7)).unwrap(),
    )
    .unwrap();
    let output = run(
        temp.path(),
        &[
            "assemble-owned-recipe",
            "recipe.json",
            "--output",
            "assembled",
        ],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(manifest["calculation"], "not_run");
    assert_eq!(manifest["whole_build_parity"], "not_established");
    let dir = temp.path().join("assembled");
    let registry = decode_registry(
        &fs::read(dir.join("registry.json")).unwrap(),
        OwnedMappingLimits::default(),
    )
    .unwrap();
    assert_eq!(registry.input(), &fixture::recipe(7).registry);
    let schema = decode_schema_package(
        &fs::read(dir.join("schema.json")).unwrap(),
        OwnedSchemaLimits::default(),
    )
    .unwrap();
    let rules = decode_rule_package(
        &fs::read(dir.join("rules.json")).unwrap(),
        &schema,
        RuleStorageLimits::default(),
    )
    .unwrap();
    let routing = decode_action_routing(
        &fs::read(dir.join("routing.json")).unwrap(),
        &schema,
        RoutingLimits::default(),
    )
    .unwrap();
    assert_eq!(
        manifest["rules"],
        serde_json::to_value(rules.identity()).unwrap()
    );
    assert_eq!(
        manifest["routing"],
        serde_json::to_value(routing.identity()).unwrap()
    );
    assert_eq!(fs::read_dir(&dir).unwrap().count(), 5);
    let check = run(
        temp.path(),
        &[
            "check-owned-rules",
            "assembled/rules.json",
            "--definitions",
            "assembled/schema.json",
        ],
    );
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let checked: serde_json::Value = serde_json::from_slice(&check.stdout).unwrap();
    assert_eq!(checked["verification"]["operations"], "compiled");
    let before = fs::read(dir.join("manifest.json")).unwrap();
    assert!(
        !run(
            temp.path(),
            &[
                "assemble-owned-recipe",
                "recipe.json",
                "--output",
                "assembled"
            ]
        )
        .status
        .success()
    );
    assert_eq!(fs::read(dir.join("manifest.json")).unwrap(), before);
}
#[test]
fn invalid_or_stale_recipes_leave_no_output_or_staging_directory() {
    let temp = tempfile::tempdir().unwrap();
    let mut input = fixture::recipe(7);
    input.rules.definitions.content_sha256 = "ab".repeat(32);
    fs::write(
        temp.path().join("recipe.json"),
        serde_json::to_vec(&input).unwrap(),
    )
    .unwrap();
    let output = run(
        temp.path(),
        &[
            "assemble-owned-recipe",
            "recipe.json",
            "--output",
            "assembled",
        ],
    );
    assert!(!output.status.success());
    assert!(!temp.path().join("assembled").exists());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    fs::write(
        temp.path().join("recipe.json"),
        b"{\"schema_version\":1,\"schema_version\":1}",
    )
    .unwrap();
    assert!(
        !run(
            temp.path(),
            &[
                "assemble-owned-recipe",
                "recipe.json",
                "--output",
                "assembled"
            ]
        )
        .status
        .success()
    );
    assert!(!temp.path().join("assembled").exists());
}
#[test]
fn help_and_existing_empty_file_or_directory_never_clobber() {
    let temp = tempfile::tempdir().unwrap();
    let help = run(temp.path(), &["assemble-owned-recipe", "--help"]);
    assert!(help.status.success());
    assert!(String::from_utf8_lossy(&help.stdout).contains("--output"));
    fs::write(
        temp.path().join("recipe.json"),
        serde_json::to_vec(&fixture::recipe(7)).unwrap(),
    )
    .unwrap();
    fs::create_dir(temp.path().join("empty")).unwrap();
    fs::write(temp.path().join("file"), b"sentinel").unwrap();
    for destination in ["empty", "file"] {
        assert!(
            !run(
                temp.path(),
                &[
                    "assemble-owned-recipe",
                    "recipe.json",
                    "--output",
                    destination
                ]
            )
            .status
            .success()
        );
    }
    assert_eq!(fs::read_dir(temp.path().join("empty")).unwrap().count(), 0);
    assert_eq!(fs::read(temp.path().join("file")).unwrap(), b"sentinel");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
}
