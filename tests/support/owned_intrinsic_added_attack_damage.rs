//! A bounded Actor-profile producer and one exact Action route. This packet
//! preserves raw percentage and one unrounded factor, without grouped reduction.
#[path = "owned_intrinsic_added_attack_damage_evidence.rs"]
mod evidence;
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_routing::ActionOutputRoutes,
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
pub const KIND: &str = "intrinsic-added-attack-damage";
pub const PROFILE_PROGRAM: &str = "intrinsic-added-attack-percentage";
pub const ACTION_PROGRAM: &str = "intrinsic-added-attack-factor";
const FILES: [&str; 7] = [
    "authoring.json",
    "bindings.json",
    "migration.json",
    "consumer.json",
    "routes.json",
    "dependencies.json",
    "source-vectors.json",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/intrinsic-added-attack-damage")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
fn digest() -> OwnedContentDigest {
    digest_owned(KIND, &FILES.map(read::<Value>), 4 * 1024 * 1024).unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    before: OwnedContentDigest,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
    owners: Vec<DefinitionRules>,
    route_before: ActionOutputRoutes,
    query_registry_closure: SchemaClosure,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn routes() -> Vec<ActionOutputRoutes> {
    read("routes.json")
}
pub fn checked_source() -> Value {
    let v = read("source-vectors.json");
    evidence::check(&v, false);
    v
}
pub fn check_source(full: bool) {
    evidence::check(&read("source-vectors.json"), full);
}
pub fn check_source_vectors(v: &Value) {
    evidence::check(v, false);
}
fn expected_programs(b: &Value) -> Vec<RuleProgram> {
    let f = &b["units"]["factor"];
    let p = &b["units"]["percentage"];
    serde_json::from_value(json!([
        {"id":PROFILE_PROGRAM,"context":"actor","reads":[{"id":"scale","value_type":{"kind":"quantity","value":{"unit":f}},"source":{"kind":"stat","value":{"entity":"current","stat":b["profile_scale"]}}}],"nodes":[
            {"id":"scale","expression":{"kind":"read","input":"scale"}},
            {"id":"one","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":1.0,"unit":f}}}},
            {"id":"delta","expression":{"kind":"subtract","left":"scale","right":"one"}},
            {"id":"hundred","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":100.0,"unit":p}}}},
            {"id":"percentage","expression":{"kind":"scale","value":"hundred","factor":"delta"}}],
            "effects":[{"id":"percentage","when":null,"effect":{"kind":"derive","entity":"current","stat":b["actor_percentage"],"value":"percentage"}}]},
        {"id":ACTION_PROGRAM,"context":"action","reads":[{"id":"percentage","value_type":{"kind":"quantity","value":{"unit":p}},"source":{"kind":"stat","value":{"entity":"current","stat":b["action_percentage"]}}}],"nodes":[
            {"id":"percentage","expression":{"kind":"read","input":"percentage"}},
            {"id":"zero","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0.0,"unit":p}}}},
            {"id":"identity","expression":{"kind":"compare","operation":"equal","left":"percentage","right":"zero"}},
            {"id":"applies","expression":{"kind":"not","value":"identity"}},
            {"id":"fraction","expression":{"kind":"percent_as_factor","percent":"percentage","unit":f}},
            {"id":"one","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":1.0,"unit":f}}}},
            {"id":"factor","expression":{"kind":"add","left":"one","right":"fraction"}}],
            "effects":[{"id":"factor","when":"applies","effect":{"kind":"contribute","entity":"current","stat":b["action_factor"],"contribution":"multiply","value":"factor"}}]}
    ])).unwrap()
}
pub fn check_programs(c: &Value, rs: &Value) {
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let actual: Consumer = serde_json::from_value(c.clone()).unwrap();
    let routes: Vec<ActionOutputRoutes> = serde_json::from_value(rs.clone()).unwrap();
    assert_eq!(actual.owners.len(), 2);
    assert_eq!(routes.len(), 1);
    for ((owner, expected), before) in actual
        .owners
        .iter()
        .zip(expected_programs(&b))
        .zip(&d.owners)
    {
        assert_eq!(owner.owner, before.owner);
        assert_eq!(owner.programs.closure, before.programs.closure);
        assert!(!owner.programs.is_complete());
        assert_eq!(owner.programs.members, vec![expected]);
    }
    let mut expected = d.route_before.clone();
    let mut added = expected.routes.members[0].clone();
    added.id = key(PROFILE_PROGRAM);
    added.source = serde_json::from_value(
        json!({"kind":"action_actor","value":{"stat":b["actor_percentage"]}}),
    )
    .unwrap();
    added.target = serde_json::from_value(b["action_percentage"].clone()).unwrap();
    expected.routes.members = vec![added];
    assert_eq!(routes[0], expected);
    assert!(!routes[0].routes.is_complete());
    assert!(
        routes[0]
            .source_selectors
            .as_ref()
            .is_some_and(|s| !s.is_complete())
    );
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let m = migration();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(a["before"], b["before"]);
    assert_eq!(m.before, d.before);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V27)
    );
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(a["registry_last_issued_before"], 0x336a);
    assert_eq!(a["registry_last_issued_after"], 0x336d);
    assert_eq!(
        a["scope"],
        json!({"new_definitions":3,"new_programs":2,"new_routes":1,"new_queries":0,"new_receivers":0,"source_profile_scope":"exact-summoned-actor-slot","other_profiles_admitted":false,"whole_build_parity":false,"complete_added_damage_domain":false,"final_group_factor":false,"rounding_game_intent_claim":false})
    );
    for file in FILES.iter().skip(1) {
        let bytes = fs::read(data().join(file)).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(a["artifacts"][file]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][file]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert_eq!(m.schema.len(), 3);
    assert_eq!(m.owners.len(), 3);
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    for ((row, owner), (channel, scope, unit)) in m.schema.iter().zip(&m.owners).zip([
        ("actor_percentage", "actor", "percentage"),
        ("action_percentage", "action", "percentage"),
        ("action_factor", "action", "factor"),
    ]) {
        let SchemaExtensionEntry::Definition(definition) = row else {
            panic!("only scalar Stat additions")
        };
        let value = json!(definition);
        assert_eq!(value["kind"], "stat");
        assert_eq!(value["value"]["id"], b[channel]);
        assert_eq!(
            value["value"]["schema"],
            json!({"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["units"][unit]}},"targets":[scope]}})
        );
        assert_eq!(owner.owner, SchemaSubject::Definition(definition.address()));
        assert!(owner.programs.is_complete() && owner.programs.members.is_empty());
    }
    assert_eq!(b["actor"]["key"], "def.0000000000003091");
    assert_eq!(b["actor_slot"]["slot"]["key"], "def.000000000000001f");
    assert_eq!(b["source_skill"]["key"], "def.0000000000000012");
    assert_eq!(b["action"]["slot"]["key"], "def.0000000000000022");
    let slot = d
        .slots
        .iter()
        .find(|s| json!(s)["value"]["id"] == b["actor_slot"])
        .unwrap();
    assert_eq!(
        json!(slot)["value"]["schema"]["value"]["provider_definition"],
        b["actor"]
    );
    assert_eq!(
        json!(d.owners[0].owner),
        json!({"kind":"slot","value":{"kind":"actor","value":b["actor_slot"]}})
    );
    assert_eq!(
        json!(d.owners[1].owner),
        json!({"kind":"slot","value":{"kind":"action_output","value":b["action"]}})
    );
    assert!(d.owners.iter().all(|o| !o.programs.is_complete()));
    assert!(matches!(
        d.query_registry_closure,
        SchemaClosure::Partial { .. }
    ));
    check_programs(&read("consumer.json"), &read("routes.json"));
    check_source(false);
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    let d: Dependencies = read("dependencies.json");
    let recipe = &endpoint.input().recipe;
    for row in &d.definitions {
        assert!(recipe.schema.definitions.contains(row));
    }
    for row in &d.slots {
        assert!(recipe.schema.slots.contains(row));
    }
    for row in migration().schema {
        let SchemaExtensionEntry::Definition(definition) = row else {
            panic!()
        };
        assert!(recipe.schema.definitions.contains(&definition));
    }
    for (before, addition) in d.owners.iter().zip(consumer().owners) {
        let actual = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == before.owner)
            .unwrap();
        assert_eq!(actual.programs.closure, before.programs.closure);
        for p in before
            .programs
            .members
            .iter()
            .chain(&addition.programs.members)
        {
            assert!(actual.programs.members.contains(p));
        }
    }
    let actual = recipe
        .routing
        .outputs
        .iter()
        .find(|r| r.output == d.route_before.output)
        .unwrap();
    assert_eq!(actual.routes.closure, d.route_before.routes.closure);
    assert_eq!(actual.source_selectors, d.route_before.source_selectors);
    for route in d
        .route_before
        .routes
        .members
        .iter()
        .chain(&routes()[0].routes.members)
    {
        assert!(actual.routes.members.contains(route));
    }
    assert_eq!(
        recipe.rules.contribution_queries.as_ref().unwrap().closure,
        d.query_registry_closure
    );
    assert_eq!(
        endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND)
                && p.prior_input == d.before
                && p.authoring_input == digest())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, d.before);
    for row in &d.definitions {
        assert!(prior.input().recipe.schema.definitions.contains(row));
    }
    for row in &d.slots {
        assert!(prior.input().recipe.schema.slots.contains(row));
    }
    for row in &d.owners {
        assert!(prior.input().recipe.rules.owners.contains(row));
    }
    assert!(
        prior
            .input()
            .recipe
            .routing
            .outputs
            .contains(&d.route_before)
    );
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    for append in consumer().owners {
        let owner = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        assert_eq!(owner.programs.closure, append.programs.closure);
        for p in append.programs.members {
            assert!(!owner.programs.members.iter().any(|old| old.id == p.id));
            owner.programs.members.push(p);
        }
    }
    for append in routes() {
        let route = input
            .recipe
            .routing
            .outputs
            .iter_mut()
            .find(|r| r.output == append.output)
            .unwrap();
        assert_eq!(route.routes.closure, append.routes.closure);
        assert_eq!(route.source_selectors, append.source_selectors);
        for row in append.routes.members {
            assert!(!route.routes.members.iter().any(|old| old.id == row.id));
            route.routes.members.push(row);
        }
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for append in consumer().owners {
        let owner = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        for p in append.programs.members {
            let pos = owner
                .programs
                .members
                .iter()
                .position(|old| old.id == p.id)
                .unwrap();
            assert_eq!(owner.programs.members.remove(pos), p);
        }
    }
    for append in routes() {
        let route = inverse
            .recipe
            .routing
            .outputs
            .iter_mut()
            .find(|r| r.output == append.output)
            .unwrap();
        for row in append.routes.members {
            let pos = route
                .routes
                .members
                .iter()
                .position(|old| old.id == row.id)
                .unwrap();
            assert_eq!(route.routes.members.remove(pos), row);
        }
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert_eq!(
        inverse,
        *migrated.input(),
        "only two exact programs and one exact route follow the three-Stat migration"
    );
    // The shared import proof deliberately permits no routing membership
    // change. Apply it to the checked three-Stat migration; the exact inverse
    // above separately proves the only subsequent route/program additions.
    migration_preservation::assert_import_rebindings_only(prior, &migrated);
    next
}
