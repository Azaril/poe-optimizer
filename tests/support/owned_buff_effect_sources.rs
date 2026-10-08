//! Source-Skill reducers use the existing checked query graph. Empty admitted
//! inventories are a capability restriction, never a claim of global absence.
use super::source_evidence as evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "buff-effect-source-empty-domains";
const FILES: [&str; 5] = [
    "authoring.json",
    "programs.json",
    "queries.json",
    "dependencies.json",
    "migration.json",
];
pub const PROGRAMS: [&str; 3] = [
    "source-empty-buff-effect-increase",
    "source-empty-buff-effect-more",
    "source-empty-magnitude",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/buff-effect-sources")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProgramAddition {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    before: OwnedContentDigest,
    pub owner: DefinitionRules,
    pub definitions: Vec<DefinitionDescriptor>,
    pub query_registry_closure: SchemaClosure,
    pins: Vec<Value>,
}
pub fn programs() -> Vec<ProgramAddition> {
    read("programs.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}

/// Retain all source cases, including nonempty controls that this packet refuses.
/// Compact observations are authenticated against the same full reports at publication.
pub fn check_source(full: bool) {
    let proof = evidence::read();
    evidence::check(&proof, full);
    let mut empty = 0;
    let mut nonempty = 0;
    for vector in proof["vectors"].as_array().unwrap() {
        for invocation in vector["invocations"].as_array().unwrap() {
            assert_eq!(invocation["source_store_is_skill"], true);
            let mut supported = true;
            for (field, name, identity) in [
                ("source_buff_increased", "BuffEffect", 0.),
                ("source_buff_more", "BuffEffect", 1.),
                ("source_magnitude_increased", "Magnitude", 0.),
                ("source_magnitude_more", "Magnitude", 1.),
            ] {
                let channel = &invocation["source_scaling"][field];
                assert_eq!(channel["names"], json!([name]));
                let records = &channel["records"];
                let no_records = records.as_array().is_some_and(Vec::is_empty)
                    || records.as_object().is_some_and(serde_json::Map::is_empty);
                if no_records {
                    assert_eq!(channel["value"].as_f64(), Some(identity));
                } else {
                    assert!(records.as_array().is_some_and(|r| !r.is_empty()));
                    supported = false;
                }
            }
            if supported {
                empty += 1;
            } else {
                nonempty += 1;
            }
            if vector["case"] == "original-05" {
                assert!(supported);
            }
        }
    }
    assert_eq!((empty, nonempty), (17, 6));
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let deps = dependencies();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(deps.before));
    assert_eq!(m.before, deps.before);
    assert_eq!(m.schema_version, 5);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V24)
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(
        a["scope"],
        json!({"new_programs":3,"new_queries":4,"complete_empty_groups":4,
        "new_definitions":0,"owner_closure_changed":false,"global_registry_closure_changed":false,
        "nonempty_source_admission":false,"complete_build_claim":false})
    );
    for name in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest: Value = serde_json::from_slice(
        &fs::read(root.join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    for pin in &deps.pins {
        let path = pin["path"].as_str().unwrap();
        if let Some(source) = path.strip_prefix("vendor/path-of-building-poe2/") {
            let matches: Vec<_> = manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|f| f["path"] == source)
                .collect();
            assert_eq!(matches.len(), 1);
            assert_eq!(matches[0]["bytes"], pin["bytes"]);
            assert_eq!(matches[0]["sha256"], pin["sha256"]);
        } else {
            let bytes = fs::read(root.join(path)).unwrap();
            assert_eq!(pin["bytes"], bytes.len());
            assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(bytes)));
        }
    }
    let proof = evidence::read();
    for field in ["source_revision", "source_manifest_sha256"] {
        assert_eq!(a[field], proof[field]);
    }
    check_source(false);
    assert!(!deps.owner.programs.is_complete());
    assert!(deps.owner.programs.members.is_empty());
    let queries = queries();
    assert_eq!(queries.len(), 4);
    for (i, query) in queries.iter().enumerate() {
        let multiply = i % 2 == 1;
        assert_eq!(
            query.contribution,
            if multiply {
                ContributionKind::Multiply
            } else {
                ContributionKind::Increase
            }
        );
        assert_eq!(query.groups.len(), 1);
        let group = &query.groups[0];
        assert_eq!(group.id, key("empty"));
        assert!(group.members.is_complete() && group.members.members.is_empty());
        assert_eq!(group.ordering, ContributionOrdering::Ordered);
        assert_eq!(
            group.reduction,
            if multiply {
                ContributionReduction::Product
            } else {
                ContributionReduction::Sum
            }
        );
    }
    let programs = programs();
    assert_eq!(programs.len(), 3);
    for (i, row) in programs.iter().enumerate() {
        assert_eq!(row.owner, deps.owner.owner);
        assert_eq!(row.program.id, key(PROGRAMS[i]));
        assert_eq!(row.program.context, RuleEntityKind::Skill);
        assert_eq!(row.program.reads.len(), if i == 2 { 2 } else { 1 });
        let expected = if i == 2 {
            &queries[2..4]
        } else {
            &queries[i..i + 1]
        };
        for (read, query) in row.program.reads.iter().zip(expected) {
            assert_eq!(
                read.source,
                RuleReadSource::ContributionQuery {
                    entity: RuleEntity::Current,
                    query: query.id.clone(),
                    group: key("empty"),
                }
            );
        }
        assert_eq!(row.program.effects.len(), 1);
        assert_eq!(
            row.program.effects[0].effect,
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: queries[i.min(2)].stat.clone(),
                value: key("resolved"),
            }
        );
    }
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let recipe = &endpoint.input().recipe;
    assert!(
        RuleOperationsVersion::parse(recipe.rules.operations_version.as_str())
            .unwrap()
            .supports_skill_contribution_queries()
    );
    let deps = dependencies();
    for definition in deps.definitions {
        assert!(recipe.schema.definitions.contains(&definition));
    }
    let owner = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == deps.owner.owner)
        .unwrap();
    assert_eq!(owner.programs.closure, deps.owner.programs.closure);
    for row in programs() {
        assert_eq!(
            owner
                .programs
                .members
                .iter()
                .filter(|p| **p == row.program)
                .count(),
            1
        );
    }
    let registry = recipe.rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, deps.query_registry_closure);
    for query in queries() {
        assert_eq!(registry.members.iter().filter(|q| **q == query).count(), 1);
    }
    assert_eq!(
        endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND)
                && p.prior_input == deps.before
                && p.authoring_input == digest())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let deps = dependencies();
    assert_eq!(prior.receipt().input, deps.before);
    let contract =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let mut input = contract.input().clone();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == deps.owner.owner)
        .unwrap();
    assert_eq!(*owner, deps.owner);
    owner
        .programs
        .members
        .extend(programs().into_iter().map(|p| p.program));
    let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
    for query in queries() {
        assert!(!registry.members.iter().any(|q| q.id == query.id));
        registry.members.push(query);
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: deps.before,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    let inverse_owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == deps.owner.owner)
        .unwrap();
    for row in programs().into_iter().rev() {
        assert_eq!(inverse_owner.programs.members.pop(), Some(row.program));
    }
    assert_eq!(*inverse_owner, deps.owner);
    for query in queries().into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(query)
        );
    }
    inverse.provenance = contract.input().provenance.clone();
    assert!(
        inverse == *contract.input(),
        "only three programs/four queries follow the contract change"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[test]
fn source_scaling_packet_preserves_empty_and_nonempty_evidence() {
    check_authored();
}
