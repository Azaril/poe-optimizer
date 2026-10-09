//! Existing Player actor emits the derived inherent amount exactly once when
//! enabled. The pure receiver, all other owners and whole-build gates remain.
use super::attribute_flag_family;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "inherent-life-contribution";
const FILES: [&str; 5] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "program.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/inherent-life-contribution")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = FILES.map(read).into();
    digest_owned(KIND, &values, 1024 * 1024).unwrap()
}
#[derive(Deserialize)]
struct Dependencies {
    owner_before: DefinitionRules,
    consumer: DefinitionRules,
    receiver: StatReceiver,
    definitions: Vec<DefinitionDescriptor>,
    existing_actor_rules: DeclaredSet<ExistingActorRuleApplication>,
    query_registry_closure: SchemaClosure,
}
pub fn program() -> RuleProgram {
    read("program.json")
}
pub fn owner_before() -> DefinitionRules {
    read::<Dependencies>("dependencies.json").owner_before
}
fn owner_after() -> DefinitionRules {
    let mut owner = owner_before();
    owner.programs.members.push(program());
    owner
}
fn expected_program(b: &Value) -> Value {
    let names = ["all-disabled", "strength-disabled", "life-disabled"];
    let mut reads = vec![
        json!({"id":"amount","value_type":{"kind":"quantity","value":{"unit":b["unit"]}},"source":{"kind":"stat","value":{"entity":"current","stat":b["amount"]}}}),
    ];
    for (i, name) in names.iter().enumerate() {
        reads.push(json!({"id":name,"value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["disabled_flags"][i]}}}));
    }
    let mut nodes: Vec<_> = reads
        .iter()
        .map(|r| json!({"id":r["id"],"expression":{"kind":"read","input":r["id"]}}))
        .collect();
    nodes.extend([
        json!({"id":"disabled","expression":{"kind":"any","values":names}}),
        json!({"id":"enabled","expression":{"kind":"not","value":"disabled"}}),
    ]);
    json!({"id":b["program"],"context":"actor","reads":reads,"nodes":nodes,"effects":[{"id":b["effect"],"when":"enabled","effect":{"kind":"contribute","entity":"current","stat":b["life"],"contribution":"add","value":"amount"}}]})
}
fn source(full: bool) -> Value {
    let v: Value = read("source-vectors.json");
    assert_eq!(v["status"], "passed");
    let pins = v["source_packets"].as_array().unwrap();
    assert_eq!(pins.len(), 2);
    assert_ne!(pins[0]["path"], pins[1]["path"]);
    for pin in pins {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let proof = attribute_flag_family::authenticate_source(full);
    assert_eq!(v["source_reports"], proof["reports"]);
    assert_eq!(v["emission_cases"], proof["native_cases"]);
    let cases = proof["native_cases"].as_array().unwrap();
    assert_eq!(cases.len(), 13);
    let mut disabled_count = 0;
    let mut enabled_zero = 0;
    for (case, projection) in cases.iter().zip(proof["projections"].as_array().unwrap()) {
        assert_eq!(case["name"], projection["name"]);
        let disabled = [
            "NoAttributeBonuses",
            "NoStrengthAttributeBonuses",
            "NoStrBonusToLife",
        ]
        .iter()
        .any(|name| case["flags"][name] == true);
        assert_eq!(case["emitted_count"], u64::from(!disabled));
        if disabled {
            disabled_count += 1;
            assert_eq!(case["inherent_life"], 0);
        }
        if !disabled && case["strength"] == 0 {
            enabled_zero += 1;
            assert_eq!(case["inherent_life"], 0);
        }
        for mode in ["MAIN", "CALCS"] {
            let observed = &projection["state"]["modes"][mode];
            assert_eq!(
                observed["provenance"]["emitted_count"],
                case["emitted_count"]
            );
            // The legacy evidence representation uses {} for an empty source
            // table. It is inspected only here, never admitted to owned data.
            if disabled {
                assert!(observed["records"].as_object().unwrap().is_empty());
            } else {
                let records = observed["records"].as_array().unwrap();
                assert_eq!(records.len(), 1);
                assert_eq!(records[0]["source"], "Strength");
                assert_eq!(records[0]["name"], "Life");
                assert_eq!(records[0]["type"], "BASE");
                assert_eq!(records[0]["value"], case["inherent_life"]);
                assert_eq!(observed["provenance"]["exact_emitted_record"], true);
            }
        }
    }
    assert_eq!((disabled_count, enabled_zero), (3, 1));
    proof
}
pub fn checked_source() -> Value {
    source(false)
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["scope"], b["scope"]);
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(a["artifacts"][file]["sha256"], hash(&bytes));
    }
    assert_eq!(b["actor"]["key"], "def.000000000000332a");
    assert_eq!(b["amount"]["key"], "def.000000000000331a");
    assert_eq!(b["life"]["key"], "def.000000000000311a");
    assert_eq!(b["unit"]["key"], "def.0000000000003119");
    assert_eq!(b["disabled_flags"].as_array().unwrap().len(), 3);
    for i in 0..3 {
        assert_eq!(
            b["disabled_flags"][i]["key"],
            format!("def.{:016x}", 0x3315 + i)
        );
    }
    assert_eq!(
        json!(program()),
        expected_program(&b),
        "one guarded typed read-to-contribution bridge, without numerical literals"
    );
    let d: Dependencies = read("dependencies.json");
    assert_eq!(
        json!(d.owner_before.owner),
        json!({"kind":"definition","value":{"kind":"actor","value":b["actor"]}})
    );
    assert!(!d.owner_before.programs.is_complete());
    assert!(
        d.owner_before
            .programs
            .members
            .iter()
            .all(|p| p.id != program().id)
    );
    assert_eq!(d.definitions.len(), 7);
    assert_eq!(d.existing_actor_rules.members.len(), 1);
    assert!(d.existing_actor_rules.is_complete());
    let binding = &d.existing_actor_rules.members[0];
    assert_eq!(json!(binding.owner), b["actor"]);
    assert_eq!(binding.targets, vec![ExistingActorRuleTarget::Player]);
    assert!(!matches!(d.query_registry_closure, SchemaClosure::Complete));
    assert_eq!(json!(d.receiver.stat), b["amount"]);
    assert_eq!(d.receiver.targets, vec![StatReceiverTarget::Player]);
    assert!(d.consumer.programs.is_complete());
    assert_eq!(d.consumer.programs.members.len(), 1);
    assert_eq!(d.consumer.programs.members[0].effects.len(), 1);
    assert!(matches!(
        d.consumer.programs.members[0].effects[0].effect,
        RuleEffectKind::Derive { .. }
    ));
    source(false);
}
fn dependencies(endpoint: &StagedOwnedRelease, after: bool, receivers: &[StatReceiver]) {
    let d: Dependencies = read("dependencies.json");
    let rules = &endpoint.input().recipe.rules;
    let owner = if after { owner_after() } else { d.owner_before };
    let actual = rules
        .owners
        .iter()
        .find(|o| o.owner == owner.owner)
        .unwrap();
    if after {
        assert_eq!(actual.programs.closure, owner.programs.closure);
        for p in &owner.programs.members {
            assert_eq!(
                actual.programs.members.iter().find(|a| a.id == p.id),
                Some(p)
            );
        }
    } else {
        assert_eq!(actual, &owner, "exact offline predecessor");
    }
    assert_eq!(rules.owners.iter().filter(|o| **o == d.consumer).count(), 1);
    assert_eq!(receivers.iter().filter(|r| **r == d.receiver).count(), 1);
    assert_eq!(rules.existing_actor_rules, Some(d.existing_actor_rules));
    assert_eq!(
        rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    for descriptor in d.definitions {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|r| **r == descriptor)
                .count(),
            1
        );
    }
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    attribute_flag_family::assert_component(endpoint);
    assert_component_receivers(endpoint, &endpoint.input().recipe.rules.receivers.members);
}

#[allow(dead_code)]
pub fn assert_component_with_reviewed_receivers(
    endpoint: &StagedOwnedRelease,
    before: &[StatReceiver],
    after: &[StatReceiver],
) {
    attribute_flag_family::assert_component_with_reviewed_receivers(endpoint, before, after);
    let restored = attribute_flag_family::reviewed_receiver_inverse(endpoint, before, after);
    assert_component_receivers(endpoint, &restored);
}

fn assert_component_receivers(endpoint: &StagedOwnedRelease, receivers: &[StatReceiver]) {
    check_authored();
    dependencies(endpoint, true, receivers);
    assert!(
        endpoint
            .receipt()
            .provenance
            .iter()
            .any(|p| p.kind.as_str() == KIND && p.authoring_input == digest())
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source(true);
    attribute_flag_family::assert_component(prior);
    dependencies(prior, false, &prior.input().recipe.rules.receivers.members);
    let b: Value = read("bindings.json");
    let receipt = json!(prior.receipt());
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(
            b[field],
            receipt[if field == "before" { "input" } else { field }]
        );
    }
    let before = owner_before();
    let mut input = prior.input().clone();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == before.owner)
        .unwrap();
    assert_eq!(*owner, before);
    owner.programs.members.push(program());
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    let owner = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == before.owner)
        .unwrap();
    assert_eq!(owner.programs.members.pop().unwrap(), program());
    assert_eq!(*owner, before);
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "only one Actor program and provenance are appended"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[test]
fn source_distinguishes_disabled_absence_from_enabled_zero() {
    let proof = checked_source();
    let cases = proof["native_cases"].as_array().unwrap();
    let zero = cases
        .iter()
        .find(|c| c["name"] == "original-05-zero-strength")
        .unwrap();
    let disabled = cases
        .iter()
        .find(|c| c["name"] == "original-05-disable-strength-life")
        .unwrap();
    assert_eq!(zero["inherent_life"], disabled["inherent_life"]);
    assert_ne!(zero["emitted_count"], disabled["emitted_count"]);
    let mut unguarded = program();
    unguarded.effects[0].when = None;
    assert_ne!(json!(unguarded), expected_program(&read("bindings.json")));
}
