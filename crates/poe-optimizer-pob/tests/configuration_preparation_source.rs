//! Complete original ConfigTab lifecycle with paired native loader-local prefixes.
//! No callback, effective configuration, or numerical native parity is claimed.
#![cfg(not(target_arch = "wasm32"))]
#[path = "support/configuration_preparation_source.rs"]
mod source;

use poe_optimizer_core::{build_identity::BuildLineage, build_view::ViewRequest};
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{
    CompiledGameData,
    configuration::{
        ConfigurationBlockText, ConfigurationContinuationStage, ConfigurationPrefixStatus,
        ConfigurationPreparationLimits, ConfigurationValue, prepare_authored_configuration,
    },
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::Arc};
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
        let data = CompiledGameData::bundled().unwrap();
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
            pair_prefix(&xml, &result, &data);
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
            pair_prefix(&xml, &result, &data);
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

fn pair_prefix(xml: &str, trace: &Value, data: &Arc<CompiledGameData>) {
    let build = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([73; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap();
    let view = resolve_view(
        &build,
        data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let prepared = prepare_authored_configuration(
        &build,
        &view,
        data,
        ConfigurationPreparationLimits::default(),
    )
    .unwrap();
    prepared.validate_binding(&build, &view, data).unwrap();
    let report = prepared.report();
    assert_eq!(report.status, ConfigurationPrefixStatus::Prepared);
    assert_eq!(
        report.continuation.as_ref().unwrap().stage,
        if trace["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["name"] == "ConfigTab.Load")
        {
            ConfigurationContinuationStage::UpdateControls
        } else {
            ConfigurationContinuationStage::InitialBuildModList
        }
    );
    assert!(report.failure.is_none());
    assert_eq!(
        report.diagnostics.len(),
        trace["diagnostics"].as_array().map_or(0, Vec::len)
    );
    assert_eq!(
        report.source_sha256,
        format!("{:x}", Sha256::digest(xml.as_bytes()))
    );
    let expected = &trace["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| {
            e["kind"] == "enter"
                && e["name"]
                    == if report.continuation.as_ref().unwrap().stage
                        == ConfigurationContinuationStage::UpdateControls
                    {
                        "ConfigTab.UpdateControls"
                    } else {
                        "ConfigTab.BuildModList"
                    }
        })
        .unwrap()["state"];
    pair_fields(
        &report.default_state,
        &expected["defaultState"],
        "defaultState",
    );
    assert_eq!(
        report
            .continuation
            .as_ref()
            .unwrap()
            .unexecuted_containers
            .len(),
        trace["events"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|event| event["kind"] == "enter" && event["name"] == "ConfigTab.Load")
            .count()
            .saturating_sub(1)
    );
    let winners: Vec<_> = report.sets.iter().filter(|set| set.winner).collect();
    assert_eq!(winners.len(), expected["sets"].as_object().unwrap().len());
    let expected_order = expected["order"].as_object().unwrap();
    assert_eq!(report.order.iter().flatten().count(), expected_order.len());
    for (position, key) in report.order.iter().enumerate() {
        if let Some(key) = key {
            assert_eq!(
                key.value().to_bits(),
                expected_order[&(position + 1).to_string()]
                    .as_f64()
                    .unwrap()
                    .to_bits()
            );
        }
    }
    let active = winners
        .iter()
        .find(|set| {
            serde_json::to_value(set.origin).unwrap()
                == serde_json::to_value(report.active_set).unwrap()
        })
        .unwrap();
    assert_eq!(
        active.key.value().to_bits(),
        expected["active"].as_f64().unwrap().to_bits()
    );
    for actual in winners {
        let expected_set = &expected["sets"][actual.key.value().to_string()];
        assert_eq!(
            serde_json::to_value(&actual.title).unwrap(),
            expected_set["title"]
        );
        pair_fields(&actual.inputs, &expected_set["input"], "input");
        pair_fields(
            &actual.placeholders,
            &expected_set["placeholder"],
            "placeholder",
        );
        let empty = Vec::new();
        let blocks = expected_set["customModsList"]
            .as_array()
            .unwrap_or_else(|| {
                assert_eq!(
                    expected_set["customModsList"],
                    serde_json::json!({}),
                    "only an empty Lua table can encode an empty block list"
                );
                &empty
            });
        assert_eq!(actual.blocks.len(), blocks.len());
        for (actual, expected) in actual.blocks.iter().zip(blocks) {
            assert_eq!(actual.title, expected["title"]);
            assert_eq!(actual.enabled, expected["enabled"]);
            let ConfigurationBlockText::Value { value } = &actual.text else {
                panic!("source case block is text");
            };
            pair_value(value, &expected["text"], "custom modifier text");
        }
    }
}
fn pair_fields(actual: &BTreeMap<String, ConfigurationValue>, expected: &Value, label: &str) {
    let expected = expected.as_object().unwrap();
    assert_eq!(
        actual.keys().collect::<Vec<_>>(),
        expected.keys().collect::<Vec<_>>(),
        "{label} keys"
    );
    for (name, actual) in actual {
        pair_value(actual, &expected[name], &format!("{label}/{name}"));
    }
}
fn pair_value(actual: &ConfigurationValue, expected: &Value, label: &str) {
    match actual {
        ConfigurationValue::Boolean(value) => {
            assert_eq!(Some(*value), expected.as_bool(), "{label}")
        }
        ConfigurationValue::Text(value) => {
            assert_eq!(Some(value.as_str()), expected.as_str(), "{label}")
        }
        ConfigurationValue::Number(value) => assert_eq!(
            value.value().to_bits(),
            expected.as_f64().unwrap().to_bits(),
            "{label}"
        ),
    }
}
