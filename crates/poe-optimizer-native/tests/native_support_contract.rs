//! Source projection and evidence contracts for injected support loadouts.
//! Numeric expectations are covered independently by fresh PoB parity tests.
use poe_optimizer_core::{evaluation::*, options::EvaluationOptions};
use poe_optimizer_data::game_data::{
    self, GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy,
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportLoadout, NormalMaceAlternative,
};
use poe_optimizer_native::{CompiledGameData, HostClock, NativeBackend};
use std::{collections::BTreeMap, sync::Arc};

const TEMPLATE: &str = include_str!("../../../tests/fixtures/calibration/mace-smithing.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 10_000 };
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
fn loadouts() -> Vec<MaceSupportLoadout> {
    [
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
    .into_iter()
    .map(|keys| MaceSupportLoadout::new(keys.into_iter().map(str::to_owned).collect()).unwrap())
    .collect()
}
fn catalog(data: Arc<GameDataSnapshot>, template: &str) -> ControlledMaceCatalog {
    let weapons = data
        .package()
        .weapons
        .iter()
        .map(|weapon| NormalMaceAlternative {
            id: weapon.id.clone(),
            item_text: format!(
                "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: 0\nImplicits: 0",
                weapon.name
            ),
        })
        .collect();
    ControlledMaceCatalog::with_loadouts(data, template.into(), weapons, loadouts()).unwrap()
}
fn values(
    result: &EvaluationResult,
) -> BTreeMap<String, poe_optimizer_core::metrics::MeasurementValue> {
    result
        .measurements
        .iter()
        .map(|m| (m.query.id.clone(), m.value.clone()))
        .collect()
}
fn diagnostic(result: &EvaluationResult) -> serde_json::Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=4")
            .unwrap()
            .content,
    )
    .unwrap()
}
fn support_nodes(xml: &str) -> Vec<String> {
    roxmltree::Document::parse(xml)
        .unwrap()
        .descendants()
        .filter(|node| node.has_tag_name("Gem"))
        .skip(1)
        .map(|node| xml[node.range()].to_owned())
        .collect()
}
#[test]
fn every_loadout_evaluates_and_reimports_with_exact_source_order_and_selected_data() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data.clone(), TEMPLATE);
    let backend = NativeBackend::new();
    let engine = Engine::new(NativeBackend::new());
    let baseline = engine.evaluate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    for alternative in registry.alternatives() {
        let xml = registry
            .materialize(&alternative.candidate)
            .unwrap()
            .content;
        let result = engine.evaluate(&request(&xml), BUDGET).unwrap();
        registry
            .validate_native_realization(&alternative.candidate, &result, &scenario)
            .unwrap();
        assert_eq!(result.exports[0].content, xml);
        let evidence = diagnostic(&result);
        assert_eq!(
            evidence["support_loadout"],
            serde_json::json!(alternative.support.keys())
        );
        assert_eq!(
            evidence["configured_supports"],
            serde_json::json!(
                alternative
                    .support
                    .keys()
                    .iter()
                    .map(|key| data.package().support(key).unwrap())
                    .collect::<Vec<_>>()
            )
        );
        assert_eq!(
            result.coverage.groups[0].gems.len(),
            1 + alternative.support.keys().len()
        );
        let prepared = backend.prepare(&request(&xml)).unwrap();
        assert_eq!(
            format!("{:?}", prepared.calculate().unwrap()),
            format!("{:?}", prepared.calculate().unwrap())
        );
        let nodes = support_nodes(&xml);
        if nodes.len() == 2 {
            let first = xml.find(&nodes[0]).unwrap();
            let second = xml.find(&nodes[1]).unwrap();
            let mut reversed = xml.clone();
            reversed.replace_range(second..second + nodes[1].len(), &nodes[0]);
            reversed.replace_range(first..first + nodes[0].len(), &nodes[1]);
            let reordered = engine.evaluate(&request(&reversed), BUDGET).unwrap();
            assert_eq!(values(&reordered), values(&result));
            assert_eq!(diagnostic(&reordered), evidence);
            assert_eq!(reordered.exports[0].content, reversed);
            assert_eq!(
                reordered.coverage.groups[0].gems[1].skill_id,
                result.coverage.groups[0].gems[2].skill_id
            );
            // A reordered imported baseline retains gem-instance order while candidates canonicalize it.
            let from_reordered = catalog(data.clone(), &reversed);
            let scenario = from_reordered
                .bind_native_baseline(&reordered, &backend.identity())
                .unwrap();
            let candidate = &from_reordered.alternatives()[0].candidate;
            let materialized = from_reordered.materialize(candidate).unwrap();
            let recalculated = engine
                .evaluate(&request(&materialized.content), BUDGET)
                .unwrap();
            from_reordered
                .validate_native_realization(candidate, &recalculated, &scenario)
                .unwrap();
        }
    }
}
#[test]
fn support_evidence_tampering_and_old_attachment_versions_fail_realization() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data, TEMPLATE);
    let engine = Engine::new(NativeBackend::new());
    let baseline = engine.evaluate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry
        .bind_native_baseline(&baseline, &poe_optimizer_native::backend_identity())
        .unwrap();
    let pair = registry
        .alternatives()
        .iter()
        .find(|alt| alt.support.keys().len() == 2)
        .unwrap();
    let xml = registry.materialize(&pair.candidate).unwrap().content;
    let result = engine.evaluate(&request(&xml), BUDGET).unwrap();
    for pointer in [
        "/support_loadout/0",
        "/configured_supports/0/id",
        "/configured_supports/0/family",
        "/configured_supports/0/modifiers/0/value",
    ] {
        let mut changed: EvaluationResult =
            serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
        let attachment = changed
            .attachments
            .iter_mut()
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=4")
            .unwrap();
        let mut evidence: serde_json::Value = serde_json::from_str(&attachment.content).unwrap();
        *evidence.pointer_mut(pointer).unwrap() = serde_json::json!("tampered");
        attachment.content = evidence.to_string();
        assert!(
            registry
                .validate_native_realization(&pair.candidate, &changed, &scenario)
                .is_err(),
            "accepted {pointer}"
        );
    }
    let mut changed = result;
    changed
        .attachments
        .iter_mut()
        .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=4")
        .unwrap()
        .media_type = "application/vnd.poe-optimizer.native-profile+json;version=1".into();
    assert!(
        registry
            .validate_native_realization(&pair.candidate, &changed, &scenario)
            .is_err()
    );
}
#[test]
fn duplicate_third_unknown_and_nondefault_support_configurations_are_rejected() {
    let data = Arc::new(game_data::bundled_snapshot().unwrap());
    let registry = catalog(data, TEMPLATE);
    let pair = registry
        .alternatives()
        .iter()
        .find(|alt| alt.support.keys().len() == 2)
        .unwrap();
    let xml = registry.materialize(&pair.candidate).unwrap().content;
    let nodes = support_nodes(&xml);
    for bad in [
        xml.replace(&nodes[1], &nodes[0]),
        xml.replace(&nodes[1], &format!("{}{}", nodes[0], nodes[1])),
        xml.replace(
            &nodes[1],
            &nodes[1].replace("quality=\"0\"", "quality=\"1\""),
        ),
        xml.replace(
            &nodes[1],
            &nodes[1].replace("enabled=\"true\"", "enabled=\"false\""),
        ),
        xml.replace(
            &nodes[1],
            &nodes[1].replace("skillId=\"", "skillId=\"Unknown"),
        ),
    ] {
        assert!(NativeBackend::new().prepare(&request(&bad)).is_err());
    }
}
fn custom(edit: impl FnOnce(&mut GameDataPackage)) -> Arc<GameDataSnapshot> {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    Arc::new(
        GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap(),
    )
}
#[test]
fn injected_support_modifiers_drive_evaluation_identity_and_bound_evidence() {
    let data = custom(|package| {
        package
            .supports
            .iter_mut()
            .find(|gem| gem.id == "rapid_attacks_i")
            .unwrap()
            .modifiers[0]
            .value = 37.0;
    });
    let registry = catalog(data.clone(), TEMPLATE);
    let compiled = CompiledGameData::compile(data.clone()).unwrap();
    let backend = NativeBackend::with_data(Arc::new(compiled), HostClock).unwrap();
    let identity = backend.identity();
    let engine = Engine::new(backend);
    let baseline = engine.evaluate(&request(TEMPLATE), BUDGET).unwrap();
    let scenario = registry.bind_native_baseline(&baseline, &identity).unwrap();
    let loadout = MaceSupportLoadout::new(vec!["rapid_attacks_i".into()]).unwrap();
    let candidate = registry
        .resolve_loadout_candidate(&data.package().weapons[0].id, &loadout)
        .unwrap();
    let xml = registry.materialize(candidate).unwrap().content;
    let changed = engine.evaluate(&request(&xml), BUDGET).unwrap();
    let reviewed = Engine::new(NativeBackend::new())
        .evaluate(&request(&xml), BUDGET)
        .unwrap();
    assert_ne!(
        values(&changed)["selected_hit_dps"],
        values(&reviewed)["selected_hit_dps"]
    );
    assert_eq!(
        diagnostic(&changed)["configured_supports"][0]["modifiers"][0]["value"],
        37.0
    );
    assert_ne!(changed.backend.data, reviewed.backend.data);
    registry
        .validate_native_realization(candidate, &changed, &scenario)
        .unwrap();
    assert!(
        registry
            .validate_native_realization(candidate, &reviewed, &scenario)
            .is_err()
    );
}
