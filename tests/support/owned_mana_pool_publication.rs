//! Publish the existing arithmetic without granting supplier or request coverage.
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "mana-pool";
const FILES: [&str; 3] = ["authoring.json", "consumer.json", "dependencies.json"];

pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/mana-pool")
}
fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    before: OwnedContentDigest,
    queries: Vec<ContributionQuery>,
    adjustment_owners: Vec<DefinitionRules>,
    query_registry_closure: SchemaClosure,
    receiver_registry_closure: SchemaClosure,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}

pub fn check_authored() {
    let authored: Value = read("authoring.json");
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(authored["kind"], KIND);
    assert_eq!(authored["before"], json!(dependencies.before));
    assert_eq!(
        authored["scope"],
        json!({"new_programs":1,"new_receivers":1,"new_queries":0,
            "closed_existing_owners":0,"supplier_admission":false,
            "complete_mana":false,"evaluation_bundle":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(authored["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(
            authored["artifacts"][file]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    let consumer = consumer();
    assert_eq!((consumer.owners.len(), consumer.receivers.len()), (1, 1));
    let owner = &consumer.owners[0];
    assert!(owner.programs.is_complete());
    assert_eq!(owner.programs.members.len(), 1);
    let program = &owner.programs.members[0];
    assert_eq!(program.context, RuleEntityKind::Actor);
    assert_eq!(consumer.receivers[0].program, program.id);
    assert_eq!(dependencies.queries.len(), 4);
    assert_eq!(dependencies.adjustment_owners.len(), 5);
    // The existing joined native tests exercise these exact bytes, including
    // absent/unknown overrides, unsupported operands and frozen-stage reads.
    assert!(
        program
            .nodes
            .iter()
            .any(|node| matches!(node.expression, RuleExpression::CheckNumericDomain { .. }))
    );
    assert!(
        program
            .effects
            .iter()
            .all(|effect| !matches!(effect.effect, RuleEffectKind::Contribute { .. }))
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let authored: Value = read("authoring.json");
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.before);
    let old = &prior.input().recipe.rules;
    let queries = old.contribution_queries.as_ref().unwrap();
    assert_eq!(queries.closure, dependencies.query_registry_closure);
    assert_eq!(
        old.receivers.closure,
        dependencies.receiver_registry_closure
    );
    for query in &dependencies.queries {
        assert!(queries.members.contains(query));
    }
    for owner in &dependencies.adjustment_owners {
        assert!(old.owners.contains(owner));
    }
    let consumer = consumer();
    assert!(
        !old.owners
            .iter()
            .any(|owner| owner.owner == consumer.owners[0].owner)
    );
    assert!(
        !old.receivers
            .members
            .iter()
            .any(|receiver| receiver.id == consumer.receivers[0].id)
    );
    assert_eq!(
        authored["contract"],
        json!({
            "schema_version":prior.input().recipe.schema.schema_version,
            "schema_semantics_version":prior.input().recipe.schema.semantics_version,
            "operations_version":old.operations_version,
            "rule_semantics_version":old.semantics_version
        })
    );
    // This is an explicitly authored release, not a monotonic successor. The
    // complete receiver inventory changes by the one reviewed receiver; no
    // existing owner or contribution inventory receives new coverage authority.
    let mut input = prior.input().clone();
    input.recipe.rules.release = serde_json::from_value(authored["release"].clone()).unwrap();
    input.recipe.rules.owners.extend(consumer.owners);
    input
        .recipe
        .rules
        .receivers
        .members
        .extend(consumer.receivers);
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let consumer = self::consumer();
    let rules = &mut inverse.recipe.rules;
    let index = rules
        .owners
        .iter()
        .position(|owner| owner.owner == consumer.owners[0].owner)
        .unwrap();
    assert_eq!(rules.owners.remove(index), consumer.owners[0]);
    let index = rules
        .receivers
        .members
        .iter()
        .position(|receiver| receiver.id == consumer.receivers[0].id)
        .unwrap();
    assert_eq!(rules.receivers.members.remove(index), consumer.receivers[0]);
    rules.release = old.release.clone();
    inverse.provenance.pop();
    assert!(
        inverse == *prior.input(),
        "only arithmetic, receiver and provenance may change"
    );
    assert!(next.evaluation().is_none());
    next
}
