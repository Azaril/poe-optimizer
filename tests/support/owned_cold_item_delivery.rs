//! Finite ordinary unscaled item delivery, independent of final resistance.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::{OwnedDefinitionKey, StatDefinition},
    owned_rules::DefinitionRules,
    owned_schema::{DefinitionDescriptor, SchemaClosure, SchemaDefinitionId, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_mapping::OwnedIdRegistry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

pub const KIND: &str = "source-bound-cold-item-delivery";
const DOMAIN: &str = "owned-cold-item-delivery-v1";
const CASES: [&str; 6] = [
    "original",
    "quality-twenty",
    "legacy-range-zero",
    "legacy-range-one",
    "xml-variant-two",
    "loader-fallback-title",
];
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/cold-item-delivery")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let assets: Vec<Value> = [
        "authoring",
        "bindings",
        "dependencies",
        "coverage",
        "source-vectors",
    ]
    .map(|name| read(&format!("{name}.json")))
    .into();
    let migration: OwnedReleaseMigrationInput = read("migration.json");
    digest_owned(DOMAIN, &(assets, migration), 1024 * 1024).unwrap()
}
fn old_owners() -> Vec<DefinitionRules> {
    serde_json::from_value(read::<Value>("dependencies.json")["owners"].clone()).unwrap()
}
fn expected_programs(b: &Value) -> [Value; 2] {
    [
        json!({"id":b["direct_program"],"context":"equipment_use",
        "reads":[{"id":"effective","value_type":{"kind":"quantity","value":{"unit":b["percent_unit"]}},"source":{"kind":"stat","value":{"entity":"modifier","stat":b["effective"]}}},
                 {"id":"applicable","value_type":{"kind":"boolean"},"source":{"kind":"stat","value":{"entity":"current","stat":b["applicability"]}}}],
        "nodes":[{"id":"effective","expression":{"kind":"read","input":"effective"}},{"id":"applicable","expression":{"kind":"read","input":"applicable"}}],
        "effects":[{"id":"direct-cold-resistance","when":"applicable","effect":{"kind":"contribute","entity":"player","stat":b["contribution"],"contribution":"add","value":"effective"}}]}),
        json!({"id":b["applicability_program"],"context":"equipment_use","reads":[],
        "nodes":[{"id":"applicable","expression":{"kind":"literal","value":{"kind":"boolean","value":true}}}],
        "effects":[{"id":"unscaled-direct-applicability","when":null,"effect":{"kind":"derive","entity":"current","stat":b["applicability"],"value":"applicable"}}]}),
    ]
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let c: Value = read("coverage.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    for (field, count) in [
        ("allocated_definitions", 1),
        ("new_programs", 2),
        ("new_receivers", 0),
        ("closed_existing_rule_owners", 0),
        ("registry_last_issued_before", 0x3313),
        ("registry_last_issued_after", 0x3314),
    ] {
        assert_eq!(a[field], count);
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
    for (field, key) in [
        ("modifier", "2542"),
        ("template", "09dc"),
        ("effective", "253e"),
        ("contribution", "09d4"),
        ("applicability", "3314"),
        ("percent_unit", "0002"),
    ] {
        assert_eq!(b[field]["key"], format!("def.000000000000{key}"));
    }
    assert_eq!(b["direct_program"], "contribute-player-cold-resistance");
    assert_eq!(
        b["applicability_program"],
        "ordinary-unscaled-numeric-item-delivery-applicability"
    );
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(json!(m.release), b["release"]);
    assert_eq!(m.release.as_str(), "pob-3887ae68-cold-item-delivery-v1");
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v20"
    );
    assert!(
        m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 1);
    assert_eq!(
        json!(m.schema[0]),
        json!({"kind":"definition","value":{"kind":"stat","value":{"id":b["applicability"],"schema":{"kind":"known","value":{"value":{"kind":"boolean"},"targets":["equipment_use"]}}}}})
    );
    let old = old_owners();
    assert_eq!(old.len(), 2);
    assert_eq!(old[0].programs.members.len(), 4);
    assert_eq!(old[1].programs.members.len(), 2);
    assert_eq!(
        json!(old[0].owner),
        json!({"kind":"definition","value":{"kind":"modifier","value":b["modifier"]}})
    );
    assert_eq!(
        json!(old[1].owner),
        json!({"kind":"definition","value":{"kind":"item_template","value":b["template"]}})
    );
    assert_eq!(m.owners.len(), 2);
    for (i, program) in expected_programs(&b).into_iter().enumerate() {
        assert_eq!(m.owners[i].owner, old[i].owner);
        assert_eq!(m.owners[i].programs.closure, old[i].programs.closure);
        assert!(!old[i].programs.is_complete());
        assert_eq!(json!(m.owners[i].programs.members), json!([program]));
    }
    assert_eq!(c["owner"], json!(old[0].owner));
    assert_eq!(c["before"], json!(old[0].programs.closure));
    assert_eq!(c["before"]["value"]["gaps"].as_array().unwrap().len(), 4);
    assert_eq!(
        c["added_codes"],
        json!([
            "ordinary-item-routing-unconverted",
            "amulet-bonus-copy-unconverted",
            "external-contributor-membership-unconverted"
        ])
    );
    let mut after = c["before"].clone();
    for code in c["added_codes"].as_array().unwrap() {
        after["value"]["gaps"]
            .as_array_mut()
            .unwrap()
            .push(json!({"subject":c["owner"],"facet":"game_rules","code":code}));
    }
    assert_eq!(
        after, c["after"],
        "preserve all old obligations and add only the explicit residuals"
    );
    assert_eq!(d["definitions"].as_array().unwrap().len(), 25);
    assert_eq!(d["slots"].as_array().unwrap().len(), 30);
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for field in [
        "whole_build_parity",
        "final_cold_resistance",
        "template_owner_closed",
        "modifier_owner_closed",
        "incoming_contributor_inventory_closed",
        "all_item_routing_implemented",
        "bonus_copy_implemented",
        "placement_or_participation_implemented",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    assert_eq!(b["scope"]["ordinary_unscaled_direct_delivery_only"], true);
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    check_vectors(&a, &v, false);
}
fn dependencies(endpoint: &StagedOwnedRelease) {
    let d: Value = read("dependencies.json");
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(d["definitions"].clone()).unwrap()
    {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| **x == row)
                .count(),
            1
        );
    }
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
}
pub fn assert_endpoint(endpoint: &StagedOwnedRelease) {
    check_authored();
    dependencies(endpoint);
    let a: Value = read("authoring.json");
    let c: Value = read("coverage.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let proof = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(proof.kind.as_str(), KIND);
    assert_eq!(json!(proof.prior_input), a["before"]);
    assert_eq!(proof.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    let expected: DefinitionDescriptor =
        serde_json::from_value(json!(m.schema[0])["value"].clone()).unwrap();
    assert_eq!(
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| **x == expected)
            .count(),
        1
    );
    for (i, mut row) in old_owners().into_iter().enumerate() {
        row.programs
            .members
            .extend(m.owners[i].programs.members.clone());
        if i == 0 {
            row.programs.closure = serde_json::from_value(c["after"].clone()).unwrap();
        }
        assert!(!row.programs.is_complete());
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
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    dependencies(prior);
    let a: Value = read("authoring.json");
    let c: Value = read("coverage.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_vectors(&a, &v, true);
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
    assert!(prior.evaluation().is_none());
    let old = old_owners();
    for row in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .rules
                .owners
                .iter()
                .filter(|x| *x == row)
                .count(),
            1
        );
    }
    let migrated = compile_owned_release_migration(prior, m, Default::default()).unwrap();
    let mut input = migrated.input().clone();
    assert_eq!(input.provenance.len(), prior.input().provenance.len() + 1);
    assert_eq!(
        input.provenance[..prior.input().provenance.len()],
        prior.input().provenance
    );
    let cold = input
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == old[0].owner)
        .unwrap();
    assert_eq!(cold.programs.closure, old[0].programs.closure);
    cold.programs.closure = serde_json::from_value::<SchemaClosure>(c["after"].clone()).unwrap();
    *input.provenance.last_mut().unwrap() = OwnedReleaseProvenance {
        kind: OwnedDefinitionKey::new(KIND).unwrap(),
        prior_input: prior.receipt().input,
        authoring_input: digest(),
    };
    let next = assemble_owned_release(input, Default::default()).unwrap();
    assert_endpoint(&next);
    let mut inverse = next.input().clone();
    inverse
        .recipe
        .rules
        .owners
        .iter_mut()
        .find(|x| x.owner == old[0].owner)
        .unwrap()
        .programs
        .closure = old[0].programs.closure.clone();
    *inverse.provenance.last_mut().unwrap() = migrated.input().provenance.last().unwrap().clone();
    assert_eq!(
        inverse,
        *migrated.input(),
        "only exact residual closure and authored provenance overlay"
    );
    let mut registry =
        OwnedIdRegistry::new(prior.input().recipe.registry.clone(), Default::default()).unwrap();
    let added = registry
        .allocate_definition::<StatDefinition>()
        .unwrap()
        .address();
    assert_eq!(next.input().recipe.registry, *registry.input());
    let mut restored = next.input().recipe.clone();
    assert_eq!(
        restored.schema.definitions.len(),
        prior.input().recipe.schema.definitions.len() + 1
    );
    restored.schema.definitions.retain(|x| x.address() != added);
    for row in old {
        let target = restored
            .rules
            .owners
            .iter_mut()
            .find(|x| x.owner == row.owner)
            .unwrap();
        *target = row;
    }
    restored.registry = prior.input().recipe.registry.clone();
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only one applicability Stat and two exact owner changes"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn project(state: &Value) -> Value {
    let mut facts = serde_json::Map::new();
    for name in [
        "main_output_preserved",
        "original_functions_preserved",
        "saved_items_preserved",
        "saved_selections_preserved",
        "selected_config",
        "selected_items",
        "selected_skills",
        "selected_spec",
    ] {
        facts.insert(name.into(), state[name].clone());
    }
    let item = &state["items"][0];
    json!({"state":facts,"item":{"id":item["id"],"base_facts":item["base_facts"],"selected_slots":item["selected_slots"],"authored_receiving_uses":item["authored_receiving_uses"],
        "loaded":{"base":item["loaded"]["base"],"type":item["loaded"]["type"],"name":item["loaded"]["name"],"active":item["loaded"]["active"]},
        "implicit":item["loaded"]["lists"]["implicit"],"player_cold_records":item["player_records"]["ColdResist"],"slot_lists":item["slot_lists"]}})
}
fn check_vectors(a: &Value, v: &Value, full: bool) {
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
    assert_eq!(paths.len(), 10);
    assert_eq!(
        v["report_metadata"]["source_hash"],
        a["source_manifest_sha256"]
    );
    assert_eq!(
        v["report_metadata"]["evidence"]["manifest_sha256"],
        a["source_manifest_sha256"]
    );
    assert_eq!(
        v["report_metadata"]["evidence"]["native_input_closure"],
        false
    );
    assert_eq!(
        v["report_metadata"]["evidence"]["native_owner_coverage"],
        false
    );
    for pin in v["report_metadata"]["evidence"]["files"]
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
    assert_eq!(v["observations"].as_array().unwrap().len(), 6);
    assert_eq!(v["native_cases"].as_array().unwrap().len(), 4);
    for (i, name) in CASES.into_iter().enumerate() {
        let obs = &v["observations"][i];
        assert_eq!(obs["case_index"], i);
        assert_eq!(obs["name"], name);
        assert_eq!(obs["pointer"], format!("/cases/{i}/state"));
        for field in [
            "main_output_preserved",
            "original_functions_preserved",
            "saved_items_preserved",
            "saved_selections_preserved",
        ] {
            assert_eq!(obs["value"]["state"][field], true);
        }
        for (field, expected) in [
            ("selected_config", 1),
            ("selected_items", 2),
            ("selected_skills", 4),
            ("selected_spec", 3),
        ] {
            assert_eq!(obs["value"]["state"][field], expected);
        }
        let item = &obs["value"]["item"];
        assert_eq!(item["id"], 26);
        assert_eq!(item["base_facts"]["name"], "Sapphire Ring");
        assert_eq!(item["base_facts"]["type"], "Ring");
        assert_eq!(item["loaded"]["base"], "Sapphire Ring");
        assert_eq!(item["loaded"]["type"], "Ring");
        assert_eq!(item["selected_slots"], json!(["Ring 1", "Ring 2"]));
        let records = item["player_cold_records"].as_array().unwrap();
        assert_eq!(records.len(), 2);
        let slots: BTreeSet<_> = records
            .iter()
            .map(|r| r["source_slot"].as_str().unwrap())
            .collect();
        assert_eq!(slots, BTreeSet::from(["Ring 1", "Ring 2"]));
        for record in records {
            assert_eq!(record["name"], "ColdResist");
            assert_eq!(record["type"], "BASE");
            assert_eq!(record["flags"], 0);
            assert_eq!(record["keyword_flags"], 0);
            assert!(
                record["tags"].as_object().is_some_and(|t| t.is_empty())
                    || record["tags"].as_array().is_some_and(|t| t.is_empty())
            );
            assert_eq!(
                record["source"],
                format!("Item:26:{}", item["loaded"]["name"].as_str().unwrap())
            );
            assert_eq!(record["value"], item["implicit"][0]["records"][0]["value"]);
        }
        if i < 4 {
            let native = &v["native_cases"][i];
            assert_eq!(native["case_index"], i);
            assert_eq!(native["name"], name);
            assert_eq!(
                native["raw_amount"],
                item["implicit"][0]["records"][0]["value"]
            );
            assert_eq!(native["expected_per_use"], records[0]["value"]);
            assert_eq!(native["source_slots"], json!(["Ring 1", "Ring 2"]));
        }
    }
    assert_eq!(v["reports"].as_array().unwrap().len(), 2);
    assert_eq!(v["reports"][0]["sha256"], v["reports"][1]["sha256"]);
    if !full {
        return;
    }
    let mut first = None;
    for (i, pin) in v["reports"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            pin["path"],
            format!(
                "runs/owned-sapphire-item-inputs-source-01/source-jit-{}.json",
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
        assert_eq!(metadata, v["report_metadata"]);
        assert_eq!(report["cases"].as_array().unwrap().len(), 6);
        for obs in v["observations"].as_array().unwrap() {
            let index = obs["case_index"].as_u64().unwrap() as usize;
            assert_eq!(report["cases"][index]["name"], obs["name"]);
            assert_eq!(
                project(report.pointer(obs["pointer"].as_str().unwrap()).unwrap()),
                obs["value"]
            );
        }
    }
}
