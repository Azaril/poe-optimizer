use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn problem(template: &Path) -> Value {
    json!({"schema_version":1,"template":template,
        "weapons":[
            {"id":"wood","item_text":"Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 0\nImplicits: 0"},
            {"id":"smith","item_text":"Rarity: NORMAL\nSmithing Hammer\nItem Level: 1\nQuality: 0\nImplicits: 0"}],
        "supports":["none","brutality_i"],
        "objective":{"schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":"selected_hit_dps"},"unit":"damage_per_second","direction":"maximize"},"constraints":[]}})
}
fn command() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(root());
    c
}
fn run(path: &Path, strategy: &str, max: usize, export: Option<&Path>) -> Output {
    let mut c = command();
    c.args(["search-experimental", "--problem"])
        .arg(path)
        .args([
            "--strategy",
            strategy,
            "--jobs",
            "2",
            "--timeout-seconds",
            "120",
            "--max-evaluations",
        ])
        .arg(max.to_string());
    if let Some(p) = export {
        c.arg("--export").arg(p);
    }
    c.output().unwrap()
}
fn report(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
#[test]
fn mutation_search_budgets_baseline_verification_and_exports_supplied_choices() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("problem.json");
    let export = temp.path().join("winner.xml");
    fs::write(
        &path,
        serde_json::to_vec(&problem(
            &root().join("tests/fixtures/calibration/mace-wooden.xml"),
        ))
        .unwrap(),
    )
    .unwrap();
    let result = report(run(&path, "exhaustive", 6, Some(&export)));
    assert_eq!(result["scope"], "normal_mace_weapon_support_profile_v1");
    assert_eq!(result["preparation"]["attempts"], 1);
    assert_eq!(result["total_evaluations"], 6);
    assert_eq!(result["search"]["statistics"]["evaluations"], 5);
    assert_eq!(
        result["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(
        result["search"]["statistics"]["evaluation_failures"], 0,
        "{result}"
    );
    assert_eq!(
        result["search"]["feasible"].as_array().unwrap().len(),
        4,
        "{result}"
    );
    assert_eq!(result["best_verified"]["alternative_id"], "smith/none");
    assert!(
        (result["best_verified"]["assessment"]["objective_value"]["value"]
            .as_f64()
            .unwrap()
            - 18.208694)
            .abs()
            < 1e-8
    );
    assert!(
        fs::read_to_string(&export)
            .unwrap()
            .contains("Smithing Hammer")
    );
    assert_eq!(result["export"]["status"], "written");
    let partial = report(run(&path, "exhaustive", 3, None));
    assert_eq!(partial["total_evaluations"], 3);
    assert_eq!(partial["termination"], "evaluation_budget");
    assert_eq!(
        partial["search"]["statistics"]["verification_evaluations"],
        1
    );
}
#[test]
fn guided_search_keeps_weapon_and_support_locks_and_matches_finite_locked_domain() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("problem.json");
    let mut input = problem(&root().join("tests/fixtures/calibration/mace-wooden.xml"));
    input["locks"] = json!({"support":"brutality_i"});
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    let result = report(run(&path, "guided", 4, None));
    assert_eq!(
        result["best_verified"]["alternative_id"], "wood/brutality_i",
        "{result}"
    );
    assert_eq!(result["total_evaluations"], 4);
    for entry in result["search"]["feasible"].as_array().unwrap() {
        assert_eq!(entry["candidate"]["choices"][1], 1);
    }
    input["locks"] = json!({"support":"brutality_i","weapon_id":"smith"});
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    let locked = report(run(&path, "guided", 3, None));
    assert_eq!(
        locked["best_verified"]["alternative_id"],
        "smith/brutality_i"
    );
    assert_eq!(locked["total_evaluations"], 3);
}
#[test]
fn unsupported_profile_unknown_options_and_collisions_fail_before_spending_budget() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("problem.json");
    let mut input = problem(&root().join("tests/fixtures/calibration/mace-wooden.xml"));
    input["unknown"] = json!(true);
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(!run(&path, "exhaustive", 6, None).status.success());
    input.as_object_mut().unwrap().remove("unknown");
    input["locks"] = json!({"weapon_id":"absent"});
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(!run(&path, "exhaustive", 6, None).status.success());
    input["locks"] = json!({});
    input["template"] = json!(root().join("tests/fixtures/builds/pobarchives-Dfz36mCq.xml"));
    fs::write(&path, serde_json::to_vec(&input).unwrap()).unwrap();
    assert!(!run(&path, "exhaustive", 6, None).status.success());
    let invalid = command()
        .args(["search-experimental", "--problem"])
        .arg(&path)
        .args(["--max-evaluations", "2"])
        .output()
        .unwrap();
    assert!(!invalid.status.success());
    let out = temp.path().join("exists");
    fs::write(&out, b"preserve").unwrap();
    let collision = command()
        .args(["search-experimental", "--problem"])
        .arg(&path)
        .arg("--output")
        .arg(&out)
        .output()
        .unwrap();
    assert!(!collision.status.success());
    assert_eq!(fs::read(out).unwrap(), b"preserve");
}
#[test]
fn tree_cli_exports_versioned_data_and_rejects_overwrite_or_unknown_version() {
    let temp = tempfile::tempdir().unwrap();
    let out = temp.path().join("tree.json");
    let summary = report(
        command()
            .args(["extract-tree", "--timeout-seconds", "30", "--output"])
            .arg(&out)
            .output()
            .unwrap(),
    );
    assert_eq!(summary["classes"], 8);
    assert_eq!(summary["ascendancies"], 23);
    assert_eq!(summary["nodes"], 4914);
    assert_eq!(summary["dangling_connections"], 14);
    let snapshot: poe_optimizer_pob::tree_data::TreeDataSnapshot =
        serde_json::from_slice(&fs::read(&out).unwrap()).unwrap();
    assert_eq!(snapshot.sha256().unwrap(), summary["snapshot_sha256"]);
    assert_eq!(
        snapshot.classes[&1].start_node_id,
        snapshot.classes[&7].start_node_id
    );
    assert!(
        !command()
            .args(["extract-tree", "--output"])
            .arg(&out)
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(
        !command()
            .args(["extract-tree", "--tree-version", "unknown", "--output"])
            .arg(temp.path().join("bad.json"))
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(!temp.path().join("bad.json").exists());
}
