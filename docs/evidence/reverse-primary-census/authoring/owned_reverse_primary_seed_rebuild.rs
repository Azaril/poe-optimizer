//! One-off offline reproduction of the maintained tree seed.
//! Historical bytes are authenticated as evidence, never loaded by a legacy reader.
use crate::{
    owned_catalog_recipe::StagedCatalogRecipe,
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, NamedQuerySet, SuccessorBundleInput,
        TreePolicyTransitionInput, transition_owned_bundle, transition_owned_catalog_with_tree,
    },
    owned_tree_catalog::{
        TreeCatalogInput, TreeCatalogPolicy, compile_owned_tree_catalog_extension,
    },
};
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

const OLD_TRANSITION_SHA: &str = "a84e9c57648d6d8108d3291120d0b84e649632d4ae597e63a14aaae818233a7e";
const OLD_TRANSITION_BYTES: usize = 4502;
const FILES: [&str; 20] = [
    "registry.json",
    "schema.json",
    "rules.json",
    "routing.json",
    "manifest.json",
    "recipe.json",
    "mapping.json",
    "roles.json",
    "normalization.json",
    "rewards.json",
    "items.json",
    "item-source.json",
    "queries-original-01.json",
    "queries-original-02.json",
    "queries-original-03.json",
    "queries-original-04.json",
    "queries-original-05.json",
    "catalog-append.json",
    "tree-normalization.json",
    "transition.json",
];

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bytes(path: &Path) -> Vec<u8> {
    assert!(fs::metadata(path).unwrap().len() <= 64 * 1024 * 1024);
    fs::read(path).unwrap()
}
fn read<T: DeserializeOwned>(path: &Path) -> T {
    serde_json::from_slice(&bytes(path)).unwrap()
}
fn authenticated_seed(path: &Path) -> (Value, BTreeMap<String, Vec<u8>>) {
    let transition_bytes = bytes(&path.join("transition.json"));
    assert_eq!(transition_bytes.len(), OLD_TRANSITION_BYTES);
    assert_eq!(hash(&transition_bytes), OLD_TRANSITION_SHA);
    let transition: Value = serde_json::from_slice(&transition_bytes).unwrap();
    let pins = transition["artifacts"].as_array().unwrap();
    assert_eq!(pins.len(), FILES.len() - 1);
    let mut files = BTreeMap::new();
    let mut total = transition_bytes.len();
    for pin in pins {
        let name = pin["file"].as_str().unwrap();
        assert!(FILES.contains(&name) && name != "transition.json");
        let body = bytes(&path.join(name));
        total = total.checked_add(body.len()).unwrap();
        assert!(total <= 128 * 1024 * 1024);
        assert_eq!(pin["bytes"].as_u64().unwrap(), body.len() as u64);
        assert_eq!(pin["sha256"].as_str().unwrap(), hash(&body));
        assert!(files.insert(name.to_owned(), body).is_none());
    }
    assert!(
        files
            .insert("transition.json".to_owned(), transition_bytes)
            .is_none()
    );
    assert_eq!(
        files.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        FILES.into_iter().collect()
    );
    let disk = fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            assert!(entry.file_type().unwrap().is_file());
            entry.file_name().into_string().unwrap()
        })
        .filter(|name| name != "README.md")
        .collect::<BTreeSet<_>>();
    assert_eq!(disk, files.keys().cloned().collect());
    (transition, files)
}

pub fn rebuild_current(root: &Path, out: &Path, compiled: &StagedCatalogRecipe) -> Value {
    assert!(out.is_dir());
    let target = out.join("current");
    assert!(!target.exists(), "new candidate destination");
    let data = root.join("data/owned/poe2/3887ae68");
    // Authenticate every historical byte before inspecting its role observations.
    let (old_transition, old) = authenticated_seed(&data.join("current"));
    let queries = (1..=5)
        .map(|case| NamedQuerySet {
            name: OwnedDefinitionKey::new(format!("original-{case:02}")).unwrap(),
            queries: read(&data.join(format!("import/queries/original-{case:02}.json"))),
        })
        .collect();
    // Exact publish-owned-successor inputs from owned_bundle_cli::publish.
    let prior = transition_owned_bundle(
        SuccessorBundleInput {
            schema_version: 1,
            prior: compiled.recipe().clone(),
            successor: read(&data.join("resistance/recipe.json")),
            mapping: compiled.mapping().input().clone(),
            roles: compiled.roles().input().clone(),
            normalization: read(&data.join("import/policies/normalization.json")),
            rewards: read(&data.join("import/policies/rewards.json")),
            query_sets: queries,
            items: read(&data.join("resistance/items.json")),
            item_source: read(&data.join("resistance/item-source.json")),
        },
        Default::default(),
    )
    .unwrap();
    assert!(prior.tree().is_none());
    let catalog: TreeCatalogInput = read(&data.join("tree/tree-catalog.json"));
    let policy: TreeCatalogPolicy = read(&data.join("tree/catalog-policy.json"));
    let tree = compile_owned_tree_catalog_extension(
        prior.assembled(),
        prior.mapping(),
        &catalog,
        &policy,
        Default::default(),
    )
    .unwrap();
    assert_eq!(
        json!(tree.receipt.counts),
        json!({
            "allocated_definitions":4582,"allocated_slots":298,
            "reused_definitions":0,"reused_slots":0
        })
    );
    // Exact extend-owned-tree-catalog staging, with no alternate authoring model.
    let next = transition_owned_catalog_with_tree(
        SuccessorBundleInput {
            schema_version: 1,
            prior: prior.recipe().clone(),
            successor: tree.successor,
            mapping: prior.mapping().input().clone(),
            roles: prior.roles().input().clone(),
            normalization: prior.normalization().clone(),
            rewards: prior.rewards().input().clone(),
            query_sets: prior.query_sets().to_vec(),
            items: prior.items().input().clone(),
            item_source: prior.item_source().input().clone(),
        },
        CatalogAppend {
            mappings: tree.new_mappings,
            source: catalog.source,
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::Install {
            content: Box::new(tree.content),
        },
        Default::default(),
    )
    .unwrap();
    let fresh: BTreeMap<String, Vec<u8>> = next
        .artifacts()
        .map(|(name, body)| (name.to_owned(), body.to_vec()))
        .collect();
    assert_eq!(fresh.len(), FILES.len());
    assert_eq!(
        fresh.keys().collect::<Vec<_>>(),
        old.keys().collect::<Vec<_>>()
    );
    for name in FILES {
        if !matches!(name, "roles.json" | "transition.json") {
            assert_eq!(&fresh[name], &old[name], "unchanged seed artifact {name}");
        }
    }
    let old_roles: Value = serde_json::from_slice(&old["roles.json"]).unwrap();
    let mut role_inverse: Value = serde_json::from_slice(&fresh["roles.json"]).unwrap();
    assert_eq!(old_roles["roles"].as_array().unwrap().len(), 966);
    assert_eq!(old_roles["roles"], role_inverse["roles"]);
    assert_eq!(role_inverse["roles"], json!(compiled.roles().input().roles));
    assert!(old_roles.get("reverse_primary").is_none());
    let reverse_primary = role_inverse
        .as_object_mut()
        .unwrap()
        .remove("reverse_primary")
        .expect("required compiler reverse census");
    assert_eq!(
        reverse_primary,
        json!(compiled.roles().input().reverse_primary)
    );
    assert_eq!(reverse_primary["kind"], "observed");
    assert_eq!(role_inverse, old_roles, "exact whole role-package inverse");
    let new_transition = json!(next.transition());
    let mut transition_inverse = new_transition.clone();
    assert_ne!(new_transition["input"], old_transition["input"]);
    for lane in ["before", "after"] {
        assert_ne!(new_transition[lane]["roles"], old_transition[lane]["roles"]);
        transition_inverse[lane]["roles"] = old_transition[lane]["roles"].clone();
    }
    transition_inverse["input"] = old_transition["input"].clone();
    let old_pin = old_transition["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pin| pin["file"] == "roles.json")
        .unwrap();
    let new_pin = transition_inverse["artifacts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|pin| pin["file"] == "roles.json")
        .unwrap();
    assert_eq!(new_pin["bytes"], json!(fresh["roles.json"].len()));
    assert_eq!(new_pin["sha256"], hash(&fresh["roles.json"]));
    *new_pin = old_pin.clone();
    assert_eq!(
        transition_inverse, old_transition,
        "only checked role commitments and the resulting transition input change"
    );
    assert_eq!(next.transition().query_rows, 110);
    assert_eq!(next.transition().query_sets, 5);
    fs::create_dir(&target).unwrap();
    for (name, body) in &fresh {
        fs::write(target.join(name), body).unwrap();
    }
    let (_, unchanged) = authenticated_seed(&data.join("current"));
    assert_eq!(unchanged, old, "checked-in evidence remains immutable");
    json!({
        "candidate":target,
        "old_transition_sha256":OLD_TRANSITION_SHA,
        "new_transition_sha256":hash(&fresh["transition.json"]),
        "old_role_sha256":hash(&old["roles.json"]),
        "new_role_sha256":hash(&fresh["roles.json"]),
        "artifact_count":fresh.len(),
        "unchanged_artifact_count":fresh.len()-2,
        "source_construction_rows":966,
        "reverse_primary_routes":reverse_primary["routes"].as_array().unwrap().len(),
        "queries":110,
        "exact_role_inverse":true,
        "exact_transition_inverse":true,
        "native_recipe_unchanged":true,
        "source_execution":false,
        "calculation":"not_run"
    })
}
