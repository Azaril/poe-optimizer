//! Complete receiver preparation stays source/data/scenario bound through typed dispatch.
use poe_optimizer_core::{candidate::*, evaluation::*, options::EvaluationOptions};
use poe_optimizer_import::{actor_assembly::receiving_defence_evidence, controlled_build::*};
use poe_optimizer_native::{ActorScratch, NativeBackend};
use serde_json::Value;
use std::sync::Arc;
const MACE: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const BUDGET: EvaluationBudget = EvaluationBudget { timeout_ms: 30_000 };
fn request(source: &str) -> EvaluationRequest {
    EvaluationRequest {
        build: BuildDocument {
            format: BuildFormat::PathOfBuilding2Xml,
            content: source.into(),
        },
        options: EvaluationOptions::default(),
        metrics: vec![],
    }
}
fn make_domain(backend: &NativeBackend, source: &str) -> ControlledBuildDomain {
    ControlledBuildDomain::new(Arc::new(ControlledBuildCatalog::new(backend.data().clone(),source.into(),vec![EquipmentAlternative{
        instance_id:"lunar".into(),pob_item_id:73,item_text:"Rarity: RARE\nReceiving Pendant\nLunar Amulet\nItem Level: 82\nQuality: 0\nImplicits: 1\n+25 to maximum Energy Shield\n+9% to all Elemental Resistances\n+30 to Strength".into()
    }]).unwrap()),CandidateConstraints{budgets:CandidateBudgets{ordinary_passive_points:20,ascendancy_passive_points:8,active_skill_count:1,supports_per_skill:2,..Default::default()},..Default::default()},AttributeOptionLocks::default()).unwrap()
}
#[test]
fn receiving_layer_sources_conditions_and_removals_match_document_dispatch_and_reject_tampering() {
    let backend = NativeBackend::new();
    for (source, passives) in [
        (MACE, vec![38646, 1913]),
        (SPARK, vec![44871, 30346, 34006, 15408]),
    ] {
        let source=source.replace("</ConfigSet>","<CustomModifierBlock title=\"Receiving\" enabled=\"true\">+101 to Armour\n17% increased Armour\n+103 to Evasion Rating\n7% increased Evasion Rating\n+31 to maximum Energy Shield\n15% increased maximum Energy Shield\n+100% to Fire Resistance\n11% increased Fire Resistance\n25% increased Armour if Strength is higher than Intelligence</CustomModifierBlock></ConfigSet>");
        let domain = make_domain(&backend, &source);
        let catalog = domain.catalog();
        let prepared = backend.prepare_controlled_build(catalog, &[]).unwrap();
        let mut reference = None;
        for with_items_and_passives in [false, true, false] {
            let mut selected = catalog.source_selection();
            if with_items_and_passives {
                selected
                    .candidate
                    .equipment
                    .insert("Amulet".into(), "lunar".into());
                selected.candidate.passives = passives.iter().copied().collect();
            }
            let handle = domain
                .admit(selected, &mut ActorScratch::default())
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
            catalog
                .validate_native_realization(&handle, &full, &backend.identity())
                .unwrap();
            let attachment = full
                .attachments
                .iter()
                .find(|a| a.media_type.contains("native-profile+"))
                .unwrap();
            assert!(
                attachment
                    .media_type
                    .ends_with(if source.contains("Mace Strike") {
                        "version=6"
                    } else {
                        "version=4"
                    })
            );
            let evidence: Value = serde_json::from_str(&attachment.content).unwrap();
            let receiver = handle.actor().receiving().unwrap();
            assert_eq!(
                evidence["receiving_defence"],
                receiving_defence_evidence(receiver)
            );
            assert_eq!(
                handle.requirements().available.strength as f64,
                handle.actor().values().attributes.strength
            );
            if !with_items_and_passives {
                if let Some(expected) = &reference {
                    assert_eq!(&evidence["receiving_defence"], expected);
                } else {
                    reference = Some(evidence["receiving_defence"].clone());
                }
            } else {
                assert_ne!(&evidence["receiving_defence"], reference.as_ref().unwrap());
            }
            for pointer in [
                "/receiving_defence/armour",
                "/receiving_defence/resistance_totals/fire",
                "/receiving_defence/resistance_cap",
            ] {
                let mut tampered: EvaluationResult =
                    serde_json::from_value(serde_json::to_value(&full).unwrap()).unwrap();
                let attachment = tampered
                    .attachments
                    .iter_mut()
                    .find(|a| a.media_type.contains("native-profile+"))
                    .unwrap();
                let mut value: Value = serde_json::from_str(&attachment.content).unwrap();
                *value.pointer_mut(pointer).unwrap() = serde_json::json!(-12345);
                attachment.content = value.to_string();
                assert!(
                    catalog
                        .validate_native_realization(&handle, &tampered, &backend.identity())
                        .is_err(),
                    "accepted {pointer}"
                );
            }
        }
        let changed_source = source.replace(
            "</ConfigSet>",
            "<Input name=\"resistancePenalty\" number=\"-30\"/></ConfigSet>",
        );
        let changed = make_domain(&backend, &changed_source);
        let foreign = changed
            .admit(
                changed.catalog().source_selection(),
                &mut ActorScratch::default(),
            )
            .unwrap();
        assert_eq!(
            prepared.calculate(&foreign).err().unwrap().kind,
            EvaluationErrorKind::BackendContract
        );
        let changed_prepared = backend
            .prepare_controlled_build(changed.catalog(), &[])
            .unwrap();
        let own = domain
            .admit(catalog.source_selection(), &mut ActorScratch::default())
            .unwrap();
        assert_ne!(
            own.actor().receiving().unwrap().resistance_totals,
            foreign.actor().receiving().unwrap().resistance_totals
        );
        assert!(changed_prepared.calculate(&foreign).is_ok());
    }
}

#[test]
fn legacy_mace_passive_actor_migration_retains_one_receiver_contribution_and_fresh_evidence() {
    use poe_optimizer_data::class_tree::ClassTreeSelection;
    use poe_optimizer_import::controlled_mace::{
        ControlledMaceCatalog, MaceSupportLoadout, NormalMaceAlternative,
    };
    let backend = NativeBackend::new();
    let selections = [None, Some(38646)].map(|node| ClassTreeSelection {
        class_id: 6,
        ascendancy_id: None,
        entrance_node_id: node,
        ascendancy_node_id: None,
    });
    let catalog = ControlledMaceCatalog::with_tree_loadouts(
        Arc::new(backend.data().snapshot().clone()),
        MACE.into(),
        vec![NormalMaceAlternative {
            id: "wood".into(),
            item_text: "Rarity: NORMAL\nWooden Club\nItem Level: 1\nQuality: 20\nImplicits: 0"
                .into(),
        }],
        vec![MaceSupportLoadout::new(vec![]).unwrap()],
        selections.to_vec(),
    )
    .unwrap();
    assert!(!catalog.uses_receiving_defence_scope());
    let baseline = backend.calculate(&request(MACE), BUDGET).unwrap();
    let scenario = catalog
        .bind_native_baseline(&baseline, &backend.identity())
        .unwrap();
    let components = catalog
        .native_components(&scenario, &backend.identity())
        .unwrap();
    let prepared = backend.prepare_controlled_mace(&components, &[]).unwrap();
    let mut armour = Vec::new();
    for alternative in catalog.alternatives() {
        let handle = catalog
            .validated_native_candidate(&alternative.candidate, &components)
            .unwrap();
        let full = backend
            .calculate(
                &request(&catalog.materialize(&alternative.candidate).unwrap().content),
                BUDGET,
            )
            .unwrap();
        let typed = prepared.measure(&handle).unwrap();
        assert_eq!(
            serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
            serde_json::to_value(&full.measurements).unwrap()
        );
        let evidence: Value = serde_json::from_str(
            &full
                .attachments
                .iter()
                .find(|a| a.media_type.contains("native-profile+"))
                .unwrap()
                .content,
        )
        .unwrap();
        armour.push(evidence["receiving_defence"]["armour"].as_f64().unwrap());
    }
    armour.sort_by(f64::total_cmp);
    assert_eq!(armour, vec![0.0, 20.0]);
    let mut tampered: EvaluationResult =
        serde_json::from_value(serde_json::to_value(&baseline).unwrap()).unwrap();
    let attachment = tampered
        .attachments
        .iter_mut()
        .find(|a| a.media_type.contains("native-profile+"))
        .unwrap();
    let mut value: Value = serde_json::from_str(&attachment.content).unwrap();
    value["receiving_defence"]["resistances"]["chaos"] = serde_json::json!(123);
    attachment.content = value.to_string();
    assert!(
        catalog
            .bind_native_baseline(&tampered, &backend.identity())
            .is_err()
    );
}

#[test]
fn injected_receiving_grammar_passives_quests_and_caps_drive_both_native_paths() {
    use poe_optimizer_data::game_data::{
        ActorModifierEffect, GameDataLoader, LoadLimits, TrustPolicy,
    };
    use poe_optimizer_native::{CompiledGameData, HostClock};
    let default = NativeBackend::new();
    let normal=MACE.replace("</ConfigSet>","<CustomModifierBlock enabled=\"true\">+101 to Armour\n17% increased Armour\n+21% to Fire Resistance\n11% increased Fire Resistance</CustomModifierBlock></ConfigSet>");
    let source = normal.replace("+101 to Armour", "+101 to Study Armour");
    let mut package = default.data().snapshot().package().clone();
    package
        .actor
        .modifier_rules
        .iter_mut()
        .find(|rule| rule.id == "armour_base")
        .unwrap()
        .template = "{0} to Study Armour".into();
    package.quests.elemental_resistance += 7.5;
    package.character.base_evasion += 0.5;
    package.defence.player_resistance_cap = 69.9;
    package.defence.resistance_floor = -12.9;
    let passive = package
        .passive_effects
        .iter_mut()
        .find(|view| view.key.physical_node_id == 38646)
        .unwrap();
    let ActorModifierEffect::Numeric { value, .. } = &mut passive.actor_modifiers[0].effect else {
        panic!("numeric armour view")
    };
    *value += 13.5;
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
    assert_ne!(backend.identity(), default.identity());
    assert!(default.prepare(&request(&source)).is_err());
    assert!(backend.prepare(&request(&normal)).is_err());
    let domain = make_domain(&backend, &source);
    let mut selected = domain.catalog().source_selection();
    selected.candidate.passives.insert(38646);
    let handle = domain
        .admit(selected, &mut ActorScratch::default())
        .unwrap();
    let xml = domain.materialize(&handle).unwrap();
    let result = backend.calculate(&request(&xml.content), BUDGET).unwrap();
    let prepared = backend
        .prepare_controlled_build(domain.catalog(), &[])
        .unwrap();
    let typed = prepared.measure(&handle).unwrap();
    assert_eq!(
        serde_json::to_value(prepared.snapshot_measurements(&typed)).unwrap(),
        serde_json::to_value(&result.measurements).unwrap()
    );
    domain
        .catalog()
        .validate_native_realization(&handle, &result, &backend.identity())
        .unwrap();
    let reference = make_domain(&default, &normal);
    let mut selected = reference.catalog().source_selection();
    selected.candidate.passives.insert(38646);
    let original = reference
        .admit(selected, &mut ActorScratch::default())
        .unwrap();
    let receiver = handle.actor().receiving().unwrap();
    let baseline = original.actor().receiving().unwrap();
    assert_ne!(receiver.armour, baseline.armour);
    assert_ne!(receiver.evasion, baseline.evasion);
    assert_ne!(
        receiver.resistance_totals.fire,
        baseline.resistance_totals.fire
    );
    assert_eq!(receiver.resistance_cap, 69.0);
    assert_eq!(receiver.resistance_floor, -12.0);
    assert_eq!(
        serde_json::to_value(handle.requirements()).unwrap(),
        serde_json::to_value(original.requirements()).unwrap()
    );
    assert_eq!(handle.actor().values(), original.actor().values());
}
