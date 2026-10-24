//! Offline catalogs stay optional, reproducible and separate from native use.
use std::process::Command;

#[cfg(feature = "pob")]
#[test]
fn finite_catalogs_reproduce_reviewed_data_and_refuse_overwrite() {
    use std::{collections::BTreeMap, fs, path::Path};
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    for (command, folder) in [
        ("export-owned-actor-baselines", "actor-baselines"),
        ("export-owned-augments", "augments"),
        (
            "export-owned-skill-definition-facts",
            "skill-definition-facts",
        ),
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
        let snapshot = || {
            fs::read_dir(&destination)
                .unwrap()
                .map(|entry| {
                    let entry = entry.unwrap();
                    (entry.file_name(), fs::read(entry.path()).unwrap())
                })
                .collect::<BTreeMap<_, _>>()
        };
        let before = snapshot();
        assert!(!invoke().status.success());
        assert_eq!(snapshot(), before);
    }
}

#[cfg(feature = "pob")]
#[test]
fn optional_export_reproduces_checked_catalog_without_runtime_ui() {
    use std::{collections::BTreeMap, fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let data = root.join("data/owned/poe2/3887ae68/intrinsic-attack");
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(temp.path())
            .arg("export-owned-intrinsic-attack")
            .arg("--data-lua")
            .arg(root.join("vendor/path-of-building-poe2/src/Modules/Data.lua"))
            .arg("--misc-lua")
            .arg(root.join("vendor/path-of-building-poe2/src/Data/Misc.lua"))
            .arg("--source-pin")
            .arg(data.join("source-pin.json"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let first = call();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["classes"], 9);
    assert_eq!(
        fs::read(output.join("catalog.json")).unwrap(),
        fs::read(data.join("catalog.json")).unwrap()
    );
    let policy: serde_json::Value =
        serde_json::from_slice(&fs::read(data.join("policy.json")).unwrap()).unwrap();
    assert_eq!(report["catalog_sha256"], policy["catalog_sha256"]);
    let snapshot = || {
        fs::read_dir(&output)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = snapshot();
    assert!(!call().status.success());
    assert_eq!(snapshot(), before);
}

#[cfg(feature = "pob")]
#[test]
fn optional_item_base_export_reproduces_catalog_and_does_not_replace_output() {
    use std::{collections::BTreeMap, fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(temp.path())
            .arg("export-owned-item-bases")
            .arg("--source-root")
            .arg(root.join("vendor/path-of-building-poe2"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let first = call();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["bases"], 1756);
    assert_eq!(report["table_weapon_fields"], 337);
    assert_eq!(
        fs::read(output.join("catalog.json")).unwrap(),
        fs::read(root.join("data/owned/poe2/3887ae68/item-bases/catalog.json")).unwrap()
    );
    let snapshot = || {
        fs::read_dir(&output)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = snapshot();
    assert!(!call().status.success());
    assert_eq!(snapshot(), before);
}

#[cfg(feature = "pob")]
#[test]
fn optional_export_reproduces_both_catalogs_without_overwrite() {
    use std::{collections::BTreeMap, fs, path::Path};

    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("exported");
    let call = || {
        Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
            .current_dir(temp.path())
            .arg("export-owned-weapon-profiles")
            .arg("--source-root")
            .arg(root.join("vendor/path-of-building-poe2"))
            .arg("--output")
            .arg(&output)
            .output()
            .unwrap()
    };
    let first = call();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&first.stdout).unwrap();
    assert_eq!(report["profiles"], 337);
    for (actual, expected) in [
        ("catalog.json", "weapon-profiles/catalog.json"),
        ("base-catalog.json", "item-bases/catalog.json"),
    ] {
        assert_eq!(
            fs::read(output.join(actual)).unwrap(),
            fs::read(root.join("data/owned/poe2/3887ae68").join(expected)).unwrap()
        );
    }
    let snapshot = || {
        fs::read_dir(&output)
            .unwrap()
            .map(|entry| {
                let entry = entry.unwrap();
                (entry.file_name(), fs::read(entry.path()).unwrap())
            })
            .collect::<BTreeMap<_, _>>()
    };
    let before = snapshot();
    assert!(!call().status.success());
    assert_eq!(snapshot(), before);
}

#[cfg(feature = "pob")]
#[test]
fn absent_source_does_not_publish_an_acquisition() {
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "export-owned-actor-baselines",
        "export-owned-augments",
        "export-owned-skill-definition-facts",
    ] {
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
    assert!(!help.contains("export-owned-intrinsic-attack"));
    assert!(!help.contains("export-owned-item-bases"));
    assert!(!help.contains("export-owned-weapon-profiles"));
    assert!(!help.contains("export-owned-skill-definition-facts"));
}
