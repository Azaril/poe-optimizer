//! Actual flat physical producer; no reference Action is fabricated for receipts.
use super::*;

fn rebind_flat(f: &mut fixture::Fixture) {
    let definition = f.base.schema.identity().clone();
    let mut mapping = f.base.mapping.input().clone();
    mapping.definitions = definition.clone();
    mapping.registry = f.base.registry.identity().unwrap();
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.base.roles.input().clone();
    roles.definitions = definition.clone();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    let GemQualityPolicy::Attributes(quality) = &mut f.policy.gem_quality else {
        unreachable!()
    };
    quality.definitions = definition.clone();
    f.policy.gem_inputs.as_mut().unwrap().definitions = definition.clone();
    let scalar = gem_inventory_scalar_inputs_identity(&f.policy, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        definitions,
        roles,
        scalar_inputs,
        ..
    } = f.policy.usage_inputs.as_mut().unwrap();
    *definitions = definition.clone();
    *roles = *f.base.roles.identity();
    *scalar_inputs = scalar;
    let usage = usage_inputs_identity(&f.policy, Default::default()).unwrap();
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        definitions,
        roles,
        scalar_inputs,
        usage_inputs,
        ..
    } = f.policy.gem_inventory.as_mut().unwrap();
    *definitions = definition.clone();
    *roles = *f.base.roles.identity();
    *scalar_inputs = scalar;
    *usage_inputs = usage;
    let mut rewards = f.rewards.input().clone();
    rewards.definitions = definition;
    rewards.mapping = *f.base.mapping.identity();
    f.rewards =
        OwnedRewardPolicy::new(rewards, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
}

fn flat_fixture(complete_parameters: bool) -> fixture::Fixture {
    let mut f = group_selector_fixture();
    let designated = f
        .base
        .registry
        .allocate_definition::<UsagePolicyDefinition>()
        .unwrap();
    let parameter = f
        .base
        .registry
        .allocate_slot::<ParameterSlotDefinition>(SlotOwnerDefId::UsagePolicy(designated.clone()))
        .unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
        f.policy.usage_inputs.as_ref().unwrap();
    let original = physical[0].policies[0].clone();
    let mut schema = f.base.schema.input().clone();
    let SchemaLookup::Known(policy_schema) = f.base.schema.definition(&original.policy) else {
        unreachable!()
    };
    let mut policy_schema = policy_schema.clone();
    policy_schema.declarations.parameters = DeclaredSet::complete(vec![parameter.clone()]);
    schema
        .definitions
        .push(DefinitionDescriptor::UsagePolicy(DefinitionEntry {
            id: designated.clone(),
            schema: SchemaState::Known(policy_schema),
        }));
    let SchemaLookup::Known(parameter_schema) = f.base.schema.slot(&original.parameters[1].slot)
    else {
        unreachable!()
    };
    schema
        .slots
        .push(SlotDescriptor::Parameter(DefinitionEntry {
            id: parameter.clone(),
            schema: SchemaState::Known(parameter_schema.clone()),
        }));
    for descriptor in &mut schema.definitions {
        match descriptor {
            DefinitionDescriptor::Gem(entry) if entry.id == f.base.gem => {
                let SchemaState::Known(value) = &mut entry.schema else {
                    unreachable!()
                };
                if complete_parameters {
                    value.declarations.parameters =
                        DeclaredSet::complete(value.declarations.parameters.members.clone());
                }
            }
            DefinitionDescriptor::Skill(entry) => {
                let SchemaState::Known(value) = &mut entry.schema else {
                    unreachable!()
                };
                value.declarations.outputs = DeclaredSet::partial(
                    vec![],
                    vec![SchemaGap {
                        subject: SchemaSubject::Definition(DefinitionAddress::Skill(
                            entry.id.clone(),
                        )),
                        facet: SchemaFacet::StaticLinks,
                        code: actions::key("fixture-no-reference-output"),
                    }],
                );
            }
            _ => {}
        }
    }
    for descriptor in &mut schema.slots {
        if let SlotDescriptor::SkillGrant(entry) = descriptor {
            let SchemaState::Known(value) = &mut entry.schema else {
                unreachable!()
            };
            value.outputs = DeclaredSet::partial(
                vec![],
                vec![SchemaGap {
                    subject: SchemaSubject::Slot(SlotAddress::SkillGrant(entry.id.clone())),
                    facet: SchemaFacet::StaticLinks,
                    code: actions::key("fixture-no-reference-output"),
                }],
            );
        }
    }
    f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let UsageInputPolicy::PobOccurrenceUsageV3 {
        physical,
        inventory: Some(inventory),
        ..
    } = f.policy.usage_inputs.as_mut().unwrap()
    else {
        unreachable!()
    };
    physical[0].policies.push(OccurrenceUsagePolicy {
        policy: designated.clone(),
        parameters: vec![UsageParameterInput {
            slot: parameter.clone(),
            source: original.parameters[1].source.clone(),
        }],
    });
    physical[0].guards.extend([
        GemInputGuard {
            attribute: "count".into(),
            allowed: vec![SourceComponent::Text("1".into())],
        },
        GemInputGuard {
            attribute: "enableGlobal2".into(),
            allowed: vec![SourceComponent::Text("true".into())],
        },
    ]);
    inventory.rows[0].fields.iter_mut().find(|field| matches!(&field.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == "enableGlobal1")).unwrap().destinations.push(UsageInventoryDestination::UsageParameter { policy: designated.clone(), parameter });
    let GemInventoryPolicy::PobFreshPhysicalV3 {
        primary_dispositions,
        primary_skills,
        ..
    } = f.policy.gem_inventory.as_mut().unwrap();
    let disposition = primary_dispositions.remove(0);
    primary_skills.push(PrimarySkillGemInventory {
        physical: disposition.physical,
        usage_policy: designated,
    });
    rebind_flat(&mut f);
    f
}

fn disable_inventory(f: &mut fixture::Fixture) {
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    rebind_flat(f);
}

fn assert_only_usage_retirement(before: &NormalizedImport, after: &NormalizedImport) {
    let mut expected = before.draft().input().clone();
    let mut origins = before.sidecar().origins.clone();
    for preset in &mut expected.skill_presets.members {
        let usage = preset.usage_preferences.as_mut().unwrap();
        let DraftListCompletion::Pending { id, .. } = usage.completion else {
            unreachable!()
        };
        usage.completion = DraftListCompletion::Complete;
        for origin in &mut origins {
            origin
                .links
                .retain(|link| *link != OwnedOriginTarget::Issue(id));
        }
    }
    assert_eq!(
        &expected,
        after.draft().input(),
        "including allocator and every unrelated output"
    );
    assert_eq!(
        origins,
        after.sidecar().origins,
        "no redundant source/group links"
    );
}

#[test]
fn actual_flat_primary_proof_completes_usage_without_reference_actions_for_both_scalar_closures() {
    let text = selected_group(GEM, "nil", "1");
    let mut results = Vec::new();
    for scalar_complete in [false, true] {
        let mut f = flat_fixture(scalar_complete);
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [true]);
        assert!(matches!(
            after.draft().input().gems.members[0].parameters.completion,
            DraftListCompletion::Complete
        ));
        let UsageInputPolicy::PobOccurrenceUsageV3 { physical, .. } =
            f.policy.usage_inputs.as_ref().unwrap();
        let SchemaLookup::Known(skill) = f.base.schema.definition(&physical[0].primary) else {
            unreachable!()
        };
        assert!(!skill.declarations.outputs.is_complete());
        assert!(skill.declarations.outputs.members.is_empty());
        disable_inventory(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&before), [false]);
        assert_only_usage_retirement(&before, &after);
        results.push(after.draft().input().clone());
    }
    assert_eq!(
        results[0], results[1],
        "scalar coverage cannot erase the source proof"
    );
}

#[test]
fn flat_primary_refuses_reference_fields_children_and_failed_designated_usage() {
    for change in [
        "statSetIndex",
        "statSetIndexCalcs",
        "children",
        "bad-usage",
        "wrong-skill",
        "wrong-level",
    ] {
        let mut f = flat_fixture(true);
        let mut raw = GEM.to_owned();
        match change {
            "statSetIndex" | "statSetIndexCalcs" => {
                raw = raw.replace("/>", &format!(" {change}=\"nil\"/>"));
                let UsageInputPolicy::PobOccurrenceUsageV3 {
                    physical,
                    inventory: Some(inventory),
                    ..
                } = f.policy.usage_inputs.as_mut().unwrap()
                else {
                    unreachable!()
                };
                physical[0].attributes.push(change.into());
                inventory.rows[0].fields.push(UsageInventoryField {
                    field: UsageInventoryFieldSelector::OccurrenceAttribute {
                        name: change.into(),
                    },
                    destinations: vec![UsageInventoryDestination::InputDisposition],
                });
            }
            "children" => raw = fixture::with_children(&fixture::maps("1", "1")),
            "bad-usage" => raw = raw.replace("enableGlobal1=\"true\"", "enableGlobal1=\"bad\""),
            "wrong-skill" => raw = raw.replace("skillId=\"effect\"", "skillId=\"other\""),
            _ => raw = raw.replace("level=\"17\"", "level=\"999\""),
        }
        rebind_flat(&mut f);
        let text = selected_group(&raw, "nil", "1");
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [false], "{change}");
        disable_inventory(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_eq!(before.draft().input(), after.draft().input(), "{change}");
        assert_eq!(
            before.sidecar().origins,
            after.sidecar().origins,
            "{change}"
        );
    }
}

#[test]
fn designated_policy_is_exact_and_other_consumers_remain_conjunctive() {
    let mut f = flat_fixture(false);
    let GemInventoryPolicy::PobFreshPhysicalV3 { primary_skills, .. } =
        f.policy.gem_inventory.as_mut().unwrap();
    primary_skills[0].usage_policy =
        UsagePolicyDefId::new(actions::ns(), actions::key("absent-designated"));
    rebind_flat(&mut f);
    assert!(f.run(&xml(GEM), Default::default()).is_err());
    for field in ["count", "enableGlobal1", "enableGlobal2"] {
        let mut f = flat_fixture(false);
        inventory::inventory(&mut f).rows[0].fields.iter_mut().find(|row| matches!(&row.field, UsageInventoryFieldSelector::OccurrenceAttribute { name } if name == field)).unwrap().destinations.push(UsageInventoryDestination::Deferred { reason: actions::key("independent-consumer") });
        rebind_flat(&mut f);
        let after = f
            .run(&selected_group(GEM, "nil", "1"), Default::default())
            .unwrap();
        assert_eq!(completed(&after), [false]);
        assert!(matches!(
            after.draft().input().gems.members[0].parameters.completion,
            DraftListCompletion::Complete
        ));
        disable_inventory(&mut f);
        let before = f
            .run(&selected_group(GEM, "nil", "1"), Default::default())
            .unwrap();
        assert_eq!(before.draft().input(), after.draft().input());
        assert_eq!(before.sidecar().origins, after.sidecar().origins);
    }
}

#[test]
fn flat_primary_uses_actual_source_and_set_links_when_no_group_value_contributed() {
    for scalar_complete in [false, true] {
        let mut f = flat_fixture(scalar_complete);
        let UsageInputPolicy::PobOccurrenceUsageV3 {
            physical,
            inventory: Some(inventory),
            ..
        } = f.policy.usage_inputs.as_mut().unwrap()
        else {
            unreachable!()
        };
        let UsageValueSource::ContainingGroupOverride { occurrence, .. } =
            physical[0].policies[0].parameters[0].source.clone()
        else {
            unreachable!()
        };
        physical[0].policies[0].parameters[0].source =
            UsageValueSource::Occurrence { value: occurrence };
        physical[0].group_attributes.clear();
        inventory.rows[0].fields.retain(|field| !matches!(&field.field, UsageInventoryFieldSelector::GroupAttribute { name } if name == "groupCount"));
        rebind_flat(&mut f);
        let text = selected_group(GEM, "nil", "1");
        let source = poe_optimizer_import::build_instance::ImportedBuildInstance::from_decoded(
            poe_optimizer_import::decode_build(text.as_bytes()).unwrap(),
            poe_optimizer_core::build_identity::BuildLineage::from_bytes([71; 16]),
            Default::default(),
        )
        .unwrap();
        let groups: Vec<_> = source
            .occurrences()
            .iter()
            .filter(|row| row.name() == "Skill")
            .collect();
        assert_eq!(groups.len(), 1);
        let after = f.run(&text, Default::default()).unwrap();
        assert_eq!(completed(&after), [true]);
        let group_origin = after
            .sidecar()
            .origins
            .iter()
            .find(|row| row.source == groups[0].id())
            .unwrap();
        assert!(
            !group_origin
                .links
                .iter()
                .any(|link| matches!(link, OwnedOriginTarget::SkillPreset(_))),
            "the exact noncontributing group gains no fabricated preset attribution"
        );
        disable_inventory(&mut f);
        let before = f.run(&text, Default::default()).unwrap();
        assert_only_usage_retirement(&before, &after);
    }
}

#[test]
fn repeated_flat_sources_keep_independent_presets_bindings_and_allocator() {
    let mut f = flat_fixture(false);
    let text = format!(
        r#"<PathOfBuilding2><Build level="70"/><Skills activeSkillSet="1"><SkillSet id="1"><Skill enabled="true" mainActiveSkill="nil" mainActiveSkillCalcs="1">{GEM}</Skill></SkillSet><SkillSet id="2"><Skill enabled="true" mainActiveSkill="1" mainActiveSkillCalcs="nil">{GEM}</Skill></SkillSet></Skills></PathOfBuilding2>"#
    );
    let after = f.run(&text, Default::default()).unwrap();
    assert_eq!(completed(&after), [true, true]);
    let presets = &after.draft().input().skill_presets.members;
    assert_ne!(presets[0].skills.members, presets[1].skills.members);
    assert_ne!(
        presets[0].usage_preferences.as_ref().unwrap().members[1].target,
        presets[1].usage_preferences.as_ref().unwrap().members[1].target
    );
    disable_inventory(&mut f);
    let before = f.run(&text, Default::default()).unwrap();
    assert_only_usage_retirement(&before, &after);
}

#[test]
fn complete_scalars_retain_full_reference_proof_only_after_existing_destination() {
    let mut f = group_selector_fixture();
    let mut schema = f.base.schema.input().clone();
    for descriptor in &mut schema.definitions {
        if let DefinitionDescriptor::Gem(entry) = descriptor
            && entry.id == f.base.gem
        {
            let SchemaState::Known(value) = &mut entry.schema else {
                unreachable!()
            };
            value.declarations.parameters =
                DeclaredSet::complete(value.declarations.parameters.members.clone());
        }
    }
    f.base.schema = OwnedDefinitionSchemaPackage::new(schema, Default::default()).unwrap();
    let mut mapping = f.base.mapping.input().clone();
    mapping.definitions = f.base.schema.identity().clone();
    f.base.mapping = OwnedMappingIndex::new(
        mapping,
        &f.base.registry,
        &f.base.schema,
        Default::default(),
    )
    .unwrap();
    let mut roles = f.base.roles.input().clone();
    roles.definitions = f.base.schema.identity().clone();
    roles.mapping = *f.base.mapping.identity();
    f.base.roles =
        OwnedSkillRoleIndex::new(roles, &f.base.mapping, &f.base.schema, Default::default())
            .unwrap();
    rebind_selector_fixture(&mut f);
    let text = selected_group(GEM, "nil", "1");
    let after = f.run(&text, Default::default()).unwrap();
    assert_eq!(completed(&after), [true]);
    let UsageInputPolicy::PobOccurrenceUsageV3 { inventory, .. } =
        f.policy.usage_inputs.as_mut().unwrap();
    *inventory = None;
    inventory::rebind(&mut f);
    let before = f.run(&text, Default::default()).unwrap();
    assert_only_usage_retirement(&before, &after);
}
