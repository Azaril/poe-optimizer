use poe_optimizer_core::{
    build_identity::{BuildLineage, InstanceId, SkillSetId},
    build_view::{SelectionRequest, ViewRequest, WeaponStateRequest},
};
use poe_optimizer_data::game_data::{GameDataSnapshot, bundled_snapshot};
use poe_optimizer_import::{
    build_instance::{AuthoredInstanceId, ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    selected_view::{ResolveError, ResolveLimits, SetOrigin, resolve_view},
};
use std::sync::OnceLock;
fn data() -> &'static GameDataSnapshot {
    static DATA: OnceLock<GameDataSnapshot> = OnceLock::new();
    DATA.get_or_init(|| bundled_snapshot().unwrap())
}
fn build(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([31; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn base() -> ImportedBuildInstance {
    build(
        "<PathOfBuilding2><Skills activeSkillSet='4'><SkillSet id='2'/><SkillSet id='4'/></Skills><Items activeItemSet='8'><ItemSet id='8' useSecondWeaponSet='true'/><ItemSet id='2'/></Items><Tree activeSpec='2'><Spec id='777'/><Spec id='777'/></Tree><Config activeConfigSet='7'><ConfigSet id='7'/></Config></PathOfBuilding2>",
    )
}
#[test]
fn owner_binding_rejects_independent_same_bytes_and_cloned_definition_owner() {
    let original = base();
    let view = resolve_view(
        &original,
        data(),
        &ViewRequest::default(),
        ResolveLimits::default(),
    )
    .unwrap();
    view.validate_binding(&original.clone(), data()).unwrap();
    let independent = build(original.source_xml());
    assert!(matches!(
        view.validate_binding(&independent, data()),
        Err(ResolveError::ForeignBinding)
    ));
    let other_data = data().clone();
    assert!(matches!(
        view.validate_binding(&original, &other_data),
        Err(ResolveError::ForeignBinding)
    ));
    assert_eq!(
        view.report().skills.selected.as_ref().unwrap().key.value(),
        4.0
    );
    assert_eq!(
        view.report().items.selected.as_ref().unwrap().key.value(),
        8.0
    );
    assert_eq!(
        view.report()
            .passives
            .selected
            .as_ref()
            .unwrap()
            .key
            .value(),
        2.0
    );
    assert_eq!(
        view.report()
            .configuration
            .selected
            .as_ref()
            .unwrap()
            .key
            .value(),
        7.0
    );
    assert_eq!(view.report().weapon_state.use_second_weapon_set, Some(true));
    assert!(!view.report().frontiers.is_empty());
}
#[test]
fn overrides_use_exact_live_instances_and_keep_saved_selectors() {
    let original = base();
    let id = original
        .instances()
        .iter()
        .find_map(|b| {
            if let AuthoredInstanceId::SkillSet(id) = b.instance() {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let request = ViewRequest {
        skills: SelectionRequest::Instance(id),
        weapon_state: WeaponStateRequest::Primary,
        ..Default::default()
    };
    let view = resolve_view(&original, data(), &request, ResolveLimits::default()).unwrap();
    assert_eq!(view.report().skills.authored.raw.as_deref(), Some("4"));
    assert_eq!(
        view.report()
            .skills
            .selected
            .as_ref()
            .unwrap()
            .origin
            .instance(),
        Some(AuthoredInstanceId::SkillSet(id))
    );
    assert_eq!(
        view.report().skills.selected.as_ref().unwrap().key.value(),
        2.0
    );
    assert_eq!(
        view.report().items.selected.as_ref().unwrap().key.value(),
        8.0
    );
    assert_eq!(view.report().weapon_state.authored.as_deref(), Some("true"));
    assert_eq!(
        view.report().weapon_state.use_second_weapon_set,
        Some(false)
    );
    let bad = ViewRequest {
        skills: SelectionRequest::Instance(SkillSetId::from_instance_id(
            InstanceId::from_parts(original.lineage(), 99999).unwrap(),
        )),
        ..Default::default()
    };
    assert!(matches!(
        resolve_view(&original, data(), &bad, ResolveLimits::default()),
        Err(ResolveError::Import(_))
    ));
}
#[test]
fn shadowed_overrides_do_not_retarget_equal_external_keys() {
    let original = build(
        "<PathOfBuilding2><Skills><SkillSet id='1'/><SkillSet id='1.0'/></Skills></PathOfBuilding2>",
    );
    let first = original
        .instances()
        .iter()
        .find_map(|b| {
            if let AuthoredInstanceId::SkillSet(id) = b.instance() {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let request = ViewRequest {
        skills: SelectionRequest::Instance(first),
        ..Default::default()
    };
    let view = resolve_view(&original, data(), &request, ResolveLimits::default()).unwrap();
    assert!(view.report().skills.selected.is_none());
    assert_eq!(
        view.report().skills.problem.as_ref().unwrap().code,
        "shadowed_override"
    );
}
#[test]
fn positional_selection_retains_ieee_inputs_and_upper_only_clamping() {
    for (raw, expected) in [
        ("0", None),
        ("-0", None),
        ("-1", None),
        ("1.5", None),
        ("99", Some(2.0)),
        ("inf", Some(2.0)),
        ("nan", Some(2.0)),
        ("invalid", Some(1.0)),
    ] {
        let original = build(&format!(
            "<PathOfBuilding2><Tree activeSpec='{raw}'><Spec id='99'/><Spec id='1'/></Tree></PathOfBuilding2>"
        ));
        let view =
            resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
        assert_eq!(
            view.report()
                .passives
                .selected
                .as_ref()
                .map(|s| s.key.value()),
            expected,
            "{raw}"
        );
        let wire = serde_json::to_value(view.report()).unwrap();
        if raw == "-0" {
            assert_eq!(wire["passives"]["authored"]["parsed"], "8000000000000000");
        }
        if raw == "nan" {
            assert!(
                view.report()
                    .passives
                    .authored
                    .parsed
                    .unwrap()
                    .value()
                    .is_nan()
            );
        }
    }
}
#[test]
fn item_generated_keys_and_legacy_weapon_state_are_domain_specific() {
    let original = build(
        "<PathOfBuilding2><Items activeItemSet='bad'><ItemSet id='2'/><ItemSet id='8'/><ItemSet/></Items></PathOfBuilding2>",
    );
    let view = resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
    assert_eq!(
        view.report()
            .items
            .sets
            .iter()
            .map(|s| s.key.value())
            .collect::<Vec<_>>(),
        [2.0, 8.0, 1.0]
    );
    assert_eq!(
        view.report().items.selected.as_ref().unwrap().key.value(),
        1.0
    );
    let legacy = build(
        "<PathOfBuilding2><Items useSecondWeaponSet='true'><Slot name='Ring 1' itemId='26'/><Slot name='Ring 2' itemId='26'/></Items></PathOfBuilding2>",
    );
    let view = resolve_view(&legacy, data(), &Default::default(), Default::default()).unwrap();
    let chosen = view.report().items.selected.as_ref().unwrap();
    assert!(matches!(chosen.origin, SetOrigin::Default { .. }));
    assert_eq!(chosen.members.len(), 2);
    assert_ne!(chosen.members[0], chosen.members[1]);
    assert_eq!(view.report().weapon_state.use_second_weapon_set, Some(true));
}
#[test]
fn selected_identity_evidence_uses_source_occurrences_and_injected_catalog() {
    let original = build(
        "<PathOfBuilding2><Skills activeSkillSet='2'><SkillSet id='1'><Skill><Gem skillId='InactiveUnknown'/></Skill></SkillSet><SkillSet id='2'><Skill><Gem skillId='SparkPlayer'/><Mystery gemId='Unknown' skillId='SparkPlayer'/><Gem nameSpec='Needs matching'/></Skill></SkillSet></Skills></PathOfBuilding2>",
    );
    let view = resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
    assert_eq!(view.report().skill_identities.entries.len(), 3);
    assert!(view.report().skill_identities.problem.is_none());
    use poe_optimizer_import::skill_definitions::InstanceIdentityResolution as R;
    assert!(matches!(
        &view.report().skill_identities.entries[0].resolution,
        R::ExplicitEffect {
            matched: Some(_),
            ..
        }
    ));
    assert!(
        matches!(&view.report().skill_identities.entries[1].resolution,R::ExternalGem{candidates,..} if candidates.is_empty())
    );
    assert!(matches!(
        &view.report().skill_identities.entries[2].resolution,
        R::NameOnlyNotResolved
    ));
    for entry in &view.report().skill_identities.entries {
        assert_eq!(
            original.binding(entry.instance).unwrap().source(),
            entry.source
        );
    }
}
#[test]
fn repeated_containers_keep_earlier_problems_and_reject_old_override() {
    let original = build(
        "<PathOfBuilding2><Skills><SkillSet id='NaN'/></Skills><Skills><SkillSet id='8'/></Skills><Config><ConfigSet id='2'/></Config><Config><ConfigSet id='4'/></Config></PathOfBuilding2>",
    );
    let view = resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
    assert_eq!(
        view.report().skills.selected.as_ref().unwrap().key.value(),
        8.0
    );
    assert_eq!(
        view.report()
            .configuration
            .selected
            .as_ref()
            .unwrap()
            .key
            .value(),
        4.0
    );
    assert_eq!(view.report().load_problems.len(), 1);
    assert!(
        view.report()
            .frontiers
            .iter()
            .any(|f| f.stage == "root_lifecycle")
    );
}
#[test]
fn limits_and_portable_request_validation_do_not_truncate_results() {
    let original = base();
    for limits in [
        ResolveLimits {
            max_records: 0,
            ..Default::default()
        },
        ResolveLimits {
            max_fragments: 0,
            ..Default::default()
        },
        ResolveLimits {
            max_text_bytes: 0,
            ..Default::default()
        },
    ] {
        assert!(resolve_view(&original, data(), &Default::default(), limits).is_err());
    }
    let request = ViewRequest {
        label: "bad\nlabel".into(),
        ..Default::default()
    };
    assert!(matches!(
        resolve_view(&original, data(), &request, Default::default()),
        Err(ResolveError::Request(_))
    ));
    let wire = serde_json::to_string(&ViewRequest::default()).unwrap();
    assert_eq!(
        serde_json::from_str::<ViewRequest>(&wire).unwrap(),
        ViewRequest::default()
    );
    fn send_sync<T: Send + Sync>() {}
    send_sync::<poe_optimizer_import::selected_view::SelectedView<'static>>();
}

#[test]
fn explicit_passive_choice_replaces_invalid_saved_position_but_keeps_evidence() {
    let original = build("<PathOfBuilding2><Tree activeSpec='0'><Spec/></Tree></PathOfBuilding2>");
    let id = original
        .instances()
        .iter()
        .find_map(|b| {
            if let AuthoredInstanceId::PassiveSpec(id) = b.instance() {
                Some(id)
            } else {
                None
            }
        })
        .unwrap();
    let request = ViewRequest {
        passives: SelectionRequest::Instance(id),
        ..Default::default()
    };
    let view = resolve_view(&original, data(), &request, Default::default()).unwrap();
    assert_eq!(view.report().passives.authored.raw.as_deref(), Some("0"));
    assert_eq!(
        view.report()
            .passives
            .selected
            .as_ref()
            .unwrap()
            .origin
            .instance(),
        Some(AuthoredInstanceId::PassiveSpec(id))
    );
    assert!(view.report().passives.problem.is_none());
    assert_eq!(
        view.report()
            .passives
            .saved_selection_problem
            .as_ref()
            .unwrap()
            .code,
        "invalid_spec_position"
    );
}
#[test]
fn nested_namespace_and_all_copied_selection_text_are_bounded() {
    for xml in [
        "<PathOfBuilding2><Tree><Spec xmlns='urn:future'/></Tree></PathOfBuilding2>",
        "<PathOfBuilding2><Items><ItemSet xmlns='urn:future'/></Items></PathOfBuilding2>",
    ] {
        let original = build(xml);
        let view =
            resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
        assert!(
            view.report()
                .load_problems
                .iter()
                .any(|p| p.code == "namespace_context")
        );
    }
    for xml in [
        "<PathOfBuilding2><Items useSecondWeaponSet='true'/></PathOfBuilding2>",
        "<PathOfBuilding2><Skills><SkillSet><Skill><Gem skillId='SparkPlayer'/></Skill></SkillSet></Skills></PathOfBuilding2>",
    ] {
        let original = build(xml);
        assert!(
            resolve_view(
                &original,
                data(),
                &Default::default(),
                ResolveLimits {
                    max_text_bytes: 0,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn many_generated_item_ids_and_separate_empty_specs_preserve_full_order() {
    let xml = format!(
        "<PathOfBuilding2><Items activeItemSet='1024'>{}</Items><Tree activeSpec='512'>{}</Tree></PathOfBuilding2>",
        "<ItemSet/>".repeat(1024),
        "<Spec/>".repeat(512)
    );
    let original = build(&xml);
    let view = resolve_view(&original, data(), &Default::default(), Default::default()).unwrap();
    assert_eq!(view.report().items.sets.len(), 1024);
    assert_eq!(
        view.report().items.selected.as_ref().unwrap().key.value(),
        1024.0
    );
    assert_eq!(view.report().passives.sets.len(), 512);
    assert_eq!(
        view.report()
            .passives
            .selected
            .as_ref()
            .unwrap()
            .key
            .value(),
        512.0
    );
}
