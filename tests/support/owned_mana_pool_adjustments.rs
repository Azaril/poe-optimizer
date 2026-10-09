//! Data-only Player Mana adjustment inputs. Empty domains guard the published
//! source inventory; they do not establish complete conversion mechanics.
use poe_optimizer_core::{owned_content::digest_owned, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
const KIND: &str = "mana-pool-adjustments";
const FILES: [&str; 5] = [
    "authoring.json",
    "migration.json",
    "consumer.json",
    "dependencies.json",
    "bindings.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/mana-pool-adjustments")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
#[derive(Deserialize)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub queries: Vec<ContributionQuery>,
}
#[derive(Deserialize)]
struct Dependencies {
    definitions: Vec<DefinitionDescriptor>,
    query_registry_closure: SchemaClosure,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
fn census(
    owners: &[DefinitionRules],
    applications: &[EffectApplicationRule],
    queries: &[ContributionQuery],
) {
    for e in owners
        .iter()
        .flat_map(|o| &o.programs.members)
        .flat_map(|p| &p.effects)
        .chain(applications.iter().flat_map(|a| &a.program.effects))
    {
        if let RuleEffectKind::Contribute { stat, .. } = &e.effect {
            assert!(
                !queries.iter().any(|q| q.stat == *stat),
                "new Mana adjustment writer needs reviewed membership"
            );
        }
    }
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let m = migration();
    let c = consumer();
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V25
    );
    assert_eq!(m.schema.len(), 5);
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(
        (c.owners.len(), c.receivers.len(), c.queries.len()),
        (5, 5, 5)
    );
    for name in FILES.iter().skip(1) {
        let b = fs::read(data().join(name)).unwrap();
        assert_eq!(a["artifacts"][name]["bytes"], b.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(b))
        );
    }
    for q in &c.queries {
        assert_eq!(q.contribution, ContributionKind::Add);
        assert_eq!(q.groups.len(), 1);
        let g = &q.groups[0];
        assert_eq!(g.reduction, ContributionReduction::Sum);
        assert_eq!(g.ordering, ContributionOrdering::Ordered);
        assert!(g.members.is_complete() && g.members.members.is_empty());
    }
    let b: Value = read("bindings.json");
    assert_eq!(b["scope"]["complete_conversion_mechanics"], false);
    assert_eq!(b["scope"]["final_mana"], false);
    census(&c.owners, &[], &c.queries);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let deps: Dependencies = read("dependencies.json");
    let c = consumer();
    let r = &prior.input().recipe;
    for d in deps.definitions {
        assert!(r.schema.definitions.contains(&d));
    }
    assert_eq!(
        r.rules.contribution_queries.as_ref().unwrap().closure,
        deps.query_registry_closure
    );
    census(
        &r.rules.owners,
        &r.rules.effect_applications.as_ref().unwrap().members,
        &c.queries,
    );
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    input.recipe.rules.owners.extend(c.owners);
    input.recipe.rules.receivers.members.extend(c.receivers);
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    let c = consumer();
    for o in c.owners {
        let rows = &mut inverse.recipe.rules.owners;
        let at = rows.iter().position(|r| r.owner == o.owner).unwrap();
        assert_eq!(rows.remove(at), o);
    }
    for r in c.receivers {
        let rows = &mut inverse.recipe.rules.receivers.members;
        let at = rows.iter().position(|x| x.id == r.id).unwrap();
        assert_eq!(rows.remove(at), r);
    }
    for q in c.queries {
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
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only reviewed input reducers follow schema extension"
    );
    crate::migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}

/// Extend only the explicitly finite replay. No production coverage is repaired.
pub fn install(i: &mut super::replay::ReplayInput) {
    use poe_optimizer_core::owned_stages::*;
    use poe_optimizer_import::owned_recipe_extension::SchemaExtensionEntry;
    for row in migration().schema {
        let SchemaExtensionEntry::Definition(d) = row else {
            panic!()
        };
        i.schema.definitions.push(d);
    }
    let c = consumer();
    for o in &c.owners {
        for p in &o.programs.members {
            super::stage(
                i,
                o.owner.clone(),
                p,
                "resolve-intelligence-inherent-disabled",
            );
        }
        i.rules.support_discovery.as_mut().unwrap().providers.push(
            SupportSourceDomainDeclaration {
                owner: o.owner.clone(),
                domain: SchemaState::Known(SupportSourceDomain::AuthoredAssignmentsOnly),
            },
        );
    }
    let template=i.stages.frozen_channels.iter().find(|f|matches!(&f.channel,StageChannel::Contributions{stat,..} if *stat==super::def(0x3355))).unwrap().clone();
    for q in &c.queries {
        let mut row = template.clone();
        let StageChannel::Contributions {
            stat, contribution, ..
        } = &mut row.channel
        else {
            unreachable!()
        };
        *stat = q.stat.clone();
        *contribution = q.contribution;
        i.stages.frozen_channels.push(row);
    }
    i.rules.owners.extend(c.owners);
    i.rules.receivers.members.extend(c.receivers);
    i.rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(c.queries);
    i.rebind_test_edit().unwrap();
}
#[test]
fn mana_adjustment_data_preserves_five_guarded_empty_domains() {
    check_authored();
}
