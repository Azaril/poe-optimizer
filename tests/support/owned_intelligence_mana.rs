//! Data publication for shared Player Intelligence-to-Mana contributions.
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
pub const PROGRAM: &str = "contribute-inherent-intelligence-mana";
const KIND: &str = "inherent-intelligence-mana";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/intelligence-mana")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
#[derive(Deserialize)]
pub struct Consumer {
    pub actor: DefinitionRules,
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub queries: Vec<ContributionQuery>,
}
#[derive(Deserialize)]
struct Dependencies {
    definitions: Vec<DefinitionDescriptor>,
    existing_actor_rules: DeclaredSet<ExistingActorRuleApplication>,
    queries: Vec<ContributionQuery>,
    receivers: Vec<StatReceiver>,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let m = migration();
    let c = consumer();
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(m.schema.len(), 2);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V25
    );
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(c.actor.programs.members.len(), 1);
    assert!(!c.actor.programs.is_complete());
    assert_eq!(c.actor.programs.members[0].id.as_str(), PROGRAM);
    assert_eq!(c.owners.len(), 2);
    assert_eq!(c.receivers.len(), 2);
    assert_eq!(c.queries.len(), 2);
    for q in &c.queries {
        assert_eq!(q.contribution, ContributionKind::Flag);
        assert_eq!(q.groups.len(), 1);
        let g = &q.groups[0];
        assert_eq!(g.reduction, ContributionReduction::Any);
        assert!(g.members.is_complete());
        assert!(g.members.members.is_empty());
    }
    for (name, pin) in a["artifacts"].as_object().unwrap() {
        let b = fs::read(data().join(name)).unwrap();
        assert_eq!(b.len(), pin["bytes"].as_u64().unwrap() as usize);
        assert_eq!(hash(&b), pin["sha256"]);
    }
    assert_eq!(a["closed_existing_owners"], 0);
    assert_eq!(a["final_mana"], false);
    assert_eq!(a["whole_build"], false);
}
fn source() {
    let v: Value = read("source-vectors.json");
    let mut previous = None;
    for pin in v["reports"].as_array().unwrap() {
        let b = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(b.len(), pin["bytes"].as_u64().unwrap() as usize);
        assert_eq!(hash(&b), pin["sha256"]);
        if let Some(old) = &previous {
            assert_eq!(&b, old);
        }
        previous = Some(b.clone());
        let r: Value = serde_json::from_slice(&b).unwrap();
        for k in [
            "source_revision",
            "manifest_sha256",
            "observers",
            "bootstrap_sha256",
            "driver_sha256",
            "files",
            "vectors",
            "final_mana_claim",
            "game_flag_source_closure",
        ] {
            assert_eq!(r[k], v[k]);
        }
        assert_eq!(r["cases"].as_array().unwrap().len(), 17);
        assert_eq!(r["complete_loads"], 20);
        for (i, row) in r["cases"].as_array().unwrap().iter().take(5).enumerate() {
            assert_eq!(
                hash(
                    &fs::read(root().join(format!(
                        "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                        i + 1
                    )))
                    .unwrap()
                ),
                row["xml_sha256"]
            );
        }
    }
    for pin in v["observers"].as_array().unwrap() {
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    for (field, path) in [
        ("bootstrap_sha256", "configuration_preparation_source.rs"),
        ("driver_sha256", "player_resource_source.rs"),
    ] {
        assert_eq!(
            hash(
                &fs::read(
                    root()
                        .join("crates/poe-optimizer-pob/tests/support")
                        .join(path)
                )
                .unwrap()
            ),
            v[field]
        );
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
    source();
    let m = migration();
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, m.before);
    for x in d.definitions {
        assert!(prior.input().recipe.schema.definitions.contains(&x));
    }
    assert_eq!(
        prior.input().recipe.rules.existing_actor_rules,
        Some(d.existing_actor_rules)
    );
    for q in d.queries {
        assert!(
            prior
                .input()
                .recipe
                .rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&q)
        );
    }
    for r in d.receivers {
        assert!(prior.input().recipe.rules.receivers.members.contains(&r));
    }
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let c = consumer();
    let actor = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.actor.owner)
        .unwrap();
    assert_eq!(actor.programs.closure, c.actor.programs.closure);
    assert!(
        !actor
            .programs
            .members
            .iter()
            .any(|p| p.id.as_str() == PROGRAM)
    );
    actor.programs.members.extend(c.actor.programs.members);
    input.recipe.rules.owners.extend(c.owners);
    input.recipe.rules.receivers.members.extend(c.receivers);
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    // Empty membership guards the currently admitted domain, including every
    // potential writer, not just active selected sources. Other game domains
    // retain their existing Partial owners and request obligations.
    let c = consumer();
    for o in &input.recipe.rules.owners {
        for p in &o.programs.members {
            for e in &p.effects {
                if let RuleEffectKind::Contribute { stat, .. } = &e.effect {
                    assert!(
                        !c.queries.iter().any(|q| q.stat == *stat),
                        "new flag producer needs explicit membership"
                    );
                }
            }
        }
    }
    for a in &input
        .recipe
        .rules
        .effect_applications
        .as_ref()
        .unwrap()
        .members
    {
        for e in &a.program.effects {
            if let RuleEffectKind::Contribute { stat, .. } = &e.effect {
                assert!(!c.queries.iter().any(|q| q.stat == *stat));
            }
        }
    }
    let payload: Vec<Value> = [
        "authoring.json",
        "migration.json",
        "consumer.json",
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
    let mut inverse = next.input().clone();
    let c = consumer();
    let actor = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.actor.owner)
        .unwrap();
    assert_eq!(
        actor.programs.members.pop().unwrap(),
        c.actor.programs.members[0]
    );
    for o in c.owners.into_iter().rev() {
        let rows = &mut inverse.recipe.rules.owners;
        let at = rows.iter().position(|x| x.owner == o.owner).unwrap();
        assert_eq!(rows.remove(at), o);
    }
    for r in c.receivers.into_iter().rev() {
        let rows = &mut inverse.recipe.rules.receivers.members;
        let at = rows.iter().position(|x| x.id == r.id).unwrap();
        assert_eq!(rows.remove(at), r);
    }
    for q in c.queries.into_iter().rev() {
        let rows = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = rows.iter().position(|x| x.id == q.id).unwrap();
        assert_eq!(rows.remove(at), q);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only the reviewed programs, receivers and queries change"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
