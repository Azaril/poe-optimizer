//! Two owned quantities from one authenticated saved control. No native effect.
use crate::{configuration_sources as sources, configuration_stage as common};
use poe_optimizer_import::{
    owned_normalize::{ConfigurationDefaultInput, ConfigurationDispositionRule},
    owned_recipe_extension::OwnedRecipeExtension,
    owned_release::StagedOwnedRelease,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{fs, path::PathBuf, sync::OnceLock};

pub const INPUTS: [&str; 2] = ["def.0000000000003380", "def.0000000000003381"];
pub const UNIT: &str = "def.0000000000002425";
const WITNESS: &str = "a61e14e329084002bdc1b1ef9034f33e62574e0982619a9a6a5dcb4554786890";
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
pub fn directory() -> PathBuf {
    root().join("data/owned/poe2/3887ae68/configuration-distance-inputs")
}
pub fn read<T: DeserializeOwned>(name: &str) -> T {
    serde_json::from_slice(&fs::read(directory().join(name)).unwrap()).unwrap()
}
fn value(v: impl Serialize) -> Value {
    serde_json::to_value(v).unwrap()
}
fn hash(v: &[u8]) -> String {
    format!("{:x}", Sha256::digest(v))
}

pub fn check_authored() {
    let inputs: Vec<ConfigurationDefaultInput> = read("input.json");
    assert_eq!(inputs.len(), 2);
    for (i, (key, selection)) in inputs.iter().zip(
        INPUTS
            .into_iter()
            .zip(["input_then_placeholder", "nonzero_input_then_placeholder"]),
    ) {
        assert_eq!(i.source_name, "enemyDistance");
        assert_eq!(i.value_input.key().as_str(), key);
        assert_eq!(value(i.target), "enemy");
        assert_eq!(value(i.selection), selection);
        assert!(!i.ignore_numeric_placeholder);
        assert_eq!(value(&i.constructor_default)["value"]["value"], 20.);
        assert_eq!(value(&i.constructor_default)["value"]["unit"]["key"], UNIT);
        assert_eq!(
            value(&i.recipe)["tiers"],
            json!([{"selectors":[{"lane":"input_number","name":"enemyDistance"}],"duplicates":"reject"}])
        );
        assert_eq!(
            value(&i.recipe)["codec"]["codec"]["value"]["scale"],
            json!({"numerator":1,"denominator":1})
        );
        assert_eq!(
            value(&i.recipe)["codec"]["codec"]["value"]["unit"]["key"],
            UNIT
        );
    }
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
    let deps: Value = read("dependencies.json");
    assert_eq!(
        deps["before"],
        "fe061a37c795102c8ca025cf52ebca199cb0ba3be42c9fa901e6c42ac5bc6f3f"
    );
    assert_eq!(deps["registry_last_issued"], 0x337f);
    assert_eq!(
        deps["unit_descriptor"]["value"]["schema"]["value"]["dimension"],
        "distance"
    );
    let mut encounter = deps["encounter"].clone();
    encounter["value"]["schema"]["value"]["external_inputs"]["members"]
        .as_array_mut()
        .unwrap()
        .extend(inputs.iter().map(|i| value(&i.value_input)));
    assert_eq!(value(&e.schema[0])["value"], encounter);
    for (def, input) in e.schema.iter().skip(1).zip(&inputs) {
        let def = value(def);
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
    }
    let rows: Vec<ConfigurationDispositionRule> = read("accounting.json");
    assert_eq!(rows.len(), 2);
    for (row, lane) in rows.iter().zip(["input_number", "placeholder_number"]) {
        assert_eq!(
            value(&row.selector),
            json!({"lane":lane,"name":"enemyDistance"})
        );
        assert_eq!(value(&row.destinations), value(inputs.iter().map(|i| json!({"kind":"adapter","control":{"kind":"scalar_input","input":i.value_input}})).collect::<Vec<_>>()));
    }
    check_source();
}

const ARCHIVES: &[&str] = &[
    "src/Modules/ConfigOptions.lua",
    "src/Classes/ConfigTab.lua",
    "src/Modules/CalcActiveSkill.lua",
    "src/Classes/ModStore.lua",
    "src/Modules/CalcSetup.lua",
    "src/Modules/CalcPerform.lua",
    "src/Modules/CalcDefence.lua",
    "src/Modules/CalcOffence.lua",
    "src/Classes/CalcsTab.lua",
    "src/Modules/ModParser.lua",
    "src/Data/Skills/sup_dex.lua",
];
const SPANS: &[(&str, usize, usize, &str)] = &[
    (
        "src/Modules/ConfigOptions.lua",
        1639,
        1641,
        "distance-control-and-callback",
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
        "src/Modules/CalcActiveSkill.lua",
        673,
        689,
        "skill-distance-selection-and-view-gate",
    ),
    (
        "src/Classes/ModStore.lua",
        722,
        739,
        "distance-ramp-consumer",
    ),
    (
        "src/Modules/CalcSetup.lua",
        111,
        113,
        "effective-store-condition",
    ),
    (
        "src/Modules/CalcSetup.lua",
        804,
        825,
        "effective-view-policy",
    ),
    ("src/Modules/CalcPerform.lua", 524, 527, "presence-consumer"),
    (
        "src/Modules/CalcDefence.lua",
        1476,
        1477,
        "defence-distance-consumer",
    ),
    (
        "src/Modules/CalcOffence.lua",
        2715,
        2727,
        "accuracy-distance-consumer",
    ),
    (
        "src/Classes/CalcsTab.lua",
        13,
        16,
        "calculation-view-options",
    ),
    (
        "src/Classes/CalcsTab.lua",
        170,
        175,
        "sidebar-effective-view",
    ),
    (
        "src/Modules/ModParser.lua",
        2161,
        2162,
        "generic-distance-threshold-translation",
    ),
    (
        "src/Classes/ModStore.lua",
        421,
        423,
        "multiplier-override-and-parent-composition",
    ),
    (
        "src/Classes/ModStore.lua",
        574,
        611,
        "generic-distance-threshold-consumer",
    ),
    (
        "src/Modules/CalcOffence.lua",
        5207,
        5219,
        "ailment-distance-cfg-copy",
    ),
    (
        "src/Data/Skills/sup_dex.lua",
        3536,
        3543,
        "threshold-data-example",
    ),
];
pub fn authenticate(e: &Value) {
    sources::authenticate(e, ARCHIVES, SPANS);
    assert_eq!(
        e["scope"],
        json!({"controls":12,"rows":14,"whole_build_parity":false,"native_coverage":false,"new_source_execution":false,"plain_control":false,"independent_distance_repetition":false,"contribution_emission":false,"view_mode_mapping":false,"admitted_range":[-1000000,1000000]})
    );
    for field in ["sha256", "jit_off_sha256", "jit_on_sha256"] {
        assert_eq!(e["witness"][field], WITNESS);
    }
    assert_eq!(
        e["witness"]["authoring_file"],
        "data/owned/poe2/3887ae68/minion-accuracy/authoring.json"
    );
    let authoring =
        fs::read(root().join(e["witness"]["authoring_file"].as_str().unwrap())).unwrap();
    assert_eq!(hash(&authoring), e["witness"]["authoring_sha256"]);
    let authoring: Value = serde_json::from_slice(&authoring).unwrap();
    assert_eq!(authoring["source_validation"]["evidence_sha256"], WITNESS);
    let raw = sources::retained_gzip(&e["witness"], 6 * 1024 * 1024);
    let witness: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(witness["source_revision"], e["source_revision"]);
    let manifest =
        fs::read(root().join("crates/poe-optimizer-pob/data/pob-source-manifest.json")).unwrap();
    assert_eq!(witness["source_hash"], hash(&manifest));
    let manifest: Value = serde_json::from_slice(&manifest).unwrap();
    for file in witness["evidence"]["files"].as_array().unwrap() {
        assert!(
            manifest["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|p| p["path"] == file["path"] && p["sha256"] == file["sha256"])
        );
    }
    for original in witness["evidence"]["originals"].as_array().unwrap() {
        assert_eq!(
            hash(
                &fs::read(
                    root()
                        .join("tests/fixtures/builds/breadth-20260908")
                        .join(original["name"].as_str().unwrap())
                )
                .unwrap()
            ),
            original["sha256"]
        );
    }
    let observer = sources::retained_gzip(&e["witness"]["observer"], 128 * 1024);
    assert_eq!(hash(&observer), witness["evidence"]["observer_sha256"]);
    validate_witness(&witness);
    let acquisition = sources::acquisition();
    for report in [&acquisition.jit_off, &acquisition.jit_on] {
        let catalogue: Vec<_> = report["state"]["configuration_catalogue"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["fields"]["var"] == "enemyDistance")
            .collect();
        assert_eq!(catalogue.len(), 1);
        assert_eq!(catalogue[0]["fields"]["type"], "count");
        assert_eq!(catalogue[0]["default_placeholder"], 20);
        let apply = &catalogue[0]["apply"];
        assert_eq!(apply["first_line"], 1639);
        assert_eq!(apply["last_line"], 1641);
        assert!(
            apply["path"]
                .as_str()
                .unwrap()
                .ends_with("/src/Modules/ConfigOptions.lua")
        );
        for config in report["state"]["configuration"].as_array().unwrap() {
            assert!(config["input"].get("enemyDistance").is_none());
            assert_eq!(config["placeholder"]["enemyDistance"], 20);
            // The count control's UI state is zero; the separately declared
            // effective Placeholder is 20 and is the owned absence fallback.
            assert_eq!(config["default_state"]["enemyDistance"], 0);
        }
    }
}

/// Reconcile actual stored Config records separately from view-dependent reads.
/// This finite witness is not a producer/recipient coverage certificate.
pub fn validate_witness(w: &Value) {
    assert_eq!(w["evidence"]["case_count"], 31);
    assert_eq!(w["evidence"]["complete_load_attempts_per_jit"], 32);
    for field in [
        "actor_level_mutation",
        "business_method_wrappers",
        "native_coverage",
        "whole_build_parity",
    ] {
        assert_eq!(w["evidence"][field], false);
    }
    let controls = [
        ("distance-negative", Some("-10"), None, -10., -10.),
        ("distance-start", Some("20"), None, 20., 20.),
        ("distance-fractional", Some("55.5"), None, 55.5, 55.5),
        ("distance-end", Some("90"), None, 90., 90.),
        ("distance-above", Some("120"), None, 120., 120.),
        ("distance-absent", None, None, 20., 20.),
        ("distance-placeholder-80", None, Some("80"), 80., 80.),
        (
            "distance-input-zero-placeholder-80",
            Some("0"),
            Some("80"),
            0.,
            80.,
        ),
        (
            "distance-input-50-placeholder-80",
            Some("50"),
            Some("80"),
            50.,
            50.,
        ),
        ("distance-placeholder-zero", None, Some("0"), 0., 0.),
        ("distance-calcs-effective", None, Some("80"), 80., 80.),
        ("distance-calcs-combat", None, Some("80"), 80., 80.),
    ];
    let cases: Vec<_> = w["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|r| r["name"].as_str().unwrap().starts_with("distance-"))
        .collect();
    assert_eq!(cases.len(), controls.len());
    let mut observed_rows = 0;
    for (case, (name, input, placeholder, range, candidate)) in cases.into_iter().zip(controls) {
        assert_eq!(case["name"], name);
        assert_eq!(case["available"], true);
        let saved = &case["xml_config"]["enemyDistance"];
        assert_eq!(saved.get("input").and_then(Value::as_str), input);
        assert_eq!(
            saved.get("placeholder").and_then(Value::as_str),
            placeholder
        );
        let raw = &case["state"]["config"]["enemy_distance"];
        assert_eq!(raw["input_present"], input.is_some());
        assert_eq!(
            raw.get("input").and_then(Value::as_f64),
            input.map(|i| i.parse::<f64>().unwrap())
        );
        assert_eq!(raw["placeholder_present"], true);
        assert_eq!(
            raw["placeholder"],
            placeholder
                .map(|p| p.parse::<f64>().unwrap())
                .unwrap_or(20.)
        );
        let records = &case["state"]["config"]["distance_records"];
        if candidate == 0. {
            assert_eq!(records, &json!({}));
        } else {
            assert_eq!(records[0]["value"].as_f64(), Some(candidate));
            assert_eq!(records[0]["mod"]["value"].as_f64(), Some(candidate));
            assert_eq!(
                records,
                &json!([{"mod":{"flags":0,"keyword_flags":0,"name":"Multiplier:enemyDistance","source":"Config","tags":[{"type":"Condition","var":"Effective"}],"type":"BASE","value":records[0]["mod"]["value"]},"value":records[0]["value"]}])
            );
        }
        let modes = if name.starts_with("distance-calcs-") {
            vec!["main", "calcs"]
        } else {
            vec!["main"]
        };
        for mode in modes {
            let player = &case["state"][mode]["player"];
            assert_eq!(player["effect_id"], "TwisterPlayer");
            let passes = player["passes"].as_array().unwrap();
            assert_eq!(passes.len(), 1);
            let pass = &passes[0];
            let effective = !(name == "distance-calcs-combat" && mode == "calcs");
            assert_eq!(pass["effective"], effective);
            assert_eq!(pass["query_state_preserved"], true);
            if effective {
                assert_eq!(pass["accuracy"]["skill_distance"].as_f64(), Some(range));
            } else {
                assert_eq!(pass["accuracy"]["skill_distance"], false);
            }
            assert_eq!(
                pass["accuracy"]["distance_sum"],
                if effective { candidate } else { 0. }
            );
            observed_rows += 1;
        }
    }
    assert_eq!(observed_rows, 14);
}
pub fn witness() -> Value {
    let evidence: Value = read("source-evidence.json");
    serde_json::from_slice(&sources::retained_gzip(
        &evidence["witness"],
        6 * 1024 * 1024,
    ))
    .unwrap()
}
pub fn check_source() {
    static CHECKED: OnceLock<()> = OnceLock::new();
    CHECKED.get_or_init(|| authenticate(&read("source-evidence.json")));
}
pub fn stage(prior: &StagedOwnedRelease) -> StagedOwnedRelease {
    check_authored();
    common::stage(
        prior,
        &directory(),
        "configured-target-distance-projections",
        2,
    )
}
