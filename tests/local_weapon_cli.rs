//! Real CLI integration for supplied normal/rare local weapon inputs.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
fn problem() -> Value {
    let mut p: Value =
        serde_json::from_str(include_str!("../examples/mace-local-weapon-search.json")).unwrap();
    p["template"] = json!("build.xml");
    p["tree_search"]["selections"] = json!([
        {"class_id":6,"ascendancy_id":"Warrior3","entrance_node_id":3936,"ascendancy_node_id":14960},
        {"class_id":10,"ascendancy_id":"Monk3","entrance_node_id":10364,"ascendancy_node_id":24475}
    ]);
    p
}
fn prepare(dir: &Path, p: &Value) {
    fs::write(dir.join("problem.json"), serde_json::to_vec(p).unwrap()).unwrap();
    fs::write(dir.join("build.xml"), TEMPLATE).unwrap();
}
fn cli(dir: &Path) -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    c.current_dir(dir);
    c
}
fn search(dir: &Path, mode: &str, jobs: u32) -> Command {
    let mut c = cli(dir);
    c.args([
        "search-experimental",
        "--backend",
        "native",
        "--native-evaluation",
        mode,
        "--problem",
        "problem.json",
        "--max-evaluations",
        "100",
        "--timeout-seconds",
        "120",
        "--pob",
        "absent-reference-checkout",
        "--jobs",
        &jobs.to_string(),
    ]);
    c
}
fn success(out: Output) -> Value {
    assert!(
        out.status.success(),
        "stderr:{} stdout:{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    serde_json::from_slice(&out.stdout).unwrap()
}
fn same(a: &Value, b: &Value) {
    for k in ["feasible", "infeasible", "verifications", "statistics"] {
        assert_eq!(a["search"][k], b["search"][k], "search.{k}");
    }
    for k in [
        "best_verified",
        "requirements",
        "admission",
        "catalog",
        "alternatives",
        "total_evaluations",
        "termination",
    ] {
        assert_eq!(a[k], b[k], "{k}");
    }
}
#[test]
fn rare_normal_items_jointly_match_across_document_typed_and_rayon_with_fresh_exports() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    prepare(dir, &problem());
    let mut reference: Option<Value> = None;
    let mut reference_xml = None;
    for mode in ["typed", "document"] {
        for jobs in [1, 4] {
            let name = format!("{mode}-{jobs}.xml");
            let report = success(
                search(dir, mode, jobs)
                    .args(["--export", &name])
                    .output()
                    .unwrap(),
            );
            assert_eq!(report["schema_version"], 6);
            assert_eq!(
                report["scope"],
                "mace_local_weapon_class_passive_support_loadouts_v1"
            );
            assert_eq!(report["admission"]["checked_candidates"], 84);
            assert_eq!(
                report["requirements"]["legal_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                59
            );
            assert_eq!(
                report["requirements"]["rejected_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                25
            );
            assert_eq!(report["total_evaluations"], 61);
            assert_eq!(
                report["search"]["statistics"]["verification_evaluations"],
                1
            );
            assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
            assert_eq!(report["search"]["verifications"][0]["consistent"], true);
            assert_eq!(report["best_verified"]["diagnostic_only"], true);
            let xml = fs::read(dir.join(&name)).unwrap();
            assert!(String::from_utf8_lossy(&xml).contains("Rarity: RARE"));
            let fresh = success(
                cli(dir)
                    .args([
                        "evaluate",
                        &name,
                        "--backend",
                        "native",
                        "--metric",
                        "player.selected_hit_dps",
                    ])
                    .output()
                    .unwrap(),
            );
            assert_eq!(
                fresh["evaluation"]["measurements"][0]["value"],
                report["best_verified"]["assessment"]["objective_value"]
            );
            let companion: Value =
                serde_json::from_slice(&fs::read(dir.join(format!("{name}.data.json"))).unwrap())
                    .unwrap();
            assert_eq!(companion["backend"], fresh["evaluation"]["backend"]);
            if let Some(a) = &reference {
                same(a, &report);
                assert_eq!(reference_xml.as_ref().unwrap(), &xml);
            } else {
                reference = Some(report);
                reference_xml = Some(xml);
            }
        }
    }
}
#[test]
fn old_problem_schemas_and_unknown_item_lines_reject_before_any_output() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    for (schema, line) in [
        (4, None),
        (5, Some("Adds 1 to 2 Physical Damage to Attacks")),
        (5, Some("10% increased Attack Speed while on Full Life")),
        (5, Some("+1 to Level of all Skills")),
    ] {
        let mut p = problem();
        p["schema_version"] = json!(schema);
        if let Some(line) = line {
            p["weapons"][1]["item_text"] = json!(format!(
                "{}\n{line}",
                p["weapons"][1]["item_text"].as_str().unwrap()
            ));
        }
        prepare(dir, &p);
        let out = search(dir, "typed", 1)
            .args(["--export", "rejected.xml"])
            .output()
            .unwrap();
        assert!(!out.status.success());
        assert!(!dir.join("rejected.xml").exists());
        assert!(out.stdout.is_empty());
        if schema == 4 {
            assert!(String::from_utf8_lossy(&out.stderr).contains("require problem schema 5"));
        }
    }
}
#[test]
fn explicit_equip_level_can_empty_the_domain_without_using_item_level_or_calculations() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut p = problem();
    p["locks"]["weapon_id"] = json!("wooden-level-80");
    prepare(dir, &p);
    for mode in ["typed", "document"] {
        let r = success(
            search(dir, mode, 4)
                .args(["--export", "empty.xml"])
                .output()
                .unwrap(),
        );
        assert_eq!(r["termination"], "empty_legal_domain");
        assert_eq!(r["total_evaluations"], 0);
        assert!(r["native_candidate_preparation"].is_null());
        assert!(!dir.join("empty.xml").exists());
        for item in r["requirements"]["rejected_candidates"].as_array().unwrap() {
            assert_eq!(item["assessment"]["required"]["level"], 80);
            assert_eq!(item["assessment"]["available"]["level"], 60);
        }
    }
}
#[test]
fn custom_rule_grammar_and_critical_cap_are_injected_and_export_replays_them() {
    use poe_optimizer_data::game_data::bundled_snapshot;
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.character.critical_chance_cap = 17.5;
    let rule = package
        .item_modifier_rules
        .iter_mut()
        .find(|r| r.template.contains("Critical Hit Chance"))
        .unwrap();
    rule.template = rule
        .template
        .replace("Critical Hit Chance", "Critical Test Chance");
    package.refresh_section_digests().unwrap();
    fs::write(dir.join("custom.json"), package.canonical_bytes().unwrap()).unwrap();
    let mut p = problem();
    p["objective"]["constraints"] = json!([]);
    p["locks"] = json!({"class_id":10,"ascendancy":{"kind":"id","id":"Monk3"},"allocated_passives":[10364,24475],"weapon_id":"wooden-critical","support_loadout":["brutality_i","rapid_attacks_i"]});
    prepare(dir, &p);
    let original = success(search(dir, "typed", 1).output().unwrap());
    for item in p["weapons"].as_array_mut().unwrap() {
        item["item_text"] = json!(
            item["item_text"]
                .as_str()
                .unwrap()
                .replace("Critical Hit Chance", "Critical Test Chance")
        );
    }
    prepare(dir, &p);
    let mut reference = None;
    for mode in ["typed", "document"] {
        let name = format!("custom-{mode}.xml");
        let r = success(
            search(dir, mode, 4)
                .args(["--data", "custom.json", "--export", &name])
                .output()
                .unwrap(),
        );
        assert_eq!(r["total_evaluations"], 3);
        assert_eq!(r["search"]["verifications"][0]["consistent"], true);
        assert!(
            r["best_verified"]["assessment"]["objective_value"]["value"]
                .as_f64()
                .unwrap()
                < original["best_verified"]["assessment"]["objective_value"]["value"]
                    .as_f64()
                    .unwrap()
        );
        assert_ne!(r["data"]["identity"], original["data"]["identity"]);
        let fresh = success(
            cli(dir)
                .args([
                    "evaluate",
                    &name,
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
            fresh["evaluation"]["measurements"][0]["value"],
            r["best_verified"]["assessment"]["objective_value"]
        );
        if let Some(a) = &reference {
            same(a, &r);
        } else {
            reference = Some(r);
        }
    }
    let incompatible = search(dir, "typed", 1).output().unwrap();
    assert!(!incompatible.status.success());
}

#[test]
fn noncanonical_item_headers_reject_in_legacy_and_expanded_problem_schemas() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut p = problem();
    let canonical = p["weapons"][0]["item_text"].as_str().unwrap().to_owned();
    p["support_loadouts"] = json!([[]]);
    p["objective"]["constraints"] = json!([]);
    for schema in [4, 5] {
        p["schema_version"] = json!(schema);
        for source in [
            canonical.replace("Item Level: 1", "Item Level: 01"),
            canonical.replace("Quality: 0", "Quality: 00"),
            canonical.replace("Implicits: 0", "LevelReq: 01\nImplicits: 0"),
        ] {
            p["weapons"] = json!([{"id":"noncanonical-normal", "item_text":source}]);
            prepare(dir, &p);
            let rejected = search(dir, "typed", 1).output().unwrap();
            assert!(!rejected.status.success());
            assert!(rejected.stdout.is_empty());
            assert!(
                String::from_utf8_lossy(&rejected.stderr).contains("canonical unsigned integer")
            );
        }
    }
    p["weapons"] = json!([{"id":"canonical-normal", "item_text":canonical}]);
    prepare(dir, &p);
    let report = success(search(dir, "typed", 1).output().unwrap());
    assert_eq!(report["total_evaluations"], 4);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
}
