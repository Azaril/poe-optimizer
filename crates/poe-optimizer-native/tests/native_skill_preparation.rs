#![cfg(not(target_arch = "wasm32"))]
use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{SelectionRequest, ViewRequest},
    evaluation::EvaluationErrorKind,
};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveLimits, resolve_view},
};
use poe_optimizer_native::{
    CompiledGameData,
    skills::{
        PreparedSkills, SkillFailureKind, SkillIdentityStatus, SkillPreparationLimits,
        SkillPreparationStatus, prepare_authored_skills,
    },
};
use std::sync::{Arc, OnceLock};

fn data() -> &'static Arc<CompiledGameData> {
    static DATA: OnceLock<Arc<CompiledGameData>> = OnceLock::new();
    DATA.get_or_init(|| CompiledGameData::bundled().unwrap())
}
fn import(xml: &str, lineage: u8) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([lineage; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn source(groups: &str) -> String {
    format!(
        "<PathOfBuilding2><Skills><SkillSet id='1'>{groups}</SkillSet></Skills></PathOfBuilding2>"
    )
}
fn prepare(xml: &str) -> PreparedSkills {
    let build = import(xml, 70);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    prepare_authored_skills(&build, &view, data(), SkillPreparationLimits::default()).unwrap()
}

#[test]
fn completed_authored_stage_retains_owner_and_compiled_data_after_callers_drop() {
    let xml =
        source("<Skill enabled='true'><Gem skillId='SparkPlayer' level='1' quality='0'/></Skill>");
    let stage = Arc::new(prepare(&xml));
    assert_eq!(stage.report().status, SkillPreparationStatus::Complete);
    assert_eq!(stage.report().selected_groups.len(), 1);
    assert_eq!(stage.report().groups[0].processing_passes, 2);
    assert!(stage.report().groups[0].gems[0].processed);
    assert_eq!(
        stage.report().groups[0].gems[0].identity_status,
        SkillIdentityStatus::ResolvedGem
    );
    let expected = serde_json::to_vec(stage.report()).unwrap();
    let handles: Vec<_> = (0..4)
        .map(|_| {
            let stage = Arc::clone(&stage);
            std::thread::spawn(move || serde_json::to_vec(stage.report()).unwrap())
        })
        .collect();
    for handle in handles {
        assert_eq!(handle.join().unwrap(), expected);
    }
}

#[test]
fn private_stage_rejects_foreign_owners_compiled_snapshots_and_other_selected_views() {
    let xml = "<PathOfBuilding2><Skills><SkillSet id='1'><Skill><Gem skillId='SparkPlayer' level='1'/></Skill></SkillSet><SkillSet id='2'><Skill><Gem skillId='SparkPlayer' level='2'/></Skill></SkillSet></Skills></PathOfBuilding2>";
    let build = import(xml, 71);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    let stage =
        prepare_authored_skills(&build, &view, data(), SkillPreparationLimits::default()).unwrap();
    stage
        .validate_binding(&build.clone(), &view, data())
        .unwrap();
    let foreign = import(xml, 71);
    let foreign_view = resolve_view(
        &foreign,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        stage
            .validate_binding(&foreign, &foreign_view, data())
            .is_err()
    );
    let second = build
        .instances()
        .iter()
        .filter_map(|binding| match binding.instance() {
            AuthoredInstanceId::SkillSet(id) => Some(id),
            _ => None,
        })
        .nth(1)
        .unwrap();
    let request = ViewRequest {
        skills: SelectionRequest::Instance(second),
        ..ViewRequest::default()
    };
    let another_view = resolve_view(
        &build,
        data().snapshot(),
        &request,
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        stage
            .validate_binding(&build, &another_view, data())
            .is_err()
    );
    let foreign_data =
        Arc::new(CompiledGameData::compile(Arc::new(data().snapshot().clone())).unwrap());
    let foreign_data_view = resolve_view(
        &build,
        foreign_data.snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    assert!(
        stage
            .validate_binding(&build, &foreign_data_view, &foreign_data)
            .is_err()
    );
}

#[test]
fn source_runtime_failure_retains_processed_prefix_and_does_not_attach_failed_group() {
    let xml = source(
        "<Skill><Gem skillId='SparkPlayer' level='1'/></Skill><Skill><Gem skillId='SparkPlayer'/></Skill><Skill><Gem skillId='SparkPlayer' level='5'/></Skill>",
    );
    let stage = prepare(&xml);
    let report = stage.report();
    assert_eq!(report.status, SkillPreparationStatus::SourceFailure);
    let failure = report.failure.as_ref().unwrap();
    assert_eq!(failure.kind, SkillFailureKind::SourceRuntime);
    assert_eq!(failure.stage, "validate_gem_level");
    assert_eq!(report.groups.len(), 2);
    assert!(report.groups[0].attached);
    assert!(!report.groups[1].attached);
    assert!(report.groups[0].gems[0].processed);
    assert!(!report.groups[1].gems[0].processed);
    assert_eq!(
        report.groups[1].gems[0].identity_status,
        SkillIdentityStatus::ResolvedGem
    );
    assert!(report.selected_groups.is_empty());
}

#[test]
fn unresolved_and_ambiguous_names_are_completed_source_results_not_admitted_actions() {
    let xml = source(
        "<Skill><Gem nameSpec='impossible 0123456789 9876543210' level='1'/><Gem nameSpec='.*' level='1'/></Skill>",
    );
    let stage = prepare(&xml);
    let report = stage.report();
    assert_eq!(report.status, SkillPreparationStatus::Complete);
    let gems = &report.groups[0].gems;
    assert_eq!(gems[0].identity_status, SkillIdentityStatus::UnresolvedName);
    assert_eq!(gems[1].identity_status, SkillIdentityStatus::AmbiguousName);
    assert!(
        gems.iter()
            .all(|gem| gem.processed && gem.gem_data.is_none() && gem.granted_effect.is_none())
    );
    assert!(!report.frontiers.is_empty());
}

#[test]
fn unknown_name_pattern_errors_and_namespace_frontiers_remain_distinct() {
    let stage = prepare(&source("<Skill><Gem nameSpec='[' level='1'/></Skill>"));
    assert_eq!(stage.report().status, SkillPreparationStatus::SourceFailure);
    assert_eq!(
        stage.report().failure.as_ref().unwrap().stage,
        "find_skill_gem"
    );
    let stage =
        prepare("<PathOfBuilding2><Skills xmlns='urn:other'><Skill/></Skills></PathOfBuilding2>");
    assert_eq!(stage.report().status, SkillPreparationStatus::Unsupported);
    assert_eq!(
        stage.report().failure.as_ref().unwrap().kind,
        SkillFailureKind::UnsupportedSource
    );
}

#[test]
fn authored_triggered_attribute_is_not_a_provider_and_stage_resources_fail_explicitly() {
    let xml = source(
        "<Skill><Gem skillId='SparkPlayer' level='1' triggered='true'/><Gem skillId='SparkPlayer' level='1'/></Skill>",
    );
    let stage = prepare(&xml);
    assert!(stage.report().effect_cost_overrides.is_empty());
    let gems = &stage.report().groups[0].gems;
    assert_ne!(gems[0].instance, gems[1].instance);
    assert!(gems.iter().all(|gem| !gem.fields.contains_key("triggered")));
    let build = import(&xml, 72);
    let view = resolve_view(
        &build,
        data().snapshot(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    for limits in [
        SkillPreparationLimits {
            max_groups: 0,
            ..SkillPreparationLimits::default()
        },
        SkillPreparationLimits {
            max_entries: 0,
            ..SkillPreparationLimits::default()
        },
        SkillPreparationLimits {
            max_text_bytes: 0,
            ..SkillPreparationLimits::default()
        },
    ] {
        let error = match prepare_authored_skills(&build, &view, data(), limits) {
            Err(error) => error,
            Ok(_) => panic!("expected explicit resource failure"),
        };
        assert_eq!(error.kind, EvaluationErrorKind::InvalidRequest);
    }
}

#[test]
fn independent_preparation_runs_share_definitions_without_sharing_build_state() {
    let xml = source(
        "<Skill><Gem skillId='SparkPlayer' level='1'/><Gem nameSpec='impossible 0123456789' level='1'/></Skill>",
    );
    let data = Arc::clone(data());
    let build = import(&xml, 73);
    let expected = {
        let view = resolve_view(
            &build,
            data.snapshot(),
            &ViewRequest::default(),
            ResolveLimits::default(),
        )
        .unwrap();
        serde_json::to_vec(
            prepare_authored_skills(&build, &view, &data, SkillPreparationLimits::default())
                .unwrap()
                .report(),
        )
        .unwrap()
    };
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let build = build.clone();
            let data = Arc::clone(&data);
            std::thread::spawn(move || {
                let view = resolve_view(
                    &build,
                    data.snapshot(),
                    &ViewRequest::default(),
                    ResolveLimits::default(),
                )
                .unwrap();
                let prepared = prepare_authored_skills(
                    &build,
                    &view,
                    &data,
                    SkillPreparationLimits::default(),
                )
                .unwrap();
                assert!(prepared.report().effect_cost_overrides.is_empty());
                serde_json::to_vec(prepared.report()).unwrap()
            })
        })
        .collect();
    for worker in workers {
        assert_eq!(worker.join().unwrap(), expected);
    }
}
