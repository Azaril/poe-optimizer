//! Offline publication: actual Command self-contributions join shared minion damage.
use super::{activation_family as partitioning, migration_preservation};
#[allow(dead_code)]
#[path = "owned_command_damage_evidence.rs"]
mod command_evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "action-minion-damage";
pub const PROGRAM: &str = "owned-minion-action-increase";
pub const FACTOR: &str = "owned-minion-action-increase-factor";
pub const QUERY: &str = "owned-minion-command-action-increase";
const FILES: [&str; 7] = [
    "authoring.json",
    "migration.json",
    "consumer.json",
    "queries.json",
    "dependencies.json",
    "gas-partition.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/action-minion-damage")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub definitions: Vec<DefinitionDescriptor>,
    pub slots: Vec<SlotDescriptor>,
    pub source_programs: Vec<SourceProgram>,
    pub receivers: Vec<StatReceiver>,
    pub query_registry_closure: SchemaClosure,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgram {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
pub fn partition() -> partitioning::ActivationPartition {
    read("gas-partition.json")
}
pub fn vectors() -> Vec<Value> {
    read::<Value>("source-vectors.json")["vectors"]
        .as_array()
        .unwrap()
        .clone()
}
pub fn check_source(full: bool) {
    let source: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/command-damage/source-vectors.json"))
            .unwrap(),
    )
    .unwrap();
    command_evidence::check(&source, full);
    let expected = vectors();
    assert_eq!(expected.len(), 14);
    for (case, vector) in source["report_projection"]["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(&expected)
    {
        assert_eq!(case["name"], vector["case"]);
        assert_eq!(case["xml_sha256"], vector["xml_sha256"]);
        assert_eq!(case["removed"], vector["removed"]);
        let mut calls = Vec::new();
        for recipient in case["state"]["main"]["recipients"].as_array().unwrap() {
            if let Some(rows) = recipient["original_offence_calls"].as_array() {
                for call in rows {
                    calls.push((recipient, call));
                }
            }
        }
        assert_eq!(calls.len(), 1);
        let (recipient, call) = calls[0];
        assert_eq!(recipient["effect"], vector["effect_id"]);
        assert_eq!(call["stat_set_index"], vector["stat_set"]);
        assert_eq!(call["diagnostic"]["sum"], vector["subtotal"]);
        assert_eq!(call["diagnostic"]["query_state_preserved"], true);
        let sum: f64 = call["diagnostic"]["reviewed_records"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|r| r["value"].as_f64().unwrap())
            .sum();
        assert_eq!(sum, vector["command_contribution"].as_f64().unwrap());
        let damage: Vec<_> = call["original_damage_calls"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["inc"].is_number())
            .collect();
        assert!(!damage.is_empty());
        for damage in damage {
            assert_eq!(damage["original_calc_damage"], true);
            assert_eq!(damage["inc"], vector["increase_factor"]);
            assert!(
                vector["damage_types"]
                    .as_array()
                    .unwrap()
                    .contains(&damage["damage_type"])
            );
        }
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let m = migration();
    let deps: Dependencies = read("dependencies.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(deps.before));
    assert_eq!(m.before, deps.before);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V26)
    );
    assert_eq!(
        a["scope"],
        json!({"new_definitions":2,"new_action_programs":4,"new_queries":1,"partitioned_supply_programs":1,"whole_build_parity":false,"complete_damage_domain":false})
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
    for pin in a["source_packets"].as_array().unwrap() {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
    assert_eq!(m.schema.len(), 2);
    assert!(
        m.tables.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
            && m.receivers.is_empty()
    );
    assert_eq!(m.owners.len(), 2);
    assert!(
        m.owners
            .iter()
            .all(|o| o.programs.is_complete() && o.programs.members.is_empty())
    );
    let c = consumer();
    assert_eq!(c.owners.len(), 2);
    assert_eq!(c.owners[0].programs, c.owners[1].programs);
    for owner in &c.owners {
        assert_eq!(owner.programs.members.len(), 2);
        assert!(owner.programs.is_complete());
        let subtotal = &owner.programs.members[0];
        assert_eq!(subtotal.id, key(PROGRAM));
        assert_eq!(subtotal.context, RuleEntityKind::Action);
        assert_eq!(subtotal.reads.len(), 2);
        assert!(
            matches!(&subtotal.reads[0].source, RuleReadSource::Stat {entity: RuleEntity::Actor, stat} if stat.key().as_str()=="def.0000000000003353")
        );
        assert!(
            matches!(&subtotal.reads[1].source, RuleReadSource::ContributionQuery {entity: RuleEntity::Current, query, group} if *query==key(QUERY) && *group==key("sources"))
        );
        assert_eq!(owner.programs.members[1].id, key(FACTOR));
        let literals: Vec<_> = owner
            .programs
            .members
            .iter()
            .flat_map(|p| &p.nodes)
            .filter_map(|n| match &n.expression {
                RuleExpression::Literal { value } => Some(json!(value)),
                _ => None,
            })
            .collect();
        assert_eq!(literals.len(), 1);
        assert_eq!(literals[0]["value"]["value"], 1.);
    }
    let qs = queries();
    assert_eq!(qs.len(), 1);
    let q = &qs[0];
    assert_eq!(q.id, key(QUERY));
    assert_eq!(q.stat.key().as_str(), "def.0000000000003304");
    assert_eq!(q.contribution, ContributionKind::Increase);
    assert_eq!(q.groups.len(), 1);
    let g = &q.groups[0];
    assert!(g.members.is_complete());
    assert_eq!(g.members.members.len(), 2);
    for (member, owner) in g.members.members.iter().zip(c.owners) {
        let p = member.producer.as_program_effect().unwrap();
        assert_eq!(p.owner, owner.owner);
        assert_eq!(p.program, key("conditional-command-damage"));
        assert_eq!(p.effect, key("grant"));
        assert!(
            matches!(&p.origin, ContributionOrigin::Action {authored: false, supplies} if supplies.len()==1)
        );
    }
    partitioning::check_partition(&partition());
    check_source(false);
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = &endpoint.input().recipe;
    assert!(
        RuleOperationsVersion::parse(r.rules.operations_version.as_str())
            .unwrap()
            .supports_action_contribution_queries()
    );
    let deps: Dependencies = read("dependencies.json");
    for d in deps.definitions {
        assert!(r.schema.definitions.contains(&d));
    }
    for d in deps.slots {
        assert!(r.schema.slots.contains(&d));
    }
    for s in migration().schema {
        let SchemaExtensionEntry::Definition(d) = s else {
            panic!()
        };
        assert!(r.schema.definitions.contains(&d));
    }
    for p in deps.source_programs {
        let owner = r.rules.owners.iter().find(|o| o.owner == p.owner).unwrap();
        assert!(owner.programs.members.contains(&p.program));
    }
    for expected in consumer().owners {
        let actual = r
            .rules
            .owners
            .iter()
            .find(|o| o.owner == expected.owner)
            .unwrap();
        assert!(!actual.programs.is_complete());
        for p in expected.programs.members {
            assert!(actual.programs.members.contains(&p));
        }
    }
    for receiver in deps.receivers {
        assert!(r.rules.receivers.members.contains(&receiver));
    }
    let registry = r.rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, deps.query_registry_closure);
    for q in queries() {
        assert!(registry.members.contains(&q));
    }
    let partition = partition();
    let actual = r
        .rules
        .owners
        .iter()
        .find(|o| o.owner == partition.owner)
        .unwrap();
    assert!(!actual.programs.is_complete());
    assert!(
        actual
            .programs
            .members
            .contains(&partition.numerical.program)
    );
    assert!(
        actual
            .programs
            .members
            .contains(&partition.activation.program)
    );
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
    let m = migration();
    assert_eq!(prior.receipt().input, m.before);
    let contract = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = contract.input().clone();
    for append in consumer().owners {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        for p in append.programs.members {
            assert!(!owner.programs.members.iter().any(|old| old.id == p.id));
            owner.programs.members.push(p);
        }
    }
    let partition = partition();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == partition.owner)
        .unwrap();
    let original = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == partition.original.id)
        .unwrap();
    assert_eq!(original, &partition.original);
    *original = partition.numerical.program.clone();
    assert!(
        !owner
            .programs
            .members
            .iter()
            .any(|p| p.id == partition.activation.program.id)
    );
    owner
        .programs
        .members
        .push(partition.activation.program.clone());
    let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
    for q in queries() {
        assert!(!registry.members.iter().any(|old| old.id == q.id));
        registry.members.push(q);
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for append in consumer().owners {
        let owner = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        for p in append.programs.members {
            let at = owner
                .programs
                .members
                .iter()
                .position(|old| old.id == p.id)
                .unwrap();
            assert_eq!(owner.programs.members.remove(at), p);
        }
    }
    let owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == partition.owner)
        .unwrap();
    let numerical = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == partition.numerical.program.id)
        .unwrap();
    assert_eq!(numerical, &partition.numerical.program);
    *numerical = partition.original;
    let at = owner
        .programs
        .members
        .iter()
        .position(|p| p.id == partition.activation.program.id)
        .unwrap();
    assert_eq!(
        owner.programs.members.remove(at),
        partition.activation.program
    );
    for q in queries() {
        let registry = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = registry.iter().position(|old| old.id == q.id).unwrap();
        assert_eq!(registry.remove(at), q);
    }
    inverse.provenance = contract.input().provenance.clone();
    assert!(
        inverse == *contract.input(),
        "only checked Action consumers, membership and exact supply partition may change"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
