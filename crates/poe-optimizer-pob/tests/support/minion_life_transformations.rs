//! Original minion resource-pass observations, without a native formula claim.
use super::*;
#[path = "minion_life_transformations_evidence.rs"]
mod evidence;

const TEST: &str = "life_transformations::minion_life_transformations_observe_original_sources";
const CHILD: &str = "POE_MINION_LIFE_TRANSFORMATION_SOURCE_CHILD";
const DONORS: [&str; 5] = ["Armour", "Evasion", "EnergyShield", "Mana", "Ward"];
const RESOURCES: [&str; 6] = ["Armour", "Evasion", "EnergyShield", "Life", "Mana", "Ward"];

#[test]
#[ignore = "requires complete pinned PoB runtime; exact incoming minion Life resource inputs"]
fn minion_life_transformations_observe_original_sources() {
    run_life_source_modes(
        TEST,
        CHILD,
        "POE_MINION_LIFE_TRANSFORMATION_SOURCE_OUT",
        "runs/owned-minion-life-transformation-source-01",
        run_child,
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    run_actor_life_child(root, out, enabled, ActorLifeEvidence::Transformations);
}

#[test]
#[ignore = "requires retained POE_MINION_LIFE_TRANSFORMATION_SOURCE_OUT reports"]
fn retained_transformation_reports_match_and_authenticate() {
    let dir = PathBuf::from(
        std::env::var_os("POE_MINION_LIFE_TRANSFORMATION_SOURCE_OUT")
            .expect("retained source output"),
    );
    let dir = if dir.is_absolute() {
        dir
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(dir)
    };
    let off = dir.join("source-jit-off.json");
    let on = dir.join("source-jit-on.json");
    json_evidence::assert_files_equal(&off, &on, "complete minion Life resource census");
    for path in [off, on] {
        let report: Json = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(report["observer_sha256"], digest(OBSERVE.as_bytes()));
        assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
        assert_eq!(
            report["driver_sha256"],
            digest(include_bytes!("../owned_minion_physical_damage_source.rs"))
        );
        assert_eq!(
            report["source_helper_sha256"],
            digest(include_bytes!("minion_life_transformations.rs"))
        );
        assert_eq!(
            report["bootstrap_sha256"],
            digest(include_bytes!("configuration_preparation_source.rs"))
        );
        check_life_delivery(&report);
        life_adjustments::check(&report);
        check(&report);
    }
}

fn zero(value: &Json) {
    assert_eq!(
        value.as_f64(),
        Some(0.0),
        "original numerical zero is present"
    );
}
fn resource<'a>(snapshot: &'a Json, name: &str) -> &'a Json {
    let resources = rows(&snapshot["resources"]);
    assert_eq!(resources.len(), RESOURCES.len());
    for (row, expected) in resources.iter().zip(RESOURCES) {
        assert_eq!(row["name"], expected);
    }
    let found: Vec<_> = resources.iter().filter(|r| r["name"] == name).collect();
    assert_eq!(found.len(), 1);
    found[0]
}
fn zero_record(record: &Json, name: &str) {
    assert_eq!(
        record,
        &json!({"name":name,"type":"BASE","value":0,"source":"Conversion","flags":0,"keyword_flags":0,"tags":{}})
    );
}

pub(super) fn check(report: &Json) {
    assert_eq!(
        report["capture"]["original_minion_resource_transformation"],
        true
    );
    assert_eq!(report["capture"]["incoming_life_channels"], 10);
    assert_eq!(
        report["capture"]["original_generated_record_identity"],
        true
    );
    assert_eq!(report["capture"]["observer_jit_enabled"], false);
    assert_eq!(
        report["capture"]["unhooked_jit_mode_from_report_filename"],
        true
    );
    assert_eq!(report["scope"]["positive_transformation_admission"], false);
    assert_eq!(report["scope"]["native_coverage"], false);
    assert_eq!(report["scope"]["final_life_formula_parity"], false);
    assert_eq!(report["complete_load_attempts_per_jit"], 16);
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 7);
    for (case_index, case) in cases.iter().enumerate() {
        assert_eq!(case["state"]["benefit_snapshot"], case["unhooked"]);
        for mode in ["main", "calcs"] {
            let actors: Vec<_> = rows(&case["state"][mode]["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            let source = &actor["source_occurrence"];
            let transformation = &actor["life_transformations"];
            assert_eq!(transformation["exact_parent"], true);
            assert_eq!(transformation["exact_summoner"], true);
            let selected = mode == "main" || case_index == 6;
            assert_eq!(actor["is_environment_minion"], selected);
            let calls = rows(&transformation["original_resource_calls"]);
            let snapshots = rows(&case["unhooked"][mode]["actors"]);
            assert_eq!(snapshots.len(), 1);
            assert_eq!(snapshots[0]["source"], *source);
            if !selected {
                assert!(calls.is_empty(), "unexecuted actor is not a neutral pass");
                assert!(rows(&snapshots[0]["raw_life_resource_inputs"]).is_empty());
                continue;
            }
            assert_eq!(calls.len(), 1);
            let call = &calls[0];
            assert_eq!(call["index"], 1);
            assert_eq!(call["source"], *source);
            assert_eq!(call["mode"], mode.to_uppercase());
            assert_eq!(call["selected"], true);
            assert_eq!(call["actor_profile"], "RaisedSkeletonSniper");
            assert_eq!(call["original_function_line"], 789);
            assert_eq!(call["store_is_player"], false);
            for field in [
                "exact_actor_store",
                "exact_parent",
                "exact_summoner",
                "return_observed",
            ] {
                assert_eq!(call[field], true, "original call identity: {field}");
            }
            assert_eq!(call["before"]["line"], 1375);
            assert_eq!(call["after"]["line"], 1430);
            assert!(call["before_events"].as_u64().unwrap() >= 1);
            assert!(call["after_events"].as_u64().unwrap() >= 1);
            let before_life = resource(&call["before"], "Life");
            let after_life = resource(&call["after"], "Life");
            zero(&before_life["globalBase"]);
            zero(&before_life["totalBase"]);
            zero(&after_life["globalBase"]);
            zero(&after_life["totalBase"]);
            let inputs = &call["inputs"];
            assert_eq!(inputs["channel_queries_are_supplemental"], true);
            assert!(rows(&inputs["raw"]).is_empty());
            assert!(rows(&inputs["life_total"]["raw"]).is_empty());
            assert!(rows(&inputs["life_total"]["eligible"]).is_empty());
            zero(&inputs["life_total"]["supplemental_sum"]);
            assert_eq!(
                before_life["totalBase"],
                inputs["life_total"]["supplemental_sum"]
            );
            let incoming = rows(&inputs["incoming"]);
            let steps = rows(&call["incoming_steps"]);
            assert_eq!((incoming.len(), steps.len()), (5, 5));
            for (index, ((input, step), donor)) in
                incoming.iter().zip(steps).zip(DONORS).enumerate()
            {
                assert_eq!(input["source"], donor);
                for (name, suffix) in [("conversion", "ConvertToLife"), ("gain", "GainAsLife")] {
                    let channel = &input[name];
                    assert_eq!(channel["name"], format!("{donor}{suffix}"));
                    assert!(rows(&channel["raw"]).is_empty());
                    assert!(rows(&channel["eligible"]).is_empty());
                    zero(&channel["supplemental_sum"]);
                }
                assert_eq!(step["index"], index + 1);
                assert!(step["line_events"].as_u64().unwrap() >= 1);
                assert_eq!(step["transfer_branch_entered"], false);
                let actual = &step["inputs"];
                assert_eq!(actual["source"], donor);
                assert_eq!(actual["target"], "Life");
                assert_eq!(actual["source_is_defence"], donor != "Mana");
                assert_eq!(
                    actual["checkpoint"],
                    if donor == "Mana" { 1412 } else { 1386 }
                );
                assert_eq!(actual["source_before"]["name"], donor);
                assert_eq!(actual["target_before"]["name"], "Life");
                zero(&actual["conversion_rate"]);
                zero(&actual["gain_rate"]);
                zero(&actual["combined_rate"]);
                assert_eq!(actual["gain_rate"], input["gain"]["supplemental_sum"]);
                assert_eq!(
                    actual["conversion_rate"],
                    resource(&call["before"], donor)["conversionRate"]["Life"]
                );
            }
            let insertions = rows(&call["insertions"]);
            let returned = rows(&call["generated_records_at_return"]);
            assert_eq!((insertions.len(), returned.len()), (2, 2));
            for (index, ((insertion, record), name)) in insertions
                .iter()
                .zip(returned)
                .zip(["ExtraLife", "LifeTotal"])
                .enumerate()
            {
                assert_eq!(insertion["index"], index + 1);
                assert_eq!(insertion["name"], name);
                assert_eq!(insertion["resource_name"], "Life");
                assert_eq!(insertion["caller_source"], "Modules/CalcDefence.lua");
                assert_eq!(insertion["caller_line"], 1440 + index);
                assert_eq!(insertion["original_newmod_line"], 142);
                assert_eq!(insertion["original_addmod_line"], 31);
                for field in [
                    "original_newmod_seen",
                    "exact_actor_store",
                    "exact_resource_object",
                ] {
                    assert_eq!(insertion[field], true);
                }
                assert_eq!(insertion["source"], *source);
                assert_eq!(insertion["stored_identity_count"], 1);
                let field = if index == 0 {
                    "globalBase"
                } else {
                    "totalBase"
                };
                assert_eq!(insertion["source_local"], field);
                assert_eq!(insertion["source_value"], after_life[field]);
                zero_record(&insertion["record"], name);
                assert_eq!(record["name"], name);
                assert_eq!(record["position"], 1);
                assert_eq!(record["insertion_index"], index + 1);
                assert_eq!(record["exact_inserted_object"], true);
                assert_eq!(record["record"], insertion["record"]);
            }
            let life_calls = rows(&actor["life_adjustments"]["original_life_calls"]);
            assert_eq!(life_calls.len(), 3);
            for first in &life_calls[..2] {
                assert!(rows(&first["computation"]["resource_generated_records"]).is_empty());
            }
            assert_eq!(life_calls[2]["caller_line"], 1631);
            assert_eq!(
                life_calls[2]["computation"]["resource_generated_records"],
                call["generated_records_at_return"]
            );
            assert_eq!(life_calls[2]["post_return_life"], call["output"]["Life"]);
            // Offence subsequently extends Actor output; the whole final
            // output is compared independently by the existing driver.
            assert_eq!(snapshots[0]["output"]["Life"], call["output"]["Life"]);
            assert_eq!(
                snapshots[0]["raw_life_resource_inputs"],
                json!(
                    insertions
                        .iter()
                        .map(|i| json!({"ancestor_depth":0,"mod":i["record"]}))
                        .collect::<Vec<_>>()
                )
            );
        }
    }
}
