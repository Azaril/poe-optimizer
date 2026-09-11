//! Dataset-bound controlled search exercised through the public CLI.
use poe_optimizer_data::game_data::{
    GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy, bundled_snapshot,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const TEMPLATE: &str = include_str!("fixtures/calibration/mace-wooden.xml");
// These assertions cover dataset identity, ranking, budgets by evaluation count,
// and verified export. Hosted debug preparation can consume the old 30s wall
// limit before its first attempt; throughput is measured separately.
const SEARCH_COMPLETION_TIMEOUT_SECONDS: &str = "300";

fn cli(directory: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(directory);
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

fn problem() -> Value {
    json!({
        "schema_version": 1,
        "template": "build.xml",
        "weapons": [
            {"id": "wood", "item_text": "Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 0\nImplicits: 0"},
            {"id": "smith", "item_text": "Rarity: NORMAL\nSmithing Hammer\nItem Level: 1\nQuality: 0\nImplicits: 0"}
        ],
        "supports": ["none", "brutality_i"],
        "objective": {
            "schema_version": 1,
            "objective": {
                "kind": "scalar", "metric": {"actor": "player", "id": "selected_hit_dps"},
                "unit": "damage_per_second", "direction": "maximize"
            },
            "constraints": []
        }
    })
}

fn prepare(directory: &Path, problem: &Value, xml: &str) {
    fs::write(
        directory.join("problem.json"),
        serde_json::to_vec(problem).unwrap(),
    )
    .unwrap();
    fs::write(directory.join("build.xml"), xml).unwrap();
}

fn search(directory: &Path, data: Option<&Path>, jobs: usize, max: usize) -> Command {
    let mut command = cli(directory);
    command.args([
        "search-experimental",
        "--backend",
        "native",
        "--problem",
        "problem.json",
        "--strategy",
        "exhaustive",
        "--seed",
        "17",
        "--timeout-seconds",
        SEARCH_COMPLETION_TIMEOUT_SECONDS,
        "--pob",
        "absent-reference-checkout",
    ]);
    command.arg("--jobs").arg(jobs.to_string());
    command.arg("--max-evaluations").arg(max.to_string());
    if let Some(path) = data {
        command.arg("--data").arg(path);
    }
    command
}

fn write_package(directory: &Path, name: &str, mut package: GameDataPackage) -> (PathBuf, Value) {
    package.refresh_section_digests().unwrap();
    let bytes = package.canonical_bytes().unwrap();
    let snapshot =
        GameDataLoader::from_bytes(&bytes, &TrustPolicy::AllowCustom, &LoadLimits::default())
            .unwrap();
    let identity = serde_json::to_value(snapshot.identity()).unwrap();
    let path = directory.join(name);
    fs::write(&path, bytes).unwrap();
    (path, identity)
}

fn changed_damage() -> GameDataPackage {
    let mut package = bundled_snapshot().unwrap().package().clone();
    let wood = package
        .weapons
        .iter_mut()
        .find(|weapon| weapon.name == "Wooden Club")
        .unwrap();
    wood.physical_minimum *= 10.0;
    wood.physical_maximum *= 10.0;
    package
}

fn assert_verified(report: &Value, total: u64) {
    let diagnostic = serde_json::to_string_pretty(report).unwrap();
    assert_ne!(
        report["termination"], "time_budget",
        "Completion test exhausted its {SEARCH_COMPLETION_TIMEOUT_SECONDS}s limit: {diagnostic}"
    );
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["requested_backend"], "native-poe2");
    assert_eq!(report["execution_kind"], "rust_cpu");
    assert_eq!(report["preparation"]["attempts"], 1, "{diagnostic}");
    assert_eq!(
        report["total_evaluations"],
        total,
        "search report: {}",
        serde_json::to_string_pretty(report).unwrap()
    );
    assert_eq!(
        report["search"]["statistics"]["verification_evaluations"],
        1
    );
    assert_eq!(report["search"]["statistics"]["evaluation_failures"], 0);
    assert_eq!(report["search"]["verifications"][0]["consistent"], true);
    assert_eq!(
        report["preparation"]["evaluation"]["backend"]["data"],
        report["data"]["identity"]
    );
    assert!(!report["best_verified"].is_null(), "{report}");
}

fn recheck_export(directory: &Path, export: &Path, data: &Path, report: &Value) -> Value {
    let reevaluation = success(
        cli(directory)
            .arg("evaluate")
            .arg(export)
            .args([
                "--backend",
                "native",
                "--metric",
                "player.selected_hit_dps",
                "--data",
            ])
            .arg(data)
            .output()
            .unwrap(),
    );
    assert_eq!(
        reevaluation["evaluation"]["backend"],
        report["preparation"]["evaluation"]["backend"]
    );
    assert_eq!(
        reevaluation["evaluation"]["measurements"][0]["value"]["value"],
        report["best_verified"]["assessment"]["objective_value"]["value"]
    );
    let metadata: Value =
        serde_json::from_slice(&fs::read(export.with_extension("xml.data.json")).unwrap()).unwrap();
    assert_eq!(metadata["backend"], reevaluation["evaluation"]["backend"]);
    assert_eq!(metadata["data_trust"], report["data"]["trust"]);
    assert_eq!(
        metadata["uses_packaged_default"],
        report["data"]["uses_packaged_default"]
    );
    assert_eq!(
        metadata["xml_sha256"],
        report["best_verified"]["source_xml_sha256"]
    );
    reevaluation
}

#[test]
fn custom_data_changes_ranking_consistently_across_workers_and_survives_export() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &problem(), TEMPLATE);
    let default = success(search(temp.path(), None, 1, 6).output().unwrap());
    assert_verified(&default, 6);
    assert_eq!(default["best_verified"]["alternative_id"], "smith/none");
    assert_eq!(default["data"]["uses_packaged_default"], true);
    let (data, identity) = write_package(temp.path(), "custom.json", changed_damage());
    let export = temp.path().join("winner.xml");
    let serial = success(
        search(temp.path(), Some(&data), 1, 6)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    let parallel = success(search(temp.path(), Some(&data), 4, 6).output().unwrap());
    for report in [&serial, &parallel] {
        assert_verified(report, 6);
        assert_eq!(report["termination"], "finite_domain_processed");
        assert_eq!(report["data"]["identity"], identity);
        assert_eq!(report["data"]["trust"]["status"], "custom_unreviewed");
        assert_eq!(report["data"]["uses_packaged_default"], false);
        assert_eq!(
            report["requirements"]["legal_candidates"]
                .as_array()
                .unwrap()
                .len(),
            4
        );
        assert_eq!(report["requirements"]["rejected_candidates"], json!([]));
        assert_eq!(
            report["best_verified"]["alternative_id"],
            "wood/brutality_i"
        );
    }
    assert_ne!(
        serial["catalog"]["identity"],
        default["catalog"]["identity"]
    );
    assert_eq!(serial["search"]["feasible"], parallel["search"]["feasible"]);
    assert_eq!(serial["best_verified"], parallel["best_verified"]);
    recheck_export(temp.path(), &export, &data, &serial);
}

#[test]
fn matching_content_and_host_review_change_trust_without_changing_search_identity() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &problem(), TEMPLATE);
    let (packaged, _) = write_package(
        temp.path(),
        "reviewed.json",
        bundled_snapshot().unwrap().package().clone(),
    );
    let default = success(search(temp.path(), None, 1, 6).output().unwrap());
    let external = success(search(temp.path(), Some(&packaged), 1, 6).output().unwrap());
    assert_eq!(external["data"]["identity"], default["data"]["identity"]);
    assert_eq!(external["data"]["trust"], default["data"]["trust"]);
    assert_eq!(external["data"]["uses_packaged_default"], false);
    assert_eq!(external["catalog"], default["catalog"]);
    assert_eq!(external["best_verified"], default["best_verified"]);
    let (custom, _) = write_package(temp.path(), "custom.json", changed_damage());
    let unreviewed = success(search(temp.path(), Some(&custom), 1, 6).output().unwrap());
    let digest = format!("{:x}", Sha256::digest(fs::read(&custom).unwrap()));
    let reviewed = success(
        search(temp.path(), Some(&custom), 1, 6)
            .arg("--data-sha256")
            .arg(&digest)
            .output()
            .unwrap(),
    );
    assert_verified(&reviewed, 6);
    assert_eq!(
        reviewed["data"]["trust"],
        json!({"status":"reviewed","expected_sha256":digest})
    );
    assert_eq!(reviewed["data"]["uses_packaged_default"], false);
    assert_eq!(reviewed["data"]["identity"], unreviewed["data"]["identity"]);
    assert_eq!(reviewed["catalog"], unreviewed["catalog"]);
    assert_eq!(reviewed["best_verified"], unreviewed["best_verified"]);
}

fn attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// Custom identity fixtures must rename references in both the legacy numerical
// profile and the independently injected source loader. Exact string/key rewrites
// keep declarations, constructed rows, effect references, and lookup maps aligned.
// Provenance remains the original evidence; this is an unreviewed custom package.
fn rename_identity_references(value: &mut Value, renames: &BTreeMap<String, String>) {
    match value {
        Value::String(text) => {
            if let Some(replacement) = renames.get(text) {
                *text = replacement.clone();
            }
        }
        Value::Array(values) => {
            for value in values {
                rename_identity_references(value, renames);
            }
        }
        Value::Object(values) => {
            for (key, mut value) in std::mem::take(values) {
                if key != "source" {
                    rename_identity_references(&mut value, renames);
                }
                let key = renames.get(&key).cloned().unwrap_or(key);
                assert!(values.insert(key, value).is_none(), "fixture key collision");
            }
        }
        _ => {}
    }
}

#[test]
fn selected_dataset_gem_identifiers_names_and_quest_keys_round_trip_through_locked_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut package = bundled_snapshot().unwrap().package().clone();
    let mut xml = TEMPLATE.to_string();
    let mut renames = BTreeMap::new();
    for (field, xml_attribute) in [
        ("skill_id", "skillId"),
        ("game_id", "gemId"),
        ("variant_id", "variantId"),
        ("name", "nameSpec"),
    ] {
        let mut data = serde_json::to_value(&package).unwrap();
        let old = data["mace"][field].as_str().unwrap().to_string();
        let replacement = format!("{old} &'\"<> caf\u{e9}");
        xml = xml.replace(
            &format!("{xml_attribute}=\"{}\"", attribute(&old)),
            &format!("{xml_attribute}=\"{}\"", attribute(&replacement)),
        );
        renames.insert(old, replacement.clone());
        data["mace"][field] = json!(replacement);
        let support = data["supports"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["id"] == "brutality_i")
            .unwrap();
        let old = support[field].as_str().unwrap().to_string();
        let replacement = format!("{old} &'\"<> caf\u{e9}");
        renames.insert(old, replacement.clone());
        support[field] = json!(replacement);
        package = serde_json::from_value(data).unwrap();
    }
    let mut identities = serde_json::to_value(&package.skill_identities).unwrap();
    rename_identity_references(&mut identities, &renames);
    package.skill_identities = serde_json::from_value(identities).unwrap();
    let mut preparation = serde_json::to_value(&package.skill_preparation).unwrap();
    rename_identity_references(&mut preparation, &renames);
    // This ordered loader input is the one non-provenance field under `source`.
    rename_identity_references(&mut preparation["source"]["canonical_gem_order"], &renames);
    package.skill_preparation = serde_json::from_value(preparation).unwrap();
    for key in &mut package.quests.config_keys {
        key.push_str(" &'\"<> quest");
    }
    let selected_skill = package.mace.skill_id.clone();
    let selected_support = package.support("brutality_i").unwrap().skill_id.clone();
    let quest_keys = package.quests.config_keys.clone();
    let quest_inputs = quest_keys
        .iter()
        .map(|key| {
            format!(
                "      <Input name=\"{}\" boolean=\"false\"/>\n",
                attribute(key)
            )
        })
        .collect::<String>();
    xml = xml.replace(
        "    </ConfigSet>",
        &format!("{quest_inputs}    </ConfigSet>"),
    );
    let mut input = problem();
    input["locks"] = json!({"weapon_id":"wood", "support":"brutality_i"});
    prepare(temp.path(), &input, &xml);
    let (data, identity) = write_package(temp.path(), "quoted.json", package);
    let export = temp.path().join("quoted.xml");
    let report = success(
        search(temp.path(), Some(&data), 4, 3)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    assert_verified(&report, 3);
    assert_eq!(
        report["best_verified"]["alternative_id"],
        "wood/brutality_i"
    );
    assert_eq!(report["data"]["identity"], identity);
    assert_eq!(
        report["candidate_constraints"]["required_skill_ids"],
        json!([selected_skill])
    );
    assert_eq!(
        report["requirements"]["legal_candidates"],
        json!(["wood/brutality_i"])
    );
    let exported = fs::read_to_string(&export).unwrap();
    assert!(
        exported.contains(&attribute(&selected_support)),
        "{exported}"
    );
    for key in quest_keys {
        assert!(exported.contains(&attribute(&key)), "{exported}");
    }
    let result = recheck_export(temp.path(), &export, &data, &report);
    assert_eq!(
        result["evaluation"]["coverage"]["selected_player"]["skill_id"],
        selected_skill
    );
}

#[test]
fn legacy_identity_changes_without_source_loader_changes_never_publish_results() {
    let temp = tempfile::tempdir().unwrap();
    let mut package = bundled_snapshot().unwrap().package().clone();
    let original = package.mace.skill_id.clone();
    package.mace.skill_id.push_str(" caller-only");
    let xml = TEMPLATE.replace(&original, &package.mace.skill_id);
    prepare(temp.path(), &problem(), &xml);
    let (data, _) = write_package(temp.path(), "inconsistent.json", package);
    let export = temp.path().join("inconsistent.xml");
    let report = success(
        search(temp.path(), Some(&data), 1, 6)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    assert_eq!(report["termination"], "preparation_failed", "{report:#}");
    assert_eq!(report["total_evaluations"], 1, "{report:#}");
    assert_eq!(
        report["preparation"]["error"],
        "Authored skill resolution differs from the closed numerical adapter",
        "{report:#}"
    );
    assert!(report["search"].is_null(), "{report:#}");
    assert!(report["best_verified"].is_null(), "{report:#}");
    assert_eq!(report["export"]["status"], "not_written", "{report:#}");
    assert!(!export.exists());
    assert!(!export.with_extension("xml.data.json").exists());
}

#[test]
fn invalid_digest_and_reference_backend_data_selection_never_publish_results() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &problem(), TEMPLATE);
    let (data, _) = write_package(temp.path(), "custom.json", changed_damage());
    let result = temp.path().join("result.json");
    let export = temp.path().join("result.xml");
    let fail = search(temp.path(), Some(&data), 1, 6)
        .arg("--data-sha256")
        .arg("0".repeat(64))
        .arg("--output")
        .arg(&result)
        .arg("--export")
        .arg(&export)
        .output()
        .unwrap();
    assert!(!fail.status.success());
    assert!(String::from_utf8_lossy(&fail.stderr).contains("SHA-256"));
    assert!(
        !result.exists() && !export.exists() && !export.with_extension("xml.data.json").exists()
    );
    let fail = cli(temp.path())
        .args([
            "search-experimental",
            "--backend",
            "native",
            "--problem",
            "problem.json",
            "--data-sha256",
        ])
        .arg("0".repeat(64))
        .output()
        .unwrap();
    assert!(!fail.status.success());
    #[cfg(feature = "pob")]
    {
        let fail = cli(temp.path())
            .args([
                "search-experimental",
                "--backend",
                "pob",
                "--problem",
                "problem.json",
                "--data",
            ])
            .arg(&data)
            .arg("--pob")
            .arg("absent-reference-checkout")
            .arg("--output")
            .arg(&result)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap();
        assert!(!fail.status.success());
        assert!(String::from_utf8_lossy(&fail.stderr).contains("--data"));
        assert!(
            !result.exists()
                && !export.exists()
                && !export.with_extension("xml.data.json").exists()
        );
    }
}

#[test]
fn requirement_filtering_keeps_maximum_semantics_and_respects_fresh_evaluation_budget() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &problem(), TEMPLATE);
    let mut package = bundled_snapshot().unwrap().package().clone();
    let strength = package
        .tree
        .class(package.mace.default_class_id)
        .unwrap()
        .base_strength;
    for weapon in &mut package.weapons {
        weapon.requirements.attributes.strength = strength;
    }
    package.mace.requirements.attributes.strength = strength;
    package.mace.support_attribute_costs.strength = strength;
    let (legal, _) = write_package(temp.path(), "at-maximum.json", package.clone());
    let at_maximum = success(search(temp.path(), Some(&legal), 4, 6).output().unwrap());
    assert_verified(&at_maximum, 6);
    assert_eq!(
        at_maximum["requirements"]["legal_candidates"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    // Individual equipment/gem and aggregate support requirements share a maximum,
    // so three requirements equal to available Strength must not be added.
    package.mace.support_attribute_costs.strength = strength + 1;
    let (over, _) = write_package(temp.path(), "over-maximum.json", package);
    let filtered = success(search(temp.path(), Some(&over), 4, 6).output().unwrap());
    assert_verified(&filtered, 4);
    assert_eq!(
        filtered["requirements"]["legal_candidates"],
        json!(["smith/none", "wood/none"])
    );
    let rejected = filtered["requirements"]["rejected_candidates"]
        .as_array()
        .unwrap();
    assert_eq!(rejected.len(), 2);
    for entry in rejected {
        assert!(
            entry["alternative_id"]
                .as_str()
                .unwrap()
                .ends_with("/brutality_i")
        );
        assert_eq!(entry["assessment"]["available"]["strength"], strength);
        assert_eq!(entry["assessment"]["required"]["strength"], strength + 1);
        assert!(
            !entry["assessment"]["violations"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    assert_eq!(filtered["search"]["feasible"].as_array().unwrap().len(), 2);
    let limited = success(search(temp.path(), Some(&over), 4, 3).output().unwrap());
    assert_verified(&limited, 3);
    assert_eq!(limited["search"]["feasible"].as_array().unwrap().len(), 1);
    assert_eq!(limited["termination"], "evaluation_budget");
}

#[test]
fn locked_empty_legal_domain_reports_requirements_without_calculation_or_export() {
    let temp = tempfile::tempdir().unwrap();
    let mut package = bundled_snapshot().unwrap().package().clone();
    let strength = package
        .tree
        .class(package.mace.default_class_id)
        .unwrap()
        .base_strength;
    package.mace.support_attribute_costs.strength = strength + 1;
    let (data, _) = write_package(temp.path(), "illegal-support.json", package);
    let mut input = problem();
    input["locks"] = json!({"weapon_id":"wood", "support":"brutality_i"});
    prepare(temp.path(), &input, TEMPLATE);
    let export = temp.path().join("never.xml");
    let report = success(
        search(temp.path(), Some(&data), 4, 3)
            .arg("--export")
            .arg(&export)
            .output()
            .unwrap(),
    );
    assert_eq!(report["schema_version"], 2);
    assert_eq!(report["termination"], "empty_legal_domain");
    assert_eq!(report["total_evaluations"], 0);
    assert_eq!(report["preparation"]["attempts"], 0);
    assert!(report["search"].is_null());
    assert!(report["best_verified"].is_null());
    assert_eq!(report["requirements"]["legal_candidates"], json!([]));
    assert_eq!(
        report["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        report["requirements"]["rejected_candidates"][0]["alternative_id"],
        "wood/brutality_i"
    );
    assert_eq!(report["export"]["status"], "not_written");
    assert!(!export.exists() && !export.with_extension("xml.data.json").exists());

    // Legality preflight is independent of the caller's proposal budget: even an
    // unenumerated four-choice domain can report why every choice is impossible.
    let mut package = bundled_snapshot().unwrap().package().clone();
    package.mace.requirements.level = 61;
    let (data, _) = write_package(temp.path(), "illegal-level.json", package);
    prepare(temp.path(), &problem(), TEMPLATE);
    let report = success(
        search(temp.path(), Some(&data), 4, 3)
            .args(["--max-proposals", "1"])
            .output()
            .unwrap(),
    );
    assert_eq!(report["termination"], "empty_legal_domain");
    assert_eq!(report["total_evaluations"], 0);
    assert_eq!(report["preparation"]["attempts"], 0);
    assert_eq!(
        report["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        4
    );
    assert_eq!(report["requirements"]["legal_candidates"], json!([]));
    for candidate in report["requirements"]["rejected_candidates"]
        .as_array()
        .unwrap()
    {
        assert_eq!(candidate["assessment"]["available"]["level"], 60);
        assert_eq!(candidate["assessment"]["required"]["level"], 61);
    }
}

#[test]
fn guided_search_recovers_from_an_illegal_first_weapon_without_dispatching_it() {
    let temp = tempfile::tempdir().unwrap();
    prepare(temp.path(), &problem(), TEMPLATE);
    let mut package = bundled_snapshot().unwrap().package().clone();
    let strength = package
        .tree
        .class(package.mace.default_class_id)
        .unwrap()
        .base_strength;
    let smith = package
        .weapons
        .iter_mut()
        .find(|weapon| weapon.name == "Smithing Hammer")
        .unwrap();
    smith.requirements.attributes.strength = strength + 1;
    let (data, _) = write_package(temp.path(), "illegal-first-weapon.json", package);
    let report = success(
        cli(temp.path())
            .args([
                "search-experimental",
                "--backend",
                "native",
                "--problem",
                "problem.json",
                "--strategy",
                "guided",
                "--jobs",
                "4",
                "--max-evaluations",
                "4",
                "--seed",
                "17",
                "--max-rounds",
                "16",
                "--max-proposals",
                "64",
                "--data",
            ])
            .arg(&data)
            .output()
            .unwrap(),
    );
    assert_verified(&report, 4);
    assert_eq!(
        report["best_verified"]["alternative_id"],
        "wood/brutality_i"
    );
    assert_eq!(
        report["requirements"]["legal_candidates"],
        json!(["wood/none", "wood/brutality_i"])
    );
    assert_eq!(
        report["requirements"]["rejected_candidates"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let feasible = report["search"]["feasible"].as_array().unwrap();
    assert_eq!(feasible.len(), 2);
    assert!(
        feasible
            .iter()
            .all(|entry| entry["candidate"]["choices"][0] == 1)
    );
}
