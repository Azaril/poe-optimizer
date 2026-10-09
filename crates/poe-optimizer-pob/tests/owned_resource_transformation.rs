//! Full original resource transformation controls; never a native implementation.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/player_resource_source.rs"]
mod resource;
use poe_optimizer_pob::source as pinned;
use resource::{hash, observe};
use serde_json::{Value, json};
use std::{fs, path::Path};

const OBSERVER: &str = include_str!("support/resource_transformation_source.lua");

fn controlled(xml: &str, text: &str) -> String {
    assert!(!text.contains(['<', '>', '&', '"']));
    let doc = roxmltree::Document::parse(xml).unwrap();
    let config = doc
        .root_element()
        .children()
        .find(|n| n.has_tag_name("Config"))
        .unwrap();
    let active = config.attribute("activeConfigSet").unwrap();
    let set = config
        .children()
        .find(|n| n.has_tag_name("ConfigSet") && n.attribute("id") == Some(active))
        .unwrap();
    assert!(
        !set.children()
            .any(|n| n.has_tag_name("CustomModifierBlock"))
    );
    let at = set.range().end - "</ConfigSet>".len();
    assert_eq!(&xml[at..set.range().end], "</ConfigSet>");
    let addition = format!(
        "<CustomModifierBlock title=\"Resource Transformation Control\" enabled=\"true\">{text}</CustomModifierBlock>"
    );
    let mut changed = xml.to_owned();
    changed.insert_str(at, &addition);
    let mut inverse = changed.clone();
    inverse.replace_range(at..at + addition.len(), "");
    assert_eq!(inverse, xml);
    changed
}

fn resource_row<'a>(trace: &'a Value, name: &str) -> &'a Value {
    trace
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()
}

// Observation-only comparison. Preserve each source-qualified record and its
// multiplicity, but do not turn PoB table traversal into a native ordering law.
// Raw order remains in a separate diagnostic artifact. Numerical outputs and
// original transformation intermediates are never normalized or recomputed.
fn compare_record_membership(observed: &mut Value) {
    for mode in ["MAIN", "CALCS"] {
        for store in observed["state"]["modes"][mode]["stores"]
            .as_array_mut()
            .unwrap()
        {
            for bucket in store.as_object_mut().unwrap().values_mut() {
                if let Some(records) = bucket.as_array_mut() {
                    records.sort_by_cached_key(|record| serde_json::to_string(record).unwrap());
                } else {
                    assert!(bucket.as_object().unwrap().is_empty());
                }
            }
        }
    }
}

fn child(root: &Path, out: &Path, jit: bool) {
    let originals = resource::originals(root);
    let mut cases: Vec<_> = originals
        .iter()
        .enumerate()
        .map(|(i, xml)| (format!("original-{:02}", i + 1), xml.clone(), String::new()))
        .collect();
    let controls = [
        (
            "es-to-mana-quarter",
            "Convert 25% of maximum Energy Shield to maximum Mana",
        ),
        (
            "es-to-mana-cap",
            "Convert 150% of maximum Energy Shield to maximum Mana",
        ),
        (
            "es-global-to-mana",
            "+100 to maximum Energy Shield\nConvert 25% of maximum Energy Shield to maximum Mana",
        ),
        (
            "armour-gain-ward",
            "Gain 25% of Armour as Extra maximum Runic Ward",
        ),
        (
            "armour-convert-es",
            "25% of Armour converted to Energy Shield",
        ),
        (
            "armour-convert-and-gain",
            "25% of Armour converted to Energy Shield\nGain 25% of Armour as Extra maximum Runic Ward",
        ),
        (
            "armour-convert-and-gain-reversed",
            "Gain 25% of Armour as Extra maximum Runic Ward\n25% of Armour converted to Energy Shield",
        ),
        (
            "evasion-two-conversions",
            "+100 to Evasion Rating\nConverts all Evasion Rating to Armour\n50% of Evasion Rating converted to Energy Shield",
        ),
        (
            "mana-gain-es",
            "Gain 25% of maximum Mana as Extra maximum Energy Shield",
        ),
        (
            "mana-convert-es",
            "25% of maximum Mana converted to Energy Shield",
        ),
        (
            "mana-gain-es-then-es-convert-mana",
            "Gain 25% of maximum Mana as Extra maximum Energy Shield\nConvert 25% of maximum Energy Shield to maximum Mana",
        ),
        (
            "es-to-mana-zero-override",
            "Convert 25% of maximum Energy Shield to maximum Mana\nYou have no Mana",
        ),
    ];
    for (name, text) in controls {
        cases.push((
            name.to_owned(),
            controlled(&originals[4], text),
            text.to_owned(),
        ));
    }
    cases.push((
        "fresh-repeat".to_owned(),
        originals[4].clone(),
        String::new(),
    ));
    cases.push((
        "warm-restoration".to_owned(),
        originals[4].clone(),
        String::new(),
    ));
    let warm_xml = controlled(&originals[4], controls[5].1);
    let mut rows = Vec::new();
    let mut raw_orders = Vec::new();
    for (name, xml, text) in &cases {
        let warm = (name == "warm-restoration").then_some(warm_xml.as_str());
        let mut observed = observe(root, xml, warm, jit, OBSERVER);
        raw_orders.push(json!({"name":name,"xml_sha256":hash(xml.as_bytes()),
            "MAIN":observed["state"]["modes"]["MAIN"]["stores"],
            "CALCS":observed["state"]["modes"]["CALCS"]["stores"]}));
        compare_record_membership(&mut observed);
        assert_eq!(
            observed["state"]["modes"]["MAIN"], observed["state"]["modes"]["CALCS"],
            "{name}"
        );
        assert_eq!(
            observed["state"]["traces"]["MAIN"], observed["state"]["traces"]["CALCS"],
            "{name}"
        );
        rows.push(json!({"name":name,"xml_sha256":hash(xml.as_bytes()),"control":text,"observed":observed}));
    }
    // Persist original observations before the diagnostic assertions: a changed
    // source must leave inspectable evidence, not prompt a retry-until-pass.
    let report = json!({
        "source_revision":pinned::UPSTREAM_REVISION,"manifest_sha256":pinned::manifest_sha256(),
        "observer_sha256":hash(OBSERVER.as_bytes()),
        "driver_sha256":hash(include_bytes!("owned_resource_transformation.rs")),
        "shared_driver_sha256":hash(include_bytes!("support/player_resource_source.rs")),
        "bootstrap_sha256":hash(include_bytes!("support/configuration_preparation_source.rs")),
        "files":(["src/Modules/CalcDefence.lua","src/Modules/ModParser.lua","src/Classes/ModStore.lua","src/Classes/CalcsTab.lua"].map(|path|json!({"path":path,"sha256":pinned::expected_file_sha256(path).unwrap()}))),
        "cases":rows,"complete_loads":cases.len()+1,"native_transformation":false,"whole_build":false,
        "comparison":{"numeric_values_exact":true,"counted_source_record_membership":true,
            "source_record_sequence":false,"raw_order_retained_separately":true}
    });
    fs::write(
        out.join(format!(
            "raw-record-orders-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&raw_orders).unwrap(),
    )
    .unwrap();
    fs::write(
        out.join(format!(
            "source-jit-{}.json",
            if jit { "on" } else { "off" }
        )),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    validate(&report);
}

fn validate(report: &Value) {
    let rows = report["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 19);
    assert_eq!(report["complete_loads"], 20);
    assert_eq!(report["native_transformation"], false);
    assert_eq!(report["whole_build"], false);
    assert_eq!(
        report["comparison"],
        json!({"numeric_values_exact":true,"counted_source_record_membership":true,
        "source_record_sequence":false,"raw_order_retained_separately":true})
    );
    for row in rows {
        let state = &row["observed"]["state"];
        assert_eq!(state["original_methods"], true);
        assert_eq!(state["initial_and_three_rebuilds_equal"], true);
        assert_eq!(state["uninstrumented_results_equal"], true);
        assert_eq!(state["observer_removed"], true);
        assert_eq!(state["native_transformations_claim"], false);
        assert_eq!(state["modes"]["MAIN"], state["modes"]["CALCS"]);
        assert_eq!(state["traces"]["MAIN"], state["traces"]["CALCS"]);
    }
    assert_eq!(rows[4]["observed"], rows[rows.len() - 2]["observed"]);
    assert_eq!(rows[4]["observed"], rows[rows.len() - 1]["observed"]);
    let case = |name: &str| &rows.iter().find(|r| r["name"] == name).unwrap()["observed"]["state"];
    for (name, rate) in [("es-to-mana-quarter", 25.), ("es-to-mana-cap", 100.)] {
        let state = case(name);
        let before = resource_row(&state["traces"]["MAIN"]["before"], "EnergyShield");
        assert_eq!(before["conversionRate"]["Mana"], rate);
        assert!(
            state["modes"]["MAIN"]["resources"]["Mana"]["extra"]
                .as_f64()
                .unwrap()
                > 0.
        );
    }
    for name in [
        "armour-gain-ward",
        "armour-convert-and-gain",
        "armour-convert-and-gain-reversed",
    ] {
        assert_eq!(
            case(name)["modes"]["MAIN"]["resources"]["Armour"]["rates"]["Ward"]["gain"],
            25.
        );
    }
    for name in [
        "armour-convert-es",
        "armour-convert-and-gain",
        "armour-convert-and-gain-reversed",
    ] {
        assert_eq!(
            case(name)["modes"]["MAIN"]["resources"]["Armour"]["rates"]["EnergyShield"]["conversion"],
            25.
        );
    }
    let multi = case("evasion-two-conversions");
    let before = resource_row(&multi["traces"]["MAIN"]["before"], "Evasion");
    assert_eq!(before["conversionRate"]["Armour"], 100.);
    assert_eq!(before["conversionRate"]["EnergyShield"], 50.);
    assert_eq!(before["totalConversion"], 100.);
    let after = &multi["traces"]["MAIN"]["after"];
    assert_eq!(resource_row(after, "Armour")["globalBase"], 107.);
    assert_eq!(resource_row(after, "EnergyShield")["globalBase"], 53.5);
    assert_eq!(resource_row(after, "Evasion")["globalBase"], 0.);

    // These are observations of the pinned source, not adopted native laws.
    // A second gain-as target causes an additional depletion of the same slot.
    let slot_value = |name: &str, phase: &str, resource: &str, slot: &str| {
        resource_row(&case(name)["traces"]["MAIN"][phase], resource)["basePerSlot"][slot]
            .as_f64()
            .unwrap()
    };
    for (slot, base) in [("Boots", 161.), ("Gloves", 16.), ("Helmet", 31.)] {
        assert_eq!(
            slot_value("armour-convert-and-gain", "before", "Armour", slot),
            base
        );
        assert_eq!(
            slot_value("armour-gain-ward", "after", "Armour", slot),
            base
        );
        assert_eq!(
            slot_value("armour-gain-ward", "after", "Ward", slot),
            base * 0.25
        );
        assert_eq!(
            slot_value("armour-convert-es", "after", "Armour", slot),
            base * 0.75
        );
        assert_eq!(
            slot_value("armour-convert-and-gain", "after", "Armour", slot),
            base * 0.75 * 0.75
        );
        assert_eq!(
            slot_value("armour-convert-and-gain", "after", "Ward", slot),
            base * 0.75 * 0.25
        );
    }
    let combined = case("armour-convert-and-gain");
    let reversed = case("armour-convert-and-gain-reversed");
    assert_eq!(combined["traces"], reversed["traces"]);
    assert_eq!(
        combined["modes"]["MAIN"]["resources"],
        reversed["modes"]["MAIN"]["resources"]
    );
    for (name, extra, final_mana) in [
        ("es-to-mana-quarter", 25.25, 665.),
        ("es-to-mana-cap", 101., 744.),
        ("es-global-to-mana", 50.25, 691.),
        ("mana-gain-es-then-es-convert-mana", 25.25, 665.),
    ] {
        let mana = &case(name)["modes"]["MAIN"]["resources"]["Mana"];
        assert_eq!(mana["extra"], extra);
        assert_eq!(mana["final"], final_mana);
    }
    for (name, operation) in [("mana-gain-es", "gain"), ("mana-convert-es", "conversion")] {
        assert_eq!(
            case(name)["modes"]["MAIN"]["resources"]["Mana"]["rates"]["EnergyShield"][operation],
            25.
        );
    }
    assert_eq!(
        case("es-to-mana-zero-override")["modes"]["MAIN"]["resources"]["Mana"]["final"],
        0.
    );
}

#[test]
fn retained_resource_transformations_match_current_source_and_driver() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let report: Value = serde_json::from_slice(
        &fs::read(root.join("tests/fixtures/owned-resource-transformation-source.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report["source_revision"], pinned::UPSTREAM_REVISION);
    assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
    assert_eq!(report["observer_sha256"], hash(OBSERVER.as_bytes()));
    assert_eq!(
        report["driver_sha256"],
        hash(include_bytes!("owned_resource_transformation.rs"))
    );
    assert_eq!(
        report["shared_driver_sha256"],
        hash(include_bytes!("support/player_resource_source.rs"))
    );
    assert_eq!(
        report["bootstrap_sha256"],
        hash(include_bytes!(
            "support/configuration_preparation_source.rs"
        ))
    );
    for file in report["files"].as_array().unwrap() {
        assert_eq!(
            file["sha256"],
            pinned::expected_file_sha256(file["path"].as_str().unwrap()).unwrap()
        );
    }
    for (row, xml) in report["cases"]
        .as_array()
        .unwrap()
        .iter()
        .zip(resource::originals(&root))
    {
        assert_eq!(row["xml_sha256"], hash(xml.as_bytes()));
    }
    validate(&report);
}

#[test]
fn source_record_comparison_preserves_values_sources_duplicates_and_numeric_order() {
    let first = json!({"name":"Evasion","type":"INC","value":5,"source":"Skill:first"});
    let second = json!({"name":"Evasion","type":"INC","value":5,"source":"Skill:second"});
    let sample = |records: Vec<Value>| {
        let mode = json!({"stores":[{"Evasion":records}],"resources":{"Evasion":{"final":100}}});
        json!({"state":{"modes":{"MAIN":mode.clone(),"CALCS":mode},
            "traces":{"MAIN":{"before":[{"name":"Armour"},{"name":"Evasion"}]}}}})
    };
    let mut expected = sample(vec![first.clone(), second.clone(), first.clone()]);
    compare_record_membership(&mut expected);
    let mut reversed = sample(vec![first.clone(), first.clone(), second.clone()]);
    compare_record_membership(&mut reversed);
    assert_eq!(reversed, expected);
    let mut missing_duplicate = sample(vec![first.clone(), second]);
    compare_record_membership(&mut missing_duplicate);
    assert_ne!(missing_duplicate, expected);
    for (field, value) in [
        ("value", json!(6)),
        ("source", json!("Skill:other")),
        ("tag", json!("changed")),
    ] {
        let mut changed = expected.clone();
        changed["state"]["modes"]["MAIN"]["stores"][0]["Evasion"][0][field] = value;
        compare_record_membership(&mut changed);
        assert_ne!(
            changed, expected,
            "record field {field} must remain significant"
        );
    }
    let mut changed = expected.clone();
    changed["state"]["modes"]["MAIN"]["resources"]["Evasion"]["final"] = json!(101);
    compare_record_membership(&mut changed);
    assert_ne!(changed, expected);
    let mut changed = expected.clone();
    changed["state"]["traces"]["MAIN"]["before"]
        .as_array_mut()
        .unwrap()
        .reverse();
    compare_record_membership(&mut changed);
    assert_ne!(
        changed, expected,
        "transformation ordering must not be normalized"
    );
}

#[test]
#[ignore = "requires pinned PoB and fresh RESOURCE_TRANSFORMATION_SOURCE_OUT"]
fn original_resource_transformations_preserve_full_source_evidence() {
    resource::supervise(
        "original_resource_transformations_preserve_full_source_evidence",
        "POE_RESOURCE_TRANSFORMATION_SOURCE_CHILD",
        "POE_OPTIMIZER_TEST_RESOURCE_TRANSFORMATION_SOURCE_OUT",
        child,
    );
}
