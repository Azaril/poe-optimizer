//! Offline data authoring for final Player results, with supplier coverage open.
#[path = "owned_release_migration_preservation.rs"]
mod migration_preservation;

use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "player-critical-receivers";
const FILES: [&str; 5] = [
    "authoring.json",
    "extension.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
];
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/player-critical-receivers")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub queries: Vec<ContributionQuery>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    before: OwnedContentDigest,
    registry_last: i64,
    definitions: Vec<DefinitionDescriptor>,
    shared_actor: DefinitionRules,
    query_registry_closure: SchemaClosure,
    receiver_registry_closure: SchemaClosure,
}
pub fn extension() -> OwnedRecipeExtension {
    read("extension.json")
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}

/// The tracked source packet contains observed operands, never native defaults.
/// Its full reports were authenticated at acquisition; ordinary CI needs only
/// these pinned compact vectors and the pinned original source manifest.
pub fn source_vectors() -> Value {
    let authored: Value = read("authoring.json");
    let pin = &authored["source"];
    assert_eq!(
        pin["path"],
        "data/owned/poe2/3887ae68/incoming-critical-effect/source-vectors.json"
    );
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    let proof: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(proof["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        proof["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(proof["source_revision"], manifest["upstream_revision"]);
    for source in proof["source_files"].as_array().unwrap() {
        assert!(manifest["files"].as_array().unwrap().contains(source));
    }
    assert!(
        proof["source_files"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| { p["path"] == "src/Modules/CalcDefence.lua" })
    );
    for pin in proof["local_pins"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
    assert_eq!(proof["vectors"].as_array().unwrap().len(), 52);
    proof
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let d: Dependencies = read("dependencies.json");
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(
        a["scope"],
        json!({"new_definitions":13,"new_programs":3,"new_receivers":3,"new_queries":4,
            "closed_existing_owners":0,"supplier_admission":false,
            "complete_critical_effect":false,"evaluation_bundle":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][file]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    source_vectors();
    let e = extension();
    assert_eq!(e.schema.len(), 13);
    assert!(e.owners.is_empty() && e.receivers.is_empty() && e.tables.is_empty());
    assert_eq!(d.registry_last, 0x3372);
    assert!(!d.shared_actor.programs.is_complete());
    let c = consumer();
    assert_eq!(
        (c.owners.len(), c.receivers.len(), c.queries.len()),
        (3, 3, 4)
    );
    for (owner, receiver) in c.owners.iter().zip(&c.receivers) {
        assert_eq!(
            owner.owner,
            SchemaSubject::Definition(receiver.stat.address())
        );
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        let p = &owner.programs.members[0];
        assert_eq!(p.id, receiver.program);
        assert_eq!(p.context, RuleEntityKind::Actor);
        assert_eq!(receiver.targets, [StatReceiverTarget::Player]);
        assert_eq!(p.effects.len(), 1);
        assert!(matches!(&p.effects[0].effect, RuleEffectKind::Derive {
            entity: RuleEntity::Current, stat, ..
        } if *stat == receiver.stat));
        assert!(p.effects[0].when.is_some());
    }
    for query in &c.queries {
        assert_eq!(query.groups.len(), 1);
        let g = &query.groups[0];
        assert!(!g.members.is_complete() && g.members.members.is_empty());
        assert_eq!(g.ordering, ContributionOrdering::Unordered);
        if query.contribution == ContributionKind::Flag {
            assert_eq!(g.reduction, ContributionReduction::Any);
        } else {
            assert_eq!(query.contribution, ContributionKind::Override);
            assert_eq!(g.reduction, ContributionReduction::RequireAgreement);
            assert!(g.empty.is_none());
        }
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, d.before);
    let old = &prior.input().recipe;
    assert_eq!(old.registry.last_issued.get(), d.registry_last);
    for definition in &d.definitions {
        assert!(old.schema.definitions.contains(definition));
    }
    assert!(old.rules.owners.contains(&d.shared_actor));
    assert_eq!(
        old.rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    assert_eq!(old.rules.receivers.closure, d.receiver_registry_closure);
    assert_eq!(
        a["contract"],
        json!({"schema_version":old.schema.schema_version,
        "schema_semantics_version":old.schema.semantics_version,
        "operations_version":old.rules.operations_version,"rule_semantics_version":old.rules.semantics_version})
    );
    let e = extension();
    let migrated = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: d.before,
            release: serde_json::from_value(a["release"].clone()).unwrap(),
            reason: KIND.parse().unwrap(),
            contract: serde_json::from_value(a["contract"].clone()).unwrap(),
            schema: e.schema,
            tables: vec![],
            owners: vec![],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    let c = consumer();
    let mut input = migrated.input().clone();
    let rules = &mut input.recipe.rules;
    rules.owners.extend(c.owners);
    rules.receivers.members.extend(c.receivers);
    rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let c = consumer();
    for owner in c.owners {
        let rows = &mut inverse.recipe.rules.owners;
        let at = rows.iter().position(|r| r.owner == owner.owner).unwrap();
        assert!(
            rows.remove(at) == owner,
            "authored owner changed during assembly"
        );
    }
    for receiver in c.receivers {
        let rows = &mut inverse.recipe.rules.receivers.members;
        let at = rows.iter().position(|r| r.id == receiver.id).unwrap();
        assert_eq!(rows.remove(at), receiver);
    }
    for query in c.queries {
        let rows = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = rows.iter().position(|r| r.id == query.id).unwrap();
        assert_eq!(rows.remove(at), query);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only final result receivers and open queries may follow schema extension"
    );
    assert!(next.input().recipe.rules.owners.contains(&d.shared_actor));
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    next
}
