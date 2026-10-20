//! Offline, source-bound potential Skill membership; no gameplay closure.
use poe_optimizer_core::owned_schema::{DefinitionDescriptor, SlotDescriptor};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release_migration::OwnedReleaseMigrationInput,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, io::Read, path::PathBuf};

#[path = "owned_gem_skill_inventories_evidence.rs"]
mod evidence;

const BEFORE: &str = "520832751a5c6650954aa23c70f0b7a7b3ca5cc47c5cc39b8bd62277ee7f41e6";
const AUTHORING: &str = "98cec2bad291263e9c2a79ab774df6c3cf67f54c12a569996bfd8b29f35555e0";
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
    root().join("data/owned/poe2/3887ae68/gem-skill-inventories")
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
    let migration = serde_json::from_value(read("migration.json")).unwrap();
    let prior = read("dependencies.json");
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
    assert_eq!(bytes.len(), 1758);
    assert_eq!(hash(&bytes), AUTHORING);
    let a: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["before"], BEFORE);
    for (field, count) in [
        ("replaced_definitions", 4),
        ("closed_potential_skill_inventories", 4),
        ("closed_direct_inventories", 0),
        ("preserved_nonempty_skill_inventories", 3),
        ("allocated_definitions", 0),
        ("new_members", 0),
        ("ice_nova_declared_stat_set_aliases", 2),
        ("ice_nova_constructed_stat_sets", 2),
    ] {
        assert_eq!(a[field], count);
    }
    assert_eq!(
        a["scope"],
        json!({
            "potential_skill_membership_only":true, "activation_certified":false,
            "direct_declarations_certified":false, "descendant_topology_certified":false,
            "null_stat_mappings_classified":false, "numerical_coverage_certified":false,
            "scenario_census_complete":false
        })
    );
    let artifacts = a["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 3);
    for (pin, name) in
        artifacts
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
    assert_eq!(dep["source"]["definitions"], a["definitions"]);
    assert_eq!(dep["prior_artifacts"].as_array().unwrap().len(), 17);
    assert_eq!(
        dep["prior_release"]["sha256"],
        "5df5141dde6ce6826edc273822d12aa27777844b3305907a96ea0f9a175bc42e"
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
    assert_eq!(m.release.as_str(), "pob-3887ae68-gem-skill-inventories-v1");
    assert_eq!(m.reason.as_str(), "reviewed-gem-potential-skill-membership");
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
    for (index, ((entry, old), id)) in m.schema.iter().zip(before).zip(GEMS).enumerate() {
        let SchemaExtensionEntry::Definition(new) = entry else {
            panic!("existing Gem definitions only")
        };
        let mut restored = json!(new);
        assert_eq!(restored["kind"], "gem");
        assert_eq!(restored["value"]["id"]["key"], id);
        assert_eq!(restored["value"]["id"], old["value"]["id"]);
        let schema = &mut restored["value"]["schema"]["value"];
        assert_eq!(
            schema["roles"],
            json!([if index == 0 {
                "support_assignment"
            } else {
                "skill_use"
            }])
        );
        let old_skills = &old["value"]["schema"]["value"]["skills"];
        assert_eq!(old_skills["closure"]["kind"], "partial");
        assert_eq!(schema["skills"]["closure"], json!({"kind":"complete"}));
        assert_eq!(schema["skills"]["members"], old_skills["members"]);
        if index == 0 {
            assert_eq!(old_skills["members"], json!([]));
        } else {
            assert_eq!(old_skills["members"].as_array().unwrap().len(), 1);
            assert_eq!(old_skills["members"][0]["key"], SKILLS[index - 1]);
        }
        schema["skills"]["closure"] = old_skills["closure"].clone();
        assert_eq!(restored, *old, "only potential-Skill closure changes");
        let typed: DefinitionDescriptor = serde_json::from_value(old.clone()).unwrap();
        assert_eq!(json!(typed), *old);
        for field in ["grants", "actors", "skill_grants"] {
            assert_eq!(
                old["value"]["schema"]["value"]["declarations"][field]["closure"]["kind"],
                "partial"
            );
        }
    }
    let descendants: Vec<DefinitionDescriptor> =
        serde_json::from_value(prior["skills"].clone()).unwrap();
    assert_eq!(descendants.len(), 3);
    for id in SKILLS {
        let skill = exact(&prior["skills"], "/value/id/key", &json!(id));
        for field in ["grants", "actors", "skill_grants"] {
            assert_eq!(
                skill["value"]["schema"]["value"]["declarations"][field]["closure"]["kind"],
                "partial"
            );
        }
    }
    for owner in prior["owners"].as_array().unwrap() {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    check_existing_relations(prior);
}
fn check_existing_relations(prior: &Value) {
    let slots: Vec<SlotDescriptor> = serde_json::from_value(prior["slots"].clone()).unwrap();
    assert_eq!(slots.len(), 7);
    for (gem, grant, supply, skill) in [
        ("0797", "3266", "3265", "0149"),
        ("07dd", "327f", "327e", "01df"),
        ("086b", "3222", "3221", "02a2"),
    ] {
        let id = |short| json!(format!("def.000000000000{short}"));
        let gem = exact(&prior["definitions"], "/value/id/key", &id(gem));
        let grant = exact(&prior["slots"], "/value/id/slot/key", &id(grant));
        let supply = exact(&prior["slots"], "/value/id/slot/key", &id(supply));
        for slot in [grant, supply] {
            assert_eq!(
                slot["value"]["id"]["declaration"]["definition"],
                gem["value"]["id"]
            );
        }
        assert_eq!(
            grant["value"]["schema"]["value"]["target"],
            json!({"kind":"skill","value":supply["value"]["id"]})
        );
        assert_eq!(
            supply["value"]["schema"]["value"]["skill"]["key"],
            id(skill)
        );
        assert_eq!(
            gem["value"]["schema"]["value"]["declarations"]["grants"]["members"],
            json!([grant["value"]["id"]])
        );
        assert_eq!(
            gem["value"]["schema"]["value"]["declarations"]["skill_grants"]["members"],
            json!([supply["value"]["id"]])
        );
    }
    let output = exact(
        &prior["slots"],
        "/value/id/slot/key",
        &json!("def.0000000000003280"),
    );
    assert_eq!(
        output["value"]["id"]["declaration"]["definition"]["key"],
        SKILLS[1]
    );
    let sets = &output["value"]["schema"]["value"]["stat_sets"];
    assert_eq!(sets["closure"]["kind"], "complete");
    let keys: Vec<_> = sets["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["key"].clone())
        .collect();
    assert_eq!(
        json!(keys),
        json!(["def.0000000000003283", "def.0000000000003284"])
    );
}

pub fn delta_controls() {
    let prior = read("dependencies.json");
    let migration = read("migration.json");
    for field in ["grants", "actors", "skill_grants", "outputs", "parameters"] {
        let mut bad = migration.clone();
        bad["schema"][0]["value"]["value"]["schema"]["value"]["declarations"][field]["closure"] =
            json!({"kind":"complete"});
        let bad = serde_json::from_value(bad).unwrap();
        rejected(|| check_delta(&bad, &prior));
    }
    for index in [0, 1, 2, 3] {
        let mut bad = migration.clone();
        bad["schema"][index]["value"]["value"]["schema"]["value"]["skills"]["members"] =
            if index == 0 {
                prior["definitions"][1]["value"]["schema"]["value"]["skills"]["members"].clone()
            } else {
                json!([])
            };
        let bad = serde_json::from_value(bad).unwrap();
        rejected(|| check_delta(&bad, &prior));
    }
    let mut bad = prior.clone();
    let output = bad["slots"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|s| s["value"]["id"]["slot"]["key"] == "def.0000000000003280")
        .unwrap();
    output["value"]["schema"]["value"]["stat_sets"]["members"]
        .as_array_mut()
        .unwrap()
        .pop();
    rejected(|| check_existing_relations(&bad));
}
pub fn identity_controls() {
    evidence::identity_controls();
}
pub fn reference_controls() {
    evidence::reference_controls();
}
pub fn source_controls() {
    evidence::source_controls();
}
pub fn alias_controls() {
    evidence::alias_controls();
}
