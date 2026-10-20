//! Offline proof for four intrinsic ItemTemplate inventories, not a runtime classifier.
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SlotDescriptor};
use poe_optimizer_import::owned_release_migration::OwnedReleaseMigrationInput;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[path = "owned_item_structural_inventories_evidence.rs"]
mod evidence;

pub type Result<T> = std::result::Result<T, String>;
pub const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];
const TEMPLATES: [&str; 4] = [
    "def.0000000000001d75",
    "def.0000000000001e84",
    "def.0000000000002007",
    "def.000000000000238c",
];
const AUTHORING: &str = "807889cdf6177eca72da6aac49417af9dd20cd4565f2b39072c006a716f5902c";
const BEFORE: &str = "091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd";

pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/item-structural-inventories")
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn ensure(ok: bool, reason: &str) -> Result<()> {
    if ok { Ok(()) } else { Err(reason.to_owned()) }
}
fn rows(value: &Value) -> Result<&[Value]> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| "explicit bounded sequence required".into())
}
fn read_bytes(name: &str, limit: u64) -> Result<Vec<u8>> {
    let path = data().join(name);
    ensure(
        fs::metadata(&path).map_err(|e| e.to_string())?.len() <= limit,
        "retained artifact byte limit",
    )?;
    fs::read(path).map_err(|e| e.to_string())
}
pub fn read(name: &str) -> Value {
    serde_json::from_slice(&read_bytes(name, 2 * 1024 * 1024).unwrap()).unwrap()
}
fn typed<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_value(read(name)).unwrap()
}

/// Restore every changed closure, then compare the entire descriptor.
/// This rejects accidental widening of modifiers, sockets, parameters or outputs.
pub fn check_descriptor(before: &Value, after: &Value) -> Result<()> {
    ensure(
        before["kind"] == "item_template" && after["kind"] == "item_template",
        "exact ItemTemplate owner",
    )?;
    ensure(
        before["value"]["schema"]["kind"] == "known",
        "known predecessor descriptor",
    )?;
    let mut inverse = after.clone();
    for name in CLOSED {
        let old = &before["value"]["schema"]["value"]["declarations"][name];
        ensure(
            old["closure"]["kind"] == "partial" && old["members"] == json!([]),
            "only empty Partial intrinsic inventories may close",
        )?;
        let pointer = format!("/value/schema/value/declarations/{name}");
        let new = inverse
            .pointer_mut(&pointer)
            .ok_or("missing intrinsic inventory")?;
        ensure(
            *new == json!({"members":[],"closure":{"kind":"complete"}}),
            "exact empty Complete intrinsic inventory required",
        )?;
        *new = old.clone();
    }
    ensure(
        inverse == *before,
        "descriptor changed beyond three intrinsic closures",
    )
}

pub fn check_firebolt(dependencies: &Value, proof: &Value) -> Result<()> {
    let relation = &proof["firebolt_owned_relation"];
    let definitions = rows(&dependencies["preserved_definitions"])?;
    let slots = rows(&dependencies["preserved_slots"])?;
    for (name, set, key) in [
        ("modifier", definitions, "def.00000000000031c6"),
        ("skill", definitions, "def.0000000000000134"),
        ("grant", slots, "def.00000000000031c8"),
        ("supply", slots, "def.00000000000031c9"),
    ] {
        let expected = &relation[name];
        let id = if matches!(name, "grant" | "supply") {
            &expected["value"]["id"]["slot"]
        } else {
            &expected["value"]["id"]
        };
        ensure(id["key"] == key, "exact protected Firebolt identity")?;
        ensure(
            set.iter().filter(|row| *row == expected).count() == 1,
            "missing or changed protected Firebolt dependency",
        )?;
    }
    let modifier = &relation["modifier"]["value"];
    let grant = &relation["grant"]["value"];
    let supply = &relation["supply"]["value"];
    let owner = json!({"kind":"modifier","definition":modifier["id"]});
    ensure(
        grant["id"]["declaration"] == owner && supply["id"]["declaration"] == owner,
        "Firebolt must remain owned by its ItemModifier",
    )?;
    let declarations = &modifier["schema"]["value"]["declarations"];
    for (name, member) in [("grants", &grant["id"]), ("skill_grants", &supply["id"])] {
        ensure(
            declarations[name] == json!({"members":[member],"closure":{"kind":"complete"}}),
            "complete existing modifier grant membership must survive",
        )?;
    }
    ensure(
        grant["schema"]["value"]["provider_roles"] == json!(["item_modifier"])
            && grant["schema"]["value"]["target"] == json!({"kind":"skill","value":supply["id"]})
            && supply["schema"]["value"]["skill"] == relation["skill"]["value"]["id"],
        "exact modifier to grant to supply to Skill chain",
    )?;
    ensure(
        declarations["outputs"]["closure"]["kind"] == "partial"
            && supply["schema"]["value"]["outputs"]["closure"]["kind"] == "partial"
            && relation["skill"]["value"]["schema"]["value"]["declarations"]["outputs"]["closure"]
                ["kind"]
                == "partial",
        "Firebolt output coverage remains incomplete",
    )
}

pub fn check_delta(dependencies: &Value, migration: &Value, proof: &Value) -> Result<()> {
    let before = rows(&dependencies["definitions"])?;
    let after = rows(&migration["schema"])?;
    ensure(
        before.len() == 4 && after.len() == 4,
        "exact four template replacement census",
    )?;
    for (index, key) in TEMPLATES.into_iter().enumerate() {
        ensure(
            before[index]["value"]["id"]["key"] == key && after[index]["kind"] == "definition",
            "exact ordered template replacement identity",
        )?;
        check_descriptor(&before[index], &after[index]["value"])?;
    }
    for name in ["tables", "owners", "receivers", "query_targets"] {
        ensure(
            migration[name] == json!([]),
            "no numerical or query mutation",
        )?;
    }
    ensure(
        migration.get("evaluation").is_none(),
        "no evaluation artifacts introduced",
    )?;
    check_firebolt(dependencies, proof)
}

pub fn check_proof(proof: &Value) -> Result<()> {
    evidence::check(proof)
}

pub fn check_authored() {
    let authoring_bytes = read_bytes("authoring.json", 16 * 1024).unwrap();
    assert_eq!(
        hash(&authoring_bytes),
        AUTHORING,
        "independent packet commitment"
    );
    let authoring: Value = serde_json::from_slice(&authoring_bytes).unwrap();
    let assets = authoring["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<Vec<_>>(),
        ["bindings", "dependencies", "migration", "source-vectors"]
    );
    for (name, pin) in assets {
        assert_eq!(
            *pin,
            hash(&read_bytes(&format!("{name}.json"), 2 * 1024 * 1024).unwrap())
        );
    }
    let dependencies = read("dependencies.json");
    let migration = read("migration.json");
    let proof = read("source-vectors.json");
    let bindings = read("bindings.json");
    assert_eq!(authoring["source"], dependencies["source"]);
    assert_eq!(authoring["source"], bindings["source"]);
    assert_eq!(authoring["source"]["input"], BEFORE);
    assert_eq!(migration["before"], BEFORE);
    assert_eq!(migration["schema_version"], 5);
    assert_eq!(migration["contract"]["schema_version"], 7);
    assert_eq!(
        migration["contract"]["operations_version"],
        "owned-domain-operations-v28"
    );
    assert_eq!(migration["release"], bindings["release"]);
    assert_eq!(authoring["allocated_definitions"], 0);
    assert_eq!(authoring["replaced_definitions"], 4);
    assert_eq!(authoring["closed_declaration_inventories"], 12);
    assert_eq!(authoring["closed_existing_rule_owners"], 0);
    assert_eq!(authoring["scope"], proof["scope"]);
    assert_eq!(authoring["scope"], bindings["scope"]);
    assert!(
        authoring["scope"]
            .as_object()
            .unwrap()
            .values()
            .all(|v| *v == false || *v == 0)
    );
    let _: OwnedReleaseMigrationInput = serde_json::from_value(migration.clone()).unwrap();
    let _: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["definitions"].clone()).unwrap();
    let _: Vec<DefinitionDescriptor> =
        serde_json::from_value(dependencies["preserved_definitions"].clone()).unwrap();
    let _: Vec<SlotDescriptor> =
        serde_json::from_value(dependencies["preserved_slots"].clone()).unwrap();
    check_delta(&dependencies, &migration, &proof).unwrap();
    check_proof(&proof).unwrap();
}

/// Existing offline migration API; root publication owns whole-release inverses.
pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_authored();
    typed("migration.json")
}

/// The exact predecessor rows, never a reduced or identity-stripped projection.
pub fn prior_descriptors() -> Vec<DefinitionDescriptor> {
    check_authored();
    serde_json::from_value(read("dependencies.json")["definitions"].clone()).unwrap()
}
