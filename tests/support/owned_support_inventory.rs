//! Checked physical support inventories leave targets and effect discovery open.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::SupportOriginOrderPolicy,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_tree_policy::OwnedTreeNormalizationPolicy,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};

fn read<T: DeserializeOwned>(name: &str) -> T {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(
        &fs::read(
            root.join("data/owned/poe2/3887ae68/support-inventory")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let p: SupportOriginOrderPolicy = read("policy.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(json!(p)["mapping_source"], a["mapping_source"]);
    assert_eq!(json!(p)["roles"], a["roles"]);
    assert_eq!(
        json!(p)["kind"],
        "saved_manual_group_order_with_physical_inventory_v2"
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    for field in [
        "input",
        "definitions",
        "registry",
        "mapping",
        "roles",
        "normalization",
        "tree",
    ] {
        assert_eq!(receipt[field], a[field], "exact predecessor {field}");
    }
    assert_eq!(
        json!(prior.mapping().source_identity()),
        a["mapping_source"]
    );
    let mut full = prior.input().clone();
    assert_eq!(
        full.normalization.support_origin_order,
        Some(SupportOriginOrderPolicy::SavedManualGroupOrder {})
    );
    full.normalization.support_origin_order = Some(read("policy.json"));
    full.tree = Some(
        OwnedTreeNormalizationPolicy::bind_new(
            full.tree.as_ref().unwrap().content.clone(),
            prior.assembled().registry(),
            prior.assembled().schema(),
            prior.mapping(),
            &full.normalization,
            Default::default(),
        )
        .unwrap()
        .input()
        .clone(),
    );
    full.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("physical-support-inventory").unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned("owned-physical-support-inventory-v1", &a, 1024 * 1024)
            .unwrap(),
    });
    let next = assemble_owned_release(full, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.normalization.support_origin_order =
        prior.normalization().support_origin_order.clone();
    restored.tree = prior.input().tree.clone();
    restored.provenance.pop();
    assert_eq!(&restored, prior.input(), "no unrelated release changes");
    next
}
