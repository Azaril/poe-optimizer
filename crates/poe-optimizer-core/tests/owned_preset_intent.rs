//! Data-aware preset laws, independent of game catalogs and source formats.
use poe_optimizer_core::{
    build_identity::*, data::DataIdentity, owned_binding::*, owned_build::*, owned_definitions::*,
    owned_draft::*, owned_preset_intent::*, owned_project::*, owned_schema::*,
};
use std::collections::BTreeMap;
#[path = "support/owned_preset_intent_fixture.rs"]
mod fixture;
use fixture::compose;
use fixture::*;
fn project(raw: ProjectInput) -> BuildProject {
    BuildProject::new(raw, OwnedInputLimits::default()).unwrap()
}
fn proof(index: &Index, p: &BuildProject) -> ProjectIntentProof {
    prove_project_intent(index, p, BindingLimits::default()).unwrap()
}

#[test]
fn exact_selection_is_not_activation_and_a_b_a_is_stable() {
    let index = schema();
    let p = project(project_input());
    let proof = proof(&index, &p);
    let a = compose(&index, &p, &selection(), &proof, vec![]).unwrap();
    assert_eq!(
        a.request()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings[0]
            .parameters[0]
            .value,
        integer(12)
    );
    assert!(
        a.diagnostics()
            .iter()
            .all(|r| r.disposition == IntentDisposition::Applied)
    );
    assert_eq!(a.receipt().project_digest, proof.content_digest());
    assert_eq!(a.receipt().selection, selection());
    let b = compose(
        &index,
        &p,
        &VariantSelection {
            equipment: id(111),
            ..selection()
        },
        &proof,
        vec![],
    )
    .unwrap();
    assert!(
        b.request()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings
            .is_empty()
    );
    assert!(b.request().scenario().input().usage.is_empty());
    assert!(b.diagnostics().iter().all(|r|matches!(&r.disposition,IntentDisposition::NotApplicable {excluded_sources} if excluded_sources==&[ProviderRoot::EquipmentUse(id(11))])));
    assert_eq!(
        a,
        compose(&index, &p, &selection(), &proof, vec![]).unwrap()
    );
    assert_ne!(a.receipt().request_digest, b.receipt().request_digest);
}

#[test]
fn skill_presets_independently_configure_one_exact_external_occurrence() {
    let mut raw = project_input();
    let mut alternate = raw.skill_presets[0].clone();
    alternate.id = id(131);
    let alternate_intent = alternate.intent.as_mut().unwrap();
    alternate_intent.usage[0].selection = usage(7);
    alternate_intent.generated_inputs[0].parameters[0].value = integer(24);
    raw.skill_presets.push(alternate);

    let original = OwnedDocument::Project(Box::new(project(raw)));
    let bytes = encode_owned(&original, OwnedInputLimits::default()).unwrap();
    let decoded = decode_owned(&bytes, OwnedInputLimits::default()).unwrap();
    assert_eq!(decoded, original);
    assert_eq!(
        encode_owned(&decoded, OwnedInputLimits::default()).unwrap(),
        bytes
    );
    let OwnedDocument::Project(p) = decoded else {
        panic!("project round trip")
    };
    assert_eq!(p.input().skill_presets.len(), 2);

    let index = schema();
    let proof = proof(&index, &p);
    let a = compose(&index, &p, &selection(), &proof, vec![]).unwrap();
    let selection_b = VariantSelection {
        skills: id(131),
        ..selection()
    };
    let b = compose(&index, &p, &selection_b, &proof, vec![]).unwrap();
    for (result, preset, quality, count) in [(&a, id(130), 12, 3), (&b, id(131), 24, 7)] {
        let bindings = &result
            .request()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings;
        assert_eq!(bindings.len(), 1);
        assert_eq!(bindings[0].target, target());
        assert_eq!(bindings[0].origin.skill_preset, preset);
        assert_eq!(
            bindings[0].parameters,
            vec![ParameterAssignment {
                slot: parameter(),
                value: integer(quality)
            }]
        );
        assert_eq!(
            result.request().scenario().input().usage,
            vec![usage(count)]
        );
        assert_eq!(result.receipt().selection.skills, preset);
        assert_eq!(result.receipt().project_digest, proof.content_digest());
        assert_eq!(&result.receipt().data, index.identity());
        assert!(
            result
                .diagnostics()
                .iter()
                .all(|d| d.skill_preset == preset && d.disposition == IntentDisposition::Applied)
        );
    }
    assert_eq!(
        a.request().build().input().equipment,
        b.request().build().input().equipment
    );
    assert_eq!(a.receipt().scenario_digest, b.receipt().scenario_digest);
    assert_ne!(a.receipt().request_digest, b.receipt().request_digest);
    assert_eq!(
        a,
        compose(&index, &p, &selection(), &proof, vec![]).unwrap()
    );
}

#[test]
fn required_source_exclusion_and_explicit_override_are_distinct() {
    let index = schema();
    let mut raw = project_input();
    raw.skill_presets[0].intent = Some(intent(PresetApplicability::Required));
    let p = project(raw);
    let proof = proof(&index, &p);
    assert!(matches!(
        compose(
            &index,
            &p,
            &VariantSelection {
                equipment: id(111),
                ..selection()
            },
            &proof,
            vec![]
        ),
        Err(IntentError::RequiredSources(_))
    ));
    // The source is selected but off-loadout: the override cannot hide schema
    // invalidity, and selection itself remains independent from activation.
    let mut selected = selection();
    selected.active_weapon_loadout = id(2);
    let overridden = compose(&index, &p, &selected, &proof, vec![usage(7)]).unwrap();
    assert_eq!(
        overridden.request().scenario().input().usage,
        vec![usage(7)]
    );
    assert!(
        overridden
            .diagnostics()
            .iter()
            .any(|r| r.disposition == IntentDisposition::Overridden)
    );
    let mut bad = usage(7);
    bad.parameters[0].value = ParameterValue::Boolean(true);
    assert!(matches!(
        compose(&index, &p, &selected, &proof, vec![bad]),
        Err(IntentError::Schema(_))
    ));
}

#[test]
fn dormant_invalid_values_and_partial_definitions_cannot_be_certified() {
    let mut raw = project_input();
    let mut alternate = raw.skill_presets[0].clone();
    alternate.id = id(131);
    alternate.intent.as_mut().unwrap().generated_inputs[0].parameters[0].value = integer(41);
    raw.skill_presets.push(alternate);
    assert!(matches!(
        prove_project_intent(&schema(), &project(raw), BindingLimits::default()),
        Err(IntentError::Schema(_))
    ));
    let mut index = schema();
    let address = def::<SkillDefinition>("child").address();
    let descriptor = index.definitions.get_mut(&address).unwrap();
    if let DefinitionDescriptor::Skill(entry) = descriptor {
        entry.schema = SchemaState::Unmapped {
            gaps: vec![SchemaGap {
                subject: SchemaSubject::Definition(address),
                facet: SchemaFacet::InputSchema,
                code: OwnedDefinitionKey::new("pending").unwrap(),
            }],
        };
    } else {
        panic!()
    }
    let p = project(project_input());
    let proof = proof(&index, &p);
    assert!(!proof.schema_issues().is_empty());
    assert!(matches!(
        compose(
            &index,
            &p,
            &VariantSelection {
                equipment: id(111),
                ..selection()
            },
            &proof,
            vec![]
        ),
        Err(IntentError::Schema(_))
    ));
}

#[test]
fn proof_binds_full_project_data_and_limits_even_without_rows() {
    let index = schema();
    let p = project(project_input());
    let proof = proof(&index, &p);
    let mut changed = p.input().clone();
    changed.revision = BuildRevision::from_u64(2);
    assert!(matches!(
        compose(&index, &project(changed), &selection(), &proof, vec![]),
        Err(IntentError::ProofMismatch)
    ));
    let mut changed = index.clone();
    changed.identity.content_sha256 = "b".repeat(64);
    assert!(matches!(
        compose(&changed, &p, &selection(), &proof, vec![]),
        Err(IntentError::ProofMismatch)
    ));
    let mut empty = project_input();
    empty.skill_presets[0].intent = None;
    let empty = project(empty);
    for limits in [
        BindingLimits {
            max_work: 0,
            ..Default::default()
        },
        BindingLimits {
            max_issues: 0,
            ..Default::default()
        },
        BindingLimits {
            max_work: 100_000_001,
            ..Default::default()
        },
    ] {
        assert!(prove_project_intent(&index, &empty, limits).is_err());
    }
    let mut foreign = index;
    foreign.identity.game = "foreign".into();
    assert!(prove_project_intent(&foreign, &empty, BindingLimits::default()).is_err());
}

#[test]
fn envelope_is_explicit_and_deleted_sources_are_not_dormant() {
    let raw = project_input();
    let mut json = serde_json::to_value(&raw).unwrap();
    json["skill_presets"][0]["intent"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<ProjectInput>(json).is_err());
    let mut mixed = raw.clone();
    mixed.skill_presets[0].usage_preferences = Some(vec![]);
    assert!(BuildProject::new(mixed, OwnedInputLimits::default()).is_err());
    let mut duplicate = raw.clone();
    let row = duplicate.skill_presets[0]
        .intent
        .as_ref()
        .unwrap()
        .generated_inputs[0]
        .clone();
    duplicate.skill_presets[0]
        .intent
        .as_mut()
        .unwrap()
        .generated_inputs
        .push(row);
    assert!(BuildProject::new(duplicate, OwnedInputLimits::default()).is_err());
    let mut deleted = raw.clone();
    deleted.equipment.clear();
    deleted.equipment_presets[0].equipment.clear();
    assert!(BuildProject::new(deleted, OwnedInputLimits::default()).is_err());
    let p = project(raw);
    assert!(
        poe_optimizer_core::owned_project::compose(
            &p,
            &selection(),
            None,
            OwnedInputLimits::default()
        )
        .is_err()
    );
}

#[test]
fn schema_issue_budget_is_shared_across_presets_and_legacy_bytes_are_unchanged() {
    let mut raw = project_input();
    let mut second = raw.skill_presets[0].clone();
    second.id = id(131);
    raw.skill_presets.push(second);
    let mut index = schema();
    let address = def::<UsagePolicyDefinition>("usage").address();
    let DefinitionDescriptor::UsagePolicy(entry) = index.definitions.get_mut(&address).unwrap()
    else {
        panic!()
    };
    entry.schema = SchemaState::Unmapped {
        gaps: vec![SchemaGap {
            subject: SchemaSubject::Definition(address),
            facet: SchemaFacet::InputSchema,
            code: OwnedDefinitionKey::new("pending").unwrap(),
        }],
    };
    let p = project(raw);
    let mut one = p.input().clone();
    one.skill_presets.truncate(1);
    let one = project(one);
    let one_issue_count = prove_project_intent(&index, &one, BindingLimits::default())
        .unwrap()
        .schema_issues()[0]
        .issues
        .len();
    let one_preset_budget = BindingLimits {
        max_issues: one_issue_count,
        ..Default::default()
    };
    assert!(prove_project_intent(&index, &one, one_preset_budget).is_ok());
    assert!(matches!(
        prove_project_intent(&index, &p, one_preset_budget),
        Err(IntentError::Binding(BindingError::IssueLimit))
    ));
    assert_eq!(
        prove_project_intent(&index, &p, BindingLimits::default())
            .unwrap()
            .schema_issues()
            .len(),
        2
    );
    let mut old = project_input();
    old.skill_presets[0].intent = None;
    let bytes = serde_json::to_vec(&old).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("\"intent\"")
    );
    assert_eq!(
        bytes,
        serde_json::to_vec(&serde_json::from_slice::<ProjectInput>(&bytes).unwrap()).unwrap()
    );
}

#[test]
fn pending_finalization_still_validates_current_limits_and_index() {
    let mut raw = draft(project_input());
    raw.items.members[0].template = pending(901);
    let session = DraftSession::new(raw, DraftLimits::default()).unwrap();
    let index = schema();
    let proof = session
        .prove_intent(&index, DraftLimits::default(), BindingLimits::default())
        .unwrap();
    let checked = session
        .finalize_selection_checked(
            &index,
            evaluation(),
            &proof,
            DraftLimits::default(),
            BindingLimits::default(),
        )
        .unwrap();
    assert!(
        matches!(checked.finalization(), DraftFinalization::Pending {issues, ..} if issues.iter().any(|i| i.id == id(901)))
    );
    for limits in [
        BindingLimits {
            max_work: 0,
            ..Default::default()
        },
        BindingLimits {
            max_issues: 0,
            ..Default::default()
        },
        BindingLimits {
            max_work: 100_000_001,
            ..Default::default()
        },
        BindingLimits {
            max_issues: 65_537,
            ..Default::default()
        },
    ] {
        assert!(matches!(
            session.finalize_selection_checked(
                &index,
                evaluation(),
                &proof,
                DraftLimits::default(),
                limits
            ),
            Err(FinalizationError::Intent(IntentError::Binding(
                BindingError::InvalidLimit
            )))
        ));
    }
    let mut foreign = index;
    foreign.namespace = GameVersionNamespace::new("different-game", "v1").unwrap();
    assert!(matches!(
        session.finalize_selection_checked(
            &foreign,
            evaluation(),
            &proof,
            DraftLimits::default(),
            BindingLimits::default()
        ),
        Err(FinalizationError::Intent(IntentError::Binding(
            BindingError::ForeignNamespace
        )))
    ));
}

#[test]
fn draft_proof_precedes_trimming_and_preserves_selected_raw_values() {
    let index = schema();
    let session = DraftSession::new(draft(project_input()), DraftLimits::default()).unwrap();
    assert!(
        session
            .finalize_selection(evaluation(), DraftLimits::default())
            .is_err()
    );
    let proof = session
        .prove_intent(&index, DraftLimits::default(), BindingLimits::default())
        .unwrap();
    let a = session
        .finalize_selection_checked(
            &index,
            evaluation(),
            &proof,
            DraftLimits::default(),
            BindingLimits::default(),
        )
        .unwrap();
    let DraftFinalization::Ready(ready) = a.finalization() else {
        panic!("{a:?}")
    };
    assert_eq!(
        ready
            .request()
            .build()
            .input()
            .generated_inputs
            .as_ref()
            .unwrap()
            .bindings[0]
            .parameters[0]
            .value,
        integer(12)
    );
    let b = session
        .finalize_selection_checked(
            &index,
            EvaluationSelection {
                build: VariantSelection {
                    equipment: id(111),
                    ..selection()
                },
                ..evaluation()
            },
            &proof,
            DraftLimits::default(),
            BindingLimits::default(),
        )
        .unwrap();
    let DraftFinalization::Ready(ready) = b.finalization() else {
        panic!("{b:?}")
    };
    assert!(ready.request().build().input().equipment.is_empty());
    assert_eq!(b.diagnostics().len(), 2);
}

#[test]
fn unresolved_modifier_and_host_remain_obligations_even_when_excluded() {
    for host in [false, true] {
        let mut raw = draft(project_input());
        if host {
            raw.equipment.members.push(
                EquipmentUse {
                    id: id(12),
                    item: id(10),
                    destination: EquipmentDestination::CharacterSlot(def("host")),
                    scope: LoadoutScope::Shared,
                }
                .into(),
            );
            raw.equipment.members[0].destination = EquipmentDestination::ItemSocket {
                container: id(12),
                slot: def("socket"),
            }
            .into();
            raw.equipment.members[1].item = pending(901);
        } else {
            raw.items.members[0].modifier_order = vec![id::<ModifierInstanceId>(13)].into();
            raw.items.members[0].modifiers.members.push(ModifierDraft {
                id: id(13),
                definition: pending(901),
                rolls: Vec::<ParameterAssignment>::new().into(),
            });
        }
        let session = DraftSession::new(raw, DraftLimits::default()).unwrap();
        let index = schema();
        let proof = session
            .prove_intent(&index, DraftLimits::default(), BindingLimits::default())
            .unwrap();
        assert_eq!(proof.unresolved_dependencies()[&id(130)][0].id, id(901));
        let result = session
            .finalize_selection_checked(
                &index,
                EvaluationSelection {
                    build: VariantSelection {
                        equipment: id(111),
                        ..selection()
                    },
                    ..evaluation()
                },
                &proof,
                DraftLimits::default(),
                BindingLimits::default(),
            )
            .unwrap();
        assert!(
            matches!(result.finalization(),DraftFinalization::Pending {issues,..} if issues.iter().any(|v|v.id==id(901)))
        );
        assert!(result.diagnostics().is_empty());
        assert!(matches!(
            session.prove_intent(
                &index,
                DraftLimits::default(),
                BindingLimits {
                    max_work: proof.work_used() - 1,
                    ..Default::default()
                }
            ),
            Err(IntentError::Binding(BindingError::WorkLimit))
        ));
        assert!(
            session
                .prove_intent(&index, DraftLimits::default(), BindingLimits::default())
                .is_ok()
        );
    }
}
