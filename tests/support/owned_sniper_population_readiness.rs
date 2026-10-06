//! Offline, reversible partition of one existing owned rule program.
//! This is authoring verification, not a runtime migration or a closure claim.
use poe_optimizer_core::{
    owned_content::digest_owned,
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{DefinitionRules, RuleEffectKind, RuleExpression, RuleProgram},
    owned_schema::SchemaSubject,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseInput, OwnedReleaseProvenance, OwnedReleaseReceipt, StagedOwnedRelease,
    assemble_owned_release,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fmt::Debug, fs, path::PathBuf};

const KIND: &str = "sniper-population-readiness-v1";
pub fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/sniper-population-readiness")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn key(v: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(v).unwrap()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowIndices {
    pub reads: Vec<usize>,
    pub nodes: Vec<usize>,
    pub effects: Vec<usize>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fragment {
    pub program: RuleProgram,
    pub indices: RowIndices,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    pub schema_version: u32,
    pub owner: SchemaSubject,
    pub original: RuleProgram,
    pub facts: Fragment,
    pub requirements: Fragment,
    pub unreferenced_original_rows: RowIndices,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    schema_version: u32,
    prior_receipt: OwnedReleaseReceipt,
    prior_rules_release: OwnedDefinitionKey,
    owner: DefinitionRules,
}

fn subsequence<T: PartialEq + Debug>(old: &[T], new: &[T], indices: &[usize]) {
    assert_eq!(new.len(), indices.len());
    assert!(
        indices.windows(2).all(|w| w[0] < w[1]),
        "ordered unique indices"
    );
    for (value, index) in new.iter().zip(indices) {
        assert_eq!(
            Some(value),
            old.get(*index),
            "unchanged original row {index}"
        );
    }
}
fn reconstructed<T: Clone + PartialEq + Debug>(
    old: &[T],
    facts: &[T],
    fact_indices: &[usize],
    requirements: &[T],
    requirement_indices: &[usize],
    allow_shared: bool,
) {
    let mut rows = vec![None; old.len()];
    for (values, indices) in [(facts, fact_indices), (requirements, requirement_indices)] {
        subsequence(old, values, indices);
        for (value, index) in values.iter().zip(indices) {
            if let Some(previous) = &rows[*index] {
                assert!(allow_shared, "effect appears in both phases");
                assert_eq!(previous, value, "shared dependency must be identical");
            } else {
                rows[*index] = Some(value.clone());
            }
        }
    }
    assert_eq!(rows.into_iter().collect::<Option<Vec<_>>>().unwrap(), old);
}

/// This finite dependency walker covers only expressions in the authenticated
/// predecessor. Unknown operations fail instead of inventing dependency rules.
fn exact_reachable(program: &RuleProgram) {
    let mut pending = Vec::new();
    for effect in &program.effects {
        pending.extend(effect.when.iter().cloned());
        pending.push(match &effect.effect {
            RuleEffectKind::ProjectActorStat { value, .. } => value.clone(),
            RuleEffectKind::ActivateGrant { enabled, .. } => enabled.clone(),
            RuleEffectKind::Requirement { satisfied, .. } => satisfied.clone(),
            _ => panic!("unexpected partition effect"),
        });
    }
    let mut nodes = BTreeSet::new();
    let mut reads = BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !nodes.insert(id.clone()) {
            continue;
        }
        let node = program
            .nodes
            .iter()
            .find(|n| n.id == id)
            .expect("required node");
        match &node.expression {
            RuleExpression::Literal { .. } => {}
            RuleExpression::Read { input } => {
                reads.insert(input.clone());
            }
            RuleExpression::LookupIntegerTable { key, .. } => pending.push(key.clone()),
            RuleExpression::PercentAsFactor { percent, .. } => pending.push(percent.clone()),
            RuleExpression::Add { left, right } | RuleExpression::Compare { left, right, .. } => {
                pending.extend([left.clone(), right.clone()]);
            }
            RuleExpression::Scale { value, factor } => {
                pending.extend([value.clone(), factor.clone()]);
            }
            RuleExpression::Round { value, .. } => pending.push(value.clone()),
            _ => panic!("unexpected predecessor expression"),
        }
    }
    let declared_nodes: BTreeSet<_> = program.nodes.iter().map(|n| n.id.clone()).collect();
    let declared_reads: BTreeSet<_> = program.reads.iter().map(|r| r.id.clone()).collect();
    assert_eq!(declared_nodes.len(), program.nodes.len(), "unique node IDs");
    assert_eq!(declared_reads.len(), program.reads.len(), "unique read IDs");
    assert_eq!(
        nodes, declared_nodes,
        "no omitted or unused node dependency"
    );
    assert_eq!(
        reads, declared_reads,
        "no omitted or unused read dependency"
    );
}

pub fn check_partition(p: &Partition) {
    assert_eq!(p.schema_version, 1);
    assert_eq!(p.original.id.as_str(), "ordinary-population-inputs");
    assert_eq!(p.facts.program.id, p.original.id);
    assert_eq!(
        p.requirements.program.id.as_str(),
        "ordinary-population-requirements"
    );
    assert_eq!(p.facts.program.context, p.original.context);
    assert_eq!(p.requirements.program.context, p.original.context);
    assert!(p.unreferenced_original_rows.reads.is_empty());
    assert!(p.unreferenced_original_rows.nodes.is_empty());
    assert!(p.unreferenced_original_rows.effects.is_empty());
    assert_eq!(p.facts.program.effects.len(), 3);
    assert_eq!(p.requirements.program.effects.len(), 1);
    assert!(p.facts.program.effects.iter().all(|e| matches!(
        e.effect,
        RuleEffectKind::ProjectActorStat { .. } | RuleEffectKind::ActivateGrant { .. }
    )));
    assert!(matches!(
        p.requirements.program.effects[0].effect,
        RuleEffectKind::Requirement { .. }
    ));
    let a = &p.facts;
    let b = &p.requirements;
    reconstructed(
        &p.original.reads,
        &a.program.reads,
        &a.indices.reads,
        &b.program.reads,
        &b.indices.reads,
        true,
    );
    reconstructed(
        &p.original.nodes,
        &a.program.nodes,
        &a.indices.nodes,
        &b.program.nodes,
        &b.indices.nodes,
        true,
    );
    reconstructed(
        &p.original.effects,
        &a.program.effects,
        &a.indices.effects,
        &b.program.effects,
        &b.indices.effects,
        false,
    );
    exact_reachable(&p.original);
    exact_reachable(&p.facts.program);
    exact_reachable(&p.requirements.program);
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let p: Partition = read("partition.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(b["schema_version"], 1);
    assert_eq!(d.schema_version, 1);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], json!(d.prior_receipt.input));
    assert_eq!(a["release"], b["release"]);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(
        a["scope"],
        json!({"owned_rule_partition_only":true,
        "new_definitions":0,"new_stats":0,"new_effects":0,"removed_effects":0,
        "closed_rule_owners":0,"evaluation_bundle_added":false,
        "native_complete_builds":0,"new_source_parity_claim":false})
    );
    assert_eq!(a["artifacts"].as_array().unwrap().len(), 3);
    for (record, expected) in a["artifacts"].as_array().unwrap().iter().zip([
        "bindings.json",
        "dependencies.json",
        "partition.json",
    ]) {
        assert_eq!(record["file"], expected);
        let bytes = fs::read(data(expected)).unwrap();
        assert_eq!(record["bytes"], bytes.len());
        assert_eq!(record["sha256"], format!("{:x}", Sha256::digest(&bytes)));
    }
    assert_eq!(json!(p.owner), b["owner"]);
    assert_eq!(p.owner, d.owner.owner);
    assert_eq!(b["programs"]["facts"], json!(p.facts.program.id));
    assert_eq!(
        b["programs"]["requirements"],
        json!(p.requirements.program.id)
    );
    assert!(!d.owner.programs.is_complete());
    assert_eq!(
        d.owner
            .programs
            .members
            .iter()
            .filter(|v| v.id == p.original.id)
            .count(),
        1
    );
    assert_eq!(
        d.owner
            .programs
            .members
            .iter()
            .find(|v| v.id == p.original.id),
        Some(&p.original)
    );
    assert!(
        !d.owner
            .programs
            .members
            .iter()
            .any(|v| v.id == p.requirements.program.id)
    );
    check_partition(&p);
}

fn provenance() -> OwnedReleaseProvenance {
    let b: Value = read("bindings.json");
    OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: decode(&b["before"]),
        authoring_input: digest_owned(
            "owned-sniper-population-readiness-v1",
            &(
                read::<Value>("authoring.json"),
                read::<Value>("bindings.json"),
                read::<Dependencies>("dependencies.json"),
                read::<Partition>("partition.json"),
            ),
            4 * 1024 * 1024,
        )
        .unwrap(),
    }
}
fn expected_owner(d: &Dependencies, p: &Partition) -> DefinitionRules {
    let mut owner = d.owner.clone();
    let index = owner
        .programs
        .members
        .iter()
        .position(|v| v.id == p.original.id)
        .unwrap();
    owner.programs.members[index] = p.facts.program.clone();
    owner
        .programs
        .members
        .insert(index + 1, p.requirements.program.clone());
    owner
}
fn inverse(next: &StagedOwnedRelease, d: &Dependencies, p: &Partition) -> OwnedReleaseInput {
    let b: Value = read("bindings.json");
    assert_eq!(json!(next.input().recipe.rules.release), b["release"]);
    assert_eq!(next.receipt().definitions, d.prior_receipt.definitions);
    let mut restored = next.input().clone();
    let owner = restored
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == p.owner)
        .unwrap();
    assert_eq!(*owner, expected_owner(d, p));
    *owner = d.owner.clone();
    restored.recipe.rules.release = d.prior_rules_release.clone();
    assert_eq!(restored.provenance.pop(), Some(provenance()));
    restored
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    check_authored();
    let d: Dependencies = read("dependencies.json");
    let p: Partition = read("partition.json");
    let restored = assemble_owned_release(inverse(next, &d, &p), Default::default()).unwrap();
    assert_eq!(
        restored.receipt(),
        &d.prior_receipt,
        "whole authenticated predecessor, including every unrelated constituent"
    );
    assert_eq!(next.query_sets(), restored.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let p: Partition = read("partition.json");
    assert_eq!(prior.receipt(), &d.prior_receipt);
    assert_eq!(prior.input().recipe.rules.release, d.prior_rules_release);
    let mut input = prior.input().clone();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == p.owner)
        .unwrap();
    assert_eq!(*owner, d.owner);
    *owner = expected_owner(&d, &p);
    input.recipe.rules.release = decode(&b["release"]);
    input.provenance.push(provenance());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(
        inverse(&next, &d, &p),
        *prior.input(),
        "only the exact rule partition, rule-release identity and final provenance differ"
    );
    assert_endpoint(&next);
    next
}
