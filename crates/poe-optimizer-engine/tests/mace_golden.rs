//! Unchanged independent full-build outputs, never used by production evaluation.
#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_engine::{
    mace::{self, MaceInput, MaceWeapon},
    spark::SparkQuestRewards,
};
use sha2::{Digest, Sha256};

#[test]
fn mace_pipeline_matches_four_immutable_independent_attack_goldens() {
    for (xml, json, weapon, brutality) in [
        (
            include_str!("../../../tests/fixtures/calibration/mace-wooden.xml"),
            include_str!("../../../tests/fixtures/calibration/mace-wooden.reference.json"),
            MaceWeapon::WoodenClub,
            false,
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-smithing.xml"),
            include_str!("../../../tests/fixtures/calibration/mace-smithing.reference.json"),
            MaceWeapon::SmithingHammer,
            false,
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-wooden-brutality.xml"),
            include_str!(
                "../../../tests/fixtures/calibration/mace-wooden-brutality.reference.json"
            ),
            MaceWeapon::WoodenClub,
            true,
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-smithing-brutality.xml"),
            include_str!(
                "../../../tests/fixtures/calibration/mace-smithing-brutality.reference.json"
            ),
            MaceWeapon::SmithingHammer,
            true,
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
        assert_eq!(
            golden["method"],
            "bundled-dll-independent-host-direct-main-attack-v1"
        );
        let output = mace::evaluate(&MaceInput {
            character_level: 60,
            weapon,
            quality: 0,
            item_level: 1,
            brutality,
            resistance_penalty: -60.0,
            quests: SparkQuestRewards::default(),
            enemy_armour: 0.0,
            enemy_evasion: mace::monster_evasion(60).unwrap(),
            enemy_fire_resistance: 0.0,
        })
        .unwrap();
        for (name, actual) in [
            ("Str", output.strength),
            ("Dex", output.dexterity),
            ("Int", output.intelligence),
            ("Life", output.life),
            ("Mana", output.mana),
            ("EnergyShield", output.energy_shield),
            ("FireResist", output.fire_resistance),
            ("ColdResist", output.cold_resistance),
            ("LightningResist", output.lightning_resistance),
            ("ChaosResist", output.chaos_resistance),
            ("HitChance", output.hit_chance),
            ("AverageDamage", output.average_damage),
            ("TotalDPS", output.hit_dps),
            ("Speed", output.attack_rate),
            ("CritChance", output.crit_chance),
            ("CritMultiplier", output.crit_multiplier),
        ] {
            let expected = golden["metrics"][name].as_f64().unwrap();
            assert!(
                (actual - expected).abs() <= 1e-8 + 1e-9 * expected.abs(),
                "{name}: native {actual} vs independent {expected}"
            );
        }
        for (name, actual) in [
            ("Accuracy", output.accuracy),
            ("AverageHit", output.main_hand_average_hit),
            ("PhysicalHitAverage", output.physical_hit_average),
            ("FireHitAverage", output.fire_hit_average),
        ] {
            let expected = golden["attack"]["main_hand"][name].as_f64().unwrap();
            assert!(
                (actual - expected).abs() <= 1e-8 + 1e-9 * expected.abs(),
                "main-hand {name}: native {actual} vs independent {expected}"
            );
        }
        for (name, actual) in [
            ("physical_min", output.weapon_physical_minimum),
            ("physical_max", output.weapon_physical_maximum),
            ("fire_min", output.weapon_fire_minimum),
            ("fire_max", output.weapon_fire_maximum),
        ] {
            assert_eq!(actual, golden["attack"][name].as_f64().unwrap());
        }
        assert!(
            golden["metrics"].get("AverageHit").is_none(),
            "Per-hand AverageHit must not be mistaken for a top-level attack metric"
        );
    }
}
