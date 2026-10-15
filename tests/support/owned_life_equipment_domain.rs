//! Checked numeric support for one existing equipment contribution group.
//! This changes neither supplier completeness nor the upstream source rules.
use poe_optimizer_core::{
    owned_content::*, owned_definitions::OwnedDefinitionKey, owned_rules::*, owned_schema::*,
};
use poe_optimizer_engine::owned_plan::PlanLimits;
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const KIND: &str = "life-equipment-domain";
const FILES: [&str; 5] = [
    "authoring.json",
    "dependencies.json",
    "programs.json",
    "query.json",
    "proof.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/life-equipment-domain")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LifeWriter {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    pub before: OwnedContentDigest,
    pub operations_version: OwnedDefinitionKey,
    pub owner: DefinitionRules,
    pub query: ContributionQuery,
    pub potential_writers: Vec<LifeWriter>,
}
pub fn dependencies() -> Dependencies {
    read("dependencies.json")
}
pub fn programs() -> Vec<RuleProgram> {
    read("programs.json")
}
pub fn query() -> ContributionQuery {
    read("query.json")
}
fn guard(id: &str, value: &str, domain: NumericDomain) -> RuleNode {
    RuleNode {
        id: id.parse().unwrap(),
        expression: RuleExpression::CheckNumericDomain {
            value: value.parse().unwrap(),
            domain,
        },
    }
}
pub fn check_transition(programs: &[RuleProgram], query: &ContributionQuery) {
    let d = dependencies();
    let proof: Value = read("proof.json");
    let life: NumericDomain = serde_json::from_value(proof["life_domain"].clone()).unwrap();
    let count: NumericDomain = serde_json::from_value(proof["count_domain"].clone()).unwrap();
    assert_eq!(programs.len(), 2);
    for (actual, id) in programs
        .iter()
        .zip(["contribute-player-flat-life", "amulet-copy-flat-life"])
    {
        assert_eq!(actual.id.as_str(), id);
        let old = d
            .owner
            .programs
            .members
            .iter()
            .find(|p| p.id == actual.id)
            .unwrap();
        let mut inverse = actual.clone();
        assert_eq!(
            inverse.nodes.pop().unwrap(),
            guard("bounded-life-points", "life-points", life.clone())
        );
        if id == "amulet-copy-flat-life" {
            assert_eq!(
                inverse.nodes.pop().unwrap(),
                guard("effective", "unbounded-effective", count.clone())
            );
            let source = inverse
                .nodes
                .iter_mut()
                .find(|n| n.id.as_str() == "unbounded-effective")
                .unwrap();
            assert_eq!(
                source.expression,
                RuleExpression::Read {
                    input: "effective".parse().unwrap()
                }
            );
            source.id = "effective".parse().unwrap();
        }
        assert_eq!(inverse.effects.len(), 1);
        let RuleEffectKind::Contribute { value, .. } = &mut inverse.effects[0].effect else {
            panic!()
        };
        assert_eq!(value.as_str(), "bounded-life-points");
        *value = "life-points".parse().unwrap();
        assert_eq!(
            &inverse, old,
            "guards preserve every source read, recipient, condition and arithmetic operation"
        );
    }
    let mut inverse = query.clone();
    let group = inverse
        .groups
        .iter_mut()
        .find(|g| g.id.as_str() == "equipment")
        .unwrap();
    assert!(group.members.is_complete());
    let old = d.query.groups.iter().find(|g| g.id == group.id).unwrap();
    let SchemaClosure::Partial { gaps } = &old.members.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 1);
    assert_eq!(
        gaps[0].code.as_str(),
        "life-contributor-domain-and-order-unproved"
    );
    group.members.closure = old.members.closure.clone();
    assert_eq!(
        inverse, d.query,
        "only the equipment numeric-domain gap is retired"
    );
}
pub fn check_authored() {
    let author: Value = read("authoring.json");
    let proof: Value = read("proof.json");
    let d = dependencies();
    assert_eq!(author["kind"], KIND);
    assert_eq!(author["before"], json!(d.before));
    assert_eq!(
        author["scope"],
        json!({"changed_programs":2,"closed_groups":1,
        "closed_existing_owners":0,"new_definitions":0,"new_receivers":0,
        "supplier_admission":false,"evaluation_bundle":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(author["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(
            author["artifacts"][file]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    // This is an evaluator support bound, never an item-legality restriction.
    // The engine separately rejects attempts to raise its hard resource limits.
    assert_eq!(PlanLimits::default().max_effects, 65_536);
    assert_eq!(proof["max_effects"], 65_536);
    assert_eq!(proof["component_bound"], 137_438_953_472.0);
    assert_eq!(proof["maximum_absolute_prefix"], 9_007_199_254_740_992.0);
    assert_eq!(
        PlanLimits::default().max_effects as u64 * 137_438_953_472u64,
        1u64 << 53
    );
    for (field, unit) in [
        ("count_domain", "def.000000000000295a"),
        ("life_domain", "def.0000000000003119"),
    ] {
        let NumericDomain::Quantity(range) = serde_json::from_value(proof[field].clone()).unwrap()
        else {
            panic!()
        };
        assert_eq!(range.minimum.value(), -137_438_953_472.0);
        assert_eq!(range.maximum.value(), 137_438_953_472.0);
        assert_eq!(range.minimum.unit(), range.maximum.unit());
        assert_eq!(range.minimum.unit().key().as_str(), unit);
    }
    let SchemaClosure::Partial { gaps } = &d.owner.programs.closure else {
        panic!()
    };
    assert_eq!(gaps.len(), 4);
    assert_eq!(
        d.owner
            .programs
            .members
            .iter()
            .filter(|p| p.id.as_str() == "effective-amount")
            .count(),
        1
    );
    assert_eq!(d.potential_writers.len(), 7);
    check_transition(&programs(), &query());
}

fn census(rules: &RulePackageInput, guarded: bool) {
    let d = dependencies();
    let replacements = programs();
    let mut expected = d.potential_writers;
    if guarded {
        for writer in &mut expected {
            if writer.owner == d.owner.owner {
                writer.program = replacements
                    .iter()
                    .find(|p| p.id == writer.program.id)
                    .unwrap()
                    .clone();
            }
        }
    }
    let mut found = vec![];
    for owner in &rules.owners {
        for program in &owner.programs.members {
            if program.effects.iter().any(|e| matches!(&e.effect,
                RuleEffectKind::Contribute { stat, contribution: ContributionKind::Add, .. } if *stat == d.query.stat)) {
                found.push(LifeWriter { owner: owner.owner.clone(), program: program.clone() });
            }
        }
    }
    assert_eq!(
        found, expected,
        "every potential Life/Add writer must remain exact, including inactive or unselected sources"
    );
    for application in rules.effect_applications.iter().flat_map(|r| &r.members) {
        assert!(!application.program.effects.iter().any(|e| matches!(&e.effect,
            RuleEffectKind::Contribute { stat, contribution: ContributionKind::Add, .. } if *stat == d.query.stat)),
            "an application Life writer needs separately reviewed membership");
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let d = dependencies();
    assert_eq!(prior.receipt().input, d.before);
    assert_eq!(
        prior.input().recipe.rules.operations_version,
        d.operations_version
    );
    census(&prior.input().recipe.rules, false);
    let mut input = prior.input().clone();
    let owner = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owner.owner)
        .unwrap();
    assert_eq!(*owner, d.owner);
    for program in programs() {
        let old = owner
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == program.id)
            .unwrap();
        *old = program;
    }
    let target = input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == d.query.id)
        .unwrap();
    assert_eq!(*target, d.query);
    *target = query();
    census(&input.recipe.rules, true);
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let current = inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|o| o.owner == d.owner.owner)
        .unwrap();
    let mut expected = d.owner.clone();
    for program in programs() {
        let id = program.id.clone();
        *expected
            .programs
            .members
            .iter_mut()
            .find(|p| p.id == id)
            .unwrap() = program;
    }
    assert_eq!(*current, expected);
    *current = d.owner;
    let current = inverse
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .iter_mut()
        .find(|q| q.id == d.query.id)
        .unwrap();
    assert_eq!(*current, query());
    *current = d.query;
    inverse.provenance.pop();
    assert!(
        inverse == *prior.input(),
        "exact whole-input inverse: only three range guards, two output bindings and equipment closure/provenance change"
    );
    assert!(next.evaluation().is_none());
    assert_eq!(next.query_sets(), prior.query_sets());
    next
}
