//! Shared checked V1 release fixtures; no caller-specific semantic rewrites.
use poe_optimizer_core::owned_definitions::OwnedDefinitionKey;
use poe_optimizer_import::{
    owned_recipe::OwnedRecipeInput,
    owned_release::{
        OwnedReleaseInput, OwnedReleaseReceipt, StagedOwnedRelease, assemble_owned_release,
    },
    owned_successor::NamedQuerySet,
};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::Path,
    process::{Command, Output},
};
fn read<T: DeserializeOwned>(p: impl AsRef<Path>) -> T {
    serde_json::from_slice(&fs::read(p).unwrap()).unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}
fn command(name: &str) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.arg(name);
    c
}
fn success(o: Output) -> Value {
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    serde_json::from_slice(&o.stdout).unwrap()
}
pub fn inventory(p: &Path) -> BTreeMap<String, String> {
    fs::read_dir(p)
        .unwrap()
        .map(|v| {
            let v = v.unwrap();
            assert!(v.file_type().unwrap().is_file());
            (
                v.file_name().into_string().unwrap(),
                format!("{:x}", Sha256::digest(fs::read(v.path()).unwrap())),
            )
        })
        .collect()
}
pub fn load(p: &Path) -> StagedOwnedRelease {
    let r: OwnedReleaseReceipt = read(p.join("release.json"));
    assert_eq!(r.schema_version, 1);
    assert!(r.evaluation.is_none());
    let s = assemble_owned_release(
        OwnedReleaseInput {
            schema_version: 1,
            recipe: OwnedRecipeInput {
                schema_version: 1,
                registry: read(p.join("registry.json")),
                schema: read(p.join("schema.json")),
                rules: read(p.join("rules.json")),
                routing: read(p.join("routing.json")),
            },
            mapping: read(p.join("mapping.json")),
            roles: read(p.join("roles.json")),
            normalization: read(p.join("normalization.json")),
            rewards: read(p.join("rewards.json")),
            items: read(p.join("items.json")),
            item_source: read(p.join("item-source.json")),
            tree: r.tree.map(|_| read(p.join("tree-normalization.json"))),
            evaluation: None,
            query_sets: r
                .artifacts
                .iter()
                .filter_map(|v| {
                    v.file
                        .strip_prefix("queries-")
                        .and_then(|v| v.strip_suffix(".json"))
                        .map(|n| NamedQuerySet {
                            name: key(n),
                            queries: read(p.join(&v.file)),
                        })
                })
                .collect(),
            provenance: r.provenance.clone(),
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(*s.receipt(), r);
    assert_eq!(s.artifacts().count(), inventory(p).len());
    for (n, b) in s.artifacts() {
        assert_eq!(b, fs::read(p.join(n)).unwrap());
    }
    s
}
pub fn normalize(package: &Path, xml: &Path, case: usize, out: &Path) -> Value {
    let mut c = command("normalize-owned");
    c.arg(xml);
    for (flag, file) in [
        ("--policy", "normalization.json"),
        ("--registry", "registry.json"),
        ("--definitions", "schema.json"),
        ("--mapping", "mapping.json"),
        ("--roles", "roles.json"),
        ("--rewards", "rewards.json"),
        ("--items", "items.json"),
        ("--item-source", "item-source.json"),
        ("--tree-policy", "tree-normalization.json"),
    ] {
        c.arg(flag).arg(package.join(file));
    }
    let r = success(
        c.arg("--queries")
            .arg(package.join(format!("queries-original-{case:02}.json")))
            .arg("--output")
            .arg(out)
            .output()
            .unwrap(),
    );
    assert_eq!(r["normalization_status"], "pending");
    assert_eq!(r["verification"]["calculation"], "not_run");
    r
}
