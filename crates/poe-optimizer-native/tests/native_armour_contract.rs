//! Armour source projections, per-slot preparation and fresh receiver evidence.
use poe_optimizer_core::{candidate::*, evaluation::*, metrics::*, options::EvaluationOptions};
use poe_optimizer_data::game_data::{GameDataLoader, LoadLimits, TrustPolicy};
use poe_optimizer_import::controlled_build::*;
use poe_optimizer_native::{ActorScratch, CompiledGameData, HostClock, NativeBackend};
use serde_json::{Value, json};
use std::sync::Arc;
const MACE: &str = include_str!("../../../tests/fixtures/builds/mace-local-armour.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/builds/spark-local-armour.xml");
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
fn profile(result: &EvaluationResult) -> Value {
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
#[test]
fn armour_remove_replace_replay_and_parallel_dispatch_match_fresh_document_evidence() {
    let backend = NativeBackend::new();
    for (source, media) in [(MACE, "version=7"), (SPARK, "version=5")] {
        let source = source.replace("\r\n", "\n").replace('\n', "\r\n");
        let domain = make_domain(&backend, &source);
        let catalog = domain.catalog();
        assert!(catalog.uses_local_armour_scope());
        assert_eq!(catalog.footprint().local_armour_components, 3);
        assert!(catalog.footprint().local_armour_heap_bytes > 0);
        let prepared = backend.prepare_controlled_build(catalog, &[]).unwrap();
        let mut expected = None;
        let mut handles = Vec::new();
        for selected_slots in [
            &[][..],
            &["Helmet"][..],
            &["Helmet", "Gloves", "Boots"][..],
            &["Gloves", "Boots"][..],
            &[][..],
        ] {
            let mut selection = catalog.source_selection();
            for slot in ["Helmet", "Gloves", "Boots"] {
                if !selected_slots.contains(&slot) {
                    selection.candidate.equipment.remove(slot);
                }
            }
            let handle = domain
                .admit(selection, &mut ActorScratch::default())
                .unwrap();
            let document = domain.materialize(&handle).unwrap();
            let result = backend
                .calculate(&request(&document.content), BUDGET)
                .unwrap();
            let snapshot = prepared.measure(&handle).unwrap();
            assert_eq!(snapshot.values().len(), 12);
            assert_eq!(
                serde_json::to_value(prepared.snapshot_measurements(&snapshot)).unwrap(),
                serde_json::to_value(&result.measurements).unwrap()
            );
            catalog
                .validate_native_realization(&handle, &result, &backend.identity())
                .unwrap();
            assert_eq!(result.exports[0].content, document.content);
            assert!(
                result
                    .attachments
                    .iter()
                    .any(|a| a.media_type.ends_with(media))
            );
            let evidence = profile(&result);
            assert_eq!(
                evidence["local_armour"]["items"].as_object().unwrap().len(),
                selected_slots.len()
            );
            for id in ["armour", "evasion"] {
                let value = result
                    .measurements
                    .iter()
                    .find(|m| m.query.id == id)
                    .unwrap();
                assert_eq!(value.unit, MetricUnit::RatingPoints);
                assert_eq!(
                    value.value.finite(),
                    evidence["receiving_defence"][id].as_f64()
                );
            }
            assert_eq!(
                handle.requirements().available.strength as f64,
                handle.actor().values().attributes.strength
            );
            if selected_slots.is_empty() {
                if let Some(expected) = &expected {
                    assert_eq!(expected, &evidence["receiving_defence"]);
                } else {
                    expected = Some(evidence["receiving_defence"].clone());
                }
            }
            if selected_slots.len() == 3 {
                assert_ne!(expected.as_ref().unwrap(), &evidence["receiving_defence"]);
                for pointer in [
                    "/local_armour/items/Helmet/base_armour",
                    "/local_armour/items/Helmet/armour",
                    "/local_armour/items/Helmet/source_sha256",
                    "/local_armour/items/Helmet/slot",
                    "/local_armour/items/Gloves/consumed_modifier_count",
                    "/local_armour/items/Boots/global_modifiers",
                    "/receiving_defence/armour",
                ] {
                    let mut altered: EvaluationResult =
                        serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
                    let attachment = altered
                        .attachments
                        .iter_mut()
                        .find(|a| a.media_type.contains("native-profile+"))
                        .unwrap();
                    let mut value: Value = serde_json::from_str(&attachment.content).unwrap();
                    *value.pointer_mut(pointer).unwrap() = json!("changed");
                    attachment.content = value.to_string();
                    assert!(
                        catalog
                            .validate_native_realization(&handle, &altered, &backend.identity())
                            .is_err(),
                        "accepted {pointer}"
                    );
                }
            }
            handles.push(handle);
        }
        std::thread::scope(|scope| {
            for _ in 0..4 {
                let handles = &handles;
                let prepared = &prepared;
                scope.spawn(move || {
                    for handle in handles.iter().cycle().take(128) {
                        let left = prepared.measure(handle).unwrap();
                        let right = prepared.measure(handle).unwrap();
                        assert_eq!(left.values(), right.values());
                    }
                });
            }
        });
        let foreign = make_domain(&backend, &source);
        let foreign = foreign
            .admit(
                foreign.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        assert_eq!(
            prepared.calculate(&foreign).unwrap_err().kind,
            EvaluationErrorKind::BackendContract
        );
    }
}
#[test]
fn armour_bases_grammar_and_requirements_follow_selected_injected_data() {
    let normal = NativeBackend::new();
    let mut package = normal.data().snapshot().package().clone();
    let base = package
        .armour_bases
        .iter_mut()
        .find(|b| b.name == "Brimmed Helm")
        .unwrap();
    base.name = "Study Helm".into();
    base.armour += 0.5;
    base.evasion += 11.5;
    base.requirements.level = 61;
    package
        .actor
        .modifier_rules
        .iter_mut()
        .find(|r| r.id == "armour_and_evasion_base")
        .unwrap()
        .template = "{0} to Study Armour and Evasion".into();
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    let custom = NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
        HostClock,
    )
    .unwrap();
    let source = MACE.replace("Brimmed Helm", "Study Helm").replace(
        "+17 to Armour and Evasion",
        "+17 to Study Armour and Evasion",
    );
    assert!(normal.prepare(&request(&source)).is_err());
    assert!(custom.prepare(&request(MACE)).is_err());
    let rejected = make_domain(&custom, &source);
    let selection = rejected.catalog().source_selection();
    let available = rejected
        .requirements(&selection, &mut ActorScratch::default())
        .unwrap();
    assert!(
        available
            .violations
            .iter()
            .any(|r| r.requirement == "level" && r.required == 61 && r.available == 60)
    );
    assert!(
        rejected
            .admit(selection, &mut ActorScratch::default())
            .is_err()
    );
    let source = source.replace(
        "Study Helm\nItem Level: 60\nQuality: 20",
        "Study Helm\nItem Level: 60\nQuality: 20\nLevelReq: 1",
    );
    let domain = make_domain(&custom, &source);
    let handle = domain
        .admit(
            domain.catalog().source_selection(),
            &mut ActorScratch::default(),
        )
        .unwrap();
    let prepared = custom
        .prepare_controlled_build(domain.catalog(), &[])
        .unwrap();
    let xml = domain.materialize(&handle).unwrap();
    let full = custom.calculate(&request(&xml.content), BUDGET).unwrap();
    let typed = prepared.measure(&handle).unwrap();
    assert_eq!(
        serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
        serde_json::to_value(&full.measurements).unwrap()
    );
    domain
        .catalog()
        .validate_native_realization(&handle, &full, &custom.identity())
        .unwrap();
    let evidence = profile(&full);
    assert_eq!(
        evidence["local_armour"]["items"]["Helmet"]["base_armour"],
        26.5
    );
    assert_eq!(
        evidence["local_armour"]["items"]["Helmet"]["base_evasion"],
        31.5
    );
    assert_eq!(evidence["equipment"]["Helmet"]["requirements"]["level"], 1);
    assert_eq!(evidence["equipment"]["Helmet"]["item_level"], 60);
    assert_eq!(evidence["equipment"]["Helmet"]["base_name"], "Study Helm");
    assert_ne!(custom.identity(), normal.identity());
    assert!(
        domain
            .catalog()
            .validate_native_realization(&handle, &full, &normal.identity())
            .is_err()
    );
}
#[test]
fn native_armour_source_rejects_wrong_slots_sets_and_unmodeled_local_effects() {
    let backend = NativeBackend::new();
    for source in [
        MACE.replace(
            "name=\"Helmet\" itemId=\"41\"",
            "name=\"Gloves\" itemId=\"41\"",
        ),
        MACE.replace(
            "useSecondWeaponSet=\"false\"",
            "useSecondWeaponSet=\"true\"",
        ),
        MACE.replace("Brimmed Helm", "Rusted Cuirass"),
        MACE.replace(
            "35% increased Armour and Evasion",
            "Has +1 to Evasion Rating per Player Level",
        ),
        MACE.replace(
            "35% increased Armour and Evasion",
            "20% increased Movement Speed",
        ),
        MACE.replace("35% increased Armour and Evasion", "+15 to Runic Ward"),
    ] {
        assert_eq!(
            backend.prepare(&request(&source)).err().unwrap().kind,
            EvaluationErrorKind::UnsupportedCapability
        );
    }
    let mut request = request(MACE);
    request.metrics = vec![MetricQuery {
        id: "armour".into(),
        actor: ActorScope::SelectedMinion,
    }];
    assert_eq!(
        backend.prepare(&request).err().unwrap().kind,
        EvaluationErrorKind::UnsupportedCapability
    );
}

#[test]
fn selected_item_formatting_is_bound_to_typed_and_document_evidence_without_formatting_config() {
    let normal = NativeBackend::new();
    let source = MACE
        .replace("+17 to Armour and Evasion", "+17.5 to Armour")
        .replace("+101 to Armour", "+101.5 to Armour");
    let mut results = Vec::new();
    for mode in 0..4 {
        let mut package = normal.data().snapshot().package().clone();
        let key = "# to Armour";
        match mode {
            0 => {}
            1 => {
                package
                    .item_formatting
                    .rules
                    .iter_mut()
                    .find(|rule| rule.template == key)
                    .unwrap()
                    .captures[0]
                    .precision = 10.0
            }
            2 => package
                .item_formatting
                .rules
                .retain(|rule| rule.template != key),
            _ => {
                package
                    .item_formatting
                    .rules
                    .iter_mut()
                    .find(|rule| rule.template == key)
                    .unwrap()
                    .template = "# to armour".into()
            }
        }
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
        let domain = make_domain(&backend, &source);
        let handle = domain
            .admit(
                domain.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        let prepared = backend
            .prepare_controlled_build(domain.catalog(), &[])
            .unwrap();
        let document = domain.materialize(&handle).unwrap();
        let full = backend
            .calculate(&request(&document.content), BUDGET)
            .unwrap();
        let snapshot = prepared.measure(&handle).unwrap();
        assert_eq!(
            serde_json::to_value(prepared.snapshot_measurements(&snapshot)).unwrap(),
            serde_json::to_value(&full.measurements).unwrap()
        );
        domain
            .catalog()
            .validate_native_realization(&handle, &full, &backend.identity())
            .unwrap();
        let evidence = profile(&full);
        assert_eq!(
            evidence["equipment"]["Helmet"]["modifier_lines"][0]["values"][0],
            17.5
        );
        assert_eq!(
            evidence["equipment"]["Helmet"]["modifier_lines"][0]["effective_values"][0],
            if mode == 0 { 18.0 } else { 17.5 }
        );
        let config = evidence["actor_modifiers"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|line| line["rule_id"] == "armour_base")
            .unwrap();
        assert_eq!(config["values"][0], 101.5);
        for pointer in [
            "/equipment/Helmet/modifier_lines/0/values/0",
            "/equipment/Helmet/modifier_lines/0/effective_values/0",
            "/equipment/Helmet/modifier_lines/0/records/0/effect/value",
        ] {
            let mut tampered: EvaluationResult =
                serde_json::from_value(serde_json::to_value(&full).unwrap()).unwrap();
            let attachment = tampered
                .attachments
                .iter_mut()
                .find(|a| a.media_type.contains("native-profile+"))
                .unwrap();
            let mut altered: Value = serde_json::from_str(&attachment.content).unwrap();
            *altered.pointer_mut(pointer).unwrap() = json!(1234.5);
            attachment.content = altered.to_string();
            assert!(
                domain
                    .catalog()
                    .validate_native_realization(&handle, &tampered, &backend.identity())
                    .is_err()
            );
        }
        results.push((
            backend.identity(),
            evidence["local_armour"]["items"]["Helmet"]["armour"].clone(),
            evidence["receiving_defence"]["armour"].clone(),
        ));
    }
    assert_ne!(results[0].1, results[1].1);
    assert_ne!(results[0].2, results[1].2);
    for result in &results[2..] {
        assert_eq!(result.1, results[1].1);
        assert_eq!(result.2, results[1].2);
    }
    for pair in results.windows(2) {
        assert_ne!(pair[0].0, pair[1].0);
    }
}
