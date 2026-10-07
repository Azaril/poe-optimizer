//! Offline authentication of existing Crown witnesses, never a native data loader.
use super::{hash, root};
use serde_json::{Map, Value, json};
use std::{collections::BTreeSet, fs};

const CATALOGUE_STAGES: [&str; 3] = ["fresh", "rebuild-one", "rebuild-two"];
const RUNTIME_STAGES: [&str; 3] = ["fresh", "rebuilt_once", "rebuilt_twice"];
const CONSTRUCTION_PHASES: [&str; 5] = [
    "loaded",
    "fresh_before_build",
    "fresh",
    "reparsed",
    "fresh_after_reparse",
];

fn select(value: &Value, fields: &[&str]) -> Value {
    Value::Object(
        fields
            .iter()
            .map(|&name| (name.to_owned(), value.get(name).unwrap().clone()))
            .collect(),
    )
}
fn only_crown(items: &Value) -> &Value {
    let rows: Vec<_> = items
        .as_array()
        .unwrap()
        .iter()
        .filter(|item| item["id"] == 21)
        .collect();
    assert_eq!(rows.len(), 1);
    rows[0]
}
fn catalogue(state: &Value) -> Value {
    let mut result = select(
        state,
        &[
            "selected",
            "original_functions_preserved",
            "method_wrappers",
        ],
    );
    result["item"] = select(
        only_crown(&state["items"]),
        &[
            "id",
            "base_name",
            "source",
            "exact_catalogue_base",
            "exact_registered",
            "base",
        ],
    );
    result
}
fn construction(state: &Value) -> Value {
    let mut result = select(
        state,
        &[
            "original_functions_preserved",
            "main_output_preserved",
            "saved_items_preserved",
            "saved_selections_preserved",
            "selected_config",
            "selected_items",
            "selected_skills",
            "selected_spec",
        ],
    );
    let source = only_crown(&state["items"]);
    let mut item = select(
        source,
        &["id", "raw", "xml", "base_facts", "selected_slots"],
    );
    for phase in CONSTRUCTION_PHASES {
        item[phase] = select(
            &source[phase],
            &[
                "base",
                "baseName",
                "type",
                "sockets",
                "base_mods",
                "active",
                "lists",
            ],
        );
    }
    result["item"] = item;
    result
}
fn runtime(consumer: &Value) -> Value {
    let mut result = select(
        consumer,
        &[
            "original_functions_preserved",
            "hook_removed",
            "native_field_disposition",
            "whole_supplier_domain_complete",
        ],
    );
    result["environments"] = Value::Array(consumer["environments"].as_array().unwrap().iter().enumerate().map(|(i, env)| {
        json!({
            "environment_index":i,"mode":env["mode"],"axes":env["axes"],
            "item":select(only_crown(&env["items"]), &[
                "id", "slot", "source", "type", "saved_object_exact", "selected_item_id", "grants",
            ])
        })
    }).collect());
    result
}

pub fn check(proof: &Value, full: bool) {
    assert_eq!(proof["schema_version"], 1);
    assert_eq!(proof["status"], "retained-source-witnesses-passed");
    assert!(
        proof["authority"]
            .as_object()
            .unwrap()
            .values()
            .all(|value| *value == false)
    );
    assert_eq!(
        proof["source_binding"],
        json!({"original":5,"item_id":21,"source_ordinal":576,"equipment_slot":"Helmet"})
    );
    let manifest_bytes =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    assert_eq!(hash(&manifest_bytes), proof["source_manifest_sha256"]);
    assert_eq!(manifest["upstream_revision"], proof["source_revision"]);
    let mut paths = BTreeSet::new();
    for pin in proof["source_files"].as_array().unwrap() {
        assert!(paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| *row == pin)
                .count(),
            1
        );
        if full {
            let bytes = fs::read_to_string(
                root()
                    .join("vendor/path-of-building-poe2")
                    .join(pin["path"].as_str().unwrap()),
            )
            .unwrap()
            .replace("\r\n", "\n");
            assert_eq!(bytes.len() as u64, pin["bytes"].as_u64().unwrap());
            assert_eq!(hash(bytes.as_bytes()), pin["sha256"]);
        }
    }
    for required in [
        "src/Classes/Item.lua",
        "src/Data/Bases/helmet.lua",
        "src/Modules/CalcSetup.lua",
    ] {
        assert!(paths.contains(required));
    }
    let mut local_paths = BTreeSet::new();
    for pin in proof["local_pins"].as_array().unwrap() {
        assert!(local_paths.insert(pin["path"].as_str().unwrap()));
        assert_eq!(
            hash(&fs::read(root().join(pin["path"].as_str().unwrap())).unwrap()),
            pin["sha256"]
        );
    }
    assert_eq!(local_paths.len(), 5);
    assert_eq!(
        proof["original_path"],
        "tests/fixtures/builds/breadth-20260908/build-05.xml"
    );
    assert!(local_paths.contains(proof["original_path"].as_str().unwrap()));
    assert_eq!(
        hash(&fs::read(root().join(proof["original_path"].as_str().unwrap())).unwrap()),
        proof["original_sha256"]
    );
    assert_eq!(
        proof["base"],
        json!({
            "armour":{"Armour":26,"EnergyShield":13},"implicitModTypes":{},"quality":20,
            "req":{"int":7,"level":5,"str":7},"socketLimit":3,"subType":"Armour/Energy Shield",
            "tags":{"armour":true,"default":true,"ezomyte_basetype":true,"helmet":true,"str_int_armour":true},
            "type":"Helmet"
        }),
        "whole base record, including the nonempty socket capacity"
    );
    let witnesses = proof["witnesses"].as_array().unwrap();
    assert_eq!(witnesses.len(), 3);
    for (i, kind) in ["catalogue", "construction", "runtime_grants"]
        .into_iter()
        .enumerate()
    {
        assert_eq!(witnesses[i]["kind"], kind);
        check_witness(proof, &witnesses[i], kind, full);
    }
    let source = fs::read_to_string(
        root().join("crates/poe-optimizer-pob/tests/support/armour_item_input_source.rs"),
    )
    .unwrap();
    let observer = source
        .split_once("pub const LOCAL_DEFENCE_OBSERVER: &str = r##\"")
        .unwrap()
        .1
        .split_once("\"##;")
        .unwrap()
        .0;
    assert_eq!(
        hash(observer.as_bytes()),
        witnesses[0]["report_metadata"]["observer_sha256"]
    );
    let observer = fs::read(
        root().join("crates/poe-optimizer-pob/tests/support/generated_extra_stat_consumption.lua"),
    )
    .unwrap();
    assert_eq!(
        hash(&observer),
        witnesses[2]["report_metadata"]["observer_sha256"]
    );
}

fn check_witness(proof: &Value, witness: &Value, kind: &str, full: bool) {
    let metadata = &witness["report_metadata"];
    let evidence = if kind == "construction" {
        &metadata["evidence"]
    } else {
        metadata
    };
    assert_eq!(evidence["manifest_sha256"], proof["source_manifest_sha256"]);
    if kind != "construction" {
        assert_eq!(metadata["source_revision"], proof["source_revision"]);
    } else {
        assert_eq!(metadata["source_hash"], proof["source_manifest_sha256"]);
        assert_eq!(evidence["native_input_closure"], false);
        assert_eq!(evidence["native_owner_coverage"], false);
    }
    for pin in evidence["files"].as_array().unwrap() {
        assert_eq!(
            proof["source_files"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|row| row["path"] == pin["path"] && row["sha256"] == pin["sha256"])
                .count(),
            1
        );
    }
    if kind == "runtime_grants" {
        for flag in [
            "native_inventory_authority",
            "native_build_parity",
            "field_non_applicability_certificate",
            "all_suppliers_or_transforms_proved",
            "business_wrappers",
        ] {
            assert_eq!(metadata[flag], false);
        }
        assert_eq!(metadata["original_sha256"], proof["original_sha256"]);
        assert_eq!(metadata["original_path"], proof["original_path"]);
    }
    if kind == "catalogue" {
        assert_eq!(metadata["native_owner_closure"], false);
    }
    let observations = witness["observations"].as_array().unwrap();
    assert_eq!(
        observations.len(),
        match kind {
            "catalogue" => 9,
            "construction" => 1,
            "runtime_grants" => 6,
            _ => unreachable!(),
        }
    );
    for (index, obs) in observations.iter().enumerate() {
        let value = &obs["value"];
        assert_eq!(value["original_functions_preserved"], true);
        match kind {
            "catalogue" => {
                let case = index / 3;
                let stage = CATALOGUE_STAGES[index % 3];
                assert_eq!(obs["case_index"], case);
                assert_eq!(
                    obs["name"],
                    [
                        "original",
                        "independent-repeat",
                        "warm-quality-thirty-then-original"
                    ][case]
                );
                assert_eq!(obs["stage"], stage);
                assert_eq!(
                    obs["pointer"],
                    format!("/cases/{case}/observed/states/{stage}")
                );
                assert_eq!(
                    value["selected"],
                    json!({"config":1,"items":2,"skills":4,"spec":3})
                );
                assert_eq!(value["method_wrappers"], false);
                assert_eq!(
                    value["item"],
                    json!({"id":21,"base_name":"Iron Crown","source":"Item:21:New Item, Iron Crown","exact_catalogue_base":true,"exact_registered":true,"base":proof["base"]})
                );
            }
            "construction" => check_construction(proof, obs),
            "runtime_grants" => {
                let case = index / 3;
                let stage = RUNTIME_STAGES[index % 3];
                assert_eq!(obs["case_index"], case);
                assert_eq!(obs["name"], ["original-05", "repeat-original-05"][case]);
                assert_eq!(obs["stage"], stage);
                assert_eq!(
                    obs["pointer"],
                    format!("/cases/{case}/states/{stage}/consumer")
                );
                assert_eq!(value["hook_removed"], true);
                assert_eq!(value["native_field_disposition"], false);
                assert_eq!(value["whole_supplier_domain_complete"], false);
                let mut modes = BTreeSet::new();
                for (j, env) in value["environments"].as_array().unwrap().iter().enumerate() {
                    assert_eq!(env["environment_index"], j);
                    let mode = env["mode"].as_str().unwrap();
                    assert!(["MAIN", "CALCS", "CALCULATOR"].contains(&mode));
                    modes.insert(mode);
                    assert_eq!(
                        env["axes"],
                        json!({"config":1,"items":2,"passives":3,"skills":4})
                    );
                    assert_eq!(
                        env["item"],
                        json!({
                            "id":21,"slot":"Helmet","source":"Item:21:New Item, Iron Crown","type":"Helmet",
                            "saved_object_exact":true,"selected_item_id":21,
                            "grants":{"kind":"raw_table","has_metatable":false,"fields":{}}
                        })
                    );
                }
                assert_eq!(modes, BTreeSet::from(["MAIN", "CALCS", "CALCULATOR"]));
            }
            _ => unreachable!(),
        }
    }
    // The observations use identical unchanged-item values across lifecycle and replay.
    if kind == "catalogue" || kind == "runtime_grants" {
        for obs in observations {
            assert_eq!(obs["value"], observations[0]["value"]);
        }
    }
    let reports = witness["reports"].as_array().unwrap();
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["bytes"], reports[1]["bytes"]);
    assert_eq!(reports[0]["sha256"], reports[1]["sha256"]);
    let stem = match kind {
        "catalogue" => "owned-armour-local-defence-source-03",
        "construction" => "owned-armour-item-inputs-source-01",
        "runtime_grants" => "owned-extra-stat-consumption-source-05",
        _ => unreachable!(),
    };
    for (i, report) in reports.iter().enumerate() {
        assert_eq!(
            report["path"],
            format!(
                "runs/{stem}/source-jit-{}.json",
                if i == 0 { "off" } else { "on" }
            )
        );
        if full {
            let bytes = fs::read(root().join(report["path"].as_str().unwrap())).unwrap();
            assert_eq!(bytes.len() as u64, report["bytes"].as_u64().unwrap());
            assert_eq!(hash(&bytes), report["sha256"]);
            // Equal independently authenticated digests cover both modes; parse once.
            if i == 0 {
                check_report(
                    proof,
                    witness,
                    kind,
                    &serde_json::from_slice(&bytes).unwrap(),
                );
            }
        }
    }
}

fn check_construction(proof: &Value, obs: &Value) {
    assert_eq!(obs["case_index"], 0);
    assert_eq!(obs["name"], "original");
    assert_eq!(obs["pointer"], "/cases/0/state");
    assert_eq!(obs["xml_sha256"], proof["original_sha256"]);
    let state = &obs["value"];
    for flag in [
        "main_output_preserved",
        "saved_items_preserved",
        "saved_selections_preserved",
    ] {
        assert_eq!(state[flag], true);
    }
    for (field, n) in [
        ("selected_config", 1),
        ("selected_items", 2),
        ("selected_skills", 4),
        ("selected_spec", 3),
    ] {
        assert_eq!(state[field], n);
    }
    let item = &state["item"];
    assert_eq!(item["id"], 21);
    assert_eq!(item["xml"]["attributes"]["id"], "21");
    assert_eq!(item["selected_slots"], json!(["Helmet"]));
    assert_eq!(
        item["base_facts"],
        json!({
            "armour":proof["base"]["armour"],"implicit_mod_types":{},"name":"Iron Crown",
            "quality":20,"requirements":proof["base"]["req"],"socket_limit":3,
            "subtype":"Armour/Energy Shield","type":"Helmet"
        })
    );
    for phase in CONSTRUCTION_PHASES {
        let source = &item[phase];
        assert_eq!(source["base"], "Iron Crown");
        assert_eq!(source["baseName"], "Iron Crown");
        assert_eq!(source["type"], "Helmet");
        assert_eq!(
            source["sockets"],
            json!([{"group":1},{"group":2},{"group":3}])
        );
        for category in ["buff", "classRequirement", "enchant", "implicit", "rune"] {
            assert_eq!(source["lists"][category], json!({}));
        }
        let explicit = source["lists"]["explicit"].as_array().unwrap();
        assert_eq!(explicit.len(), 1);
        assert_eq!(explicit[0]["line"], "+1 to Level of all Minion Skills");
        assert_eq!(explicit[0]["field_types"]["extra"], "nil");
        assert_eq!(explicit[0]["records"], source["base_mods"]);
        let records = source["base_mods"].as_array().unwrap();
        assert_eq!(records.len(), 1);
        assert_eq!(records[0]["name"], "GemProperty");
        assert_eq!(records[0]["type"], "LIST");
        assert_eq!(
            records[0]["value"],
            json!({"key":"level","keyOfScaledMod":"value","keyword":"minion","value":1})
        );
        if phase == "fresh_before_build" {
            assert_eq!(source["active"], json!({}));
        } else {
            let active = source["active"].as_array().unwrap();
            assert_eq!(active.len(), 2);
            assert_eq!(active[0]["name"], "GemProperty");
            assert_eq!(active[1]["name"], "Multiplier:QualityOnHelmet");
            assert_eq!(*source, item["fresh"]);
        }
    }
}

fn check_report(proof: &Value, witness: &Value, kind: &str, report: &Value) {
    let metadata: Map<_, _> = report
        .as_object()
        .unwrap()
        .iter()
        .filter(|(key, _)| key.as_str() != "cases")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    assert_eq!(Value::Object(metadata), witness["report_metadata"]);
    assert_eq!(
        report["cases"].as_array().unwrap().len(),
        if kind == "catalogue" { 6 } else { 3 }
    );
    for obs in witness["observations"].as_array().unwrap() {
        let state = report.pointer(obs["pointer"].as_str().unwrap()).unwrap();
        let project = match kind {
            "catalogue" => catalogue,
            "construction" => construction,
            "runtime_grants" => runtime,
            _ => unreachable!(),
        };
        assert_eq!(project(state), obs["value"]);
        let case = &report["cases"][obs["case_index"].as_u64().unwrap() as usize];
        assert_eq!(case["name"], obs["name"]);
        if kind == "catalogue" {
            assert_eq!(case["game_obtainability_authority"], false);
            assert_eq!(case["native_source_admission_authority"], false);
            assert_eq!(case["synthetic_local_control"], false);
            for branch in ["observed", "unhooked"] {
                assert_eq!(case[branch]["xml_sha256"], proof["original_sha256"]);
                assert_eq!(case[branch]["source_hash"], proof["source_manifest_sha256"]);
                assert_eq!(
                    case[branch]["source_identity"]["source_sha256"],
                    proof["original_sha256"]
                );
                assert_eq!(case[branch]["independent_source_bindings_verified"], true);
                let bindings: Vec<_> = case[branch]["source_bindings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter(|row| row["item_id"] == 21)
                    .collect();
                assert_eq!(bindings.len(), 1);
                assert_eq!(bindings[0]["ordinal"], 576);
                assert_eq!(
                    catalogue(&case[branch]["states"][obs["stage"].as_str().unwrap()]),
                    obs["value"]
                );
            }
        } else {
            assert_eq!(case["xml_sha256"], proof["original_sha256"]);
        }
    }
    if kind == "runtime_grants" {
        for (i, case) in report["cases"].as_array().unwrap().iter().enumerate() {
            assert_eq!(case["instrumented"], i != 2);
            assert_eq!(
                case["selected"],
                json!({"config":1,"items":2,"passives":3,"skills":4})
            );
            for stage in RUNTIME_STAGES {
                assert_eq!(
                    case["states"][stage]["outputs"],
                    report["cases"][0]["states"][stage]["outputs"],
                    "independent repeat and uninstrumented outputs preserved"
                );
            }
        }
    }
}
