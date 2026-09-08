//! Joint class/ascendancy/entrance search through the public native-only CLI seam.
use poe_optimizer_core::candidate::Candidate;
use poe_optimizer_data::{
    class_tree::{ClassTreeSelection, selections},
    game_data::{GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy, bundled_snapshot},
};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    fs,
    path::Path,
    process::{Command, Output},
    sync::Arc,
};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");

fn input() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/mace-search.json")).unwrap();
    value["schema_version"] = json!(2);
    value["template"] = json!("build.xml");
    value["tree_search"] = json!({"ordinary_passive_points":1,"ascendancy_passive_points":0});
    value
}
fn prepare(directory: &Path, problem: &Value, template: &str) {
    fs::write(
        directory.join("problem.json"),
        serde_json::to_vec(problem).unwrap(),
    )
    .unwrap();
    fs::write(directory.join("build.xml"), template).unwrap();
}
fn cli(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(directory);
    command
}
fn search(directory: &Path, strategy: &str, jobs: usize, max: usize) -> Command {
    let mut command = cli(directory);
    command.args([
        "search-experimental",
        "--backend",
        "native",
        "--problem",
        "problem.json",
        "--strategy",
        strategy,
        "--seed",
        "17",
        "--timeout-seconds",
        "30",
        "--pob",
        "absent-reference-checkout",
    ]);
    command
        .arg("--jobs")
        .arg(jobs.to_string())
        .arg("--max-evaluations")
        .arg(max.to_string());
    command
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stderr: {}\nstdout: {}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn verified(report: &Value, total: usize) {
    assert_eq!(report["schema_version"], 3);
    assert_eq!(report["requested_backend"], "native-poe2");
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["total_evaluations"], total);
    assert_eq!(report["preparation"]["attempts"], 1);
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["statistics"]["discarded_late"], 0);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert_eq!(report["diagnostic_only"], true);
    assert_eq!(report["best_verified"]["diagnostic_only"], true);
}
fn best_alternative(report: &Value) -> &Value {
    report["alternatives"]
        .as_array()
        .unwrap()
        .iter()
        .find(|alternative| alternative["id"] == report["best_verified"]["alternative_id"])
        .unwrap()
}
fn write_package(directory: &Path, mut package: GameDataPackage) -> Value {
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let loaded =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    fs::write(directory.join("data.json"), bytes).unwrap();
    serde_json::to_value(loaded.identity()).unwrap()
}

#[test]
fn full_744_state_domain_filters_class_requirements_and_matches_across_workers() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &input(), TEMPLATE);
    let serial = success(search(temp.path(), "exhaustive", 1, 746).output().unwrap());
    let parallel = success(search(temp.path(), "exhaustive", 4, 746).output().unwrap());
    for report in [&serial, &parallel] {
        verified(report, 506);
        assert_eq!(report["termination"], "finite_domain_processed");
        assert_eq!(report["tree_choices"].as_array().unwrap().len(), 93);
        assert_eq!(report["alternatives"].as_array().unwrap().len(), 744);
        assert_eq!(
            report["requirements"]["legal_candidates"]
                .as_array()
                .unwrap()
                .len(),
            504
        );
        let rejected = report["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap();
        assert_eq!(rejected.len(), 240);
        for entry in rejected {
            assert!(
                entry["alternative_id"]
                    .as_str()
                    .unwrap()
                    .contains("/smithing-")
            );
            assert_eq!(entry["assessment"]["available"]["strength"], 7);
            assert_eq!(entry["assessment"]["required"]["strength"], 11);
            assert_eq!(
                entry["assessment"]["violations"],
                json!([{"requirement":"strength","required":11,"available":7}])
            );
        }
        let classes: BTreeSet<_> = report["tree_choices"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tree| tree["class_id"].as_u64().unwrap())
            .collect();
        assert_eq!(classes, BTreeSet::from([1, 2, 6, 7, 8, 9, 10, 11]));
    }
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
}

#[test]
fn explicit_point_budgets_and_allocation_locks_precede_any_calculation() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    problem["tree_search"]["ordinary_passive_points"] = json!(0);
    prepare(temp.path(), &problem, TEMPLATE);
    let roots = success(search(temp.path(), "exhaustive", 4, 250).output().unwrap());
    verified(&roots, 170);
    assert_eq!(roots["tree_choices"].as_array().unwrap().len(), 31);
    assert!(
        roots["tree_choices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|tree| tree["entrance_node_id"].is_null())
    );
    problem["tree_search"]["selections"] =
        json!([{"class_id":1,"ascendancy_id":"Witch3b","entrance_node_id":4739}]);
    prepare(temp.path(), &problem, TEMPLATE);
    let empty = success(search(temp.path(), "exhaustive", 4, 3).output().unwrap());
    assert_eq!(empty["termination"], "empty_legal_domain");
    assert_eq!(empty["total_evaluations"], 0);
    assert_eq!(empty["preparation"]["attempts"], 0);
    assert!(empty["search"].is_null());
    assert!(empty["best_verified"].is_null());
    assert_eq!(
        empty["admission"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    problem["tree_search"]["ordinary_passive_points"] = json!(1);
    problem["locks"] = json!({"class_id":1,"ascendancy":{"kind":"id","id":"Witch3b"},"allocated_passives":[4739],"weapon_id":"wooden-q0","support":"brutality_i"});
    prepare(temp.path(), &problem, TEMPLATE);
    let locked = success(search(temp.path(), "guided", 4, 3).output().unwrap());
    verified(&locked, 3);
    let candidate = &locked["best_verified"]["candidate"];
    assert_eq!(candidate["class_id"], "1");
    assert_eq!(candidate["ascendancy_id"], "Witch3b");
    assert_eq!(candidate["passives"], json!([4739]));
    assert_eq!(best_alternative(&locked)["weapon_id"], "wooden-q0");
    assert_eq!(best_alternative(&locked)["support"], "brutality_i");
}

#[test]
fn invalid_identity_ownership_and_schema_inputs_never_publish_artifacts() {
    let temp = tempfile::tempdir().unwrap();
    let mut variants = Vec::new();
    for tree in [
        json!({"class_id":6,"ascendancy_id":"Witch3b","entrance_node_id":null}),
        json!({"class_id":6,"ascendancy_id":null,"entrance_node_id":4739}),
        json!({"class_id":999,"ascendancy_id":null,"entrance_node_id":null}),
        json!({"class_id":1,"ascendancy_id":null,"entrance_node_id":54447}),
    ] {
        let mut problem = input();
        problem["tree_search"]["selections"] = json!([tree]);
        variants.push(problem);
    }
    let mut problem = input();
    problem["tree_search"]["ascendancy_passive_points"] = json!(1);
    variants.push(problem);
    let mut problem = input();
    problem["tree_search"]["ordinary_passive_points"] = json!(2);
    variants.push(problem);
    let mut problem = input();
    problem["schema_version"] = json!(1);
    variants.push(problem);
    let mut problem = input();
    problem.as_object_mut().unwrap().remove("tree_search");
    variants.push(problem);
    let mut problem = input();
    problem["locks"] = json!({"allocated_passives":[4739],"unallocated_passives":[4739]});
    variants.push(problem);
    let mut problem = input();
    problem["locks"] = json!({"unallocated_passives":[999999]});
    variants.push(problem);
    let mut problem = input();
    problem["schema_version"] = json!(1);
    problem.as_object_mut().unwrap().remove("tree_search");
    problem["locks"] = json!({"class_id":6});
    variants.push(problem);
    for (index, problem) in variants.into_iter().enumerate() {
        prepare(temp.path(), &problem, TEMPLATE);
        let output = search(temp.path(), "exhaustive", 4, 3)
            .args(["--output", "never.json", "--export", "never.xml"])
            .output()
            .unwrap();
        assert!(
            !output.status.success(),
            "case {index} unexpectedly succeeded: {}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(!output.stderr.is_empty(), "case {index} had no diagnostic");
        assert!(!temp.path().join("never.json").exists());
        assert!(!temp.path().join("never.xml").exists());
        assert!(!temp.path().join("never.xml.data.json").exists());
    }
}

#[test]
fn guided_search_is_reproducible_for_joint_choices_and_honors_a_tight_budget() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    problem["tree_search"]["selections"] = json!([
        {"class_id":1,"ascendancy_id":"Witch3b","entrance_node_id":4739},
        {"class_id":6,"ascendancy_id":null,"entrance_node_id":null},
        {"class_id":8,"ascendancy_id":"Huntress1","entrance_node_id":56651},
        {"class_id":9,"ascendancy_id":null,"entrance_node_id":null}
    ]);
    prepare(temp.path(), &problem, TEMPLATE);
    let exhaustive = success(search(temp.path(), "exhaustive", 1, 34).output().unwrap());
    let serial = success(search(temp.path(), "guided", 1, 34).output().unwrap());
    let parallel = success(search(temp.path(), "guided", 4, 34).output().unwrap());
    for report in [&exhaustive, &serial, &parallel] {
        verified(report, 26);
        assert_eq!(
            report["requirements"]["legal_candidates"]
                .as_array()
                .unwrap()
                .len(),
            24
        );
        assert_eq!(report["best_verified"], exhaustive["best_verified"]);
    }
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    let limited = success(search(temp.path(), "guided", 4, 3).output().unwrap());
    verified(&limited, 3);
    assert_eq!(limited["termination"], "evaluation_budget");
    assert_eq!(limited["search"]["feasible"].as_array().unwrap().len(), 1);
    problem["locks"] = json!({"class_id":6,"ascendancy":{"kind":"none"}});
    prepare(temp.path(), &problem, TEMPLATE);
    let unascended = success(search(temp.path(), "exhaustive", 4, 10).output().unwrap());
    verified(&unascended, 10);
    assert_eq!(
        unascended["tree_choices"],
        json!([{"class_id":6,"ascendancy_id":null,"entrance_node_id":null}])
    );
    assert!(unascended["best_verified"]["candidate"]["ascendancy_id"].is_null());
}

#[test]
fn custom_data_changes_joint_ranking_and_survives_fresh_export_evaluation() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    problem["tree_search"]["selections"] = json!([
        {"class_id":1,"ascendancy_id":"Witch3b","entrance_node_id":4739},
        {"class_id":6,"ascendancy_id":null,"entrance_node_id":null}
    ]);
    prepare(temp.path(), &problem, TEMPLATE);
    let default = success(search(temp.path(), "exhaustive", 1, 18).output().unwrap());
    assert_eq!(best_alternative(&default)["weapon_id"], "smithing-q20");
    let mut package = bundled_snapshot().unwrap().package().clone();
    let wood = package
        .weapons
        .iter_mut()
        .find(|weapon| weapon.name == "Wooden Club")
        .unwrap();
    wood.physical_minimum *= 10.0;
    wood.physical_maximum *= 10.0;
    let identity = write_package(temp.path(), package);
    let serial = success(
        search(temp.path(), "exhaustive", 1, 18)
            .args(["--data", "data.json", "--export", "winner.xml"])
            .output()
            .unwrap(),
    );
    let parallel = success(
        search(temp.path(), "exhaustive", 4, 18)
            .args(["--data", "data.json"])
            .output()
            .unwrap(),
    );
    for report in [&serial, &parallel] {
        verified(report, 14);
        assert_eq!(report["data"]["identity"], identity);
        assert_eq!(report["data"]["trust"]["status"], "custom_unreviewed");
        assert_eq!(report["data"]["uses_packaged_default"], false);
        assert_eq!(best_alternative(report)["weapon_id"], "wooden-q20");
        assert_eq!(best_alternative(report)["support"], "brutality_i");
    }
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
    assert_ne!(
        serial["catalog"]["identity"],
        default["catalog"]["identity"]
    );
    let rechecked = success(
        cli(temp.path())
            .args([
                "evaluate",
                "winner.xml",
                "--backend",
                "native",
                "--data",
                "data.json",
                "--metric",
                "player.selected_hit_dps",
            ])
            .output()
            .unwrap(),
    );
    assert_eq!(rechecked["evaluation"]["backend"]["data"], identity);
    assert_eq!(
        rechecked["evaluation"]["measurements"][0]["value"]["value"],
        serial["best_verified"]["assessment"]["objective_value"]["value"]
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(temp.path().join("winner.xml.data.json")).unwrap())
            .unwrap();
    assert_eq!(metadata["backend"], rechecked["evaluation"]["backend"]);
    assert_eq!(metadata["data_trust"], serial["data"]["trust"]);
    assert_eq!(
        metadata["xml_sha256"],
        serial["best_verified"]["source_xml_sha256"]
    );
}

#[test]
fn locked_override_export_preserves_unrelated_source_and_resolves_exact_tree_identity() {
    let temp = tempfile::tempdir().unwrap();
    let template = TEMPLATE.replace(
        "  <Tree",
        "  <!-- keep café &amp; source layout -->\n  <Tree",
    );
    let mut problem = input();
    problem["locks"] = json!({"class_id":1,"ascendancy":{"kind":"id","id":"Witch3b"},"allocated_passives":[4739],"weapon_id":"wooden-q0","support":"brutality_i"});
    prepare(temp.path(), &problem, &template);
    let report = success(
        search(temp.path(), "exhaustive", 4, 3)
            .args(["--export", "winner.xml"])
            .output()
            .unwrap(),
    );
    verified(&report, 3);
    let snapshot = Arc::new(bundled_snapshot().unwrap());
    let trees = selections(&snapshot.package().tree).unwrap();
    let weapons: Vec<NormalMaceAlternative> =
        serde_json::from_value(problem["weapons"].clone()).unwrap();
    let catalog = ControlledMaceCatalog::with_tree_choices(
        snapshot,
        template,
        weapons,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        trees,
    )
    .unwrap();
    let candidate: Candidate =
        serde_json::from_value(report["best_verified"]["candidate"].clone()).unwrap();
    let imported =
        poe_optimizer_import::decode_build(&fs::read(temp.path().join("winner.xml")).unwrap())
            .unwrap();
    assert_eq!(
        imported.xml,
        catalog.materialize(&candidate).unwrap().content
    );
    assert_eq!(
        imported.sha256,
        report["best_verified"]["source_xml_sha256"]
    );
    assert!(
        imported
            .xml
            .contains("<!-- keep café &amp; source layout -->")
    );
    let rechecked = success(
        cli(temp.path())
            .args([
                "evaluate",
                "winner.xml",
                "--backend",
                "native",
                "--metric",
                "player.selected_hit_dps",
                "--raw",
            ])
            .output()
            .unwrap(),
    );
    let result = &rechecked["evaluation"];
    assert_eq!(result["build"]["class_name"], "Witch");
    assert_eq!(result["build"]["ascendancy_name"], "Abyssal Lich");
    assert_eq!(
        result["build"]["allocated_nodes"],
        json!([4739, 23710, 54447])
    );
    assert_eq!(
        result["measurements"][0]["value"]["value"],
        report["best_verified"]["assessment"]["objective_value"]["value"]
    );
    let attachment = result["attachments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| {
            entry["media_type"] == "application/vnd.poe-optimizer.native-tree+json;version=1"
        })
        .unwrap();
    let projection: Value = serde_json::from_str(attachment["content"].as_str().unwrap()).unwrap();
    assert_eq!(projection["paid_nodes"][0]["physical_node_id"], 4739);
    assert_eq!(projection["paid_nodes"][0]["effective_node_id"], 17306);
    assert_eq!(projection["point_budget_verified"], false);
    let selected: ClassTreeSelection =
        serde_json::from_value(best_alternative(&report)["tree"].clone()).unwrap();
    assert_eq!(selected.class_id, 1);
    assert_eq!(selected.ascendancy_id.as_deref(), Some("Witch3b"));
    assert_eq!(selected.entrance_node_id, Some(4739));
}

#[cfg(feature = "pob")]
#[test]
fn optional_reference_backend_searches_joint_choices_and_checks_its_fresh_finalist() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    problem["weapons"] = json!([problem["weapons"][0].clone()]);
    problem["tree_search"]["selections"] = json!([
        {"class_id":1,"ascendancy_id":"Witch3b","entrance_node_id":4739},
        {"class_id":2,"ascendancy_id":null,"entrance_node_id":null}
    ]);
    prepare(temp.path(), &problem, TEMPLATE);
    let native = success(search(temp.path(), "exhaustive", 2, 6).output().unwrap());
    verified(&native, 6);
    let reference = success(
        cli(temp.path())
            .args([
                "search-experimental",
                "--backend",
                "pob",
                "--problem",
                "problem.json",
                "--jobs",
                "2",
                "--strategy",
                "exhaustive",
                "--seed",
                "17",
                "--max-evaluations",
                "6",
                "--timeout-seconds",
                "90",
                "--pob",
            ])
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2"))
            .args(["--export", "pob-winner.xml"])
            .output()
            .unwrap(),
    );
    assert_eq!(reference["schema_version"], 3);
    assert_eq!(reference["requested_backend"], "pob-poe2-mlua");
    assert_eq!(reference["execution_kind"], "external_process");
    assert_eq!(reference["total_evaluations"], 6);
    assert_eq!(reference["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(
        reference["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(reference["search"]["verifications"][0]["consistent"], true);
    assert_eq!(
        reference["best_verified"]["candidate"],
        native["best_verified"]["candidate"]
    );
    assert_eq!(
        reference["best_verified"]["alternative_id"],
        native["best_verified"]["alternative_id"]
    );
    let a = native["best_verified"]["assessment"]["objective_value"]["value"]
        .as_f64()
        .unwrap();
    let b = reference["best_verified"]["assessment"]["objective_value"]["value"]
        .as_f64()
        .unwrap();
    assert!((a - b).abs() < 1e-8);
    let imported =
        poe_optimizer_import::decode_build(&fs::read(temp.path().join("pob-winner.xml")).unwrap())
            .unwrap();
    assert_eq!(
        imported.sha256,
        reference["best_verified"]["source_xml_sha256"]
    );
    assert_eq!(reference["export"]["status"], "written");
}

#[test]
fn forbidden_passive_locks_keep_other_choices_and_class_requirements_can_empty_the_domain() {
    let temp = tempfile::tempdir().unwrap();
    let mut problem = input();
    problem["locks"] = json!({"class_id":1,"ascendancy":{"kind":"id","id":"Witch3b"},"unallocated_passives":[4739]});
    prepare(temp.path(), &problem, TEMPLATE);
    let report = success(search(temp.path(), "exhaustive", 4, 18).output().unwrap());
    verified(&report, 10);
    assert_eq!(report["tree_choices"].as_array().unwrap().len(), 2);
    assert!(
        report["tree_choices"]
            .as_array()
            .unwrap()
            .iter()
            .all(|choice| choice["entrance_node_id"] != 4739)
    );
    assert!(
        report["tree_choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["entrance_node_id"].is_null())
    );
    assert!(
        report["tree_choices"]
            .as_array()
            .unwrap()
            .iter()
            .any(|choice| choice["entrance_node_id"].is_number())
    );
    problem["locks"]["weapon_id"] = json!("smithing-q0");
    problem["locks"]["support"] = json!("brutality_i");
    prepare(temp.path(), &problem, TEMPLATE);
    let empty = success(
        search(temp.path(), "exhaustive", 4, 3)
            .args(["--export", "never.xml"])
            .output()
            .unwrap(),
    );
    assert_eq!(empty["termination"], "empty_legal_domain");
    assert_eq!(empty["total_evaluations"], 0);
    assert_eq!(empty["preparation"]["attempts"], 0);
    assert_eq!(empty["requirements"]["legal_candidates"], json!([]));
    assert_eq!(
        empty["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    for rejected in empty["requirements"]["rejected_candidates"]
        .as_array()
        .unwrap()
    {
        assert_eq!(rejected["assessment"]["available"]["strength"], 7);
        assert_eq!(rejected["assessment"]["required"]["strength"], 11);
    }
    assert!(empty["search"].is_null());
    assert!(empty["best_verified"].is_null());
    assert!(!temp.path().join("never.xml").exists());
    assert!(!temp.path().join("never.xml.data.json").exists());
}

#[test]
fn native_realization_rejects_tampered_tree_identity_and_injected_effect_evidence() {
    use poe_optimizer_core::{evaluation::*, options::EvaluationOptions};
    use poe_optimizer_native::NativeBackend;
    let snapshot = Arc::new(bundled_snapshot().unwrap());
    let trees = selections(&snapshot.package().tree).unwrap();
    let weapons: Vec<NormalMaceAlternative> =
        serde_json::from_value(input()["weapons"].clone()).unwrap();
    let catalog = ControlledMaceCatalog::with_tree_choices(
        snapshot,
        TEMPLATE.into(),
        weapons,
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
        trees,
    )
    .unwrap();
    let backend = NativeBackend::new();
    let identity = backend.identity();
    let native = Engine::new(backend);
    let request = |build| EvaluationRequest {
        build,
        options: EvaluationOptions::default(),
        metrics: vec![],
    };
    let budget = EvaluationBudget { timeout_ms: 30_000 };
    let baseline = native
        .evaluate(&request(catalog.template_build()), budget)
        .unwrap();
    let scenario = catalog.bind_native_baseline(&baseline, &identity).unwrap();
    let selection = ClassTreeSelection {
        class_id: 1,
        ascendancy_id: Some("Witch3b".into()),
        entrance_node_id: Some(4739),
    };
    let candidate = catalog
        .resolve_tree_candidate(&selection, "wooden-q0", MaceSupportChoice::BrutalityI)
        .unwrap();
    let result = native
        .evaluate(&request(catalog.materialize(candidate).unwrap()), budget)
        .unwrap();
    catalog
        .validate_native_realization(candidate, &result, &scenario)
        .unwrap();
    let saved = serde_json::to_value(&result).unwrap();
    for (pointer, replacement) in [
        ("/class/internal_id", json!(6)),
        ("/ascendancy/internal_id", json!("Witch3")),
        ("/paid_nodes/0/physical_node_id", json!(56651)),
        ("/paid_nodes/0/effective_node_id", json!(39263)),
        ("/configured_effects", json!([])),
        ("/data_identity/content_sha256", json!("0".repeat(64))),
    ] {
        let mut tampered: EvaluationResult = serde_json::from_value(saved.clone()).unwrap();
        let attachment = tampered
            .attachments
            .iter_mut()
            .find(|entry| {
                entry.media_type == "application/vnd.poe-optimizer.native-tree+json;version=1"
            })
            .unwrap();
        let mut diagnostic: Value = serde_json::from_str(&attachment.content).unwrap();
        let field = diagnostic
            .pointer_mut(pointer)
            .unwrap_or_else(|| panic!("fixture lacks {pointer}"));
        assert_ne!(*field, replacement);
        *field = replacement;
        attachment.content = serde_json::to_string(&diagnostic).unwrap();
        assert!(
            catalog
                .validate_native_realization(candidate, &tampered, &scenario)
                .is_err(),
            "accepted tampered {pointer}"
        );
    }
    let mut tampered: EvaluationResult = serde_json::from_value(saved).unwrap();
    tampered.build.allocated_nodes.retain(|node| *node != 4739);
    assert!(
        catalog
            .validate_native_realization(candidate, &tampered, &scenario)
            .is_err()
    );
}
