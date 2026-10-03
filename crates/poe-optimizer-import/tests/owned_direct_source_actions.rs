#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
pub(crate) mod actions;
#[allow(dead_code)]
#[path = "support/direct_minion_actions_fixture.rs"]
pub(crate) mod direct_minions;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
pub(crate) mod inventory;
#[allow(dead_code)]
#[path = "support/minion_actions_fixture.rs"]
pub(crate) mod minions;
#[allow(dead_code)]
#[path = "support/direct_dispositions_fixture.rs"]
mod normalization;
use direct_minions::Fixture;
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::*, owned_definitions::*, owned_draft::*,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_mapping::*, owned_normalize::*,
    owned_skill_catalog::*, owned_source::SourceProjectEvidence, owned_source_actions::*,
};

fn source(text: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(text.as_bytes()).unwrap(),
        BuildLineage::from_bytes([65; 16]),
        Default::default(),
    )
    .unwrap()
}
fn report(f: &Fixture, text: &str, context: ImportReferenceContext) -> SourceDirectActionReport {
    let source = source(text);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    f.compile(Default::default())
        .unwrap()
        .resolve_direct(&evidence, &f.request(&source, 0, context))
        .unwrap()
}
fn target(report: &SourceDirectActionReport) -> &ImportActionTarget<ImportDirectSkillUseLocator> {
    let ImportQueryTarget::DirectAction(target) = &report.target else {
        panic!("expected resolved Direct target: {:?}", report.selection)
    };
    target
}

#[test]
fn manual_direct_contexts_use_exact_owned_actor_paths_without_a_gem_edge() {
    let f = Fixture::new();
    let text = actions::xml(&actions::gem(
        " skillMinion=\"fixture-minion\" skillMinionCalcs=\"fixture-minion\" skillMinionSkill=\"2\" skillMinionSkillCalcs=\"1\"",
        &minions::maps(&minions::entry_map("2", "2"), &minions::entry_map("1", "1")),
    ));
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { minion, .. } =
        &f.input
    else {
        unreachable!()
    };
    for (context, child, set) in [
        (ImportReferenceContext::Main, 1, 1),
        (ImportReferenceContext::Calcs, 0, 0),
    ] {
        let result = report(&f, &text, context);
        let target = target(&result);
        assert_eq!(target.provider.skill_use.expected_skill, *f.skill());
        assert_eq!(target.provider.skill_use.catalog_gem, f.base.gem);
        assert_eq!(
            target.provider.grant_path,
            vec![
                minion.entering_grant.clone(),
                f.actions()[child].entering_grant.clone()
            ]
        );
        assert_eq!(target.output, f.actions()[child].output);
        assert_eq!(target.stat_set, f.base.sets[set]);
        let ImportActorTarget::Owned { provider, slot } = &target.actor else {
            panic!("expected owned actor")
        };
        assert!(provider.grant_path.is_empty());
        assert_eq!(*slot, minion.population);
        assert_eq!(result.minion.unwrap().accounted_occurrences.len(), 4);
    }
}

#[test]
fn direct_and_physical_wire_and_resolver_authorities_are_distinct() {
    let f = Fixture::new();
    let source = source(&actions::xml(&actions::gem("", "")));
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let direct = f.request(&source, 0, ImportReferenceContext::Main);
    let physical = f.base.request(&source, 0, ImportReferenceContext::Main);
    let bytes = serde_json::to_vec(&direct).unwrap();
    assert_eq!(
        serde_json::from_slice::<SourceDirectActionRequest>(&bytes).unwrap(),
        direct
    );
    assert!(serde_json::from_slice::<SourceActionRequest>(&bytes).is_err());
    let bytes = serde_json::to_vec(&physical).unwrap();
    assert_eq!(
        serde_json::from_slice::<SourceActionRequest>(&bytes).unwrap(),
        physical
    );
    assert!(serde_json::from_slice::<SourceDirectActionRequest>(&bytes).is_err());
    assert!(
        f.compile(Default::default())
            .unwrap()
            .resolve(&evidence, &physical)
            .is_err()
    );
    let old = minions::Fixture::new();
    assert!(
        old.compile(Default::default())
            .unwrap()
            .resolve_direct(&evidence, &direct)
            .is_err()
    );
    let result = f
        .compile(Default::default())
        .unwrap()
        .resolve_direct(&evidence, &direct)
        .unwrap();
    let wire = serde_json::to_value(&result.target).unwrap();
    assert_eq!(wire["kind"], "direct_action");
    let old_result = old
        .compile(Default::default())
        .unwrap()
        .resolve(&evidence, &physical)
        .unwrap();
    let old_wire = serde_json::to_value(&old_result.target).unwrap();
    assert_eq!(old_wire["kind"], "action");
    assert!(
        !serde_json::to_string(&old_wire)
            .unwrap()
            .contains("catalog_gem")
    );
    let mut input = serde_json::to_value(&f.input).unwrap();
    input["primary_supply"] = serde_json::json!({});
    assert!(serde_json::from_value::<SourceActionCorrespondenceInput>(input).is_err());
}

#[test]
fn exact_manual_source_is_required_while_disabled_and_archived_references_survive() {
    let f = Fixture::new();
    let gem = actions::gem(" enabled=\"false\"", "");
    let text = format!(
        "<PathOfBuilding2><Skills activeSkillSet=\"2\"><SkillSet id=\"1\"><Skill source=\"\" enabled=\"false\">{gem}</Skill></SkillSet><SkillSet id=\"2\"/></Skills></PathOfBuilding2>"
    );
    assert_eq!(
        target(&report(&f, &text, ImportReferenceContext::Main)).output,
        f.actions()[0].output
    );
    for replacement in ["Tree:1", "Item:1", "manual", " "] {
        let invalid = text.replace("source=\"\"", &format!("source=\"{replacement}\""));
        assert!(matches!(
            report(&f, &invalid, ImportReferenceContext::Main).target,
            ImportQueryTarget::Unresolved(_)
        ));
    }
    let namespaced = text.replace("<Skill source", "<Skill xmlns=\"urn:foreign\" source");
    assert!(matches!(
        report(&f, &namespaced, ImportReferenceContext::Main).target,
        ImportQueryTarget::Unresolved(_)
    ));
    let source = source(&text);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let compiled = f.compile(Default::default()).unwrap();
    let request = f.request(&source, 0, ImportReferenceContext::Main);
    let mut wrong = request.clone();
    wrong.skill_use.expected_skill = f.actions()[0].skill.clone();
    assert!(matches!(
        compiled.resolve_direct(&evidence, &wrong).unwrap().target,
        ImportQueryTarget::Unresolved(_)
    ));
    wrong = request.clone();
    wrong.skill_use.source_sha256 = "0".repeat(64);
    assert!(matches!(
        compiled.resolve_direct(&evidence, &wrong).unwrap().target,
        ImportQueryTarget::Unresolved(_)
    ));
    wrong = request;
    wrong.skill_use.occurrence_ordinal = 0;
    assert!(matches!(
        compiled.resolve_direct(&evidence, &wrong).unwrap().target,
        ImportQueryTarget::Unresolved(_)
    ));
}

#[test]
fn both_actor_branches_and_every_nested_map_require_reviewed_correspondence() {
    let f = Fixture::new();
    for attribute in ["skillMinion", "skillMinionCalcs"] {
        let text = actions::xml(&actions::gem(
            &format!(" {attribute}=\"unreviewed-actor\""),
            "",
        ));
        assert!(matches!(
            report(&f, &text, ImportReferenceContext::Calcs).target,
            ImportQueryTarget::Unresolved(_)
        ));
    }
    for token in ["", "0", "-1", "1.5", "3", "nil", "NaN", " 1", "1 "] {
        for (attribute, context) in [
            ("skillMinionSkill", ImportReferenceContext::Main),
            ("skillMinionSkillCalcs", ImportReferenceContext::Calcs),
        ] {
            let text = actions::xml(&actions::gem(&format!(" {attribute}=\"{token}\""), ""));
            assert!(
                matches!(
                    report(&f, &text, context).target,
                    ImportQueryTarget::Unresolved(_)
                ),
                "{attribute}={token:?}"
            );
        }
    }
    for children in [
        minions::maps(&minions::entry_map("3", "1"), ""),
        minions::maps(&minions::entry_map("2", "99"), ""),
        minions::maps(
            &format!(
                "{}{}",
                minions::entry_map("2", "1"),
                minions::entry_map("02", "2")
            ),
            "",
        ),
        minions::maps("", &minions::entry_map("2", "bad")),
        minions::maps(&minions::entry_map("1", "1"), "")
            .replace("grantedEffect=\"effect\"", "grantedEffect=\"foreign\""),
        "<Unreviewed/>".into(),
    ] {
        let text = actions::xml(&actions::gem("", &children));
        assert!(
            matches!(
                report(&f, &text, ImportReferenceContext::Main).target,
                ImportQueryTarget::Unresolved(_)
            ),
            "unselected fields cannot disappear: {children}"
        );
    }
    let mut no_absence = Fixture::new();
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { minion, .. } =
        &mut no_absence.input
    else {
        unreachable!()
    };
    minion.allow_absent = false;
    assert!(matches!(
        report(
            &no_absence,
            &actions::xml(&actions::gem("", "")),
            ImportReferenceContext::Main
        )
        .target,
        ImportQueryTarget::Unresolved(_)
    ));
}

#[test]
fn direct_catalog_role_dependencies_and_exact_actor_topology_are_checked() {
    let mut f = Fixture::new();
    let mut roles = f.base.roles.input().clone();
    roles.roles[0].materialization = OwnedGemMaterialization::Physical;
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 { roles, .. } =
        &mut f.input
    else {
        unreachable!()
    };
    *roles = *f.base.roles.identity();
    assert!(matches!(
        f.compile(Default::default()),
        Err(SourceActionError::Policy("direct primary role"))
    ));
    for change in 0..5 {
        let mut f = Fixture::new();
        let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
            source,
            manual_sources,
            minion,
            actions,
            ..
        } = &mut f.input
        else {
            unreachable!()
        };
        match change {
            0 => source.revision = "d".repeat(40),
            1 => manual_sources.clear(),
            2 => manual_sources.push(SourceComponent::Missing),
            3 => minion.entering_grant = actions[0].entering_grant.clone(),
            4 => actions[0].output = actions[1].output.clone(),
            _ => unreachable!(),
        }
        assert!(
            f.compile(Default::default()).is_err(),
            "invalid correspondence {change}"
        );
    }
}

#[test]
fn direct_resolution_has_one_bounded_budget_and_no_failure_state() {
    let f = Fixture::new();
    let text = actions::xml(&actions::gem(
        "",
        &minions::maps(&minions::entry_map("1", "2"), &minions::entry_map("2", "1")),
    ));
    let source = source(&text);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let request = f.request(&source, 0, ImportReferenceContext::Main);
    let full = f
        .compile(Default::default())
        .unwrap()
        .resolve_direct(&evidence, &request)
        .unwrap();
    let exact = f
        .compile(SourceActionLimits {
            max_work: full.work,
            ..Default::default()
        })
        .unwrap();
    assert_eq!(exact.resolve_direct(&evidence, &request).unwrap(), full);
    let tight = f
        .compile(SourceActionLimits {
            max_work: full.work - 1,
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        tight.resolve_direct(&evidence, &request),
        Err(SourceActionError::Limit("work"))
    ));
    let maps = f
        .compile(SourceActionLimits {
            max_map_rows: 2,
            ..Default::default()
        })
        .unwrap();
    assert!(matches!(
        maps.resolve_direct(&evidence, &request),
        Err(SourceActionError::Limit("map rows"))
    ));
    let short = f
        .compile(SourceActionLimits {
            max_output_bytes: serde_json::to_vec(&full).unwrap().len() - 1,
            ..Default::default()
        })
        .unwrap();
    assert!(short.resolve_direct(&evidence, &request).is_err());
    assert!(
        f.compile(SourceActionLimits {
            max_wire_bytes: 1,
            ..Default::default()
        })
        .is_err()
    );
    assert!(
        f.compile(SourceActionLimits {
            max_work: SourceActionLimits::default().max_work + 1,
            ..Default::default()
        })
        .is_err()
    );
    assert_eq!(exact.resolve_direct(&evidence, &request).unwrap(), full);
}

fn query(id: &str, target: ImportQueryTarget) -> ImportQueryTemplate {
    ImportQueryTemplate {
        id: QueryId::new(id).unwrap(),
        metric: ExternalSelector::Catalog {
            kind: ExternalCatalogKind::Metric,
            key: SourceComponent::Text("unreviewed-metric".into()),
            version: SourceComponent::Missing,
            variant: SourceComponent::Missing,
        },
        target,
    }
}
fn queries(result: &NormalizedImport) -> &[MetricRequestDraft] {
    &result.draft().input().query_presets.members[0]
        .queries
        .requests
        .members
}
fn direct_target(
    f: &normalization::Fixture,
    text: &str,
    nth: usize,
) -> ImportActionTarget<ImportDirectSkillUseLocator> {
    let source = source(text);
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
        catalog_gem,
        skill,
        ..
    } = &f.reference
    else {
        unreachable!()
    };
    let request = SourceActionRequest {
        skill_use: ImportDirectSkillUseLocator {
            source_sha256: source.source_sha256().into(),
            occurrence_ordinal: source
                .occurrences()
                .iter()
                .filter(|r| r.name() == "Gem")
                .nth(nth)
                .unwrap()
                .id()
                .ordinal(),
            catalog_gem: catalog_gem.clone(),
            expected_skill: skill.clone(),
        },
        context: ImportReferenceContext::Main,
    };
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let adapter = SourceActionCorrespondence::new(
        f.reference.clone(),
        &f.base.base.schema,
        &f.base.base.roles,
        &f.base.base.mapping,
        Default::default(),
    )
    .unwrap();
    let result = adapter.resolve_direct(&evidence, &request).unwrap();
    target(&result).clone()
}

#[test]
fn normalized_direct_queries_use_exact_materialized_occurrences_and_preserve_paths() {
    let f = normalization::Fixture::new();
    let gem = inventory::GEM.replace("enabled=\"true\"", "enabled=\"false\"");
    let text = format!(
        "<PathOfBuilding2><Build level=\"70\"/><Skills activeSkillSet=\"2\"><SkillSet id=\"1\"><Skill enabled=\"false\">{gem}</Skill></SkillSet><SkillSet id=\"2\"><Skill enabled=\"true\">{}{}</Skill></SkillSet></Skills></PathOfBuilding2>",
        inventory::GEM,
        inventory::GEM
    );
    let first = direct_target(&f, &text, 0);
    let second = direct_target(&f, &text, 1);
    let third = direct_target(&f, &text, 2);
    let targets = [&first, &second, &third];
    let requests = targets
        .iter()
        .enumerate()
        .map(|(n, target)| {
            query(
                &format!("direct-{n}"),
                ImportQueryTarget::DirectAction(Box::new((*target).clone())),
            )
        })
        .collect::<Vec<_>>();
    let result = f
        .base
        .run_with_queries(&text, &requests, Default::default())
        .unwrap();
    assert!(
        result.draft().input().gems.members.is_empty(),
        "catalog identities must not fabricate physical Gems"
    );
    assert_eq!(result.draft().input().skills.members.len(), 3);
    let mut ids = std::collections::BTreeSet::new();
    for (n, expected) in targets.iter().enumerate() {
        let request = &queries(&result)[n];
        assert!(
            matches!(request.metric, DraftField::Pending(_)),
            "query selection proves no numerical metric"
        );
        let Some(MetricTarget::Action(action)) = request.target.to_resolved() else {
            panic!("missing materialized Direct target")
        };
        let skill = result.draft().input().skills.members[n].id;
        ids.insert(skill);
        assert_eq!(action.action.provider.root, ProviderRoot::SkillUse(skill));
        assert_eq!(
            action.action.provider.grant_path,
            expected.provider.grant_path
        );
        assert_eq!(action.action.output, expected.output);
        let ActorKey::Owned(actor) = action.action.actor else {
            panic!("missing owned actor")
        };
        assert_eq!(actor.provider.root, ProviderRoot::SkillUse(skill));
        assert!(actor.provider.grant_path.is_empty());
        let origin =
            &result.sidecar().origins[expected.provider.skill_use.occurrence_ordinal as usize];
        assert_eq!(
            origin
                .links
                .iter()
                .filter(|link| matches!(link, OwnedOriginTarget::QueryPreset(_)))
                .count(),
            1
        );
    }
    assert_eq!(
        ids.len(),
        3,
        "identical and archived rows keep occurrence identity"
    );
    assert!(
        result
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .all(|preset| matches!(
                &preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Pending { .. }
            ))
    );
}

#[test]
fn normalized_direct_query_never_substitutes_physical_or_unmaterialized_sources() {
    let mut f = normalization::Fixture::new();
    let text = inventory::xml(inventory::GEM);
    let target = direct_target(&f, &text, 0);
    let mut wrong_skill = target.clone();
    wrong_skill.provider.skill_use.expected_skill =
        SkillDefId::new(actions::ns(), actions::key("wrong-skill"));
    let mut wrong_catalog = target.clone();
    wrong_catalog.provider.skill_use.catalog_gem =
        GemDefId::new(actions::ns(), actions::key("wrong-catalog"));
    let physical = ImportActionTarget {
        provider: ImportProviderTarget {
            skill_use: ImportSkillUseLocator {
                source_sha256: target.provider.skill_use.source_sha256.clone(),
                occurrence_ordinal: target.provider.skill_use.occurrence_ordinal,
                expected_gem: target.provider.skill_use.catalog_gem.clone(),
            },
            grant_path: target.provider.grant_path.clone(),
        },
        actor: ImportActorTarget::Player,
        output: target.output.clone(),
        part: target.part.clone(),
        mode: target.mode.clone(),
        stat_set: target.stat_set.clone(),
    };
    let requests = [
        query(
            "wrong-skill",
            ImportQueryTarget::DirectAction(Box::new(wrong_skill)),
        ),
        query(
            "wrong-catalog",
            ImportQueryTarget::DirectAction(Box::new(wrong_catalog)),
        ),
        query("physical", ImportQueryTarget::Action(Box::new(physical))),
    ];
    let result = f
        .base
        .run_with_queries(&text, &requests, Default::default())
        .unwrap();
    assert!(
        queries(&result)
            .iter()
            .all(|query| matches!(query.target, DraftMetricTarget::Pending(_)))
    );
    let source_origin =
        &result.sidecar().origins[target.provider.skill_use.occurrence_ordinal as usize];
    assert!(
        !source_origin
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::QueryPreset(_)))
    );
    f.base.policy.direct_skill_inputs = None;
    let result = f
        .base
        .run_with_queries(
            &text,
            &[query(
                "unmaterialized",
                ImportQueryTarget::DirectAction(Box::new(target)),
            )],
            Default::default(),
        )
        .unwrap();
    assert!(result.draft().input().skills.members.is_empty());
    assert!(matches!(
        queries(&result)[0].target,
        DraftMetricTarget::Pending(_)
    ));
}
