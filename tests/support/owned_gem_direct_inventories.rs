//! Offline proof of physical-Gem declarations, retaining Skill-owned obligations.
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SlotDescriptor};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

#[path = "owned_gem_direct_inventories_evidence.rs"]
mod evidence;
#[path = "owned_gem_skill_inventories.rs"]
mod potential;

const BEFORE: &str = "420b4a071361ba61b95c6cb92a84ecd82ca36c2ba733c2bab8086647caeec3be";
const AUTHORING: &str = "711868d6711940a60c3964d6f83381712659ce3cd6a2cecad7ed057500991527";
const GEMS: [&str; 4] = [
    "def.00000000000006c5",
    "def.0000000000000797",
    "def.00000000000007dd",
    "def.000000000000086b",
];
const SKILLS: [&str; 3] = [
    "def.0000000000000149",
    "def.00000000000001df",
    "def.00000000000002a2",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/gem-direct-inventories")
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
fn rejected(f: impl FnOnce() + std::panic::UnwindSafe) {
    assert!(std::panic::catch_unwind(f).is_err());
}

pub fn checked_migration() -> OwnedReleaseMigrationInput {
    check_artifacts();
    let established = potential::checked_migration();
    let prior = read("dependencies.json");
    check_established(&prior, &established);
    let migration = serde_json::from_value(read("migration.json")).unwrap();
    check_delta(&migration, &prior);
    evidence::check(&prior);
    migration
}
pub fn prior_descriptors() -> Vec<DefinitionDescriptor> {
    let _ = checked_migration();
    serde_json::from_value(read("dependencies.json")["definitions"].clone()).unwrap()
}
fn check_artifacts() {
    let bytes = bounded_read(data().join("authoring.json"), 16 * 1024);
    assert_eq!(bytes.len(), 1814);
    assert_eq!(hash(&bytes), AUTHORING);
    let a: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["before"], BEFORE);
    assert_eq!(a["replaced_definitions"], 4);
    assert_eq!(a["closed_direct_inventories"], 12);
    assert_eq!(a["preserved_nonempty_primary_chains"], 3);
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(a["new_members"], 0);
    assert_eq!(
        a["scope"],
        json!({
            "direct_gem_declarations_only":true,
            "existing_primary_chains_preserved":true,
            "skill_owned_creation_obligations_preserved":true,
            "external_owner_grants_excluded":true,
            "activation_certified":false,
            "descendant_topology_certified":false,
            "null_stat_mappings_classified":false,
            "numerical_coverage_certified":false,
            "scenario_census_complete":false,
            "receiving_ground_topology_certified":false,
            "trigger_coverage_certified":false
        })
    );
    let artifacts = a["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 3);
    for (pin, file) in
        artifacts
            .iter()
            .zip(["dependencies.json", "evidence.json", "migration.json"])
    {
        assert_eq!(pin["file"], file);
        let bytes = bounded_read(data().join(file), 1024 * 1024);
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let prior = read("dependencies.json");
    assert_eq!(prior["source"]["input"], BEFORE);
    assert_eq!(prior["source"]["definitions"], a["definitions"]);
    assert_eq!(prior["prior_artifacts"].as_array().unwrap().len(), 17);
    let schema_pin = exact(&prior["prior_artifacts"], "/file", &json!("schema.json"));
    assert_eq!(
        schema_pin["sha256"],
        prior["source"]["definitions"]["content_sha256"]
    );
    let license = &a["license"];
    assert_eq!(
        license["path"],
        "data/owned/poe2/3887ae68/passive-structural-inventories/sources/LICENSE.md"
    );
    let bytes = bounded_read(root().join(license["path"].as_str().unwrap()), 128 * 1024);
    assert_eq!(license["bytes"], bytes.len());
    assert_eq!(license["sha256"], hash(&bytes));
}
fn check_established(prior: &Value, established: &OwnedReleaseMigrationInput) {
    // Reuse the full authenticated catalog/constructor proof and exact existing
    // relations, rather than reimplementing its identity or missing-ref checks.
    for gem in GEMS {
        let row = exact(&prior["definitions"], "/value/id/key", &json!(gem));
        let matches: Vec<_> = established
            .schema
            .iter()
            .filter_map(|e| {
                let SchemaExtensionEntry::Definition(d) = e else {
                    return None;
                };
                let d = json!(d);
                (d["value"]["id"]["key"] == gem).then_some(d)
            })
            .collect();
        assert_eq!(matches, vec![row.clone()]);
    }
    let prior_bytes = bounded_read(potential::data().join("dependencies.json"), 1024 * 1024);
    let established_prior: Value = serde_json::from_slice(&prior_bytes).unwrap();
    for field in ["skills", "slots", "owners"] {
        assert_eq!(
            prior[field], established_prior[field],
            "retain complete authenticated {field} inventory"
        );
    }
    check_relations(prior);
}
fn check_relations(prior: &Value) {
    let slots: Vec<SlotDescriptor> = serde_json::from_value(prior["slots"].clone()).unwrap();
    assert_eq!(slots.len(), 7);
    let support = exact(&prior["definitions"], "/value/id/key", &json!(GEMS[0]));
    let support_schema = &support["value"]["schema"]["value"];
    assert_eq!(support_schema["roles"], json!(["support_assignment"]));
    assert_eq!(
        support_schema["skills"]["closure"],
        json!({"kind":"complete"})
    );
    assert_eq!(support_schema["skills"]["members"], json!([]));
    for field in ["grants", "actors", "skill_grants"] {
        assert_eq!(support_schema["declarations"][field]["members"], json!([]));
    }
    for (gem, grant, supply, skill) in [
        (GEMS[1], "3266", "3265", SKILLS[0]),
        (GEMS[2], "327f", "327e", SKILLS[1]),
        (GEMS[3], "3222", "3221", SKILLS[2]),
    ] {
        let d = exact(&prior["definitions"], "/value/id/key", &json!(gem));
        let find = |id| {
            exact(
                &prior["slots"],
                "/value/id/slot/key",
                &json!(format!("def.000000000000{id}")),
            )
        };
        let g = find(grant);
        let s = find(supply);
        for slot in [g, s] {
            assert_eq!(
                slot["value"]["id"]["declaration"]["definition"],
                d["value"]["id"]
            );
        }
        assert_eq!(
            g["value"]["schema"]["value"]["target"],
            json!({"kind":"skill","value":s["value"]["id"]})
        );
        let child = exact(&prior["skills"], "/value/id/key", &json!(skill));
        assert_eq!(s["value"]["schema"]["value"]["skill"], child["value"]["id"]);
        let schema = &d["value"]["schema"]["value"];
        assert_eq!(schema["roles"], json!(["skill_use"]));
        assert_eq!(schema["skills"]["closure"], json!({"kind":"complete"}));
        assert_eq!(schema["skills"]["members"], json!([child["value"]["id"]]));
        assert_eq!(
            schema["declarations"]["grants"]["members"],
            json!([g["value"]["id"]])
        );
        assert_eq!(
            schema["declarations"]["skill_grants"]["members"],
            json!([s["value"]["id"]])
        );
        assert_eq!(schema["declarations"]["actors"]["members"], json!([]));
        for field in ["grants", "actors", "skill_grants"] {
            assert_eq!(
                child["value"]["schema"]["value"]["declarations"][field]["closure"]["kind"],
                "partial",
                "no descendant absence claim"
            );
        }
    }
    assert_eq!(prior["skills"].as_array().unwrap().len(), 3);
    assert_eq!(prior["owners"].as_array().unwrap().len(), 5);
    for owner in prior["owners"].as_array().unwrap() {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
}
fn check_delta(m: &OwnedReleaseMigrationInput, prior: &Value) {
    assert_eq!(json!(m.before), BEFORE);
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 7);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v28"
    );
    assert_eq!(
        m.contract.schema_semantics_version.as_str(),
        "owned-mechanics-skill-inputs-v1"
    );
    assert_eq!(
        m.contract.rule_semantics_version.as_str(),
        "owned-mechanics-effective-gem-inputs-v1"
    );
    assert_eq!(m.release.as_str(), "pob-3887ae68-gem-direct-inventories-v1");
    assert_eq!(m.reason.as_str(), "reviewed-gem-direct-declarations");
    assert!(
        m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let before = prior["definitions"].as_array().unwrap();
    assert_eq!(before.len(), 4);
    assert_eq!(m.schema.len(), 4);
    for ((entry, old), id) in m.schema.iter().zip(before).zip(GEMS) {
        let SchemaExtensionEntry::Definition(new) = entry else {
            panic!("existing physical Gem definitions only")
        };
        let mut restored = json!(new);
        assert_eq!(restored["kind"], "gem");
        assert_eq!(restored["value"]["id"]["key"], id);
        for field in ["grants", "actors", "skill_grants"] {
            let current = &mut restored["value"]["schema"]["value"]["declarations"][field];
            let previous = &old["value"]["schema"]["value"]["declarations"][field];
            assert_eq!(previous["closure"]["kind"], "partial");
            assert_eq!(current["closure"], json!({"kind":"complete"}));
            assert_eq!(current["members"], previous["members"]);
            current["closure"] = previous["closure"].clone();
        }
        assert_eq!(restored, *old, "three direct closure flags only");
        let typed: DefinitionDescriptor = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(json!(typed), *old);
    }
    check_relations(prior);
}
pub fn delta_controls() {
    let prior = read("dependencies.json");
    let migration = read("migration.json");
    for index in 0..4 {
        for field in ["grants", "actors", "skill_grants"] {
            let mut bad = migration.clone();
            bad["schema"][index]["value"]["value"]["schema"]["value"]["declarations"][field]["closure"] =
                prior["definitions"][index]["value"]["schema"]["value"]["declarations"][field]["closure"].clone();
            let bad = serde_json::from_value(bad).unwrap();
            rejected(|| check_delta(&bad, &prior));
        }
    }
    for field in ["parameters", "choices", "outputs", "sockets"] {
        let mut bad = migration.clone();
        bad["schema"][0]["value"]["value"]["schema"]["value"]["declarations"][field]["closure"] =
            json!({"kind":"complete"});
        let bad = serde_json::from_value(bad).unwrap();
        rejected(|| check_delta(&bad, &prior));
    }
    let mut bad = migration.clone();
    bad["schema"][1]["value"]["value"]["schema"]["value"]["skills"]["members"] = json!([]);
    let bad = serde_json::from_value(bad).unwrap();
    rejected(|| check_delta(&bad, &prior));
    let mut bad = migration.clone();
    bad["schema"][0]["value"]["value"]["schema"]["value"]["roles"] = json!(["skill_use"]);
    let bad = serde_json::from_value(bad).unwrap();
    rejected(|| check_delta(&bad, &prior));
    let mut bad = migration;
    bad["schema"].as_array_mut().unwrap().push(json!({
        "kind":"definition","value":prior["skills"][2]
    }));
    let bad = serde_json::from_value(bad).unwrap();
    rejected(|| check_delta(&bad, &prior));
}
pub fn ownership_controls() {
    let prior = read("dependencies.json");
    let established = potential::checked_migration();
    for field in ["skills", "slots", "owners"] {
        let mut bad = prior.clone();
        bad[field].as_array_mut().unwrap().pop();
        rejected(|| check_established(&bad, &established));
    }
    for pointer in [
        "/value/id/declaration/definition/key",
        "/value/schema/value/target/value/slot/key",
    ] {
        let mut bad = prior.clone();
        let slot = bad["slots"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["value"]["id"]["slot"]["key"] == "def.0000000000003266")
            .unwrap();
        *slot.pointer_mut(pointer).unwrap() = json!("def.000000000000327f");
        rejected(|| check_relations(&bad));
    }
    let mut bad = prior.clone();
    bad["skills"][2]["value"]["schema"]["value"]["declarations"]["actors"]["closure"] =
        json!({"kind":"complete"});
    rejected(|| check_relations(&bad));
    let mut bad = prior;
    bad["definitions"][0]["value"]["schema"]["value"]["declarations"]["actors"]["members"] =
        json!([{"unreviewed":"child copied to carrier"}]);
    rejected(|| check_relations(&bad));
}
pub fn inherited_proof_controls() {
    assert_eq!(potential::prior_descriptors().len(), 4);
    potential::delta_controls();
    potential::identity_controls();
    potential::reference_controls();
    potential::source_controls();
    potential::alias_controls();
}
pub fn source_ownership_controls() {
    evidence::ownership_controls();
}
pub fn factory_controls() {
    evidence::factory_controls();
}
pub fn descendant_controls() {
    evidence::descendant_controls();
}
