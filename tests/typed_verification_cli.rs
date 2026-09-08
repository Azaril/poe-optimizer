//! Independent CLI checks that prepared scoring retains document-finalist verification.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
fn problem() -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/mace-support-search.json")).unwrap();
    value["template"] = json!("build.xml");
    value["tree_search"]["selections"] = json!([
        {"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":3936,"ascendancy_node_id":14960},
        {"class_id":10,"ascendancy_id":"Monk3","entrance_node_id":10364,"ascendancy_node_id":24475}
    ]);
    value
}
fn prepare(directory: &Path, problem: &Value) {
    fs::write(
        directory.join("problem.json"),
        serde_json::to_vec(problem).unwrap(),
    )
    .unwrap();
    fs::write(directory.join("build.xml"), TEMPLATE).unwrap();
}
fn cli(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(directory);
    command
}
fn search(directory: &Path, mode: &str, jobs: usize, max_evaluations: usize) -> Command {
    let mut command = cli(directory);
    command
        .args([
            "search-experimental",
            "--backend",
            "native",
            "--native-evaluation",
            mode,
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
        .arg(max_evaluations.to_string());
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
fn check_verified(report: &Value, total: usize) {
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["preparation"]["attempts"], 1);
    assert_eq!(report["total_evaluations"], total);
    assert_eq!(report["search"]["statistics"]["evaluations"], total - 1);
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["statistics"]["discarded_late"], 0);
    let verifications = report["search"]["verifications"].as_array().unwrap();
    assert_eq!(verifications.len(), 1);
    assert_eq!(verifications[0]["consistent"], true);
    assert_eq!(
        verifications[0]["candidate"],
        report["search"]["feasible"][0]["candidate"]
    );
    assert_eq!(
        verifications[0]["fresh_assessment"],
        report["best_verified"]["assessment"]
    );
    assert_eq!(report["best_verified"]["diagnostic_only"], true);
    assert_eq!(report["export"]["status"], "written");
}
fn same_search_evidence(a: &Value, b: &Value) {
    for key in ["feasible", "infeasible", "verifications", "statistics"] {
        assert_eq!(a["search"][key], b["search"][key], "search.{key}");
    }
    for key in [
        "best_verified",
        "total_evaluations",
        "termination",
        "requirements",
        "admission",
        "alternatives",
        "catalog",
        "warnings",
    ] {
        assert_eq!(a[key], b[key], "{key}");
    }
}
fn manifest(directory: &Path, filename: &str) -> Value {
    serde_json::from_slice(&fs::read(directory.join(format!("{filename}.data.json"))).unwrap())
        .unwrap()
}

#[test]
fn complete_typed_and_document_search_match_both_archives_finalists_and_exact_exports() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    prepare(directory, &problem());
    let mut reference = None;
    for mode in ["document", "typed"] {
        for jobs in [1, 4] {
            let filename = format!("{mode}-{jobs}.xml");
            let report = success(
                search(directory, mode, jobs, 100)
                    .args(["--export", &filename])
                    .output()
                    .unwrap(),
            );
            check_verified(&report, 42);
            assert_eq!(
                report["calculation_path"],
                if mode == "typed" {
                    "native_typed_candidates"
                } else {
                    "native_documents"
                }
            );
            if mode == "typed" {
                assert_eq!(report["native_candidate_preparation"]["calculations"], 0);
                assert_eq!(
                    report["native_candidate_preparation"]["admitted_handles"],
                    40
                );
                assert_eq!(
                    report["native_candidate_preparation"]["caches_results"],
                    false
                );
            }
            assert_eq!(report["termination"], "finite_domain_processed");
            assert_eq!(report["admission"]["checked_candidates"], 56);
            assert_eq!(
                report["requirements"]["legal_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                40
            );
            assert!(!report["search"]["feasible"].as_array().unwrap().is_empty());
            assert!(
                !report["search"]["infeasible"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
            let source = fs::read(directory.join(&filename)).unwrap();
            let metadata = manifest(directory, &filename);
            assert_eq!(
                metadata["xml_sha256"],
                report["best_verified"]["source_xml_sha256"]
            );
            assert_eq!(
                metadata["backend"],
                report["preparation"]["evaluation"]["backend"]
            );
            if let Some((expected_report, expected_source, expected_metadata)) = &reference {
                same_search_evidence(expected_report, &report);
                assert_eq!(expected_source, &source);
                assert_eq!(expected_metadata, &metadata);
            } else {
                reference = Some((report, source, metadata));
            }
        }
    }
}

#[test]
fn tight_budgets_include_one_fresh_document_finalist_and_do_not_add_export_calculations() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let mut input = problem();
    input["objective"]["constraints"] = json!([]);
    prepare(directory, &input);
    for total in [3, 7] {
        let mut reference = None;
        for mode in ["document", "typed"] {
            for jobs in [1, 4] {
                let filename = format!("{mode}-{jobs}-{total}.xml");
                let report = success(
                    search(directory, mode, jobs, total)
                        .args(["--export", &filename])
                        .output()
                        .unwrap(),
                );
                check_verified(&report, total);
                assert_eq!(report["termination"], "evaluation_budget");
                assert_eq!(report["admission"]["checked_candidates"], 56);
                assert_eq!(report["admission"]["complete"], true);
                let source = fs::read(directory.join(&filename)).unwrap();
                if let Some((expected_report, expected_source)) = &reference {
                    same_search_evidence(expected_report, &report);
                    assert_eq!(expected_source, &source);
                } else {
                    reference = Some((report, source));
                }
            }
        }
    }
}

#[test]
fn empty_legal_domains_never_prepare_evaluate_verify_or_export_in_either_mode() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let mut input = problem();
    input["tree_search"]["ordinary_passive_points"] = json!(0);
    prepare(directory, &input);
    let mut reference = None;
    for mode in ["document", "typed"] {
        let filename = format!("empty-{mode}.xml");
        let report = success(
            search(directory, mode, 4, 3)
                .args(["--export", &filename])
                .output()
                .unwrap(),
        );
        assert_eq!(report["termination"], "empty_legal_domain");
        assert_eq!(report["preparation"]["attempts"], 0);
        assert_eq!(report["total_evaluations"], 0);
        assert!(report["search"].is_null());
        assert!(report["best_verified"].is_null());
        assert_eq!(report["export"]["status"], "not_written");
        assert!(!directory.join(&filename).exists());
        assert!(!directory.join(format!("{filename}.data.json")).exists());
        if let Some(expected) = &reference {
            same_search_evidence(expected, &report);
        } else {
            reference = Some(report);
        }
    }
}

#[test]
fn exhausted_infeasible_domains_preserve_evidence_without_spending_reserved_verification() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let mut input = problem();
    input["objective"]["constraints"][0]["threshold"] = json!(100.0);
    prepare(directory, &input);
    let mut reference = None;
    for mode in ["document", "typed"] {
        let filename = format!("infeasible-{mode}.xml");
        let report = success(
            search(directory, mode, 4, 100)
                .args(["--export", &filename])
                .output()
                .unwrap(),
        );
        assert_eq!(report["total_evaluations"], 41);
        assert_eq!(report["search"]["statistics"]["evaluations"], 40);
        assert_eq!(
            report["search"]["statistics"]["verification_evaluations"],
            0
        );
        assert_eq!(report["search"]["verifications"], json!([]));
        assert_eq!(report["search"]["feasible"], json!([]));
        assert!(
            !report["search"]["infeasible"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(report["best_verified"].is_null());
        assert!(!directory.join(&filename).exists());
        assert!(!directory.join(format!("{filename}.data.json")).exists());
        if let Some(expected) = &reference {
            same_search_evidence(expected, &report);
        } else {
            reference = Some(report);
        }
    }
}

#[cfg(feature = "pob")]
#[test]
fn optional_pob_stays_on_document_path_and_rejects_native_evaluation_flags() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let mut input = problem();
    input["objective"]["constraints"] = json!([]);
    input["locks"] = json!({"class_id":6,"weapon_id":"wooden-q0","support_loadout":[]});
    prepare(directory, &input);
    let reference = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    let mut command = cli(directory);
    command
        .args([
            "search-experimental",
            "--backend",
            "pob",
            "--problem",
            "problem.json",
            "--max-evaluations",
            "3",
            "--timeout-seconds",
            "120",
            "--pob",
        ])
        .arg(&reference)
        .args(["--export", "pob.xml"]);
    let report = success(command.output().unwrap());
    assert_eq!(report["execution_kind"], "external_process");
    assert_eq!(report["calculation_path"], "pob_documents");
    assert_eq!(report["preparation"]["attempts"], 1);
    assert_eq!(report["total_evaluations"], 3);
    assert_eq!(report["search"]["statistics"]["evaluations"], 2);
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert_eq!(report["export"]["status"], "written");
    assert!(directory.join("pob.xml").exists());
    assert!(!directory.join("pob.xml.data.json").exists());
    for mode in ["typed", "document"] {
        let output = cli(directory)
            .args([
                "search-experimental",
                "--backend",
                "pob",
                "--native-evaluation",
                mode,
                "--problem",
                "problem.json",
                "--pob",
                "absent-reference-checkout",
                "--export",
                "rejected.xml",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(error.contains("native"), "{error}");
        assert!(!directory.join("rejected.xml").exists());
    }
}

#[test]
fn unavailable_objective_keeps_both_modes_unranked_and_unverified() {
    let temporary = tempfile::tempdir().unwrap();
    let directory = temporary.path();
    let mut input = problem();
    input["objective"]["objective"]["metric"]["id"] = json!("selected_average_hit");
    input["objective"]["objective"]["unit"] = json!("damage");
    input["objective"]["constraints"] = json!([]);
    prepare(directory, &input);
    let mut reference = None;
    for mode in ["document", "typed"] {
        let filename = format!("unavailable-{mode}.xml");
        let report = success(
            search(directory, mode, 4, 100)
                .args(["--export", &filename])
                .output()
                .unwrap(),
        );
        assert_eq!(report["total_evaluations"], 41);
        assert_eq!(report["search"]["statistics"]["evaluations"], 40);
        assert_eq!(report["search"]["statistics"]["unavailable"], 40);
        assert_eq!(
            report["search"]["statistics"]["verification_evaluations"],
            0
        );
        assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
        assert_eq!(report["search"]["verifications"], json!([]));
        assert_eq!(report["search"]["feasible"], json!([]));
        assert_eq!(report["search"]["infeasible"], json!([]));
        assert!(report["best_verified"].is_null());
        assert!(!directory.join(&filename).exists());
        assert!(!directory.join(format!("{filename}.data.json")).exists());
        if let Some(expected) = &reference {
            same_search_evidence(expected, &report);
        } else {
            reference = Some(report);
        }
    }
}
