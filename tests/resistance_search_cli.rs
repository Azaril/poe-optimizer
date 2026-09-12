//! Ascendancy passive budgets, resistance objectives, injected data and CLI exports.
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
// These searches check results and evaluation counts, not debug-build throughput.
const SEARCH_COMPLETION_TIMEOUT_SECONDS: &str = "300";

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
fn input() -> Value {
    let mut problem: Value =
        serde_json::from_str(include_str!("../examples/mace-class-search.json")).unwrap();
    problem["schema_version"] = json!(3);
    problem["template"] = json!("build.xml");
    problem["tree_search"]["ascendancy_passive_points"] = json!(1);
    problem
}
fn prepare(path: &Path, problem: &Value) {
    fs::write(
        path.join("problem.json"),
        serde_json::to_vec(problem).unwrap(),
    )
    .unwrap();
    fs::write(path.join("build.xml"), TEMPLATE).unwrap();
}
fn cli(path: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(path);
    c
}
fn search(path: &Path, jobs: usize, max: usize) -> Command {
    let mut c = cli(path);
    c.args([
        "search-experimental",
        "--backend",
        "native",
        "--problem",
        "problem.json",
        "--timeout-seconds",
        SEARCH_COMPLETION_TIMEOUT_SECONDS,
        "--pob",
        "absent-reference-checkout",
    ])
    .arg("--jobs")
    .arg(jobs.to_string())
    .arg("--max-evaluations")
    .arg(max.to_string());
    c
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn verified(report: &Value, total: usize) {
    assert_ne!(
        report["termination"],
        "time_budget",
        "search completion limit={SEARCH_COMPLETION_TIMEOUT_SECONDS}s, expected evaluations={total}:\n{}",
        serde_json::to_string_pretty(report).unwrap()
    );
    assert_eq!(report["schema_version"], 4);
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["total_evaluations"], total);
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert_eq!(report["diagnostic_only"], true);
    assert!(!report["best_verified"].is_null());
}
fn chaos_goal(problem: &mut Value) {
    problem["objective"]["constraints"] = json!([{"id":"chaos-floor","metric":{"actor":"player","id":"chaos_resistance_capped_pct"},"unit":"percent","operator":">=","threshold":1.0,"violation_scale":1.0}]);
}
#[test]
fn all_840_choices_agree_across_workers_and_resistance_constraint_selects_paid_ascendancy() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    chaos_goal(&mut problem);
    prepare(temp.path(), &problem);
    let serial = success(search(temp.path(), 1, 842).output().unwrap());
    let parallel = success(
        search(temp.path(), 4, 842)
            .args(["--export", "winner.xml"])
            .output()
            .unwrap(),
    );
    for report in [&serial, &parallel] {
        verified(report, 578);
        assert_eq!(report["tree_choices"].as_array().unwrap().len(), 105);
        assert_eq!(report["alternatives"].as_array().unwrap().len(), 840);
        assert_eq!(report["admission"]["checked_candidates"], 840);
        assert_eq!(report["admission"]["complete"], true);
        assert_eq!(
            report["requirements"]["legal_candidates"]
                .as_array()
                .unwrap()
                .len(),
            576
        );
        assert_eq!(
            report["requirements"]["rejected_candidates"]
                .as_array()
                .unwrap()
                .len(),
            264
        );
        assert_eq!(report["best_verified"]["candidate"]["class_id"], "10");
        assert_eq!(
            report["best_verified"]["candidate"]["ascendancy_id"],
            "Monk3"
        );
        assert!(
            report["best_verified"]["candidate"]["passives"]
                .as_array()
                .unwrap()
                .contains(&json!(24475))
        );
        assert_eq!(
            report["best_verified"]["assessment"]["constraints"][0]["observed"]["value"],
            7.0
        );
    }
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    assert_eq!(
        serial["search"]["infeasible"],
        parallel["search"]["infeasible"]
    );
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
    let fresh = success(
        cli(temp.path())
            .args([
                "evaluate",
                "winner.xml",
                "--backend",
                "native",
                "--raw",
                "--metric",
                "player.chaos_resistance_capped_pct",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        fresh["evaluation"]["measurements"][0]["value"]["value"],
        7.0
    );
    assert_eq!(
        fresh["evaluation"]["backend"],
        parallel["preparation"]["evaluation"]["backend"]
    );
    let tree: Value = serde_json::from_str(
        fresh["evaluation"]["attachments"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["media_type"] == "application/vnd.poe-optimizer.native-tree+json;version=3")
            .unwrap()["content"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(tree["ascendancy_allocated_count"], 1);
    assert_eq!(tree["point_budget_verified"], false);
    assert!(
        tree["paid_nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["physical_node_id"] == 24475 && n["allocation_kind"] == "ascendancy")
    );
}
#[test]
fn explicit_ascendancy_budget_and_independent_node_locks_precede_dispatch() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = input();
    p["tree_search"]["selections"] = json!([{"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":3936,"ascendancy_node_id":14960}]);
    p["tree_search"]["ascendancy_passive_points"] = json!(0);
    prepare(temp.path(), &p);
    let empty = success(search(temp.path(), 4, 3).output().unwrap());
    assert_eq!(empty["termination"], "empty_legal_domain");
    assert_eq!(empty["total_evaluations"], 0);
    assert_eq!(
        empty["admission"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    p["tree_search"]["ascendancy_passive_points"] = json!(1);
    p["locks"] = json!({"class_id":6,"ascendancy":{"kind":"id","id":"Warrior3"},"allocated_passives":[3936,14960],"weapon_id":"wooden-q0","support":"none"});
    prepare(temp.path(), &p);
    let locked = success(search(temp.path(), 4, 3).output().unwrap());
    verified(&locked, 3);
    assert_eq!(
        locked["best_verified"]["candidate"]["passives"],
        json!([3936, 14960])
    );
    p["tree_search"]["ordinary_passive_points"] = json!(0);
    prepare(temp.path(), &p);
    let empty = success(search(temp.path(), 4, 3).output().unwrap());
    assert_eq!(empty["total_evaluations"], 0);
    assert_eq!(empty["termination"], "empty_legal_domain");
}
#[test]
fn wrong_owners_unsupported_nodes_and_legacy_schema_ascendancy_choices_reject() {
    let temp = tempfile::tempdir().unwrap();
    let mut bad = Vec::new();
    for selection in [
        json!({"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":null,"ascendancy_node_id":24475}),
        json!({"class_id":6,"ascendancy_id":null,"entrance_node_id":null,"ascendancy_node_id":14960}),
        json!({"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":null,"ascendancy_node_id":5852}),
        json!({"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":null,"ascendancy_node_id":999999}),
    ] {
        let mut p = input();
        p["tree_search"]["selections"] = json!([selection]);
        bad.push(p);
    }
    let mut p = input();
    p["schema_version"] = json!(2);
    p["tree_search"]["ascendancy_passive_points"] = json!(0);
    p["tree_search"]["selections"] = json!([{"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":null,"ascendancy_node_id":14960}]);
    bad.push(p);
    let mut p = input();
    p["tree_search"]["ascendancy_passive_points"] = json!(2);
    bad.push(p);
    for p in bad {
        prepare(temp.path(), &p);
        let output = search(temp.path(), 1, 3)
            .args(["--export", "bad.xml"])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{p}");
        assert!(!temp.path().join("bad.xml").exists());
    }
}
#[test]
fn injected_signed_values_change_feasibility_and_export_provenance() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = input();
    chaos_goal(&mut p);
    p["tree_search"]["selections"] = json!([
        {"class_id":10,"ascendancy_id":"Monk3","entrance_node_id":null,"ascendancy_node_id":null},
        {"class_id":10,"ascendancy_id":"Monk3","entrance_node_id":null,"ascendancy_node_id":24475}
    ]);
    p["locks"] = json!({"weapon_id":"wooden-q0","support":"none"});
    prepare(temp.path(), &p);
    let reviewed = success(search(temp.path(), 1, 4).output().unwrap());
    verified(&reviewed, 4);
    for jobs in [1, 4] {
        let guided = success(
            search(temp.path(), jobs, 4)
                .args(["--strategy", "guided", "--seed", "17"])
                .output()
                .unwrap(),
        );
        verified(&guided, 4);
        assert_eq!(guided["best_verified"], reviewed["best_verified"]);
    }

    let mut package = bundled_snapshot().unwrap().package().clone();
    let effect = package
        .passive_effects
        .iter_mut()
        .find(|e| e.key.physical_node_id == 24475)
        .unwrap();
    assert!(effect.effects.is_empty());
    assert_eq!(
        effect.actor_modifiers[0].stat,
        poe_optimizer_data::game_data::ActorStat::ChaosResist
    );
    let poe_optimizer_data::game_data::ActorModifierEffect::Numeric { value, .. } =
        &mut effect.actor_modifiers[0].effect
    else {
        panic!("Expected source resistance BASE record")
    };
    *value = -7.5;
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let custom =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    fs::write(temp.path().join("custom.json"), bytes).unwrap();
    let rejected = success(
        search(temp.path(), 4, 4)
            .args(["--data", "custom.json", "--export", "infeasible.xml"])
            .output()
            .unwrap(),
    );
    assert!(rejected["best_verified"].is_null());
    assert!(!temp.path().join("infeasible.xml").exists());
    assert_eq!(
        rejected["data"]["identity"],
        serde_json::to_value(custom.identity()).unwrap()
    );
    assert_ne!(
        reviewed["catalog"]["identity"],
        rejected["catalog"]["identity"]
    );
    p["objective"]["constraints"] = json!([]);
    p["locks"]["allocated_passives"] = json!([24475]);
    prepare(temp.path(), &p);
    let saved = success(
        search(temp.path(), 4, 3)
            .args(["--data", "custom.json", "--export", "signed.xml"])
            .output()
            .unwrap(),
    );
    verified(&saved, 3);
    let fresh = success(
        cli(temp.path())
            .args([
                "evaluate",
                "signed.xml",
                "--backend",
                "native",
                "--data",
                "custom.json",
                "--metric",
                "player.chaos_resistance_capped_pct",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        fresh["evaluation"]["measurements"][0]["value"]["value"],
        -7.0
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(temp.path().join("signed.xml.data.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["backend"], fresh["evaluation"]["backend"]);
    assert_eq!(metadata["data_trust"]["status"], "custom_unreviewed");
}

#[cfg(feature = "pob")]
#[test]
fn optional_pob_search_verifies_two_paid_allocations_against_root_only_baseline() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = input();
    p["tree_search"]["selections"] = json!([{"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":3936,"ascendancy_node_id":14960}]);
    p["locks"] =
        json!({"weapon_id":"wooden-q0","support":"brutality_i","allocated_passives":[3936,14960]});
    prepare(temp.path(), &p);
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    let report = success(
        cli(temp.path())
            .args([
                "search-experimental",
                "--backend",
                "pob",
                "--problem",
                "problem.json",
                "--jobs",
                "2",
                "--max-evaluations",
                "3",
                "--timeout-seconds",
                "60",
                "--export",
                "pob.xml",
                "--pob",
            ])
            .arg(source)
            .output()
            .unwrap(),
    );
    assert_eq!(report["schema_version"], 4);
    assert_eq!(report["total_evaluations"], 3);
    assert_eq!(report["execution_kind"], "external_process");
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert_eq!(
        report["best_verified"]["candidate"]["passives"],
        json!([3936, 14960])
    );
    assert!(temp.path().join("pob.xml").exists());
}
