#![cfg(feature = "pob")]
//! Fresh command processes independently regenerate the reviewed package, load
//! it through the native evaluator, and exercise preflight failure boundaries.
use poe_optimizer_data::game_data::{
    GameDataLoader, LoadLimits, TrustPolicy, bundled_package_bytes, bundled_package_sha256,
    bundled_snapshot,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const SECTIONS: [&str; 28] = [
    "tree",
    "character",
    "actor",
    "receiving_defence",
    "quests",
    "spark",
    "mace",
    "supports",
    "weapons",
    "item_modifier_rules",
    "defence",
    "monsters",
    "encounters",
    "passive_effects",
    "passive_exclusions",
    "jewellery_bases",
    "armour_bases",
    "item_formatting",
    "movement",
    "action_speed",
    "direct_action_timing",
    "configuration",
    "skill_identities",
    "item_loading",
    "item_scalability",
    "modifier_parser",
    "unique_requirements",
    "item_assembly",
];
fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
fn source() -> PathBuf {
    repository().join("vendor/path-of-building-poe2")
}
fn cli() -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"));
    command.current_dir(repository());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    command
}
fn evidence_path(package: &Path) -> PathBuf {
    let mut path = OsString::from(package.as_os_str());
    path.push(".extraction.json");
    PathBuf::from(path)
}
fn success(output: Output) -> Value {
    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "Invalid JSON stdout: {error}: {}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}
fn failure(output: Output) -> String {
    assert!(
        !output.status.success(),
        "CLI unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(output.stdout.is_empty(), "Failure emitted a success report");
    String::from_utf8(output.stderr).unwrap()
}
fn extract(output: &Path, explicit_source: bool) -> Value {
    let mut command = cli();
    // This checks complete catalog/evidence reproducibility, not extraction speed.
    // Hosted debug builds have exceeded 30 seconds; deadline rejection is tested separately.
    command
        .args(["extract-game-data", "--timeout-seconds", "120", "--output"])
        .arg(output);
    if explicit_source {
        // An explicit source path must work independently of the working directory.
        command
            .arg("--pob")
            .arg(source())
            .current_dir(output.parent().unwrap());
    }
    success(command.output().unwrap())
}
fn evaluate(package: Option<&Path>) -> Value {
    let mut command = cli();
    command
        .arg("evaluate")
        .arg(repository().join("tests/fixtures/calibration/spark-mapping.xml"))
        .args(["--backend", "native"]);
    if let Some(package) = package {
        command.arg("--data").arg(package);
    }
    success(command.output().unwrap())
}
fn measurements(report: &Value) -> BTreeMap<String, f64> {
    report["evaluation"]["measurements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|metric| {
            (
                metric["query"]["id"].as_str().unwrap().to_owned(),
                metric["value"]["value"].as_f64().unwrap(),
            )
        })
        .collect()
}
fn assert_no_outputs(path: &Path) {
    assert!(
        !path.exists(),
        "Unexpected package output {}",
        path.display()
    );
    assert!(
        !evidence_path(path).exists(),
        "Unexpected evidence output {}",
        path.display()
    );
}

#[test]
fn fresh_cli_extractions_reproduce_all_twenty_eight_sections_and_stable_source_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first extracted package.json");
    let second = temp.path().join("second extracted package.json");
    let report_a = extract(&first, false);
    let report_b = extract(&second, true);
    let bytes_a = fs::read(&first).unwrap();
    let bytes_b = fs::read(&second).unwrap();
    assert_eq!(
        bytes_a,
        bundled_package_bytes(),
        "Fresh extraction differs from reviewed package bytes"
    );
    assert_eq!(
        bytes_a, bytes_b,
        "Fresh extraction depends on process or source working directory"
    );
    let digest = format!("{:x}", Sha256::digest(&bytes_a));
    assert_eq!(digest, bundled_package_sha256());
    let expected: Value = serde_json::from_slice(bundled_package_bytes()).unwrap();
    let actual: Value = serde_json::from_slice(&bytes_a).unwrap();
    assert_eq!(
        actual["manifest"]["section_sha256"]
            .as_object()
            .unwrap()
            .len(),
        SECTIONS.len()
    );
    for section in SECTIONS {
        assert_eq!(
            actual[section], expected[section],
            "Section {section} differs from reviewed data"
        );
        assert_eq!(
            actual["manifest"]["section_sha256"][section],
            expected["manifest"]["section_sha256"][section],
            "Section digest {section}"
        );
    }
    let snapshot = GameDataLoader::from_bytes(
        &bytes_a,
        &TrustPolicy::Reviewed {
            expected_sha256: digest.clone(),
        },
        &LoadLimits::default(),
    )
    .unwrap();
    let expected_identity = serde_json::to_value(snapshot.identity()).unwrap();
    assert_eq!(snapshot.identity(), bundled_snapshot().unwrap().identity());
    let evidence_a = fs::read(evidence_path(&first)).unwrap();
    let evidence_b = fs::read(evidence_path(&second)).unwrap();
    assert_eq!(
        evidence_a, evidence_b,
        "Extraction evidence contains unstable process/path/timing state"
    );
    let evidence: Value = serde_json::from_slice(&evidence_a).unwrap();
    assert_eq!(evidence["schema_version"], 1);
    assert_eq!(
        evidence["package_schema_version"],
        actual["manifest"]["schema_version"]
    );
    assert_eq!(
        evidence["semantics_version"],
        actual["manifest"]["semantics_version"]
    );
    assert_eq!(evidence["package_sha256"], digest);
    let manifest_text = include_str!("../crates/poe-optimizer-pob/data/pob-source-manifest.json");
    let manifest: Value = serde_json::from_str(manifest_text).unwrap();
    assert_eq!(evidence["upstream_revision"], manifest["upstream_revision"]);
    assert_eq!(
        evidence["source_manifest_sha256"],
        format!(
            "{:x}",
            Sha256::digest(manifest_text.replace("\r\n", "\n").as_bytes())
        )
    );
    let policy = include_str!("../crates/poe-optimizer-pob/src/game_data_policy.json");
    assert_eq!(
        evidence["policy_sha256"],
        format!(
            "{:x}",
            Sha256::digest(policy.replace("\r\n", "\n").as_bytes())
        )
    );
    let extractor_digest = evidence["extractor_sha256"].as_str().unwrap();
    assert_eq!(extractor_digest.len(), 64);
    assert!(
        extractor_digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    );
    let source_files = evidence["source_files_sha256"].as_object().unwrap();
    assert_eq!(source_files.len(), 129);
    assert!(source_files.contains_key("src/Classes/SkillsTab.lua"));
    assert!(source_files.contains_key("src/Data/Bosses.lua"));
    assert!(source_files.contains_key("src/Data/BossSkills.lua"));
    assert_eq!(actual["unique_requirements"]["state"]["status"], "complete");
    for (section, source_inventory) in [
        (
            "skill_identities",
            &actual["skill_identities"]["source"]["files"],
        ),
        ("item_loading", &actual["item_loading"]["source"]["files"]),
        ("item_assembly", &actual["item_assembly"]["source"]["files"]),
        (
            "modifier_parser",
            &actual["modifier_parser"]["source"]["files"],
        ),
        (
            "unique_requirements",
            &actual["unique_requirements"]["state"]["source"]["files"],
        ),
    ] {
        for (path, digest) in source_inventory.as_object().unwrap() {
            assert_eq!(
                source_files.get(path),
                Some(digest),
                "{section} dependency {path}"
            );
        }
    }
    for (path, recorded_hash) in source_files {
        let entry = manifest["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|entry| entry["path"] == *path)
            .unwrap();
        assert_eq!(
            recorded_hash, &entry["sha256"],
            "Manifest evidence for {path}"
        );
        let source_text = fs::read_to_string(source().join(path)).unwrap();
        assert_eq!(
            *recorded_hash,
            format!(
                "{:x}",
                Sha256::digest(source_text.replace("\r\n", "\n").as_bytes())
            ),
            "Actual source bytes for {path}"
        );
    }
    for (path, digest) in actual["manifest"]["provenance"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(path, _)| path.starts_with("src/"))
    {
        assert_eq!(
            source_files.get(path),
            Some(digest),
            "Package provenance {path}"
        );
    }
    for (report, path) in [(&report_a, &first), (&report_b, &second)] {
        assert_eq!(report["schema_version"], 1);
        assert_eq!(report["status"], "extracted_game_data");
        assert_eq!(report["package_sha256"], digest);
        assert_eq!(report["package_bytes"], bytes_a.len());
        assert_eq!(report["data"], expected_identity);
        assert_eq!(
            report["section_sha256"],
            actual["manifest"]["section_sha256"]
        );
        assert_eq!(report["evidence"], evidence);
        assert_eq!(Path::new(report["output"].as_str().unwrap()), path);
        assert_eq!(
            Path::new(report["evidence_output"].as_str().unwrap()),
            evidence_path(path)
        );
    }
    // This follows the public loader and evaluator boundary, using the extracted
    // artifact directly rather than manufacturing another package inside the test.
    let baseline = evaluate(None);
    let imported = evaluate(Some(&first));
    assert_eq!(imported["evaluation"]["backend"]["data"], expected_identity);
    assert_eq!(
        imported["evaluation"]["backend"],
        baseline["evaluation"]["backend"]
    );
    assert_eq!(measurements(&imported), measurements(&baseline));
    let golden: Value = serde_json::from_str(include_str!(
        "fixtures/calibration/spark-mapping.reference.json"
    ))
    .unwrap();
    let values = measurements(&imported);
    for (metric, source) in [
        ("life", "Life"),
        ("mana", "Mana"),
        ("energy_shield", "EnergyShield"),
        ("fire_resistance_capped_pct", "FireResist"),
        ("cold_resistance_capped_pct", "ColdResist"),
        ("lightning_resistance_capped_pct", "LightningResist"),
        ("chaos_resistance_capped_pct", "ChaosResist"),
        ("selected_hit_dps", "TotalDPS"),
        ("selected_average_hit", "AverageHit"),
    ] {
        let expected = golden["metrics"][source].as_f64().unwrap();
        assert!(
            (values[metric] - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            "Extracted-package evaluation {metric}: {} vs {expected}",
            values[metric]
        );
    }
}

#[test]
fn package_or_sidecar_collisions_preserve_existing_bytes_and_create_no_other_output() {
    for existing_package in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("occupied package.json");
        let evidence = evidence_path(&package);
        let occupied = if existing_package {
            &package
        } else {
            &evidence
        };
        let sentinel = b"preserve existing bytes\n\0";
        fs::write(occupied, sentinel).unwrap();
        let message = failure(
            cli()
                .args(["extract-game-data", "--output"])
                .arg(&package)
                .output()
                .unwrap(),
        );
        assert!(
            message.to_lowercase().contains("exist"),
            "Expected output preflight rejection: {message}"
        );
        assert_eq!(fs::read(occupied).unwrap(), sentinel);
        let other = if existing_package {
            &evidence
        } else {
            &package
        };
        assert!(
            !other.exists(),
            "Preflight created {} despite occupied companion output",
            other.display()
        );
    }
}

#[test]
fn invalid_sources_and_zero_deadline_fail_without_creating_package_or_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing PoB source");
    let empty = temp.path().join("empty PoB source");
    fs::create_dir(&empty).unwrap();
    let file = temp.path().join("not a source directory");
    fs::write(&file, b"source sentinel").unwrap();
    for (index, source) in [&missing, &empty, &file].into_iter().enumerate() {
        let package = temp.path().join(format!("invalid-source-{index}.json"));
        failure(
            cli()
                .args(["extract-game-data", "--pob"])
                .arg(source)
                .args(["--timeout-seconds", "30", "--output"])
                .arg(&package)
                .output()
                .unwrap(),
        );
        assert_no_outputs(&package);
    }
    assert_eq!(fs::read(&file).unwrap(), b"source sentinel");
    let package = temp.path().join("zero-deadline.json");
    let message = failure(
        cli()
            .args(["extract-game-data", "--pob"])
            .arg(source())
            .args(["--timeout-seconds", "0", "--output"])
            .arg(&package)
            .output()
            .unwrap(),
    );
    assert!(
        message.to_lowercase().contains("timeout") || message.to_lowercase().contains("deadline"),
        "Zero budget should reject explicitly: {message}"
    );
    assert_no_outputs(&package);
}
