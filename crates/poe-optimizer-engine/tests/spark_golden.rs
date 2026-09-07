//! Native-only data loading; the production Spark pipeline has no dependencies.
#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_engine::spark::{self, SparkInput, SparkQuestRewards};
use sha2::{Digest, Sha256};

#[test]
fn closed_spark_pipeline_matches_immutable_independent_calibration_goldens() {
    for (xml, json, resistance) in [
        (
            include_str!("../../../tests/fixtures/calibration/spark-mapping.xml"),
            include_str!("../../../tests/fixtures/calibration/spark-mapping.reference.json"),
            0.0,
        ),
        (
            include_str!("../../../tests/fixtures/calibration/spark-bossing.xml"),
            include_str!("../../../tests/fixtures/calibration/spark-bossing.reference.json"),
            50.0,
        ),
    ] {
        let golden: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(xml.replace("\r\n", "\n"))),
            golden["provenance"]["fixture_sha256"].as_str().unwrap()
        );
        assert_eq!(
            golden["provenance"]["upstream_revision"],
            poe_optimizer_engine::UPSTREAM_REVISION
        );
        assert_eq!(golden["method"], "bundled-dll-independent-host-direct-main");
        let output = spark::evaluate(&SparkInput {
            character_level: 60,
            resistance_penalty: -60.0,
            enemy_lightning_resistance: resistance,
            quests: SparkQuestRewards::default(),
        })
        .unwrap();
        for (name, actual) in [
            ("Life", output.life),
            ("Mana", output.mana),
            ("EnergyShield", output.energy_shield),
            ("Str", output.strength),
            ("Dex", output.dexterity),
            ("Int", output.intelligence),
            ("FireResist", output.fire_resistance),
            ("ColdResist", output.cold_resistance),
            ("LightningResist", output.lightning_resistance),
            ("ChaosResist", output.chaos_resistance),
            ("AverageHit", output.average_hit),
            ("TotalDPS", output.hit_dps),
            ("Speed", output.cast_rate),
            ("CritChance", output.crit_chance),
            ("CritMultiplier", output.crit_multiplier),
        ] {
            let expected = golden["metrics"][name].as_f64().unwrap();
            let tolerance = 1e-8 + expected.abs() * 1e-9;
            assert!(
                (actual - expected).abs() <= tolerance,
                "{name}: native {actual} vs independent golden {expected}"
            );
        }
    }
}
