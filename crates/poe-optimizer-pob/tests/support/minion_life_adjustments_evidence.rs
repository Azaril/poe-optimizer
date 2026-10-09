//! Retained source-only census. No observed value becomes native rule data.
use super::*;

const PACKET: &str = "data/owned/poe2/3887ae68/minion-life-adjustments";
const REPORT_DIR: &str = "runs/owned-minion-life-adjustment-source-02";
const OBSERVER: &str =
    "crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua";
const ARCHIVE: &str = "evidence/observer.lua";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn keep(value: &Json, fields: &[&str]) -> Json {
    let object = value.as_object().expect("source object");
    Json::Object(
        fields
            .iter()
            .map(|field| {
                (
                    (*field).to_owned(),
                    object.get(*field).expect("required source field").clone(),
                )
            })
            .collect(),
    )
}

fn snapshot(value: &Json) -> Json {
    let mut result = json!({});
    for mode in ["main", "calcs"] {
        result[mode] = json!({"actors":rows(&value[mode]["actors"]).iter().map(|original| {
            let mut actor = keep(original, &[
                "actor_profile", "ordinal", "selected", "source", "summon_effect",
                "life_adjustment_identity", "raw_inherent_attributes",
                "raw_inherent_life_flags", "raw_life_adjustments",
            ]);
            // An unexecuted CALCS actor has no output fields, not Life zero.
            actor["output"] = original["output"].as_object().map(|output| {
                Json::Object(output.iter().filter(|(name, _)| name.as_str() == "Life")
                    .map(|(name, value)| (name.clone(), value.clone())).collect())
            }).unwrap_or_else(|| original["output"].clone());
            actor
        }).collect::<Vec<_>>()});
    }
    result
}

fn project(report: &Json) -> Json {
    let mut projection = keep(
        report,
        &[
            "schema_version",
            "source_revision",
            "manifest_sha256",
            "observer_sha256",
            "original_xml_sha256",
            "files",
            "case_count",
            "complete_load_attempts_per_jit",
            "business_method_wrappers",
            "capture",
            "scope",
        ],
    );
    projection["cases"] = Json::Array(
        rows(&report["cases"])
            .iter()
            .map(|case| {
                let mut projected = keep(
                    case,
                    &[
                        "name",
                        "xml_sha256",
                        "warm_xml_sha256",
                        "authored_domain_boundary_only",
                    ],
                );
                let mut state = keep(
                    &case["state"],
                    &[
                        "life_adjustment_methods_preserved",
                        "original_functions_preserved",
                        "life_delivery_methods_preserved",
                        "intrinsic_life_methods_preserved",
                        "loaded_state_preserved",
                        "saved_specs_preserved",
                        "cached_outputs_preserved",
                        "query_state_preserved",
                        "fresh_actor_construction",
                        "source_actor_level_mutated",
                        "business_method_wrappers",
                        "selected",
                    ],
                );
                for mode in ["main", "calcs"] {
                    state[mode] = json!({"actors":rows(&case["state"][mode]["actors"])
                .iter().filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .map(|actor| {
                    let mut selected = keep(actor, &[
                        "actor_profile", "actor_level", "effective_level", "source_occurrence",
                        "summon_effect_id", "is_environment_minion", "life_adjustments",
                    ]);
                    // Keep every original call, its order and all raw records unchanged.
                    selected["gigantic_benefits"] = keep(&actor["gigantic_benefits"],
                        &["original_life_calls"]);
                    selected
                }).collect::<Vec<_>>()});
                }
                state["benefit_snapshot"] = snapshot(&case["state"]["benefit_snapshot"]);
                projected["state"] = state;
                projected["unhooked"] = snapshot(&case["unhooked"]);
                projected
            })
            .collect(),
    );
    projection
}

fn packet() -> Json {
    serde_json::from_slice(&fs::read(root().join(PACKET).join("source-vectors.json")).unwrap())
        .unwrap()
}

fn authenticate(packet: &Json) {
    assert_eq!(packet["schema_version"], 1);
    assert_eq!(packet["status"], "passed");
    assert_eq!(packet["kind"], "original-minion-life-adjustment-census");
    let report = &packet["projection"];
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(report["case_count"], 7);
    assert_eq!(report["complete_load_attempts_per_jit"], 16);
    assert_eq!(report["business_method_wrappers"], false);
    assert_eq!(report["capture"]["copied_formula_as_evidence"], false);
    assert_eq!(report["capture"]["unhooked_controls"], true);
    assert_eq!(
        rows(&report["cases"])
            .iter()
            .map(|case| case["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        [
            "original-05",
            "repeat-original-05",
            "warm-empty-to-original",
            "without-life-229",
            "without-life-1218",
            "without-six-life-nodes",
            "sniper-calcs-selected"
        ]
    );
    for case in &rows(&report["cases"])[..3] {
        assert_eq!(case["xml_sha256"], report["original_xml_sha256"]);
    }
    for (index, case) in rows(&report["cases"]).iter().enumerate() {
        if index == 2 {
            assert_eq!(case["warm_xml_sha256"], report["cases"][5]["xml_sha256"]);
            assert_ne!(case["warm_xml_sha256"], report["original_xml_sha256"]);
        } else {
            assert!(case["warm_xml_sha256"].is_null());
        }
    }
    for case in rows(&report["cases"]) {
        for field in [
            "original_functions_preserved",
            "loaded_state_preserved",
            "saved_specs_preserved",
            "cached_outputs_preserved",
            "query_state_preserved",
            "fresh_actor_construction",
            "intrinsic_life_methods_preserved",
            "life_delivery_methods_preserved",
        ] {
            assert_eq!(case["state"][field], true, "original preservation: {field}");
        }
        for field in ["source_actor_level_mutated", "business_method_wrappers"] {
            assert_eq!(
                case["state"][field], false,
                "forbidden acquisition mutation: {field}"
            );
        }
    }
    assert_eq!(
        packet["projection_sha256"],
        digest(&serde_json::to_vec(report).unwrap())
    );
    let original = root().join("tests/fixtures/builds/breadth-20260908/build-05.xml");
    assert_eq!(
        report["original_xml_sha256"],
        digest(&fs::read(original).unwrap())
    );
    let mut files: Vec<_> = FILES
        .iter()
        .copied()
        .chain(["src/Modules/CalcDefence.lua"])
        .collect();
    files.sort_unstable();
    files.dedup();
    assert_eq!(
        report["files"],
        json!(
            files
                .iter()
                .map(|path| json!({
                    "path":path,"sha256":pinned::expected_file_sha256(path).unwrap()
                }))
                .collect::<Vec<_>>()
        )
    );
    let archive = &packet["archived_observer"];
    assert_eq!(archive["source_path"], OBSERVER);
    assert_eq!(archive["path"], ARCHIVE);
    let bytes = fs::read(root().join(PACKET).join(ARCHIVE)).unwrap();
    assert_eq!(archive["bytes"], bytes.len());
    assert_eq!(archive["sha256"], digest(&bytes));
    assert_eq!(archive["sha256"], report["observer_sha256"]);
    assert_eq!(archive["executable"], false);
    let reports = rows(&packet["reports"]);
    assert_eq!(reports.len(), 2);
    for (pin, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(pin["path"], format!("{REPORT_DIR}/source-jit-{mode}.json"));
        assert!(pin["bytes"].as_u64().unwrap() > 0);
        assert_eq!(pin["bytes"], reports[0]["bytes"]);
        let hash = pin["sha256"].as_str().unwrap();
        assert_eq!(hash.len(), 64);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_eq!(pin["sha256"], reports[0]["sha256"]);
    }
    super::check(report);
}

#[test]
fn retained_census_authenticates_source_and_preserves_real_zero_records() {
    authenticate(&packet());
}

#[test]
fn substituted_provenance_and_projection_hashes_are_rejected() {
    for field in ["source_revision", "manifest_sha256", "observer_sha256"] {
        let mut value = packet();
        value["projection"][field] = json!("substituted");
        assert!(std::panic::catch_unwind(|| authenticate(&value)).is_err());
    }
    let mut value = packet();
    value["projection_sha256"] = json!("0".repeat(64));
    assert!(std::panic::catch_unwind(|| authenticate(&value)).is_err());
}

#[test]
fn changed_preservation_or_case_identity_rejects_a_recomputed_projection_hash() {
    for operation in 0..4 {
        let mut value = packet();
        match operation {
            0 => {
                value["projection"]["cases"][0]["state"]["original_functions_preserved"] =
                    json!(false)
            }
            1 => value["projection"]["cases"][0]["name"] = json!("substituted"),
            2 => value["projection"]["cases"]
                .as_array_mut()
                .unwrap()
                .swap(0, 1),
            _ => value["projection"]["cases"][0]["xml_sha256"] = json!("0".repeat(64)),
        }
        value["projection_sha256"] =
            json!(digest(&serde_json::to_vec(&value["projection"]).unwrap()));
        assert!(std::panic::catch_unwind(|| authenticate(&value)).is_err());
    }
}

fn reject(change: impl FnOnce(&mut Json)) {
    let mut report = packet()["projection"].clone();
    change(&mut report);
    assert!(
        std::panic::catch_unwind(|| super::check(&report)).is_err(),
        "malformed original-source census must be rejected"
    );
}

fn change_call(report: &mut Json, index: usize, change: impl FnOnce(&mut Json)) {
    let actor = &mut report["cases"][0]["state"]["main"]["actors"][0];
    change(&mut actor["life_adjustments"]["original_life_calls"][index]["computation"]);
    actor["gigantic_benefits"]["original_life_calls"] =
        actor["life_adjustments"]["original_life_calls"].clone();
}

fn raw_record(name: &str, kind: &str) -> Json {
    json!({"ancestor_depth":0,"mod":{"name":name,"type":kind,"source":"Malformed control",
        "value":0,"flags":0,"keyword_flags":0,"tags":{}}})
}

#[test]
fn missing_duplicate_and_reordered_zero_strength_records_are_rejected() {
    for operation in 0..3 {
        reject(|report| {
            change_call(report, 0, |c| {
                let raw = c["adjustments"]["raw_modifiers"].as_array_mut().unwrap();
                let index = raw
                    .iter()
                    .position(|r| r["mod"]["source"] == "Strength")
                    .unwrap();
                match operation {
                    0 => {
                        raw.remove(index);
                    }
                    1 => {
                        raw.push(raw[index].clone());
                    }
                    _ => {
                        raw.swap(0, index);
                    }
                }
            })
        });
    }
}

#[test]
fn extra_life_and_life_override_zero_records_are_rejected() {
    for name in ["ExtraLife", "Life"] {
        reject(|report| {
            change_call(report, 0, |c| {
                c["adjustments"]["raw_modifiers"]
                    .as_array_mut()
                    .unwrap()
                    .push(raw_record(name, "OVERRIDE"));
            })
        });
    }
}

#[test]
fn conversion_zero_records_must_match_the_original_call_stage_and_order() {
    for operation in 0..3 {
        reject(|report| {
            change_call(report, if operation == 1 { 0 } else { 2 }, |c| {
                let raw = c["adjustments"]["raw_modifiers"].as_array_mut().unwrap();
                match operation {
                    0 => {
                        let index = raw
                            .iter()
                            .position(|r| r["mod"]["name"] == "LifeTotal")
                            .unwrap();
                        raw.remove(index);
                    }
                    1 => {
                        let mut extra = raw_record("ExtraLife", "BASE");
                        extra["mod"]["source"] = json!("Conversion");
                        raw.insert(0, extra);
                    }
                    _ => {
                        let index = raw
                            .iter()
                            .position(|r| r["mod"]["name"] == "LifeTotal")
                            .unwrap();
                        raw.swap(0, index);
                    }
                }
            })
        });
    }
}

#[test]
fn missing_and_nonempty_raw_attribute_inventories_are_rejected() {
    reject(|report| {
        report["cases"][0]["state"]["main"]["actors"][0]["life_adjustments"]["strength_insertions"]
            [0]["inputs"]
            .as_object_mut()
            .unwrap()
            .remove("raw_attributes");
    });
    reject(|report| {
        change_call(report, 0, |c| {
            c["adjustments"]["inherent_inputs"]
                .as_object_mut()
                .unwrap()
                .remove("raw_attributes");
        })
    });
    for name in ["Str", "Attributes"] {
        reject(|report| {
            change_call(report, 0, |c| {
                c["adjustments"]["inherent_inputs"]["raw_attributes"] =
                    json!([raw_record(name, "BASE")]);
            })
        });
    }
}

#[test]
fn original_adjustment_values_cannot_disagree_with_supplemental_queries() {
    for field in ["extra", "total"] {
        reject(|report| change_call(report, 0, |c| c[field] = json!(1)));
    }
}

#[test]
fn wrong_strength_depth_and_position_are_rejected() {
    for field in ["ancestor_depth", "position"] {
        reject(|report| {
            change_call(report, 0, |c| {
                c["adjustments"]["strength_records"][0][field] = json!(999);
            })
        });
    }
    reject(|report| {
        change_call(report, 0, |c| {
            let raw = c["adjustments"]["raw_modifiers"].as_array_mut().unwrap();
            raw.iter_mut()
                .find(|r| r["mod"]["source"] == "Strength")
                .unwrap()["ancestor_depth"] = json!(1);
        })
    });
}

#[test]
fn missing_conversion_inventory_and_present_zero_override_are_rejected() {
    reject(|report| {
        change_call(report, 0, |c| {
            c["adjustments"]["conversion_before_cap"]
                .as_object_mut()
                .unwrap()
                .remove("records");
        })
    });
    reject(|report| {
        change_call(report, 0, |c| {
            c["adjustments"]["channels"]["LifeConvertToArmour"]
                .as_object_mut()
                .unwrap()
                .remove("records");
        })
    });
    reject(|report| {
        change_call(report, 0, |c| {
            c["override_present"] = json!(true);
            c["override"] = json!(0);
            c["adjustments"]["selected_override"] = json!({"present":true,"value":0});
        })
    });
}

#[test]
fn player_identity_cannot_replace_the_exact_minion_store() {
    reject(|report| {
        change_call(report, 0, |c| {
            c["adjustments"]["store_is_player"] = json!(true)
        })
    });
    reject(|report| {
        report["cases"][0]["state"]["main"]["actors"][0]["life_adjustments"]["strength_insertions"]
            [0]["store_is_player"] = json!(true);
    });
    reject(|report| {
        report["cases"][0]["unhooked"]["main"]["actors"][0]["life_adjustment_identity"]["store_is_player"] =
            json!(true);
        report["cases"][0]["state"]["benefit_snapshot"] = report["cases"][0]["unhooked"].clone();
    });
}

#[test]
fn unexecuted_calcs_is_not_an_observed_empty_domain() {
    reject(|report| {
        let call = report["cases"][0]["state"]["main"]["actors"][0]
            ["life_adjustments"]["original_life_calls"][0].clone();
        let actor = &mut report["cases"][0]["state"]["calcs"]["actors"][0];
        actor["life_adjustments"]["original_life_calls"] = json!([call]);
        actor["gigantic_benefits"]["original_life_calls"] =
            actor["life_adjustments"]["original_life_calls"].clone();
    });
    reject(|report| {
        report["cases"][0]["unhooked"]["calcs"]["actors"][0]["life_adjustment_identity"]["strength"] =
            json!({"present":true,"value":0});
        report["cases"][0]["state"]["benefit_snapshot"] = report["cases"][0]["unhooked"].clone();
    });
}

#[test]
#[ignore = "explicit source-packet authoring from both completed retained reports"]
fn author_source_projection_from_completed_reports() {
    let off = fs::read(root().join(REPORT_DIR).join("source-jit-off.json")).unwrap();
    let on = fs::read(root().join(REPORT_DIR).join("source-jit-on.json")).unwrap();
    assert_eq!(digest(&off), digest(&on));
    assert_eq!(off.len(), on.len());
    let mut projections = Vec::new();
    for bytes in [&off, &on] {
        let full: Json = serde_json::from_slice(bytes).unwrap();
        check_life_delivery(&full);
        super::check(&full);
        let projected = project(&full);
        super::check(&projected);
        projections.push(projected);
    }
    assert_eq!(projections[0], projections[1]);
    let observer = fs::read(root().join(OBSERVER)).unwrap();
    assert_eq!(projections[0]["observer_sha256"], digest(&observer));
    let reports: Vec<_> = ["off", "on"].iter().map(|mode| json!({
        "path":format!("{REPORT_DIR}/source-jit-{mode}.json"),"bytes":off.len(),"sha256":digest(&off)
    })).collect();
    let value = json!({
        "schema_version":1,"kind":"original-minion-life-adjustment-census","status":"passed",
        "projection_sha256":digest(&serde_json::to_vec(&projections[0]).unwrap()),
        "reports":reports,
        "archived_observer":{"source_path":OBSERVER,"path":ARCHIVE,"bytes":observer.len(),
            "sha256":digest(&observer),"executable":false},
        "projection":projections.remove(0),
    });
    let directory = root().join(PACKET);
    let target = directory.join("source-vectors.json");
    assert!(
        !target.exists(),
        "authoring never overwrites retained evidence"
    );
    fs::create_dir_all(directory.join("evidence")).unwrap();
    assert!(!directory.join(ARCHIVE).exists());
    fs::write(directory.join(ARCHIVE), observer).unwrap();
    authenticate(&value);
    fs::write(target, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
}

#[test]
#[ignore = "requires both complete retained source02 reports, no source VM"]
fn retained_full_reports_authenticate_the_exact_projection() {
    let packet = packet();
    authenticate(&packet);
    let mut previous = None;
    for pin in rows(&packet["reports"]) {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(pin["bytes"], bytes.len());
        assert_eq!(pin["sha256"], digest(&bytes));
        if let Some(previous) = &previous {
            assert_eq!(&bytes, previous);
        }
        let full: Json = serde_json::from_slice(&bytes).unwrap();
        check_life_delivery(&full);
        super::check(&full);
        let actual = project(&full);
        assert!(
            json_evidence::first_difference(&actual, &packet["projection"], "$").is_none(),
            "retained source projection changed"
        );
        previous = Some(bytes);
    }
}
