//! Shared action-speed and direct timing source/evidence contracts.
use poe_optimizer_core::{candidate::*, evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, CompiledGameData, HostClock, NativeBackend};
use serde_json::{Value, json};
use std::sync::Arc;
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-body-armour.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/builds/spark-body-armour.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 30_000 };
fn request(xml: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: xml.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn make_domain(backend: &NativeBackend, source: &str) -> ControlledBuildDomain {
    ControlledBuildDomain::new(
        Arc::new(
            ControlledBuildCatalog::new(backend.data().clone(), source.into(), vec![]).unwrap(),
        ),
        CandidateConstraints {
            budgets: CandidateBudgets {
                ordinary_passive_points: 20,
                ascendancy_passive_points: 8,
                active_skill_count: 1,
                supports_per_skill: 2,
                ..Default::default()
            },
            ..Default::default()
        },
        AttributeOptionLocks::default(),
    )
    .unwrap()
}
fn config(source: &str, line: &str) -> String {
    assert_eq!(source.matches("</ConfigSet>").count(), 1);
    source.replace("</ConfigSet>",&format!("<CustomModifierBlock title=\"Action Study\" enabled=\"true\">{line}</CustomModifierBlock></ConfigSet>"))
}
fn item_line(source: &str, id: u32, line: &str) -> String {
    let start = source.find(&format!("<Item id=\"{id}\">")).unwrap();
    let end = start + source[start..].find("</Item>").unwrap();
    format!("{}\n{line}{}", &source[..end], &source[end..])
}
fn evidence(result: &EvaluationResult) -> Value {
    serde_json::from_str(
        &result
            .attachments
            .iter()
            .find(|a| a.media_type.contains("native-profile+"))
            .unwrap()
            .content,
    )
    .unwrap()
}
fn compare(
    backend: &NativeBackend,
    domain: &ControlledBuildDomain,
    handle: &AdmittedBuildSelection,
) -> EvaluationResult {
    let prepared = backend
        .prepare_controlled_build(domain.catalog(), &[])
        .unwrap();
    let xml = domain.materialize(handle).unwrap();
    let full = backend.calculate(&request(&xml.content), BUDGET).unwrap();
    let snapshot = prepared.measure(handle).unwrap();
    assert_eq!(snapshot.values().len(), 14);
    assert_eq!(
        serde_json::to_value(prepared.snapshot_measurements(&snapshot)).unwrap(),
        serde_json::to_value(&full.measurements).unwrap()
    );
    domain
        .catalog()
        .validate_native_realization(handle, &full, &backend.identity())
        .unwrap();
    assert_eq!(full.exports[0].content, xml.content);
    full
}
fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 1e-10 * expected.abs().max(1.0),
        "{actual} != {expected}"
    );
}
#[test]
fn action_speed_drives_both_skills_and_movement_in_lf_and_crlf_sources() {
    let backend = NativeBackend::new();
    for template in [MACE, SPARK] {
        let lf = template.replace("\r\n", "\n");
        for fixture in [lf.clone(), lf.replace('\n', "\r\n")] {
            for (lines, expected) in [
                ("20% increased Action Speed\n40% reduced Action Speed", 0.8),
                (
                    "20% increased Action Speed\n40% reduced Action Speed\nYour speed is unaffected by slows",
                    1.2,
                ),
                ("100% reduced Action Speed", 0.0),
                (
                    "200% reduced Action Speed\nAction Speed cannot be modified to below base value",
                    1.0,
                ),
                (
                    "20% increased Action Speed\nYour Action Speed is at least 130% of base value",
                    1.3,
                ),
                (
                    "20% increased Action Speed\nYour Action Speed is at least 0% of base value",
                    1.2,
                ),
                ("10000% increased Action Speed", 101.0),
            ] {
                let source = config(&fixture, lines);
                let domain = make_domain(&backend, &source);
                assert!(domain.catalog().uses_action_speed_scope());
                let handle = domain
                    .admit(
                        domain.catalog().source_selection(),
                        &mut ActorScratch::default(),
                    )
                    .unwrap();
                let full = compare(&backend, &domain, &handle);
                let info = evidence(&full);
                close(
                    info["action_speed"]["action_speed_mod"].as_f64().unwrap(),
                    expected,
                );
                assert_eq!(info["action_speed"]["schema_version"], 1);
                assert_eq!(info["action_timing"]["schema_version"], 1);
                assert_eq!(
                    info["movement"]["action_speed_mod"],
                    info["action_speed"]["action_speed_mod"]
                );
                let metric = full
                    .measurements
                    .iter()
                    .find(|m| m.query.id == "action_speed_pct")
                    .unwrap();
                assert_eq!(metric.unit, MetricUnit::Percent);
                close(metric.value.finite().unwrap(), 100.0 * expected);
                let timing = &info["action_timing"];
                let speed = timing["speed"].as_f64().unwrap();
                let cast = timing["cast_rate"].as_f64().unwrap();
                if expected == 0.0 {
                    assert_eq!(speed, 0.0);
                    assert_eq!(timing["time"], 0.0);
                } else {
                    close(timing["time"].as_f64().unwrap(), 1.0 / speed);
                }
                if expected == 101.0 {
                    assert!(cast > speed, "server cap must leave CastRate pre-cap");
                } else {
                    close(cast, speed);
                }
                assert_eq!(timing["non_finite_values"], json!({}));
                if lines.contains("at least 0%") {
                    // ModDB.Tabulate excludes MAX zero before the query; source
                    // records still retain the authored value and provenance.
                    assert_eq!(info["action_speed"]["minimum_action_speed"], Value::Null);
                    assert!(
                        info["actor_modifiers"]["records"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|r| r["stat"] == "minimum_action_speed"
                                && r["effect"]["value"] == 0.0)
                    );
                }
                if lines.contains("below base value") {
                    let records = info["actor_modifiers"]["records"].as_array().unwrap();
                    let minimum = records
                        .iter()
                        .find(|r| r["stat"] == "minimum_action_speed")
                        .unwrap();
                    assert_eq!(
                        minimum["tags"],
                        json!([{"type":"global_effect","effect_type":"global","unscalable":true}])
                    );
                }
            }
        }
    }
}
#[test]
fn equipment_removal_replay_and_foreign_handles_keep_action_sources_bound() {
    let backend = NativeBackend::new();
    for template in [MACE, SPARK] {
        let source = item_line(
            &config(template, "10% increased Action Speed"),
            41,
            "20% increased Action Speed\nAction speed cannot be modified to below base value",
        );
        let domain = make_domain(&backend, &source);
        let prepared = backend
            .prepare_controlled_build(domain.catalog(), &[])
            .unwrap();
        let original = domain.catalog().source_selection();
        let mut handles = Vec::new();
        for helmet in [true, false, true] {
            let mut selected = original.clone();
            if !helmet {
                selected.candidate.equipment.remove("Helmet");
            }
            let handle = domain
                .admit(selected, &mut ActorScratch::default())
                .unwrap();
            let full = compare(&backend, &domain, &handle);
            close(
                evidence(&full)["action_speed"]["action_speed_mod"]
                    .as_f64()
                    .unwrap(),
                if helmet { 1.3 } else { 1.1 },
            );
            for path in [
                "/action_speed/action_speed_mod",
                "/action_speed/minimum_action_speed",
                "/action_speed/unaffected_by_slows",
                "/action_timing/speed_multiplier",
                "/action_timing/base_time",
                "/action_timing/cast_rate",
                "/action_timing/speed",
                "/action_timing/time",
                "/action_timing/action_speed_mod",
                "/action_timing/non_finite_values",
            ] {
                let mut altered: EvaluationResult =
                    serde_json::from_value(serde_json::to_value(&full).unwrap()).unwrap();
                let attachment = altered
                    .attachments
                    .iter_mut()
                    .find(|a| a.media_type.contains("native-profile+"))
                    .unwrap();
                let mut changed: Value = serde_json::from_str(&attachment.content).unwrap();
                *changed.pointer_mut(path).unwrap() = json!("tampered");
                attachment.content = changed.to_string();
                assert!(
                    domain
                        .catalog()
                        .validate_native_realization(&handle, &altered, &backend.identity())
                        .is_err(),
                    "{path}"
                );
            }
            handles.push(handle);
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let prepared = &prepared;
                let handles = &handles;
                scope.spawn(move || {
                    for handle in handles.iter().cycle().take(128) {
                        assert_eq!(
                            prepared.measure(handle).unwrap().values()[13].finite(),
                            Some(100.0 * handle.actor().action_speed().action_speed_mod)
                        );
                    }
                });
            }
        });
        // A separately compiled catalog cannot fabricate an admitted numerical handle.
        let foreign = make_domain(&backend, &source);
        let foreign = foreign
            .admit(
                foreign.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        assert!(prepared.calculate(&foreign).is_err());
    }
}
#[test]
fn unsupported_action_producers_never_disappear_during_source_admission() {
    let backend = NativeBackend::new();
    for line in [
        "20% more Action Speed",
        "+20 to Action Speed",
        "20% increased Action Speed while using a Skill",
        "Nearby allies' Action Speed cannot be modified to below base value",
        "Nearby Enemy Monsters' Action Speed is at most 80% of base value",
        "Unaffected by Temporal Chains",
        "20% increased Temporal Chains Action Speed",
    ] {
        for source in [config(MACE, line), item_line(MACE, 44, line)] {
            assert_eq!(
                backend.prepare(&request(&source)).err().unwrap().kind,
                EvaluationErrorKind::UnsupportedCapability,
                "{line}"
            );
        }
    }
    let mut req = request(MACE);
    req.metrics = vec![MetricQuery {
        actor: ActorScope::SelectedMinion,
        id: "action_speed_pct".into(),
    }];
    assert_eq!(
        backend.prepare(&req).err().unwrap().kind,
        EvaluationErrorKind::UnsupportedCapability
    );
}

#[test]
fn injected_grammar_action_formula_and_server_cap_change_selected_outputs() {
    let original = NativeBackend::new();
    let mut identities = Vec::new();
    for (divisor, tick) in [(100.0, 0.25), (50.0, 0.5)] {
        let mut package = original.data().snapshot().package().clone();
        package.action_speed.percent_divisor = divisor;
        package.direct_action_timing.server_tick_rate = tick;
        let rule = package
            .actor
            .modifier_rules
            .iter_mut()
            .find(|r| {
                r.template
                    .eq_ignore_ascii_case("{0}% increased Action Speed")
            })
            .unwrap();
        rule.template = "{0}% increased Authored Tempo".into();
        package.refresh_section_digests().unwrap();
        let snapshot = GameDataLoader::from_bytes(
            &package.canonical_bytes().unwrap(),
            &TrustPolicy::AllowCustom,
            &LoadLimits::default(),
        )
        .unwrap();
        let backend = NativeBackend::with_data(
            Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
            HostClock,
        )
        .unwrap();
        for template in [MACE, SPARK] {
            let source = config(template, "20% increased Authored Tempo");
            assert!(original.prepare(&request(&source)).is_err());
            let domain = make_domain(&backend, &source);
            let handle = domain
                .admit(
                    domain.catalog().source_selection(),
                    &mut ActorScratch::default(),
                )
                .unwrap();
            let full = compare(&backend, &domain, &handle);
            let info = evidence(&full);
            close(
                info["action_speed"]["action_speed_mod"].as_f64().unwrap(),
                1.0 + 20.0 / divisor,
            );
            close(info["action_timing"]["speed"].as_f64().unwrap(), tick);
            assert!(info["action_timing"]["cast_rate"].as_f64().unwrap() > tick);
            close(info["action_timing"]["time"].as_f64().unwrap(), 1.0 / tick);
            assert_eq!(full.backend.data, Some(backend.data().identity().clone()));
            assert!(
                domain
                    .catalog()
                    .validate_native_realization(&handle, &full, &original.identity())
                    .is_err()
            );
        }
        identities.push(backend.identity());
    }
    assert_ne!(identities[0], identities[1]);
}
#[test]
fn finite_action_inputs_preserve_nonfinite_timing_and_metric_availability() {
    let original = NativeBackend::new();
    let mut package = original.data().snapshot().package().clone();
    // Exactly round-trippable 2^-1019 keeps the shared actor finite.
    package.action_speed.percent_divisor = f64::from_bits(4_u64 << 52);
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let backend = NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
        HostClock,
    )
    .unwrap();
    for template in [MACE, SPARK] {
        // Source override zero isolates movement while direct CastRate overflows
        // after finite action speed. Source server capping still produces finite DPS.
        let source = config(
            template,
            "25% increased Action Speed\nYour movement speed is 0% of its base value",
        );
        let domain = make_domain(&backend, &source);
        let handle = domain
            .admit(
                domain.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        let full = compare(&backend, &domain, &handle);
        let info = evidence(&full);
        assert!(
            info["action_speed"]["action_speed_mod"]
                .as_f64()
                .unwrap()
                .is_finite()
        );
        assert_eq!(info["action_timing"]["cast_rate"], Value::Null);
        assert_eq!(
            info["action_timing"]["non_finite_values"],
            json!({"cast_rate":"positive_infinity"})
        );
        assert_eq!(
            full.measurements
                .iter()
                .find(|m| m.query.id == "action_speed_pct")
                .unwrap()
                .value,
            MeasurementValue::NonFinite {
                kind: NonFiniteKind::PositiveInfinity
            }
        );
        assert!(
            full.measurements
                .iter()
                .find(|m| m.query.id == "selected_hit_dps")
                .unwrap()
                .value
                .finite()
                .unwrap()
                .is_finite()
        );
        let mut changed: EvaluationResult =
            serde_json::from_value(serde_json::to_value(&full).unwrap()).unwrap();
        let attachment = changed
            .attachments
            .iter_mut()
            .find(|a| a.media_type.contains("native-profile+"))
            .unwrap();
        let mut value: Value = serde_json::from_str(&attachment.content).unwrap();
        value["action_timing"]["non_finite_values"] = json!({"cast_rate":"not_a_number"});
        attachment.content = value.to_string();
        assert!(
            domain
                .catalog()
                .validate_native_realization(&handle, &changed, &backend.identity())
                .is_err()
        );
    }
}
#[test]
fn action_speed_attribute_conditions_follow_final_shared_actor_requirements() {
    let backend = NativeBackend::new();
    for template in [MACE, SPARK] {
        for (attribute, expected) in [("Strength", 1.2), ("Intelligence", 1.0)] {
            let source = config(
                template,
                &format!(
                    "+100 to {attribute}\n20% increased Action Speed if Strength is higher than Intelligence"
                ),
            );
            let domain = make_domain(&backend, &source);
            let handle = domain
                .admit(
                    domain.catalog().source_selection(),
                    &mut ActorScratch::default(),
                )
                .unwrap();
            let full = compare(&backend, &domain, &handle);
            close(
                evidence(&full)["action_speed"]["action_speed_mod"]
                    .as_f64()
                    .unwrap(),
                expected,
            );
            assert_eq!(
                f64::from(handle.requirements().available.strength),
                handle.actor().values().attributes.strength
            );
        }
    }
}
