//! Offline, source-bound ordinary Player routing. Copy routing remains separate.
#[path = "owned_ordinary_item_routing_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_readiness::*,
    owned_rules::*,
    owned_schema::*,
    owned_stages::*,
};
use poe_optimizer_import::{
    owned_mapping::{MappingEntry, OwnedIdRegistry},
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "ordinary-item-routing-v1";
pub fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/ordinary-item-routing")
        .join(name)
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data(name)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(value: &Value) -> T {
    serde_json::from_value(value.clone()).unwrap()
}
fn key(text: &str) -> OwnedDefinitionKey {
    text.parse().unwrap()
}
fn subject(kind: &str, definition: &Value) -> Value {
    json!({"kind":"definition","value":{"kind":kind,"value":definition}})
}
fn quantity(value: f64, unit: &Value) -> Value {
    json!({"kind":"quantity","value":{"value":value,"unit":unit}})
}

/// Explicit full-endpoint authoring authority, not a public migration format.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RoutingAuthoring {
    schema_version: u32,
    before: OwnedContentDigest,
    release: OwnedDefinitionKey,
    owner: DefinitionRules,
    prior_owner: DefinitionRules,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadinessFragment {
    schema_version: u32,
    reference_only: bool,
    minimum_stages_version: u32,
    stages: Vec<EvaluationStage>,
    programs: Vec<StagedRuleProgram>,
    readiness: ReadinessInput,
    frozen_channels: Vec<FrozenStageChannel>,
}

pub fn authoring_digest() -> OwnedContentDigest {
    digest_owned(
        "owned-ordinary-item-routing-v1",
        &(
            read::<Value>("authoring.json"),
            read::<Value>("bindings.json"),
            read::<Value>("dependencies.json"),
            read::<Value>("source-vectors.json"),
            read::<OwnedReleaseMigrationInput>("migration.json"),
            read::<RoutingAuthoring>("routing-authoring.json"),
            read::<ReadinessFragment>("readiness.json"),
        ),
        8 * 1024 * 1024,
    )
    .unwrap()
}

fn guarded_owner(prior: &DefinitionRules, b: &Value) -> DefinitionRules {
    let mut expected = prior.clone();
    let program = expected
        .programs
        .members
        .iter_mut()
        .find(|p| p.id.as_str() == b["modifier"]["direct_program"].as_str().unwrap())
        .unwrap();
    assert_eq!(program.context, RuleEntityKind::EquipmentUse);
    assert_eq!(program.reads.len(), 1);
    assert_eq!(program.nodes.len(), 1);
    assert_eq!(program.effects.len(), 1);
    assert!(program.effects[0].when.is_none());
    assert_eq!(
        json!(program.effects[0].effect),
        json!({"kind":"contribute",
        "entity":"player","stat":b["modifier"]["minion_level"],
        "contribution":"add","value":"effective"})
    );
    program
        .reads
        .push(decode(&json!({"id":"direct-applicability",
        "value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{
            "entity":"current","stat":b["channels"]["direct_applicability"]}}})));
    program
        .nodes
        .push(decode(&json!({"id":"direct-applicability",
        "expression":{"kind":"read","input":"direct-applicability"}})));
    program.effects[0].when = Some(key("direct-applicability"));
    expected
}

pub fn expected_modifier_owner() -> DefinitionRules {
    let b: Value = read("bindings.json");
    let overlay: RoutingAuthoring = read("routing-authoring.json");
    let expected = guarded_owner(&overlay.prior_owner, &b);
    assert_eq!(
        overlay.owner, expected,
        "only the required direct guard changes"
    );
    expected
}

fn expected_applicability(template: &Value, b: &Value) -> RuleProgram {
    let (reads, nodes) = if template["is_amulet"].as_bool().unwrap() {
        (
            json!([{"id":"player-retention","value_type":{"kind":"quantity","value":{
            "unit":b["units"]["factor"]}},"source":{"kind":"stat","value":{
            "entity":"player","stat":b["channels"]["player_retention_factor"]}}}]),
            json!([{"id":"player-retention","expression":{"kind":"read","input":"player-retention"}},
            {"id":"identity","expression":{"kind":"literal","value":quantity(1.0,&b["units"]["factor"])}},
            {"id":"applicable","expression":{"kind":"compare","operation":"equal",
                "left":"player-retention","right":"identity"}}]),
        )
    } else {
        (
            json!([]),
            json!([{"id":"applicable","expression":{"kind":"literal",
            "value":{"kind":"boolean","value":true}}}]),
        )
    };
    decode(
        &json!({"id":b["programs"]["applicability"],"context":"equipment_use",
        "reads":reads,"nodes":nodes,"effects":[{"id":"direct-applicability","when":null,
            "effect":{"kind":"derive","entity":"current",
                "stat":b["channels"]["direct_applicability"],"value":"applicable"}}]}),
    )
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let overlay: RoutingAuthoring = read("routing-authoring.json");
    assert_eq!(a["kind"], KIND);
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
    assert_eq!(a["before"], d["source"]["input"]);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(m.before, overlay.before);
    assert_eq!(a["definitions"], b["definitions"]);
    assert_eq!(a["release"], b["release"]);
    assert_eq!(b["release"], json!(m.release));
    assert_eq!(m.release, overlay.release);
    assert_eq!(m.release.as_str(), "pob-3887ae68-ordinary-item-routing-v1");
    assert_eq!(overlay.schema_version, 1);
    assert_eq!((m.schema_version, m.contract.schema_version), (5, 6));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V20
    );
    assert_eq!(m.reason.as_str(), "ordinary-item-routing");
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    for (field, expected) in [
        ("allocated_definitions", 2),
        ("new_programs", 8),
        ("replaced_programs", 1),
        ("new_receivers", 1),
        ("closed_existing_owners", 0),
        ("registry_last_issued_before", 0x3304),
        ("registry_last_issued_after", 0x3306),
    ] {
        assert_eq!(a[field], expected);
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"]["ordinary_player_delivery_only"], true);
    assert_eq!(a["scope"]["reviewed_allocated_passive_domain"], true);
    assert_eq!(a["scope"]["early_copy_program_unchanged"], true);
    for name in [
        "incoming_contributor_inventory_closed",
        "existing_owner_closures_changed",
        "other_item_routing_complete",
        "minion_property_consumer_claimed",
        "late_granted_passive_authority",
        "complete_build_parity",
        "evaluation_bundle",
    ] {
        assert_eq!(a["scope"][name], false);
    }
    assert_eq!(
        b["channels"]["player_retention_factor"]["key"],
        "def.0000000000003305"
    );
    assert_eq!(
        b["channels"]["direct_applicability"]["key"],
        "def.0000000000003306"
    );
    assert_eq!(b["units"]["factor"]["key"], "def.0000000000000001");
    assert_eq!(
        b["preserved_channels"]["copy_eligibility"]["key"],
        "def.00000000000032e3"
    );
    assert_eq!(
        b["preserved_channels"]["pre_amulet_percent"]["key"],
        "def.00000000000032e4"
    );
    assert_eq!(b["modifier"]["definition"]["key"], "def.00000000000030ca");
    assert_eq!(b["passive"]["definition"]["key"], "def.0000000000001338");
    assert_eq!(b["passive"]["source_node"], 39935);
    assert_eq!(
        b["modifier"]["direct_program"],
        "contribute-player-minion-gem-level"
    );
    assert_eq!(
        b["modifier"]["copy_program"],
        "amulet-copy-minion-gem-level"
    );
    assert_eq!(b["modifier"]["effective"]["key"], "def.000000000000295b");
    assert_eq!(b["modifier"]["minion_level"]["key"], "def.00000000000030ab");
    assert_eq!(
        b["passive"]["program"],
        "talisman-ordinary-amulet-retention"
    );
    assert_eq!(
        b["programs"]["retention"],
        "ordinary-amulet-retention-factor"
    );
    assert_eq!(
        b["programs"]["receiver"],
        "player-ordinary-amulet-retention-factor"
    );
    assert_eq!(
        b["programs"]["applicability"],
        "ordinary-item-direct-applicability"
    );
    assert_eq!(
        d["mapping_rows"],
        json!([{"source":{"kind":"definition","value":{
        "kind":"passive_node","value":{"tree_version":{"kind":"text","value":"0_5"},
            "node_id":{"kind":"text","value":"39935"},"view":{"kind":"missing"}}}},
        "outcome":{"kind":"mapped","value":{"target":subject("passive_node",&b["passive"]["definition"]),
            "basis":{"kind":"exact"}}}}])
    );
    assert_eq!(
        (m.schema.len(), m.owners.len(), m.receivers.len()),
        (2, 8, 1)
    );
    let factor = &b["channels"]["player_retention_factor"];
    let unit = &b["units"]["factor"];
    assert_eq!(
        d["supporting_definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| **row
                == json!({"kind":"unit","value":{"id":unit,"schema":{"kind":"known",
            "value":{"dimension":"dimensionless_factor"}}}}))
            .count(),
        1
    );
    for (i, name, value, scope) in [
        (
            0,
            "player_retention_factor",
            json!({"kind":"quantity","value":{"unit":unit}}),
            "actor",
        ),
        (
            1,
            "direct_applicability",
            json!({"kind":"boolean"}),
            "equipment_use",
        ),
    ] {
        assert_eq!(
            json!(m.schema[i]),
            json!({"kind":"definition","value":{
            "kind":"stat","value":{"id":b["channels"][name],"schema":{"kind":"known",
                "value":{"value":value,"targets":[scope]}}}}})
        );
    }
    let old: Vec<DefinitionRules> = decode(&d["owners"]);
    assert_eq!(old.len(), 8);
    assert!(old.iter().all(|o| !o.programs.is_complete()));
    let factor_owner: DefinitionRules = decode(&json!({"owner":subject("stat",factor),
        "programs":{"closure":{"kind":"complete"},"members":[{
            "id":b["programs"]["retention"],"context":"actor","reads":[{
                "id":"incoming","value_type":{"kind":"quantity","value":{"unit":unit}},
                "source":{"kind":"contributions","value":{"entity":"current","stat":factor,
                    "contribution":"multiply","reduction":"product","empty":quantity(1.0,unit)}}}],
            "nodes":[{"id":"incoming","expression":{"kind":"read","input":"incoming"}}],
            "effects":[{"id":"retention","when":null,"effect":{"kind":"derive",
                "entity":"current","stat":factor,"value":"incoming"}}]}]}}));
    assert_eq!(m.owners.iter().filter(|o| **o == factor_owner).count(), 1);
    assert_eq!(
        json!(m.receivers),
        json!([{"id":b["programs"]["receiver"],"stat":factor,
        "program":b["programs"]["retention"],"targets":[{"kind":"player"}]}])
    );
    let passive_program: RuleProgram = decode(&json!({"id":b["passive"]["program"],
        "context":"actor","reads":[],"nodes":[{"id":"retention","expression":{
            "kind":"literal","value":quantity(0.0,unit)}}],"effects":[{
                "id":"deny-ordinary-amulet-player-delivery","when":null,"effect":{
                    "kind":"contribute","entity":"player","stat":factor,
                    "contribution":"multiply","value":"retention"}}]}));
    let templates = b["templates"].as_array().unwrap();
    assert_eq!(templates.len(), 6);
    for (row, (id, is_amulet, complete)) in templates.iter().zip([
        (0x1f1c, false, true),
        (0x1f37, false, true),
        (0x1f45, true, true),
        (0x1fe3, false, false),
        (0x22f5, false, false),
        (0x2343, true, true),
    ]) {
        assert_eq!(row["template"]["key"], format!("def.{id:016x}"));
        assert_eq!(row["is_amulet"], is_amulet);
        assert_eq!(row["complete_placement"], complete);
        let observations = v["template_evidence"]["observations"].as_array().unwrap();
        let source = observations
            .iter()
            .filter(|o| o["value"]["template"] == row["template"])
            .collect::<Vec<_>>();
        assert_eq!(source.len(), 1);
        assert_eq!(source[0]["value"]["source_type"] == "Amulet", is_amulet);
        let descriptor = d["supporting_definitions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["kind"] == "item_template" && d["value"]["id"] == row["template"])
            .unwrap();
        let schema = &descriptor["value"]["schema"]["value"];
        assert_eq!(
            schema["equipment_slots"]["closure"]["kind"],
            if complete { "complete" } else { "partial" }
        );
        if is_amulet {
            assert_eq!(
                schema["equipment_slots"]["members"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                schema["equipment_slots"]["members"][0]["key"],
                "def.000000000000006a"
            );
        }
    }
    for (owner_subject, program) in std::iter::once((
        decode::<SchemaSubject>(&subject("passive_node", &b["passive"]["definition"])),
        passive_program,
    ))
    .chain(templates.iter().map(|t| {
        (
            decode(&subject("item_template", &t["template"])),
            expected_applicability(t, &b),
        )
    })) {
        let prior = old.iter().find(|o| o.owner == owner_subject).unwrap();
        let mut expected = prior.clone();
        assert!(!expected.programs.members.iter().any(|p| p.id == program.id));
        expected.programs.members.push(program);
        assert_eq!(
            m.owners.iter().filter(|o| **o == expected).count(),
            1,
            "exact append with unchanged Partial inventory"
        );
    }
    assert_eq!(old.iter().filter(|o| **o == overlay.prior_owner).count(), 1);
    assert_eq!(overlay.owner, expected_modifier_owner());
    assert_eq!(
        overlay.owner.programs.closure,
        overlay.prior_owner.programs.closure
    );
    let copy_name = b["modifier"]["copy_program"].as_str().unwrap();
    assert_eq!(
        overlay
            .owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == copy_name),
        overlay
            .prior_owner
            .programs
            .members
            .iter()
            .find(|p| p.id.as_str() == copy_name)
    );
    check_readiness(&b, &m);
    check_assets(&a);
    evidence::check(&v, false);
}

fn check_assets(a: &Value) {
    let expected = BTreeSet::from([
        "bindings",
        "dependencies",
        "migration",
        "readiness",
        "routing-authoring",
        "source-vectors",
    ]);
    let assets = a["artifact_sha256"].as_object().unwrap();
    assert_eq!(
        assets.keys().map(String::as_str).collect::<BTreeSet<_>>(),
        expected
    );
    for (name, digest) in assets {
        let bytes = fs::read(data(&format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(*digest, format!("{:x}", Sha256::digest(&bytes)));
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("crates/poe-optimizer-pob/data/pob-source-manifest.json");
    let bytes = fs::read(path).unwrap();
    assert_eq!(
        a["source_manifest_sha256"],
        format!("{:x}", Sha256::digest(&bytes))
    );
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 8);
    let mut paths = BTreeSet::new();
    for pin in pins {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
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
}

fn check_readiness(b: &Value, m: &OwnedReleaseMigrationInput) {
    let r: ReadinessFragment = read("readiness.json");
    assert_eq!((r.schema_version, r.minimum_stages_version), (1, 4));
    assert!(
        r.reference_only && !r.readiness.programs.is_complete() && r.readiness.skills.is_empty()
    );
    assert_eq!(
        json!(r.stages),
        json!([
        {"id":"ordinary-routing-contributors","predecessors":[]},
        {"id":"ordinary-routing-factor","predecessors":["ordinary-routing-contributors"]},
        {"id":"ordinary-routing-applicability","predecessors":["ordinary-routing-factor"]},
        {"id":"ordinary-item-contribution","predecessors":["ordinary-routing-applicability"]}])
    );
    let contributions = StageChannel::Contributions {
        scope: RuleEntityKind::Actor,
        stat: decode(&b["channels"]["player_retention_factor"]),
        contribution: ContributionKind::Multiply,
    };
    let scalar = StageChannel::Stat {
        scope: RuleEntityKind::Actor,
        stat: decode(&b["channels"]["player_retention_factor"]),
    };
    let applicability = StageChannel::Stat {
        scope: RuleEntityKind::EquipmentUse,
        stat: decode(&b["channels"]["direct_applicability"]),
    };
    let mut expected = Vec::new();
    for owner in &m.owners {
        let (program, stage, channel) =
            if json!(owner.owner) == subject("stat", &b["channels"]["player_retention_factor"]) {
                (
                    &b["programs"]["retention"],
                    "ordinary-routing-factor",
                    scalar.clone(),
                )
            } else if json!(owner.owner) == subject("passive_node", &b["passive"]["definition"]) {
                (
                    &b["passive"]["program"],
                    "ordinary-routing-contributors",
                    contributions.clone(),
                )
            } else {
                (
                    &b["programs"]["applicability"],
                    "ordinary-routing-applicability",
                    applicability.clone(),
                )
            };
        expected.push((
            owner.owner.clone(),
            program.as_str().unwrap(),
            stage,
            channel,
        ));
    }
    expected.push((
        decode(&subject("modifier", &b["modifier"]["definition"])),
        b["modifier"]["direct_program"].as_str().unwrap(),
        "ordinary-item-contribution",
        StageChannel::Contributions {
            scope: RuleEntityKind::Actor,
            stat: decode(&b["modifier"]["minion_level"]),
            contribution: ContributionKind::Add,
        },
    ));
    assert_eq!(r.programs.len(), expected.len());
    assert_eq!(r.readiness.programs.members.len(), expected.len());
    for (owner, program, stage, output) in expected {
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
        r.frozen_channels,
        [
            FrozenStageChannel {
                channel: contributions,
                stage: key("ordinary-routing-contributors")
            },
            FrozenStageChannel {
                channel: scalar,
                stage: key("ordinary-routing-factor")
            },
            FrozenStageChannel {
                channel: applicability,
                stage: key("ordinary-routing-applicability")
            }
        ]
    );
}

fn assert_dependencies(prior: &StagedOwnedRelease, d: &Value) {
    for row in decode::<Vec<DefinitionDescriptor>>(&d["supporting_definitions"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<SlotDescriptor>>(&d["slots"]) {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
    for row in decode::<Vec<MappingEntry>>(&d["mapping_rows"]) {
        assert_eq!(
            prior
                .input()
                .mapping
                .entries
                .iter()
                .filter(|r| **r == row)
                .count(),
            1
        );
    }
}

pub fn assert_endpoint(next: &StagedOwnedRelease) {
    check_authored();
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = next.input().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(proof.prior_input, m.before);
    assert_eq!(proof.authoring_input, authoring_digest());
    assert_eq!(next.input().recipe.schema.release, m.release);
    for entry in &m.schema {
        let SchemaExtensionEntry::Definition(row) = entry else {
            panic!("Stat definitions only")
        };
        assert_eq!(
            next.input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| *r == row)
                .count(),
            1
        );
    }
    let modifier_owner = expected_modifier_owner();
    for owner in m.owners.iter().chain(std::iter::once(&modifier_owner)) {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|r| *r == owner)
                .count(),
            1
        );
    }
    for receiver in &m.receivers {
        assert_eq!(
            next.input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|r| *r == receiver)
                .count(),
            1
        );
    }
    assert!(next.evaluation().is_none());
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence::check(&read("source-vectors.json"), true);
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let overlay: RoutingAuthoring = read("routing-authoring.json");
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
    let old = &prior.input().recipe;
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
    let old_owners: Vec<DefinitionRules> = decode(&d["owners"]);
    for owner in &old_owners {
        assert_eq!(old.rules.owners.iter().filter(|o| *o == owner).count(), 1);
    }
    let migrated = compile_owned_release_migration(prior, m.clone(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    let row = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == overlay.owner.owner)
        .unwrap();
    assert_eq!(
        *row, overlay.prior_owner,
        "migration must not modify the guarded program"
    );
    *row = overlay.owner.clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
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
        .find(|o| o.owner == overlay.prior_owner.owner)
        .unwrap() = overlay.prior_owner.clone();
    *temporary_inverse.provenance.last_mut().unwrap() =
        migrated.input().provenance.last().unwrap().clone();
    assert!(
        temporary_inverse == *migrated.input(),
        "only exact direct guard and provenance differ from checked migration"
    );
    let mut registry = OwnedIdRegistry::new(old.registry.clone(), Default::default()).unwrap();
    let mut added = Vec::new();
    for entry in &m.schema {
        let SchemaExtensionEntry::Definition(DefinitionDescriptor::Stat(stat)) = entry else {
            panic!("only Stats allocated")
        };
        let allocated = registry
            .allocate_definition::<StatDefinition>()
            .unwrap()
            .address();
        assert_eq!(allocated, stat.id.address());
        added.push(allocated);
    }
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        old.schema.definitions.len() + 2
    );
    assert_eq!(restored.rules.owners.len(), old.rules.owners.len() + 1);
    assert_eq!(
        restored.rules.receivers.members.len(),
        old.rules.receivers.members.len() + 1
    );
    restored
        .schema
        .definitions
        .retain(|row| !added.contains(&row.address()));
    restored.rules.owners.retain(|row| match &row.owner {
        SchemaSubject::Definition(definition) => !added.contains(definition),
        SchemaSubject::Slot(_) => true,
    });
    for owner in old_owners {
        let row = restored
            .rules
            .owners
            .iter_mut()
            .find(|r| r.owner == owner.owner)
            .unwrap();
        *row = owner;
    }
    restored
        .rules
        .receivers
        .members
        .retain(|r| r.id != m.receivers[0].id);
    restored.registry = old.registry.clone();
    restored.schema.release = old.schema.release.clone();
    restored.rules.definitions = old.rules.definitions.clone();
    restored.routing.definitions = old.routing.definitions.clone();
    assert!(
        restored == *old,
        "full prior recipe inverse: only allocations, exact routing programs and one receiver"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert_endpoint(&next);
    next
}
