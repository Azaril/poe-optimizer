//! Full-release host validation uses the public assembler and finite test data.
//! No source backend, hidden dependency repair, or successor compatibility claim.
#[path = "../crates/poe-optimizer-import/tests/support/owned_compact_fixture.rs"]
mod fixture;

use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SchemaState},
};
use poe_optimizer_import::{
    owned_release::{
        OWNED_RELEASE_VERSION, OwnedReleaseInput, OwnedReleaseReceipt, assemble_owned_release,
    },
    owned_release_revision::{OwnedReleaseRevisionInput, compile_owned_release_revision},
    owned_successor::{
        StagedSuccessorBundle, transition_owned_bundle, transition_owned_catalog_with_tree_compact,
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
    sync::OnceLock,
};

type Files = BTreeMap<String, Vec<u8>>;
struct Inputs {
    input: OwnedReleaseInput,
    release: Files,
    legacy: Files,
    compact: Files,
    legacy_release: Files,
}
fn owned(files: impl Iterator<Item = (impl AsRef<str>, impl AsRef<[u8]>)>) -> Files {
    files
        .map(|(name, bytes)| (name.as_ref().to_owned(), bytes.as_ref().to_vec()))
        .collect()
}
fn from_successor(prior: &StagedSuccessorBundle) -> OwnedReleaseInput {
    OwnedReleaseInput {
        schema_version: OWNED_RELEASE_VERSION,
        recipe: prior.recipe().clone(),
        mapping: prior.mapping().input().clone(),
        roles: prior.roles().input().clone(),
        normalization: prior.normalization().clone(),
        rewards: prior.rewards().input().clone(),
        items: prior.items().input().clone(),
        item_source: prior.item_source().input().clone(),
        tree: prior.tree().map(|tree| tree.input().clone()),
        query_sets: prior.query_sets().to_vec(),
        provenance: vec![],
    }
}
fn inputs() -> &'static Inputs {
    static INPUTS: OnceLock<Inputs> = OnceLock::new();
    INPUTS.get_or_init(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let legacy = transition_owned_bundle(fixture::input(&root), Default::default()).unwrap();
        let next = fixture::next(&legacy);
        let compact = transition_owned_catalog_with_tree_compact(
            next.clone(),
            fixture::append(&next),
            fixture::tree(&next),
            Default::default(),
        )
        .unwrap();
        let input = from_successor(&compact);
        let release = assemble_owned_release(input.clone(), Default::default()).unwrap();
        let legacy_release =
            assemble_owned_release(from_successor(&legacy), Default::default()).unwrap();
        Inputs {
            input,
            release: owned(release.artifacts()),
            legacy: owned(legacy.artifacts()),
            compact: owned(compact.artifacts()),
            legacy_release: owned(legacy_release.artifacts()),
        }
    })
}
fn write_files(root: &Path, files: &Files) {
    fs::create_dir(root).unwrap();
    for (name, bytes) in files {
        fs::write(root.join(name), bytes).unwrap();
    }
}
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
fn write_json(root: &Path, name: &str, value: &impl serde::Serialize) -> PathBuf {
    let path = root.join(name);
    fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    path
}
fn run(cwd: &Path, input: &Path, output: &Path, revision: Option<&Path>) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(cwd)
        .arg("assemble-owned-release")
        .arg(input)
        .arg("--output")
        .arg(output);
    if let Some(revision) = revision {
        command.arg("--revision").arg(revision);
    }
    command.output().unwrap()
}
fn success(output: &Output) -> OwnedReleaseReceipt {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn rejected(output: &Output, destination: &Path) {
    assert!(
        !output.status.success(),
        "unexpected receipt: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(
        !destination.exists(),
        "invalid input created an output directory"
    );
}
fn receipt(files: &Files) -> Value {
    serde_json::from_slice(&files["release.json"]).unwrap()
}
fn replace_receipt(files: &mut Files, receipt: Value) {
    files.insert("release.json".into(), serde_json::to_vec(&receipt).unwrap());
}
fn rewrite_hashed_file(files: &mut Files, name: &str, bytes: Vec<u8>) {
    let mut manifest = receipt(files);
    let entry = manifest["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|entry| entry["file"].as_str() == Some(name))
        .unwrap();
    entry["bytes"] = bytes.len().into();
    entry["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    files.insert(name.into(), bytes);
    replace_receipt(files, manifest);
}

#[test]
fn explicit_json_and_checked_release_reproduce_all_artifacts_and_ordered_queries() {
    let temp = tempfile::tempdir().unwrap();
    let mut input = inputs().input.clone();
    // Only constructor-owned ordering canonicalizes. Named query-set ordering
    // is authored, so directory reconstruction must use the receipt's order.
    input.recipe.registry.entries.reverse();
    input.recipe.schema.definitions.reverse();
    input.query_sets.reverse();
    let expected = assemble_owned_release(input.clone(), Default::default()).unwrap();
    let path = write_json(temp.path(), "input.json", &input);
    let first = temp.path().join("first");
    let first_receipt = success(&run(temp.path(), &path, &first, None));
    assert_eq!(&first_receipt, expected.receipt());
    assert_eq!(read_files(&first), owned(expected.artifacts()));
    assert_eq!(first_receipt.query_sets, 5);
    assert_eq!(first_receipt.query_rows, 110);
    assert_eq!(first_receipt.provenance.len(), 0);
    assert!(!first.join("recipe.json").exists());
    assert!(!first.join("transition.json").exists());
    let second = temp.path().join("second");
    assert_eq!(
        success(&run(temp.path(), &first, &second, None)),
        first_receipt
    );
    assert_eq!(read_files(&first), read_files(&second));
    assert_eq!(
        fs::read(&path).unwrap(),
        serde_json::to_vec(&input).unwrap()
    );
}

#[test]
fn legacy_and_compact_successors_are_revalidated_as_independent_releases() {
    let temp = tempfile::tempdir().unwrap();
    for (label, source, expected) in [
        ("legacy", &inputs().legacy, &inputs().legacy_release),
        ("compact", &inputs().compact, &inputs().release),
    ] {
        let prior = temp.path().join(label);
        write_files(&prior, source);
        let output = temp.path().join(format!("{label}-release"));
        success(&run(temp.path(), &prior, &output, None));
        assert_eq!(read_files(&output), *expected);
        assert_eq!(read_files(&prior), *source);
    }
    let mut stale = inputs().compact.clone();
    let mut transition: Value = serde_json::from_slice(&stale["transition.json"]).unwrap();
    transition["after"]["roles"] =
        serde_json::to_value(digest_owned("foreign-role", &1, 128).unwrap()).unwrap();
    stale.insert(
        "transition.json".into(),
        serde_json::to_vec(&transition).unwrap(),
    );
    let prior = temp.path().join("stale-endpoint");
    write_files(&prior, &stale);
    let output = temp.path().join("stale-endpoint-output");
    let result = run(temp.path(), &prior, &output, None);
    rejected(&result, &output);
    assert!(String::from_utf8_lossy(&result.stderr).contains("endpoint identities differ"));
}

#[test]
fn reversed_named_query_sets_survive_legacy_and_compact_conversion_with_row_order_intact() {
    let temp = tempfile::tempdir().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut authored = fixture::input(&root);
    authored.query_sets.reverse();
    let expected_queries = authored.query_sets.clone();
    assert_eq!(expected_queries[0].name.as_str(), "original-05");
    let legacy = transition_owned_bundle(authored, Default::default()).unwrap();
    let next = fixture::next(&legacy);
    let compact = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    for (label, successor) in [("legacy", &legacy), ("compact", &compact)] {
        let source_files = owned(successor.artifacts());
        let source = temp.path().join(label);
        write_files(&source, &source_files);
        let expected =
            assemble_owned_release(from_successor(successor), Default::default()).unwrap();
        assert_eq!(expected.query_sets(), expected_queries);
        let destination = temp.path().join(format!("{label}-release"));
        let published = success(&run(temp.path(), &source, &destination, None));
        assert_eq!(&published, expected.receipt());
        let files = read_files(&destination);
        assert_eq!(files, owned(expected.artifacts()));
        let ordered_query_files: Vec<_> = published
            .artifacts
            .iter()
            .filter(|artifact| artifact.file.starts_with("queries-"))
            .map(|artifact| artifact.file.clone())
            .collect();
        assert_eq!(
            ordered_query_files,
            expected_queries
                .iter()
                .map(|set| format!("queries-{}.json", set.name.as_str()))
                .collect::<Vec<_>>()
        );
        for set in &expected_queries {
            let name = format!("queries-{}.json", set.name.as_str());
            assert_eq!(
                files[&name], source_files[&name],
                "{label}: query rows changed"
            );
        }
        assert_eq!(read_files(&source), source_files);
    }
}
#[test]
fn stale_constituent_bindings_and_unknown_json_are_not_silently_repaired() {
    let temp = tempfile::tempdir().unwrap();
    let wrong = digest_owned("foreign-release", &1, 128).unwrap();
    for case in 0..5 {
        let mut input = inputs().input.clone();
        match case {
            0 => input.roles.mapping = wrong,
            1 => input.rewards.mapping = wrong,
            2 => input.item_source.item_lines = wrong,
            3 => input.mapping.registry = wrong,
            4 => input.recipe.rules.definitions.release = "wrong-release".into(),
            _ => unreachable!(),
        }
        let path = write_json(temp.path(), &format!("stale-{case}.json"), &input);
        let output = temp.path().join(format!("stale-{case}-output"));
        rejected(&run(temp.path(), &path, &output, None), &output);
    }
    let mut unknown = serde_json::to_value(&inputs().input).unwrap();
    unknown["repair_bindings"] = true.into();
    let path = write_json(temp.path(), "unknown.json", &unknown);
    let output = temp.path().join("unknown-output");
    rejected(&run(temp.path(), &path, &output, None), &output);
    let path = temp.path().join("malformed.json");
    fs::write(&path, b"{\"schema_version\":").unwrap();
    let output = temp.path().join("malformed-output");
    rejected(&run(temp.path(), &path, &output, None), &output);
}

#[test]
fn release_directory_inventory_hashes_bindings_and_receipt_are_all_checked() {
    let temp = tempfile::tempdir().unwrap();
    for case in 0..10 {
        let mut files = inputs().release.clone();
        match case {
            0 => {
                files.insert("unexpected.json".into(), b"{}".to_vec());
            }
            1 => {
                files.remove("rules.json");
            }
            2 => {
                files.get_mut("mapping.json").unwrap().push(b' ');
            }
            3 => {
                let mut value = receipt(&files);
                value["artifacts"][0]["file"] = "../registry.json".into();
                replace_receipt(&mut files, value);
            }
            4 => {
                let mut value = receipt(&files);
                let duplicate = value["artifacts"][0].clone();
                value["artifacts"].as_array_mut().unwrap().push(duplicate);
                replace_receipt(&mut files, value);
            }
            5 => {
                let mut value = receipt(&files);
                value["input"] =
                    serde_json::to_value(digest_owned("unrelated-input", &7, 128).unwrap())
                        .unwrap();
                replace_receipt(&mut files, value);
            }
            6 => {
                // Correct file hash does not validate a cross-release dependency.
                let mut value: Value = serde_json::from_slice(&files["roles.json"]).unwrap();
                value["mapping"] =
                    serde_json::to_value(digest_owned("other-mapping", &7, 128).unwrap()).unwrap();
                rewrite_hashed_file(
                    &mut files,
                    "roles.json",
                    serde_json::to_vec(&value).unwrap(),
                );
            }
            7 => {
                // Even semantically equivalent bytes are not a canonical release.
                let value: Value = serde_json::from_slice(&files["normalization.json"]).unwrap();
                rewrite_hashed_file(
                    &mut files,
                    "normalization.json",
                    serde_json::to_vec_pretty(&value).unwrap(),
                );
            }
            8 => {
                let mut value = receipt(&files);
                value["artifacts"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|row| row["file"] != "queries-original-01.json");
                value["query_sets"] = 4.into();
                replace_receipt(&mut files, value);
            }
            9 => {
                let mut value = receipt(&files);
                value["artifacts"][0]["bytes"] = usize::MAX.into();
                replace_receipt(&mut files, value);
            }
            _ => unreachable!(),
        }
        let source = temp.path().join(format!("invalid-{case}"));
        write_files(&source, &files);
        let output = temp.path().join(format!("invalid-{case}-output"));
        rejected(&run(temp.path(), &source, &output, None), &output);
        assert_eq!(read_files(&source), files);
    }
    let source = temp.path().join("directory-entry");
    write_files(&source, &inputs().release);
    fs::create_dir(source.join("extra-directory")).unwrap();
    let output = temp.path().join("directory-entry-output");
    rejected(&run(temp.path(), &source, &output, None), &output);
}

#[test]
fn publication_never_clobbers_existing_files_or_directories() {
    let temp = tempfile::tempdir().unwrap();
    let path = write_json(temp.path(), "input.json", &inputs().input);
    for case in ["empty", "occupied", "file"] {
        let output = temp.path().join(case);
        match case {
            "file" => fs::write(&output, b"original file").unwrap(),
            "empty" => fs::create_dir(&output).unwrap(),
            "occupied" => write_files(&output, &inputs().release),
            _ => unreachable!(),
        }
        let result = run(temp.path(), &path, &output, None);
        assert!(!result.status.success());
        match case {
            "file" => assert_eq!(fs::read(output).unwrap(), b"original file"),
            "empty" => assert_eq!(fs::read_dir(output).unwrap().count(), 0),
            "occupied" => assert_eq!(read_files(&output), inputs().release),
            _ => unreachable!(),
        }
    }
    assert!(!fs::read_dir(temp.path()).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with(".owned-recipe-")
    }));
}

#[test]
fn explicit_revision_binds_prior_release_and_persists_provenance_without_transition() {
    let temp = tempfile::tempdir().unwrap();
    let prior = assemble_owned_release(inputs().input.clone(), Default::default()).unwrap();
    let mut replacement = prior.input().recipe.schema.definitions.iter().find(|row| {
        matches!(row, DefinitionDescriptor::Gem(entry) if matches!(&entry.schema, SchemaState::Unmapped { gaps } if !gaps.is_empty()))
    }).expect("finite fixture has unresolved physical Gems").clone();
    if let DefinitionDescriptor::Gem(entry) = &mut replacement
        && let SchemaState::Unmapped { gaps } = &mut entry.schema
    {
        gaps[0].code = OwnedDefinitionKey::new("reviewed-still-unresolved").unwrap();
    } else {
        unreachable!();
    }
    let policy = OwnedReleaseRevisionInput {
        schema_version: 1,
        before: prior.receipt().input,
        release: OwnedDefinitionKey::new("cli-explicit-revision").unwrap(),
        reason: OwnedDefinitionKey::new("reviewed-gap-classification").unwrap(),
        definitions: vec![replacement],
        slots: vec![],
    };
    let expected =
        compile_owned_release_revision(&prior, policy.clone(), Default::default()).unwrap();
    let source = temp.path().join("prior");
    write_files(&source, &inputs().release);
    let policy_path = write_json(temp.path(), "revision.json", &policy);
    let output = temp.path().join("revised");
    assert_eq!(
        &success(&run(temp.path(), &source, &output, Some(&policy_path))),
        expected.receipt()
    );
    assert_eq!(read_files(&output), owned(expected.artifacts()));
    assert_eq!(expected.receipt().provenance.len(), 1);
    assert_eq!(
        expected.receipt().provenance[0].prior_input,
        prior.receipt().input
    );
    assert_eq!(expected.receipt().registry, prior.receipt().registry);
    assert_ne!(expected.receipt().definitions, prior.receipt().definitions);
    assert_eq!(expected.input().query_sets, prior.input().query_sets);
    assert!(!output.join("transition.json").exists());
    let roundtrip = temp.path().join("revised-roundtrip");
    success(&run(temp.path(), &output, &roundtrip, None));
    assert_eq!(read_files(&roundtrip), read_files(&output));
    let stale = temp.path().join("stale-revision-output");
    rejected(
        &run(temp.path(), &output, &stale, Some(&policy_path)),
        &stale,
    );
    assert_eq!(read_files(&source), inputs().release);
}

#[test]
fn release_feeds_existing_normalization_publisher_and_checked_successor_loader() {
    let temp = tempfile::tempdir().unwrap();
    let release = temp.path().join("release");
    write_files(&release, &inputs().release);
    let successor = temp.path().join("normalization-successor");
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(temp.path())
        .arg("publish-owned-normalization")
        .arg(&release)
        .arg("--normalization")
        .arg(release.join("normalization.json"))
        .arg("--output")
        .arg(&successor)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    let transition = &response["publication"];
    assert_eq!(transition["schema_version"], 2);
    assert_eq!(transition["before"], transition["after"]);
    assert_eq!(transition["query_sets"], 5);
    assert_eq!(transition["query_rows"], 110);
    assert!(successor.join("transition.json").exists());
    assert!(!successor.join("recipe.json").exists());
    assert!(!successor.join("release.json").exists());
    let restored = temp.path().join("restored-release");
    success(&run(temp.path(), &successor, &restored, None));
    assert_eq!(read_files(&restored), inputs().release);
    assert_eq!(read_files(&release), inputs().release);
}
