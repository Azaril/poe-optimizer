//! Exact generated applicability and final projection are storage authority only.
use super::*;

fn provider(context: RuleEntityKind) -> SlotOwnerDefId {
    match context {
        RuleEntityKind::Actor => SlotOwnerDefId::PassiveNode(id("generated-provider")),
        RuleEntityKind::EquipmentUse => SlotOwnerDefId::Modifier(id("generated-provider")),
        RuleEntityKind::Skill => SlotOwnerDefId::Skill(id("generated-provider")),
        _ => unreachable!(),
    }
}
fn provider_subject(context: RuleEntityKind) -> SchemaSubject {
    SchemaSubject::Definition(match provider(context) {
        SlotOwnerDefId::PassiveNode(id) => id.address(),
        SlotOwnerDefId::Modifier(id) => id.address(),
        SlotOwnerDefId::Skill(id) => id.address(),
        _ => unreachable!(),
    })
}
fn generated(context: RuleEntityKind) -> Fixture {
    generated_with(context, |_| {}, |_| {}, |_| {})
}
fn generated_with(
    context: RuleEntityKind,
    edit_schema: impl FnOnce(&mut SchemaPackageInput),
    edit_rules: impl FnOnce(&mut RulePackageInput),
    edit_stages: impl FnOnce(&mut EvaluationStagesInput),
) -> Fixture {
    let supply = slot(provider(context), "source");
    let final_input = slot(parent(), "prepared-level");
    let raw_input = slot(parent(), "raw-level");
    let mut f = fixture_with_stages(
        |schema| {
            schema.schema_version = OWNED_SCHEMA_PACKAGE_V6;
            let mut declarations = ports();
            for name in ["source", "sibling"] {
                let slot = slot(provider(context), name);
                declarations.skill_grants.members.push(slot.clone());
                schema.slots.push(SlotDescriptor::SkillGrant(entry(
                    slot,
                    SkillGrantSlotSchema {
                        skill: id("parent"),
                        outputs: complete(vec![]),
                        preset_inputs: None,
                    },
                )));
            }
            schema.definitions.push(match context {
                RuleEntityKind::Actor => DefinitionDescriptor::PassiveNode(entry(
                    id("generated-provider"),
                    PassiveNodeSchema {
                        pools: complete(vec![]),
                        adjacent: complete(vec![]),
                        declarations,
                    },
                )),
                RuleEntityKind::EquipmentUse => DefinitionDescriptor::Modifier(entry(
                    id("generated-provider"),
                    ModifierSchema { declarations },
                )),
                RuleEntityKind::Skill => DefinitionDescriptor::Skill(entry(
                    id("generated-provider"),
                    SkillSchema {
                        directly_selectable: false,
                        declarations,
                    },
                )),
                _ => unreachable!(),
            });
            for row in &mut schema.definitions {
                if let DefinitionDescriptor::Skill(row) = row
                    && row.id == id::<SkillDefinition>("parent")
                {
                    let SchemaState::Known(skill) = &mut row.schema else {
                        unreachable!()
                    };
                    skill.declarations.parameters =
                        complete(vec![raw_input.clone(), final_input.clone()]);
                }
            }
            for parameter in [&raw_input, &final_input] {
                schema.slots.push(SlotDescriptor::Parameter(entry(
                    parameter.clone(),
                    ParameterSlotSchema {
                        value: ValueSchema::Integer(range()),
                        presence: SlotPresence::RequiredOnce,
                        sites: vec![],
                        skill_input: Some(SkillInputAuthority::Projected),
                    },
                )));
            }
            edit_schema(schema);
        },
        |rules| {
            let mut program = assembly();
            program.id = key("source-provider-assembly");
            program.context = context;
            program.effects[0].effect = RuleEffectKind::ProjectSkillParameter {
                skill: supply.clone(),
                parameter: final_input.clone(),
                value: key("value"),
            };
            rules.owners.push(DefinitionRules {
                owner: provider_subject(context),
                programs: complete(vec![program]),
            });
            edit_rules(rules);
        },
        |stages| {
            stages
                .programs
                .members
                .iter_mut()
                .find(|p| p.program == key("source-provider-assembly"))
                .unwrap()
                .stage = key("source-assembly");
            let readiness = stages.readiness.as_mut().unwrap();
            let program = readiness
                .programs
                .members
                .iter_mut()
                .find(|p| p.program == key("source-provider-assembly"))
                .unwrap();
            program.phase = ReadinessPhase::Preparation;
            program.role = ReadinessProgramRole::SourceFinalInputAssembly;
            program.outputs = vec![StageChannel::SkillParameter {
                parameter: final_input.clone(),
            }];
            readiness.skills.push(SkillReadiness {
                skill: id("parent"),
                parameters: complete(vec![
                    ParameterReadiness {
                        parameter: raw_input.clone(),
                        phase: ReadinessPhase::Preparation,
                    },
                    ParameterReadiness {
                        parameter: final_input.clone(),
                        phase: ReadinessPhase::Execution,
                    },
                ]),
                participation: None,
            });
            edit_stages(stages);
        },
    );
    let row = relation(&mut f.input);
    row.occurrence = SourcePropertyOccurrence::GeneratedSkill {
        skill_supply: supply,
    };
    row.assembly.members.push(SourcePropertyAssemblyProgram {
        program: key("source-provider-assembly"),
        binding: SourcePropertyAssemblyBinding::ExactSupplyingProvider,
    });
    f
}

#[test]
fn generated_sources_preserve_three_natural_provider_contexts_and_roundtrip() {
    for context in [
        RuleEntityKind::Actor,
        RuleEntityKind::EquipmentUse,
        RuleEntityKind::Skill,
    ] {
        let f = generated(context);
        let checked = f.build(f.input.clone()).unwrap();
        let bytes = encode_support_receiving(&checked, Default::default()).unwrap();
        let round = decode_support_receiving(
            &bytes,
            &f.schema,
            &f.rules,
            &f.preparation,
            &f.inputs,
            &f.stages,
            Default::default(),
        )
        .unwrap();
        assert_eq!(checked.identity(), round.identity());
        assert!(checked.declarations_complete());
    }
}

#[test]
fn generated_applicability_is_distinct_from_authored_and_other_exact_supplies() {
    let f = generated(RuleEntityKind::EquipmentUse);
    let mut raw = f.input.clone();
    let mut authored = relation(&mut raw).clone();
    authored.id = key("authored");
    authored.occurrence = SourcePropertyOccurrence::AuthoredSkillUse {};
    authored
        .assembly
        .members
        .retain(|p| p.binding == SourcePropertyAssemblyBinding::InputOwner);
    raw.source_properties
        .as_mut()
        .unwrap()
        .relations
        .members
        .push(authored);
    let mut sibling = relation(&mut raw).clone();
    sibling.id = key("sibling");
    sibling.occurrence = SourcePropertyOccurrence::GeneratedSkill {
        skill_supply: slot(provider(RuleEntityKind::EquipmentUse), "sibling"),
    };
    sibling
        .assembly
        .members
        .retain(|p| p.binding == SourcePropertyAssemblyBinding::InputOwner);
    raw.source_properties
        .as_mut()
        .unwrap()
        .relations
        .members
        .push(sibling);
    f.build(raw.clone()).unwrap();
    let mut duplicate = relation(&mut raw).clone();
    duplicate.id = key("duplicate");
    raw.source_properties
        .as_mut()
        .unwrap()
        .relations
        .members
        .push(duplicate);
    f.bad(raw, "applicability");
}

#[test]
fn generated_owner_requires_exact_declared_supply_but_not_direct_selectability() {
    let f = generated_with(
        RuleEntityKind::Actor,
        |schema| {
            for row in &mut schema.definitions {
                if let DefinitionDescriptor::Skill(row) = row
                    && row.id == id::<SkillDefinition>("parent")
                {
                    let SchemaState::Known(skill) = &mut row.schema else {
                        unreachable!()
                    };
                    skill.directly_selectable = false;
                }
            }
        },
        |_| {},
        |_| {},
    );
    f.build(f.input.clone()).unwrap();
    let mut raw = f.input.clone();
    relation(&mut raw).occurrence = SourcePropertyOccurrence::AuthoredSkillUse {};
    f.bad(raw, "not directly selectable");
    let mut raw = f.input.clone();
    relation(&mut raw).occurrence = SourcePropertyOccurrence::GeneratedSkill {
        skill_supply: slot(provider(RuleEntityKind::Actor), "undeclared"),
    };
    f.bad(raw, "not declared");
    let mut raw = f.input.clone();
    relation(&mut raw).owner = SupportTargetDefinition::Gem(id("active"));
    // Complete input-owner rules are still mandatory even for an invalid relation.
    assert!(f.build(raw).is_err());
    let f = generated_with(
        RuleEntityKind::Actor,
        |schema| {
            for row in &mut schema.slots {
                if let SlotDescriptor::SkillGrant(row) = row
                    && row.id == slot(provider(RuleEntityKind::Actor), "source")
                {
                    let SchemaState::Known(supply) = &mut row.schema else {
                        unreachable!()
                    };
                    supply.skill = id("child");
                }
            }
        },
        |_| {},
        |_| {},
    );
    f.bad(f.input.clone(), "differs from exact supplied Skill");
}

#[test]
fn exact_provider_assembly_rejects_authored_binding_siblings_and_wrong_context() {
    let f = generated(RuleEntityKind::EquipmentUse);
    let mut raw = f.input.clone();
    relation(&mut raw).occurrence = SourcePropertyOccurrence::AuthoredSkillUse {};
    f.bad(raw, "requires a generated source");
    let mut raw = f.input.clone();
    relation(&mut raw).occurrence = SourcePropertyOccurrence::GeneratedSkill {
        skill_supply: slot(provider(RuleEntityKind::EquipmentUse), "sibling"),
    };
    f.bad(raw, "exact supplied Skill");
    let f = generated_with(
        RuleEntityKind::EquipmentUse,
        |_| {},
        |rules| {
            rules
                .owners
                .iter_mut()
                .find(|o| o.owner == provider_subject(RuleEntityKind::EquipmentUse))
                .unwrap()
                .programs
                .members[0]
                .context = RuleEntityKind::Actor;
        },
        |_| {},
    );
    f.bad(f.input.clone(), "preserve its input owner context");
}

#[test]
fn exact_provider_final_inputs_require_explicit_execution_projection_authority() {
    for authority in [
        None,
        Some(SkillInputAuthority::Projected),
        Some(SkillInputAuthority::AuthoredOrProjected),
    ] {
        let f = generated_with(
            RuleEntityKind::Actor,
            |schema| {
                for row in &mut schema.slots {
                    if let SlotDescriptor::Parameter(row) = row
                        && row.id == slot(parent(), "prepared-level")
                    {
                        let SchemaState::Known(parameter) = &mut row.schema else {
                            unreachable!()
                        };
                        parameter.skill_input = authority;
                        if authority == Some(SkillInputAuthority::AuthoredOrProjected) {
                            parameter.sites = vec![ParameterSite::SkillParameter];
                        }
                    }
                }
            },
            |_| {},
            |_| {},
        );
        if authority.is_some() {
            f.build(f.input.clone()).unwrap();
        } else {
            f.bad(f.input.clone(), "projected required execution inputs");
        }
    }
    for phase in [ReadinessPhase::Structural, ReadinessPhase::Preparation] {
        let f = generated_with(
            RuleEntityKind::Actor,
            |_| {},
            |_| {},
            |stages| {
                stages.readiness.as_mut().unwrap().skills[0]
                    .parameters
                    .members[1]
                    .phase = phase;
            },
        );
        f.bad(f.input.clone(), "projected required execution inputs");
    }
    let f = generated_with(
        RuleEntityKind::Actor,
        |_| {},
        |_| {},
        |stages| {
            stages.readiness.as_mut().unwrap().skills.clear();
        },
    );
    f.bad(f.input.clone(), "projected required execution inputs");
}

#[test]
fn exact_provider_binding_does_not_grant_local_stat_or_optional_parameter_writes() {
    let f = generated_with(
        RuleEntityKind::Skill,
        |_| {},
        |rules| {
            rules
                .owners
                .iter_mut()
                .find(|o| o.owner == provider_subject(RuleEntityKind::Skill))
                .unwrap()
                .programs
                .members[0]
                .effects[0]
                .effect = RuleEffectKind::Derive {
                entity: RuleEntity::Current,
                stat: id("source-final"),
                value: key("value"),
            };
        },
        |stages| {
            stages
                .readiness
                .as_mut()
                .unwrap()
                .programs
                .members
                .iter_mut()
                .find(|p| p.program == key("source-provider-assembly"))
                .unwrap()
                .outputs = vec![StageChannel::Stat {
                scope: RuleEntityKind::Skill,
                stat: id("source-final"),
            }];
        },
    );
    f.bad(f.input.clone(), "unauthorized effect");
    let f = generated_with(
        RuleEntityKind::Actor,
        |schema| {
            for row in &mut schema.slots {
                if let SlotDescriptor::Parameter(row) = row
                    && row.id == slot(parent(), "prepared-level")
                {
                    let SchemaState::Known(parameter) = &mut row.schema else {
                        unreachable!()
                    };
                    parameter.presence = SlotPresence::OptionalOnce;
                }
            }
        },
        |_| {},
        |stages| {
            stages.readiness.as_mut().unwrap().skills[0]
                .parameters
                .members
                .retain(|p| p.parameter != slot(parent(), "prepared-level"));
        },
    );
    f.bad(f.input.clone(), "projected required execution inputs");
}
