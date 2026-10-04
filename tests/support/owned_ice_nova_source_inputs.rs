//! Checked source-input fragments; publication never closes real build inventories.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/ice-nova-source-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(m.schema_version, 3);
    assert_eq!(m.contract.schema_version, 5);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v18"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert!(m.tables.is_empty() && m.receivers.is_empty() && m.query_targets.is_empty());
    assert!(m.evaluation.is_none());
    assert_eq!(a["registry_last_issued_before"], 0x32df);
    assert_eq!(a["registry_last_issued_after"], 0x32e2);
    assert_eq!(a["allocated_definitions"], 3);
    assert_eq!(m.schema.len(), 3);
    for field in ["before", "definitions", "registry", "roles"] {
        assert_eq!(b[field], a[field]);
    }
    assert_eq!(d["source"]["input"], a["before"]);
    assert_eq!(d["source"]["definitions"], a["definitions"]);
    for owner in &m.owners {
        assert!(
            !owner.programs.is_complete(),
            "fragments cannot certify an actual owner"
        );
        assert!(!owner.programs.members.is_empty());
    }
    let artifacts = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        artifacts.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, hash) in artifacts {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*hash, json!(format!("{:x}", Sha256::digest(bytes))));
    }
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut paths = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    assert_eq!(paths.len(), 28);
}

fn source_proof() {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    let p = &a["source_validation"];
    assert_eq!(p["status"], "passed");
    let mut reports = Vec::new();
    for (path, bytes, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let raw = fs::read(root().join(p[path].as_str().unwrap())).unwrap();
        assert_eq!(raw.len() as u64, p[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), p[hash]);
        reports.push(raw);
    }
    assert_eq!(reports[0], reports[1], "source JIT modes must agree");
    assert_eq!(v["source_report_sha256"], p["evidence_sha256"]);
    let report: Value = serde_json::from_slice(&reports[0]).unwrap();
    assert_eq!(
        report["cases"].as_array().unwrap().len() as u64,
        p["cases"].as_u64().unwrap()
    );
    assert_eq!(report["lifecycle_stages"], p["lifecycle_stages"]);
    let prior_authoring: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/ice-nova-intrinsics/authoring.json"))
            .unwrap(),
    )
    .unwrap();
    let dense = &a["dense_level_policy_evidence"]["source_validation"];
    assert_eq!(*dense, prior_authoring["source_validation"]);
    for (path, bytes, hash) in [
        ("evidence_json", "evidence_bytes", "evidence_sha256"),
        (
            "evidence_on_json",
            "evidence_on_bytes",
            "evidence_on_sha256",
        ),
    ] {
        let raw = fs::read(root().join(dense[path].as_str().unwrap())).unwrap();
        assert_eq!(raw.len() as u64, dense[bytes].as_u64().unwrap());
        assert_eq!(format!("{:x}", Sha256::digest(&raw)), dense[hash]);
    }
    for (field, authored) in [
        ("source_revision", "source_revision"),
        ("manifest_sha256", "source_manifest_sha256"),
        ("files", "source_files"),
        ("original_sources", "original_sources"),
    ] {
        assert_eq!(report[field], a[authored]);
    }
    for field in [
        "business_wrappers",
        "source_tables_mutated",
        "native_inventory_authority",
        "final_input_authority",
        "native_build_parity",
        "canonical_parity_lifecycle_selected",
    ] {
        assert_eq!(report[field], false);
    }
    let mut pointers = BTreeSet::new();
    for observation in v["observations"].as_array().unwrap() {
        let pointer = observation["pointer"].as_str().unwrap();
        assert!(pointers.insert(pointer));
        assert_eq!(
            report.pointer(pointer).unwrap(),
            &observation["value"],
            "{pointer}"
        );
    }
    assert!(!pointers.is_empty());
    for original in a["original_sources"].as_array().unwrap() {
        let bytes = fs::read(root().join(original["path"].as_str().unwrap())).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(bytes)), original["sha256"]);
    }
}

fn preserve(prior: &StagedOwnedRelease, next: &StagedOwnedRelease, m: &OwnedReleaseMigrationInput) {
    let old = prior.input();
    let new = next.input();
    let mut schema = old.recipe.schema.clone();
    schema.release = m.release.clone();
    schema.schema_version = m.contract.schema_version;
    schema.semantics_version = m.contract.schema_semantics_version.clone();
    for row in &m.schema {
        match row {
            SchemaExtensionEntry::Definition(row) => {
                assert!(
                    !schema
                        .definitions
                        .iter()
                        .any(|d| d.address() == row.address())
                );
                schema.definitions.push(row.clone());
            }
            SchemaExtensionEntry::Slot(_) => panic!("this packet does not revise input storage"),
        }
    }
    schema
        .definitions
        .sort_by_cached_key(DefinitionDescriptor::address);
    schema.slots.sort_by_cached_key(SlotDescriptor::address);
    assert_eq!(schema, new.recipe.schema);
    let mut rules = old.recipe.rules.clone();
    rules.definitions = next.receipt().definitions.clone();
    rules.operations_version = m.contract.operations_version.clone();
    rules.semantics_version = m.contract.rule_semantics_version.clone();
    for row in &m.owners {
        if let Some(owner) = rules.owners.iter_mut().find(|o| o.owner == row.owner) {
            assert_eq!(owner.programs.closure, row.programs.closure);
            for program in &row.programs.members {
                if let Some(old) = owner.programs.members.iter().find(|p| p.id == program.id) {
                    assert_eq!(old, program);
                } else {
                    owner.programs.members.push(program.clone());
                }
            }
        } else {
            rules.owners.push(row.clone());
        }
    }
    assert_eq!(
        rules, new.recipe.rules,
        "only explicit version and program fragments differ"
    );
    assert_eq!(new.recipe.registry.last_issued.get(), 0x32e2);
    assert_eq!(
        new.recipe.registry.entries.len(),
        old.recipe.registry.entries.len() + 3
    );
    assert_eq!(
        new.recipe.registry.entries[..old.recipe.registry.entries.len()],
        old.recipe.registry.entries
    );
    assert_eq!(new.query_sets, old.query_sets);
    assert!(next.evaluation().is_none());
    migration_preservation::assert_import_rebindings_only(prior, next);
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source_proof();
    let a: Value = read("authoring.json");
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in ["definitions", "registry", "roles", "normalization"] {
        assert_eq!(receipt[field], a[field]);
    }
    let d: Value = read("dependencies.json");
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["definitions"].clone()).unwrap()
    {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    preserve(prior, &migrated, &m);
    let mut input = migrated.input().clone();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key("source-bound-ice-nova-source-inputs"),
        prior_input: migrated.receipt().input,
        authoring_input: digest_owned(
            "owned-ice-nova-source-inputs-v1",
            &(
                a,
                read::<Value>("bindings.json"),
                d,
                read::<Value>("source-vectors.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut restored = next.input().clone();
    restored.provenance.pop();
    assert_eq!(restored, *migrated.input());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
