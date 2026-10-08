//! Checked direct passive flag sources and three guarded empty domains.
//! Global/owner closure and unconverted game sources remain separate obligations.
use super::migration_preservation;
#[allow(dead_code)]
#[path = "owned_strength_life.rs"]
mod strength_life;
use poe_optimizer_core::{
    owned_build::ParameterValue,
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

pub const KIND: &str = "inherent-attribute-flag-membership";
const FILES: [&str; 5] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "queries.json",
    "source-vectors.json",
];
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/inherent-attribute-flag-membership")
}
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
#[derive(Clone, Deserialize)]
pub struct Replacement {
    pub before: ContributionQuery,
    pub after: ContributionQuery,
}
#[derive(Deserialize)]
struct Dependencies {
    owners: Vec<DefinitionRules>,
    definitions: Vec<DefinitionDescriptor>,
    receivers: Vec<StatReceiver>,
    query_registry_closure: SchemaClosure,
}
pub fn replacements() -> Vec<Replacement> {
    read("queries.json")
}
fn flag_stat(effect: &RuleEffectKind, queries: &[Replacement]) -> bool {
    match effect {
        RuleEffectKind::Contribute { stat, .. } | RuleEffectKind::Derive { stat, .. } => {
            queries.iter().any(|q| q.after.stat == *stat)
        }
        _ => false,
    }
}
/// Inspect every potential writer, without evaluating guards, values or saved
/// selection. Appended support rules are ordinary owner programs in this census;
/// applications must have no matching write. The binder separately checks every
/// concrete occurrence, including late, inactive and generated provider paths.
fn census(owners: &[DefinitionRules], applications: &[EffectApplicationRule]) {
    let d: Dependencies = read("dependencies.json");
    let queries = replacements();
    let mut actual = BTreeSet::new();
    let mut expected = BTreeSet::new();
    for owner in &d.owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                if flag_stat(&effect.effect, &queries) {
                    assert!(expected.insert(
                        serde_json::to_string(&json!([owner.owner, program.id, effect])).unwrap()
                    ));
                }
            }
        }
    }
    assert_eq!(
        expected.len(),
        7,
        "two Flag contributors and five resolved Boolean writers"
    );
    for owner in owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                if flag_stat(&effect.effect, &queries) {
                    let exact = d.owners.iter().find(|o| o.owner == owner.owner).expect(
                        "new potential inherent flag writer requires renewed membership evidence",
                    );
                    assert_eq!(
                        owner, exact,
                        "full writer body, guard, recipient and closure remain exact"
                    );
                    assert!(
                        actual.insert(
                            serde_json::to_string(&json!([owner.owner, program.id, effect]))
                                .unwrap()
                        ),
                        "duplicate flag writer identity"
                    );
                }
            }
        }
    }
    assert_eq!(
        actual, expected,
        "complete current compiled-source census, not global game absence"
    );
    for application in applications {
        assert!(
            application
                .program
                .effects
                .iter()
                .all(|e| !flag_stat(&e.effect, &queries)),
            "an application writer requires explicit source/recipient authority even when inactive or false"
        );
    }
}
pub fn authenticate_source(full: bool) -> Value {
    let v: Value = read("source-vectors.json");
    assert_eq!(v["status"], "passed");
    let pins = v["source_packets"].as_array().unwrap();
    assert_eq!(pins.len(), 4);
    let mut paths = BTreeSet::new();
    for pin in pins {
        let path = pin["path"].as_str().unwrap();
        assert!(paths.insert(path));
        let bytes = fs::read(root().join(path)).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], hash(&bytes));
    }
    let source_rules: Value =
        serde_json::from_slice(&fs::read(root().join(pins[0]["path"].as_str().unwrap())).unwrap())
            .unwrap();
    let source_pins: Value =
        serde_json::from_slice(&fs::read(root().join(pins[1]["path"].as_str().unwrap())).unwrap())
            .unwrap();
    let manifest: Value = serde_json::from_slice(
        &fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        source_pins["upstream_revision"],
        manifest["upstream_revision"]
    );
    for pin in source_pins["pins"].as_array().unwrap() {
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|r| *r == pin)
                .count(),
            1
        );
        if full {
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(pin["bytes"], text.len());
            assert_eq!(pin["sha256"], hash(text.as_bytes()));
        }
    }
    assert_eq!(v["producer_sources"], source_pins["producers"]);
    let d: Dependencies = read("dependencies.json");
    assert_eq!(source_rules["producers"].as_array().unwrap().len(), 2);
    for producer in source_rules["producers"].as_array().unwrap() {
        let owner: SchemaSubject = serde_json::from_value(producer["owner"].clone()).unwrap();
        let actual = d.owners.iter().find(|o| o.owner == owner).unwrap();
        assert_eq!(json!(actual.programs.members), json!([producer["program"]]));
        assert!(!actual.programs.is_complete());
    }
    assert_eq!(
        json!(
            replacements()
                .into_iter()
                .map(|q| q.before)
                .collect::<Vec<_>>()
        ),
        source_rules["queries"]
    );
    let proof = strength_life::authenticate_source(full);
    assert_eq!(v["source_reports"], proof["reports"]);
    assert_eq!(v["original"], proof["native_cases"][4]);
    assert_eq!(v["original"]["name"], "original-05");
    assert_eq!(v["original"]["strength"], 27);
    assert_eq!(v["original"]["inherent_life"], 54);
    assert_eq!(v["original"]["emitted_count"], 1);
    assert_eq!(v["original"]["source_only_control"], false);
    let controls: Vec<_> = proof["native_cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["source_only_control"] == true)
        .map(|c| c["name"].clone())
        .collect();
    assert_eq!(v["source_only_controls"], json!(controls));
    for mode in ["MAIN", "CALCS"] {
        let observed = &proof["projections"][4]["state"]["modes"][mode];
        assert_eq!(observed["read_set"].as_array().unwrap().len(), 1);
        let flags = observed["flags"].as_object().unwrap();
        assert_eq!(flags.len(), 5);
        for (name, result) in flags {
            assert_eq!(result, &json!({"present":false,"resolved":false}));
            for store in observed["read_set"].as_array().unwrap() {
                assert!(store["flags"][name].as_object().unwrap().is_empty());
            }
        }
    }
    proof
}
pub fn checked_source() -> Value {
    authenticate_source(false)
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
    let d: Dependencies = read("dependencies.json");
    assert!(!matches!(d.query_registry_closure, SchemaClosure::Complete));
    assert_eq!(
        (d.owners.len(), d.definitions.len(), d.receivers.len()),
        (8, 10, 6)
    );
    let queries = replacements();
    assert_eq!(queries.len(), 5);
    for (i, q) in queries.iter().enumerate() {
        assert_eq!(
            q.after.stat.key().as_str(),
            format!("def.{:016x}", 0x3315 + i)
        );
        assert_eq!(q.after.contribution, ContributionKind::Flag);
        assert_eq!(q.after.groups.len(), 1);
        let group = &q.after.groups[0];
        assert_eq!(group.ordering, ContributionOrdering::Unordered);
        assert_eq!(group.reduction, ContributionReduction::Any);
        assert_eq!(group.empty, ParameterValue::Boolean(false));
        assert!(group.members.is_complete());
        assert_eq!(group.members.members.len(), [0, 0, 0, 1, 1][i]);
        assert!(!q.before.groups[0].members.is_complete());
        let mut inverse = q.after.clone();
        inverse.groups[0].members.closure = q.before.groups[0].members.closure.clone();
        assert_eq!(inverse, q.before, "only bounded member closure changes");
        for member in &group.members.members {
            assert_eq!(member.origin, ContributionOrigin::Allocation);
            assert!(member.order.is_none());
            let owner = d.owners.iter().find(|o| o.owner == member.owner).unwrap();
            assert!(
                !owner.programs.is_complete(),
                "unrelated passive mechanics remain Partial"
            );
            let program = owner
                .programs
                .members
                .iter()
                .find(|p| p.id == member.program)
                .unwrap();
            let effect = program
                .effects
                .iter()
                .find(|e| e.id == member.effect)
                .unwrap();
            let RuleEffectKind::Contribute {
                entity,
                stat,
                contribution,
                value,
            } = &effect.effect
            else {
                panic!("actual flag producer")
            };
            assert_eq!(*entity, RuleEntity::Player);
            assert_eq!(*stat, q.after.stat);
            assert_eq!(*contribution, ContributionKind::Flag);
            let node = program.nodes.iter().find(|n| n.id == *value).unwrap();
            assert_eq!(
                node.expression,
                RuleExpression::Literal {
                    value: ParameterValue::Boolean(true)
                }
            );
        }
    }
    census(&d.owners, &[]);
    authenticate_source(false);
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Dependencies = read("dependencies.json");
    for owner in &d.owners {
        assert_eq!(
            endpoint
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
    for descriptor in &d.definitions {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|row| *row == descriptor)
                .count(),
            1
        );
    }
    for receiver in &d.receivers {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .filter(|row| *row == receiver)
                .count(),
            1
        );
    }
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .closure,
        d.query_registry_closure
    );
    census(
        &endpoint.input().recipe.rules.owners,
        &endpoint
            .input()
            .recipe
            .rules
            .effect_applications
            .as_ref()
            .unwrap()
            .members,
    );
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let registry = endpoint
        .input()
        .recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap();
    assert!(!registry.is_complete());
    for q in replacements() {
        assert!(registry.members.contains(&q.after));
    }
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
    authenticate_source(true);
    dependencies(prior);
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
    let mut input = prior.input().clone();
    for q in replacements() {
        let row = input
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|r| r.id == q.before.id)
            .unwrap();
        assert_eq!(*row, q.before);
        *row = q.after;
    }
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for q in replacements() {
        let row = inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .iter_mut()
            .find(|r| r.id == q.after.id)
            .unwrap();
        assert_eq!(*row, q.after);
        *row = q.before;
    }
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "exact inverse: five membership closures and provenance only"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[test]
fn changed_flag_guard_or_recipient_invalidates_membership_proof() {
    for change_guard in [true, false] {
        let mut d: Dependencies = read("dependencies.json");
        let member = replacements()[3].after.groups[0].members.members[0].clone();
        let owner = d
            .owners
            .iter_mut()
            .find(|o| o.owner == member.owner)
            .unwrap();
        let effect = &mut owner.programs.members[0].effects[0];
        if change_guard {
            effect.when = Some("unreviewed-guard".parse().unwrap());
        } else {
            let RuleEffectKind::Contribute { entity, .. } = &mut effect.effect else {
                unreachable!()
            };
            *entity = RuleEntity::Current;
        }
        assert!(std::panic::catch_unwind(|| census(&d.owners, &[])).is_err());
    }
}
#[test]
fn unknown_false_or_inactive_writer_cannot_be_hidden_by_empty_domains() {
    for application in [false, true] {
        let mut d: Dependencies = read("dependencies.json");
        let source = d.owners.iter().find(|o| !o.programs.is_complete()).unwrap();
        let mut program = source.programs.members[0].clone();
        program.id = "unreviewed-flag".parse().unwrap();
        program.nodes.push(RuleNode {
            id: "inactive".parse().unwrap(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        });
        let effect = &mut program.effects[0];
        effect.when = Some("inactive".parse().unwrap());
        let RuleEffectKind::Contribute { stat, value, .. } = &mut effect.effect else {
            unreachable!()
        };
        *stat = replacements()[0].after.stat.clone();
        *value = "inactive".parse().unwrap();
        if application {
            let mut apps: DeclaredSet<EffectApplicationRule> = serde_json::from_slice(
                &fs::read(
                    data()
                        .parent()
                        .unwrap()
                        .join("pain-offering/applications.json"),
                )
                .unwrap(),
            )
            .unwrap();
            apps.members[0].program = program;
            assert!(std::panic::catch_unwind(|| census(&d.owners, &apps.members)).is_err());
        } else {
            d.owners
                .iter_mut()
                .find(|o| !o.programs.is_complete())
                .unwrap()
                .programs
                .members
                .push(program);
            assert!(std::panic::catch_unwind(|| census(&d.owners, &[])).is_err());
        }
    }
}
