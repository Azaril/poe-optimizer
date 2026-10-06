//! Exact parameter-port inventories; every other item domain remains unchanged.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-selected-item-parameters";
const DOMAIN: &str = "owned-selected-item-parameters-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/selected-item-parameters")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(a, b, d, v, m), 1024 * 1024).unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, n) in [
        ("allocated_definitions", 0),
        ("replaced_definitions", 3),
        ("closed_parameter_inventories", 3),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3314),
        ("registry_last_issued_after", 0x3314),
    ] {
        assert_eq!(a[field], n);
    }
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
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(
        m.release.as_str(),
        "pob-3887ae68-selected-item-parameters-v1"
    );
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 3);
    let replacements: Vec<_> = m
        .schema
        .iter()
        .map(|entry| match entry {
            SchemaExtensionEntry::Definition(row) => row,
            _ => panic!("exact descriptor replacements only"),
        })
        .collect();
    assert_eq!(
        replacements
            .iter()
            .map(|row| json!(row)["value"]["id"]["key"].clone())
            .collect::<Vec<_>>(),
        vec![
            json!("def.00000000000009dc"),
            json!("def.0000000000001e0e"),
            json!("def.0000000000001f1c"),
        ],
        "migration replacements retain unique canonical allocation-key order"
    );
    for (field, count) in [
        ("definitions", 3),
        ("slots", 18),
        ("owners", 3),
        ("physical_input_bindings", 3),
        ("template_defaults", 3),
    ] {
        assert_eq!(d[field].as_array().unwrap().len(), count);
    }
    assert_eq!(b["templates"].as_array().unwrap().len(), 3);
    for (i, (key, name, item, ordinal, first)) in [
        ("1f1c", "Iron Crown", 21, 576, 0x3141),
        ("1e0e", "Cryptic Leggings", 22, 578, 0x3147),
        ("09dc", "Sapphire Ring", 26, 587, 0x316c),
    ]
    .into_iter()
    .enumerate()
    {
        let binding = &b["templates"][i];
        assert_eq!(binding["template"]["key"], format!("def.000000000000{key}"));
        assert_eq!(binding["name"], name);
        assert_eq!(binding["source_item_id"], item);
        assert_eq!(binding["original"], 5);
        assert_eq!(binding["source_ordinal"], ordinal);
        assert_eq!(binding["content_entry"], 0);
        let old_matches: Vec<_> = d["definitions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|row| row["value"]["id"] == binding["template"])
            .collect();
        assert_eq!(old_matches.len(), 1);
        let old = old_matches[0];
        let prior_descriptor: DefinitionDescriptor = serde_json::from_value(old.clone()).unwrap();
        let new_matches: Vec<_> = replacements
            .iter()
            .filter(|row| row.address() == prior_descriptor.address())
            .collect();
        assert_eq!(new_matches.len(), 1);
        let new = new_matches[0];
        let mut restored = json!(new);
        let params = &mut restored["value"]["schema"]["value"]["declarations"]["parameters"];
        let prior = &old["value"]["schema"]["value"]["declarations"]["parameters"];
        assert_eq!(prior["closure"]["kind"], "partial");
        assert_eq!(params["closure"], json!({"kind":"complete"}));
        assert_eq!(params["members"], prior["members"]);
        assert_eq!(params["members"].as_array().unwrap().len(), 6);
        params["closure"] = prior["closure"].clone();
        assert_eq!(
            restored, *old,
            "only parameters.closure changes; sockets/quality/placements/members stay exact"
        );
        let physical = d["physical_input_bindings"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["template"] == binding["template"])
            .unwrap();
        let defaults = d["template_defaults"]
            .as_array()
            .unwrap()
            .iter()
            .find(|x| x["template"] == binding["template"])
            .unwrap();
        // The entire admitted parameter path has six destinations, including
        // explicit absent-header defaults. Other record fields are not ports.
        let mut destinations: BTreeSet<String> = physical["header_inputs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["slot"].to_string())
            .collect();
        destinations.insert(physical["corruption_slot"].to_string());
        destinations.insert(physical["capacity_slot"].to_string());
        for row in defaults["parameters"].as_array().unwrap() {
            destinations.insert(row["assignment"]["slot"].to_string());
        }
        assert_eq!(
            destinations,
            prior["members"]
                .as_array()
                .unwrap()
                .iter()
                .map(Value::to_string)
                .collect()
        );
        assert_eq!(binding["parameters"].as_array().unwrap().len(), 6);
        for (j, label) in [
            "catalyst-kind",
            "catalyst-amount",
            "rarity",
            "corrupted",
            "raw-level-requirement",
            "socket-capacity",
        ]
        .into_iter()
        .enumerate()
        {
            let port = &binding["parameters"][j];
            assert_eq!(port["name"], label);
            assert_eq!(port["slot"], prior["members"][j]);
            let number = if i == 2 && j < 2 {
                0x09f9 + j
            } else {
                first + j
            };
            assert_eq!(port["slot"]["slot"]["key"], format!("def.{number:016x}"));
            let slots: Vec<_> = d["slots"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["value"]["id"] == port["slot"])
                .collect();
            assert_eq!(slots.len(), 1);
            let schema = &slots[0]["value"]["schema"]["value"];
            assert_eq!(
                schema["presence"],
                if j == 4 {
                    "optional_once"
                } else {
                    "required_once"
                }
            );
            assert_eq!(schema["sites"], json!(["item_parameter"]));
        }
    }
    for owner in d["owners"].as_array().unwrap() {
        assert_eq!(owner["programs"]["closure"]["kind"], "partial");
    }
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for field in [
        "socket_configuration_implemented",
        "derived_quality_implemented",
        "requirements_implemented",
        "modifier_inventory_closed",
        "numerical_owners_closed",
        "whole_build_parity",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    check_vectors(&a, &b, &v, false);
}
fn unchanged_dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(d["slots"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .slots
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<DefinitionRules>>(d["owners"].clone()).unwrap() {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
    let normalization = json!(endpoint.input().normalization);
    let source = json!(endpoint.input().item_source);
    for row in d["physical_input_bindings"].as_array().unwrap() {
        assert_eq!(
            normalization["item_parameter_inputs"]["templates"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    for row in d["template_defaults"].as_array().unwrap() {
        assert_eq!(
            source["template_defaults"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    let a: Value = read("authoring.json");
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    for entry in &m.schema {
        let SchemaExtensionEntry::Definition(expected) = entry else {
            unreachable!()
        };
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == expected)
                .count(),
            1
        );
    }
    unchanged_dependencies(endpoint, &d);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &b, &v, true);
    let receipt = json!(prior.receipt());
    assert_eq!(receipt["input"], a["before"]);
    for field in [
        "definitions",
        "registry",
        "rules",
        "mapping",
        "roles",
        "normalization",
    ] {
        assert_eq!(receipt[field], a[field]);
    }
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["definitions"].clone()).unwrap();
    for row in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    unchanged_dependencies(prior, &d);
    assert!(prior.evaluation().is_none());
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(inverse, *migrated.input());
    let mut restored = next.input().recipe.clone();
    for row in old {
        let target = restored
            .schema
            .definitions
            .iter_mut()
            .find(|x| x.address() == row.address())
            .unwrap();
        *target = row;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only three parameter inventories change; rules, registry and all other schemas remain exact"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn snapshot(source: &Value) -> Value {
    let mut value = source.clone();
    for field in ["active", "base_mods", "lists", "armourData"] {
        value.as_object_mut().unwrap().remove(field);
    }
    value
}
fn project(item: &Value) -> Value {
    let mut value = serde_json::Map::new();
    for field in ["id", "base_facts", "raw"] {
        value.insert(field.into(), item[field].clone());
    }
    for field in ["loaded", "fresh", "fresh_after_reparse"] {
        if let Some(raw) = item.get(field) {
            value.insert(field.into(), snapshot(raw));
        }
    }
    let probes = item
        .get("probes")
        .and_then(Value::as_array)
        .map(|probes| {
            probes
                .iter()
                .map(|p| json!({"name":p["name"],"value":snapshot(&p["after"])}))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    value.insert("probes".into(), json!(probes));
    Value::Object(value)
}
fn check_vectors(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(a["source_revision"], manifest["upstream_revision"]);
    let mut paths = BTreeSet::new();
    for pin in a["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| *x == pin)
                .count(),
            1
        );
        if full {
            let source = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(source.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(source.as_bytes()), pin["sha256"]);
        }
    }
    assert_eq!(paths.len(), 11);
    let families = v["families"].as_array().unwrap();
    assert_eq!(families.len(), 2);
    for (index, family) in families.iter().enumerate() {
        let name = if index == 0 { "armour" } else { "sapphire" };
        assert_eq!(family["family"], name);
        let names = if index == 0 {
            json!(["original", "display-and-affix-headers", "xml-variant-two"])
        } else {
            json!([
                "original",
                "quality-twenty",
                "legacy-range-zero",
                "legacy-range-one",
                "xml-variant-two",
                "loader-fallback-title"
            ])
        };
        assert_eq!(family["case_names"], names);
        assert_eq!(
            family["report_metadata"]["source_hash"],
            a["source_manifest_sha256"]
        );
        for pin in family["report_metadata"]["evidence"]["files"]
            .as_array()
            .unwrap()
        {
            assert!(
                a["source_files"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
            );
        }
        let obs = family["observations"].as_array().unwrap();
        assert_eq!(obs.len(), 6);
        for (i, observation) in obs.iter().enumerate() {
            let (case, item) = if index == 0 { (i / 2, i % 2) } else { (i, 0) };
            let binding = &b["templates"][if index == 0 { item } else { 2 }];
            assert_eq!(observation["case_index"], case);
            assert_eq!(observation["item_index"], item);
            assert_eq!(observation["case"], names[case]);
            assert_eq!(
                observation["pointer"],
                format!("/cases/{case}/state/items/{item}")
            );
            let source = &observation["value"];
            assert_eq!(source["id"], binding["source_item_id"]);
            assert_eq!(source["loaded"]["base"], binding["name"]);
            if case == 0 {
                assert_eq!(source["loaded"], source["fresh"]);
                assert_eq!(source["fresh_after_reparse"], source["fresh"]);
                let probes = source["probes"].as_array().unwrap();
                assert_eq!(
                    probes.len(),
                    if index == 1 {
                        44
                    } else if item == 0 {
                        19
                    } else {
                        16
                    }
                );
                let probe =
                    |name: &str| &probes.iter().find(|p| p["name"] == name).unwrap()["value"];
                assert_eq!(probe("corrupted")["corrupted"], true);
                assert_eq!(probe("catalyst-zero")["catalystQuality"], 0);
                assert_eq!(
                    probe("catalyst-amount-only")["field_types"]["catalyst"],
                    "nil"
                );
                assert!(
                    probe("level-high")["requirements"]["level"]
                        .as_u64()
                        .is_some()
                );
                assert_eq!(
                    probe("level-absent")["requirements"]["level"],
                    source["base_facts"]["requirements"]["level"]
                );
            }
        }
        assert_eq!(family["reports"].as_array().unwrap().len(), 2);
        assert_eq!(
            family["reports"][0]["sha256"],
            family["reports"][1]["sha256"]
        );
        if !full {
            continue;
        }
        let mut first = None;
        for (i, pin) in family["reports"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                pin["path"],
                format!(
                    "runs/owned-{name}-item-inputs-source-01/source-jit-{}.json",
                    if i == 0 { "off" } else { "on" }
                )
            );
            let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), pin["sha256"]);
            if let Some(previous) = &first {
                assert_eq!(&bytes, previous);
            } else {
                first = Some(bytes.clone());
            }
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            let mut metadata = report.clone();
            metadata.as_object_mut().unwrap().remove("cases");
            assert_eq!(metadata, family["report_metadata"]);
            assert_eq!(
                report["cases"].as_array().unwrap().len(),
                names.as_array().unwrap().len()
            );
            for observation in obs {
                let ci = observation["case_index"].as_u64().unwrap() as usize;
                assert_eq!(report["cases"][ci]["name"], observation["case"]);
                for field in [
                    "original_functions_preserved",
                    "main_output_preserved",
                    "saved_selections_preserved",
                ] {
                    assert_eq!(report["cases"][ci]["state"][field], true);
                }
                assert_eq!(
                    project(
                        report
                            .pointer(observation["pointer"].as_str().unwrap())
                            .unwrap()
                    ),
                    observation["value"]
                );
            }
        }
    }
}
