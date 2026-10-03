#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
pub(crate) mod actions;
#[path = "support/minion_actions_fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod inventory;
use fixture::*;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_core::{owned_definitions::*, owned_draft::DraftListCompletion, owned_schema::*};
use poe_optimizer_import::owned_value_policy::*;
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_normalize::*,
    owned_source::SourceProjectEvidence, owned_source_actions::*,
};

fn report(f: &Fixture, text: &str, context: ImportReferenceContext) -> SourceActionReport {
    let source = ImportedBuildInstance::from_decoded(
        decode_build(text.as_bytes()).unwrap(),
        BuildLineage::from_bytes([64; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let request = f.base.request(&source, 0, context);
    f.compile(Default::default())
        .unwrap()
        .resolve(&evidence, &request)
        .unwrap()
}
fn target(report: &SourceActionReport) -> &ImportActionTarget {
    let ImportQueryTarget::Action(target) = &report.target else {
        panic!("unresolved selection: {:?}", report.selection)
    };
    target
}

#[test]
fn independent_minion_contexts_select_exact_actor_child_paths_and_stat_sets() {
    let f = Fixture::new();
    let text = player::xml(&player::gem(
        " skillMinion=\"fixture-minion\" skillMinionCalcs=\"fixture-minion\" skillMinionSkill=\"2\" skillMinionSkillCalcs=\"1\"",
        &maps(&entry_map("2", "2"), &entry_map("1", "1")),
    ));
    for (context, action, set) in [
        (ImportReferenceContext::Main, 1, 1),
        (ImportReferenceContext::Calcs, 0, 0),
    ] {
        let result = report(&f, &text, context);
        let target = target(&result);
        let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            entering_grant,
            minion,
            ..
        } = &f.input
        else {
            unreachable!()
        };
        assert_eq!(
            target.provider.grant_path,
            vec![
                entering_grant.clone(),
                minion.entering_grant.clone(),
                f.actions()[action].entering_grant.clone()
            ]
        );
        assert_eq!(target.output, f.actions()[action].output);
        assert_eq!(target.stat_set, f.base.sets[set]);
        let ImportActorTarget::Owned { provider, slot } = &target.actor else {
            panic!("expected exact owned actor")
        };
        assert_eq!(slot, f.population());
        assert_eq!(provider.grant_path, vec![entering_grant.clone()]);
        let provenance = result.minion.unwrap();
        assert_eq!(provenance.accounted_occurrences.len(), 4);
        assert_eq!(
            provenance.actor_attributes.len(),
            if context == ImportReferenceContext::Main {
                1
            } else {
                2
            }
        );
    }
}

#[test]
fn absent_singleton_is_explicit_and_calcs_checks_both_actor_branches() {
    let mut f = Fixture::new();
    let text = player::xml(&player::gem("", ""));
    let result = report(&f, &text, ImportReferenceContext::Calcs);
    assert_eq!(target(&result).output, f.actions()[0].output);
    assert!(result.minion.as_ref().unwrap().actor_attributes.is_empty());
    assert_eq!(result.selection, SourceActionSelection::Absent);
    for attribute in ["skillMinion", "skillMinionCalcs"] {
        let wrong = player::xml(&player::gem(
            &format!(" {attribute}=\"different-minion\""),
            "",
        ));
        assert!(matches!(
            report(&f, &wrong, ImportReferenceContext::Calcs).target,
            ImportQueryTarget::Unresolved(_)
        ));
    }
    let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { minion, .. } =
        &mut f.input
    else {
        unreachable!()
    };
    minion.allow_absent = false;
    assert!(matches!(
        report(&f, &text, ImportReferenceContext::Main).target,
        ImportQueryTarget::Unresolved(_)
    ));
}

#[test]
fn present_invalid_action_or_stat_indices_never_use_absence_or_source_clamps() {
    let f = Fixture::new();
    for token in ["", "0", "-1", "1.5", "3", "nil", "NaN", " 1", "1 "] {
        for attribute in ["skillMinionSkill", "skillMinionSkillCalcs"] {
            let context = if attribute == "skillMinionSkill" {
                ImportReferenceContext::Main
            } else {
                ImportReferenceContext::Calcs
            };
            let text = player::xml(&player::gem(&format!(" {attribute}=\"{token}\""), ""));
            assert!(
                matches!(
                    report(&f, &text, context).target,
                    ImportQueryTarget::Unresolved(_)
                ),
                "{attribute}={token:?}"
            );
        }
        let text = player::xml(&player::gem("", &maps(&entry_map("1", token), "")));
        assert!(
            matches!(
                report(&f, &text, ImportReferenceContext::Main).target,
                ImportQueryTarget::Unresolved(_)
            ),
            "stat {token:?}"
        );
    }
    let mut f = Fixture::new();
    let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
        absent_action, ..
    } = &mut f.input
    else {
        unreachable!()
    };
    *absent_action = None;
    assert!(matches!(
        report(
            &f,
            &player::xml(&player::gem("", "")),
            ImportReferenceContext::Main
        )
        .target,
        ImportQueryTarget::Unresolved(_)
    ));
    let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 { actions, .. } =
        &mut f.input
    else {
        unreachable!()
    };
    actions[0].absent_stat_set = None;
    assert!(matches!(
        report(
            &f,
            &player::xml(&player::gem(" skillMinionSkill=\"1\"", "")),
            ImportReferenceContext::Main
        )
        .target,
        ImportQueryTarget::Unresolved(_)
    ));
}

#[test]
fn every_nested_map_is_checked_even_for_unselected_children_or_other_context() {
    let f = Fixture::new();
    for children in [
        maps(
            &format!("{}{}", entry_map("1", "1"), entry_map("1.0", "2")),
            "",
        ),
        format!("{}{}", maps("", ""), maps("", "")),
        maps(&entry_map("3", "1"), ""),
        maps("", &entry_map("2", "bad")),
        maps("<MinionSkillIndexMap skillIndex=\"1\"/>", ""),
        maps("<MinionSkillIndexMap statSetIndex=\"1\"/>", ""),
        maps("<Future skillIndex=\"1\" statSetIndex=\"1\"/>", ""),
        maps(
            "<MinionSkillIndexMap xmlns=\"urn:foreign\" skillIndex=\"1\" statSetIndex=\"1\"/>",
            "",
        ),
        maps(&entry_map("1", "1").replace("/>", " future=\"true\"/>"), ""),
        maps(&entry_map("1", "1"), "")
            .replace("grantedEffect=\"effect\"", "grantedEffect=\"other\""),
        "<StatSetIndex grantedEffect=\"effect\" index=\"1\"/>".into(),
    ] {
        for context in [ImportReferenceContext::Main, ImportReferenceContext::Calcs] {
            assert!(
                matches!(
                    report(&f, &player::xml(&player::gem("", &children)), context).target,
                    ImportQueryTarget::Unresolved(_)
                ),
                "{children}"
            );
        }
    }
}

#[test]
fn constructor_checks_every_injected_topology_binding_and_recipe() {
    for case in 0..13 {
        let mut f = Fixture::new();
        let SourceActionCorrespondenceInput::PobPhysicalSingletonMinionActionsV1 {
            definitions,
            source,
            roles,
            minion,
            actions,
            absent_action,
            main_action_index,
            map_skill_index,
            ..
        } = &mut f.input
        else {
            unreachable!()
        };
        match case {
            0 => definitions.release.push_str("-stale"),
            1 => source.files[0].sha256 = "f".repeat(64),
            2 => *roles = "f".repeat(64).parse().unwrap(),
            3 => {
                minion.actor = ActorDefId::new(
                    minion.actor.namespace().clone(),
                    OwnedDefinitionKey::new("missing-actor").unwrap(),
                )
            }
            4 => minion.population.declaration = SlotOwnerDefId::Actor(minion.actor.clone()),
            5 => minion.entering_grant = actions[0].entering_grant.clone(),
            6 => actions[0].skill_id = "unmapped-child".into(),
            7 => actions[0].supply = actions[1].supply.clone(),
            8 => actions[0].output = actions[1].output.clone(),
            9 => actions[1].source_index = actions[0].source_index,
            10 => *absent_action = Some(3),
            11 => main_action_index.missing = MissingValuePolicy::Absent,
            12 => map_skill_index.numeric_aliases.push(NumericTokenAlias {
                token: "nil".into(),
                replacement: "1".into(),
            }),
            _ => unreachable!(),
        }
        assert!(
            f.compile(Default::default()).is_err(),
            "invalid case {case}"
        );
    }
    let f = Fixture::new();
    let mut wire = serde_json::to_value(&f.input).unwrap();
    wire["minion"]["ignore_unknown"] = true.into();
    assert!(serde_json::from_value::<SourceActionCorrespondenceInput>(wire).is_err());
}

#[test]
fn bounded_replay_retains_disabled_archived_identity_without_activity_claims() {
    let f = Fixture::new();
    let xml = player::xml(&player::gem(" enabled=\"false\"", ""))
        .replace("activeSkillSet=\"1\"", "activeSkillSet=\"2\"")
        .replace("</Skills>", "<SkillSet id=\"2\"/></Skills>");
    let source = ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([65; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let request = f.base.request(&source, 0, ImportReferenceContext::Main);
    let adapter = f.compile(Default::default()).unwrap();
    let first = adapter.resolve(&evidence, &request).unwrap();
    assert_eq!(target(&first).provider.skill_use, request.skill_use);
    let mut wrong = request.clone();
    wrong.skill_use.source_sha256 = "0".repeat(64);
    assert!(matches!(
        adapter.resolve(&evidence, &wrong).unwrap().target,
        ImportQueryTarget::Unresolved(_)
    ));
    assert_eq!(
        serde_json::to_vec(&first).unwrap(),
        serde_json::to_vec(&adapter.resolve(&evidence, &request).unwrap()).unwrap()
    );
    let limits = SourceActionLimits {
        max_work: first.work,
        ..Default::default()
    };
    assert!(
        f.compile(limits)
            .unwrap()
            .resolve(&evidence, &request)
            .is_ok()
    );
    let limits = SourceActionLimits {
        max_work: first.work - 1,
        ..Default::default()
    };
    assert!(
        f.compile(limits)
            .unwrap()
            .resolve(&evidence, &request)
            .is_err()
    );
    assert!(
        f.compile(SourceActionLimits {
            max_wire_bytes: 100,
            ..Default::default()
        })
        .is_err()
    );
    let wire = serde_json::to_vec(&f.input).unwrap();
    let roundtrip: SourceActionCorrespondenceInput = serde_json::from_slice(&wire).unwrap();
    assert_eq!(serde_json::to_vec(&roundtrip).unwrap(), wire);
}

fn inventory_fixture() -> inventory::Fixture {
    let mut f = Fixture::new();
    f.base.input = f.input;
    inventory::Fixture::from_base(f.base)
}

#[test]
fn actual_v3_normalization_closes_only_physical_inputs_with_exact_deferred_provenance() {
    let f = inventory_fixture();
    assert!(f.policy.usage_inputs.is_none());
    let children = maps(&entry_map("1", "2"), &entry_map("2", "1"));
    let gem=inventory::with_children(&children).replace("<Gem ","<Gem skillMinion=\"fixture-minion\" skillMinionCalcs=\"fixture-minion\" skillMinionSkill=\"1\" skillMinionSkillCalcs=\"2\" ");
    let normalized = f.run(&inventory::xml(&gem), Default::default()).unwrap();
    let draft = normalized.draft().input();
    assert!(matches!(
        draft.gems.members[0].parameters.completion,
        DraftListCompletion::Complete
    ));
    assert_eq!(draft.gems.members[0].parameters.members.len(), 2);
    let usage = draft.skill_presets.members[0]
        .usage_preferences
        .as_ref()
        .unwrap();
    assert!(usage.members.is_empty());
    assert!(matches!(
        usage.completion,
        DraftListCompletion::Pending { .. }
    ));
    let gem_id = draft.gems.members[0].id;
    let skill_id = draft.skills.members[0].id;
    assert_eq!(
        normalized
            .sidecar()
            .origins
            .iter()
            .filter(|row| row.links
                == [
                    OwnedOriginTarget::Gem(gem_id),
                    OwnedOriginTarget::Skill(skill_id)
                ])
            .count(),
        4
    );
    for bad in [
        gem.replace(
            "skillMinionCalcs=\"fixture-minion\"",
            "skillMinionCalcs=\"unknown\"",
        ),
        gem.replace("skillMinionSkillCalcs=\"2\"", "skillMinionSkillCalcs=\"3\""),
        gem.replace("<Gem ", "<Gem skillMinionItemSet=\"1\" "),
        gem.replace(
            "skillIndex=\"2\" statSetIndex=\"1\"",
            "skillIndex=\"2\" statSetIndex=\"99\"",
        ),
    ] {
        let result = f.run(&inventory::xml(&bad), Default::default()).unwrap();
        assert!(
            matches!(
                result.draft().input().gems.members[0].parameters.completion,
                DraftListCompletion::Pending { .. }
            ),
            "{bad}"
        );
    }
    let SchemaLookup::Known(gem) = f.base.schema.definition(&f.base.gem) else {
        unreachable!()
    };
    assert!(!gem.declarations.parameters.is_complete());
}
