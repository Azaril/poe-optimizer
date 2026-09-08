//! Reproduce the portable subset through the isolated, hash-verified source extractor.
use poe_optimizer_data::{bundled::BundledClassTree, tree_projection::AuthenticatedTreeSnapshot};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_owned()
}
#[test]
#[ignore = "trusted offline extraction child used by bundle parity and explicit generation"]
fn bundle_extraction_child() {
    let snapshot = poe_optimizer_pob::tree_data::extract_pinned_tree(
        &root().join("vendor/path-of-building-poe2"),
        "0_5",
    )
    .unwrap();
    let digest = snapshot.sha256().unwrap();
    let source = AuthenticatedTreeSnapshot::from_trusted_extraction(snapshot, &digest).unwrap();
    let bundle = BundledClassTree::from_authenticated_snapshot(&source).unwrap();
    fs::write(
        std::env::var_os("POE_TREE_BUNDLE_OUTPUT").expect("explicit private output path"),
        bundle.canonical_bytes().unwrap(),
    )
    .unwrap();
}
#[test]
fn compiled_bundle_matches_fresh_full_source_extraction() {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("bundle.json");
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--ignored",
            "--exact",
            "bundle_extraction_child",
            "--nocapture",
        ])
        .env("POE_TREE_BUNDLE_OUTPUT", &output)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let mut child = command.spawn().unwrap();
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "source extraction child failed");
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("source extraction child timed out");
        }
        thread::sleep(Duration::from_millis(10));
    }
    assert!(fs::metadata(&output).unwrap().len() < 16 * 1024 * 1024);
    let bytes = fs::read(output).unwrap();
    let fresh = poe_optimizer_data::bundled::authenticate_bundle(&bytes).unwrap();
    assert_eq!(&fresh, poe_optimizer_data::bundled::class_tree().unwrap());
    assert_eq!(fresh.allocation_nodes.len(), 4109);
    assert_eq!(fresh.allocation_views.len(), 4758);
    assert_eq!(fresh.classes.len(), 8);
    assert_eq!(fresh.ascendancies.len(), 23);
    assert_eq!(
        fresh
            .class_entrances
            .values()
            .map(|nodes| nodes.len())
            .sum::<usize>(),
        16
    );
    assert_eq!(fresh.coverage.source_node_count, 4914);
    assert_eq!(fresh.coverage.source_dangling_connections.len(), 14);
}
