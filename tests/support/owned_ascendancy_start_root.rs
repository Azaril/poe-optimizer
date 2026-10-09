//! Bounded default Ascendancy-root closure using the existing root evidence and
//! passive-refinement publisher. Other owners and transformations stay separate.
use super::{family, passive_publication};
use poe_optimizer_core::{owned_content::digest_owned, owned_schema::DefinitionDescriptor};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "ascendancy-start-root-default-inventory";
const DOMAIN: &str = "owned-ascendancy-start-root-v1";
const ROOT: &str = "def.0000000000001b5d";
const FIELDS: [&str; 7] = [
    "parameters",
    "choices",
    "grants",
    "actors",
    "skill_grants",
    "outputs",
    "sockets",
];
pub fn data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/owned/poe2/3887ae68/ascendancy-start-root")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn check_rows(d: &Value, c: &Value) {
    assert_eq!(d["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(d["owners"].as_array().unwrap().len(), 1);
    assert_eq!(c["definitions"].as_array().unwrap().len(), 1);
    assert_eq!(c["owners"].as_array().unwrap().len(), 1);
    let old = &d["definitions"][0];
    assert_eq!(old["kind"], "passive_node");
    assert_eq!(old["value"]["id"]["key"], ROOT);
    let subject = &d["owners"][0]["owner"];
    assert_eq!(subject["value"]["value"], old["value"]["id"]);
    let mut expected = old.clone();
    let declarations = &mut expected["value"]["schema"]["value"]["declarations"];
    assert_eq!(declarations.as_object().unwrap().len(), FIELDS.len());
    for field in FIELDS {
        assert_eq!(declarations[field]["members"], json!([]));
        assert_eq!(
            declarations[field]["closure"],
            json!({"kind":"partial","value":{"gaps":[{
                "subject":subject,"facet":"input_schema","code":"tree-declarations-not-converted"
            }]}})
        );
        declarations[field]["closure"] = json!({"kind":"complete"});
    }
    assert_eq!(
        c["definitions"][0], expected,
        "only reviewed default inventories change"
    );
    let mut owner = d["owners"][0].clone();
    assert_eq!(owner["programs"]["members"], json!([]));
    assert_eq!(
        owner["programs"]["closure"],
        json!({"kind":"partial","value":{"gaps":[{
            "subject":subject,"facet":"game_rules","code":"tree-game-rules-not-converted"
        }]}})
    );
    owner["programs"]["closure"] = json!({"kind":"complete"});
    assert_eq!(c["owners"][0], owner);
    assert_eq!(d["slots"], json!([]));
    let definitions = d["supporting_definitions"].as_array().unwrap();
    let owners = d["action_owners"].as_array().unwrap();
    assert_eq!(definitions.len(), 4);
    assert_eq!(owners.len(), 4);
    for ((definition, owner), (id, complete)) in definitions.iter().zip(owners).zip([
        ("def.0000000000000a23", false),
        ("def.0000000000000a36", false),
        ("def.0000000000001790", true),
        ("def.000000000000332a", false),
    ]) {
        assert_eq!(definition["value"]["id"]["key"], id);
        assert_eq!(owner["owner"]["value"]["value"], definition["value"]["id"]);
        assert_eq!(
            owner["programs"]["closure"]["kind"],
            if complete { "complete" } else { "partial" }
        );
    }
    assert_eq!(
        definitions[1]["value"]["schema"]["value"]["implicit_passives"]["members"],
        json!([old["value"]["id"]])
    );
}

pub fn check_vectors(a: &Value, v: &Value, authenticate: bool) {
    family::check_root_vectors(a, v, authenticate, true);
    assert_eq!(v["metadata"]["source_root"], 8305);
    let projection = &v["projection"];
    // Every source/constructed field has an explicit reviewed disposition. A new
    // field cannot become harmless merely by being retained in a JSON report.
    let policy: Value = read("bindings.json");
    for (object, key) in [
        (&projection["raw"], "source_fields"),
        (&projection["constructed"]["fields"], "constructed_fields"),
    ] {
        let actual: BTreeSet<_> = object
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        let dispositions = policy[key].as_array().unwrap();
        let expected: BTreeSet<_> = dispositions
            .iter()
            .map(|v| {
                let name = v["name"].as_str().unwrap();
                let disposition = match name {
                    "stats" | "sd" | "mods" | "modKey" => "empty_intrinsic_inventory",
                    "ascendancyName" | "isAscendancyStart" | "type" => {
                        "root_kind_and_owner_relation"
                    }
                    "connections" | "linkedId" => "preserved_topology",
                    "skill" | "stringId" | "iname" | "id" => "exact_identity",
                    "angle" | "group" | "g" | "orbit" | "orbitIndex" | "o" | "oidx" | "x" | "y" => {
                        "preserved_geometry_external_transformations"
                    }
                    "icon" | "name" | "nodeOverlay" | "dn" | "rsq" | "size" => "presentation",
                    _ => panic!("unreviewed root field disposition"),
                };
                assert_eq!(v["disposition"], disposition);
                name
            })
            .collect();
        assert_eq!(dispositions.len(), expected.len());
        assert_eq!(actual, expected, "unreviewed source/constructed field");
    }
    assert_eq!(
        projection["constructed"]["fields"]["isAscendancyStart"],
        true
    );
    assert_eq!(
        projection["constructed"]["fields"]["ascendancyName"],
        "Disciple of Varashta"
    );
    assert_eq!(
        projection["selected_root"]["default_modifier_object_identity"],
        false
    );
    assert_eq!(
        projection["selected_modifiers"],
        projection["constructed"]["default_modifiers"]
    );
}

pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    for field in [
        "before",
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(a[field], b[field]);
        assert_eq!(
            a[field],
            d["source"][if field == "before" { "input" } else { field }]
        );
    }
    for (field, value) in [
        ("allocated_definitions", 0),
        ("new_programs", 0),
        ("new_receivers", 0),
        ("closed_passive_owners", 1),
        ("closed_empty_declaration_inventories", 7),
    ] {
        assert_eq!(a[field], value);
    }
    assert_eq!(a["registry_last_issued_before"], 0x335d);
    assert_eq!(
        a["registry_last_issued_after"],
        a["registry_last_issued_before"]
    );
    assert_eq!(b["registry_last_issued"], a["registry_last_issued_before"]);
    assert_eq!(b["source_node"], 8305);
    assert_eq!(b["node"]["key"], ROOT);
    assert_eq!(b["declaration_fields"], json!(FIELDS));
    assert_eq!(b["scope"], a["scope"]);
    assert_eq!(a["scope"]["default_root_only"], true);
    for key in [
        "class_rule_closure_changed",
        "ascendancy_rule_closure_changed",
        "universal_player_initialization_closed",
        "external_transformations_closed",
        "whole_build_parity",
    ] {
        assert_eq!(a["scope"][key], false);
    }
    let mut names = BTreeSet::new();
    for row in a["artifacts"].as_array().unwrap() {
        let name = row["file"].as_str().unwrap();
        assert!(names.insert(name));
        let bytes = fs::read(data().join(name)).unwrap();
        assert_eq!(bytes.len() as u64, row["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), row["sha256"]);
        assert!(!bytes.contains(&b'\r'));
    }
    assert_eq!(
        names,
        [
            "bindings.json",
            "dependencies.json",
            "closure.json",
            "migration.json",
            "source-vectors.json"
        ]
        .into_iter()
        .collect()
    );
    check_rows(&d, &c);
    check_vectors(&a, &v, false);
    let source_ids: BTreeSet<_> = v["projection"]["constructed"]["fields"]["linkedId"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_u64().unwrap().to_string())
        .collect();
    let mapped_neighbors: BTreeSet<_> = d["mapping_rows"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|row| {
            if row["source"]["value"]["kind"] != "passive_node" {
                return None;
            }
            let id = row["source"]["value"]["value"]["node_id"]["value"]
                .as_str()
                .unwrap();
            source_ids
                .contains(id)
                .then(|| row["outcome"]["value"]["target"]["value"]["value"].clone())
        })
        .map(|id| serde_json::to_string(&id).unwrap())
        .collect();
    let neighbors: BTreeSet<_> =
        d["definitions"][0]["value"]["schema"]["value"]["adjacent"]["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| serde_json::to_string(id).unwrap())
            .collect();
    assert_eq!(mapped_neighbors.len(), source_ids.len());
    assert_eq!(mapped_neighbors, neighbors);
    assert_eq!(
        d["tree_tokens"],
        json!([{"token":"8305","role":{"kind":"implicit_root","value":{"node":b["node"]}}}])
    );
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("closure.json");
    let v: Value = read("source-vectors.json");
    check_vectors(&a, &v, true);
    let next = passive_publication::stage_refinement(prior, &data(), KIND, DOMAIN);
    let proof = next.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(proof.prior_input, prior.receipt().input);
    assert_eq!(
        proof.authoring_input,
        digest_owned(DOMAIN, &(a, b, d, c.clone(), v), 4 * 1024 * 1024).unwrap()
    );
    for row in c["definitions"].as_array().unwrap() {
        let expected: DefinitionDescriptor = serde_json::from_value(row.clone()).unwrap();
        assert!(next.input().recipe.schema.definitions.contains(&expected));
    }
    assert_eq!(next.input().recipe.registry, prior.input().recipe.registry);
    next
}
