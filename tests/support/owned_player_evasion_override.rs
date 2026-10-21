//! Checked global-base replacement; no final-stat or equipment override.
use super::{equipment, evasion_family};
use flate2::read::GzDecoder;
use poe_optimizer_core::owned_rules::*;
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::PathBuf, sync::OnceLock};

pub const QUERY: &str = "player-evasion-global-base-override";
pub fn data() -> PathBuf {
    evasion_family::root().join("data/owned/poe2/3887ae68/player-evasion-override")
}
fn hash(b: &[u8]) -> String {
    format!("{:x}", Sha256::digest(b))
}
fn bounded(path: &std::path::Path) -> Vec<u8> {
    assert!(fs::metadata(path).unwrap().len() <= 2 * 1024 * 1024);
    fs::read(path).unwrap()
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&bounded(&data().join(name))).unwrap()
}
fn authenticated(pin: &Value) -> Vec<u8> {
    let b = bounded(&evasion_family::root().join(pin["path"].as_str().unwrap()));
    assert_eq!(b.len(), pin["bytes"]);
    assert_eq!(hash(&b), pin["sha256"]);
    b
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owner: DefinitionRules,
    pub query: ContributionQuery,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn previous_owner() -> DefinitionRules {
    serde_json::from_value(read::<Value>("dependencies.json")["previous_owner"].clone()).unwrap()
}

/// Restore only the two new reads, three new nodes and one changed edge.
pub fn check_inverse(c: &Consumer) {
    let old = previous_owner();
    assert_eq!(old, equipment::consumer().final_owner);
    let mut inverse = c.owner.clone();
    assert_eq!(inverse.programs.members.len(), 1);
    let p = &mut inverse.programs.members[0];
    let quantity = old.programs.members[0].reads[0].value_type.clone();
    let expected_reads: Vec<RuleRead> = serde_json::from_value(json!([
        {"id":"override-present","value_type":{"kind":"boolean"},"source":{"kind":"contribution_selection","value":{"entity":"current","query":QUERY,"group":"sources","projection":"present"}}},
        {"id":"override-value","value_type":quantity,"source":{"kind":"contribution_selection","value":{"entity":"current","query":QUERY,"group":"sources","projection":"value"}}}
    ])).unwrap();
    assert_eq!(p.reads.len(), old.programs.members[0].reads.len() + 2);
    assert_eq!(p.reads.split_off(p.reads.len() - 2), expected_reads);
    let at = p
        .nodes
        .iter()
        .position(|n| n.id.as_str() == "override-present")
        .unwrap();
    let expected_nodes: Vec<RuleNode> = serde_json::from_value(json!([
        {"id":"override-present","expression":{"kind":"read","input":"override-present"}},
        {"id":"override-value","expression":{"kind":"read","input":"override-value"}},
        {"id":"selected-global-base","expression":{"kind":"select","condition":"override-present","when_true":"override-value","when_false":"base"}}
    ])).unwrap();
    assert_eq!(
        p.nodes.drain(at..at + 3).collect::<Vec<_>>(),
        expected_nodes
    );
    let increased = p
        .nodes
        .iter_mut()
        .find(|n| n.id.as_str() == "increased")
        .unwrap();
    assert_eq!(
        increased.expression,
        RuleExpression::Scale {
            value: "selected-global-base".parse().unwrap(),
            factor: "increase-factor".parse().unwrap(),
        }
    );
    increased.expression = RuleExpression::Scale {
        value: "base".parse().unwrap(),
        factor: "increase-factor".parse().unwrap(),
    };
    assert!(
        inverse == old,
        "exact complete predecessor owner, including Partial and rounding guards"
    );
    let expected_query: ContributionQuery = serde_json::from_value(json!({
        "id":QUERY,"stat":evasion_family::consumer().queries[0].stat,
        "contribution":"override","groups":[{"id":"sources","reduction":"require_agreement",
        "ordering":"unordered","empty":null,"members":{"members":[],"closure":{"kind":"complete"}}}]
    }))
    .unwrap();
    assert_eq!(c.query, expected_query);
}

pub fn authenticate_source(proof: &Value) {
    evasion_family::check_source();
    let prior = evasion_family::domain();
    let bytes = authenticated(&proof["prior_source_domain"]);
    assert_eq!(
        hash(&bytes),
        "db6069d8e120c3feb2312f113162189bc64b02bcd8295d05965f650cdfcb306a"
    );
    assert_eq!(proof["source_revision"], prior["source_revision"]);
    let names = [
        "src/Modules/CalcDefence.lua",
        "src/Classes/ModDB.lua",
        "src/Classes/ModStore.lua",
    ];
    let supplied = proof["complete_bodies"].as_array().unwrap();
    assert_eq!(supplied.len(), names.len());
    let mut bodies = BTreeMap::new();
    for (pin, name) in supplied.iter().zip(names) {
        let expected: Vec<_> = prior["complete_bodies"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["source_path"] == name)
            .collect();
        assert_eq!(expected.len(), 1);
        assert_eq!(
            pin, expected[0],
            "independent complete source/manifest identity"
        );
        let compressed = authenticated(&pin["archive"]);
        let mut raw = vec![];
        GzDecoder::new(compressed.as_slice())
            .take(2 * 1024 * 1024 + 1)
            .read_to_end(&mut raw)
            .unwrap();
        assert!(raw.len() <= 2 * 1024 * 1024);
        assert_eq!(raw.len(), pin["bytes"]);
        assert_eq!(hash(&raw), pin["sha256"]);
        bodies.insert(name, String::from_utf8(raw).unwrap());
    }
    let ranges = [
        (names[0], 1342, 1348, "resource-aliases"),
        (
            names[0],
            1374,
            1443,
            "override-before-transfers-and-ordered-scaling",
        ),
        (names[0], 1445, 1458, "final-evasion-rounding"),
        (names[2], 301, 316, "override-dispatch"),
        (
            names[1],
            344,
            389,
            "override-membership-and-first-active-source",
        ),
    ];
    let spans = proof["spans"].as_array().unwrap();
    assert_eq!(spans.len(), ranges.len());
    for (span, (name, first, last, role)) in spans.iter().zip(ranges) {
        assert_eq!(span["source_path"], name);
        assert_eq!(span["first"], first);
        assert_eq!(span["last"], last);
        assert_eq!(span["role"], role);
        let body = bodies[name]
            .lines()
            .skip(first - 1)
            .take(last - first + 1)
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert_eq!(span["body"], body);
        assert_eq!(span["sha256"], hash(body.as_bytes()));
    }
    assert_eq!(
        proof["native_admissible_override_domain"],
        json!(["empty", "one-active-value", "multiple-equal-active-values"])
    );
    assert_eq!(
        proof["scope"],
        json!({"source_global_base_only":true,"zero_is_present":true,
        "source_insertion_order_emulated":false,"new_source_execution":false,"candidate_source_closure":false,"final_owner_complete":false})
    );
}
pub fn check_authored() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        equipment::check_authored();
        let a: Value = read("authoring.json");
        for pin in a["artifacts"].as_array().unwrap() { authenticated(pin); }
        let d: Value = read("dependencies.json");
        authenticated(&d["predecessor_authoring"]);
        assert_eq!(d["registry_last"], 0x33a9);
        assert_eq!(d["override_writers"], json!([]));
        check_inverse(&consumer());
        authenticate_source(&read("source-evidence.json"));
        let b: Value = read("bindings.json");
        assert_eq!(b["final_stat"], json!(evasion_family::consumer().receiver.stat));
        assert_eq!(b["global_base_stat"], json!(consumer().query.stat));
        assert_eq!(b["query"], QUERY);
        assert_eq!(b["equipment_subtotal"], json!(super::family::def::<poe_optimizer_core::owned_definitions::StatDefinition>(equipment::SUBTOTAL)));
        assert_eq!(b["unit"], json!(super::family::def::<poe_optimizer_core::owned_definitions::UnitDefinition>(evasion_family::UNIT)));
        assert_eq!(b["replacement_edge"], json!({"node":"increased","field":"value","before":"base","after":"selected-global-base"}));
        assert_eq!(b["scope"], json!({"new_definitions":0,"new_programs":0,"final_owner_complete":false,"candidate_source_closure":false,"positive_source_observations":false}));
    });
}

/// Enumerate potential writers without evaluating their value or activation.
pub fn override_writers(rules: &Value) -> Value {
    let matches = |e: &Value| {
        e["effect"]["stat"]["key"] == "def.00000000000029f1"
            && e["effect"]["contribution"] == "override"
    };
    let mut rows = vec![];
    for o in rules["owners"].as_array().unwrap() {
        for p in o["programs"]["members"].as_array().unwrap() {
            for e in p["effects"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|e| matches(e))
            {
                rows.push(json!({"owner":o["owner"],"program":p,"effect":e["id"]}));
            }
        }
    }
    for a in rules["effect_applications"]["members"].as_array().unwrap() {
        for e in a["program"]["effects"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| matches(e))
        {
            rows.push(json!({"application":a,"effect":e["id"]}));
        }
    }
    json!(rows)
}
pub fn restore_rules(rules: &RulePackageInput) -> RulePackageInput {
    let c = consumer();
    assert_eq!(
        override_writers(&json!(rules)),
        read::<Value>("dependencies.json")["override_writers"]
    );
    let mut old = rules.clone();
    let owners: Vec<_> = old
        .owners
        .iter()
        .enumerate()
        .filter(|(_, o)| o.owner == c.owner.owner)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(owners.len(), 1);
    assert!(old.owners[owners[0]] == c.owner, "exact override consumer");
    old.owners[owners[0]] = previous_owner();
    let queries = &mut old.contribution_queries.as_mut().unwrap().members;
    let rows: Vec<_> = queries
        .iter()
        .enumerate()
        .filter(|(_, q)| q.id == c.query.id)
        .map(|(i, _)| i)
        .collect();
    assert_eq!(rows.len(), 1);
    assert_eq!(queries.remove(rows[0]), c.query);
    old
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let r = restore_rules(&endpoint.input().recipe.rules);
    let _ = equipment::previous_rules(&json!(endpoint.input().recipe.schema), &json!(r));
    equipment::check_slot_bindings(
        &json!(endpoint.mapping().input()),
        &json!(endpoint.normalization()),
    );
    assert!(endpoint.evaluation().is_none());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    equipment::assert_component(prior);
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    assert_eq!(a["scope"]["publication_ready"], true);
    assert_eq!(a["before"], json!(prior.receipt().input));
    assert_eq!(d["before"], json!(prior.receipt().input));
    assert_eq!(prior.input().recipe.registry.last_issued.get(), 0x33a9);
    assert_eq!(
        override_writers(&json!(prior.input().recipe.rules)),
        d["override_writers"]
    );
    let mut input = prior.input().clone();
    let c = consumer();
    let row = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == c.owner.owner)
        .unwrap();
    assert!(*row == previous_owner(), "exact predecessor final owner");
    *row = c.owner;
    let queries = &mut input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members;
    assert!(!queries.iter().any(|q| q.id == c.query.id));
    queries.push(c.query);
    input.recipe.rules.release = serde_json::from_value(a["release"].clone()).unwrap();
    let provenance = OwnedReleaseProvenance {
        kind: "player-evasion-override".parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: poe_optimizer_core::owned_content::digest_owned(
            "player-evasion-override",
            &a,
            2 * 1024 * 1024,
        )
        .unwrap(),
    };
    input.provenance.push(provenance.clone());
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    inverse.recipe.rules = restore_rules(&inverse.recipe.rules);
    inverse.recipe.rules.release = prior.input().recipe.rules.release.clone();
    assert_eq!(inverse.provenance.pop(), Some(provenance));
    assert!(
        inverse == *prior.input(),
        "only exact final-owner/query/provenance changes"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
