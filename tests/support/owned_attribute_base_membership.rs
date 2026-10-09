//! A bounded class/passive BASE domain, not general item or copied BASE order.
#[path = "owned_attribute_base_source_audit.rs"]
pub mod source_audit;
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
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "attribute-base-class-passive-membership";
const FILES: [&str; 5] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "queries.json",
    "source-vectors.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/attribute-base-class-passive")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = FILES.map(read).into();
    digest_owned(KIND, &values, 8 * 1024 * 1024).unwrap()
}
#[derive(Clone, Deserialize)]
pub struct Replacement {
    pub before: ContributionQuery,
    pub after: ContributionQuery,
}
#[derive(Deserialize)]
pub struct Dependencies {
    pub owners: Vec<DefinitionRules>,
    pub query_registry_before: DeclaredSet<ContributionQuery>,
}
pub fn replacements() -> Vec<Replacement> {
    read("queries.json")
}
fn registry(after: bool) -> DeclaredSet<ContributionQuery> {
    let mut registry = read::<Dependencies>("dependencies.json").query_registry_before;
    assert!(!registry.is_complete());
    if after {
        for replacement in replacements() {
            let row = registry
                .members
                .iter_mut()
                .find(|q| q.id == replacement.before.id)
                .unwrap();
            assert_eq!(*row, replacement.before);
            *row = replacement.after;
        }
    }
    registry
}
fn domain(owner: &SchemaSubject) -> Option<ContributionOrigin> {
    match owner {
        SchemaSubject::Definition(DefinitionAddress::Class(_)) => {
            Some(ContributionOrigin::Character)
        }
        SchemaSubject::Definition(DefinitionAddress::PassiveNode(_)) => {
            Some(ContributionOrigin::Allocation)
        }
        _ => None,
    }
}
/// Freeze the full donor bodies, including their guards, scope, and paired effects.
/// Excluded donors stay in the package: the candidate binder rejects them before
/// evaluating value/activation. No unlisted source is asserted to be absent.
fn census(owners: &[DefinitionRules], applications: &[EffectApplicationRule]) {
    let dependencies: Dependencies = read("dependencies.json");
    let queries = replacements();
    let b: Value = read("bindings.json");
    let mut excluded = Vec::new();
    let mut admitted = 0;
    for owner in owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                let RuleEffectKind::Contribute {
                    stat,
                    contribution: ContributionKind::Add,
                    entity,
                    ..
                } = &effect.effect
                else {
                    continue;
                };
                let Some(query) = queries.iter().find(|q| q.after.stat == *stat) else {
                    continue;
                };
                let original = dependencies
                    .owners
                    .iter()
                    .find(|o| o.owner == owner.owner)
                    .expect("new potential BASE producer requires a renewed domain audit");
                assert_eq!(
                    owner, original,
                    "complete donor body/guard/ownership must remain exact"
                );
                if domain(&owner.owner).is_some() {
                    assert_eq!(
                        query.after.groups[0]
                            .members
                            .members
                            .iter()
                            .filter(|m| m.producer.as_program_effect().unwrap().owner
                                == owner.owner
                                && m.producer.as_program_effect().unwrap().program == program.id
                                && m.producer.as_program_effect().unwrap().effect == effect.id)
                            .count(),
                        1
                    );
                    admitted += 1;
                } else {
                    excluded.push(json!({"owner":owner.owner,"program":program.id,"effect":effect.id,"stat":stat,"entity":entity}));
                }
            }
        }
    }
    assert_eq!(admitted, 1972);
    assert_eq!(
        json!(excluded),
        b["excluded"],
        "all twelve current item effects are explicitly unsupported, not removed"
    );
    for application in applications {
        for effect in &application.program.effects {
            if let RuleEffectKind::Contribute {
                stat,
                contribution: ContributionKind::Add,
                ..
            } = &effect.effect
            {
                assert!(
                    !queries.iter().any(|q| q.after.stat == *stat),
                    "new application BASE producer requires an explicit domain audit, even when inactive"
                );
            }
        }
    }
}
fn source_projection() {
    source_audit::verify(false);
    let v: Value = read("source-vectors.json");
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(v["source_audit_path"].as_str().unwrap());
    let bytes = fs::read(path).unwrap();
    assert_eq!(v["source_audit_sha256"], hash(&bytes));
    let source: Value = serde_json::from_slice(&bytes).unwrap();
    let mut stages = Vec::new();
    for case in source["cases"].as_array().unwrap() {
        for mode in ["MAIN", "CALCS"] {
            for stage in case["modes"][mode]["stages"].as_array().unwrap() {
                let records: Vec<_> = stage["base_records"].as_array().unwrap().iter().map(|r| json!({
                    "index":r["index"],"source":r["record"]["source"],"value":r["record"]["value"]["root"]["value"],
                    "flags":r["record"]["flags"],"keyword_flags":r["record"]["keyword_flags"],"tag_count":r["record"]["tag_count"]
                })).collect();
                let base = stage["queries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|q| q["contribution"] == "BASE")
                    .unwrap();
                stages.push(json!({"case":case["name"],"mode":mode,"xml_sha256":case["xml_sha256"],
                    "pass":stage["pass"],"stat":stage["stat"],"final":stage["value"],"base":base["result"],"records":records}));
            }
        }
    }
    assert_eq!(v["stages"], json!(stages));
    let original: Vec<_> = stages
        .iter()
        .filter(|s| s["case"] == "original-05")
        .collect();
    assert_eq!(original.len(), 12);
    for s in original {
        let records = s["records"].as_array().unwrap();
        assert!(
            records.iter().all(
                |r| r["source"] == "Base" || r["source"].as_str().unwrap().starts_with("Tree:")
            )
        );
        assert!(
            records
                .iter()
                .all(|r| r["tag_count"] == 0 && r["flags"] == 0 && r["keyword_flags"] == 0)
        );
        assert!(records.iter().all(|r| r["value"].as_f64().unwrap() > 0.));
    }
    // Other builds carry both item and executed zero-valued copy records. They
    // remain evidence against generalizing this restriction to all BASE inputs.
    assert!(
        stages
            .iter()
            .flat_map(|s| s["records"].as_array().unwrap())
            .any(|r| r["value"] == 0
                && r["source"]
                    .as_str()
                    .unwrap()
                    .contains("Amulet Bonus Effect"))
    );
    let choice_bytes = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(v["choice_source_path"].as_str().unwrap()),
    )
    .unwrap();
    assert_eq!(v["choice_source_sha256"], hash(&choice_bytes));
    let choice_source: Value = serde_json::from_slice(&choice_bytes).unwrap();
    assert_eq!(v["choice_control"], choice_source["cases"][1]);
    assert_eq!(
        v["choice_control"]["name"],
        "original-05-strength-choice-to-dexterity"
    );
    // Both packets authenticate the very same complete source reports. The
    // publication additionally checks this control's projection below.
    for report in choice_source["source_reports"].as_array().unwrap() {
        assert!(
            source["source_reports"]
                .as_array()
                .unwrap()
                .contains(report)
        );
    }
}
fn check_choice_report() {
    let v: Value = read("source-vectors.json");
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source: Value = serde_json::from_slice(
        &fs::read(root.join(v["choice_source_path"].as_str().unwrap())).unwrap(),
    )
    .unwrap();
    let pin = &source["source_reports"][0];
    let bytes = fs::read(root.join(pin["path"].as_str().unwrap())).unwrap();
    assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
    assert_eq!(hash(&bytes), pin["sha256"]);
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    let case = &report["cases"][5];
    let expected = &v["choice_control"];
    for field in ["name", "xml_sha256", "choice_control"] {
        assert_eq!(case[field], expected[field]);
    }
    for mode in ["MAIN", "CALCS"] {
        let actual = &case["original"]["state"]["modes"][mode];
        assert_eq!(actual["class_id"], expected["modes"][mode]["class_id"]);
        let stages = actual["provenance"]["stages"].as_array().unwrap();
        assert_eq!(stages.len(), 6);
        for (stage, expected) in stages
            .iter()
            .zip(expected["modes"][mode]["stages"].as_array().unwrap())
        {
            for field in ["index", "pass", "stat", "value", "queries", "reads"] {
                assert_eq!(stage[field], expected[field]);
            }
            let chain = stage["before_chain"].as_array().unwrap();
            assert_eq!(chain.len(), 1);
            for field in ["depth", "parent_kind"] {
                assert_eq!(chain[0][field], expected[field]);
            }
            let records: Vec<_> = chain[0]["attributes"][stage["stat"].as_str().unwrap()]
                .as_array().unwrap().iter().map(|row| {
                    let record = &row["record"];
                    assert_eq!(record["value"]["root"]["kind"], "number");
                    json!({"index":row["index"],"source":record["source"],"name":record["name"],"type":record["type"],
                        "flags":record["flags"],"keyword_flags":record["keyword_flags"],"tag_count":record["tag_count"],
                        "value":record["value"]["root"]["value"]})
                }).collect();
            assert_eq!(json!(records), expected["records"]);
        }
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["scope"], b["scope"]);
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(a["artifacts"][file]["sha256"], hash(&bytes));
    }
    let d: Dependencies = read("dependencies.json");
    let queries = replacements();
    assert_eq!(queries.len(), 6);
    let mut total = 0;
    let mut classes = BTreeSet::new();
    for (i, q) in queries.iter().enumerate() {
        assert_eq!(q.before.id, q.after.id);
        assert_eq!(q.after.contribution, ContributionKind::Add);
        assert_eq!(q.after.groups.len(), 1);
        let group = &q.after.groups[0];
        assert_eq!(group.ordering, ContributionOrdering::Ordered);
        assert_eq!(group.reduction, ContributionReduction::Sum);
        assert!(group.members.is_complete());
        assert_eq!(
            group.members.members.len(),
            [328, 328, 330, 328, 328, 330][i]
        );
        assert!(!q.before.groups[0].members.is_complete());
        let mut inverse = q.after.clone();
        inverse.groups[0].members = q.before.groups[0].members.clone();
        assert_eq!(inverse, q.before, "only membership changes");
        let ParameterValue::Quantity(empty) = &group.empty else {
            panic!("Count identity")
        };
        assert_eq!(empty.value(), 0.);
        assert_eq!(empty.unit().key().as_str(), "def.000000000000295a");
        let mut positions = BTreeSet::new();
        let mut sum = 0.;
        for member in &group.members.members {
            let expected = domain(&member.producer.as_program_effect().unwrap().owner)
                .expect("only direct class and passive origins");
            assert_eq!(
                member.producer.as_program_effect().unwrap().origin,
                expected
            );
            let owner = d
                .owners
                .iter()
                .find(|o| o.owner == member.producer.as_program_effect().unwrap().owner)
                .unwrap();
            if expected == ContributionOrigin::Character {
                assert!(!owner.programs.is_complete());
                let SchemaSubject::Definition(DefinitionAddress::Class(class)) = &owner.owner
                else {
                    unreachable!("Character origins were classified from exact Class owners")
                };
                classes.insert(class.clone());
            } else {
                assert!(owner.programs.is_complete());
            }
            let program = owner
                .programs
                .members
                .iter()
                .find(|p| p.id == member.producer.as_program_effect().unwrap().program)
                .unwrap();
            assert_eq!(program.context, RuleEntityKind::Actor);
            let (index, effect) = program
                .effects
                .iter()
                .enumerate()
                .find(|(_, e)| e.id == member.producer.as_program_effect().unwrap().effect)
                .unwrap();
            let RuleEffectKind::Contribute {
                entity,
                stat,
                contribution,
                value,
            } = &effect.effect
            else {
                panic!("contribution")
            };
            assert!(matches!(entity, RuleEntity::Current | RuleEntity::Player));
            assert_eq!(*stat, q.after.stat);
            assert_eq!(*contribution, ContributionKind::Add);
            let node = program.nodes.iter().find(|n| n.id == *value).unwrap();
            let RuleExpression::Literal {
                value: ParameterValue::Quantity(amount),
            } = &node.expression
            else {
                panic!("fixed exact integer")
            };
            assert_eq!(amount.unit(), empty.unit());
            assert!((3.0..=25.0).contains(&amount.value()));
            assert_eq!(amount.value().fract(), 0.);
            let order = member.order.as_ref().unwrap();
            assert!(order.slot_ranks.is_empty());
            assert_eq!(order.effect_rank as usize, index);
            assert!(positions.insert((order.source_rank, order.program_rank, order.effect_rank)));
            sum += amount.value();
        }
        // Every duplicate instance of one member has the same position and is
        // rejected by the cold binder. Thus each positive term appears at most
        // once, not once per caller-supplied occurrence or resource-limit unit.
        assert_eq!(sum, [1711., 1707., 1743., 1711., 1707., 1743.][i]);
        assert!(sum < (1_u64 << 53) as f64);
        assert_eq!(
            b["proof"]["stages"][i]["maximum_sum"].as_f64().unwrap(),
            sum
        );
        total += group.members.members.len();
    }
    assert_eq!(total, 1972);
    assert_eq!(classes.len(), 8);
    census(&d.owners, &[]);
    assert_eq!(registry(false).closure, registry(true).closure);
    source_projection();
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    assert!(
        !endpoint
            .input()
            .recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .is_complete(),
        "this packet never closes the global contributor registry"
    );
    for replacement in replacements() {
        assert!(
            endpoint
                .input()
                .recipe
                .rules
                .contribution_queries
                .as_ref()
                .unwrap()
                .members
                .contains(&replacement.after)
        );
    }
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
    source_audit::verify(true);
    check_choice_report();
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
    assert_eq!(
        prior.input().recipe.rules.contribution_queries,
        Some(registry(false))
    );
    census(
        &prior.input().recipe.rules.owners,
        &prior
            .input()
            .recipe
            .rules
            .effect_applications
            .as_ref()
            .unwrap()
            .members,
    );
    let mut input = prior.input().clone();
    input.recipe.rules.contribution_queries = Some(registry(true));
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    inverse.recipe.rules.contribution_queries = Some(registry(false));
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "exact inverse: six memberships and provenance only"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[test]
fn changed_donor_body_is_not_covered_by_the_bounded_sum_proof() {
    let mut d: Dependencies = read("dependencies.json");
    let q = replacements().remove(0);
    let member = &q.after.groups[0].members.members[0];
    let owner = d
        .owners
        .iter_mut()
        .find(|o| o.owner == member.producer.as_program_effect().unwrap().owner)
        .unwrap();
    let program = owner
        .programs
        .members
        .iter_mut()
        .find(|p| p.id == member.producer.as_program_effect().unwrap().program)
        .unwrap();
    program
        .effects
        .iter_mut()
        .find(|e| e.id == member.producer.as_program_effect().unwrap().effect)
        .unwrap()
        .when = Some("unreviewed-guard".parse().unwrap());
    let failure = std::panic::catch_unwind(|| census(&d.owners, &[]));
    assert!(
        failure.is_err(),
        "a guard/body change invalidates exact source authority"
    );
}

#[test]
fn inactive_application_source_is_not_hidden_by_the_domain_census() {
    let d: Dependencies = read("dependencies.json");
    let path = data()
        .parent()
        .unwrap()
        .join("pain-offering/applications.json");
    let mut apps: DeclaredSet<EffectApplicationRule> =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let program = &mut apps.members[0].program;
    program.nodes.extend([
        RuleNode {
            id: "unsupported-zero".parse().unwrap(),
            expression: RuleExpression::Literal {
                value: replacements()[0].after.groups[0].empty.clone(),
            },
        },
        RuleNode {
            id: "unsupported-inactive".parse().unwrap(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        },
    ]);
    program.effects.push(RuleEffect {
        id: "unsupported-base".parse().unwrap(),
        when: Some("unsupported-inactive".parse().unwrap()),
        effect: RuleEffectKind::Contribute {
            entity: RuleEntity::Player,
            stat: replacements()[0].after.stat.clone(),
            contribution: ContributionKind::Add,
            value: "unsupported-zero".parse().unwrap(),
        },
    });
    assert!(std::panic::catch_unwind(|| census(&d.owners, &apps.members)).is_err());
}
