#![cfg(feature = "pob")]

use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn mapping() -> PathBuf {
    root().join("tests/fixtures/calibration/spark-mapping.xml")
}
fn run(path: &Path, options: &Value, metrics: &[&str]) -> Output {
    let temp = tempfile::tempdir().unwrap();
    let config = temp.path().join("options.json");
    fs::write(&config, serde_json::to_vec(options).unwrap()).unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command
        .current_dir(root())
        .arg("evaluate")
        .arg(path)
        .arg("--options")
        .arg(config)
        .args(["--timeout-seconds", "60"]);
    for metric in metrics {
        command.arg("--metric").arg(metric);
    }
    command.output().unwrap()
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice::<Value>(&output.stdout).unwrap()["evaluation"].take()
}
fn failure(output: Output, message: &str) {
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(error.contains(message), "expected {message}: {error}");
}
fn metric<'a>(result: &'a Value, actor: &str, id: &str) -> &'a Value {
    result["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .find(|metric| metric["query"]["actor"] == actor && metric["query"]["id"] == id)
        .unwrap()
}

#[test]
fn explicit_encounter_and_action_selection_preserve_requested_context() {
    let options = json!({"selection":{"socket_group":1,"active_skill":1},
        "encounter":{"name":"mapping-calibration","enemy_level":60,"boss":"normal",
            "incoming_hit":{"physical":1000.0,"fire":0.0,"cold":0.0,"lightning":0.0,"chaos":0.0}}});
    let result = success(run(
        &mapping(),
        &options,
        &["player.selected_hit_dps", "player.life"],
    ));
    assert_eq!(result["measurements"].as_array().unwrap().len(), 2);
    assert_eq!(result["context"]["enemy_level"], 60);
    assert_eq!(
        result["context"]["requested"]["encounter"]["name"],
        "mapping-calibration"
    );
    assert_eq!(result["context"]["config_inputs"]["enemyIsBoss"], "None");
    assert_eq!(result["context"]["config_inputs"]["enemyFireDamage"], 0.0);
    let dps = metric(&result, "player", "selected_hit_dps");
    assert_eq!(dps["unit"], "damage_per_second");
    assert!((dps["value"]["value"].as_f64().unwrap() - 8.5642857142857).abs() < 1e-8);
    assert!(result["attachments"].as_array().unwrap().is_empty());
    assert_eq!(result["diagnostic_only"], true);
}

#[test]
fn supplied_minion_coverage_preserves_ambiguity_and_nonfinite_kind() {
    let result = success(run(
        &root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt"),
        &json!({}),
        &[],
    ));
    let coverage = &result["coverage"];
    assert_eq!(coverage["unresolved_entry_count"], 3);
    assert_eq!(coverage["full_dps"]["included_group_count"], 0);
    assert_eq!(
        coverage["groups"][5]["gems"][0]["hint"],
        "ambiguous_spectre_name_match"
    );
    assert_eq!(
        coverage["groups"][5]["gems"][0]["related_candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for group in [12, 13] {
        assert_eq!(
            coverage["groups"][group]["gems"][0]["hint"],
            "minion_action_name_match"
        );
    }
    assert_eq!(
        coverage["selected_minion"]["skill_id"],
        "ExplosiveTeleportSandDjinn"
    );
    assert_eq!(coverage["selected_minion"]["group_index"], 1);
    assert_eq!(coverage["selected_minion"]["show_average"], false);
    assert_eq!(coverage["groups"][15]["provenance"]["kind"], "tree_granted");
    assert_eq!(coverage["groups"][18]["provenance"]["kind"], "item_granted");
    let edges = coverage["tree_connections"].as_array().unwrap();
    assert_eq!(edges.len(), 14);
    assert!(
        edges
            .iter()
            .all(|edge| edge["missing_target_allocated"] == false)
    );
    let maximum_hit = metric(&result, "player", "chaos_max_hit");
    assert_eq!(maximum_hit["value"]["status"], "non_finite");
    assert_eq!(maximum_hit["value"]["kind"], "positive_infinity");
    let dps = metric(&result, "selected_minion", "selected_hit_dps");
    // PoB clears average mode after applying this self-cast action's cooldown.
    // This is the selected action's modeled rate, not a certified minion rotation.
    assert_eq!(dps["value"]["status"], "finite");
    assert_eq!(dps["unit"], "damage_per_second");
    assert!(dps["value"]["value"].as_f64().unwrap() > 0.0);
}

#[test]
fn minion_action_selection_changes_resolved_action() {
    let result = success(run(
        &root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.import.txt"),
        &json!({"selection":{"socket_group":1,"active_skill":1,"minion_skill":1}}),
        &[],
    ));
    let minion = &result["coverage"]["selected_minion"];
    assert_eq!(minion["actor_skill_index"], 1);
    assert_ne!(minion["skill_id"], "ExplosiveTeleportSandDjinn");
    assert_eq!(minion["group_index"], 1);
}

#[test]
fn invalid_metric_and_request_options_fail_without_fallback() {
    failure(
        run(&mapping(), &json!({}), &["player.CombinedDPS"]),
        "Unsupported metric",
    );
    failure(
        run(
            &mapping(),
            &json!({"encounter":{"name":"bad","enemy_level":0}}),
            &[],
        ),
        "Enemy level",
    );
    failure(
        run(&mapping(), &json!({"selection":{"socket_group":0}}), &[]),
        "positive",
    );
    failure(
        run(&mapping(), &json!({"selection":{"socket_group":999}}), &[]),
        "does not exist",
    );
    failure(
        run(
            &mapping(),
            &json!({"selection":{"socket_group":1,"active_skill":999}}),
            &[],
        ),
        "does not exist",
    );
    failure(
        run(
            &mapping(),
            &json!({"selection":{"socket_group":1,"minion_skill":1}}),
            &[],
        ),
        "no selected minion",
    );
    failure(
        run(
            &mapping(),
            &json!({"selection":{"socket_group":1,"typo":1}}),
            &[],
        ),
        "unknown field",
    );
}

#[test]
fn rejects_invalid_imported_action_and_dot_hit_override() {
    let temp = tempfile::tempdir().unwrap();
    let original = fs::read_to_string(mapping()).unwrap();
    let input = temp.path().join("invalid-action.xml");
    fs::write(
        &input,
        original.replace("mainActiveSkill=\"1\"", "mainActiveSkill=\"0\""),
    )
    .unwrap();
    failure(
        run(&input, &json!({"selection":{"socket_group":1}}), &[]),
        "mainActiveSkill",
    );
    let input = temp.path().join("dot.xml");
    fs::write(
        &input,
        original.replace("string=\"Melee\"", "string=\"DamageOverTime\""),
    )
    .unwrap();
    failure(
        run(
            &input,
            &json!({"encounter":{"name":"hit","incoming_hit":
        {"physical":1000.0,"fire":0.0,"cold":0.0,"lightning":0.0,"chaos":0.0}}}),
            &[],
        ),
        "DamageOverTime",
    );
}

#[test]
fn resistance_mapping_uses_capped_percent_units() {
    let temp = tempfile::tempdir().unwrap();
    let original = fs::read_to_string(mapping()).unwrap();
    let input = temp.path().join("resistance.xml");
    fs::write(
        &input,
        original.replace(
            "</ConfigSet>",
            "<Input name=\"customMods\" string=\"+200% to Fire Resistance\"/></ConfigSet>",
        ),
    )
    .unwrap();
    let result = success(run(
        &input,
        &json!({}),
        &["player.fire_resistance_capped_pct"],
    ));
    let resist = metric(&result, "player", "fire_resistance_capped_pct");
    assert_eq!(resist["unit"], "percent");
    assert_eq!(resist["value"]["value"], 75.0);
}

#[test]
fn nonfinite_imported_encounter_number_is_rejected_before_calculation() {
    let temp = tempfile::tempdir().unwrap();
    let xml = fs::read_to_string(mapping()).unwrap();
    let mutated = xml.replace(
        "</ConfigSet>",
        "<Input name=\"enemyLevel\" number=\"1e999\"/></ConfigSet>",
    );
    assert_ne!(mutated, xml);
    let path = temp.path().join("nonfinite.xml");
    fs::write(&path, mutated).unwrap();
    failure(run(&path, &json!({}), &[]), "finite number");
}
