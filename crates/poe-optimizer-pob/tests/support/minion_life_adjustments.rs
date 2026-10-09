//! Exact original minion Life inputs. Observed neutral values do not close
//! native supplier inventories or certify the final resource formula.
use super::*;
#[path = "minion_life_adjustments_evidence.rs"]
mod evidence;

const TEST: &str = "life_adjustments::minion_life_adjustments_observe_original_sources";
const CHILD: &str = "POE_MINION_LIFE_ADJUSTMENT_SOURCE_CHILD";
#[test]
#[ignore = "requires complete pinned PoB runtime; exact minion resource inputs"]
fn minion_life_adjustments_observe_original_sources() {
    run_life_source_modes(
        TEST,
        CHILD,
        "POE_MINION_LIFE_ADJUSTMENT_SOURCE_OUT",
        "runs/owned-minion-life-adjustment-source-01",
        run_child,
    );
}
fn run_child(root: &Path, out: &Path, enabled: bool) {
    run_actor_life_child(root, out, enabled, ActorLifeEvidence::Adjustments);
}

#[test]
#[ignore = "requires retained POE_MINION_LIFE_ADJUSTMENT_SOURCE_OUT reports"]
fn retained_adjustment_reports_match_and_authenticate() {
    let dir = PathBuf::from(
        std::env::var_os("POE_MINION_LIFE_ADJUSTMENT_SOURCE_OUT").expect("retained source output"),
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
    json_evidence::assert_files_equal(&off, &on, "complete minion Life source census");
    for path in [off, on] {
        let report: Json = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        assert_eq!(report["observer_sha256"], digest(OBSERVE.as_bytes()));
        assert_eq!(report["manifest_sha256"], pinned::manifest_sha256());
        check_life_delivery(&report);
        check(&report);
    }
}

pub(super) fn check(report: &Json) {
    assert_eq!(report["capture"]["minion_life_adjustment_census"], true);
    assert_eq!(report["capture"]["raw_zero_valued_sources_retained"], true);
    assert_eq!(report["capture"]["observer_jit_enabled"], false);
    assert_eq!(
        report["capture"]["unhooked_jit_mode_from_report_filename"],
        true
    );
    assert_eq!(report["scope"]["native_coverage"], false);
    assert_eq!(report["scope"]["whole_build_parity"], false);
    assert_eq!(report["scope"]["final_life_formula_parity"], false);
    let cases = rows(&report["cases"]);
    assert_eq!(cases.len(), 7);
    for (index, case) in cases.iter().enumerate() {
        let state = &case["state"];
        assert_eq!(state["life_adjustment_methods_preserved"], true);
        assert_eq!(state["benefit_snapshot"], case["unhooked"]);
        for mode in ["main", "calcs"] {
            let actors: Vec<_> = rows(&state[mode]["actors"])
                .iter()
                .filter(|a| a["actor_profile"] == "RaisedSkeletonSniper")
                .collect();
            assert_eq!(actors.len(), 1);
            let actor = actors[0];
            let selected = mode == "main" || index == 6;
            assert_eq!(actor["is_environment_minion"], selected);
            let observed = &actor["life_adjustments"];
            assert_eq!(observed["exact_parent"], true);
            assert_eq!(observed["exact_summoner"], true);
            let calls = rows(&observed["original_life_calls"]);
            assert_eq!(
                observed["original_life_calls"],
                actor["gigantic_benefits"]["original_life_calls"]
            );
            let insertions = rows(&observed["strength_insertions"]);
            let snapshots = rows(&case["unhooked"][mode]["actors"]);
            assert_eq!(snapshots.len(), 1);
            let snapshot = &snapshots[0];
            assert_eq!(snapshot["source"], actor["source_occurrence"]);
            assert_eq!(snapshot["selected"], selected);
            assert_eq!(snapshot["actor_profile"], actor["actor_profile"]);
            assert_eq!(
                snapshot["life_adjustment_identity"]["output_present"],
                selected
            );
            assert!(rows(&snapshot["raw_inherent_attributes"]).is_empty());
            for field in ["exact_actor_store", "exact_parent", "exact_summoner"] {
                assert_eq!(snapshot["life_adjustment_identity"][field], true);
            }
            assert_eq!(
                snapshot["life_adjustment_identity"]["store_is_player"],
                false
            );
            if !selected {
                assert!(
                    calls.is_empty() && insertions.is_empty(),
                    "unexecuted is not observed empty"
                );
                assert_eq!(
                    snapshot["life_adjustment_identity"]["strength"],
                    json!({"present":false})
                );
                continue;
            }
            assert_eq!(calls.len(), 3);
            assert_eq!(insertions.len(), 1);
            let insertion = &insertions[0];
            assert_eq!(insertion["index"], 1);
            assert_eq!(insertion["caller_source"], "Modules/CalcPerform.lua");
            assert_eq!(insertion["original_function_line"], 264);
            assert_eq!(insertion["caller_line"], 506);
            assert_eq!(insertion["source"], actor["source_occurrence"]);
            assert_eq!(insertion["selected"], true);
            assert_eq!(insertion["mode"], mode.to_uppercase());
            assert_eq!(insertion["actor_profile"], actor["actor_profile"]);
            assert_eq!(insertion["stored_identity_count"], 1);
            assert_eq!(insertion["inherent_attribute_multiplier"], 1);
            for field in [
                "exact_actor_store",
                "exact_summoner",
                "original_record_preserved",
            ] {
                assert_eq!(insertion[field], true);
            }
            assert_eq!(insertion["store_is_player"], false);
            assert_inherent_inputs(&insertion["inputs"]);
            for (call, line) in calls.iter().zip([998, 1189, 1631]) {
                assert_eq!(call["caller_source"], "Modules/CalcDefence.lua");
                assert_eq!(call["caller_line"], line);
                assert_eq!(call["source"], actor["source_occurrence"]);
                assert_eq!(call["exact_actor_store"], true);
                assert_eq!(call["exact_actor_output"], true);
                let c = &call["computation"];
                let a = &c["adjustments"];
                check_adjustments(c, &insertion["record"], line == 1631);
                assert_eq!(a["channels"]["Life"]["base"]["value"], c["base"]);
                assert_eq!(a["channels"]["Life"]["increased"]["value"], c["increased"]);
                assert_eq!(a["channels"]["Life"]["more"]["value"], c["more"]);
                assert_eq!(a["original_capped_conversion"], c["conversion"]);
                assert_eq!(a["selected_override"]["present"], c["override_present"]);
                assert!(c["override"].is_null());
                assert_eq!(a["channels"]["ExtraLife"]["value"], c["extra"]);
                assert_eq!(a["channels"]["LifeTotal"]["value"], c["total"]);
                assert_eq!(call["return_life"], call["post_return_life"]);
                assert_eq!(call["post_return_life"], observed["actor_output_life"]);
            }
            assert_eq!(snapshot["output"]["Life"], observed["actor_output_life"]);
            assert_eq!(
                snapshot["raw_life_adjustments"],
                calls.last().unwrap()["computation"]["adjustments"]["raw_modifiers"]
            );
            assert!(rows(&snapshot["raw_inherent_life_flags"]).is_empty());
            assert_eq!(
                snapshot["life_adjustment_identity"]["strength"],
                json!({"present":true,"value":0})
            );
        }
    }
}

fn assert_inherent_inputs(inputs: &Json) {
    assert_eq!(inputs["strength"], json!({"present":true,"value":0}));
    assert!(rows(&inputs["raw_attributes"]).is_empty());
    let flags = rows(&inputs["flags"]);
    assert_eq!(flags.len(), 5);
    for (flag, name) in flags.iter().zip([
        "NoAttributeBonuses",
        "NoStrengthAttributeBonuses",
        "NoStrBonusToLife",
        "DoubledInherentAttributeBonuses",
        "HalvesLifeFromStrength",
    ]) {
        assert_eq!(flag["name"], name);
        assert_eq!(flag["present"], false);
        assert!(flag["value"].is_null());
        assert!(rows(&flag["raw"]).is_empty() && rows(&flag["eligible"]).is_empty());
    }
}

fn check_adjustments(c: &Json, insertion: &Json, after_conversion: bool) {
    let a = &c["adjustments"];
    assert_eq!(a["observed_at"], 97);
    assert_eq!(a["exact_actor_store"], true);
    assert_eq!(a["exact_summoner"], true);
    assert_eq!(a["store_is_player"], false);
    assert_eq!(a["channel_queries_are_supplemental"], true);
    assert_eq!(a["tabulate_omits_zero_non_override"], true);
    let raw = rows(&a["raw_modifiers"]);
    let mut life = vec![c["intrinsic_base"]["record"].clone()];
    life.extend(
        rows(&c["life_increase_delivery"]["eligible"])
            .iter()
            .map(|r| r["record"].clone()),
    );
    life.extend(
        rows(&c["eligible_life_more"])
            .iter()
            .map(|r| r["mod"].clone()),
    );
    life.push(insertion.clone());
    let life_raw: Vec<_> = life
        .iter()
        .map(|m| json!({"ancestor_depth":0,"mod":m}))
        .collect();
    let mut expected = life_raw.clone();
    if after_conversion {
        // Actual generated zero sources appear only before the third original
        // call. This authenticates their records, not their generating formula.
        for name in ["ExtraLife", "LifeTotal"] {
            expected.push(json!({"ancestor_depth":0,"mod":{"name":name,"type":"BASE","source":"Conversion","value":0,"flags":0,"keyword_flags":0,"tags":{}}}));
        }
        expected.sort_by(|a, b| a["mod"]["name"].as_str().cmp(&b["mod"]["name"].as_str()));
    }
    assert_eq!(
        raw, expected,
        "every raw source, including zero, must be accounted for"
    );
    let eligible: Vec<_> = life
        .iter()
        .filter(|m| m["source"] != "Strength")
        .map(|m| json!({"mod":m,"value":m["value"]}))
        .collect();
    assert_eq!(rows(&a["eligible_modifiers"]), eligible);
    for (channel, kind) in [("base", "BASE"), ("increased", "INC"), ("more", "MORE")] {
        assert_eq!(a["channels"]["Life"][channel]["names"], json!(["Life"]));
        assert_eq!(
            rows(&a["channels"]["Life"][channel]["records"]),
            eligible
                .iter()
                .filter(|r| r["mod"]["type"] == kind)
                .cloned()
                .collect::<Vec<_>>()
        );
    }
    let strength: Vec<_> = raw
        .iter()
        .filter(|r| r["mod"]["source"] == "Strength")
        .collect();
    assert_eq!(strength.len(), 1, "zero is an actual supplier");
    assert_eq!(strength[0]["ancestor_depth"], 0);
    assert_eq!(strength[0]["mod"], *insertion);
    assert_eq!(insertion["name"], "Life");
    assert_eq!(insertion["type"], "BASE");
    assert_eq!(insertion["value"], 0);
    assert_eq!(insertion["flags"], 0);
    assert_eq!(insertion["keyword_flags"], 0);
    assert!(rows(&insertion["tags"]).is_empty());
    let strength = rows(&a["strength_records"]);
    assert_eq!(strength.len(), 1);
    assert_eq!(strength[0]["record"], *insertion);
    assert_eq!(strength[0]["original_insertion_observed"], true);
    assert_eq!(strength[0]["insertion_index"], 1);
    assert_eq!(strength[0]["ancestor_depth"], 0);
    assert_eq!(strength[0]["position"], life_raw.len());
    assert_eq!(strength[0]["tabulated_base_occurrences"], 0);
    assert!(
        rows(&a["eligible_modifiers"])
            .iter()
            .all(|r| r["mod"]["source"] != "Strength")
    );
    for name in [
        "ExtraLife",
        "LifeTotal",
        "LifeConvertToEnergyShield",
        "LifeConvertToArmour",
        "LifeConvertToEvasion",
    ] {
        let channel = &a["channels"][name];
        assert_eq!(channel["names"], json!([name]));
        assert_eq!(channel["value"], 0);
        assert!(rows(&channel["records"]).is_empty());
    }
    assert_eq!(
        a["conversion_before_cap"]["names"],
        json!([
            "LifeConvertToEnergyShield",
            "LifeConvertToArmour",
            "LifeConvertToEvasion"
        ])
    );
    assert_eq!(a["conversion_before_cap"]["value"], 0);
    assert!(rows(&a["conversion_before_cap"]["records"]).is_empty());
    assert_eq!(a["original_capped_conversion"], 0);
    assert_eq!(a["selected_override"], json!({"present":false}));
    assert!(rows(&a["channels"]["Life"]["overrides"]).is_empty());
    let ci = &a["chaos_inoculation"];
    assert_eq!(ci["present"], false);
    assert!(ci["value"].is_null());
    assert!(rows(&ci["raw"]).is_empty() && rows(&ci["eligible"]).is_empty());
    assert_inherent_inputs(&a["inherent_inputs"]);
}
