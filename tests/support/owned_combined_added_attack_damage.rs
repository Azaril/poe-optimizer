//! Exact-source added-damage reduction, with a bounded reviewed numeric domain.
#[path = "owned_combined_added_attack_damage_evidence.rs"]
mod evidence;
use super::{attack_selection_family, migration_preservation};
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
pub const KIND: &str = "combined-added-attack-damage";
pub const PROGRAM: &str = "combined-added-attack-factor";
pub const QUERY: &str = "intrinsic-added-attack-factors";
const FILES: [&str; 8] = [
    "bindings.json",
    "dependencies.json",
    "migration.json",
    "consumer.json",
    "queries.json",
    "source-evidence.json",
    "source-review.md",
    "README.md",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/combined-added-attack-damage")
}
pub fn read<T: DeserializeOwned>(n: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(n)).unwrap()).unwrap()
}
fn key(s: &str) -> OwnedDefinitionKey {
    s.parse().unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Consumer {
    pub owners: Vec<DefinitionRules>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BoundProgram {
    owner: SchemaSubject,
    program: RuleProgram,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    before: OwnedContentDigest,
    definitions: Vec<DefinitionDescriptor>,
    owners_before: Vec<DefinitionRules>,
    programs: Vec<BoundProgram>,
    route: ActionOutputRoutes,
    query_registry_closure: SchemaClosure,
    supply_inventory: Value,
}
pub fn migration() -> OwnedReleaseMigrationInput {
    read("migration.json")
}
pub fn consumer() -> Consumer {
    read("consumer.json")
}
pub fn queries() -> Vec<ContributionQuery> {
    read("queries.json")
}
fn commitment() -> OwnedContentDigest {
    digest_owned(
        "owned-combined-added-attack-damage-v1",
        &read::<Value>("authoring.json"),
        1024 * 1024,
    )
    .unwrap()
}
/// Offline/finite-fixture correspondence precondition. Unsupported raw Party
/// payloads remain owned import obligations; this never rewrites their disposition.
pub fn assert_empty_party_source(xml: &str) {
    let p = poe_optimizer_import::build_source::project_xml(xml).unwrap();
    let rows: Vec<_> = p
        .sections()
        .iter()
        .filter(|s| s.element().name() == "Party")
        .collect();
    assert!(rows.len() <= 1, "duplicate Party source");
    if let Some(row) = rows.first() {
        let e = row.element();
        assert!(
            e.namespace().is_none() && !e.has_namespaces(),
            "namespaced Party source"
        );
        assert!(
            e.attributes().iter().all(|a| a.namespace().is_none()
                && ["destination", "append", "ShowAdvanceTools"].contains(&a.name())),
            "unknown Party field"
        );
        assert_eq!(
            e.child_element_count(),
            0,
            "unaccounted imported party payload"
        );
        assert!(!e.has_non_whitespace_text(), "unaccounted Party text");
        for name in ["append", "ShowAdvanceTools"] {
            if let Some(v) = e.attribute(name) {
                assert!(
                    matches!(v.decoded(), "true" | "false"),
                    "Party boolean syntax"
                );
            }
        }
    }
}
pub fn check_source(full: bool) {
    evidence::check(&read("source-evidence.json"), full)
}
pub fn check_source_value(v: &Value) {
    evidence::check_semantics(v)
}
fn expected_program(b: &Value) -> RuleProgram {
    let f = &b["units"]["factor"];
    serde_json::from_value(json!({"id":PROGRAM,"context":"action","reads":[{"id":"product","value_type":{"kind":"quantity","value":{"unit":f}},"source":{"kind":"contribution_query","value":{"entity":"current","query":QUERY,"group":"sources"}}}],
 "nodes":[{"id":"product","expression":{"kind":"read","input":"product"}},{"id":"precision","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":100.0,"unit":f}}}},{"id":"scaled","expression":{"kind":"scale","value":"product","factor":"precision"}},{"id":"half","expression":{"kind":"literal","value":{"kind":"quantity","value":{"value":0.5,"unit":f}}}},{"id":"shifted","expression":{"kind":"add","left":"scaled","right":"half"}},{"id":"rounded","expression":{"kind":"round","value":"shifted","quantum":{"value":1.0,"unit":f},"mode":"floor"}},{"id":"factor","expression":{"kind":"divide_factor","value":"rounded","divisor":"precision"}}],
 "effects":[{"id":"factor","when":null,"effect":{"kind":"derive","entity":"current","stat":b["combined_factor"],"value":"factor"}}]})).unwrap()
}
pub fn check_programs(c: &Value, q: &Value) {
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let c: Consumer = serde_json::from_value(c.clone()).unwrap();
    let q: Vec<ContributionQuery> = serde_json::from_value(q.clone()).unwrap();
    assert_eq!(c.owners.len(), 1);
    assert_eq!(q.len(), 1);
    let before = d
        .owners_before
        .iter()
        .find(|o| json!(o.owner)["value"]["kind"] == "action_output")
        .unwrap();
    assert_eq!(c.owners[0].owner, before.owner);
    assert_eq!(c.owners[0].programs.closure, before.programs.closure);
    assert!(!c.owners[0].programs.is_complete());
    assert_eq!(c.owners[0].programs.members, vec![expected_program(&b)]);
    let expected:ContributionQuery=serde_json::from_value(json!({"id":QUERY,"stat":b["individual_factor"],"contribution":"multiply","groups":[{"id":"sources","reduction":"product","ordering":"ordered","empty":{"kind":"quantity","value":{"value":1.0,"unit":b["units"]["factor"]}},"members":{"closure":{"kind":"complete"},"members":[{"producer":{"kind":"program_effect","owner":before.owner,"program":"intrinsic-added-attack-factor","effect":"factor","origin":{"kind":"action","authored":false,"supplies":[{"declaration":{"kind":"actor","definition":b["actor"]},"slot":{"kind":"skill_grant_slot","namespace":b["actor"]["namespace"],"key":"def.0000000000003092"}}]}},"order":{"source_rank":0,"program_rank":0,"effect_rank":0,"slot_ranks":[]}}]}}]})).unwrap();
    assert_eq!(q, vec![expected]);
}
fn references_supply(v: &Value) -> bool {
    match v {
        Value::Object(o) => {
            o.get("key").and_then(Value::as_str).is_some_and(|k| {
                ["0021", "0022", "001f", "3091", "3092", "3093"]
                    .iter()
                    .any(|s| k == format!("def.000000000000{s}"))
            }) || o.values().any(references_supply)
        }
        Value::Array(a) => a.iter().any(references_supply),
        _ => false,
    }
}
/// Exact original producer bodies and every potential channel writer are part of
/// the bounded arithmetic proof. Program IDs or positive selected observations
/// alone cannot authorize a new profile, inactive writer, or inherited source.
pub fn check_dependencies(schema: &Value, owners: &Value, routes: &Value) {
    let d: Dependencies = read("dependencies.json");
    let mut before = owners.clone();
    for append in consumer().owners {
        let row = before
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|o| o["owner"] == json!(append.owner))
            .unwrap();
        let members = row["programs"]["members"].as_array_mut().unwrap();
        if let Some(i) = members.iter().position(|p| p["id"] == PROGRAM) {
            assert_eq!(members.remove(i), json!(append.programs.members[0]));
        }
    }
    for (name, values) in [
        ("definitions", &schema["definitions"]),
        ("slots", &schema["slots"]),
        ("owners", &before),
    ] {
        let rows: Vec<_> = values
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| references_supply(r))
            .collect();
        assert_eq!(
            json!(rows),
            d.supply_inventory[name],
            "exact potential intrinsic source inventory {name}"
        );
    }
    let actual: Vec<DefinitionRules> = serde_json::from_value(before).unwrap();
    let mut census = Vec::new();
    for o in &actual {
        for p in &o.programs.members {
            if p.effects.iter().any(|e| {
                let v = json!(e);
                v["effect"]["stat"]["key"].as_str().is_some_and(|k| {
                    ["2537", "336b", "336c", "336d"]
                        .iter()
                        .any(|s| k == format!("def.000000000000{s}"))
                })
            }) {
                census.push(json!({"owner":o.owner,"program":p}));
            }
        }
    }
    assert_eq!(
        json!(census),
        json!(
            d.programs
                .iter()
                .map(|r| json!({"owner":r.owner,"program":r.program}))
                .collect::<Vec<_>>()
        )
    );
    let baseline = d
        .programs
        .iter()
        .find(|r| r.program.id.as_str() == "finite-actor-baseline")
        .unwrap();
    let p = json!(baseline.program);
    assert_eq!(p["nodes"][1]["id"], "fact-1");
    assert_eq!(p["nodes"][1]["expression"]["value"]["value"]["value"], 1.15);
    assert_eq!(
        p["effects"][1]["effect"]["stat"]["key"],
        "def.0000000000002537"
    );
    assert_eq!(p["effects"][1]["effect"]["value"], "fact-1");
    assert_eq!(p["effects"][1]["when"], Value::Null);
    let actual: Vec<ActionOutputRoutes> = serde_json::from_value(routes.clone()).unwrap();
    assert_eq!(
        actual
            .iter()
            .filter(|r| r.output == d.route.output)
            .collect::<Vec<_>>(),
        vec![&d.route]
    );
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    let m = migration();
    assert_eq!(a["kind"], KIND);
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(b["before"], json!(d.before));
    assert_eq!(m.before, d.before);
    assert_eq!(a["registry_last_issued_before"], 0x336d);
    assert_eq!(a["registry_last_issued_after"], 0x336e);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(
        b["scope"],
        json!({"new_definitions":1,"new_programs":1,"new_queries":1,"new_routes":0,"new_receivers":0,"complete_query_group":true,"whole_build_parity":false,"other_profiles_admitted":false,"arbitrary_numeric_domain":false,"requires_accounted_inputs":true,"party_imports_admitted":false})
    );
    for name in FILES {
        let bytes = fs::read(data().join(name)).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version,
        key(OWNED_RULE_OPERATIONS_V27)
    );
    assert_eq!(m.schema.len(), 1);
    assert_eq!(m.owners.len(), 1);
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    let SchemaExtensionEntry::Definition(def) = &m.schema[0] else {
        panic!("one Stat only")
    };
    assert_eq!(
        json!(def),
        json!({"kind":"stat","value":{"id":b["combined_factor"],"schema":{"kind":"known","value":{"value":{"kind":"quantity","value":{"unit":b["units"]["factor"]}},"targets":["action"]}}}})
    );
    assert_eq!(b["combined_factor"]["key"], "def.000000000000336e");
    assert_eq!(b["individual_factor"]["key"], "def.000000000000336d");
    assert_eq!(m.owners[0].owner, SchemaSubject::Definition(def.address()));
    assert!(m.owners[0].programs.is_complete() && m.owners[0].programs.members.is_empty());
    check_programs(&read("consumer.json"), &read("queries.json"));
    check_source(false);
}
pub fn assert_component(endpoint: &StagedOwnedRelease) {
    check_authored();
    attack_selection_family::assert_component_after_program_append(endpoint, &consumer().owners);
    let d: Dependencies = read("dependencies.json");
    let r = &endpoint.input().recipe;
    for def in migration().schema {
        let SchemaExtensionEntry::Definition(def) = def else {
            unreachable!()
        };
        assert!(r.schema.definitions.contains(&def));
    }
    for def in d.definitions {
        assert!(r.schema.definitions.contains(&def));
    }
    for added in consumer().owners {
        let o = r
            .rules
            .owners
            .iter()
            .find(|o| o.owner == added.owner)
            .unwrap();
        assert_eq!(o.programs.closure, added.programs.closure);
        for p in added.programs.members {
            assert_eq!(
                o.programs
                    .members
                    .iter()
                    .filter(|x| x.id == p.id)
                    .collect::<Vec<_>>(),
                vec![&p]
            );
        }
    }
    let registry = r.rules.contribution_queries.as_ref().unwrap();
    assert_eq!(registry.closure, d.query_registry_closure);
    assert!(!registry.is_complete());
    for q in queries() {
        assert_eq!(
            registry
                .members
                .iter()
                .filter(|x| x.id == q.id)
                .collect::<Vec<_>>(),
            vec![&q]
        );
    }
    check_dependencies(
        &json!(r.schema),
        &json!(r.rules.owners),
        &json!(r.routing.outputs),
    );
    assert_eq!(
        endpoint
            .input()
            .provenance
            .iter()
            .filter(|p| p.kind == key(KIND)
                && p.prior_input == d.before
                && p.authoring_input == commitment())
            .count(),
        1
    );
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let d: Dependencies = read("dependencies.json");
    assert_eq!(prior.receipt().input, d.before);
    for o in &d.owners_before {
        assert!(prior.input().recipe.rules.owners.contains(o));
    }
    check_dependencies(
        &json!(prior.input().recipe.schema),
        &json!(prior.input().recipe.rules.owners),
        &json!(prior.input().recipe.routing.outputs),
    );
    let migrated = compile_owned_release_migration(prior, migration(), Default::default()).unwrap();
    let mut input = migrated.input().clone();
    for append in consumer().owners {
        let o = input
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        assert_eq!(o.programs.closure, append.programs.closure);
        for p in append.programs.members {
            assert!(!o.programs.members.iter().any(|old| old.id == p.id));
            o.programs.members.push(p);
        }
    }
    let registry = input.recipe.rules.contribution_queries.as_mut().unwrap();
    assert_eq!(registry.closure, d.query_registry_closure);
    for q in queries() {
        assert!(!registry.members.iter().any(|old| old.id == q.id));
        registry.members.push(q);
    }
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: commitment(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    let mut inverse = next.input().clone();
    for append in consumer().owners {
        let o = inverse
            .recipe
            .rules
            .owners
            .iter_mut()
            .find(|o| o.owner == append.owner)
            .unwrap();
        for p in append.programs.members {
            let i = o
                .programs
                .members
                .iter()
                .position(|old| old.id == p.id)
                .unwrap();
            assert_eq!(o.programs.members.remove(i), p);
        }
    }
    for q in queries() {
        let rows = &mut inverse
            .recipe
            .rules
            .contribution_queries
            .as_mut()
            .unwrap()
            .members;
        let i = rows.iter().position(|old| old.id == q.id).unwrap();
        assert_eq!(rows.remove(i), q);
    }
    inverse.provenance = migrated.input().provenance.clone();
    assert_eq!(
        inverse,
        *migrated.input(),
        "exact one program/query addition only"
    );
    migration_preservation::assert_import_rebindings_only(prior, &migrated);
    next
}
