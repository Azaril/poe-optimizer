//! Exact integer support for the union of the existing Player Life BASE groups.
//! This proof supplies no missing source, conversion or whole-build authority.
use poe_optimizer_core::{owned_content::*, owned_rules::*, owned_schema::*};
use poe_optimizer_data::owned_schema::SchemaPackageInput;
use poe_optimizer_engine::owned_plan::PlanLimits;
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, io::Read, path::PathBuf};

const KIND: &str = "life-base-domain";
const FILES: [&str; 4] = [
    "authoring.json",
    "dependencies.json",
    "program.json",
    "proof.json",
];
pub const BOUND: f64 = 137_438_953_472.;
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/life-base-domain")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Writer {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub operations_version: poe_optimizer_core::owned_definitions::OwnedDefinitionKey,
    pub shared_player: DefinitionRules,
    pub final_life: DefinitionRules,
    pub inherent_owner: DefinitionRules,
    pub equipment_owner: DefinitionRules,
    pub inherent_writers: Vec<Writer>,
    pub inherent_receivers: Vec<StatReceiver>,
    pub receiver_registry_closure: SchemaClosure,
    pub queries: Vec<ContributionQuery>,
    pub writers: Vec<Writer>,
    pub existing_actor_rules: Option<DeclaredSet<ExistingActorRuleApplication>>,
    pub classes: Vec<DefinitionDescriptor>,
    pub query_registry_closure: SchemaClosure,
    pub native_schema: SchemaPackageInput,
    pub native_rules: RulePackageInput,
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}
pub fn program() -> RuleProgram {
    read("program.json")
}
pub fn shared_after() -> DefinitionRules {
    let mut owner = dependencies().shared_player;
    let program = program();
    let target = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == program.id)
        .unwrap();
    *target = program;
    owner
}
pub fn final_after() -> DefinitionRules {
    let mut owner = dependencies().final_life;
    let SchemaClosure::Partial { gaps } = &mut owner.programs.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].code.as_str(),
        "unproved-life-cross-group-and-conversion-order-domain"
    );
    gaps[0].code = "unproved-life-conversion-order-domain".parse().unwrap();
    owner
}
pub fn check_transition(
    actual: &RuleProgram,
    shared: &DefinitionRules,
    final_owner: &DefinitionRules,
) {
    let d = dependencies();
    let mut inverse = actual.clone();
    let domain: NumericDomain =
        serde_json::from_value(read::<Value>("proof.json")["life_domain"].clone()).unwrap();
    assert_eq!(
        inverse.nodes.pop().unwrap(),
        RuleNode {
            id: "bounded-inherent-life".parse().unwrap(),
            expression: RuleExpression::CheckNumericDomain {
                value: "amount".parse().unwrap(),
                domain
            },
        }
    );
    assert_eq!(inverse.effects.len(), 1);
    let RuleEffectKind::Contribute { value, .. } = &mut inverse.effects[0].effect else {
        panic!()
    };
    assert_eq!(value.as_str(), "bounded-inherent-life");
    *value = "amount".parse().unwrap();
    assert_eq!(
        &inverse,
        d.shared_player
            .programs
            .members
            .iter()
            .find(|p| p.id == inverse.id)
            .unwrap()
    );
    assert_eq!(*shared, shared_after());
    assert_eq!(*final_owner, final_after());
    assert!(!shared.programs.is_complete() && !final_owner.programs.is_complete());
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 2 * 1024 * 1024).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn expanded(path: &std::path::Path) -> Vec<u8> {
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(fs::File::open(path).unwrap())
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .unwrap();
    assert!(bytes.len() <= 2 * 1024 * 1024);
    bytes
}
fn source() {
    let p: Value = read("proof.json");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = expanded(&root.join(p["source_manifest"].as_str().unwrap()));
    assert_eq!(
        hash(&manifest),
        "8ed40a4464dd9ec223fa7756381da18d02b3999b5c1d88ac73af16f48d412675"
    );
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(
        manifest["upstream_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let sources = p["source_archives"].as_array().unwrap();
    assert_eq!(sources.len(), 3);
    for (archive, (name, ranges)) in sources.iter().zip([
        ("src/Modules/CalcPerform.lua", vec![(494, 509)]),
        ("src/Modules/CalcSetup.lua", vec![(828, 837)]),
        ("src/Classes/ModDB.lua", vec![(137, 161), (185, 212)]),
    ]) {
        assert_eq!(archive["source"], name);
        let compressed = fs::read(root.join(archive["file"].as_str().unwrap())).unwrap();
        assert_eq!(archive["compressed_sha256"], hash(&compressed));
        let raw = expanded(&root.join(archive["file"].as_str().unwrap()));
        assert_eq!(archive["sha256"], hash(&raw));
        assert_eq!(archive["bytes"], raw.len());
        let entry: Vec<_> = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["path"] == name)
            .collect();
        assert_eq!(entry.len(), 1);
        assert_eq!(archive["sha256"], entry[0]["sha256"]);
        let body = String::from_utf8(raw).unwrap();
        let lines: Vec<_> = body.lines().collect();
        let spans = archive["spans"].as_array().unwrap();
        assert_eq!(spans.len(), ranges.len());
        for (span, (first, last)) in spans.iter().zip(ranges) {
            assert_eq!(span["first"], first);
            assert_eq!(span["last"], last);
            assert_eq!(span["body"], lines[first - 1..last].join("\n"));
        }
    }
    let evidence = p["prior_evidence"].as_array().unwrap();
    assert_eq!(evidence.len(), 6);
    for (entry, expected) in evidence.iter().zip([
        "strength-life/source-vectors.json",
        "player-intrinsic-life/source-vectors.json",
        "life-equipment-domain/proof.json",
        "life-equipment-domain/programs.json",
        "life-equipment-domain/dependencies.json",
        "player-life-pool/source-vectors.json",
    ]) {
        assert_eq!(entry["path"], expected);
        assert_eq!(
            entry["sha256"],
            hash(&fs::read(data().parent().unwrap().join(expected)).unwrap())
        );
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let p: Value = read("proof.json");
    let d = dependencies();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(
        a["scope"],
        json!({"changed_programs":1,"refined_gaps":1,"new_definitions":0,"new_receivers":0,"closed_owners":0,"closed_supplier_queries":0,"supplier_admission":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1).copied().chain(["ModDB.lua.gz"]) {
        let b = fs::read(data().join(file)).unwrap();
        assert_eq!(a["artifacts"][file]["bytes"], b.len());
        assert_eq!(a["artifacts"][file]["sha256"], hash(&b));
    }
    assert_eq!(PlanLimits::default().max_effects, 65_536);
    assert_eq!(p["max_effects"], 65_536);
    assert_eq!(p["component_bound"], 1u64 << 37);
    assert_eq!(p["maximum_absolute_prefix"], 1u64 << 53);
    assert_eq!(
        p["scope"],
        json!({"base_order_proved":true,"player_increase_order_proved":true,"conversion_order_proved":false,"supplier_admission":false,"whole_build":false})
    );
    assert_eq!(
        (PlanLimits::default().max_effects as u64) * (1u64 << 37),
        1u64 << 53
    );
    let NumericDomain::Quantity(range) = serde_json::from_value(p["life_domain"].clone()).unwrap()
    else {
        panic!()
    };
    assert_eq!(
        (range.minimum.value(), range.maximum.value()),
        (-BOUND, BOUND)
    );
    assert_eq!(range.minimum.unit(), range.maximum.unit());
    assert_eq!(range.minimum.unit().key().as_str(), "def.0000000000003119");
    assert_eq!(d.classes.len(), 8);
    for class in &d.classes {
        let DefinitionDescriptor::Class(row) = class else {
            panic!()
        };
        let SchemaState::Known(schema) = &row.schema else {
            panic!()
        };
        assert_eq!(json!(schema.level), json!({"minimum":1,"maximum":100}));
    }
    let base = &d.queries[0];
    assert_eq!(base.id.as_str(), "life-base-contributions");
    assert_eq!(base.groups.len(), 4);
    let mut unique = BTreeSet::new();
    for (group, (name, count)) in base.groups.iter().zip([
        ("intrinsic", 2),
        ("inherent", 2),
        ("equipment", 2),
        ("rewards", 1),
    ]) {
        assert_eq!(group.id.as_str(), name);
        assert!(group.members.is_complete());
        assert_eq!(group.members.members.len(), count);
        assert_eq!(group.reduction, ContributionReduction::Sum);
        for member in &group.members.members {
            let producer = member.producer.as_program_effect().unwrap();
            assert!(
                unique.insert(
                    serde_json::to_string(&json!([
                        producer.owner,
                        producer.program,
                        producer.effect
                    ]))
                    .unwrap()
                )
            );
            let matches: Vec<_> = d
                .writers
                .iter()
                .filter(|w| w.owner == producer.owner && w.program.id == producer.program)
                .collect();
            assert_eq!(matches.len(), 1);
            assert!(
                matches[0]
                    .program
                    .effects
                    .iter()
                    .any(|e| e.id == producer.effect)
            );
        }
    }
    assert_eq!(unique.len(), 7);
    assert_eq!(d.writers.len(), 10);
    assert!(matches!(
        d.query_registry_closure,
        SchemaClosure::Partial { .. }
    ));
    // Shared Player is one exact Actor application, not a Class-dependent copy.
    let applications = d.existing_actor_rules.as_ref().unwrap();
    assert!(applications.is_complete());
    assert_eq!(applications.members.len(), 1);
    assert_eq!(
        applications.members[0].id.as_str(),
        "shared-player-initialization"
    );
    assert_eq!(
        applications.members[0].targets,
        [ExistingActorRuleTarget::Player]
    );
    assert_eq!(
        SchemaSubject::Definition(applications.members[0].owner.address()),
        d.shared_player.owner
    );
    check_transition(&program(), &shared_after(), &final_after());
    source();
}
pub fn census(rules: &RulePackageInput, guarded: bool) {
    let d = dependencies();
    assert_eq!(rules.operations_version, d.operations_version);
    assert_eq!(rules.existing_actor_rules, d.existing_actor_rules);
    let registry = rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, d.query_registry_closure);
    for query in &d.queries {
        assert_eq!(
            registry
                .members
                .iter()
                .filter(|q| q.id == query.id)
                .collect::<Vec<_>>(),
            [query]
        );
    }
    let stat = d.queries[0].stat.clone();
    let found: Vec<_> = rules
        .owners
        .iter()
        .flat_map(|o| {
            o.programs
                .members
                .iter()
                .filter(|p| {
                    p.effects.iter().any(
                        |e| matches!(&e.effect,RuleEffectKind::Contribute {stat:s,..} if *s==stat),
                    )
                })
                .map(|p| Writer {
                    owner: o.owner.clone(),
                    program: p.clone(),
                })
        })
        .collect();
    let mut expected = d.writers;
    if guarded {
        let p = program();
        let target = expected
            .iter_mut()
            .find(|w| w.owner == d.shared_player.owner && w.program.id == p.id)
            .unwrap();
        *target = Writer {
            owner: d.shared_player.owner,
            program: p,
        };
    }
    assert_eq!(
        found, expected,
        "exact entire potential Life writer census, including inactive and non-Player effects"
    );
    for application in rules.effect_applications.iter().flat_map(|r| &r.members) {
        assert!(
            !application
                .program
                .effects
                .iter()
                .any(|e| matches!(&e.effect,RuleEffectKind::Contribute{stat:s,..} if *s==stat))
        );
    }
    let inherited = d.inherent_receivers[0].stat.clone();
    let found: Vec<_> = rules
        .owners
        .iter()
        .flat_map(|o| {
            o.programs
                .members
                .iter()
                .filter(|p| {
                    p.effects.iter().any(
                        |e| matches!(&e.effect,RuleEffectKind::Derive{stat,..} if *stat==inherited),
                    )
                })
                .map(|p| Writer {
                    owner: o.owner.clone(),
                    program: p.clone(),
                })
        })
        .collect();
    assert_eq!(found, d.inherent_writers);
    assert_eq!(rules.receivers.closure, d.receiver_registry_closure);
    assert_eq!(
        rules
            .receivers
            .members
            .iter()
            .filter(|r| r.stat == inherited)
            .collect::<Vec<_>>(),
        d.inherent_receivers.iter().collect::<Vec<_>>()
    );
    for application in rules.effect_applications.iter().flat_map(|r| &r.members) {
        assert!(
            !application
                .program
                .effects
                .iter()
                .any(|e| matches!(&e.effect,RuleEffectKind::Derive{stat,..} if *stat==inherited))
        );
    }
}
/// Restore only two authenticated component owners for older proof comparisons.
/// No release is rebuilt; runtime callers continue to use the current rules.
pub fn previous_rules(endpoint: &StagedOwnedRelease) -> RulePackageInput {
    assert_component(endpoint);
    let d = dependencies();
    let mut rules = endpoint.input().recipe.rules.clone();
    for (before, after) in [
        (d.shared_player, shared_after()),
        (d.final_life, final_after()),
    ] {
        let current = rules
            .owners
            .iter_mut()
            .find(|o| o.owner == before.owner)
            .unwrap();
        assert_eq!(*current, after);
        *current = before;
    }
    rules
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let d = dependencies();
    let recipe = &endpoint.input().recipe;
    check_component_rules(&recipe.schema, &recipe.rules);
    assert!(
        endpoint
            .receipt()
            .provenance
            .contains(&OwnedReleaseProvenance {
                kind: KIND.parse().unwrap(),
                prior_input: d.before,
                authoring_input: digest()
            })
    );
}
pub fn check_component_rules(schema: &SchemaPackageInput, rules: &RulePackageInput) {
    let d = dependencies();
    census(rules, true);
    for expected in [
        shared_after(),
        final_after(),
        d.inherent_owner,
        d.equipment_owner,
    ] {
        assert_eq!(
            rules
                .owners
                .iter()
                .filter(|o| o.owner == expected.owner)
                .collect::<Vec<_>>(),
            [&expected]
        );
    }
    assert_eq!(
        schema
            .definitions
            .iter()
            .filter(|d| matches!(d, DefinitionDescriptor::Class(_)))
            .collect::<Vec<_>>(),
        d.classes.iter().collect::<Vec<_>>()
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d = dependencies();
    assert_eq!(prior.receipt().input, d.before);
    let mut input = prior.input().clone();
    census(&input.recipe.rules, false);
    assert_eq!(
        input
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|d| matches!(d, DefinitionDescriptor::Class(_)))
            .collect::<Vec<_>>(),
        d.classes.iter().collect::<Vec<_>>()
    );
    for (before, after) in [
        (&d.shared_player, shared_after()),
        (&d.final_life, final_after()),
    ] {
        let old = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == before.owner)
            .unwrap();
        assert_eq!(old, before);
        *old = after;
    }
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: d.before,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for (before, after) in [
        (d.shared_player, shared_after()),
        (d.final_life, final_after()),
    ] {
        let current = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == before.owner)
            .unwrap();
        assert_eq!(*current, after);
        *current = before;
    }
    inverse.provenance.pop();
    assert!(
        inverse == *prior.input(),
        "one exact bridge guard and final-order gap refinement only"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert!(next.evaluation().is_none());
    next
}
