#![cfg(feature = "pob")]
//! Independent complete-build actor checks, including unchanged source XML reimport.
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
const SPARK_MAPPING: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const SPARK_BOSSING: &str = include_str!("fixtures/calibration/spark-bossing.xml");
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: ["life", "mana", "spirit", "selected_hit_dps"]
            .into_iter()
            .map(|id| MetricQuery {
                actor: ActorScope::Player,
                id: id.into(),
            })
            .collect(),
    }
}
fn oracle() -> Engine<PobBackend> {
    Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ))
}
fn near(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8_f64.max(expected.abs() * 1e-9),
        "{label}: native {actual} vs PoB {expected}"
    );
}
fn attachment(result: &EvaluationResult, needle: &str) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains(needle))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn compare(label: &str, native: &EvaluationResult, pob: &EvaluationResult, mace: bool) {
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
        near(
            &format!("{label}/{}", actual.query.id),
            actual.value.finite().unwrap(),
            expected.value.finite().unwrap(),
        );
    }
    let profile = attachment(native, "native-profile+");
    let snapshot = attachment(pob, "pob-snapshot+");
    let values = &profile["actor_resources"];
    for (key, source) in [
        ("strength", "Str"),
        ("dexterity", "Dex"),
        ("intelligence", "Int"),
        ("life", "Life"),
        ("mana", "Mana"),
        ("spirit", "Spirit"),
        ("lowest_attribute", "LowestAttribute"),
        ("total_attributes", "TotalAttr"),
        ("low_life_percentage", "LowLifePercentage"),
        ("full_life_percentage", "FullLifePercentage"),
        (
            "lowest_of_maximum_life_and_maximum_mana",
            "LowestOfMaximumLifeAndMaximumMana",
        ),
    ] {
        near(
            &format!("{label}/{key}"),
            values[key].as_f64().unwrap(),
            snapshot["player"]["metrics"][source]
                .as_f64()
                .unwrap_or_else(|| panic!("source missing {source}")),
        );
    }
    // Attack Accuracy lives in a nested hand output in this pinned PoB snapshot,
    // whose public attachment includes scalar player values only. The exact
    // accuracy stage is separately checked against actual CalcOffence source.
    if let Some(accuracy) = snapshot["player"]["metrics"]["Accuracy"].as_f64() {
        near(
            &format!("{label}/accuracy"),
            values["accuracy"].as_f64().unwrap(),
            accuracy,
        );
    }
    if mace {
        near(
            &format!("{label}/hit chance"),
            profile["hit_chance"].as_f64().unwrap(),
            snapshot["player"]["metrics"]["HitChance"].as_f64().unwrap(),
        );
    }
    for (name, actual) in &native.context.player_conditions {
        assert_eq!(
            *actual,
            pob.context
                .player_conditions
                .get(name)
                .copied()
                .unwrap_or(false),
            "{label}/{name}"
        );
    }
    assert_eq!(native.build.class_name, pob.build.class_name);
    assert_eq!(native.build.ascendancy_name, pob.build.ascendancy_name);
    assert_eq!(native.build.allocated_nodes, pob.build.allocated_nodes);
}
fn escaped(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn block(template: &str, text: &str) -> String {
    template.replace("</ConfigSet>",&format!("<CustomModifierBlock title=\"Actor study\" enabled=\"true\">{}</CustomModifierBlock></ConfigSet>",escaped(text)))
}
fn legacy(template: &str, text: &str) -> String {
    template.replace(
        "</ConfigSet>",
        &format!(
            "<Input name=\"customMods\" string=\"{}\"/></ConfigSet>",
            escaped(text)
        ),
    )
}
fn cases() -> Vec<(&'static str, String)> {
    let mut cases = vec![("baseline", String::new())];
    for operation in ["base", "increased", "more", "reduced", "less"] {
        let mut lines = Vec::new();
        for stat in [
            "Strength",
            "Dexterity",
            "Intelligence",
            "maximum Life",
            "maximum Mana",
            "Spirit",
            "Accuracy Rating",
        ] {
            lines.push(if operation == "base" {
                format!("+11 to {stat}")
            } else {
                format!("13% {operation} {stat}")
            });
        }
        cases.push((operation, lines.join("\n")));
    }
    for (name, text) in [
        ("no-bonuses", "Gain no inherent bonuses from attributes"),
        ("no-strength", "Gain no inherent bonuses from strength"),
        ("no-dexterity", "Gain no inherent bonuses from dexterity"),
        (
            "no-intelligence",
            "Gain no inherent bonuses from intelligence",
        ),
        (
            "half-strength",
            "Inherent life granted by strength is halved",
        ),
        (
            "double-bonuses",
            "Inherent bonuses gained from attributes are doubled",
        ),
        (
            "no-strength-life",
            "Strength provides no inherent bonus to maximum life",
        ),
        (
            "no-intelligence-mana",
            "Intelligence provides no inherent bonus to maximum mana",
        ),
        (
            "zero-mana-spirit",
            "+11 to maximum Mana\n50% more maximum Mana\nRemoves all mana\n+11 to Spirit\n50% increased Spirit\nRemoves all spirit",
        ),
        (
            "zero-dexterity-override",
            "Dexterity's accuracy bonus instead grants +0 to accuracy rating per dexterity",
        ),
        (
            "condition-composition",
            "+10 to Strength\n+10 to Dexterity\n+10 to Intelligence\n+20 to Dexterity if Strength is higher than Intelligence\n13% more maximum Life if Dexterity is higher than Intelligence\n20% increased Accuracy Rating if Strength is higher than Intelligence",
        ),
    ] {
        cases.push((name, text.into()));
    }
    cases
}
#[test]
fn actor_operations_and_inherent_flags_match_fresh_spark_mapping_bossing_and_composed_mace() {
    let native = Engine::new(NativeBackend::new());
    let pob = oracle();
    let example: Value =
        serde_json::from_str(include_str!("../examples/mace-local-weapon-search.json")).unwrap();
    let weapons: Vec<MaceWeaponAlternative> =
        serde_json::from_value(example["weapons"].clone()).unwrap();
    let support =
        MaceSupportLoadout::new(vec!["heavy_swing".into(), "rapid_attacks_i".into()]).unwrap();
    let tree = ClassTreeSelection {
        class_id: 10,
        ascendancy_id: Some("Monk3".into()),
        entrance_node_id: Some(10364),
        ascendancy_node_id: Some(24475),
    };
    let registry = ControlledMaceCatalog::with_tree_loadouts(
        Arc::new(bundled_snapshot().unwrap()),
        MACE.into(),
        weapons,
        vec![support.clone()],
        vec![tree.clone()],
    )
    .unwrap();
    let candidate = registry
        .resolve_tree_loadout_candidate(&tree, "wooden-balanced", &support)
        .unwrap();
    let composed = registry.materialize(candidate).unwrap().content;
    let mut comparisons = 0;
    let mut reimports = 0;
    for (scenario, template, is_mace) in [
        ("mapping", SPARK_MAPPING, false),
        ("bossing", SPARK_BOSSING, false),
        ("mace", composed.as_str(), true),
    ] {
        for (name, text) in cases() {
            let xml = if text.is_empty() {
                template.into()
            } else {
                block(template, &text)
            };
            let actual = native
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|error| panic!("{scenario}/{name} native: {error}"));
            let expected = pob
                .evaluate(&request(&xml), BUDGET)
                .unwrap_or_else(|error| panic!("{scenario}/{name} PoB: {error}"));
            compare(&format!("{scenario}/{name}"), &actual, &expected, is_mace);
            assert_eq!(actual.exports[0].content, xml);
            comparisons += 1;
            if ["more", "zero-mana-spirit", "condition-composition"].contains(&name) {
                let reimport = pob
                    .evaluate(&request(&actual.exports[0].content), BUDGET)
                    .unwrap();
                compare(
                    &format!("{scenario}/{name}/reimport"),
                    &actual,
                    &reimport,
                    is_mace,
                );
                reimports += 1;
            }
        }
    }
    assert_eq!(comparisons, 51);
    assert_eq!(reimports, 9);
    eprintln!(
        "Actor matrix: {comparisons} fresh complete build pairs, {reimports} exact native-export PoB reimports"
    );
}
#[test]
fn legacy_multiline_blocks_disabled_sources_two_pass_conditions_and_removal_match_pob() {
    let native = Engine::new(NativeBackend::new());
    let pob = oracle();
    let mut compared = 0;
    for newline in ["\n", "\r\n"] {
        // Literal XML attribute whitespace is intentionally preserved by PoB's
        // parser, unlike standard XML attribute normalization.
        let text = format!(
            "\t+20 to Strength{newline}\t-30 to Strength if Strength is higher than Intelligence{newline}\t+11 to maximum Life"
        );
        let mut results = Vec::new();
        for xml in [legacy(SPARK_MAPPING, &text), block(SPARK_MAPPING, &text)] {
            let actual = native.evaluate(&request(&xml), BUDGET).unwrap();
            let expected = pob.evaluate(&request(&xml), BUDGET).unwrap();
            compare(
                "literal whitespace and exactly two passes",
                &actual,
                &expected,
                false,
            );
            assert_eq!(
                attachment(&actual, "native-profile+")["actor_resources"]["strength"],
                0.0
            );
            assert_eq!(actual.exports[0].content, xml);
            let reimport = pob
                .evaluate(&request(&actual.exports[0].content), BUDGET)
                .unwrap();
            compare("literal whitespace reimport", &actual, &reimport, false);
            results.push(actual);
            compared += 1;
        }
        assert_eq!(
            serde_json::to_value(&results[0].measurements).unwrap(),
            serde_json::to_value(&results[1].measurements).unwrap()
        );
    }
    let disabled=SPARK_MAPPING.replace("</ConfigSet>","<CustomModifierBlock title=\"Ignored unknown\" enabled=\"false\">Unimplemented alien mechanic</CustomModifierBlock></ConfigSet>");
    let boosted = block(SPARK_MAPPING, "+20 to maximum Life\n50% more Spirit");
    for (name, xml) in [
        ("disabled unknown", disabled.as_str()),
        ("effect present", boosted.as_str()),
        ("effect removed", SPARK_MAPPING),
    ] {
        let actual = native.evaluate(&request(xml), BUDGET).unwrap();
        let expected = pob.evaluate(&request(xml), BUDGET).unwrap();
        compare(name, &actual, &expected, false);
        assert_eq!(actual.exports[0].content, xml);
        compared += 1;
        if name == "disabled unknown" {
            assert_eq!(
                attachment(&actual, "native-profile+")["actor_modifiers"]["blocks"][0]["text"],
                "Unimplemented alien mechanic"
            );
        }
    }
    let data = bundled_snapshot().unwrap();
    for selected in [[false, false, false], [true, false, true]] {
        let inputs = data
            .package()
            .actor
            .spirit_quests
            .iter()
            .zip(selected)
            .map(|(quest, enabled)| {
                format!(
                    "<Input name=\"{}\" boolean=\"{enabled}\"/>",
                    quest.config_key
                )
            })
            .collect::<String>();
        let xml = SPARK_MAPPING.replace("</ConfigSet>", &format!("{inputs}</ConfigSet>"));
        let actual = native.evaluate(&request(&xml), BUDGET).unwrap();
        let expected = pob.evaluate(&request(&xml), BUDGET).unwrap();
        compare("selected Spirit quest records", &actual, &expected, false);
        compared += 1;
    }
    assert_eq!(compared, 9);
    eprintln!(
        "Actor source configuration: {compared} fresh complete build pairs, 4 exact literal-whitespace native-export PoB reimports"
    );
}
