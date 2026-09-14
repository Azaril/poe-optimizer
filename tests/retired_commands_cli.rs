//! Retired schemas must not silently route into a different search product.
use std::process::Command;
#[test]
fn retired_finite_catalog_command_is_not_advertised_or_redirected() {
    let help = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("search-experimental"));
    let temp = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(temp.path())
        .args([
            "search-experimental",
            "--problem",
            "not-read.json",
            "--output",
            "not-written.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unrecognized subcommand"));
    assert!(!temp.path().join("not-written.json").exists());
}
