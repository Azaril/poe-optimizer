//! Offline data publication for inherited and post-stacking increased damage.
//! This adds no runtime evaluator and grants no complete damage-domain coverage.
use super::{migration_preservation, plain_damage_source as source_family};
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::*,
    owned_rules::*,
    owned_schema::*,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "mixed-minion-damage";
pub const PROGRAM: &str = "owned-minion-unconditional-damage-increase";
pub const FACTOR: &str = "owned-minion-unconditional-increase-factor";
pub const QUERY: &str = "owned-minion-applied-damage-increase";
pub const MORE: &str = "owned-minion-unconditional-more-factor";
pub const MORE_QUERY: &str = "owned-minion-unconditional-more-contributions";
const FILES: [&str; 6] = [
    "authoring.json",
    "migration.json",
    "consumer.json",
    "queries.json",
    "dependencies.json",
    "source-vectors.json",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/mixed-minion-damage")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    before: OwnedContentDigest,
    pub definitions: Vec<DefinitionDescriptor>,
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
    pub applications: DeclaredSet<EffectApplicationRule>,
    pub query_registry_closure: SchemaClosure,
    pub source_programs: Vec<SourceProgram>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceProgram {
    pub owner: SchemaSubject,
    pub program: RuleProgram,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
    pub receivers: Vec<StatReceiver>,
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let deps: Dependencies = read("dependencies.json");
    let m = migration();
    let c = consumer();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["before"], json!(deps.before));
    assert_eq!(m.before, deps.before);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V25)
    );
    assert_eq!(
        a["scope"],
        json!({"new_definitions":2,"new_programs":3,"new_receivers":3,"new_queries":2,"closed_existing_owners":0,"complete_damage_domain":false,"whole_build_parity":false})
    );
    for name in FILES.iter().skip(1) {
        let b = fs::read(data().join(name)).unwrap();
        assert!(!b.contains(&b'\r') && !b.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(a["artifacts"][name]["bytes"], b.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(b))
        );
    }
    for pin in a["source_packets"].as_array().unwrap() {
        let b =
            fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(pin["path"].as_str().unwrap()))
                .unwrap();
        assert_eq!(pin["bytes"], b.len());
        assert_eq!(pin["sha256"], format!("{:x}", Sha256::digest(b)));
    }
    assert_eq!(m.schema.len(), 2);
    assert!(m.tables.is_empty() && m.query_targets.is_empty() && m.evaluation.is_none());
    assert!(m.owners.is_empty() && m.receivers.is_empty());
    assert_eq!(c.owners.len(), 3);
    assert_eq!(c.receivers.len(), 3);
    for (owner, receiver) in c.owners.iter().zip(&c.receivers) {
        assert!(owner.programs.is_complete());
        assert_eq!(owner.programs.members.len(), 1);
        assert_eq!(receiver.program, owner.programs.members[0].id);
        assert_eq!(receiver.targets, deps.receivers[1].targets);
        assert_eq!(owner.programs.members[0].effects.len(), 1);
    }
    let p = &c.owners[0].programs.members[0];
    assert_eq!(p.id, key(PROGRAM));
    assert_eq!(p.context, RuleEntityKind::Actor);
    assert_eq!(p.reads.len(), 2);
    assert!(
        matches!(&p.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Current, stat } if stat.key().as_str()=="def.0000000000001d34")
    );
    assert!(
        matches!(&p.reads[1].source, RuleReadSource::ContributionQuery { entity: RuleEntity::Current, query, group } if *query==key(QUERY) && *group==key("all"))
    );
    // Literals express identity and the reviewed decimal rounding boundary.
    // Build levels, passive amounts, Offering strength and quality remain inputs.
    let literals: Vec<_> = c
        .owners
        .iter()
        .flat_map(|o| &o.programs.members)
        .flat_map(|p| &p.nodes)
        .filter_map(|n| match &n.expression {
            RuleExpression::Literal { value } => Some(json!(value)),
            _ => None,
        })
        .collect();
    assert_eq!(
        literals
            .iter()
            .map(|v| v["value"]["value"].as_f64().unwrap())
            .collect::<Vec<_>>(),
        [1., 100., 0.5]
    );
    assert_eq!(c.owners[1].programs.members[0].id, key(FACTOR));
    let qs = queries();
    assert_eq!(qs.len(), 2);
    assert_eq!(qs[0].id, key(QUERY));
    assert_eq!(qs[0].contribution, ContributionKind::Add);
    assert_eq!(qs[0].groups.len(), 1);
    let group = &qs[0].groups[0];
    assert!(group.members.is_complete());
    assert_eq!(group.members.members.len(), 1);
    let ContributionProducer::ApplicationGroup(g) = &group.members.members[0].producer else {
        panic!("post-stacking ownership")
    };
    assert_eq!(g.declarations.len(), 1);
    assert!(!deps.applications.is_complete());
    assert_eq!(deps.applications.members.len(), 1);
    let app = &deps.applications.members[0];
    assert_eq!(g.declarations[0].application, app.id);
    let mapping = app
        .stacking
        .iter()
        .find(|s| s.effect == g.declarations[0].effect)
        .unwrap();
    assert_eq!(
        (&g.family, &g.modifier),
        (&mapping.family, &mapping.modifier)
    );
    let more = &c.owners[2].programs.members[0];
    assert_eq!(more.id, key(MORE));
    assert_eq!(more.context, RuleEntityKind::Actor);
    assert!(
        matches!(&more.reads[0].source, RuleReadSource::Stat { entity: RuleEntity::Current, stat } if stat.key().as_str() == "def.000000000000001d")
    );
    assert!(
        matches!(&more.reads[1].source, RuleReadSource::ContributionQuery { entity: RuleEntity::Current, query, group } if *query == key(MORE_QUERY) && *group == key("sources"))
    );
    assert_eq!(qs[1].id, key(MORE_QUERY));
    assert_eq!(qs[1].stat.key().as_str(), "def.000000000000330b");
    assert_eq!(qs[1].contribution, ContributionKind::Multiply);
    assert_eq!(qs[1].groups.len(), 1);
    let group = &qs[1].groups[0];
    assert_eq!(group.reduction, ContributionReduction::Product);
    assert!(group.members.is_complete());
    assert_eq!(group.members.members.len(), 1);
    let producer = group.members.members[0]
        .producer
        .as_program_effect()
        .unwrap();
    assert_eq!(producer.program, key("gigantic-life-and-damage"));
    assert_eq!(producer.effect, key("damage_more"));
    assert!(
        matches!(&producer.origin, ContributionOrigin::SuppliedActor { slots } if slots.len() == 1)
    );
    check_source(false);
}

pub fn vectors() -> Vec<Value> {
    read::<Value>("source-vectors.json")["vectors"]
        .as_array()
        .unwrap()
        .clone()
}
pub fn check_source(full: bool) {
    let report = full.then(|| source_family::source_proof(&source_family::read("authoring.json")));
    let vs = vectors();
    assert_eq!(vs.len(), 12);
    for v in &vs {
        let records = v["records"].as_array().unwrap();
        let mut total = 0.;
        let mut offering = 0;
        for r in records {
            let m = &r["mod"];
            assert_eq!(m["name"], "Damage");
            assert_eq!(m["type"], "INC");
            assert_eq!(m["flags"], 0);
            assert_eq!(m["keyword_flags"], 0);
            assert_eq!(r["value"], m["value"]);
            let origin = m["source"].as_str().unwrap();
            if origin == "Skill:PainOfferingPlayer" {
                offering += 1;
            } else {
                assert!(origin.starts_with("Tree:"));
            }
            total += r["value"].as_f64().unwrap();
        }
        assert!(offering <= 1);
        assert_eq!(1. + total / 100., v["increased_factor"].as_f64().unwrap());
        // The selected sources share one reviewed unconditional-damage rounding
        // domain. This is not a universal rounding rule for every MORE channel.
        let more = v["more_records"].as_array().unwrap();
        let mut product = 1.;
        for row in more {
            let m = &row["mod"];
            assert_eq!(m["name"], "Damage");
            assert_eq!(m["type"], "MORE");
            assert_eq!(m["flags"], 0);
            assert_eq!(m["keyword_flags"], 0);
            assert_eq!(row["value"], m["value"]);
            assert!(
                ["Gigantic", "Skill:SummonSkeletalSnipersPlayer"]
                    .contains(&m["source"].as_str().unwrap())
            );
            product *= 1. + row["value"].as_f64().unwrap() / 100.;
        }
        assert_eq!(
            (product * 100. + 0.5).floor() / 100.,
            v["more_factor"].as_f64().unwrap()
        );
        if let Some(report) = &report {
            let c = report["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == v["case"])
                .unwrap();
            assert_eq!(c["available"], true);
            assert_eq!(c["xml_sha256"], v["xml_sha256"]);
            let a = c["state"]["main"]["actors"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["summon_effect_id"] == "SummonSkeletalSnipersPlayer")
                .unwrap();
            assert_eq!(a["actor_profile"], v["actor_profile"]);
            let action = a["children"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["effect_id"] == v["effect_id"])
                .unwrap();
            let calls: Vec<_> = action["damage_calls"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|q| q["damage_type"] == "Physical" && q["critical"] == false)
                .collect();
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0]["query_state_preserved"], true);
            assert_eq!(calls[0]["increased_records"], v["records"]);
            assert_eq!(calls[0]["increased_factor"], v["increased_factor"]);
            let source_more = &calls[0]["more_records"];
            if more.is_empty() {
                // Normalize the observer's empty Lua table only at acquisition.
                assert_eq!(source_more, &json!({}));
            } else {
                assert_eq!(source_more, &v["more_records"]);
            }
            assert_eq!(calls[0]["more_factor"], v["more_factor"]);
        }
    }
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let m = migration();
    let c = consumer();
    let deps: Dependencies = read("dependencies.json");
    let r = &endpoint.input().recipe;
    assert!(
        RuleOperationsVersion::parse(r.rules.operations_version.as_str())
            .unwrap()
            .supports_application_group_contributions()
    );
    for d in deps.definitions {
        assert!(r.schema.definitions.contains(&d));
    }
    for s in m.schema {
        let SchemaExtensionEntry::Definition(d) = s else {
            panic!()
        };
        assert!(r.schema.definitions.contains(&d));
    }
    for owner in c.owners.into_iter().chain(deps.owners) {
        assert!(r.rules.owners.contains(&owner));
    }
    for source in deps.source_programs {
        let owner = r
            .rules
            .owners
            .iter()
            .find(|o| o.owner == source.owner)
            .unwrap();
        assert!(owner.programs.members.contains(&source.program));
    }
    for receiver in c.receivers.into_iter().chain(deps.receivers) {
        assert!(r.rules.receivers.members.contains(&receiver));
    }
    assert_eq!(
        r.rules.effect_applications.as_ref(),
        Some(&deps.applications)
    );
    let registry = r.rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, deps.query_registry_closure);
    for q in queries() {
        assert!(registry.members.contains(&q));
    }
    assert_eq!(
        endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND)
                && p.prior_input == deps.before
                && p.authoring_input == digest())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let m = migration();
    assert_eq!(prior.receipt().input, m.before);
    let contract = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = contract.input().clone();
    let c = consumer();
    for owner in c.owners {
        assert!(
            !input
                .recipe
                .rules
                .owners
                .iter()
                .any(|old| old.owner == owner.owner)
        );
        input.recipe.rules.owners.push(owner);
    }
    for receiver in c.receivers {
        assert!(
            !input
                .recipe
                .rules
                .receivers
                .members
                .iter()
                .any(|old| old.id == receiver.id)
        );
        input.recipe.rules.receivers.members.push(receiver);
    }
    for q in queries() {
        let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
        assert!(!registry.members.iter().any(|old| old.id == q.id));
        registry.members.push(q);
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    let c = consumer();
    for owner in c.owners.into_iter().rev() {
        let rows = &mut inverse.recipe.rules.owners;
        let at = rows.iter().position(|r| r.owner == owner.owner).unwrap();
        assert_eq!(rows.remove(at), owner);
    }
    for receiver in c.receivers.into_iter().rev() {
        let rows = &mut inverse.recipe.rules.receivers.members;
        let at = rows.iter().position(|r| r.id == receiver.id).unwrap();
        assert_eq!(rows.remove(at), receiver);
    }
    for q in queries().into_iter().rev() {
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
    inverse.provenance = contract.input().provenance.clone();
    assert!(
        inverse == *contract.input(),
        "only reviewed consumer and query follow checked migration"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
