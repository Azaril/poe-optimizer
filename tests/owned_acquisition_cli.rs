//! Offline catalogs stay optional, reproducible and separate from native use.
use std::process::Command;

#[cfg(feature = "pob")]
#[test]
fn finite_catalogs_reproduce_reviewed_data_and_refuse_overwrite() {
    use std::{fs, path::Path};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    for (command, folder) in [
        ("export-owned-actor-baselines", "actor-baselines"),
        ("export-owned-augments", "augments"),
    ] {
        let destination = temp.path().join(folder);
        let invoke = || {
            Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
                .current_dir(temp.path())
                .arg(command)
                .arg("--source-root")
                .arg(root.join("vendor/path-of-building-poe2"))
                .arg("--output")
                .arg(&destination)
                .output()
                .unwrap()
        };
        let first = invoke();
        assert!(
            first.status.success(),
            "{}",
            String::from_utf8_lossy(&first.stderr)
        );
        let expected = root
            .join("data/owned/poe2/3887ae68")
            .join(folder)
            .join("catalog.json");
        let bytes = fs::read(destination.join("catalog.json")).unwrap();
        assert_eq!(bytes, fs::read(expected).unwrap(), "{folder}");
        let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
        let evidence: serde_json::Value =
            serde_json::from_slice(&fs::read(destination.join("evidence.json")).unwrap()).unwrap();
        assert_eq!(report, evidence);
        let reviewed: serde_json::Value = serde_json::from_slice(
            &fs::read(
                root.join("data/owned/poe2/3887ae68")
                    .join(folder)
                    .join("evidence.json"),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            evidence, reviewed,
            "stale acquisition evidence for {folder}"
        );
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 2);
        assert!(!invoke().status.success());
        assert_eq!(fs::read(destination.join("catalog.json")).unwrap(), bytes);
    }
}

#[cfg(feature = "pob")]
#[test]
fn absent_source_does_not_publish_an_acquisition() {
    let temp = tempfile::tempdir().unwrap();
    for name in ["export-owned-actor-baselines", "export-owned-augments"] {
        let destination = temp.path().join(name);
        let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .arg(name)
            .arg("--source-root")
            .arg(temp.path())
            .arg("--output")
            .arg(&destination)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!destination.exists());
    }
}

#[cfg(not(feature = "pob"))]
#[test]
fn native_cli_excludes_optional_source_acquisition_commands() {
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());
    let help = String::from_utf8(output.stdout).unwrap();
    assert!(!help.contains("export-owned-actor-baselines"));
    assert!(!help.contains("export-owned-augments"));
}
