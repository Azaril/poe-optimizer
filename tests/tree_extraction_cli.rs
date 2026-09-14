#![cfg(feature = "pob")]
//! Offline tree extraction remains independent of retired finite-catalog search.
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};
fn command() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")));
    command
}
fn report(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn tree_cli_exports_versioned_data_and_rejects_overwrite_or_unknown_version() {
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("tree.json");
    let summary = report(
        command()
            .args(["extract-tree", "--timeout-seconds", "30", "--output"])
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert_eq!(summary["classes"], 8);
    assert_eq!(summary["ascendancies"], 23);
    assert_eq!(summary["nodes"], 4914);
    assert_eq!(summary["dangling_connections"], 14);
    let snapshot: poe_optimizer_pob::tree_data::TreeDataSnapshot =
        serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(snapshot.sha256().unwrap(), summary["snapshot_sha256"]);
    assert_eq!(
        snapshot.classes[&1].start_node_id,
        snapshot.classes[&7].start_node_id
    );
    assert!(
        !command()
            .args(["extract-tree", "--output"])
            .arg(&out)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !command()
            .args(["extract-tree", "--tree-version", "unknown", "--output"])
            .arg(temp.path().join("bad.json"))
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!temp.path().join("bad.json").exists());
}
