//! A source-bounded physical component, never a complete metric or build.
use poe_optimizer_core::{owned_rules::*, owned_schema::*};
use serde::de::DeserializeOwned;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const PROGRAM: &str = "ordinary-physical-hit-dps";
pub const OUTPUTS: [u64; 3] = [0x33b8, 0x33b9, 0x33ba];
pub const UNIT: u64 = 0x33bb;
const AUTHORING: &str = "368e0d0cfd426d0ff3a09a9c71f5aa9427d1092091c691987cd17c81011b7f19";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/physical-hit-dps")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&bounded(&data().join(name), 2 * 1024 * 1024)).unwrap()
}
pub fn bounded(path: &std::path::Path, limit: u64) -> Vec<u8> {
    assert!(fs::metadata(path).unwrap().len() <= limit);
    fs::read(path).unwrap()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn authenticate(pin: &Value, limit: u64) -> Vec<u8> {
    let bytes = bounded(&root().join(pin["path"].as_str().unwrap()), limit);
    assert_eq!(bytes.len(), pin["bytes"]);
    assert_eq!(hash(&bytes), pin["sha256"]);
    bytes
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub queries: Vec<ContributionQuery>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn definitions() -> Vec<DefinitionDescriptor> {
    read("definitions.json")
}
pub fn check() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        let bytes = bounded(&data().join("authoring.json"), 16 * 1024);
        assert_eq!(hash(&bytes), AUTHORING, "independent authoring seal");
        let authoring: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(authoring["artifacts"].as_array().unwrap().len(), 8);
        for pin in authoring["artifacts"].as_array().unwrap() {
            authenticate(pin, 2 * 1024 * 1024);
        }
        assert_eq!(authoring["scope"]["complete_request"], false);
        assert_eq!(authoring["scope"]["final_metric_binding"], false);
        assert_eq!(authoring["scope"]["candidate_source_closure"], false);
        let c = consumer();
        assert_eq!(c.owners.len(), 1);
        assert!(c.queries.is_empty());
        assert!(!c.owners[0].programs.is_complete());
        assert_eq!(c.owners[0].programs.members.len(), 1);
        let p = &c.owners[0].programs.members[0];
        assert_eq!(p.id.as_str(), PROGRAM);
        assert_eq!(p.context, RuleEntityKind::Action);
        assert_eq!(p.reads.len(), 5);
        assert_eq!(p.effects.len(), 3);
        let dependencies: Value = read("dependencies.json");
        assert_eq!(dependencies["before"], authoring["before"]);
        assert_eq!(dependencies["inspection_endpoint"], authoring["before"]);
        assert_eq!(dependencies["registry_last"], 13239);
        let old: Vec<DefinitionRules> =
            serde_json::from_value(dependencies["owners_before"].clone()).unwrap();
        assert_eq!(old.len(), 1);
        assert_eq!(old[0].owner, c.owners[0].owner);
        assert_eq!(old[0].programs.closure, c.owners[0].programs.closure);
        assert!(old[0].programs.members.iter().all(|v| v.id != p.id));
        let defs = definitions();
        assert_eq!(defs.len(), 4);
        for (d, n) in defs.iter().zip([0x33b8, 0x33b9, 0x33ba, UNIT]) {
            assert!(
                serde_json::to_string(d)
                    .unwrap()
                    .contains(&format!("def.{n:016x}"))
            );
        }
        let migration: Value = read("migration.json");
        assert_eq!(migration["before"], dependencies["before"]);
        assert_eq!(migration["schema"].as_array().unwrap().len(), 4);
        for (row, d) in migration["schema"].as_array().unwrap().iter().zip(defs) {
            assert_eq!(row["kind"], "definition");
            assert_eq!(
                serde_json::from_value::<DefinitionDescriptor>(row["value"].clone()).unwrap(),
                d
            );
        }
        for name in ["owners", "tables", "receivers", "query_targets"] {
            assert!(migration[name].as_array().unwrap().is_empty());
        }
        assert!(migration["evaluation"].is_null());
    });
}
