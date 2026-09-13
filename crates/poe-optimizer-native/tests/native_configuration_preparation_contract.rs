//! Public preparation must retain loader state and reject stale raw-input adapters.
use poe_optimizer_core::{build_identity::BuildLineage, evaluation::*, options::EvaluationOptions};
use poe_optimizer_data::{
    configuration::ConfigStringRewrite,
    game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot},
};
use poe_optimizer_import::controlled_build::ControlledBuildCatalog;
use poe_optimizer_native::{
    CompiledGameData, HostClock, NativeBackend, PreparationOutcome,
    configuration::{ConfigurationContinuationStage, ConfigurationPrefixStatus},
};
use std::{path::Path, sync::Arc};

const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-passive-equipment.xml");
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn changed_policy_for(key: &str, pattern: &str, replacement: &str) -> NativeBackend {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let rules = &mut package.configuration.authored_load.input_string_rewrites;
    let operations = vec![ConfigStringRewrite::LuaGsub {
        pattern: pattern.into(),
        replacement: replacement.into(),
    }];
    if let Some(rule) = rules.iter_mut().find(|rule| rule.key == key) {
        rule.operations.extend(operations);
    } else {
        let mut rule = rules[0].clone();
        rule.key = key.into();
        rule.operations = operations;
        rules.push(rule);
    }
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
        HostClock,
    )
    .unwrap()
}
#[test]
fn all_five_originals_report_loaded_configuration_prefix_and_pending_effects() {
    let backend = NativeBackend::new();
    for ordinal in 1..=5u8 {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../tests/fixtures/builds/breadth-20260908/build-{ordinal:02}.xml"
        ));
        let xml = std::fs::read_to_string(&path).unwrap();
        let PreparationOutcome::Incomplete(report) = backend
            .prepare_request_with_lineage(&request(&xml), BuildLineage::from_bytes([ordinal; 16]))
            .unwrap()
        else {
            panic!("original {ordinal} gained unsupported numeric admission");
        };
        let prefix = report.authored_configuration.as_ref().unwrap();
        assert_eq!(prefix.status, ConfigurationPrefixStatus::Prepared);
        assert_eq!(prefix.source_sha256, report.view.source_sha256);
        assert_eq!(prefix.data, *backend.data().identity());
        assert_eq!(
            prefix.continuation.as_ref().unwrap().stage,
            ConfigurationContinuationStage::UpdateControls
        );
        assert_eq!(prefix.sets.len(), 1);
        let active = &prefix.sets[0];
        assert!(active.winner);
        assert_eq!(active.inputs["enemyIsBoss"].text(), Some("Pinnacle"));
        assert!(!active.inputs.contains_key("enemyLevel"));
        assert_eq!(active.placeholders["enemyLevel"].number(), Some(82.0));
        assert!(
            report
                .issues
                .iter()
                .any(|issue| issue.stage == "configuration_effects")
        );
        let serialized = serde_json::to_value(report.as_ref()).unwrap();
        assert_eq!(serialized["schema_version"], 5);
        for key in [
            "metrics",
            "measurements",
            "modifiers",
            "effective_configuration",
        ] {
            assert!(serialized["authored_configuration"].get(key).is_none());
        }
        assert_eq!(std::fs::read_to_string(&path).unwrap(), xml);
    }
}
#[test]
fn injected_input_migration_cannot_calculate_from_stale_raw_configuration() {
    let backend = changed_policy_for("enemyIsBoss", "^None$", "Boss");
    for xml in [SPARK, MACE] {
        let input = request(xml);
        let PreparationOutcome::Incomplete(report) = backend
            .prepare_request_with_lineage(&input, BuildLineage::from_bytes([122; 16]))
            .unwrap()
        else {
            panic!("changed source policy gained stale numeric admission");
        };
        let prefix = report.authored_configuration.as_ref().unwrap();
        assert_eq!(prefix.status, ConfigurationPrefixStatus::Prepared);
        assert_eq!(prefix.sets[0].inputs["enemyIsBoss"].text(), Some("Boss"));
        let expected = "Processed configuration input enemyIsBoss differs";
        assert!(
            report
                .legacy_adapter_error
                .as_ref()
                .unwrap()
                .contains(expected)
        );
        let failure = backend
            .prepare_with_lineage(&input, BuildLineage::from_bytes([123; 16]))
            .err()
            .unwrap();
        assert_eq!(failure.kind, EvaluationErrorKind::UnsupportedCapability);
        assert!(failure.message.contains(expected));
    }
    // This separate candidate entry point prepares a fixed scenario directly.
    // It must use the same loader guard without retaining XML in worker state.
    let catalog = ControlledBuildCatalog::new(backend.data().clone(), MACE.into(), vec![]).unwrap();
    let failure = backend
        .prepare_controlled_build_with_lineage(&catalog, &[], BuildLineage::from_bytes([124; 16]))
        .err()
        .unwrap();
    assert_eq!(failure.kind, EvaluationErrorKind::UnsupportedCapability);
    assert!(
        failure
            .message
            .contains("Processed configuration input enemyIsBoss differs")
    );
}

#[test]
fn injected_legacy_modifier_rewrite_cannot_bypass_document_or_candidate_projection() {
    let backend = changed_policy_for("customMods", "100", "200");
    let original = roxmltree::Document::parse(MACE).unwrap();
    let block = original
        .descendants()
        .find(|node| node.has_tag_name("CustomModifierBlock"))
        .unwrap();
    let mut source = MACE.to_owned();
    source.replace_range(block.range(), "");
    let source = source.replace(
        "</ConfigSet>",
        "<Input name=\"customMods\" string=\"+100 to Strength\"/></ConfigSet>",
    );
    let PreparationOutcome::Incomplete(report) = backend
        .prepare_request_with_lineage(&request(&source), BuildLineage::from_bytes([125; 16]))
        .unwrap()
    else {
        panic!("rewritten legacy actor modifiers gained stale admission");
    };
    let prefix = report.authored_configuration.as_ref().unwrap();
    assert_eq!(prefix.status, ConfigurationPrefixStatus::Prepared);
    let serialized = serde_json::to_value(&prefix.sets[0].blocks).unwrap();
    assert_eq!(serialized[0]["text"]["value"]["value"], "+200 to Strength");
    assert!(
        report
            .legacy_adapter_error
            .as_ref()
            .unwrap()
            .contains("Processed configuration modifier blocks differ")
    );
    let catalog = ControlledBuildCatalog::new(backend.data().clone(), source, vec![]).unwrap();
    let failure = backend
        .prepare_controlled_build_with_lineage(&catalog, &[], BuildLineage::from_bytes([126; 16]))
        .err()
        .unwrap();
    assert_eq!(failure.kind, EvaluationErrorKind::UnsupportedCapability);
    assert!(
        failure
            .message
            .contains("Processed configuration modifier blocks differ")
    );
}
