#![cfg(feature = "pob")]
//! Fresh full-build oracle for every support loadout, coupled speed, armour and export.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_data::{class_tree::ClassTreeSelection, game_data::bundled_snapshot};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportLoadout, NormalMaceAlternative,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use std::{path::PathBuf, sync::Arc};
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
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
    .map(|keys| MaceSupportLoadout::new(keys.into_iter().map(String::from).collect()).unwrap())
    .collect()
}
#[test]
fn all_support_loadouts_match_fresh_pob_builds_and_reimports() {
    let exe = PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer"));
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    let data = Arc::new(bundled_snapshot().unwrap());
    let backend = NativeBackend::new();
    let identity = backend.identity();
    let native = Engine::new(backend);
    let oracle = Engine::new(PobBackend::new(exe, source));
    let mut evaluated = 0;
    for (armour, fire) in [(0.0, 0.0), (75.25, -37.5)] {
        let template = MACE
            .replace(
                "enemyArmour\" number=\"0\"",
                &format!("enemyArmour\" number=\"{armour}\""),
            )
            .replace(
                "enemyFireResist\" number=\"0\"",
                &format!("enemyFireResist\" number=\"{fire}\""),
            );
        let trees = vec![
            ClassTreeSelection {
                class_id: 6,
                ascendancy_id: None,
                entrance_node_id: None,
                ascendancy_node_id: None,
            },
            ClassTreeSelection {
                class_id: 10,
                ascendancy_id: Some("Monk3".into()),
                entrance_node_id: Some(10364),
                ascendancy_node_id: Some(24475),
            },
            ClassTreeSelection {
                class_id: 6,
                ascendancy_id: Some("Warrior3".into()),
                entrance_node_id: Some(3936),
                ascendancy_node_id: Some(14960),
            },
            ClassTreeSelection {
                class_id: 11,
                ascendancy_id: Some("Druid2".into()),
                entrance_node_id: None,
                ascendancy_node_id: Some(61722),
            },
            ClassTreeSelection {
                class_id: 8,
                ascendancy_id: Some("Huntress3".into()),
                entrance_node_id: None,
                ascendancy_node_id: Some(17058),
            },
        ];
        let catalog = ControlledMaceCatalog::with_tree_loadouts(
            data.clone(),
            template,
            ["Wooden Club", "Smithing Hammer"]
                .into_iter()
                .enumerate()
                .map(|(i, name)| NormalMaceAlternative {
                    id: i.to_string(),
                    item_text: format!(
                        "Rarity: NORMAL\n{name}\nItem Level: 1\nQuality: 20\nImplicits: 0"
                    ),
                })
                .collect(),
            loadouts(),
            trees.clone(),
        )
        .unwrap();
        let nb = native
            .evaluate(&request(catalog.template_build()), BUDGET)
            .unwrap();
        let pb = oracle
            .evaluate(&request(catalog.template_build()), BUDGET)
            .unwrap();
        compare("baseline", &nb, &pb);
        let ns = catalog.bind_native_baseline(&nb, &identity).unwrap();
        let ps = catalog.bind_baseline(&pb).unwrap();
        for (tree_index, tree) in trees.iter().enumerate() {
            for weapon in ["0", "1"] {
                for loadout in loadouts() {
                    // Every loadout x both weapons x two mitigation scenarios, plus
                    // the speed entrance and each resistance owner with paired support.
                    if tree_index > 0
                        && (weapon != "0"
                            || (tree_index > 1 && loadout.id() != "heavy_swing+rapid_attacks_i"))
                    {
                        continue;
                    }
                    let candidate = catalog
                        .resolve_tree_loadout_candidate(tree, weapon, &loadout)
                        .unwrap();
                    let build = catalog.materialize(candidate).unwrap();
                    let req = request(build);
                    let label = format!("{armour}/{tree_index}/{weapon}/{loadout:?}");
                    let actual = native
                        .evaluate(&req, BUDGET)
                        .unwrap_or_else(|e| panic!("{label}: {e}"));
                    let expected = oracle
                        .evaluate(&req, BUDGET)
                        .unwrap_or_else(|e| panic!("{label}: {e}"));
                    compare(&label, &actual, &expected);
                    catalog
                        .validate_native_realization(candidate, &actual, &ns)
                        .unwrap();
                    catalog
                        .validate_realization(candidate, &expected, &ps)
                        .unwrap();
                    assert_eq!(actual.exports[0].content, req.build.content);
                    evaluated += 1;
                    if tree_index == 0 && weapon == "1" && loadout.id() == "brutality_i+heavy_swing"
                    {
                        let reimport = oracle
                            .evaluate(&request(actual.exports[0].clone()), BUDGET)
                            .unwrap();
                        compare("reimport", &actual, &reimport);
                    }
                }
            }
        }
    }
    assert_eq!(evaluated, 48);
}
