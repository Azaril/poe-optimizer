//! Exact current Increase membership; no BASE order or whole-owner promotion.
use super::source;
use poe_optimizer_core::{
    owned_build::ParameterValue,
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::*,
    owned_schema::{DeclaredSet, RuleEntityKind},
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "attribute-increase-membership";
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/attribute-increase-membership")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let values: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "dependencies.json",
        "queries.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(KIND, &values, 1024 * 1024).unwrap()
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

fn expected_registry(after: bool) -> DeclaredSet<ContributionQuery> {
    let d: Dependencies = read("dependencies.json");
    let mut result = d.query_registry_before;
    assert!(!result.is_complete());
    if after {
        for replacement in read::<Vec<Replacement>>("queries.json") {
            let row = result
                .members
                .iter_mut()
                .find(|q| q.id == replacement.before.id)
                .unwrap();
            assert_eq!(*row, replacement.before);
            *row = replacement.after;
        }
    }
    result
}

/// Exhaust every actual effect on all six channels, including guarded effects.
/// No item, inactive branch, zero value or foreign program can be dropped here.
fn census(owners: &[DefinitionRules], queries: &[Replacement]) {
    for replacement in queries {
        let query = &replacement.after;
        let members = &query.groups[0].members.members;
        let mut matched = BTreeSet::new();
        for owner in owners {
            for program in &owner.programs.members {
                for effect in &program.effects {
                    if let RuleEffectKind::Contribute {
                        stat, contribution, ..
                    } = &effect.effect
                        && *stat == query.stat
                        && *contribution == query.contribution
                    {
                        let matches: Vec<_> = members
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| {
                                m.producer.as_program_effect().unwrap().owner == owner.owner
                                    && m.producer.as_program_effect().unwrap().program == program.id
                                    && m.producer.as_program_effect().unwrap().effect == effect.id
                            })
                            .collect();
                        assert_eq!(
                            matches.len(),
                            1,
                            "every actual potential Increase effect is declared once"
                        );
                        assert!(matched.insert(matches[0].0));
                    }
                }
            }
        }
        assert_eq!(
            matched.len(),
            members.len(),
            "every declared member has an actual effect"
        );
    }
}

/// Applications contain independent programs, unlike receivers which refer to
/// owner programs. This packet admits only the direct Allocation origins above.
/// Inspect potential effects even when their source/recipient is not selected.
fn application_census(applications: &[EffectApplicationRule], queries: &[Replacement]) {
    for application in applications {
        for effect in &application.program.effects {
            if let RuleEffectKind::Contribute {
                stat, contribution, ..
            } = &effect.effect
            {
                assert!(
                    !queries
                        .iter()
                        .any(|q| q.after.stat == *stat && q.after.contribution == *contribution),
                    "potential application Increase effect is outside the declared membership"
                );
            }
        }
    }
}

#[test]
fn application_census_rejects_zero_inactive_increase_before_selection() {
    let path = data()
        .parent()
        .unwrap()
        .join("pain-offering/applications.json");
    let mut applications: DeclaredSet<EffectApplicationRule> =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let queries: Vec<Replacement> = read("queries.json");
    application_census(&applications.members, &queries);
    let application = &mut applications.members[0];
    let zero: OwnedDefinitionKey = "fixture-increase-zero".parse().unwrap();
    let disabled: OwnedDefinitionKey = "fixture-increase-disabled".parse().unwrap();
    application.program.nodes.extend([
        RuleNode {
            id: zero.clone(),
            expression: RuleExpression::Literal {
                value: queries[0].after.groups[0].empty.clone(),
            },
        },
        RuleNode {
            id: disabled.clone(),
            expression: RuleExpression::Literal {
                value: ParameterValue::Boolean(false),
            },
        },
    ]);
    application.activation = disabled.clone();
    application.program.effects.push(RuleEffect {
        id: "fixture-unadmitted-increase".parse().unwrap(),
        when: Some(disabled),
        effect: RuleEffectKind::Contribute {
            entity: RuleEntity::Player,
            stat: queries[0].after.stat.clone(),
            contribution: ContributionKind::Increase,
            value: zero,
        },
    });
    // This authoring gate has no BuildInput: an absent selected source cannot
    // hide its future potential contribution. Exercise it before recipe checks.
    let error = std::panic::catch_unwind(|| application_census(&applications.members, &queries))
        .expect_err("even a zero, disabled application must be accounted for");
    let message = error
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| error.downcast_ref::<&str>().copied())
        .unwrap();
    assert!(message.contains("potential application Increase effect"));
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let replacements: Vec<Replacement> = read("queries.json");
    assert_eq!(replacements.len(), 6);
    assert_eq!(d.owners.len(), 9);
    assert_eq!(b["contributors"].as_array().unwrap().len(), 9);
    assert_eq!(b["stages"].as_array().unwrap().len(), 6);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(b["scope"]["complete_increase_groups"], 6);
    assert_eq!(b["scope"]["declared_effect_members"], 34);
    for field in [
        "closed_existing_rule_owners",
        "new_definitions",
        "new_programs",
        "new_receivers",
        "numeric_program_changes",
    ] {
        assert_eq!(b["scope"][field], 0);
    }
    for field in [
        "global_registry_closure_changed",
        "base_membership_changed",
        "source_admission_widened",
        "whole_build_parity",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    assert_eq!(b["proof"]["maximum_members_per_query"], 7);
    assert_eq!(b["proof"]["minimum_literal"], 3);
    assert_eq!(b["proof"]["maximum_literal"], 8);
    assert_eq!(
        b["proof"]["full_member_sums"],
        json!([29, 24, 22, 29, 24, 22])
    );
    for owner in &d.owners {
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        assert_eq!(owner.programs.members[0].id.as_str(), "passive-view");
        assert_eq!(owner.programs.members[0].context, RuleEntityKind::Actor);
        assert!(owner.programs.members[0].reads.is_empty());
    }
    census(&d.owners, &replacements);
    for (i, replacement) in replacements.iter().enumerate() {
        let old = &replacement.before;
        let new = &replacement.after;
        assert_eq!(json!(new.id), b["stages"][i]["increase_query"]);
        assert_eq!(json!(new.stat), b["stages"][i]["input"]);
        assert_eq!(new.contribution, ContributionKind::Increase);
        assert_eq!(old.groups.len(), 1);
        assert_eq!(new.groups.len(), 1);
        assert!(!old.groups[0].members.is_complete());
        assert!(old.groups[0].members.members.is_empty());
        let group = &new.groups[0];
        assert!(group.members.is_complete());
        assert_eq!(group.id.as_str(), "ordinary");
        assert_eq!(group.reduction, ContributionReduction::Sum);
        let ParameterValue::Quantity(empty) = &group.empty else {
            panic!("percentage-point identity")
        };
        assert_eq!(empty.value(), 0.);
        assert_eq!(json!(empty.unit()), b["percent_unit"]);
        assert_eq!(group.members.members.len(), [7, 5, 5, 7, 5, 5][i]);
        let mut inverse = new.clone();
        inverse.groups[0].members = old.groups[0].members.clone();
        assert_eq!(inverse, *old, "only this membership declaration changes");
        let mut positions = BTreeSet::new();
        let mut sum = 0.;
        for member in &group.members.members {
            assert_eq!(
                member.producer.as_program_effect().unwrap().origin,
                ContributionOrigin::Allocation
            );
            assert_eq!(member.order.as_ref().unwrap().program_rank, 0);
            let contributor = b["contributors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["owner"] == json!(member.producer.as_program_effect().unwrap().owner))
                .unwrap();
            assert_eq!(
                json!(member.order.as_ref().unwrap().source_rank),
                contributor["source_rank"]
            );
            assert!(positions.insert((
                member.order.as_ref().unwrap().source_rank,
                member.order.as_ref().unwrap().program_rank,
                member.order.as_ref().unwrap().effect_rank
            )));
            let owner = d
                .owners
                .iter()
                .find(|o| o.owner == member.producer.as_program_effect().unwrap().owner)
                .unwrap();
            let program = &owner.programs.members[0];
            assert_eq!(
                member.producer.as_program_effect().unwrap().program,
                program.id
            );
            let (index, effect) = program
                .effects
                .iter()
                .enumerate()
                .find(|(_, e)| e.id == member.producer.as_program_effect().unwrap().effect)
                .unwrap();
            assert_eq!(member.order.as_ref().unwrap().effect_rank as usize, index);
            assert_eq!(effect.when.as_ref().unwrap().as_str(), "default");
            let RuleEffectKind::Contribute {
                entity,
                stat,
                contribution,
                value,
            } = &effect.effect
            else {
                panic!("actual Increase effect")
            };
            assert_eq!(*entity, RuleEntity::Player);
            assert_eq!(*stat, new.stat);
            assert_eq!(*contribution, ContributionKind::Increase);
            let node = program.nodes.iter().find(|n| n.id == *value).unwrap();
            let RuleExpression::Literal {
                value: ParameterValue::Quantity(amount),
            } = &node.expression
            else {
                panic!("fixed integral literal")
            };
            assert_eq!(amount.unit(), empty.unit());
            assert!((3.0..=8.0).contains(&amount.value()));
            assert_eq!(amount.value().fract(), 0.);
            sum += amount.value();
        }
        assert_eq!(sum, [29., 24., 22., 29., 24., 22.][i]);
    }
    let before = expected_registry(false);
    let after = expected_registry(true);
    assert_eq!(before.closure, after.closure);
    assert_eq!(before.members.len(), 18);
    assert_eq!(after.members.len(), 18);
    for (old, new) in before.members.iter().zip(&after.members) {
        if old.contribution != ContributionKind::Increase {
            assert_eq!(old, new);
        }
    }
    assert_eq!(a["artifacts"].as_object().unwrap().len(), 4);
    for name in [
        "bindings.json",
        "dependencies.json",
        "queries.json",
        "source-vectors.json",
    ] {
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(a["artifacts"][name]["sha256"], hash(&bytes));
    }
    source::verify(false);
}

fn dependencies(endpoint: &StagedOwnedRelease, after: bool) {
    let d: Dependencies = read("dependencies.json");
    let rules = &endpoint.input().recipe.rules;
    assert_eq!(rules.operations_version.as_str(), OWNED_RULE_OPERATIONS_V21);
    assert_eq!(rules.contribution_queries, Some(expected_registry(after)));
    for owner in d.owners {
        assert_eq!(rules.owners.iter().filter(|o| **o == owner).count(), 1);
    }
    census(&rules.owners, &read::<Vec<Replacement>>("queries.json"));
    application_census(
        &rules
            .effect_applications
            .as_ref()
            .expect("V21 application inventory")
            .members,
        &read::<Vec<Replacement>>("queries.json"),
    );
}
pub fn assert_endpoint(next: &StagedOwnedRelease) {
    check_authored();
    let b: Value = read("bindings.json");
    dependencies(next, true);
    let proof = next.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), b["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(json!(next.receipt().definitions), b["definitions"]);
    assert_eq!(json!(next.receipt().registry), b["registry"]);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    source::verify(true);
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
    dependencies(prior, false);
    assert!(prior.evaluation().is_none());
    let mut input = prior.input().clone();
    input.recipe.rules.contribution_queries = Some(expected_registry(true));
    input.provenance.push(OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    inverse.recipe.rules.contribution_queries = Some(expected_registry(false));
    inverse.provenance.pop().unwrap();
    assert!(
        inverse == *prior.input(),
        "whole-release inverse: only six memberships and provenance change"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
