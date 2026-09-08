#![cfg(feature = "pob")]
//! Whole-build passive parity uses fresh PoB calculations and separately extracted
//! pinned tree data. Native embedded tables never provide expected metric values.
use poe_optimizer_core::{
    EvaluationSnapshot,
    coverage::{PassiveCoverage, PassiveSwitchKind},
    evaluation::*,
    metrics::{ActorScope, MeasurementValue, MetricMeasurement, MetricQuery},
    options::{EvaluationOptions, Scalar},
};
use poe_optimizer_native::{NativeBackend, NativeCalculation};
use poe_optimizer_pob::{
    backend::PobBackend,
    tree_data::{OverrideProvenance, TreeDataSnapshot},
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

static MAX_ABSOLUTE_DELTA: AtomicU64 = AtomicU64::new(0);
static MAX_SCALED_DELTA: AtomicU64 = AtomicU64::new(0);
const SPARK: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
const FINITE_METRICS: [&str; 8] = [
    "life",
    "mana",
    "energy_shield",
    "fire_resistance_capped_pct",
    "cold_resistance_capped_pct",
    "lightning_resistance_capped_pct",
    "chaos_resistance_capped_pct",
    "selected_hit_dps",
];
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Skill {
    Spark,
    Mace,
}
impl Skill {
    fn fixture(self) -> &'static str {
        match self {
            Self::Spark => SPARK,
            Self::Mace => MACE,
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Spark => "SparkPlayer",
            Self::Mace => "Melee1HMacePlayer",
        }
    }
}
struct Case {
    label: String,
    class_id: u32,
    ascendancy_id: Option<String>,
    paid: Option<u32>,
    skill: Skill,
    xml: String,
    oracle_reimport: bool,
}
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: FINITE_METRICS
            .into_iter()
            .chain(["selected_average_hit"])
            .map(|id| MetricQuery {
                actor: ActorScope::Player,
                id: id.into(),
            })
            .collect(),
    }
}
fn case(
    tree: &TreeDataSnapshot,
    class_id: u32,
    ascendancy_id: Option<&str>,
    paid: Option<u32>,
    skill: Skill,
) -> Case {
    let class = &tree.classes[&class_id];
    let ascendancy = ascendancy_id.map(|id| &tree.ascendancies[id]);
    let fixture = skill.fixture();
    let (old_class, old_id, old_internal_id) = match skill {
        Skill::Spark => ("Sorceress", 7, 7),
        Skill::Mace => ("Warrior", 3, 6),
    };
    let xml = fixture
        .replace(
            &format!("className=\"{old_class}\""),
            &format!("className=\"{}\"", class.name),
        )
        .replace(
            "ascendClassName=\"None\"",
            &format!(
                "ascendClassName=\"{}\"",
                ascendancy.map_or("None", |asc| asc.name.as_str())
            ),
        )
        .replace(
            &format!("classId=\"{old_id}\""),
            &format!("classId=\"{class_id}\""),
        )
        .replace(
            &format!("classInternalId=\"{old_internal_id}\""),
            &format!("classInternalId=\"{}\"", class.integer_id),
        )
        .replace(
            "ascendClassId=\"0\"",
            &format!(
                "ascendClassId=\"{}\" ascendancyInternalId=\"{}\"",
                ascendancy.map_or(0, |asc| asc.class_index),
                ascendancy_id.unwrap_or("")
            ),
        )
        .replace(
            "nodes=\"\"",
            &format!(
                "nodes=\"{}\"",
                paid.map_or(String::new(), |id| id.to_string())
            ),
        )
        .replace(
            "<Tree",
            "<!-- passive source annotation stays unchanged -->\n<Tree",
        )
        .replace(
            "<Notes>",
            "<Notes>Native passive parity: café &amp; identity choices. ",
        );
    let oracle_reimport = false;
    Case {
        label: format!(
            "{skill:?}/class{class_id}/{}/{}",
            ascendancy_id.unwrap_or("None"),
            paid.map_or("root".into(), |id| id.to_string())
        ),
        class_id,
        ascendancy_id: ascendancy_id.map(str::to_owned),
        paid,
        skill,
        xml,
        oracle_reimport,
    }
}
fn cases(tree: &TreeDataSnapshot) -> Vec<Case> {
    let mut values = Vec::new();
    for skill in [Skill::Spark, Skill::Mace] {
        let before = values.len();
        for (&id, class) in &tree.classes {
            values.push(case(tree, id, None, None, skill));
            for ascendancy in &class.ascendancy_ids {
                values.push(case(tree, id, Some(ascendancy), None, skill));
            }
        }
        assert_eq!(values.len() - before, 31);
        for &id in tree.classes.keys() {
            for node in tree.ordinary_entrances(id).unwrap() {
                values.push(case(tree, id, None, Some(node), skill));
            }
        }
        assert_eq!(values.len() - before, 47);
        assert!(tree.ordinary_entrances(1).unwrap().contains(&4739));
        assert!(tree.ordinary_entrances(8).unwrap().contains(&56651));
        let huntress_ascendancy = tree.classes[&8].ascendancy_ids.first().unwrap();
        let mut witch = case(tree, 1, Some("Witch3b"), Some(4739), skill);
        witch.oracle_reimport = skill == Skill::Spark;
        values.push(witch);
        let mut huntress = case(tree, 8, Some(huntress_ascendancy), Some(56651), skill);
        huntress.oracle_reimport = skill == Skill::Mace;
        values.push(huntress);
        assert_eq!(values.len() - before, 49);
    }
    values
}
fn map(result: &EvaluationResult) -> BTreeMap<&str, &MetricMeasurement> {
    result
        .measurements
        .iter()
        .map(|measurement| (measurement.query.id.as_str(), measurement))
        .collect()
}
fn close(label: &str, id: &str, actual: f64, expected: f64) {
    let tolerance = 1e-8_f64.max(expected.abs() * 1e-9);
    assert!(
        (actual - expected).abs() <= tolerance,
        "{label} {id}: native {actual}, PoB {expected}, tolerance {tolerance}"
    );
    let delta = (actual - expected).abs();
    // Finite nonnegative f64 bit ordering matches numeric ordering.
    MAX_ABSOLUTE_DELTA.fetch_max(delta.to_bits(), Ordering::Relaxed);
    MAX_SCALED_DELTA.fetch_max(
        (delta / expected.abs().max(1.0)).to_bits(),
        Ordering::Relaxed,
    );
}
fn compare(case: &Case, actual: &EvaluationResult, expected: &EvaluationResult) {
    actual.validate_recorded().unwrap();
    expected.validate_recorded().unwrap();
    let actual = map(actual);
    let expected = map(expected);
    assert_eq!(actual.len(), 9, "{}", case.label);
    assert_eq!(expected.len(), 9, "{}", case.label);
    for id in FINITE_METRICS {
        assert_eq!(actual[id].unit, expected[id].unit);
        assert_eq!(actual[id].schema_version, expected[id].schema_version);
        close(
            &case.label,
            id,
            actual[id].value.finite().unwrap(),
            expected[id].value.finite().unwrap(),
        );
    }
    if case.skill == Skill::Spark {
        close(
            &case.label,
            "selected_average_hit",
            actual["selected_average_hit"].value.finite().unwrap(),
            expected["selected_average_hit"].value.finite().unwrap(),
        );
    } else {
        for value in [
            &actual["selected_average_hit"].value,
            &expected["selected_average_hit"].value,
        ] {
            assert!(
                matches!(value, MeasurementValue::Unavailable { reason } if !reason.is_empty())
            );
        }
    }
}
fn check_passives(
    case: &Case,
    tree: &TreeDataSnapshot,
    native: &EvaluationResult,
    live: &PassiveCoverage,
) {
    live.validate().unwrap();
    let class = &tree.classes[&case.class_id];
    assert!(
        native.coverage.passives.is_none(),
        "Native projections must not fabricate observed Lua pointer evidence"
    );
    let attachment = native
        .attachments
        .iter()
        .find(|attachment| {
            attachment.media_type == "application/vnd.poe-optimizer.native-tree+json;version=3"
        })
        .expect("native class/tree projection diagnostic");
    let projection: serde_json::Value = serde_json::from_str(&attachment.content).unwrap();
    assert_eq!(projection["schema_version"], 3);
    assert_eq!(projection["evidence_kind"], "native_source_resolution");
    assert_eq!(projection["source"]["tree_version"], "0_5");
    assert_eq!(
        projection["source"]["upstream_revision"],
        tree.identity.upstream_revision
    );
    let digest = projection["source"]["bundled_content_sha256"]
        .as_str()
        .unwrap();
    assert_eq!(digest.len(), 64);
    assert!(digest.bytes().all(|byte| byte.is_ascii_hexdigit()));
    assert_eq!(projection["class"]["internal_id"], class.integer_id);
    assert_eq!(projection["class"]["index"], case.class_id);
    assert_eq!(projection["class"]["source_index"], class.source_index);
    assert_eq!(projection["class"]["name"], class.name);
    assert_eq!(projection["class"]["start_node_id"], class.start_node_id);
    assert_eq!(live.class.internal_id, class.integer_id);
    assert_eq!(live.class.index, case.class_id);
    assert_eq!(live.class.name, class.name);
    assert_eq!(
        projection["ordinary_allocated_count"],
        u32::from(case.paid.is_some())
    );
    assert_eq!(projection["point_budget_verified"], false);
    assert_eq!(
        live.allocation_counts.ordinary,
        u32::from(case.paid.is_some())
    );
    assert_eq!(live.allocation_counts.ascendancy, 0);
    assert!(live.secondary_ascendancy.is_none());
    let mut allocated = BTreeSet::from([class.start_node_id]);
    allocated.extend(case.paid);
    if let Some(id) = &case.ascendancy_id {
        let expected = &tree.ascendancies[id];
        let observed = live.ascendancy.as_ref().unwrap();
        assert_eq!(observed.internal_id.as_deref(), Some(id.as_str()));
        assert_eq!(
            observed.catalog_id.as_deref(),
            Some(expected.catalog_id.as_str())
        );
        assert_eq!(projection["ascendancy"]["internal_id"], id.as_str());
        assert_eq!(projection["ascendancy"]["catalog_id"], expected.catalog_id);
        assert_eq!(projection["ascendancy"]["name"], expected.name);
        assert_eq!(projection["ascendancy"]["index"], expected.class_index);
        assert_eq!(
            projection["ascendancy"]["start_node_id"],
            expected.start_node_id
        );
        allocated.insert(expected.start_node_id);
    } else {
        assert!(live.ascendancy.is_none());
        assert!(projection["ascendancy"].is_null());
    }
    let allocated: Vec<_> = allocated.into_iter().collect();
    assert_eq!(native.build.allocated_nodes, allocated);
    assert_eq!(projection["allocated_nodes"], serde_json::json!(allocated));
    assert_eq!(
        live.allocated_nodes
            .iter()
            .map(|node| node.physical_node_id)
            .collect::<Vec<_>>(),
        allocated
    );
    for observed in &live.allocated_nodes {
        let source = tree
            .effective_node(
                case.class_id,
                case.ascendancy_id.as_deref(),
                observed.physical_node_id,
            )
            .unwrap();
        assert_eq!(
            observed.stats, source.stats,
            "{} live/source stats",
            case.label
        );
        assert_eq!(
            observed.display_name, source.name,
            "{} live/source display name",
            case.label
        );
        assert_eq!(observed.name, tree.nodes[&observed.physical_node_id].name);
        assert_eq!(observed.allocation_mode, 0);
        if let Some(switch) = &observed.switch {
            assert_eq!(switch.selected_source_id, source.effective_source_id);
            assert!(switch.stats_reference_matches_selected_source);
            assert!(switch.display_name_matches_selected_source);
            let expected_kind = match &source.provenance {
                OverrideProvenance::Base => PassiveSwitchKind::Base,
                OverrideProvenance::Class { .. } => PassiveSwitchKind::Class,
                OverrideProvenance::Ascendancy { .. } => PassiveSwitchKind::Ascendancy,
            };
            assert_eq!(switch.kind, expected_kind);
        }
    }
    let paid = projection["paid_nodes"].as_array().unwrap();
    assert_eq!(paid.len(), usize::from(case.paid.is_some()));
    if let Some(id) = case.paid {
        let source = tree
            .effective_node(case.class_id, case.ascendancy_id.as_deref(), id)
            .unwrap();
        let observed = live
            .allocated_nodes
            .iter()
            .find(|node| node.physical_node_id == id)
            .unwrap();
        assert_eq!(paid[0]["physical_node_id"], id);
        assert_eq!(paid[0]["effective_node_id"], source.effective_source_id);
        assert_eq!(paid[0]["name"], observed.display_name);
        assert_eq!(paid[0]["stats"], serde_json::json!(observed.stats));
        assert_eq!(
            paid[0]["override_provenance"],
            serde_json::to_value(&source.provenance).unwrap()
        );
    }
}

fn verify(case: &Case, tree: &TreeDataSnapshot) {
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    let req = request(&case.xml);
    let actual = native
        .evaluate(&req, BUDGET)
        .unwrap_or_else(|error| panic!("{} native: {error}", case.label));
    let expected = oracle
        .evaluate(&req, BUDGET)
        .unwrap_or_else(|error| panic!("{} PoB: {error}", case.label));
    compare(case, &actual, &expected);
    assert!(actual.diagnostic_only && expected.diagnostic_only);
    assert_eq!(
        actual.backend.rules_revision,
        tree.identity.upstream_revision
    );
    assert_eq!(
        actual.backend.rules_revision,
        expected.backend.rules_revision
    );
    assert_eq!(actual.build.class_name, expected.build.class_name);
    assert_eq!(actual.build.ascendancy_name, expected.build.ascendancy_name);
    assert_eq!(actual.build.level, 60);
    assert_eq!(actual.build.level, expected.build.level);
    assert_eq!(actual.build.allocated_nodes, expected.build.allocated_nodes);
    assert_eq!(actual.context.enemy_level, expected.context.enemy_level);
    assert_eq!(actual.context.requested, req.options);
    assert_eq!(expected.context.requested, req.options);
    for (name, value) in &actual.context.config_inputs {
        assert_eq!(
            expected.context.config_inputs.get(name),
            Some(value),
            "{} config {name}",
            case.label
        );
    }
    check_passives(
        case,
        tree,
        &actual,
        expected.coverage.passives.as_ref().unwrap(),
    );
    let selected = actual.coverage.selected_player.as_ref().unwrap();
    assert_eq!(selected.skill_id.as_deref(), Some(case.skill.id()));
    assert_eq!(
        selected.skill_id,
        expected.coverage.selected_player.as_ref().unwrap().skill_id
    );
    assert_eq!(actual.coverage.groups[0].gems.len(), 1);
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
    let prepared = NativeBackend::new().prepare(&req).unwrap();
    let fields = match prepared.calculate().unwrap() {
        NativeCalculation::Spark(output) => {
            assert_eq!(case.skill, Skill::Spark);
            vec![
                ("Str", output.strength),
                ("Dex", output.dexterity),
                ("Int", output.intelligence),
                ("Armour", output.armour),
                ("Evasion", output.evasion),
                ("Speed", output.cast_rate),
                ("CritChance", output.crit_chance),
                ("CritMultiplier", output.crit_multiplier),
                ("AverageHit", output.average_hit),
            ]
        }
        NativeCalculation::Mace(output) => {
            assert_eq!(case.skill, Skill::Mace);
            let evasion = expected
                .context
                .config_inputs
                .get("enemyEvasion")
                .or_else(|| expected.context.config_placeholders.get("enemyEvasion"));
            assert!(matches!(evasion, Some(Scalar::Number(value)) if *value > 0.0));
            assert!(
                output.hit_chance < 100.0,
                "The matrix must exercise accuracy against nonzero enemy evasion"
            );
            if let Some(accuracy) = source.player.metrics.get("Accuracy") {
                close(&case.label, "Accuracy", output.accuracy, *accuracy);
            }
            vec![
                ("Str", output.strength),
                ("Dex", output.dexterity),
                ("Int", output.intelligence),
                ("Armour", output.armour),
                ("Evasion", output.evasion),
                ("Speed", output.attack_rate),
                ("CritChance", output.crit_chance),
                ("CritMultiplier", output.crit_multiplier),
                ("AverageDamage", output.average_damage),
                ("HitChance", output.hit_chance),
            ]
        }
    };
    for (id, actual) in fields {
        close(&case.label, id, actual, source.player.metrics[id]);
    }
    assert!(
        actual.exports[0].content == case.xml,
        "{} native export changed source bytes",
        case.label
    );
    let reimport = request(&actual.exports[0].content);
    let repeated = native.evaluate(&reimport, BUDGET).unwrap();
    compare(case, &repeated, &actual);
    assert_eq!(repeated.build.allocated_nodes, actual.build.allocated_nodes);
    assert_eq!(repeated.context.config_inputs, actual.context.config_inputs);
    check_passives(
        case,
        tree,
        &repeated,
        expected.coverage.passives.as_ref().unwrap(),
    );
    if case.oracle_reimport {
        let fresh = oracle.evaluate(&reimport, BUDGET).unwrap();
        compare(case, &fresh, &actual);
        check_passives(
            case,
            tree,
            &actual,
            fresh.coverage.passives.as_ref().unwrap(),
        );
    }
}

#[test]
fn identities_entrances_and_joint_ascendancy_allocations_match_100_fresh_pob_builds() {
    let tree = poe_optimizer_pob::tree_worker::extract_tree(
        &PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
        "0_5",
        Duration::from_secs(60),
    )
    .unwrap();
    assert_eq!(tree.classes.len(), 8);
    assert_eq!(tree.ascendancies.len(), 23);
    let cases = cases(&tree);
    assert_eq!(cases.len(), 98);
    assert_eq!(cases.iter().filter(|case| case.oracle_reimport).count(), 2);
    std::thread::scope(|scope| {
        let workers: Vec<_> = (0..2)
            .map(|worker| {
                let cases = &cases;
                let tree = &tree;
                scope.spawn(move || {
                    for case in cases
                        .iter()
                        .enumerate()
                        .filter(|(index, _)| index % 2 == worker)
                        .map(|(_, case)| case)
                    {
                        verify(case, tree);
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
    });
    eprintln!(
        "Validated 98 primary cases and 2 native-export PoB reimports with 2 workers; largest absolute numeric delta {:.3e}, largest delta scaled by max(1, abs(PoB)) {:.3e}",
        f64::from_bits(MAX_ABSOLUTE_DELTA.load(Ordering::Relaxed)),
        f64::from_bits(MAX_SCALED_DELTA.load(Ordering::Relaxed)),
    );
}
