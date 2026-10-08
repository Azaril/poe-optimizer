//! Offline exact rule partition. No runtime gate policy or coverage is changed.
#[allow(dead_code)]
#[path = "owned_sniper_population_readiness.rs"]
pub mod population_partition;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::{DefinitionRules, RuleEffectKind, RuleExpression, RuleProgram},
    owned_schema::SchemaSubject,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseInput, OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
pub use population_partition::Fragment;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "sniper-activation-readiness-v1";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/sniper-activation-readiness")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(value).unwrap()
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActivationPartition {
    pub owner: SchemaSubject,
    pub original: RuleProgram,
    pub numerical: Fragment,
    pub activation: Fragment,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Partitions {
    schema_version: u32,
    partitions: Vec<ActivationPartition>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgramReference {
    path: String,
    sha256: String,
    pointer: String,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    schema_version: u32,
    prior_input: OwnedContentDigest,
    prior_rules_release: OwnedDefinitionKey,
    original_programs: Vec<ProgramReference>,
}
pub fn partitions() -> Vec<ActivationPartition> {
    let packet: Partitions = read("partitions.json");
    assert_eq!(packet.schema_version, 1);
    packet.partitions
}
pub fn check_partition(partition: &ActivationPartition) {
    assert_eq!(partition.original.id, partition.numerical.program.id);
    assert_ne!(partition.original.id, partition.activation.program.id);
    population_partition::check_exact_partition(
        &partition.original,
        &partition.numerical,
        &partition.activation,
    );
    let activation = &partition.activation.program;
    assert!(activation.reads.is_empty());
    assert_eq!(activation.nodes.len(), 1);
    assert_eq!(activation.effects.len(), 1);
    assert!(matches!(
        &activation.nodes[0].expression,
        RuleExpression::Literal {
            value: poe_optimizer_core::owned_build::ParameterValue::Boolean(true)
        }
    ));
    assert!(activation.effects[0].when.is_none());
    assert!(matches!(
        &activation.effects[0].effect,
        RuleEffectKind::ActivateGrant { enabled, .. } if *enabled == activation.nodes[0].id
    ));
    assert!(partition.numerical.program.effects.iter().all(|effect| {
        matches!(
            effect.effect,
            RuleEffectKind::ProjectActorStat { .. } | RuleEffectKind::ProjectSkillParameter { .. }
        )
    }));
}
pub fn check_authored() {
    let authoring: Value = read("authoring.json");
    let dependencies: Dependencies = read("dependencies.json");
    let rows = partitions();
    assert_eq!(authoring["schema_version"], 1);
    assert_eq!(authoring["kind"], KIND);
    assert_eq!(dependencies.schema_version, 1);
    assert_eq!(authoring["before"], json!(dependencies.prior_input));
    assert_eq!(rows.len(), 2);
    assert_eq!(dependencies.original_programs.len(), rows.len());
    assert_eq!(
        authoring["scope"],
        json!({
            "owned_rule_partition_only":true,"partitioned_programs":2,"new_programs":2,
            "new_definitions":0,"new_stats":0,"new_effects":0,"removed_effects":0,
            "closed_rule_owners":0,"evaluation_bundle_added":false,
            "native_complete_builds":0,"new_source_parity_claim":false
        })
    );
    for (row, (old, new)) in rows.iter().zip([
        (
            "ordinary-population-inputs",
            "ordinary-population-activation",
        ),
        ("basic-attack-supply", "basic-attack-activation"),
    ]) {
        assert_eq!(row.original.id, key(old));
        assert_eq!(row.activation.program.id, key(new));
        check_partition(row);
    }
    for (row, reference) in rows.iter().zip(&dependencies.original_programs) {
        let bytes =
            fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(&reference.path)).unwrap();
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), reference.sha256);
        let document: Value = serde_json::from_slice(&bytes).unwrap();
        let original: RuleProgram =
            serde_json::from_value(document.pointer(&reference.pointer).unwrap().clone()).unwrap();
        assert_eq!(row.original, original, "exact existing authored program");
    }
    let artifacts = authoring["artifacts"].as_array().unwrap();
    assert_eq!(artifacts.len(), 2);
    for (artifact, name) in artifacts
        .iter()
        .zip(["partitions.json", "dependencies.json"])
    {
        assert_eq!(artifact["file"], name);
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(artifact["bytes"], bytes.len());
        assert_eq!(artifact["sha256"], format!("{:x}", Sha256::digest(bytes)));
    }
}
fn provenance() -> OwnedReleaseProvenance {
    let dependencies: Dependencies = read("dependencies.json");
    OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: dependencies.prior_input,
        authoring_input: digest_owned(
            "owned-sniper-activation-readiness-v1",
            &(
                read::<Value>("authoring.json"),
                dependencies,
                read::<Partitions>("partitions.json"),
            ),
            1024 * 1024,
        )
        .unwrap(),
    }
}
fn owner_inverse(owner: &DefinitionRules, partition: &ActivationPartition) -> DefinitionRules {
    assert_eq!(owner.owner, partition.owner);
    let mut restored = owner.clone();
    let programs = &mut restored.programs.members;
    let indices: Vec<_> = programs
        .iter()
        .enumerate()
        .filter(|(_, program)| program.id == partition.numerical.program.id)
        .map(|(index, _)| index)
        .collect();
    assert_eq!(indices.len(), 1);
    let index = indices[0];
    assert_eq!(programs[index], partition.numerical.program);
    assert_eq!(programs.get(index + 1), Some(&partition.activation.program));
    assert_eq!(
        programs
            .iter()
            .filter(|p| p.id == partition.activation.program.id)
            .count(),
        1
    );
    programs[index] = partition.original.clone();
    programs.remove(index + 1);
    restored
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let matching: Vec<_> = endpoint
        .input()
        .provenance
        .iter()
        .filter(|p| p.kind == key(KIND))
        .collect();
    assert_eq!(matching, [&provenance()]);
    for partition in partitions() {
        let owners: Vec<_> = endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|owner| owner.owner == partition.owner)
            .collect();
        assert_eq!(owners.len(), 1);
        assert!(!owners[0].programs.is_complete());
        owner_inverse(owners[0], &partition);
    }
}
/// An exact component inverse for existing fixture authoring checks. No caller
/// can select a loose alternative body; a successor must carry this proof.
pub fn restored_owner(
    endpoint: &StagedOwnedRelease,
    subject: &SchemaSubject,
) -> Option<DefinitionRules> {
    if !endpoint
        .input()
        .provenance
        .iter()
        .any(|p| p.kind == key(KIND))
    {
        return None;
    }
    assert_component(endpoint);
    let partition = partitions().into_iter().find(|p| &p.owner == subject)?;
    let owner = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| &o.owner == subject)
        .unwrap();
    Some(owner_inverse(owner, &partition))
}
fn inverse(next: &StagedOwnedRelease) -> OwnedReleaseInput {
    assert_component(next);
    let dependencies: Dependencies = read("dependencies.json");
    let authoring: Value = read("authoring.json");
    assert_eq!(
        json!(next.input().recipe.rules.release),
        authoring["release"]
    );
    let mut restored = next.input().clone();
    for partition in partitions() {
        let owner = restored
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == partition.owner)
            .unwrap();
        *owner = owner_inverse(owner, &partition);
    }
    restored.recipe.rules.release = dependencies.prior_rules_release;
    assert_eq!(restored.provenance.pop(), Some(provenance()));
    restored
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    let dependencies: Dependencies = read("dependencies.json");
    let restored = assemble_owned_release(inverse(next), Default::default()).unwrap();
    assert_eq!(
        restored.receipt().input,
        dependencies.prior_input,
        "entire predecessor input"
    );
    assert_eq!(next.query_sets(), restored.query_sets());
    assert_eq!(next.receipt().definitions, restored.receipt().definitions);
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let dependencies: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, dependencies.prior_input);
    assert_eq!(
        prior.input().recipe.rules.release,
        dependencies.prior_rules_release
    );
    let mut input = prior.input().clone();
    for partition in partitions() {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == partition.owner)
            .unwrap();
        assert!(!owner.programs.is_complete());
        let programs = &mut owner.programs.members;
        let index = programs
            .iter()
            .position(|p| p.id == partition.original.id)
            .unwrap();
        assert_eq!(programs[index], partition.original);
        assert!(
            !programs
                .iter()
                .any(|p| p.id == partition.activation.program.id)
        );
        programs[index] = partition.numerical.program;
        programs.insert(index + 1, partition.activation.program);
    }
    input.recipe.rules.release =
        serde_json::from_value(read::<Value>("authoring.json")["release"].clone()).unwrap();
    input.provenance.push(provenance());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_eq!(inverse(&next), *prior.input(), "exact full input inverse");
    assert_endpoint(&next);
    next
}
