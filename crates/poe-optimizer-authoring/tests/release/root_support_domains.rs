//! Intrinsic root/ascendancy proofs checked against the maintained KDL release.
//! These tests read authenticated source bytes; they execute no Lua or historical
//! publication chain. Source authentication requires the pinned PoB submodule.
use super::tracked_release;
use poe_optimizer_core::{owned_definitions::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_mapping::{ExternalSelector, MappingOutcome};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[path = "root_support_domains/ascendancy.rs"]
mod ascendancy;
#[path = "root_support_domains/ascendancy_root.rs"]
mod ascendancy_family;
#[path = "root_support_domains/class_root.rs"]
mod family;
#[path = "root_support_domains/laws.rs"]
mod laws;

const FIELDS: [&str; 7] = [
    "parameters",
    "choices",
    "grants",
    "actors",
    "skill_grants",
    "outputs",
    "sockets",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reviewed owners, independent of the production metadata under test. Empty
/// intrinsic inventories do not close neighboring or externally supplied rules.
pub(super) fn declarations() -> Vec<SupportSourceDomainDeclaration> {
    let namespace = GameVersionNamespace::new("poe2", "owned-mechanics-v1").unwrap();
    [
        AscendancyDefId::parse(namespace.clone(), "def.0000000000000a36")
            .unwrap()
            .address(),
        PassiveNodeDefId::parse(namespace.clone(), "def.0000000000001790")
            .unwrap()
            .address(),
        PassiveNodeDefId::parse(namespace, "def.0000000000001b5d")
            .unwrap()
            .address(),
    ]
    .into_iter()
    .map(|address| SupportSourceDomainDeclaration {
        owner: SchemaSubject::Definition(address),
        domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
    })
    .collect()
}

#[test]
fn maintained_release_preserves_exact_intrinsic_owners_and_topology() {
    let release = tracked_release().release();
    let recipe = &release.input().recipe;
    for (proof, domain) in [
        ascendancy::read::<Value>("closure.json"),
        family::read("closure.json"),
        ascendancy_family::read("closure.json"),
    ]
    .iter()
    .zip(declarations())
    {
        assert_eq!(proof["definitions"].as_array().unwrap().len(), 1);
        assert_eq!(proof["owners"].as_array().unwrap().len(), 1);
        let definition: DefinitionDescriptor =
            serde_json::from_value(proof["definitions"][0].clone()).unwrap();
        let owner: DefinitionRules = serde_json::from_value(proof["owners"][0].clone()).unwrap();
        assert_eq!(
            domain.owner,
            SchemaSubject::Definition(definition.address())
        );
        assert_eq!(owner.owner, domain.owner);
        assert!(owner.programs.is_complete() && owner.programs.members.is_empty());
        let definitions: Vec<_> = recipe
            .schema
            .definitions
            .iter()
            .filter(|row| row.address() == definition.address())
            .collect();
        assert_eq!(definitions, vec![&definition]);
        let owners: Vec<_> = recipe
            .rules
            .owners
            .iter()
            .filter(|row| row.owner == owner.owner)
            .collect();
        assert_eq!(owners, vec![&owner]);
        let domains: Vec<_> = recipe
            .rules
            .support_discovery
            .as_ref()
            .unwrap()
            .providers
            .iter()
            .filter(|row| row.owner == domain.owner)
            .collect();
        assert_eq!(domains, vec![&domain]);
    }

    // Identity and neighbor mappings are independent of intrinsic closure. An
    // omitted or retargeted mapping must not preserve a false source proof.
    for dependencies in [
        ascendancy::read::<Value>("dependencies.json"),
        ascendancy_family::read("dependencies.json"),
    ] {
        for row in dependencies["mapping_rows"].as_array().unwrap() {
            let source: ExternalSelector = serde_json::from_value(row["source"].clone()).unwrap();
            let outcome: MappingOutcome = serde_json::from_value(row["outcome"].clone()).unwrap();
            assert_eq!(release.mapping().lookup(&source), Some(&outcome));
        }
    }

    // These independent owners are still incomplete in the maintained release.
    // Archived laws retain the entire seventeen-owner historical preservation
    // check; independently completed passives are not reverted by that history.
    let dependencies: Value = ascendancy::read("dependencies.json");
    for prior in &dependencies["action_owners"].as_array().unwrap()[..2] {
        let subject: SchemaSubject = serde_json::from_value(prior["owner"].clone()).unwrap();
        let current = recipe
            .rules
            .owners
            .iter()
            .find(|row| row.owner == subject)
            .unwrap();
        assert!(
            !current.programs.is_complete(),
            "Class and shared Actor remain independent"
        );
    }
}
