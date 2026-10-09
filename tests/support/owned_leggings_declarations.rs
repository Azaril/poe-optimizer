//! Checked Leggings declaration refinement using existing offline migration authority.
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

const KIND: &str = "source-bound-leggings-intrinsic-declarations";
const DOMAIN: &str = "owned-leggings-intrinsic-declarations-v1";
const CLOSED: [&str; 5] = ["choices", "grants", "actors", "skill_grants", "outputs"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/leggings-declarations")
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
        ("registry_last_issued_before", 0x336d),
        ("registry_last_issued_after", 0x336d),
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
    assert_eq!(m.release.as_str(), "pob-3887ae68-leggings-declarations-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v27"
    );
    assert!(
        m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 1);
    assert_eq!(b["template"]["key"], "def.0000000000001e0e");
    assert_eq!(b["base_name"], "Cryptic Leggings");
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
    check_descriptor(old, &new);
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
        assert_eq!(id["slot"]["key"], format!("def.{:016x}", 0x3147 + i));
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
    assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 5);
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
    evidence::check_item(&v, false, evidence::ItemWitness::Leggings);
}

fn check_descriptor(old: &Value, new: &Value) {
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
}

pub fn check_descriptor_negatives() {
    let d: Value = read("dependencies.json");
    let m: Value = read("migration.json");
    let old = &d["definitions"][0];
    let new = &m["schema"][0]["value"];
    check_descriptor(old, new);
    for name in CLOSED {
        let mut unknown = new.clone();
        unknown["value"]["schema"]["value"]["declarations"][name]["closure"] =
            old["value"]["schema"]["value"]["declarations"][name]["closure"].clone();
        assert!(std::panic::catch_unwind(|| check_descriptor(old, &unknown)).is_err());
        let mut nonempty = old.clone();
        nonempty["value"]["schema"]["value"]["declarations"][name]["members"] = json!(["unknown"]);
        assert!(std::panic::catch_unwind(|| check_descriptor(&nonempty, new)).is_err());
    }
    for pointer in [
        "/value/schema/value/declarations/sockets/closure",
        "/value/schema/value/socket_destinations/closure",
        "/value/schema/value/modifiers/closure",
        "/value/schema/value/quality/allowed_kinds/closure",
    ] {
        let mut bad = new.clone();
        *bad.pointer_mut(pointer).unwrap() = json!({"kind":"complete"});
        assert!(std::panic::catch_unwind(|| check_descriptor(old, &bad)).is_err());
    }
}

pub fn check_evidence_negatives() {
    let proof: Value = read("source-vectors.json");
    evidence::check_item(&proof, false, evidence::ItemWitness::Leggings);
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
        assert!(
            std::panic::catch_unwind(|| evidence::check_item(
                &bad,
                false,
                evidence::ItemWitness::Leggings
            ))
            .is_err()
        );
    }
    for pointer in [
        "/witnesses/0/observations/0/value/item/exact_registered",
        "/witnesses/1/observations/0/value/original_functions_preserved",
        "/witnesses/2/observations/0/value/hook_removed",
    ] {
        let mut bad = proof.clone();
        *bad.pointer_mut(pointer).unwrap() = json!(false);
        assert!(
            std::panic::catch_unwind(|| evidence::check_item(
                &bad,
                false,
                evidence::ItemWitness::Leggings
            ))
            .is_err()
        );
    }
}

/// One-time packet authoring from the current checked predecessor and unchanged
/// retained reports. JSON passes through Rust's exact float-roundtrip path.
pub fn author() {
    assert!(
        !data().join("authoring.json").exists(),
        "authoring never overwrites a packet"
    );
    let prior = super::release::load(&PathBuf::from(
        std::env::var_os("POE_OPTIMIZER_TEST_LEGGINGS_DECLARATIONS_PRIOR")
            .expect("checked predecessor"),
    ));
    assert_eq!(
        prior.receipt().input.to_string(),
        "bdfb1811f1707b493a2d2277b6ac714cc7a5b91df1a5b4b0ce07518c78948a73"
    );
    let receipt = json!(prior.receipt());
    let mut proof: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/crown-declarations/source-vectors.json"))
            .unwrap(),
    )
    .unwrap();
    // Preserve every historical report pin, metadata record and observation
    // locator. Only the explicitly selected item projection changes.
    proof["source_binding"] = evidence::ItemWitness::Leggings.binding();
    proof["base"] = evidence::ItemWitness::Leggings.base();
    for witness in proof["witnesses"].as_array_mut().unwrap() {
        let bytes = fs::read(root().join(witness["reports"][0]["path"].as_str().unwrap())).unwrap();
        assert_eq!(
            bytes.len() as u64,
            witness["reports"][0]["bytes"].as_u64().unwrap()
        );
        assert_eq!(hash(&bytes), witness["reports"][0]["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let kind = witness["kind"].as_str().unwrap().to_owned();
        for obs in witness["observations"].as_array_mut().unwrap() {
            let state = report.pointer(obs["pointer"].as_str().unwrap()).unwrap();
            obs["value"] = evidence::project(state, &kind, evidence::ItemWitness::Leggings);
        }
    }
    // Validate both whole reports in every pair before authoring any file.
    evidence::check_item(&proof, true, evidence::ItemWitness::Leggings);
    let schema = json!(prior.input().recipe.schema);
    let old = schema["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["value"]["id"]["key"] == "def.0000000000001e0e")
        .unwrap()
        .clone();
    let template = &old["value"]["id"];
    let slots: Vec<_> = schema["slots"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| s["value"]["id"]["declaration"]["definition"] == *template)
        .cloned()
        .collect();
    let rules = json!(prior.input().recipe.rules);
    let owner = rules["owners"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["owner"]["value"]["value"] == *template)
        .unwrap()
        .clone();
    let mut successor = old.clone();
    for name in CLOSED {
        successor["value"]["schema"]["value"]["declarations"][name]["closure"] =
            json!({"kind":"complete"});
    }
    check_descriptor(&old, &successor);
    let release = "pob-3887ae68-leggings-declarations-v1";
    let m: OwnedReleaseMigrationInput = serde_json::from_value(json!({
        "schema_version":5,"before":receipt["input"],"release":release,
        "reason":"leggings-intrinsic-non-socket-declarations",
        "contract":{
            "schema_version":schema["schema_version"],"schema_semantics_version":schema["semantics_version"],
            "operations_version":rules["operations_version"],"rule_semantics_version":rules["semantics_version"]
        },
        "schema":[{"kind":"definition","value":successor}],
        "tables":[],"owners":[],"receivers":[],"query_targets":[]
    })).unwrap();
    let mut source = serde_json::Map::new();
    let mut header = serde_json::Map::new();
    for field in [
        "input",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        source.insert(field.into(), receipt[field].clone());
        header.insert(
            if field == "input" { "before" } else { field }.into(),
            receipt[field].clone(),
        );
    }
    let dependencies = json!({"source":source,"definitions":[old],"slots":slots,"owners":[owner]});
    let mut bindings = Value::Object(header.clone());
    bindings["schema_version"] = json!(1);
    bindings["release"] = json!(release);
    bindings["template"] = template.clone();
    bindings["base_name"] = json!("Cryptic Leggings");
    bindings["closed_empty_intrinsic_declarations"] = json!(CLOSED);
    bindings["unchanged_declarations"] = json!(["parameters", "sockets"]);
    bindings["source_binding"] = proof["source_binding"].clone();
    bindings["scope"] = proof["scope"].clone();
    let mut authoring = Value::Object(header);
    for (name, value) in [
        ("schema_version", 1),
        ("allocated_definitions", 0),
        ("replaced_definitions", 1),
        ("closed_declaration_inventories", 5),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x336d),
        ("registry_last_issued_after", 0x336d),
    ] {
        authoring[name] = json!(value);
    }
    authoring["source_revision"] = proof["source_revision"].clone();
    authoring["source_manifest_sha256"] = proof["source_manifest_sha256"].clone();
    authoring["scope"] = proof["scope"].clone();
    fs::create_dir_all(data()).unwrap();
    let mut hashes = serde_json::Map::new();
    for (name, value) in [
        ("bindings", bindings),
        ("dependencies", dependencies),
        ("migration", json!(m)),
        ("source-vectors", proof),
    ] {
        let mut bytes = serde_json::to_vec_pretty(&value).unwrap();
        bytes.push(b'\n');
        hashes.insert(name.into(), json!(hash(&bytes)));
        fs::write(data().join(format!("{name}.json")), bytes).unwrap();
    }
    authoring["artifact_sha256"] = Value::Object(hashes);
    let mut bytes = serde_json::to_vec_pretty(&authoring).unwrap();
    bytes.push(b'\n');
    fs::write(data().join("authoring.json"), bytes).unwrap();
    check_authored();
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
    evidence::check_item(&v, true, evidence::ItemWitness::Leggings);
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
