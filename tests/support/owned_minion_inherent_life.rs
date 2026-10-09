//! Exact supplied-Actor receiver reuse. Observed zero is evidence, never input.
use super::migration_preservation;
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
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "minion-inherent-strength-life";
const FILES: [&str; 7] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "receivers.json",
    "program.json",
    "query.json",
    "source-vectors.json",
];
const STATS: [&str; 8] = [
    "3321", "1d2e", "3315", "3316", "3317", "3318", "3319", "331a",
];
const FLAGS: [&str; 5] = [
    "NoAttributeBonuses",
    "NoStrengthAttributeBonuses",
    "NoStrBonusToLife",
    "DoubledInherentAttributeBonuses",
    "HalvesLifeFromStrength",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/minion-inherent-life")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = FILES.map(read).into();
    digest_owned(KIND, &values, 8 * 1024 * 1024).unwrap()
}
#[derive(Clone, Deserialize)]
pub struct Dependencies {
    pub owner_before: DefinitionRules,
    pub query_before: ContributionQuery,
    pub receiver_before: Vec<StatReceiver>,
    consumers: Vec<DefinitionRules>,
    queries: Vec<ContributionQuery>,
    donors: Vec<DefinitionRules>,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
    query_registry_closure: SchemaClosure,
    receiver_registry_closure: SchemaClosure,
    existing_actor_rules: Option<DeclaredSet<ExistingActorRuleApplication>>,
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}
pub fn receivers() -> Vec<StatReceiver> {
    read("receivers.json")
}
pub fn program() -> RuleProgram {
    read("program.json")
}
pub fn query() -> ContributionQuery {
    read("query.json")
}
pub fn owner_after() -> DefinitionRules {
    let mut owner = dependencies().owner_before;
    owner.programs.members.push(program());
    owner
}
/// Check the complete delta, not a subset accepting another recipient or law.
pub fn check_changes(receivers: &[StatReceiver], bridge: &RuleProgram, after: &ContributionQuery) {
    let b: Value = read("bindings.json");
    let d = dependencies();
    assert_eq!(json!(d.owner_before.owner), b["owner"]);
    assert_eq!(
        b["slot"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    assert_eq!(b["slot"]["slot"]["key"], "def.000000000000001f");
    assert!(!d.owner_before.programs.is_complete());
    assert!(
        d.owner_before
            .programs
            .members
            .iter()
            .all(|p| p.id != bridge.id)
    );
    let original: RuleProgram = serde_json::from_slice(
        &fs::read(root().join("data/owned/poe2/3887ae68/inherent-life-contribution/program.json"))
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        bridge, &original,
        "same guarded Current-Actor bridge, no zero literal"
    );
    assert_eq!(receivers.len(), 8);
    assert_eq!(d.receiver_before.len(), 8);
    let mut ids = BTreeSet::new();
    for ((actual, old), stat) in receivers.iter().zip(&d.receiver_before).zip(STATS) {
        assert_eq!(old.targets, vec![StatReceiverTarget::Player]);
        assert_eq!(json!(old.stat)["key"], format!("def.{stat:0>16}"));
        let mut expected = old.clone();
        expected.targets.push(StatReceiverTarget::OwnedSlot {
            slot: serde_json::from_value(b["slot"].clone()).unwrap(),
        });
        assert_eq!(actual, &expected);
        assert!(ids.insert(actual.id.clone()));
        let owners: Vec<_> = d
            .consumers
            .iter()
            .filter(|o| json!(o.owner)["value"]["value"] == json!(actual.stat))
            .collect();
        assert_eq!(owners.len(), 1);
        assert!(owners[0].programs.is_complete());
        assert_eq!(
            owners[0]
                .programs
                .members
                .iter()
                .filter(|p| p.id == actual.program)
                .count(),
            1
        );
    }
    assert_eq!(d.consumers.len(), 8);
    assert_eq!(d.queries.len(), 11);
    // Freeze the complete input query rows and all their possible donor bodies.
    // This does not equate observed absence with general source completeness.
    let mut members = BTreeSet::new();
    for q in &d.queries {
        for g in &q.groups {
            assert!(g.members.is_complete());
            for m in &g.members.members {
                let p = m.producer.as_program_effect().unwrap();
                members.insert(serde_json::to_string(&p.owner).unwrap());
                let donor = d.donors.iter().find(|o| o.owner == p.owner).unwrap();
                let program = donor
                    .programs
                    .members
                    .iter()
                    .find(|r| r.id == p.program)
                    .unwrap();
                let effect = program.effects.iter().find(|r| r.id == p.effect).unwrap();
                assert!(
                    matches!(&effect.effect, RuleEffectKind::Contribute {entity:RuleEntity::Player, stat, contribution,..}
                    if *stat == q.stat && *contribution == q.contribution)
                );
            }
        }
    }
    assert_eq!(members.len(), d.donors.len());
    assert!(!matches!(d.query_registry_closure, SchemaClosure::Complete));
    let mut expected = d.query_before;
    assert_eq!(expected.id.as_str(), "life-base-contributions");
    let inherent = expected
        .groups
        .iter_mut()
        .find(|g| g.id.as_str() == "inherent")
        .unwrap();
    assert!(inherent.members.is_complete());
    assert_eq!(inherent.members.members.len(), 1);
    let mut member = json!(inherent.members.members[0]);
    assert_eq!(member["producer"]["program"], json!(bridge.id));
    assert_eq!(member["producer"]["effect"], "inherent-life");
    assert_eq!(
        member["order"],
        json!({"source_rank":0,"program_rank":0,"effect_rank":0,"slot_ranks":[]})
    );
    member["producer"]["owner"] = b["owner"].clone();
    member["producer"]["origin"] = json!({"kind":"supplied_actor","slots":[b["slot"]]});
    inherent
        .members
        .members
        .push(serde_json::from_value(member).unwrap());
    assert_eq!(
        after, &expected,
        "exactly one recipient-disjoint inherent source is appended"
    );
}
fn read_pin(pin: &Value) -> Vec<u8> {
    let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], hash(&bytes));
    bytes
}
fn empty(v: &Value) {
    assert!(
        v.as_object().is_some_and(|v| v.is_empty()),
        "explicit source empty table"
    );
}
fn check_inputs(inputs: &Value) {
    assert_eq!(inputs["strength"], json!({"present":true,"value":0}));
    empty(&inputs["raw_attributes"]);
    let flags = inputs["flags"].as_array().unwrap();
    assert_eq!(flags.len(), FLAGS.len());
    for (flag, name) in flags.iter().zip(FLAGS) {
        assert_eq!(flag["name"], name);
        assert_eq!(flag["present"], false);
        assert!(flag.get("value").is_none());
        empty(&flag["raw"]);
        empty(&flag["eligible"]);
    }
}
/// Relevant actual source provenance; zero does not become native input data.
pub fn check_source_projection(p: &Value) {
    assert_eq!(p["case_count"], 7);
    assert_eq!(p["complete_load_attempts_per_jit"], 16);
    assert_eq!(p["capture"]["observer_jit_enabled"], false);
    assert_eq!(p["capture"]["unhooked_controls"], true);
    let cases = p["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 7);
    for (i, c) in cases.iter().enumerate() {
        assert_eq!(c["state"]["benefit_snapshot"], c["unhooked"]);
        for mode in ["main", "calcs"] {
            let actors = c["state"][mode]["actors"].as_array().unwrap();
            assert_eq!(actors.len(), 1);
            let actor = &actors[0];
            assert_eq!(actor["summon_effect_id"], "SummonSkeletalSnipersPlayer");
            assert_eq!(actor["actor_profile"], "RaisedSkeletonSniper");
            let observed = mode == "main" || i == 6;
            assert_eq!(actor["is_environment_minion"], observed);
            let life = &actor["life_adjustments"];
            assert_eq!(life["exact_parent"], true);
            assert_eq!(life["exact_summoner"], true);
            if !observed {
                empty(&life["strength_insertions"]);
                empty(&life["original_life_calls"]);
                assert!(life.get("actor_output_life").is_none());
                continue;
            }
            let insertions = life["strength_insertions"].as_array().unwrap();
            assert_eq!(insertions.len(), 1);
            let source = &insertions[0];
            for field in [
                "exact_actor_store",
                "exact_summoner",
                "original_record_preserved",
                "selected",
            ] {
                assert_eq!(source[field], true);
            }
            assert_eq!(source["store_is_player"], false);
            assert_eq!(source["stored_identity_count"], 1);
            assert_eq!(source["index"], 1);
            assert_eq!(source["source"], actor["source_occurrence"]);
            assert_eq!(source["caller_source"], "Modules/CalcPerform.lua");
            assert_eq!(source["original_function_line"], 264);
            assert_eq!(source["caller_line"], 506);
            assert_eq!(source["inherent_attribute_multiplier"], 1);
            check_inputs(&source["inputs"]);
            assert_eq!(
                source["record"],
                json!({"name":"Life","type":"BASE","source":"Strength","value":0,"flags":0,"keyword_flags":0,"tags":{}})
            );
            let calls = life["original_life_calls"].as_array().unwrap();
            assert_eq!(calls.len(), 3);
            for (call, line) in calls.iter().zip([998, 1189, 1631]) {
                assert_eq!(call["caller_line"], line);
                for field in [
                    "exact_actor_store",
                    "exact_actor_output",
                    "summoner_actor_is_parent",
                    "summoner_owns_actor",
                ] {
                    assert_eq!(call[field], true);
                }
                assert_eq!(call["source"], source["source"]);
                let adjustment = &call["computation"]["adjustments"];
                check_inputs(&adjustment["inherent_inputs"]);
                let records = adjustment["strength_records"].as_array().unwrap();
                assert_eq!(records.len(), 1);
                assert_eq!(records[0]["record"], source["record"]);
                assert_eq!(records[0]["original_insertion_observed"], true);
                assert_eq!(records[0]["insertion_index"], 1);
                assert_eq!(records[0]["tabulated_base_occurrences"], 0);
            }
        }
    }
}
pub fn checked_source() -> Value {
    check_source(false)
}
pub fn check_source(full: bool) -> Value {
    let v: Value = read("source-vectors.json");
    assert_eq!(v["status"], "passed");
    assert_eq!(v["injected_observations"], false);
    assert_eq!(v["original_attribute_pass_parity"], false);
    assert_eq!(v["conversion_source_admission"], false);
    assert_eq!(v["final_life"], false);
    let p: Value = serde_json::from_slice(&read_pin(&v["source_packet"])).unwrap();
    read_pin(&v["bridge_packet"]);
    assert_eq!(p["status"], "passed");
    assert_eq!(p["projection_sha256"], v["projection_sha256"]);
    assert_eq!(
        p["projection_sha256"],
        hash(&serde_json::to_vec(&p["projection"]).unwrap())
    );
    assert_eq!(p["reports"], v["reports"]);
    let projection = &p["projection"];
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(projection["manifest_sha256"], hash(&manifest));
    assert_eq!(
        projection["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    let mut archive = p["archived_observer"].clone();
    archive["path"] = json!(format!(
        "data/owned/poe2/3887ae68/minion-life-adjustments/{}",
        archive["path"].as_str().unwrap()
    ));
    read_pin(&archive);
    assert_eq!(archive["sha256"], projection["observer_sha256"]);
    check_source_projection(projection);
    if full {
        let reports = v["reports"].as_array().unwrap();
        assert_eq!(reports.len(), 2);
        let mut previous = None;
        for report in reports {
            let bytes = read_pin(report);
            if let Some(prior) = &previous {
                assert_eq!(&bytes, prior);
            }
            let full: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(full["cases"].as_array().unwrap().len(), 7);
            for (saved, actual) in projection["cases"]
                .as_array()
                .unwrap()
                .iter()
                .zip(full["cases"].as_array().unwrap())
            {
                for field in ["name", "xml_sha256", "warm_xml_sha256"] {
                    assert_eq!(saved[field], actual[field]);
                }
                for mode in ["main", "calcs"] {
                    let actual = actual["state"][mode]["actors"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .find(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                        .unwrap();
                    let saved = &saved["state"][mode]["actors"][0];
                    for field in [
                        "life_adjustments",
                        "source_occurrence",
                        "summon_effect_id",
                        "is_environment_minion",
                    ] {
                        assert_eq!(saved[field], actual[field]);
                    }
                }
            }
            previous = Some(bytes);
        }
    }
    p
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
    check_changes(&receivers(), &program(), &query());
    check_source(false);
}
fn check_dependencies(endpoint: &StagedOwnedRelease, after: bool) {
    let d = dependencies();
    let recipe = &endpoint.input().recipe;
    let rules = &recipe.rules;
    let expected = if after {
        owner_after()
    } else {
        d.owner_before.clone()
    };
    assert_eq!(
        rules
            .owners
            .iter()
            .filter(|o| o.owner == expected.owner)
            .collect::<Vec<_>>(),
        vec![&expected]
    );
    for row in d.consumers.iter().chain(&d.donors) {
        assert_eq!(
            rules
                .owners
                .iter()
                .filter(|o| o.owner == row.owner)
                .collect::<Vec<_>>(),
            vec![row]
        );
    }
    for row in &d.definitions {
        assert!(recipe.schema.definitions.contains(row));
    }
    for row in &d.slots {
        assert!(recipe.schema.slots.contains(row));
    }
    assert_eq!(rules.existing_actor_rules, d.existing_actor_rules);
    assert_eq!(rules.receivers.closure, d.receiver_registry_closure);
    let expected_receivers = if after {
        receivers()
    } else {
        d.receiver_before.clone()
    };
    for row in expected_receivers {
        let actual: Vec<_> = rules
            .receivers
            .members
            .iter()
            .filter(|r| r.id == row.id)
            .collect();
        assert_eq!(actual, vec![&row]);
    }
    let registry = rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, d.query_registry_closure);
    assert!(!registry.is_complete());
    let expected = if after { query() } else { d.query_before };
    for row in d.queries.iter().chain([&expected]) {
        assert_eq!(
            registry
                .members
                .iter()
                .filter(|q| q.id == row.id)
                .collect::<Vec<_>>(),
            vec![row]
        );
    }
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    check_dependencies(endpoint, true);
    let b: Value = read("bindings.json");
    assert_eq!(
        endpoint
            .receipt()
            .provenance
            .iter()
            .filter(|p| p.kind.as_str() == KIND
                && json!(p.prior_input) == b["before"]
                && p.authoring_input == digest())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    check_dependencies(prior, false);
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
    let d = dependencies();
    let mut input = prior.input().clone();
    *input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owner_before.owner)
        .unwrap() = owner_after();
    for row in receivers() {
        let current = input
            .recipe
            .rules
            .receivers
            .members
            .iter_mut()
            .find(|r| r.id == row.id)
            .unwrap();
        assert!(d.receiver_before.contains(current));
        *current = row;
    }
    *input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == d.query_before.id)
        .unwrap() = query();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
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
        .find(|o| o.owner == d.owner_before.owner)
        .unwrap();
    assert_eq!(*owner, owner_after());
    *owner = d.owner_before;
    for (old, after) in d.receiver_before.into_iter().zip(receivers()) {
        let current = inverse
            .recipe
            .rules
            .receivers
            .members
            .iter_mut()
            .find(|r| r.id == old.id)
            .unwrap();
        assert_eq!(*current, after);
        *current = old;
    }
    let current = inverse
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == d.query_before.id)
        .unwrap();
    assert_eq!(*current, query());
    *current = d.query_before;
    let provenance = inverse.provenance.pop().unwrap();
    assert_eq!(provenance.kind, key(KIND));
    assert_eq!(
        inverse,
        *prior.input(),
        "only eight exact receiver target additions, one bridge/member and provenance change"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
