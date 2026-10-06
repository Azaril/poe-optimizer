//! One authenticated offline format cutover; never part of the runtime loader.
//! Preserve old evidence while rebuilding the single current usage contract.
use poe_optimizer_core::{owned_content::digest_owned, owned_definitions::OwnedDefinitionKey};
use poe_optimizer_import::{
    owned_normalize::{
        GemInventoryPolicy, NormalizationLimits, NormalizationPolicy, usage_inputs_identity,
    },
    owned_recipe::OwnedRecipeInput,
    owned_release::{
        OwnedReleaseInput, OwnedReleaseProvenance, OwnedReleaseReceipt, StagedOwnedRelease,
        assemble_owned_release,
    },
    owned_successor::NamedQuerySet,
    owned_tree_policy::TreeNormalizationPackageInput,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub const BEFORE: &str = "344d3a2c278844c6ea9b2d1f2e27edde492e23392f8500aec5f3f3baf533051f";
const RECEIPT_SHA: &str = "c63b75dc2c10f629e1c3c0aa7c1579a589700940a6034ce0614080f6792cb688";
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read<T: DeserializeOwned>(files: &BTreeMap<String, Vec<u8>>, name: &str) -> T {
    serde_json::from_slice(&files[name]).unwrap()
}

fn usage_shape(before: &Value) -> Value {
    assert_eq!(before["kind"], "pob_occurrence_usage_v3");
    let mut after = before.clone();
    let object = after.as_object_mut().unwrap();
    assert!(!object.contains_key("physical"));
    let boolean = object.remove("gems").unwrap();
    let numeric = object.remove("numeric_gems").unwrap();
    assert_eq!(boolean.as_array().unwrap().len(), 2);
    assert_eq!(numeric.as_array().unwrap().len(), 4);
    let mut physical = vec![];
    for (rows, occurrence_only) in [
        (boolean.as_array().unwrap(), true),
        (numeric.as_array().unwrap(), false),
    ] {
        for old in rows {
            let mut row = old.clone();
            let row = row.as_object_mut().unwrap();
            let policy = row.remove("policy").unwrap();
            let mut parameters = row.remove("parameters").unwrap();
            if occurrence_only {
                assert!(row.insert("group_attributes".into(), json!([])).is_none());
                assert!(row.insert("group_guards".into(), json!([])).is_none());
                for p in parameters.as_array_mut().unwrap() {
                    let p = p.as_object_mut().unwrap();
                    let value = p.remove("value").unwrap();
                    assert!(
                        p.insert("source".into(), json!({"kind":"occurrence","value":value}))
                            .is_none()
                    );
                }
            }
            row.insert(
                "policies".into(),
                json!([{"policy":policy,"parameters":parameters}]),
            );
            let converted = Value::Object(row.clone());
            // Prove this is exactly a representation change, including source
            // frame order and every recipe/default/guard, before typed loading.
            let mut inverse = converted.clone();
            let inverse = inverse.as_object_mut().unwrap();
            let policies = inverse.remove("policies").unwrap();
            assert_eq!(policies.as_array().unwrap().len(), 1);
            inverse.insert("policy".into(), policies[0]["policy"].clone());
            let mut restored = policies[0]["parameters"].clone();
            if occurrence_only {
                assert_eq!(inverse.remove("group_attributes"), Some(json!([])));
                assert_eq!(inverse.remove("group_guards"), Some(json!([])));
                for p in restored.as_array_mut().unwrap() {
                    let p = p.as_object_mut().unwrap();
                    let source = p.remove("source").unwrap();
                    assert_eq!(source["kind"], "occurrence");
                    p.insert("value".into(), source["value"].clone());
                }
            }
            inverse.insert("parameters".into(), restored);
            assert_eq!(&Value::Object(inverse.clone()), old);
            physical.push(converted);
        }
    }
    object.insert("physical".into(), json!(physical));
    after
}

pub fn stage(path: &Path) -> StagedOwnedRelease {
    let receipt_bytes = fs::read(path.join("release.json")).unwrap();
    assert_eq!(hash(&receipt_bytes), RECEIPT_SHA);
    let receipt: OwnedReleaseReceipt = serde_json::from_slice(&receipt_bytes).unwrap();
    assert_eq!(json!(receipt.input), BEFORE);
    assert_eq!(receipt.schema_version, 1);
    assert!(receipt.evaluation.is_none());
    let mut files = BTreeMap::new();
    let mut total = receipt_bytes.len();
    for artifact in &receipt.artifacts {
        assert!(!artifact.file.contains(['/', '\\']) && !artifact.file.contains(".."));
        let bytes = fs::read(path.join(&artifact.file)).unwrap();
        total = total.checked_add(bytes.len()).unwrap();
        assert!(total <= 64 * 1024 * 1024);
        assert_eq!(json!(artifact.bytes), json!(bytes.len()));
        assert_eq!(json!(artifact.sha256), hash(&bytes));
        assert!(files.insert(artifact.file.clone(), bytes).is_none());
    }
    assert_eq!(files.len() + 1, fs::read_dir(path).unwrap().count());
    let before: Value = read(&files, "normalization.json");
    let usage_before = before["usage_inputs"].clone();
    let usage_after = usage_shape(&usage_before);
    let mut current = before.clone();
    current["usage_inputs"] = usage_after.clone();
    let mut normalization: NormalizationPolicy = serde_json::from_value(current).unwrap();
    let usage = usage_inputs_identity(&normalization, Default::default()).unwrap();
    let Some(GemInventoryPolicy::PobFreshPhysicalV3 { usage_inputs, .. }) =
        &mut normalization.gem_inventory
    else {
        panic!("expected exact current physical inventory");
    };
    *usage_inputs = usage;
    let mut inverse = json!(normalization);
    inverse["usage_inputs"] = usage_before.clone();
    inverse["gem_inventory"]["usage_inputs"] = before["gem_inventory"]["usage_inputs"].clone();
    assert_eq!(
        inverse, before,
        "no other normalization rule or coverage changes"
    );
    let mut tree: TreeNormalizationPackageInput = read(&files, "tree-normalization.json");
    let old_tree = tree.clone();
    assert_eq!(tree.normalization, receipt.normalization);
    tree.normalization = digest_owned(
        "owned-normalization-policy-v3",
        &normalization,
        NormalizationLimits::default().max_policy_bytes,
    )
    .unwrap();
    let mut restored_tree = tree.clone();
    restored_tree.normalization = old_tree.normalization;
    assert_eq!(
        restored_tree, old_tree,
        "tree changes only its dependency commitment"
    );
    let mut provenance = receipt.provenance.clone();
    provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new("physical-usage-policy-unification").unwrap(),
        prior_input: receipt.input,
        authoring_input: digest_owned(
            "physical-usage-policy-unification",
            &json!({"before":usage_before,"after":usage_after}),
            1024 * 1024,
        )
        .unwrap(),
    });
    let staged = assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: OwnedRecipeInput {
                schema_version: 1,
                registry: read(&files, "registry.json"),
                schema: read(&files, "schema.json"),
                rules: read(&files, "rules.json"),
                routing: read(&files, "routing.json"),
            },
            mapping: read(&files, "mapping.json"),
            roles: read(&files, "roles.json"),
            normalization,
            rewards: read(&files, "rewards.json"),
            items: read(&files, "items.json"),
            item_source: read(&files, "item-source.json"),
            tree: Some(tree),
            evaluation: None,
            query_sets: receipt
                .artifacts
                .iter()
                .filter_map(|a| {
                    a.file
                        .strip_prefix("queries-")
                        .and_then(|s| s.strip_suffix(".json"))
                        .map(|name| NamedQuerySet {
                            name: OwnedDefinitionKey::new(name).unwrap(),
                            queries: read(&files, &a.file),
                        })
                })
                .collect(),
            provenance,
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(staged.receipt().query_rows, 110);
    for field in [
        "registry",
        "definitions",
        "rules",
        "compiled_rules",
        "routing",
        "mapping",
        "roles",
        "rewards",
        "items",
        "item_source",
    ] {
        assert_eq!(
            json!(staged.receipt())[field],
            json!(receipt)[field],
            "preserved {field}"
        );
    }
    for (name, bytes) in staged.artifacts() {
        if ![
            "normalization.json",
            "tree-normalization.json",
            "release.json",
        ]
        .contains(&name)
        {
            assert_eq!(bytes, files[name], "unchanged artifact {name}");
        }
    }
    assert_ne!(staged.receipt().input, receipt.input);
    staged
}

pub fn publish(staged: &StagedOwnedRelease, path: &Path) {
    assert!(!path.exists());
    fs::create_dir_all(path).unwrap();
    for (name, bytes) in staged.artifacts() {
        fs::write(path.join(name), bytes).unwrap();
    }
}
