#![cfg(feature = "pob")]
//! Independent full PoB builds exercise source composition through the shared receiver.
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
fn equipped(template: &str, weapon_extra: &str, amulet: Option<(&str, &str, &str)>) -> String {
    let mace = template.contains("Melee1HMacePlayer");
    let mut items = String::from("<Items activeItemSet=\"1\">");
    if mace {
        items.push_str(&format!("<Item id=\"17\">{}</Item>",escape(&format!("Rarity: RARE\nReceiver Club\nWooden Club\nItem Level: 60\nQuality: 20\nImplicits: 0\nAdds 5 to 10 Physical Damage\n{weapon_extra}"))));
    }
    if let Some((base, implicit, extra)) = amulet {
        items.push_str(&format!("<Item id=\"29\">{}</Item>",escape(&format!("Rarity: RARE\nReceiver Pendant\n{base}\nItem Level: 60\nQuality: 0\nImplicits: 1\n{implicit}\n{extra}"))));
    }
    items.push_str("<ItemSet id=\"1\" title=\"Receiving\" useSecondWeaponSet=\"false\">");
    if mace {
        items.push_str("<Slot name=\"Weapon 1\" itemId=\"17\"/>");
    }
    if amulet.is_some() {
        items.push_str("<Slot name=\"Amulet\" itemId=\"29\"/>");
    }
    items.push_str("</ItemSet></Items>");
    let mut xml = template.to_owned();
    let start = xml.find("<Items ").unwrap();
    let end = xml.find("</Items>").unwrap() + "</Items>".len();
    xml.replace_range(start..end, &items);
    xml
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
#[test]
fn source_global_groups_signed_rounding_and_conditions_match_full_builds() {
    let (native, oracle) = engines();
    let cases = [
        (
            "grouped",
            "+101 to Armour\n+51 to Evasion Rating\n+23 to maximum Energy Shield\n15% increased Defences\n20% increased Armour and Evasion\n+13 to Armour and Evasion\n+7 to global Evasion Rating and Energy Shield",
        ),
        (
            "fractional",
            "+0.5 to Armour\n+1.5 to Evasion Rating\n+2.5 to maximum Energy Shield\n+60.75% to Fire Resistance\n+4.25% to all Elemental Resistances\n10% increased all Resistances",
        ),
        (
            "negative",
            "-10 to Armour\n150% reduced Armour\n-7 to Evasion Rating\n-5 to maximum Energy Shield\n+100% to Fire Resistance\n150% reduced Fire Resistance\n-210% to Cold Resistance",
        ),
        (
            "conditional",
            "+100 to Dexterity\n+31 to Armour if Dexterity is higher than Intelligence\n20% increased maximum Energy Shield if Dexterity is higher than Intelligence\n+19 to maximum Energy Shield\n+80% to all Elemental Resistances if Strength is higher than Intelligence\n25% increased all Resistances",
        ),
        (
            "all-and-pairs",
            "+17% to all Resistances\n+63% to all Elemental Resistances\n+11% to Fire and Chaos Resistances\n+7% to Cold and Lightning Resistances\n20% increased Fire and Cold Resistances\n10% reduced Lightning and Chaos Resistances",
        ),
        (
            "global-marker",
            "+37 to Global Armour\n13% increased Global Armour\n+41 to Global Evasion Rating\n19% increased Global Evasion Rating\n+43 to Global Energy Shield\n23% increased maximum Energy Shield",
        ),
    ];
    for (skill, template) in [("spark", SPARK), ("mace", MACE)] {
        for (name, text) in cases {
            pair(
                &format!("{skill}/{name}"),
                &block(template, text),
                &native,
                &oracle,
                name == "global-marker",
            );
        }
    }
}
#[test]
fn selected_gear_passives_removal_and_reimports_preserve_full_receivers() {
    let (native, oracle) = engines();
    let receiving = "+51 to Armour\n+23 to Evasion Rating\n+17 to maximum Energy Shield\n20% increased Defences\n+37% to Fire and Chaos Resistances\n15% increased all Elemental Resistances";
    for (skill, template) in [("spark", SPARK), ("mace", MACE)] {
        let bare = equipped(template, "", None);
        let config = block(&bare, receiving);
        pair(
            &format!("{skill}/configuration"),
            &config,
            &native,
            &oracle,
            false,
        );
        let lunar = equipped(
            template,
            "",
            Some(("Lunar Amulet", "+25 to maximum Energy Shield", receiving)),
        );
        pair(&format!("{skill}/lunar"), &lunar, &native, &oracle, true);
        let pearl = equipped(
            template,
            if skill == "mace" { receiving } else { "" },
            Some((
                "Pearlescent Amulet",
                "+9% to all Elemental Resistances",
                "+11 to Global Armour\n23% increased maximum Energy Shield\n+7% to all Resistances",
            )),
        );
        let split = block(&pearl, receiving);
        pair(&format!("{skill}/split"), &split, &native, &oracle, true);
        pair(
            &format!("{skill}/duplicated"),
            &block(&split, receiving),
            &native,
            &oracle,
            false,
        );
        pair(&format!("{skill}/removed"), &bare, &native, &oracle, false);
        if skill == "mace" {
            pair(
                "mace/weapon-only",
                &equipped(template, receiving, None),
                &native,
                &oracle,
                false,
            );
        }
    }
    for (name, template) in [
        (
            "spark/passive-composition",
            include_str!("fixtures/builds/spark-receiving-defence.xml"),
        ),
        (
            "mace/passive-composition",
            include_str!("fixtures/builds/mace-receiving-defence.xml"),
        ),
    ] {
        pair(name, template, &native, &oracle, true);
        let mut crlf = template.replace("\r\n", "\n").replace('\n', "\r\n");
        crlf = crlf.replace(
            "</Notes>",
            " Source formatting retained.</Notes><!-- receiver parity -->",
        );
        pair(
            &format!("{name}/source-format"),
            &crlf,
            &native,
            &oracle,
            true,
        );
    }
}
