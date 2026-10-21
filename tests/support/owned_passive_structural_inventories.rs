//! Authenticated, offline refinement of six intrinsic passive owners.
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SlotDescriptor};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

#[path = "owned_passive_structural_inventories_evidence.rs"]
mod evidence;

const BEFORE: &str = "35d446c73be3aa977d55a49174aaf4656e27a38ebaeaab354e86cd7f9f36b5a5";
const AUTHORING: &str = "7879f2a670d186294253c4e3707cb4b976e53a70d71fa307f463435ac4814799";
const FAMILIES: [(u64, &str); 6] = [
    (13289, "def.0000000000000b34"),
    (19318, "def.0000000000000d0f"),
    (32705, "def.00000000000010d4"),
    (35560, "def.00000000000011c2"),
    (45333, "def.00000000000014d8"),
    (62210, "def.00000000000019ec"),
];
const EXCLUDED: [(u64, &str); 6] = [
    (10561, "def.0000000000000a6d"),
    (14686, "def.0000000000000bb3"),
    (23265, "def.0000000000000e15"),
    (26926, "def.0000000000000f2a"),
    (53324, "def.000000000000173f"),
    (53795, "def.0000000000001760"),
];
const CLOSED: [&str; 3] = ["grants", "actors", "skill_grants"];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/passive-structural-inventories")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn bounded_read(path: impl AsRef<std::path::Path>, limit: usize) -> Vec<u8> {
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
fn exact<'a>(rows: &'a Value, pointer: &str, id: &Value) -> &'a Value {
    let matches: Vec<_> = rows
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row.pointer(pointer) == Some(id))
        .collect();
    assert_eq!(matches.len(), 1, "one exact retained identity");
    matches[0]
}

pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_artifacts();
    let migration = serde_json::from_value(read("migration.json")).unwrap();
    check_delta(&migration, &read("dependencies.json"));
    evidence::check();
    migration
}

pub fn prior_descriptors() -> Vec<DefinitionDescriptor> {
    let _ = checked_migration();
    serde_json::from_value(read("dependencies.json")["definitions"].clone()).unwrap()
}

fn check_artifacts() {
    // Seal the reviewed predecessor and finite evidence, not just a mutable
    // chain of manifests. Publication independently checks the actual prior.
    let bytes = bounded_read(data().join("authoring.json"), 16 * 1024);
    assert_eq!(bytes.len(), 1690);
    assert_eq!(hash(&bytes), AUTHORING);
    let authoring: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(authoring["before"], BEFORE);
    for (field, count) in [
        ("candidate_owners", 12),
        ("reviewed_owners", 6),
        ("unresolved_owners", 6),
        ("closed_direct_inventories", 18),
        ("preserved_nonempty_grants", 2),
        ("preserved_nonempty_supplies", 2),
        ("allocated_definitions", 0),
        ("new_members", 0),
        ("reviewed_source_stat_rows", 7),
        ("remainder_source_stat_rows", 4),
    ] {
        assert_eq!(authoring[field], count);
    }
    assert_eq!(
        authoring["scope"],
        json!({
            "complete_build":false, "candidate_closure_complete":false,
            "numerical_coverage_changed":false, "descendant_coverage_changed":false,
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
    let dep = read("dependencies.json");
    assert_eq!(dep["source"]["input"], BEFORE);
    assert_eq!(dep["source"]["definitions"], authoring["definitions"]);
    assert_eq!(dep["prior_artifacts"].as_array().unwrap().len(), 17);
    assert_eq!(
        dep["prior_release"]["sha256"],
        "2806e12ab87ea85127e58dd97e3f355c82ad15ee384a8142736b782841032c1d"
    );
    let license = &authoring["license"];
    assert_eq!(license["file"], "sources/LICENSE.md");
    let bytes = bounded_read(data().join("sources/LICENSE.md"), 128 * 1024);
    assert_eq!(license["bytes"], bytes.len());
    assert_eq!(license["sha256"], hash(&bytes));
    assert!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .contains("Copyright (c) 2016 David Gowor")
    );
}

fn check_delta(migration: &OwnedReleaseMigrationInput, dep: &Value) {
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
        "pob-3887ae68-passive-structural-inventories-v1"
    );
    assert_eq!(
        migration.reason.as_str(),
        "reviewed-passive-direct-declarations"
    );
    assert!(
        migration.tables.is_empty()
            && migration.owners.is_empty()
            && migration.receivers.is_empty()
            && migration.query_targets.is_empty()
            && migration.evaluation.is_none()
    );
    let prior = dep["definitions"].as_array().unwrap();
    assert_eq!(prior.len(), FAMILIES.len());
    assert_eq!(migration.schema.len(), FAMILIES.len());
    for ((entry, old), (_, id)) in migration.schema.iter().zip(prior).zip(FAMILIES) {
        let SchemaExtensionEntry::Definition(new) = entry else {
            panic!("existing passive definitions only")
        };
        let mut restored = json!(new);
        assert_eq!(restored["kind"], "passive_node");
        assert_eq!(restored["value"]["id"]["key"], id);
        assert_eq!(restored["value"]["id"], old["value"]["id"]);
        assert_eq!(old["value"]["schema"]["kind"], "known");
        for field in CLOSED {
            let before = &old["value"]["schema"]["value"]["declarations"][field];
            let after = &mut restored["value"]["schema"]["value"]["declarations"][field];
            assert_eq!(before["closure"]["kind"], "partial");
            assert_eq!(after["closure"], json!({"kind":"complete"}));
            assert_eq!(
                after["members"], before["members"],
                "positive chains cannot be deleted"
            );
            after["closure"] = before["closure"].clone();
        }
        assert_eq!(restored, *old, "exact three-closure inverse");
        let typed: DefinitionDescriptor = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(json!(typed), *old);
    }
    assert_eq!(dep["owners"].as_array().unwrap().len(), FAMILIES.len());
    for (_, id) in FAMILIES {
        let owner = exact(&dep["owners"], "/owner/value/value/key", &json!(id));
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    assert_eq!(
        dep["unresolved_definitions"].as_array().unwrap().len(),
        EXCLUDED.len()
    );
    for (_, id) in EXCLUDED {
        let old = exact(&dep["unresolved_definitions"], "/value/id/key", &json!(id));
        for field in CLOSED {
            assert_eq!(
                old["value"]["schema"]["value"]["declarations"][field]["closure"]["kind"],
                "partial"
            );
        }
    }
    check_chains(dep);
}

fn check_chains(dep: &Value) {
    let typed: Vec<SlotDescriptor> = serde_json::from_value(dep["slots"].clone()).unwrap();
    assert_eq!(typed.len(), 4);
    assert_eq!(json!(typed), dep["slots"]);
    assert_eq!(dep["skills"].as_array().unwrap().len(), 2);
    for (passive, grant, supply, skill) in [
        ("0b34", "32d3", "32d2", "0322"),
        ("10d4", "32d5", "32d4", "032c"),
    ] {
        let id = |short| json!(format!("def.000000000000{short}"));
        let owner = exact(&dep["definitions"], "/value/id/key", &id(passive));
        let grant = exact(&dep["slots"], "/value/id/slot/key", &id(grant));
        let supply = exact(&dep["slots"], "/value/id/slot/key", &id(supply));
        let recipient = exact(&dep["skills"], "/value/id/key", &id(skill));
        assert_eq!(recipient["kind"], "skill");
        assert_eq!(grant["kind"], "grant");
        assert_eq!(supply["kind"], "skill_grant");
        for slot in [grant, supply] {
            assert_eq!(
                slot["value"]["id"]["declaration"]["definition"],
                owner["value"]["id"]
            );
            assert_eq!(slot["value"]["schema"]["kind"], "known");
        }
        assert_eq!(
            grant["value"]["schema"]["value"]["provider_roles"],
            json!(["allocation"])
        );
        assert_eq!(
            grant["value"]["schema"]["value"]["target"],
            json!({
                "kind":"skill", "value":supply["value"]["id"]
            })
        );
        assert_eq!(
            supply["value"]["schema"]["value"]["skill"],
            recipient["value"]["id"]
        );
        assert_eq!(
            supply["value"]["schema"]["value"]["outputs"]["closure"]["kind"],
            "partial"
        );
        let ports = &owner["value"]["schema"]["value"]["declarations"];
        assert_eq!(ports["grants"]["members"], json!([grant["value"]["id"]]));
        assert_eq!(
            ports["skill_grants"]["members"],
            json!([supply["value"]["id"]])
        );
        assert_eq!(ports["actors"]["members"], json!([]));
    }
    for (_, id) in FAMILIES {
        if !id.ends_with("0b34") && !id.ends_with("10d4") {
            let owner = exact(&dep["definitions"], "/value/id/key", &json!(id));
            for field in CLOSED {
                assert_eq!(
                    owner["value"]["schema"]["value"]["declarations"][field]["members"],
                    json!([])
                );
            }
        }
    }
}

fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}
pub fn delta_controls() {
    let migration = read("migration.json");
    let dep = read("dependencies.json");
    for pointer in [
        "/schema/0/value/value/schema/value/declarations/parameters/closure",
        "/schema/0/value/value/schema/value/declarations/outputs/closure",
        "/schema/0/value/value/schema/value/declarations/sockets/closure",
    ] {
        let mut bad = migration.clone();
        *bad.pointer_mut(pointer).unwrap() = json!({"kind":"complete"});
        let bad = serde_json::from_value(bad).unwrap();
        rejected(|| check_delta(&bad, &dep));
    }
    let mut removed = migration.clone();
    removed["schema"][0]["value"]["value"]["schema"]["value"]["declarations"]["grants"]["members"] =
        json!([]);
    let removed = serde_json::from_value(removed).unwrap();
    rejected(|| check_delta(&removed, &dep));
    let mut bad = dep.clone();
    bad["slots"][0]["value"]["schema"]["value"]["skill"] = bad["skills"][1]["value"]["id"].clone();
    rejected(|| check_chains(&bad));
    let mut bad = dep.clone();
    bad["slots"][1]["value"]["schema"]["value"]["target"]["value"] =
        bad["slots"][2]["value"]["id"].clone();
    rejected(|| check_chains(&bad));
}
pub fn identity_controls() {
    evidence::identity_controls();
}
pub fn semantic_controls() {
    evidence::semantic_controls();
}
pub fn structural_controls() {
    evidence::structural_controls();
}
pub fn census_controls() {
    evidence::census_controls();
}
