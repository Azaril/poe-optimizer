//! Retained copy-law certificate; source admission and contributors remain open.
#[allow(dead_code)]
#[path = "owned_amulet_level_copy.rs"]
mod copy;
#[allow(dead_code)]
#[path = "owned_amulet_bonus_snapshot.rs"]
mod snapshot;

use poe_optimizer_core::{
    owned_definitions::ItemTemplateDefId,
    owned_rules::{DefinitionRules, RuleProgram, StatReceiver},
    owned_schema::{DefinitionDescriptor, SchemaState, SchemaSubject, SlotDescriptor},
};
use poe_optimizer_import::owned_release::StagedOwnedRelease;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, fs, path::PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}
fn peer(family: &str, name: &str) -> Value {
    read(&format!("data/owned/poe2/3887ae68/{family}/{name}.json"))
}

pub fn check(v: &Value, full: bool) {
    assert_eq!(v["schema_version"], 1);
    assert_eq!(v["status"], "retained-source-law-and-transport-passed");
    assert_eq!(
        v["numeric_contract"],
        json!({
            "input":"finite-binary64-canonical-zero","resolved_stat_ranges":"no-interval",
            "raw_amount_and_initial_scale_range":[0,1000000],"percent_divisor":100,
            "scaled_copy_rounding":"floor-per-record","identity_branch":"factor-equals-one-preserves-effective",
            "copy_unscalable_branch":"record-tag-predicate-preserves-effective",
            "nonfinite_result":"native-numerical-error-not-source-infinity-parity"
        })
    );
    assert_eq!(
        v["admission"],
        json!({
            "current_import_copy_flag":false,"line_flag_is_record_copy_tag":false,
            "canonical_true_mapping_certified":false,"canonical_true_controls":"synthetic-consumer-input-only",
            "raw_import":"fixed-unsigned-integer-0-through-1000000-with-source-condition-guards"
        })
    );
    assert_eq!(
        v["scope"],
        json!({
            "local_consumer_algorithm_certified":true,"current_false_flag_source_transport_certified":true,
            "snapshot_reference_fragment_preserved":true,"whole_modifier_owner_complete":false,
            "all_snapshot_contributors_complete":false,"ordinary_routing_complete":false,
            "source_or_numeric_admission_complete":false,"ordered_transform_producers_complete":false,
            "new_source_execution":false,"new_evaluation_bundle":false
        })
    );
    let mut expected = BTreeSet::new();
    for (family, names) in [
        (
            "amulet-level-copy",
            vec![
                "authoring",
                "bindings",
                "dependencies",
                "migration",
                "source-vectors",
            ],
        ),
        (
            "amulet-bonus-snapshot",
            vec![
                "authoring",
                "bindings",
                "dependencies",
                "snapshot-authoring",
                "readiness",
                "source-vectors",
            ],
        ),
        (
            "global-minion-gem-level",
            vec!["item-rule", "source-condition", "numeric-binding"],
        ),
    ] {
        for name in names {
            expected.insert(format!("data/owned/poe2/3887ae68/{family}/{name}.json"));
        }
    }
    let mut seen = BTreeSet::new();
    for pin in v["artifacts"].as_array().unwrap() {
        let path = pin["path"].as_str().unwrap();
        assert!(expected.contains(path) && seen.insert(path.to_owned()));
        assert_eq!(hash(&fs::read(root().join(path)).unwrap()), pin["sha256"]);
    }
    assert_eq!(seen, expected);
    copy::check_authored();
    snapshot::check_authored(); // Includes exact frozen contribution/scalar reference stages.
    if full {
        copy::source_proof();
        snapshot::check_source_evidence(true);
    }
    check_sections(v, full);
    check_eligibility(v);
    check_domains(v);
    check_line_flag(v, full);
    let authored = peer("amulet-bonus-snapshot", "snapshot-authoring");
    assert_eq!(v["snapshot"]["owner"], authored["owners"][0]);
    assert_eq!(v["snapshot"]["receiver"], authored["receivers"][0]);
}

fn check_sections(v: &Value, full: bool) {
    let bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["source_manifest_sha256"], hash(&bytes));
    assert_eq!(v["source_revision"], manifest["upstream_revision"]);
    let sections = v["source_sections"].as_array().unwrap();
    assert_eq!(sections.len(), 2);
    for (section, (path, first, last)) in sections.iter().zip([
        ("src/Modules/CalcSetup.lua", 1661, 1669),
        ("src/Classes/ModStore.lua", 82, 119),
    ]) {
        assert_eq!(section["path"], path);
        assert_eq!(section["first_line"], first);
        assert_eq!(section["last_line"], last);
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|p| p["path"] == path && p["sha256"] == section["source_file_sha256"])
                .count(),
            1
        );
        let excerpt = section["text"].as_str().unwrap();
        assert_eq!(hash(excerpt.as_bytes()), section["sha256"]);
        assert!(!excerpt.contains('\r') && excerpt.ends_with('\n'));
        assert_eq!(excerpt.lines().count(), last - first + 1);
        if full {
            let source = fs::read_to_string(root().join("vendor/path-of-building-poe2").join(path))
                .unwrap()
                .replace("\r\n", "\n");
            assert_eq!(hash(source.as_bytes()), section["source_file_sha256"]);
            let actual = source
                .lines()
                .skip(first - 1)
                .take(last - first + 1)
                .collect::<Vec<_>>()
                .join("\n")
                + "\n";
            assert_eq!(actual, excerpt);
        }
    }
}

fn check_eligibility(v: &Value) {
    let bindings = peer("amulet-level-copy", "bindings");
    let migration = peer("amulet-level-copy", "migration");
    let vectors = peer("amulet-level-copy", "source-vectors");
    let placement = vectors["reports"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"] == "runs/amulet-placement-evidence-01.json")
        .unwrap();
    let rows = v["eligibility"].as_array().unwrap();
    assert_eq!(rows.len(), 6);
    for (i, row) in rows.iter().enumerate() {
        let binding = &bindings["templates"][i];
        assert_eq!(row["template"], binding["template"]);
        assert_eq!(row["eligible"], binding["eligible"]);
        assert_eq!(row["complete_placement"], binding["complete_placement"]);
        let facts: Vec<_> = placement["observations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|o| {
                o["pointer"].as_str().unwrap().starts_with("/templates/")
                    && o["value"]["template"] == row["template"]
            })
            .collect();
        assert_eq!(facts.len(), 1);
        let fact = &facts[0]["value"];
        assert_eq!(row["source_type"], fact["source_type"]);
        assert_eq!(row["eligible"], fact["copy_eligible"]);
        assert_eq!(row["eligible"], row["source_type"] == "Amulet");
        for name in ["equipment_slots", "socket_destinations"] {
            assert_eq!(
                &row[name],
                if row["complete_placement"] == true {
                    &fact[if name == "equipment_slots" {
                        "reviewed_static_equipment_slots"
                    } else {
                        "reviewed_socket_destinations"
                    }]
                } else {
                    &fact["current_placement"][name]
                }
            );
        }
        let owner = migration["owners"]
            .as_array()
            .unwrap()
            .iter()
            .find(|owner| owner["owner"]["value"]["value"] == row["template"])
            .unwrap();
        assert_eq!(owner["programs"]["members"].as_array().unwrap().len(), 1);
        assert_eq!(row["program"], owner["programs"]["members"][0]);
        assert_eq!(row["program"]["id"], "amulet-copy-eligibility");
        assert_eq!(row["program"]["reads"], json!([]));
        assert_eq!(
            row["program"]["nodes"][0]["expression"],
            json!({"kind":"literal","value":{"kind":"boolean","value":row["eligible"]}})
        );
    }
}

fn check_domains(v: &Value) {
    let stats = v["domain"]["stats"].as_array().unwrap();
    let slots = v["domain"]["slots"].as_array().unwrap();
    assert_eq!((stats.len(), slots.len()), (3, 3));
    for (stat, key, unit, target) in [
        (&stats[0], "295b", Some("295a"), "modifier"),
        (&stats[1], "32e3", None, "equipment_use"),
        (&stats[2], "32e4", Some("0002"), "actor"),
    ] {
        assert_eq!(stat["value"]["id"]["key"], format!("def.000000000000{key}"));
        let schema = &stat["value"]["schema"];
        assert_eq!(schema["kind"], "known");
        assert_eq!(schema["value"]["targets"], json!([target]));
        let ty = &schema["value"]["value"];
        if let Some(unit) = unit {
            assert_eq!(ty["kind"], "quantity");
            assert_eq!(
                ty["value"].as_object().unwrap().len(),
                1,
                "computed quantity has no interval"
            );
            assert_eq!(
                ty["value"]["unit"]["key"],
                format!("def.000000000000{unit}")
            );
        } else {
            assert_eq!(*ty, json!({"kind":"boolean"}));
        }
    }
    for (slot, suffix) in slots.iter().zip(["30cb", "30e0", "30e1"]) {
        assert_eq!(
            slot["value"]["id"]["slot"]["key"],
            format!("def.000000000000{suffix}")
        );
        assert_eq!(
            slot["value"]["id"]["declaration"]["definition"]["key"],
            "def.00000000000030ca"
        );
        let schema = &slot["value"]["schema"]["value"];
        assert_eq!(schema["presence"], "required_once");
        assert_eq!(schema["sites"], json!(["modifier_roll"]));
        if suffix == "30e0" {
            assert_eq!(schema["value"], json!({"kind":"boolean"}));
        } else {
            assert_eq!(schema["value"]["kind"], "quantity");
            assert_eq!(schema["value"]["value"]["minimum"]["value"], 0.0);
            assert_eq!(schema["value"]["value"]["maximum"]["value"], 1_000_000.0);
        }
    }
    let rule = peer("global-minion-gem-level", "item-rule");
    let emitted: Vec<_> = rule["emissions"][0]["value"]["rolls"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["slot"] == slots[1]["value"]["id"])
        .collect();
    assert_eq!(emitted.len(), 1);
    assert_eq!(
        emitted[0]["value"],
        json!({"kind":"literal","value":{"kind":"boolean","value":false}})
    );
    let condition = peer("global-minion-gem-level", "source-condition");
    assert_eq!(
        condition,
        json!({"rule":"fixed-global-minion-level","all":[
            {"kind":"no_source_scaling_tags"},{"kind":"initial_scaling_is_one"},
            {"kind":"no_generated_buff_members"},{"kind":"unsigned_integer_capture","value":{"capture":"amount","min":0,"max":1000000}}
        ]})
    );
}

fn check_line_flag(v: &Value, full: bool) {
    let control = &v["line_unscalable_control"];
    assert_eq!(control["pointer"], "/builds/1/state/probes/15");
    let probe = &control["value"];
    assert_eq!(probe["name"], "unscalable");
    assert_eq!(
        probe["input_line"],
        "{unscalable}{tags:minion}+5 to Level of all Minion Skills"
    );
    let state = &probe["after_build"];
    assert_eq!(state["lines"].as_array().unwrap().len(), 1);
    assert_eq!(state["lines"][0]["unscalable"], true);
    for records in [&state["lines"][0]["records"], &state["active"]] {
        assert_eq!(records.as_array().unwrap().len(), 1);
        assert_eq!(records[0]["name"], "GemProperty");
        assert_eq!(
            records[0]["tags"],
            json!({}),
            "line metadata does not become a record copy tag"
        );
        assert_eq!(records[0]["value"]["value"], 5);
    }
    let existing = peer("amulet-level-copy", "source-vectors");
    assert_eq!(control["reports"].as_array().unwrap().len(), 2);
    for (i, pin) in control["reports"].as_array().unwrap().iter().enumerate() {
        for field in ["path", "bytes", "sha256"] {
            assert_eq!(pin[field], existing["reports"][i][field]);
        }
        if full {
            let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
            assert_eq!(hash(&bytes), pin["sha256"]);
            assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
            let report: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(
                report
                    .pointer(control["pointer"].as_str().unwrap())
                    .unwrap(),
                probe
            );
        }
    }
}

/// Current endpoints may have later unrelated template programs/declarations.
/// Compare only the exact previously proved program members and placement facets.
pub fn assert_dependencies(endpoint: &StagedOwnedRelease) {
    let v = peer("amulet-copy-consumer", "source-vectors");
    let recipe = &endpoint.input().recipe;
    for row in v["eligibility"].as_array().unwrap() {
        let id: ItemTemplateDefId = serde_json::from_value(row["template"].clone()).unwrap();
        let definition = recipe
            .schema
            .definitions
            .iter()
            .find(|d| matches!(d, DefinitionDescriptor::ItemTemplate(t) if t.id == id))
            .unwrap();
        let DefinitionDescriptor::ItemTemplate(template) = definition else {
            unreachable!()
        };
        let SchemaState::Known(schema) = &template.schema else {
            panic!("known template")
        };
        assert_eq!(json!(schema.equipment_slots), row["equipment_slots"]);
        assert_eq!(
            json!(schema.socket_destinations),
            row["socket_destinations"]
        );
        let program: RuleProgram = serde_json::from_value(row["program"].clone()).unwrap();
        let owner = recipe
            .rules
            .owners
            .iter()
            .find(|o| o.owner == SchemaSubject::Definition(definition.address()))
            .unwrap();
        assert!(
            !owner.programs.is_complete(),
            "eligibility alone never closes this template"
        );
        assert_eq!(
            owner
                .programs
                .members
                .iter()
                .filter(|p| **p == program)
                .count(),
            1
        );
    }
    for row in
        serde_json::from_value::<Vec<DefinitionDescriptor>>(v["domain"]["stats"].clone()).unwrap()
    {
        assert_eq!(
            recipe
                .schema
                .definitions
                .iter()
                .filter(|d| **d == row)
                .count(),
            1
        );
    }
    for row in serde_json::from_value::<Vec<SlotDescriptor>>(v["domain"]["slots"].clone()).unwrap()
    {
        assert_eq!(recipe.schema.slots.iter().filter(|s| **s == row).count(), 1);
    }
    let expected: DefinitionRules = serde_json::from_value(v["snapshot"]["owner"].clone()).unwrap();
    let current = recipe
        .rules
        .owners
        .iter()
        .find(|o| o.owner == expected.owner)
        .unwrap();
    assert_eq!(current, &expected, "exact complete snapshot owner");
    let receiver: StatReceiver = serde_json::from_value(v["snapshot"]["receiver"].clone()).unwrap();
    assert_eq!(
        recipe
            .rules
            .receivers
            .members
            .iter()
            .filter(|r| **r == receiver)
            .count(),
        1
    );
}
