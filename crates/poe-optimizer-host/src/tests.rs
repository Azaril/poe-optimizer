use super::*;

#[test]
fn destination_requires_a_name_and_existing_parent_without_creating_paths() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("result.json");
    assert_eq!(
        destination(&input).unwrap(),
        root.path().canonicalize().unwrap().join("result.json")
    );
    for path in [
        Path::new(""),
        Path::new("."),
        Path::new(".."),
        Path::new("/"),
    ] {
        assert!(destination(path).is_err(), "accepted {path:?}");
    }
    assert!(destination(&root.path().join("missing").join("result.json")).is_err());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[test]
fn single_file_publication_preserves_existing_file_and_cleans_failed_staging() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("result.json");
    write_new(&path, b"first").unwrap();
    assert!(write_new(&path, b"second").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"first");
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    let directory = root.path().join("directory");
    fs::create_dir(&directory).unwrap();
    assert!(write_new(&directory, b"replacement").is_err());
    assert!(directory.is_dir());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 2);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn directory_publication_exposes_exact_artifacts_and_leaves_no_staging_directory() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("published");
    publish_artifacts(
        &output,
        [
            ("manifest.json", b"complete".as_slice()),
            ("payload.bin", &[0, 255]),
        ],
    )
    .unwrap();
    assert_eq!(fs::read(output.join("manifest.json")).unwrap(), b"complete");
    assert_eq!(fs::read(output.join("payload.bin")).unwrap(), [0, 255]);
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
    assert!(publish_artifacts(&output, [("other", b"replacement".as_slice())]).is_err());
    assert_eq!(fs::read(output.join("manifest.json")).unwrap(), b"complete");
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 1);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn artifact_names_are_basenames_on_every_host_and_failures_remove_partial_staging() {
    let root = tempfile::tempdir().unwrap();
    let output = root.path().join("published");
    for name in [
        "",
        ".",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        "C:stream",
        "a\0b",
    ] {
        let error = publish_artifacts(
            &output,
            [
                ("first.json", b"already staged".as_slice()),
                (name, b"invalid"),
            ],
        )
        .unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{name:?}");
        assert!(!output.exists());
        assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
    }
    // Duplicate basenames fail only after the first artifact has been written.
    assert!(
        publish_artifacts(
            &output,
            [("same", b"first".as_slice()), ("same", b"second")],
        )
        .is_err()
    );
    assert!(!output.exists());
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn publication_race_never_replaces_an_existing_empty_directory() {
    let parent = tempfile::tempdir().unwrap();
    let from = parent.path().join("staging");
    let to = parent.path().join("published");
    fs::create_dir(&from).unwrap();
    fs::write(from.join("manifest.json"), b"complete").unwrap();
    assert!(!to.exists());
    // Simulate a concurrent publisher after the caller's absence check.
    fs::create_dir(&to).unwrap();
    assert!(publish_noclobber(&from, &to).is_err());
    assert_eq!(fs::read_dir(&to).unwrap().count(), 0);
    assert_eq!(fs::read(from.join("manifest.json")).unwrap(), b"complete");
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn publication_has_one_winner_and_keeps_the_losing_staged_bundle() {
    let parent = tempfile::tempdir().unwrap();
    let a = parent.path().join("a");
    let b = parent.path().join("b");
    let output = parent.path().join("output");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    fs::write(a.join("manifest.json"), b"first").unwrap();
    fs::write(b.join("manifest.json"), b"second").unwrap();
    publish_noclobber(&a, &output).unwrap();
    assert!(publish_noclobber(&b, &output).is_err());
    assert_eq!(fs::read(output.join("manifest.json")).unwrap(), b"first");
    assert_eq!(fs::read(b.join("manifest.json")).unwrap(), b"second");
}

#[cfg(not(any(windows, target_os = "linux")))]
#[test]
fn unsupported_directory_publication_performs_no_staging_io() {
    let root = tempfile::tempdir().unwrap();
    assert_eq!(
        publish_artifacts(&root.path().join("published"), [("a", b"b".as_slice())])
            .unwrap_err()
            .kind(),
        io::ErrorKind::Unsupported
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
