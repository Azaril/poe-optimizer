//! One source-bound placement inventory; every other item domain stays exact.
use super::migration_preservation;
use poe_optimizer_core::{
    owned_content::{OwnedContentDigest, digest_owned},
    owned_definitions::OwnedDefinitionKey,
    owned_rules::DefinitionRules,
    owned_schema::DefinitionDescriptor,
};
use poe_optimizer_import::{
    owned_recipe_extension::SchemaExtensionEntry,
    owned_release::{OwnedReleaseProvenance, StagedOwnedRelease, assemble_owned_release},
    owned_release_migration::{OwnedReleaseMigrationInput, compile_owned_release_migration},
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
};

pub const KIND: &str = "source-bound-leggings-placement";
const DOMAIN: &str = "owned-leggings-placement-v1";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/leggings-placement")
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
        ("replaced_definitions", 1),
        ("closed_equipment_slot_inventories", 1),
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
    assert_eq!(m.release.as_str(), "pob-3887ae68-leggings-placement-v1");
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
    assert_eq!(m.schema.len(), 1);
    assert_eq!(d["definitions"].as_array().unwrap().len(), 2);
    assert_eq!(d["owners"].as_array().unwrap().len(), 1);
    assert_eq!(b["template"]["key"], "def.0000000000001e0e");
    assert_eq!(b["equipment_slot"]["key"], "def.0000000000000069");
    assert_eq!(b["base_name"], "Cryptic Leggings");
    assert_eq!(b["source_item_id"], 22);
    assert_eq!(b["source_ordinal"], 578);
    assert_eq!(b["source_slot"], "Boots");
    assert_eq!(b["original_item"]["local"], "000000000000028e");
    assert_eq!(b["original_uses"].as_array().unwrap().len(), 4);
    for (row, id) in b["original_uses"]
        .as_array()
        .unwrap()
        .iter()
        .zip(["02ca", "02d4", "02de", "02e8"])
    {
        assert_eq!(row["id"]["local"], format!("000000000000{id}"));
        assert_eq!(
            row["item"],
            json!({"kind":"known","value":b["original_item"]})
        );
        assert_eq!(
            row["destination"],
            json!({"kind":"character_slot","value":{"kind":"known","value":b["equipment_slot"]}})
        );
        assert_eq!(
            row["scope"],
            json!({"kind":"known","value":{"kind":"shared"}})
        );
    }
    let old = &d["definitions"][0];
    assert_eq!(old["value"]["id"], b["template"]);
    assert_eq!(d["definitions"][1]["value"]["id"], b["equipment_slot"]);
    let SchemaExtensionEntry::Definition(new) = &m.schema[0] else {
        panic!("only one exact descriptor replacement")
    };
    let mut inverse = json!(new);
    let slots = &mut inverse["value"]["schema"]["value"]["equipment_slots"];
    let old_slots = &old["value"]["schema"]["value"]["equipment_slots"];
    assert_eq!(old_slots["members"], json!([]));
    assert_eq!(old_slots["closure"]["kind"], "partial");
    assert_eq!(
        *slots,
        json!({"members":[b["equipment_slot"]],"closure":{"kind":"complete"}})
    );
    *slots = old_slots.clone();
    assert_eq!(
        inverse, *old,
        "only equipment_slots changes; every other field stays exact"
    );
    assert_eq!(
        d["slot_mapping"]["source"],
        json!({"kind":"catalog","value":{"kind":"equipment_slot","key":{"kind":"text","value":"Boots"},"version":{"kind":"missing"},"variant":{"kind":"missing"}}})
    );
    assert_eq!(
        d["slot_mapping"]["outcome"],
        json!({"kind":"mapped","value":{"target":{"kind":"definition","value":{"kind":"equipment_slot","value":b["equipment_slot"]}},"basis":{"kind":"exact"}}})
    );
    assert_eq!(
        d["owners"][0]["owner"],
        json!({"kind":"definition","value":{"kind":"item_template","value":b["template"]}})
    );
    assert_eq!(d["owners"][0]["programs"]["closure"]["kind"], "partial");
    assert_eq!(a["scope"], b["scope"]);
    assert_eq!(v["scope"], b["scope"]);
    for field in [
        "numerical_owners_closed",
        "item_set_membership_changed",
        "stock_modeled",
        "socket_configuration_implemented",
        "whole_build_parity",
    ] {
        assert_eq!(b["scope"][field], false);
    }
    for (name, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{name}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r') && !bytes.starts_with(&[0xef, 0xbb, 0xbf]));
        assert_eq!(*expected, hash(&bytes));
    }
    check_source(&a, &b, &v, false);
}
fn unchanged_dependencies(endpoint: &StagedOwnedRelease, d: &Value) {
    let slot: DefinitionDescriptor = serde_json::from_value(d["definitions"][1].clone()).unwrap();
    assert_eq!(
        endpoint
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| **x == slot)
            .count(),
        1
    );
    let owner: DefinitionRules = serde_json::from_value(d["owners"][0].clone()).unwrap();
    assert_eq!(
        endpoint
            .input()
            .recipe
            .rules
            .owners
            .iter()
            .filter(|x| **x == owner)
            .count(),
        1
    );
    let mapping = json!(endpoint.input().mapping);
    assert_eq!(
        mapping["entries"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| **x == d["slot_mapping"])
            .count(),
        1
    );
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
    let SchemaExtensionEntry::Definition(expected) = &m.schema[0] else {
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
    unchanged_dependencies(endpoint, &d);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    check_source(&a, &b, &v, true);
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
    let old: DefinitionDescriptor = serde_json::from_value(d["definitions"][0].clone()).unwrap();
    assert_eq!(
        prior
            .input()
            .recipe
            .schema
            .definitions
            .iter()
            .filter(|x| **x == old)
            .count(),
        1
    );
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
    let target = restored
        .schema
        .definitions
        .iter_mut()
        .find(|x| x.address() == old.address())
        .unwrap();
    *target = old;
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "only the one equipment-slot inventory changes"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
fn without_calls(mut host: Value) -> Value {
    let state = host["state"].as_object_mut().unwrap();
    for name in ["calls", "executed", "boundary"] {
        state.remove(name);
    }
    host
}
fn project(report: &Value) -> Value {
    assert_eq!(report["original"], report["repeat"]);
    assert_eq!(
        without_calls(report["control"].clone()),
        without_calls(report["original"].clone())
    );
    assert_eq!(report["control"]["state"]["executed"], false);
    let mut state = report["original"]["state"].clone();
    let slots: BTreeSet<_> = state["registered_slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(slots.len(), 113);
    let sets: BTreeSet<_> = std::iter::once("active-default".to_owned())
        .chain((1..=6).map(|i| format!("saved-{i}")))
        .collect();
    let flags: BTreeSet<_> = std::iter::once("actual".to_owned())
        .chain((0..8).map(|i| format!("mask-{i}")))
        .collect();
    let calls = state["calls"].as_array().unwrap();
    assert_eq!(calls.len(), slots.len() * sets.len() * flags.len());
    let mut keys = BTreeSet::new();
    let mut outcomes: BTreeMap<&str, BTreeMap<String, (Value, usize)>> = BTreeMap::new();
    for call in calls {
        let set = call["set"].as_str().unwrap();
        let slot = call["slot"].as_str().unwrap();
        let flag = call["flags"].as_str().unwrap();
        assert!(sets.contains(set) && slots.contains(slot) && flags.contains(flag));
        assert!(keys.insert((set, slot, flag)));
        let outcome = &call["outcome"];
        let group = outcomes
            .entry(slot)
            .or_default()
            .entry(outcome.to_string())
            .or_insert_with(|| (outcome.clone(), 0));
        group.1 += 1;
    }
    let slot_outcomes: Vec<_> = outcomes
        .into_iter()
        .map(|(slot, groups)| {
            let groups: Vec<_> = groups
                .into_values()
                .map(|(outcome, count)| json!({"outcome":outcome,"count":count}))
                .collect();
            json!({"slot":slot,"outcomes":groups})
        })
        .collect();
    let state_object = state.as_object_mut().unwrap();
    for field in ["calls", "read_set", "main_output"] {
        state_object.remove(field);
    }
    json!({"state":state,"slot_outcomes":slot_outcomes,"call_count":7119,"set_count":7,"flag_count":9})
}
fn check_source(a: &Value, b: &Value, v: &Value, full: bool) {
    assert_eq!(v["status"], "passed");
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest_bytes), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 7);
    let mut unique = BTreeSet::new();
    for pin in pins {
        assert!(unique.insert(pin["path"].as_str().unwrap()));
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
            let text = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(text.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(text.as_bytes()), pin["sha256"]);
        }
    }
    let metadata = &v["report_metadata"];
    assert_eq!(metadata["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(
        metadata["source_item"],
        json!({"id":22,"ordinal":578,"content_entry":0})
    );
    assert_eq!(metadata["source_xml_sha256"], a["source_xml_sha256"]);
    let evidence = &metadata["evidence"];
    assert_eq!(evidence["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(evidence["scope"], "registered-source-slot-catalogue-only");
    assert_eq!(evidence["fresh_runtimes"], 3);
    for field in [
        "unhooked",
        "independent_replay_equal",
        "no_call_control_equal",
    ] {
        assert_eq!(evidence[field], true);
    }
    for field in ["binding_sha256", "observer_sha256"] {
        assert_eq!(evidence[field], a[field]);
    }
    assert_eq!(evidence["files"].as_array().unwrap().len(), pins.len());
    for pin in evidence["files"].as_array().unwrap() {
        assert_eq!(
            pins.iter()
                .filter(|p| p["path"] == pin["path"] && p["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    let projection = &v["projection"];
    assert_eq!(projection["call_count"], 7119);
    assert_eq!(projection["set_count"], 7);
    assert_eq!(projection["flag_count"], 9);
    let state = &projection["state"];
    assert_eq!(state["executed"], true);
    assert_eq!(
        state["method"],
        json!({"path":"Classes/ItemsTab.lua","first":2603,"last":2687})
    );
    assert_eq!(
        state["constructor_wrapper"],
        json!({"path":"Modules/Common.lua","first":167,"last":183})
    );
    assert_eq!(
        state["constructor"],
        json!({"path":"Classes/ItemsTab.lua","first":140,"last":1191})
    );
    assert_eq!(
        state["loader"],
        json!({"path":"Classes/ItemsTab.lua","first":1193,"last":1320})
    );
    let slots: Vec<_> = state["registered_slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect();
    assert_eq!(slots.len(), 113);
    assert!(slots.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(slots.iter().filter(|x| x.starts_with("Jewel ")).count(), 19);
    assert_eq!(
        slots
            .iter()
            .filter(|x| x.contains(" Jewel Socket "))
            .count(),
        72
    );
    assert_eq!(state["base_slots"].as_array().unwrap().len(), 20);
    for name in state["base_slots"].as_array().unwrap() {
        assert!(slots.contains(&name.as_str().unwrap()));
    }
    assert_eq!(state["set_ids"], json!([1, 2, 3, 4, 5, 6]));
    assert_eq!(state["item"]["id"], b["source_item_id"]);
    assert_eq!(state["item"]["base_name"], b["base_name"]);
    assert_eq!(state["item"]["item_type"], "Boots");
    assert_eq!(state["item"]["base"]["type"], "Boots");
    assert_eq!(state["item"]["base"]["subType"], "Armour/Energy Shield");
    assert_eq!(state["item"]["base"]["socketLimit"], 3);
    for tag in [
        "onehand",
        "twohand",
        "one_hand_weapon",
        "axe",
        "mace",
        "sword",
    ] {
        assert!(state["item"]["base"]["tags"].get(tag).is_none());
    }
    assert_eq!(
        state["selected"],
        json!({"items":2,"spec":3,"skills":4,"config":1,"group":3})
    );
    assert_eq!(
        state["selected_uses"],
        json!([{"set":2,"slot":"Boots"},{"set":3,"slot":"Boots"},{"set":4,"slot":"Boots"},{"set":5,"slot":"Boots"}])
    );
    assert_eq!(state["flag_cases"].as_array().unwrap().len(), 9);
    assert_eq!(state["flag_cases"][0], json!({"id":"actual"}));
    for mask in 0..8 {
        assert_eq!(
            state["flag_cases"][mask + 1],
            json!({"id":format!("mask-{mask}"),"values":{"giantsBlood":mask&1!=0,"instrumentsOfPower":mask&2!=0,"lordOfTheWilds":mask&4!=0}})
        );
    }
    let outcomes = projection["slot_outcomes"].as_array().unwrap();
    assert_eq!(outcomes.len(), slots.len());
    for (row, slot) in outcomes.iter().zip(slots) {
        assert_eq!(row["slot"], slot);
        let groups = row["outcomes"].as_array().unwrap();
        assert!(!groups.is_empty());
        let mut count = 0;
        let mut unique = BTreeSet::new();
        for group in groups {
            let result = &group["outcome"];
            assert!(unique.insert(result.to_string()));
            let n = group["count"].as_u64().unwrap();
            assert!(n > 0);
            count += n;
            match result["kind"].as_str().unwrap() {
                "none" => assert_eq!(*result, json!({"kind":"none","return_count":0})),
                "nil" => assert_eq!(*result, json!({"kind":"nil","return_count":1})),
                "boolean" => {
                    assert_eq!(result["return_count"], 1);
                    assert!(result["value"].is_boolean());
                }
                other => panic!("unrepresented source outcome {other}"),
            }
            assert_eq!(result["value"] == true, slot == "Boots");
        }
        assert_eq!(
            count, 63,
            "all seven contexts and nine flag cases remain represented"
        );
    }
    assert_eq!(
        state["boundary"],
        json!({"slot":"Boots 1","registered":false,"outcome":{"return_count":1,"kind":"boolean","value":true},"native_admission":false})
    );
    for field in [
        "exact_catalogue_base",
        "exact_registered_item",
        "exact_selected_main_and_calcs_item",
        "original_functions_preserved",
        "saved_items_preserved",
        "saved_selections_preserved",
        "main_output_preserved",
        "repeated_calls_equal",
        "read_set_unchanged",
    ] {
        assert_eq!(state["evidence"][field], true);
    }
    for field in [
        "call_hook",
        "method_wrappers",
        "native_owner_coverage",
        "numerical_parity",
        "socket_configuration_admission",
    ] {
        assert_eq!(state["evidence"][field], false);
    }
    let reports = v["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
    assert_eq!(reports[0]["bytes"], reports[1]["bytes"]);
    if !full {
        return;
    }
    for (field, path) in [
        (
            "binding_sha256",
            "crates/poe-optimizer-pob/tests/support/item_slot_validity_source.lua",
        ),
        (
            "observer_sha256",
            "crates/poe-optimizer-pob/tests/support/leggings_placement_source.lua",
        ),
    ] {
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), a[field]);
    }
    assert_eq!(
        hash(
            &fs::read(root().join("tests/fixtures/builds/breadth-20260908/build-05.xml")).unwrap()
        ),
        a["source_xml_sha256"]
    );
    let mut first = None;
    for pin in reports {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        if let Some(previous) = &first {
            assert_eq!(&bytes, previous);
        } else {
            first = Some(bytes.clone());
        }
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut actual_metadata = report.clone();
        for field in ["control", "original", "repeat"] {
            actual_metadata.as_object_mut().unwrap().remove(field);
        }
        assert_eq!(actual_metadata, *metadata);
        assert_eq!(project(&report), *projection);
    }
}
