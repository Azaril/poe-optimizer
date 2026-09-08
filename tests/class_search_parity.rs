#![cfg(feature = "pob")]
//! Fresh reference calculations validate joint materializations, not copied native tables.
use poe_optimizer_core::{
    EvaluationSnapshot,
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_data::{
    class_tree::{ClassTreeSelection, selections},
    game_data::bundled_snapshot,
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
};
use poe_optimizer_native::{NativeBackend, NativeCalculation};
use poe_optimizer_pob::backend::PobBackend;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
fn request(build: BuildDocument) -> EvaluationRequest {
    EvaluationRequest {
        build,
        options: EvaluationOptions::default(),
        metrics: [
            "life",
            "mana",
            "energy_shield",
            "fire_resistance_capped_pct",
            "cold_resistance_capped_pct",
            "lightning_resistance_capped_pct",
            "chaos_resistance_capped_pct",
            "selected_hit_dps",
        ]
        .into_iter()
        .map(|id| MetricQuery {
            actor: ActorScope::Player,
            id: id.into(),
        })
        .collect(),
    }
}
fn close(label: &str, metric: &str, actual: f64, expected: f64) {
    let tolerance = 1e-8_f64.max(expected.abs() * 1e-9);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label} {metric}: native {actual}, fresh PoB {expected}, tolerance {tolerance}"
    );
}
fn compare(label: &str, actual: &EvaluationResult, expected: &EvaluationResult) {
    actual.validate_recorded().unwrap();
    expected.validate_recorded().unwrap();
    assert!(actual.diagnostic_only && expected.diagnostic_only);
    assert_eq!(actual.measurements.len(), 8);
    assert_eq!(expected.measurements.len(), 8);
    for measurement in &actual.measurements {
        let oracle = expected
            .measurements
            .iter()
            .find(|other| other.query == measurement.query)
            .unwrap();
        assert_eq!(measurement.unit, oracle.unit);
        assert_eq!(measurement.schema_version, oracle.schema_version);
        close(
            label,
            &measurement.query.id,
            measurement.value.finite().unwrap(),
            oracle.value.finite().unwrap(),
        );
    }
    assert_eq!(
        actual.build.class_name, expected.build.class_name,
        "{label}"
    );
    assert_eq!(
        actual.build.ascendancy_name, expected.build.ascendancy_name,
        "{label}"
    );
    assert_eq!(
        actual.build.allocated_nodes, expected.build.allocated_nodes,
        "{label}"
    );
    assert_eq!(
        actual.backend.rules_revision,
        expected.backend.rules_revision
    );
    assert_eq!(actual.context.requested, expected.context.requested);
    assert_eq!(actual.context.enemy_level, expected.context.enemy_level);
}

#[test]
fn joint_materializations_and_fresh_export_reimports_match_pinned_pob() {
    let executable = PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer"));
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    let independently_extracted = poe_optimizer_pob::tree_worker::extract_tree(
        &executable,
        &source,
        "0_5",
        Duration::from_secs(60),
    )
    .unwrap();
    let snapshot = Arc::new(bundled_snapshot().unwrap());
    let trees = selections(&snapshot.package().tree).unwrap();
    let weapons = [("wood", "Wooden Club"), ("smith", "Smithing Hammer")]
        .into_iter()
        .map(|(id, name)| NormalMaceAlternative {
            id: id.into(),
            item_text: format!("Rarity: NORMAL\n{name}\nItem Level: 1\nQuality: 20\nImplicits: 0"),
        })
        .collect();
    let template = TEMPLATE.replace("  <Tree", "  <!-- unchanged café source note -->\n  <Tree");
    let catalog = ControlledMaceCatalog::with_tree_choices(
        snapshot.clone(),
        template.clone(),
        weapons,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        trees.clone(),
    )
    .unwrap();
    let backend = NativeBackend::new();
    let native_identity = backend.identity();
    let native = Engine::new(backend);
    let oracle = Engine::new(PobBackend::new(executable, source));
    let baseline_request = request(catalog.template_build());
    let native_baseline = native.evaluate(&baseline_request, BUDGET).unwrap();
    let pob_baseline = oracle.evaluate(&baseline_request, BUDGET).unwrap();
    let native_scenario = catalog
        .bind_native_baseline(&native_baseline, &native_identity)
        .unwrap();
    let pob_scenario = catalog.bind_baseline(&pob_baseline).unwrap();
    let mut cases = Vec::new();
    // Every supported class participates with its own entrance and an ascendancy.
    for &class_id in &[1, 2, 6, 7, 8, 9, 10, 11] {
        let class = snapshot.package().tree.class(class_id).unwrap();
        let selection = trees
            .iter()
            .find(|tree| {
                tree.class_id == class_id
                    && tree.ascendancy_id.as_ref() == class.ascendancy_ids.first()
                    && tree.entrance_node_id.is_some()
            })
            .unwrap()
            .clone();
        let weapon = if class.base_strength >= 11 {
            "smith"
        } else {
            "wood"
        };
        let support = if class_id % 2 == 0 {
            MaceSupportChoice::BrutalityI
        } else {
            MaceSupportChoice::None
        };
        cases.push((selection, weapon, support));
    }
    // Explicit aliases sharing a root and two class-dependent physical-node overrides.
    for selection in [
        ClassTreeSelection {
            class_id: 1,
            ascendancy_id: Some("Witch3".into()),
            entrance_node_id: None,
            ascendancy_node_id: None,
        },
        ClassTreeSelection {
            class_id: 1,
            ascendancy_id: Some("Witch3b".into()),
            entrance_node_id: Some(4739),
            ascendancy_node_id: None,
        },
        ClassTreeSelection {
            class_id: 8,
            ascendancy_id: Some("Huntress1".into()),
            entrance_node_id: Some(56651),
            ascendancy_node_id: None,
        },
        ClassTreeSelection {
            class_id: 6,
            ascendancy_id: None,
            entrance_node_id: None,
            ascendancy_node_id: None,
        },
    ] {
        cases.push((selection, "wood", MaceSupportChoice::BrutalityI));
    }
    assert_eq!(cases.len(), 12);
    let mut reimports = 0;
    for (selection, weapon, support) in cases {
        let candidate = catalog
            .resolve_tree_candidate(&selection, weapon, support)
            .unwrap();
        catalog.validate_requirements(candidate).unwrap();
        let materialized = catalog.materialize(candidate).unwrap();
        let label = format!("{selection:?}/{weapon}/{support:?}");
        assert!(
            materialized
                .content
                .contains("<!-- unchanged café source note -->")
        );
        let decoded = poe_optimizer_import::decode_build(materialized.content.as_bytes()).unwrap();
        assert_eq!(decoded.xml, materialized.content);
        let req = request(materialized);
        let actual = native
            .evaluate(&req, BUDGET)
            .unwrap_or_else(|error| panic!("{label} native: {error}"));
        let expected = oracle
            .evaluate(&req, BUDGET)
            .unwrap_or_else(|error| panic!("{label} PoB: {error}"));
        compare(&label, &actual, &expected);
        catalog
            .validate_native_realization(candidate, &actual, &native_scenario)
            .unwrap_or_else(|error| panic!("{label} native realization: {error}"));
        catalog
            .validate_realization(candidate, &expected, &pob_scenario)
            .unwrap_or_else(|error| panic!("{label} PoB realization: {error}"));
        assert_eq!(actual.exports[0].content, req.build.content);
        assert!(actual.coverage.passives.is_none());
        let live = expected
            .coverage
            .passives
            .as_ref()
            .expect("fresh source-owned passive coverage");
        live.validate().unwrap();
        let independent_class = &independently_extracted.classes[&selection.class_id];
        assert_eq!(live.class.internal_id, selection.class_id);
        assert_eq!(live.class.name, independent_class.name);
        let mut allocated = BTreeSet::from([independent_class.start_node_id]);
        if let Some(ascendancy_id) = &selection.ascendancy_id {
            let independent_ascendancy = &independently_extracted.ascendancies[ascendancy_id];
            allocated.insert(independent_ascendancy.start_node_id);
            assert_eq!(
                live.ascendancy.as_ref().unwrap().internal_id.as_ref(),
                Some(ascendancy_id)
            );
        } else {
            assert!(live.ascendancy.is_none());
        }
        allocated.extend(selection.entrance_node_id);
        assert_eq!(
            actual.build.allocated_nodes,
            allocated.into_iter().collect::<Vec<_>>()
        );
        assert_eq!(
            live.allocation_counts.ordinary,
            u32::from(selection.entrance_node_id.is_some())
        );
        assert_eq!(live.allocation_counts.ascendancy, 0);
        for node in &live.allocated_nodes {
            let independently_resolved = independently_extracted
                .effective_node(
                    selection.class_id,
                    selection.ascendancy_id.as_deref(),
                    node.physical_node_id,
                )
                .unwrap();
            assert_eq!(node.stats, independently_resolved.stats, "{label}");
            assert_eq!(node.display_name, independently_resolved.name, "{label}");
            if let Some(switch) = &node.switch {
                assert_eq!(
                    switch.selected_source_id,
                    independently_resolved.effective_source_id
                );
                assert!(switch.stats_reference_matches_selected_source);
                assert!(switch.display_name_matches_selected_source);
            }
        }
        let source: EvaluationSnapshot = serde_json::from_str(
            &expected
                .attachments
                .iter()
                .find(|attachment| {
                    attachment
                        .media_type
                        .starts_with("application/vnd.poe-optimizer.pob-snapshot+")
                })
                .unwrap()
                .content,
        )
        .unwrap();
        let NativeCalculation::Mace(calculated) = NativeBackend::new()
            .prepare(&req)
            .unwrap()
            .calculate()
            .unwrap()
        else {
            panic!("Mace fixture changed skill")
        };
        for (metric, value) in [
            ("Str", calculated.strength),
            ("Dex", calculated.dexterity),
            ("Int", calculated.intelligence),
            ("Speed", calculated.attack_rate),
            ("AverageDamage", calculated.average_damage),
            ("HitChance", calculated.hit_chance),
        ] {
            close(&label, metric, value, source.player.metrics[metric]);
        }
        if selection.ascendancy_id.as_deref() == Some("Witch3b")
            || (selection.class_id == 8
                && selection.entrance_node_id == Some(56651)
                && support == MaceSupportChoice::BrutalityI)
        {
            let reimport = request(actual.exports[0].clone());
            let fresh = oracle.evaluate(&reimport, BUDGET).unwrap();
            compare(&label, &actual, &fresh);
            catalog
                .validate_realization(candidate, &fresh, &pob_scenario)
                .unwrap();
            reimports += 1;
        }
    }
    assert!(reimports >= 2);
}
