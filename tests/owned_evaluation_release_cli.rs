//! A verified release supplies the same owned evaluator inputs as explicit files.
#[path = "support/owned_evaluation_release_fixture.rs"]
mod fixture;
use poe_optimizer_core::owned_build::{OwnedDocument, ParameterValue, encode_owned};
use poe_optimizer_engine::owned_plan::EffectValue;
use poe_optimizer_import::owned_release::{
    OWNED_RELEASE_VERSION, OwnedReleaseReceipt, assemble_owned_release,
};
use poe_optimizer_import::owned_release_migration::{
    OwnedReleaseContractMigration, OwnedReleaseMigrationInput, compile_owned_release_migration,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};

type Files = BTreeMap<String, Vec<u8>>;
fn read_files(root: &Path) -> Files {
    fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into_string().unwrap(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
fn write_files(root: &Path, files: &Files) {
    fs::create_dir(root).unwrap();
    for (name, bytes) in files {
        fs::write(root.join(name), bytes).unwrap();
    }
}
fn publish(input: &Path, output: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output)
        .output()
        .unwrap()
}
fn evaluate(input: &Path, release: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .arg("evaluate-owned")
        .arg("--input")
        .arg(input)
        .arg("--release")
        .arg(release);
    command
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn rejected(output: &Output) -> String {
    assert!(
        !output.status.success(),
        "unexpected result: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        output.stdout.is_empty(),
        "failure must not publish a numerical result"
    );
    String::from_utf8_lossy(&output.stderr).into_owned()
}
fn save_request(root: &Path, f: &fixture::Fixture) -> std::path::PathBuf {
    let request = root.join("request.json");
    fs::write(
        &request,
        encode_owned(
            &OwnedDocument::Request(Box::new(f.request.clone())),
            Default::default(),
        )
        .unwrap(),
    )
    .unwrap();
    request
}
fn valid_files(f: &fixture::Fixture) -> Files {
    assemble_owned_release(f.input.clone(), Default::default())
        .unwrap()
        .artifacts()
        .map(|(name, bytes)| (name.to_string(), bytes.to_vec()))
        .collect()
}
fn replace_hashed(files: &mut Files, name: &str, bytes: Vec<u8>) {
    let mut receipt: Value = serde_json::from_slice(&files["release.json"]).unwrap();
    let entry = receipt["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|row| row["file"] == name)
        .unwrap();
    entry["bytes"] = bytes.len().into();
    entry["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    files.insert(name.into(), bytes);
    files.insert("release.json".into(), serde_json::to_vec(&receipt).unwrap());
}

#[test]
fn full_publication_and_directory_rebuild_match_the_explicit_native_api() {
    let f = fixture::fixture();
    let temp = tempfile::tempdir().unwrap();
    let request = save_request(temp.path(), &f);
    let input = temp.path().join("release-input.json");
    fs::write(&input, serde_json::to_vec(&f.input).unwrap()).unwrap();
    let first = temp.path().join("published");
    let receipt: OwnedReleaseReceipt =
        serde_json::from_value(success(publish(&input, &first))).unwrap();
    let expected_files = valid_files(&f);
    assert_eq!(read_files(&first), expected_files);
    assert_eq!(receipt.schema_version, 2);
    for name in [
        "metrics.json",
        "support-stages.json",
        "support-preparation.json",
        "support-inputs.json",
        "support-receiving.json",
        "support-outputs.json",
    ] {
        assert!(
            receipt.artifacts.iter().any(|row| row.file == name),
            "{name}"
        );
    }
    let actual = success(evaluate(&request, &first).output().unwrap());
    assert_eq!(actual["schema_version"], 3);
    assert_eq!(
        actual["evaluation"],
        serde_json::to_value(&f.expected.evaluation).unwrap()
    );
    assert_eq!(
        actual["support_preparation"],
        serde_json::to_value(&f.expected.support).unwrap()
    );
    let EffectValue::Known {
        value: ParameterValue::Quantity(value),
    } = &f.expected.evaluation.results[0].value
    else {
        panic!("expected native known quantity: {:?}", f.expected);
    };
    assert_eq!(value.value(), 16.0);
    let second = temp.path().join("rebuilt");
    assert_eq!(
        success(publish(&first, &second)),
        serde_json::to_value(&receipt).unwrap()
    );
    assert_eq!(read_files(&second), expected_files);
    assert_eq!(
        success(evaluate(&request, &second).output().unwrap()),
        actual
    );
    let before = read_files(&first);
    assert!(
        !publish(&input, &first).status.success(),
        "publication remains no-clobber"
    );
    assert_eq!(read_files(&first), before);
}

#[test]
fn missing_stale_extra_and_rehashed_incoherent_artifacts_never_fall_back() {
    let f = fixture::fixture();
    let temp = tempfile::tempdir().unwrap();
    let request = save_request(temp.path(), &f);
    let base = valid_files(&f);
    for case in [
        "missing",
        "stale",
        "extra",
        "rehashed-binding",
        "partial-package",
    ] {
        let mut files = base.clone();
        match case {
            "missing" => {
                files.remove("support-inputs.json");
            }
            "stale" => {
                files.get_mut("metrics.json").unwrap().push(b' ');
            }
            "extra" => {
                files.insert("unexpected.json".into(), b"{}".to_vec());
            }
            "rehashed-binding" => {
                let mut output: Value =
                    serde_json::from_slice(&files["support-outputs.json"]).unwrap();
                output["rules"] = output["receiving"].clone();
                replace_hashed(
                    &mut files,
                    "support-outputs.json",
                    serde_json::to_vec(&output).unwrap(),
                );
            }
            "partial-package" => {
                files.remove("support-receiving.json");
                let mut receipt: Value = serde_json::from_slice(&files["release.json"]).unwrap();
                receipt["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|row| row["file"] != "support-receiving.json");
                files.insert("release.json".into(), serde_json::to_vec(&receipt).unwrap());
            }
            _ => unreachable!(),
        }
        let root = temp.path().join(case);
        write_files(&root, &files);
        let output_path = temp.path().join(format!("{case}-result.json"));
        let result = evaluate(&request, &root)
            .arg("--output")
            .arg(&output_path)
            .output()
            .unwrap();
        let error = rejected(&result);
        assert!(!output_path.exists(), "{case}: {error}");
        let rebuilt = temp.path().join(format!("{case}-rebuilt"));
        assert!(!publish(&root, &rebuilt).status.success(), "{case}");
        assert!(!rebuilt.exists(), "{case}");
    }
}

#[test]
fn partial_authored_evaluation_packages_are_rejected_before_publication() {
    let f = fixture::fixture();
    let temp = tempfile::tempdir().unwrap();
    for (i, missing) in ["metrics", "stages", "preparation", "inputs", "receiving"]
        .into_iter()
        .enumerate()
    {
        let mut input = serde_json::to_value(&f.input).unwrap();
        if missing == "metrics" {
            input["evaluation"].as_object_mut().unwrap().remove(missing);
        } else {
            input["evaluation"]["support"]
                .as_object_mut()
                .unwrap()
                .remove(missing);
        }
        let path = temp.path().join(format!("partial-{i}.json"));
        fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
        let destination = temp.path().join(format!("partial-{i}"));
        assert!(rejected(&publish(&path, &destination)).contains("missing field"));
        assert!(!destination.exists());
    }
}

#[test]
fn release_mode_cannot_mix_manual_artifacts_and_still_requires_an_explicit_request() {
    let f = fixture::fixture();
    let temp = tempfile::tempdir().unwrap();
    let request = save_request(temp.path(), &f);
    let root = temp.path().join("published");
    write_files(&root, &valid_files(&f));
    for flag in [
        "--schema",
        "--rules",
        "--routing",
        "--metrics",
        "--stages",
        "--support-preparation",
        "--support-inputs",
        "--support-receiving",
        "--support-outputs",
    ] {
        let output = evaluate(&request, &root)
            .arg(flag)
            .arg("absent.json")
            .output()
            .unwrap();
        assert!(rejected(&output).contains("cannot be used with"), "{flag}");
    }
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("evaluate-owned")
        .arg("--release")
        .arg(&root)
        .output()
        .unwrap();
    assert!(rejected(&output).contains("--input"));
}

#[test]
fn legacy_release_without_metrics_is_valid_but_not_an_evaluation_release() {
    let mut f = fixture::fixture();
    f.input.schema_version = OWNED_RELEASE_VERSION;
    f.input.evaluation = None;
    let temp = tempfile::tempdir().unwrap();
    let request = save_request(temp.path(), &f);
    let root = temp.path().join("legacy");
    write_files(&root, &valid_files(&f));
    let rebuilt = temp.path().join("legacy-rebuilt");
    success(publish(&root, &rebuilt));
    let error = rejected(&evaluate(&request, &root).output().unwrap());
    assert!(
        error.contains("does not contain evaluation artifacts"),
        "{error}"
    );
}

#[test]
fn legacy_authoring_cannot_silently_drop_validated_evaluation_artifacts() {
    let f = fixture::fixture();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("release");
    write_files(&root, &valid_files(&f));
    let destination = temp.path().join("authoring-output");
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .arg("publish-owned-normalization")
            .arg(&root)
            .arg("--normalization")
            .arg(root.join("normalization.json"))
            .arg("--output")
            .arg(&destination)
            .output()
            .unwrap()
    };
    let error = rejected(&run());
    assert!(
        error.contains("cannot preserve evaluation artifacts"),
        "{error}"
    );
    assert!(!destination.exists());
    fs::write(root.join("metrics.json"), b"{}").unwrap();
    let error = rejected(&run());
    assert!(
        error.contains("hash/size differs"),
        "must validate first: {error}"
    );
    assert!(!destination.exists());
}

fn evaluation_migration(
    prior: &poe_optimizer_import::owned_release::StagedOwnedRelease,
    endpoint: &fixture::Fixture,
) -> OwnedReleaseMigrationInput {
    OwnedReleaseMigrationInput {
        schema_version: 2,
        before: prior.receipt().input,
        release: endpoint.input.recipe.schema.release.clone(),
        reason: poe_optimizer_core::owned_definitions::OwnedDefinitionKey::new(
            "explicit-evaluation-migration",
        )
        .unwrap(),
        contract: OwnedReleaseContractMigration {
            schema_version: endpoint.input.recipe.schema.schema_version,
            schema_semantics_version: endpoint.input.recipe.schema.semantics_version.clone(),
            operations_version: endpoint.input.recipe.rules.operations_version.clone(),
            rule_semantics_version: endpoint.input.recipe.rules.semantics_version.clone(),
        },
        schema: vec![],
        tables: vec![],
        owners: vec![],
        receivers: vec![],
        query_targets: vec![],
        evaluation: endpoint.input.evaluation.clone(),
    }
}

#[test]
fn explicit_evaluation_migration_publishes_exact_endpoint_and_runs_the_same_native_request() {
    let endpoint = fixture::fixture_with_contract("migrated-evaluation", "migrated-rules");
    for legacy in [false, true] {
        let mut prior_input = fixture::fixture().input;
        if legacy {
            prior_input.schema_version = OWNED_RELEASE_VERSION;
            prior_input.evaluation = None;
        }
        let prior = assemble_owned_release(prior_input, Default::default()).unwrap();
        let policy = evaluation_migration(&prior, &endpoint);
        let expected =
            compile_owned_release_migration(&prior, policy.clone(), Default::default()).unwrap();
        let temp = tempfile::tempdir().unwrap();
        let request = save_request(temp.path(), &endpoint);
        let source = temp.path().join("prior");
        let original: Files = prior
            .artifacts()
            .map(|(name, bytes)| (name.into(), bytes.to_vec()))
            .collect();
        write_files(&source, &original);
        let migration_path = temp.path().join("migration.json");
        fs::write(&migration_path, serde_json::to_vec(&policy).unwrap()).unwrap();
        let output = temp.path().join("migrated");
        let actual = success(
            Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .arg("assemble-owned-release")
                .arg(&source)
                .arg("--migration")
                .arg(&migration_path)
                .arg("--output")
                .arg(&output)
                .output()
                .unwrap(),
        );
        assert_eq!(actual, serde_json::to_value(expected.receipt()).unwrap());
        assert_eq!(
            read_files(&output),
            expected
                .artifacts()
                .map(|(name, bytes)| (name.into(), bytes.to_vec()))
                .collect()
        );
        assert_eq!(read_files(&source), original);
        let evaluated = success(evaluate(&request, &output).output().unwrap());
        assert_eq!(
            evaluated["evaluation"],
            serde_json::to_value(&endpoint.expected.evaluation).unwrap()
        );
        assert_eq!(
            evaluated["support_preparation"],
            serde_json::to_value(&endpoint.expected.support).unwrap()
        );
        let rebuilt = temp.path().join("rebuilt");
        assert_eq!(success(publish(&output, &rebuilt)), actual);
        assert_eq!(read_files(&rebuilt), read_files(&output));
    }
}

#[test]
fn evaluation_migration_rejects_stale_or_missing_endpoint_packages_without_publication() {
    let prior = assemble_owned_release(fixture::fixture().input, Default::default()).unwrap();
    let endpoint = fixture::fixture_with_contract("migrated-evaluation", "migrated-rules");
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("prior");
    let original: Files = prior
        .artifacts()
        .map(|(name, bytes)| (name.into(), bytes.to_vec()))
        .collect();
    write_files(&source, &original);
    for case in [
        "missing",
        "prior-group",
        "outputs",
        "legacy-version",
        "stale-prior",
        "wrong-contract",
    ] {
        let mut policy = evaluation_migration(&prior, &endpoint);
        match case {
            "missing" => policy.evaluation = None,
            "prior-group" => policy.evaluation = prior.input().evaluation.clone(),
            "outputs" => {
                let support = policy
                    .evaluation
                    .as_mut()
                    .unwrap()
                    .support
                    .as_mut()
                    .unwrap();
                let outputs = support.outputs.as_mut().unwrap();
                outputs.receiving = outputs.stages;
            }
            "legacy-version" => policy.schema_version = 1,
            "stale-prior" => policy.before = prior.receipt().rules,
            "wrong-contract" => policy.contract.schema_version = 3,
            _ => unreachable!(),
        }
        let path = temp.path().join(format!("{case}.json"));
        fs::write(&path, serde_json::to_vec(&policy).unwrap()).unwrap();
        let destination = temp.path().join(case);
        rejected(
            &Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .arg("assemble-owned-release")
                .arg(&source)
                .arg("--migration")
                .arg(&path)
                .arg("--output")
                .arg(&destination)
                .output()
                .unwrap(),
        );
        assert!(!destination.exists(), "{case}");
        assert_eq!(read_files(&source), original);
    }
}
