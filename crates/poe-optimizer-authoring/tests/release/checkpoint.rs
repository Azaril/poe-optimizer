//! The persisted authoring checkpoint is the only CLI allocation predecessor.
use super::*;
use poe_optimizer_authoring::{
    AuthoringCheckpointReceipt, CheckedAuthoringCheckpoint, CheckpointBase, CheckpointBlob,
    CheckpointError, CheckpointLimits, StagedAuthoringCheckpoint, build_authoring_checkpoint,
    load_authoring_checkpoint_artifacts,
};
use poe_optimizer_core::owned_content::OwnedContentDigest;

fn build(source: &Sources, base: CheckpointBase<'_>) -> StagedAuthoringCheckpoint {
    build_authoring_checkpoint(
        source.manifest.as_bytes(),
        &source.sources(),
        base,
        Default::default(),
    )
    .unwrap()
}

fn roots(staged: &StagedAuthoringCheckpoint) -> BTreeMap<String, Vec<u8>> {
    staged
        .root_artifacts()
        .map(|(name, bytes)| (name.into(), bytes.to_vec()))
        .collect()
}

fn load(staged: &StagedAuthoringCheckpoint) -> CheckedAuthoringCheckpoint {
    let root = roots(staged);
    load_authoring_checkpoint_artifacts(
        &root["checkpoint.json"],
        &root["names.json"],
        &files(staged.built().release()),
        staged.identity(),
        Default::default(),
    )
    .unwrap()
}

fn blob(bytes: &[u8]) -> CheckpointBlob {
    CheckpointBlob {
        bytes: bytes.len(),
        sha256: format!("{:x}", Sha256::digest(bytes)),
    }
}

fn write_source(path: &Path, source: &Sources) -> std::path::PathBuf {
    fs::create_dir(path).unwrap();
    for (name, bytes) in &source.files {
        let target = path.join(name);
        fs::create_dir_all(target.parent().unwrap()).unwrap();
        fs::write(target, bytes).unwrap();
    }
    let manifest = path.join("release.kdl");
    fs::write(&manifest, &source.manifest).unwrap();
    manifest
}

fn predecessor_cli(
    manifest: &Path,
    output: &Path,
    prior: &Path,
    identity: OwnedContentDigest,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .args(["release", "build", "--predecessor"])
        .arg(prior)
        .arg("--expected-predecessor")
        .arg(identity.to_string())
        .arg("--output")
        .arg(output)
        .arg(manifest)
        .output()
        .unwrap()
}

fn verify_cli(path: &Path, identity: OwnedContentDigest) -> Output {
    Command::new(env!("CARGO_BIN_EXE_poe-author"))
        .current_dir(path.parent().unwrap())
        .args(["release", "verify", "--expected"])
        .arg(identity.to_string())
        .arg(path)
        .output()
        .unwrap()
}

fn write_checkpoint(path: &Path, checkpoint: &StagedAuthoringCheckpoint) {
    fs::create_dir(path).unwrap();
    fs::create_dir(path.join("runtime")).unwrap();
    for (name, bytes) in checkpoint.root_artifacts() {
        fs::write(path.join(name), bytes).unwrap();
    }
    for (name, bytes) in checkpoint.built().release().artifacts() {
        fs::write(path.join("runtime").join(name), bytes).unwrap();
    }
}

#[test]
fn transported_checkpoint_verifies_and_continues_without_its_sources_or_ancestors() {
    let (dsl, _) = declarations::baseline(false);
    let first = build(&dsl.source, CheckpointBase::Bootstrap);
    let first_checked = load(&first);
    let mut source = dsl.source.clone();
    source.replace_plain(declarations::DECLARATIONS, source.text(declarations::DECLARATIONS)
        .replace(" stat \"z_declaration_flag\"", " stat \"a_earlier_flag\" type=\"boolean\" { target \"action\"; }\n stat \"z_declaration_flag\""));
    let second = build(&source, CheckpointBase::Predecessor(&first_checked));
    let second_checked = load(&second);
    for binding in &first_checked.names().input().bindings {
        assert!(second_checked.names().input().bindings.contains(binding));
    }
    assert_eq!(
        second_checked.names().input().bindings.len(),
        first_checked.names().input().bindings.len() + 1
    );

    // Only the current complete artifact directory is transported. Neither
    // generation's source nor the ancestor checkpoint is on this filesystem.
    let temp = tempfile::tempdir().unwrap();
    let transported = temp.path().join("-checkpoint λ");
    write_checkpoint(&transported, &second);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    let before = checkpoint_files(&transported);
    let verified = verify_cli(&transported, second.identity());
    assert_eq!(
        verified.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&verified.stderr)
    );
    assert!(verified.stderr.is_empty());
    let report: serde_json::Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(
        report,
        serde_json::json!({
            "checkpoint": second.identity(),
            "names": second.receipt().names.identity,
            "runtime_input": second.receipt().runtime.input,
            "runtime_registry": second.receipt().runtime.registry,
        })
    );
    assert_eq!(checkpoint_files(&transported), before);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);

    // A new generation needs only its own sources and the transported snapshot.
    let manifest = write_source(&temp.path().join("current sources"), &source);
    let next = temp.path().join("next");
    let result = predecessor_cli(&manifest, &next, &transported, second.identity());
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(next.join("names.json")).unwrap(),
        before["names.json"]
    );
    assert_eq!(
        directory_files(&next.join("runtime")),
        directory_files(&transported.join("runtime"))
    );
    assert_eq!(checkpoint_files(&transported), before);
}

#[test]
fn checkpoint_verification_rejects_wrong_identity_corruption_and_extra_files_without_writing() {
    let (dsl, _) = declarations::baseline(false);
    let staged = build(&dsl.source, CheckpointBase::Bootstrap);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("checkpoint");
    write_checkpoint(&path, &staged);
    let before = checkpoint_files(&path);
    let assert_rejected = |identity| {
        let snapshot = || {
            let mut contents = directory_files(&path.join("runtime"))
                .into_iter()
                .map(|(name, bytes)| (format!("runtime/{name}"), bytes))
                .collect::<BTreeMap<_, _>>();
            for entry in fs::read_dir(&path).unwrap() {
                let entry = entry.unwrap();
                if entry.file_type().unwrap().is_file() {
                    contents.insert(
                        entry.file_name().into_string().unwrap(),
                        fs::read(entry.path()).unwrap(),
                    );
                }
            }
            contents
        };
        let contents = snapshot();
        let entries = directory_entries(&path);
        let result = verify_cli(&path, identity);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!result.stderr.is_empty());
        assert_eq!(snapshot(), contents);
        assert_eq!(directory_entries(&path), entries);
        assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 1);
    };
    assert_rejected("0".repeat(64).parse().unwrap());
    for name in ["checkpoint.json", "names.json", "runtime/schema.json"] {
        assert!(
            before.contains_key(name),
            "test must corrupt an existing artifact: {name}"
        );
        fs::write(path.join(name), b"{}").unwrap();
        assert_rejected(staged.identity());
        fs::write(path.join(name), &before[name]).unwrap();
    }
    for name in ["unexpected.txt", "runtime/unexpected.txt"] {
        fs::write(path.join(name), b"unexpected").unwrap();
        assert_rejected(staged.identity());
        fs::remove_file(path.join(name)).unwrap();
    }
    assert_eq!(checkpoint_files(&path), before);
}

#[test]
fn checkpoint_commits_exact_sources_and_replays_persisted_allocation_history() {
    let (dsl, _) = declarations::baseline(false);
    let first = build(&dsl.source, CheckpointBase::Bootstrap);
    let before = roots(&first);
    assert_eq!(
        before.keys().map(String::as_str).collect::<Vec<_>>(),
        ["checkpoint.json", "names.json"]
    );
    let receipt = first.receipt();
    assert!(receipt.predecessor.is_none());
    assert_eq!(
        receipt.source.manifest,
        blob(dsl.source.manifest.as_bytes())
    );
    let expected_sources: BTreeMap<_, _> = dsl
        .source
        .files
        .iter()
        .map(|(path, bytes)| (path.clone(), blob(bytes)))
        .collect();
    assert_eq!(
        receipt
            .source
            .files
            .iter()
            .map(|source| (source.path.clone(), source.blob.clone()))
            .collect::<BTreeMap<_, _>>(),
        expected_sources
    );
    assert!(
        receipt
            .source
            .files
            .windows(2)
            .all(|pair| pair[0].path < pair[1].path)
    );
    assert_eq!(receipt.names.blob, blob(&before["names.json"]));
    assert_eq!(
        receipt.runtime.receipt,
        blob(&files(first.built().release())["release.json"])
    );

    let checked = load(&first);
    assert_eq!(checked.receipt(), first.receipt());
    assert_eq!(checked.identity(), first.identity());
    let saved = checked.names().input().clone();
    let prior = saved
        .bindings
        .iter()
        .find(|binding| binding.name == "z_declaration_flag")
        .unwrap()
        .clone();
    let replay = build(&dsl.source, CheckpointBase::Predecessor(&checked));
    assert_eq!(
        replay.built().release().receipt(),
        first.built().release().receipt()
    );
    assert_eq!(replay.built().name_allocation().names().input(), &saved);
    assert_eq!(
        replay.receipt().predecessor.as_ref().unwrap().checkpoint,
        first.identity()
    );
    assert_eq!(
        replay.receipt().predecessor.as_ref().unwrap().names,
        saved_identity(&checked)
    );
    assert_ne!(replay.identity(), first.identity());
    load(&replay);

    let mut earlier = dsl.source.clone();
    earlier.replace_plain(declarations::DECLARATIONS, earlier.text(declarations::DECLARATIONS)
        .replace(" stat \"z_declaration_flag\"", " stat \"a_earlier_flag\" type=\"boolean\" { target \"action\"; }\n stat \"z_declaration_flag\""));
    let expanded = build(&earlier, CheckpointBase::Predecessor(&checked));
    let expanded_checked = load(&expanded);
    assert_eq!(
        expanded_checked
            .names()
            .input()
            .bindings
            .iter()
            .find(|binding| binding.name == prior.name),
        Some(&prior)
    );
    assert_ne!(
        expanded_checked
            .names()
            .input()
            .bindings
            .iter()
            .find(|binding| binding.name == "a_earlier_flag")
            .unwrap()
            .target,
        prior.target
    );
    assert_eq!(checked.names().input(), &saved);
    assert_eq!(roots(&first), before);
}

fn saved_identity(checked: &CheckedAuthoringCheckpoint) -> OwnedContentDigest {
    checked.names().identity().unwrap()
}

#[test]
fn checkpoint_source_transport_identity_is_distinct_from_runtime_semantics() {
    let source = DslRelease::new(&fixture().input, false).source;
    let first = build(&source, CheckpointBase::Bootstrap);
    let mut commented = source.clone();
    commented.manifest.push_str("// retained source comment\n");
    let second = build(&commented, CheckpointBase::Bootstrap);
    assert_eq!(
        first.built().release().receipt(),
        second.built().release().receipt()
    );
    assert_eq!(first.receipt().names, second.receipt().names);
    assert_ne!(
        first.receipt().source.manifest,
        second.receipt().source.manifest
    );
    assert_ne!(first.identity(), second.identity());
}

#[test]
fn checkpoint_loader_checks_exact_budgets_canonical_bytes_and_runtime_authority() {
    let source = DslRelease::new(&fixture().input, false).source;
    let staged = build(&source, CheckpointBase::Bootstrap);
    let root = roots(&staged);
    let runtime = files(staged.built().release());
    let exact = CheckpointLimits {
        max_receipt_bytes: root["checkpoint.json"].len(),
        max_names_bytes: root["names.json"].len(),
        max_checkpoint_bytes: root.values().map(Vec::len).sum::<usize>()
            + runtime.values().map(Vec::len).sum::<usize>(),
        max_entries: 3 + runtime.len(),
        ..Default::default()
    };
    for limits in [CheckpointLimits::default(), exact] {
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &runtime,
            staged.identity(),
            limits,
        )
        .unwrap();
        build_authoring_checkpoint(
            source.manifest.as_bytes(),
            &source.sources(),
            CheckpointBase::Bootstrap,
            limits,
        )
        .unwrap();
    }
    for field in 0..4 {
        let mut short = exact;
        match field {
            0 => short.max_receipt_bytes -= 1,
            1 => short.max_names_bytes -= 1,
            2 => short.max_checkpoint_bytes -= 1,
            3 => short.max_entries -= 1,
            _ => unreachable!(),
        }
        assert!(matches!(
            load_authoring_checkpoint_artifacts(
                &root["checkpoint.json"],
                &root["names.json"],
                &runtime,
                staged.identity(),
                short
            ),
            Err(CheckpointError::Limit(_))
        ));
        assert!(
            build_authoring_checkpoint(
                source.manifest.as_bytes(),
                &source.sources(),
                CheckpointBase::Bootstrap,
                short
            )
            .is_err()
        );
    }
    let mut runtime_exact = CheckpointLimits::default();
    runtime_exact.build.release.max_input_bytes = runtime.values().map(Vec::len).sum();
    load_authoring_checkpoint_artifacts(
        &root["checkpoint.json"],
        &root["names.json"],
        &runtime,
        staged.identity(),
        runtime_exact,
    )
    .unwrap();
    build_authoring_checkpoint(
        source.manifest.as_bytes(),
        &source.sources(),
        CheckpointBase::Bootstrap,
        runtime_exact,
    )
    .unwrap();
    let mut runtime_short = runtime_exact;
    runtime_short.build.release.max_input_bytes -= 1;
    assert!(matches!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &runtime,
            staged.identity(),
            runtime_short
        ),
        Err(CheckpointError::Limit("runtime bytes"))
    ));
    assert!(matches!(
        build_authoring_checkpoint(
            source.manifest.as_bytes(),
            &source.sources(),
            CheckpointBase::Bootstrap,
            runtime_short
        ),
        Err(CheckpointError::Limit("runtime bytes"))
    ));
    let mut registry_short = CheckpointLimits::default();
    registry_short.build.declarations.registry.max_entries = 1;
    assert!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &runtime,
            staged.identity(),
            registry_short
        )
        .is_err()
    );
    assert!(
        build_authoring_checkpoint(
            source.manifest.as_bytes(),
            &source.sources(),
            CheckpointBase::Bootstrap,
            registry_short
        )
        .is_err()
    );
    let zero = "0".repeat(64).parse().unwrap();
    assert!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &runtime,
            zero,
            exact
        )
        .is_err()
    );
    let mut changed = root["checkpoint.json"].clone();
    changed.push(b'\n');
    assert!(
        load_authoring_checkpoint_artifacts(
            &changed,
            &root["names.json"],
            &runtime,
            staged.identity(),
            Default::default()
        )
        .is_err()
    );
    let mut changed_names = root["names.json"].clone();
    changed_names.push(b' ');
    assert!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &changed_names,
            &runtime,
            staged.identity(),
            Default::default()
        )
        .is_err()
    );
    let mut extra = runtime.clone();
    extra.insert("unlisted.json".into(), b"{}".to_vec());
    assert!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &extra,
            staged.identity(),
            Default::default()
        )
        .is_err()
    );
    let mut broken = runtime.clone();
    let artifact = broken
        .iter_mut()
        .find(|(name, _)| name.as_str() != "release.json")
        .unwrap()
        .1;
    artifact[0] ^= 1;
    assert!(matches!(
        load_authoring_checkpoint_artifacts(
            &root["checkpoint.json"],
            &root["names.json"],
            &broken,
            staged.identity(),
            Default::default()
        ),
        Err(CheckpointError::Runtime(_))
    ));

    // Re-signing a claim does not supply registry authority to a forged ledger.
    let mut names: poe_optimizer_authoring::NamesInput =
        serde_json::from_slice(&root["names.json"]).unwrap();
    names.registry = zero;
    let forged_names = serde_json::to_vec(&names).unwrap();
    let mut claim = staged.receipt().clone();
    claim.names.blob = blob(&forged_names);
    let claim_identity = claim.identity(Default::default()).unwrap();
    assert!(matches!(
        load_authoring_checkpoint_artifacts(
            &serde_json::to_vec(&claim).unwrap(),
            &forged_names,
            &runtime,
            claim_identity,
            Default::default()
        ),
        Err(CheckpointError::Allocation(_))
    ));
}

#[test]
fn cli_predecessor_replay_and_zero_additions_preserve_saved_names() {
    let temp = tempfile::tempdir().unwrap();
    let (dsl, _) = declarations::baseline(false);
    let manifest = write_source(&temp.path().join("sources"), &dsl.source);
    let first = temp.path().join("first");
    let result = cli(&manifest, &first);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = checkpoint_files(&first);
    assert_checkpoint_success(&result, &before["checkpoint.json"]);
    let receipt: AuthoringCheckpointReceipt =
        serde_json::from_slice(&before["checkpoint.json"]).unwrap();
    let identity = receipt.identity(Default::default()).unwrap();
    let replay = temp.path().join("replay");
    let result = predecessor_cli(&manifest, &replay, &first, identity);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let repeated = checkpoint_files(&replay);
    assert_checkpoint_success(&result, &repeated["checkpoint.json"]);
    assert_eq!(before["names.json"], repeated["names.json"]);
    assert_eq!(
        directory_files(&first.join("runtime")),
        directory_files(&replay.join("runtime"))
    );

    let mut input = fixture().input.clone();
    input.evaluation = None;
    input.schema_version = OWNED_RELEASE_VERSION;
    // Continue with compressed constituents too: gzip decoding must fit the
    // ordinary CLI main-thread stack, including an unoptimized Windows build.
    let source = DslRelease::new(&input, true).source;
    let without = write_source(&temp.path().join("without declarations"), &source);
    let zero = temp.path().join("zero additions");
    let result = predecessor_cli(&without, &zero, &first, identity);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert_eq!(
        fs::read(zero.join("names.json")).unwrap(),
        before["names.json"]
    );
    assert_checkpoint_success(&result, &fs::read(zero.join("checkpoint.json")).unwrap());
    assert_eq!(checkpoint_files(&first), before);
}

#[test]
fn cli_corrupt_or_wrong_predecessor_and_late_source_failure_never_publish() {
    let temp = tempfile::tempdir().unwrap();
    let (dsl, _) = declarations::baseline(false);
    let manifest = write_source(&temp.path().join("sources"), &dsl.source);
    let prior = temp.path().join("prior");
    let result = cli(&manifest, &prior);
    assert_eq!(
        result.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let before = checkpoint_files(&prior);
    let receipt: AuthoringCheckpointReceipt =
        serde_json::from_slice(&before["checkpoint.json"]).unwrap();
    let identity = receipt.identity(Default::default()).unwrap();
    let output = temp.path().join("must not publish");
    let prior_runtime = prior.join("runtime");
    for wrong in [true, false] {
        let entries = directory_entries(temp.path());
        let result = predecessor_cli(
            &manifest,
            &output,
            if wrong { &prior } else { &prior_runtime },
            if wrong {
                "0".repeat(64).parse().unwrap()
            } else {
                identity
            },
        );
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!output.exists());
        assert_eq!(directory_entries(temp.path()), entries);
        assert_eq!(checkpoint_files(&prior), before);
    }
    for name in ["names.json", "checkpoint.json"] {
        fs::write(prior.join(name), b"{}").unwrap();
        let entries = directory_entries(temp.path());
        let result = predecessor_cli(&manifest, &output, &prior, identity);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(!output.exists());
        assert_eq!(directory_entries(temp.path()), entries);
        fs::write(prior.join(name), &before[name]).unwrap();
    }
    for (name, contents, diagnostic) in [
        (
            declarations::DECLARATIONS,
            "not definitions",
            declarations::DECLARATIONS,
        ),
        (
            "definitions/names.kdl",
            "not names",
            "definitions/names.kdl",
        ),
        (
            "rules/declaration-probe.rule",
            "program \"bad\" context action { let x = ; }",
            "rules/declaration-probe.kdl",
        ),
    ] {
        let path = manifest.parent().unwrap().join(name);
        let original = fs::read(&path).unwrap();
        fs::write(&path, contents).unwrap();
        let entries = directory_entries(temp.path());
        let result = predecessor_cli(&manifest, &output, &prior, identity);
        assert_eq!(result.status.code(), Some(2));
        assert!(result.stdout.is_empty());
        assert!(
            String::from_utf8_lossy(&result.stderr).contains(diagnostic),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
        assert!(!output.exists());
        assert_eq!(directory_entries(temp.path()), entries);
        assert_eq!(checkpoint_files(&prior), before);
        fs::write(path, original).unwrap();
    }
}
