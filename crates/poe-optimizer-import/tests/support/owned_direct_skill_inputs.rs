//! Source-bound Direct inputs use the same typed values as projected inputs.
use super::*;

pub(crate) const DIRECT: &str = r#"<Gem gemId="active" variantId="v" skillId="synthetic-effect" nameSpec="Synthetic effect" level="17" quality="0" enabled="true" corrupted="false"/>"#;

fn xml(rows: &str) -> String {
    format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="7"><SkillSet id="7"><Skill enabled="true">{rows}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    )
}
pub(crate) fn fixture() -> (Artifacts, NormalizationPolicy) {
    let mut a = artifacts(true);
    replace_materialization(&mut a, "active", OwnedGemMaterialization::ProviderOnly);
    let role = a
        .roles
        .input()
        .roles
        .iter()
        .find(|role| role.materialization == OwnedGemMaterialization::ProviderOnly)
        .unwrap()
        .clone();
    let OwnedPrimarySkill::Known(skill) = role.primary else {
        unreachable!()
    };
    let level = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Skill(skill.clone()))
        .unwrap();
    let quality = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::Skill(skill.clone()))
        .unwrap();
    let unit = a.registry.allocate_definition::<UnitDefinition>().unwrap();
    let mut schema = a.schema.input().clone();
    schema.schema_version = poe_optimizer_data::owned_schema::OWNED_SCHEMA_PACKAGE_VERSION;
    for entry in &mut schema.definitions {
        if let DefinitionDescriptor::Skill(entry) = entry
            && entry.id == skill
        {
            entry.schema = SchemaState::Known(SkillSchema {
                directly_selectable: true,
                declarations: DeclaredSlots {
                    parameters: DeclaredSet::partial(
                        vec![level.clone(), quality.clone()],
                        vec![SchemaGap {
                            subject: subject(&skill),
                            facet: SchemaFacet::InputSchema,
                            code: key("other-inputs-unreviewed"),
                        }],
                    ),
                    choices: DeclaredSet::complete(vec![]),
                    grants: DeclaredSet::complete(vec![]),
                    actors: DeclaredSet::complete(vec![]),
                    skill_grants: DeclaredSet::complete(vec![]),
                    outputs: DeclaredSet::complete(vec![]),
                    sockets: DeclaredSet::complete(vec![]),
                },
            });
        }
    }
    schema
        .definitions
        .push(DefinitionDescriptor::Unit(DefinitionEntry {
            id: unit.clone(),
            schema: SchemaState::Known(UnitSchema {
                dimension: UnitDimension::Count,
            }),
        }));
    for slot in [&level, &quality] {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: slot.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    skill_input: Some(SkillInputAuthority::AuthoredOrProjected),
                    value: ValueSchema::Quantity(QuantityRange {
                        minimum: FiniteQuantity::new(-1000.0, unit.clone()).unwrap(),
                        maximum: FiniteQuantity::new(1000.0, unit.clone()).unwrap(),
                    }),
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::SkillParameter],
                }),
            }));
    }
    let mut p = policy();
    rebind_quality_schema(&mut a, &mut p, schema);
    p.skill_scopes = Some(SkillScopePolicy {
        slot_attribute: "slot".into(),
        shared_slots: vec![SourceComponent::Missing],
    });
    p.direct_skill_inputs = Some(DirectSkillInputPolicy::PobManualDirectSkillV2 {
        dispositions: vec![],
        definitions: a.schema.identity().clone(),
        source: pin(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        manual_sources: vec![SourceComponent::Missing],
        group_attributes: ["source", "enabled", "slot"].map(str::to_string).to_vec(),
        skills: vec![DirectSkillInputRule {
            gem: role.gem,
            game_id: "active".into(),
            variant_id: "v".into(),
            skill_id: "synthetic-effect".into(),
            name_spec: "Synthetic effect".into(),
            skill,
            attributes: [
                "gemId",
                "variantId",
                "skillId",
                "nameSpec",
                "level",
                "quality",
                "enabled",
                "corrupted",
            ]
            .map(str::to_string)
            .to_vec(),
            guards: vec![GemInputGuard {
                attribute: "corrupted".into(),
                allowed: vec![SourceComponent::Text("false".into())],
            }],
            parameters: [(level, "level"), (quality, "quality")]
                .into_iter()
                .map(|(slot, name)| {
                    let mut value = value_recipe(name, name, false);
                    value.codec.codec = ValueCodecKind::Quantity {
                        syntax: DecimalSyntax::Scientific,
                        unit: unit.clone(),
                        scale: RationalScale {
                            numerator: BoundedInteger::new(1).unwrap(),
                            denominator: BoundedInteger::new(1).unwrap(),
                        },
                    };
                    DirectSkillParameterInput { slot, value }
                })
                .collect(),
        }],
    });
    (a, p)
}
pub(crate) fn row(p: &mut NormalizationPolicy) -> &mut DirectSkillInputRule {
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { skills, .. } =
        p.direct_skill_inputs.as_mut().unwrap();
    &mut skills[0]
}
pub(crate) fn run(
    a: &Artifacts,
    p: &NormalizationPolicy,
    xml: &str,
) -> Result<NormalizedImport, NormalizationError> {
    usage_input_tests::run_limits(xml, a, p, Default::default())
}
fn values(skill: &SkillDraft) -> Vec<Option<f64>> {
    skill
        .parameters
        .as_ref()
        .unwrap()
        .members
        .iter()
        .map(|parameter| match parameter.value.to_resolved() {
            Some(ParameterValue::Quantity(value)) => Some(value.value()),
            None => None,
            _ => panic!("expected typed quantity or Pending"),
        })
        .collect()
}

#[test]
fn direct_occurrences_preserve_raw_values_actual_preset_and_pending_destinations() {
    let (a, p) = fixture();
    let other = DIRECT
        .replace("level=\"17\"", "level=\"1.25e1\"")
        .replace("quality=\"0\"", "quality=\"-2.5\"");
    let source = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7"><Skill enabled="false">{DIRECT}{SUPPORT}</Skill></SkillSet><SkillSet id="8"><Skill enabled="true">{other}{DIRECT}{SUPPORT}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let result = run(&a, &p, &source).unwrap();
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 3);
    assert_eq!(
        draft.gems.members.len(),
        2,
        "only the two physical supports are Gems"
    );
    assert_eq!(draft.supports.members.len(), 2);
    assert!(
        draft
            .supports
            .members
            .iter()
            .all(|support| matches!(support.target, DraftSkillTarget::Pending(_)))
    );
    assert_eq!(
        draft.skill_presets.members[0].skills.members,
        vec![draft.skills.members[0].id]
    );
    assert_eq!(
        draft.skill_presets.members[1].skills.members,
        vec![draft.skills.members[1].id, draft.skills.members[2].id]
    );
    assert_eq!(
        values(&draft.skills.members[0]),
        vec![Some(17.0), Some(0.0)]
    );
    assert_eq!(
        values(&draft.skills.members[1]),
        vec![Some(12.5), Some(-2.5)]
    );
    assert_eq!(
        values(&draft.skills.members[2]),
        vec![Some(17.0), Some(0.0)]
    );
    assert_eq!(draft.skills.members[0].enabled.to_resolved(), Some(false));
    assert_eq!(draft.skills.members[1].enabled.to_resolved(), Some(true));
    for skill in &draft.skills.members {
        assert!(matches!(
            skill.source,
            DraftAuthoredSkillSource::Direct(DraftField::Known { .. })
        ));
        assert!(matches!(
            skill.parameters.as_ref().unwrap().completion,
            DraftListCompletion::Pending { .. }
        ));
        let sources = result
            .sidecar()
            .origins
            .iter()
            .filter(|origin| origin.links.contains(&OwnedOriginTarget::Skill(skill.id)))
            .count();
        assert_eq!(
            sources, 2,
            "one exact source row plus its shared-scope group"
        );
    }
    for preset in &draft.skill_presets.members {
        assert!(matches!(
            preset.skills.completion,
            DraftListCompletion::Pending { .. }
        ));
        let usage = preset.usage_preferences.as_ref().unwrap();
        assert!(usage.members.is_empty());
        assert!(matches!(
            usage.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
}

#[test]
fn missing_malformed_out_of_range_inputs_remain_pending_even_when_disabled() {
    let (a, p) = fixture();
    for token in [None, Some("oops"), Some("NaN"), Some("1001"), Some(" 17 ")] {
        let replacement = token.map_or_else(String::new, |token| format!("level=\"{token}\""));
        let source = xml(&DIRECT.replace("level=\"17\"", &replacement))
            .replace("<Skill enabled=\"true\">", "<Skill enabled=\"false\">");
        let result = run(&a, &p, &source).unwrap();
        let skill = &result.draft().input().skills.members[0];
        assert_eq!(values(skill), vec![None, Some(0.0)], "{token:?}");
        assert_eq!(skill.enabled.to_resolved(), Some(false));
        assert!(matches!(
            skill.parameters.as_ref().unwrap().members[0].slot,
            DraftField::Known { .. }
        ));
    }
}

#[test]
fn nonmanual_unknown_local_namespaced_and_ambiguous_frames_never_create_direct_instances() {
    let (a, p) = fixture();
    let baseline = xml(DIRECT);
    for source in [
        baseline.replace("<Skill enabled", "<Skill source=\"Tree:42\" enabled"),
        baseline.replace("<Gem ", "<Gem mystery=\"1\" "),
        baseline.replace("<Skill enabled", "<Skill mystery=\"1\" enabled"),
        baseline.replace("skillId=\"synthetic-effect\"", "skillId=\"other-effect\""),
        baseline.replace("nameSpec=\"Synthetic effect\"", "nameSpec=\"alias\""),
        baseline.replace("corrupted=\"false\"", "corrupted=\"true\""),
        baseline.replace("<Gem ", "<Gem xmlns=\"foreign\" "),
        baseline.replace(
            "<SkillSet id=\"7\">",
            "<SkillSet xmlns=\"foreign\" id=\"7\">",
        ),
        baseline.replace("</Skills>", "<SkillSet id=\"7\"/></Skills>"),
        baseline.replace("id=\"7\"", "id=\"07\""),
    ] {
        let result = run(&a, &p, &source).unwrap();
        assert!(result.draft().input().skills.members.is_empty(), "{source}");
        assert!(result.draft().input().gems.members.is_empty());
    }
    let duplicated = baseline.replace("level=\"17\"", "level=\"17\" level=\"18\"");
    assert!(match decode_build(duplicated.as_bytes()) {
        Err(_) => true,
        Ok(decoded) => match ImportedBuildInstance::from_decoded(
            decoded,
            BuildLineage::from_bytes([93; 16]),
            InstanceImportLimits::default()
        ) {
            Err(_) => true,
            Ok(source) =>
                SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).is_err(),
        },
    });
}

#[test]
fn unreviewed_siblings_do_not_discard_raw_direct_inputs_or_authorize_complete_inventories() {
    let (a, p) = fixture();
    let source = xml(&format!("{DIRECT}{SUPPORT}"))
        .replace("</SkillSet>", "<Skill><Unexpected/></Skill></SkillSet>");
    let result = run(&a, &p, &source).unwrap();
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 1);
    let skill = &draft.skills.members[0];
    assert_eq!(values(skill), [Some(17.0), Some(0.0)]);
    assert!(matches!(
        skill.parameters.as_ref().unwrap().completion,
        DraftListCompletion::Pending { .. }
    ));
    assert_eq!(draft.supports.members.len(), 1);
    assert!(matches!(
        draft.supports.members[0].target,
        DraftSkillTarget::Pending(_)
    ));
    let preset = &draft.skill_presets.members[0];
    assert_eq!(preset.skills.members, [skill.id]);
    assert!(matches!(
        preset.skills.completion,
        DraftListCompletion::Pending { .. }
    ));
    assert!(matches!(
        preset.supports.completion,
        DraftListCompletion::Pending { .. }
    ));
    let usage = preset.usage_preferences.as_ref().unwrap();
    assert!(usage.members.is_empty());
    assert!(matches!(
        usage.completion,
        DraftListCompletion::Pending { .. }
    ));
}

#[test]
fn stale_source_definition_catalog_and_role_bindings_reject_the_policy() {
    let (a, p) = fixture();
    for case in 0..4 {
        let mut changed = p.clone();
        let DirectSkillInputPolicy::PobManualDirectSkillV2 {
            definitions,
            source,
            catalog,
            roles,
            ..
        } = changed.direct_skill_inputs.as_mut().unwrap();
        match case {
            0 => source.revision = "d".repeat(40),
            1 => definitions.release.push_str("-stale"),
            2 => *catalog = "1".repeat(64).parse().unwrap(),
            _ => *roles = "2".repeat(64).parse().unwrap(),
        }
        assert!(matches!(
            run(&a, &changed, &xml(DIRECT)),
            Err(NormalizationError::Binding)
        ));
    }
}

#[test]
fn catalog_provenance_may_be_a_strict_subset_but_every_required_pin_is_immutable() {
    let (mut a, mut p) = fixture();
    let before = run(&a, &p, &xml(DIRECT)).unwrap();
    let mut mapping = a.mapping.input().clone();
    mapping.source.files.push(SourceFilePin {
        path: "unrelated/additional-catalog.json".into(),
        sha256: "e".repeat(64),
    });
    a.mapping = OwnedMappingIndex::new(
        mapping,
        &a.registry,
        &a.schema,
        OwnedMappingLimits::default(),
    )
    .unwrap();
    let mut roles = a.roles.input().clone();
    roles.mapping = *a.mapping.identity();
    a.roles = OwnedSkillRoleIndex::new(roles, &a.mapping, &a.schema, SkillCatalogLimits::default())
        .unwrap();
    a.rewards = empty_rewards(&a.mapping, &a.schema);
    assert!(
        matches!(run(&a, &p, &xml(DIRECT)), Err(NormalizationError::Binding)),
        "the role digest still requires exact rebinding"
    );
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { roles, .. } =
        p.direct_skill_inputs.as_mut().unwrap();
    *roles = *a.roles.identity();
    let after = run(&a, &p, &xml(DIRECT)).unwrap();
    assert_eq!(before.draft(), after.draft());
    assert_eq!(before.allocator_after(), after.allocator_after());
    assert_eq!(a.roles.input().compilation.source.files.len(), 1);
    assert_eq!(a.mapping.input().source.files.len(), 2);

    let mut widened = p.clone();
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { source, .. } =
        widened.direct_skill_inputs.as_mut().unwrap();
    *source = a.mapping.input().source.clone();
    assert!(
        matches!(
            run(&a, &widened, &xml(DIRECT)),
            Err(NormalizationError::Binding)
        ),
        "policy must still match the exact immutable role receipt"
    );

    for changed_hash in [false, true] {
        let mut mapping = a.mapping.input().clone();
        let required = &pin().files[0].path;
        if changed_hash {
            mapping
                .source
                .files
                .iter_mut()
                .find(|file| &file.path == required)
                .unwrap()
                .sha256 = "f".repeat(64);
        } else {
            mapping.source.files.retain(|file| &file.path != required);
        }
        a.mapping = OwnedMappingIndex::new(
            mapping,
            &a.registry,
            &a.schema,
            OwnedMappingLimits::default(),
        )
        .unwrap();
        assert!(
            matches!(run(&a, &p, &xml(DIRECT)), Err(NormalizationError::Binding)),
            "missing or changed required catalog pins cannot authorize import"
        );
        // Restore the successful extended mapping for the independent control.
        let mut mapping = a.mapping.input().clone();
        mapping.source.files.retain(|file| &file.path != required);
        mapping.source.files.push(pin().files[0].clone());
        a.mapping = OwnedMappingIndex::new(
            mapping,
            &a.registry,
            &a.schema,
            OwnedMappingLimits::default(),
        )
        .unwrap();
    }
}

#[test]
fn typed_inputs_require_exact_authored_authority_owner_and_unique_recipe() {
    let (a, p) = fixture();
    for case in 0..7 {
        let mut changed = p.clone();
        let input = row(&mut changed);
        match case {
            0 => input.parameters.push(input.parameters[0].clone()),
            1 => input.parameters[0].slot.declaration = SlotOwnerDefId::Gem(input.gem.clone()),
            2 => input.parameters[0].value.tiers[0].selectors[0].lane = ValueLane::ParentAttribute,
            3 => input.parameters[0].value.tiers[0].duplicates = DuplicatePolicy::LastInSourceOrder,
            4 => input.parameters[0].value.missing = MissingValuePolicy::Absent,
            5 => input.parameters[0].value.codec.whitespace = WhitespacePolicy::TrimAscii,
            _ => input.parameters[0].value.codec.codec = ValueCodecKind::Boolean { tokens: vec![] },
        }
        assert!(run(&a, &changed, &xml(DIRECT)).is_err(), "{case}");
    }
    for authority in [None, Some(SkillInputAuthority::Projected)] {
        let (mut a, mut p) = fixture();
        let mut schema = a.schema.input().clone();
        for slot in &mut schema.slots {
            if let SlotDescriptor::Parameter(slot) = slot
                && let SchemaState::Known(value) = &mut slot.schema
            {
                value.skill_input = authority;
                value.sites.clear();
            }
        }
        rebind_quality_schema(&mut a, &mut p, schema);
        let DirectSkillInputPolicy::PobManualDirectSkillV2 {
            definitions, roles, ..
        } = p.direct_skill_inputs.as_mut().unwrap();
        *definitions = a.schema.identity().clone();
        *roles = *a.roles.identity();
        assert!(run(&a, &p, &xml(DIRECT)).is_err());
    }
    let (mut a, mut p) = fixture();
    replace_materialization(&mut a, "active", OwnedGemMaterialization::Physical);
    let DirectSkillInputPolicy::PobManualDirectSkillV2 { roles, .. } =
        p.direct_skill_inputs.as_mut().unwrap();
    *roles = *a.roles.identity();
    assert!(matches!(
        run(&a, &p, &xml(DIRECT)),
        Err(NormalizationError::Policy(_))
    ));
}

#[test]
fn direct_omission_is_byte_and_allocation_compatible_null_and_unknown_version_reject() {
    let (a, mut p) = fixture();
    let mut json = serde_json::to_value(&p).unwrap();
    let current: NormalizationPolicy = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&current).unwrap(), json);
    assert_eq!(
        run(&a, &current, &xml(DIRECT)).unwrap().draft(),
        run(&a, &p, &xml(DIRECT)).unwrap().draft()
    );
    json["direct_skill_inputs"]["kind"] = "pob_manual_direct_skill_v999".into();
    assert!(serde_json::from_value::<NormalizationPolicy>(json).is_err());
    let mut null = serde_json::to_value(&p).unwrap();
    null["direct_skill_inputs"] = serde_json::Value::Null;
    assert!(serde_json::from_value::<NormalizationPolicy>(null).is_err());
    p.direct_skill_inputs = None;
    let bytes = serde_json::to_vec(&p).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("direct_skill_inputs")
    );
    let restored: NormalizationPolicy = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    let first = run(&a, &p, &xml(DIRECT)).unwrap();
    let second = run(&a, &restored, &xml(DIRECT)).unwrap();
    assert!(first.draft().input().skills.members.is_empty());
    assert_eq!(first.draft(), second.draft());
    assert_eq!(first.allocator_after(), second.allocator_after());
    assert_eq!(
        serde_json::to_value(first.sidecar()).unwrap(),
        serde_json::to_value(second.sidecar()).unwrap()
    );
}

#[test]
fn policy_and_source_work_are_bounded_without_partial_results() {
    let (a, p) = fixture();
    let source = xml(DIRECT);
    for max_work in [0, NormalizationLimits::default().max_work + 1] {
        let limits = NormalizationLimits {
            max_work,
            ..Default::default()
        };
        assert!(matches!(
            usage_input_tests::run_limits(&source, &a, &p, limits),
            Err(NormalizationError::InvalidLimit("work"))
        ));
    }
    let policy_bytes = serde_json::to_vec(&p.direct_skill_inputs).unwrap().len();
    let limits = NormalizationLimits {
        max_work: policy_bytes - 1,
        ..Default::default()
    };
    assert!(matches!(
        usage_input_tests::run_limits(&source, &a, &p, limits),
        Err(NormalizationError::Limit(_))
    ));
    let mut limits = NormalizationLimits::default();
    limits.value.value.max_source_bytes = 5;
    let at_limit = source.replace("level=\"17\"", "level=\"1.000\"");
    let result = usage_input_tests::run_limits(&at_limit, &a, &p, limits).unwrap();
    assert_eq!(
        values(&result.draft().input().skills.members[0]),
        vec![Some(1.0), Some(0.0)]
    );
    let oversized = source.replace("level=\"17\"", "level=\"1.0000\"");
    assert_eq!(
        values(
            &run(&a, &p, &oversized)
                .unwrap()
                .draft()
                .input()
                .skills
                .members[0]
        ),
        vec![Some(1.0), Some(0.0)]
    );
    assert!(matches!(
        usage_input_tests::run_limits(&oversized, &a, &p, limits),
        Err(NormalizationError::Value(ValuePolicyError::ResourceLimit {
            resource: ValuePolicyResource::CandidateBytes,
            maximum: 5,
        }))
    ));
    let mut changed = p;
    let parameter = row(&mut changed).parameters[0].clone();
    row(&mut changed).parameters = vec![parameter; 65];
    assert!(run(&a, &changed, &source).is_err());
}

#[test]
fn raw_only_inputs_remain_bounded_and_do_not_replace_independent_inventory_proofs() {
    let (a, p) = fixture();
    let mut omitted = p.clone();
    omitted.direct_skill_inputs = None;
    // These retained rows do not match the Direct catalog. Exact raw-input
    // transport must not pretend to certify their whole saved-set inventory.
    const ROW: &str = r#"<Gem gemId="unmapped" variantId="v" skillId="unmapped" nameSpec="Unmapped" level="17" quality="0" enabled="true" enableGlobal1="true" enableGlobal2="false" count="1" skillMinion="unmapped" skillMinionSkill="1" skillMinionSkillCalcs="1" corrupted="false" corruptLevel="0"/>"#;
    let source = |directs: usize| {
        xml(&format!(
            "{}{SUPPORT}{}",
            ROW.repeat(256),
            DIRECT.repeat(directs)
        ))
    };
    let minimum_work = |source: &str, policy: &NormalizationPolicy| {
        let mut low = 1;
        let mut high = NormalizationLimits::default().max_work;
        assert!(run(&a, policy, source).is_ok());
        while low < high {
            let middle = low + (high - low) / 2;
            let limits = NormalizationLimits {
                max_work: middle,
                ..Default::default()
            };
            match usage_input_tests::run_limits(source, &a, policy, limits) {
                Ok(_) => high = middle,
                Err(NormalizationError::Limit(_)) => low = middle + 1,
                Err(error) => panic!("unexpected budget outcome: {error}"),
            }
        }
        low
    };
    let unrelated = source(0);
    let one = source(1);
    let two = source(2);
    let result = run(&a, &p, &unrelated).unwrap();
    let prior = run(&a, &omitted, &unrelated).unwrap();
    assert!(result.draft().input().skills.members.is_empty());
    assert_eq!(result.draft().input().gems.members.len(), 1);
    assert_eq!(result.draft(), prior.draft());
    assert_eq!(result.allocator_after(), prior.allocator_after());
    assert_eq!(
        run(&a, &p, &two)
            .unwrap()
            .draft()
            .input()
            .skills
            .members
            .len(),
        2
    );

    let invalidate =
        |source: &str| source.replace("</SkillSet>", "<Skill><Unexpected/></Skill></SkillSet>");
    let invalid_two = invalidate(&two);
    // Inventory adapters still require their own complete frame/classification
    // proofs. Their failure cannot promote raw transport to inventory authority
    // or discard independently checked inputs. Shared census cache behavior is
    // covered directly by the gem_inventory::dispositions unit tests.
    let mut inventory = omitted.clone();
    inventory.support_origin_order = Some(
        SupportOriginOrderPolicy::SavedManualGroupOrderWithPhysicalInventoryV2 {
            mapping_source: *a.mapping.source_identity(),
            roles: *a.roles.identity(),
        },
    );
    inventory.payload_inventory = Some(
        PayloadInventoryPolicy::SavedGroupsWithoutAuthoredContainersV1 {
            mapping_source: *a.mapping.source_identity(),
            roles: *a.roles.identity(),
            non_container_gems: a
                .roles
                .input()
                .roles
                .iter()
                .map(|role| role.gem.clone())
                .collect(),
            nonphysical_non_container_skill_ids: vec!["synthetic-effect".into(), "unmapped".into()],
        },
    );
    let mut combined = inventory.clone();
    combined.direct_skill_inputs = p.direct_skill_inputs.clone();
    for policy in [&p, &combined] {
        for (source, count) in [(&one, 1), (&invalid_two, 2)] {
            let baseline = run(&a, policy, source).unwrap();
            let draft = baseline.draft().input();
            assert_eq!(draft.skills.members.len(), count);
            for skill in &draft.skills.members {
                assert_eq!(values(skill), [Some(17.0), Some(0.0)]);
                assert!(matches!(
                    skill.parameters.as_ref().unwrap().completion,
                    DraftListCompletion::Pending { .. }
                ));
            }
            assert_eq!(draft.supports.members.len(), 1);
            assert!(matches!(
                draft.supports.members[0].target,
                DraftSkillTarget::Pending(_)
            ));
            let preset = &draft.skill_presets.members[0];
            assert!(matches!(
                preset.skills.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                preset.supports.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                preset.payload_links.completion,
                DraftListCompletion::Pending { .. }
            ));
            assert!(matches!(
                preset.usage_preferences.as_ref().unwrap().completion,
                DraftListCompletion::Pending { .. }
            ));

            let work = minimum_work(source, policy);
            assert!(matches!(
                usage_input_tests::run_limits(
                    source,
                    &a,
                    policy,
                    NormalizationLimits {
                        max_work: work - 1,
                        ..Default::default()
                    }
                ),
                Err(NormalizationError::Limit(_))
            ));
            let exact = usage_input_tests::run_limits(
                source,
                &a,
                policy,
                NormalizationLimits {
                    max_work: work,
                    ..Default::default()
                },
            )
            .unwrap();
            assert_eq!(exact.draft(), baseline.draft());
            assert_eq!(exact.allocator_after(), baseline.allocator_after());
            assert_eq!(
                serde_json::to_value(exact.sidecar()).unwrap(),
                serde_json::to_value(baseline.sidecar()).unwrap()
            );
        }
    }
}
