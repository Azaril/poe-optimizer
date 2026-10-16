//! One typed scenario input; later enemy speed/action-speed consumers stay open.
use crate::configuration_sources as sources;

use crate::configuration_stage as common;
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const INPUT: &str = "def.0000000000003370";
pub const UNIT: &str = "def.000000000000336f";
const KIND: &str = "configured-enemy-attack-cast-interval";
pub fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn directory() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/configuration-enemy-speed-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
pub fn input() -> ConfigurationDefaultInput {
    read("input.json")
}
pub fn accounting() -> Vec<ConfigurationDispositionRule> {
    read("accounting.json")
}

pub fn check_authored() {
    let i = input();
    assert_eq!(i.source_name, "enemySpeed");
    assert_eq!(i.value_input.key().as_str(), INPUT);
    assert_eq!(value(i.target), "enemy");
    assert_eq!(value(&i.constructor_default)["value"]["value"], 700.0);
    assert_eq!(value(&i.constructor_default)["value"]["unit"]["key"], UNIT);
    assert!(i.ignore_numeric_placeholder);
    assert_eq!(
        value(&i.recipe)["tiers"],
        json!([{"selectors":[{"lane":"input_number","name":"enemySpeed"}],"duplicates":"reject"}])
    );
    assert_eq!(
        value(&i.recipe)["codec"]["codec"]["value"]["scale"],
        json!({"numerator":1,"denominator":1})
    );
    let e: OwnedRecipeExtension = read("extension.json");
    assert_eq!(
        (
            e.schema.len(),
            e.owners.len(),
            e.tables.len(),
            e.receivers.len()
        ),
        (3, 0, 0, 0)
    );
    let d: Value = read("dependencies.json");
    let mut expected = d["encounter"].clone();
    expected["value"]["schema"]["value"]["external_inputs"]["members"]
        .as_array_mut()
        .unwrap()
        .push(value(&i.value_input));
    assert_eq!(value(&e.schema[0])["value"], expected);
    assert_eq!(
        value(&e.schema[1])["value"]["value"]["schema"]["value"]["dimension"],
        "time"
    );
    assert_eq!(
        value(&e.schema[2])["value"]["value"]["schema"]["value"]["targets"],
        json!(["scenario_enemy_actor"])
    );
    let rows = accounting();
    assert_eq!(rows.len(), 2);
    for (row, lane) in rows.iter().zip(["input_number", "placeholder_number"]) {
        assert_eq!(
            value(&row.selector),
            json!({"lane":lane,"name":"enemySpeed"})
        );
        assert_eq!(
            value(&row.destinations),
            json!([{"kind":"adapter","control":{"kind":"scalar_input","input":i.value_input}}])
        );
    }
    assert_eq!(
        d["encounter_policy"]["absent_input_names"],
        json!(["enemyIsBoss", "presetBossSkills", "enemySizePreset"])
    );
    check_source();
}

const REQUIRED_SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        1982,
        2139,
        "unconditional-placeholder-reset",
    ),
    (
        "src/Modules/ConfigOptions.lua",
        2189,
        2268,
        "boss-speed-replacement",
    ),
    (
        "src/Modules/ConfigOptions.lua",
        2279,
        2279,
        "millisecond-default-declaration",
    ),
    (
        "src/Modules/CalcDefence.lua",
        3409,
        3416,
        "input-placeholder-default-and-millisecond-consumer",
    ),
    (
        "src/Classes/ConfigTab.lua",
        315,
        324,
        "numeric-placeholder-notification",
    ),
    (
        "src/Classes/EditControl.lua",
        110,
        115,
        "placeholder-change-callback",
    ),
];
pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        sources::check(
            &read::<Value>("source-evidence.json"),
            sources::CONFIGURATION_ARCHIVES,
            REQUIRED_SPANS,
            "enemySpeed",
            700.,
        )
    });
}

pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    common::stage(prior, &directory(), KIND, 2)
}
