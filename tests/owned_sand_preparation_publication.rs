//! Publish authored preparation while preserving all five real build inputs.
#[path = "support/owned_release_migration_preservation.rs"]
mod migration_preservation;
#[allow(dead_code)]
#[path = "support/owned_physical_inventory_preservation.rs"]
mod preservation;
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_sand_preparation_publication.rs"]
mod sand;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
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
fn authored_sand_preparation_retains_partial_coverage_and_only_corrects_proven_roles() {
    sand::check_authored();
}

#[test]
#[ignore = "requires SAND_PREPARATION_PRIOR, retained source witnesses and fresh SAND_PREPARATION_OUTPUT"]
fn publish_sand_preparation_preserving_all_five_originals() {
    let path = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    let prior_path = path("POE_OPTIMIZER_TEST_SAND_PREPARATION_PRIOR");
    let out = path("POE_OPTIMIZER_TEST_SAND_PREPARATION_OUTPUT");
    assert!(!out.exists(), "publication evidence is immutable");
    let prior_inventory = release::inventory(&prior_path);
    let authoring_inventory = release::inventory(&sand::data());
    let prior = release::load(&prior_path);
    // Authentication and checked migration finish before creating output.
    let next = sand::stage(&prior);
    fs::create_dir_all(&out).unwrap();
    let endpoint = out.join("endpoint.json");
    fs::write(&endpoint, serde_json::to_vec(next.input()).unwrap()).unwrap();
    let package = out.join("package");
    assert_eq!(publish(&endpoint, &package), json!(next.receipt()));
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let published = release::load(&package);
    assert_eq!(published.receipt(), next.receipt());
    let package_inventory = release::inventory(&package);
    assert_eq!(package_inventory.len(), 18);
    assert_eq!(package_inventory, release::inventory(&out.join("rebuilt")));
    assert_eq!(
        package_inventory.keys().collect::<Vec<_>>(),
        prior_inventory.keys().collect::<Vec<_>>()
    );
    assert!(next.evaluation().is_none());
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.query_sets(), prior.query_sets());
    // No offline proof, Partial receiving fragment or source program is shipped.
    for name in [
        "extension.json",
        "corrections.json",
        "dependencies.json",
        "source-properties.json",
        "readiness.json",
        "source-vectors.json",
    ] {
        assert!(!package.join(name).exists());
    }
    assert_eq!(
        read(package.join("release.json"))["provenance"],
        json!(next.receipt().provenance)
    );
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &[],
        selected_before: [107, 117, 109, 123, 5],
        selected_after: [107, 117, 109, 123, 5],
        rebind_definitions: true,
    };
    let mut cases = vec![];
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&xml).unwrap();
        let before = out.join(format!("prior-original-{case:02}"));
        let after = out.join(format!("original-{case:02}"));
        release::normalize(&prior_path, &xml, case, &before);
        release::normalize(&package, &xml, case, &after);
        let row = preservation::compare_original(case, &bytes, &comparison);
        assert_eq!(row["physical_lists_completed"], 0);
        let replay = out.join(format!("replay-{case:02}"));
        release::normalize(&package, &xml, case, &replay);
        preservation::authenticate(
            &read(replay.join("sidecar.json")),
            &replay,
            &package,
            case,
            &next,
        );
        for name in ["draft.json", "sidecar.json"] {
            let mut a = read(after.join(name));
            let mut b = read(replay.join(name));
            selected::canonical(&mut a);
            selected::canonical(&mut b);
            if name == "sidecar.json" {
                a.as_object_mut().unwrap().remove("draft");
                b.as_object_mut().unwrap().remove("draft");
            }
            assert_eq!(a, b, "independent repeat import: case {case}, {name}");
        }
        assert_eq!(fs::read(xml).unwrap(), bytes);
        cases.push(row);
    }
    assert_eq!(prior_inventory, release::inventory(&prior_path));
    assert_eq!(authoring_inventory, release::inventory(&sand::data()));
    fs::write(
        out.join("validation.json"),
        serde_json::to_vec_pretty(&json!({
            "before":prior.receipt().input,"after":next.receipt().input,"originals":cases,
            "new_definitions":3,"new_parameter_slots":2,"corrected_grant_roles":2,
            "appended_programs":4,"retired_input_issues":0,"complete_original_builds":0,
            "evaluation_bundle_added":false,"source_property_inventory_complete":false,
            "queries":110,"artifacts":18,"rebuild_byte_identical":true,
            "independent_import_replay":true,"prior_unchanged":true,"authoring_unchanged":true
        }))
        .unwrap(),
    )
    .unwrap();
}
