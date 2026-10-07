//! Checked Crown declaration refinement using existing offline migration authority.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
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
use std::{fs, path::PathBuf};

#[path = "owned_crown_declarations_evidence.rs"]
mod evidence;

const KIND: &str = "source-bound-crown-intrinsic-declarations";
const DOMAIN: &str = "owned-crown-intrinsic-declarations-v1";
const CLOSED: [&str; 5] = ["choices", "grants", "actors", "skill_grants", "outputs"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/crown-declarations")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> poe_optimizer_core::owned_content::OwnedContentDigest {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(a, b, d, v, m), 1024 * 1024).unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, value) in [
        ("allocated_definitions", 0),
        ("replaced_definitions", 1),
        ("closed_declaration_inventories", 5),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3352),
        ("registry_last_issued_after", 0x3352),
    ] {
        assert_eq!(a[field], value);
    }
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-crown-declarations-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v22"
    );
    assert!(
        m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 1);
    assert_eq!(b["template"]["key"], "def.0000000000001f1c");
    assert_eq!(b["base_name"], "Iron Crown");
    assert_eq!(b["closed_empty_intrinsic_declarations"], json!(CLOSED));
    assert_eq!(
        b["unchanged_declarations"],
        json!(["parameters", "sockets"])
    );
    assert_eq!(d["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(d["slots"].as_array().unwrap().len(), 6);
    assert_eq!(d["owners"].as_array().unwrap().len(), 1);
    let old = &d["definitions"][0];
    assert_eq!(old["value"]["id"], b["template"]);
    let SchemaExtensionEntry::Definition(new) = &m.schema[0] else {
        panic!("one exact descriptor replacement")
    };
    let new = json!(new);
    let mut restored = new.clone();
    let ports = restored["value"]["schema"]["value"]["declarations"]
        .as_object_mut()
        .unwrap();
    assert_eq!(ports.len(), 7);
    for name in CLOSED {
        let before = &old["value"]["schema"]["value"]["declarations"][name];
        let after = ports.get_mut(name).unwrap();
        assert_eq!(before["closure"]["kind"], "partial");
        assert_eq!(before["members"], json!([]));
        assert_eq!(*after, json!({"members":[],"closure":{"kind":"complete"}}));
        after["closure"] = before["closure"].clone();
    }
    assert_eq!(
        restored, *old,
        "only five intrinsic declaration closures change"
    );
    let schema = &new["value"]["schema"]["value"];
    assert_eq!(
        schema["declarations"]["parameters"]["closure"]["kind"],
        "complete"
    );
    assert_eq!(
        schema["declarations"]["sockets"]["closure"]["kind"],
        "partial"
    );
    assert_eq!(schema["declarations"]["sockets"]["members"], json!([]));
    for (i, slot) in d["slots"].as_array().unwrap().iter().enumerate() {
        let id = &slot["value"]["id"];
        assert_eq!(id["declaration"]["definition"], b["template"]);
        assert_eq!(id["slot"]["key"], format!("def.{:016x}", 0x3141 + i));
        assert_eq!(schema["declarations"]["parameters"]["members"][i], *id);
    }
    assert_eq!(
        schema["quality"]["allowed_kinds"]["closure"]["kind"],
        "partial"
    );
    assert_eq!(schema["modifiers"]["closure"]["kind"], "partial");
    assert_eq!(schema["modifiers"]["members"].as_array().unwrap().len(), 31);
    let owner = &d["owners"][0];
    assert_eq!(
        owner["owner"],
        json!({"kind":"definition","value":{"kind":"item_template","value":b["template"]}})
    );
    assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 7);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for value in b["scope"].as_object().unwrap().values() {
        assert!(*value == false || *value == 0);
    }
    assert_eq!(a["source_revision"], v["source_revision"]);
    assert_eq!(a["source_manifest_sha256"], v["source_manifest_sha256"]);
    assert_eq!(b["source_binding"], v["source_binding"]);
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, digest) in assets {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*digest, hash(&bytes));
    }
    evidence::check(&v, false);
}

pub fn check_evidence_negatives() {
    let proof: Value = read("source-vectors.json");
    evidence::check(&proof, false);
    for pointer in [
        "/base/socketLimit",
        "/witnesses/2/observations/0/value/environments/0/item/selected_item_id",
        "/witnesses/2/observations/0/value/environments/0/item/grants/fields",
    ] {
        let mut bad = proof.clone();
        *bad.pointer_mut(pointer).unwrap() = if pointer.ends_with("fields") {
            json!({"1":{"skillId":"unknown-granted-skill"}})
        } else {
            json!(0)
        };
        assert!(std::panic::catch_unwind(|| evidence::check(&bad, false)).is_err());
    }
}

fn unchanged_dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            endpoint
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
    for row in serde_json::from_value::<Vec<DefinitionRules>>(d["owners"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    evidence::check(&v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    let old: DefinitionDescriptor = serde_json::from_value(d["definitions"][0].clone()).unwrap();
    assert_eq!(
        prior
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| **x == old)
            .count(),
        1
    );
    unchanged_dependencies(prior, &d);
    assert!(prior.evaluation().is_none());
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let proof = next.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(next.input().recipe.schema.release, m.release);
    let SchemaExtensionEntry::Definition(expected) = &m.schema[0] else {
        unreachable!()
    };
    assert_eq!(
        next.input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| *x == expected)
            .count(),
        1
    );
    unchanged_dependencies(&next, &d);
    let mut inverse = next.input().clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(inverse, *migrated.input());
    let mut restored = next.input().recipe.clone();
    let target = restored
        .schema
        .definitions
        .iter_mut()
        .find(|d| d.address() == old.address())
        .unwrap();
    *target = old;
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "one descriptor only; every other schema, rule, registry and routing body preserved"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
