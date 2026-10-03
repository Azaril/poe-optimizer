//! Synthetic occurrence/preset correspondence, independent of game identities.
use super::*;

fn declarations() -> DeclaredSlots {
    DeclaredSlots {
        parameters: DeclaredSet::complete(vec![]),
        choices: DeclaredSet::complete(vec![]),
        grants: DeclaredSet::complete(vec![]),
        actors: DeclaredSet::complete(vec![]),
        skill_grants: DeclaredSet::complete(vec![]),
        outputs: DeclaredSet::complete(vec![]),
        sockets: DeclaredSet::complete(vec![]),
    }
}
fn fixture() -> (Artifacts, NormalizationPolicy) {
    let mut a = artifacts(true);
    replace_gem_input_schema(
        &mut a,
        AuthoredGemRole::SkillUse,
        GemParameterShape::Required,
        false,
    );
    let role = a
        .roles
        .input()
        .roles
        .iter()
        .find(|r| r.role == OwnedGemRole::Known(AuthoredGemRole::SkillUse))
        .unwrap()
        .clone();
    let OwnedPrimarySkill::Known(primary) = role.primary else {
        unreachable!()
    };
    let owner = SlotOwnerDefId::Gem(role.gem.clone());
    let supply = a
        .registry
        .allocate_slot::<SkillGrantSlotDefinition>(owner.clone())
        .unwrap();
    let grant = a
        .registry
        .allocate_slot::<GrantSlotDefinition>(owner)
        .unwrap();
    let usage = a
        .registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    let first = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(usage.clone()))
        .unwrap();
    let second = a
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(usage.clone()))
        .unwrap();
    let mut schema = a.schema.input().clone();
    let mut intrinsic = None;
    for definition in &mut schema.definitions {
        match definition {
            DefinitionDescriptor::Gem(row) if row.id == role.gem => {
                let SchemaState::Known(gem) = &mut row.schema else {
                    unreachable!()
                };
                intrinsic = Some(gem.declarations.parameters.members[0].clone());
                gem.declarations.parameters = DeclaredSet::partial(
                    gem.declarations.parameters.members.clone(),
                    vec![SchemaGap {
                        subject: subject(&role.gem),
                        facet: SchemaFacet::InputSchema,
                        code: key("additional-intrinsic-inputs"),
                    }],
                );
                gem.declarations.skill_grants = DeclaredSet::complete(vec![supply.clone()]);
                gem.declarations.grants = DeclaredSet::complete(vec![grant.clone()]);
            }
            DefinitionDescriptor::Skill(row) if row.id == primary => {
                row.schema = SchemaState::Known(SkillSchema {
                    directly_selectable: false,
                    declarations: declarations(),
                })
            }
            _ => {}
        }
    }
    let mut usage_declarations = declarations();
    usage_declarations.parameters = DeclaredSet::complete(vec![first.clone(), second.clone()]);
    schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            id: usage.clone(),
            schema: SchemaState::Known(UsagePolicySchema {
                targets: vec![UsageTargetKind::Skill],
                declarations: usage_declarations,
            }),
        }));
    schema
        .slots
        .push(SlotDescriptor::SkillGrant(DefinitionEntry {
            id: supply.clone(),
            schema: SchemaState::Known(SkillGrantSlotSchema {
                skill: primary.clone(),
                outputs: DeclaredSet::complete(vec![]),
            }),
        }));
    schema.slots.push(SlotDescriptor::Grant(DefinitionEntry {
        id: grant.clone(),
        schema: SchemaState::Known(GrantSlotSchema {
            provider_roles: vec![ProviderRole::SkillUse],
            target: GrantTarget::Skill(supply.clone()),
        }),
    }));
    for id in [&first, &second] {
        schema
            .slots
            .push(SlotDescriptor::Parameter(DefinitionEntry {
                id: id.clone(),
                schema: SchemaState::Known(ParameterSlotSchema {
                    value: ValueSchema::Boolean,
                    presence: SlotPresence::RequiredOnce,
                    sites: vec![ParameterSite::UsagePolicyParameter],
                }),
            }));
    }
    let mut p = policy();
    p.gem_inputs = Some(GemInputPolicy {
        definitions: a.schema.identity().clone(),
        gems: vec![GemInputRule {
            gem: role.gem.clone(),
            guards: vec![],
            parameters: vec![GemParameterInput {
                slot: intrinsic.unwrap(),
                value: value_recipe("intrinsic", "intrinsic", true),
            }],
        }],
    });
    rebind_quality_schema(&mut a, &mut p, schema);
    p.usage_inputs = Some(UsageInputPolicy::PobPhysicalPrimarySkillV1 {
        definitions: a.schema.identity().clone(),
        roles: *a.roles.identity(),
        catalog: a.roles.input().compilation.catalog_digest,
        scalar_inputs: gem_inventory_scalar_inputs_identity(&p, Default::default()).unwrap(),
        gems: vec![PrimarySkillUsageInput {
            gem: role.gem,
            game_id: "active".into(),
            variant_id: "v".into(),
            skill_id: "synthetic-effect".into(),
            name_spec: "Synthetic effect".into(),
            primary,
            supply,
            grant,
            policy: usage,
            attributes: [
                "gemId",
                "variantId",
                "skillId",
                "nameSpec",
                "level",
                "enabled",
                "intrinsic",
                "primaryFlag",
                "secondaryFlag",
                "reviewed",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            guards: vec![GemInputGuard {
                attribute: "reviewed".into(),
                allowed: vec![SourceComponent::Text("yes".into())],
            }],
            parameters: vec![
                GemParameterInput {
                    slot: first,
                    value: value_recipe("first-preference", "primaryFlag", true),
                },
                GemParameterInput {
                    slot: second,
                    value: value_recipe("second-preference", "secondaryFlag", true),
                },
            ],
        }],
    });
    (a, p)
}
const GEM: &str = r#"<Gem gemId="active" variantId="v" skillId="synthetic-effect" nameSpec="Synthetic effect" level="17" enabled="true" intrinsic="true" primaryFlag="true" secondaryFlag="false" reviewed="yes"/>"#;
fn row(p: &mut NormalizationPolicy) -> &mut PrimarySkillUsageInput {
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 { gems, .. } = p.usage_inputs.as_mut().unwrap();
    &mut gems[0]
}
fn preferences(result: &NormalizedImport) -> &DraftList<UsagePolicyDraft> {
    result.draft().input().skill_presets.members[0]
        .usage_preferences
        .as_ref()
        .unwrap()
}
fn run_limits(
    xml: &str,
    a: &Artifacts,
    p: &NormalizationPolicy,
    limits: NormalizationLimits,
) -> Result<NormalizedImport, NormalizationError> {
    let source = source(xml, 93);
    let evidence =
        SourceProjectEvidence::collect(&source, SourceEvidenceLimits::default()).unwrap();
    normalize_fresh(
        &evidence,
        *source.allocator_state(),
        NormalizationArtifacts {
            tree: None,
            items: &empty_items(&a.schema),
            item_source: &empty_item_source(&a.schema),
            mappings: &a.mapping,
            registry: &a.registry,
            definitions: &a.schema,
            roles: &a.roles,
            rewards: &a.rewards,
        },
        p,
        &[],
        limits,
    )
}

#[test]
fn usage_omission_preserves_wire_draft_sidecar_and_allocator() {
    let (a, mut p) = fixture();
    p.usage_inputs = None;
    let bytes = serde_json::to_vec(&p).unwrap();
    assert!(
        !String::from_utf8(bytes.clone())
            .unwrap()
            .contains("usage_inputs")
    );
    let decoded: NormalizationPolicy = serde_json::from_slice(&bytes).unwrap();
    let first = normalize_with_loadouts(&group(GEM), &a, &p).unwrap();
    let second = normalize_with_loadouts(&group(GEM), &a, &decoded).unwrap();
    assert_eq!(serde_json::to_vec(&decoded).unwrap(), bytes);
    assert_eq!(first.draft(), second.draft());
    assert_eq!(
        serde_json::to_vec(first.sidecar()).unwrap(),
        serde_json::to_vec(second.sidecar()).unwrap()
    );
    assert!(
        first
            .draft()
            .input()
            .skill_presets
            .members
            .iter()
            .all(|preset| preset.usage_preferences.is_none())
    );
    let mut unknown = serde_json::to_value(&p).unwrap();
    unknown["invented_usage_field"] = true.into();
    assert!(serde_json::from_value::<NormalizationPolicy>(unknown).is_err());
}

#[test]
fn usage_maps_each_fresh_occurrence_to_its_containing_preset_and_primary_supply() {
    let (a, p) = fixture();
    let off = GEM.replace("primaryFlag=\"true\"", "primaryFlag=\"false\"");
    let xml = format!(
        r#"<PathOfBuilding2><Skills activeSkillSet="8"><SkillSet id="7"><Skill enabled="false">{GEM}{off}</Skill></SkillSet><SkillSet id="8"><Skill enabled="true">{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let source = source(&xml, 93);
    let result = run_with_policy(&source, &a, &[], &p);
    let draft = result.draft().input();
    assert_eq!(draft.skills.members.len(), 3);
    assert_eq!(draft.skill_presets.members.len(), 2);
    let UsageInputPolicy::PobPhysicalPrimarySkillV1 { gems, .. } = p.usage_inputs.as_ref().unwrap();
    for (preset, expected) in draft
        .skill_presets
        .members
        .iter()
        .zip([vec![true, false], vec![true]])
    {
        let preferences = preset.usage_preferences.as_ref().unwrap();
        assert!(
            matches!(&preferences.completion, DraftListCompletion::Pending { code, .. } if code.as_str() == "usage-preferences-not-converted")
        );
        assert_eq!(preferences.members.len(), expected.len());
        for ((record, skill), enabled) in preferences
            .members
            .iter()
            .zip(&preset.skills.members)
            .zip(expected)
        {
            assert_eq!(record.policy.to_resolved(), Some(gems[0].policy.clone()));
            assert_eq!(
                record.target.to_resolved(),
                Some(UsageTarget::Skill(SkillTarget::Generated(Box::new(
                    GeneratedSkillKey {
                        provider: ProviderKey {
                            root: ProviderRoot::SkillUse(*skill),
                            grant_path: vec![]
                        },
                        slot: gems[0].supply.clone(),
                    }
                ))))
            );
            assert!(matches!(
                record.parameters.completion,
                DraftListCompletion::Complete
            ));
            assert_eq!(
                record.parameters.members[0].value.to_resolved(),
                Some(ParameterValue::Boolean(enabled))
            );
        }
    }
    assert_eq!(draft.skills.members[0].enabled.to_resolved(), Some(false));
    assert!(draft.gems.members.iter().all(|gem| matches!(
        gem.parameters.completion,
        DraftListCompletion::Pending { .. }
    ) && gem.parameters.members.len() == 1));
    assert!(
        draft
            .scenario_presets
            .members
            .iter()
            .all(|scenario| matches!(
                scenario.scenario.usage.completion,
                DraftListCompletion::Pending { .. }
            ))
    );
    for origin in &result.sidecar().origins {
        if origin
            .links
            .iter()
            .any(|link| matches!(link, OwnedOriginTarget::Skill(_)))
            && source.occurrence(origin.source).unwrap().name() == "Gem"
        {
            assert!(
                origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::SkillPreset(_)))
            );
            assert!(
                origin
                    .links
                    .iter()
                    .any(|link| matches!(link, OwnedOriginTarget::Gem(_)))
            );
        }
    }
    origin_integrity(&source, &result);
}

#[test]
fn usage_failed_frame_or_guard_preserves_known_intrinsic_values_and_pending_inventory() {
    let (a, p) = fixture();
    for raw in [
        GEM.replace("reviewed=\"yes\"", "reviewed=\"no\""),
        GEM.replace(" reviewed=\"yes\"", ""),
        GEM.replace(
            "skillId=\"synthetic-effect\"",
            "skillId=\"unreviewed-effect\"",
        ),
        GEM.replace("nameSpec=\"Synthetic effect\"", "nameSpec=\"different\""),
        GEM.replace("/>", " unexpected=\"true\"/>"),
        GEM.replace("/>", "><Input/></Gem>"),
        GEM.replace("/>", ">unexpected text</Gem>"),
    ] {
        let result = normalize_with_loadouts(&group(&raw), &a, &p).unwrap();
        assert!(preferences(&result).members.is_empty(), "{raw}");
        assert!(matches!(
            preferences(&result).completion,
            DraftListCompletion::Pending { .. }
        ));
        let gem = &result.draft().input().gems.members[0];
        assert_eq!(
            gem.parameters.members[0].value.to_resolved(),
            Some(ParameterValue::Boolean(true))
        );
        assert!(matches!(
            gem.parameters.completion,
            DraftListCompletion::Pending { .. }
        ));
    }
    // XML duplicate attributes are rejected before source evidence exists.
    let duplicate = group(&GEM.replace("reviewed=\"yes\"", "reviewed=\"yes\" reviewed=\"no\""));
    assert!(decode_build(duplicate.as_bytes()).is_err());
}

#[test]
fn usage_malformed_missing_and_unknown_boolean_keeps_other_known_parameter() {
    let (a, p) = fixture();
    for raw in [
        GEM.replace("secondaryFlag=\"false\"", "secondaryFlag=\"unrecognized\""),
        GEM.replace("secondaryFlag=\"false\"", "secondaryFlag=\"1\""),
        GEM.replace("secondaryFlag=\"false\"", "secondaryFlag=\" false\""),
        GEM.replace(" secondaryFlag=\"false\"", ""),
    ] {
        let result = normalize_with_loadouts(&group(&raw), &a, &p).unwrap();
        let records = &preferences(&result).members;
        assert_eq!(records.len(), 1, "{raw}");
        assert!(
            matches!(&records[0].parameters.completion, DraftListCompletion::Pending { code, .. } if code.as_str() == "usage-parameters-not-converted")
        );
        assert_eq!(records[0].parameters.members.len(), 1);
        assert_eq!(
            records[0].parameters.members[0].value.to_resolved(),
            Some(ParameterValue::Boolean(true))
        );
    }
}

#[test]
fn usage_authoring_rejects_invalid_ownership_domain_guards_and_recipe_types() {
    let (a, p) = fixture();
    for case in 0..16 {
        let mut bad = p.clone();
        let rule = row(&mut bad);
        match case {
            0 => rule.parameters[0].slot.declaration = SlotOwnerDefId::Gem(rule.gem.clone()),
            1 => rule.parameters[0].value = value_recipe("not-boolean", "primaryFlag", false),
            2 => {
                rule.parameters[0].value.missing = MissingValuePolicy::Explicit {
                    value: ParameterValue::Boolean(true),
                }
            }
            3 => rule.parameters[0].value.missing = MissingValuePolicy::Absent,
            4 => rule.parameters[0].value.tiers[0].selectors[0].lane = ValueLane::ParentAttribute,
            5 => rule.parameters[0].value.tiers[0].selectors[0].name = "not-admitted".into(),
            6 => {
                rule.parameters.pop();
            }
            7 => rule.parameters.push(rule.parameters[0].clone()),
            8 => rule.guards.push(rule.guards[0].clone()),
            9 => rule.guards[0].allowed.clear(),
            10 => rule.attributes.push(rule.attributes[0].clone()),
            11 => rule.attributes.retain(|name| name != "gemId"),
            12 => rule.supply.declaration = SlotOwnerDefId::Skill(rule.primary.clone()),
            13 => rule.grant.declaration = SlotOwnerDefId::Skill(rule.primary.clone()),
            14 => rule.game_id = "unknown-source".into(),
            15 => {
                rule.parameters[0].value.codec.namespace =
                    GameVersionNamespace::new("other", "v2").unwrap()
            }
            _ => unreachable!(),
        }
        assert!(
            normalize_with_loadouts(&group(GEM), &a, &bad).is_err(),
            "case {case}"
        );
    }
    let mut json = serde_json::to_value(p.usage_inputs.unwrap()).unwrap();
    json["gems"][0]["invented_admission"] = true.into();
    assert!(serde_json::from_value::<UsageInputPolicy>(json).is_err());
}

#[test]
fn usage_checks_all_bindings_and_charges_schema_recipe_and_row_work() {
    let (a, p) = fixture();
    let wrong = "0".repeat(64).parse().unwrap();
    for case in 0..6 {
        let mut bad = p.clone();
        let UsageInputPolicy::PobPhysicalPrimarySkillV1 {
            definitions,
            roles,
            catalog,
            scalar_inputs,
            gems,
        } = bad.usage_inputs.as_mut().unwrap();
        match case {
            0 => definitions.release = "stale".into(),
            1 => *roles = wrong,
            2 => *catalog = wrong,
            3 => *scalar_inputs = wrong,
            4 => bad.gem_enabled = value_recipe("changed-source-contract", "enabled", true),
            5 => gems.push(gems[0].clone()),
            _ => unreachable!(),
        }
        assert!(
            normalize_with_loadouts(&group(GEM), &a, &bad).is_err(),
            "case {case}"
        );
    }
    let before = serde_json::to_vec(&p).unwrap();
    let xml = group(GEM);
    for maximum in [1, 100, 1_000] {
        assert!(
            run_limits(
                &xml,
                &a,
                &p,
                NormalizationLimits {
                    max_work: maximum,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
    assert!(
        run_limits(
            &xml,
            &a,
            &p,
            NormalizationLimits {
                max_policy_bytes: 100,
                ..Default::default()
            }
        )
        .is_err()
    );
    let result = normalize_with_loadouts(&xml, &a, &p).unwrap();
    assert_eq!(preferences(&result).members.len(), 1);
    let bounded = NormalizationLimits {
        max_work: 20_000,
        ..Default::default()
    };
    assert!(run_limits(&xml, &a, &p, bounded).is_ok());
    let costly = group(&GEM.replace(
        "reviewed=\"yes\"",
        &format!("reviewed=\"{}\"", "a".repeat(30_000)),
    ));
    assert!(matches!(
        run_limits(&costly, &a, &p, bounded),
        Err(NormalizationError::Limit(_))
    ));
    assert_eq!(serde_json::to_vec(&p).unwrap(), before);
}
