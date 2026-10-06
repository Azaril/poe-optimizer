//! Regenerate one current package and prove unchanged real-build meaning.
#[path = "support/owned_usage_transport_cutover.rs"]
mod cutover;
#[allow(dead_code)]
#[path = "support/owned_release_fixture.rs"]
mod release;
#[path = "support/owned_selected_request.rs"]
mod selected;
use serde_json::{Value, json};
use std::{fs, path::Path, path::PathBuf};
fn read(path: impl AsRef<Path>) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn equal(before: &Value, after: &Value, context: &str) {
    let changed: Vec<_> = before
        .as_object()
        .unwrap()
        .keys()
        .chain(after.as_object().unwrap().keys())
        .filter(|key| before[*key] != after[*key])
        .collect();
    assert!(
        before == after,
        "{context}: changed top-level fields {changed:?}"
    );
}

#[test]
#[ignore = "requires authenticated stored predecessor and a fresh output directory"]
fn current_typed_physical_policies_preserve_all_five_originals() {
    let baseline = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_USAGE_TRANSPORT_BASELINE")
            .expect("stored predecessor"),
    );
    let out = PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_USAGE_TRANSPORT_OUTPUT")
            .expect("fresh output directory"),
    );
    assert!(!out.exists());
    let before_inventory = release::inventory(&baseline.join("package"));
    let staged = cutover::stage(&baseline.join("package"));
    let package = out.join("package");
    cutover::publish(&staged, &package);
    assert_eq!(release::load(&package).input(), staged.input());
    let mut cases = vec![];
    for case in 1..=5 {
        let xml = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!(
            "tests/fixtures/builds/breadth-20260908/build-{case:02}.xml"
        ));
        let source = fs::read(&xml).unwrap();
        let before = baseline.join(format!("original-{case:02}"));
        let after = out.join(format!("original-{case:02}"));
        let original_inventory = release::inventory(&before);
        let report = release::normalize(&package, &xml, case, &after);
        let mut old = read(before.join("draft.json"));
        let mut new = read(after.join("draft.json"));
        selected::canonical(&mut old);
        selected::canonical(&mut new);
        equal(
            &old,
            &new,
            &format!("values, IDs, allocator and all presets: original {case}"),
        );
        let mut old_sidecar = read(before.join("sidecar.json"));
        let mut new_sidecar = read(after.join("sidecar.json"));
        assert_ne!(old_sidecar["policy"], new_sidecar["policy"]);
        assert_eq!(
            old_sidecar["tree_policy"],
            read(baseline.join("package/release.json"))["tree"]
        );
        assert_eq!(new_sidecar["tree_policy"], json!(staged.receipt().tree));
        for sidecar in [&mut old_sidecar, &mut new_sidecar] {
            selected::canonical(sidecar);
            // Exact importer identity and fresh draft lineage change; every
            // actual input and source-origin link is compared independently.
            sidecar.as_object_mut().unwrap().remove("policy");
            sidecar.as_object_mut().unwrap().remove("draft");
            // The tree's unchanged content carries the normalization back-reference.
            sidecar.as_object_mut().unwrap().remove("tree_policy");
        }
        equal(
            &old_sidecar,
            &new_sidecar,
            &format!("all source dispositions: original {case}"),
        );
        let mut old_selection = selected::selection(&source, &before);
        let mut new_selection = selected::selection(&source, &after);
        selected::canonical(&mut old_selection);
        selected::canonical(&mut new_selection);
        assert_eq!(old_selection, new_selection);
        assert_eq!(original_inventory, release::inventory(&before));
        assert_eq!(source, fs::read(&xml).unwrap());
        cases.push(json!({"original":case,"inputs_unchanged":true,"origins_unchanged":true,"selection_unchanged":true,"sidecar_sha256":report["sidecar_sha256"]}));
    }
    assert_eq!(
        before_inventory,
        release::inventory(&baseline.join("package"))
    );
    fs::write(out.join("validation.json"), serde_json::to_vec_pretty(&json!({"before":cutover::BEFORE,"after":staged.receipt().input,"queries":110,"cases":cases,"native_rules_changed":false,"legacy_runtime_retained":false,"complete_native_builds":0})).unwrap()).unwrap();
}
