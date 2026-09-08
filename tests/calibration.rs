#![cfg(feature = "pob")]

//! Independent host/extractor parity for shared PoB calculations, not independent
//! certification of game mechanics. Expected values come from the standalone
//! bundled-DLL harness documented in docs/calibration-reference.md, never this CLI.
use poe_optimizer_core::{
    EvaluationSnapshot,
    coverage::{SkillActor, SkillOrigin, SkillResolution},
    evaluation::EvaluationResult,
    metrics::{ActorScope, MeasurementValue, MetricDefinition, MetricMeasurement, MetricUnit},
    options::{
        BossKind, DamageAmounts, EncounterOverrides, EvaluationOptions, Scalar, SkillSelection,
    },
};
use serde_json::Value;
use std::{fs, path::PathBuf, process::Command};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn assert_close(name: &str, actual: f64, expected: f64, absolute: f64, relative: f64) {
    let tolerance = absolute.max(expected.abs() * relative);
    assert!(
        actual.is_finite() && (actual - expected).abs() <= tolerance,
        "{name}: actual {actual}, independent reference {expected}, tolerance {tolerance}"
    );
}

fn measurement<'a>(
    evaluation: &'a EvaluationResult,
    actor: ActorScope,
    id: &str,
) -> &'a MetricMeasurement {
    let mut matches = evaluation
        .measurements
        .iter()
        .filter(|value| value.query.actor == actor && value.query.id == id);
    let value = matches
        .next()
        .unwrap_or_else(|| panic!("Missing {actor:?}.{id}"));
    assert!(matches.next().is_none(), "Duplicate {actor:?}.{id}");
    assert_eq!(value.schema_version, 1);
    value
}

fn compare_reference(scenario: &str) {
    let directory = root().join("tests/fixtures/calibration");
    let expected: Value = serde_json::from_slice(
        &fs::read(directory.join(format!("{scenario}.reference.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(expected["schema_version"], 1);
    assert_eq!(
        expected["method"],
        "bundled-dll-independent-host-direct-main"
    );
    assert_eq!(expected["runtime"]["lua_version"], "Lua 5.1");
    assert_eq!(expected["runtime"]["jit_version"], "LuaJIT 2.1.1784580905");
    assert_eq!(expected["runtime"]["architecture"], "x64");
    assert!(
        expected["comparison"]["scope"]
            .as_str()
            .unwrap()
            .contains("not independent game-mechanics certification")
    );
    let provenance = &expected["provenance"];
    for field in [
        "source_manifest_sha256_lf",
        "fixture_sha256",
        "lua51_dll_sha256",
        "lua_utf8_dll_sha256",
        "driver_source_sha256_lf",
        "driver_executable_sha256",
        "harness_sha256_lf",
        "generator_sha256_lf",
    ] {
        let hash = provenance[field].as_str().unwrap();
        assert_eq!(hash.len(), 64, "Reference provenance {field}");
        assert!(hash.bytes().all(|byte| byte.is_ascii_hexdigit()));
    }
    assert_eq!(
        provenance["upstream_revision"],
        poe_optimizer_pob::runtime::UPSTREAM_REVISION
    );
    let absolute = expected["comparison"]["absolute_tolerance"]
        .as_f64()
        .unwrap();
    let relative = expected["comparison"]["relative_tolerance"]
        .as_f64()
        .unwrap();
    assert_eq!(absolute, 1e-8);
    assert_eq!(relative, 1e-9);

    // Explicitly select the reference's action and restate its encounter. This
    // also checks that equivalent options preserve the imported assumptions.
    let config = &expected["config"];
    let options = EvaluationOptions {
        selection: Some(SkillSelection {
            socket_group: 1,
            active_skill: Some(1),
            minion_skill: None,
        }),
        encounter: Some(EncounterOverrides {
            name: scenario.into(),
            enemy_level: Some(config["enemyLevel"].as_u64().unwrap().try_into().unwrap()),
            boss: Some(match config["enemyIsBoss"].as_str().unwrap() {
                "None" => BossKind::Normal,
                "Pinnacle" => BossKind::Pinnacle,
                other => panic!("Unreviewed reference encounter: {other}"),
            }),
            incoming_hit: Some(DamageAmounts {
                physical: config["enemyPhysicalDamage"].as_f64().unwrap(),
                ..DamageAmounts::default()
            }),
        }),
    };
    let scratch = tempfile::tempdir().unwrap();
    let options_path = scratch.path().join("options.json");
    fs::write(&options_path, serde_json::to_vec(&options).unwrap()).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .current_dir(root())
        .arg("evaluate")
        .arg(directory.join(format!("{scenario}.xml")))
        .args(["--timeout-seconds", "60", "--raw", "--options"])
        .arg(options_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{scenario}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).expect("CLI emits one JSON report");
    assert_eq!(report["schema_version"], 3);
    assert_eq!(report["status"], "experimental_evaluation");
    assert_eq!(report["source"]["format"], "raw_xml");
    assert_eq!(report["source"]["xml_sha256"], provenance["fixture_sha256"]);
    let evaluation: EvaluationResult =
        serde_json::from_value(report["evaluation"].clone()).unwrap();
    assert!(evaluation.diagnostic_only);
    assert_eq!(evaluation.backend.id, "pob-poe2-mlua");
    assert_eq!(
        evaluation.backend.rules_revision,
        provenance["upstream_revision"]
    );
    assert_eq!(
        evaluation.backend.source_fingerprint,
        provenance["source_manifest_sha256_lf"]
    );
    assert_eq!(evaluation.attachments.len(), 1);
    assert_eq!(
        evaluation.attachments[0].media_type,
        "application/vnd.poe-optimizer.pob-snapshot+json;version=2"
    );
    let raw: EvaluationSnapshot = serde_json::from_str(&evaluation.attachments[0].content).unwrap();
    assert_eq!(
        raw.runtime.upstream_revision,
        evaluation.backend.rules_revision
    );
    assert_eq!(
        raw.runtime.source_hash,
        evaluation.backend.source_fingerprint
    );
    assert_eq!(
        raw.runtime.adapter_hash,
        evaluation.backend.adapter_fingerprint
    );
    assert_eq!(raw.runtime.mlua_version, "0.12.1");
    assert_eq!(raw.runtime.luajit_source, "210.7.3+1ee778a");
    assert_eq!(raw.runtime.utf8_version, "0.1.6");
    assert!(raw.runtime.lua_version.starts_with("LuaJIT 2.1."));
    assert_eq!(
        raw.player.skill_id.as_deref(),
        expected["skill"]["id"].as_str()
    );
    assert_eq!(
        raw.player.skill_name.as_deref(),
        expected["skill"]["name"].as_str()
    );
    assert!(raw.player.has_hit_damage);
    assert!(raw.minion.is_none());
    let metrics = expected["metrics"].as_object().unwrap();
    assert_eq!(metrics.len(), 19, "Review any change to reference coverage");
    for (name, expected) in metrics {
        let actual = raw
            .player
            .metrics
            .get(name)
            .unwrap_or_else(|| panic!("Missing {name}"));
        assert_close(
            &format!("{scenario}.{name}"),
            *actual,
            expected.as_f64().unwrap(),
            absolute,
            relative,
        );
    }

    for (id, raw_name, unit) in [
        ("life", "Life", MetricUnit::PoolPoints),
        ("mana", "Mana", MetricUnit::PoolPoints),
        ("energy_shield", "EnergyShield", MetricUnit::PoolPoints),
        (
            "fire_resistance_capped_pct",
            "FireResist",
            MetricUnit::Percent,
        ),
        (
            "cold_resistance_capped_pct",
            "ColdResist",
            MetricUnit::Percent,
        ),
        (
            "lightning_resistance_capped_pct",
            "LightningResist",
            MetricUnit::Percent,
        ),
        (
            "chaos_resistance_capped_pct",
            "ChaosResist",
            MetricUnit::Percent,
        ),
        ("pob_total_ehp", "TotalEHP", MetricUnit::Damage),
        (
            "physical_max_hit",
            "PhysicalMaximumHitTaken",
            MetricUnit::Damage,
        ),
        ("selected_hit_dps", "TotalDPS", MetricUnit::DamagePerSecond),
        ("selected_average_hit", "AverageHit", MetricUnit::Damage),
    ] {
        let measurement = measurement(&evaluation, ActorScope::Player, id);
        assert_eq!(measurement.unit, unit, "{id} unit");
        let value = measurement
            .value
            .finite()
            .unwrap_or_else(|| panic!("{id} must be finite"));
        assert_close(
            id,
            value,
            metrics[raw_name].as_f64().unwrap(),
            absolute,
            relative,
        );
    }
    for (id, unit) in [
        ("selected_hit_dps", MetricUnit::DamagePerSecond),
        ("selected_average_hit", MetricUnit::Damage),
    ] {
        let measurement = measurement(&evaluation, ActorScope::SelectedMinion, id);
        assert_eq!(measurement.unit, unit);
        assert!(matches!(
            &measurement.value,
            MeasurementValue::Unavailable { reason } if !reason.is_empty()
        ));
    }

    assert_eq!(evaluation.build.level, expected["build"]["level"]);
    assert_eq!(evaluation.build.class_name, expected["build"]["class_name"]);
    assert_eq!(
        evaluation.build.ascendancy_name,
        expected["build"]["ascendancy_name"]
    );
    assert_eq!(
        evaluation.build.tree_version,
        expected["build"]["tree_version"]
    );
    assert_eq!(evaluation.build.main_socket_group, 1);
    assert_eq!(evaluation.build.skill_groups, 1);
    assert_eq!(evaluation.context.requested, options);
    assert_eq!(evaluation.context.calculation_mode, "MAIN");
    assert_eq!(evaluation.context.enemy_level, config["enemyLevel"]);

    for (name, value) in config.as_object().unwrap() {
        let expected_input: Scalar = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(
            evaluation.context.config_inputs.get(name),
            Some(&expected_input),
            "{scenario} effective {name}"
        );
    }
    let coverage = &evaluation.coverage;
    assert_eq!(coverage.schema_version, 1);
    assert_eq!(coverage.unresolved_entry_count, 0);
    assert!(coverage.selected_minion.is_none());
    let selected = coverage.selected_player.as_ref().unwrap();
    assert_eq!(selected.actor, SkillActor::Player);
    assert_eq!(selected.skill_id.as_deref(), Some("SparkPlayer"));
    assert_eq!(selected.skill_name.as_deref(), Some("Spark"));
    assert_eq!(selected.group_index, Some(1));
    assert_eq!(selected.gem_index, Some(1));
    assert!(!selected.show_average);
    assert!(!selected.synthesized_default_attack);
    assert_eq!(coverage.groups.len(), 1);
    let group = &coverage.groups[0];
    assert!(group.enabled);
    assert_eq!(group.provenance.kind, SkillOrigin::Manual);
    assert_eq!(group.gems.len(), 1);
    let gem = &group.gems[0];
    assert_eq!(gem.resolution, SkillResolution::ResolvedGem);
    assert_eq!(gem.is_support, Some(false));
    assert_eq!(gem.level, Some(1.0));
    assert_eq!(gem.quality, Some(0.0));
    assert_eq!(coverage.full_dps.included_group_count, 1);
    assert!(coverage.full_dps.selected_group_included);
}

#[test]
fn spark_mapping_and_bossing_match_independent_host_references() {
    for scenario in ["spark-mapping", "spark-bossing"] {
        compare_reference(scenario);
    }
    let output = Command::new(env!("CARGO_BIN_EXE_poe-optimizer"))
        .arg("metrics")
        .output()
        .unwrap();
    assert!(output.status.success());
    let catalog: Vec<MetricDefinition> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!catalog.is_empty());
    for definition in catalog {
        let name = definition.id.to_ascii_lowercase().replace('_', "");
        assert!(
            !name.contains("combineddps") && !name.contains("fulldps"),
            "Raw rollups need separate semantic validation before catalog exposure: {}",
            definition.id
        );
    }
}
