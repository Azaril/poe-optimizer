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

fn bundle_limits() -> ArtifactBundleLimits {
    ArtifactBundleLimits {
        max_entries: 8,
        max_bytes: 128,
    }
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn bundle_publication_preserves_nested_inventory_and_exact_aggregate_limits() {
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("checkpoint μ");
    let root = [
        ArtifactFile {
            name: "names.json",
            bytes: b"names",
        },
        ArtifactFile {
            name: "checkpoint.json",
            bytes: b"receipt",
        },
    ];
    let runtime = [
        ArtifactFile {
            name: "release.json",
            bytes: b"release",
        },
        ArtifactFile {
            name: "registry.json",
            bytes: b"registry",
        },
    ];
    let evidence = [ArtifactFile {
        name: "names.json",
        bytes: b"source",
    }];
    let directories = [
        ArtifactDirectory {
            name: "runtime",
            files: &runtime,
        },
        ArtifactDirectory {
            name: "evidence",
            files: &evidence,
        },
    ];
    // Two root files + two directories + three child files; 5+7+7+8+6 bytes.
    let exact = ArtifactBundleLimits {
        max_entries: 7,
        max_bytes: 33,
    };
    for limits in [
        ArtifactBundleLimits {
            max_entries: 6,
            ..exact
        },
        ArtifactBundleLimits {
            max_bytes: 32,
            ..exact
        },
        ArtifactBundleLimits {
            max_entries: 0,
            ..exact
        },
        ArtifactBundleLimits {
            max_bytes: 0,
            ..exact
        },
    ] {
        let error = publish_artifact_bundle(&output, &root, &directories, limits).unwrap_err();
        assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
        assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
    }
    publish_artifact_bundle(&output, &root, &directories, exact).unwrap();
    assert_eq!(fs::read_dir(&output).unwrap().count(), 4);
    assert_eq!(fs::read_dir(output.join("runtime")).unwrap().count(), 2);
    assert_eq!(fs::read_dir(output.join("evidence")).unwrap().count(), 1);
    for file in root {
        assert_eq!(fs::read(output.join(file.name)).unwrap(), file.bytes);
    }
    for directory in directories {
        for file in directory.files {
            assert_eq!(
                fs::read(output.join(directory.name).join(file.name)).unwrap(),
                file.bytes
            );
        }
    }
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 1);
    assert_eq!(
        publish_artifact_bundle(&output, &root, &directories, exact)
            .unwrap_err()
            .kind(),
        io::ErrorKind::AlreadyExists,
    );
    assert_eq!(
        fs::read(output.join("runtime/release.json")).unwrap(),
        b"release"
    );
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 1);
}

#[test]
fn bundle_name_collisions_are_rejected_before_any_staging() {
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("checkpoint");
    for second in ["same", "SaMe"] {
        let file = ArtifactFile {
            name: "same",
            bytes: b"first",
        };
        let duplicate = ArtifactFile {
            name: second,
            bytes: b"second",
        };
        let children = [file, duplicate];
        for (roots, directories) in [
            (vec![file, duplicate], vec![]),
            (
                vec![file],
                vec![ArtifactDirectory {
                    name: second,
                    files: &[],
                }],
            ),
            (
                vec![],
                vec![
                    ArtifactDirectory {
                        name: "same",
                        files: &[],
                    },
                    ArtifactDirectory {
                        name: second,
                        files: &[],
                    },
                ],
            ),
            (
                vec![],
                vec![ArtifactDirectory {
                    name: "runtime",
                    files: &children,
                }],
            ),
        ] {
            let error = publish_artifact_bundle(&output, &roots, &directories, bundle_limits())
                .unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
            assert_eq!(error.to_string(), "artifact names collide");
            assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
        }
    }
}

#[test]
fn bundle_names_are_portable_in_every_position_before_filesystem_access() {
    let parent = tempfile::tempdir().unwrap();
    // The absent parent would produce an I/O error if validation reached staging.
    let output = parent.path().join("missing/checkpoint");
    let too_long = "a".repeat(256);
    for name in [
        "",
        ".",
        "..",
        "../escape",
        "a/b",
        "a\\b",
        "C:stream",
        "a\0b",
        "a\nb",
        "CON",
        "aux.json",
        "Nul",
        "com1.txt",
        "LPT9",
        "CONIN$",
        "conout$.json",
        "clock$",
        "CON .json",
        "trailing.",
        "trailing ",
        "a?b",
        "a*b",
        "a|b",
        "a<b",
        "a>b",
        "a\"b",
        "café.json",
        too_long.as_str(),
    ] {
        let invalid_file = ArtifactFile {
            name,
            bytes: b"payload",
        };
        let child = [invalid_file];
        for (roots, directories) in [
            (vec![invalid_file], vec![]),
            (vec![], vec![ArtifactDirectory { name, files: &[] }]),
            (
                vec![],
                vec![ArtifactDirectory {
                    name: "runtime",
                    files: &child,
                }],
            ),
        ] {
            assert_eq!(
                publish_artifact_bundle(&output, &roots, &directories, bundle_limits())
                    .unwrap_err()
                    .kind(),
                io::ErrorKind::InvalidInput,
                "accepted {name:?}",
            );
        }
    }
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn staged_nested_bundle_failure_and_racing_destination_leave_no_partial_publication() {
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("checkpoint");
    // Exercise the shared publication transaction at its real write boundary.
    let error = publish_directory(&output, |staging| {
        fs::create_dir(staging.join("runtime"))?;
        write_artifact(&staging.join("names.json"), b"ledger")?;
        write_artifact(&staging.join("runtime/release.json"), b"complete")?;
        Err(io::Error::other("injected write failure"))
    })
    .unwrap_err();
    assert_eq!(error.to_string(), "injected write failure");
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 0);

    assert!(
        publish_directory(&output, |staging| {
            fs::create_dir(staging.join("runtime"))?;
            write_artifact(&staging.join("names.json"), b"loser")?;
            write_artifact(&staging.join("runtime/release.json"), b"loser")?;
            // The destination appears only after the initial absence check.
            fs::create_dir(&output)?;
            Ok(())
        })
        .is_err()
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 1);
}

#[cfg(any(windows, target_os = "linux"))]
#[test]
fn concurrent_bundle_publishers_have_one_complete_winner() {
    let parent = tempfile::tempdir().unwrap();
    let output = parent.path().join("checkpoint");
    let barrier = std::sync::Barrier::new(2);
    let results = std::thread::scope(|scope| {
        let handles: Vec<_> = [b"first".as_slice(), b"second".as_slice()]
            .into_iter()
            .map(|value| {
                let barrier = &barrier;
                let output = &output;
                scope.spawn(move || {
                    let root = [ArtifactFile {
                        name: "names.json",
                        bytes: value,
                    }];
                    let runtime = [ArtifactFile {
                        name: "release.json",
                        bytes: value,
                    }];
                    barrier.wait();
                    publish_artifact_bundle(
                        output,
                        &root,
                        &[ArtifactDirectory {
                            name: "runtime",
                            files: &runtime,
                        }],
                        bundle_limits(),
                    )
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .collect::<Vec<_>>()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert_eq!(results.iter().filter(|result| result.is_err()).count(), 1);
    let names = fs::read(output.join("names.json")).unwrap();
    assert!(names == b"first" || names == b"second");
    assert_eq!(
        fs::read(output.join("runtime/release.json")).unwrap(),
        names
    );
    assert_eq!(fs::read_dir(&output).unwrap().count(), 2);
    assert_eq!(fs::read_dir(output.join("runtime")).unwrap().count(), 1);
    assert_eq!(fs::read_dir(parent.path()).unwrap().count(), 1);
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
    assert_eq!(
        publish_artifact_bundle(
            &root.path().join("bundle"),
            &[ArtifactFile {
                name: "names.json",
                bytes: b"names"
            }],
            &[ArtifactDirectory {
                name: "runtime",
                files: &[]
            }],
            bundle_limits(),
        )
        .unwrap_err()
        .kind(),
        io::ErrorKind::Unsupported,
    );
    assert_eq!(fs::read_dir(root.path()).unwrap().count(), 0);
}
