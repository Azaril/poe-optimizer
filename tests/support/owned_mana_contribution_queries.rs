//! Checked membership for every currently published Mana writer. These guards
//! preserve global gaps; they do not certify all game Mana mechanics.
use poe_optimizer_core::{
    owned_build::ParameterValue, owned_content::digest_owned, owned_rules::*, owned_schema::*,
};
use poe_optimizer_import::owned_release::{
    OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release,
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

const KIND: &str = "mana-contribution-queries";
const FILES: [&str; 3] = ["authoring.json", "queries.json", "dependencies.json"];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/mana-contribution-queries")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
#[derive(Clone, Deserialize)]
pub struct Producer {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
#[derive(Deserialize)]
pub struct Dependencies {
    pub producers: Vec<Producer>,
    pub definitions: Vec<DefinitionDescriptor>,
    existing_actor_rules: DeclaredSet<ExistingActorRuleApplication>,
    query_registry_closure: SchemaClosure,
}

fn census(
    owners: &[DefinitionRules],
    applications: &[EffectApplicationRule],
    q: &[ContributionQuery],
) {
    let deps: Dependencies = read("dependencies.json");
    let mut found = BTreeSet::new();
    for o in owners {
        for p in &o.programs.members {
            for e in &p.effects {
                let RuleEffectKind::Contribute {
                    stat, contribution, ..
                } = &e.effect
                else {
                    continue;
                };
                if *stat != q[0].stat {
                    continue;
                }
                let donor = deps
                    .producers
                    .iter()
                    .find(|d| d.owner == o.owner && d.program.id == p.id)
                    .expect("new Mana producer requires reviewed membership");
                assert_eq!(p, &donor.program, "guard, value or recipient changed");
                let members: Vec<_> = q
                    .iter()
                    .filter(|q| q.stat == *stat && q.contribution == *contribution)
                    .flat_map(|q| &q.groups)
                    .flat_map(|g| &g.members.members)
                    .filter(|m| {
                        m.producer.as_program_effect().is_some_and(|x| {
                            x.owner == o.owner && x.program == p.id && x.effect == e.id
                        })
                    })
                    .collect();
                assert_eq!(
                    members.len(),
                    1,
                    "every potential effect needs exactly one member"
                );
                assert!(found.insert((json!(o.owner).to_string(), p.id.clone(), e.id.clone())));
            }
        }
    }
    assert_eq!(found.len(), 5);
    assert_eq!(
        q.iter()
            .flat_map(|q| &q.groups)
            .map(|g| g.members.members.len())
            .sum::<usize>(),
        5
    );
    for a in applications {
        assert!(
            !a.program.effects.iter().any(|e| matches!(&e.effect,
            RuleEffectKind::Contribute{stat,..} if *stat == q[0].stat)),
            "Mana application sources require explicit reviewed authority"
        );
    }
}
fn donor_owners(deps: &Dependencies) -> Vec<DefinitionRules> {
    let mut owners: Vec<DefinitionRules> = Vec::new();
    for d in &deps.producers {
        if let Some(o) = owners.iter_mut().find(|o| o.owner == d.owner) {
            o.programs.members.push(d.program.clone());
        } else {
            owners.push(DefinitionRules {
                owner: d.owner.clone(),
                programs: DeclaredSet::complete(vec![d.program.clone()]),
            });
        }
    }
    owners
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    for name in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    let q = queries();
    assert_eq!(q.len(), 3);
    assert_eq!(
        q.iter().map(|q| q.groups.len()).collect::<Vec<_>>(),
        [2, 2, 1]
    );
    for g in q.iter().flat_map(|q| &q.groups) {
        assert!(g.members.is_complete());
        assert_eq!(g.ordering, ContributionOrdering::Ordered);
        for (n, member) in g.members.members.iter().enumerate() {
            let order = member.order.as_ref().unwrap();
            assert_eq!(order.source_rank as usize, n);
            assert_eq!(order.program_rank, 0);
            assert_eq!(order.effect_rank, 0);
            assert!(order.slot_ranks.is_empty());
        }
    }
    assert!(q[2].groups[0].members.members.is_empty());
    let deps: Dependencies = read("dependencies.json");
    // Each passive's immutable literal is integral. At most one allocation per
    // semantic position survives candidate-wide tie checks, even if inactive.
    // All subset sums of {-10,-30} are exactly representable; either order is
    // exact. The other nonempty groups admit zero/one potential effect.
    for (suffix, amount) in [("109b", -10.), ("18d8", -30.)] {
        let p = deps
            .producers
            .iter()
            .find(|p| {
                json!(p.owner)
                    .to_string()
                    .contains(&format!("000000000000{suffix}"))
            })
            .unwrap();
        let effect = p
            .program
            .effects
            .iter()
            .find(|e| matches!(&e.effect,RuleEffectKind::Contribute{stat,..} if *stat == q[0].stat))
            .unwrap();
        let RuleEffectKind::Contribute { value, .. } = &effect.effect else {
            unreachable!()
        };
        let expression = &p
            .program
            .nodes
            .iter()
            .find(|n| n.id == *value)
            .unwrap()
            .expression;
        let RuleExpression::Literal {
            value: ParameterValue::Quantity(v),
        } = expression
        else {
            panic!("bound requires literal")
        };
        assert_eq!(v.value(), amount);
    }
    census(&donor_owners(&deps), &[], &q);
    assert!(!matches!(
        deps.query_registry_closure,
        SchemaClosure::Complete
    ));
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    assert_eq!(a["before"], json!(prior.receipt().input));
    let deps: Dependencies = read("dependencies.json");
    let r = &prior.input().recipe;
    for d in &deps.definitions {
        assert!(r.schema.definitions.contains(d));
    }
    assert_eq!(
        r.rules.existing_actor_rules,
        Some(deps.existing_actor_rules)
    );
    assert_eq!(
        r.rules.contribution_queries.as_ref().unwrap().closure,
        deps.query_registry_closure
    );
    census(
        &r.rules.owners,
        &r.rules.effect_applications.as_ref().unwrap().members,
        &queries(),
    );
    assert_eq!(
        r.rules.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V25
    );
    let mut input = prior.input().clone();
    for q in queries() {
        let members = &mut input
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        assert!(!members.iter().any(|x| x.id == q.id));
        members.push(q);
    }
    input.provenance.push(OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    for q in queries() {
        let rows = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let at = rows.iter().position(|r| r.id == q.id).unwrap();
        assert_eq!(rows.remove(at), q);
    }
    inverse.provenance = prior.input().provenance.clone();
    assert!(
        inverse == *prior.input(),
        "only checked queries and provenance change"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
#[test]
fn mana_census_covers_all_five_potential_writers_with_bounded_folds() {
    check_authored();
}
#[test]
fn missing_duplicate_changed_and_new_mana_writers_fail_the_census() {
    let d: Dependencies = read("dependencies.json");
    let owners = donor_owners(&d);
    let mut q = queries();
    q[0].groups[0].members.members.clear();
    assert!(std::panic::catch_unwind(|| census(&owners, &[], &q)).is_err());
    let mut q = queries();
    q[1].groups[1].members.members.pop();
    assert!(
        std::panic::catch_unwind(|| census(&owners, &[], &q)).is_err(),
        "publication includes passive declarations even when the example does not allocate them"
    );
    let mut q = queries();
    let duplicate = q[0].groups[0].members.members[0].clone();
    q[0].groups[0].members.members.push(duplicate);
    assert!(std::panic::catch_unwind(|| census(&owners, &[], &q)).is_err());
    let mut changed = owners.clone();
    changed[0].programs.members[0].effects[0].when = Some("inactive".parse().unwrap());
    assert!(std::panic::catch_unwind(|| census(&changed, &[], &queries())).is_err());
    let mut changed = owners;
    let mut extra = changed[0].programs.members[0].clone();
    extra.id = "new-writer".parse().unwrap();
    changed[0].programs.members.push(extra);
    assert!(std::panic::catch_unwind(|| census(&changed, &[], &queries())).is_err());
}
