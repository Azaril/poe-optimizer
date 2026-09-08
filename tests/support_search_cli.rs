//! Joint data-driven support loadouts, exact locks, requirements and replay.
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot};
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
fn input() -> Value {
    let mut problem: Value =
        serde_json::from_str(include_str!("../examples/mace-support-search.json")).unwrap();
    problem["schema_version"] = json!(4);
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
        "120",
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
fn verified(report: &Value) {
    assert_eq!(report["schema_version"], 5);
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert!(!report["best_verified"].is_null());
}
fn small() -> Value {
    let mut p = input();
    p["objective"]["constraints"] = json!([]);
    p["tree_search"]["selections"] = json!([
        {"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":3936,"ascendancy_node_id":14960}
    ]);
    p
}
fn best(report: &Value) -> &Value {
    report["alternatives"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["candidate"] == report["best_verified"]["candidate"])
        .unwrap()
}
#[test]
fn all_loadouts_joint_search_matches_serial_and_rayon() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &input());
    let a = success(search(temp.path(), 1, 2942).output().unwrap());
    let b = success(search(temp.path(), 4, 2942).output().unwrap());
    for r in [&a, &b] {
        verified(r);
        assert_eq!(r["alternatives"].as_array().unwrap().len(), 2940);
        assert_eq!(r["admission"]["checked_candidates"], 2940);
        assert_eq!(r["admission"]["complete"], true);
        assert_eq!(
            r["candidate_constraints"]["budgets"]["supports_per_skill"],
            2
        );
        let legal = r["requirements"]["legal_candidates"]
            .as_array()
            .unwrap()
            .len();
        let rejected = r["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len();
        assert_eq!((legal, rejected), (1884, 1056));
        assert_eq!(r["total_evaluations"], legal + 2);
        assert_eq!(best(r)["support"].as_array().unwrap().len(), 2);
        assert_eq!(
            r["best_verified"]["assessment"]["constraints"][0]["observed"]["value"],
            7.0
        );
    }
    assert_eq!(a["search"]["feasible"], b["search"]["feasible"]);
    assert_eq!(a["search"]["infeasible"], b["search"]["infeasible"]);
    assert_eq!(a["best_verified"], b["best_verified"]);
}
#[test]
fn tiny_guided_and_exhaustive_loadout_archives_and_partial_budgets_agree() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &small());
    let exhaustive = success(search(temp.path(), 1, 30).output().unwrap());
    verified(&exhaustive);
    assert_eq!(exhaustive["total_evaluations"], 30);
    for jobs in [1, 4] {
        let guided = success(
            search(temp.path(), jobs, 30)
                .args(["--strategy", "guided", "--seed", "73"])
                .output()
                .unwrap(),
        );
        verified(&guided);
        assert_eq!(guided["best_verified"], exhaustive["best_verified"]);
        assert_eq!(
            guided["search"]["feasible"],
            exhaustive["search"]["feasible"]
        );
        let partial = success(search(temp.path(), jobs, 3).output().unwrap());
        verified(&partial);
        assert_eq!(partial["total_evaluations"], 3);
        assert_eq!(partial["termination"], "evaluation_budget");
    }
}
#[test]
fn exact_pair_lock_canonicalizes_order_and_reimports_native_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = small();
    p["locks"] =
        json!({"weapon_id":"smithing-q20","support_loadout":["rapid_attacks_i","heavy_swing"]});
    prepare(temp.path(), &p);
    let report = success(
        search(temp.path(), 4, 3)
            .args(["--export", "winner.xml"])
            .output()
            .unwrap(),
    );
    verified(&report);
    assert_eq!(report["total_evaluations"], 3);
    assert_eq!(
        best(&report)["support"],
        json!(["heavy_swing", "rapid_attacks_i"])
    );
    let export = fs::read_to_string(temp.path().join("winner.xml")).unwrap();
    assert_eq!(export.matches("<Gem ").count(), 3);
    let fresh = success(
        cli(temp.path())
            .args([
                "evaluate",
                "winner.xml",
                "--backend",
                "native",
                "--metric",
                "player.selected_hit_dps",
            ])
            .output()
            .unwrap(),
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(temp.path().join("winner.xml.data.json")).unwrap())
            .unwrap();
    assert_eq!(fresh["evaluation"]["backend"], metadata["backend"]);
    assert_eq!(
        fresh["evaluation"]["measurements"][0]["value"]["value"],
        report["best_verified"]["assessment"]["objective_value"]["value"]
    );
}
#[test]
fn schema_mixing_unknown_and_repeated_supports_reject_without_exports() {
    let temp = tempfile::tempdir().unwrap();
    let mut invalid = Vec::new();
    for loadouts in [
        json!([]),
        json!([["unknown"]]),
        json!([["heavy_swing", "heavy_swing"]]),
        json!([["brutality_i", "heavy_swing", "rapid_attacks_i"]]),
        json!([
            ["heavy_swing", "rapid_attacks_i"],
            ["rapid_attacks_i", "heavy_swing"]
        ]),
    ] {
        let mut p = small();
        p["support_loadouts"] = loadouts;
        invalid.push(p);
    }
    let mut p = small();
    p["supports"] = json!(["none"]);
    invalid.push(p);
    let mut p = small();
    p["locks"]["support"] = json!("none");
    invalid.push(p);
    let mut p = small();
    p["schema_version"] = json!(3);
    invalid.push(p);
    let mut p = small();
    p["locks"]["support_loadout"] = json!(["unknown"]);
    invalid.push(p);
    for p in invalid {
        prepare(temp.path(), &p);
        let out = search(temp.path(), 1, 3)
            .args(["--export", "never.xml"])
            .output()
            .unwrap();
        assert!(!out.status.success(), "unexpected success: {p}");
        assert!(!temp.path().join("never.xml").exists());
    }
}
#[test]
fn per_color_costs_combine_with_item_requirements_by_maximum_and_empty_domains_spend_zero() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = small();
    p["locks"] = json!({"weapon_id":"wooden-q0"});
    let mut package = bundled_snapshot().unwrap().package().clone();
    let strength = package.tree.class(6).unwrap().base_strength;
    package.mace.support_attribute_costs.strength = strength;
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default()).unwrap();
    fs::write(temp.path().join("custom.json"), bytes).unwrap();
    // One red socket is legal even alongside item and main-skill requirements;
    // two red sockets accumulate, while red/green costs remain separate.
    p["locks"]["support_loadout"] = json!(["heavy_swing", "rapid_attacks_i"]);
    prepare(temp.path(), &p);
    let legal = success(
        search(temp.path(), 4, 3)
            .args(["--data", "custom.json"])
            .output()
            .unwrap(),
    );
    verified(&legal);
    p["locks"]["support_loadout"] = json!(["brutality_i", "heavy_swing"]);
    prepare(temp.path(), &p);
    let empty = success(
        search(temp.path(), 4, 3)
            .args(["--data", "custom.json", "--export", "never.xml"])
            .output()
            .unwrap(),
    );
    assert_eq!(empty["termination"], "empty_legal_domain");
    assert_eq!(empty["total_evaluations"], 0);
    assert_eq!(
        empty["requirements"]["rejected_candidates"][0]["assessment"]["required"]["strength"],
        2 * strength
    );
    assert!(!temp.path().join("never.xml").exists());
}

#[test]
fn injected_support_modifiers_change_result_and_export_replays_selected_data() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = small();
    p["locks"] =
        json!({"weapon_id":"smithing-q20","support_loadout":["heavy_swing","rapid_attacks_i"]});
    prepare(temp.path(), &p);
    let before = success(search(temp.path(), 1, 3).output().unwrap());
    let mut package = bundled_snapshot().unwrap().package().clone();
    let rapid = package
        .supports
        .iter_mut()
        .find(|s| s.id == "rapid_attacks_i")
        .unwrap();
    rapid.modifiers[0].value = 115.0;
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    fs::write(temp.path().join("custom.json"), bytes).unwrap();
    let changed = success(
        search(temp.path(), 4, 3)
            .args(["--data", "custom.json", "--export", "custom.xml"])
            .output()
            .unwrap(),
    );
    verified(&changed);
    assert!(
        changed["best_verified"]["assessment"]["objective_value"]["value"]
            .as_f64()
            .unwrap()
            > before["best_verified"]["assessment"]["objective_value"]["value"]
                .as_f64()
                .unwrap()
    );
    assert_eq!(
        changed["data"]["identity"],
        serde_json::to_value(snapshot.identity()).unwrap()
    );
    assert_ne!(
        before["catalog"]["identity"],
        changed["catalog"]["identity"]
    );
    let replay = success(
        cli(temp.path())
            .args([
                "evaluate",
                "custom.xml",
                "--backend",
                "native",
                "--data",
                "custom.json",
                "--metric",
                "player.selected_hit_dps",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(
        replay["evaluation"]["measurements"][0]["value"]["value"],
        changed["best_verified"]["assessment"]["objective_value"]["value"]
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(temp.path().join("custom.xml.data.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["backend"], replay["evaluation"]["backend"]);
    assert_eq!(metadata["data_trust"]["status"], "custom_unreviewed");
    let after = success(search(temp.path(), 1, 3).output().unwrap());
    assert_eq!(before["best_verified"], after["best_verified"]);
}

#[test]
fn explicit_empty_lock_removes_template_pair_and_tiny_proposals_keep_full_admission() {
    let temp = tempfile::tempdir().unwrap();
    let mut p = small();
    p["locks"] =
        json!({"weapon_id":"wooden-q0","support_loadout":["heavy_swing","rapid_attacks_i"]});
    prepare(temp.path(), &p);
    let paired = success(
        search(temp.path(), 1, 3)
            .args(["--export", "paired.xml"])
            .output()
            .unwrap(),
    );
    verified(&paired);
    p["template"] = json!("paired.xml");
    p["locks"]["support_loadout"] = json!([]);
    prepare(temp.path(), &p);
    let empty = success(
        search(temp.path(), 4, 3)
            .args(["--export", "empty.xml"])
            .output()
            .unwrap(),
    );
    verified(&empty);
    assert_eq!(best(&empty)["support"], json!([]));
    assert_eq!(
        fs::read_to_string(temp.path().join("empty.xml"))
            .unwrap()
            .matches("<Gem ")
            .count(),
        1
    );
    p["locks"] = json!({});
    prepare(temp.path(), &p);
    let limited = success(
        search(temp.path(), 4, 3)
            .args([
                "--strategy",
                "guided",
                "--max-proposals",
                "1",
                "--max-rounds",
                "1",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(limited["admission"]["checked_candidates"], 28);
    assert_eq!(limited["admission"]["complete"], true);
    assert_eq!(limited["total_evaluations"], 3);
}
