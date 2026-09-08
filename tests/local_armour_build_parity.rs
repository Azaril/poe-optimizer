#![cfg(feature = "pob")]
//! Independent complete PoB builds validate local armour, receiving ratings and exact exports.
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
    let local = &profile["local_armour"];
    assert_eq!(local["schema_version"], 1);
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
fn fixed_slots_local_pairs_quality_and_signed_values_match_full_pob_builds() {
    let (native, oracle) = engines();
    let cases = [
        (
            "armour-quality",
            "Helmet",
            "Rusted Greathelm",
            20,
            "+31 to Armour\n35% increased Armour",
        ),
        (
            "evasion-quality",
            "Gloves",
            "Suede Bracers",
            13,
            "+17.5 to Evasion Rating\n27% increased Evasion Rating",
        ),
        (
            "es-local-global",
            "Boots",
            "Straw Sandals",
            20,
            "+23 to maximum Energy Shield\n40% increased Energy Shield\n15% increased maximum Energy Shield\n+7 to Global Energy Shield",
        ),
        (
            "armour-evasion-pair",
            "Helmet",
            "Brimmed Helm",
            12,
            "+11 to Armour and Evasion\n23% increased Armour and Evasion",
        ),
        (
            "armour-es-pair",
            "Helmet",
            "Iron Crown",
            17,
            "+11 to Armour and Energy Shield\n23% increased Armour and Energy Shield",
        ),
        (
            "mixed-pairs",
            "Gloves",
            "Torn Gloves",
            20,
            "+11 to Armour and Energy Shield\n+13 to Evasion Rating and Energy Shield\n17% increased Armour and Energy Shield\n19% increased Evasion Rating and Energy Shield\n23% increased Defences",
        ),
        (
            "negative-local",
            "Boots",
            "Rough Greaves",
            20,
            "-51 to Armour\n150% reduced Armour",
        ),
        (
            "negative-received",
            "Helmet",
            "Rusted Greathelm",
            20,
            "150% reduced Armour\n+7 to Global Armour\n13% increased Global Armour",
        ),
        (
            "conditional-global",
            "Boots",
            "Rawhide Boots",
            10,
            "+31 to Evasion Rating\n+100 to Dexterity\n17% increased Evasion Rating if Dexterity is higher than Intelligence\n+13 to Armour if Dexterity is higher than Intelligence",
        ),
    ];
    for (skill, template) in [("spark", SPARK), ("mace", MACE)] {
        for (name, slot, base, quality, text) in cases {
            let xml = armour_item(slot, base, quality, text, template);
            let result = pair(
                &format!("{skill}/{name}"),
                &xml,
                &native,
                &oracle,
                name == "es-local-global" || name == "negative-received",
            );
            let profile = attachment(&result, "native-profile+");
            assert_eq!(profile["local_armour"]["schema_version"], 1);
            assert_eq!(
                profile["local_armour"]["items"].as_object().unwrap().len(),
                1
            );
            assert_eq!(profile["local_armour"]["items"][slot]["quality"], quality);
        }
    }
}

#[test]
fn all_slots_compose_with_global_gear_config_passives_and_round_trip_exactly() {
    let (native, oracle) = engines();
    for (name, template) in [
        (
            "spark",
            include_str!("fixtures/builds/spark-local-armour.xml"),
        ),
        (
            "mace",
            include_str!("fixtures/builds/mace-local-armour.xml"),
        ),
    ] {
        let actual = pair(name, template, &native, &oracle, true);
        let profile = attachment(&actual, "native-profile+");
        assert_eq!(
            profile["local_armour"]["items"].as_object().unwrap().len(),
            3
        );
        let conditioned = block(
            template,
            "+101 to Dexterity\n17% increased Armour if Dexterity is higher than Intelligence\n23% increased Evasion Rating\n11% increased maximum Energy Shield",
        );
        pair(
            &format!("{name}/global-composition"),
            &conditioned,
            &native,
            &oracle,
            false,
        );
        let formatted = template
            .replace("\r\n", "\n")
            .replace('\n', "\r\n")
            .replace(
                "</Notes>",
                " Formatting preserved.</Notes><!-- armour parity -->",
            );
        pair(
            &format!("{name}/formatting"),
            &formatted,
            &native,
            &oracle,
            true,
        );
        let mut removed = template.to_owned();
        for slot in ["Helmet", "Gloves", "Boots"] {
            let id = match slot {
                "Helmet" => 41,
                "Gloves" => 42,
                _ => 43,
            };
            removed = removed.replace(&format!("<Slot name=\"{slot}\" itemId=\"{id}\"/>"), "");
        }
        let result = pair(
            &format!("{name}/removed"),
            &removed,
            &native,
            &oracle,
            false,
        );
        assert!(
            attachment(&result, "native-profile+")["local_armour"]["items"]
                .as_object()
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn item_value_formatting_precedes_local_and_global_consumers_but_not_configuration() {
    let (native, oracle) = engines();
    let globals = "+17.5 to Strength\n+31.5 to maximum Life\n+7.5 to Global Armour\n+13.5% to Fire Resistance";
    for (skill, template) in [("spark", SPARK), ("mace", MACE)] {
        let mut xml = template.replace(
            "<ItemSet id=\"1\" title=\"No equipment\"/>",
            "<ItemSet id=\"1\" title=\"No equipment\"></ItemSet>",
        );
        let item = format!(
            "Rarity: RARE\nFractional Pendant\nLunar Amulet\nItem Level: 60\nQuality: 0\nImplicits: 1\n+25.5 to maximum Energy Shield\n{globals}"
        );
        xml = xml
            .replace(
                "<ItemSet id=\"1\"",
                &format!("<Item id=\"29\">{}</Item><ItemSet id=\"1\"", escape(&item)),
            )
            .replace(
                "</ItemSet>",
                "<Slot name=\"Amulet\" itemId=\"29\"/></ItemSet>",
            );
        pair(
            &format!("{skill}/item-format-amulet"),
            &xml,
            &native,
            &oracle,
            true,
        );
        pair(
            &format!("{skill}/item-format-config"),
            &block(&xml, globals),
            &native,
            &oracle,
            false,
        );
        let armour = armour_item(
            "Helmet",
            "Rusted Greathelm",
            13,
            &format!("+17.5 to Armour\n{globals}"),
            &xml,
        );
        pair(
            &format!("{skill}/item-format-composition"),
            &armour,
            &native,
            &oracle,
            false,
        );
    }
    let xml = MACE.replace("Implicits: 0", &format!("Implicits: 0\n{globals}"));
    pair("mace/item-format-weapon", &xml, &native, &oracle, true);
}
