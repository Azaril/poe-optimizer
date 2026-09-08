#![cfg(feature = "pob")]
//! Fresh complete PoB builds validate body equipment and movement including actual source conditions.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use serde_json::Value;
use std::path::PathBuf;
const SPARK: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: [
            "life",
            "mana",
            "spirit",
            "armour",
            "evasion",
            "movement_speed_pct",
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
fn attachment(result: &EvaluationResult, media: &str) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains(media))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn near(label: &str, actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-8_f64.max(expected.abs() * 1e-9),
        "{label}: native {actual} vs PoB {expected}"
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
    let profile = attachment(native, "native-profile+");
    let source = attachment(pob, "pob-snapshot+");
    let receiver = &profile["receiving_defence"];
    assert_eq!(receiver["schema_version"], 1);
    for (field, key) in [
        ("armour", "Armour"),
        ("evasion", "Evasion"),
        ("energy_shield", "EnergyShield"),
    ] {
        // These source outputs have already undergone their final integer rounding.
        assert_eq!(
            receiver[field].as_f64().unwrap(),
            source["player"]["metrics"][key].as_f64().unwrap(),
            "{label}/{key}"
        );
    }
    for (field, key) in [
        ("fire", "Fire"),
        ("cold", "Cold"),
        ("lightning", "Lightning"),
        ("chaos", "Chaos"),
    ] {
        for (name, suffix) in [
            ("resistances", "Resist"),
            ("resistance_totals", "ResistTotal"),
        ] {
            assert_eq!(
                receiver[name][field].as_f64().unwrap(),
                source["player"]["metrics"][format!("{key}{suffix}")]
                    .as_f64()
                    .unwrap(),
                "{label}/{name}/{field}"
            );
        }
    }
    for (field, key) in [
        ("strength", "Str"),
        ("dexterity", "Dex"),
        ("intelligence", "Int"),
        ("life", "Life"),
        ("mana", "Mana"),
        ("spirit", "Spirit"),
    ] {
        near(
            &format!("{label}/{key}"),
            profile["actor_resources"][field].as_f64().unwrap(),
            source["player"]["metrics"][key].as_f64().unwrap(),
        );
    }
    let movement = &profile["movement"];
    assert_eq!(movement["schema_version"], 1);
    for (field, key) in [
        ("movement_speed_mod", "MovementSpeedMod"),
        ("action_speed_mod", "ActionSpeedMod"),
        ("effective_movement_speed_mod", "EffectiveMovementSpeedMod"),
    ] {
        near(
            &format!("{label}/{key}"),
            movement[field].as_f64().unwrap(),
            source["player"]["metrics"][key].as_f64().unwrap(),
        );
    }
    let local = &profile["local_armour"];
    assert_eq!(local["schema_version"], 2);
    for (slot, item) in local["items"].as_object().unwrap() {
        for (field, raw) in [
            ("armour", "Armour"),
            ("evasion", "Evasion"),
            ("energy_shield", "EnergyShield"),
        ] {
            let rating = item[field].as_f64().unwrap();
            if rating > 0.0 {
                assert_eq!(
                    source["player"]["metrics"][format!("{raw}On{slot}")].as_f64(),
                    Some(rating),
                    "{label}/{raw}On{slot}"
                );
            }
        }
    }
    assert_eq!(
        native.build.allocated_nodes, pob.build.allocated_nodes,
        "{label}"
    );
    assert_eq!(native.build.class_name, pob.build.class_name);
    assert_eq!(native.build.ascendancy_name, pob.build.ascendancy_name);
    for (key, value) in &native.context.player_conditions {
        assert_eq!(
            *value,
            pob.context
                .player_conditions
                .get(key)
                .copied()
                .unwrap_or(false),
            "{label}/{key}"
        );
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn block(xml: &str, text: &str) -> String {
    xml.replace("</ConfigSet>",&format!("<CustomModifierBlock title=\"Receiver\" enabled=\"true\">{}</CustomModifierBlock></ConfigSet>",escape(text)))
}
fn pair(
    label: &str,
    xml: &str,
    native: &Engine<NativeBackend>,
    oracle: &Engine<PobBackend>,
    reimport: bool,
) -> EvaluationResult {
    let actual = native
        .evaluate(&request(xml), BUDGET)
        .unwrap_or_else(|e| panic!("{label} native: {e}"));
    let expected = oracle
        .evaluate(&request(xml), BUDGET)
        .unwrap_or_else(|e| panic!("{label} PoB: {e}"));
    compare(label, &actual, &expected);
    assert_eq!(actual.exports[0].content, xml);
    if reimport {
        let again = native
            .evaluate(&request(&actual.exports[0].content), BUDGET)
            .unwrap();
        let fresh = oracle
            .evaluate(&request(&actual.exports[0].content), BUDGET)
            .unwrap();
        compare(&format!("{label}/reimport"), &again, &fresh);
        compare(&format!("{label}/retained"), &actual, &fresh);
        assert_eq!(
            attachment(&again, "native-profile+"),
            attachment(&actual, "native-profile+")
        );
        assert_eq!(again.exports[0].content, xml);
    }
    actual
}
fn engines() -> (Engine<NativeBackend>, Engine<PobBackend>) {
    (
        Engine::new(NativeBackend::new()),
        Engine::new(PobBackend::new(
            PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
        )),
    )
}

fn armour_item(slot: &str, base: &str, quality: u32, text: &str, xml: &str) -> String {
    let id = match slot {
        "Helmet" => 41,
        "Gloves" => 42,
        "Boots" => 43,
        "Body Armour" => 44,
        _ => panic!("test slot"),
    };
    let item = format!(
        "Rarity: RARE\nLocal Armour Study\n{base}\nItem Level: 60\nQuality: {quality}\nImplicits: 0\n{text}"
    );
    xml.replace(
        "<ItemSet id=\"1\" title=\"No equipment\"/>",
        "<ItemSet id=\"1\" title=\"No equipment\"></ItemSet>",
    )
    .replace(
        "<ItemSet id=\"1\"",
        &format!(
            "<Item id=\"{id}\">{}</Item><ItemSet id=\"1\"",
            escape(&item)
        ),
    )
    .replace(
        "</ItemSet>",
        &format!("<Slot name=\"{slot}\" itemId=\"{id}\"/></ItemSet>"),
    )
}

#[test]
fn body_penalties_local_quality_and_all_movement_operations_match_full_pob() {
    let (native, oracle) = engines();
    let cases = [
        (
            "armour",
            "Rusted Cuirass",
            20,
            "+31 to Armour\n25% increased Armour",
            "",
        ),
        (
            "evasion",
            "Leather Vest",
            13,
            "+17.5 to Evasion Rating\n27% increased Evasion Rating",
            "",
        ),
        (
            "energy-shield",
            "Tattered Robe",
            17,
            "+21 to maximum Energy Shield\n35% increased Energy Shield",
            "",
        ),
        (
            "movement-inc-more",
            "Rusted Cuirass",
            20,
            "13% increased Movement Speed",
            "17% increased Movement Speed\n11% more Movement Speed\n7% less Movement Speed",
        ),
        (
            "fractional-base",
            "Rusted Cuirass",
            0,
            "17% increased Movement Speed",
            "+0.1235 to Movement Speed",
        ),
        (
            "mixed-case-item-format",
            "Leather Vest",
            13,
            "+17.5 to evasion rating\n27% INCREASED EVASION RATING",
            "",
        ),
        (
            "mixed-case-config",
            "Rusted Cuirass",
            20,
            "15% INCREASED MOVEMENT SPEED",
            "IGNORE ALL MOVEMENT PENALTIES FROM ARMOUR",
        ),
        (
            "override-division",
            "Rusted Cuirass",
            0,
            "",
            "YOUR MOVEMENT SPEED IS 57% OF ITS BASE VALUE",
        ),
        (
            "negative-speed",
            "Leather Vest",
            0,
            "150% reduced Movement Speed",
            "",
        ),
        (
            "floor",
            "Rusted Cuirass",
            0,
            "150% reduced Movement Speed",
            "Movement Speed cannot be modified to below base value",
        ),
        (
            "ignore",
            "Rusted Cuirass",
            20,
            "15% increased Movement Speed",
            "Ignore all movement penalties from armour",
        ),
        (
            "override-31",
            "Rusted Cuirass",
            0,
            "",
            "Your Movement Speed is 31% of its base value",
        ),
        (
            "override-117",
            "Rusted Cuirass",
            0,
            "17% increased Movement Speed",
            "Your Movement Speed is 117% of its base value",
        ),
        (
            "zero-override",
            "Rusted Cuirass",
            0,
            "",
            "Your Movement Speed is 0% of its base value",
        ),
        (
            "override-floor",
            "Rusted Cuirass",
            0,
            "",
            "Your Movement Speed is 31% of its base value\nMovement Speed cannot be modified to below base value",
        ),
    ];
    for (skill, template) in [("spark", SPARK), ("mace", MACE)] {
        for (name, base, quality, mods, config) in cases {
            let xml = armour_item("Body Armour", base, quality, mods, template);
            let xml = if config.is_empty() {
                xml
            } else {
                block(&xml, config)
            };
            let result = pair(
                &format!("{skill}/{name}"),
                &xml,
                &native,
                &oracle,
                matches!(name, "ignore" | "override-31" | "evasion"),
            );
            let profile = attachment(&result, "native-profile+");
            assert_eq!(
                profile["local_armour"]["items"].as_object().unwrap().len(),
                1
            );
            assert_eq!(
                profile["local_armour"]["items"]["Body Armour"]["quality"],
                quality
            );
        }
    }
}

#[test]
fn four_slots_source_globals_and_body_removal_preserve_exact_full_build_parity() {
    let (native, oracle) = engines();
    for (name, template) in [
        (
            "spark",
            include_str!("fixtures/builds/spark-body-armour.xml"),
        ),
        ("mace", include_str!("fixtures/builds/mace-body-armour.xml")),
    ] {
        let actual = pair(name, template, &native, &oracle, true);
        assert_eq!(
            attachment(&actual, "native-profile+")["local_armour"]["items"]
                .as_object()
                .unwrap()
                .len(),
            4
        );
        let formatted = template
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .replace(
                "</Notes>",
                " Formatting preserved.</Notes><!-- movement parity -->",
            );
        pair(
            &format!("{name}/format"),
            &formatted,
            &native,
            &oracle,
            true,
        );
        let removed = template.replace("<Slot name=\"Body Armour\" itemId=\"44\"/>", "");
        let result = pair(
            &format!("{name}/remove-body"),
            &removed,
            &native,
            &oracle,
            false,
        );
        assert_eq!(
            attachment(&result, "native-profile+")["local_armour"]["items"]
                .as_object()
                .unwrap()
                .len(),
            3
        );
        // Both the generated penalty and the source global increase must disappear.
        let normal = actual
            .measurements
            .iter()
            .find(|m| m.query.id == "movement_speed_pct")
            .unwrap()
            .value
            .finite()
            .unwrap();
        let without = result
            .measurements
            .iter()
            .find(|m| m.query.id == "movement_speed_pct")
            .unwrap()
            .value
            .finite()
            .unwrap();
        assert_ne!(normal, without);
        pair(
            &format!("{name}/global-order"),
            &block(
                template,
                "+101 to Dexterity\n11% more Movement Speed\n7% less Movement Speed\n17% increased Movement Speed if Dexterity is higher than Intelligence",
            ),
            &native,
            &oracle,
            false,
        );
        let mut all_removed = template.to_owned();
        for (slot, id) in [
            ("Helmet", 41),
            ("Body Armour", 44),
            ("Gloves", 42),
            ("Boots", 43),
        ] {
            all_removed =
                all_removed.replace(&format!("<Slot name=\"{slot}\" itemId=\"{id}\"/>"), "");
        }
        let none = pair(
            &format!("{name}/all-removed"),
            &all_removed,
            &native,
            &oracle,
            false,
        );
        near(
            "unmodified baseline",
            none.measurements
                .iter()
                .find(|m| m.query.id == "movement_speed_pct")
                .unwrap()
                .value
                .finite()
                .unwrap(),
            100.0,
        );
    }
}

#[test]
fn special_movement_phrases_reject_unparsed_condition_suffixes() {
    let native = Engine::new(NativeBackend::new());
    for template in [SPARK, MACE] {
        for line in [
            "Ignore all movement penalties from armour if Dexterity is higher than Intelligence",
            "Movement Speed cannot be modified to below base value if Dexterity is higher than Intelligence",
            "Your Movement Speed is 117% of its base value if Dexterity is higher than Intelligence",
        ] {
            let xml = block(
                &armour_item("Body Armour", "Rusted Cuirass", 20, "", template),
                line,
            );
            assert!(native.evaluate(&request(&xml), BUDGET).is_err(), "{line}");
        }
    }
}
