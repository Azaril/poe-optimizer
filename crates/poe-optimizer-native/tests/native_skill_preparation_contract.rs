//! Authored loading must constrain the old numeric adapters on every public route.
use poe_optimizer_core::{build_identity::BuildLineage, evaluation::*, options::EvaluationOptions};
use poe_optimizer_data::game_data::{
    self, GameDataLoader, GameDataPackage, LoadLimits, TrustPolicy,
};
use poe_optimizer_native::{
    CompiledGameData, HostClock, NativeBackend, PreparationOutcome, skills::SkillPreparationStatus,
};
use std::sync::Arc;

const SPARK: &str = include_str!("../../../tests/fixtures/calibration/spark-mapping.xml");
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
fn custom(edit: impl FnOnce(&mut GameDataPackage)) -> NativeBackend {
    let mut package = game_data::bundled_snapshot().unwrap().package().clone();
    edit(&mut package);
    package.refresh_section_digests().unwrap();
    let snapshot = GameDataLoader::from_bytes(
        &package.canonical_bytes().unwrap(),
        &TrustPolicy::AllowCustom,
        &LoadLimits::default(),
    )
    .unwrap();
    NativeBackend::with_data(
        Arc::new(CompiledGameData::compile(Arc::new(snapshot)).unwrap()),
        HostClock,
    )
    .unwrap()
}
#[test]
fn injected_loader_policy_cannot_silently_calculate_the_old_raw_profile() {
    for mutation in ["level", "role"] {
        let backend = custom(|package| match mutation {
            "level" => {
                let effect = package
                    .skill_preparation
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == "SparkPlayer")
                    .unwrap();
                effect.levels.retain(|row| row.key == 2.0);
                effect.levels_length = 0;
                effect.next_level_key = Some(2.0);
            }
            "role" => {
                package
                    .skill_preparation
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == "SparkPlayer")
                    .unwrap()
                    .support = Some(true);
                package
                    .skill_identities
                    .skills
                    .iter_mut()
                    .find(|effect| effect.id == "SparkPlayer")
                    .unwrap()
                    .support = Some(true);
                for declaration in &mut package.skill_identities.skill_declarations {
                    if declaration.id == "SparkPlayer" {
                        declaration.identity.support = Some(true);
                    }
                }
            }
            _ => unreachable!(),
        });
        let request = request(SPARK);
        let outcome = backend
            .prepare_request_with_lineage(&request, BuildLineage::from_bytes([82; 16]))
            .unwrap();
        let PreparationOutcome::Incomplete(report) = outcome else {
            panic!("{mutation}: stale closed profile admitted")
        };
        let stage = report.authored_skills.as_ref().unwrap();
        assert_eq!(stage.status, SkillPreparationStatus::Complete);
        assert_eq!(stage.source_sha256, report.view.source_sha256);
        assert_eq!(stage.data, *backend.data().identity());
        let gem = &stage.groups[0].gems[0];
        let expected_error = match mutation {
            "level" => {
                assert_eq!(gem.number("level"), Some(2.0));
                "Processed skill level differs"
            }
            "role" => "Processed support/provider role differs",
            _ => unreachable!(),
        };
        assert!(
            report
                .legacy_adapter_error
                .as_ref()
                .unwrap()
                .contains(expected_error)
        );
        let error = backend
            .prepare_with_lineage(&request, BuildLineage::from_bytes([82; 16]))
            .err()
            .unwrap();
        assert_eq!(error.kind, EvaluationErrorKind::UnsupportedCapability);
        assert!(error.message.contains(expected_error));
    }
}

#[test]
fn source_loading_failure_is_reported_without_erasing_earlier_loaded_entries() {
    let backend = NativeBackend::new();
    let xml = "<PathOfBuilding2><Skills><SkillSet id='1'><Skill><Gem skillId='SparkPlayer' level='1'/></Skill><Skill><Gem skillId='SparkPlayer'/></Skill></SkillSet></Skills></PathOfBuilding2>";
    let PreparationOutcome::Incomplete(report) = backend
        .prepare_request_with_lineage(&request(xml), BuildLineage::from_bytes([83; 16]))
        .unwrap()
    else {
        panic!("incomplete source gained numeric admission")
    };
    let stage = report.authored_skills.as_ref().unwrap();
    assert_eq!(stage.status, SkillPreparationStatus::SourceFailure);
    assert!(stage.groups[0].gems[0].processed);
    assert!(!stage.groups[1].gems[0].processed);
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.stage == "validate_gem_level"
                && issue.kind == poe_optimizer_native::PreparationIssueKind::SourceError)
    );
    let completed = Some(
        poe_optimizer_import::build_instance::AuthoredInstanceId::SkillEntry(
            stage.groups[0].gems[0].instance,
        ),
    );
    let unfinished = Some(
        poe_optimizer_import::build_instance::AuthoredInstanceId::SkillEntry(
            stage.groups[1].gems[0].instance,
        ),
    );
    assert!(
        !report
            .issues
            .iter()
            .any(|issue| issue.instance == completed && issue.stage == "skill_identity")
    );
    assert!(
        report
            .issues
            .iter()
            .any(|issue| issue.instance == unfinished
                && issue.stage == "skill_effect_producers"
                && issue.message.contains("requires levels/stat sets"))
    );
}
