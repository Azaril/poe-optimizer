//! Raw Enemy critical-weakness input backed by original constructor and callback evidence.
use crate::{configuration_sources as sources, configuration_stage as common};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const INPUT: &str = "def.0000000000003390";
pub const UNIT: &str = "def.000000000000295a";
pub const SOURCE: &str = "enemyCriticalWeaknessStacks";
pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-enemy-critical-weakness-input")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
pub fn check_authored() {
    let input: ConfigurationDefaultInput = read("input.json");
    assert_eq!(input.source_name, SOURCE);
    assert_eq!(input.value_input.key().as_str(), INPUT);
    assert_eq!(value(input.target), "enemy");
    assert_eq!(value(input.selection), "nonzero_input_then_placeholder");
    assert!(!input.ignore_numeric_placeholder);
    assert_eq!(value(&input.constructor_default)["value"]["value"], 20.);
    assert_eq!(
        value(&input.constructor_default)["value"]["unit"]["key"],
        UNIT
    );
    assert_eq!(
        value(&input.recipe)["tiers"],
        json!([{"selectors":[{"lane":"input_number","name":SOURCE}],"duplicates":"reject"}])
    );
    let codec = value(&input.recipe)["codec"].clone();
    assert_eq!(
        codec["codec"]["value"]["scale"],
        json!({"numerator":1,"denominator":1})
    );
    assert_eq!(codec["codec"]["value"]["unit"]["key"], UNIT);
    let e: OwnedRecipeExtension = read("extension.json");
    assert_eq!(
        (
            e.schema.len(),
            e.owners.len(),
            e.tables.len(),
            e.receivers.len()
        ),
        (2, 0, 0, 0)
    );
    let deps: Value = read("dependencies.json");
    assert_eq!(
        deps["before"],
        "e1c2abd299aead48ff1da38aa517c4fcbcaacd2671f754deb921364c0cad4b39"
    );
    assert_eq!(deps["registry_last_issued"], 0x338f);
    assert_eq!(
        deps["unit_descriptor"]["value"]["schema"]["value"]["dimension"],
        "count"
    );
    let mut encounter = deps["encounter"].clone();
    encounter["value"]["schema"]["value"]["external_inputs"]["members"]
        .as_array_mut()
        .unwrap()
        .push(value(&input.value_input));
    assert_eq!(value(&e.schema[0])["value"], encounter);
    let def = value(&e.schema[1]);
    assert_eq!(def["value"]["value"]["id"], value(&input.value_input));
    assert_eq!(
        def["value"]["value"]["schema"]["value"]["targets"],
        json!(["scenario_enemy_actor"])
    );
    let range = &def["value"]["value"]["schema"]["value"]["value"]["value"];
    for (side, bound) in [("minimum", -1_000_000.), ("maximum", 1_000_000.)] {
        assert_eq!(range[side]["value"], bound);
        assert_eq!(range[side]["unit"]["key"], UNIT);
    }
    let rows: Vec<ConfigurationDispositionRule> = read("accounting.json");
    assert_eq!(rows.len(), 2);
    for (row, lane) in rows.iter().zip(["input_number", "placeholder_number"]) {
        assert_eq!(value(&row.selector), json!({"lane":lane,"name":SOURCE}));
        assert_eq!(
            value(&row.destinations),
            json!([{"kind":"adapter","control":{"kind":"scalar_input","input":input.value_input}}])
        );
    }
    check_source();
}
const ARCHIVES: &[&str] = &[
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Modules/CalcOffence.lua",
];
const SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        1908,
        1913,
        "enemy-critical-weakness-control-and-callback",
    ),
    (
        "src/Classes/ConfigTab.lua",
        907,
        913,
        "saved-placeholder-loading",
    ),
    (
        "src/Classes/ConfigTab.lua",
        1174,
        1200,
        "numeric-callback-dispatch",
    ),
    (
        "src/Classes/ConfigTab.lua",
        1317,
        1333,
        "fresh-placeholder-constructor",
    ),
    (
        "src/Modules/CalcOffence.lua",
        3876,
        3885,
        "enemy-critical-chance-consumer-boundary",
    ),
];
pub fn authenticate(e: &Value) {
    sources::authenticate(e, ARCHIVES, SPANS);
    assert_eq!(e["kind"], "configured-enemy-critical-weakness-input");
    assert_eq!(e["schema_version"], 1);
    assert_eq!(
        e["scope"],
        json!({"target":"enemy","raw_configured_amount":true,"constructor":20,"count_allows_zero":false,"zero_candidate_is_emission":false,"native_effect":false,"coverage_completed":false,"new_source_execution":false,"mutation_source_observations":false,"admitted_range":[-1000000,1000000]})
    );
    assert_eq!(
        e["spans"][0]["sha256"],
        "50dd5c882592f38491c088ecc9ec34c1337498f70aa3a83fcba881807a1b2216"
    );
}
fn project(report: &Value) -> Value {
    let state = &report["state"];
    json!({
        "catalogue":state["configuration_catalogue"].as_array().unwrap().iter().filter(|r| r["fields"]["var"] == SOURCE).collect::<Vec<_>>(),
        "constructors":state["configuration"].as_array().unwrap().iter().map(|r| json!({"input":r["input"].get(SOURCE),"placeholder":r["placeholder"][SOURCE],"default_state":r["default_state"][SOURCE]})).collect::<Vec<_>>(),
        "calls":state["callback_invocations"].as_array().unwrap().iter().filter(|r| r["var"] == SOURCE).collect::<Vec<_>>()
    })
}
pub fn validate_observation(o: &Value) {
    let rows = o["catalogue"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!(row["index"], 594);
    assert_eq!(row["fields"]["var"], SOURCE);
    assert_eq!(row["fields"]["type"], "count");
    assert_eq!(row["fields"]["ifOption"], "conditionEnemyCriticalWeakness");
    assert_eq!(row["default_placeholder"], 20);
    let body = json!({"execution_not_implied":true,"first_line":1911,"last_line":1913,"kind":"callback_reference","path":"@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua","what":"Lua"});
    assert_eq!(row["apply"], body);
    assert_eq!(
        o["constructors"],
        json!([{"input":null,"placeholder":20,"default_state":0},{"input":null,"placeholder":20,"default_state":0}])
    );
    let calls = o["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    for call in calls {
        assert_eq!(call["var"], SOURCE);
        assert_eq!(call["kind"], "count");
        assert_eq!(call["body"], body);
        assert_eq!(call["argument_fields"], json!({"val":20}));
        let before = &call["before"];
        let mut after = call["after"].clone();
        let records = after["enemy_records"].as_array_mut().unwrap();
        let added = records.pop().unwrap();
        assert_eq!(
            added,
            json!({"position":records.len()+1,"body":{"_positions":[{"index":1,"value":{"type":"Condition","var":"ApplyCriticalWeakness"}}],"flags":0,"keywordFlags":0,"name":"SelfCritChance","source":"Critical Weakness","type":"BASE","value":10}})
        );
        // Every other Config field and both other existing record inventories
        // are unchanged. The callback observation is not an effect evaluation.
        assert!(
            before == &after,
            "Critical Weakness callback changed more than its one Enemy record"
        );
    }
}
pub fn observation() -> Value {
    project(&sources::acquisition().jit_off)
}
pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| {
        authenticate(&read("source-evidence.json"));
        let acquisition = sources::acquisition();
        for report in [&acquisition.jit_off, &acquisition.jit_on] {
            validate_observation(&project(report));
        }
    });
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    common::stage(
        prior,
        &directory(),
        "configured-enemy-critical-weakness-count",
        1,
    )
}
