use super::*;

fn parameter(key: &str, value: i64) -> ParameterAssignment {
    ParameterAssignment {
        slot: slot(SlotOwnerDefId::Skill(def("manual-skill")), key),
        value: ParameterValue::Integer(BoundedInteger::new(value).unwrap()),
    }
}
fn direct(raw: &mut DraftSessionInput, local: u64, value: i64) -> &mut SkillDraft {
    let row = raw
        .skills
        .members
        .iter_mut()
        .find(|row| row.id == id(local))
        .unwrap();
    row.source = AuthoredSkillSource::Direct(def("manual-skill")).into();
    row.parameters = Some(vec![parameter("raw-level", value)].into());
    row
}

#[test]
fn two_direct_occurrences_keep_independent_values_through_draft_and_preset_selection() {
    let mut raw = input();
    direct(&mut raw, 61, 12);
    direct(&mut raw, 62, 24);
    let session = DraftSession::new(raw, limits()).unwrap();
    let bytes = encode_draft(&session, limits()).unwrap();
    let restored = decode_draft(&bytes, limits()).unwrap();
    assert_eq!(encode_draft(&restored, limits()).unwrap(), bytes);
    let first = ready(&restored, selection());
    let first_skill = first
        .request()
        .build()
        .input()
        .skills
        .iter()
        .find(|s| s.id == id(61))
        .unwrap();
    assert_eq!(
        first_skill.parameters,
        Some(vec![parameter("raw-level", 12)])
    );
    assert!(
        !first
            .request()
            .build()
            .input()
            .skills
            .iter()
            .any(|s| s.id == id(62))
    );
    let mut alternate = selection();
    alternate.build.skills = id(131);
    let second = ready(&restored, alternate);
    assert_eq!(second.request().build().input().skills.len(), 1);
    assert_eq!(
        second.request().build().input().skills[0].parameters,
        Some(vec![parameter("raw-level", 24)])
    );
    assert_eq!(first.request().queries(), second.request().queries());
}

#[test]
fn pending_skill_inventory_blocks_only_its_selected_occurrence() {
    let mut raw = input();
    direct(&mut raw, 62, 24)
        .parameters
        .as_mut()
        .unwrap()
        .completion = open(990);
    let session = DraftSession::new(raw.clone(), limits()).unwrap();
    ready(&session, selection());
    let mut alternate = selection();
    alternate.build.skills = id(131);
    let DraftFinalization::Pending { issues, .. } =
        session.finalize_selection(alternate, limits()).unwrap()
    else {
        panic!("pending selected Skill input inventory")
    };
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].owner, Some(id::<SkillUseId>(62).instance_id()));
    assert!(issues[0].path.ends_with("parameters.completion"));
    direct(&mut raw, 61, 12)
        .parameters
        .as_mut()
        .unwrap()
        .members[0]
        .value = DraftField::Pending(pending(991, vec![parameter("raw-level", 12).value]));
    let session = DraftSession::new(raw, limits()).unwrap();
    let (issues, retained_queries) = pending_result(&session);
    assert_eq!(issues.len(), 1);
    assert_eq!(issues[0].id, id(991));
    assert_eq!(issues[0].owner, Some(id::<SkillUseId>(61).instance_id()));
    assert_eq!(retained_queries.to_resolved().unwrap(), queries(0));
}

#[test]
fn absent_empty_and_pending_skill_layers_remain_distinct_and_null_rejects() {
    let old = DraftSession::new(input(), limits()).unwrap();
    let old_bytes = encode_draft(&old, limits()).unwrap();
    let mut wire: serde_json::Value = serde_json::from_slice(&old_bytes).unwrap();
    for skill in wire["draft"]["skills"]["members"].as_array().unwrap() {
        assert!(skill.get("parameters").is_none());
    }
    assert_eq!(
        encode_draft(&decode_draft(&old_bytes, limits()).unwrap(), limits()).unwrap(),
        old_bytes
    );
    wire["draft"]["skills"]["members"][0]["parameters"] = serde_json::Value::Null;
    assert!(decode_draft(&serde_json::to_vec(&wire).unwrap(), limits()).is_err());
    let mut raw = input();
    direct(&mut raw, 61, 12).parameters = Some(Vec::<ParameterAssignment>::new().into());
    let complete = DraftSession::new(raw.clone(), limits()).unwrap();
    assert_ne!(encode_draft(&complete, limits()).unwrap(), old_bytes);
    assert_eq!(
        ready(&complete, selection())
            .request()
            .build()
            .input()
            .skills
            .iter()
            .find(|s| s.id == id(61))
            .unwrap()
            .parameters,
        Some(vec![])
    );
    raw.skills
        .members
        .iter_mut()
        .find(|s| s.id == id(61))
        .unwrap()
        .parameters
        .as_mut()
        .unwrap()
        .completion = open(990);
    let pending = DraftSession::new(raw, limits()).unwrap();
    let bytes = encode_draft(&pending, limits()).unwrap();
    assert_eq!(
        encode_draft(&decode_draft(&bytes, limits()).unwrap(), limits()).unwrap(),
        bytes
    );
    assert_eq!(pending_result(&pending).0.len(), 1);
}

#[test]
fn physical_gem_occurrences_reject_even_empty_or_pending_input_layers() {
    for completion in [DraftListCompletion::Complete, open(990)] {
        let mut raw = input();
        raw.skills.members[0].parameters = Some(DraftList {
            members: vec![],
            completion,
        });
        assert_eq!(
            DraftSession::new(raw, limits()).unwrap_err().kind,
            StructuralErrorKind::WrongDeclaration
        );
    }
}

#[test]
fn complete_project_composition_canonicalizes_exact_skill_parameters() {
    let mut raw = project_input();
    raw.skills[1].parameters = Some(vec![
        parameter("raw-quality", 20),
        parameter("raw-level", 12),
    ]);
    let project = BuildProject::new(raw.clone(), limits().input).unwrap();
    raw.skills[1].parameters.as_mut().unwrap().reverse();
    let reordered = BuildProject::new(raw, limits().input).unwrap();
    let first = encode_owned(
        &OwnedDocument::Project(Box::new(project.clone())),
        limits().input,
    )
    .unwrap();
    assert_eq!(
        first,
        encode_owned(&OwnedDocument::Project(Box::new(reordered)), limits().input).unwrap()
    );
    let restored = decode_owned(&first, limits().input).unwrap();
    assert_eq!(encode_owned(&restored, limits().input).unwrap(), first);
    let build = compose(&project, &selection().build, None, limits().input).unwrap();
    assert_eq!(
        build
            .input()
            .skills
            .iter()
            .find(|s| s.id == id(61))
            .unwrap()
            .parameters,
        Some(vec![
            parameter("raw-level", 12),
            parameter("raw-quality", 20)
        ])
    );
}

#[test]
fn known_draft_input_duplicates_wrong_owners_candidates_and_limits_are_checked() {
    let mut raw = input();
    let row = direct(&mut raw, 61, 12);
    let params = row.parameters.as_mut().unwrap();
    params.members.push(params.members[0].clone());
    assert_eq!(
        DraftSession::new(raw, limits()).unwrap_err().kind,
        StructuralErrorKind::DuplicateAssignment
    );
    let mut raw = input();
    let row = direct(&mut raw, 61, 12);
    row.parameters.as_mut().unwrap().members[0].slot = DraftField::Known {
        value: slot(SlotOwnerDefId::Skill(def("sibling")), "raw-level"),
    };
    assert_eq!(
        DraftSession::new(raw, limits()).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut raw = input();
    let row = direct(&mut raw, 61, 12);
    row.parameters.as_mut().unwrap().members[0].slot = DraftField::Pending(pending(
        990,
        vec![slot(SlotOwnerDefId::Gem(def("gem")), "raw-level")],
    ));
    assert_eq!(
        DraftSession::new(raw, limits()).unwrap_err().kind,
        StructuralErrorKind::WrongDeclaration
    );
    let mut raw = input();
    let row = direct(&mut raw, 61, 12);
    row.parameters = Some(
        (0..8)
            .map(|n| parameter(&format!("raw-{n}"), n))
            .collect::<Vec<_>>()
            .into(),
    );
    let session = DraftSession::new(raw, limits()).unwrap();
    let tight = DraftLimits {
        input: OwnedInputLimits {
            max_collection_entries: 7,
            ..limits().input
        },
        ..limits()
    };
    assert!(session.validate_limits(tight).is_err());
    assert!(session.finalize_selection(selection(), tight).is_err());
}
