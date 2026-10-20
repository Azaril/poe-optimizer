//! Offline, finite evidence review. No source parsing enters native evaluation.
use poe_optimizer_core::owned_schema::DefinitionDescriptor;
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

#[path = "owned_support_structural_inventories_evidence.rs"]
mod evidence;

const BEFORE: &str = "091556fee21e868c21ac3c2a61390c3e90c5900ab4576552c90a46a9fcc63ffd";
const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];
const GEMS: [&str; 11] = [
    "def.0000000000000674",
    "def.0000000000000740",
    "def.00000000000007f2",
    "def.000000000000080a",
    "def.000000000000082a",
    "def.000000000000082b",
    "def.0000000000000858",
    "def.00000000000008a4",
    "def.00000000000008bf",
    "def.000000000000090c",
    "def.000000000000092e",
];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/support-structural-inventories")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded_read(path: impl AsRef<std::path::Path>, limit: usize) -> Vec<u8> {
    use std::io::Read;
    let file = fs::File::open(path).unwrap();
    assert!(file.metadata().unwrap().len() <= limit as u64);
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes).unwrap();
    assert!(bytes.len() <= limit);
    bytes
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&bounded_read(data().join(name), 1024 * 1024)).unwrap()
}

/// The joint publisher checks these exact original descriptors in its input.
pub fn prior_descriptors() -> Vec<DefinitionDescriptor> {
    check_artifacts();
    serde_json::from_value(read("dependencies.json")["definitions"].clone()).unwrap()
}

pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_artifacts();
    let dependencies = read("dependencies.json");
    let migration: OwnedReleaseMigrationInput =
        serde_json::from_value(read("migration.json")).unwrap();
    check_delta(&migration, &dependencies["definitions"]);
    evidence::check();
    migration
}

fn check_artifacts() {
    let authoring = read("authoring.json");
    assert_eq!(authoring["before"], BEFORE);
    assert_eq!(authoring["replaced_definitions"], 11);
    assert_eq!(authoring["closed_direct_inventories"], 33);
    assert_eq!(authoring["closed_potential_skill_inventories"], 11);
    assert_eq!(authoring["closed_inventories"], 44);
    assert_eq!(authoring["total_mapping_rows"], 18);
    assert_eq!(authoring["null_mapping_rows_with_semantics"], 4);
    assert_eq!(authoring["allocated_definitions"], 0);
    assert_eq!(authoring["new_members"], 0);
    assert_eq!(
        authoring["scope"],
        json!({
            "complete_build":false, "skills_inventory_changed":true,
            "descendant_coverage_changed":false, "numerical_coverage_changed":false,
            "source_domain_coverage_changed":false, "registry_changed":false,
            "runtime_branch_added":false
        })
    );
    let pins = authoring["artifacts"].as_array().unwrap();
    assert_eq!(pins.len(), 3);
    for (pin, name) in pins
        .iter()
        .zip(["dependencies.json", "evidence.json", "migration.json"])
    {
        assert_eq!(pin["file"], name);
        let bytes = bounded_read(data().join(name), 1024 * 1024);
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let dependencies = read("dependencies.json");
    assert_eq!(dependencies["source"]["input"], BEFORE);
    assert_eq!(
        dependencies["source"]["definitions"],
        authoring["definitions"]
    );
    assert_eq!(
        dependencies["prior_artifacts"].as_array().unwrap().len(),
        17
    );
    assert_eq!(
        dependencies["prior_release"]["sha256"],
        "cbe625eb5c24724657d3688a92ac8222d7ecbd011ba92ed7d0a65cb4cb85813a"
    );
}

fn check_delta(migration: &OwnedReleaseMigrationInput, prior: &Value) {
    assert_eq!(json!(migration.before), BEFORE);
    assert_eq!(migration.schema_version, 5);
    assert_eq!(migration.contract.schema_version, 7);
    assert_eq!(
        migration.contract.operations_version.as_str(),
        "owned-domain-operations-v28"
    );
    assert_eq!(
        migration.contract.schema_semantics_version.as_str(),
        "owned-mechanics-skill-inputs-v1"
    );
    assert_eq!(
        migration.contract.rule_semantics_version.as_str(),
        "owned-mechanics-effective-gem-inputs-v1"
    );
    assert_eq!(
        migration.release.as_str(),
        "pob-3887ae68-support-structural-inventories-v1"
    );
    assert_eq!(
        migration.reason.as_str(),
        "reviewed-support-direct-declarations"
    );
    assert!(
        migration.tables.is_empty()
            && migration.owners.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    let old = prior.as_array().unwrap();
    assert_eq!(old.len(), GEMS.len());
    assert_eq!(migration.schema.len(), GEMS.len());
    for ((entry, old), id) in migration.schema.iter().zip(old).zip(GEMS) {
        let SchemaExtensionEntry::Definition(new) = entry else {
            panic!("only existing Gem descriptors may change")
        };
        let mut restored = json!(new);
        assert_eq!(restored["kind"], "gem");
        assert_eq!(restored["value"]["id"]["key"], id);
        assert_eq!(old["value"]["id"], restored["value"]["id"]);
        assert_eq!(old["value"]["schema"]["kind"], "known");
        let ports = &mut restored["value"]["schema"]["value"]["declarations"];
        for field in CLOSED {
            let before = &old["value"]["schema"]["value"]["declarations"][field];
            assert_eq!(before["members"], json!([]));
            assert_eq!(before["closure"]["kind"], "partial");
            assert_eq!(
                ports[field],
                json!({"members":[],"closure":{"kind":"complete"}})
            );
            ports[field]["closure"] = before["closure"].clone();
        }
        let before = &old["value"]["schema"]["value"]["skills"];
        let skills = &mut restored["value"]["schema"]["value"]["skills"];
        assert_eq!(before["members"], json!([]));
        assert_eq!(before["closure"]["kind"], "partial");
        assert_eq!(*skills, json!({"members":[],"closure":{"kind":"complete"}}));
        skills["closure"] = before["closure"].clone();
        assert_eq!(
            restored, *old,
            "only four reviewed empty inventories change"
        );
        let typed: DefinitionDescriptor = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(json!(typed), *old);
        assert_eq!(
            old["value"]["schema"]["value"]["roles"],
            json!(["support_assignment"])
        );
        assert_eq!(
            old["value"]["schema"]["value"]["skills"]["closure"]["kind"],
            "partial"
        );
    }
}

pub fn check_delta_controls() {
    let prior = read("dependencies.json")["definitions"].clone();
    let migration = read("migration.json");
    for pointer in [
        "/schema/0/value/value/schema/value/declarations/outputs/closure",
        "/schema/0/value/value/schema/value/declarations/parameters/closure",
        "/schema/0/value/value/schema/value/declarations/choices/closure",
    ] {
        let mut changed = migration.clone();
        *changed.pointer_mut(pointer).unwrap() = json!({"kind":"complete"});
        let changed = serde_json::from_value(changed).unwrap();
        assert!(std::panic::catch_unwind(|| check_delta(&changed, &prior)).is_err());
    }
    let mut removed = migration.clone();
    removed["schema"].as_array_mut().unwrap().pop();
    let removed = serde_json::from_value(removed).unwrap();
    assert!(std::panic::catch_unwind(|| check_delta(&removed, &prior)).is_err());
    let mut duplicate = migration;
    duplicate["schema"][1] = duplicate["schema"][0].clone();
    let duplicate = serde_json::from_value(duplicate).unwrap();
    assert!(std::panic::catch_unwind(|| check_delta(&duplicate, &prior)).is_err());
}

pub fn check_identity_controls() {
    evidence::identity_controls();
}
pub fn check_factory_controls() {
    evidence::factory_controls();
}
pub fn check_mapping_controls() {
    evidence::mapping_controls();
}
pub fn check_description_controls() {
    evidence::description_controls();
}
