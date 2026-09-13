//! Shared native preparation contract: source ownership and unresolved producer
//! evidence are distinct from the closed legacy adapters' numerical coverage.
use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{SelectionRequest, ViewRequest, WeaponStateRequest},
    evaluation::*,
    metrics::MeasurementValue,
    options::EvaluationOptions,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{NativeBackend, PreparationOutcome, PreparedEvaluation};
use std::{collections::BTreeMap, path::PathBuf};

const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
const MACE: &str = include_str!("../../../tests/fixtures/calibration/mace-wooden.xml");
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
fn imported(xml: &str, lineage: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn values(result: &EvaluationResult) -> BTreeMap<String, MeasurementValue> {
    result
        .measurements
        .iter()
        .map(|value| (value.query.id.clone(), value.value.clone()))
        .collect()
}
fn close(actual: f64, expected: f64, name: &str) {
    assert!(
        (actual - expected).abs() <= 1e-9 * expected.abs().max(1.),
        "{name}: {actual} vs {expected}"
    );
}
fn calibrations() -> [(&'static str, &'static str); 6] {
    [
        (
            SPARK,
            include_str!("../../../tests/fixtures/calibration/spark-mapping.reference.json"),
        ),
        (
            include_str!("../../../tests/fixtures/calibration/spark-bossing.xml"),
            include_str!("../../../tests/fixtures/calibration/spark-bossing.reference.json"),
        ),
        (
            MACE,
            include_str!("../../../tests/fixtures/calibration/mace-wooden.reference.json"),
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-smithing.xml"),
            include_str!("../../../tests/fixtures/calibration/mace-smithing.reference.json"),
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-wooden-brutality.xml"),
            include_str!(
                "../../../tests/fixtures/calibration/mace-wooden-brutality.reference.json"
            ),
        ),
        (
            include_str!("../../../tests/fixtures/calibration/mace-smithing-brutality.xml"),
            include_str!(
                "../../../tests/fixtures/calibration/mace-smithing-brutality.reference.json"
            ),
        ),
    ]
}

fn prepare_owner(
    backend: &NativeBackend,
    build: &ImportedBuildInstance,
    selection: &ViewRequest,
) -> PreparationOutcome {
    let view = resolve_view(
        build,
        backend.data().snapshot(),
        selection,
        ResolveLimits::default(),
    )
    .unwrap();
    backend
        .prepare_view(build, &view, &EvaluationOptions::default(), &[])
        .unwrap()
}
fn ready(backend: &NativeBackend, build: &ImportedBuildInstance) -> Box<PreparedEvaluation> {
    match prepare_owner(backend, build, &ViewRequest::default()) {
        PreparationOutcome::Ready(value) => value,
        PreparationOutcome::Incomplete(report) => panic!(
            "unexpected incomplete legacy calibration: {:?}",
            report.legacy_adapter_error
        ),
    }
}
#[test]
fn six_calibrations_retain_source_ownership_and_match_unchanged_reference_numbers() {
    let backend = NativeBackend::new();
    for (index, (xml, golden)) in calibrations().into_iter().enumerate() {
        let build = imported(xml, index as u8 + 1);
        let prepared = ready(&backend, &build);
        let skills = prepared.authored_skills().report();
        assert_eq!(
            skills.status,
            poe_optimizer_native::skills::SkillPreparationStatus::Complete
        );
        assert_eq!(skills.source_sha256, build.source_sha256());
        assert_eq!(skills.data, *backend.data().identity());
        let view = resolve_view(
            &build,
            backend.data().snapshot(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        prepared
            .authored_skills()
            .validate_binding(&build, &view, backend.data())
            .unwrap();
        prepared
            .authored_configuration()
            .validate_binding(&build, &view, backend.data())
            .unwrap();
        assert_eq!(
            prepared.authored_configuration().report().source_sha256,
            build.source_sha256()
        );
        assert!(prepared.source().shares_storage_with(&build));
        assert_eq!(prepared.source().source_xml(), xml);
        assert_eq!(prepared.request().build.content, xml);
        assert_eq!(
            prepared.selected_view().source_sha256,
            build.source_sha256()
        );
        assert_eq!(prepared.selected_view().lineage, build.lineage());
        assert_eq!(prepared.selected_view().data, *backend.data().identity());
        let result = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
        let expected: serde_json::Value = serde_json::from_str(golden).unwrap();
        for (name, raw) in [
            ("life", "Life"),
            ("mana", "Mana"),
            ("energy_shield", "EnergyShield"),
            ("fire_resistance_capped_pct", "FireResist"),
            ("cold_resistance_capped_pct", "ColdResist"),
            ("lightning_resistance_capped_pct", "LightningResist"),
            ("chaos_resistance_capped_pct", "ChaosResist"),
            ("selected_hit_dps", "TotalDPS"),
        ] {
            close(
                values(&result)[name].finite().unwrap(),
                expected["metrics"][raw].as_f64().unwrap(),
                name,
            );
        }
        assert_eq!(result.exports[0].content, xml);
        assert!(result.diagnostic_only);
        result.validate_recorded().unwrap();
        let via_request = backend
            .prepare_with_lineage(
                &request(xml),
                BuildLineage::from_bytes([40 + index as u8; 16]),
            )
            .unwrap();
        assert_eq!(
            via_request.source().lineage(),
            BuildLineage::from_bytes([40 + index as u8; 16])
        );
        assert_eq!(
            values(&backend.evaluate_prepared(&via_request, BUDGET).unwrap()),
            values(&result)
        );
    }
}
#[test]
fn unchanged_public_prepare_and_calculate_also_use_owned_selected_views() {
    let backend = NativeBackend::new();
    for xml in [SPARK, MACE] {
        let prepared = backend.prepare(&request(xml)).unwrap();
        assert_eq!(prepared.source().source_xml(), xml);
        assert_eq!(
            prepared.selected_view().source_sha256,
            prepared.source().source_sha256()
        );
        assert_eq!(
            prepared.selected_view().lineage,
            prepared.source().lineage()
        );
        assert!(prepared.selected_view().skills.selected.is_some());
        let first = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
        let public = backend.calculate(&request(xml), BUDGET).unwrap();
        assert_eq!(values(&first), values(&public));
    }
}
#[test]
fn caller_changes_are_calculated_and_cached_outputs_are_never_admission_data() {
    let backend = NativeBackend::new();
    for xml in [SPARK, MACE] {
        let original = ready(&backend, &imported(xml, 70));
        let original_result = backend.evaluate_prepared(&original, BUDGET).unwrap();
        let altered = xml
            .replace("level=\"60\"", "level=\"61\"")
            .replace("<Notes>", "<Notes>Caller supplied variant; ");
        assert_ne!(altered, xml);
        let owner = imported(&altered, 71);
        let prepared = ready(&backend, &owner);
        let changed = backend.evaluate_prepared(&prepared, BUDGET).unwrap();
        assert_ne!(values(&changed)["life"], values(&original_result)["life"]);
        assert_eq!(prepared.source().source_xml(), altered);
        assert_eq!(changed.exports[0].content, altered);
        let poison=altered.replace("viewMode=\"CALCS\"/>","viewMode=\"CALCS\"><PlayerStat stat=\"Life\" value=\"999999999\"/><MinionStat stat=\"TotalDPS\" value=\"999999999\"/></Build>");
        assert_ne!(poison, altered);
        let poisoned = ready(&backend, &imported(&poison, 72));
        let recalculated = backend.evaluate_prepared(&poisoned, BUDGET).unwrap();
        assert_eq!(values(&recalculated), values(&changed));
        assert_eq!(poisoned.source().source_xml(), poison);
        assert!(!recalculated.exports[0].content.contains("999999999"));
    }
}
#[test]
fn identical_bytes_from_another_owner_and_foreign_definition_view_are_rejected() {
    let backend = NativeBackend::new();
    let owner = imported(SPARK, 80);
    let view = resolve_view(
        &owner,
        backend.data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        backend
            .prepare_view(&owner.clone(), &view, &EvaluationOptions::default(), &[])
            .is_ok()
    );
    let independent = imported(SPARK, 81);
    assert_eq!(independent.source_sha256(), owner.source_sha256());
    assert!(!independent.shares_storage_with(&owner));
    assert!(
        backend
            .prepare_view(&independent, &view, &EvaluationOptions::default(), &[])
            .is_err()
    );
    let same_lineage_independent = imported(SPARK, 80);
    assert!(
        backend
            .prepare_view(
                &same_lineage_independent,
                &view,
                &EvaluationOptions::default(),
                &[]
            )
            .is_err()
    );
    let other_data = backend.data().snapshot().clone();
    let foreign_view = resolve_view(
        &owner,
        &other_data,
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        backend
            .prepare_view(&owner, &foreign_view, &EvaluationOptions::default(), &[])
            .is_err()
    );
}
#[test]
fn all_five_real_sources_reach_shared_preparation_with_explicit_remaining_producers() {
    let backend = NativeBackend::new();
    let corpus = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/builds/breadth-20260908");
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(corpus.join("index.json")).unwrap()).unwrap();
    for (index, row) in manifest["builds"].as_array().unwrap().iter().enumerate() {
        let xml = std::fs::read_to_string(corpus.join(row["xml"].as_str().unwrap())).unwrap();
        let owner = imported(&xml, index as u8 + 90);
        assert_eq!(owner.source_sha256(), row["xml_sha256"].as_str().unwrap());
        let outcome = prepare_owner(&backend, &owner, &ViewRequest::default());
        let PreparationOutcome::Incomplete(report) = outcome else {
            panic!(
                "R1c must not silently use a legacy numeric adapter for unsupported real source {}",
                index + 1
            )
        };
        assert_eq!(report.schema_version, 6);
        let skills = report
            .authored_skills
            .as_ref()
            .expect("executed authored stage");
        assert_eq!(
            skills.status,
            poe_optimizer_native::skills::SkillPreparationStatus::Complete
        );
        assert_eq!(skills.source_sha256, owner.source_sha256());
        assert_eq!(skills.data, *backend.data().identity());
        assert_eq!(skills.groups.len(), [19, 84, 14, 15, 68][index]);
        assert_eq!(
            skills
                .groups
                .iter()
                .map(|group| group.gems.len())
                .sum::<usize>(),
            [62, 174, 62, 62, 181][index]
        );
        let selected: Vec<_> = skills
            .groups
            .iter()
            .filter(|group| skills.selected_groups.contains(&group.instance))
            .collect();
        assert_eq!(
            selected.iter().map(|group| group.gems.len()).sum::<usize>(),
            [62, 46, 62, 62, 28][index]
        );
        for group in selected {
            for gem in &group.gems {
                assert!(gem.processed);
                if matches!(
                    gem.identity_status,
                    poe_optimizer_native::skills::SkillIdentityStatus::ResolvedGem
                        | poe_optimizer_native::skills::SkillIdentityStatus::ResolvedEffect
                ) {
                    let instance = Some(AuthoredInstanceId::SkillEntry(gem.instance));
                    assert!(!report.issues.iter().any(|issue| issue.instance == instance
                        && matches!(
                            issue.stage,
                            "skill_identity" | "skill_primary_gem_owner" | "skill_name_resolution"
                        )));
                    assert!(report.issues.iter().any(|issue| issue.instance == instance
                        && issue.stage == "skill_effect_producers"));
                }
            }
        }
        assert_eq!(report.view.source_sha256, owner.source_sha256());
        assert_eq!(report.view.lineage, owner.lineage());
        assert_eq!(report.view.data, *backend.data().identity());
        assert!(!report.issues.is_empty());
        assert!(!report.view.skill_identities.entries.is_empty());
        assert_eq!(
            report.view.skills.selected.as_ref().unwrap().key.value(),
            row["reference"]["active_skill_set_id"].as_f64().unwrap()
        );
        let stages: Vec<_> = report
            .view
            .frontiers
            .iter()
            .map(|issue| issue.stage)
            .collect();
        for required in [
            "skill_processing",
            "item_preparation",
            "passive_loading",
            "configuration_effects",
            "root_lifecycle",
        ] {
            assert!(
                stages.contains(&required),
                "source{} missing {required}",
                index + 1
            );
        }
        for evidence in &report.view.skill_identities.entries {
            let binding = owner.binding(evidence.instance).unwrap();
            assert_eq!(binding.source(), evidence.source);
            assert!(!owner.source_fragment(evidence.source).unwrap().is_empty());
        }
        for issue in &report.issues {
            assert!(!issue.stage.is_empty());
            assert!(!issue.message.is_empty());
            if let Some(source) = issue.source {
                owner.occurrence(source).unwrap();
            }
            if let Some(instance) = issue.instance {
                owner.binding(instance).unwrap();
            }
        }
        let PreparationOutcome::Incomplete(detailed) = backend
            .prepare_request_with_lineage(&request(&xml), owner.lineage())
            .unwrap()
        else {
            panic!("document convenience route must preserve incomplete outcome")
        };
        assert_eq!(
            serde_json::to_value(&detailed).unwrap(),
            serde_json::to_value(&report).unwrap()
        );
        eprintln!(
            "R1c source={} selected={}/{}/{}/{} selected_identity_entries={} preparation_issues={}",
            index + 1,
            report.view.skills.selected.as_ref().unwrap().key.value(),
            report.view.items.selected.as_ref().unwrap().key.value(),
            report.view.passives.selected.as_ref().unwrap().key.value(),
            report
                .view
                .configuration
                .selected
                .as_ref()
                .unwrap()
                .key
                .value(),
            report.view.skill_identities.entries.len(),
            report.issues.len()
        );
        assert!(
            backend
                .prepare_with_lineage(&request(&xml), owner.lineage())
                .is_err()
        );
        assert_eq!(owner.source_xml(), xml);
    }
}
#[test]
fn an_explicit_unsupported_weapon_state_stays_incomplete_and_does_not_use_saved_numbers() {
    let backend = NativeBackend::new();
    let owner = imported(MACE, 110);
    let selection = ViewRequest {
        weapon_state: WeaponStateRequest::Secondary,
        ..Default::default()
    };
    let outcome = prepare_owner(&backend, &owner, &selection);
    let PreparationOutcome::Incomplete(report) = outcome else {
        panic!("secondary weapon override was ignored")
    };
    assert_eq!(report.view.weapon_state.use_second_weapon_set, Some(true));
    assert!(!report.issues.is_empty());
    assert_eq!(report.view.source_sha256, owner.source_sha256());
}
#[test]
fn unsupported_authored_alternative_keeps_the_requested_instance() {
    let backend = NativeBackend::new();
    let xml=MACE.replace("</SkillSet>","</SkillSet><SkillSet id=\"8\" title=\"Caller alternate\"><Skill enabled=\"true\"><Gem skillId=\"TwisterPlayer\"/></Skill></SkillSet>");
    let owner = imported(&xml, 111);
    let alternate = owner
        .instances()
        .iter()
        .filter_map(|binding| match binding.instance() {
            AuthoredInstanceId::SkillSet(id) => Some(id),
            _ => None,
        })
        .nth(1)
        .unwrap();
    let selection = ViewRequest {
        skills: SelectionRequest::Instance(alternate),
        ..Default::default()
    };
    let outcome = prepare_owner(&backend, &owner, &selection);
    let PreparationOutcome::Incomplete(report) = outcome else {
        panic!("unsupported selected alternative gained legacy admission")
    };
    assert_eq!(
        report
            .view
            .skills
            .selected
            .as_ref()
            .unwrap()
            .origin
            .instance(),
        Some(AuthoredInstanceId::SkillSet(alternate))
    );
    assert_eq!(
        report.view.skills.selected.as_ref().unwrap().key.value(),
        8.
    );
    assert_eq!(report.view.skills.authored.raw.as_deref(), Some("1"));
    assert!(!report.issues.is_empty());
    assert_eq!(owner.source_xml(), xml);
}
#[test]
fn prepared_owner_and_numbers_remain_usable_after_request_lifetimes_and_across_threads() {
    fn send_sync<T: Send + Sync>() {}
    send_sync::<PreparedEvaluation>();
    let backend = NativeBackend::new();
    let prepared = {
        let temporary = String::from(SPARK);
        let owner = imported(&temporary, 120);
        ready(&backend, &owner)
    };
    assert_eq!(prepared.source().source_xml(), SPARK);
    let expected = values(&backend.evaluate_prepared(&prepared, BUDGET).unwrap());
    std::thread::scope(|scope| {
        let handles: Vec<_> = (0..4)
            .map(|_| scope.spawn(|| values(&backend.evaluate_prepared(&prepared, BUDGET).unwrap())))
            .collect();
        for handle in handles {
            assert_eq!(handle.join().unwrap(), expected);
        }
    });
}

#[test]
fn explicit_live_instances_for_the_existing_view_lower_without_ignoring_requests() {
    let backend = NativeBackend::new();
    let owner = imported(MACE, 121);
    let mut selection = ViewRequest {
        weapon_state: WeaponStateRequest::Primary,
        ..Default::default()
    };
    for binding in owner.instances() {
        match binding.instance() {
            AuthoredInstanceId::SkillSet(id) => selection.skills = SelectionRequest::Instance(id),
            AuthoredInstanceId::ItemSet(id) => selection.items = SelectionRequest::Instance(id),
            AuthoredInstanceId::PassiveSpec(id) => {
                selection.passives = SelectionRequest::Instance(id)
            }
            AuthoredInstanceId::ConfigSet(id) => {
                selection.configuration = SelectionRequest::Instance(id)
            }
            _ => {}
        }
    }
    let PreparationOutcome::Ready(prepared) = prepare_owner(&backend, &owner, &selection) else {
        panic!("exact live selected instances should retain legacy admission")
    };
    for domain in [
        &prepared.selected_view().skills,
        &prepared.selected_view().items,
        &prepared.selected_view().passives,
        &prepared.selected_view().configuration,
    ] {
        assert!(domain.override_instance.is_some());
        assert_eq!(
            domain.override_instance,
            domain.selected.as_ref().unwrap().origin.instance()
        );
    }
    let saved = ready(&backend, &owner);
    assert_eq!(
        values(&backend.evaluate_prepared(&prepared, BUDGET).unwrap()),
        values(&backend.evaluate_prepared(&saved, BUDGET).unwrap())
    );
    assert!(prepared.source().shares_storage_with(&owner));
}
