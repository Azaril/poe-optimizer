//! Native controlled search, independent reference checks and realization failure cases.
use poe_optimizer_core::{candidate::Candidate, evaluation::*, options::EvaluationOptions};
use poe_optimizer_import::controlled_mace::{
    ControlledMaceCatalog, MaceSupportChoice, NormalMaceAlternative,
};
use poe_optimizer_native::NativeBackend;
use serde_json::{Value, json};
use std::{collections::BTreeMap, fs, path::Path, process::Command};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
fn weapons() -> Vec<NormalMaceAlternative> {
    [("wood", "Wooden Club"), ("smith", "Smithing Hammer")]
        .into_iter()
        .map(|(id, base)| NormalMaceAlternative {
            id: id.into(),
            item_text: format!("Rarity: NORMAL\n{base}\nItem Level: 1\nQuality: 0\nImplicits: 0"),
        })
        .collect()
}
fn registry() -> ControlledMaceCatalog {
    ControlledMaceCatalog::new(
        TEMPLATE.into(),
        weapons(),
        vec![MaceSupportChoice::None, MaceSupportChoice::BrutalityI],
    )
    .unwrap()
}
fn problem() -> Value {
    json!({"schema_version":1,"template":"build.xml","weapons":weapons(),"supports":["none","brutality_i"],"objective":{"schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":"selected_hit_dps"},"unit":"damage_per_second","direction":"maximize"},"constraints":[]}})
}
fn command(directory: &Path) -> Command {
    let mut result = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    result.current_dir(directory);
    result
}
fn run(
    directory: &Path,
    input: &Value,
    strategy: &str,
    jobs: usize,
    max: usize,
    export: Option<&Path>,
) -> Value {
    let path = directory.join("problem.json");
    fs::write(&path, serde_json::to_vec(input).unwrap()).unwrap();
    fs::write(directory.join("build.xml"), TEMPLATE).unwrap();
    let mut cmd = command(directory);
    cmd.args(["search-experimental", "--backend", "native", "--problem"])
        .arg(path)
        .args([
            "--strategy",
            strategy,
            "--jobs",
            &jobs.to_string(),
            "--max-evaluations",
            &max.to_string(),
            "--timeout-seconds",
            "30",
            "--seed",
            "17",
            "--pob",
            "missing-pob-checkout",
        ]);
    if let Some(path) = export {
        cmd.arg("--export").arg(path);
    }
    let result = cmd.output().unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    serde_json::from_slice(&result.stdout).unwrap()
}
fn assert_ledger(report: &Value, expected: usize) {
    assert_eq!(report["requested_backend"], "native-poe2");
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["preparation"]["attempts"], 1);
    assert_eq!(report["total_evaluations"], expected, "{report}");
    assert_eq!(report["search"]["statistics"]["evaluations"], expected - 1);
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(
        report["search"]["statistics"]["evaluation_failures"], 0,
        "{report}"
    );
    assert_eq!(report["search"]["statistics"]["discarded_late"], 0);
    assert_eq!(report["diagnostic_only"], true);
    assert_eq!(report["best_verified"]["diagnostic_only"], true);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
}
fn scores(report: &Value) -> BTreeMap<String, f64> {
    let alternatives = report["alternatives"].as_array().unwrap();
    let axes: Vec<_> = alternatives
        .iter()
        .map(|entry| entry["weapon_id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    report["search"]["feasible"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| {
            let indices = entry["candidate"]["choices"].as_array().unwrap();
            let weapon = axes[indices[0].as_u64().unwrap() as usize];
            let support = if indices[1] == 0 {
                "none"
            } else {
                "brutality_i"
            };
            (
                format!("{weapon}/{support}"),
                entry["assessment"]["objective_value"]["value"]
                    .as_f64()
                    .unwrap(),
            )
        })
        .collect()
}
#[test]
fn serial_and_rayon_native_search_match_the_four_independent_reference_results() {
    let temp = tempfile::tempdir().unwrap();
    let export = temp.path().join("winner.xml");
    let serial = run(temp.path(), &problem(), "exhaustive", 1, 6, Some(&export));
    let parallel = run(temp.path(), &problem(), "exhaustive", 4, 6, None);
    for report in [&serial, &parallel] {
        assert_ledger(report, 6);
        assert_eq!(report["termination"], "finite_domain_processed");
        assert_eq!(report["search"]["feasible"].as_array().unwrap().len(), 4);
        assert_eq!(report["best_verified"]["alternative_id"], "smith/none");
        let expected = [
            (
                "wood/none",
                include_str!("fixtures/calibration/mace-wooden.reference.json"),
            ),
            (
                "wood/brutality_i",
                include_str!("fixtures/calibration/mace-wooden-brutality.reference.json"),
            ),
            (
                "smith/none",
                include_str!("fixtures/calibration/mace-smithing.reference.json"),
            ),
            (
                "smith/brutality_i",
                include_str!("fixtures/calibration/mace-smithing-brutality.reference.json"),
            ),
        ];
        let observed = scores(report);
        for (id, reference) in expected {
            let reference: Value = serde_json::from_str(reference).unwrap();
            let expected = reference["metrics"]["TotalDPS"].as_f64().unwrap();
            assert!(
                (observed[id] - expected).abs() < 1e-8,
                "{id}: {} != {expected}",
                observed[id]
            );
        }
    }
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
    let imported = poe_optimizer_import::decode_build(&fs::read(&export).unwrap()).unwrap();
    assert_eq!(
        imported.sha256,
        serial["best_verified"]["source_xml_sha256"]
    );
    let candidate: Candidate =
        serde_json::from_value(serial["best_verified"]["candidate"].clone()).unwrap();
    assert_eq!(
        registry().materialize(&candidate).unwrap().content,
        imported.xml
    );
    assert_eq!(serial["export"]["status"], "written");
    let reevaluated = command(temp.path())
        .arg("evaluate")
        .arg(&export)
        .args(["--backend", "native", "--metric", "player.selected_hit_dps"])
        .output()
        .unwrap();
    assert!(
        reevaluated.status.success(),
        "{}",
        String::from_utf8_lossy(&reevaluated.stderr)
    );
    let reevaluated: Value = serde_json::from_slice(&reevaluated.stdout).unwrap();
    assert!(
        (reevaluated["evaluation"]["measurements"][0]["value"]["value"]
            .as_f64()
            .unwrap()
            - scores(&serial)["smith/none"])
            .abs()
            < 1e-12
    );
}
#[test]
fn eight_state_example_matches_exhaustive_and_guided_native_selection() {
    let temp = tempfile::tempdir().unwrap();
    let mut input: Value =
        serde_json::from_str(include_str!("../examples/mace-search.json")).unwrap();
    input["template"] = json!("build.xml");
    let serial = run(temp.path(), &input, "exhaustive", 1, 10, None);
    let parallel = run(temp.path(), &input, "exhaustive", 4, 10, None);
    let guided = run(temp.path(), &input, "guided", 4, 10, None);
    for report in [&serial, &parallel, &guided] {
        assert_ledger(report, 10);
        assert_eq!(report["search"]["feasible"].as_array().unwrap().len(), 8);
        assert_eq!(
            report["best_verified"]["alternative_id"],
            "smithing-q20/none"
        );
        assert_eq!(scores(report), scores(&serial));
    }
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
    assert_eq!(serial["best_verified"], guided["best_verified"]);
}
#[test]
fn guided_locks_and_total_budget_include_fresh_native_verification() {
    let temp = tempfile::tempdir().unwrap();
    let mut input = problem();
    input["locks"] = json!({"support":"brutality_i"});
    let supported = run(temp.path(), &input, "guided", 4, 4, None);
    assert_ledger(&supported, 4);
    assert_eq!(
        supported["best_verified"]["alternative_id"],
        "wood/brutality_i"
    );
    assert!(
        supported["search"]["feasible"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["candidate"]["choices"][1] == 1)
    );
    assert_eq!(supported["candidate_constraints"]["locks"]["skill_groups"]["pob-group-1"]["exact_support_instance_ids"].as_array().unwrap().len(), 1);
    input["locks"] = json!({"support":"brutality_i","weapon_id":"smith"});
    let locked = run(temp.path(), &input, "guided", 4, 3, None);
    assert_ledger(&locked, 3);
    assert_eq!(
        locked["best_verified"]["alternative_id"],
        "smith/brutality_i"
    );
    assert_eq!(
        locked["candidate_constraints"]["required_item_instance_ids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let partial = run(temp.path(), &problem(), "exhaustive", 4, 3, None);
    assert_ledger(&partial, 3);
    assert_eq!(partial["termination"], "evaluation_budget");
    assert_eq!(partial["search"]["feasible"].as_array().unwrap().len(), 1);
}
fn evaluate(build: BuildDocument) -> EvaluationResult {
    Engine::new(NativeBackend::new())
        .evaluate(
            &EvaluationRequest {
                build,
                options: EvaluationOptions::default(),
                metrics: vec![],
            },
            EvaluationBudget { timeout_ms: 5000 },
        )
        .unwrap()
}
#[test]
fn native_realization_rejects_changed_projection_context_or_resolved_item_evidence() {
    let registry = registry();
    let baseline = evaluate(registry.template_build());
    let scenario = registry
        .bind_native_baseline(&baseline, &poe_optimizer_native::backend_identity())
        .unwrap();
    let candidate = registry
        .resolve_candidate("smith", MaceSupportChoice::BrutalityI)
        .unwrap();
    let result = evaluate(registry.materialize(candidate).unwrap());
    registry
        .validate_native_realization(candidate, &result, &scenario)
        .unwrap();
    let original = serde_json::to_value(result).unwrap();
    for field in [
        "export",
        "class",
        "skill",
        "item",
        "quality",
        "support",
        "input",
        "placeholder",
        "identity",
    ] {
        let mut value = original.clone();
        match field {
            "export" => {
                let changed = format!("{} ", value["exports"][0]["content"].as_str().unwrap());
                value["exports"][0]["content"] = json!(changed);
            }
            "class" => value["build"]["class_name"] = json!("Sorceress"),
            "skill" => value["coverage"]["selected_player"]["skill_id"] = json!("SparkPlayer"),
            "item" | "quality" | "support" => {
                let mut evidence: Value =
                    serde_json::from_str(value["attachments"][0]["content"].as_str().unwrap())
                        .unwrap();
                match field {
                    "item" => evidence["weapon_base"] = json!("Wooden Club"),
                    "quality" => evidence["weapon_quality"] = json!(20),
                    _ => evidence["brutality_i"] = json!(false),
                }
                value["attachments"][0]["content"] = json!(evidence.to_string());
            }
            "input" => value["context"]["config_inputs"]["enemyArmour"] = json!(1.0),
            "placeholder" => {
                value["context"]["config_placeholders"]["resistancePenalty"] = json!(0.0)
            }
            "identity" => value["backend"]["adapter_fingerprint"] = json!("stale"),
            _ => unreachable!(),
        }
        let changed: EvaluationResult = serde_json::from_value(value).unwrap();
        assert!(
            registry
                .validate_native_realization(candidate, &changed, &scenario)
                .is_err(),
            "accepted {field}"
        );
    }
    let mut conditions: EvaluationResult = serde_json::from_value(original).unwrap();
    conditions
        .context
        .player_conditions
        .insert("candidateDerivedCondition".into(), true);
    registry
        .validate_native_realization(candidate, &conditions, &scenario)
        .unwrap();
    let mut wrong_identity = poe_optimizer_native::backend_identity();
    wrong_identity.adapter_fingerprint.push('0');
    assert!(
        registry
            .bind_native_baseline(&baseline, &wrong_identity)
            .is_err()
    );
}
#[test]
fn unsupported_native_profiles_and_output_collisions_never_fall_back_to_pob() {
    let temp = tempfile::tempdir().unwrap();
    let problem_path = temp.path().join("problem.json");
    let mut input = problem();
    input["objective"]["objective"]["metric"]["id"] = json!("pob_total_ehp");
    fs::write(&problem_path, serde_json::to_vec(&input).unwrap()).unwrap();
    fs::write(temp.path().join("build.xml"), TEMPLATE).unwrap();
    let output = temp.path().join("result.json");
    let unsupported = command(temp.path())
        .args(["search-experimental", "--backend", "native", "--problem"])
        .arg(&problem_path)
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!unsupported.status.success());
    assert!(!output.exists());
    fs::write(&output, b"preserve").unwrap();
    let collision = command(temp.path())
        .args(["search-experimental", "--backend", "native", "--problem"])
        .arg(temp.path().join("missing.json"))
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!collision.status.success());
    assert!(String::from_utf8_lossy(&collision.stderr).contains("Output already exists"));
    assert_eq!(fs::read(&output).unwrap(), b"preserve");
}
#[cfg(not(feature = "pob"))]
#[test]
fn native_only_search_defaults_to_native_and_rejects_reference_backend() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("problem.json"),
        serde_json::to_vec(&problem()).unwrap(),
    )
    .unwrap();
    fs::write(temp.path().join("build.xml"), TEMPLATE).unwrap();
    let result = command(temp.path())
        .args([
            "search-experimental",
            "--problem",
            "problem.json",
            "--max-evaluations",
            "6",
        ])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_ledger(&report, 6);
    assert!(
        !command(temp.path())
            .args([
                "search-experimental",
                "--backend",
                "pob",
                "--problem",
                "problem.json"
            ])
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn pinned_native_catalog_rejects_custom_data_even_with_matching_backend_identity() {
    use poe_optimizer_data::game_data::{
        GameDataLoader, LoadLimits, TrustPolicy, bundled_snapshot,
    };
    use poe_optimizer_native::{CompiledGameData, HostClock};
    use std::sync::Arc;
    let registry = registry();
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.character.accuracy_per_level += 1.0;
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let data = Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap());
    let backend = NativeBackend::with_data(data, HostClock).unwrap();
    let identity = backend.identity();
    let result = Engine::new(backend)
        .evaluate(
            &EvaluationRequest {
                build: registry.template_build(),
                options: EvaluationOptions::default(),
                metrics: vec![],
            },
            EvaluationBudget { timeout_ms: 5000 },
        )
        .unwrap();
    let error = registry
        .bind_native_baseline(&result, &identity)
        .unwrap_err();
    assert!(error.to_string().contains("reviewed default data"));
}
