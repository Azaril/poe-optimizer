//! A supported empty incoming domain, enforced by candidate binding before gates.
//! It does not define any nonempty MORE composition or close other coverage.
use super::step;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
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

pub const KIND: &str = "attribute-empty-more-domain";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/attribute-empty-more")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
pub fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "producers.json",
        "queries.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(KIND, &values, 1024 * 1024).unwrap()
}
#[derive(Clone, Deserialize)]
pub struct OwnerReplacement {
    pub before: DefinitionRules,
    pub after: DefinitionRules,
}
#[derive(Clone, Deserialize)]
pub struct Producers {
    pub schema_version: u32,
    pub owners: Vec<OwnerReplacement>,
    pub receivers: Vec<StatReceiver>,
}
fn expected_program(stage: &Value, unit: UnitDefId) -> RuleProgram {
    RuleProgram {
        id: decode(&stage["program"]),
        context: RuleEntityKind::Actor,
        reads: vec![RuleRead {
            id: key("incoming"),
            value_type: ComputedValueType::Quantity { unit },
            source: RuleReadSource::ContributionQuery {
                entity: RuleEntity::Current,
                query: decode(&stage["query"]),
                group: decode(&stage["group"]),
            },
        }],
        nodes: vec![RuleNode {
            id: key("factor"),
            expression: RuleExpression::Read {
                input: key("incoming"),
            },
        }],
        effects: vec![RuleEffect {
            id: key("factor"),
            when: None,
            effect: RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: decode(&stage["effective_more"]),
                value: key("factor"),
            },
        }],
    }
}
fn evidence(full: bool) {
    let v: Value = read("source-vectors.json");
    let old: Value = step::read("source-vectors.json");
    let pin = &v["inherited_source_vectors"];
    assert_eq!(
        pin["path"],
        "data/owned/poe2/3887ae68/attribute-step-consumers/source-vectors.json"
    );
    let bytes = fs::read(step::data().join("source-vectors.json")).unwrap();
    assert_eq!(pin["bytes"], bytes.len());
    assert_eq!(pin["sha256"], hash(&bytes));
    assert_eq!(v["source_reports"], old["source_reports"]);
    assert_eq!(v["cases"].as_array().unwrap().len(), 3);
    for (actual, old) in v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(old["cases"].as_array().unwrap())
    {
        assert_eq!(actual["name"], old["name"]);
        assert_eq!(actual["xml_sha256"], old["xml_sha256"]);
        for mode in ["MAIN", "CALCS"] {
            let projected: Vec<_> = old["modes"][mode]["stages"].as_array().unwrap().iter().map(|s| {
                let records: Vec<_> = s["records"].as_array().unwrap().iter().filter(|r| r["type"] == "MORE").cloned().collect();
                let queries: Vec<_> = s["queries"].as_array().unwrap().iter().filter(|q| q["contribution"] == "MORE").collect();
                assert_eq!(queries.len(), 1);
                assert!(records.is_empty());
                assert_eq!(queries[0]["result"], 1);
                assert_eq!(queries[0]["actual_return_local"], true);
                assert_eq!(s["depth"], 0);
                // The original ModStore constructor stores false for no parent;
                // retain that exact witness distinction from a missing field.
                // This source diagnostic is not a native Boolean/default rule.
                assert_eq!(s["parent_kind"], "false");
                json!({"index":s["index"],"pass":s["pass"],"stat":s["stat"],"value":s["value"],
                    "depth":s["depth"],"parent_kind":s["parent_kind"],"multiply_records":records,"actual_query":queries[0]})
            }).collect();
            assert_eq!(json!(projected), actual["modes"][mode]);
        }
    }
    for field in [
        "gamewide_absence_claim",
        "nonempty_grouping_claim",
        "full_build_claim",
    ] {
        assert_eq!(v[field], false);
    }
    // Reuse the original-call authentication, fresh/JIT digest comparison and
    // complete projection proof; no second observer or copied oracle formula.
    step::source(full);
}
pub fn check_authored() {
    step::check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let p: Producers = read("producers.json");
    let q: Vec<ContributionQuery> = read("queries.json");
    let old: step::Consumers = step::read("consumers.json");
    let stages = b["stages"].as_array().unwrap();
    let old_b: Value = step::read("bindings.json");
    assert_eq!(b["factor_unit"], old_b["factor_unit"]);
    assert_eq!(p.schema_version, 1);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["new_definitions"], 0);
    assert_eq!(a["new_receivers"], 6);
    assert_eq!(a["new_queries"], 6);
    assert_eq!(a["closed_existing_rule_owners"], 6);
    assert_eq!(a["global_registry_closure_changed"], false);
    assert_eq!(a["nonempty_grouping_claim"], false);
    assert_eq!(a["full_build_claim"], false);
    assert_eq!(stages.len(), 6);
    assert_eq!(p.owners.len(), 6);
    assert_eq!(p.receivers.len(), 6);
    assert_eq!(q.len(), 6);
    let unit: UnitDefId = decode(&b["factor_unit"]);
    for (i, stage) in stages.iter().enumerate() {
        for field in ["id", "input", "output", "effective_more"] {
            assert_eq!(stage[field], old_b["stages"][i][field]);
        }
        let owner =
            SchemaSubject::Definition(decode::<StatDefId>(&stage["effective_more"]).address());
        let before = old.owners.iter().find(|o| o.owner == owner).unwrap();
        assert_eq!(&p.owners[i].before, before);
        assert!(!before.programs.is_complete() && before.programs.members.is_empty());
        assert_eq!(
            p.owners[i].after,
            DefinitionRules {
                owner,
                programs: DeclaredSet::complete(vec![expected_program(stage, unit.clone())]),
            }
        );
        assert_eq!(
            p.receivers[i],
            StatReceiver {
                id: decode(&stage["receiver"]),
                stat: decode(&stage["effective_more"]),
                program: decode(&stage["program"]),
                targets: vec![StatReceiverTarget::Player],
            }
        );
        assert_eq!(
            q[i],
            ContributionQuery {
                id: decode(&stage["query"]),
                stat: decode(&stage["input"]),
                contribution: ContributionKind::Multiply,
                groups: vec![ContributionGroup {
                    ordering: ContributionOrdering::Ordered,
                    id: key("empty"),
                    reduction: ContributionReduction::Product,
                    empty: Some(ParameterValue::Quantity(
                        FiniteQuantity::new(1., unit.clone()).unwrap()
                    )),
                    members: DeclaredSet::complete(vec![]),
                }],
            }
        );
    }
    assert_eq!(a["artifacts"].as_object().unwrap().len(), 4);
    for name in [
        "bindings.json",
        "producers.json",
        "queries.json",
        "source-vectors.json",
    ] {
        let pin = &a["artifacts"][name];
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    evidence(false);
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    let p: Producers = read("producers.json");
    let q: Vec<ContributionQuery> = read("queries.json");
    let mut expected: DeclaredSet<ContributionQuery> = step::read("queries.json");
    assert!(!expected.is_complete());
    expected.members.extend(q);
    let rules = &next.input().recipe.rules;
    assert_eq!(rules.operations_version.as_str(), OWNED_RULE_OPERATIONS_V21);
    assert_eq!(rules.contribution_queries.as_ref(), Some(&expected));
    for row in p.owners {
        assert_eq!(rules.owners.iter().filter(|o| **o == row.after).count(), 1);
    }
    for receiver in p.receivers {
        assert_eq!(
            rules
                .receivers
                .members
                .iter()
                .filter(|r| **r == receiver)
                .count(),
            1
        );
    }
    let old: step::Consumers = step::read("consumers.json");
    for owner in old.owners.iter().filter(|o| o.programs.is_complete()) {
        assert_eq!(rules.owners.iter().filter(|o| *o == owner).count(), 1);
    }
    for receiver in old.receivers {
        assert_eq!(
            rules
                .receivers
                .members
                .iter()
                .filter(|r| **r == receiver)
                .count(),
            1
        );
    }
    let b: Value = read("bindings.json");
    let proof = next.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), b["before"]);
    assert_eq!(proof.authoring_input, digest());
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    evidence(true);
    step::assert_endpoint(prior);
    let b: Value = read("bindings.json");
    for (field, actual) in [
        ("before", json!(prior.receipt().input)),
        ("definitions", json!(prior.receipt().definitions)),
        ("registry", json!(prior.receipt().registry)),
        ("rules", json!(prior.receipt().rules)),
    ] {
        assert_eq!(actual, b[field]);
    }
    assert!(prior.evaluation().is_none());
    let p: Producers = read("producers.json");
    let q: Vec<ContributionQuery> = read("queries.json");
    let mut input = prior.input().clone();
    for row in &p.owners {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.before.owner)
            .unwrap();
        assert_eq!(*owner, row.before);
        *owner = row.after.clone();
    }
    for r in &p.receivers {
        assert!(
            !input
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .any(|v| v.id == r.id || v.stat == r.stat)
        );
    }
    input
        .recipe
        .rules
        .receivers
        .members
        .extend(p.receivers.clone());
    let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
    assert!(
        q.iter()
            .all(|q| !registry.members.iter().any(|v| v.id == q.id))
    );
    registry.members.extend(q.clone());
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    for row in &p.owners {
        let owner = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == row.after.owner)
            .unwrap();
        assert_eq!(*owner, row.after);
        *owner = row.before.clone();
    }
    for r in &p.receivers {
        let i = inverse
            .recipe
            .rules
            .receivers
            .members
            .iter()
            .position(|v| v == r)
            .unwrap();
        inverse.recipe.rules.receivers.members.remove(i);
    }
    for q in &q {
        let members = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let i = members.iter().position(|v| v == q).unwrap();
        members.remove(i);
    }
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "exact whole-release inverse; no unrelated closure or input change"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
