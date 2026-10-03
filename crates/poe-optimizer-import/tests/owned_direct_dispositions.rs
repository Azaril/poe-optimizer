//! Complete intrinsic inputs still leave usage and mechanics unresolved.
#[allow(dead_code)]
#[path = "support/source_actions_fixture.rs"]
mod actions;
#[allow(dead_code)]
#[path = "support/direct_minion_actions_fixture.rs"]
mod direct_minions;
#[allow(dead_code)]
#[path = "support/direct_dispositions_fixture.rs"]
mod fixture;
#[allow(dead_code)]
#[path = "support/gem_dispositions_fixture.rs"]
mod inventory;
#[allow(dead_code)]
#[path = "support/minion_actions_fixture.rs"]
mod minions;
use fixture::Fixture;
use poe_optimizer_core::{
    build_identity::BuildLineage, owned_build::ParameterValue, owned_draft::*,
};
use poe_optimizer_import::{
    build_instance::ImportedBuildInstance, decode_build, owned_mapping::SourceComponent,
    owned_normalize::*, owned_source::SourceProjectEvidence, owned_source_actions::*,
    owned_value::*, owned_value_policy::*,
};

fn run(f: &Fixture, text: &str) -> NormalizedImport {
    f.base.run(text, Default::default()).unwrap()
}
fn complete(result: &NormalizedImport) -> Vec<bool> {
    result
        .draft()
        .input()
        .skills
        .members
        .iter()
        .map(|skill| {
            matches!(
                skill.parameters.as_ref().unwrap().completion,
                DraftListCompletion::Complete
            )
        })
        .collect()
}
fn values(skill: &SkillDraft) -> Vec<Option<f64>> {
    skill
        .parameters
        .as_ref()
        .unwrap()
        .members
        .iter()
        .map(|p| match p.value.to_resolved() {
            Some(ParameterValue::Quantity(q)) => Some(q.value()),
            None => None,
            _ => panic!("raw input must be quantity"),
        })
        .collect()
}
fn nested() -> String {
    inventory::with_children(&minions::maps(
        &minions::entry_map("1", "1"),
        &minions::entry_map("2", "2"),
    ))
}
fn pending_usage(result: &NormalizedImport) {
    for preset in &result.draft().input().skill_presets.members {
        let usage = preset.usage_preferences.as_ref().unwrap();
        assert!(
            usage.members.is_empty(),
            "source accounting is not usage evaluation"
        );
        let DraftListCompletion::Pending { id, code } = &usage.completion else {
            panic!("Direct closure cannot close usage");
        };
        assert_eq!(code.as_str(), "usage-preferences-not-converted");
        assert!(result.sidecar().origins.iter().any(|row| {
            row.links.contains(&OwnedOriginTarget::Issue(*id))
                && row
                    .links
                    .contains(&OwnedOriginTarget::SkillPreset(preset.id))
        }));
    }
}

#[test]
fn reviewed_direct_inputs_close_without_physical_gems_or_completed_usage() {
    let f = Fixture::new();
    for gem in [inventory::GEM.to_owned(), nested()] {
        let result = run(&f, &inventory::xml(&gem));
        assert_eq!(complete(&result), [true]);
        assert!(result.draft().input().gems.members.is_empty());
        let skill = &result.draft().input().skills.members[0];
        assert_eq!(values(skill), [Some(17.0), Some(0.0)]);
        assert!(matches!(skill.source, DraftAuthoredSkillSource::Direct(_)));
        assert!(!result.sidecar().origins.iter().any(|o| {
            o.links
                .iter()
                .any(|l| matches!(l, OwnedOriginTarget::Gem(_)))
        }));
        pending_usage(&result);
    }
}

#[test]
fn private_disposition_keeps_public_reference_selection_and_exact_provenance() {
    let f = Fixture::new();
    let text = inventory::xml(&nested().replace(
        "<Gem ",
        "<Gem skillMinionSkill=\"1\" skillMinionSkillCalcs=\"2\" ",
    ));
    let source = ImportedBuildInstance::from_decoded(
        decode_build(text.as_bytes()).unwrap(),
        BuildLineage::from_bytes([71; 16]),
        Default::default(),
    )
    .unwrap();
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let adapter = SourceActionCorrespondence::new(
        f.reference.clone(),
        &f.base.base.schema,
        &f.base.base.roles,
        &f.base.base.mapping,
        Default::default(),
    )
    .unwrap();
    let SourceActionCorrespondenceInput::PobManualDirectSingletonMinionActionsV1 {
        catalog_gem,
        skill,
        actions,
        ..
    } = &f.reference
    else {
        unreachable!()
    };
    let normalized = run(&f, &text);
    assert_eq!(complete(&normalized), [true]);
    let skill_use = normalized.draft().input().skills.members[0].id;
    let ordinal = evidence
        .rows()
        .iter()
        .find(|r| r.occurrence().name() == "Gem")
        .unwrap()
        .occurrence()
        .id()
        .ordinal();
    for (context, action) in [
        (ImportReferenceContext::Main, 0),
        (ImportReferenceContext::Calcs, 1),
    ] {
        let request = SourceDirectActionRequest {
            skill_use: ImportDirectSkillUseLocator {
                source_sha256: source.source_sha256().into(),
                occurrence_ordinal: ordinal,
                catalog_gem: catalog_gem.clone(),
                expected_skill: skill.clone(),
            },
            context,
        };
        let report = adapter.resolve_direct(&evidence, &request).unwrap();
        assert_eq!(
            serde_json::to_vec(&report).unwrap(),
            serde_json::to_vec(&adapter.resolve_direct(&evidence, &request).unwrap()).unwrap()
        );
        let ImportQueryTarget::DirectAction(target) = &report.target else {
            panic!("resolved public target")
        };
        assert_eq!(target.output, actions[action].output);
        let SourceActionSelection::Explicit { attribute, .. } = &report.selection else {
            panic!("explicit stat set")
        };
        let reported = report.minion.as_ref().unwrap();
        assert!(
            reported
                .accounted_occurrences
                .contains(&attribute.occurrence)
        );
        assert_eq!(reported.accounted_occurrences.len(), 4);
        for occurrence in &reported.accounted_occurrences {
            let origin = &normalized.sidecar().origins[occurrence.ordinal() as usize];
            assert_eq!(origin.source, *occurrence);
            assert_eq!(origin.links, [OwnedOriginTarget::Skill(skill_use)]);
        }
    }
    pending_usage(&normalized);
}

#[test]
fn repeated_archived_and_generated_occurrences_keep_distinct_authority() {
    let f = Fixture::new();
    let a = nested();
    let b = a
        .replace("level=\"17\"", "level=\"12.5\"")
        .replace("quality=\"0\"", "quality=\"-2.5\"");
    let text = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="2"><SkillSet id="1"><Skill enabled="false">{a}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true">{b}{a}</Skill><Skill enabled="true" source="Tree:42">{a}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = run(&f, &text);
    let draft = result.draft().input();
    assert_eq!(complete(&result), [true, true, true]);
    assert!(draft.gems.members.is_empty());
    assert_eq!(values(&draft.skills.members[0]), [Some(17.0), Some(0.0)]);
    assert_eq!(values(&draft.skills.members[1]), [Some(12.5), Some(-2.5)]);
    assert_eq!(draft.skills.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(draft.skills.members[1].enabled.to_resolved(), Some(true));
    assert_eq!(
        draft.skill_presets.members[0].skills.members,
        [draft.skills.members[0].id]
    );
    assert_eq!(
        draft.skill_presets.members[1].skills.members,
        [draft.skills.members[1].id, draft.skills.members[2].id]
    );
    pending_usage(&result);
}

#[test]
fn raw_parse_failures_remain_pending_without_closing_the_inventory() {
    let f = Fixture::new();
    for gem in [inventory::GEM.to_owned(), nested()] {
        for raw in ["bad", "nil", "NaN", "inf", "1e999", "1001"] {
            let text = inventory::xml(&gem.replace("level=\"17\"", &format!("level=\"{raw}\"")));
            let result = run(&f, &text);
            assert_eq!(complete(&result), [false], "{raw}");
            assert_eq!(
                values(&result.draft().input().skills.members[0]),
                [None, Some(0.0)]
            );
        }
    }
}

#[test]
fn unaccounted_flat_fields_do_not_close_and_unaccounted_descendants_do_not_materialize() {
    let f = Fixture::new();
    for (from, to) in [
        ("corrupted=\"false\"", "corrupted=\"true\""),
        ("corruptLevel=\"0\"", "corruptLevel=\"1\""),
        ("count=\"1\"", "count=\"bad\""),
        ("enableGlobal1=\"true\"", "enableGlobal1=\"bad\""),
        ("enabled=\"true\"", "enabled=\"bad\""),
    ] {
        let flat = run(&f, &inventory::xml(&inventory::GEM.replace(from, to)));
        assert_eq!(complete(&flat), [false], "{to}");
        let nested = run(&f, &inventory::xml(&nested().replace(from, to)));
        // Bad activation is a retained scalar Pending, not bad reference syntax.
        if from == "enabled=\"true\"" {
            assert_eq!(complete(&nested), [false]);
        } else {
            assert!(nested.draft().input().skills.members.is_empty(), "{to}");
        }
    }
    for child in [
        "<Unknown/>",
        "<MinionSkillIndexLookup grantedEffect=\"foreign\"/>",
        "<MinionSkillIndexLookup grantedEffect=\"effect\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"99\"/></MinionSkillIndexLookup>",
        "<MinionSkillIndexLookup grantedEffect=\"effect\"><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"1\"/><MinionSkillIndexMap skillIndex=\"1\" statSetIndex=\"2\"/></MinionSkillIndexLookup>",
    ] {
        let result = run(&f, &inventory::xml(&inventory::with_children(child)));
        assert!(result.draft().input().skills.members.is_empty(), "{child}");
    }
    // An unsupported nested frame must be rejected before a bad scalar creates
    // orphan obligations. NormalizedImport construction validates that invariant.
    let bad = inventory::with_children("<Unknown/>").replace("level=\"17\"", "level=\"bad\"");
    assert!(
        run(&f, &inventory::xml(&bad))
            .draft()
            .input()
            .skills
            .members
            .is_empty()
    );
}

#[test]
fn unsupported_or_ambiguous_container_frames_cannot_close_inputs() {
    let f = Fixture::new();
    for text in [
        inventory::xml(inventory::GEM)
            .replace("<Skill enabled", "<Skill label=\"unreviewed\" enabled"),
        inventory::xml(inventory::GEM)
            .replace("<Skill enabled", "<Skill mainActiveSkill=\"2\" enabled"),
        inventory::xml(inventory::GEM).replace("<Skill enabled", "<Skill unknown=\"1\" enabled"),
        inventory::xml(inventory::GEM).replace("</Skills>", "<SkillSet id=\"1\"/></Skills>"),
        inventory::xml(inventory::GEM).replace("id=\"1\"", "id=\"01\""),
        inventory::xml(inventory::GEM).replace("activeSkillSet=\"1\"", "activeSkillSet=\"2\""),
    ] {
        assert!(!complete(&run(&f, &text)).contains(&true), "{text}");
    }
}

#[test]
fn invalid_disposition_recipes_and_guards_are_rejected_as_policy_errors() {
    for case in 0..9 {
        let mut f = Fixture::new();
        let row = f.row();
        match case {
            0 => row.deferred_usage.clear(),
            1 => row.deferred_usage[1] = row.deferred_usage[0].clone(),
            2 => row.deferred_usage[0].value.missing = MissingValuePolicy::Absent,
            3 => {
                row.deferred_usage[0].value.codec.codec = ValueCodecKind::Integer {
                    syntax: DecimalSyntax::Integer,
                }
            }
            4 => row.inert_fields[0].allowed.clear(),
            5 => row.inert_fields.push(row.inert_fields[0].clone()),
            6 => row.inert_fields[0].attribute = "level".into(),
            7 => row.group_guards[0].attribute = "count".into(),
            8 => {
                row.group_guards[0].allowed =
                    vec![SourceComponent::Missing, SourceComponent::Missing]
            }
            _ => unreachable!(),
        }
        assert!(
            matches!(
                f.base
                    .run(&inventory::xml(inventory::GEM), Default::default()),
                Err(NormalizationError::Policy(_))
            ),
            "case {case}"
        );
    }
}

#[test]
fn v2_requires_declared_required_raw_inputs_while_v1_keeps_subset_pending() {
    let mut f = Fixture::new();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { skills, .. } =
        f.base.policy.direct_skill_inputs.as_mut().unwrap()
    else {
        unreachable!()
    };
    // The schema still declares both required authored inputs. The source also
    // still contains both; dropping a recipe cannot certify their inventory.
    skills[0].parameters.pop().unwrap();
    let text = inventory::xml(inventory::GEM);
    let error = f.base.run(&text, Default::default()).unwrap_err();
    assert!(matches!(&error, NormalizationError::Policy(_)));
    assert!(
        error
            .to_string()
            .contains("missing required authored input")
    );

    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
        ..
    } = f.base.policy.direct_skill_inputs.take().unwrap()
    else {
        unreachable!()
    };
    f.base.policy.direct_skill_inputs = Some(DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
    });
    let result = run(&f, &text);
    assert_eq!(complete(&result), [false]);
    assert_eq!(
        values(&result.draft().input().skills.members[0]),
        [Some(17.0)]
    );
}

#[test]
fn direct_v1_contract_retains_raw_inputs_and_does_not_admit_nested_maps() {
    let mut f = Fixture::new();
    let next = run(&f, &inventory::xml(inventory::GEM));
    let DirectSkillInputPolicy::PobManualDirectSkillV2 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
        ..
    } = f.base.policy.direct_skill_inputs.take().unwrap()
    else {
        unreachable!()
    };
    f.base.policy.direct_skill_inputs = Some(DirectSkillInputPolicy::PobManualDirectSkillV1 {
        definitions,
        source,
        roles,
        catalog,
        manual_sources,
        group_attributes,
        skills,
    });
    let old = run(&f, &inventory::xml(inventory::GEM));
    assert_eq!(complete(&old), [false]);
    let mut expected = old.draft().input().skills.clone();
    expected.members[0].parameters.as_mut().unwrap().completion = DraftListCompletion::Complete;
    assert_eq!(
        expected,
        next.draft().input().skills,
        "same occurrence ID, raw inputs and activation"
    );
    assert!(
        run(&f, &inventory::xml(&nested()))
            .draft()
            .input()
            .skills
            .members
            .is_empty()
    );
}

#[test]
fn work_exhaustion_is_a_hard_error_and_fresh_retry_recovers() {
    let f = Fixture::new();
    let text = inventory::xml(&nested().repeat(3));
    let baseline = run(&f, &text);
    let mut low = 1;
    let mut high = NormalizationLimits::default().max_work;
    while high - low > 1 {
        let mid = low + (high - low) / 2;
        if f.base
            .run(
                &text,
                NormalizationLimits {
                    max_work: mid,
                    ..Default::default()
                },
            )
            .is_ok()
        {
            high = mid;
        } else {
            low = mid;
        }
    }
    assert!(
        f.base
            .run(
                &text,
                NormalizationLimits {
                    max_work: low,
                    ..Default::default()
                }
            )
            .is_err()
    );
    let exact = f
        .base
        .run(
            &text,
            NormalizationLimits {
                max_work: high,
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(baseline.draft().input(), exact.draft().input());
    assert_eq!(baseline.draft().input(), run(&f, &text).draft().input());
}
