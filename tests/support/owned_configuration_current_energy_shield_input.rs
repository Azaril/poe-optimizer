//! Raw Player current-resource percentage preserving both Config callback responsibilities.
use crate::{configuration_sources as sources, configuration_stage as common};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const INPUT: &str = "def.0000000000003399";
pub const UNIT: &str = "def.0000000000000002";
pub const SOURCE: &str = "multiplierCurrentEnergyShield";
pub fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("data/owned/poe2/3887ae68/configuration-current-energy-shield-input")
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
    assert_eq!(value(input.target), "player");
    assert_eq!(value(input.selection), "nonzero_input_then_placeholder");
    assert!(!input.ignore_numeric_placeholder);
    assert_eq!(value(&input.constructor_default)["value"]["value"], 100.);
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
        "3b3e135ec63ce4abb92105efe56ad1bb7cb802759f8473070a11811f2d1a6e7d"
    );
    assert_eq!(deps["registry_last_issued"], 0x3398);
    assert_eq!(
        deps["unit_descriptor"]["value"]["schema"]["value"]["dimension"],
        "percentage_points"
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
        json!(["player_actor", "owned_actor"])
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
    "src/Modules/ModParser.lua",
];
const SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        157,
        160,
        "current-energy-shield-both-callback-outputs",
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
        "src/Modules/ModParser.lua",
        3321,
        3330,
        "player-current-resource-downstream-consumer-boundary",
    ),
];
pub fn authenticate(e: &Value) {
    sources::authenticate(e, ARCHIVES, SPANS);
    assert_eq!(e["kind"], "configured-player-current-energy-shield-input");
    assert_eq!(e["schema_version"], 1);
    assert_eq!(
        e["scope"],
        json!({"target":"player","raw_configured_percentage":true,"constructor":100,"count_allows_zero":false,"zero_candidate_is_emission":false,"native_flag":false,"native_effect":false,"global_condition_absence":false,"coverage_completed":false,"new_source_execution":false,"mutation_source_observations":false,"admitted_range":[-1000000,1000000]})
    );
    assert_eq!(
        e["derivation"],
        json!({
            "authority":"exact-original-callback-and-count-dispatch",
            "source_scope":"this-config-callback-only",
            "nonzero":[
                {"name":"Condition:UseCurrentEnergyShield","type":"FLAG","value":true,"source":"Config","flags":0,"keywordFlags":0},
                {"name":"Multiplier:CurrentEnergyShield","type":"BASE","value":"raw-candidate","source":"Config","flags":0,"keywordFlags":0}
            ],
            "zero":[],"missing_saved_lanes":"reviewed-constructor-100",
            "malformed_saved_lane":"pending-no-input","native_outputs_emitted":false,
            "global_condition_false_inferred":false,"downstream_150_limit_applied":false
        })
    );
}
/// Offline source derivation, not a native writer or global-condition reduction.
/// The pinned count dispatch skips zero; an invoked callback has precisely the
/// following two unconditional statements. Missing/unvalidated candidates do
/// not establish an empty source inventory. Other flag suppliers remain open.
pub fn source_outputs(candidate: Option<f64>) -> Option<Value> {
    let candidate = candidate?;
    if !candidate.is_finite() || !(-1_000_000. ..=1_000_000.).contains(&candidate) {
        return None;
    }
    if candidate == 0. {
        return Some(json!([]));
    }
    Some(json!([
        {"name":"Condition:UseCurrentEnergyShield","type":"FLAG","value":true,"source":"Config","flags":0,"keywordFlags":0},
        {"name":"Multiplier:CurrentEnergyShield","type":"BASE","value":candidate,"source":"Config","flags":0,"keywordFlags":0}
    ]))
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
    assert_eq!(row["index"], 14);
    assert_eq!(row["fields"]["var"], SOURCE);
    assert_eq!(row["fields"]["type"], "count");
    assert_eq!(row["fields"]["ifCond"], "UseCurrentEnergyShield");
    assert_eq!(row["default_placeholder"], 100);
    let body = json!({"execution_not_implied":true,"first_line":157,"last_line":160,"kind":"callback_reference","path":"@C:/code/poe-optimizer/vendor/path-of-building-poe2/src/Modules/ConfigOptions.lua","what":"Lua"});
    assert_eq!(row["apply"], body);
    assert_eq!(
        o["constructors"],
        json!([{"input":null,"placeholder":100,"default_state":0},{"input":null,"placeholder":100,"default_state":0}])
    );
    let calls = o["calls"].as_array().unwrap();
    assert_eq!(calls.len(), 2);
    for call in calls {
        assert_eq!(call["var"], SOURCE);
        assert_eq!(call["kind"], "count");
        assert_eq!(call["body"], body);
        assert_eq!(call["argument_fields"], json!({"val":100}));
        let before = &call["before"];
        let mut after = call["after"].clone();
        let records = after["player_records"].as_array_mut().unwrap();
        let count = records.len();
        assert!(count >= 2);
        let added = records.split_off(count - 2);
        let mut expected = source_outputs(Some(100.)).unwrap();
        // Preserve the captured JSON representation as well as the exact raw
        // numerical value; source_outputs is a scalar-domain derivation only.
        expected[1]["value"] = call["argument_fields"]["val"].clone();
        for (offset, (record, expected_body)) in
            added.iter().zip(expected.as_array().unwrap()).enumerate()
        {
            assert_eq!(
                record,
                &json!({"position":records.len()+offset+1,"body":expected_body})
            );
        }
        // Every other Config field and both other existing record inventories
        // are unchanged. The callback observation is not an effect evaluation.
        assert!(
            before == &after,
            "CurrentES callback changed more than its two Player records"
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
        "configured-player-current-energy-shield-percentage",
        1,
    )
}
