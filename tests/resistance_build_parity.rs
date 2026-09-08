#![cfg(feature = "pob")]
//! Fresh full-build parity for signed resistance nodes; expected values come from PoB.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_data::{
    class_tree::{self, ClassTreeSelection},
    game_data::bundled_snapshot,
    tree_data::TreeDataSnapshot,
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use std::{collections::BTreeSet, path::PathBuf, sync::Arc, time::Duration};
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const SPARK: &str = include_str!("fixtures/calibration/spark-mapping.xml");
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
fn compare(label: &str, native: &EvaluationResult, pob: &EvaluationResult) {
    native.validate_recorded().unwrap();
    pob.validate_recorded().unwrap();
    assert!(native.diagnostic_only && pob.diagnostic_only);
    for actual in &native.measurements {
        let expected = pob
            .measurements
            .iter()
            .find(|m| m.query == actual.query)
            .unwrap();
        assert_eq!(actual.unit, expected.unit);
        let (a, b) = (
            actual.value.finite().unwrap(),
            expected.value.finite().unwrap(),
        );
        assert!(
            (a - b).abs() <= 1e-8_f64.max(b.abs() * 1e-9),
            "{label} {}: native{a} vs PoB{b}",
            actual.query.id
        );
    }
    assert_eq!(
        native.build.allocated_nodes, pob.build.allocated_nodes,
        "{label}"
    );
    assert_eq!(native.build.class_name, pob.build.class_name);
    assert_eq!(native.build.ascendancy_name, pob.build.ascendancy_name);
}
fn spark(tree: &TreeDataSnapshot, selection: &ClassTreeSelection) -> BuildDocument {
    let class = &tree.classes[&selection.class_id];
    let asc = &tree.ascendancies[selection.ascendancy_id.as_ref().unwrap()];
    let paid: BTreeSet<_> = selection
        .entrance_node_id
        .into_iter()
        .chain(selection.ascendancy_node_id)
        .collect();
    let xml = SPARK
        .replace(
            "className=\"Sorceress\"",
            &format!("className=\"{}\"", class.name),
        )
        .replace(
            "ascendClassName=\"None\"",
            &format!("ascendClassName=\"{}\"", asc.name),
        )
        .replace(
            "classId=\"7\"",
            &format!("classId=\"{}\"", class.integer_id),
        )
        .replace(
            "classInternalId=\"7\"",
            &format!("classInternalId=\"{}\"", class.integer_id),
        )
        .replace(
            "ascendClassId=\"0\"",
            &format!(
                "ascendClassId=\"{}\" ascendancyInternalId=\"{}\"",
                asc.class_index, asc.internal_id
            ),
        )
        .replace(
            "nodes=\"\"",
            &format!(
                "nodes=\"{}\"",
                paid.iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        );
    BuildDocument {
        format: BuildFormat::PathOfBuilding2Xml,
        content: xml,
    }
}
#[test]
fn resistance_nodes_match_fresh_spark_mace_source_and_removal_reimports() {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer"));
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    let tree =
        poe_optimizer_pob::tree_worker::extract_tree(&exe, &source, "0_5", Duration::from_secs(60))
            .unwrap();
    let data = Arc::new(bundled_snapshot().unwrap());
    let catalog = ControlledMaceCatalog::with_tree_choices(
        data.clone(),
        MACE.into(),
        vec![NormalMaceAlternative {
            id: "wood".into(),
            item_text: "Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 20\nImplicits: 0"
                .into(),
        }],
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        class_tree::selections(data.tree()).unwrap(),
    )
    .unwrap();
    let backend = NativeBackend::new();
    let identity = backend.identity();
    let native = Engine::new(backend);
    let oracle = Engine::new(PobBackend::new(exe, source));
    let nb = native
        .evaluate(&request(catalog.template_build()), BUDGET)
        .unwrap();
    let pb = oracle
        .evaluate(&request(catalog.template_build()), BUDGET)
        .unwrap();
    let ns = catalog.bind_native_baseline(&nb, &identity).unwrap();
    let ps = catalog.bind_baseline(&pb).unwrap();
    let mut cases = Vec::new();
    for (class_id, asc, node) in [
        (6, "Warrior3", 14960),
        (11, "Druid2", 61722),
        (10, "Monk3", 24475),
        (8, "Huntress3", 17058),
    ] {
        let entrance = *data
            .tree()
            .entrances(class_id)
            .unwrap()
            .keys()
            .next()
            .unwrap();
        for ordinary in [None, Some(entrance)] {
            let selection = ClassTreeSelection {
                class_id,
                ascendancy_id: Some(asc.into()),
                entrance_node_id: ordinary,
                ascendancy_node_id: Some(node),
            };
            for support in [MaceSupportChoice::None, MaceSupportChoice::BrutalityI] {
                cases.push((selection.clone(), Some(support)));
            }
            cases.push((selection, None));
        }
        // Same owned identity with both paid effects removed checks subtraction/reset.
        let removed = ClassTreeSelection {
            class_id,
            ascendancy_id: Some(asc.into()),
            entrance_node_id: None,
            ascendancy_node_id: None,
        };
        cases.push((removed.clone(), Some(MaceSupportChoice::BrutalityI)));
        cases.push((removed, None));
    }
    assert_eq!(cases.len(), 32);
    let mut reloads = 0;
    for (selection, support) in cases {
        let candidate = support.map(|support| {
            catalog
                .resolve_tree_candidate(&selection, "wood", support)
                .unwrap()
        });
        let build = if let Some(candidate) = candidate {
            catalog.validate_requirements(candidate).unwrap();
            catalog.materialize(candidate).unwrap()
        } else {
            spark(&tree, &selection)
        };
        let label = format!("{selection:?}/{support:?}");
        let req = request(build);
        let actual = native
            .evaluate(&req, BUDGET)
            .unwrap_or_else(|e| panic!("{label} native: {e}"));
        let expected = oracle
            .evaluate(&req, BUDGET)
            .unwrap_or_else(|e| panic!("{label} PoB: {e}"));
        compare(&label, &actual, &expected);
        assert_eq!(actual.exports[0].content, req.build.content);
        if let Some(candidate) = candidate {
            catalog
                .validate_native_realization(candidate, &actual, &ns)
                .unwrap();
            catalog
                .validate_realization(candidate, &expected, &ps)
                .unwrap();
        }
        let live = expected.coverage.passives.as_ref().unwrap();
        live.validate().unwrap();
        assert_eq!(
            live.allocation_counts.ordinary,
            u32::from(selection.entrance_node_id.is_some())
        );
        assert_eq!(
            live.allocation_counts.ascendancy,
            u32::from(selection.ascendancy_node_id.is_some())
        );
        assert_eq!(
            live.ascendancy.as_ref().unwrap().internal_id,
            selection.ascendancy_id
        );
        let mut allocations = BTreeSet::from([
            tree.classes[&selection.class_id].start_node_id,
            tree.ascendancies[selection.ascendancy_id.as_ref().unwrap()].start_node_id,
        ]);
        allocations.extend(selection.entrance_node_id);
        allocations.extend(selection.ascendancy_node_id);
        assert_eq!(
            actual.build.allocated_nodes,
            allocations.into_iter().collect::<Vec<_>>()
        );
        for allocated in &live.allocated_nodes {
            let source = tree
                .effective_node(
                    selection.class_id,
                    selection.ascendancy_id.as_deref(),
                    allocated.physical_node_id,
                )
                .unwrap();
            assert_eq!(allocated.stats, source.stats, "{label}");
            assert_eq!(allocated.display_name, source.name, "{label}");
        }
        if selection.class_id == 8
            && selection.ascendancy_node_id.is_some()
            && selection.entrance_node_id.is_some()
            && support != Some(MaceSupportChoice::None)
        {
            let fresh = oracle
                .evaluate(&request(actual.exports[0].clone()), BUDGET)
                .unwrap();
            compare(&label, &actual, &fresh);
            reloads += 1;
        }
    }
    assert_eq!(reloads, 2);
}
