//! One shared Player contribution using the existing Mana channel and unit.
use super::migration_preservation;
use poe_optimizer_core::{owned_content::digest_owned, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub const PROGRAM: &str = "intrinsic-player-mana";
const KIND: &str = "shared-player-intrinsic-mana";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-intrinsic-mana")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
#[derive(Deserialize)]
pub struct Dependencies {
    pub definitions: Vec<DefinitionDescriptor>,
    pub existing_actor_rules: DeclaredSet<ExistingActorRuleApplication>,
    pub classes: Vec<Value>,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let m = migration();
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["new_programs"], 1);
    assert_eq!(a["closed_owners"], 0);
    assert_eq!(a["final_pool"], false);
    assert_eq!(a["whole_build"], false);
    for (name, pin) in a["artifacts"].as_object().unwrap() {
        let b = fs::read(data().join(name)).unwrap();
        assert_eq!(b.len(), pin["bytes"].as_u64().unwrap() as usize);
        assert_eq!(hash(&b), pin["sha256"]);
    }
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V25
    );
    assert_eq!(m.owners.len(), 1);
    let owner = &m.owners[0];
    assert!(!owner.programs.is_complete());
    assert_eq!(owner.programs.members.len(), 1);
    let p = &owner.programs.members[0];
    assert_eq!(p.id.as_str(), PROGRAM);
    assert_eq!(p.context, RuleEntityKind::Actor);
    assert_eq!(p.reads.len(), 1);
    assert_eq!(p.reads[0].source, RuleReadSource::CharacterLevel);
    assert_eq!(p.effects.len(), 1);
    assert!(
        matches!(&p.effects[0].effect,RuleEffectKind::Contribute{entity:RuleEntity::Current,stat,contribution:ContributionKind::Add,..} if stat.key().as_str()=="def.00000000000029f9")
    );
    let deps: Dependencies = read("dependencies.json");
    assert_eq!(deps.classes.len(), 8);
    assert_eq!(deps.existing_actor_rules.members.len(), 1);
    assert_eq!(
        deps.existing_actor_rules.members[0].targets,
        vec![ExistingActorRuleTarget::Player]
    );
    let v: Value = read("source-vectors.json");
    assert_eq!(v["vectors"].as_array().unwrap().len(), 10);
    assert_eq!(v["final_pool"], false);
    assert_eq!(v["complete_contributors"], false);
}
fn verify_source() {
    let v: Value = read("source-vectors.json");
    for (field, path) in [
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/player_intrinsic_mana_source.lua",
        ),
        (
            "bootstrap_sha256",
            "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs",
        ),
    ] {
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), v[field]);
    }
    let mut prior = None;
    for pin in v["reports"].as_array().unwrap() {
        let b = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(b.len(), pin["bytes"].as_u64().unwrap() as usize);
        assert_eq!(hash(&b), pin["sha256"]);
        if let Some(old) = &prior {
            assert_eq!(&b, old);
        }
        prior = Some(b.clone());
        let report: Value = serde_json::from_slice(&b).unwrap();
        for k in [
            "source_revision",
            "manifest_sha256",
            "observer_sha256",
            "bootstrap_sha256",
            "files",
        ] {
            assert_eq!(report[k], v[k]);
        }
        assert_eq!(report["complete_loads"], 13);
        let cases = report["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 10);
        for (i, (case, vector)) in cases
            .iter()
            .zip(v["vectors"].as_array().unwrap())
            .enumerate()
        {
            for k in ["name", "xml_sha256", "warm_xml_sha256"] {
                assert_eq!(case[k], vector[k]);
            }
            if i < 5 {
                assert_eq!(
                    hash(
                        &fs::read(root().join(format!(
                            "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                            i + 1
                        )))
                        .unwrap()
                    ),
                    vector["xml_sha256"]
                );
            }
            for mode in ["MAIN", "CALCS"] {
                let actual = &case["observed"]["state"]["modes"][mode];
                for k in [
                    "class_id",
                    "class_name",
                    "level",
                    "record",
                    "amount",
                    "base_source_sum",
                    "multiplier",
                ] {
                    assert_eq!(actual[k], vector[k]);
                }
            }
        }
    }
    for pin in v["files"].as_array().unwrap() {
        assert_eq!(
            hash(
                fs::read_to_string(
                    root()
                        .join("vendor/path-of-building-poe2")
                        .join(pin["path"].as_str().unwrap())
                )
                .unwrap()
                .replace("\r\n", "\n")
                .as_bytes()
            ),
            pin["sha256"]
        );
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    verify_source();
    let m = migration();
    let deps: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, m.before);
    for d in deps.definitions {
        assert!(prior.input().recipe.schema.definitions.contains(&d));
    }
    assert_eq!(
        prior.input().recipe.rules.existing_actor_rules,
        Some(deps.existing_actor_rules)
    );
    let owner = &m.owners[0];
    let old = prior
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == owner.owner)
        .unwrap();
    assert_eq!(old.programs.closure, owner.programs.closure);
    assert!(
        !old.programs
            .members
            .iter()
            .any(|p| p.id.as_str() == PROGRAM)
    );
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let payload: Vec<Value> = [
        "authoring.json",
        "migration.json",
        "dependencies.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &payload, 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut owners = next.input().recipe.rules.owners.clone();
    let changed = owners.iter_mut().find(|o| o.owner == owner.owner).unwrap();
    assert_eq!(
        changed.programs.members.pop().unwrap(),
        owner.programs.members[0]
    );
    assert_eq!(
        owners,
        prior.input().recipe.rules.owners,
        "only one program appended; every prior body and gap survives"
    );
    assert_eq!(
        next.input().recipe.schema.definitions,
        prior.input().recipe.schema.definitions
    );
    assert_eq!(
        next.input().recipe.rules.receivers,
        prior.input().recipe.rules.receivers
    );
    assert_eq!(
        next.input().recipe.rules.contribution_queries,
        prior.input().recipe.rules.contribution_queries
    );
    assert_eq!(
        next.input().recipe.rules.existing_actor_rules,
        prior.input().recipe.rules.existing_actor_rules
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
