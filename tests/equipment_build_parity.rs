#![cfg(feature = "pob")]
//! Fresh full-build evidence for source-identified weapon/global accessory assembly.
use poe_optimizer_core::{
    evaluation::*,
    metrics::{ActorScope, MetricQuery},
    options::EvaluationOptions,
};
use poe_optimizer_native::NativeBackend;
use poe_optimizer_pob::backend::PobBackend;
use serde_json::Value;
use std::path::PathBuf;
const MACE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
const SPARK_MAPPING: &str = include_str!("fixtures/calibration/spark-mapping.xml");
const SPARK_BOSSING: &str = include_str!("fixtures/calibration/spark-bossing.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 60_000 };
const WEAPON_ID: u32 = 11;
const AMULET_ID: u32 = 73;
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
fn profile(result: &EvaluationResult) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains("native-profile+"))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn source(result: &EvaluationResult) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains("pob-snapshot+"))
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
fn compare(label: &str, native: &EvaluationResult, pob: &EvaluationResult, mace: bool) {
    native.validate_recorded().unwrap();
    pob.validate_recorded().unwrap();
    assert!(native.diagnostic_only && pob.diagnostic_only);
    for actual in &native.measurements {
        let expected = pob
            .measurements
            .iter()
            .find(|measurement| measurement.query == actual.query)
            .unwrap();
        assert_eq!(actual.unit, expected.unit);
        near(
            &format!("{label}/{}", actual.query.id),
            actual.value.finite().unwrap(),
            expected.value.finite().unwrap(),
        );
    }
    let native_profile = profile(native);
    let source = source(pob);
    for (key, source_key) in [
        ("strength", "Str"),
        ("dexterity", "Dex"),
        ("intelligence", "Int"),
        ("life", "Life"),
        ("mana", "Mana"),
        ("spirit", "Spirit"),
        ("lowest_attribute", "LowestAttribute"),
        ("total_attributes", "TotalAttr"),
    ] {
        near(
            &format!("{label}/{key}"),
            native_profile["actor_resources"][key].as_f64().unwrap(),
            source["player"]["metrics"][source_key].as_f64().unwrap(),
        );
    }
    if mace {
        // Accuracy itself is nested per hand, outside the flat source snapshot.
        for (key, source_key) in [
            ("hit_chance", "HitChance"),
            ("attack_rate", "Speed"),
            ("average_damage", "AverageDamage"),
            ("crit_chance", "CritChance"),
        ] {
            near(
                &format!("{label}/{key}"),
                native_profile[key].as_f64().unwrap(),
                source["player"]["metrics"][source_key].as_f64().unwrap(),
            );
        }
    }
    for (name, value) in &native.context.player_conditions {
        assert_eq!(
            *value,
            pob.context
                .player_conditions
                .get(name)
                .copied()
                .unwrap_or(false),
            "{label}/{name}"
        );
    }
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn weapon(extra: &str) -> String {
    format!(
        "Rarity: RARE\nStudy Club\nWooden Club\nItem Level: 1\nQuality: 20\nImplicits: 0\nAdds 2 to 5 Physical Damage\n50% increased Physical Damage\n20% increased Attack Speed\n100% increased Critical Hit Chance\n{extra}"
    )
}
fn amulet(base: &str, implicit: &str, extra: &str, level_requirement: Option<u32>) -> String {
    let requirement = level_requirement
        .map(|value| format!("LevelReq: {value}\n"))
        .unwrap_or_default();
    format!(
        "Rarity: RARE\nStudy Pendant\n{base}\nItem Level: 1\nQuality: 0\n{requirement}Implicits: 1\n{implicit}\n{extra}"
    )
}
fn equip(template: &str, weapon: Option<&str>, amulet: Option<&str>) -> String {
    let mut section = String::from("<Items activeItemSet=\"1\">");
    if let Some(weapon) = weapon {
        section.push_str(&format!(
            "<Item id=\"{WEAPON_ID}\">{}</Item>",
            escape(weapon)
        ));
    }
    if let Some(amulet) = amulet {
        section.push_str(&format!(
            "<Item id=\"{AMULET_ID}\">{}</Item>",
            escape(amulet)
        ));
    }
    section.push_str("<ItemSet id=\"1\" title=\"Equipment study\" useSecondWeaponSet=\"false\">");
    if weapon.is_some() {
        section.push_str(&format!("<Slot name=\"Weapon 1\" itemId=\"{WEAPON_ID}\"/>"));
    }
    if amulet.is_some() {
        section.push_str(&format!("<Slot name=\"Amulet\" itemId=\"{AMULET_ID}\"/>"));
    }
    section.push_str("</ItemSet></Items>");
    let start = template.find("<Items ").unwrap();
    let end = template.find("</Items>").unwrap() + "</Items>".len();
    let mut xml = template.to_owned();
    xml.replace_range(start..end, &section);
    xml
}
fn verify_item_sources(
    label: &str,
    result: &EvaluationResult,
    mace: bool,
    amulet_base: Option<&str>,
) {
    let profile = profile(result);
    if mace {
        let item = &profile["equipment"]["Weapon 1"];
        assert_eq!(item["pob_item_id"], WEAPON_ID);
        assert_eq!(item["base_name"], "Wooden Club");
        assert_eq!(item["affix_legality_verified"], false);
        for modifier in item["actor_modifiers"].as_array().unwrap() {
            assert_eq!(
                modifier["source"],
                format!("Item:{WEAPON_ID}:Study Club, Wooden Club"),
                "{label}"
            );
        }
    }
    if let Some(base) = amulet_base {
        let item = &profile["equipment"]["Amulet"];
        assert_eq!(item["pob_item_id"], AMULET_ID);
        assert_eq!(item["base_name"], base);
        assert_eq!(item["affix_legality_verified"], false);
        for modifier in item["actor_modifiers"].as_array().unwrap() {
            assert_eq!(
                modifier["source"],
                format!("Item:{AMULET_ID}:Study Pendant, {base}"),
                "{label}"
            );
        }
        assert_eq!(item["modifier_lines"][0]["implicit"], true);
    }
}
fn pair(
    label: &str,
    xml: &str,
    native: &Engine<NativeBackend>,
    oracle: &Engine<PobBackend>,
    mace: bool,
    amulet_base: Option<&str>,
    reimport: bool,
) -> EvaluationResult {
    let actual = native
        .evaluate(&request(xml), BUDGET)
        .unwrap_or_else(|error| panic!("{label} native: {error}"));
    let expected = oracle
        .evaluate(&request(xml), BUDGET)
        .unwrap_or_else(|error| panic!("{label} PoB: {error}"));
    compare(label, &actual, &expected, mace);
    verify_item_sources(label, &actual, mace, amulet_base);
    assert_eq!(actual.exports[0].content, xml);
    if reimport {
        let exported = &actual.exports[0].content;
        let native_again = native.evaluate(&request(exported), BUDGET).unwrap();
        let pob_again = oracle.evaluate(&request(exported), BUDGET).unwrap();
        compare(
            &format!("{label}/reimport"),
            &native_again,
            &pob_again,
            mace,
        );
        compare(
            &format!("{label}/original-source"),
            &actual,
            &pob_again,
            mace,
        );
        verify_item_sources(label, &native_again, mace, amulet_base);
        assert_eq!(native_again.exports[0].content, xml);
        assert_eq!(
            profile(&native_again)["equipment"],
            profile(&actual)["equipment"]
        );
    }
    actual
}
#[test]
fn selected_item_local_global_and_implicit_effects_match_complete_fresh_builds() {
    let native = Engine::new(NativeBackend::new());
    let oracle = Engine::new(PobBackend::new(
        PathBuf::from(env!("CARGO_BIN_EXE_poe-optimizer")),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"),
    ));
    let bases = [
        ("Amber Amulet", "+12 to Strength"),
        ("Jade Amulet", "+14 to Dexterity"),
        ("Lapis Amulet", "+10 to Intelligence"),
        ("Bloodstone Amulet", "+35 to maximum Life"),
        ("Solar Amulet", "+15 to Spirit"),
    ];
    let weapon = weapon(
        "+20 to Strength\n+10 to Dexterity\n+5 to Intelligence\n+21 to maximum Life\n+19 to maximum Mana\n+17 to Spirit",
    );
    let extra = "+7 to Strength\n+9 to Dexterity\n+11 to Intelligence\n+40 to maximum Life\n+35 to maximum Mana\n+13 to Spirit\n+20 to Accuracy Rating";
    let mitigated = MACE
        .replace(
            "enemyArmour\" number=\"0\"",
            "enemyArmour\" number=\"125.25\"",
        )
        .replace(
            "enemyFireResist\" number=\"0\"",
            "enemyFireResist\" number=\"-37.5\"",
        );
    let mut comparisons = 0;
    let mut reimports = 0;
    for (scenario, template, mace) in [
        ("spark-mapping", SPARK_MAPPING, false),
        ("spark-bossing", SPARK_BOSSING, false),
        ("mace", MACE, true),
        ("mace-mitigated", mitigated.as_str(), true),
    ] {
        for (base, implicit) in bases {
            let amulet = amulet(base, implicit, extra, None);
            let xml = equip(template, mace.then_some(weapon.as_str()), Some(&amulet));
            let reimport =
                scenario == "spark-mapping" || (scenario == "mace" && base == "Solar Amulet");
            pair(
                &format!("{scenario}/{base}"),
                &xml,
                &native,
                &oracle,
                mace,
                Some(base),
                reimport,
            );
            comparisons += 1;
            reimports += usize::from(reimport);
        }
    }
    for condition_true in [false, true] {
        let text = if condition_true {
            "+20 to Strength\n+30 to Accuracy Rating if Strength is higher than Intelligence"
        } else {
            "+30 to Intelligence\n+30 to Accuracy Rating if Strength is higher than Intelligence"
        };
        let item = crate::weapon(text);
        let xml = equip(MACE, Some(&item), None);
        let actual = pair(
            "weapon condition-tagged Accuracy",
            &xml,
            &native,
            &oracle,
            true,
            None,
            condition_true,
        );
        assert_eq!(
            actual.context.player_conditions["StrHigherThanInt"],
            condition_true
        );
        let profile = profile(&actual);
        let accuracy = profile["equipment"]["Weapon 1"]["actor_modifiers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|record| record["stat"] == "accuracy")
            .unwrap();
        assert!(!accuracy["tags"].as_array().unwrap().is_empty());
        comparisons += 1;
        reimports += usize::from(condition_true);
    }
    // Diagnostic evaluation does not enforce equipment eligibility; the catalog
    // does. These two source comparisons preserve an explicit equip threshold
    // while proving that it is independent of authored item level and actor math.
    for level in [59, 60] {
        let item = amulet(
            "Amber Amulet",
            "+12 to Strength",
            "+40 to maximum Mana",
            Some(60),
        );
        let xml = equip(
            &SPARK_MAPPING.replace("<Build level=\"60\"", &format!("<Build level=\"{level}\"")),
            None,
            Some(&item),
        );
        let actual = pair(
            "explicit equipment threshold",
            &xml,
            &native,
            &oracle,
            false,
            Some("Amber Amulet"),
            false,
        );
        assert_eq!(actual.build.level, level);
        assert_eq!(
            profile(&actual)["equipment"]["Amulet"]["requirements"]["level"],
            60
        );
        assert_eq!(profile(&actual)["equipment"]["Amulet"]["item_level"], 1);
        comparisons += 1;
    }
    assert_eq!(comparisons, 24);
    assert_eq!(reimports, 7);
    eprintln!(
        "Equipment source matrix: {comparisons} fresh complete native/PoB build pairs, {reimports} exact native exports reimported through both backends"
    );
}
