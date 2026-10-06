//! Shared authoring publication/replay checks; no game-specific runtime path.
use super::{preservation, release};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
fn write(path: impl AsRef<Path>, value: &impl Serialize) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
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
pub fn carries_authoring(value: &Value, payloads: &[Value], effects: &[&str]) -> bool {
    if payloads.contains(value) {
        return true;
    }
    match value {
        Value::String(text) => effects
            .iter()
            .any(|effect| text.contains(&format!("skills[\"{effect}\"]"))),
        Value::Array(rows) => rows.iter().any(|r| carries_authoring(r, payloads, effects)),
        Value::Object(fields) => fields
            .values()
            .any(|r| carries_authoring(r, payloads, effects)),
        _ => false,
    }
}

#[allow(dead_code)] // New families may use explicit authoring payload inventories below.
pub fn run(
    prior_path: PathBuf,
    out: PathBuf,
    authoring: &Path,
    effects: &[&str],
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    extra: Value,
) {
    run_with_payloads(
        prior_path,
        out,
        authoring,
        effects,
        &["receiving.json", "preparation.json", "source-vectors.json"],
        stage,
        extra,
    );
}

/// Additional preparation fragments use the same release/preservation checks;
/// only their authoring-only payload names differ. Historical callers retain
/// their exact payload census through `run` above.
pub fn run_with_payloads(
    prior_path: PathBuf,
    out: PathBuf,
    authoring: &Path,
    effects: &[&str],
    payload_files: &[&str],
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    extra: Value,
) {
    run_with_scope(
        prior_path,
        out,
        authoring,
        effects,
        payload_files,
        stage,
        PublicationScope {
            closed_existing_rule_owners: 0,
            passive_refinement: false,
            extra,
        },
    );
}

/// Describes reviewed owner-inventory refinements, independently of new
/// Complete producer owners and the still-open whole-build contributor census.
pub struct PublicationScope {
    pub closed_existing_rule_owners: usize,
    pub passive_refinement: bool,
    pub extra: Value,
}

pub fn run_with_scope(
    prior_path: PathBuf,
    out: PathBuf,
    authoring: &Path,
    effects: &[&str],
    payload_files: &[&str],
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    scope: PublicationScope,
) {
    run_with_item_parameter_completions(
        prior_path,
        out,
        authoring,
        effects,
        payload_files,
        stage,
        scope,
        &[],
    );
}

/// Explicit diagnostic expectations opt in without weakening historical callers.
#[allow(clippy::too_many_arguments)] // Keep the existing scoped publication API unchanged.
pub fn run_with_item_parameter_completions(
    prior_path: PathBuf,
    out: PathBuf,
    authoring: &Path,
    effects: &[&str],
    payload_files: &[&str],
    stage: fn(&StagedOwnedRelease) -> StagedOwnedRelease,
    scope: PublicationScope,
    completions: &[preservation::ItemParameterCompletion],
) {
    assert!(!out.exists(), "evidence is immutable");
    let prior_files = release::inventory(&prior_path);
    let prior = release::load(&prior_path);
    // This gate includes exact source report authentication. An awaiting-source
    // packet must fail before the output directory or package is created.
    let next = stage(&prior);
    let closed = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .filter(|old| {
            !old.programs.is_complete()
                && next
                    .input()
                    .recipe
                    .rules
                    .owners
                    .iter()
                    .any(|new| new.owner == old.owner && new.programs.is_complete())
        })
        .count();
    assert_eq!(
        closed, scope.closed_existing_rule_owners,
        "exact prior Partial-to-Complete owner transitions; newly introduced owners are separate"
    );
    fs::create_dir_all(&out).unwrap();
    write(out.join("endpoint.json"), next.input());
    write(out.join("receipt.json"), next.receipt());
    let package = out.join("package");
    assert_eq!(
        publish(&out.join("endpoint.json"), &package),
        json!(next.receipt())
    );
    assert_eq!(
        publish(&package, &out.join("rebuilt")),
        json!(next.receipt())
    );
    let files = release::inventory(&package);
    assert_eq!(files.len(), 18);
    assert_eq!(
        files.keys().collect::<Vec<_>>(),
        prior_files.keys().collect::<Vec<_>>()
    );
    assert_eq!(files, release::inventory(&out.join("rebuilt")));
    let payloads: Vec<Value> = payload_files
        .iter()
        .map(|name| serde_json::from_slice(&fs::read(authoring.join(name)).unwrap()).unwrap())
        .collect();
    for name in files.keys() {
        assert!(
            ![
                "receiving.json",
                "preparation.json",
                "source-vectors.json",
                "authoring.json",
                "bindings.json",
                "dependencies.json",
                "migration.json"
            ]
            .contains(&name.as_str())
        );
        assert!(!payload_files.contains(&name.as_str()));
        let document: Value =
            serde_json::from_slice(&fs::read(package.join(name)).unwrap()).unwrap();
        assert!(
            !carries_authoring(&document, &payloads, effects),
            "authoring payload embedded in {name}"
        );
    }
    let published: Value =
        serde_json::from_slice(&fs::read(package.join("release.json")).unwrap()).unwrap();
    assert_eq!(published["provenance"], json!(next.receipt().provenance));
    assert_eq!(
        next.receipt().provenance.len(),
        prior.receipt().provenance.len() + 1
    );
    assert_eq!(
        next.receipt().provenance[..prior.receipt().provenance.len()],
        prior.receipt().provenance
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    let comparison = preservation::Comparison {
        prior: &prior,
        next: &next,
        prior_path: &prior_path,
        package: &package,
        out: &out,
        families: &[],
        selected_before: [106, 117, 109, 122, 5],
        selected_after: [106, 117, 109, 122, 5],
        rebind_definitions: true,
    };
    let mut originals = vec![];
    for case in 1..=5 {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let bytes = fs::read(&source).unwrap();
        release::normalize(
            &prior_path,
            &source,
            case,
            &out.join(format!("prior-original-{case:02}")),
        );
        release::normalize(
            &package,
            &source,
            case,
            &out.join(format!("original-{case:02}")),
        );
        let row = if completions.is_empty() {
            preservation::compare_original(case, &bytes, &comparison)
        } else {
            preservation::compare_original_with_item_parameter_completions(
                case,
                &bytes,
                &comparison,
                completions,
            )
        };
        assert_eq!(row["physical_lists_completed"], 0);
        assert_eq!(fs::read(&source).unwrap(), bytes);
        originals.push(row);
    }
    assert_eq!(prior_files, release::inventory(&prior_path));
    let mut validation = json!({
        "before":prior.receipt().input,"after":next.receipt().input,"originals":originals,
        "queries":110,"artifacts":18,"rebuild_byte_identical":true,"prior_unchanged":true,
        "closed_rule_owners":scope.closed_existing_rule_owners,"retired_input_issues":0,"complete_original_builds":0,
        "evaluation_bundle_added":false,"receiving_fragment_only":!scope.passive_refinement,
        "authoring_payloads_excluded":true,"provenance_preserved":true,
    });
    if !completions.is_empty() {
        let retired: usize = validation["originals"]
            .as_array()
            .unwrap()
            .iter()
            .map(|row| row["retired_item_parameter_diagnostics"].as_u64().unwrap() as usize)
            .sum();
        assert_eq!(retired, completions.len());
        validation["retired_item_parameter_diagnostics"] = json!(retired);
    }
    for (key, value) in scope.extra.as_object().unwrap() {
        assert!(
            validation
                .as_object_mut()
                .unwrap()
                .insert(key.clone(), value.clone())
                .is_none()
        );
    }
    write(out.join("validation.json"), &validation);
}
