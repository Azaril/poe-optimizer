//! Complete item source composition, movement evidence and typed replay contracts.
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
fn item_line(source: &str, id: u32, line: &str) -> String {
    let start = source.find(&format!("<Item id=\"{id}\">")).unwrap();
    let end = start + source[start..].find("</Item>").unwrap();
    format!("{}\n{line}{}", &source[..end], &source[end..])
}
// Fixture edits preserve the chosen source newline bytes, including item payloads.
fn line_ending_variants(source: &str) -> [String; 2] {
    let lf = source.replace("\r\n", "\n");
    let crlf = lf.replace('\n', "\r\n");
    [lf, crlf]
}
fn insert_item_metadata(source: &str, id: u32, after: &str, metadata: &str) -> String {
    let start = source.find(&format!("<Item id=\"{id}\">")).unwrap();
    let end = start + source[start..].find("</Item>").unwrap();
    let mut offset = start;
    let mut matches = Vec::new();
    for line in source[start..end].split_inclusive('\n') {
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == after {
            let ending = if line.ends_with("\r\n") {
                "\r\n"
            } else {
                assert!(
                    line.ends_with('\n'),
                    "metadata anchor requires a following line"
                );
                "\n"
            };
            matches.push((offset, ending));
        }
    }
    assert_eq!(
        matches.len(),
        1,
        "fixture must contain one exact metadata anchor"
    );
    let (offset, ending) = matches[0];
    let inserted = format!("{metadata}{ending}");
    let mut edited = source.to_owned();
    edited.insert_str(offset, &inserted);
    assert_ne!(edited, source);
    assert_eq!(&edited[..offset], &source[..offset]);
    assert_eq!(&edited[offset + inserted.len()..], &source[offset..]);
    assert_eq!(&edited[offset..offset + inserted.len()], inserted);
    edited
}
fn config_line(source: &str, line: &str) -> String {
    source.replacen(
        "</CustomModifierBlock>",
        &format!("\n{line}</CustomModifierBlock>"),
        1,
    )
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
    assert_eq!(snapshot.values().len(), 13);
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
#[test]
fn body_removal_replay_and_parallel_calculation_preserve_one_generated_penalty_and_fresh_evidence()
{
    let backend = NativeBackend::new();
    for source in [MACE, SPARK] {
        let source = source.replace("\r\n", "\n").replace('\n', "\r\n");
        let domain = make_domain(&backend, &source);
        assert!(domain.catalog().uses_movement_scope());
        let prepared = backend
            .prepare_controlled_build(domain.catalog(), &[])
            .unwrap();
        let mut handles = Vec::new();
        let mut replay = None;
        for body in [true, false, true] {
            let mut selection = domain.catalog().source_selection();
            if !body {
                selection.candidate.equipment.remove("Body Armour");
            }
            let handle = domain
                .admit(selection, &mut ActorScratch::default())
                .unwrap();
            let result = compare(&backend, &domain, &handle);
            let info = evidence(&result);
            assert_eq!(result.context.player_conditions.len(), 12);
            assert_eq!(info["movement"]["schema_version"], 1);
            assert_eq!(info["movement"]["action_speed_mod"], 1.0);
            assert_eq!(info["movement"]["ignore_movement_penalties"], false);
            let metric = result
                .measurements
                .iter()
                .find(|m| m.query.id == "movement_speed_pct")
                .unwrap();
            assert_eq!(metric.unit, MetricUnit::Percent);
            assert_eq!(
                metric.value.finite(),
                Some(
                    100.0
                        * info["movement"]["effective_movement_speed_mod"]
                            .as_f64()
                            .unwrap()
                )
            );
            assert_eq!(info["local_armour"]["schema_version"], 2);
            if body {
                let body = &info["local_armour"]["items"]["Body Armour"];
                let generated = body["generated_global_modifiers"].as_array().unwrap();
                assert_eq!(generated.len(), 1);
                assert_eq!(
                    generated[0]["source"],
                    "Item:44:Movement Study Cuirass, Rusted Cuirass"
                );
                assert_eq!(generated[0]["effect"]["value"], -0.05);
                assert_eq!(
                    body["global_modifiers"].as_array().unwrap().len(),
                    body["source_global_modifiers"].as_array().unwrap().len() + 1
                );
                assert!(
                    info["equipment"]["Body Armour"]["actor_modifiers"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .all(|r| r["effect"]["value"] != -0.05)
                );
                if let Some(prior) = &replay {
                    assert_eq!(prior, &info["movement"]);
                } else {
                    replay = Some(info["movement"].clone());
                }
                for pointer in [
                    "/movement/movement_speed_mod",
                    "/movement/effective_movement_speed_mod",
                    "/movement/ignore_movement_penalties",
                    "/movement/has_override",
                    "/local_armour/items/Body Armour/generated_global_modifiers/0/effect/value",
                    "/local_armour/items/Body Armour/generated_global_modifiers/0/source",
                    "/local_armour/items/Body Armour/global_modifiers",
                    "/local_armour/items/Body Armour/source_global_modifiers",
                ] {
                    let mut altered: EvaluationResult =
                        serde_json::from_value(serde_json::to_value(&result).unwrap()).unwrap();
                    let attachment = altered
                        .attachments
                        .iter_mut()
                        .find(|a| a.media_type.contains("native-profile+"))
                        .unwrap();
                    let mut tampered: Value = serde_json::from_str(&attachment.content).unwrap();
                    *tampered.pointer_mut(pointer).unwrap() = json!("changed");
                    attachment.content = tampered.to_string();
                    assert!(
                        domain
                            .catalog()
                            .validate_native_realization(&handle, &altered, &backend.identity())
                            .is_err(),
                        "{pointer}"
                    );
                }
            } else {
                assert!(info["local_armour"]["items"].get("Body Armour").is_none());
                assert_ne!(replay.as_ref().unwrap(), &info["movement"]);
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
                            prepared.measure(handle).unwrap().values(),
                            prepared.measure(handle).unwrap().values()
                        );
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
fn override_priority_follows_config_then_source_item_order_independent_of_xml_order() {
    let backend = NativeBackend::new();
    for (source, slots) in [
        (
            MACE,
            vec![
                ("Weapon 1", 1, 90),
                ("Helmet", 41, 80),
                ("Body Armour", 44, 70),
                ("Gloves", 42, 60),
                ("Boots", 43, 50),
                ("Amulet", 2, 40),
            ],
        ),
        (
            SPARK,
            vec![
                ("Helmet", 41, 80),
                ("Body Armour", 44, 70),
                ("Gloves", 42, 60),
                ("Boots", 43, 50),
                ("Amulet", 9, 40),
            ],
        ),
    ] {
        let mut source = source.to_owned();
        for (_, id, value) in &slots {
            source = item_line(
                &source,
                *id,
                &format!("Your movement speed is {value}% of its base value"),
            );
        }
        if slots[0].0 == "Weapon 1" {
            let first = backend.calculate(&request(&source), BUDGET).unwrap();
            assert_eq!(evidence(&first)["movement"]["movement_speed_mod"], 0.9);
            // Keep the required physical weapon while removing only its override.
            source = source.replace("\nYour movement speed is 90% of its base value", "");
        }
        let domain = make_domain(&backend, &source);
        let mut selection = domain.catalog().source_selection();
        for (slot, _, expected) in slots.iter().filter(|(slot, _, _)| *slot != "Weapon 1") {
            let handle = domain
                .admit(selection.clone(), &mut ActorScratch::default())
                .unwrap();
            let info = evidence(&compare(&backend, &domain, &handle));
            assert_eq!(
                info["movement"]["movement_speed_mod"].as_f64(),
                Some(f64::from(*expected) / 100.0)
            );
            assert_eq!(info["movement"]["has_override"], true);
            selection.candidate.equipment.remove(*slot);
        }
        let config = config_line(&source, "Your movement speed is 0% of its base value");
        let configured = backend.calculate(&request(&config), BUDGET).unwrap();
        assert_eq!(evidence(&configured)["movement"]["movement_speed_mod"], 0.0);
        let floor = config_line(
            &config,
            "Movement speed cannot be modified to below base value",
        );
        let floor = backend.calculate(&request(&floor), BUDGET).unwrap();
        assert_eq!(evidence(&floor)["movement"]["movement_speed_mod"], 1.0);
        assert_eq!(evidence(&floor)["movement"]["cannot_be_below_base"], true);
    }
}
#[test]
fn movement_numeric_conditions_use_final_attributes_and_ignore_flag_stays_private() {
    let backend = NativeBackend::new();
    for source in [MACE, SPARK] {
        let source = item_line(
            source,
            41,
            "20% increased Movement Speed if Strength is higher than Intelligence",
        );
        for (extra, condition) in [("+100 to Strength", true), ("+100 to Intelligence", false)] {
            for ignored in [false, true] {
                let mut source = config_line(&source, extra);
                if ignored {
                    source = config_line(&source, "Ignore all movement penalties from armour");
                }
                let domain = make_domain(&backend, &source);
                let handle = domain
                    .admit(
                        domain.catalog().source_selection(),
                        &mut ActorScratch::default(),
                    )
                    .unwrap();
                let full = compare(&backend, &domain, &handle);
                let movement = evidence(&full)["movement"].clone();
                assert_eq!(movement["ignore_movement_penalties"], ignored);
                let expected = match (condition, ignored) {
                    (true, true) => 1.45,
                    (true, false) => 1.378,
                    (false, true) => 1.25,
                    (false, false) => 1.188,
                };
                assert_eq!(movement["effective_movement_speed_mod"], expected);
                assert!(
                    !full
                        .context
                        .player_conditions
                        .contains_key("IgnoreMovementPenalties")
                );
                assert_eq!(
                    handle.requirements().available.strength as f64,
                    handle.actor().values().attributes.strength
                );
            }
        }
    }
}
#[test]
fn injected_penalty_presence_values_and_requirements_bind_both_native_paths() {
    let normal = NativeBackend::new();
    let mut identities = Vec::new();
    for penalty in [None, Some(0.0), Some(0.12345)] {
        let mut package = normal.data().snapshot().package().clone();
        let base = package
            .armour_bases
            .iter_mut()
            .find(|base| base.name == "Rusted Cuirass")
            .unwrap();
        base.name = "Study Cuirass".into();
        base.movement_penalty = penalty;
        base.requirements.level = 61;
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
        for fixture in line_ending_variants(MACE) {
            assert_eq!(fixture.matches("Rusted Cuirass").count(), 1);
            let source = fixture.replace("Rusted Cuirass", "Study Cuirass");
            assert_ne!(source, fixture);
            let invalid = make_domain(&backend, &source);
            let check = invalid
                .requirements(
                    &invalid.catalog().source_selection(),
                    &mut ActorScratch::default(),
                )
                .unwrap();
            assert!(
                check
                    .violations
                    .iter()
                    .any(|v| v.requirement == "level" && v.required == 61 && v.available == 60)
            );
            let mut bare = invalid.catalog().source_selection();
            bare.candidate.equipment.remove("Body Armour");
            assert!(invalid.admit(bare, &mut ActorScratch::default()).is_ok());
            let source = insert_item_metadata(&source, 44, "Quality: 20", "LevelReq: 1");
            let domain = make_domain(&backend, &source);
            let handle = domain
                .admit(
                    domain.catalog().source_selection(),
                    &mut ActorScratch::default(),
                )
                .unwrap();
            let full = compare(&backend, &domain, &handle);
            let info = evidence(&full);
            let generated =
                info["local_armour"]["items"]["Body Armour"]["generated_global_modifiers"]
                    .as_array()
                    .unwrap();
            assert_eq!(generated.len(), usize::from(penalty.is_some()));
            if let Some(penalty) = penalty {
                assert_eq!(generated[0]["effect"]["value"].as_f64(), Some(-penalty));
            }
            assert_eq!(info["equipment"]["Body Armour"]["requirements"]["level"], 1);
        }
        identities.push(backend.identity());
    }
    assert!(identities.windows(2).all(|pair| pair[0] != pair[1]));
}
#[test]
fn unsupported_action_party_skill_and_item_mechanics_remain_fail_closed() {
    let backend = NativeBackend::new();
    for line in [
        "20% increased Action Speed",
        "Movement Speed is equal to the highest Movement Speed among Linked Players",
        "20% increased Movement Speed while using a Skill",
        "Grants Level 1 Dash",
        "Ignore all movement penalties from armour if Strength is higher than Intelligence",
        "Your movement speed is 50% of its base value if Dexterity is higher than Intelligence",
        "Movement speed cannot be modified to below base value if Strength is higher than Intelligence",
        "+1 to Runic Ward",
        "Has +1 to Evasion Rating per Player Level",
    ] {
        let source = item_line(MACE, 44, line);
        assert_eq!(
            backend.prepare(&request(&source)).err().unwrap().kind,
            EvaluationErrorKind::UnsupportedCapability,
            "{line}"
        );
    }
    for fixture in line_ending_variants(MACE) {
        let anchor = "name=\"Body Armour\" itemId=\"44\"";
        assert_eq!(fixture.matches(anchor).count(), 1);
        let wrong_slot = fixture.replace(anchor, "name=\"Helmet\" itemId=\"44\"");
        assert_ne!(wrong_slot, fixture);
        let sockets = insert_item_metadata(&fixture, 44, "Rusted Cuirass", "Sockets: S");
        for source in [wrong_slot, sockets] {
            assert!(backend.prepare(&request(&source)).is_err());
        }
    }
    let mut req = request(MACE);
    req.metrics = vec![MetricQuery {
        id: "movement_speed_pct".into(),
        actor: ActorScope::SelectedMinion,
    }];
    assert!(backend.prepare(&req).is_err());
}

#[test]
fn injected_movement_override_division_preserves_raw_ratio_and_backend_identity() {
    let normal = NativeBackend::new();
    let source = config_line(SPARK, "Your movement speed is 57% of its base value");
    let mut identities = Vec::new();
    for divisor in [100.0, 7.0] {
        let mut package = normal.data().snapshot().package().clone();
        let rule = package
            .actor
            .modifier_rules
            .iter_mut()
            .find(|rule| rule.id == "movement_speed_override")
            .unwrap();
        rule.modifiers[0].effect = poe_optimizer_data::game_data::ActorRuleEffect::Numeric {
            operation: poe_optimizer_data::game_data::ActorNumericOperation::Override,
            value: poe_optimizer_data::game_data::ActorRuleValue::CaptureDivided {
                index: 0,
                divisor,
            },
        };
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
        let full = compare(&backend, &domain, &handle);
        assert_eq!(
            evidence(&full)["movement"]["movement_speed_mod"].as_f64(),
            Some(57.0 / divisor)
        );
        identities.push(backend.identity());
    }
    assert_ne!(identities[0], identities[1]);
}

#[test]
fn mixed_case_config_and_equipment_preserve_source_formatting_before_shared_grammar() {
    let backend = NativeBackend::new();
    let base = MACE
        .replace("+13 to maximum Energy Shield", "+17.5 to Evasion Rating")
        .replace(
            "20% increased Energy Shield",
            "27% increased Evasion Rating",
        )
        .replace(
            "Adds 5 to 10 Physical Damage",
            "aDdS 5 To 10 pHySiCaL DaMaGe",
        );
    let base = item_line(&base, 1, "yOuR mOvEmEnT sPeEd Is 57% oF iTs BaSe VaLuE");
    let base = config_line(
        &base,
        "MOVEMENT SPEED CANNOT BE MODIFIED TO BELOW BASE VALUE",
    );
    let base = config_line(&base, "iGnOrE aLl MoVeMeNt PeNaLtIeS fRoM aRmOuR");
    let mut ratings = Vec::new();
    for (line, effective) in [
        ("+17.5 to Evasion Rating", 18.0),
        ("+17.5 to evasion rating", 17.5),
    ] {
        let source = base.replace("+17.5 to Evasion Rating", line);
        let domain = make_domain(&backend, &source);
        let handle = domain
            .admit(
                domain.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        let full = compare(&backend, &domain, &handle);
        let info = evidence(&full);
        assert_eq!(
            info["equipment"]["Gloves"]["modifier_lines"][0]["source"],
            line
        );
        assert_eq!(
            info["equipment"]["Gloves"]["modifier_lines"][0]["values"][0],
            17.5
        );
        assert_eq!(
            info["equipment"]["Gloves"]["modifier_lines"][0]["effective_values"][0],
            effective
        );
        assert_eq!(info["movement"]["movement_speed_mod"], 1.0);
        assert_eq!(info["movement"]["has_override"], true);
        assert_eq!(info["movement"]["ignore_movement_penalties"], true);
        assert_eq!(info["movement"]["cannot_be_below_base"], true);
        ratings.push(info["local_armour"]["items"]["Gloves"]["evasion"].clone());
    }
    assert_ne!(ratings[0], ratings[1]);
}
