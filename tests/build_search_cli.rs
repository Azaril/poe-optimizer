//! Graph search preserves admission, ledgers and fresh verified artifacts across backends/workers.
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const TEMPLATE: &str = include_str!("fixtures/builds/mace-passive-equipment.xml");
fn problem(dir: &Path) -> Value {
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/passive-equipment-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    value
}
fn write_problem(dir: &Path, value: &Value) {
    fs::write(dir.join("template.xml"), TEMPLATE).unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn command(dir: &Path, mode: &str, jobs: usize, evaluations: usize) -> Command {
    // These fixtures compare functional results, not throughput. Concurrent debug
    // CI searches exceeded the former 60-second whole-command deadline. Keep a
    // finite allowance for cold preparation and fresh evaluations; deadline and
    // cancellation behavior has dedicated search-crate contract tests.
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(dir).args([
        "search-build",
        "--problem",
        "problem.json",
        "--native-evaluation",
        mode,
        "--jobs",
        &jobs.to_string(),
        "--max-evaluations",
        &evaluations.to_string(),
        "--max-proposals",
        "256",
        "--max-rounds",
        "3",
        "--timeout-seconds",
        "300",
    ]);
    command
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "stderr:{} stdout:{}",
        String::from_utf8_lossy(&output.stderr),
        String::from_utf8_lossy(&output.stdout)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn same_search(a: &Value, b: &Value) {
    for key in [
        "feasible",
        "infeasible",
        "verifications",
        "statistics",
        "termination",
    ] {
        assert_eq!(a["search"][key], b["search"][key], "search.{key}");
    }
    for key in [
        "catalog",
        "best_verified",
        "total_evaluations",
        "source_assembly_attempts",
        "baseline_attempts",
        "baseline_error",
    ] {
        assert_eq!(a[key], b[key], "{key}");
    }
}
fn assert_ledger(report: &Value, maximum: u64) {
    let diagnostics = || {
        json!({
            "native_evaluation": report["native_evaluation"],
            "elapsed_ms": report["elapsed_ms"],
            "search_elapsed_ms": report["search"]["elapsed_ms"],
            "termination": report["search"]["termination"],
            "statistics": report["search"]["statistics"],
            "errors": report["search"]["errors"],
            "verifications": report["search"]["verifications"],
            "total_evaluations": report["total_evaluations"],
            "maximum_evaluations": maximum,
            "baseline_attempts": report["baseline_attempts"],
            "baseline_error": report["baseline_error"],
        })
    };
    let total = report["total_evaluations"].as_u64().unwrap();
    let search = report["search"]["statistics"]["evaluations"]
        .as_u64()
        .unwrap();
    assert!(total <= maximum, "search ledger: {}", diagnostics());
    assert_eq!(
        total,
        search + report["baseline_attempts"].as_u64().unwrap(),
        "search ledger: {}",
        diagnostics()
    );
    assert_eq!(
        report["search"]["statistics"]["evaluation_failures"],
        0,
        "search ledger: {}",
        diagnostics()
    );
    assert_eq!(
        report["search"]["statistics"]["discarded_late"],
        0,
        "search ledger: {}",
        diagnostics()
    );
}
fn assert_export(dir: &Path, name: &str, report: &Value) -> (Vec<u8>, Value) {
    assert_eq!(report["export"]["status"], "written");
    let xml = fs::read(dir.join(name)).unwrap();
    let companion: Value =
        serde_json::from_slice(&fs::read(dir.join(format!("{name}.data.json"))).unwrap()).unwrap();
    let digest = format!("{:x}", Sha256::digest(&xml));
    assert_eq!(report["best_verified"]["source_xml_sha256"], digest);
    assert_eq!(companion["xml_sha256"], digest);
    assert_eq!(companion["backend"], report["backend"]);
    assert_eq!(companion["uses_packaged_default"], true);
    assert_eq!(companion["status"], "native_export_data");
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(
        report["search"]["verifications"].as_array().unwrap().len(),
        1
    );
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    (xml, companion)
}
#[test]
fn graph_archives_ledgers_and_fresh_exports_match_typed_document_and_worker_counts() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    write_problem(dir, &problem(dir));
    let mut reference: Option<(Value, Vec<u8>, Value)> = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("{mode}-{jobs}.xml");
            let report = success(
                command(dir, mode, jobs, 40)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 8);
            assert_eq!(
                report["scope"],
                "connected_passive_equipment_native_search_v1"
            );
            assert_eq!(report["diagnostic_only"], true);
            assert_ledger(&report, 40);
            assert!(!report["best_verified"].is_null());
            assert_eq!(report["native_preparation"]["retained_xml_bytes"], 0);
            assert_eq!(report["native_preparation"]["cached_candidate_results"], 0);
            assert_eq!(report["catalog_preparation"]["materialized_candidates"], 0);
            assert_eq!(report["catalog_preparation"]["cached_candidate_results"], 0);
            let (xml, companion) = assert_export(dir, &name, &report);
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
}
#[test]
fn minimum_three_attempt_budget_reserves_fresh_finalist_once() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    write_problem(dir, &problem(dir));
    let mut reference = None;
    for mode in ["typed", "document"] {
        let name = format!("tight-{mode}.xml");
        let report = success(
            command(dir, mode, 4, 3)
                .args(["--export", &name])
                .output()
                .unwrap(),
        );
        assert_ledger(&report, 3);
        assert_eq!(report["total_evaluations"], 3);
        assert_export(dir, &name, &report);
        if let Some(expected) = &reference {
            same_search(expected, &report);
        } else {
            reference = Some(report);
        }
    }
}
#[test]
fn required_two_items_and_connected_attribute_locks_are_repaired_before_search() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut problem = problem(dir);
    problem["constraints"]["required_item_instance_ids"] =
        json!(["wooden-physical", "solar-resource"]);
    problem["constraints"]["locks"]["class_id"] = json!("6");
    problem["constraints"]["locks"]["ascendancy"] = json!({"kind":"none"});
    problem["constraints"]["locks"]["allocated_passives"] = json!([13397]);
    problem["constraints"]["budgets"]["ordinary_passive_points"] = json!(2);
    problem["attribute_locks"]["required"] = json!({"13397":"intelligence"});
    write_problem(dir, &problem);
    let mut reference = None;
    for mode in ["typed", "document"] {
        let name = format!("locks-{mode}.xml");
        let report = success(
            command(dir, mode, 4, 40)
                .args(["--export", &name])
                .output()
                .unwrap(),
        );
        assert_ledger(&report, 40);
        let (xml, _) = assert_export(dir, &name, &report);
        let selected = &report["best_verified"]["candidate"];
        assert_eq!(selected["candidate"]["class_id"], "6");
        assert!(selected["candidate"]["ascendancy_id"].is_null());
        assert_eq!(
            selected["candidate"]["equipment"]["Weapon 1"],
            "wooden-physical"
        );
        assert_eq!(
            selected["candidate"]["equipment"]["Amulet"],
            "solar-resource"
        );
        assert_eq!(selected["candidate"]["passives"], json!([3936, 13397]));
        assert_eq!(selected["attribute_options"]["13397"], "intelligence");
        assert!(String::from_utf8_lossy(&xml).contains("intNodes=\"13397\""));
        if let Some(expected) = &reference {
            same_search(expected, &report);
        } else {
            reference = Some(report);
        }
    }
}
#[test]
fn impossible_locked_item_level_spends_no_calculations_and_writes_no_artifacts() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut problem = problem(dir);
    let text = problem["equipment"][1]["item_text"]
        .as_str()
        .unwrap()
        .replace("Implicits: 0", "LevelReq: 61\nImplicits: 0");
    problem["equipment"][1]["item_text"] = json!(text);
    problem["constraints"]["required_item_instance_ids"] = json!(["wooden-physical"]);
    write_problem(dir, &problem);
    for mode in ["typed", "document"] {
        let name = format!("impossible-{mode}.xml");
        let report = success(
            command(dir, mode, 4, 40)
                .args(["--export", &name])
                .output()
                .unwrap(),
        );
        assert_ledger(&report, 0);
        assert_eq!(report["total_evaluations"], 0);
        assert!(report["baseline"].is_null());
        assert!(report["best_verified"].is_null());
        assert_eq!(
            report["search"]["statistics"]["verification_evaluations"],
            0
        );
        assert!(report["search"]["feasible"].as_array().unwrap().is_empty());
        assert!(report["search"]["statistics"]["rejected"].as_u64().unwrap() > 0);
        assert_eq!(report["export"]["status"], "not_written");
        assert!(!dir.join(&name).exists());
        assert!(!dir.join(format!("{name}.data.json")).exists());
    }
}
#[test]
fn output_collisions_preserve_existing_files_and_do_not_publish_partial_artifacts() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    write_problem(dir, &problem(dir));
    fs::write(
        dir.join("keep.xml.data.json"),
        b"preserve existing companion",
    )
    .unwrap();
    let output = command(dir, "typed", 1, 3)
        .args(["--export", "keep.xml", "--output", "report.json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert_eq!(
        fs::read(dir.join("keep.xml.data.json")).unwrap(),
        b"preserve existing companion"
    );
    assert!(!dir.join("keep.xml").exists());
    assert!(!dir.join("report.json").exists());
    let output = command(dir, "typed", 1, 3)
        .args(["--export", "same.json", "--output", "same.json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.join("same.json").exists());
    fs::create_dir(dir.join("nested")).unwrap();
    let output = command(dir, "typed", 1, 3)
        .args(["--export", "alias.xml", "--output", "nested/../alias.xml"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.join("alias.xml").exists());
    assert!(!dir.join("alias.xml.data.json").exists());
    #[cfg(windows)]
    {
        let output = command(dir, "typed", 1, 3)
            .args(["--export", "case.xml", "--output", "CASE.XML"])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!dir.join("case.xml").exists());
    }
}

#[test]
fn imported_numeric_failure_is_counted_without_suppressing_legal_alternatives() {
    use poe_optimizer_data::game_data::{self, PassiveEffect, PassiveStat};
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut problem = problem(dir);
    // Root-only search keeps the failing imported composition out of the legal
    // search domain, but its actual baseline attempt must still be reported.
    problem["constraints"]["budgets"]["ordinary_passive_points"] = json!(0);
    problem["constraints"]["budgets"]["ascendancy_passive_points"] = json!(0);
    write_problem(dir, &problem);
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    for view in &mut package.passive_effects {
        if [3936, 13397].contains(&view.key.physical_node_id) {
            view.effects = vec![PassiveEffect {
                stat: PassiveStat::SkillSpeedIncreased,
                value: 750_000.0,
            }];
        }
    }
    package.refresh_section_digests().unwrap();
    fs::write(dir.join("custom.json"), package.canonical_bytes().unwrap()).unwrap();
    let mut reference: Option<(Value, Vec<u8>, Value)> = None;
    for mode in ["typed", "document"] {
        let name = format!("overflow-{mode}.xml");
        let report = success(
            command(dir, mode, 4, 40)
                .args(["--data", "custom.json", "--export", &name])
                .output()
                .unwrap(),
        );
        assert_ledger(&report, 40);
        assert!(report["baseline"].is_null());
        assert_eq!(report["baseline_attempts"], 1);
        assert!(
            report["baseline_error"]["message"]
                .as_str()
                .unwrap()
                .contains("Character non-resistance modifiers must be finite and in 0..1000000")
        );
        assert!(!report["best_verified"].is_null());
        assert_eq!(
            report["best_verified"]["candidate"]["candidate"]["passives"],
            json!([])
        );
        assert_eq!(
            report["search"]["statistics"]["verification_evaluations"],
            1
        );
        assert_eq!(report["search"]["verifications"][0]["consistent"], true);
        assert_eq!(report["export"]["status"], "written");
        let xml = fs::read(dir.join(&name)).unwrap();
        let companion: Value =
            serde_json::from_slice(&fs::read(dir.join(format!("{name}.data.json"))).unwrap())
                .unwrap();
        let digest = format!("{:x}", Sha256::digest(&xml));
        assert_eq!(report["best_verified"]["source_xml_sha256"], digest);
        assert_eq!(companion["xml_sha256"], digest);
        assert_eq!(companion["backend"], report["backend"]);
        assert_eq!(companion["uses_packaged_default"], false);
        if let Some((expected, expected_xml, expected_companion)) = &reference {
            same_search(expected, &report);
            assert_eq!(&xml, expected_xml);
            assert_eq!(&companion, expected_companion);
        } else {
            reference = Some((report, xml, companion));
        }
    }
}

#[test]
fn receiving_search_has_versioned_scope_and_identical_parallel_verified_results() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/receiving-defence-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    // Seeded complete composition meets the constraints; the budget also explores changes.
    fs::write(
        dir.join("template.xml"),
        include_str!("fixtures/builds/mace-receiving-defence.xml"),
    )
    .unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let mut reference = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("receiving-{mode}-{jobs}.xml");
            let report = success(
                command(dir, mode, jobs, 60)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 9);
            assert_eq!(report["scope"], "receiving_defence_native_search_v1");
            assert_ledger(&report, 60);
            let (xml, companion) = assert_export(dir, &name, &report);
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
    value["schema_version"] = json!(7);
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let output = command(dir, "typed", 1, 3)
        .args(["--export", "old-scope.xml"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires graph problem schema 8"));
    assert!(!dir.join("old-scope.xml").exists());
    assert!(!dir.join("old-scope.xml.data.json").exists());
}

#[test]
fn receiving_equipment_alone_requires_new_graph_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value = problem(dir);
    value["equipment"][0]["item_text"] = json!(
        "Rarity: RARE\nReceiving\nLunar Amulet\nItem Level: 60\nQuality: 0\nImplicits: 1\n+25 to maximum Energy Shield"
    );
    write_problem(dir, &value);
    let output = command(dir, "typed", 1, 3).output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("requires graph problem schema 8"));
    value["schema_version"] = json!(8);
    write_problem(dir, &value);
    let report = success(command(dir, "typed", 1, 3).output().unwrap());
    assert_eq!(report["schema_version"], 9);
    assert_ledger(&report, 3);
}

#[test]
fn legacy_actor_problem_rejects_new_authored_receiving_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/mace-actor-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    let template=include_str!("fixtures/builds/mace-actor-resources.xml").replace("</ConfigSet>","<CustomModifierBlock title=\"Defence\" enabled=\"true\">+25 to Armour</CustomModifierBlock></ConfigSet>");
    fs::write(dir.join("template.xml"), template).unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args([
            "search-experimental",
            "--backend",
            "native",
            "--problem",
            "problem.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("receiving-defence modifiers require search-build with problem schema 8"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn local_armour_search_matches_modes_workers_and_preserves_rating_constraints() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/local-armour-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    fs::write(
        dir.join("template.xml"),
        include_str!("fixtures/builds/mace-local-armour.xml"),
    )
    .unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let mut reference = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("armour-{mode}-{jobs}.xml");
            let report = success(
                command(dir, mode, jobs, 60)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 10);
            assert_eq!(report["scope"], "local_armour_native_search_v1");
            assert_ledger(&report, 60);
            let (xml, companion) = assert_export(dir, &name, &report);
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
    for schema in [7, 8] {
        value["schema_version"] = json!(schema);
        fs::write(
            dir.join("problem.json"),
            serde_json::to_vec_pretty(&value).unwrap(),
        )
        .unwrap();
        let name = format!("old-armour-{schema}.xml");
        let output = command(dir, "typed", 1, 3)
            .args(["--export", &name])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("requires graph problem schema 9")
        );
        assert!(!dir.join(&name).exists());
        assert!(!dir.join(format!("{name}.data.json")).exists());
    }
}

#[test]
fn unselected_supplied_armour_also_requires_explicit_graph_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value = problem(dir);
    value["equipment"].as_array_mut().unwrap().push(json!({
        "instance_id":"supplied-helmet", "pob_item_id":41,
        "item_text":"Rarity: RARE\nSupplied Study\nRusted Greathelm\nItem Level: 60\nQuality: 20\nImplicits: 0\n+31 to Armour"
    }));
    for schema in [7, 8] {
        value["schema_version"] = json!(schema);
        write_problem(dir, &value);
        let output = command(dir, "typed", 1, 3).output().unwrap();
        assert!(!output.status.success());
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("requires graph problem schema 9")
        );
    }
    value["schema_version"] = json!(9);
    write_problem(dir, &value);
    let report = success(command(dir, "typed", 1, 3).output().unwrap());
    assert_eq!(report["schema_version"], 10);
    assert_ledger(&report, 3);
}

#[test]
fn body_movement_search_preserves_modes_workers_locks_and_all_constraints() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/body-armour-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    value["constraints"]["required_item_instance_ids"] = json!(["body-17"]);
    fs::write(
        dir.join("template.xml"),
        include_str!("fixtures/builds/mace-body-armour.xml"),
    )
    .unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let mut reference = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("movement-{mode}-{jobs}.xml");
            let report = success(
                command(dir, mode, jobs, 60)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 11);
            assert_eq!(report["scope"], "movement_native_search_v1");
            assert_ledger(&report, 60);
            assert_eq!(
                report["best_verified"]["candidate"]["candidate"]["equipment"]["Body Armour"],
                "body-17"
            );
            assert_eq!(
                report["best_verified"]["assessment"]["status"],
                "constraints_satisfied"
            );
            let movement = report["best_verified"]["assessment"]["measurements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["query"]["id"] == "movement_speed_pct")
                .unwrap();
            assert_eq!(movement["unit"], "percent");
            assert!(movement["value"]["value"].as_f64().unwrap() >= 115.0);
            let (xml, companion) = assert_export(dir, &name, &report);
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
}

#[test]
fn old_graph_schemas_reject_body_config_and_unselected_movement_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    for kind in ["body", "config", "unselected"] {
        for schema in [7, 8, 9] {
            let mut value = problem(dir);
            value["schema_version"] = json!(schema);
            let template=match kind {
                "body"=>include_str!("fixtures/builds/mace-body-armour.xml").to_owned(),
                "config"=>TEMPLATE.replace("</ConfigSet>","<CustomModifierBlock title=\"Movement\" enabled=\"true\">20% increased Movement Speed</CustomModifierBlock></ConfigSet>"),
                _=>TEMPLATE.to_owned(),
            };
            if kind == "unselected" {
                value["equipment"].as_array_mut().unwrap().push(json!({"instance_id":"unselected-movement","pob_item_id":71,"item_text":"Rarity: RARE\nUnselected Movement\nWooden Club\nItem Level: 60\nQuality: 0\nImplicits: 0\n20% increased Movement Speed"}));
            }
            write_problem(dir, &value);
            fs::write(dir.join("template.xml"), template).unwrap();
            let name = format!("denied-{kind}-{schema}.xml");
            let output = command(dir, "typed", 1, 3)
                .args(["--export", &name])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{kind}/{schema}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("require graph problem schema 10"),
                "{kind}/{schema}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!dir.join(&name).exists());
            assert!(!dir.join(format!("{name}.data.json")).exists());
        }
    }
}

#[test]
fn injected_body_penalty_changes_search_ranking_with_exact_custom_data_exports() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value = problem(dir);
    value["schema_version"] = json!(10);
    value["equipment"] = json!([
        {"instance_id":"heavy-body","pob_item_id":81,"item_text":"Rarity: RARE\nHeavy Ranking\nRusted Cuirass\nItem Level: 60\nQuality: 20\nImplicits: 0\n+200 to Armour"},
        {"instance_id":"light-body","pob_item_id":82,"item_text":"Rarity: RARE\nLight Ranking\nLeather Vest\nItem Level: 60\nQuality: 20\nImplicits: 0\n+200 to Armour"}
    ]);
    value["objective"] = json!({"schema_version":1,"objective":{"kind":"scalar","metric":{"actor":"player","id":"movement_speed_pct"},"unit":"percent","direction":"maximize"},"constraints":[{"id":"equipped-defences","metric":{"actor":"player","id":"armour"},"unit":"rating_points","operator":">=","threshold":100,"violation_scale":100}]});
    value["constraints"]["budgets"]["ordinary_passive_points"] = json!(0);
    value["constraints"]["budgets"]["ascendancy_passive_points"] = json!(0);
    value["constraints"]["budgets"]["supports_per_skill"] = json!(0);
    value["constraints"]["locks"]["class_id"] = json!("6");
    value["constraints"]["locks"]["ascendancy"] = json!({"kind":"none"});
    write_problem(dir, &value);
    fs::write(
        dir.join("template.xml"),
        include_str!("fixtures/calibration/mace-wooden.xml"),
    )
    .unwrap();
    let mut custom = poe_optimizer_data::game_data::bundled_snapshot()
        .unwrap()
        .package()
        .clone();
    custom
        .armour_bases
        .iter_mut()
        .find(|base| base.name == "Rusted Cuirass")
        .unwrap()
        .movement_penalty = Some(0.01);
    custom.refresh_section_digests().unwrap();
    fs::write(dir.join("custom.json"), custom.canonical_bytes().unwrap()).unwrap();
    for (data, expected_item, expected_speed) in [
        (None, "light-body", 97.0),
        (Some("custom.json"), "heavy-body", 99.0),
    ] {
        let mut reference = None;
        for mode in ["typed", "document"] {
            let name = format!(
                "ranking-{}-{mode}.xml",
                if data.is_some() { "custom" } else { "bundled" }
            );
            let mut cmd = command(dir, mode, 4, 40);
            cmd.args(["--export", &name]);
            if let Some(path) = data {
                cmd.args(["--data", path]);
            }
            let report = success(cmd.output().unwrap());
            assert_ledger(&report, 40);
            assert_eq!(
                report["best_verified"]["candidate"]["candidate"]["equipment"]["Body Armour"],
                expected_item
            );
            assert_eq!(
                report["best_verified"]["assessment"]["status"],
                "constraints_satisfied"
            );
            assert!(
                (report["best_verified"]["assessment"]["objective_value"]["value"]
                    .as_f64()
                    .unwrap()
                    - expected_speed)
                    .abs()
                    < 1e-9
            );
            assert_eq!(
                report["search"]["statistics"]["verification_evaluations"],
                1
            );
            let xml = fs::read(dir.join(&name)).unwrap();
            let companion: Value =
                serde_json::from_slice(&fs::read(dir.join(format!("{name}.data.json"))).unwrap())
                    .unwrap();
            let digest = format!("{:x}", Sha256::digest(&xml));
            assert_eq!(companion["xml_sha256"], digest);
            assert_eq!(report["best_verified"]["source_xml_sha256"], digest);
            assert_eq!(companion["backend"], report["backend"]);
            assert_eq!(companion["uses_packaged_default"], data.is_none());
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
}

#[test]
fn legacy_actor_problem_rejects_new_authored_movement_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/mace-actor-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    let template=include_str!("fixtures/builds/mace-actor-resources.xml").replace("</ConfigSet>","<CustomModifierBlock title=\"Defence\" enabled=\"true\">20% increased Movement Speed</CustomModifierBlock></ConfigSet>");
    fs::write(dir.join("template.xml"), template).unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args([
            "search-experimental",
            "--backend",
            "native",
            "--problem",
            "problem.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("movement modifiers require search-build with problem schema 10"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn action_timing_search_preserves_modes_workers_locks_and_configured_floors() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/action-timing-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    value["constraints"]["required_item_instance_ids"] = json!(["body-17"]);
    fs::write(
        dir.join("template.xml"),
        include_str!("fixtures/builds/mace-action-timing.xml"),
    )
    .unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let mut reference = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("action-{mode}-{jobs}.xml");
            let report = success(
                command(dir, mode, jobs, 60)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 12);
            assert_eq!(report["scope"], "action_timing_native_search_v1");
            assert_ledger(&report, 60);
            assert_eq!(
                report["best_verified"]["candidate"]["candidate"]["equipment"]["Body Armour"],
                "body-17"
            );
            assert_eq!(
                report["best_verified"]["assessment"]["status"],
                "constraints_satisfied"
            );
            let movement = report["best_verified"]["assessment"]["measurements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["query"]["id"] == "movement_speed_pct")
                .unwrap();
            assert_eq!(movement["unit"], "percent");
            assert!(movement["value"]["value"].as_f64().unwrap() >= 115.0);
            let action = report["best_verified"]["assessment"]["measurements"]
                .as_array()
                .unwrap()
                .iter()
                .find(|m| m["query"]["id"] == "action_speed_pct")
                .unwrap();
            assert_eq!(action["unit"], "percent");
            assert!((action["value"]["value"].as_f64().unwrap() - 130.0).abs() < 1e-9);
            let (xml, companion) = assert_export(dir, &name, &report);
            if let Some((expected, expected_xml, expected_companion)) = &reference {
                same_search(expected, &report);
                assert_eq!(&xml, expected_xml);
                assert_eq!(&companion, expected_companion);
            } else {
                reference = Some((report, xml, companion));
            }
        }
    }
}

#[test]
fn old_graph_schemas_reject_action_speed_in_config_equipped_and_unselected_sources() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    for kind in ["equipped", "config", "unselected"] {
        for schema in [7, 8, 9, 10] {
            let mut value = problem(dir);
            value["schema_version"] = json!(schema);
            let template=match kind {
                "equipped"=>TEMPLATE.replacen("</Item>", "\n20% increased Action Speed\n</Item>", 1),
                "config"=>TEMPLATE.replace("</ConfigSet>","<CustomModifierBlock title=\"Movement\" enabled=\"true\">20% increased Action Speed</CustomModifierBlock></ConfigSet>"),
                _=>TEMPLATE.to_owned(),
            };
            if kind == "unselected" {
                value["equipment"].as_array_mut().unwrap().push(json!({"instance_id":"unselected-movement","pob_item_id":71,"item_text":"Rarity: RARE\nUnselected Movement\nWooden Club\nItem Level: 60\nQuality: 0\nImplicits: 0\n20% increased Action Speed"}));
            }
            write_problem(dir, &value);
            fs::write(dir.join("template.xml"), template).unwrap();
            let name = format!("denied-{kind}-{schema}.xml");
            let output = command(dir, "typed", 1, 3)
                .args(["--export", &name])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{kind}/{schema}");
            assert!(
                String::from_utf8_lossy(&output.stderr).contains("require graph problem schema 11"),
                "{kind}/{schema}: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(!dir.join(&name).exists());
            assert!(!dir.join(format!("{name}.data.json")).exists());
        }
    }
}

#[test]
fn legacy_actor_problem_rejects_action_speed_scope() {
    let temporary = tempfile::tempdir().unwrap();
    let dir = temporary.path();
    let mut value: Value =
        serde_json::from_str(include_str!("../examples/mace-actor-search.json")).unwrap();
    value["template"] = json!(dir.join("template.xml"));
    let template=include_str!("fixtures/builds/mace-actor-resources.xml").replace("</ConfigSet>","<CustomModifierBlock title=\"Defence\" enabled=\"true\">20% increased Action Speed</CustomModifierBlock></ConfigSet>");
    fs::write(dir.join("template.xml"), template).unwrap();
    fs::write(
        dir.join("problem.json"),
        serde_json::to_vec_pretty(&value).unwrap(),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(dir)
        .args([
            "search-experimental",
            "--backend",
            "native",
            "--problem",
            "problem.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("action-speed modifiers require search-build with problem schema 11"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
