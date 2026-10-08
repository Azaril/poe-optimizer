//! Reuse retained source observations for the joined item -> Sniper -> Basic
//! Attack component. No new source run, copied oracle or coverage claim.
#[allow(dead_code)]
#[path = "minion_attack_source_vectors.rs"]
mod vectors;

use super::activation_family::{self, population_partition as population};
use poe_optimizer_core::{
    owned_routing::{ActionOutputRoutes, ActionRoutingInput},
    owned_rules::{DefinitionRules, IntegerRuleTable, RuleProgram},
    owned_schema::{DefinitionDescriptor, SchemaSubject, SlotDescriptor},
};
use poe_optimizer_import::{
    owned_recipe_extension::{OwnedRecipeExtension, SchemaExtensionEntry},
    owned_release::StagedOwnedRelease,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf};

const DATA: &str = "data/owned/poe2/3887ae68/";
const CALIBRATION: &str = "tests/fixtures/calibration/intrinsic-minion-attack-3887ae68.json";
const SNAPSHOT: &str =
    "crates/poe-optimizer-engine/tests/support/minion_attack_source_snapshot.json";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn read<T: DeserializeOwned>(path: &str) -> T {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn decode<T: DeserializeOwned>(v: &Value) -> T {
    serde_json::from_value(v.clone()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn one<'a>(values: &'a [Value], field: &str, wanted: &Value) -> &'a Value {
    let found: Vec<_> = values.iter().filter(|v| v[field] == *wanted).collect();
    assert_eq!(found.len(), 1, "unique {field}={wanted}");
    found[0]
}

/// Four measured raw-level changes with the original Crown and Solar equipped.
/// Values stay in the committed source projection's shape; no expected damage
/// formula is reimplemented here. Raw40 is retained as a recovery diagnostic.
pub fn checked_cases() -> Vec<Value> {
    super::sniper_family::check_source(false);
    let a: Value = read(&format!("{DATA}minion-attack-source/authoring.json"));
    let v: Value = read(CALIBRATION);
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["source_revision"], a["source_revision"]);
    assert_eq!(v["source_revision"], manifest["upstream_revision"]);
    assert_eq!(v["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(v["source_hash"], hash(&bytes));
    for file in v["source_files"].as_array().unwrap() {
        let actual = one(manifest["files"].as_array().unwrap(), "path", &file["path"]);
        assert_eq!(actual["sha256"], file["sha256"]);
    }
    for file in a["source_files"].as_array().unwrap() {
        let actual = one(manifest["files"].as_array().unwrap(), "path", &file["path"]);
        assert_eq!(actual, file);
    }
    assert_eq!(
        v["observer_sha256"],
        hash(&fs::read(root().join(a["source_observer"].as_str().unwrap())).unwrap())
    );
    let identity = &v["owned_ability_identity"];
    assert_eq!(identity["source_effect"], "MinionMeleeBow");
    assert_eq!(identity["source_display"], "Basic Attack");
    assert_eq!(identity["skill"]["key"], "def.0000000000000021");
    assert_eq!(identity["output"]["slot"]["key"], "def.0000000000000022");
    assert_eq!(
        identity["output"]["declaration"]["definition"],
        identity["skill"]
    );
    for name in ["mapping", "bindings"] {
        let pin = &identity[name];
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    assert_eq!(
        v["observed_counts"],
        json!({"sniper_actors":18,"sniper_consumers":8,"spectre_actors":4,"spectre_consumers":4})
    );
    let rows = v["sniper"].as_array().unwrap();
    assert_eq!(rows.len(), 5);
    for row in rows {
        assert_eq!(row["summon_effect_id"], "SummonSkeletalSnipersPlayer");
        assert_eq!(row["actor_profile"], "RaisedSkeletonSniper");
        assert_eq!(row["physical_quality"], 0);
        assert_eq!(row["table_actor_level"], row["actor_level"]);
        assert_eq!(row["curve"]["level"], row["actor_level"]);
        assert_eq!(row["curve"]["kind"], "allied");
        assert!(
            row["policy"]
                .as_object()
                .unwrap()
                .values()
                .all(|x| x.as_bool() == Some(false))
        );
        let children = row["children"].as_array().unwrap();
        assert_eq!(children.len(), 1);
        let child = &children[0];
        assert_eq!(child["effect_id"], identity["source_effect"]);
        assert_eq!(
            child["effect_level"], 1,
            "child level is not final summon level"
        );
        assert_eq!(child["actor_level"], row["actor_level"]);
        assert_eq!(child["selected"], true);
        let passes = child["consumer"]["passes"].as_array().unwrap();
        assert_eq!(passes.len(), 1);
        assert_eq!(passes[0]["label"], "Main Hand");
        assert_eq!(passes[0]["copied_from_actor"], true);
        assert_eq!(passes[0]["source"], row["weapon1"]);
    }
    let baseline = one(rows, "physical_level", &json!(20));
    let inputs: Value = read(&format!("{DATA}sniper-final-inputs/source-vectors.json"));
    let original = &inputs["reports"][0]["projections"][0]["value"];
    for (field, source) in [
        ("physical_level", "raw_level"),
        ("physical_quality", "raw_quality"),
        ("effective_level", "final_level"),
        ("actor_level", "actor_level"),
    ] {
        assert_eq!(baseline[field], original[source]);
    }
    let properties = original["external_properties"].as_array().unwrap();
    assert_eq!(properties.len(), 3);
    assert_eq!(
        properties[0]["mod"]["source"],
        "Item:21:New Item, Iron Crown"
    );
    assert_eq!(
        properties[1]["mod"]["source"],
        "Item:23:New Item, Solar Amulet"
    );
    for property in &properties[..2] {
        assert_eq!(property["value"]["key"], "level");
        assert_eq!(property["value"]["value"], 1);
        assert_eq!(property["value"]["keyword"], "minion");
    }
    assert_eq!(
        properties[2]["mod"]["source"],
        "Many Sources:^x88FFFF0% Amulet Bonus Effect"
    );
    assert_eq!(properties[2]["value"]["key"], "level");
    assert_eq!(properties[2]["value"]["value"], 0);
    assert_eq!(properties[2]["value"]["keyword"], "minion");
    // PoB recovers raw40 + two item levels to final40. The owned assembler's
    // explicit final1..40 guard refuses42; this row is never a joined success.
    assert_eq!(
        one(rows, "physical_level", &json!(40))["effective_level"],
        40
    );
    [1, 7, 20, 30]
        .into_iter()
        .map(|raw| one(rows, "physical_level", &json!(raw)).clone())
        .collect()
}

fn program(endpoint: &StagedOwnedRelease, owner: &SchemaSubject, expected: &RuleProgram) {
    let owners: Vec<_> = endpoint
        .input()
        .recipe
        .rules
        .owners
        .iter()
        .filter(|actual| &actual.owner == owner)
        .collect();
    assert_eq!(owners.len(), 1);
    assert!(
        !owners[0].programs.is_complete(),
        "component evidence cannot close production owner"
    );
    // The successor's checked structural partition has one exact inverse. Use
    // that authenticated owner to compare the retained bundled source body;
    // accepting only its numerical subset would weaken this evidence check.
    let restored = activation_family::restored_owner(endpoint, owner);
    let reference = restored.as_ref().unwrap_or(owners[0]);
    let actual: Vec<_> = reference
        .programs
        .members
        .iter()
        .filter(|p| p.id == expected.id)
        .collect();
    assert_eq!(
        actual,
        vec![expected],
        "exact current {}",
        expected.id.as_str()
    );
}

/// Authenticate current published dependencies, allowing the reviewed population
/// partition and later unrelated owner additions. This never edits coverage.
pub fn assert_current(endpoint: &StagedOwnedRelease) {
    checked_cases();
    super::sniper_family::assert_component(endpoint);
    population::check_authored();
    let partition: population::Partition = population::read("partition.json");
    let old: Value = read(SNAPSHOT);
    let owners: Vec<DefinitionRules> = decode(&old["owners"]);
    for owner in &owners {
        for expected in &owner.programs.members {
            if owner.owner == partition.owner && expected.id == partition.original.id {
                assert_eq!(expected, &partition.original);
                program(endpoint, &owner.owner, &partition.facts.program);
                program(endpoint, &owner.owner, &partition.requirements.program);
            } else if expected.id.as_str() != "gas-arrow-supply"
                && json!(owner.owner)["value"]["value"]["slot"]["key"] != "def.0000000000000025"
            {
                program(endpoint, &owner.owner, expected);
            }
        }
    }
    let extension: OwnedRecipeExtension =
        read(&format!("{DATA}minion-attack-source/extension.json"));
    for owner in &extension.owners {
        for expected in &owner.programs.members {
            program(endpoint, &owner.owner, expected);
        }
    }
    let schema = &endpoint.input().recipe.schema;
    for descriptor in decode::<Vec<DefinitionDescriptor>>(&old["schema_definitions"]) {
        // Preserve exact dependencies, without requiring any entire historical
        // owner to equal its successor after later independently checked work.
        assert_eq!(
            schema
                .definitions
                .iter()
                .filter(|d| d.address() == descriptor.address())
                .collect::<Vec<_>>(),
            vec![&descriptor]
        );
    }
    for slot in decode::<Vec<SlotDescriptor>>(&old["schema_slots"]) {
        if json!(slot)["value"]["id"]["declaration"]["definition"]["key"] == "def.0000000000000024"
        {
            continue; // Gas Arrow has independent later mechanics.
        }
        assert_eq!(
            schema
                .slots
                .iter()
                .filter(|s| s.address() == slot.address())
                .collect::<Vec<_>>(),
            vec![&slot]
        );
    }
    for entry in extension.schema {
        let SchemaExtensionEntry::Definition(descriptor) = entry else {
            panic!("intrinsic extension definitions")
        };
        assert_eq!(
            schema
                .definitions
                .iter()
                .filter(|d| d.address() == descriptor.address())
                .collect::<Vec<_>>(),
            vec![&descriptor]
        );
    }
    for table in decode::<Vec<IntegerRuleTable>>(&old["tables"]) {
        assert_eq!(
            endpoint
                .input()
                .recipe
                .rules
                .tables
                .iter()
                .filter(|t| t.id == table.id)
                .collect::<Vec<_>>(),
            vec![&table]
        );
    }
    let routes: Vec<ActionOutputRoutes> = read(&format!("{DATA}minion-attack-source/routes.json"));
    let v2: ActionRoutingInput = read(&format!("{DATA}ice-nova-intrinsics/routing.json"));
    assert_eq!(v2.schema_version, 2);
    assert_eq!(routes.len(), 1);
    for expected in routes {
        let actual: Vec<_> = endpoint
            .input()
            .recipe
            .routing
            .outputs
            .iter()
            .filter(|r| r.output == expected.output)
            .collect();
        assert_eq!(actual.len(), 1);
        assert!(!actual[0].routes.is_complete());
        assert_eq!(actual[0].routes.closure, expected.routes.closure);
        // The checked Ice Nova V1 -> V2 adaptation gave every prior output an
        // explicit empty selector inventory with its unchanged Partial closure.
        // Require that exact declaration, rather than treating absence as empty
        // or allowing a later Complete declaration to pass this component proof.
        assert!(expected.source_selectors.is_none());
        let adapted: Vec<_> = v2
            .outputs
            .iter()
            .filter(|r| r.output == expected.output)
            .collect();
        assert_eq!(adapted.len(), 1);
        let selectors = adapted[0].source_selectors.as_ref().unwrap();
        assert!(selectors.members.is_empty());
        assert_eq!(selectors.closure, expected.routes.closure);
        assert_eq!(actual[0].source_selectors, adapted[0].source_selectors);
        for route in expected.routes.members {
            assert_eq!(
                actual[0]
                    .routes
                    .members
                    .iter()
                    .filter(|r| r.id == route.id)
                    .collect::<Vec<_>>(),
                vec![&route]
            );
        }
    }
}

/// Optional retained-report replay. It authenticates both existing JIT results
/// and recomputes the shared projection, without loading or running PoB.
pub fn check_retained_reports() {
    checked_cases();
    super::sniper_family::check_source(true);
    let a: Value = read(&format!("{DATA}minion-attack-source/authoring.json"));
    let p = &a["source_validation"];
    assert_eq!(p["status"], "passed");
    assert_eq!(p["whole_build_parity"], false);
    let off = fs::read(root().join(p["evidence_json"].as_str().unwrap())).unwrap();
    let on = fs::read(root().join(p["evidence_on_json"].as_str().unwrap())).unwrap();
    assert_eq!(off, on, "exact retained JIT-independent fresh construction");
    assert_eq!(off.len() as u64, p["evidence_bytes"].as_u64().unwrap());
    assert_eq!(hash(&off), p["evidence_sha256"]);
    let raw: Value = serde_json::from_slice(&off).unwrap();
    for field in [
        "actor_level_mutation",
        "business_method_wrappers",
        "native_coverage",
        "whole_build_parity",
    ] {
        assert_eq!(raw["evidence"][field], false);
    }
    assert_eq!(vectors::project(&raw), read::<Value>(CALIBRATION));
    for original in raw["evidence"]["originals"].as_array().unwrap() {
        assert_eq!(
            hash(
                &fs::read(
                    root()
                        .join("tests/fixtures/builds/breadth-20260908")
                        .join(original["name"].as_str().unwrap())
                )
                .unwrap()
            ),
            original["sha256"]
        );
    }
    for name in [
        "original-05",
        "sniper-physical-1",
        "sniper-physical-7",
        "sniper-physical-30",
        "sniper-physical-40",
    ] {
        let case = one(raw["cases"].as_array().unwrap(), "name", &json!(name));
        for mode in ["main", "calcs"] {
            let actor = one(
                case["state"][mode]["actors"].as_array().unwrap(),
                "summon_effect_id",
                &json!("SummonSkeletalSnipersPlayer"),
            );
            assert_eq!(actor["source_group"], 3);
            assert_eq!(actor["source_gem"], 1);
        }
    }
}
