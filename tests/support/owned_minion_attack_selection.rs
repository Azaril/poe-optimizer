//! Offline refinement of four existing intrinsic attack routes. The source
//! catalogue proof authorizes one Fixed selector, not complete Action mechanics.
#[path = "owned_minion_attack_selection_evidence.rs"]
mod evidence;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_routing::*,
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaClosure, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_successor::{
        CatalogAppend, CatalogItemPolicyMode, SuccessorBundleInput, TreePolicyTransitionInput,
        transition_owned_catalog_with_tree_compact,
    },
};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

pub const KIND: &str = "intrinsic-minion-attack-source-selection";
pub const SELECTOR: &str = "intrinsic-minion-attack-source";
pub const SOURCE: &str = "intrinsic-actor";
const REPLACED: [&str; 4] = [
    "intrinsic-minion-attack-rate",
    "intrinsic-minion-critical-chance",
    "intrinsic-minion-physical-max",
    "intrinsic-minion-physical-min",
];
const ARTIFACTS: [&str; 5] = [
    "bindings.json",
    "dependencies.json",
    "routes.json",
    "source-evidence.json",
    "source-review.md",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/minion-attack-selection")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn key(value: &str) -> OwnedDefinitionKey {
    value.parse().unwrap()
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Dependencies {
    before: OwnedContentDigest,
    receipt: Value,
    definitions: Vec<DefinitionDescriptor>,
    slots: Vec<SlotDescriptor>,
    owners: Vec<DefinitionRules>,
    route_before: ActionOutputRoutes,
    query_registry_closure: SchemaClosure,
    supply_inventory: Value,
}
pub fn routes() -> Vec<ActionOutputRoutes> {
    read("routes.json")
}
pub fn before_routes() -> Vec<ActionOutputRoutes> {
    vec![read::<Dependencies>("dependencies.json").route_before]
}
fn commitment() -> OwnedContentDigest {
    // The authoring manifest commits every source-review byte as well as the
    // typed payloads. No source evidence is embedded in the runtime release.
    digest_owned(
        "owned-intrinsic-minion-attack-source-selection-v1",
        &read::<Value>("authoring.json"),
        1024 * 1024,
    )
    .unwrap()
}
pub fn check_source(full: bool) {
    let value = read("source-evidence.json");
    if full {
        evidence::check(&value, true);
    } else {
        check_source_evidence(&value);
    }
}
pub fn check_source_evidence(value: &Value) {
    evidence::check(value, false);
}
pub fn check_source_refusals() {
    let value: Value = read("source-evidence.json");
    evidence::check_semantics(&value);
    let controls = evidence::invalid_controls(&value);
    assert!(!controls.is_empty());
    for (name, changed) in controls {
        assert!(
            std::panic::catch_unwind(|| evidence::check_semantics(&changed)).is_err(),
            "source-scope control was admitted: {name}"
        );
    }
}

fn references_supply(value: &Value) -> bool {
    match value {
        Value::Object(fields) => {
            fields
                .get("key")
                .and_then(Value::as_str)
                .is_some_and(|key| {
                    ["0021", "0022", "001f", "3091", "3092", "3093"]
                        .iter()
                        .any(|suffix| key == format!("def.000000000000{suffix}"))
                })
                || fields.values().any(references_supply)
        }
        Value::Array(rows) => rows.iter().any(references_supply),
        _ => false,
    }
}
pub fn check_supply_inventory(schema: &Value, owners: &Value) {
    let d: Dependencies = read("dependencies.json");
    for (field, values) in [
        ("definitions", &schema["definitions"]),
        ("slots", &schema["slots"]),
        ("owners", owners),
    ] {
        let rows: Vec<_> = values
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| references_supply(row))
            .collect();
        assert_eq!(
            json!(rows),
            d.supply_inventory[field],
            "exact potential supply census: {field}"
        );
    }
    let basic = d.supply_inventory["definitions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["kind"] == "skill" && r["value"]["id"]["key"] == "def.0000000000000021")
        .unwrap();
    assert_eq!(
        basic["value"]["schema"]["value"]["directly_selectable"],
        false
    );
    let slots = d.supply_inventory["slots"].as_array().unwrap();
    let suppliers: Vec<_> = slots
        .iter()
        .filter(|r| {
            r["kind"] == "skill_grant"
                && r["value"]["schema"]["value"]["skill"]["key"] == "def.0000000000000021"
        })
        .collect();
    assert_eq!(suppliers.len(), 1);
    assert_eq!(
        suppliers[0]["value"]["id"]["declaration"]["definition"]["key"],
        "def.0000000000003091"
    );
    assert_eq!(
        suppliers[0]["value"]["id"]["slot"]["key"],
        "def.0000000000003092"
    );
    let actors: Vec<_> = slots
        .iter()
        .filter(|r| {
            r["kind"] == "actor"
                && r["value"]["schema"]["value"]["provider_definition"]["key"]
                    == "def.0000000000003091"
        })
        .collect();
    assert_eq!(actors.len(), 1);
    assert_eq!(
        actors[0]["value"]["id"]["declaration"]["definition"]["key"],
        "def.0000000000000012"
    );
    assert_eq!(
        actors[0]["value"]["id"]["slot"]["key"],
        "def.000000000000001f"
    );
    let output = slots
        .iter()
        .find(|r| {
            r["kind"] == "action_output"
                && r["value"]["id"]["slot"]["key"] == "def.0000000000000022"
        })
        .unwrap();
    assert_eq!(
        output["value"]["schema"]["value"]["actor_role"],
        json!({"kind":"provider_actor"})
    );
}

/// Construct the sole allowed transition from the exact authored predecessor.
/// This independently rejects any additional source, binding or closure edit.
pub fn check_routes(value: &Value) {
    let actual: Vec<ActionOutputRoutes> = serde_json::from_value(value.clone()).unwrap();
    let d: Dependencies = read("dependencies.json");
    let b: Value = read("bindings.json");
    let mut expected = d.route_before.clone();
    assert_eq!(expected.output.slot.key().as_str(), "def.0000000000000022");
    assert!(!expected.routes.is_complete());
    let selectors = expected.source_selectors.as_ref().unwrap();
    assert!(!selectors.is_complete() && selectors.members.is_empty());
    assert_eq!(expected.routes.members.len(), 6);
    let selection = expected.routes.members[0].selection.clone();
    assert!(matches!(selection, ActionRouteSelection::Exact(_)));
    let mut bindings = Vec::new();
    for row in &mut expected.routes.members {
        if REPLACED.contains(&row.id.as_str()) {
            assert_eq!(row.selection, selection);
            let ActionStatRouteSource::ActionActor { stat } = &row.source else {
                panic!("only the exact prior intrinsic Actor bindings may be selected")
            };
            bindings.push(json!({"id":row.id,"source_stat":stat,"target_stat":row.target}));
            row.source = ActionStatRouteSource::Selected {
                selector: key(SELECTOR),
                stats: vec![ActionSourceStat {
                    source: key(SOURCE),
                    stat: stat.clone(),
                }],
            };
        }
    }
    assert_eq!(bindings.len(), 4);
    assert_eq!(b["selected_routes"], json!(bindings));
    let selector = ActionSourceSelector {
        id: key(SELECTOR),
        selection,
        sources: vec![NamedActionSource {
            id: key(SOURCE),
            origin: ActionSourceOrigin::ActionActor,
        }],
        policy: ActionSourcePolicy::Fixed {
            source: key(SOURCE),
        },
    };
    assert_eq!(b["selector"], json!(selector));
    expected.source_selectors = Some(poe_optimizer_core::owned_schema::DeclaredSet::complete(
        vec![selector],
    ));
    assert_eq!(actual, vec![expected]);
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Dependencies = read("dependencies.json");
    assert_eq!(a["schema_version"], 1);
    assert_eq!(a["kind"], KIND);
    assert_eq!(b["kind"], KIND);
    assert_eq!(a["before"], json!(d.before));
    assert_eq!(a["before"], b["before"]);
    assert_eq!(a["before"], d.receipt["input"]);
    assert_eq!(a["registry_last_issued_before"], 0x336d);
    assert_eq!(a["registry_last_issued_after"], 0x336d);
    assert_eq!(
        a["source_revision"],
        "3887ae68a6a6b8bb7b41d1b61998f1aa184201e4"
    );
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(
        a["scope"],
        json!({
            "new_definitions":0,"new_programs":0,"replaced_routes":4,"new_selectors":1,
            "closed_selector_inventories":1,"closed_route_inventories":0,"closed_rule_owners":0,
            "new_queries":0,"whole_build_parity":false,"other_action_outputs_admitted":false
        })
    );
    assert_eq!(a["artifacts"].as_object().unwrap().len(), ARTIFACTS.len());
    for name in ARTIFACTS {
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(a["artifacts"][name]["bytes"], bytes.len());
        assert_eq!(
            a["artifacts"][name]["sha256"],
            format!("{:x}", Sha256::digest(bytes))
        );
    }
    assert_eq!(d.receipt["definitions"]["schema_version"], 6);
    assert_eq!(d.definitions.len(), 15);
    assert_eq!(d.slots.len(), 2);
    assert_eq!(d.owners.len(), 5);
    assert!(d.owners.iter().all(|o| !o.programs.is_complete()));
    assert!(matches!(
        d.query_registry_closure,
        SchemaClosure::Partial { .. }
    ));
    assert_eq!(b["output"], json!(d.route_before.output));
    assert_eq!(
        b["unchanged_routes"],
        json!([
            "intrinsic-added-attack-percentage",
            "intrinsic-minion-accuracy-hit-chance"
        ])
    );
    check_routes(&read("routes.json"));
    check_supply_inventory(&d.supply_inventory, &d.supply_inventory["owners"]);
    check_source(false);
}

pub fn assert_component(endpoint: &StagedOwnedRelease) {
    assert_component_after_program_append(endpoint, &[]);
}

/// Check the original source proof after independently authenticated numerical
/// programs have been appended. Reverse only those exact bodies on a copy;
/// every preexisting owner, supply relation and potential supplier is still
/// compared in full. This does not admit changed or additional source rules.
pub fn assert_component_after_program_append(
    endpoint: &StagedOwnedRelease,
    appended: &[DefinitionRules],
) {
    check_authored();
    let d: Dependencies = read("dependencies.json");
    let recipe = &endpoint.input().recipe;
    let mut owners = recipe.rules.owners.clone();
    for addition in appended {
        let owner = owners
            .iter_mut()
            .find(|o| o.owner == addition.owner)
            .unwrap();
        for program in &addition.programs.members {
            let matches: Vec<_> = owner
                .programs
                .members
                .iter()
                .enumerate()
                .filter(|(_, p)| p.id == program.id)
                .collect();
            assert_eq!(matches.len(), 1, "one exact appended program");
            let (index, actual) = matches[0];
            assert_eq!(actual, program);
            owner.programs.members.remove(index);
        }
    }
    check_supply_inventory(&json!(recipe.schema), &json!(owners));
    for row in &d.definitions {
        assert!(recipe.schema.definitions.contains(row));
    }
    for row in &d.slots {
        assert!(recipe.schema.slots.contains(row));
    }
    for row in &d.owners {
        assert!(owners.contains(row));
    }
    assert!(recipe.routing.outputs.contains(&routes().remove(0)));
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
    assert!(prior.evaluation().is_none());
    for (field, expected) in d.receipt.as_object().unwrap() {
        assert_eq!(
            &json!(prior.receipt())[field],
            expected,
            "prior receipt {field}"
        );
    }
    for row in &d.definitions {
        assert!(prior.input().recipe.schema.definitions.contains(row));
    }
    for row in &d.slots {
        assert!(prior.input().recipe.schema.slots.contains(row));
    }
    for row in &d.owners {
        assert!(prior.input().recipe.rules.owners.contains(row));
    }
    let b = prior.input();
    check_supply_inventory(&json!(b.recipe.schema), &json!(b.recipe.rules.owners));
    let mut successor = b.recipe.clone();
    let actual = successor
        .routing
        .outputs
        .iter_mut()
        .find(|r| r.output == d.route_before.output)
        .unwrap();
    assert_eq!(actual, &d.route_before);
    *actual = routes().remove(0);
    let carried = transition_owned_catalog_with_tree_compact(
        SuccessorBundleInput {
            schema_version: 1,
            prior: b.recipe.clone(),
            successor,
            mapping: b.mapping.clone(),
            roles: b.roles.clone(),
            normalization: b.normalization.clone(),
            rewards: b.rewards.clone(),
            query_sets: b.query_sets.clone(),
            items: b.items.clone(),
            item_source: b.item_source.clone(),
        },
        CatalogAppend {
            mappings: vec![],
            source: b.mapping.source.clone(),
            item_policies: CatalogItemPolicyMode::RebindPrior,
        },
        TreePolicyTransitionInput::RebindPrior {
            prior: Box::new(b.tree.clone().unwrap()),
        },
        Default::default(),
    )
    .unwrap();
    let mut input = b.clone();
    input.recipe = carried.recipe().clone();
    input.mapping = carried.mapping().input().clone();
    input.roles = carried.roles().input().clone();
    input.normalization = carried.normalization().clone();
    input.rewards = carried.rewards().input().clone();
    input.items = carried.items().input().clone();
    input.item_source = carried.item_source().input().clone();
    input.tree = carried.tree().map(|t| t.input().clone());
    input.query_sets = carried.query_sets().to_vec();
    input.provenance.push(OwnedReleaseProvenance {
        kind: key(KIND),
        prior_input: prior.receipt().input,
        authoring_input: commitment(),
    });
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_component(&next);
    // No schema identity churn or broad rebind waiver: restore this one exact
    // declaration and receipt, then require byte-equivalent full input semantics.
    let mut inverse = next.input().clone();
    let actual = inverse
        .recipe
        .routing
        .outputs
        .iter_mut()
        .find(|r| r.output == d.route_before.output)
        .unwrap();
    assert_eq!(*actual, routes().remove(0));
    *actual = d.route_before;
    let receipt = inverse.provenance.pop().unwrap();
    assert_eq!(receipt.kind, key(KIND));
    assert!(
        inverse == *b,
        "routing-only selection preserves every other release input"
    );
    assert_eq!(next.receipt().definitions, prior.receipt().definitions);
    assert_eq!(next.receipt().rules, prior.receipt().rules);
    assert_eq!(
        next.receipt().compiled_rules,
        prior.receipt().compiled_rules
    );
    assert_eq!(next.receipt().query_rows, 110);
    assert_eq!(next.input().recipe.registry.last_issued.get(), 0x336d);
    next
}
