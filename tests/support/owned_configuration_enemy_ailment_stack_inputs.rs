//! Three independently identified raw Enemy inputs; no ailment effect admission.
use crate::{configuration_sources as sources, configuration_stage as common};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const INPUTS: [&str; 3] = [
    "def.0000000000003391",
    "def.0000000000003392",
    "def.0000000000003393",
];
pub const NAMES: [&str; 3] = ["ShockStacks", "ScorchStacks", "ChillStacks"];
pub const UNIT: &str = "def.000000000000295a";
const SELECTORS: [&str; 3] = [
    "nonzero_input_then_placeholder",
    "input_then_placeholder",
    "nonzero_input_then_placeholder",
];
pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-enemy-ailment-stack-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
pub fn check_authored() {
    let inputs: Vec<ConfigurationDefaultInput> = read("input.json");
    let e: OwnedRecipeExtension = read("extension.json");
    let deps: Value = read("dependencies.json");
    let accounting: Vec<ConfigurationDispositionRule> = read("accounting.json");
    assert_eq!(inputs.len(), 3);
    assert_eq!(accounting.len(), 6);
    assert_eq!(
        (
            e.schema.len(),
            e.owners.len(),
            e.tables.len(),
            e.receivers.len()
        ),
        (4, 0, 0, 0)
    );
    assert_eq!(
        deps["before"],
        "1084b2615f8ec4b8d5960077b6dda429d5220dc2a27628f32b1121ec2a23cf05"
    );
    assert_eq!(deps["registry_last_issued"], 0x3390);
    assert_eq!(
        deps["unit_descriptor"]["value"]["schema"]["value"]["dimension"],
        "count"
    );
    let mut encounter = deps["encounter"].clone();
    for (index, input) in inputs.iter().enumerate() {
        assert_eq!(input.source_name, NAMES[index]);
        assert_eq!(input.value_input.key().as_str(), INPUTS[index]);
        assert_eq!(value(input.target), "enemy");
        assert_eq!(value(input.selection), SELECTORS[index]);
        assert!(!input.ignore_numeric_placeholder);
        let default = value(&input.constructor_default);
        assert_eq!(default["value"]["value"], 1.);
        assert_eq!(default["value"]["unit"]["key"], UNIT);
        assert_eq!(
            value(&input.recipe)["tiers"],
            json!([{"selectors":[{"lane":"input_number","name":NAMES[index]}],"duplicates":"reject"}])
        );
        let codec = value(&input.recipe)["codec"].clone();
        assert_eq!(
            codec["codec"]["value"]["scale"],
            json!({"numerator":1,"denominator":1})
        );
        assert_eq!(codec["codec"]["value"]["unit"]["key"], UNIT);
        encounter["value"]["schema"]["value"]["external_inputs"]["members"]
            .as_array_mut()
            .unwrap()
            .push(value(&input.value_input));
        let def = value(&e.schema[index + 1]);
        assert_eq!(def["value"]["value"]["id"], value(&input.value_input));
        assert_eq!(
            def["value"]["value"]["schema"]["value"]["targets"],
            json!(["enemy"])
        );
        let range = &def["value"]["value"]["schema"]["value"]["value"]["value"];
        for (side, bound) in [("minimum", -1_000_000.), ("maximum", 1_000_000.)] {
            assert_eq!(range[side]["value"], bound);
            assert_eq!(range[side]["unit"]["key"], UNIT);
        }
        for (offset, lane) in ["input_number", "placeholder_number"]
            .into_iter()
            .enumerate()
        {
            let row = &accounting[index * 2 + offset];
            assert_eq!(
                value(&row.selector),
                json!({"lane":lane,"name":NAMES[index]})
            );
            assert_eq!(
                value(&row.destinations),
                json!([{"kind":"adapter","control":{"kind":"scalar_input","input":input.value_input}}])
            );
        }
    }
    assert_eq!(value(&e.schema[0])["value"], encounter);
    check_source();
}
const ARCHIVES: &[&str] = &[
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Modules/CalcPerform.lua",
];
const SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        1793,
        1803,
        "scorch-condition-effect-and-stack-callback",
    ),
    (
        "src/Modules/ConfigOptions.lua",
        1808,
        1821,
        "chill-condition-effect-and-stack-callback",
    ),
    (
        "src/Modules/ConfigOptions.lua",
        1852,
        1861,
        "shock-condition-effect-and-stack-callback",
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
        "src/Modules/CalcPerform.lua",
        3247,
        3289,
        "chill-and-shock-consumer-boundary",
    ),
];
pub fn authenticate(e: &Value) {
    sources::authenticate(e, ARCHIVES, SPANS);
    assert_eq!(e["kind"], "configured-enemy-ailment-stack-inputs");
    assert_eq!(e["schema_version"], 1);
    assert_eq!(
        e["scope"],
        json!({"target":"enemy","raw_configured_amounts":true,"constructor":1,"scorch_preserves_input_zero":true,"shock_chill_skip_input_zero":true,"candidate_is_activated_effect":false,"native_effects":false,"coverage_completed":false,"new_source_execution":false,"mutation_source_observations":false,"admitted_range":[-1000000,1000000]})
    );
}
fn project(report: &Value) -> Value {
    let state = &report["state"];
    Value::Array(NAMES.iter().map(|name| json!({
        "source":name,
        "catalogue":state["configuration_catalogue"].as_array().unwrap().iter().filter(|r| r["fields"]["var"] == *name).collect::<Vec<_>>(),
        "constructors":state["configuration"].as_array().unwrap().iter().map(|r| json!({"input":r["input"].get(*name),"placeholder":r["placeholder"][*name],"default_state":r["default_state"][*name]})).collect::<Vec<_>>(),
        "calls":state["callback_invocations"].as_array().unwrap().iter().filter(|r| r["var"] == *name).collect::<Vec<_>>()
    })).collect())
}
pub fn validate_observation(observation: &Value) {
    let observations = observation.as_array().unwrap();
    assert_eq!(observations.len(), 3);
    for (index, o) in observations.iter().enumerate() {
        let name = NAMES[index];
        assert_eq!(o["source"], name);
        let kind = if index == 1 {
            "countAllowZero"
        } else {
            "count"
        };
        let first = [1859, 1801, 1819][index];
        let rows = o["catalogue"].as_array().unwrap();
        assert_eq!(rows.len(), 1);
        let row = &rows[0];
        assert_eq!(row["index"], [578, 562, 567][index]);
        assert_eq!(row["fields"]["var"], name);
        assert_eq!(row["fields"]["type"], kind);
        assert_eq!(
            row["fields"]["ifFlag"],
            ["ShockCanStack", "ScorchCanStack", "ChillCanStack"][index]
        );
        assert_eq!(
            row["fields"]["ifOption"],
            [
                "conditionEnemyShocked",
                "conditionEnemyScorched",
                "conditionEnemyChilled"
            ][index]
        );
        assert_eq!(row["default_placeholder"], 1);
        let body = json!({"execution_not_implied":true,"first_line":first,"last_line":first+2,"kind":"callback_reference","path":"@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua","what":"Lua"});
        assert_eq!(row["apply"], body);
        assert_eq!(
            o["constructors"],
            json!([{"input":null,"placeholder":1,"default_state":0},{"input":null,"placeholder":1,"default_state":0}])
        );
        let calls = o["calls"].as_array().unwrap();
        assert_eq!(calls.len(), 2);
        for call in calls {
            assert_eq!(call["var"], name);
            assert_eq!(call["kind"], kind);
            assert_eq!(call["body"], body);
            assert_eq!(call["argument_fields"], json!({"val":1}));
            let before = &call["before"];
            let mut after = call["after"].clone();
            let records = after["enemy_records"].as_array_mut().unwrap();
            let added = records.pop().unwrap();
            assert_eq!(
                added,
                json!({"position":records.len()+1,"body":{"_positions":[{"index":1,"value":{"type":"Condition","var":"Effective"}}],"flags":0,"keywordFlags":0,"name":format!("Multiplier:{name}"),"source":"Config","type":"BASE","value":1}})
            );
            assert!(
                before == &after,
                "callback changed more than its exact one Enemy record"
            );
        }
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
        "configured-enemy-ailment-stack-counts",
        3,
    )
}
