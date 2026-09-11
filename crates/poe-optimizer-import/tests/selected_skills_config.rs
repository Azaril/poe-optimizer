use poe_optimizer_core::{
    build_identity::BuildLineage,
    build_view::{SelectionRequest, ViewRequest},
};
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{
        DomainSelection, ResolveError, ResolveLimits, SelectedViewReport, SetOrigin, resolve_view,
    },
};
use std::sync::OnceLock;

fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn build(sections: &str) -> ImportedBuildInstance {
    let xml = format!("<PathOfBuilding2>{sections}</PathOfBuilding2>");
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([7; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn view(sections: &str) -> SelectedViewReport {
    let build = build(sections);
    resolve_view(
        &build,
        data(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap()
    .report()
    .clone()
}
fn selected(selection: &DomainSelection) -> f64 {
    selection.selected.as_ref().unwrap().key.value()
}
fn keys(selection: &DomainSelection) -> Vec<f64> {
    selection.sets.iter().map(|r| r.key.value()).collect()
}
fn order(selection: &DomainSelection) -> Vec<Option<f64>> {
    selection
        .order
        .iter()
        .map(|v| v.map(|v| v.value()))
        .collect()
}
fn problem(selection: &DomainSelection) -> &str {
    selection.problem.as_ref().unwrap().code
}

#[test]
fn constructor_and_empty_containers_have_loader_defaults_without_authored_ids() {
    for source in [
        "",
        "<Skills/><Config/>",
        "<Skills></Skills><Config></Config>",
        "<Skills><!-- empty --></Skills><Config> \n <!-- empty --> </Config>",
    ] {
        let report = view(source);
        for selection in [&report.skills, &report.configuration] {
            assert!(selection.problem.is_none(), "{source}");
            assert_eq!(keys(selection), vec![1.0]);
            assert_eq!(selected(selection), 1.0);
            assert!(matches!(
                selection.selected.as_ref().unwrap().origin,
                SetOrigin::Default { .. }
            ));
            assert!(selection.selected.as_ref().unwrap().members.is_empty());
        }
    }
}

#[test]
fn skill_keys_use_lua_numbers_and_sparse_length_for_generated_ids() {
    for (source, expected) in [
        (
            "<SkillSet id='1'/><SkillSet id='3'/><SkillSet/>",
            vec![1.0, 3.0, 2.0],
        ),
        (
            "<SkillSet id='2'/><SkillSet id='8'/><SkillSet id='garbage'/>",
            vec![2.0, 8.0, 3.0],
        ),
        (
            "<SkillSet id='-2.5'/><SkillSet id='0'/><SkillSet/>",
            vec![-2.5, 0.0, 1.0],
        ),
        (
            "<SkillSet id='1'/><SkillSet id='2'/><SkillSet id='4'/><SkillSet/>",
            vec![1.0, 2.0, 4.0, 5.0],
        ),
    ] {
        let report = view(&format!("<Skills activeSkillSet='999'>{source}</Skills>"));
        assert_eq!(keys(&report.skills), expected);
        assert_eq!(selected(&report.skills), expected[0]);
    }
    for selector in ["-2.5", "-0", "0x4", "inf", "nan", "bad"] {
        let report = view(&format!(
            "<Skills activeSkillSet='{selector}'><SkillSet id='4'/><SkillSet id='-2.5'/><SkillSet id='0'/><SkillSet id='inf'/></Skills>"
        ));
        assert_eq!(report.skills.authored.raw.as_deref(), Some(selector));
        let expected = match selector {
            "-2.5" => -2.5,
            "-0" => 0.0,
            "inf" => f64::INFINITY,
            _ => 4.0,
        };
        assert_eq!(selected(&report.skills), expected);
        if selector == "-0" {
            assert_eq!(
                report.skills.authored.requested.value().to_bits(),
                (-0.0f64).to_bits()
            );
        }
        if selector == "nan" {
            assert!(report.skills.authored.requested.value().is_nan());
        }
    }
}

#[test]
fn duplicate_skill_records_keep_occurrences_but_latest_key_wins_members() {
    let report = view(
        "<Skills activeSkillSet='1'><SkillSet id='1'><Skill label='old'/></SkillSet><SkillSet id='1.0'><Skill label='new'/><Skill label='second'/></SkillSet><Skill label='legacy'/></Skills>",
    );
    let selection = &report.skills;
    assert_eq!(keys(selection), vec![1.0, 1.0]);
    assert_eq!(order(selection), vec![Some(1.0), Some(1.0)]);
    assert_ne!(selection.sets[0].origin, selection.sets[1].origin);
    assert_eq!(selection.sets[0].members.len(), 1);
    assert_eq!(selection.sets[1].members.len(), 3);
    assert_eq!(
        selection.selected.as_ref().unwrap().origin,
        selection.sets[1].origin
    );
    assert_ne!(selection.sets[0].members[0], selection.sets[1].members[0]);
}

#[test]
fn legacy_skill_groups_observe_order_initialization_and_missing_set_failure() {
    let report =
        view("<Skills><Skill label='first'/><SkillSet id='7'/><Skill label='last'/></Skills>");
    assert_eq!(keys(&report.skills), vec![1.0, 7.0]);
    assert_eq!(report.skills.sets[0].members.len(), 2);
    let report =
        view("<Skills><SkillSet id='7'/><Skill label='unloadable'/><SkillSet id='1'/></Skills>");
    assert_eq!(problem(&report.skills), "missing_legacy_skill_set");
    assert_eq!(keys(&report.skills), vec![7.0]);
    assert!(report.skills.selected.is_none());
}

#[test]
fn config_creation_order_retains_text_holes_and_duplicates() {
    let report = view(
        "<Config><ConfigSet id='7'/><Input name='a' number='1'/><ConfigSet id='2'/><ConfigSet id='7.0'/></Config>",
    );
    assert_eq!(keys(&report.configuration), vec![7.0, 1.0, 2.0, 7.0]);
    assert_eq!(
        order(&report.configuration),
        vec![Some(7.0), None, Some(2.0), Some(7.0)]
    );
    assert_eq!(selected(&report.configuration), 1.0);
    let report = view(
        "<Config activeConfigSet='999'><ConfigSet id='7'/>text<ConfigSet id='2'/><ConfigSet id='7'/></Config>",
    );
    assert_eq!(
        order(&report.configuration),
        vec![Some(7.0), None, Some(2.0), Some(7.0)]
    );
    assert_eq!(
        report.configuration.selected.as_ref().unwrap().origin,
        report.configuration.sets.last().unwrap().origin
    );
    let report = view("<Config>text<ConfigSet id='-2.5'/></Config>");
    assert_eq!(order(&report.configuration), vec![Some(1.0), Some(-2.5)]);
    assert_eq!(keys(&report.configuration), vec![1.0, -2.5]);
}

#[test]
fn nil_config_id_records_generated_creation_then_stops_before_selector() {
    for missing in ["", " id='bad'"] {
        let report = view(&format!(
            "<Config><ConfigSet id='2'/><ConfigSet id='8'/><ConfigSet{missing}/><ConfigSet id='99'/></Config>"
        ));
        assert_eq!(keys(&report.configuration), vec![2.0, 8.0, 3.0]);
        assert_eq!(
            order(&report.configuration),
            vec![Some(2.0), Some(8.0), None]
        );
        assert_eq!(problem(&report.configuration), "nil_config_set_key");
    }
    for source in [
        "<Config><ConfigSet id='2'/><ConfigSet id='nan'/></Config>",
        "<Skills><SkillSet id='2'/><SkillSet id='nan'/></Skills>",
    ] {
        let report = view(source);
        let selection = if source.starts_with("<Config>") {
            &report.configuration
        } else {
            &report.skills
        };
        assert_eq!(keys(selection), vec![2.0]);
        assert_eq!(problem(selection), "nan_set_key");
    }
}

#[test]
fn config_diagnostic_returns_do_not_abort_original_set_selection() {
    let report = view(
        "<Config activeConfigSet='-2.5'><ConfigSet id='-2.5'><Input number='1'/><Input name='missing-value'/><Placeholder name='missing-number'/></ConfigSet></Config>",
    );
    assert!(report.configuration.problem.is_none());
    assert_eq!(selected(&report.configuration), -2.5);
    assert!(
        report
            .configuration
            .selected
            .as_ref()
            .unwrap()
            .members
            .is_empty()
    );
}

#[test]
fn child_load_structural_errors_do_not_claim_a_selected_skill_set() {
    for (body, expected) in [
        ("text", "skill_child_text"),
        (
            "<UnknownGem><MinionSkillIndexLookup grantedEffect='e'>text</MinionSkillIndexLookup></UnknownGem>",
            "minion_map_text",
        ),
        (
            "<Gem><MinionSkillIndexLookup grantedEffect='e'><Unknown statSetIndex='2'/></MinionSkillIndexLookup></Gem>",
            "nil_minion_map_key",
        ),
        (
            "<Gem><MinionSkillIndexLookupCalcs grantedEffect='e'><Unknown skillIndex='nan' statSetIndex='2'/></MinionSkillIndexLookupCalcs></Gem>",
            "nan_minion_map_key",
        ),
    ] {
        let report = view(&format!(
            "<Skills><SkillSet id='1'><Skill>{body}</Skill></SkillSet></Skills>"
        ));
        assert_eq!(problem(&report.skills), expected);
        assert!(report.skills.sets[0].members.is_empty());
    }
    let report = view(
        "<Skills>ignored<SkillSet id='1'>ignored<Skill><UnknownGem>ignored<MinionSkillIndexLookup><Unknown/></MinionSkillIndexLookup><MinionSkillIndexLookup grantedEffect='e'><Unknown skillIndex='-2.5'/></MinionSkillIndexLookup></UnknownGem></Skill></SkillSet></Skills>",
    );
    assert!(report.skills.problem.is_none());
    assert_eq!(report.skills.sets[0].members.len(), 1);
}

#[test]
fn independent_instance_requests_cannot_silently_retarget_shadowed_records() {
    let build = build(
        "<Skills><SkillSet id='2'/><SkillSet id='1'/><SkillSet id='2.0'/></Skills><Config><ConfigSet id='7'/><ConfigSet id='9'/></Config>",
    );
    let ids: Vec<_> = build
        .instances()
        .iter()
        .map(|binding| binding.instance())
        .collect();
    let skills: Vec<_> = ids
        .iter()
        .filter_map(|id| {
            if let AuthoredInstanceId::SkillSet(id) = id {
                Some(*id)
            } else {
                None
            }
        })
        .collect();
    let config = ids
        .iter()
        .find_map(|id| {
            if let AuthoredInstanceId::ConfigSet(id) = id {
                Some(*id)
            } else {
                None
            }
        })
        .unwrap();
    let mut request = ViewRequest {
        skills: SelectionRequest::Instance(skills[1]),
        configuration: SelectionRequest::Instance(config),
        ..ViewRequest::default()
    };
    let report = resolve_view(&build, data(), &request, ResolveLimits::default()).unwrap();
    assert_eq!(selected(&report.report().skills), 1.0);
    assert_eq!(selected(&report.report().configuration), 7.0);
    request.skills = SelectionRequest::Instance(skills[0]);
    let report = resolve_view(&build, data(), &request, ResolveLimits::default()).unwrap();
    assert_eq!(problem(&report.report().skills), "shadowed_override");
    request.skills = SelectionRequest::Instance(skills[2]);
    assert_eq!(
        selected(
            &resolve_view(&build, data(), &request, ResolveLimits::default())
                .unwrap()
                .report()
                .skills
        ),
        2.0
    );
}

#[test]
fn source_namespace_and_resource_limits_are_explicit() {
    for source in [
        "<Skills><SkillSet xmlns='urn:test' id='1'/></Skills>",
        "<Skills><SkillSet id='1'><Skill><Gem xmlns='urn:test'/></Skill></SkillSet></Skills>",
        "<Config><ConfigSet xmlns='urn:test' id='1'/></Config>",
    ] {
        let report = view(source);
        let selection = if source.starts_with("<Config>") {
            &report.configuration
        } else {
            &report.skills
        };
        assert_eq!(problem(selection), "namespace_context");
        assert!(selection.selected.is_none());
    }
    let build = build("<Skills><SkillSet id='1'/></Skills>");
    assert!(matches!(
        resolve_view(
            &build,
            data(),
            &ViewRequest::default(),
            ResolveLimits {
                max_records: 0,
                ..ResolveLimits::default()
            }
        ),
        Err(ResolveError::Resource(_))
    ));
}
