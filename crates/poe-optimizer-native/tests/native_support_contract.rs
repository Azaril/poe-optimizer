//! Source projection and evidence contracts for injected support loadouts.
//! Numeric expectations are covered independently by fresh PoB parity tests.
use poe_optimizer_core::{candidate::*, evaluation::*, options::EvaluationOptions};
use poe_optimizer_data::game_data::{
    self, GameDataLoader, GameDataPackage, GameDataSnapshot, LoadLimits, TrustPolicy,
};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, CompiledGameData, HostClock, NativeBackend};
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
fn loadouts() -> Vec<Vec<&'static str>> {
    vec![
        vec![],
        vec!["brutality_i"],
        vec!["heavy_swing"],
        vec!["rapid_attacks_i"],
        vec!["brutality_i", "heavy_swing"],
        vec!["brutality_i", "rapid_attacks_i"],
        vec!["heavy_swing", "rapid_attacks_i"],
    ]
}
fn domain(backend: &NativeBackend, template: &str) -> ControlledBuildDomain {
    let parsed = roxmltree::Document::parse(template).unwrap();
    let first_item_id = parsed
        .descendants()
        .filter(|node| node.has_tag_name("Item"))
        .filter_map(|node| node.attribute("id").and_then(|id| id.parse::<u32>().ok()))
        .max()
        .unwrap_or(0)
        + 1;
    let weapons = backend
        .data()
        .snapshot()
        .package()
        .weapons
        .iter()
        .enumerate()
        .map(|(index, weapon)| EquipmentAlternative {
            instance_id: weapon.id.clone(),
            pob_item_id: first_item_id + index as u32,
            item_text: format!(
                "Rarity: NORMAL\n{}\nItem Level: 1\nQuality: 0\nImplicits: 0",
                weapon.name
            ),
        })
        .collect();
    ControlledBuildDomain::new(
        Arc::new(
            ControlledBuildCatalog::new(backend.data().clone(), template.into(), weapons).unwrap(),
        ),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 20,
                ascendancy_passive_points: 8,
                active_skill_count: 1,
                supports_per_skill: 2,
                ..Default::default()
            },
            ..Default::default()
        },
        AttributeOptionLocks::default(),
    )
    .unwrap()
}
fn candidate(
    domain: &ControlledBuildDomain,
    weapon: &str,
    supports: &[&str],
) -> AdmittedBuildSelection {
    let mut selection = domain.catalog().source_selection();
    selection
        .candidate
        .equipment
        .insert("Weapon 1".into(), weapon.into());
    selection
        .candidate
        .skills
        .values_mut()
        .next()
        .unwrap()
        .support_instance_ids = supports
        .iter()
        .map(|key| domain.catalog().support_instance(key).unwrap().to_owned())
        .collect();
    domain
        .admit(selection, &mut ActorScratch::default())
        .unwrap()
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
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=9")
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
    let backend = NativeBackend::new();
    let registry = domain(&backend, TEMPLATE);
    let data = backend.data().snapshot();
    let engine = Engine::new(NativeBackend::new());
    let mut cases = 0;
    for weapon in &data.package().weapons {
        for supports in loadouts() {
            let handle = candidate(&registry, &weapon.id, &supports);
            let xml = registry.materialize(&handle).unwrap().content;
            let result = engine.evaluate(&request(&xml), BUDGET).unwrap();
            registry
                .catalog()
                .validate_native_realization(&handle, &result, &backend.identity())
                .unwrap();
            assert_eq!(result.exports[0].content, xml);
            let evidence = diagnostic(&result);
            assert_eq!(evidence["support_loadout"], serde_json::json!(supports));
            assert_eq!(
                evidence["configured_supports"],
                serde_json::json!(
                    supports
                        .iter()
                        .map(|key| data.package().support(key).unwrap())
                        .collect::<Vec<_>>()
                )
            );
            assert_eq!(result.coverage.groups[0].gems.len(), 1 + supports.len());
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
                // The imported source retains gem order while selected support sets canonicalize it.
                let from_reordered = domain(&backend, &reversed);
                let handle = candidate(&from_reordered, &weapon.id, &[]);
                let materialized = from_reordered.materialize(&handle).unwrap();
                let recalculated = engine
                    .evaluate(&request(&materialized.content), BUDGET)
                    .unwrap();
                from_reordered
                    .catalog()
                    .validate_native_realization(&handle, &recalculated, &backend.identity())
                    .unwrap();
            }
            cases += 1;
        }
    }
    assert_eq!(cases, 14);
}
#[test]
fn support_evidence_tampering_and_old_attachment_versions_fail_realization() {
    let backend = NativeBackend::new();
    let registry = domain(&backend, TEMPLATE);
    let weapon = &backend.data().snapshot().package().weapons[0].id;
    let pair = candidate(&registry, weapon, &["brutality_i", "heavy_swing"]);
    let engine = Engine::new(NativeBackend::new());
    let xml = registry.materialize(&pair).unwrap().content;
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
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=9")
            .unwrap();
        let mut evidence: serde_json::Value = serde_json::from_str(&attachment.content).unwrap();
        *evidence.pointer_mut(pointer).unwrap() = serde_json::json!("tampered");
        attachment.content = evidence.to_string();
        assert!(
            registry
                .catalog()
                .validate_native_realization(&pair, &changed, &backend.identity())
                .is_err(),
            "accepted {pointer}"
        );
    }
    let mut changed = result;
    changed
        .attachments
        .iter_mut()
        .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=9")
        .unwrap()
        .media_type = "application/vnd.poe-optimizer.native-profile+json;version=1".into();
    assert!(
        registry
            .catalog()
            .validate_native_realization(&pair, &changed, &backend.identity())
            .is_err()
    );
}
#[test]
fn duplicate_third_unknown_and_nondefault_support_configurations_are_rejected() {
    let backend = NativeBackend::new();
    let registry = domain(&backend, TEMPLATE);
    let weapon = &backend.data().snapshot().package().weapons[0].id;
    let pair = candidate(&registry, weapon, &["brutality_i", "heavy_swing"]);
    let xml = registry.materialize(&pair).unwrap().content;
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
    let compiled = CompiledGameData::compile(data.clone()).unwrap();
    let backend = NativeBackend::with_data(Arc::new(compiled), HostClock).unwrap();
    let identity = backend.identity();
    let registry = domain(&backend, TEMPLATE);
    let engine = Engine::new(backend);
    let handle = candidate(
        &registry,
        &data.package().weapons[0].id,
        &["rapid_attacks_i"],
    );
    let xml = registry.materialize(&handle).unwrap().content;
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
        .catalog()
        .validate_native_realization(&handle, &changed, &identity)
        .unwrap();
    assert!(
        registry
            .catalog()
            .validate_native_realization(&handle, &reviewed, &identity)
            .is_err()
    );
}
