//! Saved reference selections are query correspondence, never gameplay inputs.
#[path = "support/source_actions_fixture.rs"]
mod fixture;
use fixture::*;
use poe_optimizer_core::build_identity::BuildLineage;
use poe_optimizer_import::{
    build_instance::{ImportedBuildInstance, InstanceImportLimits},
    decode_build,
    owned_normalize::{ImportActorTarget, ImportQueryTarget},
    owned_source::{SourceEvidenceLimits, SourceProjectEvidence},
    owned_source_actions::*,
    owned_value_policy::*,
};

fn source(xml: &str) -> ImportedBuildInstance {
    ImportedBuildInstance::from_decoded(
        decode_build(xml.as_bytes()).unwrap(),
        BuildLineage::from_bytes([42; 16]),
        InstanceImportLimits::default(),
    )
    .unwrap()
}
fn report(
    f: &Fixture,
    xml: &str,
    ordinal: usize,
    context: ImportReferenceContext,
) -> SourceActionReport {
    let source = source(xml);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let request = f.request(&source, ordinal, context);
    f.compile(Default::default())
        .unwrap()
        .resolve(&evidence, &request)
        .unwrap()
}
fn stat_set(
    report: &SourceActionReport,
) -> &poe_optimizer_core::owned_definitions::ActionStatSetDefId {
    let ImportQueryTarget::Action(target) = &report.target else {
        panic!("unresolved: {:?}", report.selection);
    };
    &target.stat_set
}
fn pending(report: &SourceActionReport, expected: &str) {
    let SourceActionSelection::Pending { code } = &report.selection else {
        panic!("expected pending");
    };
    assert_eq!(code.as_str(), expected);
    assert!(matches!(&report.target, ImportQueryTarget::Unresolved(target) if target == code));
}

#[test]
fn two_contexts_and_physical_occurrences_keep_independent_owned_query_targets() {
    let f = Fixture::new();
    let xml = xml(&format!(
        "{}{}",
        gem("", &maps("effect", "2", "1")),
        gem(" enabled=\"false\"", &maps("effect", "1", "2"))
    ));
    let source = source(&xml);
    let before = serde_json::to_vec(&f.input).unwrap();
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    let adapter = f.compile(Default::default()).unwrap();
    for (occurrence, context, expected) in [
        (0, ImportReferenceContext::Main, 1),
        (0, ImportReferenceContext::Calcs, 0),
        (1, ImportReferenceContext::Main, 0),
        (1, ImportReferenceContext::Calcs, 1),
    ] {
        let request = f.request(&source, occurrence, context);
        let report = adapter.resolve(&evidence, &request).unwrap();
        assert_eq!(stat_set(&report), &f.sets[expected]);
        let ImportQueryTarget::Action(target) = &report.target else {
            unreachable!()
        };
        assert_eq!(target.provider.skill_use, request.skill_use);
        assert_eq!(target.provider.grant_path, std::slice::from_ref(&f.grant));
        assert_eq!(target.actor, ImportActorTarget::Player);
        let SourceActionSelection::Explicit {
            attribute,
            source_index,
        } = report.selection
        else {
            panic!("expected exact source selection");
        };
        assert_eq!(source_index, expected as u32 + 1);
        assert_eq!(
            evidence.rows()[attribute.occurrence.ordinal() as usize]
                .occurrence()
                .parent()
                .unwrap()
                .ordinal(),
            request.skill_use.occurrence_ordinal
        );
    }
    assert_eq!(serde_json::to_vec(adapter.input()).unwrap(), before);
    assert_eq!(source.source_sha256(), evidence.identity().source_sha256);
}

#[test]
fn explicit_absence_is_source_bound_and_overwritten_scalar_headers_never_select() {
    let mut f = Fixture::new();
    for children in ["".to_owned(), maps("unrelated", "2", "2")] {
        let xml = xml(&gem(
            " statSetIndex=\"2\" statSetIndexCalcs=\"99\"",
            &children,
        ));
        for context in [ImportReferenceContext::Main, ImportReferenceContext::Calcs] {
            let result = report(&f, &xml, 0, context);
            assert_eq!(stat_set(&result), &f.sets[0]);
            assert_eq!(result.selection, SourceActionSelection::Absent);
            assert_eq!(result.ignored_legacy_attributes.len(), 2);
        }
    }
    let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
        absent_stat_set, ..
    } = &mut f.input;
    *absent_stat_set = None;
    pending(
        &report(
            &f,
            &xml(&gem(" statSetIndex=\"2\"", "")),
            0,
            ImportReferenceContext::Main,
        ),
        "query-source-selection-absent",
    );
}

#[test]
fn present_invalid_or_duplicate_rows_never_become_absence_or_last_wins() {
    let f = Fixture::new();
    for token in [
        "",
        "bad",
        "nil",
        " 1",
        "1 ",
        "0",
        "-1",
        "1.5",
        "3",
        "4294967296",
    ] {
        let xml = xml(&gem("", &maps("effect", token, "2")));
        let main = report(&f, &xml, 0, ImportReferenceContext::Main);
        assert!(
            matches!(main.selection, SourceActionSelection::Pending { .. }),
            "{token:?}"
        );
        assert_eq!(
            stat_set(&report(&f, &xml, 0, ImportReferenceContext::Calcs)),
            &f.sets[1]
        );
    }
    for children in [
        "<StatSetIndex grantedEffect=\"effect\"/>".into(),
        format!("{}{}", maps("effect", "1", "1"), maps("effect", "2", "2")),
        format!("{}{}", maps("effect", "2", "2"), maps("effect", "1", "1")),
    ] {
        assert!(matches!(
            report(
                &f,
                &xml(&gem("", &children)),
                0,
                ImportReferenceContext::Main
            )
            .target,
            ImportQueryTarget::Unresolved(_)
        ));
    }
}

#[test]
fn unknown_namespace_maps_and_wrong_source_identity_remain_unresolved() {
    let f = Fixture::new();
    for children in [
        "<Future/>",
        "<StatSetIndex grantedEffect=\"effect\" index=\"2\" future=\"yes\"/>",
        "<StatSetIndex xmlns=\"urn:foreign\" grantedEffect=\"effect\" index=\"2\"/>",
        "<StatSetIndex grantedEffect=\"effect\" index=\"2\"><Future/></StatSetIndex>",
        "<StatSetIndex index=\"2\"/>",
        "<MinionSkillIndexLookup grantedEffect=\"effect\"/>",
    ] {
        assert!(
            matches!(
                report(
                    &f,
                    &xml(&gem("", children)),
                    0,
                    ImportReferenceContext::Main
                )
                .target,
                ImportQueryTarget::Unresolved(_)
            ),
            "{children}"
        );
    }
    let text = xml(&gem("", ""));
    for changed in [
        text.replace("gemId=\"physical\"", "gemId=\"different\""),
        text.replace("skillId=\"effect\"", "skillId=\"alias\""),
        text.replace("<Gem ", "<Gem xmlns=\"urn:foreign\" "),
    ] {
        assert!(matches!(
            report(&f, &changed, 0, ImportReferenceContext::Main).target,
            ImportQueryTarget::Unresolved(_)
        ));
    }
    let source = source(&text);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let adapter = f.compile(Default::default()).unwrap();
    let mut request = f.request(&source, 0, ImportReferenceContext::Main);
    request.skill_use.source_sha256 = "0".repeat(64);
    pending(
        &adapter.resolve(&evidence, &request).unwrap(),
        "query-source-snapshot-mismatch",
    );
    request = f.request(&source, 0, ImportReferenceContext::Main);
    request.skill_use.occurrence_ordinal = u32::MAX;
    pending(
        &adapter.resolve(&evidence, &request).unwrap(),
        "query-source-occurrence-missing",
    );
}

#[test]
fn duplicate_xml_attributes_are_rejected_at_intake_without_a_fallback_report() {
    let text = xml(&gem(
        "",
        "<StatSetIndex grantedEffect=\"effect\" index=\"1\" index=\"2\"/>",
    ));
    assert!(decode_build(text.as_bytes()).is_err());
}

#[test]
fn archived_and_disabled_queries_resolve_without_proving_activation() {
    let f = Fixture::new();
    let text = format!(
        "<PathOfBuilding2><Skills activeSkillSet=\"2\"><SkillSet id=\"1\"><Skill enabled=\"false\">{}</Skill></SkillSet><SkillSet id=\"2\"><Skill>{}</Skill></SkillSet></Skills></PathOfBuilding2>",
        gem(" enabled=\"false\"", &maps("effect", "2", "1")),
        gem("", "")
    );
    assert_eq!(
        stat_set(&report(&f, &text, 0, ImportReferenceContext::Main)),
        &f.sets[1]
    );
    assert_eq!(
        stat_set(&report(&f, &text, 1, ImportReferenceContext::Main)),
        &f.sets[0]
    );
}

#[test]
fn checked_bindings_reject_stale_foreign_and_undeclared_correspondences() {
    for change in 0..10 {
        let mut f = Fixture::new();
        let SourceActionCorrespondenceInput::PobPhysicalPrimaryStatSetsV1 {
            definitions,
            source,
            roles,
            catalog,
            skill_id,
            primary_supply,
            stat_sets,
            absent_stat_set,
            index,
            ..
        } = &mut f.input;
        match change {
            0 => definitions.release.push_str("-stale"),
            1 => source.files[0].sha256 = "f".repeat(64),
            2 => *roles = "f".repeat(64).parse().unwrap(),
            3 => *catalog = "f".repeat(64).parse().unwrap(),
            4 => *skill_id = "unmapped".into(),
            5 => {
                primary_supply.declaration =
                    poe_optimizer_core::owned_definitions::SlotOwnerDefId::Skill(
                        poe_optimizer_core::owned_definitions::SkillDefId::new(
                            ns(),
                            key("wrong-owner"),
                        ),
                    )
            }
            6 => stat_sets[1].source_index = stat_sets[0].source_index,
            7 => {
                *absent_stat_set = Some(
                    poe_optimizer_core::owned_definitions::ActionStatSetDefId::new(
                        ns(),
                        key("not-declared"),
                    ),
                )
            }
            8 => index.missing = MissingValuePolicy::Absent,
            9 => index.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
            _ => unreachable!(),
        }
        assert!(f.compile(Default::default()).is_err(), "change{change}");
    }
}

#[test]
fn bounds_cover_source_rows_codec_work_and_atomic_report_generation() {
    let f = Fixture::new();
    let text = xml(&gem("", &maps("effect", "2", "1")));
    let source = source(&text);
    let evidence = SourceProjectEvidence::collect(&source, Default::default()).unwrap();
    let request = f.request(&source, 0, ImportReferenceContext::Main);
    let first = f
        .compile(Default::default())
        .unwrap()
        .resolve(&evidence, &request)
        .unwrap();
    let exact = SourceActionLimits {
        max_work: first.work,
        ..Default::default()
    };
    assert_eq!(
        f.compile(exact)
            .unwrap()
            .resolve(&evidence, &request)
            .unwrap(),
        first
    );
    let tighter = SourceActionLimits {
        max_work: first.work - 1,
        ..Default::default()
    };
    assert!(matches!(
        f.compile(tighter).unwrap().resolve(&evidence, &request),
        Err(SourceActionError::Limit("work"))
    ));
    let limits = SourceActionLimits {
        max_map_rows: 1,
        ..Default::default()
    };
    assert!(matches!(
        f.compile(limits).unwrap().resolve(&evidence, &request),
        Err(SourceActionError::Limit("map rows"))
    ));
    let limits = SourceActionLimits {
        max_output_bytes: 1,
        ..Default::default()
    };
    assert!(
        f.compile(limits)
            .unwrap()
            .resolve(&evidence, &request)
            .is_err()
    );
    let mut limits = SourceActionLimits::default();
    limits.value.max_total_candidate_bytes = 5; // exact "index" + one token needs 6.
    assert!(matches!(
        f.compile(limits).unwrap().resolve(&evidence, &request),
        Err(SourceActionError::Value(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::TotalCandidateBytes,
            ..
        }))
    ));
    let limits = SourceActionLimits {
        max_work: SourceActionLimits::default().max_work + 1,
        ..Default::default()
    };
    assert!(matches!(
        f.compile(limits),
        Err(SourceActionError::InvalidLimit("work"))
    ));
    assert_eq!(
        f.compile(Default::default())
            .unwrap()
            .resolve(&evidence, &request)
            .unwrap(),
        first
    );
}

#[test]
fn adapter_and_request_wire_are_strict_and_the_existing_query_shape_is_unchanged() {
    let f = Fixture::new();
    let wire = serde_json::to_value(&f.input).unwrap();
    let decoded: SourceActionCorrespondenceInput = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(decoded, f.input);
    let mut unknown = wire;
    unknown["future"] = true.into();
    assert!(serde_json::from_value::<SourceActionCorrespondenceInput>(unknown).is_err());
    let result = report(&f, &xml(&gem("", "")), 0, ImportReferenceContext::Main);
    let target = serde_json::to_value(&result.target).unwrap();
    assert_eq!(target["kind"], "action");
    assert!(target.to_string().find("main").is_none());
    assert_eq!(
        serde_json::from_value::<ImportQueryTarget>(target).unwrap(),
        result.target
    );
}
