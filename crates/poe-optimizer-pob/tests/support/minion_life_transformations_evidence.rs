//! Keep the exact reference report once, without a second projection model.
//! This is offline evidence; no report value is an injected native rule input.
use super::*;

const PACKET: &str = "data/owned/poe2/3887ae68/minion-life-transformations";
const OBSERVER: &str =
    "crates/poe-optimizer-pob/tests/support/owned_minion_physical_damage_source.lua";
const DRIVER: &str = "crates/poe-optimizer-pob/tests/owned_minion_physical_damage_source.rs";
const HELPER: &str = "crates/poe-optimizer-pob/tests/support/minion_life_transformations.rs";
const BOOTSTRAP: &str =
    "crates/poe-optimizer-pob/tests/support/configuration_preparation_source.rs";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read_packet() -> (Json, Json) {
    let directory = root().join(PACKET);
    let metadata: Json =
        serde_json::from_slice(&fs::read(directory.join("source-vectors.json")).unwrap()).unwrap();
    let bytes = fs::read(directory.join("report.json")).unwrap();
    assert_eq!(metadata["report"]["path"], "report.json");
    assert_eq!(metadata["report"]["bytes"], bytes.len());
    assert_eq!(metadata["report"]["sha256"], digest(&bytes));
    let report = serde_json::from_slice(&bytes).unwrap();
    (metadata, report)
}

fn authenticate(metadata: &Json, report: &Json) {
    assert_eq!(metadata["schema_version"], 1);
    assert_eq!(
        metadata["kind"],
        "original-minion-life-transformation-census"
    );
    assert_eq!(metadata["native_coverage"], false);
    assert_eq!(metadata["copied_formula_as_evidence"], false);
    assert_eq!(report["business_method_wrappers"], false);
    assert_eq!(report["capture"]["copied_formula_as_evidence"], false);
    assert_eq!(report["capture"]["unhooked_controls"], true);
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
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
    let archive = &metadata["archived_observer"];
    assert_eq!(archive["source_path"], OBSERVER);
    assert_eq!(archive["path"], "evidence/observer.lua");
    assert_eq!(archive["executable"], false);
    let bytes = fs::read(root().join(PACKET).join("evidence/observer.lua")).unwrap();
    assert_eq!(archive["bytes"], bytes.len());
    assert_eq!(archive["sha256"], digest(&bytes));
    assert_eq!(archive["sha256"], report["observer_sha256"]);
    let witnesses = rows(&metadata["archived_drivers"]);
    assert_eq!(witnesses.len(), 3);
    for (witness, (source, file, field)) in witnesses.iter().zip([
        (DRIVER, "driver.rs", "driver_sha256"),
        (HELPER, "source-helper.rs", "source_helper_sha256"),
        (BOOTSTRAP, "bootstrap.rs", "bootstrap_sha256"),
    ]) {
        assert_eq!(witness["source_path"], source);
        assert_eq!(witness["path"], format!("evidence/{file}"));
        assert_eq!(witness["executable"], false);
        let bytes = fs::read(root().join(PACKET).join("evidence").join(file)).unwrap();
        assert_eq!(witness["bytes"], bytes.len());
        assert_eq!(witness["sha256"], digest(&bytes));
        assert_eq!(witness["sha256"], report[field]);
    }
    let reports = rows(&metadata["reports"]);
    assert_eq!(reports.len(), 2);
    for (pin, mode) in reports.iter().zip(["off", "on"]) {
        assert_eq!(pin["jit_mode"], mode);
        assert_eq!(pin["bytes"], metadata["report"]["bytes"]);
        assert_eq!(pin["sha256"], metadata["report"]["sha256"]);
        assert!(
            pin["path"]
                .as_str()
                .unwrap()
                .ends_with(&format!("/source-jit-{mode}.json"))
        );
    }
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
    check_life_delivery(report);
    life_adjustments::check(report);
    super::check(report);
}

#[test]
fn retained_transformations_authenticate_original_source_and_exact_report() {
    let (metadata, report) = read_packet();
    authenticate(&metadata, &report);
}

#[test]
fn substituted_source_identity_or_observer_is_rejected() {
    let (metadata, report) = read_packet();
    for field in [
        "source_revision",
        "manifest_sha256",
        "original_xml_sha256",
        "observer_sha256",
        "driver_sha256",
        "source_helper_sha256",
        "bootstrap_sha256",
    ] {
        let mut changed = report.clone();
        changed[field] = json!("substituted");
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
}

#[test]
fn warm_transition_must_identify_the_retained_changed_build() {
    let (metadata, report) = read_packet();
    for replacement in [
        Json::Null,
        report["original_xml_sha256"].clone(),
        json!("substituted"),
    ] {
        let mut changed = report.clone();
        changed["cases"][2]["warm_xml_sha256"] = replacement;
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
    let mut changed = report.clone();
    changed["cases"][0]["warm_xml_sha256"] = report["cases"][2]["warm_xml_sha256"].clone();
    assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
}

#[test]
fn acquisition_claims_cannot_contradict_preservation_evidence() {
    let (metadata, report) = read_packet();
    for (pointer, replacement) in [
        ("/business_method_wrappers", true),
        ("/capture/copied_formula_as_evidence", true),
        ("/capture/unhooked_controls", false),
    ] {
        let mut changed = report.clone();
        *changed.pointer_mut(pointer).unwrap() = json!(replacement);
        assert!(std::panic::catch_unwind(|| authenticate(&metadata, &changed)).is_err());
    }
}

fn reject(change: impl FnOnce(&mut Json)) {
    let (_, mut report) = read_packet();
    change(&mut report);
    assert!(std::panic::catch_unwind(|| super::check(&report)).is_err());
}

fn actor<'a>(report: &'a mut Json, mode: &str) -> &'a mut Json {
    let actors = report["cases"][0]["state"][mode]["actors"]
        .as_array_mut()
        .unwrap();
    let matches: Vec<_> = actors
        .iter()
        .enumerate()
        .filter(|(_, row)| row["actor_profile"] == "RaisedSkeletonSniper")
        .map(|(index, _)| index)
        .collect();
    assert_eq!(matches.len(), 1);
    &mut actors[matches[0]]
}

fn first_call(report: &mut Json) -> &mut Json {
    &mut actor(report, "main")["life_transformations"]["original_resource_calls"][0]
}

#[test]
fn missing_duplicate_and_reordered_incoming_domains_are_rejected() {
    for operation in 0..4 {
        reject(|report| {
            let rows = first_call(report)["inputs"]["incoming"]
                .as_array_mut()
                .unwrap();
            match operation {
                0 => {
                    rows.remove(0);
                }
                1 => {
                    rows.push(rows[0].clone());
                }
                2 => rows.swap(0, 1),
                _ => {
                    rows[0].as_object_mut().unwrap().remove("conversion");
                }
            }
        });
    }
}

#[test]
fn raw_zero_suppliers_cannot_disappear_behind_a_neutral_sum() {
    for location in ["raw", "eligible"] {
        for kind in ["conversion", "gain"] {
            reject(|report| {
                first_call(report)["inputs"]["incoming"][0][kind][location] =
                    json!([{"value":0,"source":"unaccounted"}]);
            });
        }
        reject(|report| {
            first_call(report)["inputs"]["life_total"][location] =
                json!([{"value":0,"source":"unaccounted"}]);
        });
    }
}

#[test]
fn nonzero_resource_inputs_and_changed_original_steps_are_rejected() {
    for field in ["gain_rate", "conversion_rate", "combined_rate"] {
        reject(|report| first_call(report)["incoming_steps"][0]["inputs"][field] = json!(1));
    }
    for phase in ["before", "after"] {
        for field in ["globalBase", "totalBase"] {
            reject(|report| {
                let life = first_call(report)[phase]["resources"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r["name"] == "Life")
                    .unwrap();
                life[field] = json!(1);
            });
        }
    }
    reject(|report| {
        first_call(report)["incoming_steps"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1)
    });
    reject(|report| {
        first_call(report)["incoming_steps"][0]["transfer_branch_entered"] = json!(true)
    });
}

#[test]
fn generated_zero_records_require_original_identity_and_consumer_correspondence() {
    reject(|report| {
        first_call(report)["insertions"]
            .as_array_mut()
            .unwrap()
            .remove(0);
    });
    reject(|report| {
        let records = first_call(report)["generated_records_at_return"]
            .as_array_mut()
            .unwrap();
        records.push(records[0].clone());
    });
    reject(|report| {
        first_call(report)["insertions"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1)
    });
    reject(|report| first_call(report)["insertions"][0]["caller_line"] = json!(1441));
    reject(|report| first_call(report)["insertions"][0]["source_value"] = json!(1));
    reject(|report| {
        first_call(report)["generated_records_at_return"][0]["exact_inserted_object"] = json!(false)
    });
    reject(|report| {
        first_call(report)["generated_records_at_return"][0]["record"]["source"] = json!("Strength")
    });
    reject(|report| {
        actor(report, "main")["life_adjustments"]["original_life_calls"][2]["computation"]["resource_generated_records"] =
            json!([]);
    });
}

#[test]
fn wrong_actor_identity_and_unexecuted_pass_substitution_are_rejected() {
    for (field, value) in [
        ("store_is_player", true),
        ("exact_actor_store", false),
        ("return_observed", false),
    ] {
        reject(|report| first_call(report)[field] = json!(value));
    }
    reject(|report| first_call(report)["source"] = json!({"kind":"player"}));
    reject(|report| {
        let executed = actor(report, "main")["life_transformations"].clone();
        actor(report, "calcs")["life_transformations"] = executed;
    });
}

#[test]
#[ignore = "requires the original local JIT-off/on reports retained in source-vectors.json"]
fn retained_complete_reports_are_exact_and_pass_the_current_checker() {
    let (metadata, report) = read_packet();
    authenticate(&metadata, &report);
    let expected = fs::read(root().join(PACKET).join("report.json")).unwrap();
    for pin in rows(&metadata["reports"]) {
        let bytes = fs::read(root().join(pin["path"].as_str().unwrap())).unwrap();
        assert_eq!(bytes.len(), expected.len());
        assert_eq!(digest(&bytes), digest(&expected));
        assert!(
            bytes == expected,
            "retain exact original bytes, not a rewritten projection"
        );
        super::check(&serde_json::from_slice(&bytes).unwrap());
    }
}
