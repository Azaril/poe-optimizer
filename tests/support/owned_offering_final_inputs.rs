//! Exact offline owner replacement; append-only migration permissions stay intact.
#[path = "owned_offering_final_inputs_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_readiness::*,
    owned_rules::*,
    owned_schema::{DefinitionDescriptor, RuleEntityKind, SlotDescriptor},
    owned_source_properties::*,
    owned_stages::*,
    owned_supports::*,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{
        OwnedReleaseContractMigration, OwnedReleaseMigrationInput, compile_owned_release_migration,
    },
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "offering-final-inputs-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data(name: &str) -> PathBuf {
    root()
        .join("data/owned/poe2/3887ae68/offering-final-inputs")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadinessFragment {
    pub schema_version: u32,
    pub reference_only: bool,
    pub stages: Vec<EvaluationStage>,
    pub programs: Vec<StagedRuleProgram>,
    pub readiness: ReadinessInput,
    pub frozen_channels: Vec<FrozenStageChannel>,
}

/// Authoring-only overlay for a checked complete endpoint, deliberately not an
/// append-only migration input or a runtime package format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerReplacement {
    schema_version: u32,
    before: OwnedContentDigest,
    release: OwnedDefinitionKey,
    reason: OwnedDefinitionKey,
    contract: OwnedReleaseContractMigration,
    owners: Vec<DefinitionRules>,
}

pub fn authoring_digest() -> OwnedContentDigest {
    digest_owned(
        "owned-offering-final-inputs-v1",
        &(
            read::<Value>("authoring.json"),
            read::<Value>("bindings.json"),
            read::<Value>("dependencies.json"),
            read::<Value>("source-vectors.json"),
            read::<OwnerReplacement>("owner-replacement.json"),
            read::<SourcePropertyPreparationInput>("source-properties.json"),
            read::<ReadinessFragment>("readiness.json"),
            read::<SupportPreparationInput>("preparation.json"),
        ),
        8 * 1024 * 1024,
    )
    .unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnerReplacement = read("owner-replacement.json");
    let source: SourcePropertyPreparationInput = read("source-properties.json");
    let readiness: ReadinessFragment = read("readiness.json");
    let preparation: SupportPreparationInput = read("preparation.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d["source"]["input"]);
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!((m.schema_version, m.contract.schema_version), (1, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(a["registry_last_issued_before"], 0x3301);
    assert_eq!(a["registry_last_issued_after"], 0x3301);
    let prior: Vec<DefinitionRules> = decode(&d["owners"]);
    assert_eq!(prior.len(), 1);
    assert_eq!(m.owners.len(), 1);
    let old = &prior[0];
    let next = &m.owners[0];
    assert_eq!(next.owner, old.owner);
    assert_eq!(next.programs.closure, old.programs.closure);
    assert!(!next.programs.is_complete());
    let supply = b["programs"]["primary_supply"].as_str().unwrap();
    let assembly = b["programs"]["assembly"].as_str().unwrap();
    assert_eq!(next.programs.members.len(), old.programs.members.len() + 1);
    for p in &old.programs.members {
        let changed = next.programs.members.iter().find(|n| n.id == p.id).unwrap();
        if p.id.as_str() == supply {
            let mut expected = p.clone();
            expected.reads.clear();
            expected.nodes.retain(|n| {
                matches!(
                    n.expression,
                    RuleExpression::Literal {
                        value: poe_optimizer_core::owned_build::ParameterValue::Boolean(true)
                    }
                )
            });
            expected
                .effects
                .retain(|e| matches!(e.effect, RuleEffectKind::ActivateGrant { .. }));
            assert_eq!(expected.nodes.len(), 1);
            assert_eq!(expected.effects.len(), 1);
            assert_eq!(
                *changed, expected,
                "supply retains only exact structural activation"
            );
        } else {
            assert_eq!(
                changed, p,
                "ordinary input preparation remains byte-equivalent"
            );
        }
    }
    let final_inputs = next
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == assembly)
        .unwrap();
    assert_eq!(final_inputs.context, RuleEntityKind::Actor);
    assert_eq!(final_inputs.reads.len(), 2);
    for (read, field) in final_inputs
        .reads
        .iter()
        .zip(["pre_support_level", "pre_support_quality"])
    {
        assert!(
            matches!(&read.source, RuleReadSource::Stat { entity: RuleEntity::PropertyOwner, stat }
            if json!(stat) == b["channels"][field]),
            "only the actual computed source inputs are read"
        );
    }
    assert_eq!(final_inputs.effects.len(), 2);
    for (effect, field) in final_inputs
        .effects
        .iter()
        .zip(["final_level_parameter", "final_quality_parameter"])
    {
        let RuleEffectKind::ProjectSkillParameter {
            skill, parameter, ..
        } = &effect.effect
        else {
            panic!("final source inputs use accepted declared child projection")
        };
        assert_eq!(json!(skill), b["target"]["primary_supply"]);
        assert_eq!(json!(parameter), b["target"][field]);
        assert!(
            effect.when.is_some(),
            "unsupported keys cannot gain final inputs"
        );
    }
    assert!(source.relations.is_complete());
    assert_eq!(source.relations.members.len(), 1);
    let relation = &source.relations.members[0];
    assert_eq!(
        json!(relation.owner),
        json!({"kind":"gem","value":b["target"]["physical_gem"]})
    );
    assert_eq!(
        json!(relation.inputs),
        json!([
            b["channels"]["pre_support_level"],
            b["channels"]["pre_support_quality"]
        ])
    );
    assert_eq!(
        json!(relation.non_hidden_count),
        b["channels"]["non_hidden_count"]
    );
    assert!(relation.effects.is_complete());
    assert_eq!(relation.effects.members.len(), 1);
    assert_eq!(
        json!(relation.effects.members[0]),
        json!({"endpoint":{"kind":"generated","path":[],"skill_supply":b["target"]["primary_supply"]},"admission":{"kind":"receiving_skill","summoner_path":null}})
    );
    assert_eq!(
        relation
            .assembly
            .members
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>(),
        [assembly]
    );
    assert!(relation.assembly.is_complete() && relation.supports.is_complete());
    assert!(relation.channels.is_complete() && relation.channels.members.is_empty());
    assert!(relation.external.is_complete() && relation.external.members.is_empty());
    assert_eq!(relation.supports.members.len(), 2);
    assert!(
        relation
            .supports
            .members
            .iter()
            .all(|s| s.counted && s.programs.is_complete() && s.programs.members.is_empty())
    );
    assert!(readiness.reference_only);
    assert_eq!(readiness.schema_version, 1);
    assert_eq!(readiness.readiness.programs.members.len(), 3);
    assert_eq!(preparation.supports.len(), 2);
    assert_eq!(
        relation
            .supports
            .members
            .iter()
            .map(|s| &s.gem)
            .collect::<Vec<_>>(),
        preparation
            .supports
            .iter()
            .map(|s| &s.gem)
            .collect::<Vec<_>>()
    );
    assert_eq!(json!(preparation.definitions), a["definitions"]);
    assert_eq!(json!(preparation.rules), a["rules"]);
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(data(name)).unwrap())),
            expected.as_str().unwrap(),
            "exact authored artifact {name}"
        );
    }
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&manifest))
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    for pin in a["source_files"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| *p == pin)
                .count(),
            1
        );
    }
    evidence::check(&a, &b, &v, false);
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m: OwnerReplacement = read("owner-replacement.json");
    let provenance = endpoint.input().provenance.last().unwrap();
    assert_eq!(provenance.kind.as_str(), KIND);
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    for owner in m.owners {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|o| **o == owner)
                .count(),
            1
        );
    }
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence::check(
        &read("authoring.json"),
        &read("bindings.json"),
        &read("source-vectors.json"),
        true,
    );
    let d: Value = read("dependencies.json");
    let b: Value = read("bindings.json");
    let census = retired_consumer_census(prior, &b);
    assert_eq!(census, d["prior_consumer_census"]);
    assert_eq!(census, b["retired_consumers"]);
    for owner in decode::<Vec<DefinitionRules>>(&d["supporting_owners"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|o| **o == owner)
                .count(),
            1
        );
    }
    let next = replace_owner(prior, &d);
    assert!(
        retired_consumer_census(&next, &b)
            .as_object()
            .unwrap()
            .values()
            .all(|v| v.as_array().unwrap().is_empty()),
        "no executable retired-stat consumers or writers remain"
    );
    assert_endpoint(&next);
    next
}

fn replace_owner(prior: &StagedOwnedRelease, dependencies: &Value) -> StagedOwnedRelease {
    let a: Value = read("authoring.json");
    let m: OwnerReplacement = read("owner-replacement.json");
    let receipt = json!(prior.receipt());
    for field in [
        "input",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(
            receipt[field],
            a[if field == "input" { "before" } else { field }]
        );
    }
    assert!(prior.evaluation().is_none());
    let old_recipe = &prior.input().recipe;
    assert_eq!(m.contract.schema_version, old_recipe.schema.schema_version);
    assert_eq!(
        m.contract.schema_semantics_version,
        old_recipe.schema.semantics_version
    );
    assert_eq!(
        m.contract.operations_version,
        old_recipe.rules.operations_version
    );
    assert_eq!(
        m.contract.rule_semantics_version,
        old_recipe.rules.semantics_version
    );
    for row in decode::<Vec<DefinitionDescriptor>>(&dependencies["supporting_definitions"]) {
        assert_eq!(
            old_recipe
                .schema
                .definitions
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<SlotDescriptor>>(&dependencies["slots"]) {
        assert_eq!(
            old_recipe
                .schema
                .slots
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    let old: Vec<DefinitionRules> = decode(&dependencies["owners"]);
    assert_eq!(old.len(), 1);
    assert_eq!(
        old_recipe
            .rules
            .owners
            .iter()
            .filter(|r| **r == old[0])
            .count(),
        1
    );
    let next_owner = &m.owners[0];
    let mut addition = old[0].clone();
    let new: Vec<_> = next_owner
        .programs
        .members
        .iter()
        .filter(|p| !addition.programs.members.iter().any(|old| old.id == p.id))
        .cloned()
        .collect();
    assert_eq!(new.len(), 1);
    addition.programs.members.extend(new);
    // Reuse checked dependency rebinding with a genuine append-only intermediate.
    // It is never published. The reviewed replacement then uses full endpoint
    // assembly; the original migration API continues to reject rule rewrites.
    let migrated = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: m.before,
            release: m.release,
            reason: m.reason,
            contract: m.contract,
            schema: vec![],
            tables: vec![],
            owners: vec![addition],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|r| r.owner == next_owner.owner)
        .unwrap() = next_owner.clone();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: authoring_digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut stage_inverse = next.input().clone();
    *stage_inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|r| r.owner == next_owner.owner)
        .unwrap() = migrated
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|r| r.owner == next_owner.owner)
        .unwrap()
        .clone();
    *stage_inverse.provenance.last_mut().unwrap() =
        migrated.input().provenance.last().unwrap().clone();
    assert_eq!(
        stage_inverse,
        *migrated.input(),
        "full endpoint changes only the reviewed owner and provenance after rebinding"
    );
    let mut restored = next.input().recipe.clone();
    *restored
        .rules
        .owners
        .iter_mut()
        .find(|r| r.owner == old[0].owner)
        .unwrap() = old[0].clone();
    restored.schema.release = old_recipe.schema.release.clone();
    restored.rules.definitions = old_recipe.rules.definitions.clone();
    restored.routing.definitions = old_recipe.routing.definitions.clone();
    assert!(
        restored == *old_recipe,
        "only the exact reviewed owner and release bindings may differ"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

fn mentions(value: &Value, needle: &Value) -> bool {
    value == needle
        || match value {
            Value::Object(fields) => fields.values().any(|v| mentions(v, needle)),
            Value::Array(rows) => rows.iter().any(|v| mentions(v, needle)),
            _ => false,
        }
}

/// Recompute the one-consumer retirement proof from complete checked inputs;
/// neither an authored census nor a fixture supplies this authority.
fn retired_consumer_census(endpoint: &StagedOwnedRelease, bindings: &Value) -> Value {
    let input = endpoint.input();
    let mut census = serde_json::Map::new();
    for field in ["physical_final_level", "physical_final_quality"] {
        let stat = &bindings["target"][field];
        assert!(stat.is_object());
        let mut rows = vec![];
        for owner in &input.recipe.rules.owners {
            for program in &owner.programs.members {
                let reads: Vec<_> = program
                    .reads
                    .iter()
                    .filter(|r| mentions(&json!(r.source), stat))
                    .map(|r| r.id.clone())
                    .collect();
                let effects: Vec<_> = program
                    .effects
                    .iter()
                    .filter(|e| mentions(&json!(e.effect), stat))
                    .map(|e| e.id.clone())
                    .collect();
                assert!(!mentions(&json!(program.nodes), stat));
                if !reads.is_empty() || !effects.is_empty() {
                    rows.push(json!({"owner":owner.owner,"program":program.id,"reads":reads,"effects":effects}));
                }
            }
        }
        let mut remainder = json!(input.recipe.rules);
        remainder["owners"] = json!([]);
        assert!(!mentions(&remainder, stat));
        for other in [
            json!(input.recipe.routing),
            json!(input.normalization),
            json!(input.query_sets),
            json!(input.roles),
            json!(input.mapping),
        ] {
            assert!(!mentions(&other, stat));
        }
        census.insert(stat["key"].as_str().unwrap().to_owned(), json!(rows));
    }
    Value::Object(census)
}
