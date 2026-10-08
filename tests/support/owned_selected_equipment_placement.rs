//! Five exact placement inventories. No numerical or other item coverage changes.
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
pub const KIND: &str = "source-bound-selected-equipment-placement";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn data() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/selected-equipment-placement")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(data().join(name)).unwrap()).unwrap()
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn digest() -> OwnedContentDigest {
    let payloads: Vec<Value> = [
        "authoring.json",
        "bindings.json",
        "dependencies.json",
        "migration.json",
        "source-vectors.json",
    ]
    .map(read)
    .into();
    digest_owned(
        "owned-selected-equipment-placement-v1",
        &payloads,
        4 * 1024 * 1024,
    )
    .unwrap()
}
pub fn check_authored() {
    let a: Value = read("authoring.json");
    let b: Value = read("bindings.json");
    let d: Value = read("dependencies.json");
    let v: Value = read("source-vectors.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    assert_eq!(a["before"], b["before"]);
    assert_eq!(b["before"], d["source"]["input"]);
    assert_eq!(json!(m.before), a["before"]);
    assert_eq!(m.schema_version, 5);
    assert_eq!(m.contract.schema_version, 6);
    assert_eq!(
        m.contract.operations_version.as_str(),
        "owned-domain-operations-v22"
    );
    assert_eq!(
        m.release.as_str(),
        "pob-3887ae68-selected-equipment-placement-v1"
    );
    assert!(
        m.owners.is_empty()
            && m.tables.is_empty()
            && m.receivers.is_empty()
            && m.query_targets.is_empty()
            && m.evaluation.is_none()
    );
    assert_eq!(m.schema.len(), 5);
    assert_eq!(b["items"].as_array().unwrap().len(), 5);
    assert_eq!(d["templates"].as_array().unwrap().len(), 5);
    assert_eq!(d["slots"].as_array().unwrap().len(), 7);
    assert_eq!(d["owners"].as_array().unwrap().len(), 5);
    assert_eq!(d["mappings"].as_array().unwrap().len(), 13);
    let catalogue_path = root().join(a["catalogue"]["path"].as_str().unwrap());
    let bytes = fs::read(catalogue_path).unwrap();
    assert_eq!(hash(&bytes), a["catalogue"]["sha256"]);
    let catalogue: Value = serde_json::from_slice(&bytes).unwrap();
    for (i, (template, slots, name, uses)) in [
        ("238c", &["0067"][..], "Tattered Robe", 4),
        ("2007", &["0068"][..], "Rope Cuffs", 4),
        ("09dc", &["006b", "006c", "006d"][..], "Sapphire Ring", 8),
        ("1e84", &["006e"][..], "Fine Belt", 4),
        ("1d75", &["0064"][..], "Ashen Staff", 1),
    ]
    .into_iter()
    .enumerate()
    {
        let binding = &b["items"][i];
        assert_eq!(
            binding["template"]["key"],
            format!("def.000000000000{template}")
        );
        let allowed = binding["equipment_slots"].as_array().unwrap();
        assert_eq!(allowed.len(), slots.len());
        for (id, slot) in allowed.iter().zip(slots) {
            assert_eq!(id["key"], format!("def.000000000000{slot}"));
            assert_eq!(
                d["slots"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|s| s["value"]["id"] == *id)
                    .count(),
                1
            );
        }
        assert_eq!(binding["base_name"], name);
        let old = &d["templates"][i];
        assert_eq!(old["value"]["id"], binding["template"]);
        let new = m
            .schema
            .iter()
            .find_map(|entry| match entry {
                SchemaExtensionEntry::Definition(definition)
                    if json!(definition)["value"]["id"] == binding["template"] =>
                {
                    Some(definition)
                }
                _ => None,
            })
            .expect("exact replacement definition address");
        let mut restored = json!(new);
        assert_eq!(
            restored["value"]["schema"]["value"]["equipment_slots"],
            json!({"members":allowed,"closure":{"kind":"complete"}})
        );
        let prior = &old["value"]["schema"]["value"]["equipment_slots"];
        assert_eq!(
            prior["members"],
            if template == "09dc" {
                json!(allowed)
            } else {
                json!([])
            }
        );
        assert_eq!(prior["closure"]["kind"], "partial");
        restored["value"]["schema"]["value"]["equipment_slots"] = prior.clone();
        assert_eq!(
            restored, *old,
            "every non-placement declaration stays exact"
        );
        assert_eq!(
            d["owners"][i]["owner"],
            json!({"kind":"definition","value":{"kind":"item_template","value":binding["template"]}})
        );
        assert_eq!(d["owners"][i]["programs"]["closure"]["kind"], "partial");
        assert_eq!(
            catalogue["bases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|x| x["name"] == binding["base_name"]
                    && x["item_type"] == binding["item_type"]
                    && x["source_module"] == binding["source_module"]
                    && x["weapon_field"] == "absent")
                .count(),
            1
        );
        let mappings = d["mappings"].as_array().unwrap();
        let base: Vec<_> = mappings
            .iter()
            .filter(|row| {
                row["source"]["kind"] == "definition"
                    && row["source"]["value"]["kind"] == "item_template"
                    && row["source"]["value"]["value"]["base"]
                        == json!({"kind":"text","value":name})
            })
            .collect();
        assert_eq!(base.len(), 1);
        assert_eq!(
            base[0]["outcome"]["value"]["target"],
            json!({"kind":"definition","value":{"kind":"item_template","value":binding["template"]}})
        );
        let mut destinations = BTreeSet::new();
        for label in binding["source_slots"].as_array().unwrap() {
            let rows: Vec<_> = mappings
                .iter()
                .filter(|row| {
                    row["source"]["kind"] == "catalog"
                        && row["source"]["value"]["kind"] == "equipment_slot"
                        && row["source"]["value"]["key"] == json!({"kind":"text","value":label})
                })
                .collect();
            assert_eq!(rows.len(), 1);
            let target = &rows[0]["outcome"]["value"]["target"];
            assert_eq!(target["kind"], "definition");
            assert_eq!(target["value"]["kind"], "equipment_slot");
            assert!(allowed.contains(&target["value"]["value"]));
            destinations.insert(target["value"]["value"].to_string());
        }
        assert_eq!(destinations, allowed.iter().map(Value::to_string).collect());
        assert_eq!(binding["original_uses"].as_array().unwrap().len(), uses);
        for u in binding["original_uses"].as_array().unwrap() {
            assert_eq!(
                u["item"],
                json!({"kind":"known","value":binding["original_item"]})
            );
            assert_eq!(u["destination"]["kind"], "character_slot");
            assert_eq!(u["destination"]["value"]["kind"], "known");
            assert!(allowed.contains(&u["destination"]["value"]["value"]));
            assert_eq!(u["scope"]["kind"], "known");
            assert_eq!(
                u["scope"]["value"]["kind"],
                if template == "1d75" {
                    "selected"
                } else {
                    "shared"
                }
            );
        }
    }
    assert_eq!(
        a["scope"],
        json!({"equipment_slot_inventories_closed":5,"new_definitions":0,"new_programs":0,"closed_rule_owners":0,"other_declaration_changes":0,"numerical_parity":false,"item_set_membership_changed":false,"socket_configuration_admission":false,"whole_build_parity":false})
    );
    for (n, expected) in a["artifact_sha256"].as_object().unwrap() {
        let bytes = fs::read(data().join(format!("{n}.json"))).unwrap();
        assert!(!bytes.contains(&b'\r'));
        assert_eq!(hash(&bytes), *expected);
    }
    check_vectors(&v);
    check_source(false);
}
pub fn check_vectors(v: &Value) {
    let b: Value = read("bindings.json");
    assert_eq!(v["status"], "passed");
    assert_eq!(v["projection"].as_array().unwrap().len(), 5);
    for (p, b) in v["projection"]
        .as_array()
        .unwrap()
        .iter()
        .zip(b["items"].as_array().unwrap())
    {
        assert_eq!(p["call_count"], 7119);
        assert_eq!(p["set_count"], 7);
        assert_eq!(p["flag_count"], 9);
        let s = &p["state"];
        assert_eq!(s["executed"], true);
        assert_eq!(s["item"]["id"], b["source_item_id"]);
        assert_eq!(s["item"]["base_name"], b["base_name"]);
        assert_eq!(s["item"]["item_type"], b["item_type"]);
        assert_eq!(s["item"]["base"]["type"], b["item_type"]);
        assert_eq!(
            s["method"],
            json!({"path":"Classes/ItemsTab.lua","first":2603,"last":2687})
        );
        assert_eq!(
            s["constructor"],
            json!({"path":"Classes/ItemsTab.lua","first":140,"last":1191})
        );
        assert_eq!(
            s["constructor_wrapper"],
            json!({"path":"Modules/Common.lua","first":167,"last":183})
        );
        assert_eq!(
            s["loader"],
            json!({"path":"Classes/ItemsTab.lua","first":1193,"last":1320})
        );
        let base = &s["item"]["base"];
        assert!(!matches!(
            base["subType"].as_str(),
            Some("Transcendent Arm" | "Transcendent Leg")
        ));
        for tag in ["onehand", "one_hand_weapon", "axe", "mace", "sword"] {
            assert!(base["tags"].get(tag).is_none());
        }
        if b["item_type"] == "Staff" {
            assert_eq!(base["tags"]["twohand"], true);
        } else {
            assert!(base["tags"].get("twohand").is_none());
        }
        let selected: Vec<_> = s["selected_uses"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|u| u["set"] == 2)
            .map(|u| u["slot"].clone())
            .collect();
        assert_eq!(json!(selected), b["selected_source_slots"]);
        let slots = s["registered_slots"].as_array().unwrap();
        assert_eq!(slots.len(), 113);
        assert!(slots.windows(2).all(|w| w[0].as_str() < w[1].as_str()));
        assert_eq!(s["base_slots"].as_array().unwrap().len(), 20);
        for slot in s["base_slots"].as_array().unwrap() {
            assert!(slots.contains(slot));
        }
        assert_eq!(s["set_ids"], json!([1, 2, 3, 4, 5, 6]));
        assert_eq!(
            s["selected"],
            json!({"items":2,"spec":3,"skills":4,"config":1,"group":3})
        );
        assert_eq!(s["flag_cases"][0], json!({"id":"actual"}));
        assert_eq!(s["flag_cases"].as_array().unwrap().len(), 9);
        for mask in 0..8 {
            assert_eq!(
                s["flag_cases"][mask + 1],
                json!({"id":format!("mask-{mask}"),"values":{"giantsBlood":mask&1!=0,"instrumentsOfPower":mask&2!=0,"lordOfTheWilds":mask&4!=0}})
            );
        }
        let outcomes = p["slot_outcomes"].as_array().unwrap();
        assert_eq!(outcomes.len(), 113);
        for (row, slot) in outcomes.iter().zip(slots) {
            assert_eq!(row["slot"], *slot);
            let mut count = 0;
            let mut unique = BTreeSet::new();
            for group in row["outcomes"].as_array().unwrap() {
                let o = &group["outcome"];
                assert!(unique.insert(o.to_string()));
                let n = group["count"].as_u64().unwrap();
                assert!(n > 0);
                count += n;
                match o["kind"].as_str().unwrap() {
                    "none" => assert_eq!(*o, json!({"kind":"none","return_count":0})),
                    "nil" => assert_eq!(*o, json!({"kind":"nil","return_count":1})),
                    "boolean" => {
                        assert_eq!(o["return_count"], 1);
                        assert!(o["value"].is_boolean());
                    }
                    _ => panic!("unrepresented original result"),
                }
                assert_eq!(
                    o["value"] == true,
                    b["source_slots"].as_array().unwrap().contains(slot)
                );
            }
            assert_eq!(count, 63);
        }
        assert_eq!(
            s["boundary"],
            json!({"slot":b["boundary_slot"],"registered":false,"outcome":{"return_count":1,"kind":"boolean","value":true},"native_admission":false})
        );
        for flag in [
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
            assert_eq!(s["evidence"][flag], true);
        }
        for flag in [
            "call_hook",
            "method_wrappers",
            "native_owner_coverage",
            "numerical_parity",
            "socket_configuration_admission",
        ] {
            assert_eq!(s["evidence"][flag], false);
        }
    }
}
fn without_calls(mut host: Value) -> Value {
    for s in host["state"].as_array_mut().unwrap() {
        for field in ["calls", "executed", "boundary"] {
            s.as_object_mut().unwrap().remove(field);
        }
    }
    host
}
fn project(report: &Value) -> Value {
    assert_eq!(report["original"], report["repeat"]);
    assert_eq!(
        without_calls(report["control"].clone()),
        without_calls(report["original"].clone())
    );
    let mut projected = Vec::new();
    for (original, control) in report["original"]["state"]
        .as_array()
        .unwrap()
        .iter()
        .zip(report["control"]["state"].as_array().unwrap())
    {
        assert_eq!(control["executed"], false);
        let mut state = original.clone();
        let mut keys = BTreeSet::new();
        let mut outcomes: BTreeMap<String, BTreeMap<String, (Value, usize)>> = BTreeMap::new();
        let calls = state["calls"].as_array().unwrap();
        assert_eq!(calls.len(), 7119);
        for r in calls {
            let set = r["set"].as_str().unwrap();
            let slot = r["slot"].as_str().unwrap();
            let flag = r["flags"].as_str().unwrap();
            assert!(set == "active-default" || (1..=6).any(|i| set == format!("saved-{i}")));
            assert!(
                state["registered_slots"]
                    .as_array()
                    .unwrap()
                    .contains(&r["slot"])
            );
            assert!(
                state["flag_cases"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|x| x["id"] == r["flags"])
            );
            assert!(keys.insert((set, slot, flag)));
            let o = &r["outcome"];
            outcomes
                .entry(slot.into())
                .or_default()
                .entry(o.to_string())
                .or_insert((o.clone(), 0))
                .1 += 1;
        }
        let slot_outcomes:Vec<_>=outcomes.into_iter().map(|(slot,groups)|json!({"slot":slot,"outcomes":groups.into_values().map(|(outcome,count)|json!({"outcome":outcome,"count":count})).collect::<Vec<_>>()})).collect();
        for key in ["calls", "read_set", "main_output"] {
            state.as_object_mut().unwrap().remove(key);
        }
        projected.push(json!({"state":state,"slot_outcomes":slot_outcomes,"call_count":7119,"set_count":7,"flag_count":9}));
    }
    json!(projected)
}
fn check_source(full: bool) {
    let a: Value = read("authoring.json");
    let v: Value = read("source-vectors.json");
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(hash(&manifest), a["source_manifest_sha256"]);
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    assert_eq!(manifest["upstream_revision"], a["source_revision"]);
    let pins = a["source_files"].as_array().unwrap();
    assert_eq!(pins.len(), 11);
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
    let meta = &v["report_metadata"];
    assert_eq!(meta["source_hash"], a["source_manifest_sha256"]);
    assert_eq!(meta["source_xml_sha256"], a["source_xml_sha256"]);
    assert_eq!(
        meta["source_items"],
        json!([{"id":19,"ordinal":572,"content_entry":0},{"id":20,"ordinal":574,"content_entry":0},{"id":26,"ordinal":587,"content_entry":0},{"id":27,"ordinal":590,"content_entry":0},{"id":28,"ordinal":594,"content_entry":0}])
    );
    let e = &meta["evidence"];
    assert_eq!(e["manifest_sha256"], a["source_manifest_sha256"]);
    assert_eq!(e["scope"], "registered-source-slot-catalogue-only");
    assert_eq!(e["fresh_runtimes"], 3);
    for flag in [
        "unhooked",
        "independent_replay_equal",
        "no_call_control_equal",
    ] {
        assert_eq!(e[flag], true);
    }
    for field in ["binding_sha256", "observer_sha256"] {
        assert_eq!(e[field], a[field]);
    }
    assert_eq!(e["files"].as_array().unwrap().len(), 11);
    for p in e["files"].as_array().unwrap() {
        assert_eq!(
            pins.iter()
                .filter(|x| x["path"] == p["path"] && x["sha256"] == p["sha256"])
                .count(),
            1
        );
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
            "crates/poe-optimizer-pob/tests/support/selected_equipment_placement_source.lua",
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
    for pin in reports {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
        assert_eq!(hash(&bytes), pin["sha256"]);
        let report: Value = serde_json::from_slice(&bytes).unwrap();
        let mut actual = report.clone();
        for key in ["original", "repeat", "control"] {
            actual.as_object_mut().unwrap().remove(key);
        }
        assert_eq!(actual, *meta);
        assert_eq!(project(&report), v["projection"]);
    }
}
fn unchanged(endpoint: &StagedOwnedRelease) {
    let d: Value = read("dependencies.json");
    for slot in d["slots"].as_array().unwrap() {
        let slot: DefinitionDescriptor = serde_json::from_value(slot.clone()).unwrap();
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
    }
    for owner in d["owners"].as_array().unwrap() {
        let owner: DefinitionRules = serde_json::from_value(owner.clone()).unwrap();
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
    }
    let mapping = json!(endpoint.input().mapping);
    for row in d["mappings"].as_array().unwrap() {
        assert_eq!(
            mapping["entries"]
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
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let p = endpoint.receipt().provenance.last().unwrap();
    assert_eq!(p.kind.as_str(), KIND);
    assert_eq!(json!(p.prior_input), a["before"]);
    assert_eq!(p.authoring_input, digest());
    assert_eq!(endpoint.input().recipe.schema.release, m.release);
    for entry in m.schema {
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
                .filter(|x| **x == expected)
                .count(),
            1
        );
    }
    unchanged(endpoint);
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    check_source(true);
    let d: Value = read("dependencies.json");
    let m: OwnedReleaseMigrationInput = read("migration.json");
    let receipt = json!(prior.receipt());
    for (key, v) in d["source"].as_object().unwrap() {
        assert_eq!(receipt[key], *v);
    }
    let old: Vec<DefinitionDescriptor> = serde_json::from_value(d["templates"].clone()).unwrap();
    for descriptor in &old {
        assert_eq!(
            prior
                .input()
                .recipe
                .schema
                .definitions
                .iter()
                .filter(|x| *x == descriptor)
                .count(),
            1
        );
    }
    unchanged(prior);
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
    for old in old {
        let target = restored
            .schema
            .definitions
            .iter_mut()
            .find(|x| x.address() == old.address())
            .unwrap();
        *target = old;
    }
    restored.schema.release = prior.input().recipe.schema.release.clone();
    restored.rules.definitions = prior.input().recipe.rules.definitions.clone();
    restored.routing.definitions = prior.input().recipe.routing.definitions.clone();
    assert_eq!(
        restored,
        prior.input().recipe,
        "whole recipe inverse admits only the five placement inventories"
    );
    migration_preservation::assert_import_rebindings_only(prior, &next);
    assert_eq!(next.query_sets(), prior.query_sets());
    assert_eq!(next.receipt().query_rows, 110);
    assert!(next.evaluation().is_none());
    next
}
