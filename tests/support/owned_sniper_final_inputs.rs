//! Data-only physical Sniper assembly, using the existing source-property contract.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_readiness::*,
    owned_rules::*,
    owned_schema::{RuleEntityKind, SlotDescriptor},
    owned_source_properties::*,
    owned_stages::*,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "sniper-final-inputs-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/sniper-final-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
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

fn authoring_digest() -> OwnedContentDigest {
    digest_owned(
        "owned-sniper-final-inputs-v1",
        &(
            read::<Value>("authoring.json"),
            read::<Value>("bindings.json"),
            read::<Value>("dependencies.json"),
            read::<Value>("source-vectors.json"),
            read::<OwnedReleaseMigrationInput>("migration.json"),
            read::<SourcePropertyPreparationInput>("source-properties.json"),
            read::<ReadinessFragment>("readiness.json"),
        ),
        4 * 1024 * 1024,
    )
    .unwrap()
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let source: SourcePropertyPreparationInput = read("source-properties.json");
    let ready: ReadinessFragment = read("readiness.json");
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d["source"]["input"]);
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(a["release"], json!(m.release));
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(
        a["scope"],
        json!({"new_definitions":0,"new_programs":1,
        "closed_existing_rule_owners":0,"source_relation_reference_only":true,
        "finite_no_support_component":true,"real_support_origin_discovery_completed":false,
        "evaluation_bundle_added":false,"whole_build_parity":false,"complete_native_builds":0,
        "pob_invalid_level_recovery_reproduced":false})
    );
    for (field, key) in [
        ("physical_gem", "def.0000000000000011"),
        ("primary_skill", "def.0000000000000012"),
    ] {
        assert_eq!(b["target"][field]["key"], key);
    }
    for (field, key) in [
        ("primary_supply", "def.0000000000000016"),
        ("entering_grant", "def.0000000000000017"),
        ("final_level_parameter", "def.0000000000000013"),
        ("final_quality_parameter", "def.0000000000000014"),
        ("actor", "def.000000000000001f"),
        ("population_grant", "def.0000000000000020"),
    ] {
        assert_eq!(b["target"][field]["slot"]["key"], key);
        let owner = if matches!(field, "primary_supply" | "entering_grant") {
            "physical_gem"
        } else {
            "primary_skill"
        };
        assert_eq!(
            b["target"][field]["declaration"]["definition"],
            b["target"][owner]
        );
    }
    assert_eq!((m.schema_version, m.contract.schema_version), (5, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.owners.len(), 1);
    let prior: Vec<DefinitionRules> = decode(&d["owners"]);
    assert_eq!(prior.len(), 1);
    assert_eq!(m.owners[0].owner, prior[0].owner);
    assert_eq!(m.owners[0].programs.closure, prior[0].programs.closure);
    assert!(!m.owners[0].programs.is_complete());
    assert_eq!(m.owners[0].programs.members.len(), 1);
    let p = &m.owners[0].programs.members[0];
    assert_eq!(p.id.as_str(), "sniper-final-inputs");
    assert_eq!(p.context, RuleEntityKind::Actor);
    assert!(prior[0].programs.members.iter().all(|old| old.id != p.id));

    // Reuse the already reviewed assembler exactly, changing only its owner,
    // public program identity and declared output slots. No source behavior is
    // inferred from a final numerical sample.
    let offering: Value = serde_json::from_slice(
        &fs::read(
            root().join("data/owned/poe2/3887ae68/offering-final-inputs/owner-replacement.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let mut expected: RuleProgram = decode(
        offering["owners"][0]["programs"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .find(|p| p["id"] == "pain-offering-final-inputs")
            .unwrap(),
    );
    expected.id = p.id.clone();
    for (effect, field) in expected
        .effects
        .iter_mut()
        .zip(["final_level_parameter", "final_quality_parameter"])
    {
        let RuleEffectKind::ProjectSkillParameter {
            skill, parameter, ..
        } = &mut effect.effect
        else {
            panic!()
        };
        *skill = decode(&b["target"]["primary_supply"]);
        *parameter = decode(&b["target"][field]);
    }
    assert_eq!(
        *p, expected,
        "same exact-integral and bounded-quality assembly"
    );
    assert_eq!(source.relations.members.len(), 1);
    assert!(source.relations.is_complete());
    let relation = &source.relations.members[0];
    assert_eq!(relation.id.as_str(), "sniper-source-inputs");
    assert_eq!(
        json!(relation.owner),
        json!({"kind":"gem","value":b["target"]["physical_gem"]})
    );
    assert_eq!(
        relation.occurrence,
        SourcePropertyOccurrence::AuthoredSkillUse {}
    );
    assert_eq!(
        relation.aliases,
        SourcePropertyAliasPolicy::RejectSharedBackingGemV1
    );
    assert_eq!(
        relation.census,
        SourcePropertyCensus::ExactSelectedPositionV1
    );
    assert_eq!(relation.context, SourcePropertyContext::PlayerScenarioV1);
    assert_eq!(relation.census_stage.as_str(), "source-census");
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
    assert_eq!(
        json!(relation.effects.members),
        json!([{"endpoint":{"kind":"generated","path":[],
        "skill_supply":b["target"]["primary_supply"]},"admission":{"kind":"receiving_skill","summoner_path":null}}])
    );
    assert!(relation.assembly.is_complete());
    assert_eq!(
        relation.assembly.members,
        vec![SourcePropertyAssemblyProgram {
            program: p.id.clone(),
            binding: SourcePropertyAssemblyBinding::InputOwner,
        }]
    );
    assert!(relation.supports.is_complete() && relation.supports.members.is_empty());
    assert!(relation.external.is_complete() && relation.external.members.is_empty());
    assert!(relation.channels.is_complete() && relation.channels.members.is_empty());
    assert!(ready.reference_only);
    assert_eq!(ready.schema_version, 1);
    assert_eq!(ready.readiness.skills.len(), 1);
    assert!(
        ready.readiness.skills[0]
            .parameters
            .members
            .iter()
            .all(|p| p.phase == ReadinessPhase::Execution)
    );
    assert_eq!(ready.readiness.programs.members.len(), 3);
    let assembly = ready
        .readiness
        .programs
        .members
        .iter()
        .find(|r| r.program == p.id)
        .unwrap();
    assert_eq!(
        assembly.role,
        ReadinessProgramRole::SourceFinalInputAssembly
    );
    assert_eq!(assembly.phase, ReadinessPhase::Preparation);
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        assert_eq!(
            hash(&fs::read(data().join(name)).unwrap()),
            expected.as_str().unwrap()
        );
    }
    check_source(false);
}

pub fn check_source(full: bool) {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    assert_eq!(v["scope"], a["scope"]);
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(a["source_manifest_sha256"], hash(&bytes));
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let xml_hash = hash(
        &fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap(),
    );
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 4);
    for (index, report) in reports.iter().enumerate() {
        let projections = report["projections"].as_array().unwrap();
        if index < 2 {
            assert_eq!(projections.len(), 3);
            assert_eq!(
                projections[0]["pointer"],
                "/additional_observation/untouched_original/value"
            );
            let original = &projections[0]["value"];
            for (field, value) in [
                ("raw_level", 20),
                ("corrupt_delta", 0),
                ("raw_quality", 0),
                ("final_level", 22),
                ("final_quality", 0),
                ("actor_level", 44),
            ] {
                assert_eq!(original[field], value);
            }
            assert_eq!(original["skill"], "SummonSkeletalSnipersPlayer");
            assert_eq!(original["main_selection_preserved"], true);
            assert_eq!(original["supports"], json!({}));
            assert_eq!(original["supported_properties"], json!({}));
            let evidence = &projections[1]["value"];
            assert_eq!(evidence["xml_sha256"], xml_hash);
            assert_eq!(evidence["manifest_sha256"], a["source_manifest_sha256"]);
            for file in evidence["files"].as_array().unwrap() {
                let pinned = manifest["files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|r| r["path"] == file["path"])
                    .unwrap();
                assert_eq!(file["sha256"], pinned["sha256"]);
            }
        } else {
            assert_eq!(projections.len(), 13);
            assert_eq!(projections[0]["value"], xml_hash);
            let (contexts, remainder) = projections[1..].as_chunks::<6>();
            assert!(remainder.is_empty());
            for context in contexts {
                assert_eq!(
                    context
                        .iter()
                        .map(|p| p["value"].clone())
                        .collect::<Vec<_>>(),
                    vec![
                        json!(20),
                        json!(22),
                        json!(0),
                        json!(44),
                        json!("RaisedSkeletonSniper"),
                        json!("SummonSkeletalSnipersPlayer")
                    ]
                );
            }
        }
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(json!(bytes.len()), report["bytes"]);
            assert_eq!(hash(&bytes), report["sha256"].as_str().unwrap());
            let raw: Value = serde_json::from_slice(&bytes).unwrap();
            for p in projections {
                assert_eq!(
                    raw.pointer(p["pointer"].as_str().unwrap()),
                    Some(&p["value"])
                );
            }
        }
    }
    assert_eq!(
        reports[0]["projections"], reports[1]["projections"],
        "JIT-independent physical inputs"
    );
    assert_eq!(
        reports[2]["projections"], reports[3]["projections"],
        "JIT-independent actual population inputs"
    );
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    assert_component(endpoint);
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let provenance = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(provenance.kind.as_str(), KIND);
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    assert!(endpoint.evaluation().is_none());
}

/// Check preserved rule/table bodies independently of the last publication step.
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let d: Value = read("dependencies.json");
    let mut expected: DefinitionRules = decode(&d["owners"][0]);
    expected
        .programs
        .members
        .extend(m.owners[0].programs.members.clone());
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| **o == expected)
            .count(),
        1
    );
    let authored: Value = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/import/recipe-seed.json")).unwrap(),
    )
    .unwrap();
    let tables: Vec<IntegerRuleTable> = decode(&authored["rules"]["tables"]);
    for name in ["sniper.actor-level", "sniper.required-character-level"] {
        let expected = tables.iter().find(|t| t.id.as_str() == name).unwrap();
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .tables
                .iter()
                .filter(|t| t.id == expected.id)
                .collect::<Vec<_>>(),
            vec![expected]
        );
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(
        json!(prior.receipt()),
        d["prior_receipt"],
        "exact entire predecessor"
    );
    let owners: Vec<DefinitionRules> = decode(&d["owners"]);
    let supporting: Vec<DefinitionRules> = decode(&d["supporting_owners"]);
    for owner in owners.iter().chain(&supporting) {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|o| *o == owner)
                .count(),
            1
        );
    }
    for slot in decode::<Vec<SlotDescriptor>>(&d["slots"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|s| **s == slot)
                .count(),
            1
        );
    }
    // A whole-package writer census prevents a hidden competing parameter path.
    let b: Value = read("bindings.json");
    for owner in &prior.input().recipe.rules.owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                if let RuleEffectKind::ProjectSkillParameter { parameter, .. } = &effect.effect {
                    assert!(
                        json!(parameter) != b["target"]["final_level_parameter"]
                            && json!(parameter) != b["target"]["final_quality_parameter"]
                    );
                }
            }
        }
    }
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: authoring_digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut restored = next.input().recipe.clone();
    *restored
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == owners[0].owner)
        .unwrap() = owners[0].clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "exactly one appended program and release bindings"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    next
}
