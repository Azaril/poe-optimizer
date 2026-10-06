//! Offline snapshot authoring. Public append-only migration permissions stay intact.
#[path = "owned_amulet_bonus_snapshot_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_readiness::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
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
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "amulet-bonus-snapshot-v1";
pub fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/amulet-bonus-snapshot")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn subject(kind: &str, definition: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":kind,"value":definition}})
}

/// This is an authoring overlay, not a new public migration or runtime format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotAuthoring {
    schema_version: u32,
    before: OwnedContentDigest,
    release: OwnedDefinitionKey,
    reason: OwnedDefinitionKey,
    contract: OwnedReleaseContractMigration,
    owners: Vec<DefinitionRules>,
    receivers: Vec<StatReceiver>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessFragment {
    schema_version: u32,
    reference_only: bool,
    stages: Vec<EvaluationStage>,
    programs: Vec<StagedRuleProgram>,
    readiness: ReadinessInput,
    frozen_channels: Vec<FrozenStageChannel>,
}

pub fn authoring_digest() -> OwnedContentDigest {
    digest_owned(
        "owned-amulet-bonus-snapshot-v1",
        &(
            read::<Value>("authoring.json"),
            read::<Value>("bindings.json"),
            read::<Value>("dependencies.json"),
            read::<Value>("source-vectors.json"),
            read::<SnapshotAuthoring>("snapshot-authoring.json"),
            read::<ReadinessFragment>("readiness.json"),
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
    let m: SnapshotAuthoring = read("snapshot-authoring.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d["source"]["input"]);
    assert_eq!(a["before"], json!(m.before));
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], d["source"][field]);
    }
    assert_eq!(b["definitions"], a["definitions"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(a["release"], b["release"]);
    assert_eq!((m.schema_version, m.contract.schema_version), (1, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(m.reason.as_str(), "amulet-bonus-snapshot");
    assert_eq!(a["allocated_definitions"], 0);
    assert_eq!(a["new_programs"], 1);
    assert_eq!(a["new_receivers"], 1);
    assert_eq!(a["registry_last_issued_before"], 0x3301);
    assert_eq!(a["registry_last_issued_after"], 0x3301);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"], v["scope"]);
    assert_eq!(a["scope"]["snapshot_aggregation"], true);
    for field in [
        "incoming_contributor_closure",
        "existing_owner_closures_changed",
        "post_copy_values_reused_as_input",
        "complete_build_parity",
    ] {
        assert_eq!(a["scope"][field], false);
    }
    let stat = &b["channels"]["pre_amulet_percent"];
    let unit = &b["units"]["percent"];
    assert_eq!(stat["key"], "def.00000000000032e4");
    assert_eq!(unit["key"], "def.0000000000000002");
    assert_eq!(b["passive"]["definition"]["key"], "def.0000000000001b09");
    assert_eq!(b["passive"]["source_node"], 7068);
    assert_eq!(b["passive"]["program"], "mystic-attunement-amulet-percent");
    assert_eq!(b["snapshot"]["program"], "pre-amulet-bonus-snapshot");
    assert_eq!(
        b["snapshot"]["receiver"],
        "player-pre-amulet-bonus-snapshot"
    );
    let owner = subject("stat", stat);
    let quantity = json!({"kind":"quantity","value":{"unit":unit}});
    let expected: DefinitionRules = decode(&json!({
        "owner":owner,"programs":{"closure":{"kind":"complete"},"members":[{
            "id":b["snapshot"]["program"],"context":"actor",
            "reads":[{"id":"incoming","value_type":quantity,"source":{
                "kind":"contributions","value":{"entity":"current","stat":stat,
                    "contribution":"add","reduction":"sum","empty":{
                        "kind":"quantity","value":{"value":0.0,"unit":unit}}}}}],
            "nodes":[{"id":"percent","expression":{"kind":"read","input":"incoming"}}],
            "effects":[{"id":"snapshot","when":null,"effect":{
                "kind":"derive","entity":"current","stat":stat,"value":"percent"}}]
        }]}
    }));
    assert_eq!(m.owners, [expected]);
    let receiver: StatReceiver = decode(&json!({"id":b["snapshot"]["receiver"],
        "stat":stat,"program":b["snapshot"]["program"],"targets":[{"kind":"player"}]}));
    assert_eq!(m.receivers, [receiver]);
    let definitions = d["supporting_definitions"].as_array().unwrap();
    for expected in [
        json!({"kind":"stat","value":{"id":stat,"schema":{"kind":"known","value":{
            "value":quantity,"targets":["actor"]}}}}),
        json!({"kind":"unit","value":{"id":unit,"schema":{"kind":"known","value":{
            "dimension":"percentage_points"}}}}),
    ] {
        assert_eq!(
            definitions.iter().filter(|row| **row == expected).count(),
            1
        );
    }
    let old: Vec<DefinitionRules> = decode(&d["owners"]);
    assert_eq!(old.len(), 2);
    assert!(old.iter().all(|o| !o.programs.is_complete()));
    assert_eq!(d["prior_snapshot_owner_absent"], true);
    assert_eq!(d["prior_snapshot_receivers_absent"], true);
    let passive = old
        .iter()
        .find(|o| json!(o.owner) == subject("passive_node", &b["passive"]["definition"]))
        .unwrap();
    let producer = passive
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == b["passive"]["program"].as_str().unwrap())
        .unwrap();
    assert_eq!(
        json!(producer),
        json!({"id":b["passive"]["program"],"context":"actor","reads":[],
        "nodes":[{"id":"percent","expression":{"kind":"literal","value":{
            "kind":"quantity","value":{"value":25.0,"unit":unit}}}}],
        "effects":[{"id":"pre-copy-amulet-percent","when":null,"effect":{
            "kind":"contribute","entity":"player","stat":stat,"contribution":"add","value":"percent"}}]})
    );
    let modifier = old
        .iter()
        .find(|o| json!(o.owner)["value"]["value"]["key"] == "def.00000000000030ca")
        .unwrap();
    let copy = modifier
        .programs
        .members
        .iter()
        .find(|p| p.id.as_str() == "amulet-copy-minion-gem-level")
        .unwrap();
    assert_eq!(
        copy.reads
            .iter()
            .filter(|r| matches!(&r.source,
        RuleReadSource::Stat { entity: RuleEntity::Player, stat: id } if json!(id) == *stat))
            .count(),
        1
    );
    assert!(!copy.effects.iter().any(|e| matches!(&e.effect,
        RuleEffectKind::Contribute { stat: id, .. } | RuleEffectKind::Derive { stat: id, .. } if json!(id) == *stat)));
    check_readiness(&b, &m);
    let expected_files = BTreeSet::from([
        "bindings.json",
        "dependencies.json",
        "snapshot-authoring.json",
        "readiness.json",
        "source-vectors.json",
    ]);
    let mut seen = BTreeSet::new();
    for asset in a["assets"].as_array().unwrap() {
        let path = asset["path"].as_str().unwrap();
        assert!(expected_files.contains(path) && seen.insert(path));
        assert_eq!(
            format!("{:x}", Sha256::digest(fs::read(data(path)).unwrap())),
            asset["sha256"]
        );
    }
    assert_eq!(seen, expected_files);
    evidence::check(&v, false);
}

fn check_readiness(b: &Value, m: &SnapshotAuthoring) {
    let r: ReadinessFragment = read("readiness.json");
    assert_eq!(r.schema_version, 1);
    assert!(r.reference_only && r.readiness.skills.is_empty());
    assert!(!r.readiness.programs.is_complete());
    assert_eq!(
        json!(r.stages),
        json!([
            {"id":"pre-amulet-contributors","predecessors":[]},
            {"id":"pre-amulet-snapshot","predecessors":["pre-amulet-contributors"]},
            {"id":"amulet-copy","predecessors":["pre-amulet-snapshot"]}
        ])
    );
    assert_eq!(r.programs.len(), 3);
    assert_eq!(r.readiness.programs.members.len(), 2);
    let stat = decode(&b["channels"]["pre_amulet_percent"]);
    let contributions = StageChannel::Contributions {
        scope: RuleEntityKind::Actor,
        stat,
        contribution: ContributionKind::Add,
    };
    let scalar = StageChannel::Stat {
        scope: RuleEntityKind::Actor,
        stat: decode(&b["channels"]["pre_amulet_percent"]),
    };
    assert_ne!(contributions, scalar);
    for (owner, program, stage, output) in [
        (
            decode(&subject("passive_node", &b["passive"]["definition"])),
            b["passive"]["program"].as_str().unwrap(),
            "pre-amulet-contributors",
            contributions.clone(),
        ),
        (
            m.owners[0].owner.clone(),
            b["snapshot"]["program"].as_str().unwrap(),
            "pre-amulet-snapshot",
            scalar.clone(),
        ),
    ] {
        assert_eq!(
            r.programs
                .iter()
                .filter(|p| p.owner == owner
                    && p.program.as_str() == program
                    && p.stage.as_str() == stage)
                .count(),
            1
        );
        assert_eq!(
            r.readiness
                .programs
                .members
                .iter()
                .filter(|p| p.owner == owner
                    && p.program.as_str() == program
                    && p.phase == ReadinessPhase::Structural
                    && p.role == ReadinessProgramRole::PreparationFacts
                    && p.outputs == [output.clone()])
                .count(),
            1
        );
    }
    assert_eq!(
        r.programs
            .iter()
            .filter(|p| p.program.as_str() == "amulet-copy-minion-gem-level"
                && p.stage.as_str() == "amulet-copy"
                && json!(p.owner)["value"]["value"]["key"] == "def.00000000000030ca")
            .count(),
        1
    );
    assert_eq!(
        r.frozen_channels,
        [
            FrozenStageChannel {
                channel: contributions,
                stage: key("pre-amulet-contributors")
            },
            FrozenStageChannel {
                channel: scalar,
                stage: key("pre-amulet-snapshot")
            },
        ]
    );
}

pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m: SnapshotAuthoring = read("snapshot-authoring.json");
    let provenance = endpoint.input().provenance.last().unwrap();
    assert_eq!(provenance.kind.as_str(), KIND);
    assert_eq!(provenance.prior_input, m.before);
    assert_eq!(provenance.authoring_input, authoring_digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|o| **o == m.owners[0])
            .count(),
        1
    );
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| **r == m.receivers[0])
            .count(),
        1
    );
    assert_eq!(
        json!(endpoint.receipt().registry),
        read::<Value>("authoring.json")["registry"]
    );
    assert_dependencies(endpoint, &read("dependencies.json"));
    assert!(endpoint.evaluation().is_none());
}

fn assert_dependencies(endpoint: &StagedOwnedRelease, dependencies: &Value) {
    let recipe = &endpoint.input().recipe;
    for row in decode::<Vec<DefinitionDescriptor>>(&dependencies["supporting_definitions"]) {
        assert_eq!(
            recipe
                .schema
                .definitions
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<SlotDescriptor>>(&dependencies["slots"]) {
        assert_eq!(recipe.schema.slots.iter().filter(|r| **r == row).count(), 1);
    }
    for row in decode::<Vec<DefinitionRules>>(&dependencies["owners"]) {
        assert_eq!(recipe.rules.owners.iter().filter(|r| **r == row).count(), 1);
    }
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence::check(&read("source-vectors.json"), true);
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: SnapshotAuthoring = read("snapshot-authoring.json");
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
    let old = &prior.input().recipe;
    assert!(prior.evaluation().is_none());
    assert_eq!(m.contract.schema_version, old.schema.schema_version);
    assert_eq!(
        m.contract.schema_semantics_version,
        old.schema.semantics_version
    );
    assert_eq!(
        m.contract.rule_semantics_version,
        old.rules.semantics_version
    );
    assert_eq!(m.contract.operations_version, old.rules.operations_version);
    assert_dependencies(prior, &d);
    let new_owner = &m.owners[0];
    let new_receiver = &m.receivers[0];
    assert!(!old.rules.owners.iter().any(|o| o.owner == new_owner.owner));
    assert!(
        !old.rules
            .receivers
            .members
            .iter()
            .any(|r| r.id == new_receiver.id || r.stat == new_receiver.stat)
    );
    assert!(old.rules.receivers.is_complete());
    let mut temporary_owner = new_owner.clone();
    temporary_owner.programs.closure = SchemaClosure::Partial {
        gaps: vec![SchemaGap {
            subject: new_owner.owner.clone(),
            facet: SchemaFacet::GameRules,
            code: key("snapshot-authoring-awaits-proof"),
        }],
    };
    // Genuine append-only V5 staging performs every dependency rebind. It cannot
    // close an existing subject or extend complete receiver membership and is
    // never published. The reviewed full endpoint below has separate authority.
    let temporary = compile_owned_release_migration(
        prior,
        OwnedReleaseMigrationInput {
            schema_version: 5,
            before: m.before,
            release: m.release,
            reason: m.reason,
            contract: m.contract,
            schema: vec![],
            tables: vec![],
            owners: vec![temporary_owner],
            receivers: vec![],
            query_targets: vec![],
            evaluation: None,
        },
        Default::default(),
    )
    .unwrap();
    let mut input = temporary.input().clone();
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
        .find(|o| o.owner == new_owner.owner)
        .unwrap() = new_owner.clone();
    input
        .recipe
        .rules
        .receivers
        .members
        .push(new_receiver.clone());
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: authoring_digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut temporary_inverse = next.input().clone();
    *temporary_inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == new_owner.owner)
        .unwrap() = temporary
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == new_owner.owner)
        .unwrap()
        .clone();
    temporary_inverse
        .recipe
        .rules
        .receivers
        .members
        .retain(|r| r.id != new_receiver.id);
    *temporary_inverse.provenance.last_mut().unwrap() =
        temporary.input().provenance.last().unwrap().clone();
    assert!(
        temporary_inverse == *temporary.input(),
        "endpoint differs beyond the exact snapshot owner, receiver and provenance"
    );
    let mut restored = next.input().recipe.clone();
    assert_eq!(restored.rules.owners.len(), old.rules.owners.len() + 1);
    assert_eq!(
        restored.rules.receivers.members.len(),
        old.rules.receivers.members.len() + 1
    );
    restored.rules.owners.retain(|o| o.owner != new_owner.owner);
    restored
        .rules
        .receivers
        .members
        .retain(|r| r.id != new_receiver.id);
    restored.schema.release = old.schema.release.clone();
    restored.rules.definitions = old.rules.definitions.clone();
    restored.routing.definitions = old.routing.definitions.clone();
    assert!(
        restored == *old,
        "prior recipe changed beyond snapshot owner, receiver and release bindings"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert_endpoint(&next);
    next
}
