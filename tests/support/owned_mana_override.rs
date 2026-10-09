//! Offline adoption of the checked numeric-selection contract by actual Mana data.
use super::migration_preservation;
use poe_optimizer_core::{owned_content::digest_owned, owned_rules::*, owned_schema::*};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};
pub const KIND: &str = "mana-override";
pub const PROGRAM: &str = "set-mana-override";
const FILES: [&str; 5] = [
    "authoring.json",
    "migration.json",
    "queries.json",
    "dependencies.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/mana-override")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependencies {
    before: poe_optimizer_core::owned_content::OwnedContentDigest,
    pub owner: DefinitionRules,
    pub definitions: Vec<DefinitionDescriptor>,
    mapping: Value,
    query_registry_closure: SchemaClosure,
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let m = migration();
    let d: Dependencies = read("dependencies.json");
    let q = queries();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(m.before));
    assert_eq!(m.before, d.before);
    assert_eq!(
        m.contract.operations_version.as_str(),
        OWNED_RULE_OPERATIONS_V27
    );
    assert!(
        m.schema.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.owners.len(), 1);
    let o = &m.owners[0];
    assert_eq!(o.owner, d.owner.owner);
    assert_eq!(o.programs.closure, d.owner.programs.closure);
    assert!(!o.programs.is_complete() && d.owner.programs.members.is_empty());
    assert_eq!(o.programs.members.len(), 1);
    let p = &o.programs.members[0];
    assert_eq!(p.id.as_str(), PROGRAM);
    assert_eq!(p.context, RuleEntityKind::Actor);
    assert!(p.reads.is_empty());
    assert_eq!((p.nodes.len(), p.effects.len()), (1, 1));
    let RuleExpression::Literal {
        value: poe_optimizer_core::owned_build::ParameterValue::Quantity(v),
    } = &p.nodes[0].expression
    else {
        panic!("source is a literal quantity")
    };
    assert_eq!(v.value(), 0.);
    assert_eq!(v.unit().key().as_str(), "def.0000000000000003");
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].contribution, ContributionKind::Override);
    assert_eq!(q[0].groups.len(), 1);
    let g = &q[0].groups[0];
    assert_eq!(g.reduction, ContributionReduction::RequireAgreement);
    assert_eq!(g.ordering, ContributionOrdering::Unordered);
    assert!(g.empty.is_none() && g.members.is_complete());
    assert_eq!(g.members.members.len(), 1);
    let member = &g.members.members[0];
    assert!(member.order.is_none());
    let source = member.producer.as_program_effect().unwrap();
    assert_eq!(source.owner, o.owner);
    assert_eq!(source.program, p.id);
    assert_eq!(source.effect, p.effects[0].id);
    assert_eq!(source.origin, ContributionOrigin::Allocation);
    assert_eq!(
        a["scope"],
        json!({"new_definitions":0,"new_programs":1,"new_queries":1,"closed_owners":0,"final_mana":false,"whole_build":false})
    );
    for file in FILES.iter().skip(1) {
        let b = fs::read(data().join(file)).unwrap();
        assert_eq!(a["artifacts"][file]["bytes"], b.len());
        assert_eq!(a["artifacts"][file]["sha256"], hash(&b));
    }
    check_membership(&m.owners, &[]);
    check_source(false);
}
/// Census declarations, including unselected sources. Admitted writers must
/// match the reviewed body and recipient, not merely the selected build output.
pub fn check_membership(owners: &[DefinitionRules], applications: &[EffectApplicationRule]) {
    let expected = migration().owners.remove(0);
    let q = queries().remove(0);
    let mut count = 0;
    for o in owners {
        for p in &o.programs.members {
            for e in &p.effects {
                if matches!(&e.effect,RuleEffectKind::Contribute{stat,contribution,..} if *stat==q.stat && *contribution==q.contribution)
                {
                    assert_eq!(
                        o.owner, expected.owner,
                        "new Mana override owner needs reviewed membership"
                    );
                    assert_eq!(
                        p, &expected.programs.members[0],
                        "override body, activation or recipient changed"
                    );
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 1);
    for a in applications {
        assert!(!a.program.effects.iter().any(|e|matches!(&e.effect,RuleEffectKind::Contribute{stat,contribution,..} if *stat==q.stat && *contribution==q.contribution)), "application override sources require reviewed authority");
    }
}
fn check_source(full: bool) {
    let v: Value = read("source-vectors.json");
    let rows = v["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 16);
    assert_eq!(v["complete_loads_per_mode"], 19);
    for r in rows {
        let s = &r["source"]["override_source"];
        assert_eq!(s["node_id"], 51749);
        assert_eq!(s["tree_version"], "0_5");
        assert_eq!(s["parser_cache_preserved"], true);
        assert_eq!(
            s["descriptions"],
            json!([
                "You have no Mana",
                "Skill Mana Costs Converted to Life Costs"
            ])
        );
        let mods = s["modifiers"].as_array().unwrap();
        assert_eq!(mods.len(), 1);
        assert_eq!(
            mods[0],
            json!({"name":"Mana","type":"OVERRIDE","value":0,"flags":0,"keywordFlags":0})
        );
    }
    for (n, mana) in [876, 630, 881, 858, 638].iter().enumerate() {
        assert_eq!(rows[n]["source"]["final_mana"], *mana);
        assert_eq!(rows[n]["source"]["inputs"]["override"]["present"], false);
        assert_eq!(
            rows[n]["xml_sha256"],
            hash(
                &fs::read(root().join(format!(
                    "tests/fixtures/builds/breadth-20260908/build-{:02}.xml",
                    n + 1
                )))
                .unwrap()
            )
        );
    }
    for name in [
        "allocated-override",
        "override-zero",
        "override-zero-duplicates",
    ] {
        let r = rows.iter().find(|r| r["name"] == name).unwrap();
        assert_eq!(
            r["source"]["inputs"]["override"],
            json!({"present":true,"value":0})
        );
        assert_eq!(r["source"]["final_mana"], 0);
        let records: Vec<_> = r["source"]["read_set"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|db| db["Mana"].as_array().unwrap())
            .filter(|m| m["type"] == "OVERRIDE")
            .collect();
        assert_eq!(
            records.len(),
            if name == "override-zero-duplicates" {
                2
            } else {
                1
            }
        );
        if name == "allocated-override" {
            assert_eq!(r["source"]["override_source"]["allocated"], true);
            assert_eq!(records[0]["source"], "Tree:51749");
        }
    }
    assert_eq!(rows[4]["source"], rows[14]["source"]);
    assert_eq!(rows[4]["source"], rows[15]["source"]);
    if full {
        let a: Value = read("authoring.json");
        for pin in a["source_packets"]
            .as_array()
            .unwrap()
            .iter()
            .chain(v["reports"].as_array().unwrap())
        {
            let b = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(pin["bytes"], b.len());
            assert_eq!(pin["sha256"], hash(&b));
        }
        assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
        for pin in v["reports"].as_array().unwrap() {
            let r: Value = serde_json::from_slice(
                &fs::read(root().join(pin["path"].as_str().unwrap())).unwrap(),
            )
            .unwrap();
            assert_eq!(r["vectors"], v["vectors"]);
            for key in [
                "source_revision",
                "observer_sha256",
                "driver_sha256",
                "bootstrap_sha256",
            ] {
                assert_eq!(r[key], v[key]);
            }
            for c in r["cases"].as_array().unwrap() {
                assert_eq!(
                    c["observed"]["state"]["modes"]["MAIN"],
                    c["observed"]["state"]["modes"]["CALCS"]
                );
            }
        }
    }
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let d: Dependencies = read("dependencies.json");
    let r = &prior.input().recipe;
    assert_eq!(prior.receipt().input, d.before);
    assert_eq!(
        r.rules
            .owners
            .iter()
            .find(|o| o.owner == d.owner.owner)
            .unwrap(),
        &d.owner
    );
    for definition in d.definitions {
        assert!(r.schema.definitions.contains(&definition));
    }
    assert!(
        json!(prior.input().mapping)["entries"]
            .as_array()
            .unwrap()
            .contains(&d.mapping)
    );
    assert_eq!(
        r.rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    check_membership(
        &input.recipe.rules.owners,
        &input
            .recipe
            .rules
            .effect_applications
            .as_ref()
            .unwrap()
            .members,
    );
    input
        .recipe
        .rules
        .contribution_queries
        .as_mut()
        .unwrap()
        .members
        .extend(queries());
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: KIND.parse().unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest_owned(KIND, &FILES.map(read::<Value>), 1024 * 1024).unwrap(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    let mut inverse = next.input().clone();
    assert_eq!(
        inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members
            .pop()
            .unwrap(),
        queries().remove(0)
    );
    inverse.provenance = migrated.input().provenance.clone();
    assert!(
        inverse == *migrated.input(),
        "only the reviewed query follows the checked migration"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    next
}
