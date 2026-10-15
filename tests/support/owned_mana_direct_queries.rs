//! Offline removal of redundant computed collectors; typed supplier channels stay.
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_release::{
    OwnedReleaseInput, OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "mana-direct-queries";
const FILES: [&str; 3] = ["authoring.json", "consumer.json", "dependencies.json"];

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/mana-direct-queries")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    let bytes = fs::read(data().join(name)).unwrap();
    assert!(bytes.len() < 1024 * 1024);
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]) && !bytes.contains(&b'\r'));
    serde_json::from_slice(&bytes).unwrap()
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owner: DefinitionRules,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Channel {
    pub definition: DefinitionDescriptor,
    pub owner: DefinitionRules,
    pub receiver: StatReceiver,
    pub query: ContributionQuery,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub registry_last: i64,
    pub owner: DefinitionRules,
    pub receiver: StatReceiver,
    pub channels: Vec<Channel>,
    pub query_registry_closure: SchemaClosure,
    pub receiver_registry_closure: SchemaClosure,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}
fn provenance() -> OwnedReleaseProvenance {
    OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: dependencies().before,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    }
}

/// A typed-ID census, not string matching of program or game names. The recursive
/// check also refuses a future read variant that starts using one of these IDs.
fn contains(value: &Value, needle: &Value) -> bool {
    value == needle
        || match value {
            Value::Array(rows) => rows.iter().any(|row| contains(row, needle)),
            Value::Object(fields) => fields.values().any(|row| contains(row, needle)),
            _ => false,
        }
}
fn references_channel(value: &Value, d: &Dependencies) -> bool {
    d.channels
        .iter()
        .any(|channel| contains(value, &json!(channel.receiver.stat)))
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let d = dependencies();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(
        a["scope"],
        json!({"removed_programs":5,"removed_receivers":5,"preserved_channels":5,
            "new_definitions":0,"new_queries":0,"closed_existing_owners":0,
            "supplier_admission":false,"complete_mana":false,"evaluation_bundle":false,
            "whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][file]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert_eq!(d.channels.len(), 5);
    assert_eq!(d.owner.programs.members.len(), 1);
    assert!(d.owner.programs.is_complete());
    let mut expected = d.owner.clone();
    let mut stats = std::collections::BTreeSet::new();
    for channel in &d.channels {
        assert!(stats.insert(channel.receiver.stat.clone()));
        assert_eq!(
            channel.definition.address(),
            channel.receiver.stat.address()
        );
        assert_eq!(
            channel.owner.owner,
            SchemaSubject::Definition(channel.receiver.stat.address())
        );
        assert!(channel.owner.programs.is_complete());
        assert_eq!(channel.owner.programs.members.len(), 1);
        let p = &channel.owner.programs.members[0];
        assert_eq!(p.context, RuleEntityKind::Actor);
        assert_eq!((p.reads.len(), p.nodes.len(), p.effects.len()), (1, 1, 1));
        assert_eq!(p.id, channel.receiver.program);
        assert_eq!(channel.receiver.targets, vec![StatReceiverTarget::Player]);
        assert_eq!(channel.query.stat, channel.receiver.stat);
        assert_eq!(channel.query.contribution, ContributionKind::Add);
        assert_eq!(channel.query.groups.len(), 1);
        let group = &channel.query.groups[0];
        assert_eq!(group.reduction, ContributionReduction::Sum);
        assert_eq!(group.ordering, ContributionOrdering::Ordered);
        assert!(group.members.is_complete() && group.members.members.is_empty());
        assert_eq!(
            p.reads[0].source,
            RuleReadSource::ContributionQuery {
                entity: RuleEntity::Current,
                query: channel.query.id.clone(),
                group: group.id.clone(),
            }
        );
        assert_eq!(
            p.nodes[0].expression,
            RuleExpression::Read {
                input: p.reads[0].id.clone()
            }
        );
        assert!(p.effects[0].when.is_none());
        assert_eq!(
            p.effects[0].effect,
            RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: channel.receiver.stat.clone(),
                value: p.nodes[0].id.clone(),
            }
        );
        let reads: Vec<_> = expected.programs.members[0]
            .reads
            .iter_mut()
            .filter(|read| matches!(&read.source, RuleReadSource::Stat { entity: RuleEntity::Current, stat } if *stat == channel.receiver.stat))
            .collect();
        assert_eq!(reads.len(), 1);
        let read = reads.into_iter().next().unwrap();
        assert_eq!(read.value_type, p.reads[0].value_type);
        read.source = p.reads[0].source.clone();
    }
    assert!(
        consumer().owner == expected,
        "only five computed reads may become their exact checked query reads"
    );
    assert_eq!(d.receiver.program, expected.programs.members[0].id);
    assert_eq!(d.receiver.targets, vec![StatReceiverTarget::Player]);
}

/// Pure, independently testable reconciliation against the exact component.
/// A newly discovered reader must be reviewed before its producer is removed.
pub fn rewrite_rules(rules: &RulePackageInput) -> RulePackageInput {
    check_authored();
    let d = dependencies();
    assert_eq!(
        rules
            .owners
            .iter()
            .filter(|o| o.owner == d.owner.owner)
            .count(),
        1
    );
    assert!(rules.owners.contains(&d.owner));
    assert!(rules.receivers.members.contains(&d.receiver));
    for channel in &d.channels {
        assert!(rules.owners.contains(&channel.owner));
        assert!(rules.receivers.members.contains(&channel.receiver));
        assert!(
            rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&channel.query)
        );
    }
    let mut readers = 0;
    for owner in &rules.owners {
        for program in &owner.programs.members {
            for read in &program.reads {
                if references_channel(&json!(read.source), &d) {
                    assert!(
                        owner.owner == d.owner.owner
                            && program.id == d.owner.programs.members[0].id
                            && d.owner.programs.members[0].reads.contains(read),
                        "unreviewed reader still requires a removed computed Mana value"
                    );
                    readers += 1;
                }
            }
        }
    }
    assert_eq!(readers, 5);
    for application in &rules.effect_applications.as_ref().unwrap().members {
        assert!(
            !references_channel(&json!(application.program.reads), &d),
            "effect application requires a removed computed Mana value"
        );
    }
    let mut next = rules.clone();
    *next
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owner.owner)
        .unwrap() = consumer().owner;
    next.owners
        .retain(|o| !d.channels.iter().any(|c| c.owner.owner == o.owner));
    next.receivers
        .members
        .retain(|r| !d.channels.iter().any(|c| c.receiver.id == r.id));
    assert_eq!(next.owners.len() + 5, rules.owners.len());
    assert_eq!(
        next.receivers.members.len() + 5,
        rules.receivers.members.len()
    );
    assert_no_computed_readers(&next);
    next
}

pub fn assert_no_computed_readers(rules: &RulePackageInput) {
    let d = dependencies();
    for program in rules.owners.iter().flat_map(|o| &o.programs.members).chain(
        rules
            .effect_applications
            .iter()
            .flat_map(|a| &a.members)
            .map(|a| &a.program),
    ) {
        assert!(
            !references_channel(&json!(program.reads), &d),
            "remaining program/application requires a removed computed Mana value"
        );
    }
}

fn external_references(input: &OwnedReleaseInput, d: &Dependencies) {
    assert!(
        input.evaluation.is_none(),
        "no staged metric or dependency may be discarded"
    );
    assert!(
        !references_channel(&json!(input.recipe.routing), d),
        "routing still requires a removed computed Mana value"
    );
    let mut remaining = json!(input);
    remaining.as_object_mut().unwrap().remove("recipe");
    remaining.as_object_mut().unwrap().remove("provenance");
    assert!(
        !references_channel(&remaining, d),
        "an import/query/metric contract refers to a removed computed value"
    );
    for c in &d.channels {
        assert!(input.recipe.schema.definitions.contains(&c.definition));
        assert!(
            !input
                .recipe
                .rules
                .support_discovery
                .as_ref()
                .unwrap()
                .providers
                .iter()
                .any(|row| row.owner == c.owner.owner),
            "removed collector still owns a support source declaration"
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d = dependencies();
    let a: Value = read("authoring.json");
    assert_eq!(prior.receipt().input, d.before);
    let old = prior.input();
    assert_eq!(old.recipe.registry.last_issued.get(), d.registry_last);
    external_references(old, &d);
    assert_eq!(
        old.recipe.rules.receivers.closure,
        d.receiver_registry_closure
    );
    assert_eq!(
        old.recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .closure,
        d.query_registry_closure
    );
    assert_eq!(
        a["contract"],
        json!({"schema_version":old.recipe.schema.schema_version,
        "schema_semantics_version":old.recipe.schema.semantics_version,
        "operations_version":old.recipe.rules.operations_version,
        "rule_semantics_version":old.recipe.rules.semantics_version})
    );
    let mut input = old.clone();
    input.recipe.rules = rewrite_rules(&old.recipe.rules);
    input.recipe.rules.release = serde_json::from_value(a["release"].clone()).unwrap();
    input.provenance.push(provenance());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let rules = &mut inverse.recipe.rules;
    *rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owner.owner)
        .unwrap() = d.owner.clone();
    for (position, owner) in old.recipe.rules.owners.iter().enumerate() {
        if d.channels.iter().any(|c| c.owner.owner == owner.owner) {
            rules.owners.insert(position, owner.clone());
        }
    }
    for (position, receiver) in old.recipe.rules.receivers.members.iter().enumerate() {
        if d.channels.iter().any(|c| c.receiver.id == receiver.id) {
            rules.receivers.members.insert(position, receiver.clone());
        }
    }
    rules.release = old.recipe.rules.release.clone();
    assert_eq!(inverse.provenance.pop(), Some(provenance()));
    assert!(
        inverse == *old,
        "rewrite must preserve every other input field exactly"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    assert_component(&next);
    next
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let d = dependencies();
    let input = endpoint.input();
    external_references(input, &d);
    assert_no_computed_readers(&input.recipe.rules);
    assert!(input.recipe.rules.owners.contains(&consumer().owner));
    assert!(input.recipe.rules.receivers.members.contains(&d.receiver));
    for channel in &d.channels {
        assert!(
            !input
                .recipe
                .rules
                .owners
                .iter()
                .any(|o| o.owner == channel.owner.owner)
        );
        assert!(
            !input
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .any(|r| r.stat == channel.receiver.stat)
        );
        assert!(
            input
                .recipe
                .rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&channel.query)
        );
    }
    assert_eq!(
        input.recipe.rules.receivers.closure,
        d.receiver_registry_closure
    );
    assert_eq!(
        input
            .recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .closure,
        d.query_registry_closure
    );
    assert!(input.provenance.contains(&provenance()));
}
