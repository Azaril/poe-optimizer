//! Checked publication loader regressions; all mutations are test-only artifacts.
use super::*;
#[allow(dead_code)]
#[path = "../crates/poe-optimizer-import/tests/support/owned_compact_fixture.rs"]
mod fixture;
use poe_optimizer_import::owned_successor::{
    transition_owned_bundle, transition_owned_bundle_compact,
    transition_owned_catalog_with_tree_compact,
};
use serde_json::Value;
use std::fs;
fn input() -> SuccessorBundleInput {
    fixture::input(Path::new(env!("CARGO_MANIFEST_DIR")))
}
fn publish(staged: &StagedSuccessorBundle, directory: &Path) {
    for (name, bytes) in staged.artifacts() {
        fs::write(directory.join(name), bytes).unwrap();
    }
}
fn checked(path: &Path) -> Result<CheckedPriorBundle, Box<dyn Error>> {
    load_checked_bundle(
        path,
        &mut SuccessorBundleLimits::default().max_input_bytes,
        Default::default(),
    )
}
fn edit_manifest(root: &Path, edit: impl FnOnce(&mut Value)) {
    let file = root.join("transition.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
    edit(&mut value);
    fs::write(file, serde_json::to_vec(&value).unwrap()).unwrap();
}
fn rehash(root: &Path, name: &str) {
    let bytes = fs::read(root.join(name)).unwrap();
    edit_manifest(root, |value| {
        let row = value["artifacts"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|r| r["file"] == name)
            .unwrap();
        row["bytes"] = bytes.len().into();
        row["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    });
}

#[test]
fn checked_loader_keeps_noncanonical_v1_and_reconstructs_compact_v2() {
    let mut supplied = input();
    supplied.successor.schema.definitions.reverse();
    let legacy = transition_owned_bundle(supplied.clone(), Default::default()).unwrap();
    let compact = transition_owned_bundle_compact(supplied, Default::default()).unwrap();
    for staged in [&legacy, &compact] {
        let directory = tempfile::tempdir().unwrap();
        publish(staged, directory.path());
        let loaded = checked(directory.path()).unwrap();
        assert_eq!(&loaded.input.prior, staged.recipe());
        assert_eq!(&loaded.input.query_sets, staged.query_sets());
        assert_eq!(&loaded.input.items, staged.items().input());
        assert_eq!(&loaded.input.item_source, staged.item_source().input());
        assert_eq!(loaded.after, staged.transition().after);
        let next = transition_owned_bundle_compact(
            loaded.successor_input(loaded.input.prior.clone()),
            Default::default(),
        )
        .unwrap();
        loaded.check_transition(&next).unwrap();
    }
}

#[test]
fn compact_loader_rejects_rehashed_noncanonical_constituent_or_forged_recipe_manifest() {
    let staged = transition_owned_bundle_compact(input(), Default::default()).unwrap();
    for artifact in ["schema.json", "manifest.json"] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        publish(&staged, root);
        let file = root.join(artifact);
        let mut value: Value = serde_json::from_slice(&fs::read(&file).unwrap()).unwrap();
        if artifact == "schema.json" {
            value["definitions"].as_array_mut().unwrap().reverse();
        } else {
            value["recipe"] = "00".repeat(32).into();
        }
        fs::write(file, serde_json::to_vec(&value).unwrap()).unwrap();
        rehash(root, artifact);
        assert!(checked(root).is_err(), "{artifact}");
    }
}

#[test]
fn compact_loader_rejects_omitted_queries_and_every_unlisted_artifact() {
    let staged = transition_owned_bundle_compact(input(), Default::default()).unwrap();
    for extra in ["recipe.json", "unknown.json"] {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path();
        publish(&staged, root);
        fs::write(root.join(extra), b"{}").unwrap();
        assert!(checked(root).is_err());
    }
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    publish(&staged, root);
    edit_manifest(root, |manifest| {
        manifest["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["file"] != "queries-original-01.json");
        manifest["query_sets"] = 4.into();
        manifest["query_rows"] = 88.into();
    });
    assert!(checked(root).is_err());
}

#[test]
fn compact_tree_identity_endpoint_and_aggregate_byte_checks_remain_enforced() {
    let first = transition_owned_bundle(input(), Default::default()).unwrap();
    let next = fixture::next(&first);
    let staged = transition_owned_catalog_with_tree_compact(
        next.clone(),
        fixture::append(&next),
        fixture::tree(&next),
        Default::default(),
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path();
    publish(&staged, root);
    let loaded = checked(root).unwrap();
    assert_eq!(loaded.tree.as_ref(), Some(staged.tree().unwrap().input()));
    let size = staged
        .artifacts()
        .map(|(_, bytes)| bytes.len())
        .sum::<usize>();
    assert!(load_checked_bundle(root, &mut (size - 1), Default::default()).is_err());
    edit_manifest(root, |manifest| manifest["tree"] = "00".repeat(32).into());
    assert!(checked(root).is_err());
    publish(&staged, root);
    edit_manifest(root, |manifest| {
        manifest["artifacts"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["file"] != "tree-normalization.json");
        manifest.as_object_mut().unwrap().remove("tree");
    });
    assert!(checked(root).is_err());
    publish(&staged, root);
    edit_manifest(root, |manifest| {
        manifest["after"]["rules"] = "00".repeat(32).into()
    });
    let loaded = checked(root).unwrap();
    let next = transition_owned_bundle_compact(
        loaded.successor_input(loaded.input.prior.clone()),
        Default::default(),
    )
    .unwrap();
    assert!(loaded.check_transition(&next).is_err());
}
