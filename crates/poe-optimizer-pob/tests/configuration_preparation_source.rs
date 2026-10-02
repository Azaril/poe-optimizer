//! Complete original ConfigTab lifecycle and saved-configuration source controls.
//! This is independent reference evidence, not native evaluator parity.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::PathBuf,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

#[test]
fn complete_original_configuration_lifecycle_all_five_builds() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap();
    if std::env::var_os("POE_CONFIGURATION_SOURCE_CHILD").is_some() {
        let destination =
            PathBuf::from(std::env::var_os("POE_CONFIGURATION_SOURCE_OUTPUT").unwrap());
        let source_hash =
            poe_optimizer_pob::source::verify(&root.join("vendor/path-of-building-poe2")).unwrap();
        let manifest: Value = serde_json::from_slice(
            &fs::read(root.join("tests/fixtures/builds/breadth-20260908/index.json")).unwrap(),
        )
        .unwrap();
        for ordinal in [2, 5, 1, 3, 4] {
            let xml_path = root.join(format!(
                "tests/fixtures/builds/breadth-20260908/build-{ordinal:02}.xml"
            ));
            let xml = fs::read_to_string(&xml_path).unwrap();
            assert_eq!(
                format!("{:x}", Sha256::digest(xml.as_bytes())),
                manifest["builds"][ordinal - 1]["xml_sha256"]
            );
            let scratch = tempfile::tempdir().unwrap();
            let result = source::observe(
                &root.join("vendor/path-of-building-poe2"),
                scratch.path(),
                &xml,
                None,
                false,
            )
            .unwrap();
            validate(&result);
            assert_eq!(result["source_hash"], source_hash);
            fs::write(
                destination.join(format!("build-{ordinal:02}.json")),
                serde_json::to_vec_pretty(&result).unwrap(),
            )
            .unwrap();
        }
        // A fresh runtime importing an existing original first exercises PoB's
        // reused Build object. The observation starts before the target import.
        let xml =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-02.xml"))
                .unwrap();
        let warm =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-05.xml"))
                .unwrap();
        let scratch = tempfile::tempdir().unwrap();
        let result = source::observe(
            &root.join("vendor/path-of-building-poe2"),
            scratch.path(),
            &xml,
            Some(&warm),
            false,
        )
        .unwrap();
        validate(&result);
        assert_eq!(result["source_hash"], source_hash);
        let cold: Value =
            serde_json::from_slice(&fs::read(destination.join("build-02.json")).unwrap()).unwrap();
        let prefix = |trace: &Value| {
            trace["events"]
                .as_array()
                .unwrap()
                .iter()
                .find(|e| e["kind"] == "enter" && e["name"] == "ConfigTab.UpdateControls")
                .unwrap()["state"]
                .clone()
        };
        for field in [
            "input",
            "placeholder",
            "defaultState",
            "sets",
            "order",
            "active",
            "input_alias",
            "placeholder_alias",
            "modList",
            "enemyModList",
            "enemyLevel",
        ] {
            assert_eq!(
                prefix(&cold)[field],
                prefix(&result)[field],
                "warm prefix {field}"
            );
            assert_eq!(
                cold["final"][field], result["final"][field],
                "warm final {field}"
            );
        }
        fs::write(
            destination.join("build-02-after-05.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
        let base =
            fs::read_to_string(root.join("tests/fixtures/builds/breadth-20260908/build-01.xml"))
                .unwrap();
        for (name, replacement, expected_diagnostics) in [
            (
                "duplicate-fallback",
                r#"<Config activeConfigSet="99"><ConfigSet id="7" title="old"/><ConfigSet id="7.0" title="winner"><Input name="marker" number="1" string="ignored"/><Input name="marker" number="bad"/><Input name="enemyIsBoss" string="sHaPeR"/><Input name="customMods" string="+10 to maximum Life"/></ConfigSet><ConfigSet id="2" title="two"/></Config>"#,
                0,
            ),
            (
                "legacy-hole",
                r#"<Config activeConfigSet="1"><ConfigSet id="7" title="seven"/><Input name="zero" number="-0"/><Placeholder name="stringMarker" string="literal"/><Input name="flag" boolean="TRUE"/><Input name="customMods" string="+20 to maximum Life"/><ConfigSet id="2" title="two"/></Config>"#,
                0,
            ),
            (
                "repeated-containers",
                r#"<Config activeConfigSet="7"><ConfigSet id="7"><Input name="marker" string="first"/></ConfigSet></Config><Config activeConfigSet="9"><ConfigSet id="9"><Input name="marker" string="second"/></ConfigSet></Config>"#,
                0,
            ),
            ("no-config", "", 0),
            (
                "nonfatal-diagnostics",
                r#"<Config><Input number="2"/><Input name="missingValue"/><Placeholder number="3"/><Placeholder name="wrongType" boolean="true"/><Input name="afterDiagnostics" number="23"/></Config>"#,
                4,
            ),
        ] {
            let document = roxmltree::Document::parse(&base).unwrap();
            let range = document
                .root_element()
                .children()
                .find(|n| n.has_tag_name("Config"))
                .unwrap()
                .range();
            let mut xml = base.clone();
            xml.replace_range(range, replacement);
            let scratch = tempfile::tempdir().unwrap();
            let result = source::observe(
                &root.join("vendor/path-of-building-poe2"),
                scratch.path(),
                &xml,
                None,
                true,
            )
            .unwrap();
            assert_eq!(result["source_hash"], source_hash);
            validate_structural_case(name, &result);
            assert_eq!(
                result["diagnostics"].as_array().map_or(0, Vec::len),
                expected_diagnostics
            );
            assert_eq!(result["final"]["build"]["mainEnv"], true);
            fs::write(
                destination.join(format!("case-{name}.json")),
                serde_json::to_vec_pretty(&result).unwrap(),
            )
            .unwrap();
        }
        return;
    }
    let destination = root.join("runs/r2b-configuration-source");
    fs::create_dir_all(&destination).unwrap();
    let log_path = destination.join("child.log");
    let log = fs::File::create(&log_path).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "complete_original_configuration_lifecycle_all_five_builds",
            "--nocapture",
        ])
        .env("POE_CONFIGURATION_SOURCE_CHILD", "1")
        .env("POE_CONFIGURATION_SOURCE_OUTPUT", &destination)
        .current_dir(root.join("vendor/path-of-building-poe2/src"))
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(120);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(
                status.success(),
                "source child failed: {}",
                fs::read_to_string(log_path).unwrap()
            );
            break;
        }
        if Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("configuration source child exceeded 120 seconds");
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    for ordinal in 1..=5 {
        let result: Value = serde_json::from_slice(
            &fs::read(destination.join(format!("build-{ordinal:02}.json"))).unwrap(),
        )
        .unwrap();
        validate(&result);
    }
}

fn validate(trace: &Value) {
    let events = trace["events"].as_array().unwrap();
    let entry = |name: &str| {
        events
            .iter()
            .position(|e| e["kind"] == "enter" && e["name"] == name)
            .unwrap()
    };
    assert!(entry("ConfigTab.ConfigTab") < entry("ItemsTab.ItemsTab"));
    assert!(entry("ConfigTab.BuildModList") < entry("ConfigTab.Load"));
    assert!(entry("ConfigTab.Load") < entry("CalcsTab.BuildOutput"));
    assert!(
        events
            .iter()
            .any(|e| e["kind"] == "enter" && e["name"] == "apply")
    );
    assert_eq!(trace["final"]["build"]["mainEnv"], true);
    assert_eq!(trace["final"]["build"]["mainOutput"], true);
    assert_eq!(trace["selected"]["config"], 1);
    let constructor = events
        .iter()
        .find(|e| e["kind"] == "exit" && e["name"] == "ConfigTab.ConfigTab")
        .unwrap();
    assert_eq!(constructor["state"]["input"].as_object().unwrap().len(), 45);
    assert_eq!(
        constructor["state"]["placeholder"]
            .as_object()
            .unwrap()
            .len(),
        19
    );
    assert_eq!(
        constructor["state"]["defaultState"]
            .as_object()
            .unwrap()
            .len(),
        563
    );
    let prefix = &events[entry("ConfigTab.UpdateControls")]["state"];
    assert_eq!(prefix["input_alias"], true);
    assert_eq!(prefix["placeholder_alias"], true);
    assert_eq!(prefix["enemyLevel"], 82);
    assert!(!prefix["modList"].as_array().unwrap().is_empty());
    let boss = events
        .iter()
        .find(|e| {
            e["kind"] == "exit" && e["name"] == "apply" && e["details"]["var"] == "enemyIsBoss"
        })
        .unwrap();
    assert_eq!(boss["state"]["placeholder"]["enemyPhysicalDamage"], 965);
    assert_eq!(boss["state"]["placeholder"]["enemyLevel"], 82);
    assert!(events.iter().any(|e| e["kind"] == "control"
        && e["callback"] == "enemyIsBoss"
        && e["details"]["notify"] == true));
    let initial_load = &events[entry("ConfigTab.Load")]["state"];
    assert_eq!(initial_load["placeholder"].as_object().unwrap().len(), 33);
    assert_eq!(initial_load["placeholder"]["enemyPhysicalDamage"], 965);
    // Each new authored set restores source defaults; prior boss placeholders
    // are not implicitly copied. The XML itself supplies the saved values.
    let set_creations: Vec<_> = events
        .iter()
        .filter(|e| e["kind"] == "exit" && e["name"] == "ConfigTab.CreateConfigSet")
        .collect();
    assert_eq!(set_creations.len(), 2);
    for creation in set_creations {
        assert_eq!(
            creation["state"]["sets"]["1"]["placeholder"]["enemyPhysicalDamage"],
            7
        );
    }
}

fn validate_structural_case(name: &str, trace: &Value) {
    let events = trace["events"].as_array().unwrap();
    let loads = events
        .iter()
        .filter(|event| event["kind"] == "enter" && event["name"] == "ConfigTab.Load")
        .count();
    let boundary = if name == "no-config" {
        "ConfigTab.BuildModList"
    } else {
        "ConfigTab.UpdateControls"
    };
    let state = &events
        .iter()
        .find(|event| event["kind"] == "enter" && event["name"] == boundary)
        .unwrap()["state"];
    assert_eq!(state["input_alias"], true);
    assert_eq!(state["placeholder_alias"], true);
    assert_eq!(trace["final"]["build"]["mainOutput"], true);
    assert_eq!(
        loads,
        match name {
            "no-config" => 0,
            "repeated-containers" => 2,
            _ => 1,
        }
    );
    match name {
        "duplicate-fallback" => {
            assert_eq!(state["active"], 7);
            assert_eq!(state["order"], serde_json::json!({"1": 7, "2": 7, "3": 2}));
            assert_eq!(state["sets"].as_object().unwrap().len(), 2);
            assert_eq!(state["sets"]["7"]["title"], "winner");
            assert!(state["input"].get("marker").is_none());
            assert_eq!(state["input"]["enemyIsBoss"], "Pinnacle");
            assert_eq!(
                state["sets"]["7"]["customModsList"],
                serde_json::json!([{
                    "enabled": true, "text": "+10 to maximum Life", "title": "Default"
                }])
            );
        }
        "legacy-hole" => {
            assert_eq!(state["active"], 1);
            assert_eq!(state["order"], serde_json::json!({"1": 7, "6": 2}));
            assert_eq!(state["sets"].as_object().unwrap().len(), 3);
            assert_eq!(
                state["input"]["zero"].as_f64().unwrap().to_bits(),
                (-0.0_f64).to_bits()
            );
            assert_eq!(state["input"]["flag"], false);
            assert!(state["placeholder"].get("stringMarker").is_none());
        }
        "repeated-containers" => {
            assert_eq!(state["active"], 7);
            assert_eq!(state["input"]["marker"], "first");
            assert_eq!(trace["final"]["active"], 9);
            assert_eq!(trace["final"]["input"]["marker"], "second");
        }
        "no-config" => {
            assert_eq!(state["active"], 1);
            assert_eq!(state["order"], serde_json::json!({"1": 1}));
            assert_eq!(state["sets"].as_object().unwrap().len(), 1);
            assert_eq!(trace["final"]["active"], 1);
        }
        "nonfatal-diagnostics" => {
            assert_eq!(state["input"]["afterDiagnostics"], 23);
            assert!(state["input"].get("missingValue").is_none());
            assert!(state["placeholder"].get("wrongType").is_none());
        }
        _ => panic!("unrecognized source control {name}"),
    }
}
