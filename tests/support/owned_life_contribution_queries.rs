//! Canonical Life query data. Six groups have zero/one-effect domains; equipment
//! and global gaps remain. No final consumer, measured input or source runtime.
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "life-contribution-queries";
const FILES: [&str; 5] = [
    "authoring.json",
    "bindings.json",
    "dependencies.json",
    "migration.json",
    "queries.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/life-contribution-queries")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    OwnedDefinitionKey::new(s).unwrap()
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap()
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
#[derive(Deserialize)]
struct Dependencies {
    owners: Vec<DefinitionRules>,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
    existing_actor_rules: DeclaredSet<ExistingActorRuleApplication>,
    query_registry_closure: SchemaClosure,
}
pub fn definitions() -> Vec<DefinitionDescriptor> {
    read::<Dependencies>("dependencies.json").definitions
}

fn check_queries(queries: &[ContributionQuery]) {
    assert_eq!(queries.len(), 3);
    let b: Value = read("bindings.json");
    let mut total = 0;
    for (i, query) in queries.iter().enumerate() {
        assert_eq!(json!(query.stat), b["life"]);
        assert_eq!(json!(query.id), b["query_ids"][i]);
        assert_eq!(
            query.contribution,
            [
                ContributionKind::Add,
                ContributionKind::Increase,
                ContributionKind::Multiply
            ][i]
        );
        assert_eq!(query.groups.len(), [4, 2, 1][i]);
        for group in &query.groups {
            if group.id == key("equipment") {
                // Equipment may contain several effective amounts. Its actual
                // numerical/order domain still requires a separate proof.
                let SchemaClosure::Partial { gaps } = &group.members.closure else {
                    panic!("equipment Life contributor and order proof is still pending")
                };
                assert_eq!(gaps.len(), 1);
                assert_eq!(
                    gaps[0].subject,
                    SchemaSubject::Definition(query.stat.address())
                );
                assert_eq!(gaps[0].facet, SchemaFacet::GameRules);
                assert_eq!(
                    gaps[0].code,
                    key("life-contributor-domain-and-order-unproved")
                );
            } else {
                assert!(group.members.is_complete());
                // Every member below has position (0,0,0,0,0). Candidate-wide
                // tie refusal therefore admits at most one potential effect
                // per recipient, even if currently inactive. No associativity
                // or source traversal law is needed for zero/one-element folds.
                assert!(group.members.members.iter().all(|m| !matches!(
                    m.producer.as_program_effect().unwrap().origin,
                    ContributionOrigin::EquipmentUse { .. }
                        | ContributionOrigin::ItemModifier { .. }
                )));
            }
            assert_eq!(group.ordering, ContributionOrdering::Ordered);
            assert!(!group.members.members.is_empty());
            total += group.members.members.len();
            for member in &group.members.members {
                let order = member.order.as_ref().unwrap();
                assert_eq!(
                    (order.source_rank, order.program_rank, order.effect_rank),
                    (0, 0, 0)
                );
                if let ContributionOrigin::ItemModifier { slots } =
                    &member.producer.as_program_effect().unwrap().origin
                {
                    assert_eq!(json!(slots), b["equipment_slots"]);
                    assert_eq!(order.slot_ranks.len(), slots.len());
                    for (i, (slot, rank)) in slots.iter().zip(&order.slot_ranks).enumerate() {
                        assert_eq!(&rank.slot, slot);
                        assert_eq!(rank.rank as usize, i);
                    }
                } else {
                    assert!(order.slot_ranks.is_empty());
                }
            }
        }
    }
    assert_eq!(total, 8);
}

/// Freeze complete donor program bodies and inspect all potential Life contributions,
/// including inactive effects and unrelated selected/unselected definitions.
fn census(
    owners: &[DefinitionRules],
    applications: &[EffectApplicationRule],
    queries: &[ContributionQuery],
) {
    let expected: Dependencies = read("dependencies.json");
    let mut found = BTreeSet::new();
    for owner in owners {
        for program in &owner.programs.members {
            for effect in &program.effects {
                let RuleEffectKind::Contribute {
                    stat, contribution, ..
                } = &effect.effect
                else {
                    continue;
                };
                if !queries.iter().any(|q| q.stat == *stat) {
                    continue;
                }
                let frozen = expected
                    .owners
                    .iter()
                    .find(|o| o.owner == owner.owner)
                    .expect("new Life donor needs review");
                let frozen_program = frozen
                    .programs
                    .members
                    .iter()
                    .find(|p| p.id == program.id)
                    .expect("new Life program needs review");
                assert_eq!(
                    program, frozen_program,
                    "Life donor body, guard or recipient changed"
                );
                // Unrelated Mana programs may share this Actor owner. Every
                // potential Life writer is still censused, selected or not.
                let rows: Vec<_> = queries
                    .iter()
                    .filter(|q| q.stat == *stat && q.contribution == *contribution)
                    .flat_map(|q| &q.groups)
                    .flat_map(|g| &g.members.members)
                    .filter(|m| {
                        m.producer.as_program_effect().unwrap().owner == owner.owner
                            && m.producer.as_program_effect().unwrap().program == program.id
                            && m.producer.as_program_effect().unwrap().effect == effect.id
                    })
                    .collect();
                assert_eq!(
                    rows.len(),
                    1,
                    "every potential Life effect needs exact membership"
                );
                assert!(found.insert((
                    json!(owner.owner).to_string(),
                    program.id.clone(),
                    effect.id.clone()
                )));
            }
        }
    }
    assert_eq!(found.len(), 8);
    for application in applications {
        assert!(!application.program.effects.iter().any(|e| matches!(&e.effect, RuleEffectKind::Contribute {stat,..} if queries.iter().any(|q| q.stat == *stat))),
            "Life application origin requires its own reviewed authority");
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["scope"]["complete_groups"], 6);
    assert_eq!(a["scope"]["singleton_reductions"], true);
    assert_eq!(a["scope"]["numerical_order_proved"], false);
    assert_eq!(a["before"], b["before"]);
    assert_eq!(json!(m.before), b["before"]);
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V23)
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.owners.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    for name in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(&bytes))
        );
    }
    let queries = queries();
    check_queries(&queries);
    let deps: Dependencies = read("dependencies.json");
    assert_eq!(deps.owners.len(), 5);
    census(&deps.owners, &[], &queries);
    assert!(deps.existing_actor_rules.is_complete());
    assert!(!matches!(
        deps.query_registry_closure,
        SchemaClosure::Complete
    ));
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    assert_component_owners(
        endpoint,
        &endpoint.input().recipe.rules.owners,
        &endpoint
            .input()
            .recipe
            .rules
            .contribution_queries
            .as_ref()
            .unwrap()
            .members,
    );
}
/// A later data packet must authenticate its exact donor rewrite before using
/// this comparison. Only that reviewed body is restored for the historical
/// membership proof; the actual successor is what the native fixture executes.
#[allow(dead_code)]
pub fn assert_component_with_reviewed_donor(
    endpoint: &StagedOwnedRelease,
    before: &DefinitionRules,
    after: &DefinitionRules,
    before_query: &ContributionQuery,
    after_query: &ContributionQuery,
) {
    assert_component_with_reviewed_donors(endpoint, &[(before, after, before_query, after_query)]);
}

/// Exact authenticated changes, ordered from the current successor back to the
/// original packet. Every intermediate endpoint must match before its inverse
/// is applied; only the historical comparison uses the restored copies.
pub type ReviewedLifeDonor<'a> = (
    &'a DefinitionRules,
    &'a DefinitionRules,
    &'a ContributionQuery,
    &'a ContributionQuery,
);

fn invert_reviewed_donors(
    owners: &mut [DefinitionRules],
    registry: &mut [ContributionQuery],
    reviewed: &[ReviewedLifeDonor<'_>],
) {
    let deps: Dependencies = read("dependencies.json");
    let (oldest_owner, _, oldest_query, _) = reviewed.last().expect("reviewed Life change");
    // Preserve the original donor/query boundary, rather than accepting an
    // arbitrary caller-supplied endpoint as the historical proof.
    assert!(deps.owners.contains(oldest_owner));
    let frozen_queries = queries();
    assert!(frozen_queries.contains(oldest_query));
    for (before, after, before_query, after_query) in reviewed {
        assert!(deps.owners.iter().any(|o| o.owner == before.owner));
        assert_eq!(before.owner, after.owner);
        let current = owners.iter_mut().find(|o| o.owner == before.owner).unwrap();
        assert_eq!(current, *after, "reviewed Life successor donor changed");
        *current = (*before).clone();
        assert_eq!(before_query.id, after_query.id);
        assert!(frozen_queries.iter().any(|q| q.id == before_query.id));
        let query = registry
            .iter_mut()
            .find(|q| q.id == after_query.id)
            .unwrap();
        assert_eq!(query, *after_query, "reviewed Life successor query changed");
        *query = (*before_query).clone();
    }
}

#[allow(dead_code)]
pub fn assert_component_with_reviewed_donors(
    endpoint: &StagedOwnedRelease,
    reviewed: &[ReviewedLifeDonor<'_>],
) {
    let mut owners = endpoint.input().recipe.rules.owners.clone();
    let mut registry = endpoint
        .input()
        .recipe
        .rules
        .contribution_queries
        .as_ref()
        .unwrap()
        .members
        .clone();
    invert_reviewed_donors(&mut owners, &mut registry, reviewed);
    assert_component_owners(endpoint, &owners, &registry);
}
fn assert_component_owners(
    endpoint: &StagedOwnedRelease,
    owners: &[DefinitionRules],
    reviewed_queries: &[ContributionQuery],
) {
    check_authored();
    let recipe = &endpoint.input().recipe;
    assert!(
        RuleOperationsVersion::parse(recipe.rules.operations_version.as_str())
            .unwrap()
            .supports_actor_reward_contributions()
    );
    let deps: Dependencies = read("dependencies.json");
    for definition in deps.definitions {
        assert!(recipe.schema.definitions.contains(&definition));
    }
    for slot in deps.slots {
        assert!(recipe.schema.slots.contains(&slot));
    }
    assert_eq!(
        recipe.rules.existing_actor_rules,
        Some(deps.existing_actor_rules)
    );
    let registry = recipe.rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, deps.query_registry_closure);
    for query in queries() {
        assert_eq!(reviewed_queries.iter().filter(|q| **q == query).count(), 1);
    }
    census(
        owners,
        &recipe.rules.effect_applications.as_ref().unwrap().members,
        &queries(),
    );
    assert!(
        endpoint
            .receipt()
            .provenance
            .iter()
            .any(|p| p.kind == key(KIND) && p.authoring_input == digest())
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let b: Value = read("bindings.json");
    assert_eq!(json!(prior.receipt().input), b["before"]);
    assert_eq!(json!(prior.receipt().definitions), b["definitions"]);
    assert_eq!(json!(prior.receipt().rules), b["rules"]);
    assert_eq!(json!(prior.receipt().registry), b["registry"]);
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
        &queries(),
    );
    let contract =
        compile_owned_release_migration(prior, read("migration.json"), Default::default()).unwrap();
    let mut input = contract.input().clone();
    let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
    for query in queries() {
        assert!(!registry.members.iter().any(|q| q.id == query.id));
        registry.members.push(query);
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for query in queries().into_iter().rev() {
        assert_eq!(
            inverse
                .recipe
                .rules
                .contribution_queries
                .as_mut()
                .unwrap()
                .members
                .pop(),
            Some(query)
        );
    }
    inverse.provenance = contract.input().provenance.clone();
    assert!(
        inverse == *contract.input(),
        "only three queries and a replaced authoring receipt follow the contract migration"
    );
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}

#[test]
fn current_life_query_data_accounts_for_eight_writers_and_six_singleton_domains() {
    check_authored();
}
#[test]
fn missing_inactive_changed_and_unreviewed_life_members_cannot_hide_in_a_partial_group() {
    let d: Dependencies = read("dependencies.json");
    let mut q = queries();
    q[0].groups[0].members.members.pop();
    assert!(std::panic::catch_unwind(|| census(&d.owners, &[], &q)).is_err());
    let mut owners = d.owners.clone();
    let program = owners
        .iter_mut()
        .flat_map(|o| &mut o.programs.members)
        .find(|p| p.id == key("intrinsic-player-life"))
        .unwrap();
    program.effects[0].when = Some(key("not-active"));
    assert!(std::panic::catch_unwind(|| census(&owners, &[], &queries())).is_err());
    let mut q = queries();
    q[0].groups[2].members.closure = SchemaClosure::Complete;
    assert!(std::panic::catch_unwind(|| check_queries(&q)).is_err());
}

#[test]
fn reviewed_minion_append_requires_exact_donor_and_query_before_the_full_census() {
    let deps: Dependencies = read("dependencies.json");
    let before_query = queries().remove(0);
    let inherent = before_query
        .groups
        .iter()
        .find(|g| g.id == key("inherent"))
        .unwrap();
    let original_member = inherent.members.members[0].clone();
    let existing = original_member.producer.as_program_effect().unwrap();
    let bridge = deps
        .owners
        .iter()
        .find(|o| o.owner == existing.owner)
        .unwrap()
        .programs
        .members
        .iter()
        .find(|p| p.id == existing.program)
        .unwrap()
        .clone();
    let intrinsic = before_query
        .groups
        .iter()
        .find(|g| g.id == key("intrinsic"))
        .unwrap();
    let supplied = intrinsic
        .members
        .members
        .iter()
        .map(|m| m.producer.as_program_effect().unwrap())
        .find(|p| matches!(p.origin, ContributionOrigin::SuppliedActor { .. }))
        .unwrap();
    let before = deps
        .owners
        .iter()
        .find(|o| o.owner == supplied.owner)
        .unwrap()
        .clone();
    let mut after = before.clone();
    after.programs.members.push(bridge);
    let mut after_query = before_query.clone();
    let mut member = original_member.clone();
    let producer = member.producer.as_program_effect_mut().unwrap();
    producer.owner = supplied.owner.clone();
    producer.origin = supplied.origin.clone();
    after_query
        .groups
        .iter_mut()
        .find(|g| g.id == key("inherent"))
        .unwrap()
        .members
        .members
        .push(member);
    let selected_owners = || {
        let mut owners = deps.owners.clone();
        *owners.iter_mut().find(|o| o.owner == before.owner).unwrap() = after.clone();
        owners
    };
    let selected_queries = || {
        let mut registry = queries();
        registry[0] = after_query.clone();
        registry
    };
    let reviewed = [(&before, &after, &before_query, &after_query)];
    let mut owners = selected_owners();
    let mut registry = selected_queries();
    invert_reviewed_donors(&mut owners, &mut registry, &reviewed);
    assert_eq!(owners, deps.owners);
    assert_eq!(registry, queries());
    census(&owners, &[], &registry);

    // A modified recipient/guard or missing supplied-Actor member cannot be
    // removed under an authentic packet's expected inverse.
    for operation in 0..3 {
        let mut owners = selected_owners();
        let mut registry = selected_queries();
        if operation == 2 {
            registry[0]
                .groups
                .iter_mut()
                .find(|g| g.id == key("inherent"))
                .unwrap()
                .members
                .members
                .pop();
        } else {
            let donor = owners.iter_mut().find(|o| o.owner == before.owner).unwrap();
            let effect = &mut donor.programs.members.last_mut().unwrap().effects[0];
            if operation == 0 {
                let RuleEffectKind::Contribute { entity, .. } = &mut effect.effect else {
                    panic!()
                };
                *entity = RuleEntity::Player;
            } else {
                effect.when = Some(key("unreviewed-disabled-guard"));
            }
        }
        assert!(
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                invert_reviewed_donors(&mut owners, &mut registry, &reviewed);
            }))
            .is_err()
        );
    }

    // An unrelated unreviewed writer survives the exact inverse and is still
    // rejected by the complete historical census, even with an inactive guard.
    let mut owners = selected_owners();
    let mut registry = selected_queries();
    let owner = owners
        .iter_mut()
        .find(|o| o.owner == existing.owner)
        .unwrap();
    let mut extra = owner
        .programs
        .members
        .iter()
        .find(|p| p.id == existing.program)
        .unwrap()
        .clone();
    extra.id = key("unreviewed-life-writer");
    extra.effects[0].when = Some(key("inactive"));
    owner.programs.members.push(extra);
    invert_reviewed_donors(&mut owners, &mut registry, &reviewed);
    assert!(std::panic::catch_unwind(|| census(&owners, &[], &registry)).is_err());
}
