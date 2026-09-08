#![cfg(feature = "pob")]
//! Fresh complete PoB builds validate item assembly and downstream interactions.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_data::{class_tree::ClassTreeSelection, game_data::bundled_snapshot};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportLoadout, MaceWeaponAlternative,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use serde_json::Value;
use std::{path::PathBuf, sync::Arc};
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
fn near(label: &str, a: f64, b: f64) {
    assert!(
        (a - b).abs() <= 1e-8_f64.max(b.abs() * 1e-9),
        "{label}: native {a} vs PoB {b}"
    );
}
fn compare(label: &str, native: &EvaluationResult, pob: &EvaluationResult) {
    native.validate_recorded().unwrap();
    pob.validate_recorded().unwrap();
    assert!(native.diagnostic_only && pob.diagnostic_only);
    for actual in &native.measurements {
        let expected = pob
            .measurements
            .iter()
            .find(|v| v.query == actual.query)
            .unwrap();
        assert_eq!(actual.unit, expected.unit);
        near(
            &format!("{label}/{}", actual.query.id),
            actual.value.finite().unwrap(),
            expected.value.finite().unwrap(),
        );
    }
    assert_eq!(native.build.class_name, pob.build.class_name);
    assert_eq!(native.build.ascendancy_name, pob.build.ascendancy_name);
    assert_eq!(native.build.allocated_nodes, pob.build.allocated_nodes);
    let actual: Value = serde_json::from_str(
        &native
            .attachments
            .iter()
            .find(|a| a.media_type == "application/vnd.poe-optimizer.native-profile+json;version=4")
            .unwrap()
            .content,
    )
    .unwrap();
    let source: Value = serde_json::from_str(
        &pob.attachments
            .iter()
            .find(|a| a.media_type == "application/vnd.poe-optimizer.pob-snapshot+json;version=2")
            .unwrap()
            .content,
    )
    .unwrap();
    for (native_key, pob_key) in [
        ("attack_rate", "Speed"),
        ("crit_chance", "CritChance"),
        ("hit_chance", "HitChance"),
        ("average_damage", "AverageDamage"),
    ] {
        near(
            &format!("{label}/{native_key}"),
            actual[native_key].as_f64().unwrap(),
            source["player"]["metrics"][pob_key].as_f64().unwrap(),
        );
    }
    near(
        &format!("{label}/pre-effective critical cap"),
        actual["weapon_stats"]["critical_chance"]
            .as_f64()
            .unwrap()
            .min(100.0),
        source["player"]["metrics"]["PreEffectiveCritChance"]
            .as_f64()
            .unwrap(),
    );
    assert_eq!(actual["weapon_item"]["affix_legality_verified"], false);
}
#[test]
fn supplied_local_items_supports_speed_caps_and_effect_removal_match_fresh_pob() {
    let example: Value =
        serde_json::from_str(include_str!("../examples/mace-local-weapon-search.json")).unwrap();
    let mut weapons: Vec<MaceWeaponAlternative> =
        serde_json::from_value(example["weapons"].clone()).unwrap();
    weapons.push(MaceWeaponAlternative{id:"wooden-rare-empty".into(),item_text:"Rarity: RARE\nStudy Removal\nWooden Club\nItem Level: 10\nQuality: 20\nLevelReq: 1\nImplicits: 0".into()});
    let supports: Vec<MaceSupportLoadout> =
        serde_json::from_value(example["support_loadouts"].clone()).unwrap();
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
    ];
    let data = Arc::new(bundled_snapshot().unwrap());
    let backend = NativeBackend::new();
    let identity = backend.identity();
    let native = Engine::new(backend);
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    let mut compared = 0;
    let mut reimports = 0;
    for (armour, fire) in [(0.0, 0.0), (125.25, -37.5)] {
        let template = TEMPLATE
            .replace(
                "enemyArmour\" number=\"0\"",
                &format!("enemyArmour\" number=\"{armour}\""),
            )
            .replace(
                "enemyFireResist\" number=\"0\"",
                &format!("enemyFireResist\" number=\"{fire}\""),
            );
        let registry = ControlledMaceCatalog::with_tree_loadouts(
            data.clone(),
            template,
            weapons.clone(),
            supports.clone(),
            trees.clone(),
        )
        .unwrap();
        let baseline = native
            .evaluate(&request(registry.template_build()), BUDGET)
            .unwrap();
        let reference = oracle
            .evaluate(&request(registry.template_build()), BUDGET)
            .unwrap();
        compare("baseline", &baseline, &reference);
        let ns = registry.bind_native_baseline(&baseline, &identity).unwrap();
        let ps = registry.bind_baseline(&reference).unwrap();
        for (tree_index, tree) in trees.iter().enumerate() {
            for weapon in [
                "wooden-balanced",
                "wooden-critical",
                "wooden-physical",
                "smithing-fire",
                "wooden-rare-empty",
            ] {
                if tree_index == 1 && weapon != "wooden-balanced" {
                    continue;
                }
                for support in &supports {
                    if !["wooden-balanced", "wooden-critical"].contains(&weapon)
                        && !support.keys().is_empty()
                    {
                        continue;
                    }
                    let candidate = registry
                        .resolve_tree_loadout_candidate(tree, weapon, support)
                        .unwrap();
                    let req = request(registry.materialize(candidate).unwrap());
                    let actual = native.evaluate(&req, BUDGET).unwrap();
                    let expected = oracle.evaluate(&req, BUDGET).unwrap();
                    let label = format!("{armour}/{tree_index}/{weapon}/{}", support.id());
                    compare(&label, &actual, &expected);
                    registry
                        .validate_native_realization(candidate, &actual, &ns)
                        .unwrap();
                    registry
                        .validate_realization(candidate, &expected, &ps)
                        .unwrap_or_else(|error| panic!("{label}: {error}"));
                    assert_eq!(actual.exports[0].content, req.build.content);
                    compared += 1;
                    if tree_index == 0
                        && ((weapon == "wooden-balanced"
                            && support.id() == "brutality_i+rapid_attacks_i")
                            || (weapon == "smithing-fire" && support.keys().is_empty()))
                    {
                        let reimport = oracle
                            .evaluate(&request(actual.exports[0].clone()), BUDGET)
                            .unwrap();
                        compare("reimport", &actual, &reimport);
                        reimports += 1;
                    }
                }
            }
        }
    }
    assert_eq!(compared, 48);
    assert_eq!(reimports, 4);
}
