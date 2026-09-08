//! Actor configuration survives joint search, requirement admission, and fresh exports.
use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
const TEMPLATE: &str = include_str!("fixtures/builds/mace-actor-resources.xml");
fn problem() -> Value {
    let mut p: Value =
        serde_json::from_str(include_str!("../examples/mace-actor-search.json")).unwrap();
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
fn actor_config_joint_search_matches_document_typed_and_rayon_with_fresh_exports() {
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
            assert_eq!(report["schema_version"], 7);
            assert_eq!(
                report["scope"],
                "mace_actor_local_weapon_class_passive_support_loadouts_v1"
            );
            assert_eq!(report["admission"]["checked_candidates"], 84);
            assert_eq!(
                report["requirements"]["legal_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                70
            );
            assert_eq!(
                report["requirements"]["rejected_candidates"]
                    .as_array()
                    .unwrap()
                    .len(),
                14
            );
            assert_eq!(report["total_evaluations"], 72);
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
            let spirit = success(
                cli(dir)
                    .args([
                        "evaluate",
                        &name,
                        "--backend",
                        "native",
                        "--metric",
                        "player.spirit",
                    ])
                    .output()
                    .unwrap(),
            );
            assert_eq!(
                spirit["evaluation"]["measurements"][0]["value"]["value"],
                130.0
            );
            assert!(String::from_utf8_lossy(&xml).contains("Actor study"));
            if mode == "typed" {
                assert_eq!(
                    report["native_candidate_preparation"]["actor_preparations"],
                    2
                );
            }
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
fn actor_requirements_can_empty_domain_before_calculation_and_preserve_input() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut p = problem();
    p["locks"]["support_loadout"] = json!(["rapid_attacks_i"]);
    prepare(dir, &p);
    let xml = TEMPLATE.replace("+20 to Dexterity", "-100 to Dexterity");
    fs::write(dir.join("build.xml"), &xml).unwrap();
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
            assert_eq!(item["assessment"]["available"]["dexterity"], 0);
        }
    }
    assert_eq!(fs::read_to_string(dir.join("build.xml")).unwrap(), xml);
}
#[test]
fn legacy_schema_and_unsupported_actor_mechanics_fail_before_outputs() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    for (schema, line) in [
        (5, "+20 to Strength"),
        (6, "20% increased Minion Damage"),
        (6, "50% of Maximum Life Converted to Energy Shield"),
        (6, "Chaos Inoculation"),
        (6, "+20 to Strength while on Full Life"),
        (6, "+20 to Strength trailing unparsed text"),
    ] {
        let mut p = problem();
        p["schema_version"] = json!(schema);
        prepare(dir, &p);
        fs::write(
            dir.join("build.xml"),
            TEMPLATE.replace("+20 to Strength", line),
        )
        .unwrap();
        let out = search(dir, "typed", 1)
            .args(["--export", "rejected.xml"])
            .output()
            .unwrap();
        assert!(!out.status.success(), "accepted {schema}/{line}");
        assert!(out.stdout.is_empty());
        assert!(!dir.join("rejected.xml").exists());
        if schema == 5 {
            assert!(String::from_utf8_lossy(&out.stderr).contains("schema 6"));
        }
    }
}
#[test]
fn injected_actor_grammar_quest_values_and_precision_replay_with_selected_dataset() {
    use poe_optimizer_data::{game_data::ActorNumericOperation, game_data::bundled_snapshot};
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut p = problem();
    p["objective"]["constraints"] = json!([]);
    p["locks"] = json!({"class_id":10,"ascendancy":{"kind":"id","id":"Monk3"},"allocated_passives":[10364,24475],"weapon_id":"wooden-balanced","support_loadout":["brutality_i","rapid_attacks_i"]});
    prepare(dir, &p);
    let original = success(search(dir, "typed", 1).output().unwrap());
    let mut package = bundled_snapshot().unwrap().package().clone();
    for rule in &mut package.actor.modifier_rules {
        rule.template = rule.template.replace("Dexterity", "Test Dexterity");
    }
    if let poe_optimizer_data::game_data::ActorModifierEffect::Numeric { value, .. } =
        &mut package.actor.spirit_quests[0].modifiers[0].effect
    {
        *value += 7.0;
    } else {
        panic!("numeric Spirit quest");
    }
    package
        .actor
        .high_precision_mods
        .entry("Spirit".into())
        .or_default()
        .insert(ActorNumericOperation::More, 6);
    package.refresh_section_digests().unwrap();
    fs::write(dir.join("custom.json"), package.canonical_bytes().unwrap()).unwrap();
    fs::write(
        dir.join("build.xml"),
        TEMPLATE.replace("Dexterity", "Test Dexterity").replace(
            "+30 to Spirit",
            "+30 to Spirit\n1% more Spirit\n1% more Spirit\n1% more Spirit\n1% more Spirit",
        ),
    )
    .unwrap();
    let mut reference = None;
    for mode in ["typed", "document"] {
        let name = format!("custom-{mode}.xml");
        let report = success(
            search(dir, mode, 4)
                .args(["--data", "custom.json", "--export", &name])
                .output()
                .unwrap(),
        );
        assert_eq!(report["total_evaluations"], 3);
        assert_ne!(report["data"]["identity"], original["data"]["identity"]);
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
                    "player.spirit",
                ])
                .output()
                .unwrap(),
        );
        assert_eq!(
            fresh["evaluation"]["measurements"][0]["value"]["value"],
            143.0
        );
        if let Some(a) = &reference {
            same(a, &report);
        } else {
            reference = Some(report);
        }
    }
    assert!(!search(dir, "typed", 1).output().unwrap().status.success());
}

#[cfg(feature = "pob")]
#[test]
fn explicit_reference_search_validates_actor_blocks_legacy_migration_and_finalist() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path();
    let mut p = problem();
    p["locks"] = json!({"class_id":10,"ascendancy":{"kind":"id","id":"Monk3"},"allocated_passives":[10364,24475],"weapon_id":"wooden-balanced","support_loadout":["brutality_i","rapid_attacks_i"]});
    let pob = Path::new(env!("CARGO_MANIFEST_DIR")).join("vendor/path-of-building-poe2");
    prepare(dir, &p);
    let native = success(search(dir, "typed", 1).output().unwrap());
    for style in ["block", "legacy"] {
        let xml = if style == "legacy" {
            let start = TEMPLATE.find("<CustomModifierBlock").unwrap();
            let end = TEMPLATE[start..].find("</CustomModifierBlock>").unwrap()
                + start
                + "</CustomModifierBlock>".len();
            let text_start = TEMPLATE[start..].find('>').unwrap() + start + 1;
            let text_end = TEMPLATE[start..].find("</CustomModifierBlock>").unwrap() + start;
            let text = &TEMPLATE[text_start..text_end];
            format!(
                "{}<Input name=\"customMods\" string=\"{}\"/>{}",
                &TEMPLATE[..start],
                text,
                &TEMPLATE[end..]
            )
        } else {
            TEMPLATE.to_owned()
        };
        fs::write(dir.join("build.xml"), &xml).unwrap();
        let name = format!("pob-{style}.xml");
        let report = success(
            cli(dir)
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
                    "--jobs",
                    "1",
                    "--export",
                    &name,
                    "--pob",
                ])
                .arg(&pob)
                .output()
                .unwrap(),
        );
        assert_eq!(report["total_evaluations"], 3, "{report}");
        assert_eq!(report["search"]["verifications"][0]["consistent"], true);
        assert_eq!(
            report["best_verified"]["assessment"],
            native["best_verified"]["assessment"]
        );
        let exported = fs::read_to_string(dir.join(&name)).unwrap();
        assert!(exported.contains(if style == "legacy" {
            "name=\"customMods\""
        } else {
            "CustomModifierBlock"
        }));
        assert!(exported.contains("+20 to Strength"));
    }
}
